/*
 * **图片引导**（2026-10-16 作者提的）—— 一张**系统窗口的截图** + 图上几块**框选**（带编号）
 * + 几步文字，装在模态框里。给"这一步得照着做、但那扇窗不在我们手里"的地方用。
 *
 * # 与 `GuideCard` 是两件事，别混（都叫"引导"）
 *
 *   `GuideCard`（屏幕引导）  贴到**切片器那扇活窗口**旁边，箭头指着控件，
 *                            位置由 AX 跟着窗口走（`ipc/guide.rs`）—— 用户**当场**照着点
 *   这里（图片引导）          拿一张**截图**讲步骤：对象是系统窗口（访达的「显示简介」
 *                            那种），我们开得出来、摆不了卡，只能给张图
 *
 * 两件事共用一个词是作者的话（「后续很多步骤要不要也用图片去引导用户呢」），
 * 所以名字里都带"引导"，靠前缀分（屏幕 / 图片）。
 *
 * # 作者对它的要求（2026-10-16）
 *
 * 「到时候不知道是模态框，还是浮窗，还是什么，功能试一下，到时候效果好的话，其他的引导
 * 都可以复用。效果不好，到时候再改」＋「千万不要做复杂」。
 * ⇒ 所以这个文件只有三样东西：**标注的类型 / 画标注的那个图 / 装它的那个模态框**。
 *
 * # 框选为什么必须是"图层"，不能把框画进图里
 *
 * 作者：「我为什么不自己去框选呢？……如果我自己框选，可能就算我非常精确，也会有一些像素的
 * 偏移。框选出来的框框大小可能不一致」—— 同一处引导要给**浅色 / 深色两张图**，
 * 两张各画一遍框必然对不齐。所以标注是**数据**（百分比坐标，见 [`GuideMark`]），
 * 两张图共用同一份，由这一个组件画上去 ⇒ 位置 / 粗细 / 圆角天然一致。
 *
 * # 坐标
 *
 * `left/top/width/height` 一律**百分比**（相对那张图的宽高）⇒ 图缩放时不跑位。
 * ⚠️ 百分比是相对**整张图**算的：作者给的是系统截图（自带窗口投影的边距），
 * 量坐标时把边距一起算进去。
 *
 * # 深浅两张怎么取
 *
 * `useIsDark()`（`state/appState` 那唯一一份解析好的真值）二选一，不自己判 CSS、
 * 也不自己 `matchMedia` —— 取图那一句收在各自的引导 spec 里（**谁要这张图就走那里取**）：
 * 引导图住**资产台账**（`presets/assets.toml` 的 `type = 'guide'`），spec 按**资产 id**
 * 从 catalog 算 URL（`assets/hero/heroPile.ts` 那种 import 写法只留给"跟着源码走"的界面素材）。
 */

import { useCallback, useEffect, useRef, useState } from 'react'
import type { CSSProperties, ReactNode, Ref } from 'react'

import { useIsDark } from '../state/appState'
import { Btn } from '../ui/Controls'
import Modal from '../../components/modal/Modal'
import s from './GuideImage.module.css'

/* ---------- 放大看图那一层的几个数（都是"手感"，写在最近处好调） ---------- */

/** 缩放范围。大图本身已经约 2 倍，再放到 4 倍足够看清窗口里的字 */
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

/**
 * 一条标注**画出来什么样**（全部可选；一个都不写 = 今天的样子：圆角 8 / 线宽 2 / 编号 18）。
 *
 * 这几项由工作台「标注台」编辑、随标注源稿一起存（`presets/assets/guides/*.anno.json`），
 * 描到客户端的 CSS 变量上（`--mark-radius` / `--mark-stroke` / `--badge-size`）。
 *
 * ★ **颜色不在这儿**：颜色必须跟主题走（浅 / 深两套 token），写进标注就会和主题系统打架。
 */
