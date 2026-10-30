/*
 * 全局提示层（2026-10-30）—— 全站**只有一个**，挂在 App 根。
 *
 * # 为什么是"全局委托"而不是"每个按钮包一层"
 *
 * 作者：「还有一些那些原生的，它没有图标的，也可以替换成我们这种的吧。
 * 不用图标也能触发，嗯，去做吧，就比如说按钮那一些」。
 *
 * 全站 300 多处 `title=`，逐个包 `<Tip>` 会把每个按钮的 DOM 结构改一遍 ——
 * 布局（flex 子项、`margin-left: auto`、`flex: 1`）全得跟着重调，
 * 一处漏了就是"某个按钮突然跑到左边"。所以改成：
 *
 *   元素上写 `data-tip="…"`  →  这一个组件负责画气泡。
 *
 * 谁负责把 `title` 换成 `data-tip`：`ui/Controls.tsx` 的 `Btn` 自动换
 * （调用点一个字不用改）；其他手写的 `<span title>` / `<button title>` 逐个换成
 * `data-tip` 即可，**DOM 结构一个都不动**。
 *
 * # 行为
 *
 *   · 悬停 320ms 才出（扫过一排按钮不会乱弹）；键盘 focus 立刻出；
 *   · **位置跟着鼠标**（2026-10-30 作者：「有一些悬停的 tooltip 位置不对，应该在
 *     鼠标的位置才对」）—— 锚点是**光标右下方** 14 / 18px，不是元素矩形；
 *     键盘 focus 时没有光标，才退到元素下沿；
 *   · 出框一律夹回视口内（见 `useLayoutEffect` 那段）：不翻转、不换向，只平移。
 *   · 指针按下、滚轮滚动、离开元素 → 立刻收。
 *
 * ★ 气泡是 `position: fixed`：它挂在 App 根，祖先没有 `container-type` / `transform`，
 *   所以 fixed 相对视口，不会被 `.main` 这类滚动容器裁掉。
 */

import { useEffect, useLayoutEffect, useRef, useState } from 'react'

import s from './TipLayer.module.css'

/** 悬停多久才弹（毫秒）。太短会"扫过一排按钮时一路乱闪" */
const DELAY = 320

/** 光标 / 元素下沿 → 气泡左上角的偏移 */
const GAP_X = 14
const GAP_Y = 18
/** 气泡与视口边缘至少留这么多，别贴着窗口边 */
const EDGE = 8

interface Shown {
  text: string
  /** 气泡**左上角**想要的落点（视口坐标）；出框时由下面那段 effect 平移回来 */
  x: number
  y: number
}

