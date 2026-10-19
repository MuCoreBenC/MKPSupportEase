/*
 * **「复制后处理脚本」全站唯一实现**（2026-10-16）。
 *
 * # 为什么要有这个文件
 *
 * 作者：「复制的好几处有，是不是统一的管理实现的」—— 收这一刀之前**不是**。
 * 它散在三处、各写一遍：首页那颗 `CopyAction`、首页四个步骤底部那颗「复制脚本」、
 * 标题栏 `MKP` 字标 —— 三处各自调 `api.getPostProcessCommand` +
 * `navigator.clipboard.writeText` + 各说各的一句话。
 *
 * ⇒ 与 `notify` 那一刀是**同一类病**：同一件事写三遍 ⇒ 话术和主题适配各漏各的
 *   （MEMORY 有案：复制 toast 那套材质被抄了两遍、只改了一处，深色下就发白）。
 *   现在"向后端现要命令"与"复制 + 通知"都只有这一个出口。
 *
 * # 为什么"取命令"必须只走这一个出口
 *
 * 命令由**后端现拼**（`getPostProcessCommand`）：可执行物 = 本程序自己（`current_exe()`），
 * 预设 = 那台机器上真实的绝对落点。**前端不拼任何一段路径、也不长期缓存命令** ——
 * 早先首页硬编码的是**另一个仓库**的 Windows exe，Mac 上复制出去的命令必然跑不通
 * （贴进切片器就是 `Error code: 2`）。
 * 每次点都现要一次：下载 / 应用 / 换份都可能把指向换掉。
 *
 * # 反馈：通知里必须说清"哪一份"
 *
 * 用户可能在几分钟里连着复制两三份预设 —— 一条只写「已复制」的提示回答不了
 * "我刚才抄的是哪一份"。所以通知正文**带文件名**（作者 2026-10-16：
 * 「右键的复制也可以加上通知，哪一份预设文件的」）。
 * ★ 三处（右键 / 字标 / 首页）同一个说法、同一个停留时长 —— 见 `COPY_TTL_MS`。
 *
 * # 架构
 *
 * 前后端分离：前端只认 `src/api/contract.ts` 那一份契约，`src/api/` 里不许 fetch；
 * 网络只住 Rust 的 `runtime/net.rs`。所以"命令长什么样"**只能是后端答的**，
 * 这个文件负责的是"什么时候要、要到了怎么进剪贴板、然后怎么告诉用户"。
 */

import { api, errorText } from '../../api'
import type { PostProcessCommand } from '../../api'
/* ★ 通知系统没有 index（`src/app/notify/notify.ts`），全站都按这个文件引 */
import { notify } from '../notify/notify'

/**
 * 这一条链路上**所有**通知的停留时长（毫秒）—— 成功、抄不了、失败，一律这个数。
 *
 * ★ 比默认那条（2600）**长一倍**（作者：「通知可以停留长一点」）。
 * ★★ 为什么**失败也算长**：`reason` 与错误原文都不短（"浏览器预览里没有本机的
 *   可执行物与预设文件 —— 命令由桌面版按真实路径现拼"），2.6 秒根本读不完 ——
 *   读不完的提示等于没提示。只把成功那条调长是半件事。
 * ★ 但**不用 `sticky`（0）**：这三类都只是"告诉你发生了什么"，**没有要你点按钮**，
 *   该自己走 —— 常驻不走的提示堆在右上角，久了就没人看了。
 */
const COPY_TTL_MS = 5200

export interface CopyScriptTarget {
  machineId: string
  versionId: string
  /**
   * **被点名的那一份**在用户根里的相对落点（`presets-local/…`）—— 预设页右键那一行传它。
   *
   * ★ 为什么必须传（2026-10-19）：只按 (机型, 版本) 寻址时，同一组合下有几份副本
   *   （`A1-fast.toml` / `A1-fastv3.3 copy.toml` …）后端会优先挑「正在使用」的那份
   *   ⇒ 右键 A 抄到 B，通知里却写着 A（作者实测）。给了落点就**点名**，不再猜。
   * 不传 = "当前使用的那一套"（首页 / 标题栏字标那一侧）。
   */
  path?: string
  /**
   * 这一份预设文件的名字（通知里要说"哪一份"）。
   * 不给就从命令带回的绝对路径末段取（`cmd.toml`）—— 两条路都能落到同一个名字。
   */
  fileName?: string
}

/** 取绝对路径的末段。`\` 与 `/` 都要认（Windows / macOS 是两个落点） */
function baseName(p: string): string {
  const i = Math.max(p.lastIndexOf('/'), p.lastIndexOf('\\'))
  return i < 0 ? p : p.slice(i + 1)
}

/**
 * **向后端现要一次**那一整条命令（不缓存、不拼路径）。
 *
 * 首页那颗 `CopyAction` 要自己决定怎么反馈（它有自己的"按钮就地变已复制"），
 * 所以它用这个出口，不走下面那条带通知的。
 */
export function postProcessCommand(
  machineId: string,
  versionId: string,
  path?: string,
): Promise<PostProcessCommand> {
  return api.getPostProcessCommand(machineId, versionId, path)
}

/**
 * 复制后处理脚本 + 发一条通知。**返回有没有真抄到**。
 *
 * 每一种"抄不了"都**照实说、说不抄假命令**（`ready: false` 时后端给的是 `reason`，
 * 不是一条跑不通的命令）—— 贴进切片器只会得到 `Error code: 2`，那比不复制更糟。
 */
export async function copyPostProcessScript(t: CopyScriptTarget): Promise<boolean> {
  /*
   * 两条寻址方式，有一条能指到东西就行：
   * - 点名（`path`）：右键那一行，落点自己就说明是哪一份 ⇒ 不需要归属；
   * - 不点名：首页 / 字标按 (机型, 版本) —— 那两样缺一个就指不出来。
   */
  const named = t.path !== undefined && t.path !== ''
  if (!named && (t.machineId === '' || t.versionId === '')) {
    notify({
      kind: 'warn',
      title: '复制不了',
      text: '先下载并应用一份预设，才有可复制的脚本',
      ttl: COPY_TTL_MS,
    })
    return false
  }

  let cmd: PostProcessCommand
  try {
    cmd = await postProcessCommand(t.machineId, t.versionId, t.path)
  } catch (err: unknown) {
    /* 错误唯一入口是 `errorText`，不许静默"什么都没发生" */
    notify({ kind: 'bad', title: '复制失败', text: errorText(err), ttl: COPY_TTL_MS })
    return false
  }

  if (!cmd.ready || cmd.command === undefined) {
    notify({
      kind: 'warn',
      title: '复制不了',
      text: cmd.reason ?? '这条命令现在还抄不了',
      ttl: COPY_TTL_MS,
    })
    return false
  }

  try {
    await navigator.clipboard.writeText(cmd.command)
  } catch (err: unknown) {
    notify({ kind: 'bad', title: '复制失败', text: errorText(err), ttl: COPY_TTL_MS })
    return false
  }

  const name = t.fileName ?? (cmd.toml === undefined ? null : baseName(cmd.toml))
  notify({
    kind: 'ok',
    title: '已复制后处理脚本',
    /* 文件名在最前面 —— 一眼先看到"哪一份"，后半句才是"去哪儿粘" */
    text: name === null ? '去切片器的后处理栏粘贴' : `${name} —— 去切片器的后处理栏粘贴`,
    ttl: COPY_TTL_MS,
  })
  return true
}
