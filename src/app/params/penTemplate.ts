/*
 * 落笔 / 收笔模板骨架的前端镜像 —— 规则与内核
 * `crates/postprocess/src/postproc/pen_template.rs` 逐条对应。
 *
 * 为什么要有第二份：这段骨架在界面里被「视图」模式直接摊开给用户改，
 * 而最终拼装发生在内核（`presets/build.rs` 切行之前）。两边各写一套就一定漂移，
 * 所以这里只做**展示与即时校验**，判据靠同一批 fixture 与 Rust 对拍。
 *
 * 铁律：**骨架里没出现的占位符不替换、也不补默认值**。A 系列的骨架只动 X
 * （没有 `{wait_y}`），若在这里顺手补个 0 就会渲染出 `Y0` —— 那是绝对坐标，
 * 工具头会直接被甩到 Y 零点。
 */

export const PEN_PLACEHOLDERS = ['wait', 'wait_y', 'target', 'speed'] as const

export type PenPlaceholder = (typeof PEN_PLACEHOLDERS)[number]

/** 落笔还是收笔 —— 决定占位符对应的是哪一组字段。 */
export type PenKind = 'mount' | 'unmount'

export type PenTemplateIssue =
  | { kind: 'unknown'; name: string }
  | { kind: 'unclosed'; at: number }
  | { kind: 'malformed'; line: number; text: string }

const FIELD_OF: Record<PenKind, Record<PenPlaceholder, string>> = {
  mount: {
    wait: 'toolhead.pen_mount_wait_x',
    wait_y: 'toolhead.pen_mount_wait_y',
    target: 'toolhead.pen_mount_x',
    speed: 'toolhead.pen_mount_speed',
  },
  unmount: {
    wait: 'toolhead.pen_unmount_wait_x',
    wait_y: 'toolhead.pen_unmount_wait_y',
    target: 'toolhead.pen_unmount_x',
    speed: 'toolhead.pen_unmount_speed',
  },
}

/** 这个占位符的值住在哪条参数里（界面上那几格数字框）。 */
export function fieldOf(kind: PenKind, name: PenPlaceholder): string {
  return FIELD_OF[kind][name]
}

/** G-code 习惯：整数不带 `.0`（JS 的 `String(256)` 本来就是 `"256"`）。 */
export function fmtNum(v: number): string {
  return String(v)
}

/** 结构校验：占位符白名单 + 每行首 token。返回 `null` 表示合法。 */
export function validate(skeleton: string): PenTemplateIssue | null {
  let rest = skeleton
  while (true) {
    const open = rest.indexOf('{')
    if (open < 0) break
    const after = rest.slice(open + 1)
    const close = after.indexOf('}')
    if (close < 0) return { kind: 'unclosed', at: open }
    const name = after.slice(0, close)
    if (!(PEN_PLACEHOLDERS as readonly string[]).includes(name)) {
      return { kind: 'unknown', name }
    }
    rest = after.slice(close + 1)
  }
  const rows = skeleton.split('\n')
  for (let i = 0; i < rows.length; i += 1) {
    const t = rows[i].trim()
    if (t === '' || t.startsWith(';')) continue
    const head = t[0]
    if (head !== 'G' && head !== 'M' && head !== 'L') {
      return { kind: 'malformed', line: i + 1, text: t }
    }
  }
  return null
}

/** 骨架里**实际出现**的占位符（去重、按首次出现）。 */
export function placeholders(skeleton: string): PenPlaceholder[] {
  const seen: PenPlaceholder[] = []
  const re = /\{([^}]*)\}/g
  let m: RegExpExecArray | null = re.exec(skeleton)
  while (m !== null) {
    const name = m[1]
    if ((PEN_PLACEHOLDERS as readonly string[]).includes(name) && !seen.includes(name as PenPlaceholder)) {
      seen.push(name as PenPlaceholder)
    }
    m = re.exec(skeleton)
  }
  return seen
}

/**
 * 骨架 + 字段 → 最终 G-code。结构不合法直接抛，不产出半截代码。
 */
export function render(skeleton: string, vars: Record<PenPlaceholder, number>): string {
  const issue = validate(skeleton)
  if (issue !== null) {
    throw new Error(`invalid pen template: ${issue.kind}`)
  }
  return skeleton.replace(/\{([^}]*)\}/g, (_, name: string) =>
    fmtNum(vars[name as PenPlaceholder]),
  )
}

/** 行内差异的一段：`hit` 的才是真变了的那几个字 */
export interface DiffSeg {
  text: string
  hit: boolean
}

/** 逐行对比里的一行：左右各一格，用来画成对齐的两栏。 */
export interface DiffRow {
  left: string | null
  right: string | null
  /**
   * `same` 两边一样；`changed` 两边都有但内容不同（并排在同一格）；
   * `removed` / `added` 只有一侧有
   */
  kind: 'same' | 'changed' | 'removed' | 'added'
  /** 只有 `changed` 行有：两侧各切成「没变 / 变了」的段，只有 `hit` 的上色 */
  leftSegs?: DiffSeg[]
  rightSegs?: DiffSeg[]
}

