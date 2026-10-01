/*
 * support_style 的两件特殊对待 —— 译自上游 bbs-preset.js 的 SUPPORT_STYLE_SETS / optsFor /
 * fixSupportStyle 三段。纯函数。
 *
 * support_style 在 BBS 里是唯一一个「被特殊对待」的图案类枚举，两条都出自源码：
 *   ① 候选集按 support_type 现算 —— Tab.cpp:3610-3612
 *        enum_set_normal = {default, grid, snug}
 *        enum_set_tree   = {default, tree_slim, tree_strong, tree_hybrid, tree_organic}
 *   ② 重建时走 cb->Append(_(label))，没有 bmp 参数 —— Tab.cpp:3622。
 *      对比通用枚举的 Field.cpp:1224 是 Append(_(el), bm.bmp())。
 *      所以这一行的「网格」没有小图标，哪怕 resources/images/param_grid.svg 确实存在
 *      （param_grid 那张图只出现在稀疏填充图案、支撑面图案这些真正带图的枚举里）。
 *
 * registry.json 是上游提取器的产物、标了「别手改」，所以裁剪放在渲染层。
 */

import { norm } from './bbsMerge'
import type { BbsOpt, BbsParamMeta, BbsValues } from './bbsTypes'

const SUPPORT_STYLE_SETS: Record<'normal' | 'tree', string[]> = {
  normal: ['default', 'grid', 'snug'],
  tree: ['default', 'tree_slim', 'tree_strong', 'tree_hybrid', 'tree_organic'],
}

export function supportStyleSet(values: BbsValues, variantIdx = 0): string[] {
  const type = String(norm(values.support_type, variantIdx) ?? '')
  return SUPPORT_STYLE_SETS[type.startsWith('tree') ? 'tree' : 'normal']
}

/**
 * 一个 key 在当前值下的候选项。除 support_style 外原样返回 registry 里那份。
 * support_style 按 BBS 的 enum_set_* 重排并**剥掉图标**（见文件头 ②）。
 */
export function optsFor(key: string, meta: BbsParamMeta, values: BbsValues, variantIdx = 0): BbsOpt[] {
  const opts = meta.opts || []
  if (key !== 'support_style') return opts
  return supportStyleSet(values, variantIdx)
    .map((v) => opts.find((o) => o.value === v))       // 顺序照 BBS 的 enum_set_*，不按 registry 原序
    .filter((o): o is BbsOpt => Boolean(o))
    .map((o) => ({ ...o, icon: null }))                // 图标一律剥掉
}

/**
 * 非法组合的兜底：BBS 在 ConfigManipulation.cpp:658-668 会把不在当前候选集里的
 * support_style 写回 default，所以「普通 + 有机树」在 BBS 里是不存在的状态 ——
 * 一份预设里真带了，BBS 打开那一刻就已经改成默认。我们跟着改（改的是 userDoc，
 * 算作一处内存改动），并把原来那个值交回去，由状态条说出来。
 *
 * 返回 null = 没事可做。系统预设是基准、不可改，调用方自己拦。
 */
export function illegalSupportStyle(values: BbsValues, variantIdx = 0): string | null {
  const cur = String(norm(values.support_style, variantIdx) ?? '')
  if (!cur) return null
  return supportStyleSet(values, variantIdx).includes(cur) ? null : cur
}

const SUPPORT_TYPE_ZH: Record<string, string> = {
  'normal(auto)': '普通(自动)',
  'tree(auto)': '树状(自动)',
  'normal(manual)': '普通(手动)',
  'tree(manual)': '树状(手动)',
}

/** 状态条里那一句。`bad` 值是 illegalSupportStyle 返回的那个 */
export function styleFixNote(bad: string, values: BbsValues, variantIdx = 0): string {
  const t = String(norm(values.support_type, variantIdx) ?? '')
  return `支撑样式「${bad}」在「${SUPPORT_TYPE_ZH[t] || t}」下不存在，已按 BBS 的做法改回「默认」`
}
