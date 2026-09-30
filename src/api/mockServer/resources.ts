import type { FileKind, PresetFileInfo, VersionRef } from '../../api/contract'
import assetsJson from './data/assets.json'
import bundlesJson from './data/bundles.json'
import presetRegistryJson from './data/preset_registry.json'
import type { RawAsset, RawAssetsFile, RawBundle, RawPresetRegistry } from './types'
import { bbsFileOf, sizeTextOf } from './bbsFiles'
import { allMachines, findRawVersion } from './machines'

/**
 * 预设仓库清单 —— 这一层回答的是**倒查**。
 *
 * # 这个文件存在的理由：倒查
 *
 * 上游只存单向指针：
 *
 *   version.recommendedBundle ──▶ bundle.assetRefs ──▶ asset
 *   version.presetFile        ──▶ presets/mkp/<file>
 *
 * 反过来的问题谁都不答：「这个文件被哪些 bundle 装着、被哪些版本当 MKP 预设」。
 * `files.ts` 答的是正向那一跳（「这个版本要哪些文件」）；这里补反向的一半。
 *
 * # 「可选」不是「孤儿」
 *
 * 判据是「有没有被引用」，结论的说法是：没被引用 = 没放进默认交付集 = **可选**，
 * 那是正常状态（实测 6 个：4 个 0.2mm 的工艺 profile + 2 条编出来的 Orca），
 * 所以它不进问题清单，只以 `PresetFileInfo.delivery` 的形式被界面看见。
 *
 * # 喷嘴 / 层高只覆盖一半
 *
 * `preset_registry.json` 9 条全是 bbs_profile，9 个 mkp_preset 一条都没有 ——
 * 所以 `nozzle` / `layerHeight` 只有切片器 profile 会命中。
 * 这不是数据缺失：MKP 的 `.toml` 是涂胶预设，本来就没有喷嘴层高这回事。
 */

const assets = (assetsJson as RawAssetsFile).assets
const bundles = bundlesJson as RawBundle[]
const presetEntries = (presetRegistryJson as RawPresetRegistry).entries

const presetEntryByFileName = new Map(presetEntries.map((e) => [e.fileName, e]))

/** 与 `files.ts` 同一条规则。两处各一份是有意的：那边只认正向，这边只认倒查 */
function kindOf(a: RawAsset): FileKind {
  if (a.resourceType === 'bbs_profile') return 'bbs_profile'
  if (a.resourceType === 'orca_profile') return 'orca_profile'
  return 'mkp_preset'
}

/**
 * 倒查表：预设文件名 → 把它当 MKP 预设的那些「机型:版本」。
 *
 * 遍历的是**合成后的机型表**（`allMachines()`），所以与界面上显示的机型 / 版本是同一个来源。
 * `presetFile` 只在原始版本条目里有（契约刻意不暴露它），所以另取一次 raw。
 */
function buildPresetFileUsers(): Map<string, VersionRef[]> {
  const byPresetFile = new Map<string, VersionRef[]>()

  for (const m of allMachines()) {
    for (const v of m.versions) {
      const raw = findRawVersion(m.id, v.id)
      // A2L 的 presetFile 是空串 —— 「没配」不是「配了个空文件」
      if (raw === undefined || raw.presetFile === '') continue
      const ref: VersionRef = { machine: m.id, version: v.id }
      const list = byPresetFile.get(raw.presetFile)
      if (list === undefined) byPresetFile.set(raw.presetFile, [ref])
      else list.push(ref)
    }
  }

  return byPresetFile
}

const usedByPresetFile = buildPresetFileUsers()

/**
 * **演示用的大小与时间。** 按 `path` 稳定推出来 —— 同一个文件每次刷新都是同一个值，
 * 不是随机数（随机数会让人以为文件真的变了）。
 *
 * 为什么要有它：上游 `assets_index.json` 里 `size` 全是 0、`sha256` 全是空串，
 * 所以这一层**不可能**有真值。而整个界面还有一半是演示数据，列表上少了这两列会让版面
 * 看起来没做完。
 *
 * 编在这里而不是编在界面上：真后端接上时换掉的是这一个函数，不是一堆界面代码。
 */
