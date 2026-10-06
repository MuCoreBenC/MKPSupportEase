/**
 * 判据 —— 「Rust 侧要求的 `Channel` 参数，前端必须真的挂上一条」。
 *
 * # 为什么单独一条判据（2026-10-06 首页「下载并应用」死路）
 *
 * Tauri 的 `Channel<T>` **只能当必填参数**：它只实现了 `CommandArg`（要 `Webview` 才能建），
 * **没有 `Deserialize`** —— 所以 `Option<Channel<T>>` 编译不出来，Rust 侧没法表达"这个参数可省"。
 * 于是契约里那句「`onTick` 可选：不给就是点了等结果」，**只能在 IPC 边界（`src/api/bridge.ts`）兑现**。
 *
 * 不兑现的后果不是"少个回调"，而是 `CommandItem::deserialize_json` 找不到 key
 * （`command download_runtime_file missing required key onTick`）→ `InvalidArgs` →
 * **命令体一行都不跑**（Rust 日志里连一条都没有），界面拿到的是 bridge 兜底的
 * 「出了点问题，请重试」——一句没有信息量的话，真实原因（比如 `ShaMismatch`）被它盖住。
 *
 * 这种错**编译期抓不到、运行期只在真机上炸**（浏览器 mock 那条路根本不经过 `invoke`），
 * 所以按本仓的老规矩：用一个静态扫描把它钉住，红的时候说清楚为什么。
 *
 * 两道闸，各回答一个问题：
 *
 *   ① 调用点闸  凡 Rust 命令签名里有 `Channel<` 的，bridge 里调它的那个 `call(...)`
 *               必须把参数包在 `withTick(` 里（**哪怕调用方不给回调**）
 *   ② 兜底闸    `withTick` 自己不许"没有回调就原样返回入参" —— 那正是这次踩的坑
 *
 * 它对"间接调用"（A 调 B、B 最终 invoke）无能为力 —— 与 `check-zero-network.mjs`
 * 同一个层次的承诺：这两个具体形状一定会红，并且说清楚为什么。
 */

import { readFileSync, readdirSync, statSync } from 'node:fs'
import { join, sep } from 'node:path'

/** Rust 源码目录（命令签名在这边） */
const RUST_SRC = 'src-tauri/src'
/** IPC 桥（唯一把契约方法映射到 command 的地方） */
const BRIDGE = 'src/api/bridge.ts'
/** Tauri 里"必填 channel"的那个类型 */
const CHANNEL_PARAM = 'Channel<'

function norm(file) {
  return file.split(sep).join('/')
}

function walk(dir, only) {
  const out = []
  for (const name of readdirSync(dir)) {
    const full = join(dir, name)
    if (statSync(full).isDirectory()) out.push(...walk(full, only))
    else if (only(full)) out.push(full)
  }
  return out
}

function readSafe(file) {
  try {
    return readFileSync(file, 'utf8')
  } catch {
    return null
  }
}

/**
 * Rust 侧：哪些命令的签名里要一条 `Channel`。
 *
 * 判法 = `#[tauri::command]` 之后、下一个 `) ->` 之前的形参表里出现 `Channel<`。
 * 命令名取 `fn` 后面那个标识符。
 */
function commandsNeedingChannel() {
  const out = new Map()
  for (const file of walk(RUST_SRC, (f) => f.endsWith('.rs'))) {
    const text = readSafe(file)
    if (text === null) continue
    const re = /#\[tauri::command\][\s\S]*?\bfn\s+(\w+)\s*\(([\s\S]*?)\)\s*->/g
    let m
    while ((m = re.exec(text)) !== null) {
      const [, name, params] = m
      if (params.includes(CHANNEL_PARAM)) out.set(name, norm(file))
    }
  }
  return out
}

/**
 * `bridge.ts` 里每一个 `call(...)` 表达式（含泛型那几种写法）的原文。
 *
 * 从 `call(` / `call<...>(` 起按括号配平切一刀 —— 命令名是它的第二个实参，
 * `withTick(` 也只会出现在同一个表达式里。
 */
