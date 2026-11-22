#!/usr/bin/env node
/**
 * 发货入口（唯一一条）：**先看这一代到哪一步，再让内核往前走一步**。
 *
 * # 为什么只有一条
 *
 * 2026-11-22 之前，「发」至少四条路：`preview:build`（只在本机）、`local-release`（只在
 * `127.0.0.1`）、`presets publish`（只换预设目录）、`release`（打 tag 但清单还要人合）。
 * 四种都「像发版」，只有一种真的所有人能看到。
 *
 * 发布内核 2.0 起，软件版本发布**只有这一条**：
 *
 * ```bash
 * npm run ship -- --check                          # 只读：到哪一步、下一步做什么
 * npm run ship -- --what stable --dry-run          # 演练：一个字节都不动
 * npm run ship -- --what stable --confirm          # 真发（要人当次许可）
 * npm run ship -- --what stable --confirm --stop-after staging --platform windows
 * ```
 *
 * `--stop-after` / `--platform` 是给验收用的（模拟中断、分批构建），正常发布不用。
 *
 * # 它不判断「首发还是补平台」
 *
 * 内核自己从远端现读状态（冻结源码 / Gitee 暂存 / GitHub 草稿 / main 清单 / 正式 tag），
 * 谁没齐就补谁。所以换一台机器、上一趟被打断，都只是再跑一次同一条命令。
 */

import { execFileSync } from 'node:child_process'
import { argv, exit } from 'node:process'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const REPO = resolve(fileURLToPath(new URL('.', import.meta.url)), '..')

const ok = (m) => console.log(`\x1b[32m✓\x1b[0m ${m}`)
const bad = (m) => console.log(`\x1b[31m✗\x1b[0m ${m}`)
const info = (m) => console.log(`  ${m}`)

function die(msg, hints = []) {
  console.error(`\n\x1b[31m✗ ${msg}\x1b[0m\n`)
  for (const h of hints) console.error(`    ${h}`)
  if (hints.length) console.error('')
  exit(1)
}

/* ---------- 与发布内核同一套参数（`--what` 之外的都原样透传） ---------- */

const args = argv.slice(2)
function argOf(name) {
  const i = args.indexOf(name)
  if (i >= 0) return args[i + 1] ?? ''
  const eq = args.find((a) => a.startsWith(`${name}=`))
  return eq ? eq.slice(name.length + 1) : null
}

/** 只有这四种「发」。`--what` 必须由人明说，AI 不许自己判断。 */
const KINDS = {
  local: '只在本机编一份包（别人看不到）',
  presets: '只发预设数据（软件版本不动）',
  stable: '发正式版（所有客户端都会收到「有新版」）',
  preview: '发预览版（只有主动检查更新的人看得到）',
}

const what = (argOf('--what') ?? '').trim()
const check = args.includes('--check') || args.includes('--inspect')
const dryRun = args.includes('--dry-run')
const confirmed = args.includes('--confirm')

/* 内核那条路：cargo run --bin ship */
function kernel(passArgs) {
  const out = execFileSync(
    'cargo',
    ['run', '--quiet', '--bin', 'ship', '--features', 'workbench', '--', ...passArgs],
    { cwd: REPO, encoding: 'utf8', stdio: ['inherit', 'pipe', 'inherit'] },
  )
  const line = out.trim().split('\n').filter(Boolean).pop()
  if (!line) return null
  try {
    return JSON.parse(line)
  } catch {
    return null
  }
}

/* ---------- 只读：到哪一步了 ---------- */

if (check) {
  const report = kernel(['--check'])
  if (!report) die('内核没有给出报告')
  printMatrix(report)
  if (report.conflicts?.length) {
    bad(`${report.conflicts.length} 处冲突 —— 先解决再发`)
    for (const c of report.conflicts) info(c)
    exit(1)
  }
  ok(`下一步：${report.next}`)
  exit(0)
}

/* ---------- `--what` 那四种 ---------- */

if (!what) {
  console.log('\n  这一趟发什么？\n')
  for (const [k, v] of Object.entries(KINDS)) console.log(`    --what ${k.padEnd(8)} ${v}`)
  console.log('')
  die('要显式指定 --what（AI 不许替你判断）', [
    '先看一眼状态：npm run ship -- --check',
  ])
}
if (!(what in KINDS)) {
  die(`认不出的 --what：${what}`, [`只认 ${Object.keys(KINDS).join(' / ')}`])
}

if (what === 'local') {
  info('编译本机那一档（npm run tauri -- build）')
  execFileSync('npm', ['run', 'tauri', '--', 'build'], { cwd: REPO, stdio: 'inherit' })
  execFileSync('node', ['scripts/local-release.mjs'], { cwd: REPO, stdio: 'inherit' })
  ok('本地包好了 —— 别人看不到它')
  exit(0)
}

if (what === 'presets') {
  execFileSync('npm', ['run', 'presets', '--', 'publish', '--dry-run'], {
    cwd: REPO,
    stdio: 'inherit',
  })
  if (!confirmed) die('预设真发也要人当次许可：加 --confirm', ['这一步会推进发布序号'])
  execFileSync('npm', ['run', 'presets', '--', 'publish', '--confirm'], {
    cwd: REPO,
    stdio: 'inherit',
  })
  ok('预设目录发了 —— 软件版本没动')
  exit(0)
}

/* stable / preview */
if (!dryRun && !confirmed) {
  die('真发版要人当次许可：加 --confirm', [
    'AI 必须先问过，再带 --confirm 跑这一条',
    '只想看会做什么：--dry-run',
  ])
}

const pass = ['--channel', what]
if (dryRun) pass.push('--dry-run')
if (!dryRun) pass.push('--confirm')
const stopAfter = argOf('--stop-after')
if (stopAfter) pass.push('--stop-after', stopAfter)
const platform = argOf('--platform')
if (platform) pass.push('--platform', platform)
const version = argOf('--version')
if (version) pass.push(version)
const notes = argOf('--notes')
if (notes) pass.push('--notes', notes)

const report = kernel(pass)
if (!report) die('内核没有给出报告')
console.log('')
printMatrix(report)
if (report.conflicts?.length) {
  bad(`${report.conflicts.length} 处冲突 —— 这一趟停在这里`)
  for (const c of report.conflicts) info(c)
  exit(1)
}
if (report.sealBlockers?.length && report.phase === 'manifestMerged') {
  bad('封版判据没过：')
  for (const b of report.sealBlockers) info(b)
  exit(1)
}
if (stopAfter) {
  ok(`按 --stop-after ${stopAfter} 停在这里：${report.next}`)
  exit(0)
}
if (report.phase === 'sealed') {
  ok(`v${report.version} 已封版 —— 两个源的正式 Release 与清单一致`)
  exit(0)
}
bad(`还没封版（当前 ${report.phase}）：${report.next}`)
info('再跑一次同一条命令会从这一步接着做')
exit(1)

/* ---------- 打印 ---------- */

function printMatrix(r) {
  console.log(`  ${r.version} · ${r.channel === 'preview' ? '预览版' : '正式版'} · ${r.phase}`)
  if (r.sourceCommit) info(`冻结源码 ${String(r.sourceCommit).slice(0, 7)}`)
  console.log('')
  console.log('  平台            Gitee  GitHub  阶段')
  for (const p of r.platforms ?? []) {
    const g = p.giteeDone ? '齐' : '缺'
    const h = p.githubDone ? '齐' : '缺'
    console.log(`  ${p.label.padEnd(14)} ${g.padEnd(6)} ${h.padEnd(8)} ${p.stage}`)
  }
  console.log('')
  for (const line of r.summary ?? []) info(line)
}
