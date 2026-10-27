/*
 * 点一下把图放大看（2026-10-16，作者：「资产库的话，大部分图片的话，我希望都可以
 * 点击把它放大之类的」）。
 *
 * # 为什么单独一个小件
 *
 * 资产库那一格 200×120 的缩略图看不出细节（引导图 / 机型图 / 图标都是**当图用的**，
 * 改身份 / 删除之前必须看得清）。放大**不该新开一个窗** —— 就在当前页浮一层，
 * 与图片引导那个"点开浮起来看大的"同一个意思（客户端 `GuideImage` 的 `ZoomView`）。
 *
 * # 缩放怎么做的（2026-10-16 第二版：作者说第一版"触摸板不好用、滚轮迟钝还会闪"）
 *
 * 第一版的三个病，逐条对着治：
 *
 * 1. **闪** —— CSS 里挂了 `transition: transform`，而滚轮每来一条就改一次 transform
 *    ⇒ 每条都从头播一遍 120ms 动画，看着就是抖 / 闪。**滚轮期间不准有 transition**：
 *    改由 rAF 每帧写一次内联 transform（CSS 的 transition 撤掉）。
 * 2. **迟钝** —— 步长太小（`0.02 * 6 = 0.12/条`）且每条都 `setState` 重渲染一次 React。
 *    ⇒ 步长按"跟手"调大，且**事件只记账、rAF 才真正 setState**（一帧最多一次重渲染）。
 * 3. **触摸板捏合不好用** —— 捏合在浏览器里到的是 `wheel + ctrlKey`，`deltaY` 往往很小、
 *    甚至只有零点几。⇒ `ctrlKey` 那一档用**指数步长**（`k *= exp(-deltaY * 0.01)`），
 *    小 delta 也能连续放大，手感接近原生捏合。
 *
 * ★ 以**指针为不动点**：缩放时把图平移，让指针底下那一点不动（看图找细节的手感）。
 * ★ 每次打开都复位成默认大小（与客户端"下次点开又是默认大小"同一条）。
 */
import { useCallback, useEffect, useRef, useState } from 'react'

import s from './ImageZoom.module.css'

interface Props {
  /** 打不打开 */
  open: boolean
  /** 图地址（`/assets/<path>`，调用方已 `encodeURI`） */
  src: string
  /** 无障碍那句（一般给资产显示名） */
  alt: string
  /** 关闭 */
  onClose: () => void
}

const MIN = 0.5
const MAX = 8

export default function ImageZoom({ open, src, alt, onClose }: Props) {
  const [k, setK] = useState(1)
  /* 平移（以指针为不动点缩放时产生）；只写内联 transform，不进 React 状态之外的地方 */
  const [off, setOff] = useState({ x: 0, y: 0 })
  const boxRef = useRef<HTMLDivElement>(null)
  const imgRef = useRef<HTMLImageElement>(null)
  /** 未落地的目标值（事件只改它；rAF 每帧把它 apply 一次） */
  const pending = useRef<{ k: number; x: number; y: number } | null>(null)
  const raf = useRef(0)

  const apply = useCallback(() => {
    raf.current = 0
    const p = pending.current
    if (p === null) return
    const img = imgRef.current
    if (img !== null) {
      /* 直接写内联 transform —— 逐帧、走合成器，中间不经过 React */
      img.style.transform = `translate(${p.x}px, ${p.y}px) scale(${p.k})`
    }
    /* React 那一份只在"这一帧"同步一次（给百分比读数与复位用） */
    setK(p.k)
    setOff({ x: p.x, y: p.y })
  }, [])

  const schedule = useCallback(() => {
    if (raf.current === 0) raf.current = requestAnimationFrame(apply)
  }, [apply])

  useEffect(() => () => cancelAnimationFrame(raf.current), [])

  /* 每次打开复位（含内联 transform —— 上一轮的直接写要抹掉） */
  useEffect(() => {
    if (!open) return
    pending.current = { k: 1, x: 0, y: 0 }
    setK(1)
    setOff({ x: 0, y: 0 })
    const img = imgRef.current
    if (img !== null) img.style.transform = 'translate(0px, 0px) scale(1)'
  }, [open])

  /* 滚轮 / 触控板捏合 —— 挂在**整层**上（不是只挂图那一块：指针在遮罩上滚动也该缩放） */
  useEffect(() => {
    if (!open) return
    const box = boxRef.current
    if (box === null) return
    const onWheel = (e: WheelEvent) => {
      e.preventDefault()
      const cur = pending.current ?? { k, x: off.x, y: off.y }
      /*
       * 指数步长：`ctrlKey`（触控板捏合）一档更"跟手"，普通滚轮一档更细。
       * 用 exp 而不是 (1 + 常数) —— 小 delta 也连续、大 delta 不突变。
       * ★ 步长比第一版大得多：作者说"滚轮迟钝"，第一版 100 一次几乎不动。
       */
      const gain = e.ctrlKey ? 0.012 : 0.0028
      const next = Math.min(MAX, Math.max(MIN, cur.k * Math.exp(-e.deltaY * gain)))
      /*
       * 以**指针为不动点**：指针在图上的相对位置在缩放前后不变。
       * 设缩放前偏移 o、指针相对容器中心 p、缩放比 r = next/cur.k：
       *   不动点要求 o' = p - r * (p - o)
       */
      const rect = box.getBoundingClientRect()
      const px = e.clientX - (rect.left + rect.width / 2)
      const py = e.clientY - (rect.top + rect.height / 2)
      const r = next / cur.k
      pending.current = {
        k: next,
        x: px - r * (px - cur.x),
        y: py - r * (py - cur.y),
      }
      schedule()
    }
    box.addEventListener('wheel', onWheel, { passive: false })
    return () => box.removeEventListener('wheel', onWheel)
  }, [open, k, off.x, off.y, schedule])

  /* Esc 关 */
  useEffect(() => {
    if (!open) return
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [open, onClose])

  if (!open) return null

  return (
    <div
      className={s.scrim}
      role="dialog"
      aria-label={alt}
      ref={boxRef}
      /* 点空白关；点图本身不关（看图时手一抖别把整层关掉） */
      onClick={onClose}
    >
      <div className={s.wrap} onDoubleClick={() => {
        pending.current = { k: 1, x: 0, y: 0 }
        schedule()
      }}>
        <img
          className={s.img}
          ref={imgRef}
          src={src}
          alt={alt}
          draggable={false}
          style={{ transform: `translate(${off.x}px, ${off.y}px) scale(${k})` }}
          onClick={(e) => e.stopPropagation()}
        />
      </div>
      <span className={s.hint}>滚轮 / 双指缩放 · 双击复位 · {Math.round(k * 100)}%</span>
    </div>
  )
}
