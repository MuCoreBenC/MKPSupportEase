/*
 * 2D 俯视：一眼要看出「这条移动被抬了没有」。
 *
 * 图层的顺序是判据的顺序：预测带宽（半透明蓝紫）垫在最下面，然后是空走（灰）、
 * 落笔路径（青 = 正常档、琥珀 = 抬高档）、面框与短边、最后是选中的那一条。
 * 路径的颜色**只来自产物里的 Z**，不看预测；预测只负责垫底那一层和右栏那一列。
 */

import type { EdgeField, EdgeRule } from '../../src/app/glueLab/band'
import type { LabBlock } from '../../src/app/glueLab/parse'
import { palette } from './theme'

export interface View2dData {
  block: LabBlock
  field: EdgeField | null
  rule: EdgeRule
  /** 每条移动的预测（1 = 边缘带），下标与 `block.moves` 对齐 */
  predicted: Uint8Array
  /** 带宽覆盖的格子（由 `EdgeField.bandMask` 给） */
  mask: Uint8Array | null
}

function segDistance(
  px: number,
  py: number,
  ax: number,
  ay: number,
  bx: number,
  by: number,
): number {
  const dx = bx - ax
  const dy = by - ay
  const len2 = dx * dx + dy * dy
  const t = len2 > 0 ? Math.max(0, Math.min(1, ((px - ax) * dx + (py - ay) * dy) / len2)) : 0
  return Math.hypot(px - (ax + dx * t), py - (ay + dy * t))
}

export class View2d {
  private readonly canvas: HTMLCanvasElement
  private readonly ctx: CanvasRenderingContext2D
  private data: View2dData | null = null
  private maskCanvas: HTMLCanvasElement | null = null
  private maskKey = ''
  private scale = 4
  private cx = 0
  private cy = 0
  private w = 1
  private h = 1
  private selected: number | null = null
  private dragging = false
  private dragFrom = { x: 0, y: 0, cx: 0, cy: 0 }
  /** 比例尺读数（mm / 100px），由 `draw` 更新 */
  readonly onScale: (mmPer100px: number) => void
  readonly onPick: (index: number | null) => void

  constructor(canvas: HTMLCanvasElement, onScale: (v: number) => void, onPick: (i: number | null) => void) {
    this.canvas = canvas
    const ctx = canvas.getContext('2d')
    if (!ctx) throw new Error('2d canvas 拿不到上下文')
    this.ctx = ctx
    this.onScale = onScale
    this.onPick = onPick

    canvas.addEventListener('wheel', this.handleWheel, { passive: false })
    canvas.addEventListener('pointerdown', this.handleDown)
    canvas.addEventListener('pointermove', this.handleMove)
    canvas.addEventListener('pointerup', () => {
      this.dragging = false
    })
    canvas.addEventListener('pointerleave', () => {
      this.dragging = false
    })
    canvas.addEventListener('click', this.handleClick)
  }

  resize(): void {
    const rect = this.canvas.getBoundingClientRect()
    const dpr = window.devicePixelRatio || 1
    this.w = Math.max(1, rect.width)
    this.h = Math.max(1, rect.height)
    this.canvas.width = Math.round(this.w * dpr)
    this.canvas.height = Math.round(this.h * dpr)
    this.ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
    this.draw()
  }

  /**
   * 换一块。`refit` 只在「刚打开文件」时给 —— 切块时**保住当前的缩放与位置**，
   * 否则每点一下左边，画面就重排一次（作者原话：「这个位置总是在变化」）。
   * 新块整个落在画外时才回到铺满，不然会看到一片空白还不知道为什么。
   */
  setData(data: View2dData, refit = false): void {
    this.data = data
    this.maskCanvas = null
    this.maskKey = ''
    if (refit || !this.inView()) this.fit()
    else this.draw()
  }

