/*
 * 逐行索引：把「第几行」变成能直接查的表。
 *
 * 试验台的代码面板只显示窗口里的几行，但点一行要立刻知道它是哪条移动、
 * 属于哪一块、当时处在哪种 `; FEATURE:` 下 —— 每次现扫 8 万行来不及。
 * 这里一次遍历建四张 typed array，之后按行号 O(1) 取。
 *
 * 行号是**从 0 开始**的（与 `LabMove.line` 同一口径），显示时加一。
 */

import type { LabBlock } from './parse'
import { PRINT_KINDS, kindOfFeature } from './parse'
import type { PrintKind } from './parse'

/** 一行是什么。数值即存储值，`kindOfLine` 直接存它 */
export const LINE_OTHER = 0
export const LINE_COMMENT = 1
export const LINE_MOVE = 2
export const LINE_BLOCK_BEGIN = 3
export const LINE_BLOCK_END = 4
export const LINE_SWAP_BEGIN = 5
export const LINE_SWAP_END = 6
export const LINE_INFO = 7
export const LINE_STATS = 8
export const LINE_POINT = 9

/** `featureOfLine` 里「不在任何 `; FEATURE:` 之下」的值 */
export const FEATURE_NONE = 255

export interface LineIndex {
  /** 行 → 这一块 `moves[]` 的下标；不是落笔/空走的移动行就是 -1 */
  moveOfLine: Int32Array
  /** 行 → `blocks[]` 的下标（块起止行与块内所有行都算）；块外是 -1 */
  blockOfLine: Int32Array
  /** 行 → 当前 `; FEATURE:` 的类别下标（`PRINT_KINDS`），没有就是 `FEATURE_NONE` */
  featureOfLine: Uint8Array
  /** 行 → `LINE_*` */
  kindOfLine: Uint8Array
}

const FEATURE_INDEX = new Map<PrintKind, number>(PRINT_KINDS.map((k, i) => [k, i]))

/** 产物全文切行；解析与索引共用这一份，别把 1.9MB 拆三次 */
export function splitLines(text: string): string[] {
  return text.split(/\r?\n/)
}

const MKP_PAINT_BEGIN = ';MKP_BEGIN paint'
const MKP_PAINT_END = ';MKP_END paint'
const MKP_SWAP_BEGIN = ';MKP_BEGIN swap'
const MKP_SWAP_END = ';MKP_END swap'

const MOVE_CMD = /^G[0-3] /

function kindOf(trimmed: string): number {
  if (trimmed.startsWith(MKP_PAINT_BEGIN)) return LINE_BLOCK_BEGIN
  if (trimmed.startsWith(MKP_PAINT_END)) return LINE_BLOCK_END
  if (trimmed.startsWith(MKP_SWAP_BEGIN)) return LINE_SWAP_BEGIN
  if (trimmed.startsWith(MKP_SWAP_END)) return LINE_SWAP_END
  if (trimmed.startsWith(';MKP_INFO')) return LINE_INFO
  if (trimmed.startsWith(';MKP_STATS')) return LINE_STATS
  if (trimmed.startsWith(';MKP_POINT')) return LINE_POINT
  if (trimmed.startsWith(';')) return LINE_COMMENT
  if (MOVE_CMD.test(trimmed)) return LINE_MOVE
  return LINE_OTHER
}

/**
 * 建索引。`blocks` 来自 `parseBlocks`（同一份行切出来的），
 * 行数对不上就直接按行数取小 —— 不抛错，面板少几行比整份打不开好。
 */
export function indexLines(lines: readonly string[], blocks: readonly LabBlock[]): LineIndex {
  const n = lines.length
  const moveOfLine = new Int32Array(n).fill(-1)
  const blockOfLine = new Int32Array(n).fill(-1)
  const featureOfLine = new Uint8Array(n).fill(FEATURE_NONE)
  const kindOfLine = new Uint8Array(n)

  let feature: PrintKind | null = null
  for (let i = 0; i < n; i += 1) {
    const t = lines[i].trim()
    if (t.startsWith('; FEATURE:')) {
      feature = kindOfFeature(t.slice('; FEATURE:'.length))
      featureOfLine[i] = FEATURE_INDEX.get(feature) ?? FEATURE_NONE
    } else if (feature !== null) {
      featureOfLine[i] = FEATURE_INDEX.get(feature) ?? FEATURE_NONE
    }
    kindOfLine[i] = kindOf(t)
  }

  blocks.forEach((block, bi) => {
    const last = Math.min(block.endLine, n - 1)
    for (let i = Math.max(0, block.beginLine); i <= last; i += 1) blockOfLine[i] = bi
    block.moves.forEach((m, mi) => {
      if (m.line >= 0 && m.line < n) moveOfLine[m.line] = mi
    })
  })

  return { moveOfLine, blockOfLine, featureOfLine, kindOfLine }
}

/** `featureOfLine` 的数值 → 类别名；`FEATURE_NONE` 返回 null */
export function featureNameOf(index: number): PrintKind | null {
  return PRINT_KINDS[index] ?? null
}
