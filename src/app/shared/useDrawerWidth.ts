/*
 * 抽屉宽度的拖拽逻辑（全仓的抽屉共用这一套手感）。
 *
 * 全仓的抽屉拖宽都是同一套手感（作者：「我们拖动宽度，这个都是复用的，全局都是
 * 用同一套逻辑，都是可以有吸附回复到默认位置」）：
 *
 * # 外观上什么都不加
 *
 * 热区是抽屉贴边的一条 6px **透明**带，唯一的视觉变化是 `cursor: col-resize` ——
 * 和文件管理器一样「看不出来，悬停就能拖」。不画竖线、不画圆点。
 * 唯一例外是键盘聚焦（`:focus-visible`）时显一条细线，否则键盘用户根本不知道
 * 这里有个能调的东西。
 *
 * # 吸附要能挣脱（macOS 访达那种）
 *
 * 光做「靠近就吸」会把人黏死在默认宽度上，想拖到 300 都拖不动。所以是三态：
 *
 *     吸附     |w - 默认| ≤ 8     → 宽度硬取默认值（贴上去）
 *     挣脱     贴住后再移出 14px → 解除，并且**这一次拖动内不再吸附**
 *     重新武装 走到默认 ±40 之外 → 吸附重新生效
 *
 * `armed` 就是「这一次拖动里吸附还生不生效」，`snapped` 是「现在是否贴着」。
 * 两个状态都住在 ref 里：它们每次 pointermove 都在变，进 state 等于每帧重渲染一次。
 *
 * # 方向
 *
 * `side: 'left'` = 抽屉贴左缘（BBS 预设抽屉），往右拖变宽；
 * `side: 'right'` = 抽屉贴右缘（修改历史），往左拖变宽，键盘也跟着反过来
 * （ArrowLeft = 变宽 —— 拖的手势隐喻）。
 *
 * # 为什么不用 CSS resize / 第三方
 *
 * `resize: horizontal` 只能给 textarea 那类，且会画出一个角标（外观上就露了）。
 * 这一页要的效果一共就六十行，不值得引依赖。
 */

import { useCallback, useEffect, useRef, useState } from 'react'

const SNAP_RADIUS = 8
const BREAK_DISTANCE = 14
const REARM_DISTANCE = 40
const KEY_STEP = 16

export interface DrawerWidth {
  width: number
  dragging: boolean
  /** 摊到热区那个元素上 */
  gripProps: {
    role: 'separator'
    tabIndex: 0
    'aria-orientation': 'vertical'
    'aria-label': string
    'aria-valuenow': number
    'aria-valuemin': number
    'aria-valuemax': number
    onPointerDown: (ev: React.PointerEvent<HTMLElement>) => void
    onPointerMove: (ev: React.PointerEvent<HTMLElement>) => void
    onPointerUp: (ev: React.PointerEvent<HTMLElement>) => void
    onDoubleClick: () => void
    onKeyDown: (ev: React.KeyboardEvent<HTMLElement>) => void
  }
}

interface Options {
  /** 落 localStorage 的键。**带页面命名空间**，别的抽屉读不到；值一律 JSON 编码 */
  storageKey: string
  /** 抽屉贴哪一边：left = 往右拖变宽；right = 往左拖变宽 */
  side: 'left' | 'right'
  /** 默认宽度，也是吸附点 */
  snap: number
  min: number
  max: number
  /** 热区的无障碍标签（每个抽屉自己说自己的名字） */
  label: string
}

const clamp = (v: number, min: number, max: number) => Math.min(Math.max(v, min), Math.max(max, min))

function readWidth(key: string, snap: number, min: number, max: number): number {
  try {
    const raw = window.localStorage.getItem(key)
    if (raw === null) return snap
    const parsed: unknown = JSON.parse(raw)
    /* 坏值 / 越界值一律 clamp 或退回默认 —— 一个宽度偏好不值得让页面起不来 */
    return typeof parsed === 'number' && Number.isFinite(parsed) ? clamp(parsed, min, max) : snap
  } catch {
    return snap
  }
}

function writeWidth(key: string, w: number): void {
  try {
    window.localStorage.setItem(key, JSON.stringify(w))
  } catch {
    /* 无痕模式 / 配额满：只在这次会话里有效 */
  }
}

