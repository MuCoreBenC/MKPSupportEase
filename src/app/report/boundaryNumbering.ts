/*
 * 越界模型上的**序号**（2026-11-19）。
 *
 * 图上一块模型一支箭头，右栏逐条说「n 号模型：沿箭头方向移动 xx.x mm」——
 * 两边共用**这一份排序**，所以编号只有一个来源（图上的徽标与文字永远对得上）。
 *
 * 排序规则：**上→下、左→右**（机器坐标 y 向上 ⇒ y 大的先编），同一行里按 x 小的先。
 * 「同一行」的容差取 5mm：几块并排时不会因为 0.2mm 的高低差来回跳。
 */
import type { ReportBoundaryPiece } from '../../api'

/** 一行内判定：两块的中心差在这以内就算同一行 */
const SAME_ROW_MM = 5

export interface NumberedPiece {
  /** 从 1 开始，与图上徽标一致 */
  n: number
  piece: ReportBoundaryPiece
}

/** 这块要不要编号（`shift` 非零 = 真正越界、有箭头的那几块） */
export function overflows(piece: ReportBoundaryPiece): boolean {
  const s = piece.shift
  return s !== undefined && (s[0] !== 0 || s[1] !== 0)
}

/** 建议位移的长度（mm，一位小数）—— 与内核 0.1mm 取整同一个粒度 */
export function moveDistanceMm(piece: ReportBoundaryPiece): number {
  const s = piece.shift ?? [0, 0]
  return Math.round(Math.hypot(s[0], s[1]) * 10) / 10
}

/**
 * 越界块按「上→下、左→右」编号。没越界的块不参与（图上没有箭头，也就不需要号）。
 */
export function boundaryNumbering(pieces: ReportBoundaryPiece[]): NumberedPiece[] {
  const center = (p: ReportBoundaryPiece): [number, number] => {
    const b = p.bbox
    if (b?.xMin === undefined || b.xMax === undefined || b.yMin === undefined || b.yMax === undefined) {
      return [0, 0]
    }
    return [(b.xMin + b.xMax) / 2, (b.yMin + b.yMax) / 2]
  }

  return pieces
    .filter(overflows)
    .map((piece) => ({ piece, c: center(piece) }))
    .sort((a, b) => {
      if (Math.abs(a.c[1] - b.c[1]) > SAME_ROW_MM) return b.c[1] - a.c[1]
      return a.c[0] - b.c[0]
    })
    .map(({ piece }, i) => ({ n: i + 1, piece }))
}
