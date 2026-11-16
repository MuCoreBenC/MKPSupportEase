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
  /** 这一块对应的**层高**（`;MKP_BEGIN paint … z=`）；没有标就 `null` */
  layerZ: number | null
  /** 正常落笔 Z（笔尖高度，含笔的机械偏移）；块内一条落笔移动都没有时是 `null` */
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
  let layerZ: number | null = null

  for (let i = 0; i < lines.length; i += 1) {
    const t = lines[i].trim()
    if (t.startsWith(BLOCK_BEGIN)) {
      beginLine = i
      const m = /\bz=(-?\d+(?:\.\d+)?)/.exec(t)
      layerZ = m ? Number(m[1]) : null
      raws = []
      continue
    }
    if (t.startsWith(BLOCK_END)) {
      if (raws) blocks.push(finishBlock(blocks.length, beginLine, i, layerZ, raws))
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

function finishBlock(
  index: number,
  beginLine: number,
  endLine: number,
  layerZ: number | null,
  raws: RawMove[],
): LabBlock {
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
    layerZ,
    normalZ,
    raisedZ,
    liftMm: raisedZ !== null && normalZ !== null ? raisedZ - normalZ : 0,
    bbox: landedBBox(moves),
    note,
  }
}

/* ---------------- 打印本体（3D 里当背景） ---------------- */

/**
 * 打印本体的类别。
 *
 * 支撑拆成三档是作者要的（与切片器预览同一套区分）：**支撑**、**支撑面**
 * （`Support interface`，顶上那几层）、**熨烫面**（`Support ironing`，涂胶真正落上去的那一面）。
 * `travel` 是空走（不带 E 的移动），`other` 兜住认不出的 `; FEATURE:`。
 */
export type PrintKind =
  | 'outer'
  | 'inner'
  | 'fill'
  | 'support'
  | 'supportFace'
  | 'ironing'
  | 'travel'
  | 'other'

export const PRINT_KINDS: readonly PrintKind[] = [
  'outer',
  'inner',
  'fill',
  'support',
  'supportFace',
  'ironing',
  'travel',
  'other',
]

/**
 * `; FEATURE:` 的名字 → 类别。
 *
 * 名字是 Bambu Studio 给的（`; FEATURE: Outer wall` 这种），不是我们编的；
 * 认不出的落到 `other` —— 新版本切片器多一类时它会亮在 `other` 里，而不是消失。
 */
const KIND_OF: Record<string, PrintKind> = {
  'Outer wall': 'outer',
  'Floating vertical shell': 'outer',
  'Inner wall': 'inner',
  'Sparse infill': 'fill',
  'Internal solid infill': 'fill',
  'Gap infill': 'fill',
  'Top surface': 'fill',
  'Bottom surface': 'fill',
  Bridge: 'fill',
  Support: 'support',
  'Support transition': 'support',
  'Support interface': 'supportFace',
  'Support ironing': 'ironing',
}

/** 产物里的一个模型（`; start printing object, unique label id: N`） */
export interface PrintObject {
  /** 那个 label id；`0` = 不在任何对象里的移动（起始擦料线这类） */
  id: number
  /** 每类的线段顶点（`x,y,z` 三元组，两个端点一组），**按层升序排好** */
  kinds: Record<PrintKind, Float32Array>
  /**
   * 每类在每层的区间：`bounds[k][i] … bounds[k][i+1]`，单位是**段**，长度恒为
   * `layers.length + 1`（这一层这个对象没有东西时前后相等）。于是"看到第 i 层"
   * = `drawRange(0, bounds[i+1]*2)`，不用重建几何。
   */
  bounds: Record<PrintKind, Uint32Array>
}

export interface PrintModel {
  /** 升序的层 Z（取自 `G1 Z`），所有对象共用这一份下标 */
  layers: number[]
  /** 按出现顺序的模型；`id` 为 0 的那个只装不属于任何对象的移动 */
  objects: PrintObject[]
  /** 认不出的 `; FEATURE:` 名字（界面提示用；空数组 = 全认识） */
  unknown: string[]
  zMin: number
  zMax: number
}

const emptyBuckets = (): Record<PrintKind, number[]> => ({
  outer: [],
  inner: [],
  fill: [],
  support: [],
  supportFace: [],
  ironing: [],
  travel: [],
  other: [],
})

const OBJECT_START = '; start printing object'
const OBJECT_STOP = '; stop printing object'
const LABEL = /label id:\s*(\d+)/

/**
 * 产物全文 → 打印本体。
 *
 * 三类移动分开：
 *  - **带 E 的**按 `; FEATURE:` 分成墙 / 填充 / 支撑…，是模型本身；
 *  - **不带 E 的**进 `travel`（空走），切片器预览里那层灰线；
 *  - `;MKP_BEGIN … ;MKP_END` 之间的整段跳过 —— 那是涂胶与换笔，归 `parseBlocks`。
 *
 * 顶点按 Z 升序排好：图上那把层滑块靠 `drawRange` 切层，拖起来才是即时的。
 */
export function parsePrint(text: string): PrintModel {
  const lines = text.split(/\r?\n/)
  /** 对象 id → 层 key → 这一层各类的顶点 */
  const byObject = new Map<number, Map<string, { z: number; bucket: Record<PrintKind, number[]> }>>()
  const unknown = new Set<string>()
  const layerKeys = new Set<string>()
  let objectId = 0
  let kind: PrintKind = 'other'
  let x = 0
  let y = 0
  let z = 0
  let hasPos = false
  let inBlock = false
  let zMin = Infinity
  let zMax = -Infinity

  for (const raw of lines) {
    const t = raw.trim()
    if (t.startsWith(BLOCK_BEGIN)) {
      inBlock = true
      continue
    }
    if (t.startsWith(BLOCK_END)) {
      inBlock = false
      continue
    }
    if (inBlock) continue
    if (t.startsWith(OBJECT_START)) {
      const m = LABEL.exec(t)
      objectId = m ? Number(m[1]) : 0
      continue
    }
    if (t.startsWith(OBJECT_STOP)) {
      objectId = 0
      continue
    }
    if (t.startsWith('; FEATURE:')) {
      const name = t.slice('; FEATURE:'.length).trim()
      const k = KIND_OF[name]
      if (k === undefined) unknown.add(name)
      kind = k ?? 'other'
      continue
    }
    if (!t.startsWith('G1 ') && !t.startsWith('G2 ') && !t.startsWith('G3 ')) continue

    const nz = axis(t, 'Z')
    const nx = axis(t, 'X')
    const ny = axis(t, 'Y')
    if (nz !== null) z = nz
    if (nx === null && ny === null) continue
    const from = hasPos ? { x, y } : { x: nx ?? x, y: ny ?? y }
    if (nx !== null) x = nx
    if (ny !== null) y = ny
    hasPos = true

    /* `E` 在且大于 0 才是挤出（`E.3742` 这种省前导零的写法一并认） */
    const em = /\bE(-?(?:\d+(?:\.\d*)?|\.\d+))/.exec(t)
    const k: PrintKind = em !== null && Number(em[1]) > 0 ? kind : 'travel'
    const key = z.toFixed(3)
    let layers = byObject.get(objectId)
    if (layers === undefined) {
      layers = new Map()
      byObject.set(objectId, layers)
    }
    let slot = layers.get(key)
    if (slot === undefined) {
      slot = { z, bucket: emptyBuckets() }
      layers.set(key, slot)
    }
    slot.bucket[k].push(from.x, from.y, z, x, y, z)
    if (k !== 'travel') {
      layerKeys.add(key)
      if (z < zMin) zMin = z
      if (z > zMax) zMax = z
    }
  }

  /*
   * 层号只由**挤出**的移动决定。空走在抬笔高度（`Z10.8` 那种）也会形成一堆
   * 不相干的 Z 值，算进层里会把滑块撑到一百多层，整摞的高度也全乱。
   * 于是那些「只有空走」的 Z 直接丢掉：抬笔飞过去的那几段本来也不该按层看。
   */
  const keys = [...layerKeys].sort((a, b) => Number(a) - Number(b))
  const objects: PrintObject[] = []
  for (const [id, layers] of byObject) {
    const kinds = {} as Record<PrintKind, Float32Array>
    const bounds = {} as Record<PrintKind, Uint32Array>
    for (const k of PRINT_KINDS) {
      const flat: number[] = []
      const edge = new Uint32Array(keys.length + 1)
      let seg = 0
      keys.forEach((key, i) => {
        const arr = layers.get(key)?.bucket[k] ?? []
        for (const v of arr) flat.push(v)
        seg += arr.length / 6
        edge[i + 1] = seg
      })
      kinds[k] = Float32Array.from(flat)
      bounds[k] = edge
    }
    objects.push({ id, kinds, bounds })
  }
  /* 真实的模型排在前面，`id 0`（不属于任何对象的移动）垫底 */
  objects.sort((a, b) => (a.id === 0 ? 1 : 0) - (b.id === 0 ? 1 : 0) || a.id - b.id)

  return {
    layers: keys.map(Number),
    objects,
    unknown: [...unknown],
    zMin: Number.isFinite(zMin) ? zMin : 0,
    zMax: Number.isFinite(zMax) ? zMax : 0,
  }
}

/* ---------------- 涂胶偏移 ---------------- */

/** 涂胶路径相对产物里那条路径的平移量（mm） */
export interface GlueShift {
  dx: number
  dy: number
  dz: number
}

/**
 * 把涂胶块整体平移。
 *
 * 产物里的涂胶坐标 = 目标坐标 + 实际生效的两笔偏移，所以「把偏移调成 0」在展示端
 * 就是**减掉**它；一次平移落在这一处，2D 与 3D 都吃这份，免得两边各算一遍。
 * `layerZ` 不动：它标的是「这一块属于哪一层」，跟笔尖高低无关。
 */
export function shiftBlocks(blocks: readonly LabBlock[], shift: GlueShift): LabBlock[] {
  if (shift.dx === 0 && shift.dy === 0 && shift.dz === 0) return blocks.slice()
  return blocks.map((b) => {
    const moves = b.moves.map((m) => ({
      ...m,
      from: { x: m.from.x + shift.dx, y: m.from.y + shift.dy },
      to: { x: m.to.x + shift.dx, y: m.to.y + shift.dy },
      z: m.z + shift.dz,
    }))
    return {
      ...b,
      moves,
      normalZ: b.normalZ === null ? null : b.normalZ + shift.dz,
      raisedZ: b.raisedZ === null ? null : b.raisedZ + shift.dz,
      bbox: landedBBox(moves),
    }
  })
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
