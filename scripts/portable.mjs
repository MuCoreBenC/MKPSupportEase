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

/*
 * **卸载脚本**（2026-11-18）：`app-data/EBWebView` 里的 profile 文件带着 WebView2
 * 自己加的 `Everyone:(DENY)(X)`（Chromium 的防注入加固，不是我们写的）。资源管理器
 * 删到那一片会弹「你需要来自 <用户> 的权限」—— 普通用户没有 takeown 那一套，就以为
 * 删不掉。这个 cmd 替他把那几步做掉：关程序 → 收回权限 → 删程序与数据 → 自己退出后
 * 由新进程把整个文件夹清掉。
 *
 * 文件名用中文（用户双击的那一栏）；正文按 cmd 的口径落 CRLF + UTF-8，首行 chcp 65001。
 */
const uninstallCmd = [
  '@echo off',
  'chcp 65001 >nul',
  'setlocal',
  'cd /d "%~dp0"',
  '',
  'if /i "%~1"=="/y" goto :go',
  '',
  'echo.',
  'echo   SupportEase - 卸载',
  'echo   --------------------------------',
  'echo   这会删除本文件夹里的程序与它的全部数据（预设 / 执行记录 / 缓存）。',
  'echo.',
  'set /p ans="继续？输入 Y 回车："',
  'if /i not "%ans%"=="Y" goto :cancel',
  '',
  ':go',
  'echo.',
  'echo   正在关闭 SupportEase...',
  'taskkill /f /im SupportEase.exe >nul 2>&1',
  'ping 127.0.0.1 -n 2 >nul',
  '',
  'echo   正在收回 app-data 的权限...',
  'takeown /f "%~dp0app-data" /r /d y >nul 2>&1',
  'icacls "%~dp0app-data" /reset /t /c /q >nul 2>&1',
  '',
  'echo   正在删除程序与数据...',
  'rd /s /q "%~dp0app-data" >nul 2>&1',
  'del /f /q "%~dp0SupportEase.exe" >nul 2>&1',
  '',
  'echo   正在清理这个文件夹...',
  'echo.',
  'echo   完成，窗口会自己关掉。',
  'ping 127.0.0.1 -n 3 >nul',
  '',
  'for %%i in ("%~dp0.") do set "DIR=%%~fi"',
  /* 新进程不能把工作目录留在被删的那个文件夹里，否则删自己会失败 */
  'cd /d "%SystemRoot%"',
  'start "" /min powershell -NoProfile -Command "Start-Sleep -Seconds 2; Remove-Item -LiteralPath \'%DIR%\' -Recurse -Force -ErrorAction SilentlyContinue"',
  'exit /b 0',
  '',
  ':cancel',
  'echo   已取消。',
  'ping 127.0.0.1 -n 3 >nul',
  'exit /b 1',
  '',
].join('\r\n')
writeFileSync(join(stage, '卸载 SupportEase.cmd'), uninstallCmd, 'utf8')

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
