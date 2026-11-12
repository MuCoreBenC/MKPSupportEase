/*
 * 宽度弹簧（灵动岛那种"弹跳 / 果冻"）—— 全站唯一一份。
 *
 * 作者 2026-10-11（校准页那颗改动药丸）：「还是一瞬间就变宽变窄了，没有 iOS 那种
 * 弹跳、果冻的效果」。CSS 做不出这个效果，两轮都栽在同一个点上：
 *   - `width: fit-content` 不是可插值长度 → `transition: width` 无效；
 *   - `transition: max-width` 是空转（两态都是 100%，没有变化量）；
 *   - `width: calc-size(fit-content, size)` 理论上可插值，实测补间没跑起来，
 *     而且它只能做**单调**补间 —— 而"果冻"要的是 **overshoot（冲过头再回弹）**。
 *
 * 所以走 JS：把两端宽度**量成 px**，再解一条**弹簧轨迹**交给 Web Animations API。
 *
 * 灵动岛那个"果冻"感其实是两件事叠出来的（少了任何一个都死板）：
 *   ① **弹簧**：走 stiffness/damping 的阻尼谐振子，带 ≈4.6% 的 overshoot；
 *   ② **squash & stretch**：横向拉得快时纵向压一点、回弹时鼓一点 ——
 *      只有宽度弹性、没有这个挤压，看着就是"死板地变宽"。
 * 用 `width` 做动画会触发布局，但这类胶囊只有几十像素宽的子树，代价可忽略。
 *
 * 2026-11-11 从 `calib/PageCalib.tsx` 搬到共用：预设页「刷新云端」那颗按钮
 * （图标 → 胶囊）要的是同一种手感，不再各写一份。
 *
 * # 调用方要做的两件事
 *
 *   ① 自己量 `from` / `to`（px），差不到 `MIN_DELTA` 就别播；
 *   ② 动画是 `fill: 'forwards'` —— 收尾后**自己**把 inline 的 `width` / `transform`
 *      清回 CSS（见 PageCalib / PagePresets 那两处 `finished.then`）。
 */

/*
 * 弹簧用 **stiffness / damping / mass** 描述，而不是 duration —— 这是灵动岛
 * 那种"物理感"的来源（oksr/react-dynamic-island 的设计说明里也是这么定的：
 * "Springs are parameterized by stiffness and damping rather than duration"）。
 * 好处是**可被中断**：连续点格子时能从当前位置与速度接着走，不会跳回起点。
 *
 * 阻尼比 ζ = damping / (2·√(stiffness·mass)) ≈ 0.65 —— 欠阻尼但不过分，
 * 理论 overshoot ≈ 4.6%，正是灵动岛那种"轻轻弹一下就停"的量。
 * （ζ 再小就晃、再大就没有那一下弹；0.7 附近是它的甜点。）
 */
const SPRING = { stiffness: 600, damping: 32, mass: 1 }

/*
 * 积分步长取 **1/120**（比显示的 60fps 更细）—— 半隐式欧拉在 1/60 下有明显的
 * 数值阻尼，实测把 4.3% 的 overshoot 吃到只剩 1.1%，弹跳几乎没了；
 * 细化到 1/120 才回到物理值。多出来的帧只进关键帧数组，不做任何额外计算。
 */
const DT = 1 / 120

/** 静止判定：位置与速度都足够小就收工 */
const REST_X = 0.0005
const REST_V = 0.0005

/** 帧数上限（120 帧 = 1s），防数值不收敛时兜底 */
const MAX_FRAMES = 120

/**
 * 挤压幅度：横向速度带来的纵向形变（**squash & stretch**，果冻感的真正来源）。
 * 变宽时纵向压一点、回弹时纵向鼓一点 —— 灵动岛展开就是这一下。
 */
const SQUASH = 0.04

/** 变化小于这个像素就不值得播动画（字体渲染的亚像素抖动） */
export const MIN_DELTA = 0.5

/** 阻尼谐振子的一个采样点。x 是相对进度（0→1），v 是速度 */
interface SpringSample {
  t: number
  x: number
  v: number
}

/**
 * 解出来的轨迹**只跟 SPRING 有关**，一次就够 —— 缓存住。
 * （每次点击重新积分 50 来步也不算贵，但它会让第一帧多等一点点，
 *   而第一帧正是"弹出去"那一下。）
 */
let springPath: SpringSample[] | null = null

/** 数值积分求解弹簧（半隐式欧拉），返回从静止到静止的整条轨迹 */
function solveSpring(): SpringSample[] {
  const { stiffness: k, damping: c, mass: m } = SPRING
  const out: SpringSample[] = [{ t: 0, x: 0, v: 0 }]
  let x = 0
  let v = 0
  let t = 0
  for (let i = 0; i < MAX_FRAMES; i++) {
    const a = (-k * (x - 1) - c * v) / m
    v += a * DT
    x += v * DT
    t += DT
    out.push({ t, x, v })
    if (Math.abs(x - 1) < REST_X && Math.abs(v) < REST_V) break
  }
  /* 尾巴钉死：保证最后一帧正好落在 x=1、v=0，避免停在一个抖动的中间值上 */
  const last = out[out.length - 1]
  if (last.x !== 1 || last.v !== 0) out.push({ t: last.t + DT, x: 1, v: 0 })
  return out
}

