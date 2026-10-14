/*
 * 认领一格开发台场景（hook；拆出来只为 Fast Refresh，见 `scene.ts` 的注）。
 *
 * 持有那一格界面的组件这样用：
 *
 * ```tsx
 * const dev = useDevPreview('rename')
 * ...
 * <RenameModal open={open || dev.on} onClose={() => { setOpen(false); dev.close() }} />
 * ```
 *
 * ★★ **活口（2026-10-14 标记，整族一起说明）**：所有 `useDevPreview('<slug>')` 在
 * 产品里**恒 `false`**（`dev/context.ts` 里那个 provider 的槽位是空的），于是
 * `open={open || dev.on}` 里右边那一半、以及跟着它走的分支，**看着全是死代码** ——
 * 但它们**不是**：开发台（`npm run tauri:workbench:dev`）与探针会把场景摆进那些槽位，
 * 一摆 `dev.on` 就真、这些格子就真开。**清理死代码时别碰这些点**。
 *
 * 现有的消费点（改这一族时一起看）：`TopTabs`（update-prompt）、`PageReport`
 * （float-toast / export）、`PagePresets`（rename / compare / delete-confirm）、
 * `FileImport`（import-result / import-collision）。
 */

import { useContext } from 'react'

import { DevPreviewContext } from './context'
import { slotOf } from './scene'
import type { DevSlot } from './scene'

/**
 * @param slug 这一格的稳定 id（与 Rust `workbench/app/devsim.rs` 的 `scenes()` 里那一行**逐字一样**）
 * @returns `on` = 现在是不是该显示这一格（按**它那一类**里摆着的是不是它）；`close` = 关掉它
 *
 * ★ `on` 的判据是"**这一格所在的层类**里摆着的是不是这一格"—— 于是**同类互斥**自动成立
 * （点了「参数对比台」再点「重命名」，两类都留一个；若两者同类，后来者顶掉先来者）。
 */
export function useDevPreview(slug: string): { on: boolean; close: () => void } {
  const slot = useContext(DevPreviewContext)
  const on = slot.bySlot.get(slotOf(slug))?.scene === slug
  return { on, close: () => slot.close(slug) }
}

/** 这一格被打开的次数（认领方拿它当 effect 依赖，就能"再点一次也重开"） */
export function useDevPreviewNonce(slug: string): number {
  const slot = useContext(DevPreviewContext)
  const held = slot.bySlot.get(slotOf(slug))
  return held?.scene === slug ? held.nOnce : 0
}

/**
 * 现在摆着的那几格（**给外壳用**），按**打开先后**排（先开的在前，最后点的是最后一个）。
 *
 * 两条用途：① `App` 换页签（认领方在别的页签上时，光开框看不见）—— 取最后一个
 * （用户刚点的那个才是他想看的）；② 诊断时一眼看全"现在同时摆着什么"。
 */
export function useDevPreviewAll(): { scene: string; slot: DevSlot; nOnce: number }[] {
  const slot = useContext(DevPreviewContext)
  return [...slot.bySlot.entries()]
    .map(([s, held]) => ({ scene: held.scene, slot: s, nOnce: held.nOnce }))
    .sort((a, b) => a.nOnce - b.nOnce)
}
