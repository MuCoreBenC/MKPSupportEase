/**
 * 大图的尺寸与微移曲线 —— 固化值 + 求值函数。
 *
 * 试验场里这套值活在 `src/dev/devStore.ts`（914 行）里：六条曲线存 localStorage、可以拖图
 * 改点、Ctrl+Z 撤销、「存到项目」写回 `src/dev/heroCurves.json`。那是调参工作台。
 * 产品仓只需要**调完的结果**，所以这里只保留两样东西：
 *
 *   1. 曲线的固化值 —— 直接抄自 `src/dev/heroCurves.json`（savedAt 2026-09-18T06:23:37.401Z，
 *      base 640、stepPx 40 的那一版，也就是 v023 当前屏幕上生效的那一份）；
 *   2. 纯求值函数 —— 线性插值，无状态、无副作用。
 *
 * 编辑路径（`bumpCurveAt` / `setNudgeAt` / `commitTune` / `undo` / `redo` / `resetNear`）
 * 与上报路径（`reportSlot` / `reportHero` / `reportPage`）整套**不搬**：产品里没有调参面板，
 * 留着就是死代码，而且会把 localStorage 的历史包袱一起带进来。
 *
 * 要改这些值：回试验场调，调完把这里的数字换掉。别在产品仓里手改 —— 那边有可视化的曲线编辑器，
 * 这里只有数字。
 */

/** [容器尺寸, 值]，按尺寸升序 */
export type CurvePoint = [number, number]

export interface Nudge {
  nudgeX: number
  nudgeY: number
}

/**
 * 大图的基准宽：目标宽 = HERO_BASE × fill。
 * 必须与 `HeroFade.module.css` 里 `.hero` 的 `width: calc(640px * var(--hero-fill))` 一致。
 */
export const HERO_BASE = 640

/** 微移的上下限（百分比），固化值本身就落在这个区间内 */
export const NUDGE_LIMIT = 40

/* 尺寸：宽 × 高两条相乘。位置是偏移量，所以微移用相加。
   （相乘对偏移量没有物理意义，这是当初分成两组的原因。） */
const FILL_W: CurvePoint[] = [
  [360, 0.706],
  [680, 0.7],
  [1920, 1],
]

const FILL_H: CurvePoint[] = [
  [420, 0.804],
  [560, 0.991],
  [1200, 1],
]

const NUDGE_X_W: CurvePoint[] = [
  [360, -40],
  [600, -32.982],
  [646, -40],
  [777, -19.748],
  [960, -12.717],
  [1360, -14.371],
  [1920, 0],
]

const NUDGE_X_H: CurvePoint[] = [
  [420, 0],
  [1200, -0.724],
]

const NUDGE_Y_W: CurvePoint[] = [
  [360, 0],
  [1360, 0],
  [1920, 0],
]

const NUDGE_Y_H: CurvePoint[] = [
  [420, 0],
  [560, 0],
  [760, -0.31],
  [1200, -9.409],
]

export const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v))

/** 线性插值求值，超出两端取端点值。点已按升序写死，不再每次排序 */
export function evalCurve(points: CurvePoint[], at: number): number {
  if (points.length === 0) return 1
  if (at <= points[0][0]) return points[0][1]
  const last = points[points.length - 1]
  if (at >= last[0]) return last[1]
  for (let i = 0; i < points.length - 1; i += 1) {
    const [x1, y1] = points[i]
    const [x2, y2] = points[i + 1]
    if (at >= x1 && at <= x2) {
      const t = x2 === x1 ? 0 : (at - x1) / (x2 - x1)
      return y1 + (y2 - y1) * t
    }
  }
  return last[1]
}

/** 最终倍率 = 宽度曲线 × 高度曲线 */
export function evalFill(width: number, height: number): number {
  return evalCurve(FILL_W, width) * evalCurve(FILL_H, height)
}

/** 微移 = 宽度曲线 + 高度曲线（偏移量相加） */
export function evalNudge(width: number, height: number): Nudge {
  return {
    nudgeX: clamp(
      evalCurve(NUDGE_X_W, width) + evalCurve(NUDGE_X_H, height),
      -NUDGE_LIMIT,
      NUDGE_LIMIT,
    ),
    nudgeY: clamp(
      evalCurve(NUDGE_Y_W, width) + evalCurve(NUDGE_Y_H, height),
      -NUDGE_LIMIT,
      NUDGE_LIMIT,
    ),
  }
}
