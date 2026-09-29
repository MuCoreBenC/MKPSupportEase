/*
 * 开关 + 分段选择器（#33）——「构造上是同一个东西」。
 *
 * # 为什么合成一个组件
 *
 * #32 的参数页用 `SegChoiceV032`（两段：关闭/开启），工作台的开关也用它，但枚举换成了下拉，
 * 于是同一张界面上「开关」与「选择器」的手感靠自觉维持。参考项目的做法是构造上的：
 * `variant="switch"` 在组件内部被改写成 `[{off,关闭},{on,开启}]` 两段，走同一套渲染 ——
 * 想让它们不一样都做不到。这一稿照搬这条。
 *
 * # 边界：谁说布尔，谁说字符串
 *
 *   variant="switch"     对外收 `checked: boolean`、回 `boolean`
 *   variant="segmented"  对外收 `value: string`、回 `string`（枚举键，不是显示文字）
 *
 * 字符串 ⇄ 布尔的转换在 `FieldControl` 那一处用 `value.ts` 做，不在这里，也不在页面里。
 *
 * # 比参考项目多的两样
 *
 * 参考项目的 `SegmentedPill` 没有键盘交互，`disabled` 时按钮也仍然可点（靠回调首行 return 兜）。
 * 这两点我们这边本来就是对的（v032 的 SegChoice 有 radiogroup + 方向键），所以保留并补上 Home/End，
 * 禁用时给按钮真的 `disabled`。
 */

import { useCallback, useLayoutEffect, useRef, useState } from 'react'
import type { KeyboardEvent } from 'react'
import type { FieldOption } from './types'
import s from './SegmentedField.module.css'

/** 开关的两段。文案照旧程序：布尔在那边就是「关闭 / 开启」两段，不是胶囊开关 */
const SWITCH_OPTIONS: FieldOption[] = [
  { value: 'off', label: '关闭' },
  { value: 'on', label: '开启' },
]

/** 超过这个字数的段算长文案，降一号字 */
const LONG_LABEL = 3

interface Common {
  label: string
  disabled?: boolean
  /** 密表里的紧凑形态：更小的字、更窄的 padding */
  dense?: boolean
}

type Props =
  | (Common & { variant: 'switch'; checked: boolean; onChange: (checked: boolean) => void })
  | (Common & {
      variant: 'segmented'
      options: FieldOption[]
      value: string
      onChange: (value: string) => void
    })

