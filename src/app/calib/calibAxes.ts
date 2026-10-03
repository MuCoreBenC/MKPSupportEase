/*
 * 三轴的类型、板上刻度查表、文案。
 * 生成的板子坐标直接 import `src/calib/*`（共享资产）。
 * 里面的类型与常量名照原样 —— 它们是这一套的词汇表。
 */

import { LADDER } from './plateLadder'
import { HOTSPOTS as Z_HOTSPOTS } from '../../calib/zoffset-calibration.generated'

/**
 * 三轴校准里那些与「长什么样」无关的东西：轴的类型、板上刻度的查表、文案。
 *
 * 从向导页里搬出来的 —— 向导的两张校准卡与「校准」tab 那一页都要用同一份，
 * 留在页面文件里第二个用的人只能复制一遍，两边就会慢慢长歪。
 */

export type Axis = 'x' | 'y' | 'z'
export type Axes = Record<Axis, number>

/** 读数条与拦截弹窗里的行顺序：X → Y → Z */
export const AXIS_ROWS: [string, Axis][] = [
  ['X 偏移', 'x'],
  ['Y 偏移', 'y'],
  ['Z 偏移', 'z'],
]

/** 数值位的三种状态：有值 / 正在取 / 没有 */
export type AxisView = 'value' | 'loading' | 'blank'

/** Z 板的刻度块在产物里就带 value / label，直接查（产物是 as const，这里放宽成普通结构） */
export const Z_SPOT = new Map<string, { value?: number; label?: string }>(
  (Z_HOTSPOTS as readonly { id: string; value?: number; label?: string }[]).map((h) => [h.id, h]),
)

export const signed = (v: number) => `${v > 0 ? '+' : ''}${v.toFixed(2)}`

/**
 * 手输的值 → 板上哪一格。
 *
 * 打字改数之后板子也该有呼应：把「改动量」拿去板上找刻度相同的那一格。
 * 容差 0.001 只是为了躲浮点（0.2 × 3 = 0.6000000000000001），**不做吸附** ——
 * 落在两格之间（比如 +0.17）就返回 null，板上一格都不选中，读数照旧显示你打的数。
 */
export const matchZ = (delta: number) => {
  for (const [id, spot] of Z_SPOT) {
    if (spot.value !== undefined && Math.abs(spot.value - delta) < 0.001) return id
  }
  return null
}

export const matchXY = (axis: 'x' | 'y', delta: number) =>
  Object.entries(LADDER).find(
    ([, it]) => it.axis === axis && Math.abs(it.delta - delta) < 0.001,
  )?.[0] ?? null

/** Z 板一格的悬浮说明 */
export const zHitLabel = (id: string) => `刻度 ${Z_SPOT.get(id)?.label ?? ''}`

/** XY 板一格的悬浮说明 */
export const xyHitLabel = (id: string) => {
  const it = LADDER[id]
  return it ? `${it.axis === 'x' ? '横向' : '纵向'} ${signed(it.delta)} mm` : id
}

/** 还没取到预设时，校准页的说明。板子照旧能看，只是点了不产生读数 */
export const NEED_PRESET =
  '先回「选择机型」把机型与打印件版本选好，取到预设之后这块板上的点选才有基准。'

/** 一格偏移的文本。没值就是一条横杠 —— 不拿旧数字或默认值冒充 */
export const axisText = (axes: Axes | null, k: Axis) => (axes ? `${axes[k].toFixed(2)} mm` : '—')

/*
 * Z 步板下的两行文案（G06-8 四轮攒下的原话，一句不改）。
 * 「校准」tab 与首页向导的 Z 卡共用这一份 —— 第二个人要用时只能搬走，不能各写一遍。
 */
export const Z_LEGEND = '← 抬高笔尖 ｜ 降低笔尖 →'
export const Z_TIP =
  '校准的时候感觉都没涂上胶水 → 点最右边的格子（-0.5，降低笔尖），或者手动把上面的 Z 改小一点'
