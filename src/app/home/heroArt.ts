/*
 * 大图按品牌 / 机型 / 版本取哪一张。
 *
 * 键用契约 id（`A1` / `A1_MINI` / `P1S`）—— 与选择器是同一套 id；
 * 图片路径与回落链不动。
 */

import type { Selection } from './MachinePicker'

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

/** 键是 `Machine.brand`（品牌显示名）。目录里目前只有这一家 */
export const BRAND_ART: Record<string, string> = {
  '拓竹 (Bambu Lab)': '/bambulab.svg',
}

export const MODEL_ART: Record<string, ModelArt> = {
  A1: { plain: '/a1.webp' },
  A1_MINI: { plain: '/a1mini.webp', withVariant: '/printer-hero.png' },
  P1S: { plain: '/p1s.webp' },
  // A2L / P2S / X1C 暂缺图，自动回落到品牌 logo
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
