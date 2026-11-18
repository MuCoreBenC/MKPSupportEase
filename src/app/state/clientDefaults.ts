/*
 * 前端那两格偏好的**出厂档**（2026-11-18）。
 *
 * # 为什么要有这一层
 *
 * 切片器（`STORAGE.clientSlicer`）与「保存参数后的重新切片提示」（`STORAGE.clientShowGuides`）
 * 住在 webview 的 localStorage：壳读不到它们，出厂值也就不能由后端直接解。
 * 所以启动时问一次后端（`api.getClientDefaults`，读的是
 * `presets/client-defaults.toml` 的 `[Client]` 段），之后"没设过"按它兜底 ——
 * **判据只有那一份 toml**，这里不留第二份默认值。
 *
 * # 为什么是同步读
 *
 * `useSyncedStickyState` 在首次渲染那一刻就要一个初值。真机上 `main.tsx` 在挂 React
 * 之前已经 `await` 过那一次 IPC 并 `seed` 进来，所以页面拿到的就是 toml 那一档。
 * 下面那个常量只在"IPC 都还没通"的一小段里顶事（浏览器 mock 里后端回的就是它）。
 */

import type { ClientDefaultsDto } from '../../api/contract'

/** 还没问到后端时顶一会儿的值（与 `[Client]` 段的出厂档同值） */
const BEFORE_IPC: ClientDefaultsDto = { slicer: 'system', guideShow: true }

let current: ClientDefaultsDto = BEFORE_IPC

/** 启动时把后端递来的出厂档放进来（`main.tsx` 调一次，之后只读） */
export function seedClientDefaults(defaults: ClientDefaultsDto): void {
  current = defaults
}

/** 现在的出厂档。读者是 `app/shared/slicer.ts` 与 `app/guide/guidesPref.ts`。 */
export function clientDefaults(): ClientDefaultsDto {
  return current
}
