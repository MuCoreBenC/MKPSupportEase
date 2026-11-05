/*
 * 「去设置」的落点 id（2026-11-04）—— **唯一出处**。
 *
 * 报告页碰撞那格的「去设置」、后处理弹窗里的「去设置」、设置页那一行自己，
 * 三处都引用这一个常量。以前是三处各写一个 `'postproc-tower'` 字符串：报告页
 * 带了落点、弹窗那颗没带（作者截图：「如果是模态框的话，它就只打开了设置，
 * 它并没有滚动定位」）—— 字面对得上不对得上，肉眼根本看不出来。
 *
 * ★ 类型上也让"漏带落点"过不了闸：`onOpenSettings` 只收这一个联合类型，
 *   不是随便一个 string（见 `check-locate.mjs` 与 ESLint 那两道）。
 */

/** 智能塔位置那一行（`PostprocModeSection` 里那个 `<div>` 的 DOM id） */
export const SETTINGS_TARGET_TOWER = 'postproc-tower'

/**
 * 设置页「软件更新」那一块（2026-11-05，DOM id 就叫 `upd`，早就挂在那儿了）。
 * 右上角「有新版本」通知点一下就来这里 —— 与塔那格同一条滚动定位管道。
 */
export const SETTINGS_TARGET_UPDATE = 'upd'

/** 目前所有合法的落点。新增落点 = 在这里加一格 + 在设置页挂同名的 id */
export type SettingsTargetId = typeof SETTINGS_TARGET_TOWER | typeof SETTINGS_TARGET_UPDATE

/**
 * 外壳交给设置页的深链落点。
 *
 * `nonce` 只用来"同一次点击重复触发也要再跑一遍"（连点两下不会因为值没变而不动）。
 */
export interface SettingsTarget {
  id: SettingsTargetId
  nonce: number
}
