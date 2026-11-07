/**
 * 客户端这一档的 tauri 入口（`npm run tauri dev` / `build` / `info` … 全部透传）。
 *
 * 它只多做一件事：开跑前把「这次跑的是哪一次编译出来的」打到终端上
 * —— 与工作台共用同一份实现（`scripts/build-stamp.mjs`），两处不会各说各话。
 *
 * 为什么需要这一层：`package.json` 原来是 `"tauri": "tauri"`，直接 exec，
 * **一句话都不打**。指纹只落在 `tracing::info!` 的日志文件里
 * （`<appDataDir>/logs/supportease.log.<日期>`），于是终端上看不到任何时间
 * ——「这次到底重编了没」又回到靠猜。
 *
 * ★ 两处与工作台不同：
 *
 * 1. **没有 `presets` 探针**：那个 bin 挂着 `required-features = workbench`，
 *    客户端这一档根本编不出来。所以「提交」不在这里印（档位写死 `client`），
 *    要看完整指纹就看启动日志第一行（`构建指纹：client · <sha> · 编译于 …`）。
 * 2. **不设 `CARGO_TARGET_DIR`**：客户端用的就是 `src-tauri/target` 那一份产物。
 */

import { execFileSync, spawn } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { join, resolve } from 'node:path'
import { announceBuild } from './build-stamp.mjs'

const REPO_ROOT = resolve(fileURLToPath(new URL('../', import.meta.url)))

/* ★ **必须显式钉死**（2026-11-04 查出来的真问题，不是美观问题）。
 *
 * 这个仓库的根 `Cargo.toml` 是 workspace，`src-tauri` 是成员，所以 cargo 的
 * `target_directory` 是**仓库根**的 `target/`（`cargo metadata` 亲口这么说）：
 *
 * ```text
 * <repo>/target/debug/mkp-support-ease           ← cargo 真编出来的（11-04 03:04）
 * <repo>/src-tauri/target/debug/mkp-support-ease ← tauri 实际 exec 的（09-21）
 * ```
 *
 * 两边差了一个半月。tauri CLI 在没设 `CARGO_TARGET_DIR` 时按老路径去
 * `src-tauri/target` 找，于是**客户端这一档跑的永远是那个 9 月的僵尸文件** ——
 * cargo 编得再久也白编（这也是"改了没反应"的底层原因之一）。
 *
 * 显式给同一个目录，cargo 与 tauri 才看同一份产物（工作台那档一直是对的，
 * 因为它本来就设了 `CARGO_TARGET_DIR=target-workbench`）。
 */
const TARGET_DIR = join(REPO_ROOT, 'target')

const args = process.argv.slice(2)
if (args.length === 0) {
  console.error('用法：node scripts/tauri-dev.mjs <dev|build|info|…>')
  process.exit(2)
}

const exe = process.platform === 'win32' ? '.exe' : ''

/* ---------- `build` 开工前的两道闸（`dev` 不拦：开发期工作区本来就该是脏的） ----------
 *
 * ① **盘上那份二进制是谁编的。** 两档共用同一个 crate 与同一个 `default-run`，工作台
 *    那份配置把 `app.windows` 写成空数组（它只开工作台那一页）。它的产物一旦顶掉
 *    `target/release/mkp-support-ease`，`tauri build` 会原样复用它去封包 —— 外壳照客户端
 *    配置写（名字、版本、图标都对），内核没有窗口：装上去进程起得来，就是没有窗。
 *    工作台配置现在用 `mainBinaryName` 把自己那份改名 `mkp-workbench`，物理上不再同名；
 *    这道闸留给盘上已经躺着的旧产物（cargo 会原样复用，不重编）。
 *
 * ② **工作区。** 编出来的包要发出去。脏树编出来的二进制不对应任何提交，启动指纹带
 *    `-dirty`，发出去之后没法复现、也没法按提交还原。版本派生那四个文件除外 ——
 *    `version:ensure-preview` 抬号时它们本来就会脏，那是这一趟要发的内容。
 */
