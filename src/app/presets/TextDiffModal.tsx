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

import { useCallback, useMemo, useRef, useState } from 'react'

import { useT, useText, type Text } from '../../i18n'
import { HeadNote, Modal } from '../../components/modal'
/* 底栏按钮**直接用编辑文件那一套类**：两边高度必须是同一个数（见 `.footLeft` 那段注释） */
import ps from './PagePresets.module.css'
import s from './TextDiffModal.module.css'
import { EDIT_TEXT } from './presetTree'
import { diffSummary, lineDiff } from './textDiff'

/** 与「编辑文件」共用的一套底栏状态（词与行为都由调用方给，这一屏只管摆） */
export interface DiffShell {
  /** 在编辑模式里没有 */
  active: boolean
  canUndo: boolean
  canRedo: boolean
  saveDisabled: boolean
  /** 底栏左侧那行红字（草稿没写进去 / 没存上）—— 与编辑文件同一个位置。
   *  [`Text`]：它存的是键（`msg`），渲染时才翻 —— 这一屏开着切语言也得跟着换。 */
  error: Text | null
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
  const tr = useT()
  const say = useText()
  const rows = useMemo(() => lineDiff(left, right), [left, right])
  const sum = useMemo(() => diffSummary(rows), [rows])
  const total = sum.changed + sum.added + sum.removed
  const [layout, setLayout] = useState<'split' | 'unified'>('split')
  const [editingLine, setEditingLine] = useState<number | null>(null)

  const editable = shell.active

  /*
   * 就地改一行：**用 contentEditable 的 span，不用 `<input>`**（2026-10-21 第三版）。
   *
   * 前两版都栽在同一件事上 —— 作者：「我点击了之后，我的文字是不是有上下一点点几个像素的移动」
   * 「我右侧的绿色背景，你不要把它搞丢」：
   *   · `<input>` 是替换元素：字体 / 行高 / 基线都是它自己那套，即便尺寸算对了，
   *     字仍然比旁边的 `<span>` 偏上或偏下一两像素；白色输入底还会把这一行的红/绿底盖掉。
   *   · contentEditable 的 span **就是那一行本身** —— 同一套 class、同一个行盒、
   *     同一个 `line-height`，字的落点一个像素都不动，行的底色（红/绿/黄）原样透出来。
   *
   * 所以这里只管"把字放进去、让它可编辑、结束时把内容交出去"，
   * 颜色 / 内衬 / 字号全部交给 `.code` 那一层，不再另写一套排版。
   */
  const draftRef = useRef('')
  const editRef = useRef<HTMLSpanElement | null>(null)

  const beginEdit = (lineNo: number, text: string) => {
    if (!editable) return
    draftRef.current = text
    setEditingLine(lineNo)
  }

  /* 挂上即用：填字、聚焦、光标落到末尾。**不走 state** —— 每敲一个字重渲染会跳光标 */
  const attachEditor = useCallback((el: HTMLSpanElement | null) => {
    editRef.current = el
    if (el === null) return
    el.textContent = draftRef.current
    el.focus()
    const range = document.createRange()
    range.selectNodeContents(el)
    range.collapse(false)
    const sel = window.getSelection()
    if (sel !== null) {
      sel.removeAllRanges()
      sel.addRange(range)
    }
  }, [])

  const commitLine = () => {
    const el = editRef.current
    if (editingLine === null || el === null) return
    /*
     * 粘贴进来的东西可能带换行（复制两三行再粘）—— 一行就是一行，
     * 折成空格交给它，别把这一行撑成两行（那会让右栏行号整片错位）。
     */
    const next = (el.textContent ?? '').replace(/\s*[\r\n]+\s*/g, ' ')
    editRef.current = null
    setEditingLine(null)
    if (next !== draftRef.current) onEditLine(editingLine, next)
  }

  const cancelLine = () => {
    editRef.current = null
    setEditingLine(null)
  }

  const note =
    total === 0
      ? tr('presets.diff.same')
      : tr('presets.diff.changed', { changed: sum.changed }) +
        (sum.added > 0 ? tr('presets.diff.added', { count: sum.added }) : '') +
        (sum.removed > 0 ? tr('presets.diff.removed', { count: sum.removed }) : '')

