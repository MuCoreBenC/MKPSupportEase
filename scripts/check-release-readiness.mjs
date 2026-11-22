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

/* ---------- 全量部分：发布内核 2.0 起不在这里 ----------
 *
 * 真发的预检由内核自己做（`npm run ship -- --check` 看状态、`--dry-run` 演练）。
 * 这一份只保留**静态**那几格，挂在每个 PR 的 lint 上。
 */
console.log('\n（全量预检已并入发布内核：npm run ship -- --check）')
