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
  const bin = ['run', '--bin', 'release', '--features', 'workbench', '--']
  const out = execFileSync('cargo', [...bin, ...args], {
    encoding: 'utf8',
    cwd: ROOT,
    stdio: ['ignore', 'pipe', 'inherit'],
  })
  const line = out.trim().split('\n').filter(Boolean).pop()
  if (!line) throw new Error('内核没给出报告（stdout 是空的）')
  return JSON.parse(line)
}

/* ---------- 1. 前置检查（没有任何写操作） ---------- */

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

/* ---------- 2. 版本号与说明（先问，再花几分钟校验 —— 不让人干等着被提问） ---------- */

const current = tryRun('node', ['-p', "require('./package.json').version"]) ?? '0.0.0'

step(`版本号（当前 ${current}）`)

/* 命令行给了就不问：`npm run release -- 0.0.1 "一句话说明"`。
   这条路存在的理由不是省事，而是让这个脚本能被非交互地跑一遍 ——
   只能人工敲的脚本没法进任何自动化，也就更容易长期带着 bug 不被发现。 */
const [argVersion, argSummary] = process.argv.slice(2)

let version = argVersion?.trim()
let summary = argSummary?.trim()

if (!version || !summary) {
  const rl = createInterface({ input, output })
  version = version || (await rl.question(`  新版本号（不带 v，回车沿用 ${current}）: `)).trim() || current
  summary = summary || (await rl.question('  一句话说明（会成为 Release 正文）: ')).trim()
  rl.close()
} else {
  info(`版本 ${version}；说明「${summary}」（来自命令行参数）`)
}

if (!/^\d+\.\d+\.\d+$/.test(version)) die(`版本号格式不对：${version}（要 x.y.z）`)
if (!summary) die('说明不能为空 —— 它是 Release 正文，也是以后 git log 上唯一能看到的东西')

if (tryRun('git', ['rev-parse', '-q', '--verify', `refs/tags/v${version}`])) {
  die(`tag v${version} 已经存在。换一个版本号，或先确认那一版发到哪了`)
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
  first = core([version, summary])
} catch (e) {
  die(`内核第一趟没跑通：${e.message ?? e}`)
}
ok(`阶段 ${first.stage}：${first.summary}`)

if (first.stage === 'blockedPreflight') {
  die(`预检没过（内核一个字节都没动）：\n    ${first.blockedReasons.join('\n    ')}`)
}

const rollback = [
  `git reset --hard HEAD   # 丢掉版本号提交（确认没有别的改动再跑）`,
  `git push origin --delete ${branch}   # 如果已经推上去了`,
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

step('内核：合并 → tag → 构建 → Release → 上传 → release.json')
info('★ 内核会切到 main、在 main 的 tip 上打 tag、构建 macOS 安装包、建 Release 并上传')
info('★ release.json 那一笔会开**第二个 PR** —— 合并它之后客户端才看得到新版本')

let second
try {
  second = core([version, summary, '--merge'])
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
\u001b[32m发完了：v${version}\u001b[0m

  tag：      ${second.tag ?? `v${version}`}
  Release：  ${second.release?.url ?? '（这一趟没建 Release）'}
  安装包：   ${second.artifact?.name ?? '（这一趟没构建）'}
  release.json 的 PR：${second.infoReview?.url ?? '（没有 —— 可能上传没走到）'}

  ★ **客户端要等 release.json 那个 PR 合并**才看得到新版本（GitHub raw 只吐 main 上的东西）。

  下一步开新分支继续：
    git switch -c feat/<next>
`)
