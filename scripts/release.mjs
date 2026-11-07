/**
 * 发版：**同一事务核心的第二个入口**（第四刀）。
 *
 * 与试验场那份的根本区别：**这里不碰 main**。那边是"本地合并进 main + 推 main"，
 * 在 PR-only 下直接违规（闸②与服务端 ruleset 都会拒）。所以流程改成：
 * 在 feat/ 分支上校验 → 改版本号 → 开 PR → 等 CI 绿 → squash 合并 → 回到 main 打 tag。
 *
 * 三条纪律（原样有效）：
 * 1. **任何写操作之前先跑完全部校验** —— 失败时工作区还是干净的，不用回退；
 * 2. **破坏性命令只打印、不执行** —— 出错时给你精确的回退命令，由你决定要不要跑；
 * 3. **squash 会重写提交**，所以 tag 必须在 `git switch main && git pull` 之后、
 *    在 main 的 tip 上打。在分支上打的 tag 指向一个不在 main 历史里的提交。
 *
 * ## 这一刀改了什么（★ 重要）
 *
 * 以前这个文件**自己**改四处版本号、自己打 tag。现在：
 *
 * ```text
 * scripts/release.mjs（本文件）      校验 + 提问 + 等 CI + 回退提示   ← 人的那一半
 *         │
 *         └─→ src-tauri/src/bin/release.rs ──→ workbench::app::release_tx::run
 *                                              （工作台那颗按钮也是这个内核）
 * ```
 *
 * 版本号、提交、推送、PR、合并、tag、构建、Release、上传、release.json —— **全部在内核里**，
 * 本文件只负责"内核做不了、也不该做的那几件"（交互提问、npm/cargo 校验链、等 CI、
 * 失败时把回退命令打印给人看）。**不废弃、不复制**：两个入口，一条路。
 *
 * ## 2026-11-05：真发版必须先问人（作者定的硬约束）
 *
 * 这一趟会打 tag、上传安装包、写 `release.json`，用户随后就会收到"有新版"——
 * 是**最难撤回**的动作。所以：
 *   · 交互跑：末尾要人亲口敲一句 `yes`；
 *   · 非交互跑（版本与说明都由命令行给了）：必须显式带 `--confirm`；
 *   · 两条路最后都把 `--confirm` 交给内核，内核缺了会零副作用地拒绝。
 * AI / CI 没拿到人**当次**的明确许可，不许自己跑完这一趟。
 *
 * ## 2026-11-05：预览通道（`--channel preview`）
 *
 * 正式版与预览版是**两条通道、两份清单**：
 *
 * ```text
 * 正式通道（缺省）  版本 x.y.z          写 presets/delivery/release.json          Release 不标 prerelease
 * 预览通道          版本 x.y.z-preview.N 写 presets/delivery/release-preview.json  Release 标 prerelease
 * ```
 *
 * 发预览时**正式版那一份一个字节都不动** —— 正式版用户不会收到任何通知，
 * 只有主动点「检查更新」的人才看得到。预览号也**不消耗**正式号：发过
 * `0.0.10-preview.1` 之后，`0.0.10` 仍可照常发。
 */
import { execFileSync } from 'node:child_process'
import { existsSync, readFileSync } from 'node:fs'
import { createInterface } from 'node:readline/promises'
import { stdin as input, stdout as output } from 'node:process'

/*
 * 仓库根。**必须是 let、且先有一个可用的初值**：下面 run() 的默认 cwd 就读它，
 * 而第一次调用 run() 正是为了问出它自己 —— 写成 `const ROOT = run(...)` 会直接
 * TDZ 崩（`Cannot access 'ROOT' before initialization`）。这个 bug 上线过一次：
 * `node --check` 查语法查不出来，它是运行期错误，只有真跑一次才会暴露。
 */
let ROOT = process.cwd()

/* ---------- 小工具 ---------- */

function run(cmd, args, opts = {}) {
  return execFileSync(cmd, args, { encoding: 'utf8', cwd: opts.cwd ?? ROOT }).trim()
}

