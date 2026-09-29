/*
 * C14 版式的跨页导航类型（移植自试验场 store/types 的 GotoFocus）。
 *
 * 载荷按页隔离（App 里 lastFocus 每页一份）：不同页的 uid 语义根本不同
 * （套餐页的 uid 是套餐 id、机型页的 uid 是版本 uid），串门重放等于喂错数据。
 */

/** 跨页定位载荷：落到哪台机型（哪个版本），`key` 给矩阵/参数台定位到某一项 */
export interface GotoFocus {
  machineId: string
  /** null = 机型基底 */
  uid: string | null
  /** 参数 key（跳到某一项时用） */
  key?: string | null
}

/** 一级导航的五个页面（C14 定稿：按工作流排序，不分「维护」组） */
export type PageId = 'machines' | 'params' | 'bundles' | 'build' | 'assets'
