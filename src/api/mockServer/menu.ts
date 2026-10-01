import type { MenuEntry } from '../../api/contract'
import assetsJson from './data/assets.json'
import bundlesJson from './data/bundles.json'
import presetRegistryJson from './data/preset_registry.json'
import type { RawAssetsFile, RawBundle, RawPresetRegistry } from './types'
import { allMachines, findRawVersion } from './machines'

/**
 * 菜单表：每个文件对客户端公开到什么程度。
 *
 *   `bundled`   已分配 —— 在某个套餐里，客户端自动下
 *   `optional`  可选 —— 在菜单上，客户端看得到、可手动下。**逐瓶指定**，不是「同机型的都算」
 *   `archived`  仅归档 —— 不在菜单上，**客户端完全不知道它存在**
 *
 * # 初值怎么来的：进了套餐的算已分配，其余算可选
 *
 * 套餐分两支装东西，**两支的指针方向不一样**：
 *
 *   切片器 profile   bundle.assetRefs ──▶ asset（上游刻意只放 bbs / orca，不放 mkp）
 *   MKP 预设         version.presetFile ──▶ `presets/mkp/<file>`（完全不经过 bundle）
 *
 * 所以「这个套餐装什么」= 它的 `assetRefs` ∪ 指向它的那些版本的 `presetFile`。
 * 判据和 `PresetFileInfo.delivery` 是同一条（有没有被引用），结论也一致：
 * 仓库里有、没进默认交付集 = **可选**，那是一种正常身份，
 * 所以初值不会把它设成「仅归档」让客户端直接看不见。
 *
 * **初始一个 `archived` 都没有是有意的** —— 藏文件必须是人做的决定，不该由假后端替他决定。
 * 真实产品里这张表是可编辑的（工作台那一侧）；客户端这一侧只读。
 */

const assets = (assetsJson as RawAssetsFile).assets
const bundles = bundlesJson as RawBundle[]
const presetEntries = (presetRegistryJson as RawPresetRegistry).entries

/** 文件名 → asset id。两处都有文件名，asset id 才是跨表的标识 */
const assetIdByFileName = new Map<string, string>()
for (const a of assets) assetIdByFileName.set(a.fileName, a.id)
for (const e of presetEntries) {
  if (!assetIdByFileName.has(e.fileName)) assetIdByFileName.set(e.fileName, e.relativePath)
}

/**
 * MKP 预设那一支：bundle id → 它该装的 asset id。
 *
 * 走「版本的 bundle」而不是机型级的 `defaultBundle`：前者已经是
 * 「版本级优先、机型级兜底」归一后的值，两处各算一遍会分岔。
 */
function mkpIdsByBundle(): Map<string, string[]> {
  const out = new Map<string, string[]>()

  for (const m of allMachines()) {
    for (const v of m.versions) {
      if (v.bundle === '') continue
      const raw = findRawVersion(m.id, v.id)
      // A2L 的 presetFile 是空串 —— 「没配」不是「配了个空文件」
      if (raw === undefined || raw.presetFile === '') continue
      const id = assetIdByFileName.get(raw.presetFile) ?? raw.presetFile
      const list = out.get(v.bundle)
      if (list === undefined) out.set(v.bundle, [id])
      else if (!list.includes(id)) list.push(id)
    }
  }

  return out
}

/** 套餐里的 MKP 与 BBS 合起来的 asset id 集合 */
function bundledIds(): Set<string> {
  const mkp = mkpIdsByBundle()
  const out = new Set<string>()
  for (const b of bundles) {
    for (const id of b.assetRefs) out.add(id)
    for (const id of mkp.get(b.id) ?? []) out.add(id)
  }
  return out
}

const bundled = bundledIds()

const menu: MenuEntry[] = assets.map(
  (a): MenuEntry => ({ fileId: a.id, visibility: bundled.has(a.id) ? 'bundled' : 'optional' }),
)

export function menuEntries(): MenuEntry[] {
  return menu.map((e) => ({ ...e }))
}