function demoStat(path: string, kind: FileKind): { size: string; modified: string } {
  let h = 0x811c9dc5
  for (let i = 0; i < path.length; i += 1) {
    h ^= path.charCodeAt(i)
    h = Math.imul(h, 0x01000193) >>> 0
  }
  /* MKP 的 .toml 是几 KB 的涂胶预设，切片器 profile 是一两 KB 的 json —— 两档分开推 */
  const kb =
    kind === 'mkp_preset' ? 3.6 + ((h >>> 8) % 22) / 10 : 1.1 + ((h >>> 8) % 9) / 10
  /* 时间落在 2026 年 8 月中到 9 月中这一个月里，和上游那批资源的实际节奏对得上 */
  const day = (h >>> 3) % 31
  const month = day < 14 ? 8 : 9
  const dd = day < 14 ? 18 + day : day - 13
  return {
    size: `${kb.toFixed(1)} KB`,
    modified: `2026-${String(month).padStart(2, '0')}-${String(dd).padStart(2, '0')}`,
  }
}

function buildPresetFiles(): PresetFileInfo[] {
  return assets.map((a): PresetFileInfo => {
    const entry = presetEntryByFileName.get(a.fileName)
    const inBundles = assetToBundles.get(a.id) ?? []
    const usedByVersions = usedByPresetFile.get(a.fileName) ?? []
    const kind = kindOf(a)
    /*
     * bbs 这一档有**真值**（`data/bbs_files.json`）。查得到就用它，
     * 查不到（MKP / Orca / 编的）照旧走演示推值 —— 两档在类型上分得开，界面照实说。
     */
    const fact = bbsFileOf(a.id)
    const stat =
      fact === undefined
        ? demoStat(a.relativePath, kind)
        : { size: sizeTextOf(fact.bytes), modified: fact.updatedAt }
    return {
      id: a.id,
      fileName: a.fileName,
      path: a.relativePath,
      kind,
      category: a.category,
      /* 注册表给的是数组（可能一个文件对多台机型），asset 实体只有一个 machineId */
      machineIds: entry?.machineIds ?? (a.machineId === '' ? [] : [a.machineId]),
      /* 上游把「没有」写成空串，对外统一成不给这个字段 */
      nozzle: entry?.nozzle === undefined || entry.nozzle === '' ? undefined : entry.nozzle,
      layerHeight:
        entry?.layerHeight === undefined || entry.layerHeight === '' ? undefined : entry.layerHeight,
      inBundles,
      usedByVersions,
      delivery: inBundles.length === 0 && usedByVersions.length === 0 ? 'optional' : 'default',
      /** bbs 那两个是真值（见上），其余是**演示值**（`demoStat`）。叫 `sizeText` 是为了不和 `FileRef.size`（字节数）撞 */
      sizeText: stat.size,
      modifiedText: stat.modified,
      /** 上面那两格是哪来的 —— 界面靠它决定 tooltip 说「真值」还是「演示数据」 */
      statFrom: fact === undefined ? 'demo' : 'file',
    }
  })
}

/**
 * asset id → 装着它的 bundle id 列表。
 *
 * 只在 `resources.ts` 内部用：它是 `PresetFileInfo.inBundles` 与 `delivery` 的判据。
 * 产品仓的契约里没有「把套餐单独列一张表」那个方法（那是工作台的事），所以不对外导出。
 */
function buildAssetToBundles(): Map<string, string[]> {
  const out = new Map<string, string[]>()
  for (const b of bundles) {
    for (const ref of b.assetRefs) {
      const list = out.get(ref)
      if (list === undefined) out.set(ref, [b.id])
      else list.push(b.id)
    }
  }
  return out
}

const assetToBundles = buildAssetToBundles()

const presetFileList: PresetFileInfo[] = buildPresetFiles()

export function allPresetFiles(): PresetFileInfo[] {
  return presetFileList
}
