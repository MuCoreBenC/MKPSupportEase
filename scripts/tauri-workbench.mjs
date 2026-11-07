/**
 * 工作台的 tauri 入口（`npm run tauri:workbench:dev` / `:build`）。
 *
 * 它比直接调 `tauri dev --config …` 多做一件事：**把 cargo 的产物目录分出去**
 * （`CARGO_TARGET_DIR=target-workbench`，仓库根）。
 *
 * 为什么要分：
 *
 * 1. Windows 不给覆盖正在运行的可执行文件。**dev 下两档的 exe 是同一个文件**
 *    （`target/debug/mkp-support-ease.exe` —— 名字来自 Cargo.toml 的 `default-run`；
 *    打包时才由工作台配置里的 `mainBinaryName` 另起名叫 `mkp-workbench`，dev 不走那一步）。
 *    客户端 `tauri dev` 还开着时起工作台，链接器覆盖写就被拒：
 *    `failed to remove file … 拒绝访问。(os error 5)`。
 *    macOS 允许替换正在执行的文件，所以在 Mac 上"两边同时开"看不出问题 ——
 *    那是 OS 语义，不是配置对了。
 *
 * 2. 两档的 feature 不同（工作台带 `--features workbench`），共用一份 target 时
 *    每切一次都要重编一批 crate。分开后两边各留各的指纹，来回切不再重编。
 *
 * 光分产物目录还不够：`tauri.workbench.conf.json` 里另写了 `identifier`
 * （`SupportEase-Workbench`）与 `productName`。identifier 决定 `appDataDir`
 * 与 Windows 上 WebView2 的用户数据目录（tauri 在 windows/linux 上把 webview 的
 * data_directory 指向 `app_local_data_dir()`）—— 不改它，两个窗口会抢同一个
 * WebView2 数据目录。
 *
 * 代价：第一次跑要在这份新目录里做一次完整编译（数分钟），并多占一份磁盘。
 */

import { spawn } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { join, resolve } from 'node:path'
import { announceBuild } from './build-stamp.mjs'

const REPO_ROOT = resolve(fileURLToPath(new URL('../', import.meta.url)))
const TARGET_DIR = join(REPO_ROOT, 'target-workbench')

const action = process.argv[2]
if (action !== 'dev' && action !== 'build') {
  console.error('用法：node scripts/tauri-workbench.mjs <dev|build>')
  process.exit(2)
}

// dev 加 `--no-watch`：工作台是**发版事务的执行者**，而发版要推进版本号，改的
// 4 个文件（Cargo.toml / tauri.conf.json / package.json / Cargo.lock）全在
// tauri dev 的文件监视器清单里 —— 不关监视器，版本号一落盘 dev 就重建重启，
// 发版事务被杀在半路（2026-10-06 真机踩的）。关掉监视器，工作台 dev 与发版
// 互不干扰；代价只是改 Rust 代码要手动重启（前端 HMR 走 vite，不受影响）。
// release_tx 那道 run-env 闸按祖先进程链实时探测监视器，就是读这一行命令行里的
// `--no-watch` 来放行的。
const isDev = action === 'dev'
const cmd = `tauri ${action}${isDev ? ' --no-watch' : ''} --config src-tauri/tauri.workbench.conf.json --features workbench`

/* 开跑前的指纹提示 —— 与 `npm run tauri dev` **共用一份实现**（`scripts/build-stamp.mjs`）。
 *
 * 判据是**主程序 exe 的 mtime**：命令行里出现 `Compiling …` / `Building 450/452` 都不算
 * 重编（可以编到一半报错，也可以只编某个 bin 却不重链主程序）。三行各有分工：
 * `本次启动`（永远是当下）/ `这次要跑`（盘上那份是谁 + 源码比它新还是旧）/
 * `✔ 编译完`（**这行才是本趟的编译时刻**；等不到就直说「这一趟没有重编」）。
 */
const exe = process.platform === 'win32' ? '.exe' : ''
/* `presets` 只用来问「档位 · 提交」（主程序收到参数会直接起窗口）；
 它只在 workbench 这一档编得出来，客户端那个入口没有探针。 */
/* 只有 `dev` 打这套：`build` 的产物在 `release/`，"等 debug exe 的 mtime 变"不适用。 */
const announce = isDev
  ? announceBuild({
      repoRoot: REPO_ROOT,
      targetDir: TARGET_DIR,
      exeName: `mkp-support-ease${exe}`,
      probe: `presets${exe}`,
      restartNote: '改 Rust 必须 Ctrl+C 后重跑本命令；改前端走 HMR 不用',
    })
  : () => {}

const child = spawn(cmd, {
  cwd: REPO_ROOT,
  stdio: 'inherit',
  // Windows 上 `tauri` 是 node_modules/.bin 里的 .cmd 垫片，只有走 shell 才找得到
  shell: true,
  env: { ...process.env, CARGO_TARGET_DIR: TARGET_DIR },
})
announce()

/* Ctrl+C 由子进程（tauri → cargo / vite）自己处理；父进程先退会把它们留成孤儿 */
process.on('SIGINT', () => {})

child.on('exit', (code, signal) => {
  process.exit(code ?? (signal ? 1 : 0))
})
