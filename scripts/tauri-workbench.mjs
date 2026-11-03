/**
 * 工作台的 tauri 入口（`npm run tauri:workbench:dev` / `:build`）。
 *
 * 它比直接调 `tauri dev --config …` 多做一件事：**把 cargo 的产物目录分出去**
 * （`CARGO_TARGET_DIR=target-workbench`，仓库根）。
 *
 * 为什么要分：
 *
 * 1. Windows 不给覆盖正在运行的可执行文件。工作台与客户端的 exe 是**同一个文件**
 *    （`target/debug/mkp-support-ease.exe` —— 名字来自 Cargo.toml 的 `default-run`，
 *    不是 productName，两档都一样）。客户端 `tauri dev` 还开着时起工作台，链接器
 *    覆盖写就被拒：`failed to remove file … 拒绝访问。(os error 5)`。
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

import { spawn, spawnSync } from 'node:child_process'
import { statSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { join, resolve } from 'node:path'

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

/* ★ 开跑之前先把**当前二进制**的构建指纹打出来（2026-11-02）。
 *
 * 为什么：`--no-watch` 决定了改 Rust 要手动重启；而「重启了但没有重编」
 * 与「重编了但没有重启」在界面上长得一模一样。以前这件事只能靠猜，
 * 于是反复出现「你是不是没重新编译后端」这种没有证据的互疑。
 *
 * 指纹由 build.rs 在**编译那一刻**刻进二进制（`MKPSE_BUILD_TIME` /
 * `MKPSE_GIT_COMMIT` / `MKPSE_BUILD_TIER`），所以它说的是「这个 exe 是谁」，
 * 不是「源码现在是什么」—— 正好是我们要问的那一问。
 *
 * 打不出来**不挡启动**：这只是一个说明，不是闸。
 */
const exe = process.platform === 'win32' ? '.exe' : ''
/* ★ 用 `presets build` 而不是主程序：主程序会忽略参数直接起窗口。
 两个二进制**同一次 cargo 编译产出**，所以指纹一致。 */
const probe = join(TARGET_DIR, 'debug', `presets${exe}`)
const shown = join(TARGET_DIR, 'debug', `mkp-support-ease${exe}`)
let fingerprint = ''
try {
  fingerprint = (spawnSync(probe, ['build'], { encoding: 'utf8' }).stdout || '').trim()
} catch {
  fingerprint = ''
}
if (fingerprint) {
  console.log(`\n  盘上现有：${fingerprint}`)
} else {
  console.log(`\n  盘上现有：${shown}（还没编过 —— 本命令会先编一次）`)
}
console.log(`  （改 Rust 必须 Ctrl+C 后重跑本命令；改前端走 HMR 不用）`)
/* 2026-11-03：指纹里的「编译于」是**中国时间**，且 build.rs 现在登记了源码
   （改一行 Rust 就会重跑它、刷新时间戳）。所以这一格能直接跟 `git log` 的
   时间对照 —— 以前它印 UTC 且不随源码更新，作者因此被反复误判成"没重编"。 */
console.log(`  （时间是中国时区；编译完成后会再打一行「编译完」）\n`)

/**
 * ★ 2026-11-03：**编译之后再打一次指纹**。
 *
 * 上面那一行说的是"盘上现在这个 exe 是谁"，而 `tauri dev` 之后才真正编译 ——
 * 于是刚改完 Rust 重跑本命令时，作者看到的是**上一版**的时间，
 * 反而更像"没重编"（作者实拍：命令行里明明在 `Compiling`，
 * 上面那行还印着 11-02）。
 *
 * 这里等主程序二进制的 mtime 变一次（= 这次真的编出了新的），再读一眼指纹。
 * 只等第一次变化，之后不再打（`tauri dev` 自己会接管输出）。
 */
function announceAfterBuild() {
  const stamp = () => {
    try {
      return statSync(shown).mtimeMs
    } catch {
      return 0
    }
  }
  const before = stamp()
  /* 最长等 10 分钟（首次全量编译可能很久），之后安静放弃 —— 这不是闸 */
  const deadline = Date.now() + 10 * 60 * 1000
  const timer = setInterval(() => {
    const now = stamp()
    if (now !== 0 && now !== before) {
      clearInterval(timer)
      try {
        const after = (spawnSync(probe, ['build'], { encoding: 'utf8' }).stdout || '').trim()
        if (after) console.log(`\n  ✔ 编译完：${after}\n`)
      } catch {
        /* 读不到就安静 —— 上面那行已经说了盘上是什么 */
      }
      return
    }
    if (Date.now() > deadline) clearInterval(timer)
  }, 500)
}

const child = spawn(cmd, {
  cwd: REPO_ROOT,
  stdio: 'inherit',
  // Windows 上 `tauri` 是 node_modules/.bin 里的 .cmd 垫片，只有走 shell 才找得到
  shell: true,
  env: { ...process.env, CARGO_TARGET_DIR: TARGET_DIR },
})
announceAfterBuild()

/* Ctrl+C 由子进程（tauri → cargo / vite）自己处理；父进程先退会把它们留成孤儿 */
process.on('SIGINT', () => {})

child.on('exit', (code, signal) => {
  process.exit(code ?? (signal ? 1 : 0))
})
