/*
 * 试验台装配：拖入 → 切块 → 2D/3D → 逐条诊断。
 *
 * 两列口径别混：
 *   实际 = 产物里的 Z（`parse.ts` 读出来的 `level`）—— 内核真做的决定；
 *   预测 = `band.ts` 按当前「外几条 × 线距」算的 —— 只在这里存在。
 * 顶栏滑杆只重算预测，不重解析文件。
 */

import { unzipSync } from 'fflate'

import { findMetaName, pairsWith } from '../../src/app/glueLab/files'
import { EdgeField } from '../../src/app/glueLab/band'
import type { EdgeRule } from '../../src/app/glueLab/band'
import { countLevels, landedMoves, parseBlocks, parsePrint, shiftBlocks } from '../../src/app/glueLab/parse'
import type { LabBlock, LabMove, PrintModel } from '../../src/app/glueLab/parse'
import { ZERO_OFFSETS, metaPathOf, readGlueOffsets } from '../../src/app/glueLab/meta'
import type { GlueOffsets } from '../../src/app/glueLab/meta'
import { estimatePitch } from '../../src/app/glueLab/pitch'
import type { PitchEstimate } from '../../src/app/glueLab/pitch'
import { onThemeChange, palette } from './theme'
import { View2d } from './view2d'
import { ViewCube } from './viewcube'
import { View3d } from './view3d'

const $ = <T extends HTMLElement>(id: string): T => {
  const el = document.getElementById(id)
  if (!el) throw new Error(`缺元素 #${id}`)
  return el as T
}

const state = {
  fileName: '',
  blocks: [] as LabBlock[],
  index: 0,
  lines: 2,
  view: '2d' as '2d' | '3d',
  selected: null as number | null,
  field: null as EdgeField | null,
  pitch: { mm: 0.4, samples: 0, fallback: true } as PitchEstimate,
  mask: null as Uint8Array | null,
  predicted: new Uint8Array(0),
  /** 每块有没有窄面（后台扫，见 `scanNarrow`） */
  narrow: new Map<number, boolean>(),
  print: null as PrintModel | null,
  /** 产物原样的涂胶块（没做偏移平移） */
  rawBlocks: [] as LabBlock[],
  /** 这次运行实际生效的偏移（读不到 meta 时全是 0） */
  actualOffsets: { ...ZERO_OFFSETS } as GlueOffsets,
  /** 界面上填的偏移：与 `actualOffsets` 相等 = 照原样显示 */
  offsets: { ...ZERO_OFFSETS } as GlueOffsets,
  /** 层滑块指向第几层（`state.print.layers` 的下标） */
  layerIndex: 0,
  layerMode: 'all' as 'all' | 'current',
  /** 正拖着缩放滑杆：这期间别让视图把值写回来 */
  zoomBusy: false,
}

const el = {
  drop: $('drop'),
  file: $('file'),
  lines: $<HTMLInputElement>('lines'),
  linesVal: $('lines-val'),
  pitch: $('pitch'),
  lift: $('lift'),
  exag: $<HTMLInputElement>('exag'),
  exagVal: $('exag-val'),
  view: $('view'),
  blocks: $('blocks'),
  blocksCount: $('blocks-count'),
  detail: $('detail'),
  detailTitle: $('detail-title'),
  detailSub: $('detail-sub'),
  faces: $('faces'),
  scale: $('scale'),
  badge: $('badge'),
  layers: $('layers'),
  layerOther: $('layer-other'),
  reset: $('reset'),
  explode: $('explode'),
  real: $('real'),
  object: $<HTMLSelectElement>('object'),
  offset: $('offset'),
  offNote: $('off-note'),
  offX: $<HTMLInputElement>('off-x'),
  offY: $<HTMLInputElement>('off-y'),
  offTz: $<HTMLInputElement>('off-tz'),
  offGz: $<HTMLInputElement>('off-gz'),
  offZero: $('off-zero'),
  refit: $('refit'),
  ortho: $('ortho'),
  layermode: $('layermode'),
  side: $('side'),
  layer: $<HTMLInputElement>('layer'),
  layerVal: $('layer-val'),
  zoom: $<HTMLInputElement>('zoom'),
  zoomVal: $('zoom-val'),
  cube: $<HTMLCanvasElement>('cube'),
  c2d: $<HTMLCanvasElement>('c2d'),
  c3d: $<HTMLCanvasElement>('c3d'),
  cz: $<HTMLCanvasElement>('cz'),
  busy: $('busy'),
  busyText: $('busy-text'),
  stripTitle: $('strip-title'),
}

