import { bridgeApi } from './bridge'
import type { MkpApi } from './contract'
import { mockApi } from './mock'

export type { Axes, CalibModel, MkpApi, Preset, TestModel } from './contract'
export { NotImplementedError } from './errors'

/**
 * 整个应用取后端的唯一入口。
 *
 * dev 走 mock，build 走桌面壳注入的那一份。撤掉 mock 的方式就是构建 ——
 * 壳没接的接口会在被调用的那一刻抛 NotImplementedError 并在控制台点名，
 * 界面崩了也不兜：这一轮要的就是"哪个口子没接"看得见，而不是一个装作正常的空页面。
 */
export const api: MkpApi = import.meta.env.DEV ? mockApi : bridgeApi
