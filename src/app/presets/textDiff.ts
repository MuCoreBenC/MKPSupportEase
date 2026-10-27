/*
 * **行级文本对比**（2026-10-21，作者：「我希望像那个 IDE 那些软件一样，就这种红的绿的
 * 这种对比啊，如果位置足够就可以左右对比」）。
 *
 * 只干一件事：两份正文 → 一行一行的操作序列（相同 / 删 / 加 / 改）。
 * 不认字符级差异 —— 预设是 TOML，一行就是一个字段，行内再细分没人看。
 *
 * # 算法
 *
 * 经典 LCS（最长公共子序列）动态规划：预设几百行，`n × m` 的表格在这个量级上
 * 一两毫秒就完事，不需要 Myer's。**先算 LCS，再从尾往前回溯**出 same/del/add；
 * 相邻的 del + add 合成一条 `change`（左右并排显示）。
 *
 * # 为什么住在前端、不叫后端算
 *
 * 它比的是"最初那份 vs 现在输入的正文"，后者每敲一个字就变 —— 往返一次 IPC
 * 再回来，滚动位置和光标都会抖。纯函数在前端跑，React 重渲染天然就是"实时"。
 */

/** 一行在对比视图里的身份 */
export type DiffKind = 'same' | 'del' | 'add' | 'change'

export interface DiffRow {
  kind: DiffKind
  /** 左侧（最初那份）的行号，1 起；这一侧没有就是 `null` */
  leftNo: number | null
  /** 右侧（现在这份）的行号，1 起；这一侧没有就是 `null` */
  rightNo: number | null
  /** 左侧那行的原文（`null` = 这一侧没有对应行） */
  left: string | null
  /** 右侧那行的原文 */
  right: string | null
}

/** 行数上限：超过就只比前这么多行（预设不会有，但别让一个坏文件把界面拖死） */
const MAX_LINES = 4000

function splitLines(text: string): string[] {
  if (text === '') return []
  const lines = text.split('\n')
  /* 末尾换行会多出一个空串：它不是"一行空行"，剪掉 */
  if (lines.length > 0 && lines[lines.length - 1] === '') lines.pop()
  return lines.slice(0, MAX_LINES)
}

/**
 * 两份正文的行级差异。`left` = 基准（最初那份），`right` = 现在这份。
 *
 * 返回的行**按右侧（现在这份）的顺序**铺：`same` / `change` / `add` 各占一行；
 * `del`（基准有、现在没有）夹在它消失的位置。这样右边的行号是连续的，
 * 左边缺号的地方就是"这一段被删了"。
 */
export function lineDiff(leftText: string, rightText: string): DiffRow[] {
  const a = splitLines(leftText)
  const b = splitLines(rightText)
  const n = a.length
  const m = b.length

  /* LCS 长度表：`lcs[i][j]` = a[i..] 与 b[j..] 的最长公共子序列长度 */
  const lcs: Uint32Array[] = Array.from({ length: n + 1 }, () => new Uint32Array(m + 1))
  for (let i = n - 1; i >= 0; i -= 1) {
    for (let j = m - 1; j >= 0; j -= 1) {
      lcs[i][j] = a[i] === b[j] ? lcs[i + 1][j + 1] + 1 : Math.max(lcs[i + 1][j], lcs[i][j + 1])
    }
  }

  /* 回溯：先收成 same / del / add 的粗序列 */
  type Raw = { kind: 'same' | 'del' | 'add'; leftNo: number | null; rightNo: number | null }
  const raw: Raw[] = []
  let i = 0
  let j = 0
  while (i < n && j < m) {
    if (a[i] === b[j]) {
      raw.push({ kind: 'same', leftNo: i + 1, rightNo: j + 1 })
      i += 1
      j += 1
    } else if (lcs[i + 1][j] >= lcs[i][j + 1]) {
      raw.push({ kind: 'del', leftNo: i + 1, rightNo: null })
      i += 1
    } else {
      raw.push({ kind: 'add', leftNo: null, rightNo: j + 1 })
      j += 1
    }
  }
  while (i < n) {
    raw.push({ kind: 'del', leftNo: i + 1, rightNo: null })
    i += 1
  }
  while (j < m) {
    raw.push({ kind: 'add', leftNo: null, rightNo: j + 1 })
    j += 1
  }

  /*
   * **连续的删块 + 紧随的增块按顺序一一配对**成 `change`（2026-10-21 修：原来只配紧邻的
   * 一对，连着改两行时会剩下"两删两增"四条，统一视图看起来像把行叠在一起了）。
   *
   * 配不上的那几个照旧单独成行 —— del 只有左边、add 只有右边。
   */
  const row = (r: (typeof raw)[number]): DiffRow => ({
    kind: r.kind,
    leftNo: r.leftNo,
    rightNo: r.rightNo,
    left: r.leftNo === null ? null : a[r.leftNo - 1],
    right: r.rightNo === null ? null : b[r.rightNo - 1],
  })

  const out: DiffRow[] = []
  let k = 0
  while (k < raw.length) {
    if (raw[k].kind !== 'del') {
      out.push(row(raw[k]))
      k += 1
      continue
    }
    const dels: (typeof raw)[number][] = []
    while (k < raw.length && raw[k].kind === 'del') {
      dels.push(raw[k])
      k += 1
    }
    const adds: (typeof raw)[number][] = []
    while (k < raw.length && raw[k].kind === 'add') {
      adds.push(raw[k])
      k += 1
    }
    const paired = Math.min(dels.length, adds.length)
    for (let i = 0; i < paired; i += 1) {
      const d = dels[i]
      const ad = adds[i]
      out.push({
        kind: 'change',
        leftNo: d.leftNo,
        rightNo: ad.rightNo,
        left: d.leftNo === null ? null : a[d.leftNo - 1],
        right: ad.rightNo === null ? null : b[ad.rightNo - 1],
      })
    }
    for (let i = paired; i < dels.length; i += 1) out.push(row(dels[i]))
    for (let i = paired; i < adds.length; i += 1) out.push(row(adds[i]))
  }
  return out
}

/** 差异统计：改了几行、删了几行、加了几行（底栏与弹窗的摘要用） */
export function diffSummary(rows: DiffRow[]): { changed: number; added: number; removed: number } {
  let changed = 0
  let added = 0
  let removed = 0
  for (const r of rows) {
    if (r.kind === 'change') changed += 1
    else if (r.kind === 'add') added += 1
    else if (r.kind === 'del') removed += 1
  }
  return { changed, added, removed }
}
