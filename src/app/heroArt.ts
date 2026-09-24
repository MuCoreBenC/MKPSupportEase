import type { Selection } from './components/MachinePicker'

import { BAMBU_LOGO_DARK } from './assets/bambuLogo'

/**
 * 大图按选择层级取：品牌 logo → 整机 → 装了快拆件的整机。
 * 图片还没配齐，用显式表 + 回落链，缺图不会白屏。
 */

export type ArtKind = 'logo' | 'photo'

export interface HeroArt {
  src: string
  kind: ArtKind
}

interface ModelArt {
  /** 整机 */
  plain: string
  /** 装了快拆件的整机 */
  withVariant?: string
}

export const BRAND_ART: Record<string, string> = {
  /* data URI，随 bundle 同步到达 —— 不走网络，onError 掉层对它不再有机会触发 */
  bambu: BAMBU_LOGO_DARK,
}

/**
 * 整机图。**路径指向资产根**（`public/assets/`，b05 Task 8 的约定）——
 * 这几张已经从 `public/printers/bambu/` 搬进去了（Task 9.1），别再往旧位置写。
 *
 * 严格说这里仍是一张硬编码表：真正的引用在资产库里（`presets/assets.toml` 的
 * `a1-image` / `p2s-image` …，机型定义的 `image` 字段指着它们）。**旧 app 这一侧
 * 还没有接资产命令**（那是工作台的事，Task 14.6），所以先把路径对齐、别断图。
 *
 * `p2s` / `x1c` 是旧仓那两张（裁决 2026-09-24：暂用，将来补新图再换）；
 * `a2l` 至今没有图，自动回落到品牌 logo。
 */
export const MODEL_ART: Record<string, ModelArt> = {
  a1: { plain: '/assets/printers/a1.webp' },
  a1mini: {
    plain: '/assets/printers/a1mini.webp',
    withVariant: '/assets/printers/a1mini-variant.webp',
  },
  p1s: { plain: '/assets/printers/p1s.webp' },
  p2s: { plain: '/assets/printers/p2s.webp' },
  x1c: { plain: '/assets/printers/x1c.webp' },
}

/** 回落链：版本图 → 整机图 → 品牌 logo → 空 */
export function pickArt(sel: Selection): HeroArt | null {
  const art = sel.model ? MODEL_ART[sel.model] : undefined
  if (art) {
    if (sel.variant && art.withVariant) return { src: art.withVariant, kind: 'photo' }
    return { src: art.plain, kind: 'photo' }
  }

  const logo = sel.brand ? BRAND_ART[sel.brand] : undefined
  return logo ? { src: logo, kind: 'logo' } : null
}
