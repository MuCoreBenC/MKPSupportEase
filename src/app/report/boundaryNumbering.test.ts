import { describe, expect, it } from 'vitest'

import { boundaryNumbering, moveDistanceMm, overflows } from './boundaryNumbering'
import type { ReportBoundaryPiece } from '../../api'

/** 一块模型：框 + 建议位移（`[0, 0]` = 没越界） */
const piece = (x: number, y: number, shift: [number, number] = [0, 0]): ReportBoundaryPiece => ({
  bbox: { xMin: x, xMax: x + 10, yMin: y, yMax: y + 10 },
  polygon: [
    [x, y],
    [x + 10, y],
    [x + 10, y + 10],
  ],
  shift,
})

describe('越界模型的编号与距离', () => {
  it('只有越界的块编号，没越界的不占号', () => {
    const list = boundaryNumbering([piece(0, 0), piece(50, 50, [0, -9]), piece(100, 100)])
    expect(list).toHaveLength(1)
    expect(list[0].n).toBe(1)
    expect(list[0].piece.bbox?.xMin).toBe(50)
  })

  it('上→下、左→右：先按 y 降序，同一行按 x 升序', () => {
    const list = boundaryNumbering([
      piece(100, 10, [0, -5]), // 最下
      piece(80, 120, [0, -5]), // 最上、靠右
      piece(10, 120, [0, -5]), // 最上、靠左
      piece(40, 118, [0, -5]), // 与上一行差 2mm，算同一行
    ])
    expect(list.map((p) => p.piece.bbox?.xMin)).toEqual([10, 40, 80, 100])
    expect(list.map((p) => p.n)).toEqual([1, 2, 3, 4])
  })

  it('距离是位移的模，保留一位小数', () => {
    expect(moveDistanceMm(piece(0, 0, [0, -22.1]))).toBe(22.1)
    expect(moveDistanceMm(piece(0, 0, [3, 4]))).toBe(5)
    expect(moveDistanceMm(piece(0, 0, [0, 0]))).toBe(0)
  })

  it('越界判定只看位移是否为零', () => {
    expect(overflows(piece(0, 0, [0, -0.1]))).toBe(true)
    expect(overflows(piece(0, 0, [0, 0]))).toBe(false)
    expect(overflows({})).toBe(false)
  })
})
