#!/usr/bin/env node
/**
 * 发版前的**深度预检**（只读，不写任何东西、不建 PR、不打 tag）。
 *
 * 为什么要有它：2026-11-19 那一轮，本地 CI 与云端 CI 全绿之后才接连撞上
 * 「GitHub token 失效」「带认证推 tag 没带发布事务标记」「发布分支与 main 同一点，
 * GitHub 拒绝开空 PR」——每一处都要重等十几分钟的 CI。这些都能在开 PR 之前查出来。
 *
 * 两种跑法：
 *   node scripts/check-release-readiness.mjs            全量（ship 调；含网络与 git 状态）
 *   node scripts/check-release-readiness.mjs --static   静态部分（挂在每个 PR 的 lint 上）
 */
import { execFileSync } from 'node:child_process'
import { existsSync, readFileSync, readdirSync } from 'node:fs'
import { homedir, platform } from 'node:os'
import { join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const REPO = resolve(fileURLToPath(new URL('.', import.meta.url)), '..')
const STATIC_ONLY = process.argv.includes('--static')
const channelArg = process.argv.find((a) => a.startsWith('--channel='))?.slice(10) ?? null

const run = (cmd, args) =>
  execFileSync(cmd, args, {
    cwd: REPO,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  })
const tryRun = (cmd, args) => {
  try {
    return run(cmd, args).trim()
  } catch {
    return null
  }
}

let failed = 0
const check = (name, fn) => {
  try {
    const detail = fn()
    console.log(`  ✓ ${name}${detail ? ` — ${detail}` : ''}`)
  } catch (e) {
    console.error(`  ✗ ${name}\n      ${e.message}`)
    failed += 1
  }
}
const info = (m) => console.log(`        ${m}`)
const fail = (m) => {
  throw new Error(m)
}

/* ---------- 静态部分：每个 PR 都跑 ---------- */

let version = ''
check('版本形状与四处派生一致', () => {
  version = /version\s*=\s*"([^"]+)"/.exec(readFileSync(join(REPO, 'version.toml'), 'utf8'))?.[1] ?? ''
  if (!version) fail('version.toml 里读不到 version')
  const expected = channelArg === null ? null : channelArg === 'preview' ? /^\d+\.\d+\.\d+-preview\.\d+$/ : /^\d+\.\d+\.\d+$/
  if (expected && !expected.test(version)) fail(`版本 ${version} 与通道 ${channelArg} 不匹配`)
  run('node', ['scripts/version.mjs', 'check'])
  return version
})

check('推 tag 的两条路径都带发布事务标记', () => {
  const src = readFileSync(join(REPO, 'src-tauri/src/workbench/app/git.rs'), 'utf8')
  const marks = (src.match(/\.env\("MKP_RELEASE_TX",\s*"1"\)/g) ?? []).length
  if (marks < 2) fail(`只找到 ${marks} 处 MKP_RELEASE_TX；push_tag 与带认证的推 tag 都要带`)
  const plain = src.slice(src.indexOf('pub fn push_tag('))
  if (!/\.env\("MKP_RELEASE_TX",\s*"1"\)/.test(plain)) fail('push_tag 那条路径丢了标记')
  if (!/refspec\.starts_with\("refs\/tags\/"\)[\s\S]{0,160}\.env\("MKP_RELEASE_TX",\s*"1"\)/.test(src)) {
    fail('带认证的 push_ref_to_authenticated 推 tag 时没带标记（闸⑤会当场拒）')
  }
  return `${marks} 处`
})

check('必需检查不设 macOS rust', () => {
  const src = readFileSync(join(REPO, 'scripts/setup-ruleset.mjs'), 'utf8')
  const list = /const REQUIRED_CHECKS = \[([^\]]*)\]/.exec(src)?.[1] ?? ''
  const names = list.match(/'[^']+'/g)?.map((s) => s.slice(1, -1)) ?? []
  if (names.length === 0) fail('读不到 REQUIRED_CHECKS')
  if (names.includes('rust')) fail('必需检查里还有 macOS 的 rust —— 它会排长队；改用 rust-windows')
  if (!names.includes('web') || !names.includes('rust-windows')) fail('web / rust-windows 必须仍是必需检查')
  return names.join('、')
})

if (STATIC_ONLY) {
  if (failed > 0) {
    console.error(`\n静态预检：${failed} 条没过。`)
    process.exit(1)
  }
  console.log(`\n静态预检通过（${version}）。`)
  process.exit(0)
}

/* ---------- 全量部分：只有真发版时跑 ---------- */

check('分支与工作区', () => {
  const branch = tryRun('git', ['branch', '--show-current']) ?? ''
  if (!branch || branch === 'main') fail('不能在 main 上发版（PR-only）')
  const dirty = tryRun('git', ['status', '--porcelain'])
  if (dirty) fail(`工作区不干净：\n${dirty.split('\n').slice(0, 6).join('\n')}`)
  return branch
})

/* 旧产物残留必须在构建前拦住，而不是编了十几分钟才发现 bundle 里有多份。 */
check('构建目录没有旧版本产物残留', () => {
  const dirs = [
    'target/release/bundle/nsis',
    'target/release/bundle/dmg',
    'target/release/bundle/macos',
    'dist-portable',
  ]
  const stale = []
  for (const rel of dirs) {
    const dir = join(REPO, rel)
    if (!existsSync(dir)) continue
    for (const ent of readdirSync(dir, { withFileTypes: true })) {
      if (!ent.isFile()) continue
      if (!/\.(exe|dmg|zip)$/i.test(ent.name)) continue
      if (!ent.name.includes(version)) stale.push(`${rel}/${ent.name}`)
    }
  }
  if (stale.length > 0) {
    fail(`有旧版本产物，先移出构建目录：\n      ${stale.join('\n      ')}`)
  }
  return '没有旧版本'
})

