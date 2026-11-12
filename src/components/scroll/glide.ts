/*
 * 「滚到某一处」的**唯一实现**（2026-11-01 定稿）。
 *
 * # 为什么不能逐帧写 `scrollTop`
 *
 * WKWebView 里脚本写滚动位置，每一帧都要主线程重新排版 + 重绘一趟才画得出来；
 * 而轮子 / 触控板是**合成器线程**在滚，压根不碰主线程。两条路的上限不一样，
 * 所以「自己滚很顺、点目录跳的那一下卡」——作者连着报了四轮，前四版都在调曲线，
 * 那是在补偿一条走不通的路（原话：「可能不是曲线的问题，是你用的函数在 Tauri 上
 * 视觉效果差」）。
 *
 * # 现在这条路：**不滚，演**
 *
 *   ① 真位置**先一步落到终点**（`box.scrollTop = to`，这一步不安动画）；
 *   ② 把内容整块 `translate3d(0, shift, 0)` **推回起点** —— `shift = 落点 - 起点`，
 *      与刚滚过去的那一段正好抵消，视口里看到的还是**起点那一屏**，一个像素都不变；
 *   ③ 让位移从 `shift` 演到 `0`（WAAPI，**合成器线程**，与原生滚动同一条路）；
 *   ④ 演完位移正好是 0，抹掉 transform 即可 —— 一帧都不跳。
 *
 * 于是"看得见的滚动"全程由合成器负责，主线程只在开头动一次布局。
 *
 * # 两个必须守住的边界
 *
 *   · **载体（carrier）**：滚动容器里**包着全部内容的那一层**。没有它就退化成
 *     瞬时滚动 —— 宁可不动画，也不许只推一部分内容，那会演成"半屏空白"的假滚动。
 *   · **`will-change` 只在演的这几百毫秒里挂**，演完立刻撤：常驻会把整块内容
 *     长久栅格化成一张大位图（本仓 `PageHome` 那条注释记过它的代价：小字丢 ClearType）。
 */

export interface GlideOptions {
  /**
   * 演多久（ms）。不给就按距离：340ms 起、**按距离加长**、1600ms 封顶。
   *
   * ★ 2026-11-01 改：封顶原来是 520ms。参数台那种十几张卡的长列表里，从顶上点
   *   最下面那一项 = 三四千像素，520ms 演完 ⇒ **速度是平常手滚的十几倍**，看着就是
   *   "嗖一下过去、眼前一花"（作者：「好像是滚到了，然后又突然闪到了别的地方」）。
   *   固定封顶等于把"距离越长越该慢"这件事反过来做：同样的曲线，越长越快。
   *   现在短腿仍是 340ms 起，长腿线性加长到 1600ms 为止 —— 长腿慢下来，才看得出过程。
   */
  duration?: number
  /** 曲线。默认 `cubic-bezier(0.33, 0.1, 0.2, 1)`：慢起、中段走完、尾巴收住 */
  easing?: string
  /** 演的过程中每帧回调 —— 跟着行跑的那些覆盖层（定位闪光）用它重摆位置 */
  onFrame?: () => void
  /** 收场（演完 / 被用户接管 / 被下一次叫停）：清掉自己那点临时状态 */
  onDone?: () => void
}

/** 对中（`center`）还是对顶（`start`） */
export type Align = 'start' | 'center'

/** 同一个滚动容器上只许有一条动画在演：新的开工时先把旧的叫停 */
const RUNNING = new WeakMap<HTMLElement, () => void>()

const DURATION_MIN = 340
/**
 * 封顶从 520 抬到 1600（2026-11-01）。
 *
 * 520 那会儿的假设是"再长只是慢、不是顺"——那是短页面里的经验。参数台一列十几张卡，
 * 一次能跨三四千像素：520ms 跑完等于每秒七八千像素，比手滚快一个数量级，过程根本
 * 看不清，而且**越长越快**（曲线固定、时间固定 ⇒ 速度正比于距离）。
 * 现在按距离线性加长、到 1600ms 封顶：长腿慢下来，短腿（<1 屏）跟以前一模一样。
 */
const DURATION_MAX = 1600
/** 每像素给多少毫秒（与解封前的斜率一致：1 屏 ≈ 400ms） */
const MS_PER_PX = 0.34

