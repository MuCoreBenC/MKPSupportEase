/*
 * 右栏底部的 G-code 面板：当前行上下各 N 行，外加产物/原件的行对照。
 *
 * 只负责渲染与把「点了哪一行」报出去。选中哪条移动、视图往哪走、巡览怎么算，
 * 全在 `main.ts` 的 `selectMove` 一条路上 —— 这里不自己存选中。
 *
 * 列的顺序固定：行号 · 对照行 · 差异 · 标记 · 原文 · 行尾。前四列常驻左侧
 * （行号与对照行是 sticky），横向滚动只滚原文。
 */

import type { LineAlignment } from '../../src/app/glueLab/diff'
import { DIFF_ADDED, DIFF_CHANGED, DIFF_REMOVED } from '../../src/app/glueLab/diff'
import type { LineIndex } from '../../src/app/glueLab/lines'
import {
  FEATURE_NONE,
  LINE_BLOCK_BEGIN,
  LINE_BLOCK_END,
  LINE_INFO,
  LINE_MOVE,
  LINE_OTHER,
  LINE_POINT,
  LINE_STATS,
  LINE_SWAP_BEGIN,
  LINE_SWAP_END,
  featureNameOf,
} from '../../src/app/glueLab/lines'
import type { LabBlock, LabMove, PrintKind } from '../../src/app/glueLab/parse'

export type CodeAction = 'prev' | 'next' | 'start' | 'end' | 'mismatch' | 'block-begin' | 'block-end'

export interface CodePanelData {
  /** 产物全文（切好的行） */
  lines: readonly string[]
  index: LineIndex
  /** 当前涂胶块在 `blocks[]` 里的下标 */
  blockIndex: number
  block: LabBlock
  /** 选中移动在 `block.moves` 里的下标；`null` = 没选 */
  move: number | null
  predicted: Uint8Array
  /** 这一块与判定不一致的条数（0 时不出「不一致」钮） */
  mismatch: number
  /** 后处理原件；没有就是 `null`（这时「原始」页整块不出现） */
  original: readonly string[] | null
  originalIndex: LineIndex | null
  diff: LineAlignment | null
}

const FEATURE_LABEL: Record<PrintKind, string> = {
  outer: '外墙',
  inner: '内墙',
  fill: '填充',
  support: '支撑',
  supportFace: '支撑面',
  ironing: '熨烫',
  travel: '空走',
  other: '其它',
}

/**
 * 行尾的 Z / 类别角标。
 *
 * 只给**真有 XY 的移动**打：`G1 Z10.05` 是抬笔落笔、`G1 F42000` 是改速度，
 * 它们不是空走，写成「空走」会让人以为笔在飞。
 */
function noteOf(kind: number, move: LabMove | null, feature: number, text: string): string {
  if (move !== null && move.level !== 'up') return `Z${move.z.toFixed(2)}`
  if (kind !== LINE_MOVE) return ''
  if (!/\bX-?(?:\d+(?:\.\d*)?|\.\d+)/.test(text) && !/\bY-?(?:\d+(?:\.\d*)?|\.\d+)/.test(text)) return ''
  if (!/\bE-?(?:\d+(?:\.\d*)?|\.\d+)/.test(text)) return '空走'
  if (feature === FEATURE_NONE) return ''
  const name = featureNameOf(feature)
  return name === null ? '' : FEATURE_LABEL[name]
}

function tagOf(kind: number, move: LabMove | null): string {
  if (kind === LINE_BLOCK_BEGIN || kind === LINE_BLOCK_END) return '块'
  if (kind === LINE_SWAP_BEGIN || kind === LINE_SWAP_END) return '换'
  if (kind === LINE_INFO || kind === LINE_STATS || kind === LINE_POINT) return '注'
  if (kind === LINE_MOVE && move !== null) {
    if (move.level === 'normal') return '正'
    if (move.level === 'raised') return '抬'
    return '空'
  }
  return ''
}

function escapeHtml(s: string): string {
  return s.replace(/[&<>]/g, (c) => (c === '&' ? '&amp;' : c === '<' ? '&lt;' : '&gt;'))
}

