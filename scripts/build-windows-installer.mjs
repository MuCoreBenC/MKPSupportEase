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
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const version = JSON.parse(readFileSync(join(ROOT, 'package.json'), 'utf8')).version

if (process.platform !== 'win32') {
  console.error('安装器只有 Windows 这一档（macOS 发的是 .dmg）。')
  process.exit(1)
}

const clientExe = join(ROOT, 'target', 'release', 'mkp-support-ease.exe')
const payloadBin = join(ROOT, 'installer', 'payload.bin')
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

const cargoEnv = {
  ...process.env,
  MKP_APP_VERSION: version,
  CARGO_TARGET_DIR: installerTarget,
}

/* 2a. 卸载器：单独编一份不带 payload 的（约 0.4 MB）。
 *
 * 不这么做的话只能把安装器本体（带整个 payload）复制成 uninstall.exe，
 * 用户安装目录里就会平白多出 7 MB。 */
run('cargo', [
  'build',
  '--release',
  '--manifest-path',
  manifest,
  '--bin',
  'mkp-uninstaller',
  '--features',
  'uninstall-only',
], { env: cargoEnv })
const uninstaller = join(installerTarget, 'release', 'mkp-uninstaller.exe')
if (!existsSync(uninstaller)) {
  console.error(`没找到卸载器产物：${uninstaller}`)
  process.exit(1)
}

/* 2b. payload：极简容器（三段长度 + license + 应用 + 卸载器）再整体 xz。
 *
 * 不能先 zip 再 xz：zip 里已经是 deflate 流，LZMA 对已压缩数据没有收益
 * （实测两层压完还是 9.6 MB）。顺序反过来才对。 */
const license = readFileSync(join(ROOT, 'LICENSE'))
const appBytes = readFileSync(clientExe)
const uninstallerBytes = readFileSync(uninstaller)
const head = Buffer.alloc(12)
head.writeUInt32LE(license.length, 0)
head.writeUInt32LE(appBytes.length, 4)
head.writeUInt32LE(uninstallerBytes.length, 8)
rmSync(payloadBin, { force: true })
writeFileSync(payloadBin, Buffer.concat([head, license, appBytes, uninstallerBytes]))

run('cargo', ['build', '--release', '--manifest-path', manifest, '--bin', 'pack'], {
  env: cargoEnv,
})
rmSync(payloadXz, { force: true })
run(join(installerTarget, 'release', 'pack.exe'), [payloadBin, payloadXz])

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
