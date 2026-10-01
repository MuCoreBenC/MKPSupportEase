/**
 * 判据 2 —— 启动零网络（`docs/DATA-ARCHITECTURE.md` §5 第 2 条）。
 *
 * 总纲十问 #7 只批了三件可以联网的事：**下载**（拿文件）、**检查更新**（拿清单）、
 * 将来的**发布**。首屏不在其中：一旦程序启动时摸网络，装好的软件在无网环境
 * （比赛现场、离线车间）里就只会对着一个转圈 —— 而它本来根本不需要联网。
 *
 * 文档写完半年后一定会有人顺手在方便的地方加一次请求，所以这条判据把那句提纲变成
 * 能在 CI 上红的检查。它现在由三道闸组成，每一道都回答一个具体问题：
 *
 *   ① 范围闸   谈网络字节的代码许不许出现在别处 —— **只许在 runtime/net.rs**
 *   ② 启动闸   `.setup()` 里有没有联网动作
 *   ③ 前端闸   前端有没有绕过 IPC 自己去发 HTTP
 *
 * 三道都是"黑名单 + 白名单"，和人相关的部分（为什么）写在旁边。
 * **它不是运行时判据** —— 静态扫描抓不到"A 调 B、B 最终联网"那种间接调用，
 * 那一条留给将来的运行时判据（拦在代理层或沙箱里）。这里的承诺是：
 * 上面三个具体的常见问题，一定会红，并且说清楚为什么红。
 */

import { readdirSync, readFileSync, statSync } from 'node:fs'
import { join, sep } from 'node:path'

/** Rust 源码目录 */
const RUST_SRC = 'src-tauri/src'
/** 前端源码目录 */
const WEB_SRC = 'src'

/** ① 唯一允许谈网络字节的那个文件。**只有它**，多一个都不行 */
const NETWORK_FILE = 'src-tauri/src/runtime/net.rs'

/** Rust 侧的网络入口符号出现 anywhere 不在白名单文件里 → 红。右边是为什么 */
const RUST_SYMBOLS = [
  ['ureq', 'HTTP 客户端（联网这条路只许 `runtime/net.rs` 一个人走）'],
  ['reqwest', '另一个 HTTP 客户端 —— 本仓不引异步 HTTP 栈'],
  ['std::net::', '原始套接字'],
  ['TcpStream', 'TCP 连接'],
  ['TcpListener', 'TCP 监听（客户端产品不许开端口）'],
  ['UdpSocket', 'UDP 套接字'],
  ['tokio::net', '异步网络'],
  ['hyper::', 'HTTP 实现'],
]

/** ② 启动段里不许出现的联网入口（这些都需要̃网络，且都不是启动时该做的事） */
const STARTUP_FORBIDDEN = [
  'runtime::net',
  'download_runtime_file',
  'download_runtime_files',
  'check_remote_update',
  'apply_remote_update',
  'get_manifest',
]

