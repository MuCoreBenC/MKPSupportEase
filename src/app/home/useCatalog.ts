/*
 * 首页与校准页共用的「文件体系」目录。
 *
 * 两个读，各管一级：
 *
 *   getMachines()        品牌 / 机型 / 版本 —— 三级选择的数据源（不再是手编表）
 *   getRuntimeCatalog()  MKP 预设清单 + 资产登记 + 品牌 —— 见下
 *
 * 作者定的方向：「首页也应该是消费那个文件，而不是用硬编码」——所以 A1 就是 A1
 * （`A1.toml`），P1S 的版本就是 `LITE`，不再靠一张手编表 + id 映射。
 *
 * # MKP 清单为什么吃 runtime catalog 的 `files[]`（2026-10-06 真机 bug 钉下的）
 *
 * `getPresetFiles()` 在真机上**只出切片器预设**（MKP 由 catalog 的 `files[]` 登记，
 * 那条路的 `usedByVersions` 恒空）—— 首页的「文件」这一级吃它就整级为空：
 * 抽屉里每项没有文件名小字、`fileOf` 恒 null，第二页的「应用」按钮点击在
 * `comboFileName === null` 处**静默返回**（作者：「点了没用」）。mock 里
 * getPresetFiles 恰好带 9 份 MKP 带倒查，浏览器里测不出来 —— 分歧以真机数据源
 * 为准：MKP 的权威登记在 catalog 的 `files[]`（与预设页同源），每条自带
 * `machineId` / `versionId`，倒查就是它自己。
 *
 * 整机图这条也一样（2026-10-03 第二刀）：机型写的是**资产 id**，文件在哪由 catalog 的
 * `assets[]` 说（`assetUrlOf`），客户端不再 import 具体文件。
 *
 * 都是静态读（每编辑一次就变的是参数值，不在这里），进页面拉一次就够。
 *
 * 跨页缓存（2026-10-05）：原状是「tab 切换会重挂，一次进入一个来回」，代价是每次
 * 切回都从空数组起跳、一切「先空后满」—— 校准页的 pill 与读数要等目录、底账、参数
 * 几趟 IPC 都回来才长出真值，进场后当着用户的面再变一次（页签切换的「闪一下」）。
 * 现在挂载先吃上一份结果渲染、照旧重拉（文件可能换过），回来后覆盖缓存与 state；
 * 先旧后新在静态目录上没有一致性风险。
 */

import { useEffect, useState } from 'react'
import { api } from '../../api'
import type { CatalogAsset, Machine, PresetFileInfo, RuntimeCatalogBrand } from '../../api/contract'

export interface Catalog {
  /** 机型目录（含品牌与版本），按目录顺序 */
  machines: Machine[]
  /** MKP 预设清单（catalog `files[]` 里 `kind === 'mkp_preset'` 的那些；倒查 = 它自己的 machineId/versionId） */
  presets: PresetFileInfo[]
  /**
   * 资产登记（`assets[]`）。机型图按 id 在这里查 `path`。
   *
   * 拉不到就是空数组 —— 那条路只让大图回落品牌 logo，**不挡机型与版本**：
   * 资产登记读的是运行时那份 catalog.json，它比 TOML 树晚一步（也可能还没换新）。
   */
  assets: CatalogAsset[]
  /** 品牌（含品牌图的资产 id）。同样拉不到就是空 —— 大图再回落内置字标 */
  brands: RuntimeCatalogBrand[]
}

/** 「文件」这一级的 id：`机型/版本`，与工作台发布 uid 同一形状 */
export function uidOfFile(file: PresetFileInfo): string | null {
  const ref = file.usedByVersions[0]
  return ref === undefined ? null : `${ref.machine}/${ref.version}`
}

let catalogCache: Catalog = { machines: [], presets: [], assets: [], brands: [] }

export function useCatalog(): Catalog {
  const [cat, setCat] = useState<Catalog>(catalogCache)

  useEffect(() => {
    let alive = true
    void Promise.all([api.getMachines(), api.getRuntimeCatalog()]).then(
      ([list, world]) => {
        if (!alive) return
        catalogCache = {
          machines: list,
          presets: world.files
            .filter((f) => f.kind === 'mkp_preset')
            .map(
              (f): PresetFileInfo => ({
                id: f.fileName,
                fileName: f.fileName,
                path: f.path,
                kind: 'mkp_preset',
                category: '',
                machineIds: [f.machineId],
                usedByVersions: [{ machine: f.machineId, version: f.versionId }],
                delivery: 'default',
                inBundles: [],
              }),
            ),
          assets: world.assets ?? [],
          brands: world.brands ?? [],
        }
        setCat(catalogCache)
      },
      (err: unknown) => {
        /* 拉不到就空着：三级选择不显形（不摆假选项），控制台里有名字。
           这一层不吞也不编 —— 与预设页「三态显式」同一条规矩，但不给首页加骨架屏那一层。 */
        console.error('[catalog] 目录拉取失败', err)
      },
    )
    return () => {
      alive = false
    }
  }, [])

  return cat
}
