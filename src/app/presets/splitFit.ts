/*
 * 详情卡双列的**中线自适应**（2026-10-30 作者口径）。
 *
 * 作者看窄窗截图后说的两件事：
 *   ①「左右分布能不能灵活一点，有 10% 或 20% 的容差，就可以左一点或者右一点」；
 *   ②「主要还是看文字 —— 这个线如果再往左一点，那右边那个三行可能就能变成两行了」。
 *
 * 所以中线不再是个定值（原来 `1.15fr / 1fr` 写死在 CSS 里），改成**量出来的**：
 * 在 `SPLITS` 里试 11 条（正中 ±10%），逐条量两列**各自的内容高**，按四把尺子依次比：
 *
 *   ① 更高的那列**最矮** —— 卡片高度由它决定，这是"最省高度"；
 *   ② 两列合计**最矮** —— 卡高一样时，别让文字无谓折行（2026-10-30 第二轮：
 *      作者看截图「右边应该是够位置的吧，但它却换行了」：右侧短的那列被压窄、
 *      左边却空空一大片 —— 只比"更高那列"时这种换行不改变卡高，就永远纠正不过来）；
 *   ③ 两列高差**最小** —— 作者：「让两侧的那个行几乎一样」；
 *   ④ 离正中**最近** —— 作者：「我还是希望他能视觉居中」。
 *
 * 为什么量"内容高"而不是 `offsetHeight`：grid 会把两列拉成同一高度，量出来永远相等。
 * 取"最后一组的底边 − 列顶"才是这一列自己写了多少行。
 *
 * 为什么整轮放在一次同步调用里：中途只改 CSS 变量 + 同步读布局，**同一个任务里不会绘制**，
 * 用户看不到中间那 10 条候选；跑完才落一个值。窗口缩放那一路才用 rAF 合并。
 */

/**
 * 候选中线（左列占"文字可用宽"的比例）。
 * 正中 ±10%：再偏就不像"分两栏"了，宁可高一行。
 */
const SPLITS = [0.4, 0.42, 0.44, 0.46, 0.48, 0.5, 0.52, 0.54, 0.56, 0.58, 0.6]

/** 比"更矮"时容忍的像素：差不到 1.5px 就当一样矮，免得像素抖动来回跳 */
const EPS = 1.5

/** 这一列自己写了多少高（不看被 grid 拉齐后的高度） */
function contentHeight(col: Element | undefined): number {
  if (col === undefined) return 0
  const last = col.lastElementChild
  if (last === null) return 0
  return last.getBoundingClientRect().bottom - col.getBoundingClientRect().top
}

interface Pick {
  split: number
  /** 更高的那列 —— 卡片高度 */
  tall: number
  /** 两列合计 —— 越小说明"文字折行"越少 */
  sum: number
  /** 两列高差 —— 越小越"两栏一样" */
  diff: number
  /** 离正中多远 */
  off: number
}

/** 依次比：更矮 → 折行更少 → 更接近 → 更靠中 */
function better(a: Pick, b: Pick): boolean {
  if (a.tall < b.tall - EPS) return true
  if (a.tall > b.tall + EPS) return false
  if (a.sum < b.sum - EPS) return true
  if (a.sum > b.sum + EPS) return false
  if (a.diff < b.diff - EPS) return true
  if (a.diff > b.diff + EPS) return false
  return a.off < b.off
}

/**
 * 量一轮，把选中的中线写进 `--dSplit`（CSS 用它算 `grid-template-columns`）。
 * 同步跑完，调用方负责"什么时候量"。
 */
export function fitSplit(cols: HTMLElement): void {
  const left = cols.children[0]
  const right = cols.children[1]
  /* 不是双列（将来真有单列档）就别动它 */
  if (left === undefined || right === undefined) return

  let best: Pick | null = null
  for (const split of SPLITS) {
    cols.style.setProperty('--dSplit', String(split))
    const hl = contentHeight(left)
    const hr = contentHeight(right)
    const one: Pick = {
      split,
      tall: Math.max(hl, hr),
      sum: hl + hr,
      diff: Math.abs(hl - hr),
      off: Math.abs(split - 0.5),
    }
    if (best === null || better(one, best)) best = one
  }
  cols.style.setProperty('--dSplit', String(best?.split ?? 0.5))
}

/**
 * 挂上去：立刻量一次，之后**只在宽度变了**时重量（窗口缩放 / 换页签）。
 *
 * ★ 宽度没变就不量，是防"自己触发自己"：量完高度会变，ResizeObserver 会再叫一次，
 *   那道闸把它挡住。内容变化（比如展开「更多详情」）不值得重量 —— 那一块整卡通铺，
 *   不参与两列配比。
 *
 * 返回拆除函数（effect 里直接 `return`）。
 */
export function fitSplitWhenResized(cols: HTMLElement): () => void {
  let raf = 0
  let lastWidth = -1
  const run = () => {
    raf = 0
    const width = cols.clientWidth
    if (width === lastWidth) return
    lastWidth = width
    fitSplit(cols)
  }
  const schedule = () => {
    if (raf === 0) raf = requestAnimationFrame(run)
  }
  const ro = new ResizeObserver(schedule)
  ro.observe(cols)
  /*
   * 先**同步**量一次：调用方住在 `useLayoutEffect` 里，这一刻 DOM 已经挂好、
   * 还没绘制 —— 量完立刻定列宽，用户第一眼看到的就是量过的那一版。
   * 交给 rAF 会先画一帧 0.5 的默认值再挪线（能看出来跳一下）。
   */
  run()
  return () => {
    ro.disconnect()
    if (raf !== 0) cancelAnimationFrame(raf)
    cols.style.removeProperty('--dSplit')
  }
}
