/*
 * 预设文件那一小块。
 *
 * 「连接失败」那一行的「重试」按钮去掉了：它点的是试验场调试面板的假下载开关
 * （devStore.setPresetPhase），不是真的重取。连带去掉只为它存在的 canAct 参数。
 *
 * ★ 2026-10-13：那块**可选的三轴竖排**（`axes`，只在小窗档 `density === 'mini'` 时
 * 才摆出来）整条退役 —— 作者：「我之前设计的，在小窗的时候，它显示在下面。
 * 现在我觉得没必要了，它不要出现这个偏移值」。
 * 这一块从此只回答"这份预设文件是什么、是不是正在用的那一份"；三轴读数各有各的家
 * （首页校准卡板下/右侧那颗读数气泡、校准 tab 的下缘那条），不在这里复读。
 * 随它一起退役的还有只为它存在的那套取值（`view`：值 / 取数中 / 横杠）
 * 与 `.axes / .row / .label / .value` 四条样式。
 */

import type { PresetState } from '../calib/usePreset'
import TraceTag from '../components/TraceTag'
import Skeleton from './Skeleton'
import s from './PresetStack.module.css'

interface PresetStackProps {
  state: PresetState
  /**
   * 「正在使用」的说明行（A3）。底账正指着这一份时给「正在使用」；
   * 不命中 / 没应用 = undefined。
   * 底账没说"正在使用"时这块**什么都不标** —— 卡片只回答"这份 combo 的预设文件是什么"。
   *
   * 2026-10-13：「正在使用 · 我的文件」那个来源后缀去掉（作者：「我的一份、官方一份
   * 这些相关的都去掉」），不分来源只说「正在使用」（见 PageHome 的 inUseNote）。
   */
  inUse?: string
}

/**
 * 预设文件那一小块：文件名 + 状态。
 *
 * 只有非 ready 才有额外视觉。默认（ready）就是文件名直接摆着 ——
 * 真实情况是连上就一瞬间下完，给正常路径加过场动画反而是假的。
 */
export default function PresetStack({ state, inUse }: PresetStackProps) {
  const name =
    state.status === 'ready' ? state.preset.name : state.status === 'idle' ? null : state.name
  const path = state.status === 'ready' ? state.preset.path : undefined

  return (
    <div className={s.stack} data-status={state.status}>
      <p className={s.head}>预设文件</p>

      {state.status === 'waiting' ? (
        <Skeleton width="min(160px, 100%)" label="正在连接云端" />
      ) : (
        <p className={s.name} title={path}>
          {name ?? '—'}
        </p>
      )}

      {state.status === 'waiting' && <p className={s.note}>连接云端…</p>}

      {/* A3：与预设页横幅同源的那句「正在使用」—— 底账说在用这份，这里就跟着说 */}
      {inUse !== undefined && state.status !== 'waiting' && (
        <p className={s.note}>{inUse}</p>
      )}

      {state.status === 'downloading' && (
        <span className={s.progress} role="img" aria-label="正在下载" />
      )}

      {/*
        只报状态，不给「重试」—— 试验场那颗按钮扳的是调试面板的假下载开关，不是真的重取。
        错误本身带着 traceId：产品要求它在错误面上露出来，否则日志里那条查不到人。
      */}
      {state.status === 'failed' && (
        <p className={s.note}>
          连接失败
          {state.error && <TraceTag traceId={state.error.traceId} />}
        </p>
      )}
    </div>
  )
}
