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
 * 或者不勾（保持我现在用的值），一次落下去。
 *
 * 与行上那两个动作（「用新值」/「保持本地」）是**同一件事的两条路**：
 * 一屏能全看完时走这里，只想改一行时就地在那一行上改。两条路落的是同一个写口。
 *
 * # 默认勾选：我改过的默认不勾；**XYZ 偏移与涂胶速度一律不替他勾**（2026-10-20）
 *
 * 判据是**这一项我动过没有** —— `local`（我那份文件里写着的）与 `baselineOld`
 * （云端旧值）一样 = 我没动过 → 默认勾上（本来就该跟云端走）；不一样 = 我改过
 * → 默认不勾（不替用户放弃他自己的值）。
 *
 * 那两类（[`NEVER_PRESET_KEYS`]）额外一律不勾、**全选也跳过它们**，并且单独一组
 * 排在末尾：笔尖偏移是逐台机器标定出来的、涂胶速度是自己调出来的手感 ——
 * 云端改了默认值也不该顺手把机器自己的值换掉。要跟就自己勾。
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
const NEVER_GROUP_LABEL = '机器自己的数（不替你勾）'

interface Props {
  open: boolean
  /** 这一份的云端更新账（`null` = 还没读到） */
  sync: PresetParamSync | null
  /** 正在落决定 —— 两颗底栏按钮据此禁用 */
  busy: boolean
  /** 参数定义：把值翻成人话（`18.6` → `18.6 mm`、`true` → `开启`） */
  defOf: (key: string) => ParamDef | undefined
  /** 遮罩挂哪一层；不给就原地渲染 */
  host?: HTMLElement | null
  /** 底栏**左侧**那句提示（不写就给「暂不修改」那句常驻说明） */
  hint?: string | null
  /**
   * 落一批决定（勾选的采用、没勾的保持）—— **一次写完**（可混合 adopt / hold）。
   * 空数组 = 「应用这 0 项」（值一个不动，把新版的注释 / 新字段 / 时间戳收进来）。
   */
  onDecide: (decisions: ParamDecision[]) => void
  onClose: () => void
}

/** 这一项"我动过没有"：我那份文件里写着的与云端旧值一样就是没动过 */
function untouched(entry: ParamSyncEntry): boolean {
  return entry.local === null || entry.local === entry.baselineOld
}

/** 这一项是不是"不替用户勾"的那一类 */
function neverPreset(key: string): boolean {
  return NEVER_PRESET_KEYS.includes(key)
}

/** 一批待处理项分成两组：普通项在前，**机器自己的那几项**在末尾 */
function groupOf(pending: ParamSyncEntry[]): { label: string | null; items: ParamSyncEntry[] }[] {
  const normal = pending.filter((e) => !neverPreset(e.key))
  const held = pending.filter((e) => neverPreset(e.key))
  const groups: { label: string | null; items: ParamSyncEntry[] }[] = []
  if (normal.length > 0) groups.push({ label: null, items: normal })
  if (held.length > 0) groups.push({ label: NEVER_GROUP_LABEL, items: held })
  return groups
}