/**
 * 跑给人看的命令（输出直连终端）。
 *
 * stdin 给 'ignore' 而不是继承：校验链里的 npm / cargo 继承了 stdin 之后会把里面的内容
 * 读掉（或直接读到 EOF），于是后面 readline 的提问永远等不到答案 ——
 * 实测表现是 node 报 `Detected unsettled top-level await` 然后静默退出。
 */
function runLive(cmd, args, opts = {}) {
  execFileSync(cmd, args, { stdio: ['ignore', 'inherit', 'inherit'], cwd: opts.cwd ?? ROOT })
}

function tryRun(cmd, args, opts = {}) {
  try {
    return run(cmd, args, opts)
  } catch {
    return null
  }
}

ROOT = run('git', ['rev-parse', '--show-toplevel'])

/* 真发版的人类确认位：交互跑由末尾那句 `yes` 给，非交互跑由 `--confirm` 给。
   放在 `core` 之前声明 —— 它读这个变量，不能等到 TDZ 里才出生。 */
let humanConfirmed = false

const step = (m) => console.log(`\n\u001b[36m▶ ${m}\u001b[0m`)
const ok = (m) => console.log(`  \u001b[32mok\u001b[0m    ${m}`)
const info = (m) => console.log(`        ${m}`)

/** 停住。rollback 是给人看的回退命令，不自动执行 */
function die(message, rollback = []) {
  console.error(`\n\u001b[31m停在这里：${message}\u001b[0m`)
  if (rollback.length) {
    console.error('\n  要回到发版前的状态，按顺序跑（自己确认再跑，脚本不替你执行）：')
    for (const line of rollback) console.error(`    ${line}`)
  }
  console.error('')
  process.exit(1)
}

/**
 * 调**发布软件版本的内核**（`cargo run --bin release --features workbench -- …`）。
 *
 * 摘要走 stderr（直连终端），结果是一行 JSON（stdout）—— 这个脚本读 JSON，
 * 不解析人类语言。内核失败时 `cargo run` 带非零退出码，这里如实把消息抛上去。
 */
function core(args) {
  if (!humanConfirmed) die('内核拒绝：真发版没拿到人类确认（交互跑要敲 yes，非交互跑要带 --confirm）')
  /* ★ 演练必须**真的**演练（2026-11-06 真机事故）：这一行以前无条件送 `--confirm`，
   * 而 `--dry-run` 从来没被传下去 —— 于是 `release.mjs --dry-run` 会真的推进版本号、
   * 提交、推送（"演练"把分支写了）。模式只认一个：
   *   · `--dry-run` → 给内核 `--dry-run`（零写入），**绝不给 `--confirm`**；
   *   · 否则 → 给 `--confirm`。 */
  const mode = flagDryRun ? ['--dry-run'] : ['--confirm']
  const bin = ['run', '--bin', 'release', '--features', 'workbench', '--']
  const out = execFileSync('cargo', [...bin, ...args, ...mode], {
    encoding: 'utf8',
    cwd: ROOT,
    stdio: ['ignore', 'pipe', 'inherit'],
  })
  const line = out.trim().split('\n').filter(Boolean).pop()
  if (!line) throw new Error('内核没给出报告（stdout 是空的）')
  return JSON.parse(line)
}


/* ---------- 0. 参数与版本号（**必须最先算**：自检模式要据此跳过仓库检查） ---------- */

const current = tryRun('node', ['-p', "require('./package.json').version"]) ?? '0.0.0'

/* 参数先算：`--args-only` 决定后面要不要碰仓库状态。 */
const cliArgs = process.argv.slice(2)
const flagArgsOnly = cliArgs.includes('--args-only')

/* 命令行给了就不问：`npm run release -- 0.0.1 "一句话说明"`。
   这条路存在的理由不是省事，而是让这个脚本能被非交互地跑一遍 ——
   只能人工敲的脚本没法进任何自动化，也就更容易长期带着 bug 不被发现。 */
