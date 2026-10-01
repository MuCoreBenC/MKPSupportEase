import { bridgeApi } from './bridge'
import type { MkpApi } from './contract'
import { mockApi } from './mock'

export type { AppError, Axes, CalibModel, ErrorCode, MkpApi, Preset } from './contract'
export { isAppError } from './contract'
export { NotImplementedError } from './errors'

/*
 * 剩下的类型一律整份转出去（`export type *`）——
 * 「客户端接发布包」这一轮把机型 / 文件 / 参数表 / 说明书那一批类型都搬进了契约，
 * 页面 import 的是 `'../../api'` 这个入口（不直接摸 `./contract` 的文件名），
 * 在这里逐个列会因为「加一个字段就要改两处」而开始漏。
 * `export type *` 只带类型、不带值 —— 值仍然只有 `api` / `apiSource` / `isAppError`
 * / `NotImplementedError` 这几个明确出口。
 */
export type * from './contract'

/**
 * 整个应用取后端的唯一入口。
 *
 * 判据是**运行时探测**「我是不是跑在 Tauri 里」，不是构建期的 `import.meta.env.DEV`。
 * 区别很实际：`npm run tauri dev` 是 dev 构建**且**跑在原生窗口里 —— 按 DEV 判会走 mock，
 * 于是整个开发期都碰不到真实链路，等到打包那一刻才第一次接通，那时候出的问题最难查。
 *
 * `__TAURI_INTERNALS__` 是 Tauri v2 注入到 window 上的内部对象，有它就说明 `invoke` 能用。
 * 浏览器直接开 5321 时它不存在 → 走 mock，路径不丢。
 */
const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

export const api: MkpApi = inTauri ? bridgeApi : mockApi

/** 给界面用：告诉用户"这一份数据是从哪来的"。调试用，不参与业务判断 */
export const apiSource: 'rust' | 'mock' = inTauri ? 'rust' : 'mock'