const view2d = new View2d(
  el.c2d,
  (mmPer100) => {
    el.scale.textContent = `100px = ${mmPer100.toFixed(1)}mm`
    /* 2D 的缩放滑杆跟着滚轮走（2D 那套是「以光标为锚点」的，这里只做回显） */
    if (!state.zoomBusy && state.view === '2d') {
      el.zoom.value = String(toSlider(100 / mmPer100, ZOOM_RANGE['2d']))
      /* 2D 的比例尺在右下角那块，这里不重复写数字 */
      el.zoomVal.textContent = ''
    }
  },
  (index) => {
    state.selected = index
    view2d.select(index)
    renderDetail()
    renderStrip()
  },
)
const view3d = new View3d(el.c3d)
const viewCube = new ViewCube(el.cube)
viewCube.onPick = (dir) => view3d.setDirection(dir)

const rule = (): EdgeRule => ({ lines: state.lines, pitchMm: state.pitch.mm })

/** 底部台阶带：`renderStrip` 画一条就记一条，悬停 / 点击靠它反查移动 */
let stripHits: { x0: number; x1: number; mi: number }[] = []
const STRIP_GAP = 4

/** 先拖进来的 meta：等同名 gcode；分两次拖时靠它配对 */
let pendingMeta: { name: string; data: unknown } | null = null

/* ---------------- 文件 ---------------- */

function nextPaint(): Promise<void> {
  return new Promise((resolve) => requestAnimationFrame(() => setTimeout(resolve, 0)))
}

/**
 * 大文件解析前先把模态挂上去。
 *
 * 直接同步 parse 的话，浏览器一帧都拿不到 —— 转圈根本来不及画出来。
 * 让一帧再干活，至少用户看得见"在忙"；结束再让一帧，免得模态与结果同帧闪一下。
 */
async function loadText(name: string, text: string, meta?: unknown): Promise<void> {
  const began = performance.now()
  el.busyText.textContent = '解析中…'
  el.busy.classList.remove('hidden')
  await nextPaint()
  try {
    loadTextSync(name, text, meta)
  } finally {
    el.busyText.textContent = '完成'
    await nextPaint()
    /* 小文件解析不到一帧：让它至少挂 240ms，别闪一下就没了 */
    const left = 240 - (performance.now() - began)
    if (left > 0) await new Promise((resolve) => setTimeout(resolve, left))
    el.busy.classList.add('hidden')
  }
}

function loadTextSync(name: string, text: string, meta?: unknown): void {
  state.rawBlocks = parseBlocks(text)
  state.actualOffsets = (meta === undefined ? null : readGlueOffsets(meta)) ?? { ...ZERO_OFFSETS }
  /* 一开始按「真实生效的那一套」显示：偏移与产物里的一模一样 */
  state.offsets = { ...state.actualOffsets }
  fillOffsetInputs()
  el.offNote.textContent = meta === undefined ? '未读到 meta · 可再拖同名 meta' : '已读 meta'
  el.offset.classList.remove('hidden')
  const blocks = shiftBlocks(state.rawBlocks, offsetShift())
  state.fileName = name
  state.blocks = blocks
  state.index = 0
  state.selected = null
  el.file.textContent = `${name} · ${blocks.length} 块`
  el.blocksCount.textContent = String(blocks.length)
  el.drop.classList.add('off')
  el.badge.textContent = '实际 = 产物里的 Z · 预测 = 试验台口径（只算涂胶线）'
  el.badge.title =
    '产物里的 Z 是当时那套参数做的决定；预测按顶栏「外几条 × 线距」现算，两者本来就可能不一样。'
  el.badge.classList.remove('hidden')
  renderBlocks()
  const print = parsePrint(text)
  state.print = print
  /* 认不出的 `; FEATURE:` 不给它单开一格，折进「其它」那张；一格都没有就不显示 */
  el.layerOther.classList.toggle('hidden', print.unknown.length === 0)
  /* 对象下拉：全部 + 每个 `label id`；只有 0 号（不属于任何模型的移动）时不显示 */
  const ids = print.objects.filter((o) => o.id !== 0)
  el.object.innerHTML = '<option value="">全部模型</option>'
  for (const obj of ids) {
    const opt = document.createElement('option')
    opt.value = String(obj.id)
    opt.textContent = `#${obj.id}`
    el.object.appendChild(opt)
  }
  el.object.disabled = ids.length < 2
  view3d.setObjectFilter(null)
  view3d.setPrint(print)
  view3d.setBlocks(blocks, 0)
  loadBlock(0, { refit: true })
  el.side.classList.remove('hidden')
  /* 默认停在最顶层：一开始就是「全看」，往下拖才切 */
  const top = Math.max(0, print.layers.length - 1)
  el.layer.max = String(top)
  el.layer.value = String(top)
  setLayer(top)
  scanNarrow()
}