/**
 * 播放期间在那个元素上挂 `data-spring`，CSS 据此**卸掉磨砂 / 大投影**。
 *
 * 那两样每帧都要按新尺寸重新栅格化，比宽度本身的布局贵得多 —— 作者 2026-11-11
 * 说"帧率不高"，主要就是它们在吃帧（校准页那颗药丸带 `backdrop-filter`）。
 *
 * 记数是因为同一个元素上可能前后叠两段（上一段被 cancel、新的立刻接上）：
 * 只有**最后一段**结束才摘标记，否则中途会闪一下玻璃。
 */
const playing = new WeakMap<HTMLElement, number>()

/** 系统开了「减弱动态效果」就一律不演（宽度弹簧与阴影回补同一条判据） */
function reduceMotion(): boolean {
  return (
    typeof window.matchMedia === 'function' &&
    window.matchMedia('(prefers-reduced-motion: reduce)').matches
  )
}

/**
 * 弹簧跑完、`data-spring` 摘掉之后，**把阴影渐渐补回来**。
 *
 * 作者 2026-11-11：动画期间为了帧率卸掉阴影，但"唰"地一下回来像一个黑影突现 ——
 * 要的是**渐渐出现**。这里只动 `box-shadow`（元素尺寸已经定了，不再触发布局），
 * 从"同样几何、颜色全透明"补到 CSS 那一套，180ms。
 */
function fadeShadowBack(el: HTMLElement): void {
  if (reduceMotion() || !el.isConnected) return
  const shadow = getComputedStyle(el).boxShadow
  if (shadow === '' || shadow === 'none') return
  /* 同一条影、颜色清零 —— 只让不透明度从 0 走到 1，形状不参与插值误差。
     `color(srgb …)` 那套写法（新引擎会给）一并认，不然就只能"不淡入"了 */
  const clear = shadow
    .replace(/rgba?\([^)]*\)/g, 'rgba(0, 0, 0, 0)')
    .replace(/color\([^)]*\)/g, 'rgba(0, 0, 0, 0)')
  el.animate([{ boxShadow: clear }, { boxShadow: shadow }], {
    duration: 180,
    easing: 'ease-out',
  })
}

function markPlaying(el: HTMLElement, on: boolean): void {
  const next = Math.max(0, (playing.get(el) ?? 0) + (on ? 1 : -1))
  if (next === 0) {
    playing.delete(el)
    delete el.dataset.spring
    return
  }
  playing.set(el, next)
  el.dataset.spring = 'on'
}

/**
 * 播一段灵动岛式的宽度弹性：宽度走弹簧轨迹，同时按**横向速度**给纵向一点挤压。
 *
 * `fill: 'forwards'` —— 收尾请调用方把 inline 宽度与 transform 清掉（见文件头）。
 * 播放期间元素带 `data-spring`（见 `markPlaying`），CSS 那边据此减负。
 */
export function islandWidth(el: HTMLElement, from: number, to: number): Animation {
  springPath ??= solveSpring()
  const path = springPath
  const delta = to - from
  const total = path[path.length - 1].t

  /*
   * 正负速度**分别**归一化：回弹那一段的速度只有峰值的一成左右，
   * 若按同一个峰值归一化，回弹时几乎不鼓（实测只到 1.002），看着还是"单向压扁"。
   * 各自归一之后两个方向都吃满 SQUASH，才有真正的"伸缩"果冻感。
   */
  let peakOut = 0 // 变宽方向的最大速度
  let peakBack = 0 // 回弹方向的最大速度（正值）
  for (const p of path) {
    if (p.v > peakOut) peakOut = p.v
    if (-p.v > peakBack) peakBack = -p.v
  }

  const frames = path.map((p) => {
    /* 归一化到 [-1, 1]：+1 = 正在全速变宽，-1 = 正在全速回弹 */
    const s = p.v >= 0 ? p.v / (peakOut || 1) : p.v / (peakBack || 1)
    return {
      offset: p.t / total,
      width: `${from + delta * p.x}px`,
      /* 变宽 → 纵向压；回弹 → 纵向鼓 */
      transform: `scaleY(${1 - SQUASH * s})`,
    }
  })

  markPlaying(el, true)
  const anim = el.animate(frames, { duration: total * 1000, fill: 'forwards' })
  /* 跑完与「被 cancel」都要摘标记（cancel 会让 finished reject） */
  void anim.finished.then(
    () => {
      markPlaying(el, false)
      /* 阴影是动画期间卸掉的 —— 摘了标记别让它"唰"地出现，补一个 180ms 渐显 */
      fadeShadowBack(el)
    },
    () => markPlaying(el, false),
  )
  return anim
}
