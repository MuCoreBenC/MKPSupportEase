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

export const MODEL_ART: Record<string, ModelArt> = {
  a1: { plain: '/printers/bambu/a1.webp' },
  a1mini: {
    plain: '/printers/bambu/a1mini.webp',
    withVariant: '/printers/bambu/a1mini-variant.webp',
  },
  p1s: { plain: '/printers/bambu/p1s.webp' },
  // a2l / p2s / x1c 暂缺图，自动回落到品牌 logo
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
