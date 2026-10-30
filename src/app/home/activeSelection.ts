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

/**
 * **校准的基准**：底账正指着这一版，而且那一份是**本机文件**（`origin === 'local'`）。
 *
 * 为什么比 [`activeForSelection`] 更严（2026-10-30，作者截图：没下载也没使用，第三步
 * 已经有 X -0.70 / Y 26.30 / Z 4.00，右上角还写着抽屉里根本没有的 `A1-standard.toml`）：
 *
 * - 校准读写的对象是 `presets-local/` 里那份**工作副本**。没有本机副本时
 *   `usePreset` 只能回落到注册表的**云端默认值** —— 那是出厂值，不是"你的"校准值，
 *   把它摆在读数位上就等于替用户编了一个基准。
 * - 云端原件（`origin === 'preset'`）即便当时在盘上有字节，`getLocalCopyFor` 也读不到它，
 *   照样只会给出上面那套默认值；名字又来自目录，抽屉（只列本机文件）里自然查不到。
 *
 * 于是"名字"与"数值"共用这一个判据：没有基准时卡片只摆「选择预设」，一个数都不露。
 */
export function localActiveForSelection(
  entry: ActivePreset | null,
  model: string | null,
  variant: string | null,
): ActivePreset | null {
  const hit = activeForSelection(entry, model, variant)
  return hit !== null && hit.origin === 'local' ? hit : null
}
