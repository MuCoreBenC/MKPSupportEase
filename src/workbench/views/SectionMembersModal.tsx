/*
 * 两列成员编辑框：分组抽屉与页签栏共用一件。
 *
 * 左列按原「页签 → 分组」铺开全部参数，可搜索可勾选；右列是这一组的最终成员
 * 与已移出的那些。框内草稿，确认才把最终成员序列交给 `onApply`，由调用方决定命令。
 *
 * 组件卡 / 整块卡不在候选里：客户端那边是固定长好的，工作台不把参数搬进搬出。
 */

import { useId, useMemo, useState } from 'react'

import type { GroupMembersEdit, ParamView, SectionTarget } from '../api'
import { isAppError } from '../api'
import ModalC14 from '../c14/ModalC14'
import { toasts } from '../c14/toast'
import { usableSections, useMembersDraft } from './useMembersDraft'
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
}

interface Block {
  sectionId: string
  sectionLabel: string
  params: ParamView[]
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
  const draft = useMembersDraft({ label, keys: initialKeys, targetSection: defaultTarget })
  const [query, setQuery] = useState('')
  const [busy, setBusy] = useState(false)

  const groups = useMemo(() => {
    const usableIds = new Set(usable.map((x) => x.id))
    const bySection = new Map<string, ParamView[]>()
    for (const p of params) {
      if (!usableIds.has(p.sectionId)) continue
      const arr = bySection.get(p.sectionId) ?? []
      arr.push(p)
      bySection.set(p.sectionId, arr)
    }
    const out: Group[] = []
    for (const sec of sections) {
      if (!usableIds.has(sec.id)) continue
      const rows = (bySection.get(sec.id) ?? []).slice().sort(byOrder)
      if (rows.length === 0) continue
      let g = out.find((x) => x.tabId === sec.tabId)
      if (g === undefined) {
        g = { tabId: sec.tabId, tabLabel: sec.tabLabel, blocks: [] }
        out.push(g)
      }
      g.blocks.push({ sectionId: sec.id, sectionLabel: sec.label, params: rows })
    }
    return out
  }, [params, sections, usable])

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
          .filter((b) => b.params.length > 0),
      }))
      .filter((g) => g.blocks.length > 0)
  }, [groups, q])

  const total = groups.reduce(
    (n, g) => n + g.blocks.reduce((m, b) => m + b.params.length, 0),
    0,
  )
  const movedOut = draft.initialKeys.filter((k) => !draft.has(k))
  const name = draft.label.trim()
  const canConfirm = draft.dirty && name !== '' && draft.targetSection !== '' && !busy

  const commit = async () => {
    if (!canConfirm) return
    setBusy(true)
    try {
      await onApply({
        scope,
        id,
        label: name !== label ? name : null,
        sectionId: draft.targetSection,
        keys: draft.keys,
      })
      onClose()
    } catch (e) {
      toasts.push(isAppError(e) ? e.message : String(e))
    } finally {
      setBusy(false)
    }
  }

  return (
    <ModalC14
      open
      title={`${scope === 'section' ? '编辑分组' : '编辑页签'} · ${label}`}
      subtitle={`共 ${total} 条参数 · 名字与成员一起改`}
      size="lg"
      closeOnScrim={false}
      onClose={onClose}
      headerExtra={
        <button
          type="button"
          className={s.linkBtn}
          disabled={!draft.dirty}
          onClick={draft.reset}
        >
          恢复
        </button>
      }
      footLead={<span className={s.footNote}>保存才生效</span>}
      footer={
        <>
          <button type="button" className={s.btn} onClick={onClose}>
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
      <label className={s.nameRow} htmlFor={nameId}>
        <span className={s.nameLabel}>名字</span>
        <input
          id={nameId}
          className={s.nameInput}
          value={draft.label}
          maxLength={LABEL_MAX}
          onChange={(e) => draft.setLabel(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') (e.target as HTMLInputElement).blur()
          }}
        />
      </label>

      <div className={s.wrap}>
        <div className={s.left}>
          <input
            className={s.search}
            type="search"
            aria-label="搜索参数"
            placeholder="搜索名字或键名"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <div className={s.scroll}>
            {shown.map((g) => (
              <div key={g.tabId} className={s.group}>
                <div className={s.groupHead}>{g.tabLabel}</div>
                {g.blocks.map((b) => (
                  <div key={b.sectionId}>
                    <div className={s.sectionHead}>
                      {b.sectionLabel}<span className={s.count}>{b.params.length}</span>
                    </div>
                    {b.params.map((p) => {
                      const checked = draft.has(p.key)
                      /* 本组已有且还在清单里：不必再勾；移出之后重新可勾 = 放回 */
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
          {scope === 'tab' && (
            <div className={s.targets} role="radiogroup" aria-label="归到这一组">
              {targets.map((t) => (
                <button
                  key={t.id}
                  type="button"
                  role="radio"
                  aria-checked={draft.targetSection === t.id}
                  className={`${s.target} ${draft.targetSection === t.id ? s.targetOn : ''}`}
                  onClick={() => {
                    if (t.id === draft.targetSection) return
                    draft.retarget(
                      t.id,
                      params
                        .filter((p) => p.sectionId === t.id)
                        .sort(byOrder)
                        .map((p) => p.key),
                    )
                  }}
                >
                  {t.label}
                </button>
              ))}
            </div>
          )}
          <div className={s.rightHead}>这一组 · {draft.keys.length}</div>
          <div className={s.scroll}>
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
  )
}
