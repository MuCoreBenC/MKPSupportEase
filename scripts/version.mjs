/**
 * 版本号策略：**已发布的号永不再用；开发永远拿着下一个号的 `-dev`**。
 *
 * 为什么这么定（作者 2026-11-04）：
 * - 发过的版本是一份不可变的事实（tag + Release + 安装包都指向它），
 *   再改同一个号就变成"同一个名字两种内容" —— 客户端、issue、回滚全说不清；
 * - 所以开发分支上的版本**立刻**变成下一个补丁号的 `-dev`（如 `0.0.10-dev`），
 *   正式发版时才去掉 `-dev`（`0.0.10`），发完再 `next` 到 `0.0.11-dev`。
 *
 * 这与成熟软件的通行做法一致：SemVer 的 pre-release 段（`-dev` / `-alpha` / `-rc`）
 * 表示"还没发"，发布 = 去掉 pre-release 并打不可变 tag；`release-please` /
 * `cargo-release` / `changesets` 都是"发布后立刻 bump 下一个开发版本"。
 *
 * 三处真值必须同步（版本真值是 `src-tauri/Cargo.toml`，另两处是派生物）：
 *   - `src-tauri/Cargo.toml`      ← 真值
 *   - `package.json`              ← 前端 / CI 读它
 *   - `src-tauri/tauri.conf.json` ← 打包读它
 *   - `Cargo.lock`                ← 锁文件里的同名包版本
 *
 * 用法：
 *   node scripts/version.mjs check       三处一致 + 号没被发过（CI / pre-push 跑）
 *   node scripts/version.mjs bump-dev    推下一个 `-dev`（按**正式版**走，预览版不算）
 *   node scripts/version.mjs release     把当前 `-dev` 去掉（发布前用）
 *   node scripts/version.mjs next        发布后 → 下一个 `-dev`（预览版回到同一基础）
 *   node scripts/version.mjs current     打印当前版本（给别的脚本取）
 *
 * ★ **预览号**（2026-11-05）：`x.y.z-preview.N` 是**发布**的一种，但它**不消耗**正式号 ——
 *   发过 `0.0.10-preview.1` 之后，`0.0.10` 仍可照常发正式版；同一个预览序号不许发两次。
 */
import { execFileSync } from 'node:child_process'
import { readFileSync, writeFileSync } from 'node:fs'

const ROOT = execFileSync('git', ['rev-parse', '--show-toplevel'], { encoding: 'utf8' }).trim()

const FILES = {
  pkg: `${ROOT}/package.json`,
  tauri: `${ROOT}/src-tauri/tauri.conf.json`,
  cargo: `${ROOT}/src-tauri/Cargo.toml`,
  lock: `${ROOT}/Cargo.lock`,
}
const LOCK_PACKAGE = 'mkp-support-ease'

const read = (path) => readFileSync(path, 'utf8')

/** 三处版本号（缺一处就是 `undefined`，check 会报） */
function readVersions() {
  return {
    pkg: /"version"\s*:\s*"([^"]+)"/.exec(read(FILES.pkg))?.[1],
    tauri: /"version"\s*:\s*"([^"]+)"/.exec(read(FILES.tauri))?.[1],
    cargo: /^version\s*=\s*"([^"]+)"/m.exec(read(FILES.cargo))?.[1],
    lock: new RegExp(`name = "${LOCK_PACKAGE}"\\nversion = "([^"]+)"`).exec(read(FILES.lock))?.[1],
  }
}

/**
 * `x.y.z` / `x.y.z-dev` / `x.y.z-preview.N` → `{ text, base, major, minor, patch, kind, seq }`；
 * 不合法返回 null。
 *
 * ★ `kind` 三档（2026-11-05 预览通道）：
 *   - `stable` 正式版：**消耗这个号**，之后同基础不许再发任何东西；
 *   - `preview` 预览版：发出去过，但**不消耗正式号** —— `0.0.10-preview.1` 之后
 *     `0.0.10` 仍可照常发；
 *   - `dev` 开发占位：从没发出去过。
 */