export function useDrawerWidth(opts: Options): DrawerWidth {
  const { storageKey, side, snap, min, max, label } = opts
  const [width, setWidth] = useState<number>(() => readWidth(storageKey, snap, min, max))
  const [dragging, setDragging] = useState(false)

  /* 一次拖动里的全部瞬时量。进 state 会让每次 pointermove 都重渲染 */
  const drag = useRef({ startX: 0, startW: snap, snapped: false, armed: true })

  const commit = useCallback(
    (next: number) => {
      setWidth(next)
      writeWidth(storageKey, next)
    },
    [storageKey],
  )

  /* 拖动期间锁掉选中与光标：不然拖过参数区会把标签文字刷成一片蓝 */
  useEffect(() => {
    if (!dragging) return
    const { body } = document
    const prevSelect = body.style.userSelect
    const prevCursor = body.style.cursor
    body.style.userSelect = 'none'
    body.style.cursor = 'col-resize'
    return () => {
      body.style.userSelect = prevSelect
      body.style.cursor = prevCursor
    }
  }, [dragging])

  const onPointerDown = useCallback(
    (ev: React.PointerEvent<HTMLElement>) => {
      /* 只接主键；右键要留给别处的上下文菜单 */
      if (ev.button !== 0) return
      ev.preventDefault()
      ev.currentTarget.setPointerCapture(ev.pointerId)
      drag.current = {
        startX: ev.clientX,
        startW: width,
        snapped: Math.abs(width - snap) < 1,
        /* 起手就贴在吸附点上时先解除武装，否则第一下根本拖不走 */
        armed: Math.abs(width - snap) > SNAP_RADIUS,
      }
      setDragging(true)
    },
    [width, snap],
  )

  const onPointerMove = useCallback(
    (ev: React.PointerEvent<HTMLElement>) => {
      if (!dragging) return
      const d = drag.current
      const dir = side === 'left' ? 1 : -1
      const raw = clamp(d.startW + dir * (ev.clientX - d.startX), min, max)
      const off = Math.abs(raw - snap)

      let next = raw
      if (d.armed && off <= SNAP_RADIUS) {
        next = snap
        d.snapped = true
      } else if (d.snapped && off > BREAK_DISTANCE) {
        /* 挣脱：这一次拖动里不再对默认值吸附，不然会在吸附点来回跳 */
        d.armed = false
        d.snapped = false
      } else if (off > REARM_DISTANCE) {
        d.armed = true
      }
      setWidth(next)
    },
    [dragging, side, min, max, snap],
  )

  const onPointerUp = useCallback(
    (ev: React.PointerEvent<HTMLElement>) => {
      if (!dragging) return
      if (ev.currentTarget.hasPointerCapture(ev.pointerId)) {
        ev.currentTarget.releasePointerCapture(ev.pointerId)
      }
      setDragging(false)
      /* 落盘只在松手时做一次：拖动过程中每帧写 localStorage 是纯浪费 */
      writeWidth(storageKey, width)
    },
    [dragging, width, storageKey],
  )

  const onDoubleClick = useCallback(() => commit(snap), [commit, snap])

  const onKeyDown = useCallback(
    (ev: React.KeyboardEvent<HTMLElement>) => {
      /* 键盘跟着拖的手势走：贴右缘的抽屉，往左 = 变宽 */
      const widen = side === 'left' ? 'ArrowRight' : 'ArrowLeft'
      const narrow = side === 'left' ? 'ArrowLeft' : 'ArrowRight'
      if (ev.key === widen) {
        ev.preventDefault()
        commit(clamp(width + KEY_STEP, min, max))
      } else if (ev.key === narrow) {
        ev.preventDefault()
        commit(clamp(width - KEY_STEP, min, max))
      } else if (ev.key === 'Home') {
        ev.preventDefault()
        commit(snap)
      }
    },
    [commit, width, side, min, max, snap],
  )

  return {
    width,
    dragging,
    gripProps: {
      role: 'separator',
      tabIndex: 0,
      'aria-orientation': 'vertical',
      'aria-label': `${label}宽度 ${width} 像素，方向键调整，Home 复位`,
      'aria-valuenow': width,
      'aria-valuemin': min,
      'aria-valuemax': max,
      onPointerDown,
      onPointerMove,
      onPointerUp,
      onDoubleClick,
      onKeyDown,
    },
  }
}