  /** 这一块的外接框和当前画面有交集吗 */
  private inView(): boolean {
    const bbox = this.data?.block.bbox
    if (!bbox) return false
    const [ax, ay] = this.fromScreen(0, 0)
    const [bx, by] = this.fromScreen(this.w, this.h)
    const [x0, x1] = [Math.min(ax, bx), Math.max(ax, bx)]
    const [y0, y1] = [Math.min(ay, by), Math.max(ay, by)]
    return bbox.maxX >= x0 && bbox.minX <= x1 && bbox.maxY >= y0 && bbox.minY <= y1
  }

  select(index: number | null): void {
    this.selected = index
    this.draw()
  }

  /** 缩放滑杆：直接设 px/mm（与滚轮同一个量） */
  setScale(s: number): void {
    this.scale = Math.min(400, Math.max(0.4, s))
    this.draw()
  }

  scaleOf(): number {
    return this.scale
  }

  /** 只把这一块挪回画面正中：缩放不动（「居中」按钮的 2D 档） */
  recenter(): void {
    const bbox = this.data?.block.bbox
    if (!bbox) return
    this.cx = (bbox.minX + bbox.maxX) / 2
    this.cy = (bbox.minY + bbox.maxY) / 2
    this.draw()
  }

  /** 把这一块按比例铺满画面（「视角」按钮的 2D 档） */
  fit(): void {
    const bbox = this.data?.block.bbox
    if (!bbox) {
      this.scale = 4
      return
    }
    const bw = Math.max(0.5, bbox.maxX - bbox.minX)
    const bh = Math.max(0.5, bbox.maxY - bbox.minY)
    this.scale = Math.min(this.w / bw, this.h / bh) * 0.78
    this.cx = (bbox.minX + bbox.maxX) / 2
    this.cy = (bbox.minY + bbox.maxY) / 2
    this.draw()
  }

  private toScreen(x: number, y: number): [number, number] {
    return [(x - this.cx) * this.scale + this.w / 2, this.h / 2 - (y - this.cy) * this.scale]
  }

  private handleWheel = (e: WheelEvent): void => {
    e.preventDefault()
    const rect = this.canvas.getBoundingClientRect()
    const px = e.clientX - rect.left
    const py = e.clientY - rect.top
    const before = this.fromScreen(px, py)
    const factor = Math.exp(-e.deltaY * 0.0015)
    this.scale = Math.min(400, Math.max(0.4, this.scale * factor))
    const after = this.fromScreen(px, py)
    this.cx += before[0] - after[0]
    this.cy += before[1] - after[1]
    this.draw()
  }

  private fromScreen(px: number, py: number): [number, number] {
    return [this.cx + (px - this.w / 2) / this.scale, this.cy - (py - this.h / 2) / this.scale]
  }

  private handleDown = (e: PointerEvent): void => {
    this.dragging = true
    this.dragFrom = { x: e.clientX, y: e.clientY, cx: this.cx, cy: this.cy }
    this.canvas.setPointerCapture(e.pointerId)
  }

  private handleMove = (e: PointerEvent): void => {
    if (this.dragging) {
      this.cx = this.dragFrom.cx - (e.clientX - this.dragFrom.x) / this.scale
      this.cy = this.dragFrom.cy + (e.clientY - this.dragFrom.y) / this.scale
      this.draw()
      return
    }
    if (e.buttons !== 0) return
  }

  private handleClick = (e: MouseEvent): void => {
    const rect = this.canvas.getBoundingClientRect()
    this.onPick(this.pick(e.clientX - rect.left, e.clientY - rect.top))
  }

  /** 最近的落笔移动（屏幕距离 8px 内），没有就返回 `null` */
  pick(px: number, py: number): number | null {
    const data = this.data
    if (!data) return null
    let best: number | null = null
    let bestD = 8
    data.block.moves.forEach((m, i) => {
      if (m.level === 'up') return
      const [ax, ay] = this.toScreen(m.from.x, m.from.y)
      const [bx, by] = this.toScreen(m.to.x, m.to.y)
      const d = segDistance(px, py, ax, ay, bx, by)
      if (d < bestD) {
        bestD = d
        best = i
      }
    })
    return best
  }