export interface GuideMarkStyle {
  /** 框的圆角（px） */
  radius?: number
  /** 框的线宽（px）—— 画在 `box-shadow` 上，所以能小于 1px 也能量着缩放 */
  stroke?: number
  /** 左上角那颗编号圆点的直径（px） */
  badge?: number
}

/** 图上的一块标注 */
export interface GuideMark {
  /** 框的位置与大小：**百分比**（相对整张图的宽高） */
  at: { left: number; top: number; width: number; height: number }
  /** 框角上那颗编号 —— 与右侧那几步文字对得上 */
  step: number
  /** 画出来什么样（不写 = 站里的缺省；见 [`GuideMarkStyle`]） */
  style?: GuideMarkStyle
}

/** 一条标注的样式 → 三个 CSS 变量（写在**这条标注自己身上** —— 每条可以不一样） */
function markVars(style: GuideMarkStyle | undefined): CSSProperties {
  if (style === undefined) return {}
  const v: Record<string, string> = {}
  if (style.radius !== undefined) v['--mark-radius'] = `${style.radius}px`
  if (style.stroke !== undefined) v['--mark-stroke'] = `${style.stroke}px`
  if (style.badge !== undefined) v['--badge-size'] = `${style.badge}px`
  return v as CSSProperties
}

/** 一处图片引导的图（浅 / 深两张同裁切）＋ 标注 */
export interface GuideFigureSpec {
  /** 浅色模式那张 */
  light: string
  /** 深色模式那张（**与浅色那张同尺寸同裁切**，否则标注会飘） */
  dark: string
  /** 这张图在说什么（无障碍那句，也是图没加载出来时能读到的） */
  alt: string
  marks: GuideMark[]
  /**
   * **这张图是不是"框已经烧进像素"的成品图**（2026-10-16 方案 A）。
   *
   * 是 ⇒ **不再叠画 `marks`** —— 框已经是图的一部分了，再画一层就是双份
   * （而且位置会因百分比取整差一点点，看着像重影）。
   *
   * 不写 / `false` ⇒ 老规矩：图是原图，框由这里按 `marks` 现画
   * （成员资产还是原图时仍然成立 —— 我们不是"一刀全切"，是"图里有没有"说了算）。
   */
  rendered?: boolean
}

/** 一步。`note` 是那步下面的小字（"不点它只对这一个文件生效"这种） */
export interface GuideStep {
  text: string
  note?: string
}

/**
 * 图 + 框。**只画图与标注**，文案由下面的模态框摆 —— 图也可以单独用在别处。
 *
 * `big` 是"放大看"那一档的尺寸（`.shot` 与 `.zoomShot` 两个上限，见 CSS 那段）：
 * 同一个组件、同一份标注，只是换了两条边界的最大值 —— 大图上的框与小图上的框
 * 是同一套百分比画出来的，所以对得上。
 *
 * `onBroken`：**字节取不到**（地址在、404；云端失效那种）时报给上层。
 * 上层据此把这一处当成"没有图"——**整块不画**，而不是留一个裂图框（作者 2026-10-16：
 * 「如果云端失效的时候，这个图片显示不出来是怎么样子的……要不然就留空吧」）。
 */
export function GuideFigure({
  figure,
  big = false,
  innerRef,
  onBroken,
}: {
  figure: GuideFigureSpec
  big?: boolean
  /** 放大那一层拿它做 FLIP（量"原来在哪 / 现在在哪"）—— 别处不用给 */
  innerRef?: Ref<HTMLDivElement>
  /** 图取不到时喊一声（不重试、不写 console —— 失败就是失败，留空） */
  onBroken?: () => void
}) {
  const isDark = useIsDark()
  return (
    <div className={s.frame} ref={innerRef}>
      <img
        className={big ? `${s.shot} ${s.zoomShot}` : s.shot}
        src={isDark ? figure.dark : figure.light}
        alt={figure.alt}
        onError={onBroken}
        /* ★ 禁掉原生拖图（2026-10-17）：放大层里按住一拖，WebKit 起的是"拖走这张图"，
           pointer 序列当场断掉 ⇒ 平移就废了（与工作台标注台同一条病） */
        draggable={false}
      />
      {/*
       * ★ **成品图不再叠画框**（2026-10-16 方案 A）：`figure.rendered` 为真时，
       * 框早就烧进像素里了 —— 再画一层就是双份（百分比取整还会差一点点，像重影）。
       * 原图（没重渲过的成员资产）时照旧按 `marks` 现画。
       */}
      {!figure.rendered &&
        figure.marks.map((m) => (
          <span
            key={m.step}
            className={s.mark}
            style={{
              left: `${m.at.left}%`,
              top: `${m.at.top}%`,
              width: `${m.at.width}%`,
              height: `${m.at.height}%`,
              /* 样式随标注走（每条可以不一样）；不写 = CSS 里的缺省值 */
              ...markVars(m.style),
            }}
          >
            <span className={s.badge}>{m.step}</span>
          </span>
        ))}
    </div>
  )
}

