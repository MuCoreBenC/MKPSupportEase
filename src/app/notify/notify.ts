/*
 * **全站通知系统**（2026-10-16 定案，作者提议、我补充理由）。
 *
 * # 为什么要有它
 *
 * 作者原话：「我们好像还没有通知系统，你做一个吧，就用这个更新的作为那个基础的…
 * 因为我发现我们这一个在**深色模式下的表现不太好**，它出现这种白色的…还不如
 * 大家都用一样的通知还简单一些」。
 *
 * 他说的"深色下发白"是真的，而且根因值得记下来：那颗复制 toast 的材质是照
 * 首页 `.scriptToast` 抄的**液态玻璃处方**（`rgb(255 255 255 / 45%)` 那种白叠加）
 * —— 那是**浅色的处方**，深色下当然发白。**同一套处方在两处各写了一遍、我只改了一处**。
 *
 * ⇒ 所以"统一"不只是省事，是让**主题适配只剩一个地方要改**。
 *
 * # 三条形态口径（作者 2026-10-16 定的）
 *
 * 1. **实心，不做透明**。作者：「一样的，他们可以尝试不用一点透明度，或者不用也可以，
 *    因为你看苹果，它都不用透明度」—— 参考的是 macOS 通知中心那三条：浅色下浅灰实底、
 *    深色下深灰实底，**没有毛玻璃、没有渐变透明**。
 *    ★ 实心底还有个隐性好处：`backdrop-filter` 在深色下会把底下内容糊进来、颜色不可控；
 *      实心底走 `--surface-solid`（浅 `#fff` / 深 `#303030`）**两套主题自动就对**。
 * 2. **停在窗口右上角**，像通知中心那样**从标题栏下方往下排、后到的堆在上面**
 *    （`flex-direction: column` 且新的 unshift 到前面）。
 * 3. **两类**：`ephemeral`（自己走，就是原来的 toast）与 `sticky`（一直留着，直到
 *    用户关掉或状态自己结束 —— 更新面板属于这一类：它有"暂停/取消"这种必须点得到的按钮，
 *    不能自己消失）。
 *
 * # 用法
 *
 * ```tsx
 * // ① 应用根挂一次宿主（App.tsx）
 * <NotifyHost />
 *
 * // ② 谁想说话就 push。返回一个 id，用它收回去。
 * const id = notify({ kind: 'info', text: msg('copyScript.ok.title') }, { ttl: 2200 })
 * dismiss(id)
 * ```
 *
 * ★ 文案一律给 `msg(键, 变量)`（`i18n` 的 `Text`），**别给已经翻好的字符串** ——
 *   通知活得比一次渲染长（sticky 的更新面板能挂一整趟下载），换语言时那张卡
 *   要跟着换字，只有"存键、渲染时再翻"做得到（`NotifyHost` 用 `useText()` 摊开）。
 *
 * ★ 这是一个**极小的发布订阅**（不是 React Context）：因为**通知的发起方不一定是
 * React 组件**（将来可能从 IPC 回调、从 Rust 侧事件里发），而且发起方与渲染处
 * 隔着好几层。Context 要求同树、要求 hooks 上下文；一个模块级 store + `useSyncExternalStore`
 * 两头都松。这与 `appState` / `deliveryState` 那两处的做法一致（全站同一条路子）。
 */

import { useSyncExternalStore } from 'react'

import type { Text } from '../../i18n'

/** 通知的"语气"：只决定左侧那颗点/图标的颜色，不决定形状 */
export type NotifyKind = 'info' | 'ok' | 'warn' | 'bad'

/**
 * 可选的一颗动作按钮（更新面板的"暂停 / 取消 / 重试"就是它）。
 *
 * ★ `label` 走 [`Text`]（`msg(…)` 或原样句子）：通知会**跨语言切换一直挂着**
 *   （更新面板不自动消失），存成品字符串的话换语言后它还是旧语言那一句。
 */
export interface NotifyAction {
  label: Text
  onClick: () => void
  /** 主按钮（实心强调），如"重启并安装" */
  primary?: boolean
  disabled?: boolean
}

export interface Notice {
  id: string
  kind: NotifyKind
  /** 标题（粗体那一行）。更新面板是"软件更新"，复制是"已复制" */
  title: Text
  /** 正文（可省）。更新面板是"正在下载 40.0 MB / 64.0 MB" */
  text?: Text
  /** 0~1 的进度条；给了就画一条（更新的下载态用） */
  progress?: number
  /** 动作按钮。★ 有它 ⇒ 建议用 sticky（否则按钮还没点就飘走了） */
  actions?: NotifyAction[]
  /** 右边那颗 ×（默认 true；纯提示类可以关掉） */
  closable?: boolean
  /** 秒数（`ephemeral` 才有意义）：到点自己走 */
  ttl?: number
}

let items: Notice[] = []
const listeners = new Set<() => void>()
let seq = 0

function emit() {
  for (const l of listeners) l()
}

function subscribe(fn: () => void): () => void {
  listeners.add(fn)
  return () => {
    listeners.delete(fn)
  }
}

function getSnapshot(): Notice[] {
  return items
}

/** 渲染处读它。★ `useSyncExternalStore` 要求快照**引用稳定**（下面 push/remove 都是换数组） */
export function useNotices(): Notice[] {
  return useSyncExternalStore(subscribe, getSnapshot)
}

/**
 * 发一条通知，返回它的 id（用它 `dismiss`）。
 *
 * ★ **新来的排在前面**（通知中心的做法：最新那条在最上面，老的往下推）。
 */
export function notify(input: Omit<Notice, 'id'>): string {
  seq += 1
  const id = `n${seq}`
  const ttl = input.ttl ?? (input.actions !== undefined && input.actions.length > 0 ? 0 : 2600)
  items = [{ ...input, id }, ...items]
  emit()

  /* `ttl > 0` 才自己走。★ 有动作按钮的默认 0（不自动走）——
     否则用户还没看清"重试"那颗就没了。要覆盖就显式传 `ttl`。 */
  if (ttl > 0) {
    window.setTimeout(() => dismiss(id), ttl)
  }
  return id
}

/** 同一条通知**就地改名/改进度**（更新面板每 120ms 推一次进度，不该每次新开一条） */
export function update(id: string, patch: Partial<Omit<Notice, 'id'>>) {
  const idx = items.findIndex((n) => n.id === id)
  if (idx < 0) return
  const next = items.slice()
  next[idx] = { ...next[idx], ...patch }
  items = next
  emit()
}

export function dismiss(id: string) {
  const next = items.filter((n) => n.id !== id)
  if (next.length === items.length) return
  items = next
  emit()
}

/** 全清（切页 / 卸载时用）。★ 给测试与"换一套界面"那种场景留的 */
export function dismissAll() {
  if (items.length === 0) return
  items = []
  emit()
}
