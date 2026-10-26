/*
 * 「选择要更新的参数」—— 云端改过的参数**一次看完、一次决定**。
 *
 * ★ 2026-10-20 改过一次文案：原来叫「选择要跟随的更新」（作者：「选择要跟随的更新
 * 这是哪里的文案，需要修改，选择要更新的参数的意思吧」）—— "跟随"讲的是机制
 * （水位跟到哪一版），而用户点它时想的是"哪几项要改成云端的新值"。
 *
 * # 它回答什么
 *
 * 云端持续发布新默认值，但它**不许直接顶掉本地参数**。于是每一版发布之后，
 * 云端改过的那几项先是**待处理**：我这里列出来，用户勾选（采用云端新值）
 * 或者不勾（保持我现在用的值），一次交回去。
 *
 * ★ **这一屏不再写盘**（2026-10-20 作者：「应该保存的时候才算真的用，跟编辑一样」）：
 *   点「应用」只是把勾上的项写进**草稿**（一次撤销能整批退回来）、没勾的记成
 *   「保持原样」；真正的落盘在「保存」那一下。点「暂不修改」则什么都不记，
 *   回到修改参数里逐项照旧能选。
 *
 * 与行上那两个动作（「用新值」/「保持原样」）是**同一件事的两条路**：
 * 一屏能全看完时走这里，只想改一行时就地在那一行上改。两条路落的是同一张草稿。
 *
 * # 三组 + 默认勾选（2026-10-20 作者定的顺序）
 *
 *   ① **没冲突**（云端改了、我没动过）—— **排最上、默认勾上**，这是"本来就该跟走"的一批；
 *   ② **有冲突**（我也改过，两边值还不一样）—— 排中间、默认不勾，必须我自己选；
 *   ③ [`NEVER_PRESET_KEYS`]（XYZ 偏移 / 涂胶速度）—— 排最下、一律不勾、**全选也跳过**：
 *      笔尖偏移是逐台机器标定出来的、涂胶速度是自己调出来的手感，云端换默认值
 *      不等于"这台也该换"。
 *
 * # 三方怎么显示
 *
 *   我改过的项   我 20.0 · 云端 18.6 → 19.0     右边：「确认后：19.0 / 20.0」
 *   没改过的项   云端 18.6 → 19.0
 *
 * 「右边那一格」是这一屏独有的东西：勾 / 不勾会**当场**改变确认后的值，
 * 不必让人自己在脑子里推一遍。
 */

import { useEffect, useMemo, useState } from 'react'
import { useT, type StringKey } from '../../i18n'
import { Modal } from '../../components/modal'
/* 勾选共用件（2026-10-17「统一全站的勾和框」）：行勾 / 全选那只原生绿勾换成墨色，
   半选（`indeterminate`）也由它现设 —— 见它头注释 */
import { Check } from '../../components/check'
import type { ParamDecision, ParamSyncEntry, PresetParamSync } from '../../api'
import { valueText } from './useParams'
import type { ParamDef } from './useParams'
import s from './ParamSyncModal.module.css'

/**
 * **不替用户勾的那几项**（2026-10-20）：XYZ 轴偏移 + 涂胶速度限制。
 *
 * 这两类是**机器自己的数** —— 笔尖偏移是逐台标定出来的、涂胶速度是自己调出来的手感，
 * 云端换默认值不等于"用户这台也该换"。所以：默认不勾、**全选也跳过**、
 * 单独一组排在末尾（要跟自己勾）。
 */
const NEVER_PRESET_KEYS: readonly string[] = [
  'toolhead.offset.x',
  'toolhead.offset.y',
  'toolhead.offset.z',
  'toolhead.speed_limit',
]

/** 这一组的小标题（单独分组排末尾，让人一眼看出它们为什么要分开） */
const NEVER_GROUP_LABEL: StringKey = 'params.sync.neverGroup'
/** 「没冲突」那一组的小标题 —— 排最上，默认勾上 */
const CLEAN_GROUP_LABEL: StringKey = 'params.sync.cleanGroup'
/** 「有冲突」那一组的小标题 —— 排中间，必须自己选 */
const CONFLICT_GROUP_LABEL: StringKey = 'params.sync.conflictGroup'

interface Props {
  open: boolean
  /** 这一份的云端更新账（`null` = 还没读到） */
  sync: PresetParamSync | null
  /** 参数定义：把值翻成人话（`18.6` → `18.6 mm`、`true` → `开启`） */
  defOf: (key: string) => ParamDef | undefined
  /** 遮罩挂哪一层；不给就原地渲染 */
  host?: HTMLElement | null
  /** 底栏**左侧**那句提示（不写就给「暂不修改」那句常驻说明） */
  hint?: string | null
  /**
   * 这一项在草稿层已经表过什么态（可选）。
   *
   * 打开时按它打勾：已经在行上点过「用新值」的项不能被这一屏的默认勾选逻辑
   * 当成"没表态"再问一遍 —— 用户一点「应用」，没勾的那些会被写成"保持原样"，
   * 反而把他刚做的采用推翻。
   */
  draftOf?: (key: string) => 'adopt' | 'hold' | null
  /**
   * 把这一批勾选交回去（勾上的采用、没勾的保持）—— **只写进草稿**，
   * 「保存」才落盘。空数组 = 一项都没勾。
   */
  onDecide: (decisions: ParamDecision[]) => void
  onClose: () => void
}

