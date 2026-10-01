/*
 * G-code 的框（上色 + 没有红波浪线）（C14 §七移植）。
 *
 * # 为什么不是 CodeMirror / Monaco
 *
 * 没有引：一个 G-code 编辑器（CodeMirror 6 那一套）会带进十几个包、一份自己的
 * 主题与键盘映射 —— 换来的能力里，这里真正要的只有**给词上色** 和
 * **别把那行字标成拼错的**。
 *
 * 所以这里是那套「透明 textarea 压在着色层上」的老办法（各家在线编辑器都在用）：
 *
 *   `.pGcePre`   同一段文字，按 token 上色的只读层（`aria-hidden`，不参与交互）
 *   `.pGceTa`    真正的输入框，字是**透明的**，只留光标与选区
 *
 * 三层必须**同字体、同字号、同行高、同内衬**，否则字会错位 —— 这四样写在
 * c14.module.css 里一处（`.pGcePre, .pGceTa, .pGceNum` 共用那一条），改一处
 * 三层一起动。
 *
 * # 三条跟着来的规矩
 *
 * 1. **不许换行**（`wrap="off"` + `white-space: pre`）：两层的折行算法不一样
 *    （textarea 的软换行不是 CSS 的 `pre-wrap`），一旦折行就会越往下越偏。
 *    G-code 本来就是一行一条指令，长了横向滚 —— 行号槽也对得上。
 * 2. **滚动要同步**：textarea 是那个滚动容器，`onScroll` 把 `scrollTop/scrollLeft`
 *    抄给着色层与行号槽（两层都是 `overflow: hidden`，抄进来就能滚）。
 * 3. **拼写检查关掉**：`spellCheck={false}` 是作者看到的那条红波浪线的开关；
 *    顺带把 Grammarly 那类插件的钩子（`data-gramm`）也按住。
 *
 * 行号槽只在模态框里给：参数行上那个框只有 172px 宽，两列一放就看不见代码了。
 */

import { useMemo, useRef } from 'react'

import { tokenizeGcode } from './gcode'
import type { GTokenKind } from './gcode'
import s from '../c14.module.css'

/** 四类 token 的类名。`arg` / `space` 不上色（跟着正文色走） */
const CLS: Record<GTokenKind, string | undefined> = {
  cmd: s.pGcCmd,
  arg: undefined,
  var: s.pGcVar,
  cmt: s.pGcCmt,
  space: undefined,
}

interface Props {
  value: string
  onChange: (next: string) => void
  disabled?: boolean
  /** 行号槽（模态框里给） */
  gutter?: boolean
  /** 尺码：行上 `inline`（矮、框里滚）、模态框 `modal`（吃满给它的那一块高） */
  size?: 'inline' | 'modal'
  /** 无障碍名 / 给读屏的一句话 */
  label: string
}

export default function GcodeEditor({
  value,
  onChange,
  disabled = false,
  gutter = false,
  size = 'inline',
  label,
}: Props) {
  const taRef = useRef<HTMLTextAreaElement>(null)
  const preRef = useRef<HTMLPreElement>(null)
  const numRef = useRef<HTMLPreElement>(null)

  const lines = useMemo(() => tokenizeGcode(value), [value])

  /** 着色层：每行一段，行与行之间补一个换行（`pre` 里的换行就是换行） */
  const colored = useMemo(
    () =>
      lines.map((tokens, i) => (
        <span key={i}>
          {i > 0 && '\n'}
          {tokens.map((tk, j) =>
            CLS[tk.k] === undefined ? (
              tk.t
            ) : (
              <span key={j} className={CLS[tk.k]}>
                {tk.t}
              </span>
            ),
          )}
        </span>
      )),
    [lines],
  )

  const nums = useMemo(() => lines.map((_, i) => String(i + 1)).join('\n'), [lines])

  /**
   * 失焦时把**结尾的空行**摘掉（`"…F42000\n"` → `"…F42000"`）。
   *
   * 尾部换行是编辑过程的临时态（回车落在行尾就会带出一个），存进草稿之后
   * 行数与行号槽都会多出一行「没有内容的行」—— 作者：「我明明没有第 10 行
   * 第 11 行，为什么你那个第 10 行第 11 行空出来了呢」。打字的过程中它照常
   * 显示（行号槽认物理行），离开输入框就算写完，摘掉。
   */
  const settle = () => {
    if (disabled) return
    const next = value.replace(/\n+$/, '')
    if (next !== value) onChange(next)
  }

  /** textarea 滚了，两层跟着走（见文件头第 2 条） */
  const sync = () => {
    const ta = taRef.current
    if (ta === null) return
    const pre = preRef.current
    if (pre !== null) {
      pre.scrollTop = ta.scrollTop
      pre.scrollLeft = ta.scrollLeft
    }
    const num = numRef.current
    if (num !== null) num.scrollTop = ta.scrollTop
  }

  return (
    <div className={s.pGce} data-size={size} data-off={disabled ? '' : undefined}>
      {gutter && (
        <pre ref={numRef} className={s.pGceNum} aria-hidden>
          {nums}
        </pre>
      )}
      <div className={s.pGceBox}>
        <pre ref={preRef} className={s.pGcePre} aria-hidden>
          {colored}
          {/*
            值以换行结尾（或整个为空）时，最后一行是空行 —— 补一个零宽字符
            给它撑出那一行的高度。原来这里无条件补 `'\n'`：块末尾的换行
            自己不生成新行，反而在值**没有**尾换行时多画一行空的
            （作者截图里行号槽之外多出来的那一行就是它）。
          */}
          {(value === '' || value.endsWith('\n')) && '\u200b'}
        </pre>
        <textarea
          ref={taRef}
          className={s.pGceTa}
          value={value}
          disabled={disabled}
          aria-label={label}
          /* 红波浪线就是它画的（见文件头第 3 条） */
          spellCheck={false}
          autoCorrect="off"
          autoCapitalize="off"
          autoComplete="off"
          data-gramm="false"
          data-gramm_editor="false"
          /* 不换行：两层的折行算法不一样（见文件头第 1 条） */
          wrap="off"
          onChange={(e) => onChange(e.target.value)}
          onBlur={settle}
          onScroll={sync}
        />
      </div>
    </div>
  )
}
