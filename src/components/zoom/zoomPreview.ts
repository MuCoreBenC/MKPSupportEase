/*
 * 全站图片放大预览的「页面事务」（2026-11-25）。
 *
 * 放大层一律 portal 到 `document.body`，而页面切走只是 `PageSlot` 给一层 `hidden`
 * （组件仍挂着）—— 不统一收的话，报告页点开的板子图会跟着切到设置页，停在屏幕上
 * （作者截图：「切换到其他页面的时候，它得消失」）。
 *
 * 这里只有一份登记表：放大层打开时登记、卸载时注销；外壳每切一次页就叫它们全部收起。
 * 新的放大预览接同一个 hook 就自动归这套事务管，不必各自再判一次当前页签到没到。
 */
import { useEffect, useRef } from 'react'

/** 还开着的放大预览 → 各自的收起动作 */
const open = new Set<() => void>()

function register(close: () => void): () => void {
  open.add(close)
  return () => open.delete(close)
}

/**
 * 外壳切页时调用：把所有还开着的放大预览收掉。
 *
 * 只是"叫它们收"，不负责卸 React 树 —— 各层自己的 `onGone` 会做那一步。
 */
export function closeAllZoomPreviews(): void {
  for (const close of [...open]) close()
}

/**
 * 让一层放大预览跟着页面走。
 *
 * `close` 每次渲染都可能是新闭包，用 ref 接住；登记只装一次。
 * 切页时它应当**立即**收（不走 200ms 倒放）：旧页面已经不在了，那段倒放只会留在
 * 新页面上播。
 */
export function useCloseOnPageChange(close: () => void): void {
  const closeRef = useRef(close)
  closeRef.current = close
  useEffect(() => register(() => closeRef.current()), [])
}
