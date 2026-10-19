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
import { decimalsOf, formatNum } from './value'
import s from './NumberField.module.css'

interface Props {
  value: number
  label: string
  unit?: string
  min?: number
  max?: number
  step?: number
  /**
   * 小数位。不给就按 `step` 推。
   *
   * ★ 2026-10-17：翻成 `null` 是**合法的** —— 那是"不补零、只修浮点尾巴"
   *   （擦料塔 X/Y 就是这种：拖动落整数、手输可以有小数，所以绝不能按
   *   `step=1.0` 推出 0 位、把用户打的 58.8 抹成 59）。`undefined` 仍然表示
   *   "按 step 推"，两个缺省语义不同，别合并。
   */
  decimals?: number | null
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

/** 串的宽度估算（单位 ch，与输入框的 `widthCh` 同一口径）：CJK 一个算两格，其余一格 */
const textCh = (t: string) =>
  Array.from(t).reduce((w, c) => w + (c.charCodeAt(0) > 0x2e80 ? 2 : 1), 0)

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
  /*
   * `decimals === null` = 不补零（见 Props.decimals）：这时还按 step 推出来的
   * 小数位仍然要用来**四舍五入**（拖动/连发不许留浮点尾巴），
   * 只是**不显示**成固定位数。所以这里分成两个量：
   *   `digits`      —— 算数用（null 时退回 step 的位数）
   *   `showDigits`  —— 显示用（null 原样传下去）
   */
  const digits = decimals ?? decimalsOf(step)
  const showDigits = decimals === undefined ? decimalsOf(step) : decimals
  const fmt = useCallback((n: number) => formatNum(String(n), showDigits), [showDigits])
  const [text, setText] = useState(() => formatNum(String(value), showDigits))
  const inputRef = useRef<HTMLInputElement>(null)
  /** 整个控件（框 + 单位 + 箭头）—— 「点框外」判据用 */
  const boxRef = useRef<HTMLSpanElement>(null)
  /*
   * 校验气泡的**出现时机**（2026-10-20 作者改）：只在编辑中或悬停时浮出来，
   * 平时只留红描边 —— 否则一个没在编辑的空框头顶常年挂着一串红字，
   * 压在上一行下面，看着像版式坏了。
   */
  const [fieldFocused, setFieldFocused] = useState(false)
  const [fieldHovered, setFieldHovered] = useState(false)
  /** 失焦闸正在拉焦点回来的标记：别重复排队，也别让气泡闪一下 */
  const recaptureRef = useRef(false)

  /** 连发期间的当前值放 ref 里：interval 里读 state 会读到旧值 */
  const numRef = useRef(value)
  const delayRef = useRef<number | null>(null)
  const tickRef = useRef<number | null>(null)
  const wheelRef = useRef<number | null>(null)
  const holdingRef = useRef(false)

