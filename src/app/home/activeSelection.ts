/*
 * 首页联动：把「正在使用的那一条」（唯一底账，新世界 `run/active-preset.json`）
 * 翻成首页三级选择。
 *
 * 作者定的关系：**应用哪一份预设，首页就变成哪一台** —— 预设页「应用」之后切回首页，
 * 品牌 / 机型 / 打印件版本三级要跟过来；刷新后仍在（底账落 Internal 根，不是内存）。
 *
 * # 映射表没了，这就是一次「查得到就填」
 *
 * 首页原来还吃手编表（`a1mini` / `std`），底账里是契约 id（`A1_MINI` / `STANDARD`），
 * 于是写了一张映射表。首页改成也消费文件体系之后，**两边是同一套 id** ——
 * 机器在目录里、版本在那台机器的版本表里，直接填；查不到（目录里没有这台 / 这个版本）
 * 就整体不反填，不硬凑。
 */

import { useEffect, useState } from 'react'

import { api } from '../../api'
import type { ActivePreset, Machine } from '../../api/contract'
import type { Selection } from './MachinePicker'

/**
 * 读一次底账并翻成首页的三级选择。`null` = 不反填（没有已应用 / 机器或版本不在目录里）。
 *
 * `machines` 必须是**已就绪**的目录（空数组时不要调）—— 反填要在目录里查得到才算数。
 * 底账走 IPC（`api.getActivePreset()`），所以是异步的；读不到当"没有"（不是错误）。
 */
export async function selectionFromActive(machines: Machine[]): Promise<Selection | null> {
  const entry = await api.getActivePreset().catch(() => null)
  if (entry === null) return null

  const machine = machines.find((m) => m.id === entry.machineId)
  if (machine === undefined) return null

  const version = machine.versions.find((v) => v.id === entry.versionId)

  return { brand: machine.brand, model: machine.id, variant: version?.id ?? null }
}

/**
 * 读**底账本身**（不只反填选择）：本页是当前页签时读一次 `run/active-preset.json`。
 *
 * A3 修复的另一半：反填（上面那条）只回答"选中哪台机器"，显示层还要拿底账的
 * **fileName** 说"正在使用的是哪一份" —— 否则应用了「我的文件」，首页 / 校准页
 * 仍旧显示目录里那份官方底稿，两个界面两种答案。节奏与 `selectionFromActive`
 * 同一条：回页签时对一次，不设全局 store（仓库没有那个机制，也不需要）。
 */
export function useActivePresetOnTab(tabActive: boolean | undefined): ActivePreset | null {
  const [entry, setEntry] = useState<ActivePreset | null>(null)
  useEffect(() => {
    if (!tabActive) return
    let alive = true
    void api
      .getActivePreset()
      .then((a) => {
        if (alive) setEntry(a)
      })
      .catch(() => {
        if (alive) setEntry(null)
      })
    return () => {
      alive = false
    }
  }, [tabActive])
  return entry
}

/**
 * 底账是否正指着这台机型的这个版本。是 → 显示层用底账的 fileName（与预设页同源）；
 * 不是 → 维持"选中 combo 的目录文件"（预设页横幅仍是"正在使用"的唯一权威）。
 */
export function activeForSelection(
  entry: ActivePreset | null,
  model: string | null,
  variant: string | null,
): ActivePreset | null {
  if (entry === null || model === null || variant === null) return null
  return entry.machineId === model && entry.versionId === variant ? entry : null
}
