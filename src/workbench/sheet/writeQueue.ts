/*
 * 前端草稿 → 后端编辑树的**单飞写入队列**（参数台 v2，2026-11-02）。
 *
 * 规则书：`docs/WORKBENCH-SHEET-V2-PROTOCOL.md` §4。这个文件就是那一节的实现，
 * 改动前先读那一节。
 *
 * # 为什么不能并发
 *
 * 前端 `draft` 是**显示与徽章**的权威，后端编辑树是**保存**的权威。两者只能靠
 * "请求按发起顺序一条一条落"来对齐。并发发的后果是真实发生过的：
 *
 *   t0  80 发出（seq 1）
 *   t1  90 发出（seq 2）
 *   t2  90 先回来
 *   t3  80 后回来   ⇒ 后端最终是 80，界面上是 90
 *
 * 单飞（一次只发一条、回来才发下一条）让这个时序**不可能出现**。
 *
 * # 为什么按 seq 最小排空，而不是最新优先
 *
 * A 格连改三次、B 格改一次。若"最新优先"，A 的中间态被永远压住 —— 而 A 的最终值
 * 只由最后一次决定，中间态发不发其实无所谓；真正要紧的是**跨格的发起顺序**不能让
 * 某一格饿死。按最早发起的那条走，谁都不会被跳过去。
 *
 * # 为什么同一格只留最新一条
 *
 * 连点步进器 20 下会产生 20 条；中间 19 条没有任何意义（后一条覆盖前一条）。
 * 留最新 + 保留原来的 seq（它在队列里的位置不变）= 既不发中间态、也不插队。
 *
 * # 失败怎么办
 *
 * 该格的 `draft` **回滚到 `saved`**（不是回滚到"上一次成功的值"—— 那个值可能已经
 * 被别的路径改过），报错给上层，**继续排空**（不因一条失败卡死其它格）。
 */

import type { Patch } from '../api'

export interface QueueEntry {
  /** 一格的身份证：`${colKey}|${paramKey}` */
  cellKey: string
  /** 给撤销 / 历史看的一句人话 */
  label: string
  patch: Patch
  /** **第一次**入队的序号。同一格后续覆盖不改它（位置不变） */
  seq: number
  /** 入队时的代数。与当前代数不同 ⇒ 这次的结果一律丢弃（放弃之后迟到的那条） */
  epoch: number
}

export interface WriteQueueOptions {
  /** 真发一条。失败请抛 */
  send: (entry: QueueEntry) => Promise<void>
  /** 某一条彻底失败：上层把那一格的 draft 退回 saved + 报错 */
  onFail: (cellKey: string, err: unknown) => void
  /**
   * 队列长度变了（入队 / 发出 / 落定）。页头「保存」点不点得动、状态字写什么
   * 都读这个 —— **不许靠轮询猜**（轮询会把"刚清空"晚 120ms 才显示出来）。
   */
  onChange?: (pending: number) => void
}

export class WriteQueue {
  private pending = new Map<string, QueueEntry>()
  private inflight: QueueEntry | null = null
  private seq = 0
  private epoch = 0
  private idleWaiters: (() => void)[] = []
  private readonly send: WriteQueueOptions['send']
  private readonly onFail: WriteQueueOptions['onFail']
  private readonly onChange: ((pending: number) => void) | undefined

  constructor(opts: WriteQueueOptions) {
    this.send = opts.send
    this.onFail = opts.onFail
    this.onChange = opts.onChange
  }

  /** 还没发出去的有几格（同一格算一格） */
  get pendingCount(): number {
    return this.pending.size
  }

  /** 有没有一条正在飞 */
  get busy(): boolean {
    return this.inflight !== null
  }

  /**
   * 记一条待写。**调用前上层必须已经让本地 `draft` 立刻生效** ——
   * 界面的显示与徽章不等这个队列（协议 §4「权威划分」）。
   */
  enqueue(cellKey: string, label: string, patch: Patch): void {
    const old = this.pending.get(cellKey)
    this.pending.set(cellKey, {
      cellKey,
      label,
      patch,
      seq: old?.seq ?? ++this.seq,
      epoch: this.epoch,
    })
    this.onChange?.(this.pending.size)
    this.pump()
  }

