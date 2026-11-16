/*
 * 产物 ↔ 后处理原件的行级对齐。
 *
 * 后处理是「按行改写」：原件被原样带走，中间插入涂胶块 / 标记行，删掉擦拭与熨烫，
 * 再改掉一部分数值（E、Z 声明）。所以对齐的目标不是「找出最短编辑脚本」，
 * 而是**给产物每一行找到原件里的那一行**，找不到就说找不到（新增），
 * 别硬配一对 —— 面板上「新增」比一行错配有用得多。
 *
 * 两步：
 *   ① 唯一行锚点（patience）：两边都只出现一次的行，按出现顺序取最长递增子序列；
 *   ② 锚点之间：局部位置表 + 双向找下一处匹配，谁更近就认谁 ——
 *      一段新插入的涂胶块在原件里根本没有，于是整段判「新增」后一次跳到下一处；
 *      删掉的几行则反过来整段判「删除」；两边都接不上才按同位置「已改」。
 *
 * 不用 Myers：8 万行、插入为主，O(ND) 不可控；这里对锚点之间的局部窗口做表查找，
 * 总代价 O((n + m) log n)。
 */

/** 两侧都一致 */
export const DIFF_SAME = 0
/** 产物里有、原件里没有（后处理插入的） */
export const DIFF_ADDED = 1
/** 原件里有、产物里删掉了 */
export const DIFF_REMOVED = 2
/** 同位置、数值被改写 */
export const DIFF_CHANGED = 3

export interface DiffStats {
  added: number
  removed: number
  changed: number
}

export interface LineAlignment {
  /** 产物行 → 原件行；-1 = 原件没有这一行 */
  origOf: Int32Array
  /** 原件行 → 产物行；-1 = 产物里删掉了 */
  afterOf: Int32Array
  /** 产物行的差异种类：SAME / ADDED / CHANGED */
  afterKind: Uint8Array
  /** 原件行的差异种类：SAME / REMOVED / CHANGED */
  beforeKind: Uint8Array
  /** 产物行 → 这段之前最后一处匹配的原件行；-1 = 第一处匹配之前 */
  anchorOf: Int32Array
  stats: DiffStats
}

interface Span {
  a0: number
  a1: number
  b0: number
  b1: number
}

class Builder {
  readonly before: readonly string[]
  readonly after: readonly string[]
  readonly origOf: Int32Array
  readonly afterOf: Int32Array
  readonly afterKind: Uint8Array
  readonly beforeKind: Uint8Array
  private added = 0
  private removed = 0
  private changed = 0

  constructor(before: readonly string[], after: readonly string[]) {
    this.before = before
    this.after = after
    this.origOf = new Int32Array(after.length).fill(-1)
    this.afterOf = new Int32Array(before.length).fill(-1)
    this.afterKind = new Uint8Array(after.length)
    this.beforeKind = new Uint8Array(before.length)
  }

  matchBeforeAfter(bi: number, ai: number): void {
    this.afterOf[bi] = ai
    this.origOf[ai] = bi
  }

  /** 同位置但内容不同：两边各记一次「已改」，也互相当作映射 */
  change(bi: number, ai: number): void {
    this.matchBeforeAfter(bi, ai)
    this.afterKind[ai] = DIFF_CHANGED
    this.beforeKind[bi] = DIFF_CHANGED
    this.changed += 1
  }

  add(ai: number): void {
    this.afterKind[ai] = DIFF_ADDED
    this.added += 1
  }

  remove(bi: number): void {
    this.beforeKind[bi] = DIFF_REMOVED
    this.removed += 1
  }

  stats(): DiffStats {
    return { added: this.added, removed: this.removed, changed: this.changed }
  }
}

/** 位置表：行内容 → 升序的行号数组（只在当前这段里建） */
type Positions = Map<string, number[]>

function positionsOf(lines: readonly string[], from: number, to: number): Positions {
  const map: Positions = new Map()
  for (let i = from; i < to; i += 1) {
    const s = lines[i]
    if (s === undefined) continue
    const list = map.get(s)
    if (list === undefined) map.set(s, [i])
    else list.push(i)
  }
  return map
}

/** 位置表里第一个 >= from 且 < to 的位置；没有就是 -1 */
function nextIn(map: Positions, line: string, from: number, to: number): number {
  const list = map.get(line)
  if (list === undefined) return -1
  let lo = 0
  let hi = list.length
  while (lo < hi) {
    const mid = (lo + hi) >> 1
    if ((list[mid] as number) < from) lo = mid + 1
    else hi = mid
  }
  const v = list[lo]
  return v !== undefined && v < to ? v : -1
}

