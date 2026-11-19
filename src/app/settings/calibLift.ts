/*
 * 「校准抬升高度」那一行下面的算式（2026-11-19）。
 *
 * 内核算 G-code 用的公式就是这个：
 *
 *   挂 / 收笔高度 = 首层 + 笔尖偏移 + 校准抬升高度
 *
 * 三种校准（XY / ZOffset / Repetition）共用同一个高度，所以设置页也只摆一行。
 * 三个加数里只有抬升量是这一行的值，另两个由 `useCalibLiftBase` 从盘上读；
 * 这里只剩拼字，纯函数 —— 判据见同目录的 `calibLift.test.ts`。
 */

/** 算式的首层：切片器缺省值（内核 `ir::defaults::fill_defaults` 同一档） */
export const FIRST_LAYER_MM = 0.2

/** 算式里那两个底数（抬升量本身来自偏好，不在这里） */
export interface CalibLiftBase {
  firstLayer: number
  penOffset: number
}

/** 算式里的数：最多两位小数、去掉尾零（0.20 → 0.2） */
export function numText(v: number): string {
  return String(Number(v.toFixed(2)))
}

/**
 * `0.2 + 4.1 + 10 = 14.3 mm`。
 *
 * 等号右边是**挂 / 收笔那句 `G1 Z`** 的值（喷嘴坐标），不是笔尖高度：
 * 笔尖 = 喷嘴 − 笔偏移，减掉之后净空恒等于第三个加数。
 */
export function formulaOf(base: CalibLiftBase, lift: number): string {
  const total = base.firstLayer + base.penOffset + lift
  const addends = [base.firstLayer, base.penOffset, lift].map(numText)
  return `${addends.join(' + ')} = ${numText(total)} mm`
}
