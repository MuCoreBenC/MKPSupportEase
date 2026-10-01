/*
 * 模态框挂哪儿：外壳把**「页面区以上、窗口内」那一层**交出来（C14 原样移植）。
 *
 * 共用件的 `Modal` 只画一块 `absolute inset: 0` 的遮罩，盖住哪儿由宿主决定。
 * C14 的需求（作者原话）：模态框要盖住导航、页面与状态栏 —— 所以由外壳把
 * `.shellBody` 这个 DOM 节点通过 context 交出来，页面用 `createPortal` 挂进去。
 *
 * 注意不是 portal 到 `document.body`：宿主必须是外壳内部的节点，
 * 挂到 body 上量出来的坐标会错（试验场第 33 号坑）。
 */

import { createContext, useContext } from 'react'

const Ctx = createContext<HTMLElement | null>(null)

export const OverlayHostCtx = Ctx.Provider

/** 拿到遮罩宿主（外壳还没挂上时是 null —— 那时不画模态框） */
export function useOverlayHost(): HTMLElement | null {
  return useContext(Ctx)
}
