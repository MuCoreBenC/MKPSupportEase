/*
 * 值的唯一编解码（#33）。
 *
 * # 两层，中间只有一道门
 *
 * **数据层一律是 TOML 原文字符串**：`'true'` / `'-0.05'` / `'tower'` / 一整段 G-code。
 * 理由是字段表的真值就是 TOML 文本，gcode 与 text 本来就是字符串；统一成 string，
 * 才可能有一张字段表、一个草稿桶、一份撤销栈。
 *
 * **控件对外说强类型**：开关收/回 `boolean`，数字收/回 `number`。
 * 理由是控件内部要做步长、边界、精度、长按连发 —— 拿字符串做算术是错误的来源。
 *
 * 转换只发生在这个文件和 `FieldControl.tsx` 里。**别处一律不许再写 `=== 'true'`。**
 * #32 那会儿这件事散在四处各写一遍（真表 `'true'/'false'`、假表 `'开'/'关'`、
 * `valueText()` 里又判一次、假表的枚举干脆 value 就是 label），这一稿把它收成一处。
 */

/** 数据层的唯一形态：TOML 原文 */
export type FieldRaw = string

/**
 * 读成布尔。
 *
 * 只认 `'true'` —— 不做 `'开'` / `'1'` / `'yes'` 的兼容：多认一种写法就等于默许
 * 数据层再长出第二种形态，那正是这一稿要消灭的东西。
 */
export const boolOf = (raw: FieldRaw): boolean => raw === 'true'

export const rawOfBool = (on: boolean): FieldRaw => (on ? 'true' : 'false')

/** 小数位跟着 step 走：step=0.01 就是 2 位。别让控件自己猜精度 */
export function decimalsOf(step: number | undefined): number {
  if (step === undefined) return 0
  const tail = String(step).split('.')[1]
  return tail === undefined ? 0 : tail.length
}

export const numOf = (raw: FieldRaw): number => Number(raw)

/** 这串字符是不是一个能用的数 —— 空串、`-`、`1.2.3` 都不是 */
export const isNum = (raw: FieldRaw): boolean =>
  raw.trim() !== '' && Number.isFinite(Number(raw))

/**
 * 按 step 的精度修一次浮点尾巴再写回字符串。
 *
 * `0.1 * 3` 会得到 `0.30000000000000004`，先加 `Number.EPSILON` 再四舍五入能把它压掉。
 * 写回时不用 `toFixed`：`toFixed(2)` 会把 `15` 变成 `'15.00'`，而 TOML 里那一行写的是 `15`，
 * 无谓地制造一处「看起来改过了」。
 */
export function rawOfNum(n: number, step: number | undefined): FieldRaw {
  const digits = decimalsOf(step)
  const factor = 10 ** digits
  return String(Math.round((n + Number.EPSILON) * factor) / factor)
}
