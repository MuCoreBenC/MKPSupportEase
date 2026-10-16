/*
 * C14 版式的跨页导航类型（移植自试验场 store/types 的 GotoFocus）。
 *
 * 载荷按页隔离（App 里 lastFocus 每页一份）：不同页的 uid 语义根本不同
 * （套餐页的 uid 是套餐 id、机型页的 uid 是版本 uid），串门重放等于喂错数据。
 */

/** 跨页定位载荷：落到哪台机型（哪个版本），`key` 给矩阵/参数台定位到某一项 */
export interface GotoFocus {
  /** 套餐 / 资产页的跳转没有机型语境 —— 那就带着 null 过去（C14 同形） */
  machineId: string | null
  /** null = 机型基底 */
  uid: string | null
  /** 参数 key（跳到某一项时用） */
  key?: string | null
}

/**
 * 一级导航的页面（C14 定稿：按工作流排序，不分「维护」组）。
 *
 * `settings` 排在末尾：它不是一个业务页 —— 预设根固定是 `<repo>/presets`，
 * 这一页只读地摆开数据根与子目录职责，供排查「读错了目录」用。
 *
 * `dev`（开发台）排在 `settings` 之后：同样不是业务页 —— 把"平时看不见的那几屏"
 * 做成按钮（作者 2026-10-14）。只在 workbench 构建里存在（正式包编不到这一页）。
 *
 * `annotate`（标注台）跟在资产库后面：它管的正是**资产里那些图**（今天主要是引导图
 * `type = 'guide'`）—— 作者 2026-10-16：「你要不做一个小工具吧，在工作台里面……
 * 让我可以去自己拖动框框」。见 `views/AnnotatePage.tsx` 的头注。
 */
export type PageId =
  | 'machines'
  | 'params'
  | 'bundles'
  | 'build'
  | 'assets'
  | 'annotate'
  | 'settings'
  | 'dev'