function loadBlock(index: number, opts: { refit?: boolean; fromLayer?: boolean } = {}): void {
  const block = state.blocks[index]
  if (!block) return
  state.index = index
  /* 从左边列表点进来的：层跟着这一块走；从层滑块来的就别再回写，否则会互相拽 */
  if (opts.fromLayer !== true) syncLayerFromBlock(index)
  state.selected = null
  const landed = landedMoves(block)
  state.field = EdgeField.build(landed)
  state.pitch = estimatePitch(landed)
  el.pitch.textContent = state.pitch.fallback
    ? `${state.pitch.mm.toFixed(2)}mm 回落`
    : `${state.pitch.mm.toFixed(2)}mm 实测`
  el.lift.textContent = block.raisedZ === null ? '—' : `${block.liftMm.toFixed(2)}mm`
  refreshRule(opts.refit === true)
  view3d.select(index)
  el.c2d.classList.remove('fade-in')
  void el.c2d.offsetWidth
  el.c2d.classList.add('fade-in')
  renderBlocks()
}

function refreshRule(refit = false): void {
  const block = state.blocks[state.index]
  if (!block) return
  const r = rule()
  state.mask = state.field ? state.field.bandMask(r) : null
  state.predicted = new Uint8Array(block.moves.length)
  block.moves.forEach((m, i) => {
    if (m.level === 'up' || !state.field) return
    state.predicted[i] = state.field.isEdge(m.from, r) ? 1 : 0
  })
  view2d.setData(
    {
      block,
      field: state.field,
      rule: r,
      predicted: state.predicted,
      mask: state.mask,
    },
    refit,
  )
  view2d.select(state.selected)
  renderFaces()
  renderDetail()
  renderStrip()
}

/* ---------------- 左栏 ---------------- */

function renderBlocks(): void {
  el.blocks.innerHTML = ''
  state.blocks.forEach((block, i) => {
    const counts = countLevels(block)
    const row = document.createElement('button')
    row.type = 'button'
    row.className = `block-row${i === state.index ? ' on' : ''}`
    const narrow = state.narrow.get(block.index) === true
    row.innerHTML = `<span class="idx">#${i}</span>
      <span class="meta">Z ${block.normalZ === null ? '—' : block.normalZ.toFixed(2)} · ${counts.normal + counts.raised} 条 · 抬 ${counts.raised}</span>
      <span class="narrow" style="opacity:${narrow ? 0.9 : 0}"></span>`
    row.addEventListener('click', () => loadBlock(i))
    el.blocks.appendChild(row)
  })
}

/**
 * 后台逐块扫一遍「这一块有没有窄面」，把左栏那个红标点亮。
 *
 * 一次全算会卡半秒，所以按块让出主线程；算过的块记在 `state.narrow` 里，
 * 切块时不再重算。窄 = 短边小于 2.5mm —— 正是这次要抓的那一类。
 */
function scanNarrow(): void {
  state.narrow.clear()
  let i = 0
  const step = (): void => {
    const block = state.blocks[i]
    if (!block) return
    const field =
      i === state.index && state.field ? state.field : EdgeField.build(landedMoves(block))
    state.narrow.set(
      block.index,
      field !== null && field.faces.some((f) => f.id !== 0 && f.shortMm < 2.5),
    )
    const row = el.blocks.children[i] as HTMLElement | undefined
    const mark = row?.querySelector('.narrow')
    mark?.setAttribute('style', `opacity:${state.narrow.get(block.index) ? 0.9 : 0}`)
    i += 1
    setTimeout(step, 0)
  }
  setTimeout(step, 0)
}

/* ---------------- 右栏 ---------------- */

function renderFaces(): void {
  const block = state.blocks[state.index]
  if (!block || !state.field) {
    el.faces.innerHTML = ''
    return
  }
  const r = rule()
  const rows = state.field.faces
    .filter((f) => f.id !== 0)
    .map((f) => {
      const band = state.field?.bandMm(f.shortMm, r) ?? 0
      return `<div class="face-line${f.shortMm < 2.5 ? ' narrow' : ''}">
        <i class="dot"></i>
        <span class="short">#${f.id} ${f.shortMm.toFixed(2)}mm</span>
        <span class="band">带 ${band.toFixed(2)}mm</span>
      </div>`
    })
  el.faces.innerHTML = rows.join('') || '<div class="empty">这一块没有落笔面</div>'
}

function renderDetail(): void {
  const block = state.blocks[state.index]
  if (!block) {
    el.detail.innerHTML = ''
    return
  }
  const counts = countLevels(block)
  el.detailTitle.textContent = `逐条诊断 · #${state.index}`
  el.detailSub.textContent = `${block.segments.length} 段`
  const bad = mismatchCount(block)
  let html = `<div class="summary">
    落笔 <b>${counts.normal + counts.raised}</b> 条（正常 ${counts.normal} · 抬 ${counts.raised}）<br />
    判定不一致 <b>${bad}</b> 条${block.note ? `<br /><span style="color:#d29922">${block.note}</span>` : ''}
  </div>`

  const m = state.selected === null ? null : block.moves[state.selected]
  if (m && m.level !== 'up') {
    html += moveRows(m, state.selected as number)
  } else {
    html += '<div class="empty">点画面上任意一条涂胶线，看它那一条的判定。</div>'
  }
  el.detail.innerHTML = html
}