export default function TipLayer() {
  const [shown, setShown] = useState<Shown | null>(null)
  const boxRef = useRef<HTMLDivElement>(null)

  /*
   * 无障碍兜底：`data-tip` 替换掉原生 `title` 之后，纯图标按钮少了一层读屏名字
   * （`title` 原本兼作 accessible name）。
   *
   * 所以这里统一补：**带 `data-tip`、自己没有任何可见文字、也没有 `aria-label` /
   * `aria-labelledby`** 的元素，把 `data-tip` 那句话写成 `aria-label`。
   *
   * 「有没有可见文字」用 `textContent.trim() === ''` 判 —— 纯图标按钮正好命中，
   * 有文字的按钮/行不会被覆盖（它们本来就念得出来）。
   *
   * 挂载时扫一次，之后用 MutationObserver 认领**新渲染出来**的东西
   * （页签切换、抽屉打开都会整片重挂）。React 没声明过 `aria-label`，
   * 重渲染不会把它抹掉。
   */
  useEffect(() => {
    const one = (el: HTMLElement) => {
      const text = el.dataset.tip ?? ''
      if (text === '') return
      if (el.hasAttribute('aria-label') || el.hasAttribute('aria-labelledby')) return
      if ((el.textContent ?? '').trim() !== '') return
      el.setAttribute('aria-label', text)
    }

    const decorate = (root: ParentNode) => {
      if (root instanceof HTMLElement) {
        if (root.matches('[data-tip]')) one(root)
        root.querySelectorAll<HTMLElement>('[data-tip]').forEach(one)
        return
      }
      root.querySelectorAll<HTMLElement>('[data-tip]').forEach(one)
    }

    decorate(document)

    const mo = new MutationObserver((records) => {
      for (const r of records) {
        if (r.type === 'attributes') {
          if (r.target instanceof HTMLElement) decorate(r.target)
          continue
        }
        r.addedNodes.forEach((n) => {
          if (n instanceof HTMLElement) decorate(n)
        })
      }
    })

    mo.observe(document.body, {
      childList: true,
      subtree: true,
      attributes: true,
      attributeFilter: ['data-tip'],
    })

    return () => mo.disconnect()
  }, [])

  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined
    let host: HTMLElement | null = null
    /**
     * 光标**当前**在视口的哪儿。
     *
     * 为什么记"当前"而不是"进入元素那一瞬"：320ms 的等待里手还在动，
     * 拿进入点当锚点会明显落在鼠标后面（作者就是看见气泡挂在按钮下沿）。
     * `pointermove` 一直更新它，弹出来的那一帧取最新的。
     */
    let pt: { x: number; y: number } | null = null

    const clear = () => {
      if (timer !== undefined) {
        clearTimeout(timer)
        timer = undefined
      }
    }

    const hide = () => {
      clear()
      host = null
      setShown(null)
    }

    /** 鼠标路径：光标右下方 */
    const placeAtPointer = (el: HTMLElement) => {
      const text = el.dataset.tip ?? ''
      if (text === '') return
      if (pt === null) {
        placeAtElement(el)
        return
      }
      setShown({ text, x: pt.x + GAP_X, y: pt.y + GAP_Y })
    }

    /**
     * 键盘路径（Tab 到元素上）：没有光标可用，退到**元素下沿**。
     * 只在 `pt === null`（这一趟压根没碰鼠标）时才会走到这里。
     */
    const placeAtElement = (el: HTMLElement) => {
      const text = el.dataset.tip ?? ''
      if (text === '') return
      const r = el.getBoundingClientRect()
      setShown({ text, x: r.left, y: r.bottom + 8 })
    }

    const findTip = (target: EventTarget | null): HTMLElement | null => {
      const el = target instanceof Element ? target.closest('[data-tip]') : null
      return el instanceof HTMLElement ? el : null
    }

    const onMove = (e: Event) => {
      if (e instanceof PointerEvent) pt = { x: e.clientX, y: e.clientY }
    }

    const onOver = (e: Event) => {
      if (e instanceof PointerEvent) pt = { x: e.clientX, y: e.clientY }
      const el = findTip(e.target)
      if (el === null) {
        hide()
        return
      }
      if (el === host) return
      clear()
      host = el
      timer = setTimeout(() => placeAtPointer(el), DELAY)
    }

    const onOut = (e: Event) => {
      const el = findTip(e.target)
      if (el !== null && el === host) hide()
    }

    /* 键盘：Tab 到带 data-tip 的元素上立刻出（不等 320ms） */
    const onFocusIn = (e: Event) => {
      const el = findTip(e.target)
      if (el !== null) {
        hide()
        host = el
        placeAtElement(el)
      }
    }

    document.addEventListener('pointermove', onMove, { passive: true })
    document.addEventListener('pointerover', onOver)
    document.addEventListener('pointerout', onOut)
    document.addEventListener('focusin', onFocusIn)
    document.addEventListener('pointerdown', hide)
    /* 捕获阶段：任何滚动容器滚了都收（气泡是 fixed，不跟内容走） */
    window.addEventListener('scroll', hide, true)

    return () => {
      clear()
      document.removeEventListener('pointermove', onMove)
      document.removeEventListener('pointerover', onOver)
      document.removeEventListener('pointerout', onOut)
      document.removeEventListener('focusin', onFocusIn)
      document.removeEventListener('pointerdown', hide)
      window.removeEventListener('scroll', hide, true)
    }
  }, [])

  /*
   * ============ 出框就**平移**回来（2026-10-30） ============
   *
   * 原先靠 CSS 两个 transform 翻转（下方不够往上、右边不够往左）。跟着鼠标之后
   * 那两个 transform 的语义就错了：锚点已经是光标，再 `translate(-100%)` 会整块
   * 跳到光标**左上**，反而离鼠标更远。
   *
   * 现在只做一件事：量出气泡的实际矩形，哪条边过界就平移多少，保证整块留在视口里。
   * 用真实矩形而不是估算 —— 气泡是 `max-content` + `max-width`，行数事先算不出来。
   *
   * 直接写 DOM 的 `left/top`，不走 state：`shown` 没变就不会重渲染，也就不会被 React
   * 覆盖回去；同一次显示里这段跑两遍结果一样（第二遍量到的已经在框内，`dx/dy` 都是 0）。
   */
  useLayoutEffect(() => {
    const el = boxRef.current
    if (el === null || shown === null) return
    const r = el.getBoundingClientRect()
    let dx = 0
    let dy = 0
    if (r.right > window.innerWidth - EDGE) dx = window.innerWidth - EDGE - r.right
    if (r.left + dx < EDGE) dx = EDGE - r.left
    if (r.bottom > window.innerHeight - EDGE) dy = window.innerHeight - EDGE - r.bottom
    if (r.top + dy < EDGE) dy = EDGE - r.top
    if (dx !== 0) el.style.left = `${shown.x + dx}px`
    if (dy !== 0) el.style.top = `${shown.y + dy}px`
  }, [shown])

  if (shown === null) return null

  return (
    <div
      ref={boxRef}
      className={s.tip}
      style={{ left: shown.x, top: shown.y }}
      role="tooltip"
    >
      {shown.text}
    </div>
  )
}