/**
 * 点开看大的那一层（2026-10-16 作者点名的）。
 *
 * 「这个图片有没有可能做到可以点击，因为现在可能有点太小了……我不希望再弹一个窗出来。
 *   我是希望就在这个模态框里面实现这个图片的那种放大的感觉。它就是那种感觉可以点就
 *   悬浮出来的感觉。那应该也算是另外一个窗口吧，但是没那么突兀，它就是悬浮了出来，居中」
 *
 * 「我希望它放大的时候有过渡效果，是把那个图片**移动出来**的感觉，移动到中间居中放大的感觉」
 * ⇒ 走 `Modal` 的 `overlay`：**同一个遮罩里**浮起来（几何与模态框那一层完全一致，
 *   居中由 flex 保证，不另开一个模态框、不摆头尾）；
 *   **动效 = FLIP + WAAPI**：点的那一下量小图在哪（First），挂上之后量大图在哪（Last），
 *   用 `transform: translate+scale` 把这一段补出来 —— 读起来就是**这张图从它原来待的地方
 *   飞到中间、顺带长大**；收起时倒着放一遍（缩回原来那个位置）再由调用方卸掉。
 *   ★ 必须 WAAPI：真机 WKWebView 上 CSS `animation`/`transition` 就是不播（见 `ARCHITECTURE`）。
 *   ★ 只动 `transform`（合成器里跑），不碰布局；`cancel` 放在 effect 的清理里，重进不叠。
 */
