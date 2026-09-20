import type { MkpApi, MkpApiMethod } from './contract'
import { NotImplementedError } from './errors'

/**
 * 桌面壳注入的那一份。约定挂在 `window.__mkp_api` 上，逐个方法注入 ——
 * 所以类型是 `Partial<MkpApi>`：壳接了一半也能跑，没接的那几个调用时才炸。
 */
declare global {
  interface Window {
    __mkp_api?: Partial<MkpApi>
  }
}

/** 没接的口子：抛在**调用时**，不是模块加载时 —— 否则一个没接就整个应用白屏，看不出是哪个 */
function missing(method: MkpApiMethod): never {
  throw new NotImplementedError(method)
}

/**
 * IPC 桥。
 *
 * 每次调用都重新读 `window.__mkp_api` —— 壳注入的时机不一定早于前端首次渲染，
 * 模块加载那一刻取一次快照会永远停在"还没注入"。
 *
 * `?? missing(...)` 靠的是"实现了就一定返回 Promise（非 nullish）"：
 * 壳那边返回 undefined 也算没接好，照样报错，这是想要的。
 */
export const bridgeApi: MkpApi = {
  getPreset: (variantId) => window.__mkp_api?.getPreset?.(variantId) ?? missing('getPreset'),
  saveOffsets: (axes) => window.__mkp_api?.saveOffsets?.(axes) ?? missing('saveOffsets'),
  getCalibModels: () => window.__mkp_api?.getCalibModels?.() ?? missing('getCalibModels'),
  openModel: (modelId) => window.__mkp_api?.openModel?.(modelId) ?? missing('openModel'),
  getTestModels: () => window.__mkp_api?.getTestModels?.() ?? missing('getTestModels'),
}
