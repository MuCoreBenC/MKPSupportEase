/*
 * 大图的六条曲线 + 求值 + 「应用窗口有多大」。
 *
 * # 这份数据是哪来的
 *
 * 值逐个照抄试验场调好之后「存到项目」写出来的那份基线（axis: 'window' / base: 640）。
 * **不是我编的**，也不是随机数。
 *
 * 试验场的运行时是这样的：调试面板把六条曲线存在 localStorage 里，没存过的通道拿
 * 那份基线兜底。产品不带调试面板，所以这里只留兜底那一份，并且写死 ——
 * 要改大图的尺寸曲线就改这个文件。
 *
 * # 横轴是「应用窗口的逻辑宽高」，不是槽位
 *
 * 这一条是试验场踩出来的结论：槽位（卡片限宽与信息列算出来的那个宽度）值域窄、
 * 还在大窗里反向，不能当轴。所以 fillW / nxW / nyW 吃窗口宽，fillH / nxH / nyH 吃窗口高。
 *
 * 窗口尺寸自己量：往上找到最外层那个带 `data-density` 的节点（就是应用的根，
 * 它铺满整个应用窗口），量它的 offsetWidth / offsetHeight。用 offset* 而不是
 * getBoundingClientRect —— 外壳的「适配」模式会给窗口套一个 scale()，rect 会把那个
 * 缩放算进来，offset* 是纯布局值，不受 transform 影响。
 */

import { useCallback, useLayoutEffect, useState } from 'react'

/** [窗口尺寸, 值]，按尺寸升序 */
export type CurvePoint = [number, number]

/* 曲线横轴的下限：量不到窗口（首帧）时拿它当 x，与试验场那份兜底一致 */
const CURVE_W_MIN = 360
const CURVE_H_MIN = 420

/**
 * 六条曲线本体：尺寸两条（宽 × 高，相乘）、微移四条（宽 + 高，相加）。值照抄试验场那份基线。
 *
 * ★★ 2026-11-03：这份数据现在有**第二个消费者** —— 首屏骨架（`index.html` 里
 *   `#boot .boot-hero` 那一格）。骨架必须比任何模块都早成形，所以那边内联了一份
 *   **逐字镜像**（`@@HERO-CURVES-BEGIN@@` 那一段）；`heroCurves.test.ts` 读两个文件
 *   比对，改一边不改另一边会红。**这里是唯一权威**，要调曲线只改这里 + 镜像。
 */
export interface HeroCurves {
  fillW: CurvePoint[]
  fillH: CurvePoint[]
  nudgeXW: CurvePoint[]
  nudgeXH: CurvePoint[]
  nudgeYW: CurvePoint[]
  nudgeYH: CurvePoint[]
}

export const HERO_CURVES: HeroCurves = {
  fillW: [
    [360, 0.706],
    [680, 0.7],
    [1920, 1],
  ],
  fillH: [
    [420, 0.804],
    [560, 0.991],
    [1200, 1],
  ],
  nudgeXW: [
    [360, -40],
    [600, -32.982],
    [646, -40],
    [777, -19.748],
    [960, -12.717],
    [1360, -14.371],
    [1920, 0],
  ],
  nudgeXH: [
    [420, 0],
    [1200, -0.724],
  ],
  nudgeYW: [
    [360, 0],
    [1360, 0],
    [1920, 0],
  ],
  nudgeYH: [
    [420, 0],
    [560, 0],
    [760, -0.31],
    [1200, -9.409],
  ],
}

/** 线性插值求值，超出两端取端点值。与试验场那份实现一致 */
export function evalCurve(points: CurvePoint[], at: number): number {
  if (points.length === 0) return 1
  const sorted = [...points].sort((a, b) => a[0] - b[0])
  if (at <= sorted[0][0]) return sorted[0][1]
  const last = sorted[sorted.length - 1]
  if (at >= last[0]) return last[1]
  for (let i = 0; i < sorted.length - 1; i += 1) {
    const [x1, y1] = sorted[i]
    const [x2, y2] = sorted[i + 1]
    if (at >= x1 && at <= x2) {
      const t = x2 === x1 ? 0 : (at - x1) / (x2 - x1)
      return y1 + (y2 - y1) * t
    }
  }
  return last[1]
}

