import { bridgeApi } from './bridge'
import type { MkpApi } from './contract'
import { mockApi } from './mock'

export type { AppError, Axes, CalibModel, ErrorCode, MkpApi, Preset } from './contract'
export { isAppError } from './contract'

/**
 * 整个应用取后端的唯一入口。
 *
 * 判据是**运行时探测**「我是不是跑在 Tauri 里」，不是构建期的 `import.meta.env.DEV`。
 * 区别很实际：`npm run tauri dev` 是 dev 构建**且**跑在原生窗口里 —— 按 DEV 判会走 mock，
 * 于是整个开发期都碰不到真实链路，等到打包那一刻才第一次接通，那时候出的问题最难查。
 *
 * `__TAURI_INTERNALS__` 是 Tauri v2 注入到 window 上的内部对象，有它就说明 `invoke` 能用。
 * 浏览器直接开 5178 时它不存在 → 走 mock，路径不丢。
 */
const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

export const api: MkpApi = inTauri ? bridgeApi : mockApi

/** 给界面用：告诉用户"这一份数据是从哪来的"。调试用，不参与业务判断 */
export const apiSource: 'rust' | 'mock' = inTauri ? 'rust' : 'mock'
