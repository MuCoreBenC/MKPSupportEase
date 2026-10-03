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
 * # 回落链：快拆版整机 → 整机 → 品牌 logo → 空
 *
 * 前两级都是**选择层级**：选了机型看整机图，再选了版本看「装了快拆件那张」
 * （`Machine.imageVariant`，今天只有 A1 mini 有）。
 *
 * 第二张图 2026-10-03 之前硬编码在本文件（`MODEL_ART.A1_MINI.withVariant`）——数据侧
 * 看不见、工作台换不掉，而台账里那张图谁都不引用。现在它住机型文件：
 * 「这台机器有两张图」是机型自己的事，不是界面的事。
 *
 * 缺图一路往下落，不白屏：查不到 id / 那条资产没有 `path` / 机型压根没写 `image`
 * （A2L、P2S、X1C 今天就是）⇒ 品牌 logo。P2S / X1C 以前这里写着不存在的
 * `/assets/printers/p2s.webp`：SPA 回落成 index.html，`<img>` 静默不显示，
 * 页签点得开、控制台也干净 —— 那条错路已经随显式表一起没了。
 */

import type { CatalogAsset, Machine } from '../../api/contract'
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

/** 回落链：版本级图位 → 机型图 → 品牌 logo → 空 */
export function pickArt(
  sel: Selection,
  machine: Machine | undefined,
  assets: CatalogAsset[],
): HeroArt | null {
  /* 第二张图只在**选到版本**这一级才看（与旧表 `sel.variant && art.withVariant` 同一条件） */
  const deep = sel.variant ? assetUrlOf(assets, machine?.imageVariant) : null
  const photo = deep ?? assetUrlOf(assets, machine?.image)
  if (photo !== null) return { src: photo, kind: 'photo' }

  const logo = sel.brand ? BRAND_ART[sel.brand] : undefined
  return logo ? { src: logo, kind: 'logo' } : null
}
