/*
 * **与默认值对比**（2026-10-21）。
 *
 * # 编辑模式是**同一套外壳**
 *
 * 作者：「我不想直接就能编辑啊，你还是得给我一个按钮，就像外面的一样，进入编辑模式这种按钮」
 *     「我希望你把这个编辑模式做成一个那种后壳，就是大家都是一样的……撤销重做之类的都一样，
 *       甚至包括那个警告啊、那些出错啊，那些都一样」。
 *
 * 所以这一屏和「编辑文件」共用同一套两态：
 *   只读   底栏右侧「进入编辑模式」；右侧正文**点不动**（只看）
 *   编辑   底栏右侧 撤销 / 重做 / 放弃并退出 / 保存 —— 与编辑文件同一批 handler
 *          （`onUndo` / `onRedo` / `onExit` / `onSave` 就是那边那四个函数），
 *          红字错误也在底栏左侧同一位置（`shell.error`）。
 *
 * 差别只有一处：这里右侧每一行**点一下就改**（编辑态才有），因为这一屏就是拿来看差异的。
 *
 * 其余（2026-10-21 前几轮定的）：左右 / 上下 两档；`lineDiff` 是前端纯函数，开着改实时变；
 * 摘要进标题右边那一格（`HeadNote`）。
 */

import { useMemo, useState } from 'react'

import { HeadNote, Modal } from '../../components/modal'
/* 底栏按钮**直接用编辑文件那一套类**：两边高度必须是同一个数（见 `.footLeft` 那段注释） */
import ps from './PagePresets.module.css'
import s from './TextDiffModal.module.css'
import { diffSummary, lineDiff } from './textDiff'

/** 与「编辑文件」共用的一套底栏状态（词与行为都由调用方给，这一屏只管摆） */
export interface DiffShell {
  /** 在编辑模式里没有 */
  active: boolean
  canUndo: boolean
  canRedo: boolean
  saveDisabled: boolean
  /** 底栏左侧那行红字（草稿没写进去 / 没存上）—— 与编辑文件同一个位置 */
  error: string | null
  onEnter: () => void
  onUndo: () => void
  onRedo: () => void
  /** 放弃并退出 (= 作废这次改动) */
  onExit: () => void
  onSave: () => void
}

