/*
 * C14 · 参数台的两条竖线（拖动改宽）
 *
 * 作者的原话：「这两竖线我希望可以拖动，就改变他们的宽度；同样你还是要用带有吸附的
 * 感觉 —— 靠近默认的时候它就有吸附的感觉。还有它这个吸附呀，不要影响拖动。」
 *
 * 三条规矩，一条一条对着这句话来：
 *
 *   1. **拖得动**：线不是 1px 的 border，是一条 9px 的命中区（`.pSplit`），横跨
 *      rail 与正文之间那道 12px 的缝。cursor: col-resize；触屏也认（touch-action: none）。
 *      鼠标只是在它上面经过时，线才显形 —— 不改变 C10 定稿的那张静态图。
 *
 *   2. **吸附要能挣脱**：手感照 A 轨 BBS 抽屉那一套（`useBbsDrawerA38`，作者认可过的
 *      「macOS 访达那种」），三态：
 *
 *        吸附      |w − 默认| ≤ 8      → 宽度贴到默认值
 *        挣脱      贴住后再走 14px      → 解除，且**这次拖动里不再吸附**
 *        重新武装  离开默认 ±40 之外    → 吸附重新生效
 *
 *      「吸附不要影响拖动」就是靠后两条：贴住不是黏死，继续拖就走开了，不会出现
 *      「想拖到 300 却拽不出来」。拖动中给两层反馈：线变蓝 + 顶部宽度读数牌；
 *      贴住默认时线再粗一点、牌子上多「默认」两个字。
 *
 *   3. **拖动不进 React**：一帧一个 setState 会把 70 多行参数连同常驻的步进器/分段
 *      按钮全部重渲染，那才是这条手势最贵的部分。拖动中只做两件事 —— 往 `.pBody`
 *      写 `--p-rail-w` / `--p-aside-w`（CSS 变量，React 的 style diff 管不到手写的键），
 *      改读数牌的 textContent；松手才 `onCommit` 一次。
 *      （A38 那里是每帧 setState —— 抽屉里没几行，这里不能照抄。）
 *
 * 宽度存在页面 state（+ localStorage）：切页、刷新回来还是你拖的那个宽度 ——
 * 布局偏好丢了比参数丢了更打眼（满屏列宽跳回去）。键名带稿号（仓库约定）。
 *
 * 还有键盘与双击：`role="separator"` 可聚焦，方向键 ±16、Home / 双击复位到默认 ——
 * 照 A38 的键盘词汇，鼠标不是唯一入口。
 *
 * clamp 扣两道：栏自己的 min/max；以及不管怎么拖，正文至少留 MAIN_MIN。
 * 窗口不够宽时先保正文 —— 栏拖不动就拖不动，不会把正文挤没。
 */

import { useCallback, useEffect, useRef, useState } from 'react'

import s from '../c14.module.css'

const SNAP_RADIUS = 8
const BREAK_DISTANCE = 14
const REARM_DISTANCE = 40
const KEY_STEP = 16
/** 正文栏无论如何要留下的宽度（px） */
const MAIN_MIN = 360

interface Props {
  /**
   * 量总宽 / 写变量的宿主。参数台三条竖线是 `.pBody`（那一页的三栏容器）；
   * 外壳导航那条是 `.shell`（导航宽是整壳的事，见 AppC14 与 `.body` 的 grid）。
   */
  bodyRef: React.RefObject<HTMLElement | null>
  /** 拖的哪条：rail = 左栏右缘（往右拖变宽），aside = 右栏左缘（往左拖变宽），
      nav = 外壳一级导航的右缘（往右拖变宽） */
  side: 'rail' | 'aside' | 'nav'
  /** 这条栏宽度的 CSS 变量名，例如 `--p-rail-w` */
  varName: string
  /** 当前已提交的宽度，也就是拖动的起点 */
  width: number
  /** 对侧栏的宽度：clamp 时要一起从总宽里扣 */
  otherW: number
  defaultValue: number
  min: number
  max: number
  onCommit: (w: number) => void
  /** 无障碍标签（三条线各说各的：rail / aside / nav） */
  label: string
  /** 宿主上的类名 —— 决定这条线**画在哪**（三条线在三个不同的容器里） */
  className: string
}

