/*
 * 选择器模态框（2026-11-14）。
 *
 * # 它替掉了什么
 *
 * 「编辑定义 → 归属与条件」那几格原来是自绘**浮层下拉**（`GroupPicker` / `SectionPicker`）：
 * 名单一长（属于 / 前置条件几十项）就顶到屏幕外，只能滚一条窄条；排版也很难看清
 * 「这一组有哪些、我现在在哪儿」。作者的原话：「能不能再出一个模态框……那些好几个
 * 下拉菜单需要做成好几个类似的复用的模态框了」。
 *
 * 所以这里做**一个壳**：触发器（与原来同长相）+ 模态框（标题 / 搜索 / 分层列表 /
 * 页脚）。三种内容各填各的：
 *
 *   · 单选分层列表   —— 属于 / 前置条件 / 档位（`groups`）
 *   · 分组管理面板   —— 所属分组（`children`，把 `SectionList` 整块搬进来：
 *                       新建 / 改名 / 组上下移都还是它那一套）
 *   · 组内排序       —— 不是这一件（见 `SortModal`），它要拖拽与草稿
 *
 * # 选完要确认
 *
 * 单选列表是**框内草稿**：点一行只高亮，点「确认」才写回父框并关框；
 * 「取消」把这一次的挑选整个丢掉。作者实测：「这个都没有确认」——
 * 没有确认按钮时，人会不确定刚才那一下到底生效没有。
 *
 * 分组面板（`children`）的选择本来就在父框草稿里，父框「确认」才落盘；
 * 这一层的「确认」= 接受并关框，不让同一个动作有两种长相。
 */

import { useId, useMemo, useState } from 'react'
import type { ReactNode } from 'react'

import ModalC14 from '../../c14/ModalC14'
import type { GroupedGroup } from './types'
import s from './PickerModal.module.css'

export interface PickerModalProps {
  /** 无障碍名（「所属分组」「值类型」…） */
  label: string
  /** 触发器上那句话（已本地化的值） */
  summary: ReactNode
  /** 框标题 */
  title: string
  /** 标题下面那句：说清选完会发生什么 */
  subtitle?: string
  /** 分层列表（给了它就不用 `children`） */
  groups?: GroupedGroup[]
  /** 当前值（打勾那一行） */
  value?: string
  /** 选中一行（**点「确认」之后才调**，不是点行就调） */
  onPick?: (next: string) => void
  /** 长名单才给搜索框 */
  searchable?: boolean
  /** 页脚左侧那句说明 */
  note?: string
  /** 自定义列表（所属分组把 `SectionList` 放这儿） */
  children?: ReactNode
  /** 触发器禁用 */
  disabled?: boolean
  /** 触发器悬停说明 */
  tip?: string
}

/** 行数超过这个数才摆搜索框 —— 三档的小枚举给它反而是噪音 */
const SEARCH_OVER = 12

export default function PickerModal({
  label,
  summary,
  title,
  subtitle,
  groups,
  value,
  onPick,
  searchable = false,
  note,
  children,
  disabled = false,
  tip,
}: PickerModalProps) {
  const [open, setOpen] = useState(false)

  return (
    <>
      <button
        type="button"
        className={s.trigger}
        aria-label={label}
        aria-haspopup="dialog"
        title={tip}
        disabled={disabled}
        onClick={() => setOpen(true)}
      >
        <span className={s.summary}>{summary}</span>
        <span className={s.caret} aria-hidden>
          <svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
            <path d="m4 6 4 4 4-4" />
          </svg>
        </span>
      </button>

      {/* 关着就不挂 —— 每次打开都是一份干净的搜索状态 */}
      {open && (
        <PickDialog
          title={title}
          subtitle={subtitle}
          groups={groups}
          value={value}
          searchable={searchable}
          note={note}
          onPick={(next) => {
            onPick?.(next)
            setOpen(false)
          }}
          onClose={() => setOpen(false)}
        >
          {children}
        </PickDialog>
      )}
    </>
  )
}

interface DialogProps {
  title: string
  subtitle?: string
  groups?: GroupedGroup[]
  value?: string
  searchable: boolean
  note?: string
  onPick: (next: string) => void
  onClose: () => void
  children?: ReactNode
}

function PickDialog({
  title,
  subtitle,
  groups,
  value,
  searchable,
  note,
  onPick,
  onClose,
  children,
}: DialogProps) {
  const [query, setQuery] = useState('')
  /** 框内的挑选。初值 = 打开时那一项；「确认」才交给父框 */
  const [pending, setPending] = useState<string | null>(value ?? null)
  const searchId = useId()
  const hasList = groups !== undefined
  const canConfirm = hasList ? pending !== null && pending !== value : true

  const total = useMemo(
    () => (groups ?? []).reduce((n, g) => n + g.options.length, 0),
    [groups],
  )
  const withSearch = searchable && total > SEARCH_OVER

  const shown = useMemo(() => {
    if (groups === undefined) return []
    const q = query.trim().toLowerCase()
    if (q === '') return groups
    return groups
      .map((g) => ({
        title: g.title,
        options: g.options.filter(
          (o) => o.label.toLowerCase().includes(q) || (o.by ?? '').toLowerCase().includes(q),
        ),
      }))
      .filter((g) => g.options.length > 0)
  }, [groups, query])

  const empty = shown.every((g) => g.options.length === 0)

  return (
    <ModalC14
      open
      title={title}
      subtitle={subtitle}
      size="md"
      closeOnScrim={false}
      footLead={note === undefined ? undefined : <span className={s.note}>{note}</span>}
      onClose={onClose}
      footer={
        <>
          <button type="button" className={s.btn} onClick={onClose}>
            取消
          </button>
          <button
            type="button"
            className={`${s.btn} ${s.btnPrimary}`}
            disabled={!canConfirm}
            onClick={() => {
              if (hasList && pending !== null) onPick(pending)
              else onClose()
            }}
          >
            确认
          </button>
        </>
      }
    >
      <div className={s.body}>
        {withSearch && (
          <label className={s.search} htmlFor={searchId}>
            <input
              id={searchId}
              className={s.searchInput}
              type="search"
              value={query}
              placeholder="搜索名字或键名"
              autoFocus
              onChange={(e) => setQuery(e.target.value)}
            />
          </label>
        )}

        {children !== undefined ? (
          /* 分组面板没有自己的限高，不包一层会把模态框顶到屏幕外 */
          <div className={s.scroll}>{children}</div>
        ) : (
          <div className={s.list} role="listbox" aria-label={title}>
            {shown.map((g, gi) => (
              <div key={`${g.title ?? ''}-${gi}`} className={s.group}>
                {g.title !== null && <div className={s.groupHead}>{g.title}</div>}
                {g.options.map((o) => (
                  <button
                    key={o.value}
                    type="button"
                    role="option"
                    aria-selected={o.value === pending}
                    className={s.row}
                    data-on={o.value === pending ? 'true' : undefined}
                    disabled={o.disabled === true}
                    onClick={() => setPending(o.value)}
                  >
                    <span className={s.dot} aria-hidden />
                    <span className={s.rowText}>
                      <span className={s.rowLabel}>{o.label}</span>
                      {o.by !== undefined && <span className={s.rowBy}>{o.by}</span>}
                    </span>
                    {o.note !== undefined && <span className={s.rowNote}>{o.note}</span>}
                  </button>
                ))}
              </div>
            ))}
            {empty && <p className={s.empty}>没有匹配的项。</p>}
          </div>
        )}
      </div>
    </ModalC14>
  )
}