  /** 一行的右侧格子：编辑态才能点开改；只读态就是一个 `<span>` */
  const rightCell = (lineNo: number | null, text: string | null, cls: string) => {
    if (editingLine !== null && lineNo === editingLine) {
      return (
        <span
          className={`${cls} ${s.lineEdit}`}
          role="cell"
          /* ★ 右侧这一格必须带 `data-side="right"`（2026-10-21 作者的绿底一直没出现）：
             绿 / 红底是 `.row[data-kind] .code[data-side='right']` 那条规则画的，
             这里原来没这个属性 —— 于是右侧改过的行从来没有底色，
             反倒只有左栏（本来就有 `data-side="left"`）是红的。 */
          data-side="right"
          data-editing="true"
          ref={attachEditor}
          contentEditable
          suppressContentEditableWarning
          spellCheck={false}
          onBlur={commitLine}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              e.preventDefault()
              commitLine()
            } else if (e.key === 'Escape') {
              /*
               * 这一层先吃 Esc：退出"改这一行"，而不是把整个对比屏关掉
               * （`stopPropagation` 挡住冒泡到 document 的那个 Modal 监听）。
               */
              e.preventDefault()
              e.stopPropagation()
              cancelLine()
            }
          }}
        />
      )
    }
    return (
      <span
        className={cls}
        role="cell"
        /*
         * ★ 右侧这一格**也要** `data-side`（2026-10-21 作者第二次说绿底没出来）：
         * 上一版只给了"正在编辑的那一格"，普通行仍然是光秃秃的 `<span>` ——
         * 而 `.row[data-kind] .code[data-side='right']` 那条绿底规则认的就是这个属性。
         * 左栏从头到尾都有 `data-side="left"`，所以屏幕上一直是"左边红、右边白"。
         */
        data-side="right"
        data-editable={editable || undefined}
        data-tip={text ?? undefined}
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
      title={tr('presets.diff.title')}
      subtitle={fileName}
      onClose={onClose}
      footer={
        <>
          {/* 左簇：布局切换 + 「恢复默认值」（作者：别都堆在右边） */}
          <span className={s.footLeft}>
            <span className={s.layoutSwitch} role="group" aria-label={tr('presets.diff.layout')}>
              <button
                type="button"
                data-on={layout === 'split' || undefined}
                onClick={() => setLayout('split')}
              >
                {tr('presets.diff.sideBySide')}
              </button>
              <button
                type="button"
                data-on={layout === 'unified' || undefined}
                onClick={() => setLayout('unified')}
              >
                {tr('presets.diff.stacked')}
              </button>
            </span>
            <button
              type="button"
              className={`${s.pill} ${s.pillDanger}`}
              onClick={onRestore}
              disabled={left === right}
            >
              {tr('presets.diff.restore')}
            </button>
          </span>

          {shell.error !== null && <span className={ps.editorStateBad}>{say(shell.error)}</span>}

          <span className={s.footActs}>
            {editable ? (
              <>
                <button
                  type="button"
                  className={s.pill}
                  onClick={shell.onUndo}
                  disabled={!shell.canUndo}
                  data-tip={tr('presets.diff.undo')}
                >
                  {tr('presets.diff.undo')}
                </button>
                <button
                  type="button"
                  className={s.pill}
                  onClick={shell.onRedo}
                  disabled={!shell.canRedo}
                  data-tip={tr('presets.diff.redo')}
                >
                  {tr('presets.diff.redo')}
                </button>
                <button
                  type="button"
                  className={s.pill}
                  onClick={shell.onExit}
                  data-tip={tr(EDIT_TEXT.exitTitle)}
                >
                  {tr(EDIT_TEXT.exitMode)}
                </button>
                <button
                  type="button"
                  className={s.pillPrimary}
                  onClick={shell.onSave}
                  disabled={shell.saveDisabled}
                >
                  {tr(EDIT_TEXT.commit)}
                </button>
              </>
            ) : (
              <>
                <button type="button" className={s.pill} onClick={onClose}>
                  {tr('presets.diff.close')}
                </button>
                <button type="button" className={s.pillPrimary} onClick={shell.onEnter}>
                  {tr(EDIT_TEXT.enterMode)}
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
        <div
          className={s.diff}
          data-layout={layout}
          role="table"
          aria-label={tr('presets.diff.diffAria')}
        >
          {layout === 'split'
            ? rows.map((r, i) => (
                <div className={s.row} data-kind={r.kind} role="row" key={i}>
                  <span className={s.no} role="cell">
                    {r.leftNo ?? ''}
                  </span>
                  <span className={s.code} data-side="left" role="cell" data-tip={r.left ?? undefined}>
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
                        <span className={s.code} role="cell" data-tip={r.left ?? undefined}>
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
