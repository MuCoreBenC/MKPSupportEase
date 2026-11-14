/*
 * 可拖的次序列表：拖动、输位次、↑↓、Alt+↑↓ 都改同一份 `order`。
 *
 * 落地三段（与 ReorderList.module.css 的 data-landing / data-snap 配套）：
 *   拖动与落地共用一份 visual，让位、预览位次、落点占位在 160ms 里都不变；
 *   动画结束那一刻清 transform + 改 order 是同一份坐标；
 *   重排那一帧整列关掉 transform 过渡，下一帧恢复。
 *
 * 这一层不写盘、不认后端命令：改次序只调 onMove / onSetPosition，
 * 由调用方决定是攒草稿（SortDialog）还是立刻落盘。
 */

import { useEffect, useMemo, useRef, useState } from 'react'
import type { CSSProperties, PointerEvent as ReactPointerEvent } from 'react'

import type { SortRow } from './types'
import s from './ReorderList.module.css'

export interface ReorderListProps {
  /** 所有行（任意次序），按 key 查 */
  rows: SortRow[]
  /** 当前次序（key） */
  order: string[]
  /** 当前项：给它一行底色；拖动 / 落地时收掉 */
  currentKey?: string
  /** 把第 from 行挪到第 to 行（都是落地后的下标） */
  onMove: (from: number, to: number) => void
  /** 给了才画位次输入框；不给就只显示一个静态数字 */
  onSetPosition?: (key: string, n: number) => void
  /** 行高（px）；CSS 读同一个变量，改这里两边一起变 */
  rowHeight?: number
  /** 列表的无障碍名字 */
  label?: string
}

const DEFAULT_ROW = 30
/** 比 CSS 里 160ms 的过渡多一帧：等它滑到位再重排 */
const LANDING_MS = 170
/** 起拖阈值：手抖 5px 不算「我要拖」 */
const DRAG_SLOP = 5

interface Visual {
  from: number
  over: number
}

interface DragLive {
  from: number
  over: number
  startY: number
  /** 起拖时列表已滚了多少：滚过之后位移要把它算进去，不然落点会偏 */
  startScroll: number
  /** 真的动过手才算拖（按下不动 = 一次点击） */
  started: boolean
  el: HTMLElement
  /** 起拖时的次序：落地前被外部改过（撤销等）就不该再按旧下标重排 */
  orderAtStart: string[]
}

const sameOrder = (a: string[], b: string[]): boolean =>
  a.length === b.length && a.every((k, i) => k === b[i])

