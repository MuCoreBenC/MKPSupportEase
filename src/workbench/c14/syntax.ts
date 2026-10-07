/*
 * 极轻的 JSON / TOML 上色（2026-10-07）。
 *
 * # 为什么自己写，不引 Prism / highlight.js
 *
 * 这个仓库守「不引入新依赖」（diff 都是自己写的，`build.rs` 的注释里写着理由）。
 * 而这里要上色的只有两种文法 —— 交付目录里的 JSON 与产物 TOML。为两段扫描器
 * 拖进一个几百 KB 的高亮器不划算（作者要的是「不像 VSCode 插件那样黑压压一片」，
 * 不是完整的语法树）。
 *
 * # 一条不变式
 *
 * **每一行 token 的 `text` 拼起来必须等于原行**：上色只切分，不删、不补、不改一个字。
 * 这条比"认得全"重要得多 —— 高亮把内容吃掉一个引号，人看到的就不是盘上那份文件了。
 *
 * # 认不出就不上色
 *
 * [`langOf`] 只认 `.json` / `.toml`；其余（比如以后多出来的 `.txt`）整行原样。
 */

export type TokenKind =
  | 'plain' // 没上色的字面量（空格、认不出的词…）
  | 'key' // JSON 的键 / TOML 的键名
  | 'string' // 字符串（含引号，含多行块的每一行）
  | 'number'
  | 'keyword' // true / false / null
  | 'comment' // TOML 的 `#` 注释
  | 'section' // TOML 的 `[section]`
  | 'punct' // 括号、逗号、等号、冒号

export interface Token {
  text: string
  kind: TokenKind
}

/** 按文件名认文法。认不出 = `null`（界面据此整行原样渲染） */
export function langOf(fileName: string): 'json' | 'toml' | null {
  const lower = fileName.toLowerCase()
  if (lower.endsWith('.json')) return 'json'
  if (lower.endsWith('.toml')) return 'toml'
  return null
}

/**
 * 一批行 → 每行的 token。**按顺序走**：TOML 的 `"""` 多行块要跨行认
 * （块里的 G-code 不能当注释上色），所以它不是逐行独立能算的。
 */
export function highlightLines(lines: string[], lang: 'json' | 'toml'): Token[][] {
  if (lang === 'json') return lines.map(highlightJson)
  const out: Token[][] = []
  let inBlock = false
  for (const line of lines) {
    const [tokens, next] = highlightToml(line, inBlock)
    out.push(tokens)
    inBlock = next
  }
  return out
}

/** JSON 一行（`"\""` 里可以有转义引号；行内不会有跨行字符串） */
function highlightJson(line: string): Token[] {
  const out: Token[] = []
  let plain = ''
  let i = 0
  const flush = () => {
    if (plain !== '') {
      out.push({ text: plain, kind: 'plain' })
      plain = ''
    }
  }

  while (i < line.length) {
    const ch = line[i]

    if (ch === '"') {
      const start = i
      i += 1
      while (i < line.length) {
        if (line[i] === '\\') {
          i += 2
          continue
        }
        if (line[i] === '"') {
          i += 1
          break
        }
        i += 1
      }
      const text = line.slice(start, i)
      flush()
      out.push({ text, kind: nextNonSpace(line, i) === ':' ? 'key' : 'string' })
      continue
    }

    if (ch === '-' || (ch >= '0' && ch <= '9')) {
      const start = i
      i += 1
      while (i < line.length && isNumberChar(line[i])) i += 1
      flush()
      out.push({ text: line.slice(start, i), kind: 'number' })
      continue
    }

    if (isLetter(ch)) {
      const start = i
      while (i < line.length && isLetter(line[i])) i += 1
      const word = line.slice(start, i)
      flush()
      out.push({ text: word, kind: isKeyword(word) ? 'keyword' : 'plain' })
      continue
    }

    if (ch === '{' || ch === '}' || ch === '[' || ch === ']' || ch === ',' || ch === ':') {
      flush()
      out.push({ text: ch, kind: 'punct' })
      i += 1
      continue
    }

    plain += ch
    i += 1
  }

  flush()
  return out
}

