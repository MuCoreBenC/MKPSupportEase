/*
 * 数字输入 + 步进器（#33）。
 *
 * 由 #31 的 `NumberStepperV032` 与参考项目的 `CompactNumberInput` 合并而来，对外改说强类型：
 * 收 `value: number`、回 `number`。字符串 ⇄ 数字在 `FieldControl` 那一处转。
 *
 * # 一次手势 = 一条撤销
 *
 * 长按连发与滚轮会在几百毫秒里产生几十次数值变化。每次都 `onChange` 的话，撤销栈会被塞进
 * 几十条，按一次撤销只退回 1/60 —— 那撤销就没用了。所以连发期间**只改本地显示**，
 * 松手（或滚轮停下 350ms）才提交一次。
 *
 * # 连发节奏 400ms → 100ms
 *
 * 照参考项目：按下先走一步，停 400ms（给「点一下就一步」留出时间），之后每 100ms 一步。
 * #31 用的是 60ms + 第 10 步后 ×10 加速 —— 那个加速在 `0~600` 这种范围里很好用，但在
 * `-0.3~0.3 step 0.01` 上会一脚踩到边界。这一稿只保留稳定的 100ms，跨大范围交给键入。
 *
 * # 失焦不静默夹取
 *
 * 键入 `999`（上限 300）时**照原样提交**，并把框标红。静默夹成 300 等于「我帮你改了但不告诉你」，
 * 而宿主（参数页的行、密表的格子）本来就有标红的位置。
 * 只有「根本不是数字」（空串、`-`、`1.2.3`）才退回原值 —— 那种情况没有值可以提交。
 *
 * # 聚焦框
 *
 * global.css 给所有 input 上了 `outline: 2px` + `offset: 2px` 的全局聚焦环，它跟着 input
 * 自己的矩形走（圆角在外层容器上），于是会出现「套在圆角框外的直角方块」。
 * 全局那条不能改（v001–v032 都在用），所以这里在 .module.css 里压掉：聚焦看容器边框。
 */

import { useCallback, useEffect, useRef, useState } from 'react'
import { decimalsOf } from './value'
import s from './NumberField.module.css'

interface Props {
  value: number
  label: string
  unit?: string
  min?: number
  max?: number
  step?: number
  /** 小数位。不给就按 step 推 */
  decimals?: number
  disabled?: boolean
  /** 宿主认定这个值不合法（例如原文根本不是数字）—— 与本地的越界判定一起标红 */
  invalid?: boolean
  /** cell 档用 sm：矮一点、窄一点 */
  size?: 'sm' | 'md'
  /**
   * 点**框内任何一处**（值、单位、两侧内衬）都把焦点落到输入框上。
   *
   * 默认关（老稿一个字不动）。开着的是 C12 参数台那两处（行内与矩阵格子）：
   * 作者的原话是「我点击之后，没有进入编辑态，不是很舒服」—— 原来只有正好
   * 戳中数字那几个像素才进编辑，戳在单位或框沿上就毫无反应。
   */
  focusOnBoxClick?: boolean
  onChange: (next: number) => void
}

const HOLD_DELAY = 400
const HOLD_TICK = 100
const WHEEL_IDLE = 350