const WORKBENCH_MARK = 'SupportEase-Workbench'
const VERSION_DERIVED = [
  'src-tauri/Cargo.toml',
  'src-tauri/tauri.conf.json',
  'package.json',
  'Cargo.lock',
]
const CLIENT_BIN = join(TARGET_DIR, 'release', `mkp-support-ease${exe}`)

/** 这一趟不该开工的理由；能开工就是 `null`。 */
function refusalToBuild() {
  try {
    if (readFileSync(CLIENT_BIN).includes(WORKBENCH_MARK)) {
      return (
        `盘上的 ${CLIENT_BIN}\n` +
        '  是拿工作台配置编出来的（内核里窗口清单是空的），cargo 会原样复用它去封包。\n' +
        `  删掉它再跑：rm "${CLIENT_BIN}"`
      )
    }
  } catch {
    /* 还没编过：没有可复用的东西 */
  }

  let status
  try {
    status = execFileSync('git', ['status', '--porcelain'], {
      cwd: REPO_ROOT,
      encoding: 'utf8',
    })
  } catch (e) {
    return `问不了工作区状态（git 起不来）：${e.message}`
  }
  const dirty = status
    .split('\n')
    .map((line) => line.trimEnd())
    .filter(Boolean)
    .filter((line) => {
      const path = line.slice(3).split(' -> ').pop()
      return !VERSION_DERIVED.includes(path)
    })
  if (dirty.length > 0) {
    return (
      `工作区不干净（${dirty.length} 处）—— 编出来的包对应不上任何提交：\n` +
      dirty.map((line) => `    ${line}`).join('\n') +
      '\n  先提交（或确认这些改动就是要发的），再来编。'
    )
  }
  return null
}

if (args[0] === 'build') {
  const why = refusalToBuild()
  if (why !== null) {
    console.error(`\n✗ 不开工：${why}\n`)
    process.exit(1)
  }
}

/* 只有 `dev` 打这套：`build` 的产物在 `release/`，"等 debug exe 的 mtime 变"不适用。 */
const announce =
  args[0] === 'dev'
    ? announceBuild({
        repoRoot: REPO_ROOT,
        targetDir: TARGET_DIR,
        exeName: `mkp-support-ease${exe}`,
        tier: 'client',
        probe: null,
        restartNote: 'tauri dev 自带文件监视：改 Rust 会自己重启，不用手动停',
      })
    : () => {}

const child = spawn(`tauri ${args.join(' ')}`, {
  cwd: REPO_ROOT,
  stdio: 'inherit',
  // Windows 上 `tauri` 是 node_modules/.bin 里的 .cmd 垫片，只有走 shell 才找得到
  shell: true,
  // 与 cargo 的 target_directory 对齐（见上面那段）；tauri 会把 exe 也去这里找
  env: { ...process.env, CARGO_TARGET_DIR: TARGET_DIR },
})
announce()

/* Ctrl+C 由子进程（tauri → cargo / vite）自己处理；父进程先退会把它们留成孤儿 */
process.on('SIGINT', () => {})

child.on('exit', (code, signal) => {
  /* 兜底：开工前那道闸拦的是"沿用旧产物"，这一道拦的是"这一趟真编出来的东西带工作台
  标记"（主配置被谁改脏了）。编完了才发现，也比封成包发出去强。 */
  if (code === 0 && args[0] === 'build') {
    try {
      if (readFileSync(CLIENT_BIN).includes(WORKBENCH_MARK)) {
        console.error(
          '\n✗ 刚编出来的这颗二进制带工作台配置的标记 —— 不要发它。' +
            '检查 src-tauri/tauri.conf.json 的 app.windows 还在不在\n',
        )
        process.exit(1)
      }
    } catch {
      /* 读不到产物就不在这里判（`--no-bundle` 之外的失败已由退出码带出去了） */
    }
  }
  process.exit(code ?? (signal ? 1 : 0))
})