function callExpressions(text) {
  const out = []
  const re = /call\s*(?:<[^>]*>)?\s*\(/g
  let m
  while ((m = re.exec(text)) !== null) {
    const start = m.index
    let depth = 0
    let i = start + m[0].length - 1
    for (; i < text.length; i += 1) {
      const ch = text[i]
      if (ch === '(') depth += 1
      else if (ch === ')') {
        depth -= 1
        if (depth === 0) break
      }
    }
    out.push({ text: text.slice(start, i + 1), at: start })
  }
  return out
}

function lineOf(text, at) {
  return text.slice(0, at).split('\n').length
}

/* ---------------- ① 调用点闸 ---------------- */

function checkCallSites(bridge) {
  const hits = []
  const calls = callExpressions(bridge)
  for (const [name, where] of commandsNeedingChannel()) {
    const sites = calls.filter((c) => c.text.includes(`'${name}'`))
    // 这一层根本不调它（比如只在别处走）—— 不归这道闸管
    if (sites.length === 0) continue
    for (const site of sites) {
      if (site.text.includes('withTick(')) continue
      hits.push({
        where: `${BRIDGE}:${lineOf(bridge, site.at)}`,
        line: site.text.replace(/\s+/g, ' ').slice(0, 120),
        why:
          `命令 \`${name}\`（${where}）的签名里有 \`${CHANNEL_PARAM}\` —— Tauri 的 Channel ` +
          '没有 Deserialize，是**必填参数**。这里没经过 withTick 就等于把 onTick 留空：' +
          'Tauri 会在参数反序列化那一步拒掉，命令体一行都不跑，界面只会看到兜底的' +
          '「出了点问题，请重试」。包一层 withTick(…)（不给回调也要包）',
      })
    }
  }
  return hits
}

/* ---------------- ② 兜底闸 ---------------- */

function checkWithTick(bridge) {
  const start = bridge.indexOf('function withTick(')
  if (start === -1) {
    return [{ where: BRIDGE, line: '', why: '找不到 `function withTick` —— 契约的可选参数没人兑现了' }]
  }
  // 函数体 = 从签名起，到下一个顶格 `}` 为止
  const body = bridge.slice(start)
  const end = body.indexOf('\n}')
  const fn = end === -1 ? body : body.slice(0, end)

  const hits = []
  if (!fn.includes('new Channel')) {
    hits.push({
      where: `${BRIDGE}:${lineOf(bridge, start)}`,
      line: 'function withTick(…)',
      why: '`withTick` 里没建 Channel —— 它必须无条件挂一条（回调可省，参数不可省）',
    })
  }
  // 绊线：这一行就是 2026-10-06 的死路（"不给回调就完全不挂"）
  for (const [i, line] of fn.split('\n').entries()) {
    if (/\breturn\s+args\b/.test(line)) {
      hits.push({
        where: `${BRIDGE}:${lineOf(bridge, start) + i}`,
        line: line.trim(),
        why:
          '「没有回调就原样返回入参」= 把必填的 onTick 留空 —— 命令体跑不到，' +
          '界面只看到「出了点问题，请重试」（2026-10-06 就是这一行）',
      })
    }
  }
  return hits
}

const bridgeText = readSafe(BRIDGE)
if (bridgeText === null) {
  console.error(`✗ 读不到 ${BRIDGE} —— 这道判据没法判断`)
  process.exit(1)
}

const needing = commandsNeedingChannel()
const hits = [...checkCallSites(bridgeText), ...checkWithTick(bridgeText)]

if (hits.length > 0) {
  console.error(`✗ 看到 ${hits.length} 处「必填 Channel 参数没挂上」的迹象：`)
  for (const h of hits) console.error(`    ${h.where}\n      ${h.line}\n      → ${h.why}`)
  console.error('')
  console.error(`  要一条 Channel 的命令（Rust 侧签名里含 \`${CHANNEL_PARAM}\`）：`)
  for (const [name, where] of needing) console.error(`    ${name}  ← ${where}`)
  console.error('  改对了再跑；**不要**把这条判据放宽来"让 CI 过"——那等于把闸门拆了。')
  process.exit(1)
}

console.log('✓ Channel 参数：两道闸都干净 ——')
console.log(`    ① ${needing.size} 条要 Channel 的命令，bridge 里都包了 withTick(`)
console.log('    ② withTick 无条件挂 channel（回调可省、参数不可省）')