export default function NumberField({
  value,
  label,
  unit,
  min,
  max,
  step = 1,
  decimals,
  disabled = false,
  invalid = false,
  size = 'md',
  focusOnBoxClick = false,
  onChange,
}: Props) {
  const digits = decimals ?? decimalsOf(step)
  const [text, setText] = useState(() => String(value))
  const inputRef = useRef<HTMLInputElement>(null)

  /** 连发期间的当前值放 ref 里：interval 里读 state 会读到旧值 */
  const numRef = useRef(value)
  const delayRef = useRef<number | null>(null)
  const tickRef = useRef<number | null>(null)
  const wheelRef = useRef<number | null>(null)
  const holdingRef = useRef(false)

  useEffect(() => {
    setText(String(value))
    numRef.current = value
  }, [value])

  /** 修浮点尾巴：0.1 * 3 会得到 0.30000000000000004 */
  const round = useCallback(
    (n: number) => {
      const factor = 10 ** digits
      return Math.round((n + Number.EPSILON) * factor) / factor
    },
    [digits],
  )

  /** 步进要夹在范围里（按着箭头不该越界），键入不夹（见文件头） */
  const clamp = useCallback(
    (n: number) => {
      let v = n
      if (min !== undefined) v = Math.max(min, v)
      if (max !== undefined) v = Math.min(max, v)
      return round(v)
    },
    [max, min, round],
  )

  /** 只改显示，不提交 —— 提交由 commit() 在手势结束时做一次 */
  const nudge = useCallback(
    (dir: 1 | -1) => {
      const next = clamp(numRef.current + dir * step)
      numRef.current = next
      setText(String(next))
      return next
    },
    [clamp, step],
  )

  const commit = useCallback(() => {
    if (numRef.current !== value) onChange(numRef.current)
  }, [onChange, value])

  const stopHold = useCallback(() => {
    if (delayRef.current !== null) window.clearTimeout(delayRef.current)
    if (tickRef.current !== null) window.clearInterval(tickRef.current)
    delayRef.current = null
    tickRef.current = null
  }, [])

  const startHold = useCallback(
    (dir: 1 | -1) => {
      holdingRef.current = true
      nudge(dir) // 第一步立刻走，不等 400ms
      stopHold()
      delayRef.current = window.setTimeout(() => {
        tickRef.current = window.setInterval(() => {
          const before = numRef.current
          if (nudge(dir) === before) stopHold() // 到边界了，连发自停
        }, HOLD_TICK)
      }, HOLD_DELAY)
    },
    [nudge, stopHold],
  )

  const endHold = useCallback(() => {
    if (!holdingRef.current) return
    holdingRef.current = false
    stopHold()
    commit()
  }, [commit, stopHold])

  /* 松手可能发生在按钮外面（按住拖出去再放），所以监听在 window 上 */
  useEffect(() => {
    window.addEventListener('pointerup', endHold)
    window.addEventListener('blur', endHold)
    return () => {
      window.removeEventListener('pointerup', endHold)
      window.removeEventListener('blur', endHold)
    }
  }, [endHold])

  useEffect(() => stopHold, [stopHold])

  /*
   * 滚轮只在这个框**聚焦时**生效 —— 否则鼠标划过页面滚动就会把参数改掉，
   * 那是最糟的一种「贴心」。要 preventDefault 就不能 passive，只能手工 addEventListener。
   */
  useEffect(() => {
    const el = inputRef.current
    if (el === null || disabled) return
    const onWheel = (e: WheelEvent) => {
      if (document.activeElement !== el) return
      e.preventDefault()
      nudge(e.deltaY < 0 ? 1 : -1)
      if (wheelRef.current !== null) window.clearTimeout(wheelRef.current)
      wheelRef.current = window.setTimeout(commit, WHEEL_IDLE)
    }
    el.addEventListener('wheel', onWheel, { passive: false })
    return () => {
      el.removeEventListener('wheel', onWheel)
      if (wheelRef.current !== null) window.clearTimeout(wheelRef.current)
    }
  }, [commit, disabled, nudge])

  /** 键入的提交：不夹取越界值，只挡「不是数字」 */
  const commitTyped = () => {
    const n = Number(text)
    if (text.trim() === '' || !Number.isFinite(n)) {
      setText(String(value))
      numRef.current = value
      return
    }
    const next = round(n)
    numRef.current = next
    setText(String(next))
    if (next !== value) onChange(next)
  }

  const typed = Number(text)
  const outOfRange =
    Number.isFinite(typed) &&
    text.trim() !== '' &&
    ((min !== undefined && typed < min) || (max !== undefined && typed > max))
  const bad = invalid || outOfRange

  /*
   * 宽度按能出现的字符数算，不写死一个 px：
   * 整数位（看 min/max 的量级）+ 小数点与小数位 + 负号。
   */
  const intDigits = Math.max(
    String(Math.trunc(Math.abs(max ?? 100))).length,
    String(Math.trunc(Math.abs(min ?? 0))).length,
  )
  const negative = (min ?? 0) < 0
  const widthCh = Math.max(4, intDigits + (digits > 0 ? digits + 1 : 0) + (negative ? 1 : 0) + 1)

  const range =
    min !== undefined || max !== undefined ? `范围 ${min ?? '—'} ~ ${max ?? '—'}` : undefined

  const arrow = (dir: 1 | -1) => (
    <button
      type="button"
      className={s.arrow}
      aria-label={`${label} ${dir === 1 ? '增加' : '减少'}`}
      tabIndex={-1}
      onPointerDown={(e) => {
        e.preventDefault() // 不让按钮抢走输入框的焦点
        startHold(dir)
      }}
      onPointerUp={endHold}
      onPointerLeave={endHold}
    >
      <svg viewBox="0 0 10 6" aria-hidden>
        <path
          d={dir === 1 ? 'M1 4.5l4-3.5 4 3.5' : 'M1 1.5l4 3.5 4-3.5'}
          fill="none"
          stroke="currentColor"
          strokeWidth="1.6"
          strokeLinecap="round"
        />
      </svg>
    </button>
  )

  return (
    <span
      className={disabled ? s.boxOff : s.box}
      data-size={size}
      data-invalid={bad}
      /*
       * 点框内任何一处都进编辑态（可选，见 Props.focusOnBoxClick）。
       * 箭头是按钮，交给它自己（它按下会 nudge，抢焦点就点不动了）。
       */
      onMouseDown={
        focusOnBoxClick
          ? (e) => {
              const t = e.target as HTMLElement
              /* 箭头自己按下会 nudge，抢焦点就点不动了 */
              if (t.closest('button') !== null) return
              const input = inputRef.current
              if (input === null || t === input) return
              /*
               * 必须 preventDefault：否则浏览器接着执行它自己的「点这里」
               * 的默认行为（把焦点挪到 body），刚抢来的焦点当场又丢了
               * —— 实测就是这样：焦点给了输入框，然后被默认行为收回去。
               */
              e.preventDefault()
              input.focus()
            }
          : undefined
      }
    >
      <input
        ref={inputRef}
        className={s.input}
        style={{ width: `${widthCh}ch` }}
        value={text}
        inputMode="decimal"
        aria-label={label}
        aria-invalid={bad || undefined}
        title={bad && range !== undefined ? `超出${range}` : range}
        disabled={disabled}
        onChange={(e) => setText(e.target.value)}
        onBlur={commitTyped}
        onKeyDown={(e) => {
          if (e.key === 'Enter') {
            commitTyped()
          } else if (e.key === 'Escape') {
            /* 还原：回到外部值，什么都不提交 */
            setText(String(value))
            numRef.current = value
          } else if (e.key === 'ArrowUp') {
            e.preventDefault()
            nudge(1)
            commit()
          } else if (e.key === 'ArrowDown') {
            e.preventDefault()
            nudge(-1)
            commit()
          }
        }}
      />
      {unit !== undefined && unit !== '' && <span className={s.unit}>{unit}</span>}
      {/* 改不动的时候箭头整格不画 —— 一对点不动的箭头只是噪音 */}
      {!disabled && (
        <span className={s.arrows}>
          {arrow(1)}
          {arrow(-1)}
        </span>
      )}
    </span>
  )
}
