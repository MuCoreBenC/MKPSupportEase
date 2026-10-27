/*
 * 差异概览条的标记与测量（2026-10-19）。
 *
 * 单独一个 `.ts` 而不是放在 `DiffView.tsx` 里：那个文件只导出组件（快刷新边界），
 * 这里是可以给两个框共用的纯函数。
 */

/**
 * 右侧概览条上的一枚标记。
 *
 * 位置是**比例**（0–1），不存像素 —— 窗口 resize / 长行折行后行高会变，
 * 重测一次就能重画。`row` 是 diff 行在数组里的下标（删行的行号是旧文件行号，
 * 可能重复，不能当锚点）；点击时按它 `querySelector` 跳转。
 */
export interface RulerMark {
  top: number
  height: number
  kind: 'add' | 'del'
  row: number
  line: number
}

/**
 * 量一遍 diff 内容，给出概览条要画的标记（比例坐标）。
 *
 * 两个框共用这一处：相邻标记（<0.4%）并成一段 —— 一改十几行时不然会糊成一根实心条。
 * 只认 `DiffLines` 打上的 `data-kind` / `data-row` / `data-line` 锚点。
 */
export function measureRulerMarks(el: HTMLElement): RulerMark[] {
  const total = el.scrollHeight || 1
  const rows = el.querySelectorAll<HTMLElement>('[data-kind="added"],[data-kind="removed"]')
  const out: RulerMark[] = []
  rows.forEach((row) => {
    const kind = row.dataset.kind === 'added' ? 'add' : 'del'
    const index = Number(row.dataset.row ?? -1)
    const line = Number(row.dataset.line ?? 0)
    const top = row.offsetTop / total
    const height = Math.max(row.offsetHeight / total, 0.004)
    const prev = out[out.length - 1]
    if (prev !== undefined && top - prev.top < 0.004) {
      prev.height = Math.max(prev.height, top + height - prev.top)
      return
    }
    out.push({ top, height, kind, row: index, line })
  })
  return out
}
