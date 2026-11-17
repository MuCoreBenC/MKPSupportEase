/*
 * 逐行索引：行 → 移动 / 块 / 特征 / 行种类这四条反查，必须与 `parse.ts` 的
 * 块区间、`; FEATURE:` 的继承口径一致。
 */

import { describe, expect, it } from 'vitest'

import {
  FEATURE_NONE,
  LINE_BLOCK_BEGIN,
  LINE_BLOCK_END,
  LINE_COMMENT,
  LINE_INFO,
  LINE_MOVE,
  LINE_OTHER,
  LINE_STATS,
  LINE_SWAP_BEGIN,
  LINE_SWAP_END,
  featureNameOf,
  indexLines,
  splitLines,
} from './lines'
import { PRINT_KINDS, parseBlocks } from './parse'

const SAMPLE = [
  '; HEADER_BLOCK_END',
  '; CHANGE_LAYER',
  '; Z_HEIGHT: 0.2',
  '; FEATURE: Outer wall',
  'G1 Z0.6',
  'G1 X10 Y10 E0.1',
  '; FEATURE: Inner wall',
  'G1 X11 Y10 E0.2',
  ';MKP_BEGIN paint ev=1 z=0.2',
  'G1 Z6.85',
  'G1 X1 Y1',
  'G1 X2 Y2',
  ';MKP_END paint',
  ';MKP_BEGIN swap ev=2 to=print z=6.9',
  ';MKP_END swap',
  ';MKP_INFO tool=mkp-ssr',
  ';MKP_STATS lines=10',
  ';MKP_STAGE: legacy',
  'M106 P1 S255',
].join('\n')

const index = () => {
  const lines = splitLines(SAMPLE)
  return { lines, idx: indexLines(lines, parseBlocks(SAMPLE)) }
}

describe('indexLines', () => {
  it('块内移动行能反查到 moves 下标，块外移动是 -1', () => {
    const { lines, idx } = index()
    const at = (needle: string): number => lines.findIndex((l) => l === needle)

    expect(idx.moveOfLine[at('G1 X10 Y10 E0.1')]).toBe(-1)
    expect(idx.moveOfLine[at('G1 X1 Y1')]).toBe(0)
    expect(idx.moveOfLine[at('G1 X2 Y2')]).toBe(1)
    /* 只动 Z 的行不产生移动 */
    expect(idx.moveOfLine[at('G1 Z6.85')]).toBe(-1)
  })

  it('块起止与块内每一行都记在块号上，块外是 -1', () => {
    const { lines, idx } = index()
    const begin = lines.findIndex((l) => l.startsWith(';MKP_BEGIN paint'))
    const end = lines.findIndex((l) => l.startsWith(';MKP_END paint'))

    expect(idx.blockOfLine[begin]).toBe(0)
    expect(idx.blockOfLine[end]).toBe(0)
    expect(idx.blockOfLine[begin + 2]).toBe(0)
    expect(idx.blockOfLine[begin - 1]).toBe(-1)
    expect(idx.blockOfLine[end + 1]).toBe(-1)
  })

  it('`; FEATURE:` 往下继承，行种类按 MKP 五种标记分开', () => {
    const { lines, idx } = index()
    const at = (needle: string): number => lines.findIndex((l) => l === needle)
    const outer = PRINT_KINDS.indexOf('outer')
    const inner = PRINT_KINDS.indexOf('inner')

    expect(idx.featureOfLine[at('; FEATURE: Outer wall')]).toBe(outer)
    expect(idx.featureOfLine[at('G1 X10 Y10 E0.1')]).toBe(outer)
    expect(idx.featureOfLine[at('G1 X11 Y10 E0.2')]).toBe(inner)
    expect(idx.featureOfLine[0]).toBe(FEATURE_NONE)
    expect(featureNameOf(outer)).toBe('outer')
    expect(featureNameOf(FEATURE_NONE)).toBeNull()

    expect(idx.kindOfLine[at(';MKP_BEGIN paint ev=1 z=0.2')]).toBe(LINE_BLOCK_BEGIN)
    expect(idx.kindOfLine[at(';MKP_END paint')]).toBe(LINE_BLOCK_END)
    expect(idx.kindOfLine[at(';MKP_BEGIN swap ev=2 to=print z=6.9')]).toBe(LINE_SWAP_BEGIN)
    expect(idx.kindOfLine[at(';MKP_END swap')]).toBe(LINE_SWAP_END)
    expect(idx.kindOfLine[at(';MKP_INFO tool=mkp-ssr')]).toBe(LINE_INFO)
    expect(idx.kindOfLine[at(';MKP_STATS lines=10')]).toBe(LINE_STATS)
    expect(idx.kindOfLine[at(';MKP_STAGE: legacy')]).toBe(LINE_COMMENT)
    expect(idx.kindOfLine[at('G1 X10 Y10 E0.1')]).toBe(LINE_MOVE)
    expect(idx.kindOfLine[at('M106 P1 S255')]).toBe(LINE_OTHER)
  })

  it('空文本不抛错', () => {
    const lines = splitLines('')
    expect(lines.length).toBe(1)
    const idx = indexLines(lines, [])
    expect(idx.moveOfLine[0]).toBe(-1)
    expect(idx.kindOfLine[0]).toBe(LINE_OTHER)
  })
})