/** 距离 → 时长：短腿不磨蹭，长腿加长（封顶 1600ms） */
function durationOf(dist: number): number {
  return Math.min(DURATION_MAX, Math.max(DURATION_MIN, Math.abs(dist) * MS_PER_PX))
}

/** 这一刻**眼睛看到的**位移（读合成器上的实际值；读不出来当 0） */
export function shiftOf(el: HTMLElement): number {
  const m = getComputedStyle(el).transform
  if (m === 'none') return 0
  try {
    return new DOMMatrixReadOnly(m).m42
  } catch {
    return 0
  }
}

function reduceMotion(): boolean {
  return (
    typeof window !== 'undefined' &&
    typeof window.matchMedia === 'function' &&
    window.matchMedia('(prefers-reduced-motion: reduce)').matches
  )
}

/**
 * 滚到 `box` 的内容坐标 `to`。
 *
 * @param box     滚动容器（自己有 `overflow: auto|scroll` 的那一层）
 * @param carrier 里面**包着全部内容**的那一层；没有就传 `null`（退回瞬时）
 * @param to      目标位置（内容坐标，会被滚动范围夹住）
 * @returns 叫停函数：立刻停在"眼睛看到的位置"并把控制权交还
 */
export function glideTo(
  box: HTMLElement,
  carrier: HTMLElement | null,
  to: number,
  opts: GlideOptions = {},
): () => void {
  /* 先叫停上一条（连点第二下）—— **要在读起点之前**，它会把位置落回真实值 */
  RUNNING.get(box)?.()

  const from = box.scrollTop
  box.scrollTop = to
  /* 真落点（可能被上下限夹过）：后面所有换算都用它，不用传进来的 `to` */
  const landed = box.scrollTop
  const shift = landed - from

  const noop = () => undefined
  if (
    carrier === null ||
    Math.abs(shift) < 1 ||
    reduceMotion() ||
    typeof carrier.animate !== 'function'
  ) {
    opts.onFrame?.()
    opts.onDone?.()
    return noop
  }

  const body = carrier
  const dur = opts.duration ?? durationOf(shift)
  body.style.willChange = 'transform'
  const anim = body.animate(
    [{ transform: `translate3d(0, ${shift}px, 0)` }, { transform: 'translate3d(0, 0, 0)' }],
    { duration: dur, easing: opts.easing ?? 'cubic-bezier(0.33, 0.1, 0.2, 1)' },
  )

  /* 跟随类覆盖层：演的时候没有 scroll 事件，只能自己按帧对位置 */
  let stopped = false
  let frame = 0
  const tick = () => {
    frame = 0
    opts.onFrame?.()
    if (stopped) return
    frame = window.requestAnimationFrame(tick)
  }
  if (opts.onFrame !== undefined) frame = window.requestAnimationFrame(tick)

  const cleanup = () => {
    stopped = true
    box.removeEventListener('wheel', takeOver)
    box.removeEventListener('touchstart', takeOver)
    if (frame !== 0) window.cancelAnimationFrame(frame)
    frame = 0
    body.style.transform = ''
    body.style.willChange = ''
    RUNNING.delete(box)
  }
  const finish = () => {
    if (stopped) return
    cleanup()
    /* 落定之后再对一次位置：跟随类覆盖层要贴到最终那一行上 */
    opts.onFrame?.()
    opts.onDone?.()
  }
  /**
   * 用户中途接管（拨轮子 / 摸屏）：把**眼睛看到的位移**换算成真位置再撒手，
   * 否则他会先看到画面从终点往回跳一下。
   */
  const takeOver = () => {
    if (stopped) return
    const y = shiftOf(body)
    anim.cancel()
    cleanup()
    box.scrollTop = landed - y
    opts.onFrame?.()
    opts.onDone?.()
  }

  box.addEventListener('wheel', takeOver, { passive: true })
  box.addEventListener('touchstart', takeOver, { passive: true })
  RUNNING.set(box, takeOver)
  anim.finished.then(finish, noop)
  return takeOver
}

/**
 * 往上找第一个**真正能纵向滚**的祖先（不含 `root` 本身）。
 *
 * `overflow: auto` 不等于"它现在在滚"：对照矩阵那层 `.matrix` 也是 `overflow: auto`，
 * 但它纵向跟内容一样高、真正滚的是外面的 `.pScroll`。只看 `overflowY` 会选中
 * 那个滚不动的内层，于是「定位」只闪不滚（作者 2026-11-09 报的
 * 「点定位的时候，它没有滚动，它只高亮了」）。没有纵向余量就继续往上找。
 */
