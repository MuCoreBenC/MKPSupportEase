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

import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { useT, type StringKey } from '../../i18n'
import { SegmentedField } from '../../components/field'
import { tokenizeGcodeLine } from './gcode'
import {
  alignDiff,
  fieldOf,
  placeholders,
  render,
  validate,
  type PenKind,
  type PenPlaceholder,
  type PenTemplateIssue,
} from './penTemplate'
import type { ParamSyncEntry } from '../../api/contract'
import RestoreChip from './RestoreChip'
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
  /**
   * 云端更新那一笔（与 `ParamRow` 同一份数据）。
   *
   * 这一块走的是 `renderBlock`，**不经过 `ParamRow`** —— 所以它的「云端有新值」
   * 从来没人渲染过（作者 2026-11-11：「别的都有按钮，就它没有」）。
   */
  sync?: ParamSyncEntry
  syncBusy?: boolean
  onAdoptSync?: () => void
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
  sync,
  syncBusy = false,
  onAdoptSync,
  onEdit,
  onRevertToSaved,
}: Props) {
  const tr = useT()
  const [mode, setMode] = useState('view')
  /** 云端更新那块：展开后左右并排看「现在这一份 / 云端那一份」 */
  const [showDiff, setShowDiff] = useState(false)
  const cloudValue = sync?.pending === true ? sync.cloudNew : null
  const hasCloud = typeof cloudValue === 'string' && cloudValue !== ''
  /** 已经在用云端那一份了（骨架逐字相等）⇒ 按钮翻成「用回旧值」 */
  const adopted = hasCloud && value === cloudValue

  /* 落笔/收笔的骨架带占位符：结构错了就当场标红，别等到保存 */
  const kind = penKindOf(def.key)
  const issue = kind === null ? null : validate(value)

  /*
   * 骨架 + 兄弟字段 → **真值**。取不到值（字段没填 / 结构坏了）就返回 null，
   * 界面退回骨架原文 —— 宁可显示 `{wait}`，也不编一个数字出来。
   */
  const renderPen = useCallback(
    (text: string): string | null => {
      if (kind === null || valueOf === undefined) return null
      if (validate(text) !== null) return null
      const need = placeholders(text)
      if (need.length === 0) return text
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
      return render(text, vars)
    },
    [kind, valueOf],
  )

  const rendered = useMemo(() => renderPen(value), [renderPen, value])
  /** 当前这一份要显示的文本：能渲染就用真值，取不到才退回骨架原文 */
  const shown = rendered ?? value
  /* 云端那一份也要按**同一套字段**渲染成真值 —— 否则左边是 `X256`、右边是 `X{wait}`，
   * 两栏根本没法比（作者 2026-11-11：「到底是左边新还是右边新」）。 */
  const cloudShown = useMemo(
    () => (hasCloud ? (renderPen(cloudValue as string) ?? (cloudValue as string)) : ''),
    [renderPen, hasCloud, cloudValue],
  )

  /*
   * 左栏永远是**我原本那一份**，不随「选了哪边」变。
   *
   * 之前拿 `shown` 当左栏：点了云端之后 `shown` 就成云端了，左边跟着变，
   * 看起来像「我原来的东西被改掉了」。对比的意义是「两份摆在这儿让我挑」，
   * 不是「选了之后把另一份覆盖掉」。
   */
  const mineText = useMemo(() => {
    if (!hasCloud) return shown
    const raw = sync?.local
    const base = typeof raw === 'string' ? raw : savedValue
    return renderPen(base) ?? base
  }, [hasCloud, sync, savedValue, renderPen, shown])

  /* 逐行对齐：左边「我原来的」、右边「云端」。两栏行数相同，才能并排看 */
  const diffRows = useMemo(
    () => (hasCloud ? alignDiff(mineText, cloudShown) : []),
    [hasCloud, mineText, cloudShown],
  )

  /*
   * 代码模式的三层：行号槽 + 着色层 + 透明输入框（A43 移植，工作台 C15 那套）。
   * 行号槽自己不滚，跟着输入框的 scrollTop 走 —— 这一支 ref 就是那条链。
   */
  const taRef = useRef<HTMLTextAreaElement | null>(null)
  const numRef = useRef<HTMLDivElement | null>(null)

  /*
   * 这一屏**看到的是高亮层**（着色 `<pre>`），底下那个 textarea 是透明的、只接输入。
   * 所以真值必须同时喂给这三处，只换 textarea 的 value 是看不见的
   * （2026-11-11 作者截图：代码页仍显示 `X{wait}`，正是这一层还读着骨架）。
   */
  /*
   * 代码页是**编辑骨架**的地方，所以这一层用 `value`（含 `{wait}`），不是真值。
   *
   * 之前喂的是 `shown`（真值）：光标落在视觉位置上却写到骨架里，一输入就
   * 「字被换掉、光标跳走」—— 编辑体验直接坏掉。真值改到对比里看。
   */
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
            <RestoreChip
              title={tr('params.gcode.revertTitle', { lines: savedValue.split('\n').length })}
              onClick={() => onRevertToSaved(def.key)}
            >
              {tr('params.gcode.revertLabel')}
            </RestoreChip>
          )}
          <SegmentedField
            variant="segmented"
            label={tr('params.gcode.displayMode', { label: def.label })}
            options={MODES.map((m) => ({ value: m.value, label: tr(m.label) }))}
            value={mode}
            onChange={(next) => {
              setMode(next)
              /* 对比只活在代码视图里；切到视图就把对比收起来，否则按钮会停在「收起对比」 */
              if (next !== 'code') setShowDiff(false)
            }}
          />
        </span>
      </header>

      <p className={s.desc}>{def.desc}</p>

      {issue !== null && (
        <p className={s.structureError} role="alert">
          {tr('params.pen.structureError', { detail: issueText(issue) })}
        </p>
      )}

      {/* 云端更新：这一块不经过 `ParamRow`，所以这条 bar 得自己摆 */}
      {hasCloud && (
        <div className={s.syncBar}>
          <span className={s.syncText}>
            {tr('params.gcode.cloudDiff', {
              n: diffRows.filter((row) => row.kind !== 'same').length,
            })}
          </span>
          <button
            type="button"
            className={s.syncToggle}
            onClick={() => {
              /* 对比就是代码视图的另一面：把左边那块编辑器换成两栏，
               * 不再往下面接第二份代码（作者：「不要下面那个代码再多占一块」）。 */
              if (showDiff) {
                setShowDiff(false)
              } else {
                setMode('code')
                setShowDiff(true)
              }
            }}
          >
            {tr(showDiff ? 'params.gcode.hideDiff' : 'params.gcode.showDiff')}
          </button>
        </div>
      )}

      {mode === 'code' && hasCloud && showDiff ? (
        /*
         * 对比 = 代码视图的另一种摆法：左右两栏逐行对齐，点哪一栏就用哪一份。
         *
         * 两栏都是 `<button>`：选中态靠边框 + 右上角勾表达，不靠“点完左边变样”。
         * 行高锁死，一行一格：真删 / 真加整行铺底，改过的行两栏并排、**只有变了的
         * 那几个字铺底**（作者：「其实只是部分不同」）。
         */
        <div className={s.diff}>
          <button
            type="button"
            className={`${s.diffPane} ${adopted ? '' : s.diffPicked}`}
            disabled={syncBusy}
            onClick={() => {
              if (adopted) {
                const back = sync?.local
                onEdit(def.key, typeof back === 'string' ? back : savedValue)
              }
            }}
          >
            <span className={s.diffHead}>
              {tr('params.gcode.diffMine')}
              {!adopted && <i className={s.diffTick}>✓</i>}
            </span>
            <span className={s.diffBody}>
              {diffRows.map((row, i) => (
                <span
                  key={`l${i}`}
                  className={`${s.diffLine} ${row.kind === 'removed' ? s.diffGone : ''} ${
                    row.left === null ? s.diffBlank : ''
                  }`}
                >
                  {row.leftSegs === undefined
                    ? (row.left ?? '')
                    : row.leftSegs.map((seg, k) => (
                        <span key={k} className={seg.hit ? s.diffHitGone : undefined}>
                          {seg.text}
                        </span>
                      ))}
                </span>
              ))}
            </span>
          </button>

          <button
            type="button"
            className={`${s.diffPane} ${adopted ? s.diffPicked : ''}`}
            disabled={syncBusy || onAdoptSync === undefined}
            onClick={() => {
              if (!adopted) onAdoptSync?.()
            }}
          >
            <span className={s.diffHead}>
              {tr('params.gcode.diffCloud')}
              {adopted && <i className={s.diffTick}>✓</i>}
            </span>
            <span className={s.diffBody}>
              {diffRows.map((row, i) => (
                <span
                  key={`r${i}`}
                  className={`${s.diffLine} ${row.kind === 'added' ? s.diffNew : ''} ${
                    row.right === null ? s.diffBlank : ''
                  }`}
                >
                  {row.rightSegs === undefined
                    ? (row.right ?? '')
                    : row.rightSegs.map((seg, k) => (
                        <span key={k} className={seg.hit ? s.diffHitNew : undefined}>
                          {seg.text}
                        </span>
                      ))}
                </span>
              ))}
            </span>
          </button>
        </div>
      ) : mode === 'code' ? (
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
              /*
               * 可编辑：这里就是「直接改这一段」的入口（作者 2026-11-11：
               * 没对比的时候我连值都改不了）。代价是编辑会把骨架里的占位符
               * 换成具体数字，之后那几格字段就不再驱动它了 —— 想保持联动就回字段里改。
               */
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