const flagConfirmed = cliArgs.includes('--confirm')
const flagDryRun = cliArgs.includes('--dry-run')
/* 只解析参数、算出版本号就退出：**零写入、不联网、不碰 git**。
 * 存在的理由：发版脚本的其余路径只有真发版才跑，bug 只能在那时候暴露。
 * 这条路径进 `scripts/check-release-cli.mjs`，跟着每个 PR 跑一遍。 */

/* 通道（2026-11-05 预览发布事务）：`--channel=preview` / `--channel preview` / `--preview`。
   缺省 = 正式版；认不出的值**当场停**（兜成 stable 会把预览版发成全量通知）。 */
function parseChannel(raw) {
  const v = String(raw ?? '').trim()
  if (v === 'stable' || v === 'preview') return v
  die(`认不出的更新通道：${v || '(空)'}（只认 stable / preview）`)
}

let channel = 'stable'
const positionalArgs = []
for (let i = 0; i < cliArgs.length; i += 1) {
  const a = cliArgs[i]
  if (a === '--confirm') continue
  if (a === '--preview') {
    channel = 'preview'
    continue
  }
  if (a === '--stable') {
    channel = 'stable'
    continue
  }
  if (a === '--channel') {
    channel = parseChannel(cliArgs[i + 1])
    i += 1
    continue
  }
  if (a.startsWith('--channel=')) {
    channel = parseChannel(a.slice('--channel='.length))
    continue
  }
  positionalArgs.push(a)
}
const [argVersion, argSummary] = positionalArgs

let version = argVersion?.trim()
let summary = argSummary?.trim()

/*
 * 开发版本带 `-dev`（2026-11-04 作者裁定：发过的号永不再用；开发永远拿下一个号的
 * `-dev`）。发布就是把 `-dev` 去掉 —— 所以默认值取去掉后的那个，人只按回车就行。
 *
 * ★ 预览通道（2026-11-05）默认给 `x.y.z-preview.1`：序号从 1 起，人只在发第二个
 *   预览时改数字。发了这个号**不会**消耗 `x.y.z` 正式版。
 *
 * ★ **顺序在这里是有意义的**（2026-11-06 真机踩过）：`channel` 必须先解析出来，
 *   这两行才能算。先前把解析块放在下面，`npm run release -- … --channel preview`
 *   在 `suggested` 那一行直接 `Cannot access 'channel' before initialization` ——
 *   而 `--dry-run` 走不到这里，所以只有真发版才炸。
 */
const currentBase = current.replace(/-preview\.\d+$/i, '')
/* 预览建议值 = **当前工作区那个预览号的下一号**（2026-11-06：以前一律给 `-preview.1`，
   工作区已经在 preview.6 时那句话就是错的 —— 会让人以为要发一个更旧的号）。 */
const currentSeq = /-preview\.(\d+)$/i.exec(current)
const suggested =
  channel === 'preview'
    ? `${currentBase}-preview.${currentSeq === null ? 1 : Number(currentSeq[1]) + 1}`
    : currentBase

/* ---------- 1. 前置检查（没有任何写操作） ---------- */

/* ★ 自检模式（`--args-only`）**不碰仓库状态**：它只回答"这段参数会被解析成什么"。
 *   2026-11-06 第一版把出口放在这一段之后，于是自检在 main 上永远对着
 *   "main 上不能发版"失败 —— 那个理由和它要验的东西毫无关系。 */