function parse(text) {
  const t = String(text ?? '').trim()
  const m = /^(\d+)\.(\d+)\.(\d+)(?:-(dev)|-preview\.(\d+))?$/.exec(t)
  if (!m) return null
  // `-preview.01` 与 `-preview.1` 是同一个号，却有两种写法 —— 直接判非法
  if (m[5] !== undefined && (m[5].startsWith('0') || Number(m[5]) < 1)) return null
  const kind = m[4] ? 'dev' : m[5] !== undefined ? 'preview' : 'stable'
  return {
    text,
    base: `${m[1]}.${m[2]}.${m[3]}`,
    major: Number(m[1]),
    minor: Number(m[2]),
    patch: Number(m[3]),
    kind,
    seq: m[5] !== undefined ? Number(m[5]) : 0,
  }
}

const cmp = (a, b) => a.major - b.major || a.minor - b.minor || a.patch - b.patch

/**
 * 已发布的 tag（`v*`，只认合法形状），按版本升序。
 *
 * ★ 预览 tag（`v0.0.10-preview.1`）也在里面（2026-11-05）：不认它的话，
 *   "这个预览号发过没有"就永远答"没有"，同一个号能被发第二次。
 */
function publishedTags() {
  return execFileSync('git', ['tag', '--list', 'v*'], { cwd: ROOT, encoding: 'utf8' })
    .trim()
    .split('\n')
    .filter(Boolean)
    .map((tag) => ({ tag, v: parse(tag.slice(1)) }))
    .filter((x) => x.v !== null)
    .sort((a, b) => cmp(a.v, b.v) || a.v.seq - b.v.seq)
}

/** 这个基础版本有没有发过**正式版**（预览版不算 —— 它不消耗正式号） */
const stableReleased = (base) =>
  publishedTags().some((x) => x.v.kind === 'stable' && x.v.base === base)

/** `0.0.10` → `0.0.11`（补丁号 +1，只改最后一段） */
function bumpPatch(base) {
  const [major, minor, patch] = base.split('.').map(Number)
  return `${major}.${minor}.${patch + 1}`
}

const fail = (message, hint) => {
  console.error(`\n\u001b[31m版本号闸红：${message}\u001b[0m`)
  if (hint) console.error(`  ${hint}`)
  console.error('')
  process.exit(1)
}
const ok = (message) => console.log(`\u001b[32m✓\u001b[0m ${message}`)
const warn = (message) => console.log(`\u001b[33m!\u001b[0m ${message}`)

/** 把版本写进四处；只换值，不动格式 */
function writeAll(version) {
  const swap = (path, re, next) => {
    const before = read(path)
    const after = before.replace(re, next)
    if (after === before) throw new Error(`${path} 里没找到版本号，拒绝盲写`)
    writeFileSync(path, after)
  }
  swap(FILES.pkg, /"version"\s*:\s*"[^"]*"/, `"version": "${version}"`)
  swap(FILES.tauri, /"version"\s*:\s*"[^"]*"/, `"version": "${version}"`)
  swap(FILES.cargo, /^version\s*=\s*"[^"]*"/m, `version = "${version}"`)
  swap(
    FILES.lock,
    new RegExp(`(name = "${LOCK_PACKAGE}"\\nversion = ")[^"]*(")`),
    `$1${version}$2`,
  )
  ok(`三处 + Cargo.lock 已写成 ${version}`)
}

function assertConsistent() {
  const v = readVersions()
  const missing = Object.entries(v)
    .filter(([, value]) => !value)
    .map(([key]) => key)
  if (missing.length) fail(`读不到版本号：${missing.join(' / ')}`)
  const unique = [...new Set(Object.values(v))]
  if (unique.length !== 1) {
    fail(
      `四处版本不一致：${Object.entries(v)
        .map(([key, value]) => `${key}=${value}`)
        .join(' · ')}`,
      '版本真值是 src-tauri/Cargo.toml；跑 node scripts/version.mjs bump-dev 重新对齐',
    )
  }
  const parsed = parse(v.pkg)
  if (!parsed) {
    fail(`版本号格式不对：${v.pkg}（要 x.y.z / x.y.z-dev / x.y.z-preview.N）`)
  }
  return parsed
}