  draw(): void {
    const { ctx, w, h } = this
    ctx.clearRect(0, 0, w, h)
    if (!this.data) {
      ctx.fillStyle = palette().dim
      ctx.font = '12px -apple-system, sans-serif'
      ctx.fillText('把产物 G-code 拖进窗口', 16, 24)
      return
    }
    this.drawGrid()
    this.drawMask()
    this.drawMoves()
    this.drawFaces()
    this.drawSelection()
    this.onScale((100 / this.scale))
  }

  private drawGrid(): void {
    const { ctx } = this
    const step = this.scale > 12 ? 1 : this.scale > 4 ? 5 : 10
    const [x0, y0] = this.fromScreen(0, 0)
    const [x1, y1] = this.fromScreen(this.w, this.h)
    ctx.save()
    ctx.strokeStyle = palette().grid
    ctx.lineWidth = 1
    ctx.beginPath()
    for (let x = Math.ceil(x0 / step) * step; x <= x1; x += step) {
      const [sx] = this.toScreen(x, 0)
      ctx.moveTo(Math.round(sx) + 0.5, 0)
      ctx.lineTo(Math.round(sx) + 0.5, this.h)
    }
    for (let y = Math.ceil(y0 / step) * step; y <= y1; y += step) {
      const [, sy] = this.toScreen(0, y)
      ctx.moveTo(0, Math.round(sy) + 0.5)
      ctx.lineTo(this.w, Math.round(sy) + 0.5)
    }
    ctx.stroke()
    ctx.restore()
  }

  /** 预测带宽：按格子铺一张小图，放大时做平滑，边界自然变得柔和 */
  private drawMask(): void {
    const { data, ctx, scale } = this
    if (!data?.field || !data.mask) return
    const field = data.field
    const key = `${data.block.index}:${data.rule.lines}:${data.rule.pitchMm}`
    if (this.maskCanvas === null || this.maskKey !== key) {
      const off = document.createElement('canvas')
      off.width = field.nx
      off.height = field.ny
      const octx = off.getContext('2d')
      if (!octx) return
      const img = octx.createImageData(field.nx, field.ny)
      for (let i = 0; i < data.mask.length; i += 1) {
        if (!data.mask[i]) continue
        /* 屏幕 y 是反的，图要上下翻一下才不会镜像 */
        const x = i % field.nx
        const y = (i - x) / field.nx
        const t = ((field.ny - 1 - y) * field.nx + x) * 4
        img.data[t] = 76
        img.data[t + 1] = 141
        img.data[t + 2] = 255
        img.data[t + 3] = 64
      }
      octx.putImageData(img, 0, 0)
      this.maskCanvas = off
      this.maskKey = key
    }
    const [sx, sy] = this.toScreen(field.x0, field.y0 + field.ny * field.cellMm)
    ctx.save()
    ctx.imageSmoothingEnabled = true
    ctx.drawImage(
      this.maskCanvas,
      sx,
      sy,
      field.nx * field.cellMm * scale,
      field.ny * field.cellMm * scale,
    )
    ctx.restore()
  }

