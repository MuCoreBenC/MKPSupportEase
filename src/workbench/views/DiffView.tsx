/*
 * 行级 diff 与右侧概览条（2026-10-19 从 `GenerateDiffModal` 抽出）。
 *
 * 两个框共用这一份渲染：**生成前确认**与**交付明细**。
 * 同一件事（"这一份和盘上比变了哪几行"）不该在两个地方长成两种样子 ——
 * 行号口径、红绿底、语法上色、概览条跳转都只有这一处实现。
 */

import { useMemo } from 'react'
import type { Ref } from 'react'
import { CodeLine } from '../c14/CodeText'
import { highlightLines, langOf } from '../c14/syntax'
import type { PreviewDiffLine, PreviewFile } from '../api'
import type { RulerMark } from './diffRuler'
import s from './DiffView.module.css'

/** 详情区显示哪种：新文件全文（`full`）或与磁盘现存的行级对比（`diff`） */
export type DiffViewMode = 'full' | 'diff'

/**
 * 详情正文：`view === 'full'` 给新文件全文，`'diff'` 给整份摊开的对比。
 *
 * 行号口径：`removed` 行在后端记的是**旧文件**行号，其余记**新文件**行号 ——
 * 所以 `removed` 行的行号槽留空（「−」符已经说明它是删掉的），保证一列里
 * 只有一套「新文件第几行」的语义，不再出现 39、40 跳 41、42 的看不懂。
 *
 * **语法上色**（2026-10-07）：`fileName` 只为认文法（`.json` / `.toml`）——
 * 作者原话「不要都是黑色的字，变一下颜色，像 VSCode 里面的插件一样」。
 * 上色类名与只看盘上原文那个块共用一份（`c14.module.css` 的 `tok_*`）。
 */
export function DiffLines({
  lines,
  mode,
  view,
  fileName,
  innerRef,
}: {
  lines: PreviewDiffLine[]
  mode: PreviewFile['state']
  view: DiffViewMode
  fileName: string
  /** diff 内容节点：概览条按它的行位置测比例 */
  innerRef?: Ref<HTMLDivElement>
}) {
  /*
   * 完整视图的行 = 丢弃 `removed`（旧文件才有的行），剩下的 context / added
   * 就是新文件全文；后端按 diff 序输出（删在前、增在后），这里按新行号排回去。
   * 新文件行号唯一 → 可以直接当 key。
   */
  const fullRows = useMemo(
    () =>
      lines
        .filter((l) => l.kind !== 'removed')
        .slice()
        .sort((a, b) => a.no - b.no),
    [lines],
  )

  /*
   * 两套视图的 token 各自算：`full` 丢了 removed 行，行数与后端给的那一份对不上 ——
   * 共用一个 token 列表就会串行（一行的颜色贴到另一行上）。
   * 认不出文法（`null`）⇒ 原样渲染，一个字都不动。
   */
  const lang = langOf(fileName)
  const fullTokens = useMemo(
    () => (lang === null ? null : highlightLines(fullRows.map((l) => l.text), lang)),
    [lang, fullRows],
  )
  const diffTokens = useMemo(
    () => (lang === null ? null : highlightLines(lines.map((l) => l.text), lang)),
    [lang, lines],
  )

  if (view === 'full') {
    return (
      <div className={s.diff} ref={innerRef}>
        {fullRows.map((l, i) => (
          <div key={l.no} className={s.lineCtx}>
            <span className={s.gutter} aria-hidden />
            <span className={s.lineNo}>{l.no}</span>
            <code className={s.lineTxt}>
              <CodeLine line={l.text} tokens={fullTokens?.[i]} />
            </code>
          </div>
        ))}
      </div>
    )
  }

  /* 对比视图：**不省略** —— 后端给的每一行（含未变的 context）都平铺出来，
     与「完整」的差别只剩红绿底。行序就是后端的 diff 序（删在前、增在后，与 git 同序）。
     `data-kind` / `data-row` / `data-line` 是概览条与跳转的锚点（比按类名找行稳）。 */
  return (
    <div className={s.diff} data-mode={mode} ref={innerRef}>
      {lines.map((l, i) => (
        <div
          key={`${i}-${l.kind}-${l.no}`}
          data-kind={l.kind}
          data-row={i}
          data-line={l.no}
          className={l.kind === 'added' ? s.lineAdd : l.kind === 'removed' ? s.lineDel : s.lineCtx}
        >
          <span className={s.gutter} aria-hidden>
            {l.kind === 'added' ? '+' : l.kind === 'removed' ? '−' : ''}
          </span>
          <span className={s.lineNo}>{l.kind === 'removed' ? '' : l.no}</span>
          <code className={s.lineTxt}>
            <CodeLine line={l.text} tokens={diffTokens?.[i]} />
          </code>
        </div>
      ))}
    </div>
  )
}

/**
 * 右侧差异概览条（VSCode 的 overview ruler）：按行位置画增 / 删标记，点一下跳过去。
 *
 * 只画「对比」视图 —— 「完整」视图是素底全文，没有"哪里变了"这层信息。
 * 标记是按钮而不是装饰：作者要的就是"方便划到对应位置"。
 */
export function DiffRuler({
  marks,
  onJump,
}: {
  marks: RulerMark[]
  onJump: (row: number) => void
}) {
  if (marks.length === 0) return null
  return (
    <div className={s.ruler} role="group" aria-label="差异位置概览">
      {marks.map((m) => (
        <button
          key={`${m.kind}-${m.row}`}
          type="button"
          className={`${s.rulerMark} ${m.kind === 'add' ? s.rulerAdd : s.rulerDel}`}
          style={{ top: `${m.top * 100}%`, height: `${Math.max(m.height * 100, 0.6)}%` }}
          data-tip={
            m.kind === 'add'
              ? `跳到新增行（第 ${m.line} 行）`
              : `跳到删除行（旧文件第 ${m.line} 行）`
          }
          aria-label={
            m.kind === 'add'
              ? `跳到新增行，第 ${m.line} 行`
              : `跳到删除行，旧文件第 ${m.line} 行`
          }
          onClick={() => onJump(m.row)}
        />
      ))}
    </div>
  )
}
