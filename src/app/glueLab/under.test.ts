/*
 * 下方支撑余量：覆盖区怎么铺、内孔填不填、余量怎么量。
 *
 * 用合成 fixture：真产物的支撑分布随模型变，拿它当断言只会天天红。
 */

import { describe, expect, it } from 'vitest'

import { parsePrint } from './parse'
import { buildUnder, layerIndexBelow, layerStrokes, marginAt, minMarginOfMove } from './under'
import type { Stroke } from './band'

/** 用 0.4mm 间距的横线铺一个矩形；`skip` 命中处断开（做缺口用） */
function hatch(
  minX: number,
  minY: number,
  maxX: number,
  maxY: number,
  skip?: (x: number, y: number) => boolean,
): Stroke[] {
  const out: Stroke[] = []
  const step = 0.4
  for (let y = minY; y <= maxY + 1e-9; y += step) {
    let start: number | null = null
    for (let x = minX; x <= maxX + 1e-9; x += 0.2) {
      const ok = skip === undefined ? true : !skip(x, y)
      if (ok && start === null) start = x
      if (!ok && start !== null) {
        out.push({ from: { x: start, y }, to: { x, y } })
        start = null
      }
    }
    if (start !== null) out.push({ from: { x: start, y }, to: { x: maxX, y } })
  }
  return out
}

const inHole = (x: number, y: number): boolean => x > 3 && x < 7 && y > 3 && y < 7

describe('buildUnder / marginAt', () => {
  it('孔里有材料（墙/填充）⇒ 填平：正中间也当有料', () => {
    const support = hatch(0, 0, 10, 10, inHole)
    /* 孔里那圈零件本体 */
    const material = [...support, ...hatch(3.5, 3.5, 6.5, 3.5), ...hatch(3.5, 3.5, 3.5, 6.5)]
    const cov = buildUnder(support, material, 0.4)
    expect(cov).not.toBeNull()
    if (cov === null) return
    /* 孔 3..7 被填掉后，中心到外边界约 5mm */
    expect(marginAt(cov, { x: 5, y: 5 })).toBeGreaterThan(3)
  })

  it('真空气孔不填：涂胶走过去仍算贴边', () => {
    const support = hatch(0, 0, 10, 10, inHole)
    const cov = buildUnder(support, support, 0.4)
    expect(cov).not.toBeNull()
    if (cov === null) return
    expect(marginAt(cov, { x: 5, y: 5 })).toBe(0)
  })

  it('与外界连通的缺口不填：U 形里面还是空的', () => {
    const u = [...hatch(0, 0, 10, 1), ...hatch(0, 1, 1, 10), ...hatch(9, 1, 10, 10)]
    const cov = buildUnder(u, u, 0.4)
    expect(cov).not.toBeNull()
    if (cov === null) return
    expect(marginAt(cov, { x: 5, y: 5 })).toBe(0)
    /* 底边的料上是有余量的 */
    expect(marginAt(cov, { x: 5, y: 0.5 })).toBeGreaterThan(0)
  })

  it('一条移动取沿线最小余量：穿到边上就变小', () => {
    const support = hatch(0, 0, 10, 10, inHole)
    const cov = buildUnder(support, [...support, ...hatch(3.5, 3.5, 6.5, 3.5)], 0.4)
    if (cov === null) throw new Error('没有覆盖区')
    const mid = minMarginOfMove(cov, { x: 5, y: 5 }, { x: 5, y: 6 })
    const edge = minMarginOfMove(cov, { x: 0.3, y: 5 }, { x: 5, y: 5 })
    expect(mid).toBeGreaterThan(3)
    expect(edge).toBeLessThan(1)
    expect(edge).toBeGreaterThanOrEqual(0)
  })

  it('点太少或退化成一条线时返回 null', () => {
    expect(buildUnder([], [], 0.4)).toBeNull()
    const flat = [{ from: { x: 0, y: 0 }, to: { x: 10, y: 0 } }]
    expect(buildUnder(flat, flat, 0.4)).toBeNull()
  })
})

describe('layerIndexBelow / layerStrokes', () => {
  const SAMPLE = [
    'G1 Z0.2',
    'G1 X0 Y0',
    '; FEATURE: Support interface',
    'G1 X10 Y10 E0.1',
    'G1 X20 Y10 E0.1',
    'G1 Z0.4',
    'G1 X0 Y0',
    'G1 X10 Y10 E0.1',
    'G1 X30 Y10 E0.1',
  ].join('\n')

  it('取小于这一层的最近一层', () => {
    const print = parsePrint(SAMPLE)
    expect(print.layers).toEqual([0.2, 0.4])
    expect(layerIndexBelow(print, 0.6)).toBe(1)
    expect(layerIndexBelow(print, 0.4)).toBe(0)
    expect(layerIndexBelow(print, 0.1)).toBe(-1)
  })

  it('按层切片：只取这一层的段，跨对象合并', () => {
    const print = parsePrint(SAMPLE)
    const low = layerStrokes(print, 0, ['supportFace'])
    const high = layerStrokes(print, 1, ['supportFace'])
    expect(low).toHaveLength(2)
    expect(high).toHaveLength(2)
    expect(low[0]?.to).toEqual({ x: 10, y: 10 })
    expect(high[1]?.to).toEqual({ x: 30, y: 10 })
    /* 没有这一层时是空表，不是抛错 */
    expect(layerStrokes(print, 5, ['supportFace'])).toHaveLength(0)
    expect(layerStrokes(print, -1, ['supportFace'])).toHaveLength(0)
  })
})
