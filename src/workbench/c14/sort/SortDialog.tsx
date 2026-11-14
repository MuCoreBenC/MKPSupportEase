/*
 * 排序模态框：一份草稿、一张可拖列表、确认才落盘。
 *
 * 落盘只把最终 key 序列交给 onApply，由调用方决定是哪条命令 —— 组内顺序走
 * wb_set_section_order，别的列表各给一条，不必改这一层。撤销 / 重做 / 恢复
 * 都只动草稿，点「取消」等于没来过。
 */

import { useEffect, useState } from 'react'

import { isAppError } from '../../api'
import ModalC14 from '../ModalC14'
import { toasts } from '../toast'
import ReorderList from './ReorderList'
import type { SortRow } from './types'
import { useSortDraft } from './useSortDraft'
import s from './SortDialog.module.css'

export interface SortDialogProps {
  title: string
  subtitle?: string
  rows: SortRow[]
  /** 打开时的次序；省略 = rows 给的顺序 */
  initialOrder?: string[]
  /** 当前项：列表里给它一行底色 */
  currentKey?: string
  /** 页脚说明；不写就不占那一行 */
  footNote?: string
  /** 列表的无障碍名字 */
  listLabel?: string
  /** 确认：交最终 key 序列，由调用方落盘 */
  onApply: (order: string[]) => Promise<void>
  onClose: () => void
}

export default function SortDialog({
  title,
  subtitle,
  rows,
  initialOrder,
  currentKey,
  footNote,
  listLabel,
  onApply,
  onClose,
}: SortDialogProps) {
  const draft = useSortDraft(initialOrder ?? rows.map((r) => r.key))
  const [busy, setBusy] = useState(false)

  /* 框里的撤销 / 重做：外壳那套被 modalShortcutGate 拦着，这里自己听 */
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey)) return
      const k = e.key.toLowerCase()
      if (k === 'z' && !e.shiftKey) {
        e.preventDefault()
        draft.undo()
      } else if ((k === 'z' && e.shiftKey) || k === 'y') {
        e.preventDefault()
        draft.redo()
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [draft])

  const commit = async () => {
    if (busy || !draft.dirty) return
    setBusy(true)
    try {
      await onApply(draft.order)
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
      title={title}
      subtitle={subtitle ?? `共 ${draft.order.length} 项 · 拖动、改数字、按 Alt+↑↓ 都行`}
      size="md"
      closeOnScrim={false}
      onClose={onClose}
      headerExtra={
        <div className={s.headTools}>
          <button
            type="button"
            className={s.iconBtn}
            title="撤销（Cmd/Ctrl+Z）"
            aria-label="撤销排序"
            disabled={!draft.canUndo}
            onClick={draft.undo}
          >
            <svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
              <path d="M3 7h6.5a3.5 3.5 0 0 1 0 7H6" />
              <path d="m6 4-3 3 3 3" />
            </svg>
          </button>
          <button
            type="button"
            className={s.iconBtn}
            title="重做（Cmd/Ctrl+Shift+Z）"
            aria-label="重做排序"
            disabled={!draft.canRedo}
            onClick={draft.redo}
          >
            <svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
              <path d="M13 7H6.5a3.5 3.5 0 0 0 0 7H10" />
              <path d="m10 4 3 3-3 3" />
            </svg>
          </button>
        </div>
      }
      footLead={
        <button
          type="button"
          className={s.linkBtn}
          disabled={!draft.dirty}
          onClick={draft.reset}
        >
          恢复
        </button>
      }
      footer={
        <>
          {footNote !== undefined && <span className={s.footNote}>{footNote}</span>}
          <button type="button" className={s.btn} onClick={onClose}>
            取消
          </button>
          <button
            type="button"
            className={`${s.btn} ${s.btnPrimary}`}
            disabled={!draft.dirty || busy}
            onClick={() => void commit()}
          >
            {busy ? '保存中…' : '确认'}
          </button>
        </>
      }
    >
      <ReorderList
        rows={rows}
        order={draft.order}
        currentKey={currentKey}
        onMove={draft.move}
        onSetPosition={draft.setPosition}
        label={listLabel ?? title}
      />
    </ModalC14>
  )
}
