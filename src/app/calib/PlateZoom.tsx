/*
 * 从 v029/components/PlateZoom 整份搬来（校准板的缩放视口）。
 *
 * 10-03 作者裁决：**拖动缩放手柄整个退场**（「不要了，也不好用，还占位置了」），
 * 缩放改走两条看不见的路：
 *   - **Shift / Ctrl + 滚轮**：以光标为不动点缩放。Ctrl 那一路在触控板上就是
 *     双指捏合（系统把它翻译成 ctrlKey + wheel），所以桌面触控板不用另做手势；
 *   - **触摸屏双指开合**：pointer events 记两根手指，按「中点不动 + 间距比例」
 *     重算倍率与位移，中点走动连带平移。
 * 空白处按住拖 = 平移（只在放大后有效）；双击复位；板上的格子永远先响应点选。
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

import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type CSSProperties,
  type MouseEvent as ReactMouseEvent,
  type PointerEvent as ReactPointerEvent,
  type ReactNode,
} from 'react'
import s from './PlateZoom.module.css'

/** 最大倍率。再大刻度间距就超过一屏，看不出更多东西 */
const MAX = 6

/** 判定"这是拖动不是点击"的位移。小于它仍是一次点击 */
const THRESHOLD = 4

/** 滚轮一格 / 手指捏合的映射基准：多少像素换一个 e 倍（≈×2.72） */
const SPAN = 240

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