  /**
   * 等排空（`inflight` 结束 + `pending` 空）。
   *
   * **保存前必须调**：带着未确认的编辑去落盘，等于把"我改过"变成"我没改过"。
   * 超时**拒绝**（不是静默继续）—— 上层据此拒绝保存。
   */
  drain(timeoutMs = 5000): Promise<void> {
    if (this.isIdle()) return Promise.resolve()
    return new Promise<void>((resolve, reject) => {
      const timer = setTimeout(() => {
        this.idleWaiters = this.idleWaiters.filter((fn) => fn !== waiter)
        reject(new Error(`还有 ${this.pending.size} 处改动没落下（等满 ${timeoutMs}ms）`))
      }, timeoutMs)
      const waiter = () => {
        clearTimeout(timer)
        resolve()
      }
      this.idleWaiters.push(waiter)
    })
  }

  /**
   * 放弃：清掉还没发的；**在飞那条的结果按代数丢弃**（请求本身收不回来，
   * 但它落进编辑树之后会被随后的 `wb_discard` 整份覆盖掉）。
   */
  cancelAll(): void {
    this.epoch += 1
    this.pending.clear()
    this.onChange?.(this.pending.size)
    this.maybeIdle()
  }

  private isIdle(): boolean {
    return this.inflight === null && this.pending.size === 0
  }

  private maybeIdle(): void {
    if (!this.isIdle()) return
    const waiters = this.idleWaiters
    this.idleWaiters = []
    for (const fn of waiters) fn()
  }

  /** 取 seq 最小的那一条（不按插入顺序取 Map 的第一个 —— 覆盖过的条目位置不变，两者等价，但显式更稳） */
  private takeNext(): QueueEntry | null {
    let best: QueueEntry | null = null
    for (const e of this.pending.values()) {
      if (best === null || e.seq < best.seq) best = e
    }
    if (best !== null) this.pending.delete(best.cellKey)
    return best
  }

  private pump(): void {
    if (this.inflight !== null) return
    const next = this.takeNext()
    if (next === null) {
      this.maybeIdle()
      return
    }
    this.inflight = next
    this.onChange?.(this.pending.size)
    void this.run(next)
  }

  private async run(entry: QueueEntry): Promise<void> {
    try {
      await this.send(entry)
    } catch (err) {
      /* 迟到的那条（放弃之后才回来）**不报错、不回滚** —— 用户已经放弃了这一份，
         再弹一条"某某没写进去"只会让人以为放弃失败 */
      if (this.epoch === entry.epoch) this.onFail(entry.cellKey, err)
    }
    if (this.inflight === entry) this.inflight = null
    this.onChange?.(this.pending.size)
    this.maybeIdle()
    this.pump()
  }
}

/**
 * 外壳与这一页之间唯一的那条线（2026-11-02）。
 *
 * # 为什么需要它
 *
 * 页头那颗「保存」住在 `App.tsx`，它不知道这一页手里还压着几条没发出去的写入。
 * 保存前必须先排空 —— 否则「刚点完步进器马上点保存」会静默丢掉那一笔。
 *
 * 不用 `usePageActions` 接管页头槽，是因为那会把页头换成另一副样子（「已写进仓库 /
 * 放弃 / 保存」三件）；作者要的是**这一页的页头与旧参数台一模一样**，所以只挂一条
 * 后台钩子，外观一个字不动。
 */
export const sheetWrites = {
  drain: null as null | (() => Promise<void>),
  reset: null as null | (() => void),
  register(drain: () => Promise<void>, reset: () => void): void {
    this.drain = drain
    this.reset = reset
  },
  unregister(): void {
    this.drain = null
    this.reset = null
  },
  /** 保存 / 放弃之前调：等这一页的写入排空 */
  async settle(): Promise<void> {
    if (this.drain !== null) await this.drain()
  },
  /** 后端把已有改动撤掉了（保存成功 / 放弃 / 撤销 / 重做）⇒ 草稿层作废 */
  drop(): void {
    this.reset?.()
  },
}
