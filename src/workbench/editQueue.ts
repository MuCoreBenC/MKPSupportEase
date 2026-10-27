/*
 * 参数值写入的「单飞合批队列」+ 乐观覆盖层（2026-10-19）。
 *
 * # 它解决什么
 *
 * 参数台原来**每点一下**就走一次 `wb.apply_draft`，外壳还顺手拨 `tick`，
 * 让页面把 `wb_registry` / 整页 `wb_desk` / `wb_matrix` 再各问一遍。而 Rust 侧
 * 这些命令都是**同步的（跑主线程）**，`wb_preflight` 更重（parse recipes + 摸盘建
 * catalog）。连点步进器时主线程被这一串命令占死 —— 数字不跟手，**停下之后才
 * "慢慢追上"点击的次数**（作者 2026-10-19 的实测）。
 *
 * 这一层照客户端 `useParams` 那套手感抄，规矩三条：
 *
 *   1. **本地即时反馈**：`push` 进来的一格立刻用乐观值盖在 base 上显示
 *      （`overlayOf` + `patchDesk` / `patchMatrix`），不等后端往返；
 *   2. **一次手势一次落库**：同一格 latest-wins 覆盖，120ms 尾随合批，
 *      **单飞**（同一时刻只有一条 `applyDraft` 在飞）——
 *      连点 20 下 = 1 条 IPC、1 条撤销；
 *   3. **本地层落地也不撤**（2026-10-19 第二轮）：条目发出去、后端也回了，它**仍然留着**当
 *      "这一屏的草稿层" —— 显示只认它。撤早了就会闪一帧（见 `pump` 的注）。
 *      只有"用户改回来 / 保存 / 放弃 / 撤销重做 / 换目标"才收（`clear` / 新 push 覆盖）。
 *   4. 响应**只写 base**，本地层永远最后叠；响应回来先比 `refreshKey`，
 *      与当前页对不上就丢弃（那次派生已经过期）。
 *
 * 这就是客户端 `useParams` 的形状：**显示值不随后端往返来回改**（它只在用户自己动、
 * 或整层被清掉时才变）。作者 2026-10-19 的原话：「客户端右侧 IPC 滚得那么快，前端那个
 * 数字也不闪；工作台变动频率这么慢，数字却在闪」。
 *
 * # 为什么"单飞"是硬要求
 *
 * patch 里的值是**绝对值**，所以两条在飞不至于写错值；但两条在飞时
 * "早发晚回"会让 base 落在旧值上 —— 而那一刻 overlay 已经清掉了，
 * 显示就会当场跳回去。串行 + latest-wins 把这条缝堵死。
 *
 * # 什么不归它管
 *
 * 撤销 / 重做 / 保存 / 放弃是**低频结构操作**，走外壳那条老路（拨 `tick`、
 * 全页重取）—— 它们动手前先 `await settleAll()` 把飞在空中的写落完。
 *
 * # 与 `c14/toast.ts` / `pageActions.ts` 同一副骨架
 *
 * 模块级 store + `useSyncExternalStore`：`snapshot()` 只在队列真的变过时才换引用，
 * 于是没改动的那几帧，React 一次都不重渲染。
 */

import type { Cell, Col, Desk, DeskGroup, Matrix, Patch, Refresh, Row } from './api'

/** 一格的身份：`机型 | 版本（基底为空串） | 参数键` */
export const cellKeyOf = (machineId: string, versionUid: string | null, key: string): string =>
  `${machineId}|${versionUid ?? ''}|${key}`

/**
 * `Refresh` 的记号：响应回来拿它跟"当前页"比 —— 对不上就是过期派生，丢弃。
 *
 * 写法与 `ParamsPage` 里那个 `refresh` / `compareColRefs` 一一对应；
 * `Matrix` 那一支按列序列拼（列序变了就是另一次派生）。
 */
export const refreshKeyOf = (r: Refresh): string =>
  r.page === 'desk'
    ? `desk|${r.machineId}|${r.uid ?? ''}|${r.tab ?? ''}|${r.query ?? ''}`
    : `matrix|${r.cols.map((c) => `${c.machineId}:${c.versionUid ?? ''}`).join(',')}|${r.tab ?? ''}|${r.query ?? ''}`

