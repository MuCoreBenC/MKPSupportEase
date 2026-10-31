import { useCallback, useEffect, useRef } from 'react'

import { winStartDragging, winToggleMaximize } from './window'
import type { MouseEvent as ReactMouseEvent } from 'react'

/** 超过这个距离才算「在拖窗」。不到就当普通点击 —— 手抖 1px 不该让窗口跳一下 */
const DRAG_THRESHOLD_PX = 4

/**
 * 标题栏里两类元素要分开看（2026-10-31）。
 *
 * ★★★ 原来是**一条** `INTERACTIVE` 把 `button, a` 也整块排除掉了 —— 于是页签、
 *   `MKP` 字标、更新胶囊（全是 `<button>`）上按下根本不进这段逻辑，只有它们之间的
 *   空隙能拖窗。作者：「有一些有内容的地方我拖动不了，只有空的地方才能拖动，
 *   我不是很喜欢」—— 这就是那条选择器。
 *
 * 现在：
 *   · **文本输入**（`TEXT_ENTRY`）——整块让出去：在输入框里按下就是要定位光标 / 选字，
 *     跟着窗口一起动没法用。带 `data-no-drag` 的自绘控件（如 Windows 那三颗窗控键）
 *     也走这一档，一点都不能拖。
 *   · **能点的控件**（`CLICKABLE`）—— 不再整块让出去，改走与空白区同一套延迟判定：
 *     **没动就是点击，动过阈值才是拖窗**。所以按钮既能点、也能按着拖窗口。
 */
const TEXT_ENTRY = 'input, textarea, select, [contenteditable]:not([contenteditable="false"]), [data-no-drag]'
const CLICKABLE = 'button, a, [role="button"]'

function hits(target: EventTarget | null, selector: string) {
  return target instanceof Element && target.closest(selector) !== null
}

/**
 * 这一次按下已经变成「拖窗」了 ⇒ 随后的那次 click **不许再算点击**。
 *
 * 光靠"系统接管后收不到 mouseup"是不够的：macOS 上拖完窗 mouseup / click 照样
 * 会到，Chromium 照常派发 click —— 表现就是"窗口动了，但页签也被切走了"。
 * 所以在这个 window 的捕获阶段拦一次：它比 React 挂在 root 上的委托更早，
 * `stopPropagation` 之后组件上的 `onClick` 不会跑。
 *
 * ★ 什么时候收（**别再用固定超时**：上一版从"开始拖"起数 500ms 就摘，而拖窗口
 *   经常超过半秒，超时一摘、松手那个迟到的 click 就把页签切走了 —— 作者实测正是
 *   这一条「窗口动了，但页签也被切走了」）。三条兜底，都不依赖时间常数：
 *     ① 吃掉一次 click 就收；
 *     ② `mouseup` 到了 ⇒ 很快就有 click（同一次交互），留 300ms 再收 —— 宁可多守一会儿，
 *        也别抢在 click 前把它摘掉；
 *     ③ 系统连 mouseup 都吞了 ⇒ 由**下一次鼠标按下**收，绝不误吞后来的正常点击。
 *
 * 三条都不依赖"拖了多久"，所以拖十秒也一样。
 */
function swallowNextClick() {
  function disarm() {
    window.removeEventListener('click', kill, true)
    window.removeEventListener('mouseup', onUp, true)
    window.removeEventListener('mousedown', disarm, true)
  }
  function kill(ev: MouseEvent) {
    ev.preventDefault()
    ev.stopImmediatePropagation()
    disarm()
  }
  function onUp() {
    window.setTimeout(disarm, 300)
  }
  window.addEventListener('click', kill, true)
  window.addEventListener('mouseup', onUp, true)
  window.addEventListener('mousedown', disarm, true)
}

/**
 * 拖窗期间给 `<body>` 挂个记号（2026-10-31 作者：「拖动的之后，就不是说去点它」）。
 *
 * 系统接管拖窗之后 webview 收不到 mouseup，按钮的 `:active` 会一直挂着 ——
 * 不撤的话拖完窗口 `MKP` 字标还停在 `scale(0.96)` 的"被按住"样子。
 * 样式侧的唯一消费点在 `components/TopTabs.module.css`（`.logo:active` 那条）。
 *
 * 摘标记的时机：正常松手（`cleanup`）、以及下一次在标题栏按下 ——
 * 拖窗被系统吞掉事件时，残留的只是个 `data-` 属性，不影响任何交互。
 */
