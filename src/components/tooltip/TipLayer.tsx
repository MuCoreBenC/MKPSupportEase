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
 *   · 位置每次现算：下方不足 ~96px 往上弹，右边不足 340px 往左长；
 *   · 指针按下、滚轮滚动、离开元素 → 立刻收。
 *
 * ★ 气泡是 `position: fixed`：它挂在 App 根，祖先没有 `container-type` / `transform`，
 *   所以 fixed 相对视口，不会被 `.main` 这类滚动容器裁掉。
 */

import { useEffect, useState } from 'react'

import s from './TipLayer.module.css'

/** 悬停多久才弹（毫秒）。太短会"扫过一排按钮时一路乱闪" */
const DELAY = 320

interface Shown {
  text: string
  x: number
  y: number
  up: boolean
  right: boolean
}

export default function TipLayer() {
  const [shown, setShown] = useState<Shown | null>(null)

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

    const place = (el: HTMLElement) => {
      const text = el.dataset.tip ?? ''
      if (text === '') return
      const r = el.getBoundingClientRect()
      const up = r.bottom + 96 > window.innerHeight
      const right = r.left + 340 > window.innerWidth - 12
      setShown({
        text,
        /* 锚点：往上弹时取上沿，往左长时取右沿 —— 配合 CSS 的两个 transform */
        x: right ? r.right : r.left,
        y: up ? r.top - 6 : r.bottom + 6,
        up,
        right,
      })
    }

    const findTip = (target: EventTarget | null): HTMLElement | null => {
      const el = target instanceof Element ? target.closest('[data-tip]') : null
      return el instanceof HTMLElement ? el : null
    }

    const onOver = (e: Event) => {
      const el = findTip(e.target)
      if (el === null) {
        hide()
        return
      }
      if (el === host) return
      clear()
      host = el
      timer = setTimeout(() => place(el), DELAY)
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
        place(el)
      }
    }

    document.addEventListener('pointerover', onOver)
    document.addEventListener('pointerout', onOut)
    document.addEventListener('focusin', onFocusIn)
    document.addEventListener('pointerdown', hide)
    /* 捕获阶段：任何滚动容器滚了都收（气泡是 fixed，不跟内容走） */
    window.addEventListener('scroll', hide, true)

    return () => {
      clear()
      document.removeEventListener('pointerover', onOver)
      document.removeEventListener('pointerout', onOut)
      document.removeEventListener('focusin', onFocusIn)
      document.removeEventListener('pointerdown', hide)
      window.removeEventListener('scroll', hide, true)
    }
  }, [])

  if (shown === null) return null

  return (
    <div
      className={`${s.tip} ${shown.up ? s.tipUp : ''} ${shown.right ? s.tipRight : ''}`}
      style={{ left: shown.x, top: shown.y }}
      role="tooltip"
    >
      {shown.text}
    </div>
  )
}
