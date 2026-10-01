/*
 * G-code 分层（纯函数，不碰 DOM、不进 React 状态）（C14 §七移植）。
 *
 * 那条红波浪线是**浏览器的拼写检查**（`<textarea>` 默认 `spellcheck`）——它把
 * `G92`、`L801`、`M204`、甚至中文注释都当成拼错的词。关掉它只需要一个属性
 * （见 `GcodeEditor`），但「按 G-code 的样子显示颜色」得自己认词：
 * 这一份就是那个认词器。
 *
 * # 认什么（就四类，多一类都是自作主张）
 *
 *   cmd    行首第一个词：`G92` / `G1` / `M204` / `L801` / `T0` —— **不限于 G/M/T**：
 *          真数据里就有 `L801`（上游自己的固件指令），按「G/M/T 开头」判会漏掉它。
 *          行号 `N123` 不算指令（那个才是「第一个词」）。
 *   arg    其余的词：`X256` / `F42000` / `E-5` / `S[AUTO]` 里的 S
 *   var    `[AUTO]` 这类方括号变量（注册表那一行的说明里点名支持）
 *   cmt    注释：`;` 到行尾（真数据里是中文，比如 `L803;此处使喷嘴降温`）、`( … )`
 *
 * 不认「指令字面量」的语法：这里不解析 G-code，只给它上色。
 */

export type GTokenKind = 'cmd' | 'arg' | 'var' | 'cmt' | 'space'

export interface GToken {
  /** 原文（空白也算一个 token，原样写回去） */
  t: string
  k: GTokenKind
}

/** 一个「词」：字母 + 可选符号 + 可选数字（`E-5` / `X256.4` / `F`） */
const WORD = /[A-Za-z][+-]?(?:\d+(?:\.\d*)?|\.\d+)?/
/** 方括号变量 */
const VAR = /\[[^\]\n]*\]/
const WS = /^[ \t]+/

export function tokenizeGcodeLine(line: string): GToken[] {
  const out: GToken[] = []
  let i = 0
  /** 这一行还没出现过「指令」—— 行首第一个词才是它 */
  let head = true

  while (i < line.length) {
    const c = line[i] as string
    const rest = line.slice(i)

    if (c === ' ' || c === '\t') {
      const m = WS.exec(rest) as RegExpExecArray
      out.push({ t: m[0], k: 'space' })
      i += m[0].length
      continue
    }

    if (c === ';') {
      out.push({ t: rest, k: 'cmt' })
      break
    }
    if (c === '(') {
      /* 括号注释：认到配对的 `)`；没有闭合就把这一行剩下的都算注释 */
      const close = rest.indexOf(')')
      const t = close < 0 ? rest : rest.slice(0, close + 1)
      out.push({ t, k: 'cmt' })
      i += t.length
      continue
    }

    const v = VAR.exec(rest)
    if (v !== null && v.index === 0) {
      out.push({ t: v[0], k: 'var' })
      i += v[0].length
      continue
    }

    const w = WORD.exec(rest)
    if (w !== null && w.index === 0) {
      /* 行号 `N123` 只是标记，不是指令 —— 它不占「第一个词」这个位 */
      const isNo = /^N\d+$/.test(w[0])
      out.push({ t: w[0], k: head && !isNo ? 'cmd' : 'arg' })
      if (!isNo) head = false
      i += w[0].length
      continue
    }

    /* 认不出的单字（`%`、逗号、怪符号）原样进 arg，颜色默认 —— 不猜 */
    out.push({ t: c, k: 'arg' })
    i += 1
  }

  return out
}

/** 一段 G-code → 每行的 token */
export function tokenizeGcode(code: string): GToken[][] {
  return code.split('\n').map(tokenizeGcodeLine)
}