/**
 * TOML 一行。`inBlock` = 上一行把 `"""` 开着（多行 G-code），返回下一行的状态。
 *
 * 块内整行都算字符串 —— G-code 里有 `;`、有数字，当普通文本处理才对；
 * 唯一要认的是收尾的 `"""`（它独占一行）。
 */
function highlightToml(line: string, inBlock: boolean): [Token[], boolean] {
  const out: Token[] = []
  let plain = ''
  let i = 0
  const flush = () => {
    if (plain !== '') {
      out.push({ text: plain, kind: 'plain' })
      plain = ''
    }
  }

  if (inBlock) {
    const close = line.indexOf('"""')
    if (close < 0) return [[{ text: line, kind: 'string' }], true]
    if (close > 0) out.push({ text: line.slice(0, close), kind: 'string' })
    out.push({ text: '"""', kind: 'string' })
    i = close + 3
    inBlock = false
  }

  while (i < line.length) {
    const ch = line[i]

    // `#` 只要不在字符串里就是注释 —— 注释吃到行尾
    if (ch === '#') {
      flush()
      out.push({ text: line.slice(i), kind: 'comment' })
      i = line.length
      continue
    }

    // `[section]`：只认行首那一个（数组字面量的 `[` 不在此列）
    if (ch === '[' && line.slice(0, i).trim() === '') {
      const end = line.indexOf(']', i)
      if (end > 0) {
        flush()
        out.push({ text: line.slice(i, end + 1), kind: 'section' })
        i = end + 1
        continue
      }
    }

    if (ch === '"' || ch === "'") {
      if (line.startsWith('"""', i)) {
        flush()
        const close = line.indexOf('"""', i + 3)
        if (close < 0) {
          out.push({ text: '"""', kind: 'string' })
          i += 3
          inBlock = true
        } else {
          out.push({ text: line.slice(i, close + 3), kind: 'string' })
          i = close + 3
        }
        continue
      }
      const quote = ch
      let j = i + 1
      while (j < line.length) {
        if (quote === '"' && line[j] === '\\') {
          j += 2
          continue
        }
        if (line[j] === quote) {
          j += 1
          break
        }
        j += 1
      }
      flush()
      const text = line.slice(i, j)
      out.push({ text, kind: nextNonSpace(line, j) === '=' ? 'key' : 'string' })
      i = j
      continue
    }

    if (isLetter(ch) || (ch >= '0' && ch <= '9') || ch === '-' || ch === '_') {
      const start = i
      i += 1
      while (i < line.length && isBareChar(line[i])) i += 1
      const word = line.slice(start, i)
      flush()
      if (nextNonSpace(line, i) === '=') out.push({ text: word, kind: 'key' })
      else if (word === 'true' || word === 'false') out.push({ text: word, kind: 'keyword' })
      else if (/^-?\d/.test(word)) out.push({ text: word, kind: 'number' })
      else out.push({ text: word, kind: 'plain' })
      continue
    }

    if (ch === '=' || ch === ',' || ch === '{' || ch === '}') {
      flush()
      out.push({ text: ch, kind: 'punct' })
      i += 1
      continue
    }

    plain += ch
    i += 1
  }

  flush()
  return [out, inBlock]
}

/** `at` 之后第一个非空格字符（没有 = `''`） */
function nextNonSpace(line: string, at: number): string {
  let i = at
  while (i < line.length && (line[i] === ' ' || line[i] === '\t')) i += 1
  return i < line.length ? line[i] : ''
}

function isLetter(ch: string): boolean {
  return (ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z')
}

function isNumberChar(ch: string): boolean {
  return (ch >= '0' && ch <= '9') || ch === '.' || ch === '-' || ch === '+' || ch === 'e' || ch === 'E'
}

/** TOML 的裸键 / 值可以带 `.` `-` `_`（`offset_x`、`a.b`、`0.2mm`） */
function isBareChar(ch: string): boolean {
  return isLetter(ch) || (ch >= '0' && ch <= '9') || ch === '_' || ch === '-' || ch === '.'
}

function isKeyword(word: string): boolean {
  return word === 'true' || word === 'false' || word === 'null'
}
