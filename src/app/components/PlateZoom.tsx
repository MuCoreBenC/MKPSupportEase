import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent as ReactKeyboardEvent,
  type MouseEvent as ReactMouseEvent,
  type PointerEvent as ReactPointerEvent,
  type ReactNode,
} from 'react'
import s from './PlateZoom.module.css'

/** 最大倍率。再大刻度间距就超过一屏，看不出更多东西 */
const MAX = 6

/** 判定"这是拖动不是点击"的位移。小于它仍是一次点击 */
const THRESHOLD = 4

/** 手柄横拖多少像素换一个 e 倍（≈×2.72） */
const SPAN = 240

/** 键盘一下调多少（手柄可聚焦，方向键也能改倍率） */
const KEY_STEP = 1.15

/**
 * 放大之后额外允许拖出视口的比例（视口对应边长的 35%）。
 * 边缘卡得严丝合缝会让人觉得"拖不动"，多给一段就随手了；剩下 65% 仍在画面里，拖不丢。
 */
const SLACK = 0.35

interface View {
  k: number
  x: number
  y: number
}

const REST: View = { k: 1, x: 0, y: 0 }

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v))

/** 热区：点它是"选一格"，绝不能被拖动手势截走 */
const HIT = '[data-hit="true"], [role="button"]'

interface Drag {
  id: number
  sx: number
  sy: number
  from: View
  /** 手势开始时的视口尺寸与基准尺寸，手势全程不再量 */
  vw: number
  vh: number
  bw: number
  bh: number
  moved: boolean
}

export interface PlateZoomProps {
  /** 板名。这一行由本组件渲染 —— 缩放手柄要和它并排 */
  title?: ReactNode
  /** 这个值一变就复位（换步 / 换板不该带着 ×4 进下一页） */
  resetKey?: string | number
  /**
   * 撑满外层剩下的高度，并让板子按"容器还剩多少"连续算尺寸（见 CSS 里的 data-fill 那一组）。
   *
   * 按需开启：向导那两页的板子尺寸是一整套调死的公式（`--plate-w` / `--xy-plate`）、
   * 还牵着卡片翻页的落位动画，动它就是动那一轮的成果。所以只有「校准」tab 传 fill。
   */
  fill?: boolean
  /** 这块板的毫米尺寸。fill 模式用它换算 px/mm —— 是模型的物理尺寸，不是版面常数 */
  mm?: { w: number; h: number }
  /**
   * fill 模式下板子最长边的天花板，默认 1200px（CSS 那边的 --plate-cap）。
   *
   * 上限本身是相对量 `min(100cqw, cap)`：容器没那么宽时它不起作用、板子吃满，
   * 只有超过这个值才收住 —— Ultra 那一档不会长成横贯整页的一条。
   */
  cap?: number
  children: ReactNode
}

/**
 * 校准板的缩放视口。
 *
 * 三条互不重叠的规则：
 *
 * - **标题行右端的手柄**：按住左右拖 = 放大 / 缩小（右拖放大），双击或按 Esc 复位，
 *   方向键也能调。缩放的入口只有这一个 —— 板子本体不带缩放手势，免得和点格子抢。
 * - **板子上的空白**：按住拖 = 平移。只在真的放大过之后才有效果（否则位移被夹回 0）。
 * - **热区**：`pointerdown` 直接放行，不进任何手势。点选的手感与没有这个组件时一模一样。
 *
 * ## 为什么不用 transform: scale()
 *
 * `scale()` 既不重排也不重绘：浏览器把这一层按**当前（×1）分辨率**栅格化成位图，
 * 再把位图拉大 —— 放大后的字是"放大的像素"，所以发虚；改一下窗口尺寸让层失效、
 * 重新栅格化，就又清晰了（这正是肉眼看到的现象）。
 *
 * 所以这里是**真的把 SVG 变大**：放大期间给内层一个 `width = 基准宽 × 倍率`，
 * 里面那块板跟着层宽走，SVG 按新尺寸重新绘制，第一帧就是清晰的矢量。
 * 位移仍用 `translate` —— 纯位移不涉及重采样，不会发虚。
 *
 * 代价是放大时要真重排重绘（板子是两三百个静态 path，实测可以接受），
 * 换来的是"放大之后能读"这件事本身成立。
 */