function mismatchCount(block: LabBlock): number {
  let n = 0
  block.moves.forEach((m, i) => {
    if (m.level === 'up') return
    if ((state.predicted[i] === 1) !== (m.level === 'raised')) n += 1
  })
  return n
}

function moveRows(m: LabMove, index: number): string {
  const field = state.field
  const r = rule()
  const d = field ? field.edgeDistance(m.from) : 0
  const short = field ? field.faceShortMm(m.from) : 0
  const band = field ? field.bandMm(short, r) : 0
  const pred = state.predicted[index] === 1
  const actual = m.level === 'raised'
  const bad = pred !== actual
  const fmt = (n: number): string => n.toFixed(2)
  return `<dl class="kv">
    <dt>行号</dt><dd>${m.line}</dd>
    <dt>起点</dt><dd>${fmt(m.from.x)}, ${fmt(m.from.y)}</dd>
    <dt>终点</dt><dd>${fmt(m.to.x)}, ${fmt(m.to.y)}</dd>
    <dt>长度</dt><dd>${fmt(Math.hypot(m.to.x - m.from.x, m.to.y - m.from.y))}mm</dd>
    <dt>实际档</dt><dd class="${bad ? 'bad' : ''}">${actual ? '抬起' : '正常'}（Z ${m.z.toFixed(2)}）</dd>
    <dt>预测</dt><dd class="${bad ? 'bad' : ''}">${pred ? '边缘带' : '里面'}</dd>
    <dt>距边界</dt><dd>${fmt(d)}mm</dd>
    <dt>面短边</dt><dd>#${field ? field.faceId(m.from) : 0} ${fmt(short)}mm</dd>
    <dt>带宽</dt><dd>${fmt(band)}mm</dd>
  </dl>`
}

/* ---------------- 底部 Z 阶梯 ---------------- */

function renderStrip(): void {
  const block = state.blocks[state.index]
  const canvas = el.cz
  const rect = canvas.getBoundingClientRect()
  const dpr = window.devicePixelRatio || 1
  const w = Math.max(1, rect.width)
  const h = Math.max(1, rect.height)
  canvas.width = Math.round(w * dpr)
  canvas.height = Math.round(h * dpr)
  const ctx = canvas.getContext('2d')
  if (!ctx) return
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
  ctx.clearRect(0, 0, w, h)
  stripHits = []
  if (!block) return

  const landed: number[] = []
  block.moves.forEach((m, i) => {
    if (m.level !== 'up') landed.push(i)
  })
  if (landed.length === 0) return

  const padL = 12
  const padR = 12
  const top = 14
  const bottom = h - 16
  const gaps = Math.max(0, block.segments.length - 1)
  const bw = (w - padL - padR - gaps * STRIP_GAP) / landed.length

  /* 两条档位基线 */
  ctx.strokeStyle = palette().grid
  ctx.lineWidth = 1
  ctx.beginPath()
  for (const y of [top, bottom]) {
    ctx.moveTo(padL, Math.round(y) + 0.5)
    ctx.lineTo(w - padR, Math.round(y) + 0.5)
  }
  ctx.stroke()

  /* 一段一块：浅底 + 细边 + 段号，移动之间不再糊成一条长带 */
  block.segments.forEach((seg, si) => {
    const idxs: number[] = []
    for (let mi = seg.first; mi <= seg.last; mi += 1) {
      if (block.moves[mi]?.level !== 'up') idxs.push(mi)
    }
    if (idxs.length === 0) return
    const before = landed.indexOf(idxs[0] as number)
    const x0 = padL + before * bw + si * STRIP_GAP
    const x1 = x0 + idxs.length * bw
    ctx.fillStyle = palette().bg2
    ctx.globalAlpha = 0.5
    ctx.fillRect(x0, top - 2, x1 - x0, bottom - top + 4)
    ctx.globalAlpha = 1
    ctx.strokeStyle = palette().line
    ctx.strokeRect(Math.round(x0) + 0.5, Math.round(top - 2) + 0.5, Math.round(x1 - x0), bottom - top + 3)
    if (x1 - x0 > 28) {
      ctx.fillStyle = palette().dim
      ctx.font = '9px ui-monospace, monospace'
      ctx.fillText(`#${si + 1}`, x0 + 3, top - 5)
    }
  })

  landed.forEach((mi, k) => {
    const m = block.moves[mi]
    const raised = m.level === 'raised'
    const pred = state.predicted[mi] === 1
    const bad = pred !== raised
    const seg = block.segments.findIndex((v) => mi >= v.first && mi <= v.last)
    const x = padL + k * bw + Math.max(0, seg) * STRIP_GAP
    const y = raised ? top : bottom - 10
    ctx.fillStyle = bad ? palette().red : raised ? palette().amber : palette().teal
    ctx.fillRect(x, y, Math.max(1, bw - 0.6), 10)
    if (mi === state.selected) {
      ctx.strokeStyle = palette().text
      ctx.lineWidth = 1
      ctx.strokeRect(Math.round(x) + 0.5, Math.round(y) + 0.5, Math.max(1, bw - 1), 9)
    }
    stripHits.push({ x0: x, x1: x + bw, mi })
  })

  ctx.fillStyle = palette().dim
  ctx.font = '10px ui-monospace, monospace'
  ctx.fillText('抬起', padL, top - 4)
  ctx.fillText('正常', padL, bottom + 12)
  el.stripTitle.textContent = `落笔档（${block.segments.length} 段 · 按移动顺序）`
}

