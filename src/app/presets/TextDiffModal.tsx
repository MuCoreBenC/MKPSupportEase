/*
 * **与默认值对比**（2026-10-21）。
 *
 * 作者这一轮的七条：
 *   ① 摘要那行小字放**标题右侧**（`HeadNote`，字数不影响框的尺寸）；
 *   ② 「可以切换成像 IDE 一样，直接就在下面或者左右两侧这样对比」→ 左右 / 上下 两档；
 *   ③ 「希望它可以编辑，不只是查看」→ **点右边任意一行就地改**（回车生效）；
 *   ④ 「进到这个页面的时候，不只是一个关闭按钮，也可以选择恢复默认」→ 底栏「恢复默认值」；
 *   ⑤ 不叫「与最初那份一致」——那明明是"与默认值"比；
 *   ⑥ 底栏那颗「默认值」小按钮撤掉，恢复只在对比这一屏里决定；
 *   ⑦ 「这下面为什么做这么大」「这一行字可以不要」「默认值和现在这个提示也可以不要」
 *      → 删掉提示行与图例，正文区铺满到底。
 *
 * 左右并排时左边是默认值、右边是现在；上下（统一视图）时每行前面带 `-` / `+` / `~`，
 * 一条 `change` 在统一视图里铺成"删一行 + 加一行"（IDE 的老规矩）。
 * `lineDiff` 是前端纯函数，开着这一屏接着改，红绿块跟着变。
 */

import { useMemo, useState } from 'react'

import { HeadNote, Modal } from '../../components/modal'
import { Btn } from '../ui/Controls'
import s from './TextDiffModal.module.css'
import { diffSummary, lineDiff } from './textDiff'

export default function TextDiffModal({
  open,
  host,
  fileName,
  left,
  right,
  onRestore,
  onEditLine,
  onClose,
}: {
  open: boolean
  /** 宿主的整块页面元素（与别的模态框同一条：框要算出可用宽度） */
  host: HTMLElement | null
  fileName: string | undefined
  /** 默认值（基准） */
  left: string
  /** 现在这份 */
  right: string
  /** 恢复成默认值（底栏那颗；算一次改动，撤销能退回来） */
  onRestore: () => void
  /** 现在这份的某一行被就地改掉（`lineNo` 1 起） */
  onEditLine: (lineNo: number, next: string) => void
  onClose: () => void
}) {
  const rows = useMemo(() => lineDiff(left, right), [left, right])
  const sum = useMemo(() => diffSummary(rows), [rows])
  const total = sum.changed + sum.added + sum.removed
  /** 布局：左右并排 / 上下（统一视图，带 -/+） */
  const [layout, setLayout] = useState<'split' | 'unified'>('split')
  /** 正在就地编辑的那一行（**现在这份**的行号，1 起）；`null` = 没有 */
  const [editingLine, setEditingLine] = useState<number | null>(null)
  const [draft, setDraft] = useState('')

  const note =
    total === 0
      ? '与默认值完全一致'
      : `与默认值比：改了 ${sum.changed} 行` +
        (sum.added > 0 ? `，多了 ${sum.added} 行` : '') +
        (sum.removed > 0 ? `，少了 ${sum.removed} 行` : '')

  const beginEdit = (lineNo: number, text: string) => {
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
          <Btn variant="ghost" onClick={onClose}>
            关闭
          </Btn>
          <Btn
            variant="danger"
            className={s.restoreBtn}
            onClick={onRestore}
            disabled={left === right}
          >
            恢复默认值
          </Btn>
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
                  {editingLine !== null && r.rightNo === editingLine ? (
                    <span className={s.code} data-side="right" role="cell">
                      {editor}
                    </span>
                  ) : (
                    <span
                      className={s.code}
                      data-side="right"
                      role="cell"
                      title={r.right ?? '（这一行现在没有）'}
                      onClick={() => {
                        if (r.rightNo === null) return
                        beginEdit(r.rightNo, r.right ?? '')
                      }}
                    >
                      {r.right ?? ''}
                    </span>
                  )}
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
                        {editingLine !== null && r.rightNo === editingLine ? (
                          <span className={s.code} role="cell">
                            {editor}
                          </span>
                        ) : (
                          <span
                            className={s.code}
                            role="cell"
                            title={r.right ?? undefined}
                            onClick={() => {
                              if (r.rightNo === null) return
                              beginEdit(r.rightNo, r.right ?? '')
                            }}
                          >
                            {r.right ?? ''}
                          </span>
                        )}
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
                    {editingLine !== null && r.rightNo === editingLine ? (
                      <span className={s.code} role="cell">
                        {editor}
                      </span>
                    ) : (
                      <span
                        className={s.code}
                        role="cell"
                        title={text}
                        onClick={() => {
                          if (r.rightNo === null) return
                          beginEdit(r.rightNo, text)
                        }}
                      >
                        {text}
                      </span>
                    )}
                  </div>
                )
              })}
        </div>
      </div>
    </Modal>
  )
}