export default function SegmentedField(props: Props) {
  const { label, disabled = false, dense = false } = props
  const isSwitch = props.variant === 'switch'
  const options = isSwitch ? SWITCH_OPTIONS : props.options
  const value = isSwitch ? (props.checked ? 'on' : 'off') : props.value

  const wrapRef = useRef<HTMLDivElement>(null)
  const btnRefs = useRef<(HTMLButtonElement | null)[]>([])
  const pillRef = useRef<HTMLSpanElement>(null)

  const at = options.findIndex((o) => o.value === value)
  const activeIndex = at < 0 ? 0 : at

  /* 滑块：量当前段的位置与宽度。首帧不要过渡，否则挂载时会从 0 滑进来 */
  const [pill, setPill] = useState({ left: 0, width: 0 })
  const [animate, setAnimate] = useState(false)

  /**
   * 把滑块对准选中的那一段。`instant` = 直接改 DOM 且不吃过渡。
   *
   * # 为什么不能只量一次（C13 修的）
   *
   * 原来这里是「在 layout effect 里量一次 `offsetLeft` / `offsetWidth`」，量完就完 ——
   * 可这两个数是**那一刻的快照**，而这一组的宽度并不只由「选中了谁」决定：
   *
   *   - 密度 / 字号一变（窗口换档、窄档降字号），段的宽度全变
   *   - `dense` 那句 `font-size: calc(--fs-label − 1px)` 让长短文案的组不一样宽
   *   - 参数台的行挂着 `content-visibility: auto`：**屏幕外的行不参与布局**，
   *     那时量出来是 0 —— 宿主一旦在「这一行在屏幕外」的时候重渲染过，
   *     滑块就被写成 0 宽，滚回来也不会自己复原
   *
   * 作者看到的就是这两副样子：白块整个没了、或者白块比那一段窄一截/偏半个格
   * （「选择器的那个格子偏差了」）。所以：量到 0 就**不写**（那一行还没布局），
   * 并且交给下面那个 ResizeObserver 在**布局自己变了**的时候补量一次。
   */
  const measure = useCallback(
    (instant: boolean): boolean => {
      const btn = btnRefs.current[activeIndex]
      if (btn === null || btn === undefined) return false
      /* 0 宽 = 这一行还没被布局（见上面那条），写进去滑块就没了 */
      if (btn.offsetWidth === 0) return false
      const next = { left: btn.offsetLeft, width: btn.offsetWidth }
      if (instant) {
        /*
         * 布局自己变的时候该「跟上」，不该「滑过去」—— 拖动窗口时过渡会让滑块
         * 一直追在旧位置后面（那就是「偏差」的观感）。先关过渡写进去、逼一次
         * 样式计算，再放开：下次**换一段**时该有的动画照旧。
         */
        const el = pillRef.current
        if (el !== null) {
          el.style.transition = 'none'
          el.style.left = `${next.left}px`
          el.style.width = `${next.width}px`
          void el.offsetWidth
          el.style.transition = ''
        }
      }
      setPill((p) => (p.left === next.left && p.width === next.width ? p : next))
      return true
    },
    [activeIndex],
  )

  useLayoutEffect(() => {
    /* 换了一段：走 state + 过渡，滑块从旧位置滑到新位置 */
    if (!measure(false)) return
    const id = requestAnimationFrame(() => setAnimate(true))
    return () => cancelAnimationFrame(id)
  }, [measure, options, dense])

  /*
   * 布局自己变了（密度 / 字号 / 栏宽 / 这一行刚从屏幕外滚进来）—— 立刻补量一次。
   * 观察的是整组：段一宽，组就宽，所以「组变了」等价于「段变了」。
   */
  useLayoutEffect(() => {
    const el = wrapRef.current
    if (el === null) return
    const ro = new ResizeObserver(() => {
      if (measure(true)) setAnimate(true)
    })
    ro.observe(el)
    return () => ro.disconnect()
  }, [measure, options, dense])

  const commit = (next: string) => {
    if (disabled) return
    if (props.variant === 'switch') props.onChange(next === 'on')
    else props.onChange(next)
  }

  /** 焦点跟着选中项走，不然方向键按第二下会从头开始 */
  const focusAt = (i: number) => btnRefs.current[i]?.focus()

  const move = (to: number) => {
    const next = options[to]
    if (next === undefined) return
    commit(next.value)
    focusAt(to)
  }

  const onKeyDown = (e: KeyboardEvent) => {
    if (disabled) return
    const last = options.length - 1
    if (e.key === 'ArrowRight' || e.key === 'ArrowDown') {
      e.preventDefault()
      move((activeIndex + 1) % options.length)
    } else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') {
      e.preventDefault()
      move((activeIndex - 1 + options.length) % options.length)
    } else if (e.key === 'Home') {
      e.preventDefault()
      move(0)
    } else if (e.key === 'End') {
      e.preventDefault()
      move(last)
    }
  }

  const long = options.some((o) => o.label.length > LONG_LABEL)

  return (
    <div
      ref={wrapRef}
      className={disabled ? s.groupOff : s.group}
      data-dense={dense}
      data-long={long}
      role="radiogroup"
      aria-label={label}
      aria-disabled={disabled || undefined}
      onKeyDown={onKeyDown}
    >
      {/* 滑块画在按钮底下，位置由测量得来 —— 选中态不靠给按钮加背景，避免两段各自一块色 */}
      <span
        ref={pillRef}
        className={s.pill}
        aria-hidden
        data-on={isSwitch && value === 'on'}
        style={{
          left: pill.left,
          width: pill.width,
          transition: animate ? undefined : 'none',
        }}
      />

      {options.map((o, i) => {
        const on = o.value === value
        return (
          <button
            key={o.value}
            ref={(node) => {
              btnRefs.current[i] = node
            }}
            type="button"
            role="radio"
            aria-checked={on}
            tabIndex={on ? 0 : -1}
            className={on ? s.segOn : s.seg}
            /* 这一档已弃用（C13）：划一条线，但仍然可点 —— 落盘那道闸在调用方 */
            data-dep={o.deprecated ? '' : undefined}
            disabled={disabled}
            title={o.note}
            onClick={() => commit(o.value)}
          >
            {o.label}
          </button>
        )
      })}
    </div>
  )
}