/** 底部台阶带：悬停读行号，点击选中那一条 */
function stripHitAt(x: number): number | null {
  const hit = stripHits.find((h) => x >= h.x0 && x < h.x1)
  return hit ? hit.mi : null
}

el.cz.addEventListener('pointermove', (e) => {
  const rect = el.cz.getBoundingClientRect()
  const mi = stripHitAt(e.clientX - rect.left)
  const m = mi === null ? null : (state.blocks[state.index]?.moves[mi] ?? null)
  if (m === null) {
    el.stripTitle.textContent = `落笔档（${state.blocks[state.index]?.segments.length ?? 0} 段 · 按移动顺序）`
    return
  }
  const pred = mi !== null && state.predicted[mi] === 1
  el.stripTitle.textContent = `行 ${m.line} · ${m.level === 'raised' ? '抬起' : '正常'} · 判定${pred ? '边缘' : '内侧'}`
})

el.cz.addEventListener('pointerleave', renderStrip)

el.cz.addEventListener('click', (e) => {
  const rect = el.cz.getBoundingClientRect()
  const mi = stripHitAt(e.clientX - rect.left)
  if (mi === null) return
  state.selected = mi
  view2d.select(mi)
  renderDetail()
  renderStrip()
})

/* ---------------- 事件 ---------------- */

el.lines.addEventListener('input', () => {
  state.lines = Number(el.lines.value)
  el.linesVal.textContent = String(state.lines)
  refreshRule()
})

function setExag(n: number): void {
  el.exag.value = String(Math.round(n))
  el.exagVal.textContent = String(Math.round(n))
  view3d.setExaggeration(n)
}

el.exag.addEventListener('input', () => setExag(Number(el.exag.value)))

/* 真实高度：Z 倍数回 1:1，抬 0.5mm 就画 0.5mm */
el.real.addEventListener('click', () => {
  setExag(1)
  view3d.refit()
})

/* 爆炸动画每帧把当前倍数报回来，顶栏必须跟着走 */
view3d.onExaggeration = (z) => {
  el.exag.value = String(Math.round(z))
  el.exagVal.textContent = String(Math.round(z))
}

function syncZoomReadout(): void {
  /* 两把尺子不是同一个量，切过去先把滑杆摆到当前视图的位置 */
  if (state.view === '3d') {
    const d = view3d.distance()
    el.zoom.value = String(toSlider(d, ZOOM_RANGE['3d']))
    el.zoomVal.textContent = d < 10 ? `视距 ${d.toFixed(2)}` : `视距 ${d.toFixed(0)}`
  } else {
    el.zoom.value = String(toSlider(view2d.scaleOf(), ZOOM_RANGE['2d']))
    el.zoomVal.textContent = ''
  }
}

function applyView(next: '2d' | '3d'): void {
  state.view = next
  el.view.querySelectorAll('button').forEach((b) => {
    b.classList.toggle('on', b.dataset.view === next)
  })
  el.c2d.classList.toggle('hidden', next !== '2d')
  el.scale.classList.toggle('hidden', next !== '2d')
  el.layers.classList.toggle('hidden', next !== '3d')
  el.cube.classList.toggle('hidden', next !== '3d')
  if (next === '3d') viewCube.resize()
  view3d.setVisible(next === '3d')
  if (next === '2d') view2d.resize()
  syncZoomReadout()
}

el.view.addEventListener('click', (e) => {
  const btn = (e.target as HTMLElement).closest('button')
  if (!btn) return
  const next = btn.dataset.view === '3d' ? '3d' : '2d'
  if (next === state.view) return
  if (next === '2d') {
    /*
     * 伪 2D：3D 先翻到与 2D 同朝向、同取景的顶视；末帧画完的那一帧直接切画布，
     * 不淡入淡出 —— 2D 第一帧接的就是 3D 最后一帧。
     */
    view3d.flyToTop(view2d.viewOf(), () => applyView('2d'))
    return
  }
  /*
   * 从 2D 回 3D：把 3D 摆出来后，立刻用**当前 2D 视图**摆成伪顶视，
   * 再从这个姿态飞回原机位。两步在同一个任务里，第一帧不会露出旧相机。
   */
  const view = view2d.viewOf()
  applyView('3d')
  view3d.flyFromTop(view, () => {})
  syncZoomReadout()
})