function ZoomView({
  figure,
  from,
  returnTo,
  onGone,
  onBroken,
}: {
  figure: GuideFigureSpec
  /** 点的那一下小图在哪（FLIP 的 First） */
  from: DOMRect
  /** 收起时小图**现在**在哪（中间可能被布局动过，现量） */
  returnTo: () => DOMRect | null
  /** 收起那一趟放完 —— 真的卸掉这一层 */
  onGone: () => void
  /** 大图这一份也取不到（同一个地址，一般是同一次失败）—— 上层收回这一层并当"无图" */
  onBroken: () => void
}) {
  const layerRef = useRef<HTMLDivElement>(null)
  const figRef = useRef<HTMLDivElement>(null)
  const hintRef = useRef<HTMLSpanElement>(null)
  const closing = useRef(false)

  /*
   * ============ 用户自己缩放（作者 2026-10-16：「我希望它还可以缩放，就是我在触摸板上面
   * 可以双指那种把它缩放，下次点击放大打开又是默认的大小」＋「没有触摸板的，应该也支持那种
   * 滚轮放大吧」） ============
   *
   * 两条入口，一套状态：
   *   ① `wheel`：**触控板双指捏合在 WKWebView 里就是以 `wheel + ctrlKey` 到达的**
   *      （内核把 pinch 规整成这一种），普通滚轮是 `ctrlKey === false` ⇒ 都接，
   *      力度不同；都必须 `preventDefault()`，否则是**整页 / 整窗**缩放（真机上很难查）。
   *      ★ 监听要用原生 + `{passive:false}`：React 的 `onWheel` 是 passive，拦不住。
   *   ② 两指 `pointer`（`Map<pointerId,…>`）：距离比 = 缩放系数、中点 = 缩放中心
   *      —— 覆盖触摸屏（以及将来 iPad 那种形态）。
   *
   * 三条规矩：
   *   · **只写内联 `transform`**，`requestAnimationFrame` 合帧 —— 不碰 React 状态、不重渲染；
   *     连续输入也不需要时间轴（WAAPI 那套留给"飞进来 / 缩回去"那两趟）。
   *   · **以指针为不动点**：`t' = S - C - (S - C - t) · k'/k` —— 不这么做，捏合时图会乱跑。
   *   · **每次打开都是默认大小**：这一层是条件渲染，每次打开都是新实例；
   *     但仍然显式把三个值复位（万一将来被改成常驻，这条就是安全网）。
   */

  /** 缩放倍率 / 平移（只活在 ref 里：每帧改的是 DOM，不是 React 状态） */
  const scaleRef = useRef(1)
  const posRef = useRef({ x: 0, y: 0 })
  /** 图**未缩放时**的盒子（中心 + 尺寸）—— 缩放中心和"缩回小图"都按它算 */
  const baseRef = useRef<{ cx: number; cy: number; w: number; h: number } | null>(null)
  const rafRef = useRef(0)
  /** 最近一次手势的时刻 —— 之后一小段时间里的点击不当"收起" */
  const gestureAt = useRef(0)
  const pointers = useRef(new Map<number, { x: number; y: number }>())
  const pinchRef = useRef<{ dist: number; mid: { x: number; y: number } } | null>(null)
  const panRef = useRef<{ x: number; y: number; tx: number; ty: number } | null>(null)

  /** 量"未缩放时的盒子"：进场那一趟跑完、transform 归零之后量一次 */
  const measureBase = useCallback(() => {
    const el = figRef.current
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
    const el = figRef.current
    if (el === null) return
    const { x, y } = posRef.current
    const k = scaleRef.current
    el.style.transform = k === 1 && x === 0 && y === 0 ? '' : `translate(${x}px, ${y}px) scale(${k})`
    /*
     * ★ 框的线宽 / 圆角 / 编号圆点**视觉大小恒定**：`--zoom-k` 让 CSS 那边反着除一下
     *   （放大看图时框不该变成粗橡皮筋，编号也不该盖住半张图）。见 CSS `.mark`。
     */
    el.style.setProperty('--zoom-k', String(k))
    /* 提示那句在放大之后会被图压住/糊在一起 —— 缩放态把它收起来（不改 React 状态） */
    const hint = hintRef.current
    if (hint !== null) hint.style.opacity = k > 1.02 || k < 0.98 ? '0' : ''
  }, [])

  const schedule = useCallback(() => {
    if (rafRef.current !== 0) return
    rafRef.current = requestAnimationFrame(paint)
  }, [paint])

  /** 别让图被拽出舞台：图比舞台大时允许挪到边，小时只留一点点余量 */
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
    const el = figRef.current
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
    /* 只有那一层底（`::before`，不是这一层自己）渐显 —— 见 CSS 里那条：
       动层自己的 opacity 会在它透明的那一帧把**背后整块界面**透出来，那就是"闪" */
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
       * ★★★ **三条入口分流**（2026-10-17 作者：「不能双指拖动」）—— 与工作台标注台
       * 同一条判据（浏览器没有"这是触控板还是鼠标"的字段，只能看 delta 形状）：
       * - `ctrlKey` = 触控板**捏合** ⇒ 缩放；
       * - **鼠标滚轮** ⇒ 缩放：整档的整数大 delta（`|dy| ≥ 100`、`deltaX` 恒 0），
       *   或 `deltaMode !== 0`（按行 / 按页滚的老鼠标）；
       * - **其余** = 触控板**两指滑** ⇒ **平移**（跟着手指走，`delta` 是"内容该退的方向"）。
       *   旧写法把没带 `ctrlKey` 的 wheel 全拿去缩放 ⇒ 两指滑也成了缩放（作者实锤）。
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
       * ★★ "刚做过手势"要按**累计位移**判，不按单次事件的 delta（2026-10-17 作者：
       * 「按压住拖动松手的时候图片就缩小回去了」）—— 慢慢拖时每一事件的位移都过不了
       * `PAN_SLOP`，`gestureAt` 从没记上 ⇒ 松手后那次 `click` 就把整层当"点任意处"收了
       * （图缩回小图那趟动画，看着像"缩放被复位"）。⇒ 与**按下点**比总距离。
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
   * 收起：图缩回小图现在的落点，**顺带自己渐隐**，放完才卸这一层。
   *
   * ★★ 这里是"缩小时会闪"那一条的正解，两步都是刻意的：
   *
   * ① **渐隐加在图上，不加在这一层自己身上**。动层自己的 opacity 会在透明那一帧
   *    把背后整块界面透出来（看着就是闪一下），而只动「图 + 那一层底」时，
   *    遮罩从头到尾**一直是暗的**，收的只是图。所以底那一层用 `::before` 单独画，
   *    直接不动它（要动也只许动它，不许动父级）。
   * ② **卸这一层的那一刻，那帧里不能有"大框塌掉"**：图的渐隐在动画末尾就把它自己
   *    压到几乎看不见（0.06），`onGone` 落地时那个 626×750 的框视觉上已经没了，
   *    于是接缝那帧不会被看见。这也是为什么这里必须 `fill: 'forwards'` ——
   *    动画一结束就"弹回不透明"再被卸掉，正是原来那一闪。
   */
  const close = () => {
    if (closing.current) return
    const el = figRef.current
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
     * 当前生效的矩阵（内联 / 动画都算在内）⇒ 从 identity 起会"跳一下再收"。
     *
     * **终点按"未缩放那张图的盒子"算**（不是按当前渲染盒）：变换的坐标系永远是未缩放的
     * 那一套（`translate` 相对它、`scale` 绕它的中心）⇒ 收回到小图落点就是
     * `translate(小图中心 − 基线中心) scale(小图宽 ÷ 基线宽)`，与用户缩到几倍无关。
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
     模态框的 Esc 挂在 `document` 的冒泡阶段（见 `Modal.tsx`），所以这一条用**捕获阶段**
     抢在它前面，并且 `stopPropagation`：一按 Esc 先收这一层，不许连模态框一起关。 */
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
      className={s.zoomLayer}
      onMouseDown={(e) => {
        /* 只认落在这一层自己身上的那一下（从图上拖到外面松手不算）；
           刚捏合 / 拖过的那一下也不认 —— 那是同一个手势的尾巴，不是"点任意处收起" */
        if (e.target === e.currentTarget && !justGestured()) close()
      }}
    >
      <button
        type="button"
        className={s.zoomHit}
        onClick={() => {
          if (!justGestured()) close()
        }}
        aria-label="收起放大图"
      >
        <GuideFigure figure={figure} big innerRef={figRef} onBroken={onBroken} />
      </button>
      <span ref={hintRef} className={s.zoomHint}>
        拖动挪位置 · 滚轮 / 捏合缩放 · 点任意处或 Esc 收起
      </span>
    </div>
  )
}