if (!flagArgsOnly) {
step('前置检查')
const branch = tryRun('git', ['symbolic-ref', '--short', 'HEAD'])
if (!branch) die('当前是 detached HEAD。先切到一条 feat/ 分支上')
if (branch === 'main') {
  die(
    'main 上不能发版。发版是"把分支上的改动发出去"，本身要从分支开始：\n' +
      '    git switch -c feat/<something>',
  )
}
ok(`分支 ${branch}`)

if (tryRun('git', ['status', '--porcelain'])) {
  die('工作区不干净。先提交或 stash —— 发版会改版本号并提交，混在一起说不清')
}
ok('工作区干净')

const hooksPath = tryRun('git', ['config', '--get', 'core.hooksPath'])
if (hooksPath !== 'scripts/hooks') {
  die(
    `闸门没装好（core.hooksPath = ${hooksPath ?? '(空)'}，应为 scripts/hooks）。\n` +
      '    跑一次：npm install（或 node scripts/setup-hooks.mjs）',
  )
}
ok('本地闸门已生效')

const origin = tryRun('git', ['remote', 'get-url', 'origin'])
if (!origin) die('没配 origin。PR-only 流程离不开远端：gh repo create --source . --remote origin')
ok(`origin ${origin}`)

if (!tryRun('gh', ['auth', 'status'])) {
  die('gh 没登录（或没装）。等 CI 那一段要用它查状态：gh auth login')
}
ok('gh 可用')

/* 复述上次 tag 之后的绕过记录 —— bypass.log 的全部价值在于有人真的念它 */
const gitDir = run('git', ['rev-parse', '--absolute-git-dir'])
const bypassLog = `${gitDir}/bypass.log`
if (existsSync(bypassLog)) {
  const lines = readFileSync(bypassLog, 'utf8').trim().split('\n').filter(Boolean)
  if (lines.length) {
    console.log(`\n  \u001b[33m.git/bypass.log 里有 ${lines.length} 条绕过记录：\u001b[0m`)
    for (const l of lines) info(l)
    info('开仓阶段应该只有两条（main 直提 + push main）。多出来的那些值得看一眼。')
  }
} else {
  ok('没有绕过记录')
}
}

/* ---------- 2. 版本号与说明（先问，再花几分钟校验 —— 不让人干等着被提问） ---------- */


/* ★ **自检出口必须在前置检查之前**（2026-11-06 第一版写错了）：
 *   放在后面时它要先过"分支是不是 main / 工作区干不干净"，于是自检在 main 上永远
 *   跑不通，只能对着一个和它无关的理由失败。自检要完全不依赖仓库状态 ——
 *   `--args-only` 只回答"这段参数会被解析成什么"，零写入、不联网、不碰 git。 */
if (flagArgsOnly) {
  if (!version) die('--args-only 需要显式给版本号')
  console.log(
    JSON.stringify({ channel, current, suggested, version: resolveVersion(version, channel) }),
  )
  process.exit(0)
}

step(
  `版本号（当前 ${current}，${channel === 'preview' ? '预览' : '正式'}发布会写成 ${suggested}）`,
)

/* ★ 真发版的人当次许可：交互跑要亲口敲 yes；非交互跑必须显式带 `--confirm`。
   只给了版本号和说明**不算**许可 —— 那两样 AI 自己就能凑出来。 */
if (!version || !summary) {
  const rl = createInterface({ input, output })
  version =
    version ||
    (await rl.question(`  新版本号（不带 v，回车沿用 ${suggested}）: `)).trim() ||
    suggested
  summary = summary || (await rl.question('  一句话说明（会成为 Release 正文）: ')).trim()
  const answer = (await rl.question('  确认发布？会打 tag、上传安装包、写 release.json（输入 yes 继续）: ')).trim()
  rl.close()
  if (answer !== 'yes') die('没拿到确认 —— 一个字节都没动')
  humanConfirmed = true
} else {
  info(`版本 ${version}；说明「${summary}」（来自命令行参数）`)
  /* `--dry-run` 是**零写入**的预演，内核那一层也明说不必确认（`run()` 第一句
   * 只在 `!dry_run` 时才要 `confirmed`）。所以这里不能拿 `--confirm` 把它挡住 ——
   * 否则"只看预检请用 --dry-run"这句话自成矛盾（2026-11-06 真机踩到）。 */
  if (!flagConfirmed && !flagDryRun) {
    die('非交互真发版必须显式带 --confirm（人当次明确许可）；只看预检请加 --dry-run')
  }
  humanConfirmed = true
}

/* 通道 + 版本号一起判，判不过就 die（退出码 1）。
 * 抽成函数是为了让 `--args-only` 与真发版走**同一套**判定 —— 否则自检和实际行为会漂。 */