/** 队列上的一格。 */
export interface EditEntry {
  cellKey: string
  /** 撤销按钮上那句人话（与既有 `onApply` 的 label 同口径） */
  label: string
  patch: Patch
  /** 乐观显示值（TOML 原文；`''` = 删键 / 挂回继承） */
  raw: string
  refresh: Refresh
  refreshKey: string
  /**
   * 这一格已经发出去过了（**只是"发过"，不是"可以撤掉显示"**）。
   *
   * ★ 2026-10-19 第二轮补修（作者录屏里那一帧回跳）：这一格**落地之后也不撤显示** ——
   * 它本身就是**这一屏的本地草稿层**（与客户端 `useParams` 的 `draft` map 同一个角色）。
   * 撤早了，显示就落回后端那份 desk（手上还是上一次的值）⇒ 数字**闪一帧再回来**。
   * 只有"用户改回来 / 保存 / 放弃 / 撤销重做 / 换目标"才把它收掉（`clear` / 新 push 覆盖）。
   */
  sent: boolean
}

/** 这一页的写入口。返回 false = 这次没写进去（显示**不回滚** —— 与客户端同一条口径） */
export type EditSender = (batch: EditEntry[]) => Promise<boolean>

/** 尾随时长：连点之间隔这么久就合批成一次 */
const TAIL_MS = 120

const queue = new Map<string, EditEntry>()
/** 只在队列变过时换引用 —— `useSyncExternalStore` 靠它 */
let snap: EditEntry[] = []
let timer: number | null = null
/** 当前那一趟（单飞）。null = 没有在飞 */
let active: Promise<void> | null = null
let sender: EditSender | null = null
const listeners = new Set<() => void>()

function rebuild(): void {
  snap = [...queue.values()]
  for (const fn of listeners) fn()
}

function clearTimer(): void {
  if (timer !== null) {
    window.clearTimeout(timer)
    timer = null
  }
}

function schedule(): void {
  /*
   * **真尾随**：每次 push 都重新计时 —— 只要人还在连点（间隔 < TAIL_MS），
   * 就一条都不发，等他手停下来 120ms 才合批发一次。
   * （"排着就不重置"那种写法会在连点中途插进去发，连点 10 下会切成两三批，
   *  撤销也就跟着碎成两三条 —— 与 `NumberField` 那句"一次手势 = 一条撤销"打架。）
   */
  if (timer !== null) window.clearTimeout(timer)
  timer = window.setTimeout(() => {
    timer = null
    void pump()
  }, TAIL_MS)
}

/**
 * 取一批（同 `refreshKey`、**还没发过**的）发出去。**单飞**：在飞时原样把那一趟交回去，
 * 由它落地后自己续发 —— 不排队、不并行。
 *
 * ★ **落地之后不撤这一格**（2026-10-19 第二轮补修）：条目留在队列里继续当"本地草稿层"。
 * 上一版在落地后就删了 —— 那一刻 `patchDesk` 落回后端那份 desk（手上还是上一次的值），
 * 数字**闪一帧再回来**（作者录屏里 12 帧抓到的正是 `0.2 → 0.15 → 0.2` 那一帧）。
 */
function pump(): Promise<void> {
  if (active !== null) return active
  const all = [...queue.values()].filter((e) => !e.sent)
  if (all.length === 0) return Promise.resolve()
  /* 一次合批只收同一个 refreshKey 的条目：跨页/跨查询合成一条，
     撤销一下退掉两处的改动会说不清 */
  const key = all[0].refreshKey
  const batch = all.filter((e) => e.refreshKey === key)
  /* 先标"发过了"：飞入期间又被点过的那一格会被整条换掉（`sent: false`），下一趟再发 */
  for (const e of batch) e.sent = true

  const send = sender
  active = (async () => {
    try {
      if (send !== null) await send(batch)
    } catch {
      /* 发送失败：错误横幅由外壳 `run` 那条路负责说，这一层不重复报；
         显示**不回落**（客户端的 `patchDraft` 也是这条口径：失败只记 `draftError`，
         不回滚界面值 —— "真机上草稿没写进去"这件事要看得见） */
    } finally {
      active = null
    }
    /* 后面还有没发的（别的 refreshKey / 飞入期间又被点过）→ 立刻续发 */
    if (hasUnsent()) schedule()
  })()
  return active
}

