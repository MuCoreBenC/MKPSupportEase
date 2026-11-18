#!/usr/bin/env node
/**
 * **绿色版**（zip 解压即用）：一个 exe + 它自己旁边的 `app-data/`。
 *
 * 用法：
 *   npm run build:portable
 *
 * 产物：
 *   dist-portable/SupportEase_<版本>_x64-portable.zip
 *     └ SupportEase.exe            ← 双击就跑，不需要安装
 *
 * # 三个决定
 *
 * 1. **数据跟着 exe 走**：`--config src-tauri/tauri.portable.conf.json` 把
 *    `appDirectoriesOverride` 指到 `./app-data`（Tauri 的 `app_data/localData/cache/log`
 *    全跟着它，**连 WebView2 的 profile 一起**）。所以解压到哪个文件夹，状态 / 预设 /
 *    历史 / 缓存就都在那个文件夹里 —— 拷走整个文件夹就是完整的绿色版。
 *    ★ 别把它指到 `./`：那等于把 exe 所在目录整个交给 webview 的读写作用域（见 Tauri 配置
 *      文档的 Security 段），Windows 上有被塞 DLL 的风险。
 *
 * 2. **不打安装包**（`--no-bundle`）：安装包那条路是 `npm run tauri -- build --bundles nsis`，
 *    两条产物互不影响。
 *
 * 3. **不带 WebView2 运行时**：绿色版只有一个 exe。缺运行时的机器由 exe 自己弹一句说明并
 *    打开微软下载页（见 `src-tauri/src/webview2.rs`）—— 不替用户塞安装器。
 *
 * # 为什么走 `scripts/tauri-dev.mjs`
 *
 * 那一层设了 `CARGO_TARGET_DIR`（与 cargo 的 workspace 产物对齐），还带"工作区必须干净"
 * 的闸 —— 绿色版的包同样该对应得上一个提交。
 */

import { spawnSync } from 'node:child_process'
import { copyFileSync, existsSync, mkdirSync, readFileSync, rmSync, statSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const pkg = JSON.parse(readFileSync(join(ROOT, 'package.json'), 'utf8'))
const version = pkg.version

if (process.platform !== 'win32') {
  console.error('绿色版只做 Windows 这一档（macOS 发的是 .app，不是单文件）。')
  process.exit(1)
}

const exe = join(ROOT, 'target', 'release', 'mkp-support-ease.exe')
const outDir = join(ROOT, 'dist-portable')
const stage = join(outDir, `SupportEase_${version}_x64-portable`)
const zip = `${stage}.zip`

/* ---------- 1. 编：单 exe，配置指向 exe 旁边的 app-data ---------- */
const built = spawnSync(
  process.execPath,
  [
    join(ROOT, 'scripts', 'tauri-dev.mjs'),
    'build',
    '--no-bundle',
    '--config',
    join(ROOT, 'src-tauri', 'tauri.portable.conf.json'),
  ],
  { cwd: ROOT, stdio: 'inherit' },
)
if (built.status !== 0) process.exit(built.status ?? 1)

if (!existsSync(exe)) {
  console.error(`没找到产物：${exe}`)
  process.exit(1)
}

/* ---------- 2. 摆成一个文件夹（zip 的根就是它） ---------- */
rmSync(stage, { recursive: true, force: true })
rmSync(zip, { force: true })
mkdirSync(stage, { recursive: true })
/* 对用户叫 SupportEase.exe：文件名不参与任何判据（程序自己认的是可执行文件路径） */
copyFileSync(exe, join(stage, 'SupportEase.exe'))

/* ---------- 3. 压成 zip ---------- */
const zipped = spawnSync(
  'powershell',
  [
    '-NoProfile',
    '-NonInteractive',
    '-Command',
    `Compress-Archive -Path '${join(stage, '*')}' -DestinationPath '${zip}' -Force`,
  ],
  { stdio: 'inherit' },
)
if (zipped.status !== 0) process.exit(zipped.status ?? 1)

const size = statSync(zip).size
console.log('')
console.log(`绿色版：${zip}`)
console.log(`  ${(size / 1024 / 1024).toFixed(2)} MB —— 解压后双击 SupportEase.exe 就跑`)
console.log('  数据（run/ user/ presets-local/ caches/ logs/ 与 webview 缓存）落在它旁边的 app-data/')
