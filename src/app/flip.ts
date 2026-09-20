/**
 * FLIP：先记下旧的布局位置，改完布局再量新位置，把差值写成起点动画回 0。
 *
 * 为什么用 offsetLeft/offsetTop 而不是 getBoundingClientRect：
 * 做这件事的时机正好是露出卡在 translateX(80%) → 0 的过程中，
 * rect 会把祖先的 transform 一起算进来，量出来的差值就把"卡片自己在滑"也算了一遍。
 * offset* 是纯布局值，不受任何 transform 影响 —— 卡片的位移由卡片自己的动画负责，
 * 这里只负责"竖排位置 → 横排位置"这一段。
 */
export interface Pos {
  x: number
  y: number
}

/** 沿 offsetParent 链累加到根，得到不受 transform 影响的布局坐标 */
export function layoutPos(el: HTMLElement): Pos {
  let x = 0
  let y = 0
  let node: HTMLElement | null = el
  while (node) {
    x += node.offsetLeft
    y += node.offsetTop
    node = node.offsetParent as HTMLElement | null
  }
  return { x, y }
}

/**
 * 从 from 补间到 to（元素此刻已经在 to）。
 * 差值不足半像素就不动 —— 免得为了 0.2px 排一次合成。
 *
 * delay 是为了跟卡片自己的动画对齐：推入是「先滑动、后长大」，
 * 退回是「先缩小、后滑走」，两个方向的位移窗口不在同一段时间上。
 */
export function flipTo(el: HTMLElement, from: Pos, to: Pos, ms: number, delay = 0): void {
  if (typeof el.animate !== 'function') return

  const dx = from.x - to.x
  const dy = from.y - to.y
  if (Math.abs(dx) < 0.5 && Math.abs(dy) < 0.5) return

  el.animate([{ transform: `translate(${dx}px, ${dy}px)` }, { transform: 'translate(0, 0)' }], {
    duration: ms,
    delay,
    easing: 'cubic-bezier(0.32, 0.68, 0.12, 1)',
    // both：延迟那一段就套用起点，否则会先瞬跳到终点再退回去
    fill: 'both',
  })
}
