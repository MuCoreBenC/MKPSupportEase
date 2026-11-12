/*
 * 批量修改（对照模式右侧，与「参数详情」同一个右栏的两页签之一）（C14 移植）。
 *
 * 「**这一个参数**、在这几个版本上，都设成这个值」—— 一次动作、一条历史，
 * Ctrl+Z 整块退回来（撤销栈在壳上，按整块 inverse 回）。
 *
 * 与原型的分工：C14 的批量是前端直接写；产品的纪律是**先预览再落**
 * （「不许闷着改」）——「应用」先走 `wb_preview_bulk`：每一列的 before → after、
 * 哪几列被跳过（不适用 / 被条件关着）都由后端判好，人看过清单再「确认写入」，
 * 写的仍是**勾选的那些列各自那一层**（一次手势、一条历史）。
 *
 * 三条刻意的限制：
 *
 *   1. **机型基线也能当目标**（作者 2026-11-09）—— 勾了它就得改它。
 *      基线也在这一批里时，跟着它的版本写「跟随基线」（`value: null`），
 *      由基线把值带过去，不必每个版本各写一份相同的数。
 *   2. **gcode / 长文本 / 已弃用没有这一页**（调用方不给页签）
 *   3. **目标名字一律带机型**（作者：「把 A1 的补上」）
 */
import { useEffect, useMemo, useState } from 'react'

import {
  wb,
  type ParamView,
  type Patch,
  type Words,
  type BulkPreview,
  type BulkSkip,
  type AttachPreview,
} from '../api'
import { toasts } from '../c14/toast'
import ModalC14 from '../c14/ModalC14'
import { SelectField } from '../components/field'
import s from '../c14.module.css'

export interface BatchTarget {
  /** 列记号（后端重排后的 `Col.key`：机型 id 或版本 uid = patch 的 owner） */
  id: string
  /** chip 上的名字，**一律带机型** */
  name: string
  machineId: string
  uid: string | null
  /** 这一列在这一项上的**当前原始值**（矩阵那格给的）。预填「设置新值」、判「有没有变化」用它 */
  raw: unknown
  /**
   * 这一列**盘上**的值（`Cell.rest`）。`null` = 这一层原本没钉着它 ——
   * 「恢复」要写 `null` **删键**、让它挂回继承，不是写回一个值
   */
  rest: unknown
  /** 这一格有没有未保存的改动 —— 「恢复」只对改过的列有意义 */
  dirty: boolean
  /**
   * 这一层**正在跟随基线**（状态写在注册表 `[params.follows]` 里）。
   *
   * 「跟随基线」只对 `false` 的列有意义 —— 已经在跟随的列写下去是空操作。
   * 判据是矩阵那一格的 `Cell.badge === 'follow'`（`is_following` 的唯一实现），
   * 不按值猜、也不看这一层有没有自己的一行。机型基线是根，恒 `false`。
   */
  follows: boolean
}

interface Props {
  param: ParamView
  words: Words
  /** 勾选里的每一列（机型基线 + 版本，可跨机型） */
  targets: BatchTarget[]
  /** 唯一写入口（外壳的 run）。确认写入时带上整批 patch；返回这一笔写进去没有 */
  onApply: (label: string, patches: Patch[]) => Promise<boolean>
  /**
   * 滚到中间表格里这一参数那一行（对照模式下右栏遮着表格，不看一眼容易迷路）。
   * 不给就不画那颗按钮。
   */
  onLocate?: () => void
  /**
   * 被条件关着的列上那颗「去改「X」」：选中那个阻塞参数并滚到它那一行。
   * 不给就不画那颗按钮。
   */
  onGoFix?: (key: string) => void
}

/** 批量值按注册表的值类型归位（与单格写值同一条路） */
function coerce(param: ParamView, raw: string): unknown {
  if (param.uiComponent === 'switch') return raw === '开'
  if (param.valueType === 'bool') return raw === 'true'
  if ((param.valueType === 'float' || param.valueType === 'int') && raw !== '') return Number(raw)
  return raw
}

/**
 * 格子的原始值 → 「设置新值」那个控件认的原文。**与 `CellEditor` 同一条口径**
 * （那边是 `String(cell.raw ?? '')`），只有开关那一档要翻成开/关两个字。
 */
function toInput(param: ParamView, v: unknown): string {
  if (param.uiComponent === 'switch') return v === true || v === 'true' ? '开' : '关'
  return v === null || v === undefined ? '' : String(v)
}

