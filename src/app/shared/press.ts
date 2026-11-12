/*
 * 按钮的「按下沉、松开回位」—— 全站一套手感（作者 2026-11-11 定的三段节奏：
 * 悬停只改颜色 → 按下 90ms 沉下去 → 松开 140ms 平滑回位）。
 *
 * ★ **只往下沉，不横向缩**（作者 2026-11-11：「它会导致宽度变化，右左右跳动」）：
 *   横向 `scale()` 会让胶囊窄 4%（几十像素里就是五六个像素），右线不动、左缘来回动 ——
 *   看着就是"宽度在跳"。下沉改用 `translateY + scaleY`，横向一个像素都不变。
 * ★ **回位不是弹簧**（作者：「去掉那个弹簧果冻的效果，它和我再次点击有点冲突」）。
 * ★ 只动 `transform`（合成层），不碰 width / margin / padding。
 * ★ 走 WAAPI：WKWebView 上 CSS transition / animation 不播（全站同一条）。
 * ★ 系统开「减弱动态效果」就整个不演 —— 按下去什么也不动。
 * ★ 只给**本身可点**的控件用：药丸里嵌着「放弃 / 保存」那种，按钮的手感由按钮自己管，
 *   壳不跟着动（校准页那颗药丸就属于这种情况，所以它不接这一套）。
 */

/** 按下沉多深：1px 位移 + 竖向 4% —— 看得出"被按下去"，又不改变宽度 */
const PRESS_Y = 1
const PRESS_SCALE_Y = 0.96

/** 按下去要快 —— 慢半拍就不像被按住 */
const PRESS_MS = 90

/** 松开回位：比按下略慢一点，`ease-out` 收住，不过冲 */
const RELEASE_MS = 140

/** 每颗元素当前在播的那一段（按下 / 回位各算一段，新的一段顶掉旧的） */
const live = new WeakMap<HTMLElement, Animation>()

function reduceMotion(): boolean {
  return (
    typeof window.matchMedia === 'function' &&
    window.matchMedia('(prefers-reduced-motion: reduce)').matches
  )
}

function stop(el: HTMLElement): void {
  live.get(el)?.cancel()
  live.delete(el)
}

/** 屏上此刻的（下沉位移, 竖向缩放）：按到一半就松手时，回位要从它接着走 */
function offsetOf(el: HTMLElement): { y: number; sy: number } {
  const t = getComputedStyle(el).transform
  if (t === '' || t === 'none') return { y: 0, sy: 1 }
  try {
    const m = new DOMMatrixReadOnly(t)
    return { y: m.m42, sy: m.d }
  } catch {
    return { y: 0, sy: 1 }
  }
}

/** 按下：快速沉下去，**停在那儿**（松手前一直保持） */
export function pressDown(el: HTMLElement | null): void {
  if (el === null || reduceMotion()) return
  stop(el)
  const anim = el.animate(
    [
      { transform: 'translateY(0px) scaleY(1)' },
      { transform: `translateY(${PRESS_Y}px) scaleY(${PRESS_SCALE_Y})` },
    ],
    { duration: PRESS_MS, easing: 'ease-out', fill: 'forwards' },
  )
  live.set(el, anim)
}

/**
 * 松开 / 拖出去 / 被系统取消：从当前位置平滑回到原位。
 *
 * 幂等：没按下去（或已经回完）时调用，什么都不做 —— 三个事件各调一次很常见。
 */
export function pressUp(el: HTMLElement | null): void {
  if (el === null || reduceMotion()) return
  const pressed = live.get(el)
  const { y, sy } = offsetOf(el)
  stop(el)
  /* 没按过、屏上也是原样 —— 不值得为它起一段动画 */
  if (pressed === undefined && Math.abs(y) < 0.01 && Math.abs(sy - 1) < 0.001) return

  const anim = el.animate(
    [
      { transform: `translateY(${y}px) scaleY(${sy})` },
      { transform: 'translateY(0px) scaleY(1)' },
    ],
    { duration: RELEASE_MS, easing: 'cubic-bezier(.22,.68,.2,1)', fill: 'forwards' },
  )
  live.set(el, anim)
  void anim.finished.then(
    () => {
      if (live.get(el) !== anim) return
      live.delete(el)
      /* 撤掉 fill，把 transform 还给 CSS（终值就是原位，视觉上不会跳） */
      anim.cancel()
    },
    () => {
      /* cancel 会 reject：什么都不用做 */
    },
  )
}
