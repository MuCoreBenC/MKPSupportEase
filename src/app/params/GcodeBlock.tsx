/*
 * 自定义 G-code 的整块编辑器。
 *
 * # 为什么用户端给整块，工作台给「N 行 · 点开」
 *
 * 共用件 `FieldControl` 对 `gcode` 只转交 `renderBlock`，不自造控件 —— 多行 G-code
 * 既塞不进一个格子也塞不进一行，而两个场合的取向不同：用户端就是来改这一段的（整块摊开），
 * 工作台是在一张密表里扫很多个组合（收成一个可点开的摘要）。这个文件是用户端那一半。
 *
 * # 两个模式
 *
 *   代码  整段等宽文本，能整体粘贴替换
 *   视图  每行拆成「行号 + 指令 chip + 一个个参数输入框」，改一个进给速度不用碰整行
 *
 * # 不认识的行就别装认识
 *
 * 只有首个 token 形如 `G1` / `M106` / `L801`（字母 + 数字）才当指令拆。其余（注释、空行、宏、
 * 没见过的写法）在视图模式下整行按纯文本显示 —— 猜错了会把要发给打印机的指令改坏。
 */

import { useEffect, useMemo, useRef, useState } from 'react'
import { useT, type StringKey } from '../../i18n'
import { SegmentedField } from '../../components/field'
import { tokenizeGcodeLine } from './gcode'
import {
  fieldOf,
  placeholders,
  render,
  validate,
  type PenKind,
  type PenPlaceholder,
  type PenTemplateIssue,
} from './penTemplate'
import type { ParamDef } from './useParams'
import s from './GcodeBlock.module.css'

/** 这一块是哪一种笔动作（占位符 → 字段的映射依赖它）。 */
function penKindOf(key: string): PenKind | null {
  if (key === 'toolhead.custom_mount_gcode') return 'mount'
  if (key === 'toolhead.custom_unmount_gcode') return 'unmount'
  return null
}

/** 结构错误的具体位置 —— 只有符号与数字，不需要再翻一遍。 */
function issueText(issue: PenTemplateIssue): string {
  if (issue.kind === 'unknown') return `{${issue.name}}`
  if (issue.kind === 'unclosed') return `{ … (${issue.at})`
  return `${issue.line}: ${issue.text}`
}

interface Props {
  def: ParamDef
  value: string
  dirty: boolean
  /** 被条件参数关掉：看得见、改不动 */
  disabled?: boolean
  /** 已保存的那一层 —— 还原 chip 指向它，与普通行同一条规矩 */
  savedValue: string
  /** 取别的参数的当前值（落笔/收笔要把占位符渲染成真值，必须借它） */
  valueOf?: (key: string) => string
  onEdit: (key: string, next: string) => void
  onRevertToSaved: (key: string) => void
}

const MODES: { value: string; label: StringKey }[] = [
  { value: 'code', label: 'params.gcode.code' },
  { value: 'view', label: 'params.gcode.view' },
]

