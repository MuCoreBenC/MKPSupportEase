/*
 * 只读的代码/正文块：JSON / TOML 上色（认不出文法就整段原样）。
 *
 * # 为什么是 `<pre>` 不是 `<textarea>`
 *
 * 交付文件查看原来是一口只读 textarea —— 里面**放不了 span**，字只能一种颜色
 * （作者 2026-10-07：「不要都是黑色的字，变一下颜色，像 VSCode 里面的插件一样」）。
 * 换成 `<pre>` + 逐行 div 之后，上色、折行、行号都归我们自己管。
 *
 * # 上色在哪
 *
 * 切 token 的扫描器在 [`./syntax`]（纯函数，认不出就返回 null）；这里只负责把 token
 * 拼成节点。字色类名（`tok_*`）住在 `c14.module.css`，**生成前确认的行级 diff 用的是
 * 同一份**：它自己拼行（行号槽、红绿底、`+`/`−` 符号是它的事），但那一行的字色
 * 走这里导出的 [`CodeLine`] —— 两处各写一份上色，迟早会长成两种颜色。
 */

import { useMemo } from 'react'
import type { ReactNode } from 'react'
import { highlightLines, langOf } from './syntax'
import type { Token } from './syntax'
import c from '../c14.module.css'

interface Props {
  /** 决定用哪套文法上色（只按扩展名认，不读盘）。认不出 ⇒ 整段原样 */
  fileName: string
  text: string
  /** 外面那口框的类名（高度 / 边框 / 字体那些）—— 上色只管字色 */
  className?: string
}

export default function CodeText({ fileName, text, className }: Props) {
  /* 末尾那一个换行不算"最后一行"—— 不然每份文件尾巴上都多一条空行 */
  const lines = useMemo(() => text.replace(/\n$/, '').split('\n'), [text])
  const tokens = useMemo(() => {
    const lang = langOf(fileName)
    return lang === null ? null : highlightLines(lines, lang)
  }, [fileName, lines])

  return (
    <pre className={className}>
      {lines.map((line, i) => (
        <div key={i} className={c.codeLine}>
          <CodeLine line={line} tokens={tokens?.[i]} />
        </div>
      ))}
    </pre>
  )
}

/**
 * 一行正文 → 节点（**生成前确认的行级 diff 也用它**：那边自己拼行 —— 行号槽、
 * 红绿底、`+`/`−` 符号是它的事，字色归这里）。
 */
export function CodeLine({ line, tokens }: { line: string; tokens: Token[] | undefined }) {
  return <>{lineNodes(line, tokens)}</>
}

/**
 * 一行正文 → 节点。**整行没有可上的色就原样返回字符串**（少一层 span）；
 * 空行给一个不断行的空格，否则那一行会塌掉、行高也跟着变。
 */
function lineNodes(line: string, tokens: Token[] | undefined): ReactNode {
  if (line === '') return '\u00a0'
  if (tokens === undefined || tokens.length === 0) return line
  if (tokens.length === 1 && tokens[0].kind === 'plain') return line
  return tokens.map((t, i) =>
    t.kind === 'plain' ? (
      t.text
    ) : (
      <span key={i} className={c[`tok_${t.kind}`]}>
        {t.text}
      </span>
    ),
  )
}
