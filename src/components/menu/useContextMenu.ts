/*
 * 「谁被右键了、在哪打开」这一点状态（A34）。
 *
 * # 只有右键，没有 ⋯ 按钮
 *
 * 行尾再挂一个按钮等于同一件事有两个入口，而右键这条路本来就覆盖得全：
 *
 *   鼠标      contextmenu 事件
 *   触摸屏    长按，浏览器自己转成 contextmenu —— 不需要我们写 long-press
 *   没鼠标    Shift+F10 与菜单键，Windows 的系统级约定，用户不用学
 *
 * # 为什么键盘那一路要单独算坐标
 *
 * 键盘触发时 `contextmenu` 事件的 `clientX/clientY` 是 0 —— 直接用就会把菜单画到窗口左上角。
 * 所以两种情况分开：有光标用光标，没光标用**这一行自己的左下角**。
 * 判据是 `e.detail === 0`（鼠标右键的 detail 至少是 1）加一个坐标为 0 的兜底。
 *
 * # 行要能收键盘事件
 *
 * `triggerProps` 里带了 `tabIndex: 0`。没有它，Shift+F10 根本到不了这一行 ——
 * 焦点不在行上，keydown 会打到别处去。
 */

import { useCallback, useState } from 'react'
import type { KeyboardEvent, MouseEvent } from 'react'
import type { ContextMenuPoint } from './types'

export interface ContextMenuApi<T> {
  /** 被右键的那一行。null = 菜单没开 */
  target: T | null
  /** 打开的位置（视口坐标）。null = 菜单没开 */
  at: ContextMenuPoint | null
  close: () => void
  /** 摊在行元素上的属性 */
  triggerProps: (target: T) => {
    tabIndex: number
    onContextMenu: (e: MouseEvent<HTMLElement>) => void
    onKeyDown: (e: KeyboardEvent<HTMLElement>) => void
  }
}

/** 键盘打开时菜单贴在行的左下角，往里缩一点，别压住行的边 */
const KEY_INSET = 12

export function useContextMenu<T>(): ContextMenuApi<T> {
  const [state, setState] = useState<{ target: T; at: ContextMenuPoint } | null>(null)

  const close = useCallback(() => setState(null), [])

  const triggerProps = useCallback(
    (target: T) => ({
      tabIndex: 0,
      onContextMenu: (e: MouseEvent<HTMLElement>) => {
        e.preventDefault()
        /* 别让外层也弹一个 —— 嵌套列表里两个菜单同时开过一次，很难看 */
        e.stopPropagation()
        const byKeyboard = e.detail === 0 && e.clientX === 0 && e.clientY === 0
        if (byKeyboard) {
          const r = e.currentTarget.getBoundingClientRect()
          setState({ target, at: { x: r.left + KEY_INSET, y: r.bottom } })
          return
        }
        setState({ target, at: { x: e.clientX, y: e.clientY } })
      },
      onKeyDown: (e: KeyboardEvent<HTMLElement>) => {
        const wanted = (e.shiftKey && e.key === 'F10') || e.key === 'ContextMenu'
        if (!wanted) return
        e.preventDefault()
        const r = e.currentTarget.getBoundingClientRect()
        setState({ target, at: { x: r.left + KEY_INSET, y: r.bottom } })
      },
    }),
    [],
  )

  return {
    target: state?.target ?? null,
    at: state?.at ?? null,
    close,
    triggerProps,
  }
}
