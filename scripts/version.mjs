/**
 * 版本号策略：**已发布的号永不再用；开发永远拿着下一个号的 `-dev`**。
 *
 * 为什么这么定：
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
 * ★ **预览号**：`x.y.z-preview.N` 是发布的一种，但它**不消耗**正式号 ——
 *   发过 `0.0.10-preview.1` 之后，`0.0.10` 仍可照常发正式版；同一个预览序号不许发两次。
 */
import { execFileSync } from 'node:child_process'
import { readFileSync, writeFileSync } from 'node:fs'

const ROOT = execFileSync('git', ['rev-parse', '--show-toplevel'], { encoding: 'utf8' }).trim()

/** 唯一版本源 —— 只有它能被改，其余四处是它的产物。 */
const SOURCE = `${ROOT}/version.toml`

const FILES = {
  pkg: `${ROOT}/package.json`,
  tauri: `${ROOT}/src-tauri/tauri.conf.json`,
  cargo: `${ROOT}/src-tauri/Cargo.toml`,
  lock: `${ROOT}/Cargo.lock`,
}
const LOCK_PACKAGE = 'mkp-support-ease'

const read = (path) => readFileSync(path, 'utf8')

/** 版本号五处读数（缺一处就是 `undefined`，check 会报） */
function readVersions() {
  return {
    source: /^version\s*=\s*"([^"]+)"/m.exec(read(SOURCE))?.[1],
    pkg: /"version"\s*:\s*"([^"]+)"/.exec(read(FILES.pkg))?.[1],
    tauri: /"version"\s*:\s*"([^"]+)"/.exec(read(FILES.tauri))?.[1],
    cargo: /^version\s*=\s*"([^"]+)"/m.exec(read(FILES.cargo))?.[1],
    lock: new RegExp(`name = "${LOCK_PACKAGE}"\\nversion = "([^"]+)"`).exec(read(FILES.lock))?.[1],
  }
}

/**
 * `x.y.z` / `x.y.z-preview.N` → `{ text, base, major, minor, patch, kind, seq }`；
 * 不合法返回 null。
 *
 * ★ **没有 `-dev`**。
 *   只有两档身份：
 *   - `stable` 正式版（`0.0.11`）—— 发出去就冻结，这个号永远不再用；
 *   - `preview` 预览版（`0.0.12-preview.3`）—— 本地/云端交付过就冻结，序号往后再加。
 *   未交付的那一版**也是** preview 号（只是还没有 tag）；"有没有冻结"由 git tag 回答，
 *   不由版本号的形状回答。
 *
 * ★ `preview` 的 N 是**第几次交付**，不是版本升级；主版本一整轮恒定，发布时整个去掉
 *   （`0.0.12-preview.3` → `0.0.12`）。
 *
 * ★ 大小写不敏感：作者习惯写 `Preview.1`，历史 tag 是小写 `preview.1` —— 两种都认，
 *   但**写出来的一律小写**（`text` 保留原样，比较只认 `base` + `seq`）。
 */
function parse(text) {
  const t = String(text ?? '').trim()
  const m = /^(\d+)\.(\d+)\.(\d+)(?:-[Pp]review\.(\d+))?$/.exec(t)
  if (!m) return null
  const seqText = m[4]
  // `-preview.01` 与 `-preview.1` 是同一个号，却有两种写法 —— 直接判非法
  if (seqText !== undefined && (seqText.startsWith('0') || Number(seqText) < 1)) return null
  return {
    text: t,
    base: `${m[1]}.${m[2]}.${m[3]}`,
    major: Number(m[1]),
    minor: Number(m[2]),
    patch: Number(m[3]),
    kind: seqText === undefined ? 'stable' : 'preview',
    seq: Number(seqText ?? 0),
  }
}

const cmp = (a, b) => a.major - b.major || a.minor - b.minor || a.patch - b.patch

/**
 * 已发布的 tag（`v*`，只认合法形状），按版本升序。
 *
 * ★ 预览 tag（`v0.0.10-preview.1`）也在里面：不认它的话，
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
 * 把"版本身份"那几格摊开。
 *
 * 只打印，不改任何东西 —— `check` 与两个写操作最后都调它，于是
 * "最新已发布是谁 / 工作区现在是谁"永远同时出现在同一屏上，不会再各说各的。
 */
function reportIdentities(current) {
  const now = current ?? readVersions().pkg ?? ''
  const published = latestPublished()
  const base = parse(now)?.base
  /* **三格身份**：已发布 / 已交付 / 正在开发。
     这三个号不一样是常态 —— 界面上不许拿它当错误。 */
  const delivered = base === undefined ? 0 : Math.max(publishedPreviewSeq(base), localPreviewSeq(base))
  console.log(`  最新已发布：${published ?? '（还没发过）'}`)
  console.log(
    `  本地已交付：${delivered === 0 || base === undefined ? '（还没交付过）' : `${base}-preview.${delivered}`}`,
  )
  console.log(`  当前开发：  ${now}${now === published ? '  ← 与已发布同号（该进下一版了）' : ''}`)
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

/** 把版本写进**源 + 四处**；只换值，不动格式 */
function writeAll(version) {
  const swap = (path, re, next) => {
    const before = read(path)
    if (!re.test(before)) throw new Error(`${path} 里没找到版本号，拒绝盲写`)
    writeFileSync(path, before.replace(re, next))
  }
  /* 源**不动**（它就是权威）：只把同一份值铺到四处。 */
  swap(FILES.pkg, /"version"\s*:\s*"[^"]*"/, `"version": "${version}"`)
  swap(FILES.tauri, /"version"\s*:\s*"[^"]*"/, `"version": "${version}"`)
  swap(FILES.cargo, /^version\s*=\s*"[^"]*"/m, `version = "${version}"`)
  swap(
    FILES.lock,
    new RegExp(`(name = "${LOCK_PACKAGE}"\\nversion = ")[^"]*(")`),
    `$1${version}$2`,
  )
  ok(`version.toml + 四处已写成 ${version}`)
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
      `版本不一致：${Object.entries(v)
        .map(([key, value]) => `${key}=${value}`)
        .join(' · ')}`,
      '唯一源是 version.toml；跑 node scripts/version.mjs sync 把它同步到四处',
    )
  }
  const parsed = parse(v.source)
  if (!parsed) {
    fail(`版本号格式不对：${v.pkg}（要 x.y.z 或 x.y.z-preview.N）`)
  }
  return parsed
}