const tag = `v${version}`
const head = tryRun('git', ['rev-parse', 'HEAD']) ?? ''
const localTag = tryRun('git', ['rev-parse', '-q', '--verify', `refs/tags/${tag}^{commit}`])
/* 必须带 `*`：不带星时 git 只回 raw tag 对象那一行，没有 `^{}` 剥皮行。 */
const remoteTagLines = tryRun('git', ['ls-remote', '--tags', 'origin', `refs/tags/${tag}*`])
/* annotated tag 有两行：`refs/tags/vX` 是 **tag 对象** SHA，`refs/tags/vX^{}` 才是提交。
   拿第一行跟 HEAD 比会把"同一笔提交"误报成 tag 分叉（2026-11-21 0.0.12 就这么误判过）。 */
function remoteTagCommit(raw) {
  if (!raw) return null
  const rows = raw.trim().split('\n').filter(Boolean)
  const peeled = rows.find((line) => line.includes(`${tag}^{}`))
  const picked = peeled ?? rows[0]
  return picked ? picked.split(/\s+/)[0] : null
}
const remoteTagCommitSha = remoteTagCommit(remoteTagLines)

check('tag 未被占用，或与 HEAD 同一笔提交', () => {
  if (localTag && localTag !== head) fail(`本地 ${tag} 指向 ${localTag.slice(0, 8)}，HEAD 是 ${head.slice(0, 8)}`)
  if (remoteTagCommitSha) {
    if (remoteTagCommitSha !== head) {
      fail(`远端 ${tag} 指向 ${remoteTagCommitSha.slice(0, 8)}，与当前 HEAD 不同 —— 不许重打/移动`)
    }
    return '远端已有（补包趟）'
  }
  return '未发布'
})

if (!localTag && !remoteTagCommitSha) {
  check('发布分支有可合并的提交（不然 GitHub 会拒开空 PR）', () => {
    const n = Number(tryRun('git', ['rev-list', '--count', 'origin/main..HEAD']) ?? '0')
    if (n === 0) fail('分支与 origin/main 同一点：先把版本号 / 发版说明那一笔合进 main，再从新分支开始')
    return `${n} 笔`
  })
}

/** 发布账户解析：优先工作台配置（与内核同一份），否则 gh 当前登录态。 */
function githubCredential() {
  const appDir =
    process.env.MKPSE_APP_DIR ||
    join(homedir(), platform() === 'win32' ? 'AppData/Roaming' : 'Library/Application Support', 'SupportEase-Workbench')
  const cfgPath = join(appDir, 'publish-account.json')
  const credPath = join(appDir, 'credentials.json')
  if (existsSync(cfgPath) && existsSync(credPath)) {
    const cfg = JSON.parse(readFileSync(cfgPath, 'utf8'))
    const creds = JSON.parse(readFileSync(credPath, 'utf8'))
    const account = cfg.github
    const token = creds.secrets?.['supportease.github.token']
    if (account?.repositoryUrl && account?.username && token) {
      return { username: account.username, token, repositoryUrl: account.repositoryUrl, source: '工作台发布账户' }
    }
  }
  const token = tryRun('gh', ['auth', 'token'])
  const url = tryRun('git', ['remote', 'get-url', 'origin'])
  const m = /github\.com[:/]([^/]+)\/([^/.]+)/.exec(url ?? '')
  if (token && m) return { username: m[1], token, repositoryUrl: `https://github.com/${m[1]}/${m[2]}`, source: 'gh 登录态' }
  fail('找不到可用的 GitHub 凭据：配工作台发布账户，或先 gh auth login')
}

check('GitHub 凭据可用（真发版前先问一次远端）', () => {
  const c = githubCredential()
  const origin = tryRun('git', ['remote', 'get-url', 'origin']) ?? ''
  const norm = (s) => s.replace(/\.git$/, '').replace(/^git@github\.com:/, 'https://github.com/').toLowerCase()
  if (norm(c.repositoryUrl) !== norm(origin)) {
    fail(`发布账户的仓库 ${c.repositoryUrl} 与 origin ${origin} 不一致`)
  }
  const basic = Buffer.from(`${c.username}:${c.token}`).toString('base64')
  const got = tryRun('git', [
    '-c', 'credential.helper=',
    '-c', `http.extraHeader=Authorization: Basic ${basic}`,
    'ls-remote', 'origin', 'HEAD',
  ])
  if (!got || !/^[0-9a-f]{40}/.test(got)) fail('GitHub token 失效或没有仓库权限；去工作台重贴 Token')
  return `${c.source} · ${c.username} · ls-remote ok`
})

info('Windows 这趟会构建安装版（.exe）与绿色版（.zip），两档都进 Release。')
info('macOS 的 .dmg 要在 Mac 上用补包趟单独打；CI 只跑检查，不打包。')

if (failed > 0) {
  console.error(`\n发版预检：${failed} 条没过。留在原地，不建 PR、不打 tag。`)
  process.exit(1)
}
console.log(`\n发版预检通过（${version}）。`)
