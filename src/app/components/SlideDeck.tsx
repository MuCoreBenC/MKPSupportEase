import { forwardRef, useCallback, useEffect, useImperativeHandle, useMemo, useRef, useState } from 'react'
import type {
  CSSProperties,
  MouseEvent as ReactMouseEvent,
  PointerEvent as ReactPointerEvent,
  ReactNode,
} from 'react'
import type { Density } from '../../hooks/useDensity'
import DeckLayerContext, { type DeckLayerInfo } from './DeckLayerContext'
import s from './SlideDeck.module.css'

export interface Sheet {
  id: string
  node: ReactNode
}

type Phase = 'idle' | 'in' | 'out'

interface SlideDeckProps {
  sheets: Sheet[]
  density: Density
  /**
   * 换页前问一句：返回 false 就把这次换页拦下（比如有未保存的改动）。
   * 拦下的一方自己决定何时用 ref 的 jumpTo 再跳。不传就是从不拦。
   */
  canLeave?: (from: number, to: number) => boolean
}

/** 外部（如首页的"切换机型"按钮）用来直接推入某一页 */
export interface DeckHandle {
  jumpTo: (index: number) => void
}

const ENTER_MS = 200
const RAIL_FLASH_MS = 1200

/** 退出层永远是 out：不随 phase 变，单独提出来省一次 memo */
const EXIT_INFO: DeckLayerInfo = { layer: 'exit', phase: 'out' }

const isTyping = (t: EventTarget | null) =>
  t instanceof HTMLElement && /^(input|select|textarea)$/i.test(t.tagName)

/** 双击 / 三击时吞掉默认行为，避免浏览器把选区落到最近的可选文本上 */
const noMultiClickSelect = (e: ReactMouseEvent) => {
  if (e.detail > 1) e.preventDefault()
}


/*
 * 露出卡的出场斜坡（本稿的主角）。
 *
 * 以前是开关：density ≥ compact（容器宽 ≥ 640）才渲染露出卡，一出现就是满量 20%，
 * 640 这条线上"啪"地多出一张卡。现在改成连续 —— 520 起露头，到 640 满量：
 *
 *   p(W) = clamp(0, PEEK_MAX × (W − PEEK_FROM) / (PEEK_FULL − PEEK_FROM), PEEK_MAX)
 *
 * 摊下来是「每 6px 宽度多露 1%」。两端与老稿逐像素一致：≥640 是 0.2，≤520 是 0（不渲染）。
 *
 * 为什么在 JS 算：CSS 没法把两个长度相除得到无单位比例（`calc((100cqw - 520px) / 120px)`
 * 不合法），而 transform 的百分比与 scale 都需要那个无单位的 p。所以这里算完，
 * 作为 `--peek-ratio` 下发，几何全部由 CSS 用它推出来。
 */
const PEEK_FROM = 520
const PEEK_FULL = 640
const PEEK_MAX = 0.2

const peekFor = (w: number) =>
  Math.min(PEEK_MAX, Math.max(0, (PEEK_MAX * (w - PEEK_FROM)) / (PEEK_FULL - PEEK_FROM)))

/**
 * 换页动画时长。
 *
 * 试验场里这个数（与下面的"显示热区"开关）来自调参面板，默认 720ms / 热区不显示。
 * 产品里没有面板，取默认值定为常量 —— 屏幕上的结果与那边默认状态一致。
 */
const GLIDE_MS = 720

/**
 * 卡片推入时，内容（.frame）的动画比卡片本身长：多出来的尾段专门用来
 * 让 blur 从 2.4px 线性归零，遮住 Windows Chromium 合成层切换时的文字跳变。
 * JS 的 setPhase('idle') 定时器必须等到这段也播完，否则 data-phase 提前切走，
 * CSS 动画被中断，blur 瞬间跳 0——改了等于没改。
 *
 * 0.15 = 1.0 - 0.85，对应 frame-settle 关键帧里 85%→100% 那段。
 */
const SETTLE_TAIL_MS = Math.round(GLIDE_MS * 0.15)

/** 左右两侧翻页热区是否画出来：调试用，产品里恒为 false */
const SHOW_HIT = false