/** 这一项是不是"不替用户勾"的那一类 */
function neverPreset(key: string): boolean {
  return NEVER_PRESET_KEYS.includes(key)
}

/**
 * 一批待处理项分三组（顺序就是界面顺序）：
 *   ① 没冲突（默认勾上）② 有冲突（自己选）③ 机器自己的那几项（不替你勾）
 *
 * "有没有冲突"由后端算好放在 `conflict` 上，这一层不再自己比三方值 ——
 * 判据只有一处（`param_sync::ParamSyncEntry::is_conflict`），两边各算一套必漂。
 */
function groupOf(pending: ParamSyncEntry[]): {
  key: string
  label: StringKey
  items: ParamSyncEntry[]
}[] {
  const machine = pending.filter((e) => neverPreset(e.key))
  const rest = pending.filter((e) => !neverPreset(e.key))
  const clean = rest.filter((e) => !e.conflict)
  const conflict = rest.filter((e) => e.conflict)
  const groups: { key: string; label: StringKey; items: ParamSyncEntry[] }[] = []
  if (clean.length > 0) groups.push({ key: 'clean', label: CLEAN_GROUP_LABEL, items: clean })
  if (conflict.length > 0) {
    groups.push({ key: 'conflict', label: CONFLICT_GROUP_LABEL, items: conflict })
  }
  if (machine.length > 0) groups.push({ key: 'machine', label: NEVER_GROUP_LABEL, items: machine })
  return groups
}