/** 末尾换行会多出一个空串：它不是「多了一行空行」，两边都不算 */
function lines(text: string): string[] {
  const out = text.split('\n')
  if (out.length > 0 && out[out.length - 1] === '') out.pop()
  return out
}

/**
 * 一行之内：掐掉共同的头与尾，**只有中间那段算变了**。
 *
 * `G1 X1 F300;推到X8,笔尖下落` → `G1 X1 F300;推到底`：整行涂红涂绿等于说
 * 「这一行全换了」，而真正动的只有分号后面那几个字（作者：「其实只是部分不同」）。
 * 某一侧掐完是空串就不给这一段（纯删字 / 纯加字时对面本来就没有可标的）。
 */
export function inlineSegs(left: string, right: string): { left: DiffSeg[]; right: DiffSeg[] } {
  const n = Math.min(left.length, right.length)
  let head = 0
  while (head < n && left[head] === right[head]) head += 1
  let tail = 0
  while (tail < n - head && left[left.length - 1 - tail] === right[right.length - 1 - tail]) tail += 1

  const of = (text: string): DiffSeg[] => {
    const hit = text.slice(head, text.length - tail)
    const pre = text.slice(0, head)
    const post = text.slice(text.length - tail)
    const segs: DiffSeg[] = []
    if (pre !== '') segs.push({ text: pre, hit: false })
    if (hit !== '') segs.push({ text: hit, hit: true })
    if (post !== '') segs.push({ text: post, hit: false })
    return segs
  }
  return { left: of(left), right: of(right) }
}

/**
 * 相邻的一串「删」与紧随的一串「增」**按顺序配成 `changed`**，并排占同一行格。
 *
 * 不配的话，改一行会拆成「左边红一格 + 右边绿一格」两条，两栏各错一格：
 * 本来是同一行改了半句，看着像整块搬了家（作者 2026-11-13 截图）。
 */
function pair(rows: DiffRow[]): DiffRow[] {
  const out: DiffRow[] = []
  let k = 0
  while (k < rows.length) {
    if (rows[k].kind !== 'removed') {
      out.push(rows[k])
      k += 1
      continue
    }
    const gone: string[] = []
    while (k < rows.length && rows[k].kind === 'removed') {
      gone.push(rows[k].left ?? '')
      k += 1
    }
    const born: string[] = []
    while (k < rows.length && rows[k].kind === 'added') {
      born.push(rows[k].right ?? '')
      k += 1
    }
    const both = Math.min(gone.length, born.length)
    for (let i = 0; i < both; i += 1) {
      const segs = inlineSegs(gone[i], born[i])
      out.push({
        left: gone[i],
        right: born[i],
        kind: 'changed',
        leftSegs: segs.left,
        rightSegs: segs.right,
      })
    }
    for (let i = both; i < gone.length; i += 1) {
      out.push({ left: gone[i], right: null, kind: 'removed' })
    }
    for (let i = both; i < born.length; i += 1) {
      out.push({ left: null, right: born[i], kind: 'added' })
    }
  }
  return out
}

/**
 * 按 LCS 把两段文本对齐成行序列。
 *
 * 十行左右的 G-code，O(n·m) 的 DP 是微秒级 —— 不用为它设计什么增量算法。
 * 对齐之后左右两栏才能逐行并排：删掉的行只在左边、新增的行只在右边，
 * 这样「哪一行变了」一眼能看出来，而不是靠行号硬猜。
 */
export function alignDiff(leftText: string, rightText: string): DiffRow[] {
  const left = lines(leftText)
  const right = lines(rightText)
  const n = left.length
  const m = right.length
  /* 长度表：`len[i][j]` = left[i..] 与 right[j..] 的 LCS 长度 */
  const len: number[][] = Array.from({ length: n + 1 }, () => new Array<number>(m + 1).fill(0))
  for (let i = n - 1; i >= 0; i -= 1) {
    for (let j = m - 1; j >= 0; j -= 1) {
      len[i][j] =
        left[i] === right[j] ? len[i + 1][j + 1] + 1 : Math.max(len[i + 1][j], len[i][j + 1])
    }
  }

  const rows: DiffRow[] = []
  let i = 0
  let j = 0
  while (i < n && j < m) {
    if (left[i] === right[j]) {
      rows.push({ left: left[i], right: right[j], kind: 'same' })
      i += 1
      j += 1
    } else if (len[i + 1][j] >= len[i][j + 1]) {
      rows.push({ left: left[i], right: null, kind: 'removed' })
      i += 1
    } else {
      rows.push({ left: null, right: right[j], kind: 'added' })
      j += 1
    }
  }
  while (i < n) {
    rows.push({ left: left[i], right: null, kind: 'removed' })
    i += 1
  }
  while (j < m) {
    rows.push({ left: null, right: right[j], kind: 'added' })
    j += 1
  }
  return pair(rows)
}

/** 判断某个参数 token 是不是占位符，是就返回名字。 */
export function placeholderName(token: string): PenPlaceholder | null {
  if (!token.startsWith('{') || !token.endsWith('}')) return null
  const name = token.slice(1, -1)
  return (PEN_PLACEHOLDERS as readonly string[]).includes(name) ? (name as PenPlaceholder) : null
}