export function scrollParentOf(el: HTMLElement, root: HTMLElement | null = null): HTMLElement | null {
  for (let p = el.parentElement; p !== null && p !== root; p = p.parentElement) {
    const oy = getComputedStyle(p).overflowY
    if (oy !== 'auto' && oy !== 'scroll') continue
    /* 没有余量 ⇒ 这一层滚不动，继续往上 */
    if (p.scrollHeight - p.clientHeight > 1) return p
  }
  return null
}

/**
 * 载体里有**粘性元素**吗（表头那一类）。
 *
 * ★ 有就不许演：`translate3d` 是作用在**整棵子树**上的，粘性表头会跟着载体一起被推走
 *   —— 演的时候表头飞出去、结束再弹回来，比不动画难看得多。
 *   代价是这类容器（预设表、对比矩阵、两个 diff 弹窗）退回瞬时：**正确 > 花哨**。
 *
 * 只查"可能 sticky 的那几个位置"（载体自己 / 直接子元素 / `table.tHead` 及其首行单元格）：
 * 深挖整棵子树要几百次 `getComputedStyle`，而实践里粘性元素只有表头与头行。
 */
function hasSticky(carrier: HTMLElement): boolean {
  const probe: (Element | null)[] = [carrier, ...Array.from(carrier.children)]
  if (carrier instanceof HTMLTableElement) {
    probe.push(carrier.tHead, carrier.tHead?.rows[0]?.cells[0] ?? null)
  }
  return probe.some(
    (node) => node !== null && node instanceof HTMLElement && getComputedStyle(node).position === 'sticky',
  )
}

/**
 * 载体：滚动容器里**只有一个元素子节点**、且目标就在它里面 —— 那一层就是"全部内容"。
 * 不满足、或者里面有粘性表头，就返回 `null`（调用方退回瞬时）：
 * 绝不去猜"推一半内容"，也绝不让表头跟着飞。
 */
function carrierOf(box: HTMLElement, el: HTMLElement): HTMLElement | null {
  if (box.children.length !== 1) return null
  const only = box.firstElementChild
  if (!(only instanceof HTMLElement)) return null
  if (!only.contains(el)) return null
  return hasSticky(only) ? null : only
}

/**
 * 把一个元素滚进视口（`center` 对中 / `start` 对顶）—— 全仓"定位到某处"都走这一条。
 *
 * 不传滚动容器：自己往上找（同一页里那几层 `overflow` 各不相同，调用方拿不到类名）。
 *
 * ★ 业务侧**不直接调它**：`components/scroll/locate` 是它唯一的调用方（滚 + 落位 +
 *   绿底是一整套，拆开就会出现"有的定位不亮、有的亮法不一样"）。`offset` 是 2026-11-04
 *   为设置页左目录 / 深链那几 px 让位加的，缺省 0 ⇒ 原有判据一个字节没变。
 */
export function glideIntoView(
  el: HTMLElement,
  opts: GlideOptions & { align?: Align; offset?: number } = {},
): () => void {
  const box = scrollParentOf(el)
  /*
   * 没有可滚的祖先（内容本来就全在屏上）：**照样收场** —— 调用方的 `onDone`
   * 要拿来铺绿底。这里直接 return 的话，界面上就是"点了定位，什么都没发生"。
   */
  if (box === null) {
    opts.onFrame?.()
    opts.onDone?.()
    return () => undefined
  }

  const carrier = carrierOf(box, el)
  const boxRect = box.getBoundingClientRect()
  const elRect = el.getBoundingClientRect()
  /* rect 里含着"正在演的位移"（如果有），换算成内容坐标要扣掉它 */
  const shift = carrier === null ? 0 : shiftOf(carrier)
  const at = box.scrollTop + (elRect.top - boxRect.top) - shift
  const raw =
    (opts.align ?? 'center') === 'center'
      ? at - (box.clientHeight - elRect.height) / 2
      : at
  const room = Math.max(0, box.scrollHeight - box.clientHeight)
  const { duration, easing, onFrame, onDone, offset } = opts
  return glideTo(box, carrier, Math.min(Math.max(raw + (offset ?? 0), 0), room), {
    duration,
    easing,
    onFrame,
    onDone,
  })
}