export default function ParamSyncModal({
  open,
  sync,
  defOf,
  host = null,
  hint = null,
  draftOf,
  onDecide,
  onClose,
}: Props) {
  const tr = useT()
  const pending = useMemo(() => (sync?.entries ?? []).filter((e) => e.pending), [sync])
  const groups = useMemo(() => groupOf(pending), [pending])
  const [checked, setChecked] = useState<Record<string, boolean>>({})

  /*
   * 每次打开（或账本身换了）按默认规矩重来一遍：**已经表过态的照旧**（行上点过
   * 「用新值 / 保持原样」不能被这一屏推翻），其余**没冲突的默认勾上**，
   * 冲突与机器自己的那几项一律不勾。
   * 不沿用上一次弹窗里的勾选 —— 那是上一批待处理项的事，套到新的一批上就是个
   * 静默的错误决定；这里沿用的是**这一屏之外的草稿表态**，那是用户刚做的决定。
   */
  useEffect(() => {
    if (!open) return
    const next: Record<string, boolean> = {}
    for (const e of pending) {
      const decided = draftOf?.(e.key) ?? null
      next[e.key] = decided === 'adopt' ? true : decided === 'hold' ? false : !e.conflict && !neverPreset(e.key)
    }
    setChecked(next)
  }, [open, pending, draftOf])

  const chosen = pending.filter((e) => checked[e.key] === true).length
  const all = pending.length
  /*
   * **云端这一版的字节还没取回来**（从预设页「更新」顺势进来时常见）：
   * 那一瞬间 `pendingCount` 天然是 0（缺新值就不报待处理）——
   * 那不是"这一版没有要改值的参数"，不许拿它显示「应用这 0 项」。
   */
  const fetching = sync !== null && sync.versionAdvanced && !sync.cloudReady
  /* 「全选」只覆盖**没冲突**那一组（冲突要自己选、机器自己的数从不替用户勾） */
  const selectable = pending.filter((e) => !e.conflict && !neverPreset(e.key)).length
  /*
   * 全选框的三种状态只看**它能勾的那一批**：用户手动勾了某个冲突项之后，
   * 全选框不该变成"没勾"（那会让人以为自己勾的东西又没了）。
   */
  const cleanChosen = pending.filter(
    (e) => checked[e.key] === true && !e.conflict && !neverPreset(e.key),
  ).length
  const text = (key: string, raw: string): string => valueText(defOf(key), raw)
  const afterOf = (e: ParamSyncEntry): string => {
    if (checked[e.key] === true) {
      return e.cloudNew === null ? '—' : text(e.key, e.cloudNew)
    }
    return e.local === null ? '—' : text(e.key, e.local)
  }

  /* 全选**只动没冲突那一组**：冲突与机器自己的数不替用户勾，也不因为全选被清掉 */
  const toggleAll = () => {
    const target = cleanChosen < selectable
    setChecked((cur) => {
      const next = { ...cur }
      for (const e of pending) {
        if (!e.conflict && !neverPreset(e.key)) next[e.key] = target
      }
      return next
    })
  }

  /* 交回一批决定：勾上的采用、没勾的保持 —— 只写进草稿，「保存」才落盘 */
  const confirm = () => {
    onDecide(
      pending.map((e) => ({
        paramKey: e.key,
        kind: checked[e.key] === true ? 'adopt' : 'hold',
      })),
    )
    onClose()
  }

  return (
    <Modal
      open={open}
      title={tr('params.sync.title')}
      subtitle={
        /*
         * 2026-10-13 口径：只有「本地 / 云端」（作者：「没有我的、没有官方这种说法」——
         * 那个"官方"就是现在说的交付预设那一档）。
         *
         * ★ 2026-10-20 又砍一刀（作者：「那个副标题有没有说不要这么多文字呀」）：
         *   原来那串「勾上的那几项改成云端新值，没勾的保持你现在用的值」是**重复**——
         *   每一行右边都写着「确认后：X」，勾与不勾的结果一看就懂。副标题只留
         *   「哪一版 + 改了几项」，一行说得完。
         */
        sync === null
          ? tr('params.sync.reading')
          : tr('params.sync.summary', {
              release: sync.currentReleaseTime ?? tr('params.sync.newRelease'),
              count: all,
            })
      }
      /*
       * **`md`（480px）**：这一屏一行就三项，`lg` 的 680px 太横（2026-10-20 作者：
       * 「这个模太宽，搞这么大干嘛？」）。高度不由框定 —— 交给列表区自己管
       * （见 `.rows` 的上下限），底栏永远贴着框底。
       *
       * `boxClass` 只用来把框身顶部那条内衬收掉（让「全选」往上贴），不参与尺寸。
       */
      size="md"
      boxClass={s.syncBox}
      host={host}
      closeOnScrim={false}
      closeTitle={tr('params.sync.closeTitle')}
      onClose={onClose}
      footer={
        <>
          {/*
           * **底栏左侧那句提示**（2026-10-20）：原来这一格是空的。
           * 「暂不修改」是什么意思、0 项应用会发生什么，都写在这里 ——
           * 不再靠页面底部那条一次性 toast（关了就看不到了）。
           */}
          <span className={s.hint}>{hint ?? tr('params.sync.hint')}</span>
          <button type="button" className={s.ghost} onClick={onClose}>
            {tr('params.sync.later')}
          </button>
          <button type="button" className={s.primary} disabled={fetching} onClick={confirm}>
            {tr('params.sync.apply', { count: chosen })}
          </button>
        </>
      }
    >
      {fetching ? (
        <p className={s.empty}>{tr('params.sync.loading')}</p>
      ) : all === 0 ? (
        <p className={s.empty}>{tr('params.sync.nothing')}</p>
      ) : (
        <>
          <div className={s.selectbar}>
            <Check
              ariaLabel={tr('params.sync.selectAll')}
              className={s.checkLabel}
              checked={selectable > 0 && cleanChosen === selectable}
              indeterminate={cleanChosen > 0 && cleanChosen < selectable}
              onChange={toggleAll}
            >
              {tr('params.sync.selectAll')}
            </Check>
            <span className={s.selHint}>
              {tr('params.sync.chosen', { chosen, all })}
            </span>
          </div>

          {/* 三组共用一个滚动区（一组一个滚动框的话，三项就能把弹窗撑成半屏） */}
          <div className={s.list}>
            {groups.map((g) => (
              <div key={g.key} className={s.group}>
                <div className={s.groupLabel} data-group={g.key}>
                  {tr(g.label)}
                </div>
                <div className={s.rows}>
                  {g.items.map((e) => {
                    const def = defOf(e.key)
                    const on = checked[e.key] === true
                    const localText = e.local === null ? null : text(e.key, e.local)
                    const diff =
                      e.baselineOld !== null && e.cloudNew !== null
                        ? `${text(e.key, e.baselineOld)} → ${text(e.key, e.cloudNew)}`
                        : tr('params.sync.unknownCloud')
                    return (
                      <div key={e.key} className={on ? `${s.row} ${s.rowOn}` : s.row}>
                        <Check
                          ariaLabel={def?.label ?? e.key}
                          checked={on}
                          onChange={() => setChecked((cur) => ({ ...cur, [e.key]: !on }))}
                        />
                        <div className={s.what}>
                          <div className={s.name}>
                            {def?.label ?? e.key}
                            {def?.unit !== undefined && <span className={s.unit}>{def.unit}</span>}
                          </div>
                          <div className={s.vals}>
                            {localText !== null && e.local !== e.baselineOld && (
                              <>
                                本地 <span className={s.local}>{localText}</span>
                                {' · '}
                              </>
                            )}
                            云端 {diff}
                          </div>
                        </div>
                        <div className={s.after}>
                          确认后：<span className={s.afterVal}>{afterOf(e)}</span>
                        </div>
                      </div>
                    )
                  })}
                </div>
              </div>
            ))}
          </div>
        </>
      )}
    </Modal>
  )
}
