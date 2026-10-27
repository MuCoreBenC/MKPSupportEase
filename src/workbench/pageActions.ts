/*
 * 页头动作槽（2026-10-16 作者）：
 *
 * > 「这个保存啊，上面也有，下面也有，到底哪一个才是保存呢？……你得每一页都它都固定好的呀，
 * >   就用上面那一个啊，这个没必要自己再写」
 *
 * ⇒ **保存 / 放弃只有一个位置：页头右上角那一槽**。哪一页"手里有一份自己的东西要落盘"，
 *   就在它活着的时候把这一槽**接管**过去；没人接管时，外壳回落到它自己那套（参数草稿）。
 *
 * # 为什么是模块级 store 而不是 Context
 *
 * 与 `c14/toast.ts` 同一条路子：工作台里现成的跨层小状态都是模块级 store +
 * `useSyncExternalStore`。Context 在这里反而会绕：外壳把这棵子树缓存住
 * （`App.tsx` 的 `held.current`，非激活页 `hidden` + 元素引用不变 ⇒ **不重渲染**）。
 *
 * # 为什么槽里要带 `page`（这条最要命）
 *
 * 非激活页**不会重渲染**（元素被冻结复用）⇒ 它没机会注销自己那一槽。所以：
 *
 *   **外壳只认"当前页"那一槽**（`slot.page === 当前页`），不匹配的一律当没有。
 *   这样一来，切页之后那份"上一个页留下的旧登记"自然失效，不需要谁去清它；
 *   而当前页每次重渲染都会刷新自己那一槽（见 `usePageActions` 的用法）。
 *
 * # `save`/`discard` 为什么**不进**快照（踩过的坑）
 *
 * 这两样是**闭包**，每次渲染都是新的，而且**抓着当前 `marks`**。若把它们塞进"快照"里按字段
 * 去重，就会出现这种鬼：`marks` 变了但 `dirty` 没变 ⇒ 判定"没变" ⇒ 外壳手里还是**上一版闭包**
 * ⇒ 拿旧 `marks` 去存 / 去放弃。**表现为"保存了却没生效、放弃之后撤销回来的还是旧值"**。
 *
 * ⇒ 规矩：**快照只放"看得见的字"（比字段、比引用）**；`save`/`discard` 另放一处，
 *   **每次登记无条件刷新**，外壳点了按**最新那一份**跑（`run(type)`）。
 *
 * # 不做"两个真相源"
 *
 * 槽里那几样（状不脏、能不能存）**全由登记的那一页说了算** —— 外壳不自己算、不缓存、不猜。
 * 外壳只负责"摆出来 + 把点击还回去"。
 */

import { useEffect } from 'react'

/** 一页交上来的那一槽（含两个动作闭包）。除 `page` 外全由**页面**算好。 */
export interface PageActionSlot {
  /** 登记这一槽的是哪一页（外壳只认当前页那一槽，见头注） */
  page: string
  /** 未保存时的状态字（例：「有改动没写进仓库」） */
  dirtyLabel: string
  /** 已保存时的状态字（例：「已写进仓库」） */
  savedLabel: string
  /** 与基线比过之后：有没有没落盘的改动 */
  dirty: boolean
  /** 正在保存：按钮转灰、字换成 `savingLabel` */
  busy?: boolean
  /** 保存中那颗按钮上的字（缺省「保存中…」） */
  savingLabel?: string
  /** 「保存」可不可点 */
  canSave: boolean
  /** 「放弃」可不可点 */
  canDiscard: boolean
  /** 状态字的悬停说明 */
  dirtyTitle?: string
  saveTitle?: string
  discardTitle?: string
  save: () => void
  discard: () => void
}

/** 外壳看得见的那些字（**不含**动作闭包）—— `useSyncExternalStore` 比的就是这一份 */
export type PageActionView = Omit<PageActionSlot, 'save' | 'discard'>

