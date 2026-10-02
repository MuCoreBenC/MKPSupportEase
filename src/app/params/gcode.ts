/*
 * G-code 的高亮切词（A43 移植，来自工作台 C15 的那套）。
 *
 * 只干一件事：把一行切成 `{ text, cls }` 的片段，给着色层 `.codePre` 上色。
 * 为什么是一维数组而不是嵌套：着色层是**纯展示**，不做语法树 —— 三类各自成段就够：
 *   cmd  行首到第一个参数之间的那条指令（`G1` / `L803` / `M104` …）
 *   var  `[AUTO]` 这种方括号变量（不是字面量，是占位）
 *   cmt  分号起的注释（真数据里是中文：「此处使喷嘴降温」）
 * 其余是无 class 的普通片段（参数、数字、空白、逗号、引号 —— 用 A 的界面字色即可）。
 */

export const TOK_CLASS = {
  cmd: 'tokCmd',
  var: 'tokVar',
  cmt: 'tokCmt',
} as const

export type TokClass = (typeof TOK_CLASS)[keyof typeof TOK_CLASS]

export interface GcodeToken {
  text: string
  cls?: TokClass
}

/**
 * 一行 G-code → 高亮片段。
 *
 * 规则（照着工作台 C15 那套写，不动语义）：
 *   · `;` 之后整段是注释（含 `;` 本身），**先切** —— 注释里的字母不该被再认成指令
 *   · 注释之前：`[AUTO]` 之类的 `[...]` 变量成段（`fullMatch`）
 *   · 行首**第一条**指令成段（含前面的空白，`^(\s*)([A-Za-z]\d*)`）：
 *     只认第一条 —— G-code 一行一条主指令，后面的字母都是参数（`X`/`Y`/`F`）
 *   · 都不是的就是普通段
 *
 * 参数是从 `useParams` 分出来的：`line` 是原文，`cmtIdx` 是分号位置（没有就是 null）。
 * 之前的分法要传 `line/param/cmt` 三样、`param` 还要在 `line` 里 indexOf 回找 ——
 * 丢掉「往回找」这一步，顺带丢掉对 `param` 的依赖。
 */
export function tokenizeGcodeLine(line: string, cmtIdx: number | null): GcodeToken[] {
  const tokens: GcodeToken[] = []
  const codePart = cmtIdx === null ? line : line.slice(0, cmtIdx)

  /* 批注前置：先认定指令，再认定 `[...]`；顺序反了变量会被指令吞掉 */
  let rest = codePart
  /* 行首第一条指令（含前导空白）。只认第一条 */
  const cmdMatch = /^(\s*)([A-Za-z]\d*)/.exec(rest)
  if (cmdMatch !== null) {
    const [full] = cmdMatch
    tokens.push({ text: full, cls: TOK_CLASS.cmd })
    rest = rest.slice(full.length)
  }

  /* `[...]` 变量：先切变量段，再切普通段 */
  const VAR_RE = /\[[^\]]*\]/g
  let cursor = 0
  let m: RegExpExecArray | null
  while ((m = VAR_RE.exec(rest)) !== null) {
    if (m.index > cursor) tokens.push({ text: rest.slice(cursor, m.index) })
    tokens.push({ text: m[0], cls: TOK_CLASS.var })
    cursor = m.index + m[0].length
  }
  if (cursor < rest.length) tokens.push({ text: rest.slice(cursor) })

  /* 注释段（含分号本身） */
  if (cmtIdx !== null) tokens.push({ text: line.slice(cmtIdx), cls: TOK_CLASS.cmt })

  return tokens
}

/**
 * 一行 G-code → 上半行是「指令 + 参数」、下半行是注释（真数据里注释是中文说明）。
 * 没有注释时下半行是空串。
 */
export function splitGcodeLine(line: string): { code: string; comment: string } {
  const i = line.indexOf(';')
  if (i < 0) return { code: line, comment: '' }
  return { code: line.slice(0, i), comment: line.slice(i) }
}
