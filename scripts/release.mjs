/**
 * 发版：一条命令串起校验 → PR → 合并 → 打 tag。
 *
 * 与试验场那份的根本区别：**这里不碰 main**。那边是"本地合并进 main + 推 main"，
 * 在 PR-only 下直接违规（闸②与服务端 ruleset 都会拒）。所以流程改成：
 * 在 feat/ 分支上校验 → 改版本号 → 开 PR → 等 CI 绿 → squash 合并 → 回到 main 打 tag。
 *
 * 三条纪律：
 * 1. **任何写操作之前先跑完全部校验** —— 失败时工作区还是干净的，不用回退；
 * 2. **破坏性命令只打印、不执行** —— 出错时给你精确的回退命令，由你决定要不要跑；
 * 3. **squash 会重写提交**，所以 tag 必须在 `git switch main && git pull` 之后、
 *    在 main 的 tip 上打。在分支上打的 tag 指向一个不在 main 历史里的提交。
 */
import { execFileSync } from 'node:child_process'
import { existsSync, readFileSync, writeFileSync } from 'node:fs'
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
  die('工作区不干净。先提交或 stash —— 发版会改三个文件的版本号并提交，混在一起说不清')
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
  die('gh 没登录（或没装）。这套流程要用它开 PR 与查 CI：gh auth login')
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

const tauriDir = `${ROOT}/src-tauri`
const pkgPath = `${ROOT}/package.json`
const cargoPath = `${tauriDir}/Cargo.toml`
const confPath = `${tauriDir}/tauri.conf.json`

const pkg = JSON.parse(readFileSync(pkgPath, 'utf8'))
const current = pkg.version

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
  summary = summary || (await rl.question('  一句话说明（会成为 PR 标题）: ')).trim()
  rl.close()
} else {
  info(`版本 ${version}；说明「${summary}」（来自命令行参数）`)
}

if (!/^\d+\.\d+\.\d+$/.test(version)) die(`版本号格式不对：${version}（要 x.y.z）`)
if (!summary) die('说明不能为空 —— 它是 PR 标题，也是以后 git log 上唯一能看到的东西')

if (tryRun('git', ['rev-parse', '-q', '--verify', `refs/tags/v${version}`])) {
  die(`tag v${version} 已经存在。换一个版本号，或先确认那一版发到哪了`)
}

/* ---------- 3. 校验链 ---------- */

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

/* ---------- 4. 落版本号 ---------- */

/* 三处一起改。少改一处的后果是安装包版本与界面版本不一致，而且没人会发现 */
writeFileSync(pkgPath, JSON.stringify({ ...pkg, version }, null, 2) + '\n')

const cargo = readFileSync(cargoPath, 'utf8')
const cargoNext = cargo.replace(/^version = "[^"]*"/m, `version = "${version}"`)
if (cargoNext === cargo && !cargo.includes(`version = "${version}"`)) {
  die(`改不动 ${cargoPath} 的 version —— 格式变了？手工确认一下`)
}
writeFileSync(cargoPath, cargoNext)

const conf = JSON.parse(readFileSync(confPath, 'utf8'))
writeFileSync(confPath, JSON.stringify({ ...conf, version }, null, 2) + '\n')

/*
 * 第四处：Cargo.lock。
 *
 * 它也记着本 crate 的版本号。漏掉的后果是下一次任何人跑 cargo 都会被它自动改写 ——
 * 工作区凭空变脏，而且入库的 lock 与 manifest 不一致。v0.0.1 那次就是这么漏的。
 * `--workspace --offline`：只刷新本 workspace 的条目，不去网络升级依赖。
 */
runLive('cargo', ['update', '--workspace', '--offline'], { cwd: tauriDir })

ok(`版本号四处改成 ${version}（含 Cargo.lock）`)

const rollback = [
  `git reset --hard HEAD   # 丢掉版本号提交（确认没有别的改动再跑）`,
  `git push origin --delete ${branch}   # 如果已经推上去了`,
]

runLive('git', [
  'add',
  'package.json',
  'src-tauri/Cargo.toml',
  'src-tauri/Cargo.lock',
  'src-tauri/tauri.conf.json',
])
runLive('git', ['commit', '-m', `chore: v${version}`])
ok('版本号已提交')

/* ---------- 5. PR ---------- */

step('推分支并开 PR')
runLive('git', ['push', '-u', 'origin', 'HEAD'])

const existing = tryRun('gh', ['pr', 'view', '--json', 'number', '--jq', '.number'])
if (existing) {
  ok(`已有 PR #${existing}，沿用它`)
} else {
  runLive('gh', ['pr', 'create', '--title', summary, '--body', `发版 v${version}。\n\n${summary}`])
}
const prNumber = run('gh', ['pr', 'view', '--json', 'number', '--jq', '.number'])
ok(`PR #${prNumber}`)

/* ---------- 6. 等 CI ---------- */

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
          `    看日志：gh run list --branch ${branch}\n` +
          '    修完再跑一次 npm run release（它会沿用这个 PR）',
        [],
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
if (!checksOk) die('等了 30 分钟 CI 还没结束。自己去看一眼：gh pr checks')

/* ---------- 7. 合并 + 打 tag ---------- */

step('合并进 main')
runLive('gh', ['pr', 'merge', String(prNumber), '--squash', '--delete-branch'])
ok('已 squash 合并')

step('回到 main 打 tag')
runLive('git', ['switch', 'main'])
runLive('git', ['pull', '--ff-only'])

/* squash 之后 main 的 tip 是一个新提交，与分支上的任何提交都不同 —— tag 必须打在这里 */
runLive('git', ['tag', '-a', `v${version}`, '-m', `v${version} ${summary}`])
runLive('git', ['push', 'origin', `v${version}`])
ok(`v${version} 已打在 ${run('git', ['rev-parse', '--short', 'HEAD'])} 上并推送`)

console.log(`
\u001b[32m发完了：v${version}\u001b[0m

  main 的 tip：${run('git', ['rev-parse', '--short', 'HEAD'])}
  tag 指向的提交与分支上那几个都不是同一个 SHA（squash 重写过），这是正常的。

  下一步开新分支继续：
    git switch -c feat/<next>
`)