export default function ParamSyncModal({
  open,
  sync,
  busy,
  defOf,
  host = null,
  hint = null,
  onDecide,
  onClose,
}: Props) {
  const pending = useMemo(() => (sync?.entries ?? []).filter((e) => e.pending), [sync])
  const groups = useMemo(() => groupOf(pending), [pending])
  const [checked, setChecked] = useState<Record<string, boolean>>({})

  /*
   * 每次打开（或账本身换了）按默认规矩重来一遍：**我动过的默认不勾**、
   * **机器自己的那几项（偏移 / 涂胶速度）一律不勾**。
   * 不沿用上一次的选择 —— 那是上一批待处理项的事，套到新的一批上就是个静默的错误决定。
   */
  useEffect(() => {
    if (!open) return
    const next: Record<string, boolean> = {}
    for (const e of pending) next[e.key] = untouched(e) && !neverPreset(e.key)
    setChecked(next)
  }, [open, pending])

  const chosen = pending.filter((e) => checked[e.key] === true).length
  const all = pending.length
  /*
   * **云端这一版的字节还没取回来**（从预设页「更新」顺势进来时常见）：
   * 那一瞬间 `pendingCount` 天然是 0（缺新值就不报待处理）——
   * 那不是"这一版没有要改值的参数"，不许拿它显示「应用这 0 项」。
   */
  const fetching = sync !== null && sync.versionAdvanced && !sync.cloudReady
  /* 「全选」能勾到的那几项（偏移 / 涂胶速度那一组不算在内 —— 它们从不替用户勾） */
  const selectable = pending.filter((e) => !neverPreset(e.key)).length
  const text = (key: string, raw: string): string => valueText(defOf(key), raw)
  const afterOf = (e: ParamSyncEntry): string => {
    if (checked[e.key] === true) {
      return e.cloudNew === null ? '—' : text(e.key, e.cloudNew)
    }
    return e.local === null ? '—' : text(e.key, e.local)
  }

  /* 全选**跳过**偏移与涂胶速度：那一组不替用户勾（要跟自己勾那几项） */
  const toggleAll = () => {
    const target = !(chosen > 0 && chosen === selectable)
    const next: Record<string, boolean> = {}
    for (const e of pending) next[e.key] = target && !neverPreset(e.key)
    setChecked(next)
  }

  /* 一次写完：勾上的采用、没勾的保持 —— 一趟写完血统才好收场（分两趟会有中间态） */
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
      title="选择要更新的参数"
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
          ? '正在读云端更新账…'
          : `云端 ${sync.currentReleaseTime ?? '新版'} · ${all} 项有改动`
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
      closeTitle="关掉（一项都还没处理，云端更新仍然等着）"
      onClose={onClose}
      footer={
        <>
          {/*
           * **底栏左侧那句提示**（2026-10-20）：原来这一格是空的。
           * 「暂不修改」是什么意思、0 项应用会发生什么，都写在这里 ——
           * 不再靠页面底部那条一次性 toast（关了就看不到了）。
           */}
          <span className={s.hint}>{hint ?? '关闭后仍可继续修改'}</span>
          <button type="button" className={s.ghost} onClick={onClose}>
            暂不修改
          </button>
          <button
            type="button"
            className={s.primary}
            disabled={busy || fetching}
            onClick={confirm}
          >
            {busy ? '正在落…' : `应用这 ${chosen} 项`}
          </button>
        </>
      }
    >
      {fetching ? (
        <p className={s.empty}>正在取回云端这一版 —— 拿到才知道改了哪几项。</p>
      ) : all === 0 ? (
        <p className={s.empty}>
          这一版没有要改值的参数 —— 点「应用这 0 项」把云端的注释、新字段与时间戳收进来，
          「有新版」就此收场。
        </p>
      ) : (
        <>
          <div className={s.selectbar}>
            <Check
              ariaLabel="全选"
              className={s.checkLabel}
              checked={selectable > 0 && chosen === selectable}
              indeterminate={chosen > 0 && chosen < selectable}
              onChange={toggleAll}
            >
              全选
            </Check>
            <span className={s.selHint}>
              已选 {chosen} / {all} 项
            </span>
          </div>

          {groups.map((g) => (
            <div key={g.label ?? 'normal'} className={s.group}>
              {g.label !== null && <div className={s.groupLabel}>{g.label}</div>}
              <div className={s.rows}>
                {g.items.map((e) => {
                  const def = defOf(e.key)
                  const on = checked[e.key] === true
                  const localText = e.local === null ? null : text(e.key, e.local)
                  const diff =
                    e.baselineOld !== null && e.cloudNew !== null
                      ? `${text(e.key, e.baselineOld)} → ${text(e.key, e.cloudNew)}`
                      : '云端这一项的值还没拿到'
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
        </>
      )}
    </Modal>
  )
}
