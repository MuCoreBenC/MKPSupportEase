/*
 * 参数搜索的命中判定 —— 纯函数，不碰 DOM。
 *
 * 三条口径，写在最前面免得以后有人「顺手改进」：
 *
 *   1. **只搜看得见的行。** 这里只负责判一行是否命中；「哪些行看得见」由
 *      bbsPanel 按当前档位（全部参数 / 跟 BBS 一样）决定。
 *      「跟 BBS 一样」档收起的那 82 行不参与 —— 否则会出现「说命中了但我找不到」。
 *      被收起的行里有多少命中，由 panel 单独数出来报在状态条上。
 *
 *   2. **值要两头都认。** 你看到的是「有机树」，json 里存的是 `tree_organic`；
 *      看到的是 `1`，链里可能存着 `1.0`；看到的是 `15 %`，值是 `15%`。
 *      所以值匹配同时拿原始值与**显示值**去比。
 *
 *   3. **纯 includes，不当正则。** 搜 `0.4` 不该被 `.` 通配成 `0X4`；
 *      搜 `[` 不该抛异常。这一页的搜索是「找那一行」，不是 grep。
 */

import { norm, truthy } from './bbsMerge'
import type { BbsParamMeta, BbsValue } from './bbsTypes'

/**
 * 命中在哪一栏 —— 决定要不要给标签加 `<mark>`，也写进行的 title。
 *
 * `label_en` 单独一档不是洁癖：界面显示的是中文标签，搜 `support` 命中的是
 * `meta.label.en`，这时**标签上没有任何一个字能圈**。把它和 `label` 混成一档的话，
 * title 会说「命中：参数名」而你在那一行上看不到标记 —— 自相矛盾。
 */
export type BbsHitWhere = 'label' | 'label_en' | 'key' | 'value'

export interface BbsHit {
  where: BbsHitWhere
}

/** 去首尾空格 + 小写。空串表示「没在搜」，调用方据此整段跳过 */
export function normalizeQuery(raw: string): string {
  return raw.trim().toLowerCase()
}

/**
 * 一个值能被搜到的所有写法。
 * 例：`grid`  → ['grid', '网格', 'Grid']
 *     `1`（布尔）→ ['开', 'on', 'true']   ← **不含 '1'**，见下
 *     `15%`  → ['15%', '15']
 *     `1.0`  → ['1.0', '1']
 *
 * 布尔为什么**不把原始值**放进来：`'1'` / `'0'` 是 BBS json 的**存储形式**，
 * 不是这一行在界面上的样子 —— 界面上它是个复选框，哪儿都没写 1。
 * 带上它的后果实测过：搜「1」会命中 63 项，里面混着所有勾上的布尔，
 * 而人想找的是「值里真的含 1 的数值行」。
 * 想看 json 原文里的 `"enable_support": "1"`，走「搜 key」那条路 ——
 * 搜 `enable_support` 命中那一行，值就在眼前。
 */
function valueForms(meta: BbsParamMeta, value: BbsValue | undefined, variantIdx: number): string[] {
  const raw = norm(value, variantIdx)
  if (raw === undefined || raw === null || raw === '') return []
  const s = String(raw)

  if (meta.type === 'bool') {
    /* 只留人话。true/false 也收：链里的布尔既有 '1' 也有 'true' 的写法 */
    const on = truthy(value, variantIdx)
    return on ? ['开', 'on', 'true'] : ['关', 'off', 'false']
  }

  const out = [s]

  /* 枚举：中英文 label 都认（界面显示中文，BBS 文档里常是英文） */
  const opt = (meta.opts ?? []).find((o) => o.value === s)
  if (opt?.label_zh) out.push(opt.label_zh)
  if (opt?.label_en) out.push(opt.label_en)

  /* 数值的两处显示加工，与 BbsControl 里那三条规则对齐 */
  const co = meta.co ?? ''
  if (/^coPercents?$/.test(co) && s.endsWith('%')) out.push(s.slice(0, -1))
  if (/^-?\d+\.\d+$/.test(s)) out.push(String(Number(s)))   // 链里存 "1.0"，界面显示 1

  return out
}

/**
 * 这一行命中了吗？命中在哪？
 * `q` 必须是 `normalizeQuery` 处理过的非空串 —— 空串的情况由调用方提前跳过，
 * 免得 249 行 × 3 次比较白跑一遍。
 *
 * 优先级 label → key → value：标签命中才有「把那几个字圈出来」这件事可做。
 */
export function matchRow(
  q: string,
  key: string,
  meta: BbsParamMeta,
  value: BbsValue | undefined,
  variantIdx = 0,
): BbsHit | null {
  const label = meta.label?.zh || meta.label?.en || key
  if (label.toLowerCase().includes(q)) return { where: 'label' }
  /* 英文标签单独一档：中文界面下搜 "layer height" 也该找到，但标签上没字可圈 */
  if (meta.label?.en && meta.label.en.toLowerCase().includes(q)) return { where: 'label_en' }
  if (key.toLowerCase().includes(q)) return { where: 'key' }
  for (const form of valueForms(meta, value, variantIdx)) {
    if (form.toLowerCase().includes(q)) return { where: 'value' }
  }
  return null
}

/**
 * 标签里命中的那一段，切成 [前, 命中, 后] 给 `<mark>` 用。
 * 没命中（或命中在 key / 值上）返回 null —— 那时标签里没有可圈的字，别乱标。
 */
export function sliceLabel(label: string, q: string): [string, string, string] | null {
  if (!q) return null
  const i = label.toLowerCase().indexOf(q)
  if (i < 0) return null
  return [label.slice(0, i), label.slice(i, i + q.length), label.slice(i + q.length)]
}

/** 命中在哪一栏，写进行的 title 那一句 */
export function hitNote(hit: BbsHit, key: string, meta?: BbsParamMeta): string {
  if (hit.where === 'key') return `命中：key ${key}`
  if (hit.where === 'value') return '命中：值'
  if (hit.where === 'label_en') return `命中：英文名 ${meta?.label?.en ?? ''}`.trim()
  return '命中：参数名'
}
