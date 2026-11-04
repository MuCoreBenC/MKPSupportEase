/**
 * 终端上的「构建指纹」提示 —— `npm run tauri dev` 与 `npm run tauri:workbench:dev`
 * **共用这一份**（两个入口各写一套就会各说各话，见 `AGENTS.md §6.5`）。
 *
 * 它回答两个问题，各占一行，谁都不冒充谁：
 *
 * ```text
 *   本次启动：<现在>                       ← 永远是当下（它不可能过期，因为它就是此刻）
 *   这次要跑：<档位 · 提交 ·> 编译于 <exe mtime>
 *             （Rust 源码最后改于 … —— 比它新 ⇒ 这一趟会重编）
 * ```
 *
 * 编译完成（exe 的 mtime 真的变了）之后再打一行 `✔ 编译完：…`，**那才是这一趟的
 * 编译时刻**；等不到就直说「这一趟没有重编」。
 *
 * # 判据（`AGENTS.md §6.5`）
 *
 * 「算不算重新编译」只有一个标准：**主程序 exe 的 mtime 变没变**。命令行里出现
 * `Compiling …` / `Building [====] 450/452` **都不算** —— cargo 可以编到一半报错，
 * 也可以只编某个 bin 却不重链主程序，这两种情况下 exe 都还是旧的。
 *
 * # 为什么时间取 exe mtime，不取 build.rs 那一刻
 *
 * 「编译于」想回答的是"这个 exe 是何时生成的"。build.rs 跑在**链接之前**（实测差 1
 * 分钟），而且 cargo 判定"无需重建"时根本不跑它（实测差 11 小时）。所以 `lib.rs`
 * 与这里都改成读 exe 文件的 mtime —— 链接完成那一刻，不会早、也不会过期。
 */

import { spawnSync } from 'node:child_process'
import { readdirSync, statSync } from 'node:fs'
import { join } from 'node:path'

/** 中国时间的人读形状（`+08:00` 明写，形状 `YYYY-MM-DD HH:MM:SS +08:00`）。
 *
 * 不用系统的 `date -r`：BSD 收"秒"，GNU 收"文件"，Windows 根本没有这个命令。
 * `toLocaleString('sv-SE')` 给的就是 `YYYY-MM-DD HH:MM:SS`，三平台一致。
 */