export default function PlateZoom({
  title,
  resetKey,
  fill,
  mm,
  cap = 1200,
  children,
}: PlateZoomProps) {
  const vpRef = useRef<HTMLDivElement>(null)
  const layerRef = useRef<HTMLDivElement>(null)
  const [view, setView] = useState<View>(REST)
  /** ×1 时量下来的自然尺寸。放大期间不更新 —— 那时候量到的是放大后的值 */
  const [base, setBase] = useState<{ w: number; h: number } | null>(null)
  const [drag, setDrag] = useState<null | 'zoom' | 'pan'>(null)
  const gesture = useRef<(Drag & { mode: 'zoom' | 'pan' }) | null>(null)
  /** 拖过之后那一次 click 要吃掉，否则松手顺手点亮一格 */
  const swallow = useRef(false)

  const kRef = useRef(view.k)
  kRef.current = view.k

  useEffect(() => {
    setView(REST)
  }, [resetKey])

  /*
   * 只在 ×1 时量基准：窗口改尺寸、密度换档、板子换一块都会走到这里。
   *
   * 量的是**板子本身**（层里那唯一一个子元素），不是层。
   * 层是块级元素、宽度总是 100%；而板子自己是 `min(100%, mm × scale)` ——
   * XY 板在宽窗里只有 535px、比层窄得多。拿层宽当基准，第一格放大就会从 535 跳到整列宽。
   */
  useEffect(() => {
    const layer = layerRef.current
    if (!layer) return
    const el = layer.firstElementChild
    if (!el) return
    const measure = () => {
      if (kRef.current !== 1) return
      const r = el.getBoundingClientRect()
      if (r.width > 0 && r.height > 0) setBase({ w: r.width, h: r.height })
    }
    measure()
    const ro = new ResizeObserver(measure)
    ro.observe(el)
    return () => ro.disconnect()
  }, [])

  /**
   * 把位移夹回可拖范围。
   *
   * **原点是"居中"，不是左上角**：视口是 flex 居中（`align-items / justify-content: center`），
   * 层比视口大时两侧等量溢出 —— 所以位移 0 等于居中，可拖的余量是 `(内容 − 视口) / 2`，
   * 区间对称。之前按左上角原点写成 `[-over, 0]`，上界那个 0 正好把"往右 / 往下"整个卡死，
   * 于是放大之后左边和上边怎么拖都不动。
   *
   * 再外加一段 SLACK：放大了就是想随便拖，边缘严丝合缝地卡住反而别扭。
   * 只在 k > 1 时给 —— ×1 时没放大，位移恒为 0（空白处拖动没反应，这条规则不变）。
   */
  const clampPan = useCallback((v: View, vw: number, vh: number, bw: number, bh: number): View => {
    const slackX = v.k > 1 ? vw * SLACK : 0
    const slackY = v.k > 1 ? vh * SLACK : 0
    const halfX = Math.max(0, (bw * v.k - vw) / 2) + slackX
    const halfY = Math.max(0, (bh * v.k - vh) / 2) + slackY
    return { k: v.k, x: clamp(v.x, -halfX, halfX), y: clamp(v.y, -halfY, halfY) }
  }, [])

  const reset = useCallback(() => setView(REST), [])

  const begin = (e: ReactPointerEvent<HTMLElement>, mode: 'zoom' | 'pan') => {
    const vp = vpRef.current
    if (!vp || !base) return
    const r = vp.getBoundingClientRect()
    gesture.current = {
      id: e.pointerId,
      sx: e.clientX,
      sy: e.clientY,
      from: view,
      vw: r.width,
      /*
       * 视口的实测高度。
       * 非 fill：放大期间高度被钉成 base.h，×1 时也等于 base.h，两种情况都与实测一致。
       * fill：高度由 flex 分配，可能远高于板子自己（Z 板很扁），所以必须用实测值 ——
       *       用 base.h 会把可拖范围按板子高度算，纵向拖到一半就被夹住。
       */
      vh: r.height,
      bw: base.w,
      bh: base.h,
      moved: false,
      mode,
    }
    e.currentTarget.setPointerCapture(e.pointerId)
  }

  const onHandleDown = (e: ReactPointerEvent<HTMLButtonElement>) => {
    if (e.button !== 0 || !e.isPrimary) return
    begin(e, 'zoom')
    setDrag('zoom')
  }

  const onViewportDown = (e: ReactPointerEvent<HTMLDivElement>) => {
    if (e.button !== 0 || !e.isPrimary) return
    /* 热区不进手势：点格子这件事优先，拖动只发生在空白处 */
    if ((e.target as Element).closest(HIT)) return
    begin(e, 'pan')
    setDrag('pan')
  }

  const onMove = (e: ReactPointerEvent<HTMLElement>) => {
    const g = gesture.current
    if (!g || g.id !== e.pointerId) return
    const dx = e.clientX - g.sx
    const dy = e.clientY - g.sy
    if (!g.moved) {
      if (Math.hypot(dx, dy) < THRESHOLD) return
      g.moved = true
    }

    if (g.mode === 'zoom') {
      const k = clamp(g.from.k * Math.exp(dx / SPAN), 1, MAX)
      /*
       * 保持视口中心那一点不动。居中布局下这就是"位移按倍率等比放大"一行：
       * 位移 0 = 居中，内容围着自己的中心长大，所以中心那一点本来就不动。
       * （左上角原点的那套 `cx - (cx - x) * ratio` 在这里是错的算法。）
       */
      const ratio = k / g.from.k
      setView(clampPan({ k, x: g.from.x * ratio, y: g.from.y * ratio }, g.vw, g.vh, g.bw, g.bh))
    } else {
      setView(
        clampPan({ k: g.from.k, x: g.from.x + dx, y: g.from.y + dy }, g.vw, g.vh, g.bw, g.bh),
      )
    }
  }

  const onUp = (e: ReactPointerEvent<HTMLElement>) => {
    const g = gesture.current
    if (!g || g.id !== e.pointerId) return
    if (g.moved && g.mode === 'pan') swallow.current = true
    gesture.current = null
    setDrag(null)
  }

  /* 捕获阶段拦：热区在冒泡阶段才收到 click，这里 stop 掉它就收不到 */
  const onClickCapture = (e: ReactMouseEvent<HTMLDivElement>) => {
    if (!swallow.current) return
    swallow.current = false
    e.stopPropagation()
    e.preventDefault()
  }

  const zoomed = view.k > 1

  /** 手柄的键盘通道：方向键调倍率、Esc / Home 复位 */
  const onHandleKey = (e: ReactKeyboardEvent) => {
    const vp = vpRef.current
    if (!vp || !base) return
    const step = e.key === 'ArrowRight' || e.key === 'ArrowUp' ? KEY_STEP : 1 / KEY_STEP
    if (['ArrowRight', 'ArrowUp', 'ArrowLeft', 'ArrowDown'].includes(e.key)) {
      e.preventDefault()
      const k = clamp(view.k * step, 1, MAX)
      const r = vp.getBoundingClientRect()
      const ratio = k / view.k
      /* 与手柄拖动同一个算法：居中布局下位移等比缩放即可（见 onMove 里的注释） */
      setView(
        clampPan({ k, x: view.x * ratio, y: view.y * ratio }, r.width, r.height, base.w, base.h),
      )
      return
    }
    if (e.key === 'Escape' || e.key === 'Home') {
      e.preventDefault()
      reset()
    }
  }

  return (
    <div className={s.box} data-fill={fill ? 'true' : undefined}>
      <div className={s.head}>
        {title !== undefined && <h3 className={s.title}>{title}</h3>}
        <button
          type="button"
          className={s.handle}
          data-on={zoomed ? 'true' : undefined}
          data-active={drag === 'zoom' ? 'true' : undefined}
          title="按住左右拖动缩放，双击复位"
          aria-label={`缩放，当前 ${view.k.toFixed(1)} 倍`}
          onPointerDown={onHandleDown}
          onPointerMove={onMove}
          onPointerUp={onUp}
          onPointerCancel={onUp}
          onDoubleClick={reset}
          onKeyDown={onHandleKey}
        >
          <span className={s.arrow} aria-hidden="true">
            ⇔
          </span>
          {zoomed ? `×${view.k.toFixed(1)} · 双击复位` : '拖动缩放'}
        </button>
      </div>

      <div
        ref={vpRef}
        className={s.viewport}
        data-fill={fill ? 'true' : undefined}
        data-zoomed={zoomed ? 'true' : undefined}
        data-dragging={drag === 'pan' ? 'true' : undefined}
        /*
         * 非 fill（向导）：放大期间把高度钉在 ×1 时的值 —— 内层真的变高，不钉住板框会被顶开。
         * fill：高度本来就由布局给（撑满外层剩余高度），不用也不能钉。
         * mm 两个数交给 CSS 换算 px/mm，见 .viewport[data-fill] .layer。
         */
        style={{
          ...(fill ? undefined : zoomed && base ? { height: base.h } : undefined),
          ...(fill && mm
            ? ({
                '--plate-mm-w': mm.w,
                '--plate-mm-h': mm.h,
                '--plate-cap': `${cap}px`,
              } as CSSProperties)
            : undefined),
        }}
        onPointerDown={onViewportDown}
        onPointerMove={onMove}
        onPointerUp={onUp}
        onPointerCancel={onUp}
        onClickCapture={onClickCapture}
        onDoubleClick={reset}
      >
        <div
          ref={layerRef}
          className={s.layer}
          data-wide={zoomed && base ? 'true' : undefined}
          style={{
            width: zoomed && base ? base.w * view.k : undefined,
            transform: view.x || view.y ? `translate(${view.x}px, ${view.y}px)` : undefined,
          }}
        >
          {children}
        </div>
      </div>
    </div>
  )
}