el.object.addEventListener('change', () => {
  const v = el.object.value
  view3d.setObjectFilter(v === '' ? null : Number(v))
})

/* 「视角」= 当前视图的还原：2D 回到铺满那一块，3D 回到默认机位 */
el.reset.addEventListener('click', () => {
  if (state.view === '2d') view2d.fit()
  else view3d.resetView()
})

/* 「居中」= 只摆位置、不动缩放与朝向 */
el.refit.addEventListener('click', () => {
  if (state.view === '2d') view2d.recenter()
  else view3d.refit()
})

/* 爆炸图：一层一层沿 Z 拉开，镜头同时往后拉、往上抬 */
el.explode.addEventListener('click', () => {
  view3d.toggleExplode()
  el.explode.setAttribute('aria-pressed', view3d.isExploded() ? 'true' : 'false')
})

/* 透视 / 正交：钮上写的是**现在**是哪一档 */
el.ortho.addEventListener('click', () => {
  view3d.setOrtho(!view3d.isOrtho())
  el.ortho.textContent = view3d.isOrtho() ? '正交' : '透视'
})

/* ---- 涂胶偏移 ---- */

function offsetShift(): { dx: number; dy: number; dz: number } {
  return {
    dx: state.offsets.x - state.actualOffsets.x,
    dy: state.offsets.y - state.actualOffsets.y,
    dz: 0,
  }
}

/** 两个 Z 偏移各自与「实际」的差，加在一起交给 3D 抬高涂胶 */
function glueZDelta(): number {
  return (
    state.offsets.toolZ -
    state.actualOffsets.toolZ +
    (state.offsets.glueZ - state.actualOffsets.glueZ)
  )
}

function fillOffsetInputs(): void {
  el.offX.value = String(round2(state.offsets.x))
  el.offY.value = String(round2(state.offsets.y))
  el.offTz.value = String(round2(state.offsets.toolZ))
  el.offGz.value = String(round2(state.offsets.glueZ))
}

function round2(v: number): number {
  return Math.round(v * 100) / 100
}

function readOffsetInputs(): void {
  const num = (input: HTMLInputElement): number => {
    const v = Number(input.value)
    return Number.isFinite(v) ? v : 0
  }
  state.offsets = {
    x: num(el.offX),
    y: num(el.offY),
    toolZ: num(el.offTz),
    glueZ: num(el.offGz),
  }
}

/** 偏移改了：块整体平移（2D/3D 都吃这一份），Z 那两笔交给 3D */
function applyOffsets(): void {
  state.blocks = shiftBlocks(state.rawBlocks, offsetShift())
  view3d.setBlocks(state.blocks, state.index, false)
  view3d.setGlueZShift(glueZDelta())
  renderBlocks()
  loadBlock(state.index, { fromLayer: true })
}

for (const input of [el.offX, el.offY, el.offTz, el.offGz]) {
  input.addEventListener('input', () => {
    readOffsetInputs()
    applyOffsets()
  })
}

el.offZero.addEventListener('click', () => {
  state.offsets = { ...ZERO_OFFSETS }
  fillOffsetInputs()
  applyOffsets()
})

/* ---- 后到的 meta ---- */

/** 分两次拖：gcode 已打开，后来补上同名 meta */
function applyMeta(name: string, meta: unknown): void {
  const offsets = readGlueOffsets(meta)
  if (offsets === null) {
    el.offNote.textContent = `${name} 里没有偏移`
    return
  }
  state.actualOffsets = { ...offsets }
  state.offsets = { ...offsets }
  fillOffsetInputs()
  el.offNote.textContent = '已读 meta'
  applyOffsets()
}

/** 从 zip 里读 gcode 与配对的 meta；没有 gcode 就返回 null */
async function readZip(file: File): Promise<{ name: string; text: string; meta?: unknown } | null> {
  const entries = unzipSync(new Uint8Array(await file.arrayBuffer()))
  const names = Object.keys(entries).filter((n) => !n.endsWith('/'))
  const gcodeName = names.find((n) => /\.gcode$/i.test(n))
  if (gcodeName === undefined) return null
  const decoder = new TextDecoder()
  const text = decoder.decode(entries[gcodeName])
  const metaName = findMetaName(gcodeName, names)
  let meta: unknown
  if (metaName !== null) {
    try {
      meta = JSON.parse(decoder.decode(entries[metaName]))
    } catch {
      meta = undefined
    }
  }
  return { name: gcodeName.split(/[\\/]/).pop() ?? gcodeName, text, meta }
}

async function readMeta(file: File): Promise<unknown> {
  try {
    return JSON.parse(await file.text()) as unknown
  } catch {
    return undefined
  }
}

/* ---- 层滑块 ---- */

function blockLayer(block: LabBlock): number {
  return block.layerZ ?? block.normalZ ?? 0
}