  private drawMoves(): void {
    const { data, ctx } = this
    if (!data) return
    const moves = data.block.moves

    ctx.lineCap = 'round'
    /* 空走垫底：细灰 */
    ctx.strokeStyle = palette().up
    ctx.lineWidth = 1
    ctx.beginPath()
    for (const m of moves) {
      if (m.level !== 'up') continue
      const [ax, ay] = this.toScreen(m.from.x, m.from.y)
      const [bx, by] = this.toScreen(m.to.x, m.to.y)
      ctx.moveTo(ax, ay)
      ctx.lineTo(bx, by)
    }
    ctx.stroke()

    /* 与判定不一致的那些先描一道红底 */
    ctx.strokeStyle = palette().red
    ctx.globalAlpha = 0.55
    ctx.lineWidth = 3.5
    ctx.beginPath()
    let anyBad = false
    moves.forEach((m, i) => {
      if (m.level === 'up') return
      const pred = data.predicted[i] === 1
      if (pred === (m.level === 'raised')) return
      anyBad = true
      const [ax, ay] = this.toScreen(m.from.x, m.from.y)
      const [bx, by] = this.toScreen(m.to.x, m.to.y)
      ctx.moveTo(ax, ay)
      ctx.lineTo(bx, by)
    })
    if (anyBad) ctx.stroke()
    ctx.globalAlpha = 1

    /* 落笔两档：颜色只来自产物里的 Z */
    for (const level of ['normal', 'raised'] as const) {
      ctx.strokeStyle = level === 'raised' ? palette().amber : palette().teal
      ctx.lineWidth = 1.6
      ctx.beginPath()
      for (const m of moves) {
        if (m.level !== level) continue
        const [ax, ay] = this.toScreen(m.from.x, m.from.y)
        const [bx, by] = this.toScreen(m.to.x, m.to.y)
        ctx.moveTo(ax, ay)
        ctx.lineTo(bx, by)
      }
      ctx.stroke()
    }
  }

  private drawFaces(): void {
    const { data, ctx } = this
    if (!data?.field) return
    ctx.save()
    ctx.font = '10px ui-monospace, monospace'
    for (const face of data.field.faces) {
      if (face.id === 0) continue
      const box = this.faceBox(data, face.id)
      if (!box) continue
      const [ax, ay] = this.toScreen(box.minX, box.maxY)
      const [bx, by] = this.toScreen(box.maxX, box.minY)
      ctx.strokeStyle = face.shortMm < 2.5 ? 'rgba(240, 160, 32, 0.55)' : 'rgba(76, 141, 255, 0.35)'
      ctx.setLineDash([4, 4])
      ctx.lineWidth = 1
      ctx.strokeRect(ax, ay, bx - ax, by - ay)
      ctx.setLineDash([])
      ctx.fillStyle = face.shortMm < 2.5 ? palette().amber : palette().dim
      const band = data.field.bandMm(face.shortMm, data.rule)
      ctx.fillText(
        `#${face.id} ${face.shortMm.toFixed(2)}mm · 带 ${band.toFixed(2)}`,
        ax + 3,
        ay - 4,
      )
    }
    ctx.restore()
  }

  /** 面的外接框：用落在这一块里的点现算（判据只给了短边） */
  private faceBox(
    data: View2dData,
    id: number,
  ): { minX: number; minY: number; maxX: number; maxY: number } | null {
    const field = data.field
    if (!field) return null
    let minX = Infinity
    let minY = Infinity
    let maxX = -Infinity
    let maxY = -Infinity
    for (const m of data.block.moves) {
      if (m.level === 'up') continue
      if (field.faceId(m.from) !== id) continue
      for (const p of [m.from, m.to]) {
        minX = Math.min(minX, p.x)
        minY = Math.min(minY, p.y)
        maxX = Math.max(maxX, p.x)
        maxY = Math.max(maxY, p.y)
      }
    }
    return Number.isFinite(minX) ? { minX, minY, maxX, maxY } : null
  }

  private drawSelection(): void {
    const { data, ctx } = this
    if (!data || this.selected === null) return
    const m = data.block.moves[this.selected]
    if (!m) return
    const [ax, ay] = this.toScreen(m.from.x, m.from.y)
    const [bx, by] = this.toScreen(m.to.x, m.to.y)
    ctx.save()
    ctx.strokeStyle = palette().text
    ctx.lineWidth = 2.4
    ctx.beginPath()
    ctx.moveTo(ax, ay)
    ctx.lineTo(bx, by)
    ctx.stroke()
    ctx.fillStyle = palette().text
    ctx.beginPath()
    ctx.arc(ax, ay, 2.5, 0, Math.PI * 2)
    ctx.fill()
    ctx.restore()
  }
}