const SlideDeck = forwardRef<DeckHandle, SlideDeckProps>(function SlideDeck(
  { sheets, density, canLeave },
  ref,
) {
  const [index, setIndex] = useState(0)
  const [phase, setPhase] = useState<Phase>('idle')
  const [exitSheet, setExitSheet] = useState<Sheet | null>(null)
  const [entering, setEntering] = useState(false)
  const [railFlash, setRailFlash] = useState(false)
  const [railDrag, setRailDrag] = useState(false)
  /** 露出比例：0 = 这一档没有露出卡。由 deck 自己的宽度算，见 peekFor */
  const [peek, setPeek] = useState(0)
  const busy = useRef(false)
  const timers = useRef<number[]>([])
  const flashTimer = useRef<number | null>(null)
  const railRef = useRef<HTMLDivElement>(null)
  const railDragRef = useRef(false)
  const cardRef = useRef<HTMLDivElement>(null)
  const exitRef = useRef<HTMLDivElement>(null)
  const deckRef = useRef<HTMLDivElement>(null)
  /** go() 里要读最新的 p，但它是 useCallback 的依赖之外的东西，用 ref 取当前值 */
  const peekRef = useRef(0)
  peekRef.current = peek

  /*
   * 量 deck 自己的宽度（不是窗口、不是 shell）：露出卡是 deck 的一层，
   * 它的几何只与 deck 有关。密度档位仍由上层的 useDensity 决定，两者互不替代。
   */
  useEffect(() => {
    const el = deckRef.current
    if (!el) return
    const measure = () => setPeek(peekFor(el.getBoundingClientRect().width))
    measure()
    const ro = new ResizeObserver(measure)
    ro.observe(el)
    return () => ro.disconnect()
  }, [])

  // 换页后页条自动亮一下，给"我在第几页"的反馈
  useEffect(() => {
    setRailFlash(true)
    if (flashTimer.current !== null) window.clearTimeout(flashTimer.current)
    flashTimer.current = window.setTimeout(() => setRailFlash(false), RAIL_FLASH_MS)
  }, [index])

  useEffect(
    () => () => {
      timers.current.forEach((t) => window.clearTimeout(t))
      timers.current = []
      if (flashTimer.current !== null) window.clearTimeout(flashTimer.current)
    },
    [],
  )


  const go = useCallback(
    (to: 'next' | 'prev') => {
      if (busy.current) return
      const target = to === 'next' ? index + 1 : index - 1
      if (target < 0 || target >= sheets.length) return
      // 有人要拦（比如未保存的改动）就此打住，状态一点不动
      if (canLeave && !canLeave(index, target)) return

      /*
       * 一点都不露的那一档（容器宽 ≤ 520）没有右侧那张卡，推入与退回两段动画都没有载体
       * （showCard 和 exitSheet 的渲染条件都跟着 p）—— 动画期只是白等 glideMs。
       * 这一档直接换页。「上一步」原来就因为 setIndex 是同步的而看着是立刻的，
       * 这里只是把「下一步」拉齐；也不占 busy 闸门，没有动画要保护。
       */
      if (peekRef.current === 0) {
        setIndex(target)
        return
      }

      busy.current = true


      if (to === 'next') {
        // 右侧卡片推入，落位后接管为平面
        setPhase('in')
        timers.current.push(
          window.setTimeout(() => {
            setIndex(target)
            setPhase('idle')
            setEntering(true)
            busy.current = false
          }, GLIDE_MS + SETTLE_TAIL_MS),
        )
        timers.current.push(window.setTimeout(() => setEntering(false), GLIDE_MS + SETTLE_TAIL_MS + ENTER_MS))
        return
      }

      // 返回：当前页缩小、退回右边缘变成卡片；平面立刻换成上一页
      setExitSheet(sheets[index])
      setIndex(target)
      setPhase('out')
      timers.current.push(
        window.setTimeout(() => {
          setExitSheet(null)
          setPhase('idle')
          busy.current = false
        }, GLIDE_MS),
      )
    },
    [canLeave, index, sheets],
  )

  /** 直接换页 + 交叉淡入：跨多页与页条拖动用，不走推入动画 */
  const fadeTo = useCallback(
    (target: number) => {
      if (canLeave && !canLeave(index, target)) return
      timers.current.forEach((t) => window.clearTimeout(t))
      timers.current = []
      busy.current = false
      setExitSheet(null)
      setPhase('idle')
      setIndex(target)
      setEntering(true)
      timers.current.push(window.setTimeout(() => setEntering(false), ENTER_MS))
    },
    [canLeave, index],
  )

  const jumpTo = useCallback(
    (target: number, mode: 'auto' | 'fade' = 'auto') => {
      if (target === index || target < 0 || target >= sheets.length) return
      // 相邻一页仍走推入 / 退回，手感不变
      if (mode === 'auto' && Math.abs(target - index) === 1) {
        go(target > index ? 'next' : 'prev')
        return
      }
      fadeTo(target)
    },
    [fadeTo, go, index, sheets.length],
  )

  useImperativeHandle(ref, () => ({ jumpTo: (target: number) => jumpTo(target) }), [jumpTo])

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (isTyping(e.target) || e.metaKey || e.ctrlKey || e.altKey) return
      if (e.key === 'ArrowRight') go('next')
      else if (e.key === 'ArrowLeft') go('prev')
      else if (e.key === 'Home') jumpTo(0)
      else if (e.key === 'End') jumpTo(sheets.length - 1)
      else if (/^[1-9]$/.test(e.key)) jumpTo(Number(e.key) - 1)
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [go, jumpTo, sheets.length])


  // 卡片位与退出层都不参与交互和 Tab 序列
  useEffect(() => {
    cardRef.current?.setAttribute('inert', '')
    exitRef.current?.setAttribute('inert', '')
  }, [index, phase, exitSheet])

  /** 这一档到底露不露卡。判据从"density 是不是 mini"换成"算出来的 p 是不是 0" */
  const peeking = peek > 0
  const hasNext = index < sheets.length - 1
  const hasPrev = index > 0
  const cardSheet = sheets[index + 1]
  const showCard = cardSheet && peeking && phase !== 'out'

  /**
   * 「第 i 页的右边会不会被露出卡盖住」。CardFrame 拿这个结论决定要不要让位 ——
   * 以前让位量是纯 CSS 的 20cqw 斜坡，CSS 不知道卡有没有渲染，于是窄窗里
   * 卡没出现、右内边距照样扣掉一大块，内容在被压窄的盒子里"居中"，看着就是偏左。
   *
   * 按页算而不是整屏一个结论：卡片层里装的是 sheets[index+1] 自己，它右边有没有卡
   * 要看 sheets[index+2]。以前把结论挂在 deck 上，卡片层就继承了平面层那句"右边有卡"，
   * 于是**末页**在露出态里白让了一次位，滑到位换成平面层时内边距突然归零 ——
   * 内容先偏左、落位时往右跳一下。末页右边永远不会有卡，两态一致才是对的。
   * 这条规则自动跟着页数走：以后在末尾加页，新的末页不让位、旧末页自动恢复让位。
   *
   * 不用 showCard：后者含 `phase !== 'out'`，退出动画期间让位量会突然归零，布局要跳一下。
   */
  const peekAt = (i: number) => peeking && i + 1 < sheets.length


  // 层身份下发给内容：露出态想换布局（不只是换样式）的组件要认它
  const planeInfo = useMemo<DeckLayerInfo>(() => ({ layer: 'plane', phase }), [phase])
  const cardInfo = useMemo<DeckLayerInfo>(() => ({ layer: 'card', phase }), [phase])

  const pickFromX = (clientX: number) => {
    const r = railRef.current?.getBoundingClientRect()
    if (!r || r.width <= 0) return null
    const t = (clientX - r.left) / r.width
    return Math.min(sheets.length - 1, Math.max(0, Math.floor(t * sheets.length)))
  }

  const onRailDown = (e: ReactPointerEvent<HTMLDivElement>) => {
    e.currentTarget.setPointerCapture(e.pointerId)
    railDragRef.current = true
    setRailDrag(true)
    const i = pickFromX(e.clientX)
    if (i !== null) jumpTo(i, 'fade')
  }

  const onRailMove = (e: ReactPointerEvent<HTMLDivElement>) => {
    if (!railDragRef.current) return
    const i = pickFromX(e.clientX)
    if (i !== null) jumpTo(i, 'fade')
  }

  const onRailUp = () => {
    railDragRef.current = false
    setRailDrag(false)
  }


  return (
    <div
      ref={deckRef}
      className={s.deck}
      data-density={density}
      /* 这一个数把「露多少」贯穿到全部几何：卡的平移、卡面的缩放、右侧热区宽、
         以及 CardFrame 的让位量（--chrome-peek）。四处只有这一个来源 */
      style={{ '--glide-ms': `${GLIDE_MS}ms`, '--peek-ratio': peek } as CSSProperties}
    >
      {/*
        每层按「层名 + sheet.id」挂 key：换 id 是为了不让 React 按位置复用上一页的
        DOM（复用会继承上一页的样式起点，左下角胶囊就「先亮一下再淡掉」）；
        加层名前缀是因为光用 id 时，翻页后平面层的 key 正好等于上一帧卡片层的 key，
        React 会把卡片那个节点搬过来当平面用 —— 而卡片节点被 setAttribute('inert')
        标过，搬过来就成了一整页看得见却点不动的死页面（真踩过）。
      */}
      <div
        key={`plane-${sheets[index].id}`}
        className={s.plane}
        data-layer="plane"
        data-peek={peekAt(index) ? 'true' : undefined}
      >
        <DeckLayerContext.Provider value={planeInfo}>
          {sheets[index].node}
        </DeckLayerContext.Provider>
      </div>

      {showCard && (
        <div
          key={`card-${cardSheet.id}`}
          ref={cardRef}
          className={s.card}
          data-layer="card"
          data-phase={phase}
          data-entering={entering}
          /* 卡里装的是 sheets[index+1]，所以它的让位看 index+2：末页在这里就已经是
             "不让位"的布局，滑到位换成平面层时内边距不变，落位不跳 */
          data-peek={peekAt(index + 1) ? 'true' : undefined}
          aria-hidden="true"
        >
          <div className={s.face} />

          <DeckLayerContext.Provider value={cardInfo}>{cardSheet.node}</DeckLayerContext.Provider>
        </div>
      )}

      {exitSheet && peeking && (
        <div
          key={`exit-${exitSheet.id}`}
          ref={exitRef}
          className={s.card}
          data-layer="exit"
          data-phase="out"
          /* 退出层装的也是 index+1：go('prev') 里 setExitSheet(sheets[index]) 先于
             setIndex(index-1)，所以退完之后它正好落在 index+1 这一格 */
          data-peek={peekAt(index + 1) ? 'true' : undefined}
          aria-hidden="true"
        >
          <div className={s.face} />

          <DeckLayerContext.Provider value={EXIT_INFO}>{exitSheet.node}</DeckLayerContext.Provider>
        </div>
      )}


      {hasNext && peeking && (
        <button
          type="button"
          className={s.zoneNext}
          data-show={SHOW_HIT}
          onClick={() => go('next')}
          onMouseDown={noMultiClickSelect}
          aria-label="下一页"
          title="下一页（→）"
        />
      )}

      <button
        type="button"
        className={s.zonePrev}
        data-show={SHOW_HIT}
        disabled={!hasPrev}
        onClick={() => go('prev')}
        onMouseDown={noMultiClickSelect}
        aria-label="上一页"
        title="上一页（←）"
      />

      {sheets.length > 1 && (
        <div className={s.railHit}>
          <div
            ref={railRef}
            className={s.rail}
            data-show={railFlash || railDrag}
            onPointerDown={onRailDown}
            onPointerMove={onRailMove}
            onPointerUp={onRailUp}
            onPointerCancel={onRailUp}
            onMouseDown={noMultiClickSelect}
          >
            {sheets.map((sheet, i) => (
              <button
                key={sheet.id}
                type="button"
                className={s.seg}
                data-on={i === index}
                aria-current={i === index ? 'page' : undefined}
                aria-label={`第 ${i + 1} 页`}
                onClick={() => jumpTo(i)}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  )
})

export default SlideDeck