export class CodePanel {
  private readonly root: HTMLElement
  private readonly headLine: HTMLElement
  private readonly sideBox: HTMLElement
  private readonly body: HTMLElement
  private data: CodePanelData | null = null
  private moveIndex: number | null = null
  /** 停在某一行（点了注释行 / 块首块尾）：下次选中移动时清掉 */
  private pin: number | null = null
  private side: 'after' | 'before' = 'after'
  /** 上下各显示几行（1 / 3 / 7，对应 3 / 7 / 15 行窗口） */
  private context = 3
  private folded = false
  /** 上一次报出去的悬停行，避免每帧重复回调 */
  private hoverLine: number | null = null
  /** 图上正指着哪一条移动：换行、重排之后要按它补回高亮 */
  private hoverMove: number | null = null

  onSelectLine: ((line: number) => void) | null = null
  onHoverLine: ((line: number | null) => void) | null = null
  onAction: ((action: CodeAction) => void) | null = null

  constructor(root: HTMLElement) {
    this.root = root
    root.innerHTML = `
      <div class="code-head">
        <span class="code-title">G-code</span>
        <span class="code-line">—</span>
        <span class="code-gap"></span>
        <button type="button" class="code-btn code-ctx" title="显示行数 3 / 7 / 15">±3</button>
        <button type="button" class="code-btn" data-act="block-begin" title="跳到这一块的第一行">⇤</button>
        <button type="button" class="code-btn" data-act="block-end" title="跳到这一块的最后一行">⇥</button>
        <button type="button" class="code-btn code-fold" title="收起 / 展开">⌄</button>
      </div>
      <div class="code-tools">
        <div class="code-side" hidden>
          <button type="button" data-side="after" class="on">产物</button>
          <button type="button" data-side="before">原始</button>
        </div>
        <span class="code-gap"></span>
        <button type="button" class="code-btn" data-act="prev" title="上一条落笔">上一条</button>
        <button type="button" class="code-btn" data-act="next" title="下一条落笔">下一条</button>
        <button type="button" class="code-btn" data-act="start" title="视图居中到这条的起点">起点</button>
        <button type="button" class="code-btn" data-act="end" title="视图居中到这条的终点">终点</button>
        <button type="button" class="code-btn code-bad" data-act="mismatch" hidden>不一致</button>
      </div>
      <div class="code-body"></div>`

    const headLine = root.querySelector<HTMLElement>('.code-line')
    const sideBox = root.querySelector<HTMLElement>('.code-side')
    const body = root.querySelector<HTMLElement>('.code-body')
    if (!headLine || !sideBox || !body) throw new Error('code 面板骨架不全')
    this.headLine = headLine
    this.sideBox = sideBox
    this.body = body

    root.addEventListener('click', this.handleClick)
    body.addEventListener('pointermove', this.handleMove)
    body.addEventListener('pointerleave', this.handleLeave)
  }

  setData(data: CodePanelData): void {
    /* 换块才清悬停：拖「外几条」重排同一块时，图上指着的那一条不该丢掉 */
    if (this.data !== null && this.data.block.index !== data.block.index) this.hoverMove = null
    this.data = data
    this.moveIndex = data.move
    this.pin = null
    this.side = data.original !== null && data.diff !== null && this.side === 'before' ? 'before' : 'after'
    const hasDiff = data.original !== null && data.diff !== null
    this.sideBox.hidden = !hasDiff
    this.root.classList.toggle('has-diff', hasDiff)
    this.sideBox.querySelectorAll<HTMLElement>('[data-side]').forEach((btn) => {
      btn.classList.toggle('on', btn.dataset.side === this.side)
    })
    this.render()
  }

  /** 图上选中 / 色带选中变化时重排窗口（只动这十几行） */
  setMove(move: number | null): void {
    this.moveIndex = move
    this.pin = null
    this.render()
  }

  /** 停在某一行：注释行、`;MKP_BEGIN` 这类没有移动的行也能看 */
  showLine(line: number): void {
    this.pin = line
    this.render()
  }

