/*
 * 向导的卡片翻页：推入 / 退回 / 露出卡 / 页条。
 *
 * 调试面板那两个值（翻页时长 / 热区）A41 起在试验场又从面板取（`useDevDefaults`）——
 * A39 / A40 把它们写死成常量，面板上那两个按钮对它们空转；A41 兼作「调动画速度的
 * 试验田」。产品仓的接法（A41 README 预写）：hook 直接吃常量（见 devDefaults.ts），
 * 值仍是定值，与面板的默认值一致。
 * 页码**不**走 reportPage（那是 devStore 的接口，面板不属于产品），A41 起改成把
 * `data-deck-index` / `data-deck-total` 挂在 .deck 上 —— 面板与探针从 DOM 上读，
 * 两边零耦合（试验场见 src/dev/panelScope.ts；产品里探针也用得上）。
 * 推入收尾的 settleTailMs（blur 归零那段要等完，见组件内注释）产品这边本来有一条
 * 写死 GLIDE_MS 的同源修复（未提交），A41 已把它收编成随 hook 值缩放的版本 ——
 * 以 a41 为准。
 */

import { forwardRef, useCallback, useEffect, useImperativeHandle, useMemo, useRef, useState } from 'react'
import type {
  CSSProperties,
  MouseEvent as ReactMouseEvent,
  PointerEvent as ReactPointerEvent,
  ReactNode,
} from 'react'
import type { Density } from '../../hooks/useDensity'
import { useDevDefaults } from './devDefaults'
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

const RAIL_FLASH_MS = 1200

/** 退出层永远是 out：不随 phase 变，单独提出来省一次 memo */
const EXIT_INFO: DeckLayerInfo = { layer: 'exit', phase: 'out' }

/**
 * 预览卡那一层的内容身份（peek-in 与 peek-out 共用）：按**卡片层**来画
 * （缩过的白底、虚焦压暗），phase 恒为 idle —— 与它静止时一模一样，
 * 只是外层容器多一条滑入 / 滑出的动画。
 */
const PEEK_INFO: DeckLayerInfo = { layer: 'card', phase: 'idle' }

const isTyping = (t: EventTarget | null) =>
  t instanceof HTMLElement && /^(input|select|textarea)$/i.test(t.tagName)

/** 双击 / 三击时吞掉默认行为，避免浏览器把选区落到最近的可选文本上 */
const noMultiClickSelect = (e: ReactMouseEvent) => {
  if (e.detail > 1) e.preventDefault()
}


/*
 * 露出卡的出场斜坡（这里是主角）。
 *
 * 以前是开关：density ≥ compact（容器宽 ≥ 640）才渲染露出卡，一出现就是满量 20%，
 * 640 这条线上"啪"地多出一张卡。现在改成连续 —— 520 起露头，到 640 满量：
 *
 *   p(W) = clamp(0, PEEK_MAX × (W − PEEK_FROM) / (PEEK_FULL − PEEK_FROM), PEEK_MAX)
 *
 * 摊下来是「每 6px 宽度多露 1%」。两端与旧写法逐像素一致：≥640 是 0.2，≤520 是 0（不渲染）。
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

/*
 * 启动重栅格化的触发点（2026-10-07，见组件里那条 effect 的长注释）。
 * 要晚于入场动画与 WebView2 启动期的合成比例落定（page-in 130ms + 余量），
 * 又要早于用户看清首屏。500ms 两头都留了余量；真机上若仍见首启发糊，
 * 先加大这个值（比如 1000）再怀疑别的。
 */
const RERASTERIZE_AT_MS = 500

