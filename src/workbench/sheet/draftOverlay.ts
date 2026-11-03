/*
 * 本地草稿层 + 「脏」的唯一判据（参数台 v2 的内核之一）。
 *
 * # 它替换的是什么
 *
 * 旧页的乐观显示住在 `editQueue` 里：条目**落地之后也不撤**，靠"下次 push 覆盖"
 * 或"保存 / 放弃 / 撤销"才收掉。于是「我改过，后端还没跟上」与「我改过，
 * 后端已经跟上了」分不开 —— 徽章要等保存才灭。
 *
 * 这里把两件事分开：**草稿只在"后端还没跟上"时存在**，后端一跟上就删
 * （`cell.raw` 等于草稿的 `raw` ⇒ `settleDraft` 收掉）。
 *
 * # 「脏」的判据只有一个：值不一样
 *
 * ```
 * 脏 = 现在显示的值  ≠  保存值
 * ```
 *
 * **不读后端那个 `Cell.dirty`**。2026-11-02 的截图里出现过「值 = 65、保存值 = 65、
 * 却标着『已修改』、右上角『未保存 (0)』」—— 那是两个源各说各话：后端 `dirty_of`
 * 与事务栈数出来的数不一致。用户看得见的是值，所以判据也只能是值。
 *
 * # 「保存值」是三层算完的有效值，不是"文件里那格"
 *
 * 版本覆盖 → 机型基底 → 出厂默认。所以**文件里那格空着**也照样有保存值
 * （`A1:STANDARD` 没钉 ⇒ 继承机型基底；机型基底没有 ⇒ 出厂默认）。
 * 作者 2026-11-02 问的正是这一条：「版本和基底一样的话，它那一条就是空着的对吧，
 * 那这个徽章能知道原来的是多少吗」—— 能，`Cell.rest` 就是它。
 */

import type { Cell, Col, Desk, DeskGroup, Matrix, Row } from '../api'

/** 一格的身份：`机型 | 版本（基底为空串） | 参数键`。与后端 `Col.key` + 行键同构 */
export const cellKeyOf = (machineId: string, versionUid: string | null, key: string): string =>
  `${machineId}|${versionUid ?? ''}|${key}`

/** 一条草稿。`raw` 是 TOML 原文（`''` = 删键 / 挂回继承） */
export interface DraftEntry {
  raw: string
}

/**
 * 值 → TOML 原文。**唯一转换点**。
 *
 * 旧页那处 `String(cell.rest)` 在 `rest === null` 时会得到 `"null"`，经数字控件
 * 解析失败回落成 `0` —— 作者看到的「点 ↩ 70 变 0」就是这一行。这里显式给 `''`。
 */
export function rawOf(v: unknown): string {
  if (v === null || v === undefined) return ''
  if (typeof v === 'string') return v
  if (typeof v === 'number' || typeof v === 'boolean') return String(v)
  return JSON.stringify(v)
}

/** 这一格在盘上（BASE）的**有效值**原文。`null` = 这一项不适用 / 弃用 */
export const savedRawOf = (cell: Cell): string => rawOf(cell.rest)

/** 这一格现在显示什么原文（草稿优先） */
export const shownRawOf = (cell: Cell, e: DraftEntry | undefined): string =>
  e === undefined ? rawOf(cell.raw) : e.raw

/** 脏 = 现在显示的值 ≠ 保存值。**全页唯一判据**（见文件头） */
export const dirtyOf = (cell: Cell, e: DraftEntry | undefined): boolean =>
  shownRawOf(cell, e) !== savedRawOf(cell)

function patchRow(row: Row, cols: Col[], draft: ReadonlyMap<string, DraftEntry>): Row {
  let any = false
  const cells = row.cells.map((cell, i) => {
    const c = cols[i]
    if (c === undefined) return cell
    const key = cellKeyOf(c.machineId, c.versionUid ?? null, row.key)
    const e = draft.get(key)
    const dirty = dirtyOf(cell, e)
    if (e === undefined && cell.dirty === dirty) return cell
    any = true
    /*
     * 只换"看着就是原文"的那一档（数值）；枚举要 label、G-code 要行数摘要，
     * 都不在前端重算 —— 与旧页 `patchRow` 同一条口径。
     */
    const raw = shownRawOf(cell, e)
    const text = e === undefined || raw === '' || !Number.isFinite(Number(raw)) ? cell.text : raw
    return { ...cell, raw: e === undefined ? cell.raw : raw, text, dirty } as Cell
  })
  return any ? { ...row, cells } : row
}

function patchGroup(group: DeskGroup, cols: Col[], draft: ReadonlyMap<string, DraftEntry>): DeskGroup {
  let any = false
  const items = group.items.map((it) => {
    const row = patchRow(it.row, cols, draft)
    const children = it.children.map((c) => patchRow(c, cols, draft))
    if (row === it.row && children.every((c, i) => c === it.children[i])) return it
    any = true
    return { ...it, row, children }
  })
  return any ? { ...group, items } : group
}

/**
 * 一屏叠上草稿，**并把「脏」重算一遍**（不管有没有草稿）。
 *
 * 重算而不是沿用后端那个 `dirty`：截图里出现过「值 = 保存值、却标着已修改、
 * 右上角却是 0」—— 显示与「未保存 N」必须同一个源。
 */
export function applyDraftToDesk(
  desk: Desk | null,
  draft: ReadonlyMap<string, DraftEntry>,
): Desk | null {
  if (desk === null) return desk
  let any = false
  const groups = desk.groups.map((g) => {
    const next = patchGroup(g, desk.cols, draft)
    if (next !== g) any = true
    return next
  })
  return any ? { ...desk, groups } : desk
}

/** 对照矩阵叠上草稿。同 `applyDraftToDesk` */
export function applyDraftToMatrix(
  matrix: Matrix | null,
  draft: ReadonlyMap<string, DraftEntry>,
): Matrix | null {
  if (matrix === null) return matrix
  let any = false
  const rows = matrix.rows.map((r) => {
    const next = patchRow(r, matrix.cols, draft)
    if (next !== r) any = true
    return next
  })
  return any ? { ...matrix, rows } : matrix
}

/**
 * 后端已经跟上来的那几条草稿可以删了。
 *
 * 判据：后端这一格的 `raw` 与草稿的 `raw` 相同 ⇒ 显示换回后端那份不会跳
 * （两者字面一致），草稿这一条就没有存在意义了。
 */
export function settleDraft(
  desk: Desk | null,
  draft: ReadonlyMap<string, DraftEntry>,
): Map<string, DraftEntry> {
  if (desk === null || draft.size === 0) return draft as Map<string, DraftEntry>
  const next = new Map(draft)
  for (const g of desk.groups) {
    for (const it of g.items) {
      for (const row of [it.row, ...it.children]) {
        row.cells.forEach((cell, i) => {
          const c = desk.cols[i]
          if (c === undefined) return
          const key = cellKeyOf(c.machineId, c.versionUid ?? null, row.key)
          const e = next.get(key)
          if (e === undefined) return
          if (rawOf(cell.raw) === e.raw) next.delete(key)
        })
      }
    }
  }
  /* 一条都没删 ⇒ 返回**同一个引用**（React 靠它跳过重渲染） */
  return next.size === draft.size ? (draft as Map<string, DraftEntry>) : next
}