function markDragging(on: boolean) {
  if (on) document.body.dataset.windowDragging = 'true'
  else delete document.body.dataset.windowDragging
}

/**
 * 标题栏的拖动与双击缩放。
 *
 * **为什么不用 `data-tauri-drag-region`**：Tauri 注入的 drag.js 在第一次 mousedown
 * （`detail === 1`）就立刻发 `start_dragging`，Windows 侧随即进入模态拖窗循环，
 * WebView2 收不到这次的 mouseup —— Chromium 眼里这次点击没闭合，连击计数被打断，
 * 第二次 mousedown 往往又是 `detail === 1`。表现就是「双击最大化要点得极快才灵」。
 * 它还 `stopImmediatePropagation()`，所以连自己挂 onDoubleClick 都收不到事件。
 * （mac 上 drag.js 走的是另一条分支：`detail === 2` 时直接 return、改在 mouseup 判，
 * 所以那边一直是好的。）
 *
 * 这里的做法是把 `startDragging` **推迟到指针真的移动之后**：
 * 按下不动再松手就什么 IPC 都不发，点击序列完整闭合，原生 dblclick 正常触发。
 * 双击间隔因此等于系统的双击速度（Chromium 在 Windows 上取 `GetDoubleClickTime()`），
 * 也就是和其它软件一致，而不是我们自己定一个数。
 */
export function useTitlebarDrag() {
  /* 拖窗交给系统后 mouseup 可能收不到，组件卸载时还得能把监听摘干净 */
  const cleanupRef = useRef<(() => void) | null>(null)

  useEffect(() => () => cleanupRef.current?.(), [])

  const onMouseDown = useCallback((e: ReactMouseEvent<HTMLElement>) => {
    if (e.button !== 0) return
    if (hits(e.target, TEXT_ENTRY)) return

    /* 上一次拖窗留下的记号先摘掉，这一下是正常按下，按压观感该照常给 */
    markDragging(false)

    /*
     * **一律**阻止浏览器的默认按下行为（2026-10-31 第二轮）。
     *
     * 不 prevent 的话：① 慢慢拖经过文字时 Chromium 会发起文本拖拽，光标变成禁止符号；
     * ② WKWebView 在按钮上按住时会进它自己的"按住"内部状态（可能连后续 mouse move
     *    都收不到）—— 上一版正是放过了 `button`，结果"页签上还是拖不动"。
     *
     * `preventDefault` 不影响 click 派发（`swallowNextClick` 那条才管"拖动不算点击"），
     * 也不影响 `:active` 的按压观感；代价只是鼠标按下不再给按钮聚焦，
     * 键盘 Tab 仍然可达，这一条在标题栏里可以接受。
     */
    e.preventDefault()

    const startX = e.clientX
    const startY = e.clientY

    function cleanup() {
      window.removeEventListener('mousemove', onMove)
      window.removeEventListener('mouseup', cleanup)
      window.removeEventListener('blur', cleanup)
      cleanupRef.current = null
      markDragging(false)
    }

    function onMove(ev: MouseEvent) {
      const moved = Math.abs(ev.clientX - startX) >= DRAG_THRESHOLD_PX
        || Math.abs(ev.clientY - startY) >= DRAG_THRESHOLD_PX
      if (!moved) return
      cleanup()
      /* 已经是在拖窗，不再像"按了它一下"：撤按压观感 + 吞掉这次的 click */
      markDragging(true)
      swallowNextClick()
      void winStartDragging()
    }

    cleanupRef.current?.()
    cleanupRef.current = cleanup
    window.addEventListener('mousemove', onMove)
    window.addEventListener('mouseup', cleanup)
    // 系统接管拖窗后窗口会失焦，这是拿不到 mouseup 时的兜底
    window.addEventListener('blur', cleanup)
  }, [])

  /* 双击缩放仍只认"空白处"：双击一枚按钮在系统里也不该放大窗口 */
  const onDoubleClick = useCallback((e: ReactMouseEvent<HTMLElement>) => {
    if (hits(e.target, TEXT_ENTRY) || hits(e.target, CLICKABLE)) return
    void winToggleMaximize()
  }, [])

  return { onMouseDown, onDoubleClick }
}