  /** 没有当前块（这一层还没涂胶）：面板留在原处，只收掉内容与跟随块走的按钮 */
  clear(): void {
    this.data = null
    this.moveIndex = null
    this.pin = null
    this.hoverMove = null
    this.headLine.textContent = '—'
    this.body.innerHTML = '<div class="code-empty">—</div>'
    this.sideBox.hidden = true
    this.root.classList.remove('has-diff')
    this.root.querySelectorAll<HTMLElement>('[data-act]').forEach((btn) => {
      btn.hidden = true
    })
  }

  /** 图上的悬停（或色带悬停）→ 代码行高亮；不滚动 */
  setHoverMove(move: number | null): void {
    this.hoverMove = move
    this.paintHover()
  }

  /** 重排窗口后要把悬停补回去：行是新建的，旧 class 跟着旧节点没了 */
  private paintHover(): void {
    for (const row of this.body.querySelectorAll<HTMLElement>('.code-row')) {
      row.classList.toggle('hover', this.hoverMove !== null && Number(row.dataset.move) === this.hoverMove)
    }
  }

  private focusLine(): number {
    const d = this.data
    if (d === null) return 0
    if (this.pin !== null) return this.pin
    const m = this.moveIndex === null ? null : d.block.moves[this.moveIndex]
    return m ? m.line : d.block.beginLine
  }

  /** 原始页停在哪：能对上就用那一行，对不上就停在这一段的插入点 */
  private focusBefore(d: CodePanelData, line: number): number {
    const diff = d.diff
    if (diff === null) return 0
    const direct = diff.origOf[line] as number | undefined
    if (direct !== undefined && direct >= 0) return direct
    const anchor = diff.anchorOf[line] as number | undefined
    return anchor !== undefined && anchor >= 0 ? anchor : 0
  }

  private render(): void {
    const d = this.data
    if (d === null) return
    const side = this.side
    const afterLine = this.focusLine()
    const focus = side === 'after' ? afterLine : this.focusBefore(d, afterLine)
    const total = side === 'after' ? d.lines.length : (d.original?.length ?? 0)
    /* 涂胶行在原件里本来就不存在：停在插入点上，明说一声，别让人以为对错了行 */
    const missing = side === 'before' && d.diff !== null && (d.diff.origOf[afterLine] as number) < 0
    this.headLine.textContent =
      total === 0 ? '—' : missing ? `行 ${focus + 1} · 原件无此行` : `行 ${focus + 1}`
    this.root.querySelector('.code-ctx')!.textContent = `±${this.context}`
    this.root.querySelectorAll<HTMLElement>('[data-act="start"],[data-act="end"]').forEach((btn) => {
      const m = this.moveIndex === null ? null : d.block.moves[this.moveIndex]
      btn.hidden = m === null || m === undefined || m.level === 'up'
    })
    this.root.querySelector<HTMLElement>('[data-act="mismatch"]')!.hidden = d.mismatch === 0

    if (total === 0) {
      this.body.innerHTML = '<div class="code-empty">没有行</div>'
      return
    }

    const half = this.context
    const from = Math.max(0, focus - half)
    const to = Math.min(total - 1, focus + half)
    const rows: string[] = []
    for (let i = from; i <= to; i += 1) {
      rows.push(side === 'after' ? this.afterRow(d, i, afterLine) : this.beforeRow(d, i, focus))
    }
    this.body.innerHTML = rows.join('')
    this.paintHover()
  }

  /** 产物页的一行：本行 + 原件里对应的那一行 */
  private afterRow(d: CodePanelData, line: number, afterLine: number): string {
    const text = d.lines[line] ?? ''
    const inBlock = d.index.blockOfLine[line] === d.blockIndex
    const moveIdx = inBlock ? (d.index.moveOfLine[line] as number) : -1
    const move = moveIdx >= 0 ? (d.block.moves[moveIdx] ?? null) : null
    const kind = d.index.kindOfLine[line] as number
    const tag = tagOf(kind, move)
    const note = noteOf(kind, move, d.index.featureOfLine[line] as number, text)
    const bad = move !== null && move.level !== 'up' && (d.predicted[moveIdx] === 1) !== (move.level === 'raised')

    let ref = ''
    let mark = ''
    const diff = d.diff
    if (diff !== null) {
      const orig = diff.origOf[line] as number
      const kind = diff.afterKind[line] as number
      ref = orig >= 0 ? String(orig + 1) : ''
      mark = kind === DIFF_ADDED ? '新' : kind === DIFF_CHANGED ? '改' : ''
    }
    return this.row(line, ref, mark, tag, bad, text, note, line === afterLine, moveIdx)
  }