  useEffect(() => {
    /* 外部值一变（拖别的控件 / 撤销 / 应用预设）就重新按显示口径渲染 */
    setText(fmt(value))
    numRef.current = value
  }, [fmt, value])

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
      setText(fmt(next))
      return next
    },
    [clamp, fmt, step],
  )

  const commit = useCallback(() => {
    if (numRef.current !== value) onChange(numRef.current)
  }, [onChange, value])

  /*
   * 键入的提交：不夹取越界值，只挡「不是数字」。
   *
   * ★ 步进入口（箭头 / 滚轮 / 方向键）也要先走这一步（2026-10-20）：箭头
   *   onPointerDown preventDefault 让输入框**不失焦**，blur 那次提交不会跑 ——
   *   框里打完 `-0.2` 不回车直接点箭头，`numRef` 还停在上一次的值（比如 0），
   *   一步就从 0 算、显示当场被拽回 `0.00`，刚打的数无声丢失（作者实测）。
   *   所以任何步进都先把手输原文落进 `numRef`，再从它算步。
   */
  const commitTyped = useCallback(() => {
    const n = Number(text)
    if (text.trim() === '' || !Number.isFinite(n)) {
      setText(fmt(value))
      numRef.current = value
      return
    }
    const next = round(n)
    numRef.current = next
    setText(fmt(next))
    if (next !== value) onChange(next)
  }, [fmt, onChange, round, text, value])

  const stopHold = useCallback(() => {
    if (delayRef.current !== null) window.clearTimeout(delayRef.current)
    if (tickRef.current !== null) window.clearInterval(tickRef.current)
    delayRef.current = null
    tickRef.current = null
  }, [])

  const startHold = useCallback(
    (dir: 1 | -1) => {
      holdingRef.current = true
      commitTyped() // 框里有没落账的手输值就先落账，第一步从它算（见 commitTyped 注）
      nudge(dir) // 第一步立刻走，不等 400ms
      stopHold()
      delayRef.current = window.setTimeout(() => {
        tickRef.current = window.setInterval(() => {
          const before = numRef.current
          if (nudge(dir) === before) stopHold() // 到边界了，连发自停
        }, HOLD_TICK)
      }, HOLD_DELAY)
    },
    [commitTyped, nudge, stopHold],
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
      commitTyped() // 同箭头：聚焦着打完的数还没 blur，先落账再步进
      nudge(e.deltaY < 0 ? 1 : -1)
      if (wheelRef.current !== null) window.clearTimeout(wheelRef.current)
      wheelRef.current = window.setTimeout(commit, WHEEL_IDLE)
    }
    el.addEventListener('wheel', onWheel, { passive: false })
    return () => {
      el.removeEventListener('wheel', onWheel)
      if (wheelRef.current !== null) window.clearTimeout(wheelRef.current)
    }
  }, [commit, commitTyped, disabled, nudge])

  const typed = Number(text)
  const outOfRange =
    Number.isFinite(typed) &&
    text.trim() !== '' &&
    ((min !== undefined && typed < min) || (max !== undefined && typed > max))
  const bad = invalid || outOfRange

  /*
   * 即时校验提示（2026-10-20，作者：「只有红色描边还不够」）：框下面**用文字**当场说清
   * 为什么不行。判据全在本地 `text` 上，所以打字过程中就出、不用等 blur 提交。
   * 正在打到一半的符号（`-` / `+` / `.`）不出，免得每敲一个符号闪一次红字。
   */
  const rawTyping = text.trim()
  const hint = ((): string | null => {
    if (disabled) return null
    if (rawTyping === '') return '数值不能为空'
    if (rawTyping === '-' || rawTyping === '+' || rawTyping === '.') return null
    if (!Number.isFinite(typed)) return '不是有效的数字'
    if (outOfRange) return `超出范围 ${min ?? '不限'} ~ ${max ?? '不限'}`
    return invalid ? '不是有效的数字' : null
  })()

  /*
   * **失焦闸**（2026-10-20 作者：「空着的时候，或者超出限制的时候，不要失去焦点」）：
   * 打出来的东西不合法就不放焦点走 —— 点空白 / 点别的框都把焦点拉回本框，
   * 改到合法、或按 Esc 还原为止。
   * 放行条件：改成了合法值；或**原文一个字没动**（外部值本身就不合法的历史脏数据
   * —— 再不放行就把人锁死在这个框里了，Esc 也出不去）。
   */
  const blurBlocked = (): boolean => {
    if (disabled) return false
    if (rawTyping === '') return true
    if (!Number.isFinite(typed)) return true
    if (outOfRange) return Number.isFinite(value) ? typed !== value : true
    return false
  }

  /*
   * **点框外先落账**（2026-10-20）：底栏的「保存」在"没有改动"时是 disabled 的 ——
   * 点它既不抢焦点、也不触发 click，输入框永远收不到 blur，打进去的整数就一直
   * 悬在框里（作者实测：输 `1` 点保存，没补小数、底栏也没有"1 处未保存"）。
   * 所以聚焦期间在 document 捕获阶段挂一次 pointerdown：点在框外、且框里是一个
   * **能落账的数**，当场 commitTyped（补零 / 进草稿 / 撤销栈都照常走）；
   * 空 / 不是数 / 超范围不在这里动 —— 那是失焦闸的活（拉回焦点，见 onBlur）。
   */
  useEffect(() => {
    if (!fieldFocused || disabled) return
    const onDown = (e: PointerEvent) => {
      const t = e.target
      if (t instanceof Node && boxRef.current?.contains(t) === true) return
      if (rawTyping === '' || !Number.isFinite(typed) || outOfRange) return
      commitTyped()
    }
    document.addEventListener('pointerdown', onDown, true)
    return () => document.removeEventListener('pointerdown', onDown, true)
  }, [commitTyped, disabled, fieldFocused, outOfRange, rawTyping, typed])

  /*
   * 宽度按能出现的字符数算，不写死一个 px：
   * 整数位（看 min/max 的量级）+ 小数点与小数位 + 负号。
   * ★ 占位串（`-50 ~ 50`）也比进去（2026-10-20）：否则空框的灰字被压成
   *   `-50 ~ 5`，看起来像注册表写错了范围。
   */
  const intDigits = Math.max(
    String(Math.trunc(Math.abs(max ?? 100))).length,
    String(Math.trunc(Math.abs(min ?? 0))).length,
  )
  const negative = (min ?? 0) < 0
  const rangePlaceholder =
    min !== undefined || max !== undefined ? `${min ?? '不限'} ~ ${max ?? '不限'}` : undefined
  const widthCh = Math.max(
    4,
    intDigits + (digits > 0 ? digits + 1 : 0) + (negative ? 1 : 0) + 1,
    rangePlaceholder !== undefined ? textCh(rangePlaceholder) : 0,
  )

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

  const box = (
    <span
      ref={boxRef}
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
        placeholder={rangePlaceholder}
        disabled={disabled}
        onChange={(e) => setText(e.target.value)}
        onFocus={() => setFieldFocused(true)}
        onBlur={(e) => {
          /*
           * 失焦闸（见 blurBlocked）：不合法就不放走。同步 focus() 会被这次
           * mousedown 的默认焦点落点盖掉（实测焦点照样丢、光标没了）——
           * 所以异步补一枪：等浏览器把这次点击的焦点安置完，再把焦点抢回来。
           * 抢回期间 fieldFocused 保持 true，气泡不闪；元素都没了（模态框关了）
           * 就自然跳过，不会有循环。
           */
          if (blurBlocked()) {
            e.preventDefault()
            if (recaptureRef.current) return
            recaptureRef.current = true
            window.setTimeout(() => {
              recaptureRef.current = false
              inputRef.current?.focus()
            }, 0)
            return
          }
          setFieldFocused(false)
          commitTyped()
        }}
        onKeyDown={(e) => {
          if (e.key === 'Enter') {
            commitTyped()
          } else if (e.key === 'Escape') {
            /* 还原：回到外部值，什么都不提交 */
            setText(fmt(value))
            numRef.current = value
          } else if (e.key === 'ArrowUp') {
            e.preventDefault()
            commitTyped()
            nudge(1)
            commit()
          } else if (e.key === 'ArrowDown') {
            e.preventDefault()
            commitTyped()
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

  /*
   * **提示层恒在**（2026-10-20 真机 + 浏览器双重复现出来的那个 bug 的正解）：
   * 原来是"有问题才包一层"—— 框里第一个不合法字符出现的瞬间，React 会把
   * `<span class=box>` 整个子树换掉（父节点类型变了），**输入框被重建**：
   * 焦点当场丢、blur 事件根本不来（连提交都没发生），打进去的整数跟着消失。
   * 所以外层 relative 的 span 与气泡都**永远渲染**，只切 `data-show` 的可见性 ——
   * 结构不动，输入框就不动。气泡绝对定位悬浮在框正上方，不占布局。
   */
  return (
    <span
      className={s.withHint}
      data-show={(hint !== null && (fieldFocused || fieldHovered)) || undefined}
      onMouseEnter={() => setFieldHovered(true)}
      onMouseLeave={() => setFieldHovered(false)}
    >
      {box}
      <span className={s.hint} role="status">
        {hint ?? ''}
      </span>
    </span>
  )
}
