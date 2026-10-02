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
import { SegmentedField } from '../../components/field'
import { tokenizeGcodeLine } from './gcode'
import type { ParamDef } from './useParams'
import s from './GcodeBlock.module.css'

interface Props {
  def: ParamDef
  value: string
  dirty: boolean
  /** 被条件参数关掉：看得见、改不动 */
  disabled?: boolean
  /** 已保存的那一层 —— 还原 chip 指向它，与普通行同一条规矩 */
  savedValue: string
  onEdit: (key: string, next: string) => void
  onRevertToSaved: (key: string) => void
}

const MODES = [
  { value: 'code', label: '代码' },
  { value: 'view', label: '视图' },
]

const IS_COMMAND = /^[A-Za-z]\d+$/

interface Parsed {
  raw: string
  command: string | null
  params: string[]
}

function parse(text: string): Parsed[] {
  return text.split('\n').map((raw) => {
    const tokens = raw.trim().split(/\s+/).filter(Boolean)
    const head = tokens[0] ?? ''
    if (!IS_COMMAND.test(head)) return { raw, command: null, params: [] }
    return { raw, command: head, params: tokens.slice(1) }
  })
}

export default function GcodeBlock({
  def,
  value,
  dirty,
  disabled = false,
  savedValue,
  onEdit,
  onRevertToSaved,
}: Props) {
  const [mode, setMode] = useState('view')
  const lines = parse(value)

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

  const replaceParam = (lineAt: number, paramAt: number, next: string) => {
    const copy = lines.map((l, i) => {
      if (i !== lineAt || l.command === null) return l.raw
      const params = [...l.params]
      params[paramAt] = next
      return [l.command, ...params.filter((p) => p !== '')].join(' ')
    })
    onEdit(def.key, copy.join('\n'))
  }

  return (
    <div className={s.wrap}>
      <header className={s.head}>
        <span className={s.headLeft}>
          <strong className={s.label}>{def.label}</strong>
          <span className={s.info} title={`${def.desc}\n${def.meta?.tomlKey ?? def.key}`} aria-hidden>
            ?
          </span>
          {value.trim() !== '' && <span className={s.filled}>已填写</span>}
        </span>
        <span className={s.headRight}>
          {dirty && !disabled && (
            <button
              type="button"
              className={s.restore}
              title={`还原为已保存的那一段（${savedValue.split('\n').length} 行）`}
              onClick={() => onRevertToSaved(def.key)}
            >
              ↩ 已保存的 {savedValue.split('\n').length} 行
            </button>
          )}
          <SegmentedField
            variant="segmented"
            label={`${def.label} 显示方式`}
            options={MODES}
            value={mode}
            onChange={setMode}
          />
        </span>
      </header>

      <p className={s.desc}>{def.desc}</p>

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
              value={value}
              spellCheck={false}
              disabled={disabled}
              aria-label={`${def.label} 代码`}
              onScroll={syncScroll}
              onChange={(e) => onEdit(def.key, e.target.value)}
            />
          </div>
        </div>
      ) : (
        <ol className={s.lines}>
          {lines.map((line, i) => (
            <li key={`${i}-${line.raw}`} className={s.line}>
              <span className={s.no}>{i + 1}</span>
              {line.command === null ? (
                <span className={s.plain}>{line.raw === '' ? '\u00a0' : line.raw}</span>
              ) : (
                <>
                  <span className={s.cmd}>{line.command}</span>
                  {line.params.map((p, j) => (
                    <input
                      key={`${i}-${j}`}
                      className={s.param}
                      value={p}
                      spellCheck={false}
                      disabled={disabled}
                      aria-label={`第 ${i + 1} 行参数 ${j + 1}`}
                      onChange={(e) => replaceParam(i, j, e.target.value)}
                    />
                  ))}
                </>
              )}
            </li>
          ))}
        </ol>
      )}
    </div>
  )
}
