/*
 * 边缘带判据（试验台口径）。
 *
 * 与 `crates/postprocess/src/postproc/pass1/edge_band.rs` 同一套：
 *
 *   涂胶点 → 0.25mm 格子（每个采样点带 3×3 画笔，并排的线连成一片）
 *          → 连通块，每块一个「短边」
 *          → 3-4 倒角距离场：每格离「没涂胶」多远
 *
 * 带宽 = min(N × 线距, 该面短边 / 2)。`short/2` 封顶保住「中间那一半永远不抬」：
 * 抬高 = 笔离面 0.5mm、那一段基本不落胶，是有代价的动作。
 *
 * **这份只算「预测」**：产物里那一档 Z 才是内核真做的决定（`parse.ts` 读出来的
 * `level`）。界面分两栏显示，不把这里的结论当内核结论。两份实现的一致性只由
 * 合成 fixture 的单测钉（`band.test.ts`）——真实产物的 iface 里有哪些空走、切点
 * 落在哪，都反推不回来。
 */

import type { Point } from './parse'

/** 格子的边长（mm）。与 Rust 的 `CELL_MM` 一致：涂胶线间距约 0.4mm，这个粒度能把并排的线连成一片 */
const CELL_MM = 0.25
/** 格子总数上限，超过就把格子放大（一份大件别把浏览器吃爆） */
const MAX_CELLS = 2_000_000
/** 距离场里「还没被量到」的哨兵值（单位 1/3 格） */
const UNREACHED = 1 << 28

export interface EdgeRule {
  /** 靠外几条涂胶线；0 = 关 */
  lines: number
  /** 线距（mm），见 `pitch.ts` */
  pitchMm: number
}

export interface EdgeFace {
  id: number
  /** 这块面的短边（外接框的短边，mm） */
  shortMm: number
}

export interface Stroke {
  from: Point
  to: Point
}

export class EdgeField {
  readonly cellMm: number
  readonly x0: number
  readonly y0: number
  readonly nx: number
  readonly ny: number
  readonly faces: EdgeFace[]
  /** 每格的连通块编号（0 = 没涂胶） */
  private readonly comp: Int32Array
  /** 每格到最近「没涂胶格子」的距离（单位 1/3 格） */
  private readonly dist: Int32Array

  private constructor(
    cellMm: number,
    x0: number,
    y0: number,
    nx: number,
    ny: number,
    faces: EdgeFace[],
    comp: Int32Array,
    dist: Int32Array,
  ) {
    this.cellMm = cellMm
    this.x0 = x0
    this.y0 = y0
    this.nx = nx
    this.ny = ny
    this.faces = faces
    this.comp = comp
    this.dist = dist
  }

