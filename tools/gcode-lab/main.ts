/*
 * 试验台装配：拖入 → 切块 → 2D/3D → 逐条诊断。
 *
 * 两列口径别混：
 *   实际 = 产物里的 Z（`parse.ts` 读出来的 `level`）—— 内核真做的决定；
 *   预测 = `band.ts` 按当前「外几条 × 线距」算的 —— 只在这里存在。
 * 顶栏滑杆只重算预测，不重解析文件。
 */

import { EdgeField } from '../../src/app/glueLab/band'
import type { EdgeRule } from '../../src/app/glueLab/band'
import { countLevels, landedMoves, parseBlocks } from '../../src/app/glueLab/parse'
import type { LabBlock, LabMove } from '../../src/app/glueLab/parse'
import { estimatePitch } from '../../src/app/glueLab/pitch'
import type { PitchEstimate } from '../../src/app/glueLab/pitch'
import { onThemeChange, palette } from './theme'
import { View2d } from './view2d'
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
  c2d: $<HTMLCanvasElement>('c2d'),
  c3d: $<HTMLCanvasElement>('c3d'),
  cz: $<HTMLCanvasElement>('cz'),
}

const view2d = new View2d(
  el.c2d,
  (mmPer100) => {
    el.scale.textContent = `100px = ${mmPer100.toFixed(1)}mm`
  },
  (index) => {
    state.selected = index
    view2d.select(index)
    renderDetail()
  },
)
const view3d = new View3d(el.c3d)

const rule = (): EdgeRule => ({ lines: state.lines, pitchMm: state.pitch.mm })

/* ---------------- 文件 ---------------- */

function loadText(name: string, text: string): void {
  const blocks = parseBlocks(text)
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
  view3d.setBlocks(blocks, 0)
  loadBlock(0)
  scanNarrow()
}

function loadBlock(index: number): void {
  const block = state.blocks[index]
  if (!block) return
  state.index = index
  state.selected = null
  const landed = landedMoves(block)
  state.field = EdgeField.build(landed)
  state.pitch = estimatePitch(landed)
  el.pitch.textContent = state.pitch.fallback
    ? `${state.pitch.mm.toFixed(2)}mm 回落`
    : `${state.pitch.mm.toFixed(2)}mm 实测`
  el.lift.textContent = block.raisedZ === null ? '—' : `${block.liftMm.toFixed(2)}mm`
  refreshRule()
  view3d.select(index)
  el.c2d.classList.remove('fade-in')
  void el.c2d.offsetWidth
  el.c2d.classList.add('fade-in')
  renderBlocks()
}

function refreshRule(): void {
  const block = state.blocks[state.index]
  if (!block) return
  const r = rule()
  state.mask = state.field ? state.field.bandMask(r) : null
  state.predicted = new Uint8Array(block.moves.length)
  block.moves.forEach((m, i) => {
    if (m.level === 'up' || !state.field) return
    state.predicted[i] = state.field.isEdge(m.from, r) ? 1 : 0
  })
  view2d.setData({
    block,
    field: state.field,
    rule: r,
    predicted: state.predicted,
    mask: state.mask,
  })
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
  const bw = (w - padL - padR) / landed.length

  /* 两条档位基线 */
  ctx.strokeStyle = palette().grid
  ctx.lineWidth = 1
  ctx.beginPath()
  for (const y of [top, bottom]) {
    ctx.moveTo(padL, Math.round(y) + 0.5)
    ctx.lineTo(w - padR, Math.round(y) + 0.5)
  }
  ctx.stroke()

  landed.forEach((mi, k) => {
    const m = block.moves[mi]
    const raised = m.level === 'raised'
    const pred = state.predicted[mi] === 1
    const bad = pred !== raised
    const x = padL + k * bw
    const y = raised ? top : bottom - 10
    ctx.fillStyle = bad ? palette().red : raised ? palette().amber : palette().teal
    ctx.fillRect(x, y, Math.max(1, bw - 0.6), 10)
  })

  ctx.fillStyle = palette().dim
  ctx.font = '10px ui-monospace, monospace'
  ctx.fillText('抬起', padL, top - 4)
  ctx.fillText('正常', padL, bottom + 12)
}

/* ---------------- 事件 ---------------- */

el.lines.addEventListener('input', () => {
  state.lines = Number(el.lines.value)
  el.linesVal.textContent = String(state.lines)
  refreshRule()
})

el.exag.addEventListener('input', () => {
  const n = Number(el.exag.value)
  el.exagVal.textContent = String(n)
  view3d.setExaggeration(n)
})

el.view.addEventListener('click', (e) => {
  const btn = (e.target as HTMLElement).closest('button')
  if (!btn) return
  const next = btn.dataset.view === '3d' ? '3d' : '2d'
  state.view = next
  el.view.querySelectorAll('button').forEach((b) => b.classList.toggle('on', b === btn))
  el.c2d.classList.toggle('hidden', next !== '2d')
  el.scale.classList.toggle('hidden', next !== '2d')
  view3d.setVisible(next === '3d')
  if (next === '2d') view2d.resize()
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
  renderStrip()
})

/* 系统换深浅色：canvas 的颜色是从 CSS 变量读的，缓存清了就得整张重画 */
onThemeChange(() => {
  view2d.resize()
  view3d.refreshTheme()
  renderStrip()
})

window.addEventListener('dragover', (e) => {
  e.preventDefault()
  el.drop.classList.remove('off')
})

window.addEventListener('drop', async (e) => {
  e.preventDefault()
  const file = e.dataTransfer?.files[0]
  if (!file) return
  const text = await file.text()
  loadText(file.name, text)
})

/* 拖进来之后遮罩要能再藏回去 */
el.drop.addEventListener('dragleave', () => {
  if (state.blocks.length > 0) el.drop.classList.add('off')
})

/*
 * `?gcode=<同源路径>`：上手时省掉一次拖文件。路径要能被 dev server 取到
 * （仓库根下的文件都行，比如先拷到 `.scratch/lab/`）。
 */
const preset = new URLSearchParams(location.search).get('gcode')
if (preset) {
  fetch(preset)
    .then((r) => (r.ok ? r.text() : Promise.reject(new Error(`HTTP ${r.status}`))))
    .then((text) => loadText(preset.split('/').pop() ?? preset, text))
    .catch((err: unknown) => {
      el.file.textContent = `读不到 ${preset}：${String(err)}`
    })
}

view2d.resize()
renderStrip()