/** 还有没发出去的吗（`settleAll` 的循环条件用） */
function hasUnsent(): boolean {
  for (const e of queue.values()) if (!e.sent) return true
  return false
}

/**
 * 把**还没发的**都发完、在飞的等落地。撤销 / 重做 / 保存 / 放弃之前先 await 它。
 *
 * ★ 循环条件**不看 `queue.size > 0`** —— 条目现在是"落地也不撤"的本地草稿层，
 * 队列几乎永远是满的，按它判会死循环。
 */
export async function settleAll(): Promise<void> {
  clearTimer()
  while (active !== null || hasUnsent()) {
    if (active !== null) await active
    else await pump()
    clearTimer()
  }
}

export const editQueue = {
  subscribe(fn: () => void): () => void {
    listeners.add(fn)
    return () => {
      listeners.delete(fn)
    }
  },
  snapshot(): EditEntry[] {
    return snap
  },
  /**
   * 排一格（同键 **latest-wins**）。`send` 是这一页的写入口 —— 每次 push 都
   * 刷一遍引用（它跟着页面的闭包走，可能是新的那个）。
   *
   * ★ 条目**不会因为"发出去了"而消失**（`sent` 只是"还没发过"的记号）：
   * 它同时是**这一屏的本地草稿层** —— 与客户端 `useParams` 的 `draft` map 同一个角色。
   * 见 `pump` 的注。
   */
  push(entry: Omit<EditEntry, 'sent'>, send: EditSender): void {
    sender = send
    queue.set(entry.cellKey, { ...entry, sent: false })
    rebuild()
    schedule()
  },
  /**
   * **把本地草稿层整层丢掉**：保存 / 放弃 / 撤销 / 重做之后调 ——
   * 那些时候后端那份 desk 才是真相（显示交回它）。同键的新 push 也会覆盖单条。
   */
  clear(): void {
    if (queue.size === 0) return
    queue.clear()
    rebuild()
  },
}

/* ---------- 乐观覆盖层（只画，不改 base） ---------- */

/** 队列快照 → `cellKey → 乐观原文` */
export function overlayOf(entries: EditEntry[]): Map<string, string> {
  const m = new Map<string, string>()
  for (const e of entries) m.set(e.cellKey, e.raw)
  return m
}

function patchRow(row: Row, cols: Col[], ov: Map<string, string>): Row {
  let any = false
  const cells = row.cells.map((cell, i) => {
    const c = cols[i]
    if (c === undefined) return cell
    const raw = ov.get(cellKeyOf(c.machineId, c.versionUid ?? null, row.key))
    if (raw === undefined || String(cell.raw ?? '') === raw) return cell
    any = true
    /* 乐观那格顺手点亮 `dirty` —— 状态徽章与「改之前的值」chip 都读它；
       与 base 相同的那一格不算脏（overlay 覆盖到同值 = 等于没改） */
    return { ...cell, raw, dirty: true } as Cell
  })
  return any ? { ...row, cells } : row
}

function patchGroup(group: DeskGroup, cols: Col[], ov: Map<string, string>): DeskGroup {
  let any = false
  const items = group.items.map((it) => {
    const row = patchRow(it.row, cols, ov)
    const children = it.children.map((c) => patchRow(c, cols, ov))
    if (row === it.row && children.every((c, i) => c === it.children[i])) return it
    any = true
    return { ...it, row, children }
  })
  return any ? { ...group, items } : group
}

/** base desk 叠上 overlay。队列空时原样返回（引用不变 ⇒ 不重渲染）。 */
export function patchDesk(desk: Desk | null, ov: Map<string, string>): Desk | null {
  if (desk === null || ov.size === 0) return desk
  let any = false
  const groups = desk.groups.map((g) => {
    const next = patchGroup(g, desk.cols, ov)
    if (next !== g) any = true
    return next
  })
  return any ? { ...desk, groups } : desk
}

/** base matrix 叠上 overlay。同 `patchDesk`。 */
export function patchMatrix(matrix: Matrix | null, ov: Map<string, string>): Matrix | null {
  if (matrix === null || ov.size === 0) return matrix
  let any = false
  const rows = matrix.rows.map((r) => {
    const next = patchRow(r, matrix.cols, ov)
    if (next !== r) any = true
    return next
  })
  return any ? { ...matrix, rows } : matrix
}
