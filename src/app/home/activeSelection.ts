/*
 * 首页联动：把「正在使用的那一条」（唯一底账 `STORAGE.clientActive`）翻成首页三级选择。
 *
 * 作者定的关系：**应用哪一份预设，首页就变成哪一台** —— 预设页「应用」之后切回首页，
 * 品牌 / 机型 / 打印件版本三级要跟过来；刷新后仍在（底账落本机）。
 *
 * # 映射表没了，这就是一次「查得到就填」
 *
 * 首页原来还吃手编表（`a1mini` / `std`），底账里是契约 id（`A1_MINI` / `STANDARD`），
 * 于是写了一张映射表。首页改成也消费文件体系之后，**两边是同一套 id** ——
 * 机器在目录里、版本在那台机器的版本表里，直接填；查不到（目录里没有这台 / 这个版本）
 * 就整体不反填，不硬凑。
 */

import { activeEntry } from '../store/package'
import type { Machine } from '../../api/contract'
import type { Selection } from './MachinePicker'

/**
 * 读一次底账并翻成首页的三级选择。`null` = 不反填（没有已应用 / 机器或版本不在目录里）。
 *
 * `machines` 必须是**已就绪**的目录（空数组时不要调）—— 反填要在目录里查得到才算数。
 */
export function selectionFromActive(machines: Machine[]): Selection | null {
  const entry = activeEntry()
  if (entry === null || entry.machineId === null) return null

  const machine = machines.find((m) => m.id === entry.machineId)
  if (machine === undefined) return null

  const version =
    entry.versionId === null ? undefined : machine.versions.find((v) => v.id === entry.versionId)

  return { brand: machine.brand, model: machine.id, variant: version?.id ?? null }
}
