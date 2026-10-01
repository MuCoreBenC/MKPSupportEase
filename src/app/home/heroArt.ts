/*
 * 大图按品牌 / 机型 / 版本取哪一张。
 *
 * 键用契约 id（`A1` / `A1_MINI` / `P1S`）—— 与选择器是同一套 id。
 *
 * # 整机图是**界面素材**，不住资产台账（2026-10-01 裁决）
 *
 * 这几张图原来在 `public/assets/printers/` 下，并在 `presets/assets.toml` 里
 * 登记成 `type = 'image'` 的资产条目（还带着机型的 `image = 'a1-image'` 引用）。
 * 第三圈把"产品数据资源"（预设 / BBS 配置 / 模型 / 图标）收进 Catalog + Delivery 之后，
 * 这一类的归属才问清楚：**它不是产品数据资源，只是界面为了展示机器而用的图**——
 * 用户不需要单独下载、更新、管理它，所以把它登记进 catalog 反而把 Catalog 的职责扩大了。
 * 于是它从资产台账里剥离，搬进 `src/app/assets/printers/`，与品牌 logo
 * （`src/app/assets/bambuLogo.ts`）同住一层：
 *
 *   - 台账管的（能下载、要 SHA）= `mkp/…`，判据在 `runtime::catalog`；
 *   - 界面自带的（随程序本体走）= 这里，`import` 回来由 vite 管线给带哈希的 URL。
 *
 * 判据守着这条边界：**资产台账里已无 image 类**（`runtime::catalog` 的测试）。
 * 判断依据不是"文件是不是图片"，而是"它是不是产品数据资源"。
 *
 * # 显式表 + 回落链
 *
 * 没有照 `Machine.image` 那个字段拼路径：它是**资产 id**（`a1-image`），而资产 id 现在
 * 对应的是台账条目 —— 机型图已经不在台账里了，这个字段对界面没有意义（照实为空）。
 * 显式表也比"拼一个可能不存在的名字"稳：机器名与文件名本来就对不齐。
 */

import a1 from '../assets/printers/a1.webp'
import a1mini from '../assets/printers/a1mini.webp'
import a1miniVariant from '../assets/printers/a1mini-variant.webp'
import p1s from '../assets/printers/p1s.webp'
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
  A1: { plain: a1 },
  A1_MINI: { plain: a1mini, withVariant: a1miniVariant },
  P1S: { plain: p1s },
  /* P2S / X1C 没有外观图：台账里那两张待换的旧图在 b05 Task 9 就删了，
     这里照实留空、回落到品牌 logo（以前这里写着不存在的 `/assets/printers/p2s.webp`：
     SPA 回落成 index.html，`<img>` 静默不显示，页签点得开、控制台也干净） */
  // A2L 同样暂缺图，自动回落到品牌 logo
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
