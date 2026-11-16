/*
 * 涂胶块的产物解析（试验台口径）。
 *
 * 服务 `tools/gcode-lab`：把一份产物切成一个个「涂胶块」（`;MKP_BEGIN paint` …
 * `;MKP_END paint`），块内再分出段、移动、落笔档。语法与内核发射端一致：
 *
 *   · 空走也带 `G1 X Y`（产物里没有 E），落笔 / 空走**只能按 Z 分**；
 *   · 段界在产物里看不到 —— `;ZJUMP_START` 是 iface 内部标记，不落地。这里把
 *     「笔在落笔档连续走的一段」当一段：发射端每段落笔前都重新声明档位，所以
 *     段与段之间一定隔着一条空走；
 *   · 落笔两档取块内**最小的两个不同 Z**，相差不超过 `MAX_EDGE_LIFT_MM` 才算
 *     「正常档 / 抬高档」。再大就和段间平飞高度同档了，产物里分不出来 ——
 *     这种情况不猜，记进 `note` 交给界面显示。
 */

export interface Point {
  x: number
  y: number
}

export interface BBox {
  minX: number
  minY: number
  maxX: number
  maxY: number
}

/** 一条移动出发时笔停在哪一档 */
export type PenLevel = 'normal' | 'raised' | 'up'

export interface LabMove {
  /** 产物里的 0 基行号 */
  line: number
  from: Point
  to: Point
  /** 出发时 G-code 里的 Z */
  z: number
  level: PenLevel
}

export interface LabSegment {
  /** 这一段在 `moves[]` 里的下标区间（含两端） */
  first: number
  last: number
  /** 段首那条移动的行号 */
  firstLine: number
}

export interface LabBlock {
  index: number
  beginLine: number
  endLine: number
  moves: LabMove[]
  segments: LabSegment[]
  /** 正常落笔 Z；块内一条落笔移动都没有时是 `null` */
  normalZ: number | null
  /** 抬高档 Z；认不出来时是 `null`，原因在 `note` */
  raisedZ: number | null
  /** 抬笔量（mm）：没认出来就是 0 */
  liftMm: number
  bbox: BBox | null
  note: string | null
}

const BLOCK_BEGIN = ';MKP_BEGIN paint'
const BLOCK_END = ';MKP_END paint'

/**
 * 抬笔量上限（mm）。超过它，抬高档和段间平飞高度就落在同一个数上，从产物里
 * 再也分不出哪一档是「边缘抬高」—— 出现这种情况按「没认出来」处理。
 */
export const MAX_EDGE_LIFT_MM = 2.0

const AXIS = {
  X: /X(-?\d+(?:\.\d*)?)/,
  Y: /Y(-?\d+(?:\.\d*)?)/,
  Z: /Z(-?\d+(?:\.\d*)?)/,
} as const

function axis(line: string, name: keyof typeof AXIS): number | null {
  const m = AXIS[name].exec(line)
  return m ? Number(m[1]) : null
}

interface RawMove {
  line: number
  from: Point
  to: Point
  z: number
}

