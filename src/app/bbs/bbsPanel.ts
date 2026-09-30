/*
 * 把「版面 + 参数表 + 当前值 + 显隐规则」铺成可渲染的页/组/行 —— 纯函数。
 *
 * 上游这段逻辑写在 render() 里、边算边建 DOM（bbs-preset.js:307-404）。搬到 React
 * 就得先算清楚再画，否则「某页一行不剩要连 Tab 一起隐藏」这种判断没法做 ——
 * 那是**算完整页之后**才知道的事。
 *
 * 三条「不吞数据」的规则都在这里：
 *   1. 预设里有、版面里没有的 key → 落到最后一页末尾的「未在面板上的参数（N）」组
 *   2. 版面里有、参数表里没有的 key → 计入 stats.noMeta，状态条报出来
 *      （实测 2 个：max_volumetric_extrusion_rate_slope_positive/negative，BBS 自己的不一致）
 *   3. 某页在「跟 BBS 一样」档下一行不剩（速度页就是这样）→ 连 Tab 一起隐藏，
 *      BBS 也这么做；当前停的那页被隐藏就落到第一个还有内容的页
 */

import { matchRow, normalizeQuery } from './bbsSearch'
import type { BbsHit } from './bbsSearch'
import type {
  BbsLayoutTab,
  BbsParamMeta,
  BbsToggle,
  BbsValues,
  BbsViewMode,
} from './bbsTypes'

export interface PanelRow {
  key: string
  meta: BbsParamMeta
  /** toggle_field 为假：行还在，控件置灰 */
  greyed: boolean
  /** 搜索命中在哪一栏；没在搜或没命中就是 null */
  hit: BbsHit | null
}

export interface PanelGroup {
  name: string
  icon?: string
  rows: PanelRow[]
}

export interface PanelPage {
  id: string
  title: string
  groups: PanelGroup[]
  rowCount: number
  /** 这一页的命中数 —— tab 上那个角标。别的页没渲染，只能靠它说话 */
  hitCount: number
}

export interface PanelStats {
  rows: number
  /** 基准和用户都没给值的：画个空框只会让人以为它等于空 */
  skippedEmpty: number
  hiddenByRule: number
  greyed: number
  noMeta: string[]
  orphans: number
  pagesHidden: number
  /** 搜索命中总数（所有页加起来） */
  hits: number
  /**
   * 命中在「跟 BBS 一样」档收起的行里的个数。
   * 这些行搜不到也点不到，但闷着不说会让人以为搜索漏了 —— 状态条单独报。
   */
  hiddenHits: number
}

export interface PanelResult {
  pages: PanelPage[]
  stats: PanelStats
}

/* Tab 的英文 id 只用于 DOM 与状态，显示名取 layout 里的中文（来自 BBS 自己的 .po） */
const TAB_ID = ['quality', 'strength', 'speed', 'support', 'others']

export function buildBbsPanel(
  layout: BbsLayoutTab[],
  registry: Record<string, BbsParamMeta>,
  values: BbsValues,
  user: BbsValues,
  toggles: Map<string, BbsToggle>,
  viewMode: BbsViewMode,
  /** 搜索词（原样传进来，这里自己归一）。空串 = 没在搜，整段匹配跳过 */
  rawQuery = '',
  variantIdx = 0,
): PanelResult {
  const bbsMode = viewMode === 'bbs'
  const q = normalizeQuery(rawQuery)
  const placed = new Set<string>()
  const stats: PanelStats = {
    rows: 0, skippedEmpty: 0, hiddenByRule: 0, greyed: 0, noMeta: [], orphans: 0, pagesHidden: 0,
    hits: 0, hiddenHits: 0,
  }

  /** 没在搜就一次都不比 —— 249 行 × 3 次 includes 没必要白跑 */
  const hitOf = (key: string, meta: BbsParamMeta): BbsHit | null =>
    q ? matchRow(q, key, meta, values[key], variantIdx) : null

  const pages: PanelPage[] = layout.map((tab, i) => {
    const groups: PanelGroup[] = []
    for (const group of tab.groups) {
      const rows: PanelRow[] = []
      for (const key of group.fields) {
        placed.add(key)
        const meta = registry[key]
        /* BBS 自己也有这种不一致：Tab.cpp 还在引用的 key，参数定义早被注释掉了 */
        if (!meta) { stats.noMeta.push(key); continue }
        const value = values[key]
        /* 有值才出行 */
        if (value === undefined || value === null || value === '') { stats.skippedEmpty++; continue }
        const t = bbsMode ? toggles.get(key) : undefined
        if (t && !t.line) {
          stats.hiddenByRule++
          /* 这一行被规则收起了 —— 但它可能正是你在找的那个，记一笔好报出来 */
          if (hitOf(key, meta)) stats.hiddenHits++
          continue
        }
        const greyed = Boolean(t && !t.field)
        if (greyed) stats.greyed++
        const hit = hitOf(key, meta)
        if (hit) stats.hits++
        rows.push({ key, meta, greyed, hit })
        stats.rows++
      }
      if (rows.length) groups.push({ name: group.name, icon: group.icon, rows })
    }

    /* 预设里有、版面里没有的 key 落到最后一页末尾。
       宁可多出一组，也不能静默把数据吞掉。 */
    if (i === layout.length - 1) {
      const orphanKeys = Object.keys(user).filter((k) => !placed.has(k))
      if (orphanKeys.length) {
        stats.orphans = orphanKeys.length
        groups.push({
          name: `未在面板上的参数（${orphanKeys.length}）`,
          rows: orphanKeys.map((key) => {
            const meta = registry[key] ?? { label: { zh: key }, type: 'num' as const, unit: {} }
            const hit = hitOf(key, meta)
            if (hit) stats.hits++
            return { key, meta, greyed: false, hit }
          }),
        })
        stats.rows += orphanKeys.length
      }
    }

    return {
      id: TAB_ID[i] ?? `tab${i}`,
      title: tab.name,
      groups,
      rowCount: groups.reduce((n, g) => n + g.rows.length, 0),
      hitCount: groups.reduce((n, g) => n + g.rows.filter((r) => r.hit).length, 0),
    }
  })

  const visible = pages.filter((p) => p.rowCount > 0)
  stats.pagesHidden = pages.length - visible.length
  return { pages, stats }
}

/** 当前停的那一页还有内容吗？没有就落到第一个有内容的页（没有任何内容时返回 null） */
export function resolveActivePage(pages: PanelPage[], want: string | null): string | null {
  const visible = pages.filter((p) => p.rowCount > 0)
  if (!visible.length) return null
  if (want && visible.some((p) => p.id === want)) return want
  return visible[0].id
}
