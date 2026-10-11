/*
 * TOML 正文的**语法着色**（自己写，不引插件 —— 作者 2026-10-10：
 * 「我说的是这个里面的字啊……那个字要有颜色，五颜六色的那种标志啊，
 * 你不用插件可以自己写，因为它比较简单」）。
 *
 * 用户看得见的那份涂胶预设长这几样，够用、不贪多：
 *
 *   注释      `# …`                    → 灰斜体
 *   表头      `[table]` / `[[array]]`   → 紫
 *   键        `key =`                   → 蓝
 *   字符串    `"…"`                     → 绿
 *   数字      `20.0` / `-0.7`           → 橙
 *   布尔      `true` / `false`          → 紫
 *   括号逗号  `[ ] { } ,`               → 次级灰
 *
 * 产出的是**token（每行一组）**而不是 HTML 字符串：编辑框那层 `<pre>` 用
 * React 元素画，class 走 CSS Modules（不用 `:global`、不用
 * `dangerouslySetInnerHTML`）—— 正文来自用户自己的文件，一个字符都不当标签解释。
 *
 * 与参数页的 G-code 块（`params/gcode.ts`）同一个路数：词法在数据层、颜色在样式层。
 */

export type TomlKind = 'plain' | 'comment' | 'table' | 'key' | 'string' | 'number' | 'bool' | 'punct'

export interface TomlToken {
  text: string
  kind: TomlKind
}

/** 一行的 token 串；把整份正文切成「行 → token」两层，交给 <pre> 逐行画 */
export function tomlTokens(text: string): TomlToken[][] {
  return text.split('\n').map(lineTokens)
}

function lineTokens(line: string): TomlToken[] {
  const out: TomlToken[] = []
  const push = (text: string, kind: TomlKind) => {
    if (text !== '') out.push({ text, kind })
  }

  /* 前导空白原样留着（缩进也是正文的一部分） */
  const lead = /^\s*/.exec(line)?.[0] ?? ''
  push(lead, 'plain')
  let rest = line.slice(lead.length)

  /* 整行注释 */
  if (rest.startsWith('#')) {
    push(rest, 'comment')
    return out
  }

  /* 表头：[table] / [[array]] */
  if (rest.startsWith('[')) {
    const head = /^\[\[?[^\]]*\]?\]?/.exec(rest)?.[0] ?? ''
    push(head, 'table')
    rest = rest.slice(head.length)
    push(rest.startsWith('#') ? rest : '', 'comment')
    if (!rest.startsWith('#')) push(rest, 'plain')
    return out
  }

  /* 键 = 值 */
  const kv = /^([A-Za-z0-9_."-]+)(\s*=\s*)/.exec(rest)
  if (kv !== null) {
    push(kv[1], 'key')
    push(kv[2], 'plain')
    rest = rest.slice(kv[0].length)
  }

  /* 值那一截：字符串 / 数字 / 布尔 / 括号 / 行内注释，一路吃到行尾 */
  let guard = 0
  while (rest !== '' && guard < 2000) {
    guard += 1
    const space = /^\s+/.exec(rest)?.[0]
    if (space !== undefined) {
      push(space, 'plain')
      rest = rest.slice(space.length)
      continue
    }
    if (rest.startsWith('#')) {
      push(rest, 'comment')
      break
    }
    const str = /^"(?:[^"\\]|\\.)*"?/.exec(rest)?.[0]
    if (str !== undefined && str !== '') {
      push(str, 'string')
      rest = rest.slice(str.length)
      continue
    }
    const num = /^-?\d+(?:\.\d+)?(?:e[+-]?\d+)?/i.exec(rest)?.[0]
    if (num !== undefined && num !== '') {
      push(num, 'number')
      rest = rest.slice(num.length)
      continue
    }
    const bool = /^(?:true|false)/.exec(rest)?.[0]
    if (bool !== undefined) {
      push(bool, 'bool')
      rest = rest.slice(bool.length)
      continue
    }
    const punct = /^[[\]{},]+/.exec(rest)?.[0]
    if (punct !== undefined && punct !== '') {
      push(punct, 'punct')
      rest = rest.slice(punct.length)
      continue
    }
    /* 认不出的（日期、裸值……）：一个字符一个字符地过，绝不吞掉内容 */
    push(rest[0] ?? '', 'plain')
    rest = rest.slice(1)
  }
  return out
}