/** ③ 前端不许自己发 HTTP —— 与后端的对话只有 IPC 一条（前端不算业务，也不谈网络） */
const WEB_SYMBOLS = [
  [/^\s*(?:const|let|var|await)?\s*.*\bfetch\s*\(/, 'fetch（前端要数据请走 api.* → Tauri IPC）'],
  [/\bXMLHttpRequest\b/, 'XMLHttpRequest（同上）'],
  [/\bnew\s+WebSocket\b/, 'WebSocket（同上）'],
  [/\bnew\s+EventSource\b/, 'EventSource（同上）'],
  [/\bnavigator\.sendBeacon\b/, 'sendBeacon（同上）'],
]

/** ③ 前端闸的管辖范围：数据的出入口都在这层，让它干净等于让整个前端干净 */
const WEB_API_DIR = 'src/api'

function walk(dir, only = () => true) {
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

/** 把路径统一成 `/` 分隔 —— 判据要在三个 CI（含 Windows）上给出同一份结果 */
function norm(file) {
  return file.split(sep).join('/')
}

/* ---------------- ① 范围闸 ---------------- */

function checkRustScope() {
  const files = walk(RUST_SRC, (f) => f.endsWith('.rs'))
  const hits = []
  for (const file of files) {
    const rel = norm(file)
    if (rel === norm(NETWORK_FILE)) continue
    const text = readSafe(file)
    if (text === null) continue
    text.split('\n').forEach((line, i) => {
      for (const [symbol, why] of RUST_SYMBOLS) {
        if (line.includes(symbol)) hits.push({ where: `${rel}:${i + 1}`, why, line: line.trim() })
      }
    })
  }
  return hits
}

/* ---------------- ② 启动闸 ---------------- */

function checkStartup() {
  const entry = norm(join(RUST_SRC, 'lib.rs'))
  const text = readSafe(entry)
  if (text === null) return [{ where: entry, why: '读不到入口文件', line: '' }]

  // 启动段 = `.setup(|app| {` 到 `.run(tauri::generate_context!())` 之间。
  // 那一段才是"程序起来时干了什么"。被调用的下层（`fsx` / `runtime`）由 ① 那道闸管
  const begin = text.indexOf('.setup(')
  const end = text.indexOf('.run(')
  if (begin === -1 || end === -1 || end < begin) {
    return [{ where: `${entry}:0`, why: '找不到 `.setup()` 段，没法判断启动路径', line: '' }]
  }
  const setupLines = text.slice(begin, end).split('\n')
  const startLine = text.slice(0, begin).split('\n').length

  const hits = []
  setupLines.forEach((line, i) => {
    // 注释里提到"以后不许联网"是允许的 —— 那是给人看的文字，不是动作
    if (line.trimStart().startsWith('//')) return
    for (const symbol of STARTUP_FORBIDDEN) {
      if (line.includes(symbol)) {
        hits.push({
          where: `${entry}:${startLine + i}`,
          why: `启动段里出现了联网动作 \`${symbol}\`（首屏必须零网络）`,
          line: line.trim(),
        })
      }
    }
  })
  return hits
}

/* ---------------- ③ 前端闸 ---------------- */

function checkWebApi() {
  const hits = []
  for (const file of walk(WEB_API_DIR, (f) => f.endsWith('.ts') || f.endsWith('.tsx'))) {
    const rel = norm(file)
    const text = readSafe(file)
    if (text === null) continue
    text.split('\n').forEach((line, i) => {
      if (line.trimStart().startsWith('*') || line.trimStart().startsWith('//')) return
      for (const [re, why] of WEB_SYMBOLS) {
        if (re.test(line)) {
          hits.push({
            where: `${rel}:${i + 1}`,
            why: `${why}——HTTP 只发生在 Rust 侧，前端直接联网会绕过通道与校验收敛点`,
            line: line.trim(),
          })
        }
      }
    })
  }
  return hits
}

const hits = [...checkRustScope(), ...checkStartup(), ...checkWebApi()]

if (hits.length > 0) {
  console.error(`✗ 看到 ${hits.length} 处「启动不该有网络」的迹象（判据 2 被破坏）：`)
  for (const h of hits) console.error(`    ${h.where}\n      ${h.line}\n      → ${h.why}`)
  console.error('')
  console.error('  三道闸分别是：')
  console.error(`    ① 网络字节只许住在 ${NETWORK_FILE}`)
  console.error('    ② 程序的 .setup() 段不许联网（首屏零网络）')
  console.error(`    ③ ${WEB_API_DIR} 不许自己发 HTTP（前端与后端之间只有 IPC）`)
  console.error('  改对了再跑一遍；**不要**把这些符号加进白名单来"让 CI 过"——那等于把闸门拆了。')
  process.exit(1)
}

console.log('✓ 启动零网络：三道闸都干净 ——')
console.log(`    ① 网络符号只出现在 ${NETWORK_FILE}`)
console.log('    ② 程序的启动段没有联网动作')
console.log(`    ③ ${WEB_API_DIR} 没有任何自己的 HTTP 调用（数据一律走 IPC）`)