  /** 原始页的一行：本行 + 产物里由它来的那一行 */
  private beforeRow(d: CodePanelData, line: number, focus: number): string {
    const text = d.original?.[line] ?? ''
    const idx = d.originalIndex
    const feature = idx ? (idx.featureOfLine[line] as number) : FEATURE_NONE
    const kind = idx ? (idx.kindOfLine[line] as number) : LINE_OTHER
    const note = noteOf(kind, null, feature, text)
    const tag = idx ? tagOf(kind, null) : ''

    let ref = ''
    let mark = ''
    const diff = d.diff
    if (diff !== null) {
      const after = diff.afterOf[line] as number
      const beforeKind = diff.beforeKind[line] as number
      ref = after >= 0 ? String(after + 1) : ''
      mark = beforeKind === DIFF_REMOVED ? '删' : beforeKind === DIFF_CHANGED ? '改' : ''
    }
    /* 高亮停在窗口中心那一行本身：涂胶行在原件里没有对应行时也能看出「停在哪」 */
    return this.row(line, ref, mark, tag, false, text, note, line === focus, -1)
  }

  private row(
    line: number,
    ref: string,
    mark: string,
    tag: string,
    bad: boolean,
    text: string,
    note: string,
    on: boolean,
    move: number,
  ): string {
    const cls = `code-row${on ? ' on' : ''}`
    const markCls = mark === '新' ? 'add' : mark === '删' ? 'del' : mark === '改' ? 'chg' : ''
    const tagCls = tag === '正' ? 'ok' : tag === '抬' ? 'raised' : tag === '空' ? 'up' : ''
    return `<div class="${cls}" data-line="${line}" data-move="${move}">
      <span class="code-ln">${line + 1}</span>
      <span class="code-ref">${ref}</span>
      <span class="code-mark ${markCls}">${mark}</span>
      <span class="code-tag ${tagCls}">${tag}${bad ? '<i class="code-dot"></i>' : ''}</span>
      <span class="code-text">${escapeHtml(text)}</span>
      <span class="code-note">${note}</span>
    </div>`
  }

  private handleClick = (e: MouseEvent): void => {
    const t = e.target as HTMLElement
    const side = t.closest<HTMLElement>('[data-side]')
    if (side) {
      const next = side.dataset.side === 'before' ? 'before' : 'after'
      if (next !== this.side) {
        this.side = next
        this.sideBox.querySelectorAll<HTMLElement>('[data-side]').forEach((btn) => {
          btn.classList.toggle('on', btn.dataset.side === next)
        })
        this.render()
      }
      return
    }
    const act = t.closest<HTMLElement>('[data-act]')
    if (act) {
      const kind = act.dataset.act as CodeAction | undefined
      if (kind !== undefined) this.onAction?.(kind)
      return
    }
    if (t.closest('.code-ctx')) {
      this.cycleContext()
      return
    }
    if (t.closest('.code-fold')) {
      this.folded = !this.folded
      this.root.classList.toggle('folded', this.folded)
      return
    }
    const row = t.closest<HTMLElement>('.code-row')
    if (row === null) return
    const line = Number(row.dataset.line)
    const diff = this.data?.diff ?? null
    if (this.side === 'before') {
      const mapped = diff === null ? -1 : (diff.afterOf[line] as number)
      if (mapped >= 0) this.onSelectLine?.(mapped)
      return
    }
    this.onSelectLine?.(line)
  }

  private handleMove = (e: PointerEvent): void => {
    const row = (e.target as HTMLElement).closest<HTMLElement>('.code-row')
    const line = row === null ? null : Number(row.dataset.line)
    if (line === this.hoverLine) return
    this.hoverLine = line
    this.onHoverLine?.(line)
  }

  private handleLeave = (): void => {
    if (this.hoverLine === null) return
    this.hoverLine = null
    this.onHoverLine?.(null)
  }

  private cycleContext(): void {
    const next = this.context === 1 ? 3 : this.context === 3 ? 7 : 1
    this.context = next
    this.render()
  }

}
