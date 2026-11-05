/*
 * 「滚到某一处 + 把那一行指出来」的**唯一出口**（2026-11-04 定稿）。
 *
 * # 为什么要收成一件
 *
 * 之前这套东西有三份，各写各的：
 *
 *   · 设置页深链   `glideTo` + React state `flash` + `.field[data-flash]` 的**绿描边圈**
 *                  + 1.4 秒的定时器；
 *   · 预设表定位   `glideIntoView` + `data-flash` in/out 两段 + CSS transition（1.78 秒）；
 *   · 工作台定位   `glideIntoView` + `onDone` 落位补测 + 两条 `@keyframes`（1.8 秒）。
 *
 * 于是同一个动作三时长相、三个时长，作者 2026-11-04 一次点名三处：
 * 「消失得太快了」「能不能是绿色背景，而不是这种圈起来的呀」「以后我还有什么需求，
 * 只要是滚动定位，大家都用一样的」。
 *
 * 现在**业务侧只传参数**：滚过去、落位补测、绿底、停留、淡出全在这里。
 * `glide.ts` 仍是滚动的唯一实现，本文件是它唯一的调用方（由 `check-locate.mjs` 与
 * ESLint 一起钉住，见那两处的注释）。
 *
 * # 绿底为什么用 WAAPI 演，不写 CSS 规则
 *
 * 作者要的是「行自己的底色，像表格里被选中的那一行」，不是浮层、不是描边。
 * 行底色这一层在仓里被好几个更高特异性的规则占着（`.row:hover` /
 * `[data-selected]` / `[data-pinned]` / 工作台卡片的 `background`）；
 * 一条全局 `[data-locate]` 规则压不过它们，加 `!important` 又会盖掉悬停语义。
 *
 * WAAPI 的动画值在级联里**优先于普通声明**，正好：不占布局、不改 class、不改结构，
 * 演完自动把底色还给 CSS（悬停/选中态一个不丢）。也顺带绕开了 WKWebView 不播
 * CSS `@keyframes` 那条老坑（见 `MEMORY`）。颜色取主题 token `--accent-soft`
 * —— 浅色深色、客户端工作台两档都认它，与 `.row[data-selected]` 同一套绿。
 *
 * # 时长口径（2026-11-04 作者裁定）
 *
 * **固定停 6 秒再淡出**，不依赖点击/滚动等用户动作。淡入 160ms、淡出 620ms。
 * 期间只重绘背景色，下面几行纹丝不动。
 *
 * # PageSlot 是常驻挂着的
 *
 * 切页签只是取消 `hidden`，落点到达的那一刻目标元素还是 `display:none`
 * （`getBoundingClientRect()` 全是 0）。所以 `locateAnchor` 先按帧等它真的在屏上，
 * 最多 60 帧；等不到就什么都不做 —— 宁可不动，不许滚错位置。
 */

import { glideIntoView, scrollParentOf, type GlideOptions } from './glide'

/** 绿底停留多久（作者：「固定停约 6 秒淡出」） */
const HOLD_MS = 6000
/** 淡入 / 淡出 */
const FADE_IN_MS = 160
const FADE_OUT_MS = 620
/** 等目标真的在屏上，最多几帧 */
const WAIT_FRAMES = 60
/** 落位补测的容差（px）与最多几帧 —— 原来只在工作台那份 locate.ts 里 */
const SNAP_TOL = 2
const SNAP_TRIES = 4
/** token 读不到时的兜底（与预设表原来那层绿同色） */
const FALLBACK_GREEN = 'rgb(32 178 90 / 20%)'

/** 同一元素上一次没演完的高亮（连着定位两下时先叫停） */
const RUNNING = new WeakMap<HTMLElement, () => void>()

/** 这一档主题里"淡绿底"的真值（读目标元素自己的计算值，主题挂在哪一层都对） */
function greenOf(el: HTMLElement): string {
  const v = getComputedStyle(el).getPropertyValue('--accent-soft').trim()
  return v === '' ? FALLBACK_GREEN : v
}

/**
 * 给目标元素铺一层绿底：淡入 → 停住 → 淡出 → 还给 CSS。
 *
 * 没有 `element.animate`（极老 WebView）时退回"挂上、到点摘掉"，一样是背景色，
 * 一样不画圈。两条路都返回叫停函数。
 */