/** 产物全文 → 涂胶块（没有 `;MKP_BEGIN paint` 时返回空数组） */
export function parseBlocks(text: string): LabBlock[] {
  const lines = text.split(/\r?\n/)
  const blocks: LabBlock[] = []
  let beginLine = -1
  let raws: RawMove[] | null = null
  /*
   * 笔的位置跨块连续：一块结束时笔停在哪，下一块的段首空走就从哪起。
   * `hasPos` 为假时**不编造起点** —— 与内核 `collect_moves` 一致（第一处 XY 自己
   * 就是起点，那条移动是零长度的），否则产物开头那条「从床原点走来」的空走会被
   * 当成一段涂胶线画进网格，把整层的短边撑大。
   */
  let x = 0
  let y = 0
  let z = 0
  let hasPos = false

  for (let i = 0; i < lines.length; i += 1) {
    const t = lines[i].trim()
    if (t.startsWith(BLOCK_BEGIN)) {
      beginLine = i
      raws = []
      continue
    }
    if (t.startsWith(BLOCK_END)) {
      if (raws) blocks.push(finishBlock(blocks.length, beginLine, i, raws))
      raws = null
      continue
    }
    const isMoveCmd =
      t.startsWith('G0 ') || t.startsWith('G1 ') || t.startsWith('G2 ') || t.startsWith('G3 ')
    if (!isMoveCmd) continue

    const nz = axis(t, 'Z')
    const nx = axis(t, 'X')
    const ny = axis(t, 'Y')
    if (nz !== null) z = nz
    /* 只动 Z 的行（抬笔 / 落笔）不产生移动，但它改写后面的档位 */
    if (nx === null && ny === null) continue

    const from = hasPos ? { x, y } : { x: nx ?? x, y: ny ?? y }
    if (nx !== null) x = nx
    if (ny !== null) y = ny
    hasPos = true
    if (raws) raws.push({ line: i, from, to: { x, y }, z })
  }
  return blocks
}

function finishBlock(index: number, beginLine: number, endLine: number, raws: RawMove[]): LabBlock {
  const levels = [...new Set(raws.map((m) => m.z))].sort((a, b) => a - b)
  const normalZ = levels.length > 0 ? levels[0] : null
  let raisedZ: number | null = null
  let note: string | null = null
  if (levels.length > 1) {
    const delta = levels[1] - levels[0]
    if (delta <= MAX_EDGE_LIFT_MM + 1e-9) raisedZ = levels[1]
    else note = `未识别到抬高档：两档相差 ${delta.toFixed(2)}mm`
  } else if (levels.length === 1) {
    note = '这一块只有一档落笔高度'
  } else {
    note = '这一块没有落笔移动'
  }

  const moves: LabMove[] = raws.map((m) => ({
    line: m.line,
    from: m.from,
    to: m.to,
    z: m.z,
    level:
      normalZ !== null && Math.abs(m.z - normalZ) < 1e-9
        ? 'normal'
        : raisedZ !== null && Math.abs(m.z - raisedZ) < 1e-9
          ? 'raised'
          : 'up',
  }))

  const segments: LabSegment[] = []
  for (let i = 0; i < moves.length; i += 1) {
    if (moves[i].level === 'up') continue
    const first = i
    while (i + 1 < moves.length && moves[i + 1].level !== 'up') i += 1
    segments.push({ first, last: i, firstLine: moves[first].line })
  }

  return {
    index,
    beginLine,
    endLine,
    moves,
    segments,
    normalZ,
    raisedZ,
    liftMm: raisedZ !== null && normalZ !== null ? raisedZ - normalZ : 0,
    bbox: landedBBox(moves),
    note,
  }
}

/** 落笔移动的外接框（空走不算，它会把框撑到天上） */
export function landedBBox(moves: readonly LabMove[]): BBox | null {
  let minX = Infinity
  let minY = Infinity
  let maxX = -Infinity
  let maxY = -Infinity
  for (const m of moves) {
    if (m.level === 'up') continue
    for (const p of [m.from, m.to]) {
      minX = Math.min(minX, p.x)
      minY = Math.min(minY, p.y)
      maxX = Math.max(maxX, p.x)
      maxY = Math.max(maxY, p.y)
    }
  }
  if (!Number.isFinite(minX) || maxX <= minX || maxY <= minY) return null
  return { minX, minY, maxX, maxY }
}

/** 这一块里落笔的移动（判据与线距只看这些，空走不算「涂过胶」） */
export function landedMoves(block: LabBlock): LabMove[] {
  return block.moves.filter((m) => m.level !== 'up')
}

/** 块内落笔行数 / 抬笔行数（左栏列表用） */
export function countLevels(block: LabBlock): { normal: number; raised: number; up: number } {
  let normal = 0
  let raised = 0
  let up = 0
  for (const m of block.moves) {
    if (m.level === 'normal') normal += 1
    else if (m.level === 'raised') raised += 1
    else up += 1
  }
  return { normal, raised, up }
}