/** 最终倍率 = 宽度曲线 × 高度曲线。大图目标宽 = 640px × 这个数（见 HeroFade.module.css） */
export function evalFill(w: number, h: number): number {
  return (
    evalCurve(HERO_CURVES.fillW, w > 0 ? w : CURVE_W_MIN) *
    evalCurve(HERO_CURVES.fillH, h > 0 ? h : CURVE_H_MIN)
  )
}

/** 微移 = 宽度曲线 + 高度曲线（偏移量相加，单位是包围盒的 %） */
export function evalNudge(w: number, h: number): { nudgeX: number; nudgeY: number } {
  const ww = w > 0 ? w : CURVE_W_MIN
  const hh = h > 0 ? h : CURVE_H_MIN
  return {
    nudgeX: evalCurve(HERO_CURVES.nudgeXW, ww) + evalCurve(HERO_CURVES.nudgeXH, hh),
    nudgeY: evalCurve(HERO_CURVES.nudgeYW, ww) + evalCurve(HERO_CURVES.nudgeYH, hh),
  }
}

/**
 * 这一刻大图该多大、偏多少。
 *
 * 试验场 A41 起这个 hook 吃调试面板的曲线（devStore）；产品不带面板，按 A41 README
 * 预写的接法：**换成 evalFill / evalNudge 的结果** —— 那两个纯函数就是这里算的东西，
 * 也是唯一用到上面那六个数组的地方。没调过面板与照抄写死是同一个结果，所以两条路
 * 算出来的数一致。
 */
export function useHeroCurves(win: { w: number; h: number }): {
  fill: number
  nudgeX: number
  nudgeY: number
} {
  const fill = evalFill(win.w, win.h)
  const nudge = evalNudge(win.w, win.h)
  return { fill, nudgeX: nudge.nudgeX, nudgeY: nudge.nudgeY }
}

/** 往上找最外层带 data-density 的那个节点：应用的根，它就是整个应用窗口 */
function appRootOf(el: HTMLElement): HTMLElement {
  let found = el
  let node: HTMLElement | null = el
  while (node) {
    if (node.dataset.density !== undefined) found = node
    node = node.parentElement
  }
  return found
}

/**
 * 量应用窗口的逻辑宽高。
 *
 * 返回的 `ref` 是个回调 ref（存进 state）—— 这样元素挂上 / 卸掉都会重跑 effect，
 * 不会出现「第一帧还没有节点，之后再也不量」的情况。
 *
 * ★★ 2026-11-03：**挂上那一刻就量，而且是同步量**（作者录屏里「开屏跳动」的一个来源）。
 *
 * 原来只有 `useEffect` 里量一次 —— 那个 effect 跑在**绘制之后**。而大图挂上的第一帧
 * 拿的是 `{w: 0, h: 0}`，`evalFill` 于是退回曲线端点（0.706 × 0.804 ≈ 0.57）：
 * 360px 宽的一张图，下一帧才变成 478。这一帧平时被首屏骨架盖着，可骨架收掉的时刻
 * 是 `heroReady` 决定的（图片解码那一刻），**与这次测量没有先后关系** —— 谁先到
 * 谁说话，于是"有时候跳、有时候不跳"（作者：「不稳定触发的开屏跳动」）。
 *
 * 现在 ref 回调里先量一次（React 会把它并进同一次提交，浏览器画的还是量完的那一版），
 * effect 换成 `useLayoutEffect` 只负责挂 `ResizeObserver`。
 */
export function useWinSize(): {
  size: { w: number; h: number }
  ref: (node: HTMLElement | null) => void
} {
  const [node, setNode] = useState<HTMLElement | null>(null)
  const [size, setSize] = useState({ w: 0, h: 0 })

  const measureInto = useCallback((target: HTMLElement) => {
    const w = target.offsetWidth
    const h = target.offsetHeight
    if (w > 0 && h > 0) setSize((prev) => (prev.w === w && prev.h === h ? prev : { w, h }))
  }, [])

  const ref = useCallback(
    (el: HTMLElement | null) => {
      setNode(el)
      /* 挂上就量：不许让第一帧拿着 {0,0} 去算倍率（见上面那段） */
      if (el) measureInto(appRootOf(el))
    },
    [measureInto],
  )

  useLayoutEffect(() => {
    if (!node) return
    const target = appRootOf(node)
    measureInto(target)
    const ro = new ResizeObserver(() => measureInto(target))
    ro.observe(target)
    return () => ro.disconnect()
  }, [node, measureInto])

  return { size, ref }
}
