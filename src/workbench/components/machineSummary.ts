/*
 * 机型选集的摘要（2026-10-20）。
 *
 * 「适用机型」与「弃用机型」两个按钮上写的那一句，必须是同一套说法 ——
 * 两处各写一遍的话，「全部机型（不限）」在一处、空串在另一处，读起来像两个功能。
 *
 * 单独一个文件是为了过 `react-refresh/only-export-components`：组件文件只导出组件。
 */

import type { MachineList } from '../api'

/**
 * 选集摘要：空 = 不限；全勾 = 明说是全部；否则列前三台。
 */
export function machineSummary(selected: string[], list: MachineList | null): string {
  if (selected.length === 0) return '全部机型（不限）'
  const byId = new Map((list?.machines ?? []).map((m) => [m.id, m]))
  const total = list?.machines.length ?? 0
  if (total > 0 && selected.length === total) return `全部机型（${total} 台全勾）`
  const names = selected.map((id) => byId.get(id)?.display ?? id)
  if (names.length <= 3) return `${names.join('、')} —— ${names.length} 台`
  return `${names.slice(0, 3).join('、')} 等 —— ${names.length} 台`
}
