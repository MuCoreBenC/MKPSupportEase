/*
 * 首页与校准页共用的「文件体系」目录。
 *
 * 两个读，各管一级：
 *
 *   getMachines()      品牌 / 机型 / 版本 —— 三级选择的数据源（不再是手编表）
 *   getPresetFiles()   仓库里 9 份 MKP 预设文件（每份带 机型/版本 倒查）——「文件」这一级
 *
 * 作者定的方向：「首页也应该是消费那个文件，而不是用硬编码」——所以 A1 就是 A1
 * （`A1.toml`），P1S 的版本就是 `LITE`，不再靠一张手编表 + id 映射。
 *
 * 都是静态读（每编辑一次就变的是参数值，不在这里），进页面拉一次就够。
 * 首页与校准页各持一份：tab 切换会重挂，一次进入一个来回，不做跨页缓存。
 */

import { useEffect, useState } from 'react'
import { api } from '../../api'
import type { Machine, PresetFileInfo } from '../../api/contract'

export interface Catalog {
  /** 机型目录（含品牌与版本），按目录顺序 */
  machines: Machine[]
  /** 仓库里 `kind === 'mkp_preset'` 的文件；每份带 `usedByVersions`（机型/版本倒查） */
  presets: PresetFileInfo[]
}

/** 「文件」这一级的 id：`机型/版本`，与工作台发布 uid 同一形状 */
export function uidOfFile(file: PresetFileInfo): string | null {
  const ref = file.usedByVersions[0]
  return ref === undefined ? null : `${ref.machine}/${ref.version}`
}

export function useCatalog(): Catalog {
  const [machines, setMachines] = useState<Machine[]>([])
  const [presets, setPresets] = useState<PresetFileInfo[]>([])

  useEffect(() => {
    let alive = true
    Promise.all([api.getMachines(), api.getPresetFiles()]).then(
      ([list, files]) => {
        if (!alive) return
        setMachines(list)
        setPresets(files.filter((f) => f.kind === 'mkp_preset'))
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

  return { machines, presets }
}
