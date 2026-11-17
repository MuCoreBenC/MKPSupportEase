/*
 * 试验台判据的合成 fixture（与 `edge_band.rs` 的单测同源）。
 *
 * 这里钉的是「规则」不是「产物」：真实产物的 iface 里有哪些空走、切点落在哪，
 * 都反推不回来，拿它做一致性基线只会把两份实现的差异记成期望值。
 */

import { describe, expect, it } from 'vitest'

import { EdgeField } from './band'
import type { EdgeRule } from './band'
import { landedMoves, parseBlocks } from './parse'
import type { LabBlock } from './parse'
import { NOMINAL_PITCH_MM, estimatePitch } from './pitch'

const NORMAL_Z = 6.85
const RAISED_Z = 7.35
const HOP_Z = 9.0

function rule(lines: number, pitchMm = 0.4): EdgeRule {
  return { lines, pitchMm }
}

/** 把一串行包成一块产物；`z` 是默认落笔档 */
function paintBlock(body: string[], z = NORMAL_Z): LabBlock {
  const text = [';MKP_BEGIN paint', `G1 Z${z}`, ...body, ';MKP_END paint'].join('\n')
  const block = parseBlocks(text)[0]
  if (!block) throw new Error('fixture 没解析出块')
  return block
}

/** 蛇形填充一块 `w × h` 的矩形：每行和它前一行的方向相反，折返是一条长度为线距的短移动 */
function serpentine(w: number, h: number, pitch: number, x0 = 100, y0 = 100): string[] {
  const rows = Math.max(1, Math.round(h / pitch))
  const out: string[] = []
  for (let row = 0; row <= rows; row += 1) {
    const y = y0 + (row * h) / rows
    const ltr = row % 2 === 0
    const a = ltr ? x0 : x0 + w
    const b = ltr ? x0 + w : x0
    out.push(`G1 X${a.toFixed(3)} Y${y.toFixed(3)}`)
    out.push(`G1 X${b.toFixed(3)} Y${y.toFixed(3)}`)
  }
  return out
}

/** 40×40 实心面，中间挖 20×20 的洞：洞把每一行切成两段，中间是一次抬笔横移 */
function ringBody(x0 = 100, y0 = 100): string[] {
  const out: string[] = []
  for (let row = 0; row <= 100; row += 1) {
    const y = y0 + row * 0.4
    const ltr = row % 2 === 0
    if (y > y0 + 10 && y < y0 + 30) {
      const [a, b, c, d] = ltr
        ? [x0, x0 + 10, x0 + 30, x0 + 40]
        : [x0 + 40, x0 + 30, x0 + 10, x0]
      out.push(`G1 X${a} Y${y.toFixed(3)}`, `G1 X${b} Y${y.toFixed(3)}`)
      out.push(`G1 Z${HOP_Z}`, `G1 X${c} Y${y.toFixed(3)}`, `G1 Z${NORMAL_Z}`)
      out.push(`G1 X${d} Y${y.toFixed(3)}`)
    } else {
      const [a, b] = ltr ? [x0, x0 + 40] : [x0 + 40, x0]
      out.push(`G1 X${a} Y${y.toFixed(3)}`, `G1 X${b} Y${y.toFixed(3)}`)
    }
  }
  return out
}

function fieldOf(block: LabBlock): EdgeField {
  const field = EdgeField.build(landedMoves(block))
  if (!field) throw new Error('fixture 建不出格子')
  return field
}

describe('产物解析', () => {
  it('两档落笔 + 段间平飞：抬高档认得出，段按落笔连续段切', () => {
    const block = paintBlock(
      [
        `G1 X100 Y100`,
        `G1 X110 Y100`,
        `G1 Z${HOP_Z}`,
        `G1 X110 Y105`,
        `G1 Z${RAISED_Z}`,
        `G1 X100 Y105`,
      ],
      NORMAL_Z,
    )
    expect(block.normalZ).toBe(NORMAL_Z)
    expect(block.raisedZ).toBe(RAISED_Z)
    expect(block.liftMm).toBeCloseTo(0.5, 6)
    expect(block.segments).toHaveLength(2)
    expect(block.moves.map((m) => m.level)).toEqual(['normal', 'normal', 'up', 'raised'])
  })

  it('只有一档时不猜抬高档，把原因写进 note', () => {
    const block = paintBlock(['G1 X100 Y100', 'G1 X110 Y100'])
    expect(block.raisedZ).toBeNull()
    expect(block.note).toContain('一档')
  })
})

