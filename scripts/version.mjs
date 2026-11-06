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
 * `x.y.z` / `x.y.z-dev` / `x.y.z-dev.N` / `x.y.z-preview.N`
 * → `{ text, base, major, minor, patch, kind, seq }`；不合法返回 null。
 *
 * ★ `kind` 三档（2026-11-05 预览通道）：
 *   - `stable` 正式版：**消耗这个号**，之后同基础不许再发任何东西；
 *   - `preview` 预览版：发出去过，但**不消耗正式号** —— `0.0.10-preview.1` 之后
 *     `0.0.10` 仍可照常发；
 *   - `dev` 开发占位：从没发出去过。
 *
 * ★ **`-dev.N` 的那个 N 是构建号，不是版本号**（2026-11-06）：
 *   主版本（`0.0.11`）在一整轮开发里**恒定**，N 只在"生成一份可安装的预览产物"时 +1。
 *   没有它的话，本地反复测"更新"是做不到的 —— 客户端只摆**比当前构建新**的候选，
 *   而 `APP_VERSION` 是编译进二进制的：同号两个包，它分不出谁是新的一份。
 *   发布时 N 整个去掉（`0.0.11-dev.7` → `0.0.11`），所以它不会泄漏到发布号里。
 */
function parse(text) {
  const t = String(text ?? '').trim()
  const m = /^(\d+)\.(\d+)\.(\d+)(?:-(dev)(?:\.(\d+))?|-preview\.(\d+))?$/.exec(t)
  if (!m) return null
  const devSeq = m[5]
  const preSeq = m[6]
  // `-preview.01` 与 `-preview.1` 是同一个号，却有两种写法 —— 直接判非法
  for (const n of [devSeq, preSeq]) {
    if (n !== undefined && (n.startsWith('0') || Number(n) < 1)) return null
  }
  const kind = m[4] ? 'dev' : preSeq !== undefined ? 'preview' : 'stable'
  return {
    text,
    base: `${m[1]}.${m[2]}.${m[3]}`,
    major: Number(m[1]),
    minor: Number(m[2]),
    patch: Number(m[3]),
    kind,
    seq: Number(devSeq ?? preSeq ?? 0),
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

/**
 * **最近一次发布**的号（不管通道）。一个都没发过 = `null`。
 *
 * 为什么按 `creatordate` 而不是按版本大小：`0.0.10`（正式）与 `0.0.10-preview.2`
 * 在数字上同基础，谁"最新"取决于**谁后打上去**。作者问的是"现在最新已发布是哪个"，
 * 那是时间问题，不是大小问题。
 */
function latestPublished() {
  const out = execFileSync('git', ['tag', '--sort=-creatordate', '--list', 'v*'], {
    cwd: ROOT,
    encoding: 'utf8',
  })
  for (const tag of out.trim().split('\n').filter(Boolean)) {
    const v = parse(tag.slice(1))
    if (v !== null) return v.text
  }
  return null
}

/**
 * 把"版本身份"那两格摊开（2026-11-06 作者：「每个产物都要报自己的真实身份」）。
 *
 * 只打印，不改任何东西 —— `check` 与两个写操作最后都调它，于是
 * "最新已发布是谁 / 工作区现在是谁"永远同时出现在同一屏上，不会再各说各的。
 */
function reportIdentities(current) {
  const now = current ?? readVersions().pkg ?? ''
  const published = latestPublished()
  console.log(`  最新已发布：${published ?? '（还没发过）'}`)
  if (now === published) {
    console.log(`  当前工作区：${now}  ← 与已发布同号（该进下一版了）`)
  } else {
    console.log(`  当前工作区：${now}`)
  }
}

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
  /* ★ 两格身份先摆出来（2026-11-06）：作者要的是"最新已发布"与"当前工作区"
     同时出现 —— 以前只有一句"开发版本 X（尚未发布）"，没人知道最新发到哪儿了。 */
  reportIdentities(version.text)
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

/**
 * 把工作区收敛到**一个未发布的开发号**（幂等）。
 *
 * 这是"版本只在发布时递增"那条规则的**执行处**：不管上一轮是发布成功、
 * 发布失败、还是有人手工写了一个号，调它一次就回到"当前该用的开发号"。
 *
 * - 已经是未发布的 `-dev` ⇒ **一个字都不动**（改代码 / 生成 / 打包都调它，
 *   版本号因此恒定 —— 作者要的正是这个）；
 * - 当前号已被**正式发布** ⇒ 进下一个补丁号的 `-dev`（`0.0.10` → `0.0.11-dev`）；
 * - 当前是预览号 / 正式号但**正式版还没发** ⇒ 停在同一个基础的 `-dev`
 *   （`0.0.10-preview.2` → `0.0.10-dev`），否则 `0.0.10` 正式版会被永远跳过去。
 *
 * `bump-dev` 与 `ensure-dev` 是同一件事的两种叫法（一个是"我主动推一把"，
 * 一个是"每次开跑前对齐一下"），共用这一份实现，不各写一套。
 */
/**
 * 云端已经发过的预览序号（`v0.0.10-preview.2` → 2）。本地交付要排在它后面。
 */
function publishedPreviewSeq(base) {
  return publishedTags()
    .filter((x) => x.v.kind === 'preview' && x.v.base === base)
    .reduce((max, x) => Math.max(max, x.v.seq), 0)
}

/**
 * **收敛到"下一个待交付的预览号"**（2026-11-06 作者定的最终规则）。
 *
 * ```text
 * 正式 0.0.11 已发  →  0.0.12-preview.1 → 交付 → 0.0.12-preview.2 → … → 0.0.12
 * ```
 *
 * 三条判据，按顺序：
 * 1. 当前号**已经排在已交付之后** ⇒ 一个字不动（改十次代码还是它）；
 * 2. 当前号**已经被交付过**（或还是个历史遗留的 `-dev`）⇒ 抬到「已交付 + 1」；
 * 3. 当前号是**已发布的正式版** ⇒ 进下一个补丁号的 `preview.1`。
 *
 * 「已交付」取 **云端 tag** 与 **本地清单**（`target/local-release/release-preview.json`）
 * 两者的较大值 —— 本地交付不打 tag，只看 git 会漏。
 *
 * ★ **号在"交付成功"之后才算用掉**：`preview:build` 先收敛号、再编译、最后写清单；
 *   编译挂了就没清单，下次收敛仍给同一个号 —— 符合"交付失败不递增"。
 */
function convergePreview() {
  const version = assertConsistent()
  const delivered = Math.max(publishedPreviewSeq(version.base), localPreviewSeq(version.base))
  const base =
    version.kind === 'stable' && stableReleased(version.base) ? bumpPatch(version.base) : version.base
  const want = `${base}-preview.${delivered + 1}`

  if (version.text === want) {
    ok(`已经是待交付的预览号 ${want}，不用动`)
    reportIdentities(want)
    return
  }
  writeAll(want)
  ok(`收敛到 ${want}（已交付到 ${delivered || '还没交付过'}）`)
  reportIdentities(want)
}

/**
 * **本地预览**用过的最高序号（读 `target/local-release/release-preview.json`）。
 *
 * 这些号没有 tag（本地预览不发布），git 看不见 —— 不把它们算进来，
 * 第二次生成会重新用回 `preview.3`，而客户端装的就是那个号，于是"没有更新"。
 * 读不到 / 号不同基础一律当 0。
 */
function localPreviewSeq(base) {
  const file = `${ROOT}/target/local-release/release-preview.json`
  try {
    const text = readFileSync(file, 'utf8')
    const v = parse(JSON.parse(text).version)
    return v !== null && v.kind === 'preview' && v.base === base ? v.seq : 0
  } catch {
    return 0
  }
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
    convergeDev('bump-dev')
    break
  case 'ensure-preview':
    convergePreview()
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
    console.error('用法：check | ensure-preview | release | next | current')
    process.exit(2)
}
