/*
 * 产物解析的三件事：涂胶块（层高从 `;MKP_BEGIN paint … z=` 来）、打印本体
 * （`; FEATURE:` 分类、空走单列、按对象分桶、按层排好给层滑块切）、涂胶整体平移。
 *
 * 用合成 fixture：真产物的分类分布随模型变，拿它当断言只会天天红。
 */

import { describe, expect, it } from 'vitest'

import { parseBlocks, parsePrint, shiftBlocks } from './parse'

const SAMPLE = [
  '; start printing object, unique label id: 101',
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
  '; stop printing object, unique label id: 101',
  '; start printing object, unique label id: 202',
  'G1 Z1.0',
  '; FEATURE: Outer wall',
  'G1 X10 Y20 E0.80000',
  '; stop printing object, unique label id: 202',
  ';MKP_BEGIN paint ev=1 z=0.6',
  'G1 Z6.85',
  'G1 X1 Y1',
  'G1 X2 Y2',
  ';MKP_END paint',
].join('\n')

const kindsOf = (text: string, id: number) => {
  const print = parsePrint(text)
  const obj = print.objects.find((o) => o.id === id)
  if (!obj) throw new Error(`fixture 里没有对象 ${id}`)
  return obj.kinds
}

describe('parsePrint', () => {
  it('按 `; FEATURE:` 分类，支撑拆成支撑 / 支撑面 / 熨烫', () => {
    const kinds = kindsOf(SAMPLE, 101)
    /* 每段两个端点 × 三个分量 */
    expect(kinds.outer.length / 6).toBe(2)
    expect(kinds.inner.length / 6).toBe(1)
    expect(kinds.fill.length / 6).toBe(0)
    expect(kinds.support.length / 6).toBe(1)
    expect(kinds.supportFace.length / 6).toBe(1)
    expect(kinds.ironing.length / 6).toBe(1)
    expect(kinds.other.length / 6).toBe(1)
    expect(parsePrint(SAMPLE).unknown).toEqual(['Custom'])
  })

  it('不带 E 的移动进空走，且不算进 Z 范围', () => {
    const print = parsePrint(SAMPLE)
    const kinds = kindsOf(SAMPLE, 101)
    expect(kinds.travel.length / 6).toBe(1)
    /* 空走那行是 F3000，不影响层范围；涂胶块里的移动也不算 */
    expect(print.zMin).toBeCloseTo(0.6)
    expect(print.zMax).toBeCloseTo(1.0)
  })

  it('按 `label id` 分成两个对象，层索引全站共用一份', () => {
    const print = parsePrint(SAMPLE)
    expect(print.objects.map((o) => o.id)).toEqual([101, 202])
    expect(print.layers).toEqual([0.6, 1])
    const a = print.objects[0]
    const b = print.objects[1]
    /* 对象 101 只在第 0 层有外轮廓，202 只在第 1 层 */
    expect([...(a?.bounds.outer ?? [])]).toEqual([0, 2, 2])
    expect([...(b?.bounds.outer ?? [])]).toEqual([0, 0, 1])
    for (const obj of print.objects) {
      for (const kind of Object.keys(obj.bounds) as (keyof typeof obj.bounds)[]) {
        const edge = obj.bounds[kind]
        expect(edge.length).toBe(print.layers.length + 1)
        expect(edge[edge.length - 1]).toBe(obj.kinds[kind].length / 6)
      }
    }
  })

  it('没有对象标记的移动落在 id 0 上', () => {
    const print = parsePrint(['G1 X0 Y0', 'G1 X10 Y0 E0.1'].join('\n'))
    expect(print.objects.map((o) => o.id)).toEqual([0])
    expect(print.objects[0]?.kinds.travel.length / 6).toBe(1)
  })
})

describe('parseBlocks / shiftBlocks', () => {
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

  it('整体平移：坐标与笔尖 Z 跟着走，层高不动', () => {
    const block = parseBlocks(SAMPLE)[0]
    if (!block) throw new Error('fixture 没解析出块')
    const moved = shiftBlocks([block], { dx: 2, dy: -3, dz: -4 })[0]
    if (!moved) throw new Error('平移后没了')
    expect(moved.moves[0]?.from.x).toBeCloseTo(block.moves[0]!.from.x + 2)
    expect(moved.moves[0]?.from.y).toBeCloseTo(block.moves[0]!.from.y - 3)
    expect(moved.moves[0]?.z).toBeCloseTo(block.moves[0]!.z - 4)
    expect(moved.normalZ).toBeCloseTo((block.normalZ ?? 0) - 4)
    /* 这一块属于哪一层与笔尖高低无关 */
    expect(moved.layerZ).toBeCloseTo(0.6)
    expect(moved.liftMm).toBeCloseTo(block.liftMm)
    expect(moved.bbox?.minX).toBeCloseTo((block.bbox?.minX ?? 0) + 2)
  })

  it('平移 0 时原样返回', () => {
    const block = parseBlocks(SAMPLE)[0]
    if (!block) throw new Error('fixture 没解析出块')
    const same = shiftBlocks([block], { dx: 0, dy: 0, dz: 0 })[0]
    expect(same?.moves[0]?.from).toEqual(block.moves[0]?.from)
  })
})
