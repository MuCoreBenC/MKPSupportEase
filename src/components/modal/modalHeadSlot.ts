/*
 * 模态框**标题栏的动作槽**（2026-10-10）。
 *
 * 作者：「这个修改历史可以放到叉叉的旁边，就放到上面标题那个头部那些地方」——
 * 框里的内容要往标题栏放一颗按钮时，**用 `createPortal(node, slot)` 过去**：
 * 状态与逻辑留在原地（那颗按钮的开关、脏数据都还在它的组件里），位置借出来。
 *
 * 不在模态框里（`slot === null`）时，调用方照旧把按钮渲染在原处 ——
 * 同一份代码，两种住处。
 */

import { createContext, useContext } from 'react'

export const ModalHeadSlotContext = createContext<HTMLElement | null>(null)

/** 标题栏那颗动作槽的落点；不在框里时是 `null` */
export function useModalHeadSlot(): HTMLElement | null {
  return useContext(ModalHeadSlotContext)
}
