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
import { readdirSync, statSync } from 'node:fs'
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

/* ---------- 开跑之前：**两个时间分开说**（2026-11-03 再修） ----------

原来只有一行「盘上现有：… 编译于 X」。它有两个毛病，作者都被它误导过：

1. 它读在**编译之前** —— 这一趟还没开始编，印出来的必然是上一版的时间。
   刚改完 Rust 重跑本命令时看到旧时间，正好像"没重编"；
2. 它印的是 `presets` 那个 bin 的时间（`presets build` 读的是 `current_exe()`），
   而这一趟真正要跑的是主程序 `mkp-support-ease`。两个 bin 不是每次一起重链接
   （实测 11-03：presets 14:01:56 / 主程序 13:04:10），所以"指纹上的时间"与
   "窗口里跑的那份字节"可以是两回事。

现在分两行，各说一件事，谁都不冒充谁：

```text
  本次启动：<现在>                    ← 永远是当下（它不可能过期，因为它就是此刻）
  这次要跑：workbench · 8da551d-dirty · 编译于 <主程序 mtime>
            （Rust 源码最后改于 … —— 比盘上那份新/旧 ⇒ 这一趟会不会重编）
```

编译完成（主程序 mtime 真的变了）之后再打一行 `✔ 编译完：…`，**那才是这一趟的
编译时刻**。所以以后判断"是不是这次编的"只看最后那行，不许拿「本次启动」当编译时间。

打不出来**不挡启动**：这只是一个说明，不是闸。
*/
const exe = process.platform === 'win32' ? '.exe' : ''
/* `presets` 只用来问「档位 · 提交」（主程序收到参数会直接起窗口）。
 它的 mtime 只代表它自己，**不拿它冒充主程序的时间**。 */
const probe = join(TARGET_DIR, 'debug', `presets${exe}`)
const shown = join(TARGET_DIR, 'debug', `mkp-support-ease${exe}`)

/** 中国时间的人读形状 —— 与 build.rs 印的是同一格钟（+08:00 明写）。
 *
 * 不用系统的 `date -r`：BSD 收"秒"，GNU 收"文件"，Windows 根本没有这个命令。
 * `toLocaleString('sv-SE')` 给的就是 `YYYY-MM-DD HH:MM:SS`，三平台一致。
 */
function cstTime(epochSeconds) {
  const d = epochSeconds == null ? new Date() : new Date(epochSeconds * 1000)
  const s = d.toLocaleString('sv-SE', { timeZone: 'Asia/Shanghai' })
  return `${s} +08:00`
}

/** 文件 mtime（毫秒）；读不到给 0（= "还没有这个文件"）。 */
function mtimeOf(file) {
  try {
    return statSync(file).mtimeMs
  } catch {
    return 0
  }
}

const human = (ms) => (ms ? cstTime(Math.floor(ms / 1000)) : 'unknown')

/** `src-tauri/src` + `crates` 里 `.rs` / `.toml` 的最新 mtime —— 与 build.rs 的
 `rerun-if-changed` 同一批文件：这些不动，cargo 就不会重编。 */
function newestSourceMtime() {
  let newest = 0
  const walk = (dir) => {
    let entries
    try {
      entries = readdirSync(dir, { withFileTypes: true })
    } catch {
      return
    }
    for (const entry of entries) {
      const path = join(dir, entry.name)
      if (entry.isDirectory()) {
        if (entry.name !== 'target' && entry.name !== 'node_modules') walk(path)
      } else if (/\.(rs|toml)$/.test(entry.name)) {
        newest = Math.max(newest, mtimeOf(path))
      }
    }
  }
  walk(join(REPO_ROOT, 'src-tauri', 'src'))
  walk(join(REPO_ROOT, 'crates'))
  newest = Math.max(
    newest,
    mtimeOf(join(REPO_ROOT, 'src-tauri', 'build.rs')),
    mtimeOf(join(REPO_ROOT, 'src-tauri', 'Cargo.toml')),
  )
  return newest
}