function updateLayerLabel(): void {
  const layers = state.print?.layers ?? []
  if (layers.length === 0) return
  const i = Math.min(state.layerIndex, layers.length - 1)
  const z = layers[i] as number
  el.layerVal.textContent = `${i + 1}/${layers.length} Z${z.toFixed(2)}`
}

/**
 * 选中某一块时把层跟着挪过去。
 *
 * 取「第一个 ≥ 这一块层高」的打印层：切面画的是「看到这一层为止」，
 * 取下面那层会把这一块自己切没，于是看起来像两层混在一起。
 */
function syncLayerFromBlock(index: number): void {
  const layers = state.print?.layers ?? []
  const block = state.blocks[index]
  if (layers.length === 0 || block === undefined) return
  const z = blockLayer(block)
  let i = layers.findIndex((v) => v >= z - 1e-9)
  if (i < 0) i = layers.length - 1
  if (i === state.layerIndex) return
  state.layerIndex = i
  el.layer.value = String(i)
  updateLayerLabel()
  view3d.setLayer(i, state.layerMode)
}

function setLayer(index: number): void {
  const layers = state.print?.layers ?? []
  if (layers.length === 0) return
  const i = Math.max(0, Math.min(index, layers.length - 1))
  state.layerIndex = i
  updateLayerLabel()
  view3d.setLayer(i, state.layerMode)
  /*
   * 涂胶块跟着层走：取**不高于这一层的最后一块**。
   * 不用「最近的一块」—— 那样在两条涂胶层之间来回拖会前后跳（作者报的「位置总在变化」），
   * 而且「这一层时的涂胶状态」本来就是最近一次涂过的那块。
   */
  const z = layers[i] as number
  let best = 0
  state.blocks.forEach((block, k) => {
    if (blockLayer(block) <= z + 1e-9) best = k
  })
  if (best !== state.index) loadBlock(best, { fromLayer: true })
}

el.layer.addEventListener('input', () => setLayer(Number(el.layer.value)))

el.layermode.addEventListener('click', () => {
  state.layerMode = state.layerMode === 'all' ? 'current' : 'all'
  el.layermode.textContent = state.layerMode === 'all' ? '全部层' : '当前层'
  setLayer(state.layerIndex)
})

/* ---- 缩放滑杆（2D / 3D 同一把，对数刻度） ---- */

const ZOOM_RANGE = {
  '2d': { lo: 0.4, hi: 400 },
  '3d': { lo: 0.01, hi: 2000 },
} as const

function fromSlider(t: number, range: { lo: number; hi: number }): number {
  const a = Math.log10(range.lo)
  return 10 ** (a + (Math.log10(range.hi) - a) * t)
}

function toSlider(v: number, range: { lo: number; hi: number }): number {
  const a = Math.log10(range.lo)
  const t = (Math.log10(Math.max(range.lo, Math.min(range.hi, v))) - a) / (Math.log10(range.hi) - a)
  return Math.round(t * 1000)
}

el.zoom.addEventListener('input', () => {
  const t = Number(el.zoom.value) / 1000
  if (state.view === '2d') view2d.setScale(fromSlider(t, ZOOM_RANGE['2d']))
  else view3d.setDistance(fromSlider(t, ZOOM_RANGE['3d']))
})

for (const ev of ['pointerdown', 'pointerup', 'pointercancel'] as const) {
  el.zoom.addEventListener(ev, () => {
    state.zoomBusy = ev === 'pointerdown'
  })
}

/* 3D 每帧把视距报回来，滑杆与读数跟着滚轮走 */
view3d.onFrame = (d) => {
  viewCube.update(view3d.cameraQuaternion())
  if (state.zoomBusy || state.view !== '3d') return
  el.zoom.value = String(toSlider(d, ZOOM_RANGE['3d']))
  el.zoomVal.textContent = d < 10 ? `视距 ${d.toFixed(2)}` : `视距 ${d.toFixed(0)}`
}

/*
 * 显示开关：点一下切换；**按住往下拖会一路扫过去**，把经过的那几行都设成同一档
 * （Blender 大纲视图里按着眼睛往下抹那种）。一次拖拽里每行只设一次，不会来回翻。
 */
let paintOn: boolean | null = null

function setKindRow(row: HTMLElement, on: boolean): void {
  row.setAttribute('aria-pressed', on ? 'true' : 'false')
  row.querySelector('use')?.setAttribute('href', on ? '#eye-on' : '#eye-off')
  const kind = row.dataset.kind
  if (kind) view3d.setKindVisible(kind, on)
}

/** 光标底下是哪一行（拖拽期间指针被捕获在面板上，只能靠坐标找） */
function rowAt(x: number, y: number): HTMLElement | null {
  const hit = document.elementFromPoint(x, y)?.closest('[data-kind]')
  return hit instanceof HTMLElement && el.layers.contains(hit) ? hit : null
}