/** 热区：点它是"选一格"，绝不能被手势截走 */
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
  /** 板名。给了非空标题才渲染标题行（手柄已退场，空串 = 整行都没有） */
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
  const [drag, setDrag] = useState<null | 'pan' | 'pinch'>(null)
  const gesture = useRef<Drag | null>(null)
  /** 双指捏合：两根手指的实时位置（down 起 rec，up / cancel 删）。鼠标只有一根指针，进不来 */
  const pointers = useRef(new Map<number, { x: number; y: number }>())
  /** 捏合起始量：两指间距、中点（相对视口中心）与当时的视图。中点不动 + 间距定倍率 */
  const pinch = useRef<{ d0: number; m0: { x: number; y: number }; view0: View } | null>(null)
  /** 拖过 / 捏过之后那一次 click 要吃掉，否则松手顺手点亮一格 */
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

  /**
   * 以一个不动点缩放（滚轮与捏合共用）。ax / ay 是**视口内**坐标：
   * 缩放前后"压在不动点下面的那个内容点"必须是同一个 —— 解出来就是
   * `x' = a − (a − x)·ratio`（a = 不动点相对视口中心，ratio = 新旧倍率之比）。
   * 不动点取光标（滚轮）或两指中点（捏合），比"永远围着中心缩"更合手感。
   */
  const zoomAt = useCallback(
    (ax: number, ay: number, factor: number) => {
      const vp = vpRef.current
      if (!vp || !base) return
      const r = vp.getBoundingClientRect()
      setView((v) => {
        const k = clamp(v.k * factor, 1, MAX)
        const ratio = k / v.k
        const cx = ax - r.width / 2
        const cy = ay - r.height / 2
        return clampPan(
          { k, x: cx - (cx - v.x) * ratio, y: cy - (cy - v.y) * ratio },
          r.width,
          r.height,
          base.w,
          base.h,
        )
      })
    },
    [base, clampPan],
  )

  /*
   * Shift / Ctrl + 滚轮 = 缩放。Ctrl 那一路在触控板上就是双指捏合 —— 系统把捏合
   * 翻译成 ctrlKey + wheel 上来，preventDefault 掉浏览器自己的页面缩放，板子接手。
   * React 的 onWheel 是被动监听（preventDefault 不生效），这里手动挂非被动监听。
   * Shift + 滚轮在部分系统上把增量放进 deltaX（横向滚轮位），deltaY 空了就取它。
   */
  useEffect(() => {
    const vp = vpRef.current
    if (!vp) return
    const onWheel = (e: WheelEvent) => {
      if (!e.shiftKey && !e.ctrlKey) return
      e.preventDefault()
      const r = vp.getBoundingClientRect()
      const delta = e.deltaY !== 0 ? e.deltaY : e.deltaX
      zoomAt(e.clientX - r.left, e.clientY - r.top, Math.exp(-delta / SPAN))
    }
    vp.addEventListener('wheel', onWheel, { passive: false })
    return () => vp.removeEventListener('wheel', onWheel)
  }, [zoomAt])

  const beginPan = (e: ReactPointerEvent<HTMLElement>) => {
    const vp = vpRef.current
    if (!vp || !base) return
    const r = vp.getBoundingClientRect()
    gesture.current = {
      id: e.pointerId,
      sx: e.clientX,
      sy: e.clientY,
      from: view,
      /*
       * 视口的实测高度。
       * 非 fill：放大期间高度被钉成 base.h，×1 时也等于 base.h，两种情况都与实测一致。
       * fill：高度由 flex 分配，可能远高于板子自己（Z 板很扁），所以必须用实测值 ——
       *       用 base.h 会把可拖范围按板子高度算，纵向拖到一半就被夹住。
       */
      vh: r.height,
      vw: r.width,
      bw: base.w,
      bh: base.h,
      moved: false,
    }
    e.currentTarget.setPointerCapture(e.pointerId)
  }

  const onViewportDown = (e: ReactPointerEvent<HTMLDivElement>) => {
    if (e.button !== 0) return
    const vp = vpRef.current
    if (!vp) return

    /*
     * 新手势开始就清掉上一场的 swallow：它要吞的那次 click 在 up 与这次 down 之间
     * 就该来（来了已被捕获阶段消费掉）；捏合之后浏览器通常**不发** click，
     * 标记留着就成了"下一次点格子没反应"的哑弹 —— 10-03 冒烟抓到的。
     */
    swallow.current = false

    /*
     * 触摸：记下每根手指，第二根落下就进捏合（中点 + 间距，见 onMove）。
     * 单指不在 touch 上开平移 —— 放大后的挪动交给捏合的中点位移，单指留给点格子。
     * 第二根手指才 setPointerCapture：第一根不拦，格子上的轻点照常收到 click；
     * 进了捏合再拦，两根手指的后续事件都归视口，不会漏到格子上去。
     */
    if (e.pointerType === 'touch') {
      pointers.current.set(e.pointerId, { x: e.clientX, y: e.clientY })
      if (pointers.current.size === 2) {
        const [p1, p2] = [...pointers.current.values()]
        const r = vp.getBoundingClientRect()
        pinch.current = {
          d0: Math.hypot(p2.x - p1.x, p2.y - p1.y),
          m0: {
            x: (p1.x + p2.x) / 2 - r.left - r.width / 2,
            y: (p1.y + p2.y) / 2 - r.top - r.height / 2,
          },
          view0: view,
        }
        gesture.current = null
        try {
          vp.setPointerCapture(e.pointerId)
        } catch {
          /* 指针可能已经离场：捏合靠 map 里的坐标照算，捕获丢了只是拖出视口收不到 */
        }
        setDrag('pinch')
      }
      return
    }

    /* 热区不进手势：点格子这件事优先，拖动只发生在空白处 */
    if ((e.target as Element).closest(HIT)) return
    if (!e.isPrimary) return
    beginPan(e)
    setDrag('pan')
  }

  const onMove = (e: ReactPointerEvent<HTMLElement>) => {
    if (pointers.current.has(e.pointerId)) {
      pointers.current.set(e.pointerId, { x: e.clientX, y: e.clientY })
    }

    const pin = pinch.current
    if (pin) {
      const vp = vpRef.current
      if (!vp || !base || pointers.current.size < 2) return
      const [p1, p2] = [...pointers.current.values()]
      const d = Math.hypot(p2.x - p1.x, p2.y - p1.y)
      if (d <= 0) return
      const r = vp.getBoundingClientRect()
      const k = clamp(pin.view0.k * (d / pin.d0), 1, MAX)
      const kr = k / pin.view0.k
      const m1 = {
        x: (p1.x + p2.x) / 2 - r.left - r.width / 2,
        y: (p1.y + p2.y) / 2 - r.top - r.height / 2,
      }
      /* 起始中点压着的内容点，现在要压在当前中点下：位移 = 中点走动 + 倍率放大两段 */
      setView(
        clampPan(
          {
            k,
            x: m1.x - (pin.m0.x - pin.view0.x) * kr,
            y: m1.y - (pin.m0.y - pin.view0.y) * kr,
          },
          r.width,
          r.height,
          base.w,
          base.h,
        ),
      )
      swallow.current = true
      return
    }

    const g = gesture.current
    if (!g || g.id !== e.pointerId) return
    const dx = e.clientX - g.sx
    const dy = e.clientY - g.sy
    if (!g.moved) {
      if (Math.hypot(dx, dy) < THRESHOLD) return
      g.moved = true
    }

    /*
     * 平移：位移就是手指的位移，夹回可拖范围即可（居中原点，见 clampPan）。
     */
    setView(clampPan({ k: g.from.k, x: g.from.x + dx, y: g.from.y + dy }, g.vw, g.vh, g.bw, g.bh))
  }

  const onUp = (e: ReactPointerEvent<HTMLElement>) => {
    pointers.current.delete(e.pointerId)

    /* 捏合里抬起一根：捏合结束，剩下的那一根不接棒（点格子要重新落） */
    if (pinch.current && pointers.current.size < 2) {
      pinch.current = null
      gesture.current = null
      setDrag(null)
      return
    }

    const g = gesture.current
    if (!g || g.id !== e.pointerId) return
    if (g.moved) swallow.current = true
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

  return (
    <div className={s.box} data-fill={fill ? 'true' : undefined}>
      {title ? (
        <div className={s.head}>
          <h3 className={s.title}>{title}</h3>
        </div>
      ) : null}

      <div
        ref={vpRef}
        className={s.viewport}
        data-fill={fill ? 'true' : undefined}
        data-zoomed={zoomed ? 'true' : undefined}
        data-dragging={drag ? 'true' : undefined}
        /*
         * 非 fill：放大期间把高度钉在 ×1 时的值 —— 内层真的变高，不钉住板框会被顶开。
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
