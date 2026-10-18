/*
 * 「云端出了新版、这台机器上还是旧的」——**判据**（配套的那一块"怎么办"在
 * `StaleCatalogHelp.tsx`）。
 *
 * 2026-10-19（作者：「你显示给用户的，不只是那个错误标题，你要给一个解决办法」）：
 * 内容区那块"怎么办"**按码出现**，不按文案猜 —— 同一句"内容对不上"有两种成因，
 * 只有"目录过期"能靠重启解决；另一种（文件真坏）重启没用，指错路比不指更糟。
 *
 * 与 `NoteBar` 那套一样（`note.ts` 出判据、`.tsx` 出件）：常量与纯函数住这一份，
 * 于是那个 `.tsx` 只导出组件（eslint `react-refresh` 那条）。
 */

import type { UiError } from '../../api/errors'

/**
 * **目录过期的码**。与 Rust `runtime/delivery::sha_mismatch_code` 的那个字面量
 * **逐字一致**，改一处就得改另一处。
 */
export const STALE_CATALOG_CODE = 'E_DELIVERY_STALE_001'

/** 这条错是不是"目录过期" —— 是才给"重启"这条路；不是就别乱指路。 */
export function isStaleCatalogError(e: UiError | null): boolean {
  return e !== null && e.code === STALE_CATALOG_CODE
}