describe('线距实测', () => {
  it('0.4 行距的蛇形面：误差小于 10%', () => {
    const block = paintBlock(serpentine(40, 40, 0.4))
    const pitch = estimatePitch(landedMoves(block))
    expect(pitch.fallback).toBe(false)
    expect(Math.abs(pitch.mm - 0.4)).toBeLessThan(0.04)
  })

  it('样本不够就回落名义线距，并报出来', () => {
    const block = paintBlock(['G1 X100 Y100', 'G1 X140 Y100'])
    const pitch = estimatePitch(landedMoves(block))
    expect(pitch.fallback).toBe(true)
    expect(pitch.mm).toBe(NOMINAL_PITCH_MM)
  })
})

describe('边缘带判定', () => {
  it('40 方形：外圈算边、正中不算', () => {
    const field = fieldOf(paintBlock(serpentine(40, 40, 0.4)))
    const r = rule(2)
    expect(field.isEdge({ x: 100.05, y: 120 }, r)).toBe(true)
    expect(field.isEdge({ x: 120, y: 120 }, r)).toBe(false)
    expect(field.bandMm(field.faceShortMm({ x: 120, y: 120 }), r)).toBeCloseTo(0.8, 6)
  })

  it('环的内沿也是边界（外接矩形那套做不到的那一半）', () => {
    const block = paintBlock(ringBody())
    expect(block.segments.length).toBeGreaterThan(1)
    const field = fieldOf(block)
    const r = rule(2)
    expect(field.isEdge({ x: 100.5, y: 120 }, r)).toBe(true)
    expect(field.isEdge({ x: 110.5, y: 120 }, r)).toBe(true)
    expect(field.isEdge({ x: 105, y: 120 }, r)).toBe(false)
  })

  it('6.75×1.25 的窄条：两条就够把它整条抬起来，中间也不留缝', () => {
    /* 这一条是这次改判的根据：百分比带宽 0.125mm 比格子还细，窄条整条漏掉 */
    const block = paintBlock(serpentine(6.75, 1.25, 0.4, 121.97, 98.08))
    const field = fieldOf(block)
    const r = rule(2)
    /* 短边含 3×3 画笔的膨胀（两侧各一格），所以量出来比 1.25 大 */
    const short = field.faceShortMm({ x: 125, y: 98.7 })
    expect(short).toBeLessThan(2.5)
    /* 中线离两边各 0.625mm，带宽封顶在半张面 ⇒ 中间也在带里 */
    expect(field.isEdge({ x: 125, y: 98.7 }, r)).toBe(true)
    expect(field.isEdge({ x: 123, y: 98.2 }, r)).toBe(true)
    /* 0 条 = 关 */
    expect(field.isEdge({ x: 125, y: 98.7 }, rule(0))).toBe(false)
  })

  it('只由边界构成的一圈（8.7×5.8）：整圈都算边', () => {
    const block = paintBlock([
      `G1 X114.216 Y98.337`,
      `G1 X122.924 Y98.337`,
      `G1 X122.924 Y104.169`,
      `G1 X114.216 Y104.169`,
      `G1 X114.216 Y98.337`,
    ])
    const field = fieldOf(block)
    for (const p of [
      { x: 118.5, y: 98.34 },
      { x: 122.92, y: 101 },
      { x: 118.5, y: 104.17 },
      { x: 114.22, y: 101 },
    ]) {
      expect(field.isEdge(p, rule(2))).toBe(true)
    }
  })

  it('带宽不超过半张面：大面上两条就是 0.8mm，不是按面尺寸放大', () => {
    const big = fieldOf(paintBlock(serpentine(200, 200, 0.4, 0, 0)))
    const short = big.faceShortMm({ x: 100, y: 100 })
    expect(big.bandMm(short, rule(2))).toBeCloseTo(0.8, 6)
    expect(big.isEdge({ x: 0.6, y: 100 }, rule(2))).toBe(true)
    expect(big.isEdge({ x: 1.2, y: 100 }, rule(2))).toBe(false)
  })
})
