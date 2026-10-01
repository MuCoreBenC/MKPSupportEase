/*
 * 预设文件那一小块 + 可选的三轴竖排。
 *
 * 「连接失败」那一行的「重试」按钮去掉了：它点的是试验场调试面板的假下载开关
 * （devStore.setPresetPhase），不是真的重取。连带去掉只为它存在的 canAct 参数。
 */

import type { PresetState } from '../calib/usePreset'
import TraceTag from '../components/TraceTag'
import Skeleton from './Skeleton'
import s from './PresetStack.module.css'

const AXIS_ROWS = [
  ['X 偏移', 'x'],
  ['Y 偏移', 'y'],
  ['Z 偏移', 'z'],
] as const

interface PresetStackProps {
  state: PresetState
  /**
   * 一并竖排 X / Y / Z。三种取值是三件不同的事：
   * `undefined` = 这一块不管三轴（露出卡里由 AxisBar 负责，它还要形变成横排）；
   * `null` = 摆出三行但还没有值；对象 = 有值。
   */
  axes?: { x: number; y: number; z: number } | null
}

/**
 * 预设文件那一小块：文件名 + 状态 + 可选的三轴竖排。
 *
 * 只有非 ready 才有额外视觉。默认（ready）就是文件名和数值直接摆着 ——
 * 真实情况是连上就一瞬间下完，给正常路径加过场动画反而是假的。
 */
export default function PresetStack({ state, axes }: PresetStackProps) {
  const name =
    state.status === 'ready' ? state.preset.name : state.status === 'idle' ? null : state.name
  const path = state.status === 'ready' ? state.preset.path : undefined
  /* 数值只认一个来源：ready 才有数。正在取给骨架，其余给横杠 ——
     拿上一份预设的数字淡一点接着显示，看着像"这就是当前值"，那是骗人 */
  const view =
    state.status === 'ready'
      ? 'value'
      : state.status === 'waiting' || state.status === 'downloading'
        ? 'loading'
        : 'blank'

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

      {axes !== undefined && (
        <dl className={s.axes}>
          {AXIS_ROWS.map(([label, k]) => (
            <div className={s.row} key={k}>
              <dt className={s.label}>{label}</dt>
              <dd className={s.value}>
                {view === 'value' && axes ? (
                  `${axes[k].toFixed(2)} mm`
                ) : view === 'loading' ? (
                  <Skeleton label="正在取预设" />
                ) : (
                  '—'
                )}
              </dd>
            </div>
          ))}
        </dl>
      )}
    </div>
  )
}
