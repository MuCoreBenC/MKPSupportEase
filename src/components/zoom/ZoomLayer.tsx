/*
 * **通用「浮起来看大的」放大层**（2026-10-18）。
 *
 * 从图片引导的 `ZoomView`（`app/guide/GuideImage.tsx`）抽出来的那套交互，内容泛化成
 * **任意 ReactNode**：`<img>`、canvas、SVG 整张俯视图……都行 —— FLIP 只量"盒子在哪"，
 * 与里面装的是什么无关；`--zoom-k` 写在内容根上，后代（标注框 / 线宽）自己除回去。
 *
 * 交互定式（一条不许退）：
 *   - 进/出 = **FLIP + WAAPI**：调用方在点的那一下量 `from`（First），挂上之后量内容盒
 *     （Last），`transform` 补出"从原位飞到中间长大"；收起倒放回 `returnTo()` 的现落点。
 *     ★ 必须 WAAPI：真机 WKWebView 上 CSS animation/transition 不播。
 *   - 用户缩放只写内联 `transform`，事件只记账、`requestAnimationFrame` 合帧；
 *     **以指针为不动点**；每次打开复位默认大小。
 *   - wheel 三分流（原生 `{passive:false}` 监听 —— React 的 `onWheel` 是 passive）：
 *     `ctrlKey` = 触控板捏合 ⇒ 缩放；鼠标滚轮 ⇒ 缩放；其余两指滑 ⇒ 平移。
 *   - 渐隐只动**内容与那一层底**（`::before`），不动层自己 —— 动父级 opacity 会在透明
 *     那一帧把背后整块界面透出来（就是"闪"）。
 *   - Esc **捕获阶段**抢先收这一层，不连模态框一起关。
 *
 * 放哪：调用方说了算 —— 模态框里走 `Modal` 的 `overlay` 槽（同一个遮罩、压在框之上）；
 * 不在模态框里的（报告页整页）走 `createPortal(…, document.body)`。两层都是
 * `position: fixed` 铺满视口，几何一致。
 */
import { useCallback, useEffect, useRef } from 'react'
import type { ReactNode } from 'react'
import s from './ZoomLayer.module.css'

/** 缩放范围。大图本身已放大一档，再放到 4 倍足够看清细节 */
const MIN_SCALE = 0.5
const MAX_SCALE = 4
/** 越界阻尼：再使劲也只走剩下的 35% —— 读起来是"软停"，不是"卡住不动" */
const OVERDAMP = 0.35
/** 触控板双指捏合在 WKWebView 里以 `wheel + ctrlKey` 到达；普通滚轮没有 ctrlKey。
 *  两者都用"按指数缩放"（乘性），差别只在**每像素的力度** */
const WHEEL_ZOOM_PINCH = 0.012
const WHEEL_ZOOM_PLAIN = 0.0022
/** 手势之后这一小段时间里来的点击不算"点任意处收起"
 *  （捏合 / 拖动会顺带产生一次 click，误关会把人气到） */
const GESTURE_GUARD_MS = 220
/** 拖动超过这个距离才算"这是拖动、不是点击" */
const PAN_SLOP = 3

const clampNum = (v: number, lo: number, hi: number) => Math.min(Math.max(v, lo), hi)

