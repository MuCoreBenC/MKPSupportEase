/*
 * 「组内顺序」的入口：一颗触发器 + 共用排序框。
 *
 * 排序能挪到别人的位置。要塞进「这条参数的定义载荷」里，就得把别人的定义原样
 * 重抄一遍（抄漏一格 = 静默改坏一条参数），而且 N 条就是 N 次写盘。所以确认时
 * 只交最终 key 序列，走 wb_set_section_order：顺序值由后端算，一次写盘一条撤销。
 * 页脚那句「顺序单独生效」说的就是它不等父框那颗「确认」。
 *
 * 列表怎么拖、怎么落地、草稿怎么撤销重做都在 c14/sort，这里只管落盘。
 */

import { useState } from 'react'

import { wb } from '../../api'
import type { RegistryView } from '../../api'
import SortDialog from '../../c14/sort/SortDialog'
import type { SortRow } from '../../c14/sort/types'
import { toasts } from '../../c14/toast'
import s from './SortModal.module.css'

export type { SortRow } from '../../c14/sort/types'

export interface SortOrderButtonProps {
  sectionId: string
  /** 触发器上那截说明用的组名（「动作与挤出」） */
  sectionLabel: string
  /** 整组参数，**已按当前顺序排好**（含自己） */
  rows: SortRow[]
  /** 当前正在编辑的那一条（框里标一条细轨，触发器上写「第 N / 共 M 位」） */
  currentKey?: string
  /** 只读（分组只剩一条之类）：触发器禁用 */
  disabled?: boolean
  /**
   * 一组都排不了的时候说明一句（比如这一组还没落盘）——
   * 不给的话触发器只写「共 N 项」，那句"点了没反应"就只能靠猜
   */
  emptyHint?: string
  /** 落盘之后把新的注册表视图交回页面 */
  onApplied?: (view: RegistryView) => void
}

export default function SortOrderButton({
  sectionId,
  sectionLabel,
  rows,
  currentKey,
  disabled = false,
  emptyHint,
  onApplied,
}: SortOrderButtonProps) {
  const [open, setOpen] = useState(false)
  const at = currentKey === undefined ? -1 : rows.findIndex((r) => r.key === currentKey)
  const summary =
    at < 0 ? `共 ${rows.length} 项` : `第 ${at + 1} / ${rows.length} 位`

  const apply = async (order: string[]) => {
    const view = await wb.setSectionOrder(sectionId, order)
    onApplied?.(view)
    toasts.push(`「${sectionLabel}」的组内顺序已改，撤销可退回`)
  }

  return (
    <>
      <button
        type="button"
        className={s.trigger}
        aria-label="组内顺序"
        aria-haspopup="dialog"
        title={
          rows.length < 2 && emptyHint !== undefined
            ? emptyHint
            : disabled
              ? '这一组只有一条，排不了'
              : '点开排这一组的先后，可拖、可输位次'
        }
        disabled={disabled || rows.length < 2}
        onClick={() => setOpen(true)}
      >
        <span className={s.summary}>{summary}</span>
        <span className={s.caret} aria-hidden>
          <svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
            <path d="m4 6 4 4 4-4" />
          </svg>
        </span>
      </button>

      {open && (
        <SortDialog
          title={`组内顺序 · ${sectionLabel}`}
          footNote="顺序单独生效，不用等这一条的定义确认"
          listLabel={`${sectionLabel}的组内顺序`}
          rows={rows}
          currentKey={currentKey}
          onApply={apply}
          onClose={() => setOpen(false)}
        />
      )}
    </>
  )
}
