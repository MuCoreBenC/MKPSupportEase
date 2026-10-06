import type { FileKind, FileRef, RuntimeCatalogFile, VersionFiles } from '../../api/contract'
import assetsJson from './data/assets.json'
import { mockCatalogFiles } from './catalogFiles'
import bundlesJson from './data/bundles.json'
import type { RawAsset, RawAssetsFile, RawBundle } from './types'
import { findRawVersion, findVersion } from './machines'

/**
 * 「这个机型这个版本要哪些文件」—— 一跳答完。
 *
 * 两条互不相交的链（★ 2026-10-06 收口：MKP 那一支改成查**目录登记**，
 * 与真机 `ipc/presets.rs::version_files_dto` 一致）：
 *
 *   MKP 预设   目录登记 catalog.files[]（按机型+版本查）  （完全不经过 bundle）
 *   切片器 profile  version.recommendedBundle ──▶ bundle.assetRefs ──▶ asset.relativePath
 *
 * `bundle.assetRefs` 的注释里写明「只含 bbs_profile/orca_profile，不含 mkp_preset」，
 * 所以 bundle 只管一半文件。查一次要走三跳、碰四张表，而且机型↔bundle 那一跳没有校验。
 *
 * 这里把两支拼进同一个 FileRef[]：调用方看到的是一份扁平清单，
 * 「经过 bundle 还是直接拼路径」是我们内部的事。
 */

const bundles = bundlesJson as RawBundle[]
const assets = (assetsJson as RawAssetsFile).assets

const bundleById = new Map(bundles.map((b) => [b.id, b]))
const assetById = new Map(assets.map((a) => [a.id, a]))

/**
 * 目录里这个 (机型, 版本) 的那一份 MKP —— 与真机 `runtime::catalog::file_of` 同一口径。
 *
 * 按机型 + 版本查、**不看 kind**（真机也不看）：目录里只有 MKP 会带版本号，
 * 切片器配置与图标那两种的 `versionId` 是空串，撞不上。
 */
function catalogFileOf(machineId: string, versionId: string): RuntimeCatalogFile | null {
  return (
    mockCatalogFiles().find((f) => f.machineId === machineId && f.versionId === versionId) ?? null
  )
}

function assetKind(a: RawAsset): FileKind {
  if (a.resourceType === 'bbs_profile') return 'bbs_profile'
  if (a.resourceType === 'orca_profile') return 'orca_profile'
  return 'mkp_preset'
}

/**
 * size / sha256 一律不给。
 *
 * 上游 `assets_index.json` 里这两个字段全是空值（`sha256: ""` / `size: 0`），
 * 真值要到发布打 manifest 那一刻才算出来。
 * 宁可让界面显示「未知」，也不造一个看起来像真的假数字。
 */
function refFromAsset(a: RawAsset): FileRef {
  return { kind: assetKind(a), fileName: a.fileName, path: a.relativePath }
}

/**
 * 组合不存在时返回 `null` —— 交给调用方翻译成「后端没有这个组合」，
 * 和「存在但没配齐」区分开（后者是 `incomplete: true` + `missing[]`）。
 */
export function resolveVersionFiles(machineId: string, versionId: string): VersionFiles | null {
  const version = findVersion(machineId, versionId)
  const raw = findRawVersion(machineId, versionId)
  if (!version || !raw) return null

  const files: FileRef[] = []
  const missing: string[] = []

  /*
   * —— MKP 预设一支 ——
   *
   * ★ 从**目录登记**里查，不是上游那个 `version.presetFile`：真机这一支就是
   *   `catalog.file_of(...)`，而目录里的名字是 `{机型}-{版本小写}.toml`（消费端按这个
   *   找文件，见 `workbench/app/build.rs`）；上游老命名（`A1.toml`）消费端不认识 ——
   *   照老命名填，首页套餐会一律显示「缺」。
   */
  const registered = catalogFileOf(machineId, versionId)
  if (registered !== null) {
    files.push({ kind: 'mkp_preset', fileName: registered.fileName, path: registered.path })
  } else {
    missing.push(`${machineId} / ${versionId} 没有配 MKP 预设文件（目录里查不到）`)
  }

  // —— 切片器 profile 一支 ——
  if (!version.bundle) {
    missing.push(`${machineId} / ${versionId} 没有配 bundle，取不到切片器配置`)
  } else {
    const bundle = bundleById.get(version.bundle)
    if (!bundle) {
      missing.push(`${machineId} / ${versionId} 指向的 bundle 不存在：${version.bundle}`)
    } else {
      for (const ref of bundle.assetRefs) {
        const asset = assetById.get(ref)
        if (!asset) {
          // 上游这一跳是硬失败（E_DANGLING_ASSET_REF），我们也不静默丢弃
          missing.push(`bundle ${bundle.id} 引用了不存在的 asset：${ref}`)
          continue
        }
        files.push(refFromAsset(asset))
      }
    }
  }

  return { files, incomplete: missing.length > 0, missing }
}
