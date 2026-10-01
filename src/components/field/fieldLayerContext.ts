/*
 * 浮层层的 context（#33）。
 *
 * 单独一个文件的理由很小但很实在：`FieldLayer.tsx` 如果同时导出组件和 hook，
 * Vite 的快速刷新就失效（react-refresh 要求一个文件只导出组件）。
 * 「为什么这个 hook 不在组件旁边」的答案就是这一句。
 */

import { createContext, useContext } from 'react'

export interface FieldLayerApi {
  /** 浮层挂载的宿主节点。首帧是 null（ref 还没落地），浮层这时先不画 */
  el: HTMLElement | null
  /** 登记一个已打开的浮层，顺手关掉上一个。返回注销函数 */
  open: (close: () => void) => () => void
}

const EMPTY: FieldLayerApi = { el: null, open: () => () => undefined }

export const FieldLayerCtx = createContext<FieldLayerApi>(EMPTY)

export function useFieldLayer(): FieldLayerApi {
  return useContext(FieldLayerCtx)
}
