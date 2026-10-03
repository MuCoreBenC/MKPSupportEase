/*
 * 大图按选择层级取哪一张。
 *
 * # 客户端只认识资产 id，不认识文件（2026-10-03 第二刀）
 *
 * 取图链路从「显式机型表 + import 具体图片」改成：
 *
 * ```text
 * catalog.assets[]  →  machine.image = 资产 id  →  asset.path  →  /assets/<path>
 * ```
 *
 * 于是「机型图是哪个文件」只活在数据侧（`presets/assets.toml` 的登记 + 机型文件的引用），
 * 换图不用改 TS：
 *
 * - **换文件**（同一条目换字节）：台账 `path` 不变、内容变 ⇒ 同步器按 SHA 对账覆盖 ⇒
 *   下次构建跟着变；
 * - **换指向**（`machine.image` 指到另一条资产）：机型文件一改，下一版 catalog 就带过来。
 *
 * 这也正是这里**不再有 import** 的原因：URL 必须来自 `catalog.assets[].path`，
 * 而不是某个构建期写死的模块路径 —— 写死的那个就是「第二张表」。
 *
 * # 回落链：版本图 → 机型图 → 品牌图 → 内置字标 → 空
 *
 * 图位分三层（作者 2026-10-03 定）：**品牌图 / 机型图 / 版本图**，**版本没有图就回落机型图**
 * —— 「标准版」与「快拆版」是同一台机器下两个独立的版本实体，各自有各自的图。
 *
 * - 版本图 = `sel.variant` 选中的那一版的 `image`（**选到版本这一级**才看它）
 * - 机型图 = `machine.image`
 * - 品牌图 = `brands[].logo`（**同样是台账里的一条资产** —— 品牌图片正式进了资产体系，
 *   `presets/brands.toml` 的 `logo` 引用资产 id）
 * - 内置字标 = `BRAND_ART`（**最后兜底**：台账没配品牌图 / 认不出那张图时用。
 *   它是 data URI 而不是文件，因为原 SVG 靠 `currentColor` 随外层 CSS 变色，
 *   而 `<img>` 是独立文档拿不到 —— 随包资产只能一色，详见资产文件里的注释）
 *
 * `Machine.imageVariant`（装了快拆件那张）**不在这条链里** —— 那是**硬件外观变体**，
 * 与版本不是一回事（作者裁定：不要拿它冒充版本）。它的数据留在机型文件里，
 * 等将来有了真正的变体模型再说。
 *
 * 缺图一路往下落，不白屏：查不到 id / 那条资产没有 `path` / 机型压根没写 `image`
 * （A2L、P2S、X1C 今天就是）⇒ 品牌图 ⇒ 内置字标。P2S / X1C 以前这里写着不存在的
 * `/assets/printers/p2s.webp`：SPA 回落成 index.html，`<img>` 静默不显示，
 * 页签点得开、控制台也干净 —— 那条错路已随显式表一起没了。
 */

import type { CatalogAsset, Machine, RuntimeCatalogBrand } from '../../api/contract'
import { BAMBU_LOGO_DARK } from '../assets/bambuLogo'
import type { Selection } from './MachinePicker'

export type ArtKind = 'logo' | 'photo'

export interface HeroArt {
  src: string
  kind: ArtKind
}

/** 键是 `Machine.brand`（品牌显示名）。目录里目前只有这一家 */
export const BRAND_ART: Record<string, string> = {
  '拓竹 (Bambu Lab)': BAMBU_LOGO_DARK,
}

/**
 * 资产 id → 可取回的 URL（`/assets/<path>`）。查不到就给 `null`，由调用方回落。
 *
 * - **id 大小写不敏感**（与 Rust 侧 `Assets::get` 同一口径）；
 * - 那条资产没有 `path`（`mkPreset` 那种生成产物）也给 `null` —— 它的文件不在资产根下；
 * - `path` 要 encode：资产根下真有带空格的名字（BBS 那批就带）。
 */
export function assetUrlOf(assets: CatalogAsset[], id: string | null | undefined): string | null {
  const wanted = (id ?? '').trim().toLowerCase()
  if (wanted === '') return null
  const hit = assets.find((a) => a.id.trim().toLowerCase() === wanted)
  const path = (hit?.path ?? '').trim()
  return path === '' ? null : `/assets/${encodeURI(path)}`
}

/** 回落链：版本图 → 机型图 → 品牌图 → 内置字标 → 空 */
export function pickArt(
  sel: Selection,
  machine: Machine | undefined,
  assets: CatalogAsset[],
  brands: RuntimeCatalogBrand[] = [],
): HeroArt | null {
  /* 版本图只在**选到版本**这一级才看；那一版没配图就回落机型图（不是"没图"） */
  const version = sel.variant ? machine?.versions.find((v) => v.id === sel.variant) : undefined
  const photo = assetUrlOf(assets, version?.image) ?? assetUrlOf(assets, machine?.image)
  if (photo !== null) return { src: photo, kind: 'photo' }

  /* 品牌图也走台账（`brands[].logo` 是资产 id）。按**显示名**对 ——
     机型上的 `brand` 就是显示名（后端 collect 时已从 brands.toml 换算过） */
  const wanted = (sel.brand ?? '').trim().toLowerCase()
  const brand = brands.find((b) => b.name.trim().toLowerCase() === wanted)
  const brandArt = assetUrlOf(assets, brand?.logo)
  if (brandArt !== null) return { src: brandArt, kind: 'logo' }

  const fallback = sel.brand ? BRAND_ART[sel.brand] : undefined
  return fallback ? { src: fallback, kind: 'logo' } : null
}
