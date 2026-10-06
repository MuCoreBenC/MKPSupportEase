/*
 * 首页联动：把「正在使用的那一条」（AppState 的 activePreset 格，唯一底账）
 * 翻成首页三级选择。
 *
 * 作者定的关系：**应用哪一份预设，首页就变成哪一台** —— 预设页「应用」之后，
 * 品牌 / 机型 / 打印件版本三级要跟过来；刷新后仍在（AppState 落盘，不是内存）。
 *
 * # 映射表没了，这就是一次「查得到就填」
 *
 * 首页原来还吃手编表（`a1mini` / `std`），底账里是契约 id（`A1_MINI` / `STANDARD`），
 * 于是写了一张映射表。首页改成也消费文件体系之后，**两边是同一套 id** ——
 * 机器在目录里、版本在那台机器的版本表里，直接填；查不到（目录里没有这台 / 这个版本）
 * 就整体不反填，不硬凑。
 *
 * # A3 的第一轮补丁在这里退役（2026-10-06）
 *
 * 第一轮加过 `useActivePresetOnTab`（回页签时对一次底账）—— 那是"每页一份快照 +
 * 回页签对账"的同步机制。读取/订阅收进唯一客户端（`../state/appState.ts`，
 * `docs/APP-STATE.md`）之后它就没用了：页面订阅 `useActivePreset()`，底账一变
 * （应用 / 撤销 / 改名 / 删除）所有页面同帧换账。这里只剩两个**纯翻译**：
 * 底账 → 三级选择（[`selectionFromActive`]）、底账 → 是否命中当前 combo
 * （[`activeForSelection`]）。
 */

import type { ActivePreset, Machine } from '../../api/contract'
import type { Selection } from './MachinePicker'

/**
 * 把底账翻成首页的三级选择。`null` = 不反填（没有已应用 / 机器或版本不在目录里）。
 *
 * `machines` 必须是**已就绪**的目录（空数组时不要调）—— 反填要在目录里查得到才算数。
 */
export function selectionFromActive(
  machines: Machine[],
  entry: ActivePreset | null,
): Selection | null {
  if (entry === null) return null

  const machine = machines.find((m) => m.id === entry.machineId)
  if (machine === undefined) return null

  const version = machine.versions.find((v) => v.id === entry.versionId)

  return { brand: machine.brand, model: machine.id, variant: version?.id ?? null }
}

/**
 * 底账是否正指着这台机型的这个版本。是 → 显示层用底账的 fileName（与预设页同源）；
 * 不是 → 维持"选中 combo 的目录文件"。
 */
export function activeForSelection(
  entry: ActivePreset | null,
  model: string | null,
  variant: string | null,
): ActivePreset | null {
  if (entry === null || model === null || variant === null) return null
  return entry.machineId === model && entry.versionId === variant ? entry : null
}