export default function TextDiffModal({
  open,
  host,
  fileName,
  left,
  right,
  shell,
  onRestore,
  onEditLine,
  onClose,
}: {
  open: boolean
  host: HTMLElement | null
  fileName: string | undefined
  /** 默认值（基准） */
  left: string
  /** 现在这份 */
  right: string
  shell: DiffShell
  onRestore: () => void
  /** 现在这份的某一行被就地改掉（`lineNo` 1 起） */
  onEditLine: (lineNo: number, next: string) => void
  onClose: () => void
}) {
  const rows = useMemo(() => lineDiff(left, right), [left, right])
  const sum = useMemo(() => diffSummary(rows), [rows])
  const total = sum.changed + sum.added + sum.removed
  const [layout, setLayout] = useState<'split' | 'unified'>('split')
  const [editingLine, setEditingLine] = useState<number | null>(null)
  const [draft, setDraft] = useState('')

  const note =
    total === 0
      ? '与默认值完全一致'
      : `与默认值比：改了 ${sum.changed} 行` +
        (sum.added > 0 ? `，多了 ${sum.added} 行` : '') +
        (sum.removed > 0 ? `，少了 ${sum.removed} 行` : '')

  const editable = shell.active
  const beginEdit = (lineNo: number, text: string) => {
    if (!editable) return
    setEditingLine(lineNo)
    setDraft(text)
  }
  const commitEdit = () => {
    if (editingLine === null) return
    onEditLine(editingLine, draft)
    setEditingLine(null)
  }

  const editor = (
    <input
      className={s.lineInput}
      value={draft}
      autoFocus
      onChange={(e) => setDraft(e.target.value)}
      onBlur={commitEdit}
      onKeyDown={(e) => {
        if (e.key === 'Enter') commitEdit()
        else if (e.key === 'Escape') setEditingLine(null)
      }}
    />
  )

  /** 一行的右侧格子：编辑态才能点开改；只读态就是一个 `<span>` */
  const rightCell = (lineNo: number | null, text: string | null, cls: string) => {
    if (editingLine !== null && lineNo === editingLine) {
      return (
        <span className={cls} role="cell">
          {editor}
        </span>
      )
    }
    return (
      <span
        className={cls}
        role="cell"
        data-editable={editable || undefined}
        title={text ?? undefined}
        onClick={() => {
          if (!editable || lineNo === null) return
          beginEdit(lineNo, text ?? '')
        }}
      >
        {text ?? ''}
      </span>
    )
  }

  return (
    <Modal
      open={open}
      size="xl"
      bodyFill
      host={host}
      title="与默认值对比"
      subtitle={fileName}
      onClose={onClose}
      footer={
        <>
          {/* 左簇：布局切换 + 「恢复默认值」（作者：别都堆在右边） */}
          <span className={s.footLeft}>
            <span className={s.layoutSwitch} role="group" aria-label="对比布局">
              <button
                type="button"
                data-on={layout === 'split' || undefined}
                onClick={() => setLayout('split')}
              >
                左右
              </button>
              <button
                type="button"
                data-on={layout === 'unified' || undefined}
                onClick={() => setLayout('unified')}
              >
                上下
              </button>
            </span>
            <button
              type="button"
              className={ps.editGhost}
              onClick={onRestore}
              disabled={left === right}
            >
              恢复默认值
            </button>
          </span>

          {shell.error !== null && <span className={ps.editorStateBad}>{shell.error}</span>}

          <span className={s.footActs}>
            {editable ? (
              <>
                <button
                  type="button"
                  className={ps.editGhost}
                  onClick={shell.onUndo}
                  disabled={!shell.canUndo}
                  title="撤销"
                >
                  撤销
                </button>
                <button
                  type="button"
                  className={ps.editGhost}
                  onClick={shell.onRedo}
                  disabled={!shell.canRedo}
                  title="重做"
                >
                  重做
                </button>
                <button type="button" className={ps.editGhost} onClick={shell.onExit}>
                  放弃并退出
                </button>
                <button
                  type="button"
                  className={ps.editPrimary}
                  onClick={shell.onSave}
                  disabled={shell.saveDisabled}
                >
                  保存
                </button>
              </>
            ) : (
              <>
                <button type="button" className={ps.editGhost} onClick={onClose}>
                  关闭
                </button>
                <button type="button" className={ps.editPrimary} onClick={shell.onEnter}>
                  进入编辑模式
                </button>
              </>
            )}
          </span>
        </>
      }
    >
      {/* 摘要进标题右边那一格（作者：「应该放在标题右侧吧」） */}
      <HeadNote text={note} tone={total === 0 ? 'ok' : 'note'} />

      <div className={s.wrap}>
        <div className={s.diff} data-layout={layout} role="table" aria-label="与默认值的差异">
          {layout === 'split'
            ? rows.map((r, i) => (
                <div className={s.row} data-kind={r.kind} role="row" key={i}>
                  <span className={s.no} role="cell">
                    {r.leftNo ?? ''}
                  </span>
                  <span className={s.code} data-side="left" role="cell" title={r.left ?? undefined}>
                    {r.left ?? ''}
                  </span>
                  <span className={s.no} role="cell">
                    {r.rightNo ?? ''}
                  </span>
                  {rightCell(r.rightNo, r.right, s.code)}
                </div>
              ))
            : rows.map((r, i) => {
                if (r.kind === 'change') {
                  return (
                    <div key={i}>
                      <div className={s.rowU} data-kind="del" role="row">
                        <span className={s.mark} aria-hidden="true">
                          -
                        </span>
                        <span className={s.no} role="cell">
                          {r.leftNo ?? ''}
                        </span>
                        <span className={s.code} role="cell" title={r.left ?? undefined}>
                          {r.left ?? ''}
                        </span>
                      </div>
                      <div className={s.rowU} data-kind="add" role="row">
                        <span className={s.mark} aria-hidden="true">
                          +
                        </span>
                        <span className={s.no} role="cell">
                          {r.rightNo ?? ''}
                        </span>
                        {rightCell(r.rightNo, r.right, s.code)}
                      </div>
                    </div>
                  )
                }
                const text = r.right ?? r.left ?? ''
                return (
                  <div className={s.rowU} data-kind={r.kind} role="row" key={i}>
                    <span className={s.mark} aria-hidden="true">
                      {r.kind === 'add' ? '+' : r.kind === 'del' ? '-' : ' '}
                    </span>
                    <span className={s.no} role="cell">
                      {r.rightNo ?? r.leftNo ?? ''}
                    </span>
                    {rightCell(r.rightNo, text, s.code)}
                  </div>
                )
              })}
        </div>
      </div>
    </Modal>
  )
}
