/*
 * 两列成员编辑框：分组抽屉与页签栏共用一件。
 *
 * 左列按原「页签 → 分组」铺开全部参数，右列是分组次序与这一组的成员。
 * 分组次序复用 c14/sort/ReorderList（序号 / ↑↓ / 拖拽 / 位次同一套数学），
 * 框内草稿直到底部「确认」才交给调用方，一次事务进影子树，保存才落盘。
 */

import { useId, useMemo, useState } from 'react'
import type { ReactNode } from 'react'

import type { GroupMembersEdit, ParamView, SectionTarget } from '../api'
import { isAppError } from '../api'
import ModalC14 from '../c14/ModalC14'
import { toasts } from '../c14/toast'
import ReorderList from '../c14/sort/ReorderList'
import type { SortRow } from '../c14/sort/types'
import { track } from '../../app/log/track'
import { nextSectionId } from './sectionId'
import { usableSections, useMembersDraft } from './useMembersDraft'
import c from '../c14.module.css'
import s from './SectionMembersModal.module.css'

/** 与后端 `ensure_section` 的 24 字上限同一把尺子 */
const LABEL_MAX = 24

export interface SectionMembersModalProps {
  scope: 'section' | 'tab'
  /** 分组层 = section id；页签层 = tab id */
  id: string
  /** 打开时的名字 */
  label: string
  params: ParamView[]
  sections: SectionTarget[]
  onApply: (edit: GroupMembersEdit) => Promise<void>
  onClose: () => void
  /**
   * 页签自己的动作（上移 / 下移 / 删除）。**只有页签层给** ——
   * 分组层打开这一框时，页签是别人家的事。
   */
  tabOps?: {
    /** 这一页在第几位（0 起） */
    at: number
    total: number
    /** 空页签才给删除按钮（参数没有第二个去处） */
    canRemove: boolean
    onMove: (dir: -1 | 1) => void
    onRemove: () => void
  }
}

interface Block {
  sectionId: string
  sectionLabel: string
  params: ParamView[]
  /** 这一组的全部成员（不受搜索影响）：`0` 就是空组，搜索时也留着它这一块 */
  total: number
}

interface Group {
  tabId: string
  tabLabel: string
  blocks: Block[]
}

const byOrder = (a: ParamView, b: ParamView): number => a.order - b.order