/* ---------- check ---------- */

function check() {
  const version = assertConsistent()
  const published = publishedTags()
  const byBase = published.filter((x) => x.v.base === version.base)

  /* 开发占位：只有**正式版**发过才算"这个号用掉了"。
   * 预览版发过不影响 `0.0.10-dev` 继续开发 —— 这正是双通道要的样子
   * （先前这里按 base 一刀切，发过预览就再也回不到开发版）。 */
  if (version.kind === 'dev') {
    if (byBase.some((x) => x.v.kind === 'stable')) {
      fail(
        `v${version.base} 已经发过正式版，开发版本不能再用这个号`,
        '跑 node scripts/version.mjs next 取下一个补丁号（例如 0.0.11-dev）',
      )
    }
    ok(`开发版本 ${version.text}（尚未发布）`)
    return
  }

  /* 预览版：同一个**完整号**不许发两次；同基础的下一个序号随便发。 */
  if (version.kind === 'preview') {
    const same = byBase.find((x) => x.v.kind === 'preview' && x.v.seq === version.seq)
    if (same) {
      warn(
        `当前是已发布过的预览版 v${version.text} —— 它在 main 上合法（发布提交），` +
          '但开发新东西前必须先换成 -dev，或发下一个预览序号',
      )
      return
    }
    ok(`预览版 ${version.text}（tag 还没打 —— 这是发布流程中的状态）`)
    return
  }

  if (byBase.some((x) => x.v.kind === 'stable')) {
    warn(
      `当前是已发布版本 v${version.text} —— 它在 main 上合法（发布提交），` +
        '但开发新东西前必须换成下一个 -dev',
    )
    return
  }

  ok(`版本 ${version.text}（tag 还没打 —— 这是发布流程中的状态）`)
}

/* ---------- 写操作 ---------- */

function bumpDev() {
  const version = assertConsistent()

  if (version.kind === 'dev' && !stableReleased(version.base)) {
    ok(`已经是开发版本 ${version.text}，不用动`)
    return
  }

  /* 分两种（2026-11-05 预览通道）：
   *   · 当前号的**正式版已经发出去** → 换下一个补丁号的 -dev（`0.0.10` → `0.0.11-dev`）；
   *   · 正式号还没被消耗（当前是预览版 / 开发版）→ **停在同一个基础**
   *     （`0.0.10-preview.1` → `0.0.10-dev`），不然 `0.0.10` 正式版会被永远跳过去。 */
  const nextBase = stableReleased(version.base) ? bumpPatch(version.base) : version.base
  writeAll(`${nextBase}-dev`)
}

function release() {
  const version = assertConsistent()
  if (version.kind !== 'dev') {
    fail(`当前 ${version.text} 不是 -dev`, '发布版本由 scripts/release.mjs 显式给出')
  }
  if (stableReleased(version.base)) {
    fail(`v${version.base} 已经发过正式版，不能再用同一个号`)
  }
  writeAll(version.base)
}

function next() {
  const version = assertConsistent()
  if (version.kind === 'dev') {
    fail(`当前已经是开发版本 ${version.text}`, '先发布，再 next')
  }
  /* ★ 预览版发完之后回到**同一个基础**的 -dev（2026-11-05）：
   *   正式号还没被消耗，开发继续朝 `0.0.10` 走；
   *   直接跳 `0.0.11-dev` 会把还没发的正式版 0.0.10 永远跳过去。 */
  if (version.kind === 'preview') {
    writeAll(`${version.base}-dev`)
    return
  }
  writeAll(`${version.major}.${version.minor}.${version.patch + 1}-dev`)
}

/* ---------- 入口 ---------- */

const command = process.argv[2] ?? 'check'
switch (command) {
  case 'check':
    check()
    break
  case 'bump-dev':
    bumpDev()
    break
  case 'release':
    release()
    break
  case 'next':
    next()
    break
  case 'current':
    console.log(readVersions().pkg ?? '')
    break
  default:
    console.error(`不认识的命令：${command}`)
    console.error('用法：check | bump-dev | release | next | current')
    process.exit(2)
}
