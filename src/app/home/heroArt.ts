/*
 * 大图按品牌 / 机型 / 版本取哪一张。
 *
 * 键用契约 id（`A1` / `A1_MINI` / `P1S`）—— 与选择器是同一套 id。
 *
 * # 图片路径按**产品仓自己的资源目录**写（移植时改过）
 *
 * 试验场那五条是写它自己 `public/` 根下的文件（`/a1.webp`、`/bambulab.svg`…），
 * 产品仓的整机图在 `public/assets/printers/` 下，品牌 logo 更早就从 `public/` 搬进了
 * `src/app/assets/bambuLogo.ts`（那是同一个 SVG 的 data URI，见那个文件的头注释）。
 * 照抄那五条会**一律 404**（SPA 回落成 index.html，`<img>` 静默不显示）——
 * 所以这里换成产品仓真有的路径。
 *
 * 没有照 `Machine.image` 那个字段（契约里它写着"前端自己拼资源路径"）：
 * 目录里 `A1_MINI.image` 是 `a1_mini.webp`，而本仓的文件叫 `a1mini.webp`（名字对不上，
 * 试验场那边两个名字各存了一份）。**显式表 + 回落链**比"拼一个可能不存在的名字"稳，
 * 等资产命名统一了再改成数据驱动。
 */

import { BAMBU_LOGO_DARK } from '../assets/bambuLogo'
import type { Selection } from './MachinePicker'

/**
 * 大图按选择层级取：品牌 logo → 整机 → 装了快拆件的整机。
 * 缺图回落到品牌 logo，不白屏。
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
  '拓竹 (Bambu Lab)': BAMBU_LOGO_DARK,
}

export const MODEL_ART: Record<string, ModelArt> = {
  A1: { plain: '/assets/printers/a1.webp' },
  A1_MINI: {
    plain: '/assets/printers/a1mini.webp',
    withVariant: '/assets/printers/a1mini-variant.webp',
  },
  P1S: { plain: '/assets/printers/p1s.webp' },
  /* 这三张试验场那边缺，产品仓有 —— 顺手补上，不再一律回落到 logo */
  P2S: { plain: '/assets/printers/p2s.webp' },
  X1C: { plain: '/assets/printers/x1c.webp' },
  // A2L 暂缺图，自动回落到品牌 logo
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
