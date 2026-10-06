/*
 * 首页的**套餐清单** —— 「这个 combo 该有哪些文件」。
 *
 * # 为什么不再自己算（2026-10-06 作者裁定）
 *
 * 首页此前是"从 catalog 的 `files[]` 里挑一份 MKP"的单文件视角，于是那颗
 * 「下载并应用」永远只下得到一份 TOML —— 而套餐 = **MKP 预设 + 配套 BBS**
 * （`presets/bundles.toml`；发 MKP 不发配套 BBS，用户打出来是错的）。
 *
 * 这条路**已经有了**，就是预设页在用的 `getVersionFiles`（后端
 * `ipc/presets.rs::version_files_dto`，第二圈起就是 catalog 驱动）：
 *
 *   MKP 一支  catalog.file_of(machine, version)          → 真 path / size / sha256
 *   切片器一支 version.recommendedBundle ?? defaultBundle → bundle.assetRefs
 *              → catalog.asset(id) → kind + dest_of_asset  → `assets/<台账 path>`
 *
 * 所以这里**只做一次读 + 一次缓存**：落点算法（`dest_of_asset`）只有 Rust 那一处，
 * 前端不重算路径、不在契约里另长一份 `bundles`（那会变成同一条事实的第二份手抄，
 * 见 docs/HOME-BUNDLE-DOWNLOAD.md §3）。
 *
 * # 拿不到 ≠ 套餐为空（纪律②，2026-10-06 作者要求）
 *
 * `getVersionFiles` 返回 `null`（后端没有这个组合）、`incomplete`（该有的没配齐）、
 * 或者这次读**直接失败**时，套餐事实是**不完整的** —— 记进 `problems`，界面据此显示
 * 不可用，**绝不允许**把它当成"套餐里一个文件都没有"（后者会让按钮滑进
 * 「全齐全新 → 应用」而直接 apply，那是本次要防的那条危险路径）。
 *
 * # 一次进入读一次（套餐清单是静态的）
 *
 * 盘的现状（有没有、漂没漂）不在这里 —— 那两份单子是 `getDownloadedFiles` /
 * `getStaleFiles`，会随下载变，由页面按 `deliveryTick` 重拉。这里跨挂载缓存，
 * 与 `useCatalog` 同一条规矩：挂载先吃上一份结果渲染、照旧重拉。
 */

import { useEffect, useState } from 'react'
import { api } from '../../api'
import type { FileRef, VersionFiles } from '../../api/contract'

export interface BundleFiles {
  /**
   * 套餐里的文件（MKP + 切片器配置）。`null` = **还没拿到**（拉取中或失败）——
   * 与"套餐是空的"（空数组）是两件事，判据上必须分得开。
   */
  files: FileRef[] | null
  /** 非空 = 套餐事实不完整（后端没这个组合 / 配置没配齐 / 读取失败），一条一句能给人看 */
  problems: string[]
  /** 套餐里那一份 MKP 预设的文件名 —— **「应用」只认它**；没有就是 `null` */
  presetFileName: string | null
}

const UNKNOWN: BundleFiles = { files: null, problems: [], presetFileName: null }

/** 跨挂载缓存（key = `机型/版本`）。失败的**不写进来**：下次进这一页再试一次 */
const cache = new Map<string, BundleFiles>()

function toBundle(vf: VersionFiles | null, key: string): BundleFiles {
  // `null` 的语义是「后端没有这个组合」，不是「这个组合下没文件」
  if (vf === null) {
    return { files: null, problems: [`目录里没有 ${key} 这个组合`], presetFileName: null }
  }
  const preset = vf.files.find((f) => f.kind === 'mkp_preset')?.fileName ?? null
  /* `incomplete` 只表示"该有的没配齐" —— 缺的那几条原因原样透给界面（后端的话） */
  const problems = vf.incomplete
    ? vf.missing.length > 0
      ? vf.missing
      : [`${key} 的套餐没配齐`]
    : []
  return { files: vf.files, problems, presetFileName: preset }
}

export function useBundleFiles(machineId: string | null, versionId: string | null): BundleFiles {
  const key = machineId !== null && versionId !== null ? `${machineId}/${versionId}` : null
  const [state, setState] = useState<BundleFiles>(key === null ? UNKNOWN : (cache.get(key) ?? UNKNOWN))

  useEffect(() => {
    if (key === null || machineId === null || versionId === null) {
      setState(UNKNOWN)
      return
    }
    /* 先吃上一份（切回来不闪空），再重拉一遍 */
    const hit = cache.get(key)
    if (hit !== undefined) setState(hit)

    let alive = true
    void api.getVersionFiles(machineId, versionId).then(
      (vf) => {
        if (!alive) return
        const next = toBundle(vf, key)
        if (next.problems.length === 0) cache.set(key, next)
        setState(next)
      },
      (err: unknown) => {
        if (!alive) return
        console.error(`[bundle] 套餐拉取失败 ${key}`, err)
        /* ★ 纪律②：拿不到就说拿不到 —— 不写缓存、更不当成"套餐为空" */
        setState({
          files: null,
          problems: [`${key} 的套餐没取到 —— 稍后再试，或去看数据源配好没有`],
          presetFileName: null,
        })
      },
    )
    return () => {
      alive = false
    }
  }, [key, machineId, versionId])

  return state
}
