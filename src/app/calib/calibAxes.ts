/*
 * 三轴的类型、板上刻度查表、文案。
 * 生成的板子坐标直接 import `src/calib/*`（共享资产）。
 * 里面的类型与常量名照原样 —— 它们是这一套的词汇表。
 */

import { LADDER } from './plateLadder'
import { t, type StringKey } from '../../i18n'
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
/**
 * 三条轴的**行名**：第一格是字典键（2026-10-26），不是句子 ——
 * 这张表是模块级常量，放句子会被求值一次冻住；渲染处用 `tr(label)`。
 */
export const AXIS_ROWS: [StringKey, Axis][] = [
  ['calib.axis.x', 'x'],
  ['calib.axis.y', 'y'],
  ['calib.axis.z', 'z'],
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
export const zHitLabel = (id: string) =>
  t('calib.hit.z', { label: Z_SPOT.get(id)?.label ?? '' })

/** XY 板一格的悬浮说明 */
export const xyHitLabel = (id: string) => {
  const it = LADDER[id]
  if (!it) return id
  const delta = signed(it.delta)
  return it.axis === 'x' ? t('calib.hit.xyX', { delta }) : t('calib.hit.xyY', { delta })
}

/*
 * 「校准」tab 提示条的同一件事，写成**两截**（2026-10-10）。
 *
 * 一整句 33 字铺满一条胶囊就是它难看的根源 —— 拆成**主句 + 补充**，
 * 与「未保存的改动 · Z 4.00 → 3.70」同一节奏；右端那颗「选择预设」
 * 再把另一头锚住（指路的事交给按钮，不用写在字里）。
 */
export const NEED_PRESET_TITLE_KEY: StringKey = 'calib.needPreset.title'

/** 主句后面那半句：说清差哪一步 */
export const NEED_PRESET_HOW_KEY: StringKey = 'calib.needPreset.how'

/*
 * ★ 2026-10-13：原来还有两条给首页校准卡用的旧提示 ——
 *   NEED_PRESET（「先回『选择机型』把机型与打印件版本选好…」）与
 *   NEED_COPY（「这份还没有『你的一份』—— 先去预设页…『另存为本地预设』…」）——
 * 整条退役（作者：「你把这一行字去掉，没必要提示给用户，这些字都去掉……本地这份、
 * 云端一份这些相关的都去掉，那是以前的旧版才那么设计的」）。
 * 首页两张校准卡板下从此只剩 Z 步那两行原话（Z_LEGEND / Z_TIP），
 * 有无预设同一副版面（见 `PageHome` 里 `data-hint` 那一段与 CSS 的同名注释）。
 */

/** 一格偏移的文本。没值就是一条横杠 —— 不拿旧数字或默认值冒充 */
export const axisText = (axes: Axes | null, k: Axis) => (axes ? `${axes[k].toFixed(2)} mm` : '—')

/*
 * Z 步板下的两行文案（G06-8 四轮攒下的原话，一句不改）。
 * 「校准」tab 与首页向导的 Z 卡共用这一份 —— 第二个人要用时只能搬走，不能各写一遍。
 */
/**
 * 首页 Z 步板下那两行（2026-10-26 起存**字典键**，不存句子）。
 * 渲染处 `tr(Z_LEGEND_KEY)` / `tr(Z_TIP_KEY)` —— 语言一换这两行同帧换字。
 */
export const Z_LEGEND_KEY: StringKey = 'calib.z.legend'
export const Z_TIP_KEY: StringKey = 'calib.z.tip'