function paint(el: HTMLElement, holdMs: number): () => void {
  RUNNING.get(el)?.()
  const green = greenOf(el)
  /* 起止都用这一行**本来的**背景色：悬停 / 置顶 / 选中那些行底色在第一帧和收场
     都不会闪一下，演完把底色原样还给 CSS。 */
  const base = getComputedStyle(el).backgroundColor

  if (typeof el.animate !== 'function') {
    el.style.backgroundColor = green
    const t = window.setTimeout(() => {
      el.style.backgroundColor = ''
      RUNNING.delete(el)
    }, holdMs)
    const stop = () => {
      window.clearTimeout(t)
      el.style.backgroundColor = ''
      RUNNING.delete(el)
    }
    RUNNING.set(el, stop)
    return stop
  }

  const total = FADE_IN_MS + holdMs + FADE_OUT_MS
  /*
   * ★ easing 必须**按段给**、总时长用 linear：WAAPI 的 `easing` 选项是整条时间轴
   *   的缓动，会把 0.024→0.909 这段"绿底停住"也一起压弯 —— 实测 6 秒那一刻已经
   *   褪到 3% 透明度了（作者要的是"停满 6 秒再淡出"）。
   *   逐帧给 easing 只作用于它后面那一段，停留段保持 linear ⇒ 一秒不少。
   */
  const anim = el.animate(
    [
      { backgroundColor: base, offset: 0, easing: 'ease-in-out' },
      { backgroundColor: green, offset: FADE_IN_MS / total, easing: 'linear' },
      {
        backgroundColor: green,
        offset: (FADE_IN_MS + holdMs) / total,
        easing: 'ease-in-out',
      },
      { backgroundColor: base, offset: 1 },
    ],
    { duration: total, easing: 'linear' },
  )
  const stop = () => {
    anim.cancel()
    RUNNING.delete(el)
  }
  RUNNING.set(el, stop)
  /* 演完把登记撤掉；被叫停时 finished 会 reject，这里一并吞掉（不是故障） */
  void anim.finished.then(
    () => {
      if (RUNNING.get(el) === stop) RUNNING.delete(el)
    },
    () => undefined,
  )
  return stop
}

/** 现在把 `el` 居中时，滚动容器该在哪个位置（瞬时算，不写） */
function centerTopOf(el: HTMLElement): { box: HTMLElement; top: number } | null {
  const box = scrollParentOf(el)
  if (box === null) return null
  const at = box.scrollTop + el.getBoundingClientRect().top - box.getBoundingClientRect().top
  const room = Math.max(0, box.scrollHeight - box.clientHeight)
  const raw = at - (box.clientHeight - el.offsetHeight) / 2
  return { box, top: Math.min(Math.max(raw, 0), room) }
}

/**
 * 收场补位：演完之后再量一次"该在哪"，对不上就直接落过去（不做第二段动画）。
 *
 * 隔一帧再量是因为：演的时候内容整块挂着 `translate3d`，收场同一帧里布局还没按
 * "没有 transform"重算完；`content-visibility` 的行也是这一帧之后才渲染出来的。
 * 原来这段只服务工作台，现在是所有居中型定位共用的收尾。
 */
function settle(el: HTMLElement, tries: number, alive: () => boolean): void {
  if (tries <= 0 || !alive() || !el.isConnected) return
  const want = centerTopOf(el)
  if (want === null) return
  const diff = want.top - want.box.scrollTop
  if (Math.abs(diff) <= SNAP_TOL) return
  want.box.scrollTop = want.top
  window.requestAnimationFrame(() => settle(el, tries - 1, alive))
}

export interface LocateOptions
  extends Pick<GlideOptions, 'duration' | 'easing' | 'onFrame' | 'onDone'> {
  /** 对中（默认）还是对顶 */
  align?: 'center' | 'start'
  /** 对顶时再往上让几 px（设置页左目录 -8 / 深链 -28） */
  offset?: number
  /** 绿底停留时长，默认 6000ms */
  holdMs?: number
  /** 只滚不闪（差弹窗里跳到某一行、左目录点章节） */
  flash?: boolean
  /**
   * 历史参数：早先卡形态（false）用描边 + 底色、行形态（true）只铺底色。
   * 2026-11-04 起**两种都只铺绿底**（作者：「不是这种圈起来的」），保留这个字段
   * 只是让工作台那几个调用点不必逐个改签名；它不再影响外观。
   */
  row?: boolean
}

/**
 * 把已在 DOM 里的元素滚进视口并铺绿底。返回叫停函数（卸载 / 连点第二下时用）。
 */
export function locateIntoView(el: HTMLElement, opts: LocateOptions = {}): () => void {
  const {
    align = 'center',
    offset = 0,
    holdMs = HOLD_MS,
    flash = true,
    onDone,
    row: _row,
    ...rest
  } = opts
  void _row
  let stopped = false
  let stopFlash: () => void = () => undefined

  const cancel = glideIntoView(el, {
    ...rest,
    align,
    offset,
    onDone: () => {
      if (stopped) return
      if (align === 'center') settle(el, SNAP_TRIES, () => !stopped)
      if (flash) stopFlash = paint(el, holdMs)
      onDone?.()
    },
  })

  return () => {
    if (stopped) return
    stopped = true
    cancel()
    stopFlash()
  }
}

/**
 * 按锚点 id 定位。**先等它真的在屏上**（`PageSlot` 常驻 `hidden`），再滚 + 铺绿底。
 * 找不到元素 / 等不到就什么都不做 —— 目标项可能在某台机型某一版上不存在，那不是错误。
 */
export function locateAnchor(id: string, opts: LocateOptions = {}): () => void {
  let raf = 0
  let tries = 0
  let stopped = false
  let stop: () => void = () => undefined

  const run = () => {
    if (stopped) return
    const el = document.getElementById(id)
    if (el === null || el.offsetParent === null) {
      if (tries++ < WAIT_FRAMES) raf = window.requestAnimationFrame(run)
      return
    }
    stop = locateIntoView(el, opts)
  }
  raf = window.requestAnimationFrame(run)

  return () => {
    if (stopped) return
    stopped = true
    window.cancelAnimationFrame(raf)
    stop()
  }
}