  /**
   * 一条涂胶路径 → 距离场。点太少或退化成一个点时返回 `null`。
   *
   * `fillHoles` 把被覆盖区围住的孔一律当实心（判「下方支撑」时要的）：
   * 孔里是零件本体，笔走过去有东西托着，不该算边。与外界连通的缺口不填。
   */
  static build(
    moves: readonly Stroke[],
    opts: { material?: readonly Stroke[] } = {},
  ): EdgeField | null {
    let minX = Infinity
    let minY = Infinity
    let maxX = -Infinity
    let maxY = -Infinity
    for (const m of moves) {
      for (const p of [m.from, m.to]) {
        minX = Math.min(minX, p.x)
        minY = Math.min(minY, p.y)
        maxX = Math.max(maxX, p.x)
        maxY = Math.max(maxY, p.y)
      }
    }
    /* 材料那一份（撑 + 零件本体）也进包围盒：孔比支撑范围大时网格不能裁掉它 */
    for (const m of opts.material ?? []) {
      for (const p of [m.from, m.to]) {
        minX = Math.min(minX, p.x)
        minY = Math.min(minY, p.y)
        maxX = Math.max(maxX, p.x)
        maxY = Math.max(maxY, p.y)
      }
    }
    if (!Number.isFinite(minX) || !(maxX > minX) || !(maxY > minY)) return null

    /* 外圈留两格：距离场要有「外面」可量 */
    const pad = 2 * CELL_MM
    const x0 = minX - pad
    const y0 = minY - pad
    const w = maxX - minX + 2 * pad
    const h = maxY - minY + 2 * pad
    let cell = CELL_MM
    if ((w / cell) * (h / cell) > MAX_CELLS) cell = Math.max(CELL_MM, Math.sqrt((w * h) / MAX_CELLS))
    const nx = Math.max(1, Math.ceil(w / cell)) + 1
    const ny = Math.max(1, Math.ceil(h / cell)) + 1

    const covered = new Uint8Array(nx * ny)
    const at = (x: number, y: number): number => {
      const ix = Math.min(nx - 1, Math.max(0, Math.round((x - x0) / cell)))
      const iy = Math.min(ny - 1, Math.max(0, Math.round((y - y0) / cell)))
      return iy * nx + ix
    }
    for (const m of moves) {
      const len = Math.hypot(m.to.x - m.from.x, m.to.y - m.from.y)
      const steps = Math.min(200_000, Math.max(1, Math.ceil(len / (cell * 0.5))))
      for (let k = 0; k <= steps; k += 1) {
        const t = k / steps
        const i = at(m.from.x + (m.to.x - m.from.x) * t, m.from.y + (m.to.y - m.from.y) * t)
        const ix = i % nx
        const iy = (i - ix) / nx
        for (let dy = -1; dy <= 1; dy += 1) {
          for (let dx = -1; dx <= 1; dx += 1) {
            const xx = ix + dx
            const yy = iy + dy
            if (xx >= 0 && yy >= 0 && xx < nx && yy < ny) covered[yy * nx + xx] = 1
          }
        }
      }
    }

    /* 填内孔：只填「孔里真有材料」的（零件墙/填充/桥接），真空气孔留着 */
    if (opts.material !== undefined) {
      const material = new Uint8Array(nx * ny)
      for (const m of opts.material) {
        const len = Math.hypot(m.to.x - m.from.x, m.to.y - m.from.y)
        const steps = Math.min(200_000, Math.max(1, Math.ceil(len / (cell * 0.5))))
        for (let k = 0; k <= steps; k += 1) {
          const t = k / steps
          material[at(m.from.x + (m.to.x - m.from.x) * t, m.from.y + (m.to.y - m.from.y) * t)] = 1
        }
      }
      fillHoles(covered, material, nx, ny)
    }

    /* 连通块标号 + 每块短边（一层里可能就是好几张互不相连的面） */
    const comp = new Int32Array(nx * ny)
    const faces: EdgeFace[] = [{ id: 0, shortMm: 0 }]
    const stack: number[] = []
    let nextId = 1
    for (let i = 0; i < covered.length; i += 1) {
      if (!covered[i] || comp[i] !== 0) continue
      const id = nextId
      nextId += 1
      let bx0 = nx
      let by0 = ny
      let bx1 = 0
      let by1 = 0
      stack.push(i)
      comp[i] = id
      while (stack.length > 0) {
        const k = stack.pop() as number
        const x = k % nx
        const y = (k - x) / nx
        bx0 = Math.min(bx0, x)
        by0 = Math.min(by0, y)
        bx1 = Math.max(bx1, x + 1)
        by1 = Math.max(by1, y + 1)
        for (const [dx, dy] of [
          [1, 0],
          [-1, 0],
          [0, 1],
          [0, -1],
        ] as const) {
          const xx = x + dx
          const yy = y + dy
          if (xx < 0 || yy < 0 || xx >= nx || yy >= ny) continue
          const j = yy * nx + xx
          if (covered[j] && comp[j] === 0) {
            comp[j] = id
            stack.push(j)
          }
        }
      }
      faces.push({
        id,
        shortMm: Math.min((bx1 - bx0) * cell, (by1 - by0) * cell),
      })
    }

    /* 3-4 倒角距离：两遍扫描 */
    const dist = new Int32Array(nx * ny)
    for (let i = 0; i < covered.length; i += 1) dist[i] = covered[i] ? UNREACHED : 0
    for (let y = 0; y < ny; y += 1) {
      for (let x = 0; x < nx; x += 1) {
        const i = y * nx + x
        if (dist[i] === 0) continue
        let d = dist[i] as number
        if (x > 0) d = Math.min(d, (dist[i - 1] as number) + 3)
        if (y > 0) {
          d = Math.min(d, (dist[i - nx] as number) + 3)
          if (x > 0) d = Math.min(d, (dist[i - nx - 1] as number) + 4)
          if (x + 1 < nx) d = Math.min(d, (dist[i - nx + 1] as number) + 4)
        }
        dist[i] = d
      }
    }
    for (let y = ny - 1; y >= 0; y -= 1) {
      for (let x = nx - 1; x >= 0; x -= 1) {
        const i = y * nx + x
        if (dist[i] === 0) continue
        let d = dist[i] as number
        if (x + 1 < nx) d = Math.min(d, (dist[i + 1] as number) + 3)
        if (y + 1 < ny) {
          d = Math.min(d, (dist[i + nx] as number) + 3)
          if (x + 1 < nx) d = Math.min(d, (dist[i + nx + 1] as number) + 4)
          if (x > 0) d = Math.min(d, (dist[i + nx - 1] as number) + 4)
        }
        dist[i] = d
      }
    }

    return new EdgeField(cell, x0, y0, nx, ny, faces, comp, dist)
  }

  /** 找这个点**最近的那个涂胶格子**（点可能正好落在两条线之间） */
  private cellOf(p: Point): number {
    const cx = Math.min(this.nx - 1, Math.max(0, Math.round((p.x - this.x0) / this.cellMm)))
    const cy = Math.min(this.ny - 1, Math.max(0, Math.round((p.y - this.y0) / this.cellMm)))
    let best = -1
    let bestD = Infinity
    for (let dy = -2; dy <= 2; dy += 1) {
      for (let dx = -2; dx <= 2; dx += 1) {
        const x = cx + dx
        const y = cy + dy
        if (x < 0 || y < 0 || x >= this.nx || y >= this.ny) continue
        const i = y * this.nx + x
        if (this.comp[i] === 0) continue
        const d = dx * dx + dy * dy
        if (d < bestD) {
          bestD = d
          best = i
        }
      }
    }
    return best
  }