const SlideDeck = forwardRef<DeckHandle, SlideDeckProps>(function SlideDeck(
  { sheets, density, canLeave },
  ref,
) {
  /* 面板那两个值：翻页时长与热区。面板里点一下立刻生效（产品仓里是常量） */
  const { glideMs, showHit } = useDevDefaults()

  /*
   * 卡片推入时，内容（.frame）那条动画的尾段专门用来让 blur 从 2.4px 线性归零
   * （frame-settle 的 80% → 100%），遮住 Windows Chromium 合成层切换时的文字跳变。
   * JS 的 setPhase('idle') 定时器必须等到这段也播完，否则 data-phase 提前切走、
   * CSS 动画被中断，blur 瞬间跳 0 —— 改了等于没改。
   *
   * 0.2 = 1.0 − 0.8，对应 frame-settle 关键帧里 80%→100% 那一段。
   */
  const settleTailMs = Math.round(glideMs * 0.2)

  /*
   * 补位滑入的时长：与 CSS 里 `.card[data-entering='true']` 那条
   * `calc(var(--glide-ms) * 0.35)` 是同一个数 —— JS 得等它播完才摘 data-entering，
   * 摘早了会把动画掐掉（那就又回到"突然出现"）。
   *
   * 只有**跨页淡入**（fadeTo，页条拖拽 / 跨多页）走这一条：那一档没有推入过程，
   * 预览卡是换页之后才补上来的。推入（go('next')）不用它 —— 那条路上新预览卡
   * 是跟着推入**一起**滑进来的（peek-in 那一层），到换层时它已经在位了。
   */
  const peekInMs = Math.round(glideMs * 0.35)

  const [index, setIndex] = useState(0)
  const [phase, setPhase] = useState<Phase>('idle')
  const [exitSheet, setExitSheet] = useState<Sheet | null>(null)
  /*
   * 退回时右侧那张预览卡：它换页那一瞬就该"跟着这一页一起往右走"，而不是被替掉。
   * 所以把**换页前**的那一张、连同它的让位结论一起存下来，多渲染一层，让它把
   * 退场动画走完（层名 peek-out，见 SlideDeck 的 CSS 与下面那段渲染注释）。
   */
  const [exitPeek, setExitPeek] = useState<{ sheet: Sheet; peek: boolean } | null>(null)
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
  /** 平面层本体：启动重栅格化要直接摘/挂它的 will-change（见下面那条 effect） */
  const planeRef = useRef<HTMLDivElement>(null)
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

  /*
   * 启动重栅格化（2026-10-07，触发点见上面的 RERASTERIZE_AT_MS）。
   *
   * 现象（作者真机指认）：首启那一下首页文字发糊，切一次页签回来就锐了。
   * 机制：.plane 的 will-change 层把**第一份栅格化位图**一直攥在手里 —— 而首帧
   * 那一次栅格化可能落在 WebView2 启动期（合成比例/首帧时序）还没落定的时候；
   * 页签切换会把层连 visibility 一起销毁重建，重建才肯重栅格化，所以"切回来就锐"。
   * 真机实测（WebView2 CDP，DSF 1.25）：切页签前后栅格逐像素不变（同一上下文重建），
   * 而**把提升摘两帧再挂回**（本条做的事）会强制一份新位图 —— 零位移
   * （三探针 best=(0,0)、自配准 SAD=0）、纯变锐（step 行锐度 121.9 → 141.3、
   * 边缘判定像素 193 → 245）、AA 模式不变（仍灰度，通道差 5.8 不动）——
   * 等价于用户手动那一下，但没有任何位移可看。
   *
   * 时机取 RERASTERIZE_AT_MS（晚于 page-in 130ms 与启动落定、早于用户看清首屏）；
   * 真机若仍见首启发糊，先加大这个值（比如 1000）再怀疑别的。
   * 只在启动做一次；卸载时把 will-change 还回去，不留 inline 残留。
   */
  useEffect(() => {
    const el = planeRef.current
    if (!el) return
    let raf = 0
    const t = window.setTimeout(() => {
      el.style.willChange = 'auto'
      raf = requestAnimationFrame(() =>
        requestAnimationFrame(() => {
          el.style.willChange = ''
        }),
      )
    }, RERASTERIZE_AT_MS)
    return () => {
      window.clearTimeout(t)
      cancelAnimationFrame(raf)
      el.style.willChange = ''
    }
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
            busy.current = false
          }, glideMs + settleTailMs),
        )
        return
      }

      // 返回：当前页缩小、退回右边缘变成卡片；平面立刻换成上一页
      setExitSheet(sheets[index])
      /*
       * 右侧那张预览卡同样要退场：它现在是 `.card` 里装的 sheets[index+1]，
       * 换页那一瞬就会被换成"上一页"的预览 —— 不放这一层，它就是原地消失。
       * 让位结论也一起存：那是它此刻真实的让位状态，换页之后不能按新 index 重算。
       */
      const leavingPeek = sheets[index + 1]
      setExitPeek(
        leavingPeek
          ? { sheet: leavingPeek, peek: peekRef.current > 0 && index + 2 < sheets.length }
          : null,
      )
      setIndex(target)
      setPhase('out')
      timers.current.push(
        window.setTimeout(() => {
          setExitSheet(null)
          setExitPeek(null)
          setPhase('idle')
          busy.current = false
        }, glideMs),
      )
    },
    [canLeave, index, sheets, glideMs, settleTailMs],
  )

  /** 直接换页 + 交叉淡入：跨多页与页条拖动用，不走推入动画 */
  const fadeTo = useCallback(
    (target: number) => {
      if (canLeave && !canLeave(index, target)) return
      timers.current.forEach((t) => window.clearTimeout(t))
      timers.current = []
      busy.current = false
      setExitSheet(null)
      setExitPeek(null)
      setPhase('idle')
      setIndex(target)
      setEntering(true)
      timers.current.push(window.setTimeout(() => setEntering(false), peekInMs))
    },
    [canLeave, index, peekInMs],
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
   * 推入时**下下页**那张预览卡（层名 peek-in）。
   *
   * 它只在推入动画期间存在：跟着卡片一起从右缘外滑进露出位，卡片填满左边的那一刻
   * 它同时也到位 —— 所以换层时卡片层接手的是"已经在那儿"的一张卡，不用再补滑一次
   * （早先那版就是补滑，观感是"最后再单独出现一下"）。见 SlideDeck 的 peek-in 那条。
   */
  const peekInSheet = sheets[index + 2]
  const showPeekIn = phase === 'in' && peeking && peekInSheet !== undefined

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
      /* 面板与探针从这两个属性读「现在第几页」（零耦合，见 dev/panelScope） */
      data-deck-index={index}
      data-deck-total={sheets.length}
      /* 这一个数把「露多少」贯穿到全部几何：卡的平移、卡面的缩放、右侧热区宽、
         以及 CardFrame 的让位量（--chrome-peek）。四处只有这一个来源 */
      style={{ '--glide-ms': `${glideMs}ms`, '--peek-ratio': peek } as CSSProperties}
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
        ref={planeRef}
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

      {/*
        跟着推入一起滑进来的那张预览卡（下下页）：**在卡片层之上**（同 z-index，DOM 顺序说话），
        这样卡片往里滑的时候，右侧那一条缝里看到的是它，而不是卡片自己。
        推入结束（phase 翻 idle）它就卸掉 —— 那一瞬卡片层接手的正是同一张、同一位置。
      */}
      {showPeekIn && (
        <div
          key={`peek-in-${peekInSheet.id}`}
          className={s.card}
          data-layer="card"
          data-phase="idle"
          data-peek-in="true"
          data-peek={peekAt(index + 2) ? 'true' : undefined}
          aria-hidden="true"
        >
          <div className={s.face} />

          <DeckLayerContext.Provider value={PEEK_INFO}>{peekInSheet.node}</DeckLayerContext.Provider>
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

      {/*
        退场的那张预览卡：**换页前**装在 .card 里的那一张，让它把"往右滑出去"走完。
        它排在退出层**之后**（同一个 z-index 2，DOM 顺序说话）：换页那一瞬退出层是全屏的，
        盖住了整块 deck —— 不放在它上面的话，右侧那张卡还是一闪就没了。

        层身份用 data-layer='card' + data-phase='idle'：这是"静止的露出卡"那一条，
        内容照旧是虚焦 + 压暗 + 缩过的，只是外层容器多一条 peek-out 动画（见 CSS）。
      */}
      {exitPeek && peeking && (
        <div
          key={`peek-out-${exitPeek.sheet.id}`}
          className={s.card}
          data-layer="card"
          data-phase="idle"
          data-peek-out="true"
          data-peek={exitPeek.peek ? 'true' : undefined}
          aria-hidden="true"
        >
          <div className={s.face} />

          <DeckLayerContext.Provider value={PEEK_INFO}>
            {exitPeek.sheet.node}
          </DeckLayerContext.Provider>
        </div>
      )}


      {hasNext && peeking && (
        <button
          type="button"
          className={s.zoneNext}
          data-show={showHit}
          onClick={() => go('next')}
          onMouseDown={noMultiClickSelect}
          aria-label="下一页"
          title="下一页（→）"
        />
      )}

      <button
        type="button"
        className={s.zonePrev}
        data-show={showHit}
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