/** 锚点之间：先按局部位置表跳，跳不动才一比一判「已改」 */
function fillSpan(b: Builder, span: Span): void {
  const { a0, a1, b0, b1 } = span
  if (a0 >= a1 && b0 >= b1) return
  if (a0 >= a1) {
    for (let j = b0; j < b1; j += 1) b.add(j)
    return
  }
  if (b0 >= b1) {
    for (let i = a0; i < a1; i += 1) b.remove(i)
    return
  }
  const beforePos = positionsOf(b.before, a0, a1)
  const afterPos = positionsOf(b.after, b0, b1)

  let i = a0
  let j = b0
  while (i < a1 && j < b1) {
    if (b.before[i] === b.after[j]) {
      b.matchBeforeAfter(i, j)
      i += 1
      j += 1
      continue
    }
    const jumpAfter = nextIn(afterPos, b.before[i] as string, j + 1, b1)
    const jumpBefore = nextIn(beforePos, b.after[j] as string, i + 1, a1)
    const costAfter = jumpAfter < 0 ? Infinity : jumpAfter - j
    const costBefore = jumpBefore < 0 ? Infinity : jumpBefore - i

    if (costAfter <= costBefore && jumpAfter >= 0) {
      for (let k = j; k < jumpAfter; k += 1) b.add(k)
      j = jumpAfter
      continue
    }
    if (jumpBefore >= 0) {
      for (let k = i; k < jumpBefore; k += 1) b.remove(k)
      i = jumpBefore
      continue
    }
    b.change(i, j)
    i += 1
    j += 1
  }
  for (; i < a1; i += 1) b.remove(i)
  for (; j < b1; j += 1) b.add(j)
}

/** 两边都只出现一次的行 → 按顺序取最长递增子序列，返回 (原件行, 产物行) 升序 */
function uniqueAnchors(before: readonly string[], after: readonly string[]): [number, number][] {
  const beforeOnce = new Map<string, number>()
  for (let i = 0; i < before.length; i += 1) {
    const s = before[i] as string
    beforeOnce.set(s, beforeOnce.has(s) ? -1 : i)
  }
  const afterOnce = new Map<string, number>()
  for (let j = 0; j < after.length; j += 1) {
    const s = after[j] as string
    afterOnce.set(s, afterOnce.has(s) ? -1 : j)
  }

  const candA: number[] = []
  const candB: number[] = []
  for (let j = 0; j < after.length; j += 1) {
    const bi = beforeOnce.get(after[j] as string)
    if (bi === undefined || bi < 0) continue
    if (afterOnce.get(after[j] as string) !== j) continue
    candA.push(bi)
    candB.push(j)
  }
  if (candA.length === 0) return []

  /* 按 candA 严格递增取 LIS（candB 已是升序） */
  const tails: number[] = []
  const prev = new Int32Array(candA.length).fill(-1)
  for (let k = 0; k < candA.length; k += 1) {
    const v = candA[k] as number
    let lo = 0
    let hi = tails.length
    while (lo < hi) {
      const mid = (lo + hi) >> 1
      if ((candA[tails[mid] as number] as number) < v) lo = mid + 1
      else hi = mid
    }
    if (lo > 0) prev[k] = tails[lo - 1] as number
    tails[lo] = k
  }

  const out: [number, number][] = []
  let k = tails[tails.length - 1] as number
  while (k >= 0) {
    out.push([candA[k] as number, candB[k] as number])
    k = prev[k] as number
  }
  out.reverse()
  return out
}

/** 产物行 → 这段之前最后一处匹配的原件行（新增行靠它决定「原件停在哪」） */
function anchorTable(origOf: Int32Array): Int32Array {
  const anchor = new Int32Array(origOf.length).fill(-1)
  let last = -1
  for (let i = 0; i < origOf.length; i += 1) {
    const v = origOf[i] as number
    if (v >= 0) last = v
    anchor[i] = last
  }
  return anchor
}

/** 两份行数组做行级对齐。任一侧为空都返回一条空映射，不抛错 */
export function alignLines(before: readonly string[], after: readonly string[]): LineAlignment {
  const b = new Builder(before, after)
  const anchors = uniqueAnchors(before, after)
  let bi = 0
  let ai = 0
  for (const [anchorBefore, anchorAfter] of anchors) {
    fillSpan(b, { a0: bi, a1: anchorBefore, b0: ai, b1: anchorAfter })
    b.matchBeforeAfter(anchorBefore, anchorAfter)
    bi = anchorBefore + 1
    ai = anchorAfter + 1
  }
  fillSpan(b, { a0: bi, a1: before.length, b0: ai, b1: after.length })

  return {
    origOf: b.origOf,
    afterOf: b.afterOf,
    afterKind: b.afterKind,
    beforeKind: b.beforeKind,
    anchorOf: anchorTable(b.origOf),
    stats: b.stats(),
  }
}