el.layers.addEventListener('pointerdown', (e) => {
  const row = (e.target as HTMLElement).closest<HTMLElement>('[data-kind]')
  if (!row) return
  e.preventDefault()
  paintOn = row.getAttribute('aria-pressed') !== 'true'
  setKindRow(row, paintOn)
  el.layers.setPointerCapture(e.pointerId)
})

el.layers.addEventListener('pointermove', (e) => {
  if (paintOn === null) return
  const row = rowAt(e.clientX, e.clientY)
  if (row) setKindRow(row, paintOn)
})

for (const ev of ['pointerup', 'pointercancel'] as const) {
  window.addEventListener(ev, () => {
    paintOn = null
  })
}

/* 键盘：回车 / 空格切换当前这一行 */
el.layers.addEventListener('keydown', (e) => {
  if (e.key !== 'Enter' && e.key !== ' ') return
  const row = (e.target as HTMLElement).closest<HTMLElement>('[data-kind]')
  if (!row) return
  e.preventDefault()
  setKindRow(row, row.getAttribute('aria-pressed') !== 'true')
})

window.addEventListener('keydown', (e) => {
  if (e.key !== 'ArrowUp' && e.key !== 'ArrowDown') return
  const delta = e.key === 'ArrowUp' ? -1 : 1
  const next = Math.min(state.blocks.length - 1, Math.max(0, state.index + delta))
  if (next !== state.index) {
    e.preventDefault()
    loadBlock(next)
  }
})

window.addEventListener('resize', () => {
  view2d.resize()
  view3d.resize()
  viewCube.resize()
  renderStrip()
})

/* 系统换深浅色：canvas 的颜色是从 CSS 变量读的，缓存清了就得整张重画 */
onThemeChange(() => {
  view2d.resize()
  view3d.refreshTheme()
  viewCube.refreshTheme()
  renderStrip()
})

window.addEventListener('dragover', (e) => {
  e.preventDefault()
  el.drop.classList.remove('off')
})

window.addEventListener('drop', async (e) => {
  e.preventDefault()
  const files = [...(e.dataTransfer?.files ?? [])]
  if (files.length === 0) return

  /* zip：gcode + meta 打在一个包里 */
  const zip = files.find((f) => /\.zip$/i.test(f.name))
  if (zip) {
    const inside = await readZip(zip)
    if (inside === null) {
      el.offNote.textContent = 'zip 里没有 gcode'
      return
    }
    await loadText(inside.name, inside.text, inside.meta)
    return
  }

  const gcode = files.find((f) => /\.gcode$/i.test(f.name))
  const metaFile = files.find((f) => /_meta\.json$/i.test(f.name))

  /* 一次拖两份：按同名配对，别把别的模型的 meta 拿过来 */
  if (gcode && metaFile) {
    const meta = await readMeta(metaFile)
    await loadText(gcode.name, await gcode.text(), meta)
    return
  }

  /* 先拖 gcode：照常打开；后面再拖同名 meta 时只补偏移 */
  if (gcode) {
    const ready = pendingMeta && pairsWith(gcode.name, pendingMeta.name) ? pendingMeta.data : undefined
    pendingMeta = null
    await loadText(gcode.name, await gcode.text(), ready)
    return
  }

  /* 先拖 meta：如果 gcode 已经开着就立刻补上，否则等下一份 gcode */
  if (metaFile) {
    const meta = await readMeta(metaFile)
    if (state.fileName !== '' && pairsWith(state.fileName, metaFile.name)) {
      applyMeta(metaFile.name, meta)
      return
    }
    pendingMeta = { name: metaFile.name, data: meta }
    el.offNote.textContent = '已收 meta · 再拖同名 gcode'
  }
})

/* 拖进来之后遮罩要能再藏回去 */
el.drop.addEventListener('dragleave', () => {
  if (state.blocks.length > 0) el.drop.classList.add('off')
})

/*
 * `?gcode=<同源路径>`：上手时省掉一次拖文件。路径要能被 dev server 取到
 * （仓库根下的文件都行，比如先拷到 `.scratch/lab/`）。
 */
/** 同目录下的元数据；没有就当没读到（404 不算错） */
async function fetchMeta(path: string): Promise<unknown> {
  try {
    const r = await fetch(path)
    if (!r.ok) return undefined
    return (await r.json()) as unknown
  } catch {
    return undefined
  }
}

const preset = new URLSearchParams(location.search).get('gcode')
if (preset) {
  fetch(preset)
    .then((r) => (r.ok ? r.text() : Promise.reject(new Error(`HTTP ${r.status}`))))
    .then(async (text) => {
      const meta = await fetchMeta(metaPathOf(preset))
      await loadText(preset.split('/').pop() ?? preset, text, meta)
    })
    .catch((err: unknown) => {
      el.file.textContent = `读不到 ${preset}：${String(err)}`
    })
}

view2d.resize()
renderStrip()