function fingerprintOf() {
  try {
    return (spawnSync(probe, ['build'], { encoding: 'utf8' }).stdout || '').trim()
  } catch {
    return ''
  }
}

const fingerprint = fingerprintOf()
const exeMs = mtimeOf(shown)
const srcMs = newestSourceMtime()

console.log(`\n  本次启动：${cstTime(null)}`)
if (fingerprint) {
  /* 「档位 · 提交」取自已编好的产物；「编译于」换成**主程序**的 mtime。 */
  const [tier, commit] = fingerprint.split(' · ')
  console.log(`  这次要跑：${tier} · ${commit} · 编译于 ${human(exeMs)}`)
} else {
  console.log(`  这次要跑：${shown}（还没编过 —— 本命令会先编一次）`)
}
const probeMs = mtimeOf(probe)
if (probeMs && exeMs && Math.abs(probeMs - exeMs) > 1000) {
  console.log(
    `  （两个 bin 不是同一次链接：presets 那份是 ${human(probeMs)}，上面这格印的是主程序的）`,
  )
}
if (srcMs && exeMs) {
  console.log(
    srcMs > exeMs + 1000
      ? `  （Rust 源码最后改于 ${human(srcMs)} —— 比盘上那份新，这一趟会重编）`
      : `  （Rust 源码最后改于 ${human(srcMs)} —— 不比盘上那份新，这一趟不重编；改前端走 HMR）`,
  )
}
console.log(`  （改 Rust 必须 Ctrl+C 后重跑本命令；改前端走 HMR 不用）`)
/* 2026-11-03：指纹里的「编译于」是**中国时间**，且 build.rs 现在登记了源码
   （改一行 Rust 就会重跑它、刷新时间戳）。所以这一格能直接跟 `git log` 的
   时间对照 —— 以前它印 UTC 且不随源码更新，作者因此被反复误判成"没重编"。 */
console.log(`  （时间是中国时区；编译完成后会再打一行「编译完」）\n`)

/**
 * ★ 2026-11-03：**编译完之后再打一次** —— 这一行才是「这一趟的编译时刻」。
 *
 * 上面那两行说的是"开跑时刻"与"盘上这份是谁"，`tauri dev` 之后才真正编译。
 * 这里等二进制 mtime 变一次（= 这次真的编出了新的），再打一行带**新时间**的。
 * 只等第一次变化，之后不再打（`tauri dev` 自己会接管输出）。
 *
 * 等不到也算一种答案：源码没动 ⇒ cargo 判定无需重编 ⇒ 明确说一句
 * 「这一趟没有重编」，别让人再猜"是不是没重启/没编译"。
 */
function announceAfterBuild() {
  /* 两个 bin 谁变都算这一趟编过（cargo 可能只重链其中一个） */
  const stamp = () => Math.max(mtimeOf(shown), mtimeOf(probe))
  const before = stamp()
  /* 最长等 10 分钟（首次全量编译可能很久），之后给结论并收手 —— 这不是闸 */
  const deadline = Date.now() + 10 * 60 * 1000
  const timer = setInterval(() => {
    const now = stamp()
    if (now !== 0 && now !== before) {
      clearInterval(timer)
      const [tier, commit] = fingerprintOf().split(' · ')
      const head = tier && commit ? `${tier} · ${commit} · ` : ''
      console.log(`\n  ✔ 编译完：${head}编译于 ${human(mtimeOf(shown))}`)
      console.log(`  （这一行才是这一趟的编译时刻；「本次启动」那行只是开跑的时刻）\n`)
      return
    }
    if (Date.now() > deadline) {
      clearInterval(timer)
      if (now === 0) {
        console.log(`\n  · 等了 10 分钟还没见到二进制 —— 要么还在编，要么这次编挂了（往上翻编译输出）\n`)
      } else {
        console.log(`\n  · 这一趟没有重编：后端字节没变，跑的就是「这次要跑」那一份`)
        console.log(`  （不是"没重启"，是 cargo 判定无需重编；改的是前端的话 HMR 已经生效）\n`)
      }
    }
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