export function cstTime(epochSeconds) {
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
 `rerun-if-changed` 是同一批文件：这些不动，cargo 就不会重编。 */
function newestSourceMtime(repoRoot) {
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
  walk(join(repoRoot, 'src-tauri', 'src'))
  walk(join(repoRoot, 'crates'))
  newest = Math.max(
    newest,
    mtimeOf(join(repoRoot, 'src-tauri', 'build.rs')),
    mtimeOf(join(repoRoot, 'src-tauri', 'Cargo.toml')),
  )
  return newest
}

/** 已有产物时问它一句「你是谁」（`presets build` 读的是它自己的 mtime）。
 客户端这一档编不出 `presets`（`required-features = workbench`），传 `null` 就是没有。 */
function fingerprintOf(probePath) {
  if (!probePath) return ''
  try {
    return (spawnSync(probePath, ['build'], { encoding: 'utf8' }).stdout || '').trim()
  } catch {
    return ''
  }
}

/**
 * 打印开跑前的两行，返回「等这一趟编译完之后再说一句」的函数（`spawn` 之后调）。
 *
 * @param {object} options
 * @param {string} options.repoRoot 仓库根（`node_modules`/`target` 都在它下面）
 * @param {string} options.targetDir 这一档的 cargo 产物目录（`…/target` 或 `…/target-workbench`）
 * @param {string} options.exeName 主程序文件名（带平台后缀）
 * @param {string} [options.tier] 没有探针时自己报档位（`client` / `workbench`）
 * @param {string|null} [options.probe] 探针文件名（`presets`）；没有就传 `null`
 * @param {string} options.restartNote 「改完怎么重跑」那一句（两档不一样，见下）
 */
export function announceBuild({
  repoRoot,
  targetDir,
  exeName,
  tier = '',
  probe = null,
  restartNote,
}) {
  const exePath = join(targetDir, 'debug', exeName)
  const probePath = probe ? join(targetDir, 'debug', probe) : null
  const fingerprint = fingerprintOf(probePath)
  const exeMs = mtimeOf(exePath)
  const srcMs = newestSourceMtime(repoRoot)

  console.log(`\n  本次启动：${cstTime(null)}`)
  if (fingerprint) {
    /* 「档位 · 提交」取自已编好的产物；「编译于」换成**主程序**的 mtime ——
    `presets build` 印的是它自己的，两个 bin 不是每次都一起重链接。 */
    const [tierFromExe, commit] = fingerprint.split(' · ')
    console.log(`  这次要跑：${tierFromExe} · ${commit} · 编译于 ${human(exeMs)}`)
  } else if (exeMs) {
    console.log(`  这次要跑：${tier ? `${tier} · ` : ''}编译于 ${human(exeMs)}`)
  } else {
    console.log(`  这次要跑：${exePath}（还没编过 —— 本命令会先编一次）`)
  }

  const probeMs = probePath ? mtimeOf(probePath) : 0
  if (probeMs && exeMs && Math.abs(probeMs - exeMs) > 1000) {
    console.log(
      `  （两个 bin 不是同一次链接：${probe} 那份是 ${human(probeMs)}，上面这格印的是主程序的）`,
    )
  }
  if (srcMs && exeMs) {
    console.log(
      srcMs > exeMs + 1000
        ? `  （Rust 源码最后改于 ${human(srcMs)} —— 比盘上那份新，这一趟会重编）`
        : `  （Rust 源码最后改于 ${human(srcMs)} —— 不比盘上那份新，这一趟不重编）`,
    )
  }
  console.log(`  （${restartNote}）`)
  console.log(`  （时间是中国时区；编译完成后会再打一行「编译完」）\n`)

  /* 两个 bin 谁变都算这一趟编过（cargo 可能只重链其中一个）。 */
  const stamp = () => Math.max(mtimeOf(exePath), probePath ? mtimeOf(probePath) : 0)

  return function watchForBuild() {
    const before = stamp()
    /* 源码比 exe 新 ⇒ 这一趟应该会重编（首次全量可能好几分钟）。
     不比它新 ⇒ cargo 通常根本不动手（0.x 秒就 `Finished`），这时**不能干等**：
     2026-11-04 作者就是在这个空白里问「还是没显示编译完」——
     "没有变化"本身就是一个答案，要在几秒内说出来。 */
    const expectBuild = srcMs > exeMs + 1000
    const quietAt = Date.now() + 4000
    const deadline = Date.now() + 10 * 60 * 1000
    let saidQuiet = false

    const timer = setInterval(() => {
      const now = stamp()
      if (now !== 0 && now !== before) {
        clearInterval(timer)
        const [tierNow, commitNow] = fingerprintOf(probePath).split(' · ')
        const head = tierNow && commitNow ? `${tierNow} · ${commitNow} · ` : ''
        console.log(`\n  ✔ 编译完：${head}编译于 ${human(mtimeOf(exePath))}`)
        console.log(`  （这一行才是这一趟的编译时刻；「本次启动」那行只是开跑的时刻）\n`)
        return
      }

      /* 不预期重编：4 秒内没动静就说结论（**不退出**，之后真编了还补「编译完」） */
      if (!expectBuild && !saidQuiet && Date.now() > quietAt) {
        saidQuiet = true
        console.log(`\n  · 这一趟不重编：Rust 源码没动（最后改于 ${human(srcMs)}），跑的就是上面那份`)
        console.log(`  （之后 exe 真变了的话，这里会补一行「编译完」）\n`)
        return
      }

      if (Date.now() > deadline) {
        clearInterval(timer)
        if (now === 0) {
          console.log(
            `\n  · 等了 10 分钟还没见到二进制 —— 要么还在编，要么这次编挂了（往上翻编译输出）\n`,
          )
        } else if (!saidQuiet) {
          console.log(`\n  · 这一趟没有重编：后端字节没变，跑的就是「这次要跑」那一份`)
          console.log(`  （不是"没重启"，是 cargo 判定无需重编）\n`)
        }
      }
    }, 500)
  }
}