export default function SplitterC14({
  bodyRef,
  side,
  varName,
  width,
  otherW,
  defaultValue,
  min,
  max,
  onCommit,
  label,
  className,
}: Props) {
  const hostRef = useRef<HTMLDivElement>(null)
  const chipRef = useRef<HTMLSpanElement>(null)
  const numRef = useRef<HTMLElement>(null)
  /** 一次拖动里的全部瞬时量。**都住在这里，不进 state**（会每帧重渲染） */
  const drag = useRef({ active: false, x0: 0, w0: 0, w: 0, snapped: false, armed: true })

  const paint = useCallback(
    (w: number) => bodyRef.current?.style.setProperty(varName, `${w}px`),
    [bodyRef, varName],
  )

  /* 读数牌写字：改现有文本节点的值，不整块重写 textContent —— 每帧一次，少一次节点替换 */
  const setNum = (w: number) => {
    const el = numRef.current
    if (!el) return
    const t = el.firstChild
    if (t) t.nodeValue = String(w)
    else el.textContent = String(w)
  }

  /*
   * 读数牌**跟着指针走**，不钉在顶上：它钉在顶上会正好压住参数台的搜索框（实测截图）。
   * 位置用 down 时量到的分隔条纵向范围来算，拖动中不再读布局 —— 每帧一次
   * getBoundingClientRect 会把刚写完的样式立刻 flush 掉，那是白花的钱。
   */
  const box = useRef({ top: 0, h: 0 })
  const placeChip = (clientY: number) => {
    const el = chipRef.current
    if (!el) return
    const y = Math.min(
      Math.max(clientY - box.current.top - 9, 4),
      Math.max(4, box.current.h - 26),
    )
    el.style.top = `${y}px`
  }

  const clampW = useCallback(
    (raw: number) => {
      const bodyW = bodyRef.current?.clientWidth
      /* 窗口不够宽（扣掉对侧与正文就没地方了）时不再压自己：拖到头就是 min/max 本身 */
      const hi = bodyW && bodyW > 0 ? Math.max(min, Math.min(max, bodyW - otherW - MAIN_MIN)) : max
      return Math.round(Math.min(Math.max(raw, min), hi))
    },
    [bodyRef, max, min, otherW],
  )

  /** 键盘 / 双击的落点：变量与 state 一起走 */
  const commit = useCallback(
    (w: number) => {
      const next = clampW(w)
      paint(next)
      onCommit(next)
    },
    [clampW, onCommit, paint],
  )

  /* 拖动期间锁掉全局的选中与光标：pointer capture 只管事件，管不住「指针下是谁」——
     拖到外壳导航上也该是左右拖的光标，不然手感像断了 */
  const [dragging, setDragging] = useState(false)
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

  const down = (e: React.PointerEvent<HTMLDivElement>) => {
    /* 只接主键；右键要留给别处的上下文菜单 */
    if (e.button !== 0) return
    /* 不给文本选择和原生拖拽起手的机会 */
    e.preventDefault()
    e.currentTarget.setPointerCapture(e.pointerId)
    drag.current = {
      active: true,
      x0: e.clientX,
      w0: width,
      w: width,
      snapped: Math.abs(width - defaultValue) < 1,
      /* 起手就贴在吸附点上时先解除武装，否则第一下根本拖不走 */
      armed: Math.abs(width - defaultValue) > SNAP_RADIUS,
    }
    const rect = hostRef.current?.getBoundingClientRect()
    if (rect) box.current = { top: rect.top, h: rect.height }
    paint(width)
    hostRef.current?.setAttribute('data-on', '1')
    setNum(width)
    placeChip(e.clientY)
    setDragging(true)
  }

  const move = (e: React.PointerEvent<HTMLDivElement>) => {
    const d = drag.current
    if (!d.active) return
    /* rail / nav 都是「右缘往右 = 变宽」，aside 反过来 */
    const dx = side === 'aside' ? d.x0 - e.clientX : e.clientX - d.x0
    const raw = clampW(d.w0 + dx)
    const off = Math.abs(raw - defaultValue)

    /* 吸附三态（见文件头）：贴住 → 挣脱 → 重新武装 */
    let w = raw
    if (d.armed && off <= SNAP_RADIUS) {
      w = defaultValue
      d.snapped = true
    } else if (d.snapped && off > BREAK_DISTANCE) {
      d.armed = false
      d.snapped = false
    } else if (off > REARM_DISTANCE) {
      d.armed = true
    }
    d.w = w
    paint(w)
    /* 读数牌与「默认」提示直接写 DOM：拖动一帧都不进 React（规矩 3） */
    setNum(w)
    placeChip(e.clientY)
    const host = hostRef.current
    if (host) {
      if (d.snapped) host.setAttribute('data-snap', '1')
      else host.removeAttribute('data-snap')
    }
  }

  const end = () => {
    const d = drag.current
    if (!d.active) return
    d.active = false
    setDragging(false)
    hostRef.current?.removeAttribute('data-on')
    hostRef.current?.removeAttribute('data-snap')
    /* 落盘只在松手时做一次：拖动中宽度已经写进变量了，这里只是把它变成 state */
    onCommit(d.w)
  }

  const keyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    /* rail / nav 往右是变宽，aside 往左才是变宽 —— 方向键跟着「看起来的方向」走 */
    const dir = side === 'aside' ? -1 : 1
    if (e.key === 'ArrowRight') {
      e.preventDefault()
      commit(width + KEY_STEP * dir)
    } else if (e.key === 'ArrowLeft') {
      e.preventDefault()
      commit(width - KEY_STEP * dir)
    } else if (e.key === 'Home') {
      e.preventDefault()
      commit(defaultValue)
    }
  }

  return (
    <div
      ref={hostRef}
      className={`${s.pSplit} ${className}`}
      role="separator"
      tabIndex={0}
      aria-orientation="vertical"
      aria-label={`${label}宽度 ${width} 像素，左右方向键调整，Home 复位`}
      aria-valuenow={width}
      aria-valuemin={min}
      aria-valuemax={max}
      onPointerDown={down}
      onPointerMove={move}
      onPointerUp={end}
      onPointerCancel={end}
      /* 指针在窗口外松开（capture 丢了）也要收尾，不然线会停在「拖动中」的样子 */
      onLostPointerCapture={end}
      onDoubleClick={() => commit(defaultValue)}
      onKeyDown={keyDown}
    >
      <span className={s.pSplitChip} ref={chipRef} aria-hidden>
        <b ref={numRef}>{width}</b>
        <i>默认</i>
      </span>
    </div>
  )
}
