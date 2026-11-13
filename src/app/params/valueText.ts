/*
 * 一个值该显示成什么字。
 *
 * 从 `useParams.ts` 里搬出来的**纯函数**（2026-11-13）：那边是整整一页的状态，
 * 碰 `window` / 订阅 / 网络，一行都不该为了测一句格式化而吊起来。
 *
 * 三条规矩（判据本身与搬家前一致）：
 *   1. **有单位才拼单位**（`null` / `undefined` 都不算，见 `unitOf`）；
 *   2. **数字按显示精度渲染**（`formatNum`）：`mm` 固定两位，其余原样、只修浮点尾巴；
 *   3. **0 有专门含义的字段读作「自动」**，算得出就写数（`自动（6.50 mm）`）。
 * 盘上的原文一个字都不改 —— 这里只管"给我看的字"。
 */

import type { Tr } from '../../i18n'
import type { FieldSchema } from '../../components/field'
import { displayDecimals, formatNum } from '../../components/field/value'

/** 显示只需要这两格（`ParamDef` 结构上就满足，不必反向依赖那边那一大坨状态） */
export interface TextDef {
  unit?: string
  field: FieldSchema
}

/**
 * 单位字段的写法归一：**长得像区间的 unit 当成没有单位**。
 *
 * 真注册表里 `unit` 不是每次都写单位 —— 风扇速度那条写的是 `"unit": "0-255"`
 * （同一个对象的 min 0 / max 255 就在旁边），也就是 **min/max 被塞进了 unit 位**。
 * 照原样渲染出来是三处一样的话：
 *
 *   行里的步进器   `250 0-255`    —— 值后面跟着一个假单位
 *   行里的还原胶囊 `已保存的 255 0-255`
 *   保存确认       `255 0-255 → 250 0-255`
 *
 * 作者的原话：「不要显示区间 0-255 这种」。所以这里只认这一种脏数据，把它当**没有单位**：
 *
 *   · 范围一点没丢 —— 它在 min/max 上，行展开的「取值范围」写的是 `0 ~ 255`，
 *     步进器也照样按 0/255 夹值（`FieldControl` 已经把 min/max 递下去了）
 *   · 真单位（mm / mm/s / 秒 / %）一律照原样显示，**不在这里发明第二种规则** ——
 *     全仓库只有这一条 unit 是区间形状（实测 grep：`"unit": "<数字>-<数字>"` 命中 1 条）
 */
const RANGE_LIKE_UNIT = /^\s*\d+(?:\.\d+)?\s*[-~～—–]\s*\d+(?:\.\d+)?\s*$/

/**
 * 单位归一：`null` 与 `undefined` **同义 = 没有单位**。
 *
 * ★ 2026-10-17：「擦料塔挤出倍率」曾显示成 `1 null`。注册表里那一条根本没有 `unit`
 *   字段，但配方/定义通道过来是 JSON 的 `null` —— 而下游判的是 `!== undefined`，
 *   `null` 就那么一字不改地拼进了字符串。这不是脏数据，是**两种"没有"**。
 *   所以这里把 `null` 也吞掉，往后的类型只有 `string | undefined` 一种缺失形态。
 */
export function unitOf(raw: string | null | undefined): string | undefined {
  if (raw == null) return undefined
  return RANGE_LIKE_UNIT.test(raw) ? undefined : raw
}

/**
 * 「自动」那一格现在等于多少（算不出来就是 `null`）。
 *
 * 算法登记在注册表的 `autoValue`（跟着某条参数的当前值走），**不按 key 写死** ——
 * 界面只负责把登记的那几个动作拼起来：取不取绝对值、再加多少。
 */
function autoNumber(def: TextDef, lookup: ((key: string) => string) | undefined): number | null {
  const link = def.field.autoValue
  if (link === undefined || lookup === undefined) return null
  const raw = lookup(link.from).trim()
  if (raw === '') return null
  const base = Number(raw)
  if (!Number.isFinite(base)) return null
  return (link.abs === true ? Math.abs(base) : base) + (link.offset ?? 0)
}

/**
 * 某个值该显示成什么字 —— 还原 chip、历史条目、保存确认、行内只读值全走这一套翻译。
 *
 * `lookup` 只在**说的是"现在"**的场合给（行上那一格、只读值）：给得出就算出
 * 「自动（6.50 mm）」，给不出 / 说的是历史值时只写「自动」—— 历史那一刻的数
 * 不能拿今天的来源值顶。
 */
export function valueText(
  def: TextDef | undefined,
  raw: string,
  tr: Tr,
  lookup?: (key: string) => string,
): string {
  if (raw === '') return tr('params.value.empty')
  if (def === undefined) return raw
  /* 0 有专门含义的字段：读作「自动」而不是 `0.00`（否则等于告诉用户"不挤出"） */
  if (def.field.zeroMeansAuto === true && raw.trim() !== '' && Number(raw) === 0) {
    const n = autoNumber(def, lookup)
    if (n === null) return tr('params.row.auto')
    const text = formatNum(String(n), displayDecimals(def.unit, def.field.step))
    return tr('params.row.autoWith', { value: def.unit != null ? `${text} ${def.unit}` : text })
  }
  if (def.field.control === 'switch') {
    return raw === 'true' ? tr('params.value.on') : tr('params.value.off')
  }
  if (def.field.control === 'choice') {
    return def.field.choices?.find((c) => c.value === raw)?.label ?? raw
  }
  if (def.field.control === 'gcode') {
    return tr('params.value.lines', { count: raw.split('\n').length })
  }
  /* 数字按显示精度走；非数字（`tower` / G-code 之类）`formatNum` 原样退回 */
  const text = formatNum(raw, displayDecimals(def.unit, def.field.step))
  return def.unit != null ? `${text} ${def.unit}` : text
}