function resolveVersion(raw, ch) {
  // 允许把 `0.0.10-dev` / `0.0.10-dev.7` 原样传进来：正式发布就是去掉 `-dev[.N]`
  // （那个 N 只是**本地构建号**，不该跟着发出去）；
  // 预览号（`0.0.10-preview.1`）原样保留 —— 它本身就是要发出去的号。
  /* ★ 2026-11-06 起**没有 `-dev` 了**（作者：「不要再出现 DEV」）：
    版本号只有两种形状 —— 正式 `x.y.z`、预览 `x.y.z-preview.N`。
    大小写归一成小写：`Preview.2` 与 `preview.2` 必须是**同一个** tag，
    否则同一个号会有两个 tag，git 那边再也说不清哪个是权威。 */
  const v = raw.trim().replace(/-(preview)\./i, '-preview.')
  if (!/^\d+\.\d+\.\d+(-preview\.[1-9]\d*)?$/.test(v)) {
    die(`版本号格式不对：${v}（要 x.y.z 或 x.y.z-preview.N，没有 -dev 了）`)
  }
  /* 通道与版本身份必须一致：串台的两边后果都很难撤（内核里还会再判一次，这里只为早停） */
  const isPreview = /-preview\.\d+$/.test(v)
  if (ch === 'preview' && !isPreview) die(`预览通道要发预览号（x.y.z-preview.N），现在是 ${v}`)
  if (ch === 'stable' && isPreview) {
    die(`★ 预览号不能走正式通道（${v}）—— 全量用户会收到这条通知；加 --channel preview`)
  }
  return v
}

version = resolveVersion(version, channel)
if (!summary) die('说明不能为空 —— 它是 Release 正文，也是以后 git log 上唯一能看到的东西')

if (tryRun('git', ['rev-parse', '-q', '--verify', `refs/tags/v${version}`])) {
  die(
    `tag v${version} 已经存在 —— 已发布的号永远不再用` +
      '（同一个号两种内容，客户端与回滚都说不清）。\n' +
      `    下一个开发号：npm run version:next（当前 ${current}）`,
  )
}

/* ---------- 3. 校验链（**没有任何写操作**） ---------- */

const tauriDir = `${ROOT}/src-tauri`

step('校验：前端')
runLive('npm', ['run', 'lint'])
runLive('npx', ['tsc', '-b'])
runLive('npm', ['run', 'build'])
ok('lint / tsc / build 过')

step('校验：Rust')
runLive('cargo', ['fmt', '--all', '--', '--check'], { cwd: tauriDir })
runLive('cargo', ['clippy', '--all-targets', '--', '-D', 'warnings'], { cwd: tauriDir })
runLive('cargo', ['test'], { cwd: tauriDir })
ok('fmt / clippy / test 过')

/* ---------- 4. 第一趟：版本号 → 提交 → 推送 → PR（**内核**） ---------- */

step('内核：版本号 → 提交 → 推送 → PR')
info('版本号只改 src-tauri/Cargo.toml 一处，其余（package.json / tauri.conf.json / Cargo.lock）由内核派生')

let first
try {
  first = core([version, summary, `--channel=${channel}`])
} catch (e) {
  die(`内核第一趟没跑通：${e.message ?? e}`)
}
ok(`阶段 ${first.stage}：${first.summary}`)

if (first.stage === 'blockedPreflight') {
  die(`预检没过（内核一个字节都没动）：\n    ${first.blockedReasons.join('\n    ')}`)
}

/* ★ 演练到预检为止（2026-11-06）：`--dry-run` 的契约是"一个字节都不动"，
 * 而后面那半段（等 CI / 合并 / 打 tag / 构建 / 上传）每一步都是实写。所以这里
 * **必须停**：把内核报告原样打出来就结束，不往下走。 */
if (flagDryRun) {
  step('演练到此为止（--dry-run）')
  console.log(JSON.stringify(first, null, 2))
  info('版本号、tag、Release、清单：一个字节都没动。')
  process.exit(0)
}

const rollback = [
  `git reset --hard HEAD   # 丢掉版本号提交（确认没有别的改动再跑）`,
  /* 分支名取内核报告里那一格：`branch` 只活在前面那个 `if` 块里，这里取不到 */
  `git push origin --delete ${first.branch ?? '<发布分支>'}   # 如果已经推上去了`,
]

