/*
 * 「去设置」的落点 id —— **唯一出处**。
 *
 * 每一处「去设置」都从这里取常量，业务文件里不写字符串：带上落点，设置页才会滚到
 * 那一行并把它点亮；漏了就只有页签切过去，字面对不对得上肉眼看不出来。
 *
 * ★ 类型上也让"漏带落点"过不了闸：`onOpenSettings` 只收这个联合类型，
 *   不是随便一个 string（见 `check-locate.mjs` 与 ESLint 那两道）。
 */

/** 智能塔位置那一行（`PostprocModeSection` 里那个 `<div>` 的 DOM id） */
export const SETTINGS_TARGET_TOWER = 'postproc-tower'

/**
 * 设置页「软件更新」那一块（2026-11-05，DOM id 就叫 `upd`，早就挂在那儿了）。
 * 右上角「有新版本」通知点一下就来这里 —— 与塔那格同一条滚动定位管道。
 */
export const SETTINGS_TARGET_UPDATE = 'upd'

/**
 * 设置页「强制校准模式」那一行（`CalibSection` 里那个 `<div>` 的 DOM id）。
 * 后处理弹窗里「这一盘不是校准件」那条失败跳的就是这里：关掉它才能正常后处理。
 */
export const SETTINGS_TARGET_FORCE_CALIB = 'postproc-force-calib'

/** 目前所有合法的落点。新增落点 = 在这里加一格 + 在设置页挂同名的 id */
export type SettingsTargetId =
  | typeof SETTINGS_TARGET_TOWER
  | typeof SETTINGS_TARGET_UPDATE
  | typeof SETTINGS_TARGET_FORCE_CALIB

/**
 * 失败那屏的「去设置」：错误码 → 该去改的那一行。
 *
 * ★ 判据是**码**，不是"是不是失败"：失败有很多种，只有下面列出的这些才给那颗钮
 *   ——现在只有「这一盘不是校准件」一条（关掉强制校准模式就能重跑）。
 * ★ 按码判不按文案（句子随语言变，码是契约）。表里没有的码不摆钮，也不许拿
 *   别的落点顶上去。
 */
export const FAIL_FIX_TARGETS: Partial<Record<string, SettingsTargetId>> = {
  E_CALIB_UNRECOGNIZED_001: SETTINGS_TARGET_FORCE_CALIB,
}

/**
 * 外壳交给设置页的深链落点。
 *
 * `nonce` 只用来"同一次点击重复触发也要再跑一遍"（连点两下不会因为值没变而不动）。
 */
export interface SettingsTarget {
  id: SettingsTargetId
  nonce: number
}
