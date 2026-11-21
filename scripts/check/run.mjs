#!/usr/bin/env node
/**
 * 本地与 CI 共用的检查入口。
 *
 * 用法：
 *   node scripts/check/run.mjs web
 *   node scripts/check/run.mjs rust
 *   node scripts/check/run.mjs rust-windows
 *   node scripts/check/run.mjs build
 *   node scripts/check/run.mjs all
 *
 * 规则：
 * - 每个 job 的命令与 `.github/workflows/ci.yml` 一一对应；CI 只调这里，不再内联另一套。
 * - 同一 job 内顺序执行，失败即停；`all` 里互不依赖的 job 并行。
 * - 失败时只重跑对应 job（例如 `node scripts/check/run.mjs rust-windows`），不重跑全部。
 */

import { spawn } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { dirname, resolve } from 'node:path'

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..')
const isWin = process.platform === 'win32'
const npm = isWin ? 'npm.cmd' : 'npm'
const npx = isWin ? 'npx.cmd' : 'npx'
const base = process.env.CHECK_BASE || 'origin/main'

function run(cmd, args, extraEnv = {}) {
  return new Promise((done) => {
    console.log(`\n$ ${[cmd, ...args].join(' ')}`)
    /* Windows 上 npm.cmd / npx.cmd 不能直接 spawn（EINVAL），必须过 cmd /d /s /c；
       路径带空格的（node.exe 在 C:\Program Files）也要包引号。 */
    const quote = (s) => (/\s/.test(s) ? `"${s}"` : s)
    const child = isWin
      ? spawn('cmd.exe', ['/d', '/s', '/c', [cmd, ...args].map(quote).join(' ')], {
          cwd: ROOT,
          stdio: 'inherit',
          env: { ...process.env, ...extraEnv },
        })
      : spawn(cmd, args, {
          cwd: ROOT,
          stdio: 'inherit',
          env: { ...process.env, ...extraEnv },
        })
    child.on('error', (e) => {
      console.error(`起不来 ${cmd}：${e.message}`)
      done(1)
    })
    child.on('close', (code) => done(code ?? 1))
  })
}

async function sequence(steps) {
  for (const step of steps) {
    const code = await run(step.cmd, step.args, step.env)
    if (code !== 0) return code
  }
  return 0
}

function rustSteps() {
  return [
    { cmd: 'cargo', args: ['fmt', '--all', '--', '--check'] },
    { cmd: 'cargo', args: ['clippy', '--all-targets', '--', '-D', 'warnings'] },
    {
      cmd: 'cargo',
      args: [
        'clippy',
        '--all-targets',
        '-p',
        'mkp-support-ease',
        '--features',
        'workbench',
        '--',
        '-D',
        'warnings',
      ],
    },
    { cmd: 'cargo', args: ['test'] },
    { cmd: 'cargo', args: ['test', '-p', 'mkp-support-ease', '--features', 'workbench'] },
    { cmd: 'node', args: ['scripts/check/addressing.mjs'] },
  ]
}

function installerSteps() {
  if (!isWin) {
    console.log('  （非 Windows：安装器是 Windows 独立 crate，跳过）')
    return []
  }
  return [
    { cmd: 'cargo', args: ['check', '--manifest-path', 'installer/Cargo.toml', '--bin', 'mkp-installer'], env: { MKP_APP_VERSION: '0.0.0', CARGO_TARGET_DIR: 'target-installer' } },
    {
      cmd: 'cargo',
      args: [
        'check',
        '--manifest-path',
        'installer/Cargo.toml',
        '--bin',
        'mkp-uninstaller',
        '--features',
        'uninstall-only',
      ],
      env: { MKP_APP_VERSION: '0.0.0', CARGO_TARGET_DIR: 'target-installer' },
    },
  ]
}

const JOBS = {
  web: [
    { cmd: npm, args: ['run', 'version:check'] },
    { cmd: npm, args: ['run', 'lint'] },
    { cmd: npx, args: ['tsc', '-b'] },
    { cmd: npm, args: ['run', 'build'] },
    { cmd: npm, args: ['run', 'check:bundle'] },
    { cmd: npm, args: ['run', 'check:zero-network'] },
    { cmd: npm, args: ['run', 'check:channel-args'] },
    { cmd: npm, args: ['run', 'check:release-source'] },
    { cmd: npm, args: ['run', 'check:artifacts', '--', '--base', base] },
  ],
  rust: rustSteps(),
  'rust-windows': [...rustSteps(), ...installerSteps()],
  build: [
    { cmd: npm, args: ['run', 'build:installer'] },
    { cmd: npm, args: ['run', 'build:portable'] },
  ],
}

async function main() {
  const name = process.argv[2] || 'all'
  if (name === 'all') {
    /* rust 与 rust-windows 在同一台机器上共用 `target/`，并行会互相抢锁；
       Windows 上 rust-windows 已经包含 rust 那一套，只跑它即可。 */
    const names = isWin ? ['web', 'rust-windows'] : ['web', 'rust']
    const codes = await Promise.all(names.map(async (n) => [n, await sequence(JOBS[n])]))
    const failed = codes.filter(([, code]) => code !== 0)
    if (failed.length) {
      console.error(`\n✗ 失败：${failed.map(([n]) => n).join('、')}；只重跑失败项。`)
      process.exit(1)
    }
    console.log(`\n✓ 全部检查通过：${names.join('、')}`)
    return
  }
  if (!JOBS[name]) {
    console.error(`不认识的检查项：${name}（只认 ${Object.keys(JOBS).join(' / ')} / all）`)
    process.exit(2)
  }
  const code = await sequence(JOBS[name])
  if (code !== 0) {
    console.error(`\n✗ ${name} 没通过；修完只重跑这一项：node scripts/check/run.mjs ${name}`)
    process.exit(code)
  }
  console.log(`\n✓ ${name} 通过`)
}

main().catch((e) => {
  console.error(e?.stack ?? e)
  process.exit(1)
})
