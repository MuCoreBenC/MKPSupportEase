/*
 * **草稿代次**（draft revision）：`run/` 里那份草稿被改动 / 保存 / 作废之后
 * 广播一声"草稿变了"；读它的页面（预设列表的「未保存」标记）跟着重读。
 *
 * 另有一条**单向通知**：「正文草稿被别处作废」（`textDraftDropped`，见下）——
 * 参数页保存会把「编辑文件」那份草稿作废，编辑器还开着时得按新内容重开。
 *
 * # 为什么要有它
 *
 * 页签是「常驻 + 切显示」（与 `deliveryState.ts` 同一个前提）：预设页不重挂载，
 * 而改参数的页面是参数页（或预设页上的「修改参数」模态框）—— 用户在那边改了几处、
 * 关掉，预设列表行上的「未保存」标记得**当场看得出**，不能等下次启动。
 *
 * # 它是什么 / 不是什么
 *
 * - **只是一个代次**，不存任何草稿数据 —— 数据仍由读的页面按需问
 *   （`api.getPresetDraftSummary()`），这里只回答"要不要重读"；
 * - **草稿写了什么不归它管**，它只认"有没有变过"；成功与否都发（失败也可能是
 *   上一次成功写的值还挂着，但重读一次没有代价）。
 *
 * # 谁发、谁听
 *
 *   发   参数页（`useParams`：改值 / 保存 / 取消 / 采用云端新值）、
 *        预设页（`usePresetData`：保存正文 / 放弃正文 —— 它们会作废参数草稿）
 *   听   `usePresetData`（预设列表行上的「未保存」标记）
 */

import { useSyncExternalStore } from 'react'

let revision = 0
const listeners = new Set<() => void>()

/** 草稿变了（改了值 / 保存 / 作废）—— 写完就发 */
export function draftMutated(): void {
  revision += 1
  for (const listener of listeners) listener()
}

/** 草稿代次。放进 effect 依赖里：变了就重读你自己那一路 */
export function useDraftRevision(): number {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener)
      return () => {
        listeners.delete(listener)
      }
    },
    () => revision,
  )
}

/* ---------- 「正文草稿被别处作废」（2026-10-10） ---------- */

/*
 * 参数页保存（`commitPresetDraft`）会把**正文草稿**作废（盘上字节换了，契约里
 * 写死的）。如果「编辑文件」此刻还开着同一份，它手上那份草稿就成了孤儿 ——
 * 再改一个字就撞「现在没有正在改的那一份正文」，点保存也存不上（作者实测到过）。
 *
 * 所以保存一侧**明说一声**：作废的是哪一份。还开着编辑器的那个页面（预设页）
 * 收到之后按**盘上最新内容**把编辑区重开 —— 与参数页那边"被别人保存后自己
 * 收拾"对称（那条走摘要对账，见 `useParams`）。
 */
let dropRevision = 0
let lastDroppedPath: string | null = null
const dropListeners = new Set<() => void>()

/** 正文草稿被别处作废了（参数页保存时调）。`path` = 被作废的是哪一份 */
export function textDraftDropped(path: string): void {
  lastDroppedPath = path
  dropRevision += 1
  for (const listener of dropListeners) listener()
}

/** 这一声的代次。放进 effect 依赖里 */
export function useTextDraftDrop(): number {
  return useSyncExternalStore(
    (listener) => {
      dropListeners.add(listener)
      return () => {
        dropListeners.delete(listener)
      }
    },
    () => dropRevision,
  )
}

/** 最近一次被作废的是哪一份（配合 `useTextDraftDrop` 的代次读，不要在渲染期当数据用） */
export function lastTextDraftDroppedPath(): string | null {
  return lastDroppedPath
}
