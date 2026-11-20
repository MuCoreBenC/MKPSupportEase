#!/usr/bin/env node
/**
 * 打 Windows 安装包（2026-11-20 起换成自研安装器，不再走 Tauri 的 NSIS）。
 *
 * 用法：`npm run build:installer`
 *
 * 产物（路径与以前一致，CI / 发布脚本不用改）：
 *
 * ```
 * target/release/bundle/nsis/SupportEase_<版本>_x64-setup.exe
 * ```
 *
 * # 四步
 *
 * 1. 编客户端本体（`--no-bundle`，只出 exe）；
 * 2. 把 exe + LICENSE 压成 `installer/payload.zip`（安装器 `include_bytes!` 它）；
 * 3. 编安装器（独立 crate，见 `installer/Cargo.toml` 为什么它不在根 workspace）；
 * 4. 复制成上面那个名字。
 *
 * # 为什么 payload 里带 LICENSE
 *
 * AGPL-3.0 要求分发时随附许可证正文。安装目录里的 `LICENSE` 就是那一份。
 */

import { spawnSync } from 'node:child_process'
import { copyFileSync, existsSync, mkdirSync, readFileSync, rmSync, statSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const version = JSON.parse(readFileSync(join(ROOT, 'package.json'), 'utf8')).version

if (process.platform !== 'win32') {
  console.error('安装器只有 Windows 这一档（macOS 发的是 .dmg）。')
  process.exit(1)
}

const clientExe = join(ROOT, 'target', 'release', 'mkp-support-ease.exe')
const payloadZip = join(ROOT, 'installer', 'payload.zip')
const payloadXz = join(ROOT, 'installer', 'payload.xz')
const manifest = join(ROOT, 'installer', 'Cargo.toml')
const installerTarget = join(ROOT, 'target-installer')
const installerExe = join(installerTarget, 'release', 'mkp-installer.exe')
const outDir = join(ROOT, 'target', 'release', 'bundle', 'nsis')
const out = join(outDir, `SupportEase_${version}_x64-setup.exe`)

const run = (cmd, args, opts = {}) => {
  const r = spawnSync(cmd, args, { stdio: 'inherit', cwd: ROOT, shell: false, ...opts })
  if (r.status !== 0) process.exit(r.status ?? 1)
}

/* 1. 客户端本体 */
run(process.execPath, [join(ROOT, 'scripts', 'tauri-dev.mjs'), 'build', '--no-bundle'])
if (!existsSync(clientExe)) {
  console.error(`没找到客户端产物：${clientExe}`)
  process.exit(1)
}

/* 2. payload：exe + LICENSE → zip（deflate）→ xz（LZMA） */
rmSync(payloadZip, { force: true })
run('powershell', [
  '-NoProfile',
  '-NonInteractive',
  '-Command',
  `Compress-Archive -Path '${clientExe}','${join(ROOT, 'LICENSE')}' -DestinationPath '${payloadZip}' -Force`,
])

const cargoEnv = {
  ...process.env,
  MKP_APP_VERSION: version,
  CARGO_TARGET_DIR: installerTarget,
}
run('cargo', ['build', '--release', '--manifest-path', manifest, '--bin', 'pack'], {
  env: cargoEnv,
})
rmSync(payloadXz, { force: true })
run(join(installerTarget, 'release', 'pack.exe'), [payloadZip, payloadXz])

/* 3. 安装器（它把 payload.xz 编进 exe） */
run('cargo', ['build', '--release', '--manifest-path', manifest, '--bin', 'mkp-installer'], {
  env: cargoEnv,
})
if (!existsSync(installerExe)) {
  console.error(`没找到安装器产物：${installerExe}`)
  process.exit(1)
}

/* 4. 摆到发布脚本认的那个路径 */
mkdirSync(outDir, { recursive: true })
rmSync(out, { force: true })
copyFileSync(installerExe, out)

const mb = (statSync(out).size / 1024 / 1024).toFixed(2)
console.log('')
console.log(`安装包：${out}`)
console.log(`  ${mb} MB —— 双击后走自研安装界面（安装器内嵌 payload）`)