const prNumber = first.review?.number
if (!prNumber) {
  info('没有建 PR（没配发布账户 / 关掉了建 PR）—— 剩下的 git 步骤自己走完')
} else {
  ok(`PR #${prNumber}（${first.review.url}）`)

  /* ---------- 5. 等 CI（**脚本的活**：内核不轮询） ---------- */

  step('等 CI')
  info('轮询 gh pr checks，最多等 30 分钟')

  const deadline = Date.now() + 30 * 60 * 1000
  let checksOk = false
  while (Date.now() < deadline) {
    const out = tryRun('gh', ['pr', 'checks', String(prNumber), '--json', 'name,state'])
    if (out) {
      const checks = JSON.parse(out)
      const bad = checks.filter((c) => ['FAILURE', 'CANCELLED', 'TIMED_OUT', 'ERROR'].includes(c.state))
      const pending = checks.filter((c) => ['PENDING', 'QUEUED', 'IN_PROGRESS'].includes(c.state))
      if (bad.length) {
        die(
          `CI 红了：${bad.map((c) => `${c.name}=${c.state}`).join('、')}\n` +
            `    版本号那一笔已经提交并推送了。修完再跑一次 npm run release（它会回读这个 PR）`,
          rollback,
        )
      }
      if (checks.length && !pending.length) {
        checksOk = true
        ok(`全绿：${checks.map((c) => c.name).join('、')}`)
        break
      }
      info(`等 ${pending.length} 项：${pending.map((c) => c.name).join('、')}`)
    }
    await new Promise((r) => setTimeout(r, 15000))
  }
  if (!checksOk) die('等了 30 分钟 CI 还没结束。自己去看一眼：gh pr checks', rollback)
}

/* ---------- 6. 第二趟：合并 → 打 tag → 构建 → Release → 上传 → release.json（**内核**） ---------- */

const manifestRel =
  channel === 'preview' ? 'presets/delivery/release-preview.json' : 'presets/delivery/release.json'

step(`内核：合并 → tag → 构建 → Release → 上传 → ${manifestRel}`)
info('★ 内核会切到 main、在 main 的 tip 上打 tag、构建 macOS 安装包、建 Release 并上传')
if (channel === 'preview') {
  info('★ 预览通道：Release 标 prerelease，只写 release-preview.json —— **release.json 一个字节都不动**')
  info('★ 那份清单会开**第二个 PR** —— 合并它之后，主动点「检查更新」的人才看得到这个预览版')
} else {
  info(`★ ${manifestRel} 那一笔会开**第二个 PR** —— 合并它之后客户端才看得到新版本`)
}

let second
try {
  second = core([version, summary, `--channel=${channel}`, '--merge'])
} catch (e) {
  die(
    `内核第二趟没跑通：${e.message ?? e}`,
    [
      ...rollback,
      `gh pr view ${prNumber}   # 看看 PR 现在是什么状态`,
    ],
  )
}
ok(`阶段 ${second.stage}：${second.summary}`)

console.log(`
\u001b[32m发完了：v${version}（${channel === 'preview' ? '预览通道' : '正式通道'}）\u001b[0m

  tag：      ${second.tag ?? `v${version}`}
  Release：  ${second.release?.url ?? '（这一趟没建 Release）'}
  安装包：   ${second.artifact?.name ?? '（这一趟没构建）'}
  ${manifestRel} 的 PR：${second.infoReview?.url ?? '（没有 —— 可能上传没走到）'}

  ★ **要等 ${manifestRel} 那个 PR 合并**才生效（客户端只读 main 上的东西）。
${
  channel === 'preview'
    ? `
  ★ 这一趟**没有**改动 release.json —— 正式版用户不会收到任何通知。
    验证：装 0.0.9 的机器打开软件，不该出现"有新版"；点「检查更新」能看到这个预览版。
`
    : ''
}
  下一步（发布号已固定，别再动它）：
    npm run version:next      # 预览版会回到同一基础的 -dev（0.0.10-preview.1 → 0.0.10-dev）
    git switch -c feat/<next>
`)
