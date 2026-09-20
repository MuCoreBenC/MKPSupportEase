import { HOTSPOTS } from '../calib/precise-calibration.generated'

type Spot = (typeof HOTSPOTS)[number]

export interface LadderPick {
  axis: 'x' | 'y'
  /** 相对当前偏移的增量（mm），板上写的 mX0.2 */
  delta: number
}

/** 一格 0.2mm：板上「mX0.2=偏移值」 */
const STEP_MM = 0.2

/**
 * XY 板的两把梯子 → 轴 + 增量。
 *
 * 权宜之计：这块板的 22 个热区在产物里只有 role: 'mark'，没有 value / label
 * （工作台当时只给 Z 板的刻度块编了值）。所以这里按几何分：
 *   竖线（高 > 宽）= X 梯，左→右；横线（宽 > 高）= Y 梯，SVG 里 y 越小越上。
 * 正中那根是 0 线，往正方向每隔一根 +0.2mm（X 右为正、Y 上为正，与板上的
 * 「←负数 0 正数→」「正数↑」一致）。
 *
 * 正解是回工作台给这两把梯子编上轴与刻度值再重生成产物，应用侧就只读 value。
 * LADDER_META 暴露出来给测试断言，数据一变（根数、步距）立刻能发现。
 */
function build() {
  const vertical: Spot[] = []
  const horizontal: Spot[] = []
  for (const h of HOTSPOTS) {
    const w = h.bbox[2] - h.bbox[0]
    const ht = h.bbox[3] - h.bbox[1]
    ;(ht > w ? vertical : horizontal).push(h)
  }
  vertical.sort((a, b) => a.center[0] - b.center[0])
  horizontal.sort((a, b) => a.center[1] - b.center[1])

  const pick: Record<string, LadderPick> = {}
  const midV = (vertical.length - 1) / 2
  vertical.forEach((h, i) => {
    pick[h.id] = { axis: 'x', delta: Number(((i - midV) * STEP_MM).toFixed(2)) }
  })
  const midH = (horizontal.length - 1) / 2
  horizontal.forEach((h, i) => {
    pick[h.id] = { axis: 'y', delta: Number(((midH - i) * STEP_MM).toFixed(2)) }
  })

  const gap = (list: Spot[], axis: 0 | 1) =>
    list.length > 1 ? Number((list[1].center[axis] - list[0].center[axis]).toFixed(3)) : 0

  return {
    pick,
    meta: {
      xCount: vertical.length,
      yCount: horizontal.length,
      xGapMm: gap(vertical, 0),
      yGapMm: gap(horizontal, 1),
      stepMm: STEP_MM,
    },
  }
}

const built = build()

/** 热区 id → 这一格代表哪个轴、加多少 */
export const LADDER = built.pick
export const LADDER_META = built.meta