  /** 这个点到「这块面的边界」有多远（mm）。量不出来按 0 算（当边缘，宁可多抬） */
  edgeDistance(p: Point): number {
    const i = this.cellOf(p)
    if (i < 0) return 0
    return Math.max(0, ((this.dist[i] as number) / 3) * this.cellMm - this.cellMm)
  }

  /** 这个点所在那张面的短边（mm）；不在任何面上是 0 */
  faceShortMm(p: Point): number {
    const i = this.cellOf(p)
    if (i < 0) return 0
    const id = this.comp[i] as number
    return this.faces[id]?.shortMm ?? 0
  }

  /** 这个点所在那张面的编号；不在任何面上是 0 */
  faceId(p: Point): number {
    const i = this.cellOf(p)
    return i < 0 ? 0 : (this.comp[i] as number)
  }

  /** 带宽（mm）：`N × 线距`，最多半张面 */
  bandMm(shortMm: number, rule: EdgeRule): number {
    if (shortMm <= 0 || rule.lines <= 0) return 0
    return Math.min(rule.lines * rule.pitchMm, shortMm / 2)
  }

  /** 这个点算不算边缘带 */
  isEdge(p: Point, rule: EdgeRule): boolean {
    const band = this.bandMm(this.faceShortMm(p), rule)
    return band > 0 && this.edgeDistance(p) < band
  }

  /** 每格有没有料（1 = 覆盖）。画「下方支撑」的覆盖层用 */
  coverageMask(): Uint8Array {
    const mask = new Uint8Array(this.nx * this.ny)
    for (let i = 0; i < mask.length; i += 1) mask[i] = this.comp[i] === 0 ? 0 : 1
    return mask
  }

  /** 供 2D 视图铺热力：每格是否落在带内（1 = 带内） */
  bandMask(rule: EdgeRule): Uint8Array {
    const mask = new Uint8Array(this.nx * this.ny)
    if (rule.lines <= 0) return mask
    for (let i = 0; i < mask.length; i += 1) {
      const id = this.comp[i] as number
      if (id === 0) continue
      const band = this.bandMm(this.faces[id]?.shortMm ?? 0, rule)
      if (band <= 0) continue
      const d = Math.max(0, ((this.dist[i] as number) / 3) * this.cellMm - this.cellMm)
      if (d < band) mask[i] = 1
    }
    return mask
  }
}
/**
 * 填内孔：从四条边泛洪，泛洪不到的未覆盖格子就是「被围住的孔」，一律置为覆盖。
 *
 * 与外界连通的缺口不会被填 —— 那才是真正的边。孔里是零件本体（墙 / 填充 / 桥接），
 * 笔走过去有东西托着，所以不当边看。
 */
function fillHoles(covered: Uint8Array, material: Uint8Array, nx: number, ny: number): void {
  const outside = new Uint8Array(nx * ny)
  const stack: number[] = []
  const visit = (x: number, y: number): void => {
    if (x < 0 || y < 0 || x >= nx || y >= ny) return
    const i = y * nx + x
    if (covered[i] === 1 || outside[i] === 1) return
    outside[i] = 1
    stack.push(i)
  }
  for (let x = 0; x < nx; x += 1) {
    visit(x, 0)
    visit(x, ny - 1)
  }
  for (let y = 0; y < ny; y += 1) {
    visit(0, y)
    visit(nx - 1, y)
  }
  while (stack.length > 0) {
    const i = stack.pop() as number
    const x = i % nx
    const y = (i - x) / nx
    visit(x - 1, y)
    visit(x + 1, y)
    visit(x, y - 1)
    visit(x, y + 1)
  }

  /* 逐孔连通块：块里**有材料**才整块填平；真空气孔原样留着 */
  const seen = new Uint8Array(nx * ny)
  const hole: number[] = []
  for (let start = 0; start < covered.length; start += 1) {
    if (covered[start] === 1 || outside[start] === 1 || seen[start] === 1) continue
    hole.length = 0
    let hasMaterial = false
    seen[start] = 1
    stack.push(start)
    while (stack.length > 0) {
      const i = stack.pop() as number
      hole.push(i)
      if (material[i] === 1) hasMaterial = true
      const x = i % nx
      const y = (i - x) / nx
      const push = (xx: number, yy: number): void => {
        if (xx < 0 || yy < 0 || xx >= nx || yy >= ny) return
        const j = yy * nx + xx
        if (covered[j] === 1 || outside[j] === 1 || seen[j] === 1) return
        seen[j] = 1
        stack.push(j)
      }
      push(x - 1, y)
      push(x + 1, y)
      push(x, y - 1)
      push(x, y + 1)
    }
    if (hasMaterial) {
      for (const i of hole) covered[i] = 1
    }
  }
}
