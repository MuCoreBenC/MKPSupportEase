/*
 * 模态框**标题栏的两个借位槽**（2026-10-10 动作槽；2026-10-17 回执槽）。
 *
 * 作者：「这个修改历史可以放到叉叉的旁边，就放到上面标题那个头部那些地方」——
 * 框里的内容要往标题栏放东西时，**用 `createPortal(node, slot)` 过来**：
 * 状态与逻辑留在原地（那颗按钮的开关、脏数据都还在它的组件里），位置借出来。
 *
 * 不在模态框里（`slot === null`）时，调用方照旧把东西渲染在原处 ——
 * 同一份代码，两种住处。
 *
 * # 两个槽，别混用
 *
 *   `ModalHeadSlotContext`      动作槽 = **按钮**（修改历史、回到已应用…），排在 `×` 左边
 *   `ModalHeadNoteSlotContext`  回执槽 = **一句状态**（「已写回 xxx.toml」），排在副标题右边
 *
 * 回执槽是作者 2026-10-17 要的：「这个我想换一个位置，在模态框的框架上面可以吗」——
 * 那句保存回执原本是内容区顶上的一行绿字，每出现一次就把参数列表往下顶一行；
 * 搬到标题栏之后，副标题（文件名）旁边贴着读，下面的版面一动不动。
 * ★ 它**左对齐、不长腿**：右侧动作簇是 `margin-left: auto`，回执出现 / 消失都推不动它们。
 */

import { createContext, useContext } from 'react'

/** 标题栏那颗**动作槽**的落点；不在框里时是 `null` */
export const ModalHeadSlotContext = createContext<HTMLElement | null>(null)

/** 标题栏那颗**回执槽**的落点（副标题右边）；不在框里时是 `null` */
export const ModalHeadNoteSlotContext = createContext<HTMLElement | null>(null)

/** 动作槽的落点；不在框里时是 `null` */
export function useModalHeadSlot(): HTMLElement | null {
  return useContext(ModalHeadSlotContext)
}

/** 回执槽的落点；不在框里时是 `null` */
export function useModalHeadNoteSlot(): HTMLElement | null {
  return useContext(ModalHeadNoteSlotContext)
}
