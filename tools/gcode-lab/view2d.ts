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
  /** 下方支撑覆盖（上一层支撑）：灰色垫在带宽下面；无支撑是 `null` */
  under: { field: EdgeField; mask: Uint8Array } | null
  /** 每条移动的余量是否不足阈值（1 = 不足）：起点画红点 */
  underLow: Uint8Array
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
  /** 画布空着时左上角那句（没打开文件 / 这一层没有涂胶） */
  private emptyText = '把产物 G-code 拖进窗口'
  private maskCanvas: HTMLCanvasElement | null = null
  private maskKey = ''
  /** 下方支撑覆盖的离屏图（灰） */
  private underCanvas: HTMLCanvasElement | null = null
  private underKey = ''
  private scale = 4
  private cx = 0
  private cy = 0
  private w = 1
  private h = 1
  private selected: number | null = null
  /** 代码面板 / 色带指过来的那条移动：只加粗，不改选中 */
  private hover: number | null = null
  private hoverRaf = 0
  private hoverAt = { x: 0, y: 0 }
  /* 鼠标单键拖动；触屏一律不用单指移动 —— 一指只点选 */
  private dragging = false
  private dragFrom = { x: 0, y: 0, cx: 0, cy: 0 }
  /** 当前按在画布上的指针（client 坐标） */
  private readonly pointers = new Map<number, { x: number; y: number }>()
  /** 双指开始那一刻的状态 */
  private pinch: { cx: number; cy: number; scale: number; wx: number; wy: number } | null = null
  /** 双指操作之后，紧跟着的 click 不算点选 */
  private suppressClick = false
  /** 比例尺读数（mm / 100px），由 `draw` 更新 */
  readonly onScale: (mmPer100px: number) => void
  readonly onPick: (index: number | null) => void
  /** 指针扫过哪一条（`null` = 不在任何一条上）；代码面板据此高亮行 */
  onHover: ((index: number | null) => void) | null = null

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
    canvas.addEventListener('pointermove', this.handleHover)
    canvas.addEventListener('pointerup', this.handleUp)
    canvas.addEventListener('pointercancel', this.handleUp)
    canvas.addEventListener('pointerleave', this.handleLeave)
    canvas.addEventListener('click', this.handleClick)
    /* 触屏两指 / 鼠标右键都会发 contextmenu：在画布上吃掉，别弹浏览器菜单 */
    canvas.addEventListener('contextmenu', (e) => e.preventDefault())
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

  /** 没有当前块（这一层还没涂胶）：别留着上一块的线 */
  clear(emptyText = '把产物 G-code 拖进窗口'): void {
    this.data = null
    this.selected = null
    this.emptyText = emptyText
    this.draw()
  }

  /** 阈值变了：只换红点，不重建几何 */
  setUnderLow(low: Uint8Array): void {
    if (this.data === null) return
    this.data.underLow = low
    this.draw()
  }

  select(index: number | null): void {
    this.selected = index
    this.draw()
  }

  /** 代码行 / 色带悬停指过来：加粗那一条，不动选中 */
  setHover(index: number | null): void {
    if (this.hover === index) return
    this.hover = index
    this.draw()
  }

  /**
   * 只平移，不动缩放：代码面板的「起点 / 终点」与跳转都靠它。
   * 缩放一动，观察尺度就断了 —— 作者要的是「眼睛不动，画面挪过去」。
   */
  centerOn(x: number, y: number): void {
    this.cx = x
    this.cy = y
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

  /** 直接设整张视图（伪 2D 过渡收尾用，最后一帧要与 3D 顶视完全一致） */
  setView(cx: number, cy: number, scale: number): void {
    this.cx = cx
    this.cy = cy
    this.scale = Math.min(400, Math.max(0.4, scale))
    this.draw()
  }

  /** 当前视图（3D 伪 2D 起步时照抄它） */
  viewOf(): { cx: number; cy: number; scale: number; w: number; h: number } {
    return { cx: this.cx, cy: this.cy, scale: this.scale, w: this.w, h: this.h }
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
    this.pointers.set(e.pointerId, { x: e.clientX, y: e.clientY })
    this.canvas.setPointerCapture(e.pointerId)
    if (this.pointers.size >= 2) {
      /* 第二根手指落下：取消单指 pending，从头记一次双指 */
      this.dragging = false
      this.suppressClick = true
      this.beginPinch()
      return
    }
    if (e.pointerType === 'mouse') {
      this.dragging = true
      this.dragFrom = { x: e.clientX, y: e.clientY, cx: this.cx, cy: this.cy }
    }
  }

  /** 记下双指开始时的中心 / 距离，以及中心压着的那个世界坐标 */
  private beginPinch(): void {
    const [a, b] = [...this.pointers.values()]
    if (!a || !b) return
    const rect = this.canvas.getBoundingClientRect()
    const px = (a.x + b.x) / 2 - rect.left
    const py = (a.y + b.y) / 2 - rect.top
    const world = this.fromScreen(px, py)
    this.pinch = { cx: this.cx, cy: this.cy, scale: this.scale, wx: world[0], wy: world[1] }
    this.pinchDist0 = Math.max(1, Math.hypot(a.x - b.x, a.y - b.y))
  }

  private handleMove = (e: PointerEvent): void => {
    const p = this.pointers.get(e.pointerId)
    if (p) {
      p.x = e.clientX
      p.y = e.clientY
    }
    if (this.pointers.size >= 2) {
      this.movePinch()
      return
    }
    if (this.dragging) {
      this.cx = this.dragFrom.cx - (e.clientX - this.dragFrom.x) / this.scale
      this.cy = this.dragFrom.cy + (e.clientY - this.dragFrom.y) / this.scale
      this.draw()
    }
  }

  /**
   * 双指：移动两指中心 = 平移，捏合 / 张开 = 以双指中心为锚点缩放。
   *
   * 用「开始时中心压着的那个世界点」做锚：缩放之后它仍在**当前**双指中心下面，
   * 于是同时挪动和捏合也不会漂。
   */
  private movePinch(): void {
    const start = this.pinch
    const [a, b] = [...this.pointers.values()]
    if (!start || !a || !b) return
    const rect = this.canvas.getBoundingClientRect()
    const px = (a.x + b.x) / 2 - rect.left
    const py = (a.y + b.y) / 2 - rect.top
    const dist = Math.hypot(a.x - b.x, a.y - b.y)
    const factor = this.pinchDist0 > 1 ? dist / this.pinchDist0 : 1
    this.scale = Math.min(400, Math.max(0.4, start.scale * factor))
    this.cx = start.wx - (px - this.w / 2) / this.scale
    this.cy = start.wy + (py - this.h / 2) / this.scale
    this.draw()
  }

  /** 双指开始时的距离（`beginPinch` 里一并记） */
  private pinchDist0 = 0

  private handleUp = (e: PointerEvent): void => {
    this.pointers.delete(e.pointerId)
    if (this.pointers.size < 2) {
      this.pinch = null
      this.pinchDist0 = 0
    }
    if (this.pointers.size === 0) this.dragging = false
  }

  private handleLeave = (): void => {
    /* 指针捕获期间 leave 不该打断手势；只有鼠标真的离开且没按键才收 */
    if (this.pointers.size === 0) this.dragging = false
    if (this.hover !== null) {
      this.hover = null
      this.draw()
      this.onHover?.(null)
    }
  }

  /** 指针扫过：按帧节流挑最近的一条，报给外面（代码面板高亮那一行） */
  private handleHover = (e: PointerEvent): void => {
    const rect = this.canvas.getBoundingClientRect()
    this.hoverAt = { x: e.clientX - rect.left, y: e.clientY - rect.top }
    if (this.hoverRaf !== 0) return
    this.hoverRaf = requestAnimationFrame(() => {
      this.hoverRaf = 0
      const index = this.pick(this.hoverAt.x, this.hoverAt.y)
      if (index === this.hover) return
      this.hover = index
      this.draw()
      this.onHover?.(index)
    })
  }

  private handleClick = (e: MouseEvent): void => {
    if (this.suppressClick) {
      this.suppressClick = false
      return
    }
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
      ctx.fillText(this.emptyText, 16, 24)
      return
    }
    this.drawGrid()
    this.drawUnder()
    this.drawMask()
    this.drawMoves()
    this.drawUnderLow()
    this.drawFaces()
    this.drawHover()
    this.drawSelection()
    this.onScale((100 / this.scale))
  }

  /** 下方支撑覆盖：灰色的「有料」区，垫在蓝色带宽下面 */
  private drawUnder(): void {
    const { data, ctx, scale } = this
    if (!data?.under) return
    const f = data.under.field
    const key = `${data.block.index}:${f.nx}x${f.ny}`
    if (this.underCanvas === null || this.underKey !== key) {
      const off = document.createElement('canvas')
      off.width = f.nx
      off.height = f.ny
      const octx = off.getContext('2d')
      if (!octx) return
      const img = octx.createImageData(f.nx, f.ny)
      for (let i = 0; i < data.under.mask.length; i += 1) {
        if (!data.under.mask[i]) continue
        /* 屏幕 y 是反的，图要上下翻一下才不会镜像（与带宽同一套） */
        const x = i % f.nx
        const y = (i - x) / f.nx
        const t = ((f.ny - 1 - y) * f.nx + x) * 4
        img.data[t] = 150
        img.data[t + 1] = 150
        img.data[t + 2] = 158
        img.data[t + 3] = 46
      }
      octx.putImageData(img, 0, 0)
      this.underCanvas = off
      this.underKey = key
    }
    const [sx, sy] = this.toScreen(f.x0, f.y0 + f.ny * f.cellMm)
    ctx.save()
    ctx.imageSmoothingEnabled = true
    ctx.drawImage(this.underCanvas, sx, sy, f.nx * f.cellMm * scale, f.ny * f.cellMm * scale)
    ctx.restore()
  }

  /** 余量不够的涂胶段：起点点一个红点，扫一眼就知道哪几段悬着 */
  private drawUnderLow(): void {
    const { data, ctx } = this
    if (!data) return
    ctx.save()
    ctx.fillStyle = palette().red
    data.block.moves.forEach((m, i) => {
      if (data.underLow[i] !== 1) return
      const [x, y] = this.toScreen(m.from.x, m.from.y)
      ctx.beginPath()
      ctx.arc(x, y, 2.2, 0, Math.PI * 2)
      ctx.fill()
    })
    ctx.restore()
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

  /** 悬停：比选中细一档、用蓝色，不动选中那条的白色描边 */
  private drawHover(): void {
    const { data, ctx } = this
    if (!data || this.hover === null || this.hover === this.selected) return
    const m = data.block.moves[this.hover]
    if (!m) return
    const [ax, ay] = this.toScreen(m.from.x, m.from.y)
    const [bx, by] = this.toScreen(m.to.x, m.to.y)
    ctx.save()
    ctx.strokeStyle = palette().blue
    ctx.lineWidth = 3
    ctx.globalAlpha = 0.85
    ctx.beginPath()
    ctx.moveTo(ax, ay)
    ctx.lineTo(bx, by)
    ctx.stroke()
    ctx.restore()
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
    /* 起点实心、终点空心：代码面板那两枚「起点 / 终点」在图上各有对应物 */
    ctx.fillStyle = palette().text
    ctx.beginPath()
    ctx.arc(ax, ay, 2.5, 0, Math.PI * 2)
    ctx.fill()
    ctx.strokeStyle = palette().text
    ctx.lineWidth = 1.4
    ctx.beginPath()
    ctx.arc(bx, by, 2.6, 0, Math.PI * 2)
    ctx.stroke()
    ctx.restore()
  }
}
