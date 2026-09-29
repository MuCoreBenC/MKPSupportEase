/*
 * 贴着锚点的浮层（#33）—— 下拉列表与行菜单都从这里出。
 *
 * # 坐标相对层，不相对视口
 *
 * `left = anchor.left - layer.left`、`top = anchor.bottom - layer.top + GAP`。
 * 这样外壳的 transform / scale 是什么都无所谓（见 FieldLayer.tsx 的文件头）。
 * 贴边翻转的判据也换成**层自己的尺寸**，不再用 `window.innerWidth / innerHeight` ——
 * 包含块换了之后那个判据本来就已经失真了。
 *
 * # 为什么滚动是关掉而不是跟着走
 *
 * 锚点长在会滚动的表格里。跟随要在每次滚动重算位置，而这张表一次只可能开一个浮层，
 * 关掉更省也更诚实（浮层停在错的位置才是撒谎）。
 *
 * # 量位置要先画出来
 *
 * 翻转要知道浮层自己多高，所以第一帧先画在 (0,0) 且 `visibility: hidden`，量完再定位显形。
 */

import { useEffect, useLayoutEffect, useRef, useState } from 'react'
import type { ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { useFieldLayer } from './fieldLayerContext'
import s from './FieldPopover.module.css'

interface Props {
  /** 触发器。浮层贴着它的下沿（放不下就贴上沿） */
  anchor: HTMLElement | null
  /** 左对齐触发器左沿，还是右对齐右沿 */
  align?: 'start' | 'end'
  /** 最小宽度跟着触发器 —— 下拉要（列表不能比触发器窄），菜单不要 */
  matchWidth?: boolean
  onClose: () => void
  children: ReactNode
}

/** 浮层与触发器之间的缝，和贴边判断的安全边距 */
const GAP = 2
const EDGE = 6

interface Box {
  left: number
  top: number
  minWidth?: number
}

export default function FieldPopover({
  anchor,
  align = 'start',
  matchWidth = false,
  onClose,
  children,
}: Props) {
  const layer = useFieldLayer()
  const ref = useRef<HTMLDivElement>(null)
  const [box, setBox] = useState<Box | null>(null)

  /* onClose 每帧都是新函数，但登记/卸载不该跟着重跑，所以走 ref */
  const closeRef = useRef(onClose)
  closeRef.current = onClose

  /* 同层只留一个浮层：登记自己，顺手关掉上一个 */
  useEffect(() => layer.open(() => closeRef.current()), [layer])

  useLayoutEffect(() => {
    const el = ref.current
    const host = layer.el
    if (el === null || host === null || anchor === null) return
    const a = anchor.getBoundingClientRect()
    const l = host.getBoundingClientRect()
    const w = el.offsetWidth
    const h = el.offsetHeight

    /*
     * 层的坐标系可能被**缩放着**：预览器 fit 档给`.window` 挂了 transform: scale，
     * 层是它后代，于是 `getBoundingClientRect` 给的是**缩放后的视口坐标**，
     * 而写进 style 的 left/top 是**层自己的 CSS 像素** —— 差一个 zoom。
     * 不换算的话浮层朝层的左上角偏，偏多少 = 「到层的距离 ×（1 − zoom）」：
     * 0.88 倍时 700px 外那一格就偏 72px（实测），窗口越小偏得越狠。
     *
     * zoom 从层自己身上量（offsetWidth 是布局宽、rect 是渲染宽），
     * 与外壳怎么缩放、在哪一层都无关。zoom = 1 时下面全是恒等 ——
     * 不缩放的场合一个像素不动。
     */
    const zoom = host.offsetWidth > 0 ? l.width / host.offsetWidth : 1
    /* 层的可用空间用**布局尺寸**（层坐标系），别用 rect（视口坐标） */
    const availW = host.clientWidth
    const availH = host.clientHeight
    const aTop = (a.top - l.top) / zoom
    const aBottom = (a.bottom - l.top) / zoom
    const aLeft = (a.left - l.left) / zoom
    const aRight = (a.right - l.left) / zoom

    let top = aBottom + GAP
    /* 下面放不下、上面放得下就往上翻；上下都放不下就贴上沿（宁可挡住锚点也别切掉内容） */
    const up = top + h > availH - EDGE && aTop - h - GAP > EDGE
    if (up) top = aTop - h - GAP
    top = Math.max(EDGE, Math.min(top, Math.max(EDGE, availH - EDGE - h)))

    let left = align === 'end' ? aRight - w : aLeft
    left = Math.min(left, availW - EDGE - w)
    left = Math.max(EDGE, left)

    setBox({ left, top, minWidth: matchWidth ? a.width / zoom : undefined })
  }, [align, anchor, layer.el, matchWidth, children])

  useEffect(() => {
    const onDown = (e: MouseEvent) => {
      const t = e.target as Node
      if (ref.current?.contains(t) === true) return
      if (anchor?.contains(t) === true) return
      closeRef.current()
    }
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') closeRef.current()
    }
    const onScroll = () => closeRef.current()
    window.addEventListener('mousedown', onDown)
    window.addEventListener('keydown', onKey)
    /* capture：滚动事件不冒泡，要在捕获阶段才收得到表格容器自己的滚动 */
    window.addEventListener('scroll', onScroll, true)
    window.addEventListener('resize', onScroll)
    return () => {
      window.removeEventListener('mousedown', onDown)
      window.removeEventListener('keydown', onKey)
      window.removeEventListener('scroll', onScroll, true)
      window.removeEventListener('resize', onScroll)
    }
  }, [anchor])

  if (layer.el === null) return null

  return createPortal(
    <div
      ref={ref}
      className={s.pop}
      /* 稳定标记：宿主判断「这一下点在浮层里」用它，不要去猜 hash 类名 */
      data-popover="1"
      data-ready={box !== null}
      style={box === null ? undefined : { left: box.left, top: box.top, minWidth: box.minWidth }}
    >
      {children}
    </div>,
    layer.el,
  )
}
