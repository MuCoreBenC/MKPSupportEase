/*
 * 开发台的 context（拆出来只为 Fast Refresh：`preview.tsx` 只导出组件）。
 */

import { createContext } from 'react'

import type { Slot } from './scene'

/**
 * 没有 Provider 时的兜底（恒"一类都没摆"）—— 让认领方不必判空。
 *
 * 这一条也是"真机零影响"的保证：正式包里没有谁会 `emit('dev-preview')`，
 * 于是每一类都是空的，每个 `useDevPreview('<x>')` 都恒 `false`。
 */
export const EMPTY: Slot = { bySlot: new Map(), close: () => undefined }

export const DevPreviewContext = createContext<Slot>(EMPTY)