/* ---------- check ---------- */

function check() {
  /* `--json`：给工作台读，一行 JSON，不打印人话（否则界面得解析配色符号）。 */
  if (process.argv.includes('--json')) {
    const v = readVersions()
    const values = Object.values(v)
    const parsed = parse(v.source ?? '')
    console.log(
      JSON.stringify({
        source: v.source ?? null,
        pkg: v.pkg ?? null,
        cargo: v.cargo ?? null,
        tauri: v.tauri ?? null,
        lock: v.lock ?? null,
        consistent: new Set(values).size === 1 && values.every(Boolean),
        valid: parsed !== null,
        reason: parsed === null ? '版本号形状不对（要 x.y.z 或 x.y.z-preview.N）' : '',
      }),
    )
    return
  }
  const version = assertConsistent()
  /* 身份三格一起摆出来："最新已发布"、"本地已交付"与"当前开发"同时出现，
     免得只看一句"开发版本 X"却不知道发到哪儿了。 */
  reportIdentities(version.text)
  const published = publishedTags()
  const byBase = published.filter((x) => x.v.base === version.base)

  /* 这一号**已经冻结**（git tag 在）⇒ 工作区不该还停留在它上面：
     要么已经滚到下一轮（`ensure-preview` 会做），要么是发布提交本身还没滚。 */
  const frozen = byBase.find((x) => x.v.kind === version.kind && x.v.seq === version.seq)
  if (frozen) {
    if (version.kind === 'stable') {
      warn(
        `当前是已发布版本 v${version.text} —— 它在 main 上合法（发布提交），` +
          '但继续开发前必须进下一个补丁号的 preview.1（npm run version:ensure）',
      )
    } else {
      warn(
        `当前 ${version.text} 已经冻结（tag 在）—— 工作区不该停在这一版；` +
          '跑 npm run version:ensure 进下一轮',
      )
    }
    return
  }

  ok(`${version.kind === 'preview' ? '预览' : '正式'}版本 ${version.text}（还没冻结 —— 可以继续改、继续交付）`)
}

/* ---------- 写操作 ---------- */

/**
 * 云端已经发过的预览序号（`v0.0.10-preview.2` → 2）。本地交付要排在它后面。
 */
function publishedPreviewSeq(base) {
  return publishedTags()
    .filter((x) => x.v.kind === 'preview' && x.v.base === base)
    .reduce((max, x) => Math.max(max, x.v.seq), 0)
}

/**
 * **收敛到"下一个待交付的预览号"**。
 *
 * ```text
 * 正式 0.0.11 已发  →  0.0.12-preview.1 → 交付 → 0.0.12-preview.2 → … → 0.0.12
 * ```
 *
 * 三条判据，按顺序：
 * 1. 当前号**已经排在已交付之后** ⇒ 一个字不动（改十次代码还是它）；
 * 2. 当前号**已经被交付过** ⇒ 抬到「已交付 + 1」；
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


/** 把 `version.toml` 的值同步到四处（人改过派生文件时用；不改源本身）。 */
function sync() {
  const v = readVersions()
  if (!v.source) fail('读不到 version.toml 里的 version')
  if (!parse(v.source)) fail(`版本号格式不对：${v.source}（要 x.y.z 或 x.y.z-preview.N）`)
  writeAll(v.source)
  reportIdentities(v.source)
}

function release() {
  const version = assertConsistent()
  if (version.kind !== 'preview') {
    fail(`当前 ${version.text} 不是预览号`, '正式发布由 scripts/release.mjs 显式给出号')
  }
  if (stableReleased(version.base)) {
    fail(`v${version.base} 已经发过正式版，不能再用同一个号`)
  }
  writeAll(version.base)
}

function next() {
  const version = assertConsistent()
  const published = latestPublished()
  if (published === null) {
    fail('一个 tag 都没有，不知道从哪儿往下走')
  }
  /* 发布成功之后：**下一个补丁号**的 preview.1。
     例如 0.0.11 发完 → 0.0.12-preview.1；0.0.12-preview.3 发完 → 0.0.12-preview.4 由 ensure 处理，
     这里只负责"正式版之后跳到下一个基础"。 */
  const base = version.kind === 'stable' ? bumpPatch(version.base) : version.base
  writeAll(`${base}-preview.1`)
  reportIdentities(`${base}-preview.1`)
}

/* ---------- 入口 ---------- */

const command = process.argv[2] ?? 'check'
switch (command) {
  case 'check':
    check()
    break
  case 'sync':
    sync()
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
    console.error('用法：check [--json] | sync | ensure-preview | release | next | current')
    process.exit(2)
}
