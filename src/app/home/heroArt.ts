/*
 * 大图按品牌 / 机型 / 版本取哪一张。
 *
 * 键用契约 id（`A1` / `A1_MINI` / `P1S`）—— 与选择器是同一套 id。
 *
 * # 整机图是**界面素材**，但数据住在工作台（2026-10-03 改判）
 *
 * 这几张图在 `presets/assets/printers/` 下，并在 `presets/assets.toml` 里登记成
 * `type = 'image'` + `delivery = 'bundled'` 的资产条目（机型文件里也带
 * `image = 'a1-image'` 引用）—— **工作台里看得见、选得着、换得掉**。
 * 2026-10-01 第三刀曾把它们从台账剥离、硬编码进 `src/app/assets/`，那是**错的**：
 * 「不会编程的用户怎么改图片呢」—— 已作废。
 *
 * 到客户端的方式：**构建期从数据目录复制进这里**（`scripts/copy-assets.mjs`，
 * 落点就是下面那几个 import 路径；那个目录是生成物，不入库）。客户端**不下载**
 * 也不更新它们 —— 用户不换机器就不需要换图，而换图的人有权在工作台里换。
 * 判据在 `runtime::catalog::dest_of_asset`（**按交付档位拦**，不按类型拦）。
 *
 * # 显式表 + 回落链
 *
 * 没有照 `Machine.image` 那个字段拼路径：它是**资产 id**，而 id 到文件的映射要走
 * 资产根的解析（且 bundled 档的文件客户端拿不到 URL —— 不在 catalog 的 files[] 里）。
 * 显式表也比"拼一个可能不存在的名字"稳：机器名与文件名本来就对不齐。
 * 数据侧改了图 → `npm run build` 跑一次复制 → 这里换成新图。
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
