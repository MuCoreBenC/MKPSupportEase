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
import { useFieldLayer } from '../../components/field/fieldLayerContext'
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

    let top = a.bottom - l.top + GAP
    /* 下面放不下、上面放得下就往上翻；上下都放不下就贴上沿（宁可挡住锚点也别切掉内容） */
    const up = top + h > l.height - EDGE && a.top - l.top - h - GAP > EDGE
    if (up) top = a.top - l.top - h - GAP
    top = Math.max(EDGE, Math.min(top, Math.max(EDGE, l.height - EDGE - h)))

    let left = align === 'end' ? a.right - l.left - w : a.left - l.left
    left = Math.min(left, l.width - EDGE - w)
    left = Math.max(EDGE, left)

    setBox({ left, top, minWidth: matchWidth ? a.width : undefined })
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
    /*
     * 滚动为什么会关掉浮层：锚点长在会滚动的容器里，容器一滚浮层就停在错的位置，
     * 所以要么跟着重算位置，要么关掉 —— 这一稿选关掉（见文件头）。
     *
     * **浮层自己内部滚动不算。** `scroll` 不冒泡，所以这里挂在捕获阶段，
     * 而捕获是**从 window 往下走**的：下拉列表自己滚动（`.list` 是 `overflow-y: auto`）
     * 时 window 反而**最先**收到这个事件，浮层当场被关 —— 表现为「列表滚不动」。
     *
     * 所以判据是**事件源在不在浮层里**：在里面就放过，在外面（真正的容器滚动）才关。
     */
    const onScroll = (e: Event) => {
      if (ref.current?.contains(e.target as Node)) return
      closeRef.current()
    }
    const onResize = () => closeRef.current()
    window.addEventListener('mousedown', onDown)
    window.addEventListener('keydown', onKey)
    /* capture：滚动事件不冒泡，要在捕获阶段才收得到表格容器自己的滚动 */
    window.addEventListener('scroll', onScroll, true)
    window.addEventListener('resize', onResize)
    return () => {
      window.removeEventListener('mousedown', onDown)
      window.removeEventListener('keydown', onKey)
      window.removeEventListener('scroll', onScroll, true)
      window.removeEventListener('resize', onResize)
    }
  }, [anchor])

  if (layer.el === null) return null

  return createPortal(
    <div
      ref={ref}
      className={s.pop}
      data-ready={box !== null}
      style={box === null ? undefined : { left: box.left, top: box.top, minWidth: box.minWidth }}
    >
      {children}
    </div>,
    layer.el,
  )
}
