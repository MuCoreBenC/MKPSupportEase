/*
 * 「选择要跟随的更新」—— 官方改过的参数**一次看完、一次决定**。
 *
 * # 它回答什么
 *
 * 官方持续发布新默认值，但它**不许直接顶掉我的参数**。于是每一版发布之后，
 * 官方改过的那几项先是**待处理**：我这里列出来，用户勾选（采用官方新值）
 * 或者不勾（保持我现在用的值），一次落下去。
 *
 * 与行上那两个动作（「用新值」/「保持我的」）是**同一件事的两条路**：
 * 一屏能全看完时走这里，只想改一行时就地在那一行上改。两条路落的是同一个写口。
 *
 * # 默认勾选：我改过的默认不勾
 *
 * 判据是**这一项我动过没有** —— `mine`（我那份文件里写着的）与 `baselineOld`
 * （官方旧值）一样 = 我没动过 → 默认勾上（本来就该跟官方走）；不一样 = 我改过
 * → 默认不勾（不替用户放弃他自己的值）。
 *
 * # 三方怎么显示
 *
 *   我改过的项   我 20.0 · 官方 18.6 → 19.0     右边：「确认后：19.0 / 20.0」
 *   没改过的项   官方 18.6 → 19.0
 *
 * 「右边那一格」是这一屏独有的东西：勾 / 不勾会**当场**改变确认后的值，
 * 不必让人自己在脑子里推一遍。
 */

import { useEffect, useMemo, useState } from 'react'
import { Modal } from '../../components/modal'
import type { ParamDecisionKind, ParamSyncEntry, PresetParamSync } from '../../api'
import { valueText } from './useParams'
import type { ParamDef } from './useParams'
import s from './ParamSyncModal.module.css'

interface Props {
  open: boolean
  /** 这一份的官方更新账（`null` = 还没读到） */
  sync: PresetParamSync | null
  /** 正在落决定 —— 两颗底栏按钮据此禁用 */
  busy: boolean
  /** 参数定义：把值翻成人话（`18.6` → `18.6 mm`、`true` → `开启`） */
  defOf: (key: string) => ParamDef | undefined
  /** 遮罩挂哪一层；不给就原地渲染 */
  host?: HTMLElement | null
  /** 落一批决定（勾选的采用、没勾的保持） */
  onDecide: (keys: string[], kind: ParamDecisionKind) => void
  onClose: () => void
}

/** 这一项"我动过没有"：我那份文件里写着的与官方旧值一样就是没动过 */
function untouched(entry: ParamSyncEntry): boolean {
  return entry.mine === null || entry.mine === entry.baselineOld
}

export default function ParamSyncModal({
  open,
  sync,
  busy,
  defOf,
  host = null,
  onDecide,
  onClose,
}: Props) {
  const pending = useMemo(() => (sync?.entries ?? []).filter((e) => e.pending), [sync])
  const [checked, setChecked] = useState<Record<string, boolean>>({})

  /*
   * 每次打开（或账本身换了）按默认规矩重来一遍：**我动过的默认不勾**。
   * 不沿用上一次的选择 —— 那是上一批待处理项的事，套到新的一批上就是个静默的错误决定。
   */
  useEffect(() => {
    if (!open) return
    const next: Record<string, boolean> = {}
    for (const e of pending) next[e.key] = untouched(e)
    setChecked(next)
  }, [open, pending])

  const chosen = pending.filter((e) => checked[e.key] === true).length
  const all = pending.length
  const text = (key: string, raw: string): string => valueText(defOf(key), raw)
  const afterOf = (e: ParamSyncEntry): string => {
    if (checked[e.key] === true) {
      return e.officialNew === null ? '—' : text(e.key, e.officialNew)
    }
    return e.mine === null ? '—' : text(e.key, e.mine)
  }

  const toggleAll = () => {
    const target = !(chosen > 0 && chosen === all)
    const next: Record<string, boolean> = {}
    for (const e of pending) next[e.key] = target
    setChecked(next)
  }

  const confirm = () => {
    const adopt = pending.filter((e) => checked[e.key] === true).map((e) => e.key)
    const hold = pending.filter((e) => checked[e.key] !== true).map((e) => e.key)
    /* 勾了的先采用、没勾的保持 —— 两条都是一次写，顺序不影响结果 */
    if (adopt.length > 0) onDecide(adopt, 'adopt')
    if (hold.length > 0) onDecide(hold, 'hold')
    onClose()
  }

  return (
    <Modal
      open={open}
      title="选择要跟随的更新"
      subtitle={
        sync === null
          ? '正在读这一份的官方更新账…'
          : `官方最新版${
              sync.currentReleaseTime === null ? '' : ` ${sync.currentReleaseTime}`
            } · 有 ${all} 项官方改过了。勾选 = 用官方的新值，不勾 = 保持你现在用的值。`
      }
      size="lg"
      host={host}
      closeOnScrim={false}
      closeTitle="关掉（一项都还没处理，官方更新仍然等着）"
      onClose={onClose}
      footer={
        <>
          <button type="button" className={s.ghost} onClick={onClose}>
            取消
          </button>
          <button
            type="button"
            className={s.primary}
            disabled={busy || all === 0}
            onClick={confirm}
          >
            {busy ? '正在落…' : `应用这 ${all} 项`}
          </button>
        </>
      }
    >
      {all === 0 ? (
        <p className={s.empty}>
          这一份没有待处理的官方更新 —— 官方改过的都已经处理过了，或者官方这一版还没动过任何一项。
        </p>
      ) : (
        <>
          <div className={s.selectbar}>
            <label className={s.checkLabel}>
              <input
                type="checkbox"
                aria-label="全选"
                checked={chosen > 0 && chosen === all}
                ref={(el) => {
                  /* 「勾了一部分」那一档：原生复选框没有这个属性，只能现设 */
                  if (el !== null) el.indeterminate = chosen > 0 && chosen < all
                }}
                onChange={toggleAll}
              />
              全选
            </label>
            <span className={s.selHint}>
              已选 {chosen} / {all} 项
            </span>
          </div>

          <div className={s.rows}>
            {pending.map((e) => {
              const def = defOf(e.key)
              const on = checked[e.key] === true
              const mineText = e.mine === null ? null : text(e.key, e.mine)
              const diff =
                e.baselineOld !== null && e.officialNew !== null
                  ? `${text(e.key, e.baselineOld)} → ${text(e.key, e.officialNew)}`
                  : '官方这一项的值还没拿到'
              return (
                <div key={e.key} className={on ? `${s.row} ${s.rowOn}` : s.row}>
                  <label className={s.check}>
                    <input
                      type="checkbox"
                      aria-label={def?.label ?? e.key}
                      checked={on}
                      onChange={() => setChecked((cur) => ({ ...cur, [e.key]: !on }))}
                    />
                  </label>
                  <div className={s.what}>
                    <div className={s.name}>
                      {def?.label ?? e.key}
                      {def?.unit !== undefined && <span className={s.unit}>{def.unit}</span>}
                    </div>
                    <div className={s.vals}>
                      {mineText !== null && e.mine !== e.baselineOld && (
                        <>
                          我 <span className={s.mine}>{mineText}</span>
                          {' · '}
                        </>
                      )}
                      官方 {diff}
                    </div>
                  </div>
                  <div className={s.after}>
                    确认后：<span className={s.afterVal}>{afterOf(e)}</span>
                  </div>
                </div>
              )
            })}
          </div>
        </>
      )}
    </Modal>
  )
}