export default function BatchEdit({ param, words, targets, onApply, onLocate, onGoFix }: Props) {
  const [pickedIds, setPickedIds] = useState<string[] | null>(null)
  /** 用户自己改过的值。`null` = 还没动过 → 显示各列的**共同当前值**（见 `commonRaw`） */
  const [rawEdit, setRawEdit] = useState<string | null>(null)
  const [preview, setPreview] = useState<BulkPreview | null>(null)
  const [previewValue, setPreviewValue] = useState<unknown>(null)
  /** 「跟随基线」的预览（点下去之前先看清哪几列真的会变） */
  const [attach, setAttach] = useState<AttachPreview | null>(null)
  /**
   * 预览框正在看哪一页：`set` = 要写的值，`base` = 只改基线的确认，`attach` = 挂回基线。
   *
   * 几页共用**同一个模态框** —— 叠两层的话，被压住的那层里的已填值与勾选看不见，
   * 看完"只改基线会怎样"想反悔得整个重来（作者 2026-11-09）。
   */
  const [pane, setPane] = useState<'set' | 'base' | 'attach'>('set')

  const pickedTargets = pickedIds ?? targets.map((v) => v.id)
  const picked = targets.filter((t) => pickedTargets.includes(t.id))

  /*
   * 各目标列在这一项上的当前值，按显示文本分组。
   *
   * 跨机型勾选时值常常不一样 —— 那时「设置新值」不能预填任何一个数（预填哪个
   * 都是撒谎），但也不能空着不说：作者 2026-11-09「如果它这个每个的值都不一样，
   * 你应该显示一种什么状态，而不是把它空着」。
   */
  const valueGroups = useMemo(() => {
    const map = new Map<string, { text: string; names: string[] }>()
    for (const t of targets) {
      const text = toInput(param, t.raw)
      const hit = map.get(text)
      if (hit) hit.names.push(t.name)
      else map.set(text, { text, names: [t.name] })
    }
    return [...map.values()]
  }, [param, targets])

  /*
   * 各目标列在这一项上的当前值**全都一样**时，就把它预填进「设置新值」——
   * 作者点开抽屉看到空框，而那一行每列本来就是同一个值（「但是这一行都是一样的呀」）。
   * 不一样就不预填：没有「共同的那个值」可言。矩阵刷新（比如写完之后）会重算。
   */
  const commonRaw = valueGroups.length === 1 ? (valueGroups[0]?.text ?? '') : ''

  /** 「设置新值」的占位：值不一致 / 没有目标 / 全空，各说各的，不混成一个空框 */
  const valuePlaceholder =
    targets.length === 0
      ? '先挑应用目标'
      : valueGroups.length >= 2
        ? `各不相同（${targets.length} 列 · ${valueGroups.length} 种值）`
        : valueGroups.length === 1 && (valueGroups[0]?.text ?? '') === ''
          ? '（勾选的列当前没有值）'
          : undefined

  /* 换了参数、或者矩阵刷新出了新的共同值 → 用户手改的那份作废，回到预填 */
  useEffect(() => {
    setRawEdit(null)
  }, [param.key, commonRaw])

  const raw = rawEdit ?? commonRaw

  const toggle = (id: string) => {
    setPickedIds(
      pickedTargets.includes(id)
        ? pickedTargets.filter((x) => x !== id)
        : [...pickedTargets, id],
    )
  }

  /** 全选 / 反选（拖选手势之外也要有明写的按钮） */
  const allOn = targets.length > 0 && targets.every((t) => pickedTargets.includes(t.id))

  const shownValue = (): string => {
    if (param.uiComponent === 'switch') return raw === '开' ? '开' : '关'
    const hit = param.choices.find((o) => String(o.value) === raw)
    return hit ? hit.label : raw || '（空）'
  }

  /**
   * 勾选的那些列，值**是不是都已经等于**要写的新值。是就没变化 ——
   * 「都说了让他和原始数据对比，变化了才有变化」（作者）：按钮该灰，不给点。
   */
  const noChange = picked.length > 0 && picked.every((t) => toInput(param, t.raw) === raw)

  /** 勾选里**改过**的列 —— 「恢复修改前的」只对它们有意义 */
  const restorable = picked.filter((t) => t.dirty)

  /**
   * 恢复修改前的：**按列各自退回改动前那一层**（作者：「要根据矩阵里面那一行的
   * 各自的值恢复改之前的」）。`rest` 为 `null` = 这一层原本就没钉着它 → 写 `null`
   * **删键**、让它挂回继承；写回一个值会把继承钉死，那是另一回事。
   */
  const restore = async () => {
    if (!restorable.length) return
    const patches: Patch[] = restorable.map((t) => ({
      kind: 'setValue',
      level: t.uid !== null ? 'version' : 'machine',
      owner: t.uid ?? t.machineId,
      key: param.key,
      value: t.rest,
    }))
    const ok = await onApply(`${param.label} 恢复修改前（${patches.length} 列）`, patches)
    if (ok) toasts.push(`已恢复 ${patches.length} 列`)
  }

  /** 应用 = 先预览（后端判每一列的 before/after 与跳过原因），人看过再写 */
  const apply = async () => {
    if (!picked.length || noChange) return
    const chosen = picked
    if (param.uiComponent === 'number' && (raw === '' || Number.isNaN(Number(raw)))) {
      toasts.push('先填一个数字再应用')
      return
    }
    const value = coerce(param, raw)
    /*
     * 手势前的那道弃用闸（后端也会拒，这里先说人话）。
     * 判据**按目标机型**：一条定义可以只在 A1 上退场 —— 选中的机器里只要有一台
     * 不能写，这一批就不能整批发（后端是整批拒绝，不是跳过那一台）。
     */
    const dead = param.choices.find((c) => {
      if (String(c.value) !== raw) return false
      if (c.deprecated) return true
      return targets.some(
        (t) => t.machineId !== '' && c.deprecatedMachines.includes(t.machineId),
      )
    })
    if (dead) {
      toasts.push(`${param.label} 的「${dead.label}」${words.disabled.deprecatedWriteBlocked}`)
      return
    }
    try {
      const out = await wb.previewBulk(
        param.key,
        value,
        chosen.map((t) => ({ machineId: t.machineId, versionUid: t.uid })),
      )
      setPreviewValue(value)
      setPane('set')
      setPreview(out)
    } catch {
      /* previewBulk 拒绝时后端已给过错误横幅（外壳 fail），这里不用再说什么 */
    }
  }

  /**
   * 确认写入：只写会变的列（noChange 的列让后端的「没变化不记草稿」去兜也行，但不发是无谓的手势）。
   *
   * 版本层的两种「跟随」都写 `null`，不再各自钉一个相同的数：
   *   · `followsBase` —— 这一批里连基线一起改了，版本跟着新基线走；
   *   · `sameAsBase`  —— 新值本来就等于现有基线，没有钉住的必要。
   * 这是作者 2026-11-09 的口径：「都是同样的值的时候，它完全可以跟随基线」。
   */
  const confirmWrite = async () => {
    if (!preview) return
    const byId = new Map(targets.map((t) => [t.id, t]))
    const patches: Patch[] = []
    /** 写「跟随」而不写值的版本列 */
    let followed = 0
    /** 本来就跟随、由这一批的基线带过去的列（不单独发 patch） */
    let carried = 0
    for (const effect of preview.effects) {
      if (effect.kind === 'noChange') continue
      const t = byId.get(effect.col)
      if (!t) continue
      /* 基线也在这一批里、而这一列本来就在跟随 ⇒ 基线一写它就跟着变，不必再发一条 */
      if (effect.followsBase && t.follows) {
        carried += 1
        continue
      }
      const follow = t.uid !== null && (effect.followsBase || effect.sameAsBase)
      if (follow) followed += 1
      patches.push({
        kind: 'setValue',
        level: t.uid !== null ? 'version' : 'machine',
        owner: t.uid ?? t.machineId,
        key: param.key,
        /* `null`（版本层）= 跟随基线，值同步成基线 —— 与「跟随基线」同一个语义 */
        value: follow ? null : previewValue,
      })
    }
    setPreview(null)
    if (!patches.length) {
      toasts.push('没有要写的列 —— 勾选的值都已经是这个值了')
      return
    }
    const pinned = patches.length - followed
    const ok = await onApply(
      `${param.label} 批量设为 ${shownValue()}（${pinned} 列${
        followed > 0 ? ` · ${followed} 列跟随基线` : ''
      }${carried > 0 ? ` · ${carried} 列由基线带过去` : ''}）`,
      patches,
    )
    if (ok) {
      toasts.push(
        `已写 ${pinned} 列${followed > 0 ? ` · ${followed} 列跟随基线` : ''}${
          carried > 0 ? ` · ${carried} 列由基线带过去` : ''
        }`,
      )
    }
  }

  /* 勾选里**还在自己钉着值**的版本列 —— 「重新跟随」只对它们有意义；基线是根 */
  const attachable = picked.filter((t) => t.uid !== null && !t.follows)
  /** 勾选里有几个版本列（基线不算）—— 决定灰按钮下面那句怎么说 */
  const pickedVersions = picked.filter((t) => t.uid !== null).length

  /**
   * 「跟随基线」的预览页确认后**只写机型基线**，版本列一个都不发。
   *
   * 作者 2026-11-09：「这些版本的值和基线的值，比如都是从 1 改到 1.2，
   * 点跟随基线，也就是说就只改基线，其他的值就不动」—— 跟着基线的版本自然变成
   * 1.2，自己钉着值的版本保持原样。点开先到 `pane='base'` 那一页预览，确认才写。
   */
  const writeBaseOnly = async () => {
    if (!preview) return
    const byId = new Map(targets.map((t) => [t.id, t]))
    const patches: Patch[] = []
    for (const effect of preview.effects) {
      if (effect.level !== 'machine' || effect.kind === 'noChange') continue
      const t = byId.get(effect.col)
      if (!t || t.uid !== null) continue
      patches.push({
        kind: 'setValue',
        level: 'machine',
        owner: t.machineId,
        key: param.key,
        value: previewValue,
      })
    }
    setPreview(null)
    setAttach(null)
    if (!patches.length) {
      toasts.push('这一批里没有要改的机型基线 —— 左边勾上机型基线再试')
      return
    }
    const ok = await onApply(
      `${param.label} 只改基线 ${shownValue()}（${patches.length} 台）`,
      patches,
    )
    if (ok) toasts.push(`已写 ${patches.length} 台机型的基线`)
  }

  /** 预览里有没有要改的机型基线（决定「跟随基线」那颗按钮亮不亮） */
  const baseEffects =
    preview?.effects.filter((e) => e.level === 'machine' && e.kind !== 'noChange') ?? []

  /**
   * 「重新跟随」：把勾选的版本列**挂回基线**（右侧面板那颗）。
   *
   * 写的是 `value: null`（版本层 = 跟随状态 + 值同步成基线有效值），与单格那颗
   * 「跟随 X」同一个语义。先过一遍后端预览（哪几列会变、变成多少），人看过再写。
   */
  const askAttach = async () => {
    if (!picked.length) return
    try {
      setAttach(
        await wb.previewAttach(
          param.key,
          picked.map((t) => ({ machineId: t.machineId, versionUid: t.uid })),
        ),
      )
      setPane('attach')
      setPreview(null)
    } catch {
      /* 后端拒绝时外壳已给过错误横幅（与 `apply` 同一条路） */
    }
  }

  const confirmAttach = async () => {
    if (!attach) return
    const byId = new Map(targets.map((t) => [t.id, t]))
    const patches: Patch[] = []
    for (const e of attach.effects) {
      /* 本来就在跟随的列写下去是空操作 —— 不发是无谓的手势 */
      if (e.following) continue
      const t = byId.get(e.col)
      if (!t) continue
      patches.push({
        kind: 'setValue',
        level: t.uid !== null ? 'version' : 'machine',
        owner: t.uid ?? t.machineId,
        key: param.key,
        /* 版本层：`null` = 跟随状态置真、值同步成基线有效值（`patch.rs` 的语义）；
           机型层：删键回出厂。两档都不是"把值设成空" */
        value: null,
      })
    }
    /* 写完之后框整个收起：设值那一页的值已经不是现在这一份状态了 */
    setAttach(null)
    setPreview(null)
    if (!patches.length) {
      toasts.push('勾选的列本来就在跟随基线，不用改')
      return
    }
    const ok = await onApply(`${param.label} 跟随基线（${patches.length} 列）`, patches)
    if (ok) toasts.push(`已挂回 ${patches.length} 列`)
  }

  /** 挂回基线的预览里有几列的值真的会变（界面用它说话） */
  const attachChanged = attach?.effects.filter((e) => !e.following && e.changes).length ?? 0
  /** 已经在跟随的版本列数（副标题与确认按钮都用它） */
  const attachFollowing = attach?.effects.filter((e) => e.following).length ?? 0
  /** 真的会写下去的列数 —— 全在跟随时是 0，确认按钮要灰 */
  const attachWritable = attach?.effects.filter((e) => !e.following).length ?? 0

  /**
   * 被跳过的一行（不适用 / 被条件关着 / 基线是根）。
   *
   * 有条件阻塞时给一颗「去改「X」」—— 现在只靠 tooltip 说"要先打开哪一项"，
   * 用户得自己回去找（作者 2026-11-09）。点它关掉预览、选中那个参数并滚到它那一行。
   */
  const skipRow = (skip: BulkSkip) => {
    const blocker = skip.blocked[0]
    return (
      <div key={skip.col} className={s.pBatchRow}>
        <span className={s.pKvK}>
          {skip.label}
          <em className={s.cardNote}>{skip.machine}</em>
        </span>
        <span className={s.pKvV}>
          <span
            className={s.cardNote}
            data-tip={skip.blocked.map((b) => b.need).join('；') || undefined}
          >
            {skip.reason}
          </span>
          {onGoFix !== undefined && blocker !== undefined && (
            <button
              type="button"
              className={s.pMiniBtn}
              data-tip={`先改「${blocker.label}」：${blocker.need}`}
              onClick={() => {
                setPreview(null)
                setAttach(null)
                onGoFix(blocker.key)
              }}
            >
              去改「{blocker.label}」
            </button>
          )}
        </span>
      </div>
    )
  }

  return (
    <aside className={s.pCard}>
      <div className={s.pBatchRow}>
        <span className={s.pKvK}>参数</span>
        <span className={s.pKvV}>
          {param.label}
          <em className={s.cardNote}>{param.unit ?? ''}</em>
          {/* 右栏遮着中间那张表时，先看一眼是哪一行再决定改不改 */}
          {onLocate !== undefined && (
            <button
              type="button"
              className={s.pMiniBtn}
              data-tip="滚到中间表格的这个参数那一行"
              onClick={onLocate}
            >
              定位
            </button>
          )}
        </span>
      </div>

      <div className={s.pBatchRow}>
        <span className={s.pKvK}>
          应用目标
          {/* 目标能有两三打，全选 / 反选必须有明写的按钮 */}
          <span className={s.pMiniOps}>
            <button
              type="button"
              className={s.pMiniBtn}
              data-tip={allOn ? '全不选' : '勾上所有对照列'}
              onClick={() => setPickedIds(allOn ? [] : targets.map((t) => t.id))}
            >
              {allOn ? '全不选' : '全选'}
            </button>
            <button
              type="button"
              className={s.pMiniBtn}
              title="把当前目标反过来"
              onClick={() =>
                setPickedIds(targets.filter((t) => !pickedTargets.includes(t.id)).map((t) => t.id))
              }
            >
              反选
            </button>
          </span>
        </span>
        {/* chip 上的名字一律带机型（C14 第十五轮：「把 A1 的补上」）；一律实线 */}
        <span className={s.pBatchChips}>
          {targets.map((v) => (
            <button
              key={v.id}
              type="button"
              className={`${s.pChip} ${pickedTargets.includes(v.id) ? s.pChipOn : ''}`}
              onClick={() => toggle(v.id)}
            >
              {v.name || v.id}
            </button>
          ))}
        </span>
      </div>

      <div className={s.pBatchRow}>
        <span className={s.pKvK}>设置新值</span>
        <span className={s.pBatchVal}>
          {/* 枚举只有 string 这一档才有取值域 —— 与 CellEditor / `validate.rs`
              的 `value_type == "string"` 同一道门。float 参数身上挂着的 choices
              是「预设档」，这一格该给能填的数字框，不是切下拉
              （`wiping.ironing_coverage_threshold` 就是这种）。 */}
          {param.uiComponent === 'switch' ? (
            <button
              type="button"
              className={`${s.btn} ${s.btnSm} ${raw === '开' ? s.btnOn : ''}`}
              data-tip={
                raw === ''
                  ? `勾选的 ${targets.length} 列开关状态不一致 —— 点一下统一改成「开」`
                  : undefined
              }
              onClick={() => setRawEdit(raw === '开' ? '关' : '开')}
            >
              {raw === '' ? '各不相同' : raw === '开' ? '开' : '关'}
            </button>
          ) : param.valueType === 'string' && param.choices.length > 0 ? (
            <SelectField
              label={param.label}
              size="sm"
              value={raw}
              /* 值不一致时下拉里得有一句**看得见的状态**，不能只剩一个空触发器。
                 它不写任何值（选中等于没选），选项照旧在下面 */
              options={[
                ...(valuePlaceholder !== undefined
                  ? [{ value: '', label: valuePlaceholder }]
                  : []),
                ...param.choices.map((o) => ({
                  value: String(o.value),
                  label: o.label,
                  deprecated: o.deprecated || undefined,
                })),
              ]}
              onChange={setRawEdit}
            />
          ) : (
            <input
              className={s.pEdNum}
              type="number"
              value={raw}
              placeholder={valuePlaceholder}
              min={param.min ?? undefined}
              max={param.max ?? undefined}
              step={param.step ?? undefined}
              onChange={(e) => setRawEdit(e.target.value)}
            />
          )}
        </span>
        {/* 值不一致时把每一组值摊在框下：只留一个空框等于让人自己一列列去问 */}
        {valueGroups.length >= 2 && (
          <p
            className={s.cardNote}
            data-tip={valueGroups
              .map((g) => `${g.text || '（空）'}：${g.names.join('、')}`)
              .join('；')}
          >
            {valueGroups
              .slice(0, 3)
              .map((g) => `${g.text || '（空）'} → ${g.names.join('、')}`)
              .join(' ｜ ')}
            {valueGroups.length > 3 ? ` ｜ 等 ${valueGroups.length} 种值` : ''}
          </p>
        )}
      </div>

      <div className={s.pBatchOps}>
        <button
          type="button"
          className={`${s.btn} ${s.btnPrimary} ${s.btnSm}`}
          disabled={!picked.length || noChange}
          data-tip={
            !picked.length
              ? '先挑至少一个应用目标'
              : noChange
                ? '勾选的列都已经是这个值了 —— 没有变化就没有要写的'
                : undefined
          }
          onClick={() => void apply()}
        >
          应用
        </button>
        {/*
          「恢复修改前的」：把这几列各自退回**改动前**的值（不是恢复出厂基底，
          也不是挂回继承那一枚 ↶ —— 后者只退一层，这里是退出这一轮改的那一笔）。
        */}
        <button
          type="button"
          className={`${s.btn} ${s.btnSm}`}
          disabled={restorable.length === 0}
          data-tip={
            restorable.length === 0
              ? '勾选的列都没有改动，没有要恢复的'
              : '把改过的列各自退回改动前的值'
          }
          onClick={() => void restore()}
        >
          恢复修改前的
        </button>
        {/*
          与「设置新值」相反的方向：那一颗让这一层**自己钉住**，这一颗让它**重新跟着基线**。
          写的是 `value: null`（版本层 = 跟随状态 + 值同步成基线有效值），所以先在
          预览里看清哪几列真的会变、变成多少 —— 与写值一样，看一眼再动手。
          （预览框底部那颗「跟随基线」是另一件事：只改基线、版本不动，见 `confirmBaseOnly`）
        */}
        <button
          type="button"
          className={`${s.btn} ${s.btnSm}`}
          disabled={attachable.length === 0}
          data-tip={
            pickedVersions === 0
              ? '勾选的只有机型基线 —— 它是这条链的根，没有可挂回的'
              : attachable.length === 0
                ? '勾选的版本列本来就在跟随各自的基线，没有可挂回的'
                : '把勾选的版本列重新挂回各自的基线 —— 点开先看值会变成多少'
          }
          onClick={() => void askAttach()}
        >
          重新跟随
        </button>
      </div>
      {/* 灰按钮要说清为什么灰：版本列本来就在跟随（或只勾了基线），写下去都是空操作 */}
      {picked.length > 0 && attachable.length === 0 && (
        <p className={s.cardNote}>
          {pickedVersions === 0
            ? `勾选的 ${picked.length} 列都是机型基线 —— 它是这条链的根，没有可改回跟随的。`
            : `勾选的 ${pickedVersions} 个版本都在跟随各自的基线 —— 没有可改回跟随的。`}
        </p>
      )}
      <p className={s.note}>
        写到<b>勾选的那些列各自那一层</b>；版本列若与基线同值，会改为跟随基线。一次动作一条历史，Ctrl+Z 整块退回来。
      </p>

      <div className={s.pCardHead}>
        <h3>{param.label}</h3>
      </div>
      <p className={s.pDesc}>{param.desc}</p>

      <p className={s.cardNote}>对照列 {targets.length} 个</p>

      {/*
        预览框（产品的纪律：先看后写）。两页共用一个框：
        `set` = 把勾选的列设成这个值会落到哪几列；
        `attach` = 让它们重新跟着各自基线会落到哪几列、值变成多少。
        前后值、跳过原因、能不能写全部由后端判好，确认才写。
      */}
      {(preview !== null || attach !== null) && (
        <ModalC14
          open
          size="md"
          title={
            pane === 'attach' && attach !== null
              ? `跟随基线：${attach.label}`
              : pane === 'base' && preview !== null
                ? `跟随基线：${preview.label}`
                : `批量：${preview?.label ?? param.label}`
          }
          subtitle={
            pane === 'set' && preview !== null
              ? `${shownValue()} → ${preview.effects.length} 列${
                  preview.skipped.length ? ` · 跳过 ${preview.skipped.length}` : ''
                }`
              : pane === 'base' && preview !== null
                ? `只改 ${baseEffects.length} 台机型的基线 · ${preview.effects.length - baseEffects.length} 列不动`
                : attach !== null
                  ? `${attach.effects.length} 列 · 已在跟随 ${attachFollowing} 列 · 要改 ${attachWritable} 列${
                      attachChanged > 0 ? `（其中 ${attachChanged} 列的值会变）` : ''
                    }${attach.skipped.length ? ` · 跳过 ${attach.skipped.length}` : ''}`
                  : ''
          }
          onClose={() => {
            setPreview(null)
            setAttach(null)
          }}
          footLead={
            pane === 'set' ? (
              /*
               * 常驻在框的左下角：**这次的值只写到机型基线**，版本列不动。
               * 跟着基线的版本自然一起变；自己钉着值的版本保持原样。
               */
              <button
                type="button"
                className={`${s.btn} ${s.btnSm}`}
                disabled={baseEffects.length === 0}
                data-tip={
                  baseEffects.length === 0
                    ? '这一批里没有要改的机型基线 —— 左边把机型基线勾上再试'
                    : `只把 ${shownValue()} 写到 ${baseEffects.length} 台机型的基线上；版本列不动（跟着基线的会一起变）`
                }
                onClick={() => setPane('base')}
              >
                跟随基线
              </button>
            ) : undefined
          }
          footer={
            pane === 'set' && preview !== null ? (
              <>
                <button
                  type="button"
                  className={`${s.btn} ${s.btnSm}`}
                  onClick={() => setPreview(null)}
                >
                  再改改
                </button>
                {/*
                  与基线同值的列**不各自钉住**：确认写入时自动写 `null`（跟随基线），
                  值一样、少一行版本键 + 一行 follows；这一批里连基线一起改时，
                  版本也跟着新基线走（`effect.followsBase`）。见 `confirmWrite`。
                */}
                <button
                  type="button"
                  className={`${s.btn} ${s.btnPrimary} ${s.btnSm}`}
                  disabled={!preview.effects.some((e) => e.kind !== 'noChange')}
                  data-tip={
                    preview.effects.some((e) => e.kind !== 'noChange')
                      ? undefined
                      : '勾选的列都已经是这个值了'
                  }
                  onClick={() => void confirmWrite()}
                >
                  确认写入
                </button>
              </>
            ) : pane === 'base' && preview !== null ? (
              <>
                <button
                  type="button"
                  className={`${s.btn} ${s.btnSm}`}
                  onClick={() => setPane('set')}
                >
                  返回
                </button>
                <button
                  type="button"
                  className={`${s.btn} ${s.btnPrimary} ${s.btnSm}`}
                  disabled={baseEffects.length === 0}
                  data-tip={
                    baseEffects.length === 0
                      ? '这一批里没有要改的机型基线'
                      : `只写这 ${baseEffects.length} 台机型的基线；版本列一个都不发`
                  }
                  onClick={() => void writeBaseOnly()}
                >
                  确认写入
                </button>
              </>
            ) : (
              <>
                <button
                  type="button"
                  className={`${s.btn} ${s.btnSm}`}
                  onClick={() => setAttach(null)}
                >
                  不跟了
                </button>
                <button
                  type="button"
                  className={`${s.btn} ${s.btnPrimary} ${s.btnSm}`}
                  disabled={attachWritable === 0}
                  data-tip={
                    attachWritable === 0
                      ? '勾选的版本列本来就在跟随基线，写下去没有变化'
                      : undefined
                  }
                  onClick={() => void confirmAttach()}
                >
                  确认跟随
                </button>
              </>
            )
          }
        >
          {pane === 'base' && preview !== null
            ? [
                <p key="__base-lead" className={s.note}>
                  只写机型基线；版本列一个都不发 —— 跟着基线的版本会一起变成 {shownValue()}。
                </p>,
                ...preview.effects.map((e) => {
                  const isBase = e.level === 'machine'
                  const t = targets.find((x) => x.id === e.col)
                  return (
                    <div
                      key={e.col}
                      className={`${s.pBatchRow} ${
                        isBase && e.kind !== 'noChange' ? s.pBatchRowChanged : ''
                      }`}
                    >
                      <span className={s.pKvK}>
                        {e.label}
                        <em className={s.cardNote}>{e.machine}</em>
                      </span>
                      <span className={s.pKvV}>
                        {isBase ? (
                          e.kind === 'noChange' ? (
                            <span className={s.cardNote}>{e.before} · 不变</span>
                          ) : (
                            <>
                              {e.before} → <b>{e.after}</b>
                              <span className={s.pBatchFlag}>改基线</span>
                            </>
                          )
                        ) : (
                          <>
                            {e.before}
                            <span className={s.cardNote}>
                              {' · '}不动
                              {t?.follows ? `（跟着基线变成 ${e.after}）` : '（自己钉着值）'}
                            </span>
                          </>
                        )}
                      </span>
                    </div>
                  )
                }),
                ...preview.skipped.map(skipRow),
              ]
            : pane === 'set' && preview !== null
              ? [
                  ...(preview.effects.some((e) => e.level === 'machine')
                    ? [
                        <p key="__base-note" className={s.note}>
                          这一批里有机型基线 —— 改它会带着所有跟随它的版本一起变；自己钉着值的版本不动。
                        </p>,
                      ]
                    : []),
                  ...preview.effects.map((e) => (
                    <div key={e.col} className={s.pBatchRow}>
                      <span className={s.pKvK}>
                        {e.label}
                        <em className={s.cardNote}>{e.machine}</em>
                      </span>
                      <span className={s.pKvV}>
                        {e.kind === 'noChange' ? (
                          <span className={s.cardNote}>{words.bulkKind.noChange.label}（不变）</span>
                        ) : (
                          <>
                            {e.before} → <b>{e.after}</b>
                            {e.level === 'machine' ? (
                              <span className={s.cardNote}> · 改基线值</span>
                            ) : e.followsBase ? (
                              <span className={s.cardNote}> · 跟随基线（由这一批的基线带过去）</span>
                            ) : e.sameAsBase ? (
                              <span className={s.cardNote}> · 跟随基线（与基线 {e.base} 同值）</span>
                            ) : e.kind === 'detaching' ? (
                              <span className={s.cardNote}> · {words.bulkKind.detaching.label}</span>
                            ) : null}
                          </>
                        )}
                      </span>
                    </div>
                  )),
                  ...preview.skipped.map(skipRow),
                ]
              : attach !== null
                ? [
                    ...attach.effects.map((e) => (
                      <div
                        key={e.col}
                        className={`${s.pBatchRow} ${
                          !e.following && e.changes ? s.pBatchRowChanged : ''
                        }`}
                      >
                        <span className={s.pKvK}>
                          {e.label}
                          <em className={s.cardNote}>{e.machine}</em>
                        </span>
                        <span className={s.pKvV}>
                          {e.before} → <b>{e.after}</b>
                          <span className={s.cardNote}>（基线）</span>
                          {e.following ? (
                            <span className={s.cardNote}>已在跟随基线 · 值不变</span>
                          ) : e.changes ? (
                            <span className={s.pBatchFlag}>值会变</span>
                          ) : (
                            <span className={s.cardNote}>值不变（只把状态改成跟随）</span>
                          )}
                        </span>
                      </div>
                    )),
                    ...(attach.effects.length === 0
                      ? [
                          <p key="__empty" className={s.note}>
                            勾选的列没有可挂回的 —— 不适用的、被条件关着的、以及机型基线本身都不在这条链里。
                          </p>,
                        ]
                      : []),
                    ...attach.skipped.map(skipRow),
                  ]
                : null}
        </ModalC14>
      )}
    </aside>
  )
}