/**
 * 图片引导那个框：一句状态 + 图（可省）+ 几步。
 *
 * `status` 是"刚才那一下发生了什么"（"已帮你打开那份文件的「显示简介」"；失败时就是
 * 那条错的原话）—— 与 `figure` 分开：图讲**怎么做**，状态讲**此刻在哪**。
 */
export default function GuideImageModal({
  open,
  title,
  status,
  figure,
  steps,
  statusExtra,
  onClose,
}: {
  open: boolean
  title: string
  status: string
  /** `null` = 这一处还没有图（图没到位的那些引导照常能用：只剩文字） */
  figure: GuideFigureSpec | null
  steps: GuideStep[]
  /**
   * **状态句底下的一个槽**（2026-10-17）：失败的那几条话往往带着"你还得再做一件事"
   * （例：自动化授权被拒 ⇒ 去系统设置把「访达」勾回来）—— 这类"接着做"的入口
   * 由调用方给（按钮 / 链接），这里只负责摆在该摆的位置。`undefined` = 不摆。
   */
  statusExtra?: ReactNode
  onClose: () => void
}) {
  /* 点小图 = 在这个模态框里浮起来看大的（见 `ZoomView`）。
     `fromRef` 记的是**点的那一下小图在哪** —— FLIP 的 First，得在开之前量 */
  const [zoom, setZoom] = useState(false)
  const fromRef = useRef<DOMRect | null>(null)
  const figBtnRef = useRef<HTMLButtonElement>(null)

  /**
   * **图取不到**（台账里查不到那条资产 ⇒ `figure === null`；地址在、字节 404 ⇒ 这里）。
   * 两条路落到同一处：**整块图不画**，右列文字自己占满整幅宽
   * （作者 2026-10-16：「要不然就留空吧，不要让它显示」——不留裂图、不留灰块、不写一个字）。
   */
  const [imgBroken, setImgBroken] = useState(false)
  /* 换一处引导 / 切深浅（换了一张 URL）都要复位 —— 否则上一张坏了会把这一张也藏起来 */
  useEffect(() => {
    setImgBroken(false)
  }, [figure?.light, figure?.dark, open])
  /* 放大那一层里才坏掉的话，也把它收掉（回到"无图"那一态，文字照常） */
  useEffect(() => {
    if (imgBroken) setZoom(false)
  }, [imgBroken])

  const usable = figure !== null && !imgBroken ? figure : null

  /* 模态框一关，放大那一层也跟着收（不然下次打开还是放大的） */
  useEffect(() => {
    if (!open) setZoom(false)
  }, [open])

  return (
    <Modal
      open={open}
      title={title}
      size="lg"
      onClose={onClose}
      overlay={
        zoom && usable !== null ? (
          <ZoomView
            figure={usable}
            from={fromRef.current ?? new DOMRect(0, 0, 0, 0)}
            returnTo={() => figBtnRef.current?.getBoundingClientRect() ?? null}
            onGone={() => setZoom(false)}
            onBroken={() => setImgBroken(true)}
          />
        ) : undefined
      }
      footer={
        <Btn variant="primary" onClick={onClose}>
          知道了
        </Btn>
      }
    >
      {/*
        * 图在左、**文字全在右**（2026-10-16 作者：「这上面这段话能不能移到右侧呀，
        * 就是说左侧显示图片，右侧显示文字」）—— 状态那句原来横在顶上占满整幅宽，
        * 图被压到第二行去，视线得来回跳；现在右侧一列从上到下就是
        * 「此刻在哪（status）→ 怎么做（steps）」，图在左边同一眼界里。
        */}
      <div className={s.split}>
        {usable !== null && (
          /* 图可点（小窗里它只剩 ~200px 宽，用户要能点开看大的）—— 皮扒干净，只留"能点"。
             ★ 不摆"点一下放大"那种提示（作者 2026-10-16：不要出现那个提示）；能点的意思
             由光标（`cursor: zoom-in`）与那一下过渡自己说。 */
          <button
            ref={figBtnRef}
            type="button"
            className={s.figBtn}
            onClick={() => {
              fromRef.current = figBtnRef.current?.getBoundingClientRect() ?? null
              setZoom(true)
            }}
            aria-label={`放大看：${usable.alt}`}
          >
            <GuideFigure figure={usable} onBroken={() => setImgBroken(true)} />
          </button>
        )}
        <div className={s.side}>
          <p className={s.status}>{status}</p>
          {statusExtra}
          <ol className={s.steps}>
            {steps.map((step, i) => (
              <li key={i} className={s.step}>
                <span className={s.stepText}>{step.text}</span>
                {step.note !== undefined && <em className={s.stepNote}>{step.note}</em>}
              </li>
            ))}
          </ol>
        </div>
      </div>
    </Modal>
  )
}