/** 最新那两个动作（**每次登记无条件覆盖**，见头注"为什么 save/discard 不进快照"） */
let handlers: { save: () => void; discard: () => void } | null = null

let view: PageActionView | null = null
const listeners = new Set<() => void>()

function emit() {
  for (const fn of listeners) fn()
}

/** 逐字段比（只比数据，不比函数） */
function sameView(a: PageActionView, b: PageActionView): boolean {
  return (
    a.page === b.page &&
    a.dirtyLabel === b.dirtyLabel &&
    a.savedLabel === b.savedLabel &&
    a.dirty === b.dirty &&
    a.busy === b.busy &&
    a.savingLabel === b.savingLabel &&
    a.canSave === b.canSave &&
    a.canDiscard === b.canDiscard &&
    a.dirtyTitle === b.dirtyTitle &&
    a.saveTitle === b.saveTitle &&
    a.discardTitle === b.discardTitle
  )
}

export const pageActions = {
  subscribe(fn: () => void): () => void {
    listeners.add(fn)
    return () => {
      listeners.delete(fn)
    }
  },
  /** 外壳订阅的那一份（最近一次登记"看得见的字"） */
  get(): PageActionView | null {
    return view
  },
  /**
   * 登记 / 刷新。
   * ★ **动作闭包每次都换新**（无条件）；"看得见的字"变了才惊动订阅者。
   */
  register(next: PageActionSlot): void {
    /* 动作先无条件换新 —— 它抓着当前 marks，晚一步就是拿旧数据去存 */
    handlers = { save: next.save, discard: next.discard }
    const nextView: PageActionView = {
      page: next.page,
      dirtyLabel: next.dirtyLabel,
      savedLabel: next.savedLabel,
      dirty: next.dirty,
      busy: next.busy,
      savingLabel: next.savingLabel,
      canSave: next.canSave,
      canDiscard: next.canDiscard,
      dirtyTitle: next.dirtyTitle,
      saveTitle: next.saveTitle,
      discardTitle: next.discardTitle,
    }
    if (view !== null && sameView(view, nextView)) return
    view = nextView
    emit()
  },
  /** 注销**某一页**的登记（只有它自己那一槽才清得掉 —— 免得把别人的抹了） */
  unregister(page: string): void {
    if (view === null || view.page !== page) return
    view = null
    handlers = null
    emit()
  },
  /** 外壳用：当前页那一槽（不是当前页的登记一律当没有，见头注） */
  viewFor(page: string): PageActionView | null {
    return view !== null && view.page === page ? view : null
  },
  /**
   * 外壳用：按**最新那一份**跑动作。
   * ★ 不要拿 `view` 里的函数跑 —— 快照里根本没有函数（见头注那个坑）。
   */
  run(type: 'save' | 'discard'): void {
    handlers?.[type]()
  },
}

/**
 * 页面向外壳登记页头那一槽。`next` 传 `null` = 这一页不接管（外壳回落参数草稿）。
 *
 * ```tsx
 * usePageActions('annotate', {
 *   dirtyLabel: '有改动没写进仓库', savedLabel: '已写进仓库',
 *   dirty, canSave, canDiscard, save, discard,
 * })
 * ```
 *
 * ★ 登记在**每一次渲染之后**跑一遍（`useEffect` 无依赖数组）⇒ 页面 state 一变，
 *   槽里那几样立刻跟着变，动作闭包也立刻换新。
 * ★ 注销挂在**只跑一次的清理**上（`[]`）—— 页面真卸载时把这一槽交还。
 */
export function usePageActions(page: string, next: Omit<PageActionSlot, 'page'> | null): void {
  useEffect(() => {
    if (next === null) {
      pageActions.unregister(page)
      return
    }
    pageActions.register({ ...next, page })
  })
  useEffect(
    () => () => {
      pageActions.unregister(page)
    },
    [page],
  )
}
