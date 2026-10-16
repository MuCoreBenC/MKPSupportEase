/*
 * 资产清单「变了」的信号（2026-10-16）。
 *
 * # 为什么需要它
 *
 * 工作台把每页的节点**缓存住**（`App.tsx` 的 `held.current`，切走的页元素引用不变 ⇒
 * 不重渲染），所以"资产库页改了分组 → 标注台那一页还揣着旧清单"是**必然**的，
 * 不是偶发。分组正是那种"在这一页改、在另一页看"的东西（改在资产库、看在标注台），
 * 所以必须有个信号把它打醒。
 *
 * # 谁敲、谁听
 *
 * - **敲**：资产库页每一次**真的落盘了**的改动（建组 / 改组 / 删组 / 改档位 / 删资产）。
 * - **听**：标注台（它要按组摆图、按组取源稿）—— 收到就重取一次清单。
 *
 * ★ 与 `pageActions.ts` / `c14/toast.ts` 同一条路子：模块级 store + `useSyncExternalStore`。
 *   不要改成 Context —— 那些页**不重渲染**，Context 传不进去。
 */

let rev = 0
const listeners = new Set<() => void>()

export const assetsBus = {
  subscribe(fn: () => void): () => void {
    listeners.add(fn)
    return () => {
      listeners.delete(fn)
    }
  },
  /** 给 `useSyncExternalStore` 的快照（单调递增的版本号） */
  get(): number {
    return rev
  },
  /** 资产清单动过了 —— 敲一下 */
  bump(): void {
    rev += 1
    for (const fn of listeners) fn()
  },
}
