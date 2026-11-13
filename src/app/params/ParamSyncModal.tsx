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
 * 云端改过的那几项先是**待处理**：我这里列出来，用户勾选（采用云端新值）；
 * **不勾 = 还没表态**，它继续待处理，下次还能再选。
 *
 * ★ **这一屏不再写盘**（2026-10-20 作者：「应该保存的时候才算真的用，跟编辑一样」）：
 *   点「应用」只是把勾上的项写进**草稿**（一次撤销能整批退回来）；真正的落盘在
 *   「保存」那一下。点「暂不修改」则什么都不记。
 *
 * # 已处理的那几项也留在这里
 *
 * 保存之后它不再是待处理，但**不许从列表里消失** —— 重开这一屏就是来确认
 * 「上次我收的 / 否的是哪一个」的。所以末尾有一组「上次已处理」：采用 = 绿行，
 * 保持 = 灰行。**行上不再显示保持的痕迹**（作者：一直挂着徽章等于一直提醒"还有新值"），
 * 想改用新值就在这一屏点那一行的「改用新值」。云端再发新版时水位不在当前版，
 * 这一组自动清空。
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
/* 云端发布时刻是 ISO/UTC，副标题上要按本机时区写成「2026-11-10 17:19」—— 与预设页同一支格式化 */
import { longStatText } from '../store/package'
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
   * 把这一批**勾选的**交回去（未勾的不送 = 还没表态，继续待处理）—— **只写进草稿**，
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
  /*
   * **这一版已经处理过的项**（保存之后它不再是待处理，但 `decided` 还记着表态）。
   *
   * 必须留在列表里：重开这一屏就是来确认"上次我收的 / 否的是哪一个"的，直接消失
   * 等于把刚做过的事抹掉。**只读**：不给勾、不再交回去。
   * 采用过的排前面（它改了文件，是最该被确认的那一笔）。
   * 云端再发新版 ⇒ 水位不在当前版，`decided` 自动归空、这一组跟着消失：
   * 下一轮拿新的当前版重比，不把这一批的结论带过去。
   */
  const done = useMemo(() => {
    const rows = (sync?.entries ?? []).filter((e) => !e.pending && e.decided !== null)
    return [...rows.filter((e) => e.decided === 'adopt'), ...rows.filter((e) => e.decided !== 'adopt')]
  }, [sync])
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
   * 这一趟「应用」会落几笔「保持原值」：**机器专属那几项没勾的**（默认保持）。
   * 其余没勾的照旧不算 —— 它们还没表态，这一屏不该替用户拒绝。
   */
  const machineHolds = pending.filter(
    (e) => checked[e.key] !== true && neverPreset(e.key),
  ).length
  /** 这一屏关掉之后仍会挂着「待处理」的项数（给按钮 tooltip 用） */
  const stillPending = pending.filter(
    (e) => checked[e.key] !== true && !neverPreset(e.key),
  ).length
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
  const text = (key: string, raw: string): string => valueText(defOf(key), raw, tr)
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

  /*
   * 交回去的是两批：
   *
   *   · **勾上的** ⇒ 采用；
   *   · **机器专属那几项没勾** ⇒ 保持原值（它们的数是逐台标定出来的，云端换默认值
   *     本来就不该动它 —— 所以这几项不再"没勾就一直挂着"，而是默认保持）。
   *
   * 其余没勾的（没冲突 / 有冲突那两组）**什么都不交** = 还没表态，继续待处理。
   * 把那些也补成 `hold` 会让"勾 1 项"实际提交 1 adopt + N-1 hold，`remaining` 归零、
   * 来源记录被推进，剩下那几项再也回不来。
   * 行上显式点过的「保持原样」不靠这一屏补发 —— 它已经在 `syncDecisions` 里，
   * 保存那一趟会带上（那是用户真做过的决定，与"没勾"不是一回事）。
   */
  const confirm = () => {
    onDecide([
      ...pending
        .filter((e) => checked[e.key] === true)
        .map((e) => ({ paramKey: e.key, kind: 'adopt' as const })),
      ...pending
        .filter((e) => checked[e.key] !== true && neverPreset(e.key))
        .map((e) => ({ paramKey: e.key, kind: 'hold' as const })),
    ])
    onClose()
  }

  return (
    <Modal
      open={open}
      /* 待选清零（从「更多」进来复查）时不再叫「去选择」—— 没有要选的东西 */
      title={all === 0 && done.length > 0 ? tr('params.sync.titleView') : tr('params.sync.title')}
      subtitle={
        /*
         * 2026-10-13 口径：只有「本地 / 云端」（作者：「没有我的、没有官方这种说法」——
         * 那个"官方"就是现在说的交付预设那一档）。
         *
         * ★ 2026-10-20 又砍一刀（作者：「那个副标题有没有说不要这么多文字呀」）：
         *   原来那串「勾上的那几项改成云端新值，没勾的保持你现在用的值」是**重复**——
         *   每一行右边都写着「确认后：X」，勾与不勾的结果一看就懂。副标题只留
         *   「哪一版 + 改了几项」，一行说得完。
         *
         * 项数 = 还要选的 + 已经处理过的：这一版一共改了几项，不该因为处理过几项就变小。
         */
        sync === null
          ? tr('params.sync.reading')
          : tr('params.sync.summary', {
              /* 发布时刻是 RFC3339/UTC：直接摆上去就是 `2026-11-10T17:19:31Z`，
                 副标题一上来先念一串机器时间。按本机时区写成 `2026-11-10 17:19`。 */
              release:
                longStatText(sync.currentReleaseTime ?? undefined) ??
                tr('params.sync.newRelease'),
              count: all + done.length,
            })
      }
      /*
       * **`md`（480px）打底 + 这一屏自己抬到 560**（2026-10-29 作者：「这个模态框可以再大
       * 一点吧？现在有点小」）：2026-10-20 嫌 `lg` 的 680 太横，现在 480 又太小 ⇒ 取中间。
       * 宽度档仍归 `md`，560 是 `.syncBox` 上的 `min-width`（见那个类的注释）。
       * 高度不由框定 —— 交给列表区自己管（见 `.list` 的上下限），底栏永远贴着框底。
       */
      size="md"
      boxClass={s.syncBox}
      host={host}
      closeOnScrim={false}
      /* 「一项都还没处理」那句只在真有待选时成立；复查态（待选 0）退回「关闭」 */
      closeTitle={all > 0 ? tr('params.sync.closeTitle') : undefined}
      onClose={onClose}
      footer={
        <>
          {/*
           * **底栏左侧那句提示**（2026-10-20）：原来这一格是空的。
           * 「暂不修改」是什么意思、0 项应用会发生什么，都写在这里 ——
           * 不再靠页面底部那条一次性 toast（关了就看不到了）。
           * 没有待选（从「更多」进来复查）时不摆 —— 那时只有"关掉"这一个动作。
           */}
          {all > 0 && <span className={s.hint}>{hint ?? tr('params.sync.hint')}</span>}
          <button type="button" className={s.ghost} onClick={onClose}>
            {all > 0 ? tr('params.sync.later') : tr('params.sync.close')}
          </button>
          {all > 0 && (
            <button
              type="button"
              className={s.primary}
              disabled={fetching}
              /* 没勾的那几项**不处理**：这里说清，别让人以为"点一下全处理了" */
              title={
                stillPending > 0
                  ? tr('params.sync.applyRest', { count: stillPending })
                  : undefined
              }
              onClick={confirm}
            >
              {/* 有几笔默认保持时，说"勾选的 N 项"就漏了它们 */}
              {machineHolds > 0
                ? tr('params.sync.applyN', { count: chosen + machineHolds })
                : tr('params.sync.apply', { count: chosen })}
            </button>
          )}
        </>
      }
    >
      {fetching ? (
        <p className={s.empty}>{tr('params.sync.loading')}</p>
      ) : all === 0 && done.length === 0 ? (
        <p className={s.empty}>{tr('params.sync.nothing')}</p>
      ) : (
        <>
          {all > 0 && (
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
          )}

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
                    /* 「上次云端」用滚动记录；没有才退回水位那格（2026-10-31） */
                    const cloudOld = e.origin ?? e.baselineOld
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
                          {/*
                           * 三方值这一行：**标签是小的、值是大的**。
                           * 旧值与新值分开着色 —— `0.00 mm → -5.00 mm` 连成一串时，
                           * 谁是被换掉的那个、哪一个是云端新值，得读箭头两遍才认得出。
                           */}
                          <div className={s.vals}>
                            {localText !== null && e.local !== cloudOld && (
                              <>
                                <span className={s.valsTag}>{tr('params.sync.local')}</span>{' '}
                                <span className={s.local}>{localText}</span>
                                <i className={s.dot} aria-hidden="true">
                                  ·
                                </i>
                              </>
                            )}
                            {cloudOld !== null && e.cloudNew !== null ? (
                              <>
                                <span className={s.valsTag}>{tr('params.sync.cloud')}</span>{' '}
                                <span className={s.oldVal}>{text(e.key, cloudOld)}</span>
                                <i className={s.arrow} aria-hidden="true">
                                  →
                                </i>
                                <span className={s.newVal}>{text(e.key, e.cloudNew)}</span>
                              </>
                            ) : (
                              tr('params.sync.unknownCloud')
                            )}
                          </div>
                        </div>
                        {/* 「确认后」与参数名同一条基线；勾 / 不勾当场换值、也换这一格的轻重 */}
                        <div className={s.after} data-on={on ? 'true' : 'false'}>
                          <span className={s.afterLabel}>{tr('params.sync.after')}</span>
                          <span className={s.afterVal}>{afterOf(e)}</span>
                        </div>
                      </div>
                    )
                  })}
                </div>
              </div>
            ))}

            {/* 上次已处理的那几项：绿 = 收了的，灰 = 保持原样的 —— 排在最下面，不挡这一轮 */}
            {done.length > 0 && (
              <div className={s.group}>
                <div className={s.groupLabel} data-group="done">
                  {tr('params.sync.doneGroup')}
                </div>
                <div className={s.rows}>
                  {done.map((e) => {
                    const def = defOf(e.key)
                    const adopted = e.decided === 'adopt'
                    /* 「上次云端」用滚动记录；没有才退回水位那格（采用后它等于新值） */
                    const before = e.origin ?? e.baselineOld
                    return (
                      <div
                        key={e.key}
                        className={adopted ? `${s.row} ${s.rowDone}` : s.row}
                      >
                        <span
                          className={adopted ? s.doneMark : s.heldMark}
                          aria-hidden="true"
                        >
                          {adopted ? '✓' : '—'}
                        </span>
                        <div className={s.what}>
                          <div className={s.name}>
                            {def?.label ?? e.key}
                            {def?.unit !== undefined && <span className={s.unit}>{def.unit}</span>}
                            <span className={adopted ? s.doneTag : s.heldTag}>
                              {adopted
                                ? tr('params.sync.adoptedShort')
                                : tr('params.syncHold')}
                            </span>
                          </div>
                          <div className={s.vals}>
                            {e.cloudNew === null ? (
                              tr('params.sync.unknownCloud')
                            ) : adopted ? (
                              <>
                                {/* 采用过的那一行：云端旧值 → 云端新值（收的就是后者，绿） */}
                                <span className={s.valsTag}>{tr('params.sync.cloud')}</span>{' '}
                                {before !== null && before !== e.cloudNew && (
                                  <>
                                    <span className={s.oldVal}>{text(e.key, before)}</span>
                                    <i className={s.arrow} aria-hidden="true">
                                      →
                                    </i>
                                  </>
                                )}
                                <span className={s.doneNew}>{text(e.key, e.cloudNew)}</span>
                              </>
                            ) : (
                              <>
                                {/* 保持过的那一行：本地值 → 云端值 —— 拒绝的是哪个一眼看得出 */}
                                <span className={s.valsTag}>{tr('params.sync.local')}</span>{' '}
                                <span className={s.local}>
                                  {e.local === null ? '—' : text(e.key, e.local)}
                                </span>
                                <i className={s.arrow} aria-hidden="true">
                                  →
                                </i>
                                <span className={s.valsTag}>{tr('params.sync.cloud')}</span>{' '}
                                <span className={s.newVal}>{text(e.key, e.cloudNew)}</span>
                              </>
                            )}
                          </div>
                        </div>
                        <div className={s.after} data-on={adopted ? 'true' : 'false'}>
                          <span className={s.afterLabel}>{tr('params.sync.now')}</span>
                          <span className={s.afterVal}>{afterOf(e)}</span>
                          {/*
                            保持过的那几项：**行上不再给回头路**，改在这里 ——
                            点它把云端新值写进草稿（保存才落盘），这一屏随即关掉。
                          */}
                          {!adopted && (
                            <button
                              type="button"
                              className={s.rowAdopt}
                              disabled={e.cloudNew === null}
                              onClick={() => {
                                onDecide([{ paramKey: e.key, kind: 'adopt' }])
                                onClose()
                              }}
                            >
                              {tr('params.row.adoptShort')}
                            </button>
                          )}
                        </div>
                      </div>
                    )
                  })}
                </div>
              </div>
            )}
          </div>
        </>
      )}
    </Modal>
  )
}
