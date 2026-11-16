/*
 * 产物解析的两件事：涂胶块（层高从 `;MKP_BEGIN paint … z=` 来）与打印本体
 * （`; FEATURE:` 分类、只收带 E 的移动、按层排好给层滑块切）。
 *
 * 用合成 fixture：真产物的分类分布随模型变，拿它当断言只会天天红。
 */

import { describe, expect, it } from 'vitest'

import { parseBlocks, parsePrint } from './parse'

const SAMPLE = [
  '; FEATURE: Outer wall',
  'G1 Z0.6',
  'G1 X10 Y10 E0.10000',
  'G1 X20 Y10 E0.20000',
  '; FEATURE: Inner wall',
  'G1 X10 Y10 F3000',
  'G1 X15 Y10 E0.30000',
  '; FEATURE: Support',
  'G1 X16 Y10 E0.40000',
  '; FEATURE: Support interface',
  'G1 X17 Y10 E0.50000',
  '; FEATURE: Support ironing',
  'G1 X18 Y10 E0.60000',
  '; FEATURE: Custom',
  'G1 X19 Y10 E0.70000',
  'G1 Z1.0',
  '; FEATURE: Outer wall',
  'G1 X10 Y20 E0.80000',
  ';MKP_BEGIN paint ev=1 z=0.6',
  'G1 Z6.85',
  'G1 X1 Y1',
  'G1 X2 Y2',
  ';MKP_END paint',
].join('\n')

describe('parsePrint', () => {
  it('按 `; FEATURE:` 分类，支撑拆成支撑 / 支撑面 / 熨烫', () => {
    const print = parsePrint(SAMPLE)
    /* 每段两个端点 × 三个分量 */
    expect(print.kinds.outer.length / 6).toBe(3)
    expect(print.kinds.inner.length / 6).toBe(1)
    expect(print.kinds.fill.length / 6).toBe(0)
    expect(print.kinds.support.length / 6).toBe(1)
    expect(print.kinds.supportFace.length / 6).toBe(1)
    expect(print.kinds.ironing.length / 6).toBe(1)
    expect(print.kinds.other.length / 6).toBe(1)
    expect(print.unknown).toEqual(['Custom'])
  })

  it('空走（没有 E）不画；涂胶块里的移动不归打印本体', () => {
    const print = parsePrint(SAMPLE)
    const segs = (Object.values(print.kinds) as Float32Array[]).reduce(
      (n, a) => n + a.length / 6,
      0,
    )
    expect(segs).toBe(8)
  })

  it('层升序排好，每类的 bounds 能把任一层切出来', () => {
    const print = parsePrint(SAMPLE)
    expect(print.layers).toEqual([0.6, 1])
    /* 第 0 层两段外轮廓、第 1 层一段 */
    expect([...print.bounds.outer]).toEqual([0, 2, 3])
    /* 支撑只在第 0 层 */
    expect([...print.bounds.support]).toEqual([0, 1, 1])
    for (const kind of Object.keys(print.bounds) as (keyof typeof print.bounds)[]) {
      const edge = print.bounds[kind]
      expect(edge.length).toBe(print.layers.length + 1)
      expect(edge[edge.length - 1]).toBe(print.kinds[kind].length / 6)
    }
  })

  it('Z 取移动当时的层高', () => {
    const print = parsePrint(SAMPLE)
    expect(print.zMin).toBeCloseTo(0.6)
    expect(print.zMax).toBeCloseTo(1.0)
  })
})

describe('parseBlocks', () => {
  it('层高从 `;MKP_BEGIN paint … z=` 读，不是笔尖 Z', () => {
    const blocks = parseBlocks(SAMPLE)
    expect(blocks).toHaveLength(1)
    expect(blocks[0]?.layerZ).toBeCloseTo(0.6)
    expect(blocks[0]?.normalZ).toBeCloseTo(6.85)
  })

  it('没有 `z=` 的老产物退回 null（3D 那边再退到笔尖 Z）', () => {
    const blocks = parseBlocks(
      [';MKP_BEGIN paint', 'G1 Z6.85', 'G1 X1 Y1', ';MKP_END paint'].join('\n'),
    )
    expect(blocks[0]?.layerZ).toBeNull()
  })
})