export default function GcodeBlock({
  def,
  value,
  dirty,
  disabled = false,
  savedValue,
  valueOf,
  onEdit,
  onRevertToSaved,
}: Props) {
  const tr = useT()
  const [mode, setMode] = useState('view')

  /* 落笔/收笔的骨架带占位符：结构错了就当场标红，别等到保存 */
  const kind = penKindOf(def.key)
  const issue = kind === null ? null : validate(value)

  /*
   * 骨架 + 兄弟字段 → **真值**。取不到值（字段没填 / 结构坏了）就返回 null，
   * 界面退回骨架原文 —— 宁可显示 `{wait}`，也不编一个数字出来。
   */
  const rendered = useMemo(() => {
    if (kind === null || valueOf === undefined || issue !== null) return null
    const need = placeholders(value)
    if (need.length === 0) return value
    const vars: Record<PenPlaceholder, number> = { wait: 0, wait_y: 0, target: 0, speed: 0 }
    for (const name of need) {
      const raw = valueOf(fieldOf(kind, name))
      /* 空串必须当「没值」：`Number('') === 0`，放过去会渲染出 `X0` ——
       * 那是把工具头甩到零点的绝对坐标，比显示 `{wait}` 危险得多 */
      if (raw.trim() === '') return null
      const n = Number(raw)
      if (!Number.isFinite(n)) return null
      vars[name] = n
    }
    return render(value, vars)
  }, [kind, value, valueOf, issue])

  /*
   * 代码模式的三层：行号槽 + 着色层 + 透明输入框（A43 移植，工作台 C15 那套）。
   * 行号槽自己不滚，跟着输入框的 scrollTop 走 —— 这一支 ref 就是那条链。
   */
  const taRef = useRef<HTMLTextAreaElement | null>(null)
  const numRef = useRef<HTMLDivElement | null>(null)

  const srcLines = useMemo(() => value.split('\n'), [value])
  const highlighted = useMemo(
    () =>
      srcLines.map((line) => {
        const i = line.indexOf(';')
        return tokenizeGcodeLine(line, i < 0 ? null : i)
      }),
    [srcLines],
  )

  /* 三层同一把尺子（字体/行高/内衬写在 CSS 共用的一条里）：框高 = 行数 × 行高 + 上下内衬 */
  const CODE_LINE_H = 12.5 * 1.75
  const CODE_CHROME = 20
  const codeH = Math.max(1, srcLines.length) * CODE_LINE_H + CODE_CHROME

  /* 行号槽跟着输入框滚 —— 行号槽 overflow hidden，靠这里把它的内容挪上去 */
  const syncScroll = () => {
    if (numRef.current !== null && taRef.current !== null) {
      numRef.current.scrollTop = taRef.current.scrollTop
    }
  }
  useEffect(syncScroll, [value])

  return (
    <div className={s.wrap}>
      <header className={s.head}>
        <span className={s.headLeft}>
          <strong className={s.label}>{def.label}</strong>
          <span className={s.info} data-tip={`${def.desc}\n${def.meta?.tomlKey ?? def.key}`} aria-hidden>
            ?
          </span>
          {value.trim() !== '' && <span className={s.filled}>{tr('params.gcode.filled')}</span>}
        </span>
        <span className={s.headRight}>
          {dirty && !disabled && (
            <button
              type="button"
              className={s.restore}
              data-tip={tr('params.gcode.revertTitle', { lines: savedValue.split('\n').length })}
              onClick={() => onRevertToSaved(def.key)}
            >
              {tr('params.gcode.revertLabel', { lines: savedValue.split('\n').length })}
            </button>
          )}
          <SegmentedField
            variant="segmented"
            label={tr('params.gcode.displayMode', { label: def.label })}
            options={MODES.map((m) => ({ value: m.value, label: tr(m.label) }))}
            value={mode}
            onChange={setMode}
          />
        </span>
      </header>

      <p className={s.desc}>{def.desc}</p>

      {issue !== null && (
        <p className={s.structureError} role="alert">
          {tr('params.pen.structureError', { detail: issueText(issue) })}
        </p>
      )}

      {mode === 'code' ? (
        /*
         * 三层代码编辑器（A43 移植）：行号槽 + 着色层 + 透明输入框。
         * 框高按行数算成定值写在这里（不是 rows），行号槽「内容比框高长」也撑不开框。
         */
        <div className={s.codeEd} style={{ height: `${codeH}px` }}>
          <div ref={numRef} className={s.codeNum} aria-hidden style={{ height: `${codeH}px` }}>
            {srcLines.map((_, i) => (
              <div key={i}>{i + 1}</div>
            ))}
          </div>
          <div className={s.codeBox}>
            <pre className={s.codePre} aria-hidden>
              {highlighted.map((tokens, i) => (
                <div key={i}>
                  {tokens.map((t, j) => (
                    <span key={j} className={t.cls === undefined ? undefined : s[t.cls]}>
                      {t.text}
                    </span>
                  ))}
                  {tokens.length === 0 && '\u00a0'}
                </div>
              ))}
            </pre>
            <textarea
              ref={taRef}
              className={s.codeTa}
              value={rendered ?? value}
              spellCheck={false}
              disabled={disabled}
              /* 有真值就只读：这里展示的是拼装结果，改数值要回字段那几格 */
              readOnly={rendered !== null}
              aria-label={tr('params.gcode.codeAria', { label: def.label })}
              onScroll={syncScroll}
              onChange={(e) => onEdit(def.key, e.target.value)}
            />
          </div>
        </div>
      ) : (
        /*
         * 视图模式**不再重复铺一遍代码**：这一块的数值全在下面那几格里，
         * 代码照旧可以从「代码」页签看（那里已经是拼好的真值）。
         * 两处都放代码 = 同一份内容占两屏，而且上面那份还是可编辑的假数字。
         */
        <p className={s.viewNote}>{tr('params.pen.viewNote')}</p>
      )}
    </div>
  )
}
