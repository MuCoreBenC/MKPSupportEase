/*
 * 交付产物状态的**唯一文案口径**（生成页与「本次交付明细」模态框共用）。
 *
 * 状态词、色调、分组摘要全部从后端 `DeliveryFile.diff` / `DeliveryGroup` 的计数推导，
 * 两边只把语义映射成各自的 CSS module 类名，不各自再判定一次「谁新谁旧」。
 */
import type { DeliveryFile, DeliveryGroup, DiffKind } from '../api'

export type StateTone = 'ok' | 'todo' | 'muted'

/** 一行交付文件的状态词：发布定稿 / 版本交付独占的那几份单说「由谁管」 */
export function stateText(f: DeliveryFile): string {
  if (f.diff === 'frozen') {
    return f.stage === 'software' ? '由版本交付管理' : '由发布定稿'
  }
  switch (f.diff) {
    case 'same':
      return '已生成'
    case 'added':
      return '未生成'
    case 'modified':
      return '待更新'
    case 'removed':
      return '待删除'
    case 'unmanaged':
      return '未纳管'
  }
}

/** 色调语义：页面与模态框各自映射成自己的类名 */
export function stateTone(f: DeliveryFile): StateTone {
  if (f.diff === 'same') return 'ok'
  if (f.diff === 'modified' || f.diff === 'removed' || f.diff === 'added') return 'todo'
  return 'muted'
}

/** 组里真正要动手的份数（已生成 / 发布定稿不算） */
export function groupTodo(g: DeliveryGroup): number {
  return g.modified + g.added + g.removed + g.unmanaged
}

/**
 * 折叠后仍要看得懂的完整摘要：各类非零计数逐项列出，末尾给一致的份数。
 * 数字全部来自后端分组摘要，前端不数文件、不猜状态。
 */
export function groupSummary(g: DeliveryGroup): string {
  /* 全一致组不写「已生成」：发布定稿 / 版本交付那几份不是生成物 */
  if (g.allSame) return `全部一致 · ${g.total} 份`
  const parts: string[] = []
  if (g.modified > 0) parts.push(`待更新 ${g.modified}`)
  if (g.added > 0) parts.push(`未生成 ${g.added}`)
  if (g.removed > 0) parts.push(`待删除 ${g.removed}`)
  if (g.unmanaged > 0) parts.push(`未纳管 ${g.unmanaged}`)
  const same = g.total - groupTodo(g)
  if (same > 0) parts.push(`已生成 ${same}`)
  return parts.join(' · ')
}

/** 差异类型 → 排序权重：要动手的在前，一致的最后 */
export function rankOfDiff(kind: DiffKind): number {
  switch (kind) {
    case 'modified':
    case 'removed':
      return 0
    case 'added':
      return 1
    case 'unmanaged':
      return 2
    case 'frozen':
      return 3
    case 'same':
      return 4
  }
}

export function fmtSize(n: number | null): string {
  if (n === null) return '—'
  if (n < 1024) return `${n} B`
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`
  return `${(n / (1024 * 1024)).toFixed(1)} MB`
}

/** 预设产物显示文件名，其余显示交付相对路径 */
export function displayName(rel: string): string {
  return rel.startsWith('mkp/presets/') ? (rel.split('/').pop() ?? rel) : rel
}

/** 组内排序：待动手的在前，同档按路径稳定排 */
export function sortFiles<T extends { rel: string; diff: DiffKind }>(files: T[]): T[] {
  return [...files].sort(
    (a, b) => rankOfDiff(a.diff) - rankOfDiff(b.diff) || a.rel.localeCompare(b.rel),
  )
}
