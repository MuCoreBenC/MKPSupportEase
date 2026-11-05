/*
 * **换上一份预设之后，自动把它的后处理脚本抄进剪贴板**（2026-11-01）。
 *
 * 作者原话：「我使用之后自动复制后处理脚本……我点击使用啊，或者在首页点击那个使用。
 * 反正只要变了那个使用的状态，就从一个文件变成另外一个文件的时候，就复制那一个
 * 正在使用的后处理脚本到那个剪切板，它们的后端都是一样的，就是那个复制后处理脚本的
 * 操作，执行一次需要有通知。」
 *
 * # 为什么挂在这里、不挂在每个「使用」按钮上
 *
 * 「使用」的入口有五处（预设表那一列的「应用」、首页抽屉、首页「下载并应用」、
 * 参数页换组合、校准页换组合），它们最后都收敛到同一个地方：
 * **「使用中」这一格变了**（`app/state/appState.ts` 的 `appStateMutated`）。
 * 挂在按钮上就要五处各写一遍，漏一处就是"某个入口不抄"这种最难查的 bug。
 * 所以判据只有一处：**快照里那份预设换了没有**。
 *
 * # 它不改任何产物
 *
 * 这一档只决定"要不要顺手抄一下"。抄的动作本身复用 [`copyPostProcessScript`] ——
 * 命令由后端现拼、通知话术与手动那颗「复制脚本」一字不差（同一套 `copyScript.*` 键）：
 * 用户看不出"这次是自动抄的、那次是我点的"有什么两样，本来也是一回事。
 *
 * # 它在什么时候不说话
 *
 * - **撤销使用**（下一份是 `null`）：没有"正在使用的后处理脚本"可抄；
 * - **同一份又用了一次**：没换文件就不打扰（后端 `set_active_*` 是幂等的）；
 * - **这一档读不出来**：当作关 —— 宁可少抄一次，也不凭猜的去动用户的剪贴板。
 */

import { api } from '../../api'
import type { ActivePreset } from '../../api/contract'

import { copyPostProcessScript } from './copyScript'

/**
 * 「使用中」那一份的**身份**：`origin + 落点 + 文件名`。
 *
 * ★ 不带 `sha256`：同一份文件被更新过（内容换了、还是那一份）不算"换了一份"，
 *   作者要的是「从一个文件变成另外一个文件」。
 */
export function activeIdentity(a: ActivePreset | null): string | null {
  if (a === null) return null
  return `${a.origin}\u0000${a.path ?? ''}\u0000${a.fileName}`
}

/**
 * 「使用中」刚换了 → 该抄就抄一次。
 *
 * 调用方只有 `appState.ts`（拿到新快照、与上一份比过之后的那一处）。
 * 首读不调 —— 那是"启动时把盘上的账读进来"，不是用户换了文件。
 */
export function autoCopyOnUse(prev: ActivePreset | null, next: ActivePreset | null): void {
  if (next === null) return
  const before = activeIdentity(prev)
  const after = activeIdentity(next)
  /* 撤销使用（next = null，上一行已收）与"同一份再点一次"都不动剪贴板 */
  if (before === after) return

  void api.getAutoCopyScript().then(
    (on) => {
      if (!on) return
      /*
       * 点名那一份（`path` / 机型+版本）：应用命令刚刚才把它设成使用中，
       * 这里**不再按"当前使用的那一套"去猜** —— 猜的版本在并发切换时会抄到别人身上。
       */
      void copyPostProcessScript({
        machineId: next.machineId,
        versionId: next.versionId,
        path: next.path ?? undefined,
        fileName: next.fileName,
      })
    },
    () => {
      /* 读不出这一档 = 当作关：这一条只是"顺手抄一下"，不该反过来打扰用户 */
    },
  )
}