export default function ReorderList({
  rows,
  order,
  currentKey,
  onMove,
  onSetPosition,
  rowHeight = DEFAULT_ROW,
  label = '排序列表',
}: ReorderListProps) {
  const rowByKey = useMemo(() => new Map(rows.map((r) => [r.key, r])), [rows])
  const listRef = useRef<HTMLDivElement>(null)
  const [drag, setDrag] = useState<Visual | null>(null)
  const [landing, setLanding] = useState<Visual | null>(null)
  const landTimer = useRef<number | null>(null)
  /** 落地收尾动作：动画正常结束时由定时器调，中途又起拖则提前调一次 */
  const finishLanding = useRef<(() => void) | null>(null)
  const orderRef = useRef(order)
  orderRef.current = order
  /*
   * 拖拽的手感全在 ref 里，不走 React state：手指每动一像素就 setState 会有拖影。
   * 只把 transform 直接写到那一行 DOM 上；落点下标变了才 setState 让别的行让位。
   */
  const dragLive = useRef<DragLive | null>(null)

  /* 卸载时把落地动画与整页禁选都收干净，别留定时器去改已经卸载的 DOM */
  useEffect(() => () => {
    if (landTimer.current !== null) window.clearTimeout(landTimer.current)
    finishLanding.current = null
    document.body.style.userSelect = ''
  }, [])

  /** 拖动 + 落地共用一份视觉状态：松手不让位弹回，也不立刻重排 */
  const visual = drag ?? landing

  /** 让位之后显示的是松手后的位次，不是各自原来的下标 */
  const previewAt = (i: number): number => {
    if (visual === null) return i + 1
    const { from, over } = visual
    if (i === from) return over + 1
    if (from < over && i > from && i <= over) return i
    if (from > over && i < from && i >= over) return i + 2
    return i + 1
  }

  /** 其余行让位：被拖的行与落点之间的每一行，整体挪一格 */
  const shiftOf = (i: number): number => {
    if (visual === null || i === visual.from) return 0
    const { from, over } = visual
    if (from < over && i > from && i <= over) return -rowHeight
    if (from > over && i < from && i >= over) return rowHeight
    return 0
  }

  /** 落点 = 起点 + 相对起点挪了几行（四舍五入）；没挪过半行就停在原位 */
  const overFrom = (live: DragLive, clientY: number): number => {
    const scroll = listRef.current?.scrollTop ?? 0
    const dy = clientY - live.startY + (scroll - live.startScroll)
    return Math.min(order.length - 1, Math.max(0, live.from + Math.round(dy / rowHeight)))
  }

  /** 拖到边上自动滚 —— 一屏放不下时，不滚就等于拖不过去 */
  const autoScroll = (y: number) => {
    const el = listRef.current
    if (el === null) return
    const r = el.getBoundingClientRect()
    if (y < r.top + 24) el.scrollTop -= 10
    else if (y > r.bottom - 24) el.scrollTop += 10
  }

  const startDrag = (e: ReactPointerEvent<HTMLDivElement>, i: number) => {
    /*
     * 数字框 / ↑↓ 自己都有各自的活：按 closest 认整颗控件，图标里的 svg/path 一起算。
     * 只比 tagName 会漏掉点在图标上的那一半 —— 整行一旦接管指针，setPointerCapture
     * 会把 click 的目标挪到行上，按钮就永远收不到。
     */
    if ((e.target as Element).closest('input, button') !== null) return
    /* 别把文字拖选出来：preventDefault + 清选区 + 拖动期间整页禁选 */
    e.preventDefault()
    window.getSelection()?.removeAllRanges()
    document.body.style.userSelect = 'none'
    /* 上一次落地没走完又起拖：先强制收尾，不留半段动画和旧 transform */
    if (finishLanding.current !== null) {
      finishLanding.current()
      finishLanding.current = null
    }
    const el = e.currentTarget
    el.setPointerCapture(e.pointerId)
    dragLive.current = {
      from: i,
      over: i,
      startY: e.clientY,
      startScroll: listRef.current?.scrollTop ?? 0,
      started: false,
      el,
      orderAtStart: orderRef.current,
    }
  }

  const moveDrag = (e: ReactPointerEvent<HTMLDivElement>) => {
    const live = dragLive.current
    if (live === null) return
    const dy = e.clientY - live.startY + ((listRef.current?.scrollTop ?? 0) - live.startScroll)
    if (!live.started) {
      if (Math.abs(dy) < DRAG_SLOP) return
      live.started = true
      setDrag({ from: live.from, over: live.from })
    }
    /* 只动 Y：缩放会把行撑宽，看着像左右也在拖 */
    live.el.style.transform = `translate3d(0, ${dy}px, 0)`
    autoScroll(e.clientY)
    const over = overFrom(live, e.clientY)
    if (over !== live.over) {
      live.over = over
      setDrag({ from: live.from, over })
    }
  }

  const endDrag = () => {
    const live = dragLive.current
    if (live === null) return
    dragLive.current = null
    document.body.style.userSelect = ''
    /* 没真拖过：一次点击，什么都不做 */
    if (!live.started) return

    /* 浮起行从手指位置滑到落点占位；滑到位之前一行 DOM 都不重排 */
    const dy = (live.over - live.from) * rowHeight
    setDrag(null)
    setLanding({ from: live.from, over: live.over })
    const el = live.el
    /* 下一帧再改 transform：同一帧里改，浏览器会把起始值也吃掉，过渡不触发 */
    window.requestAnimationFrame(() => {
      el.dataset.landing = 'true'
      el.style.transform = `translate3d(0, ${dy}px, 0)`
    })

    const finish = () => {
      if (landTimer.current !== null) {
        window.clearTimeout(landTimer.current)
        landTimer.current = null
      }
      finishLanding.current = null
      /* 这一帧整列关掉 transform 过渡，否则「清旧位移」与「布局换位」会被拆成两段 */
      const list = listRef.current
      list?.setAttribute('data-snap', 'true')
      delete el.dataset.landing
      el.style.transform = ''
      setLanding(null)
      if (live.over !== live.from && sameOrder(orderRef.current, live.orderAtStart)) {
        onMove(live.from, live.over)
      }
      window.requestAnimationFrame(() => {
        window.requestAnimationFrame(() => {
          list?.removeAttribute('data-snap')
        })
      })
    }
    finishLanding.current = finish
    landTimer.current = window.setTimeout(finish, LANDING_MS)
  }

  return (
    <div
      className={s.list}
      ref={listRef}
      role="list"
      aria-label={label}
      style={{ '--sort-row-h': `${rowHeight}px` } as CSSProperties}
    >
      {/*
       * 落点占位：其余行让位之后空出来的那一格，用一整块背景色填上。
       * 让位之后行已经不在原位，画在线上的横线会落在错的缝里。
       */}
      {visual !== null && visual.over !== visual.from && (
        <div data-slot className={s.slot} style={{ top: visual.over * rowHeight }} aria-hidden />
      )}
      {order.map((key, i) => {
        const row = rowByKey.get(key) ?? { key, label: key }
        const floating = (drag !== null && drag.from === i) || (landing !== null && landing.from === i)
        return (
          <div
            key={key}
            data-row
            role="listitem"
            className={s.row}
            data-drag={floating ? 'true' : undefined}
            /* 拖动 + 落地时只留「浮起的那条」和「落点占位」两块绿 */
            data-self={key === currentKey && visual === null ? 'true' : undefined}
            /* 整行可拖（手柄只是给眼睛看的落点），数字框与箭头除外 */
            onPointerDown={(e) => startDrag(e, i)}
            onPointerMove={moveDrag}
            onPointerUp={endDrag}
            onPointerCancel={endDrag}
            /* 浮起行由 JS 直接写 transform，别让 React 的 style 覆盖它 */
            style={floating ? undefined : { transform: shiftOf(i) === 0 ? undefined : `translateY(${shiftOf(i)}px)` }}
            onKeyDown={(e) => {
              if (!e.altKey) return
              if (e.key === 'ArrowUp' && i > 0) {
                e.preventDefault()
                onMove(i, i - 1)
              } else if (e.key === 'ArrowDown' && i < order.length - 1) {
                e.preventDefault()
                onMove(i, i + 1)
              }
            }}
          >
            {visual !== null ? (
              <span className={s.posPreview} aria-hidden>{previewAt(i)}</span>
            ) : onSetPosition !== undefined ? (
              <input
                key={`${key}-${i}`}
                className={s.pos}
                type="text"
                inputMode="numeric"
                aria-label={`${row.label} 的位次`}
                defaultValue={String(i + 1)}
                onPointerDown={(e) => e.stopPropagation()}
                onBlur={(e) => {
                  const n = Number(e.target.value)
                  if (Number.isFinite(n)) onSetPosition(key, n)
                }}
                onKeyDown={(e) => {
                  if (e.key !== 'Enter') return
                  e.preventDefault()
                  const n = Number((e.target as HTMLInputElement).value)
                  if (Number.isFinite(n)) onSetPosition(key, n)
                  ;(e.target as HTMLInputElement).blur()
                }}
              />
            ) : (
              <span className={s.posPreview} aria-hidden>{i + 1}</span>
            )}
            <span className={s.label}>{row.label}</span>
            {row.by !== undefined && <span className={s.by}>{row.by}</span>}
            <span className={s.spacer} />
            <button
              type="button"
              className={s.arrow}
              aria-label={`${row.label} 上移一位`}
              disabled={i === 0}
              onPointerDown={(e) => e.stopPropagation()}
              onClick={() => onMove(i, i - 1)}
            >
              <svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
                <path d="m4 10 4-4 4 4" />
              </svg>
            </button>
            <button
              type="button"
              className={s.arrow}
              aria-label={`${row.label} 下移一位`}
              disabled={i === order.length - 1}
              onPointerDown={(e) => e.stopPropagation()}
              onClick={() => onMove(i, i + 1)}
            >
              <svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
                <path d="m4 6 4 4 4-4" />
              </svg>
            </button>
            <span className={s.handle} aria-hidden>
              <svg viewBox="0 0 16 16" width="12" height="12" fill="currentColor" aria-hidden>
                <circle cx="6" cy="4" r="1.1" />
                <circle cx="10" cy="4" r="1.1" />
                <circle cx="6" cy="8" r="1.1" />
                <circle cx="10" cy="8" r="1.1" />
                <circle cx="6" cy="12" r="1.1" />
                <circle cx="10" cy="12" r="1.1" />
              </svg>
            </span>
          </div>
        )
      })}
    </div>
  )
}