export default function SectionMembersModal({
  scope,
  id,
  label,
  params,
  sections,
  onApply,
  onClose,
  tabOps,
}: SectionMembersModalProps) {
  const nameId = useId()
  const usable = useMemo(() => usableSections(sections), [sections])
  const paramByKey = useMemo(() => new Map(params.map((p) => [p.key, p])), [params])
  const targets = useMemo(
    () => (scope === 'tab' ? usable.filter((x) => x.tabId === id) : usable.filter((x) => x.id === id)),
    [usable, scope, id],
  )
  const defaultTarget = targets[0]?.id ?? ''
  const initialKeys = useMemo(
    () =>
      params
        .filter((p) => p.sectionId === defaultTarget)
        .sort(byOrder)
        .map((p) => p.key),
    [params, defaultTarget],
  )

  /* 分组条管的是目标组所在那个页签；注册表里直接找，组件卡层也能开 */
  const tabId = scope === 'tab'
    ? id
    : (sections.find((x) => x.id === id)?.tabId ?? targets[0]?.tabId ?? '')
  const baseRows = useMemo(
    () =>
      sections
        .filter((x) => x.tabId === tabId && x.component === null && !x.standalone)
        .map((x) => ({ id: x.id, label: x.label })),
    [sections, tabId],
  )
  const initialGroupOrder = useMemo(() => baseRows.map((x) => x.id), [baseRows])
  const initialNames = useMemo(() => {
    const out: Record<string, string> = {}
    for (const sec of sections) out[sec.id] = sec.label
    return out
  }, [sections])
  const draft = useMembersDraft({
    label,
    keys: initialKeys,
    targetSection: defaultTarget,
    groupOrder: initialGroupOrder,
    names: initialNames,
    /* 分组层编辑的就是目标组本身，名字跟着它；页签层改的是页签名 */
    nameFollowsTarget: scope === 'section',
  })
  const [query, setQuery] = useState('')
  const [busy, setBusy] = useState(false)
  const [adding, setAdding] = useState(false)
  const [newName, setNewName] = useState('')
  /** 删页签先问一句（这一框里唯一一件"整块东西没了"的动作） */
  const [askingRemove, setAskingRemove] = useState(false)

  /*
   * 左列骨架 = 注册表里能装参数的分组（含暂时空掉的）+ 框里刚建的那些，
   * 按草稿次序排。**不从 params 反推**：空组一旦从列表里消失，就再也挑不回去了。
   */
  const skeleton = useMemo(() => {
    const known = new Set(usable.map((x) => x.id))
    const rows = usable.map((x) => ({
      id: x.id,
      label: draft.names[x.id] ?? x.label,
      tabId: x.tabId,
      tabLabel: x.tabLabel,
    }))
    for (const ns of draft.newSections) {
      if (known.has(ns.id)) continue
      const tab = sections.find((x) => x.tabId === ns.tabId)
      rows.push({
        id: ns.id,
        label: draft.names[ns.id] ?? ns.label,
        tabId: ns.tabId,
        tabLabel: tab?.tabLabel ?? ns.tabId,
      })
    }
    const at = new Map(draft.groupOrder.map((id, i) => [id, i] as const))
    return rows.sort((a, b) => (at.get(a.id) ?? 1e9) - (at.get(b.id) ?? 1e9))
  }, [usable, sections, draft.newSections, draft.names, draft.groupOrder])

  const groups = useMemo(() => {
    const bySection = new Map<string, ParamView[]>()
    for (const p of params) {
      const arr = bySection.get(p.sectionId) ?? []
      arr.push(p)
      bySection.set(p.sectionId, arr)
    }
    const out: Group[] = []
    for (const sec of skeleton) {
      let g = out.find((x) => x.tabId === sec.tabId)
      if (g === undefined) {
        g = { tabId: sec.tabId, tabLabel: sec.tabLabel, blocks: [] }
        out.push(g)
      }
      const rows = (bySection.get(sec.id) ?? []).slice().sort(byOrder)
      g.blocks.push({
        sectionId: sec.id,
        sectionLabel: sec.label,
        params: rows,
        total: rows.length,
      })
    }
    return out
  }, [params, skeleton])

  const q = query.trim().toLowerCase()
  const shown = useMemo(() => {
    if (q === '') return groups
    return groups
      .map((g) => ({
        ...g,
        blocks: g.blocks
          .map((b) => ({
            ...b,
            params: b.params.filter(
              (p) => p.label.toLowerCase().includes(q) || p.key.toLowerCase().includes(q),
            ),
          }))
          /* 空组是"目标"不是"结果"：搜索时也留着它这一块 */
          .filter((b) => b.params.length > 0 || b.total === 0),
      }))
      .filter((g) => g.blocks.length > 0)
  }, [groups, q])

  const total = groups.reduce(
    (n, g) => n + g.blocks.reduce((m, b) => m + b.total, 0),
    0,
  )
  const rowsOf = (sectionId: string): string[] =>
    params
      .filter((p) => p.sectionId === sectionId)
      .sort(byOrder)
      .map((p) => p.key)

  /* 确认落盘那一帧注册表里已经有新组了，别再当「待新建」画第二遍 */
  const groupRows: SortRow[] = useMemo(() => {
    const known = new Set(sections.map((x) => x.id))
    return [
      ...baseRows.map((x) => ({ key: x.id, label: draft.names[x.id] ?? x.label })),
      ...draft.newSections
        .filter((ns) => !known.has(ns.id))
        .map((ns) => ({
          key: ns.id,
          label: draft.names[ns.id] ?? ns.label,
          /* 确认时只落选中那一组 —— 别的行如实写「不保存」 */
          by: ns.id === draft.targetSection ? '待新建' : '不保存',
        })),
    ]
  }, [baseRows, draft.newSections, draft.names, draft.targetSection, sections])

  const submitAdd = () => {
    const name = newName.trim()
    if (name === '' || tabId === '') return
    const used = [...sections, ...draft.newSections].map((x) => ({ id: x.id }))
    const fresh = nextSectionId(used)
    draft.addSection({ id: fresh, tabId, label: name })
    /* 建完就是它：让作者接着往里面挑参数，不用再点一次 */
    draft.retarget(fresh, [])
    setAdding(false)
    setNewName('')
    track('members.add', 'group')
  }
  const closeAdding = () => {
    setAdding(false)
    setNewName('')
  }
  const onGroupPick = (key: string) => {
    if (key === draft.targetSection) return
    draft.retarget(key, rowsOf(key))
    track('members.retarget', 'group')
  }
  const onGroupMove = (from: number, to: number) => {
    draft.moveGroup(from, to)
    track('members.move-group', 'group')
  }
  const groupActions = (key: string): ReactNode => {
    if (!draft.newSections.some((ns) => ns.id === key)) return null
    const name = groupRows.find((r) => r.key === key)?.label ?? key
    return (
      <button
        type="button"
        className={s.mini}
        aria-label={`删除 ${name}`}
        onPointerDown={(e) => e.stopPropagation()}
        onClick={() => {
          track('members.remove-new', 'group')
          draft.removeNewSection(key)
          if (draft.targetSection === key) {
            const rest = draft.groupOrder.filter((x) => x !== key)
            const next = rest[0] ?? ''
            draft.retarget(next, rowsOf(next))
          }
        }}
      >
        删除
      </button>
    )
  }

  const movedOut = draft.initialKeys.filter((k) => !draft.has(k))
  const name = draft.label.trim()
  const canConfirm = draft.dirty && name !== '' && draft.targetSection !== '' && !busy
  /*
   * 提交时「改的是谁」跟着目标组走：分组层后端要求 `id == sectionId`
   * （`ui.presetdata.registry.351`），名字的基准也取目标组原来那个名。
   */
  const openedLabel =
    scope === 'tab'
      ? label
      : draft.newSections.some((ns) => ns.id === draft.targetSection)
        ? (draft.names[draft.targetSection] ?? '')
        : (sections.find((x) => x.id === draft.targetSection)?.label ?? label)

  const commit = async () => {
    if (!canConfirm) return
    setBusy(true)
    try {
      await onApply({
        scope,
        id: scope === 'tab' ? id : draft.targetSection,
        label: name !== openedLabel ? name : null,
        sectionId: draft.targetSection,
        keys: draft.keys,
        /*
         * 只交**选中的**那一个新组：改选别的组之后留在清单里的那些行标着「不保存」，
         * 真把它们也建出来，等于偷偷在数据里留一个空组。
         */
        newSections: draft.newSections.filter((ns) => ns.id === draft.targetSection),
        sectionOrder: draft.groupOrder,
      })
      track('members.confirm', 'group')
      onClose()
    } catch (e) {
      track('members.confirm.fail', isAppError(e) ? e.code : 'unknown')
      toasts.push(isAppError(e) ? e.message : String(e))
    } finally {
      setBusy(false)
    }
  }

  const cancel = () => {
    track('members.cancel', 'group')
    onClose()
  }

  return (
    <>
    <ModalC14
      open
      title={`${scope === 'section' ? '编辑分组' : '编辑页签'} · ${draft.label || label}`}
      subtitle={`共 ${total} 条参数 · 名字与成员一起改`}
      size="lg"
      closeOnScrim={false}
      bodyScroll={false}
      onClose={cancel}
      headerExtra={
        <button
          type="button"
          className={s.linkBtn}
          disabled={!draft.dirty}
          onClick={() => {
            track('members.reset', 'group')
            draft.reset()
          }}
        >
          恢复
        </button>
      }
      footLead={<span className={s.footNote}>保存才生效</span>}
      footer={
        <>
          <button type="button" className={s.btn} onClick={cancel}>
            取消
          </button>
          <button
            type="button"
            className={`${s.btn} ${s.btnPrimary}`}
            disabled={!canConfirm}
            onClick={() => void commit()}
          >
            {busy ? '保存中…' : '确认'}
          </button>
        </>
      }
    >
      <div className={s.head}>
        <label className={s.nameRow} htmlFor={nameId}>
          <span className={s.nameLabel}>名字</span>
          <input
            id={nameId}
            className={s.nameInput}
            value={draft.label}
            maxLength={LABEL_MAX}
            onChange={(e) => draft.setLabel(e.target.value)}
          />
        </label>
        {tabOps !== undefined && (
          <div className={s.tabOps}>
            <span className={s.tabAt}>
              第 {tabOps.at + 1} / {tabOps.total} 页
            </span>
            <button
              type="button"
              className={s.mini}
              aria-label="页签上移"
              disabled={tabOps.at <= 0}
              title="往前挪一位"
              onClick={() => tabOps.onMove(-1)}
            >
              ↑
            </button>
            <button
              type="button"
              className={s.mini}
              aria-label="页签下移"
              disabled={tabOps.at >= tabOps.total - 1}
              title="往后挪一位"
              onClick={() => tabOps.onMove(1)}
            >
              ↓
            </button>
            {tabOps.canRemove && (
              <button
                type="button"
                className={s.mini}
                aria-label="删除页签"
                onClick={() => setAskingRemove(true)}
              >
                删除页签
              </button>
            )}
          </div>
        )}
      </div>

      <div className={s.wrap}>
        <div className={s.left}>
          <div className={s.fixed}>
            <input
              className={s.search}
              type="search"
              aria-label="搜索参数"
              placeholder="搜索名字或键名"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
          </div>
          <div className={s.scroll} data-scroll="left">
            {shown.map((g) => (
              <div key={g.tabId} className={s.group}>
                <div className={s.groupHead}>{g.tabLabel}</div>
                {g.blocks.map((b) => (
                  <div key={b.sectionId}>
                    <div
                      className={b.sectionId === draft.targetSection ? s.sectionHeadOn : s.sectionHead}
                    >
                      {b.sectionLabel}<span className={s.count}>{b.total}</span>
                    </div>
                    {b.total === 0 && <div className={s.emptyBlock}>还没有参数</div>}
                    {b.params.map((p) => {
                      const checked = draft.has(p.key)
                      const disabled = p.sectionId === draft.targetSection && checked
                      return (
                        <button
                          key={p.key}
                          type="button"
                          className={`${s.row} ${checked ? s.rowOn : ''}`}
                          aria-pressed={checked}
                          aria-label={`${p.label} ${checked ? '取消勾选' : '归入这一组'}`}
                          disabled={disabled}
                          onClick={() => draft.toggle(p.key)}
                        >
                          <span className={s.check} aria-hidden>{checked ? '✓' : ''}</span>
                          <span className={s.label}>{p.label}</span>
                          <span className={s.by}>{p.key}</span>
                        </button>
                      )
                    })}
                  </div>
                ))}
              </div>
            ))}
            {shown.length === 0 && <div className={s.empty}>没有匹配的参数</div>}
          </div>
        </div>

        <div className={s.right}>
          <div className={s.fixed}>
            <div className={s.groupBarHead}>
              <span>分组</span>
              <button
                type="button"
                className={s.addBtn}
                aria-label="新增分组"
                disabled={tabId === ''}
                onClick={() => {
                  setAdding(true)
                  setNewName('')
                }}
              >
                ＋ 新增分组
              </button>
            </div>
            {adding && (
              <div className={s.addRow}>
                <input
                  className={s.addInput}
                  aria-label="新分组名字"
                  value={newName}
                  maxLength={LABEL_MAX}
                  autoFocus
                  placeholder="分组名字"
                  onChange={(e) => setNewName(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Escape') closeAdding()
                  }}
                />
                <button
                  type="button"
                  className={s.addConfirm}
                  aria-label="确认新增分组"
                  onClick={submitAdd}
                >
                  确认
                </button>
                <button type="button" className={s.mini} aria-label="取消新增分组" onClick={closeAdding}>
                  取消
                </button>
              </div>
            )}
          </div>
          <div className={s.scroll} data-scroll="right">
            <ReorderList
              rows={groupRows}
              order={draft.groupOrder}
              currentKey={draft.targetSection}
              onMove={onGroupMove}
              onPick={onGroupPick}
              actions={groupActions}
              embedded
              label="分组次序"
            />
            <div className={s.rightHead}>这一组 · {draft.keys.length}</div>
            {draft.keys.map((k) => {
              const p = paramByKey.get(k)
              const fresh = !draft.initialKeys.includes(k)
              return (
                <div key={k} className={`${s.member} ${fresh ? s.memberNew : ''}`}>
                  <span className={s.label}>{p?.label ?? k}</span>
                  <span className={s.by}>{k}</span>
                  <span className={s.spacer} />
                  {fresh && <span className={s.tag}>待归入</span>}
                  <button
                    type="button"
                    className={s.mini}
                    aria-label={`${p?.label ?? k} ${fresh ? '取消归入' : '移出本组'}`}
                    onClick={() => draft.remove(k)}
                  >
                    {fresh ? '取消' : '移出'}
                  </button>
                </div>
              )
            })}
            {draft.keys.length === 0 && <div className={s.empty}>这一组还没有参数</div>}
            {movedOut.length > 0 && (
              <>
                <div className={s.movedHead}>已移出 · 会归到「未归类」</div>
                {movedOut.map((k) => (
                  <div key={k} className={`${s.member} ${s.memberOut}`}>
                    <span className={s.label}>{paramByKey.get(k)?.label ?? k}</span>
                    <span className={s.spacer} />
                    <button
                      type="button"
                      className={s.mini}
                      aria-label={`${paramByKey.get(k)?.label ?? k} 放回这一组`}
                      onClick={() => draft.putBack(k)}
                    >
                      放回
                    </button>
                  </div>
                ))}
              </>
            )}
          </div>
        </div>
      </div>
    </ModalC14>

      {askingRemove && tabOps !== undefined && (
        <ModalC14
          open
          title="删除这个页签？"
          subtitle="页签里一条参数都没有"
          closeOnScrim={false}
          onClose={() => setAskingRemove(false)}
          footer={
            <>
              <span className={s.footNote}>保存才生效</span>
              <button type="button" className={s.btn} onClick={() => setAskingRemove(false)}>
                取消
              </button>
              <button
                type="button"
                className={`${s.btn} ${c.btnDanger}`}
                onClick={() => {
                  setAskingRemove(false)
                  tabOps.onRemove()
                }}
              >
                删除页签
              </button>
            </>
          }
        >
          <p className={s.confirmText}>
            「{draft.label.trim() === '' ? label : draft.label.trim()}」和它下面那个空分组一起消失，参数一条不动。
          </p>
        </ModalC14>
      )}
    </>
  )
}
