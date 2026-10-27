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

/*
 * ---------------- 显示格式化（2026-10-17） ----------------
 *
 * # 为什么需要它
 *
 * 参数页的值原来是**原样输出 TOML 原文**（`valueText` 直接 `raw + ' ' + unit`）。
 * 于是盘上只要有一个 `3.6999999999999997`，界面上就照原样显示 —— 作者看到的第一反应
 * 是「这些不应该是浮点数」。
 *
 * # 为什么不能全站 toFixed(2)
 *
 * 作者第二个问题问得对：「好多参数都是浮点数没问题吗？害怕改了之后都出现很多小数」。
 * 一刀切 `toFixed(2)` 会把 `70 mm/s` 变成 `70.00 mm/s`、倍率 `1` 变成 `1.00`、
 * 角度 `45` 变成 `45.00`、层数 `0` 变成 `0.00` —— 那些字段从来不是小数语义，
 * 补零等于给用户凭空造精度。
 *
 * # 所以分档（这就是"哪些显示两位、哪些不动"的答案）
 *
 *   `unit === 'mm'`          → 固定两位（补 `.00`）    `-0.50 mm` / `24.00 mm` / `3.70 mm`
 *   其它单位（mm/s ° 秒 次 层 %）→ 原样                  `70 mm/s` / `45 °`
 *   没单位（倍率 / 计数）         → 原样，只修尾巴         `1` / `0.8`
 *
 * 判据只有 `unit` 一条 —— 不去猜 valueType：配方通道根本没带 `valueType`，
 * 而 `mm` 这一档恰好就是"长度量"，也就是作者点名要看两位小数的那一批
 * （笔尖偏移 / 擦料塔 X·Y / 各种行程）。
 */

/** 固定两位小数的单位（目前只有 mm —— 见上面那段注释） */
const TWO_DECIMAL_UNITS = new Set(['mm'])

/**
 * 这个字段显示几位小数。`null` = 不补零、不截断（原样显示）。
 *
 * `step` 暂时不参与判断：`mm` 一律两位（作者要求"整数也要补 .00"），
 * 其余单位一律原样。留这个参数是为了以后某个单位要按步长出精度时有地方加。
 */
export function displayDecimals(unit: string | null | undefined, step?: number): number | null {
  void step
  if (unit == null) return null
  return TWO_DECIMAL_UNITS.has(unit.trim()) ? 2 : null
}

/**
 * 原文 → 显示字。
 *
 * `decimals === null` 时仍会**修一次浮点尾巴**（`3.6999999999999997` → `3.7`）：
 * 那个值不是用户写进去的，是某次保存把 JS 浮点加法的结果 `to_string` 下来造成的
 * （校准页 `4 - 0.3` → 过桥成 `f64` → Rust `value.to_string()`）。
 * 修尾巴只按 6 位有效小数收（不是按 step 收），够覆盖 IEEE754 的双精度噪声，
 * 又不会把 0.05 这种真精度抹掉。
 *
 * 原文不是数（`tower` / 空串 / G-code）时**原样返回** —— 这个函数不许假装懂非数字。
 */
export function formatNum(raw: FieldRaw, decimals: number | null): FieldRaw {
  if (!isNum(raw)) return raw
  const n = Number(raw)
  if (decimals !== null) return n.toFixed(decimals)
  /* 没指定小数位：只收 IEEE754 的尾巴。1e-6 这个量级既能收掉 17 位噪声，
     又动不了任何"人写进去的数"（最小的可调步长是 0.05，比它大两个数量级） */
  const trimmed = Math.round((n + Number.EPSILON) * 1e6) / 1e6
  return String(trimmed)
}
