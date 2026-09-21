import { useCallback, useEffect, useRef } from 'react'

import { winStartDragging, winToggleMaximize } from './window'
import type { MouseEvent as ReactMouseEvent } from 'react'

/** 超过这个距离才算「在拖窗」。不到就当普通点击 —— 手抖 1px 不该让窗口跳一下 */
const DRAG_THRESHOLD_PX = 4

/** 这些元素自己处理点击，不参与拖窗、也不触发双击缩放 */
const INTERACTIVE = 'button, a, input, select, textarea, [role="button"]'

function isInteractive(target: EventTarget | null) {
  return target instanceof Element && target.closest(INTERACTIVE) !== null
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
    if (isInteractive(e.target)) return

    // 阻止浏览器的文本选区拖拽（native drag）——不 prevent 的话，
    // 慢慢拖经过文字时 Chromium 会发起文本拖拽，光标变成禁止符号
    e.preventDefault()

    const startX = e.clientX
    const startY = e.clientY

    function cleanup() {
      window.removeEventListener('mousemove', onMove)
      window.removeEventListener('mouseup', cleanup)
      window.removeEventListener('blur', cleanup)
      cleanupRef.current = null
    }

    function onMove(ev: MouseEvent) {
      const moved = Math.abs(ev.clientX - startX) >= DRAG_THRESHOLD_PX
        || Math.abs(ev.clientY - startY) >= DRAG_THRESHOLD_PX
      if (!moved) return
      cleanup()
      void winStartDragging()
    }

    cleanupRef.current?.()
    cleanupRef.current = cleanup
    window.addEventListener('mousemove', onMove)
    window.addEventListener('mouseup', cleanup)
    // 系统接管拖窗后窗口会失焦，这是拿不到 mouseup 时的兜底
    window.addEventListener('blur', cleanup)
  }, [])

  const onDoubleClick = useCallback((e: ReactMouseEvent<HTMLElement>) => {
    if (isInteractive(e.target)) return
    void winToggleMaximize()
  }, [])

  return { onMouseDown, onDoubleClick }
}