export default function ZoomLayer({
  children,
  from,
  returnTo,
  onGone,
  label = '收起放大视图',
  hint = '拖动挪位置 · 滚轮 / 捏合缩放 · 点任意处或 Esc 收起',
}: {
  /** 要看大的那块内容（`<img>` / canvas / SVG …… 任意节点） */
  children: ReactNode
  /** 点的那一下小图在哪（FLIP 的 First）—— 开之前现量 */
  from: DOMRect
  /** 收起时小图**现在**在哪（中间可能被布局动过，现量） */
  returnTo: () => DOMRect | null
  /** 收起那一趟放完 —— 真的卸掉这一层 */
  onGone: () => void
  /** 收起按钮的无障碍名（内容是什么就说什么） */
  label?: string
  /** 遮罩上那句提示；`''` = 不摆（原作者定的：默认其实不提示更好，能点靠光标自己说） */
  hint?: string
}) {
  const layerRef = useRef<HTMLDivElement>(null)
  const contentRef = useRef<HTMLDivElement>(null)
  const hintRef = useRef<HTMLSpanElement>(null)
  const closing = useRef(false)

  /** 缩放倍率 / 平移（只活在 ref 里：每帧改的是 DOM，不是 React 状态） */
  const scaleRef = useRef(1)
  const posRef = useRef({ x: 0, y: 0 })
  /** 内容**未缩放时**的盒子（中心 + 尺寸）—— 缩放中心和"缩回小图"都按它算 */
  const baseRef = useRef<{ cx: number; cy: number; w: number; h: number } | null>(null)
  const rafRef = useRef(0)
  /** 最近一次手势的时刻 —— 之后一小段时间里的点击不当"收起" */
  const gestureAt = useRef(0)
  const pointers = useRef(new Map<number, { x: number; y: number }>())
  const pinchRef = useRef<{ dist: number; mid: { x: number; y: number } } | null>(null)
  const panRef = useRef<{ x: number; y: number; tx: number; ty: number } | null>(null)

  /** 量"未缩放时的盒子"：进场那一趟跑完、transform 归零之后量一次 */
  const measureBase = useCallback(() => {
    const el = contentRef.current
    if (el === null) return
    const r = el.getBoundingClientRect()
    if (r.width === 0) return
    baseRef.current = { cx: r.left + r.width / 2, cy: r.top + r.height / 2, w: r.width, h: r.height }
    scaleRef.current = 1
    posRef.current = { x: 0, y: 0 }
  }, [])

  /** 把 ref 里的三个值刷到 DOM 上（同一帧多次事件只落一次） */
  const paint = useCallback(() => {
    rafRef.current = 0
    const el = contentRef.current
    if (el === null) return
    const { x, y } = posRef.current
    const k = scaleRef.current
    el.style.transform = k === 1 && x === 0 && y === 0 ? '' : `translate(${x}px, ${y}px) scale(${k})`
    /*
     * ★ 后代里"线宽 / 圆角 / 徽标"这类**视觉大小要恒定**的记号，CSS 那边拿
     *   `--zoom-k` 反着除一下（放大时框不该变成粗橡皮筋）。
     */
    el.style.setProperty('--zoom-k', String(k))
    /* 提示那句在放大之后会被内容压住/糊在一起 —— 缩放态把它收起来（不改 React 状态） */
    const hintEl = hintRef.current
    if (hintEl !== null) hintEl.style.opacity = k > 1.02 || k < 0.98 ? '0' : ''
  }, [])

  const schedule = useCallback(() => {
    if (rafRef.current !== 0) return
    rafRef.current = requestAnimationFrame(paint)
  }, [paint])

  /** 别让内容被拽出舞台：比舞台大时允许挪到边，小时只留一点点余量 */
  const clampPos = useCallback(() => {
    const base = baseRef.current
    const layer = layerRef.current
    if (base === null || layer === null) return
    const stage = layer.getBoundingClientRect()
    const k = scaleRef.current
    const limX = Math.max(0, (base.w * k - stage.width) / 2) + base.w * 0.12
    const limY = Math.max(0, (base.h * k - stage.height) / 2) + base.h * 0.12
    posRef.current = {
      x: clampNum(posRef.current.x, -limX, limX),
      y: clampNum(posRef.current.y, -limY, limY),
    }
  }, [])

  /** 缩到 `next` 倍，并把 `at`（屏幕坐标）那个点钉住不动 */
  const zoomTo = useCallback(
    (next: number, at: { x: number; y: number }) => {
      const base = baseRef.current
      if (base === null) return
      const cur = scaleRef.current
      /* 越界软停：超出范围只走剩下的那一小截（使劲也还在动），再夹一道硬边界兜底 */
      const raw = next > MAX_SCALE || next < MIN_SCALE ? cur + (next - cur) * OVERDAMP : next
      const k = clampNum(raw, MIN_SCALE * 0.9, MAX_SCALE * 1.1)
      if (Math.abs(k - cur) < 1e-4) return
      const vx = at.x - base.cx - posRef.current.x
      const vy = at.y - base.cy - posRef.current.y
      const r = k / cur
      posRef.current = { x: at.x - base.cx - vx * r, y: at.y - base.cy - vy * r }
      scaleRef.current = k
      clampPos()
      schedule()
    },
    [clampPos, schedule],
  )

  useEffect(() => {
    const el = contentRef.current
    if (el === null) return
    /* 每次打开都是默认大小（这一层是条件渲染，本来就是新实例 —— 显式复位当安全网） */
    scaleRef.current = 1
    posRef.current = { x: 0, y: 0 }
    el.style.transform = ''
    el.style.setProperty('--zoom-k', '1')

    const to = el.getBoundingClientRect()
    if (from.width === 0 || to.width === 0) {
      measureBase()
      return
    }
    const dx = from.left + from.width / 2 - (to.left + to.width / 2)
    const dy = from.top + from.height / 2 - (to.top + to.height / 2)
    const k = from.width / to.width
    const fly = el.animate(
      [
        { transform: `translate(${dx}px, ${dy}px) scale(${k})` },
        { transform: 'translate(0, 0) scale(1)' },
      ],
      { duration: 240, easing: 'cubic-bezier(.2, .8, .2, 1)' },
    )
    /* 进场那一趟跑完再"接管"：量基线，之后用户缩放全走内联 transform（两条通道不同时写） */
    void fly.finished.then(measureBase, () => undefined)
    /* 只有那一层底（`::before`，不是这一层自己）渐显 —— 见模块头注：动层自己的
       opacity 会在它透明的那一帧把背后整块界面透出来，那就是"闪" */
    const veil = layerRef.current?.animate([{ opacity: 0 }, { opacity: 1 }], {
      duration: 160,
      easing: 'ease-out',
      pseudoElement: '::before',
    })
    return () => {
      fly.cancel()
      veil?.cancel()
      if (rafRef.current !== 0) cancelAnimationFrame(rafRef.current)
      rafRef.current = 0
    }
  }, [from, measureBase])

  /* 滚轮 / 触控板捏合（原生的非 passive 监听 —— React 的 onWheel 拦不住默认行为） */
  useEffect(() => {
    const layer = layerRef.current
    if (layer === null) return
    const onWheel = (e: WheelEvent) => {
      e.preventDefault()
      gestureAt.current = performance.now()
      /*
       * ★★★ **三条入口分流**（与工作台标注台同一条判据）：
       * - `ctrlKey` = 触控板**捏合** ⇒ 缩放；
       * - **鼠标滚轮** ⇒ 缩放：整档的整数大 delta（`|dy| ≥ 100`、`deltaX` 恒 0），
       *   或 `deltaMode !== 0`（按行 / 按页滚的老鼠标）；
       * - **其余** = 触控板**两指滑** ⇒ **平移**（跟着手指走）。
       */
      const unit = e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? 400 : 1
      const dy = e.deltaY * unit
      const zoom =
        e.ctrlKey ||
        (e.deltaMode !== 0 || (e.deltaX === 0 && Number.isInteger(e.deltaY) && Math.abs(dy) >= 100))
      if (!zoom) {
        const cur = posRef.current
        posRef.current = { x: cur.x - e.deltaX, y: cur.y - dy }
        clampPos()
        schedule()
        return
      }
      const k = e.ctrlKey ? WHEEL_ZOOM_PINCH : WHEEL_ZOOM_PLAIN
      zoomTo(scaleRef.current * Math.exp(-dy * k), { x: e.clientX, y: e.clientY })
    }
    layer.addEventListener('wheel', onWheel, { passive: false })
    return () => layer.removeEventListener('wheel', onWheel)
  }, [zoomTo, clampPos, schedule])

  /* 双指捏合 + 单指拖动（触摸屏那一路；触控板在 macOS 上走 wheel） */
  useEffect(() => {
    const layer = layerRef.current
    if (layer === null) return
    const pts = pointers.current

    const pinchOf = () => {
      const [a, b] = [...pts.values()]
      if (a === undefined || b === undefined) return null
      return {
        dist: Math.hypot(a.x - b.x, a.y - b.y),
        mid: { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 },
      }
    }

    const onDown = (e: PointerEvent) => {
      pts.set(e.pointerId, { x: e.clientX, y: e.clientY })
      if (pts.size === 1) {
        panRef.current = { x: e.clientX, y: e.clientY, tx: posRef.current.x, ty: posRef.current.y }
      } else if (pts.size === 2) {
        panRef.current = null
        pinchRef.current = pinchOf()
      }
    }

    const onMove = (e: PointerEvent) => {
      if (!pts.has(e.pointerId)) return
      pts.set(e.pointerId, { x: e.clientX, y: e.clientY })

      if (pts.size >= 2) {
        const next = pinchOf()
        const prev = pinchRef.current
        if (next !== null && prev !== null && prev.dist > 0) {
          gestureAt.current = performance.now()
          zoomTo(scaleRef.current * (next.dist / prev.dist), next.mid)
        }
        pinchRef.current = next
        return
      }

      const pan = panRef.current
      if (pan === null) return
      const dx = e.clientX - pan.x
      const dy = e.clientY - pan.y
      /*
       * ★★ "刚做过手势"要按**累计位移**判，不按单次事件的 delta —— 慢慢拖时
       * 每一事件的位移都过不了 `PAN_SLOP`，松手后那次 `click` 就把整层收了。
       */
      if (Math.hypot(e.clientX - pan.tx, e.clientY - pan.ty) > PAN_SLOP) {
        gestureAt.current = performance.now()
      }
      posRef.current = { x: pan.tx + dx, y: pan.ty + dy }
      clampPos()
      schedule()
    }

    const onUp = (e: PointerEvent) => {
      pts.delete(e.pointerId)
      if (pts.size < 2) pinchRef.current = null
      if (pts.size === 0) panRef.current = null
    }

    layer.addEventListener('pointerdown', onDown)
    layer.addEventListener('pointermove', onMove)
    layer.addEventListener('pointerup', onUp)
    layer.addEventListener('pointercancel', onUp)
    return () => {
      layer.removeEventListener('pointerdown', onDown)
      layer.removeEventListener('pointermove', onMove)
      layer.removeEventListener('pointerup', onUp)
      layer.removeEventListener('pointercancel', onUp)
      pts.clear()
    }
  }, [clampPos, schedule, zoomTo])

  /** 刚做过手势吗（捏合 / 拖动 / 滚轮会顺带产生一次 click，别让它把图收了） */
  const justGestured = () => performance.now() - gestureAt.current < GESTURE_GUARD_MS

  /**
   * 收起：内容缩回小图现在的落点，**顺带自己渐隐**，放完才卸这一层。
   * 渐隐加在内容上、不加在这一层自己身上（见模块头注）；`fill: 'forwards'`
   * 让卸层那帧之前内容已经几乎不可见 —— 接缝那帧不会被看见。
   */
  const close = () => {
    if (closing.current) return
    const el = contentRef.current
    const back = returnTo()
    const base = baseRef.current
    if (el === null || back === null || back.width === 0 || base === null) {
      onGone()
      return
    }
    closing.current = true
    if (rafRef.current !== 0) {
      cancelAnimationFrame(rafRef.current)
      rafRef.current = 0
    }
    /*
     * **起点 = 现在这一帧的样子**（可能已经缩放过）：`getComputedStyle().transform` 拿的是
     * 当前生效的矩阵 ⇒ 从 identity 起会"跳一下再收"。
     * **终点按"未缩放时的盒子"算**：变换的坐标系永远是未缩放的那一套。
     */
    const now = getComputedStyle(el).transform
    const from0 = now === 'none' ? 'translate(0, 0) scale(1)' : now
    const kBack = back.width / base.w
    const dx = back.left + back.width / 2 - base.cx
    const dy = back.top + back.height / 2 - base.cy
    const fly = el.animate(
      [
        { transform: from0, opacity: 1 },
        { transform: `translate(${dx}px, ${dy}px) scale(${kBack})`, opacity: 0.06 },
      ],
      { duration: 200, easing: 'cubic-bezier(.4, 0, .6, 1)', fill: 'forwards' },
    )
    void fly.finished.then(
      () => onGone(),
      () => onGone(),
    )
  }

  /* Esc 也走这条路（**要带那个倒放**，不能直接卸）——
     模态框的 Esc 挂在 `document` 的冒泡阶段，所以这一条用**捕获阶段**
     抢在它前面，并且 `stopPropagation`：一按 Esc 先收这一层。 */
  const closeRef = useRef(close)
  closeRef.current = close
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return
      e.preventDefault()
      e.stopPropagation()
      closeRef.current()
    }
    document.addEventListener('keydown', onKey, true)
    return () => document.removeEventListener('keydown', onKey, true)
  }, [])

  return (
    <div
      ref={layerRef}
      className={s.layer}
      onMouseDown={(e) => {
        /* 只认落在这一层自己身上的那一下（从内容上拖到外面松手不算）；
           刚捏合 / 拖过的那一下也不认 —— 那是同一个手势的尾巴 */
        if (e.target === e.currentTarget && !justGestured()) close()
      }}
    >
      <button type="button" className={s.hit} onClick={() => !justGestured() && close()} aria-label={label}>
        <div ref={contentRef} className={s.content}>
          {children}
        </div>
      </button>
      {hint !== '' && (
        <span ref={hintRef} className={s.hint}>
          {hint}
        </span>
      )}
    </div>
  )
}
