/*
 * 右侧「详情」栏的宽度状态（BBS 自己那份参数）。
 *
 * 与左抽屉同一条路：拖宽那套逻辑是全仓共享的 `shared/useDrawerWidth`
 * （参数页的修改历史抽屉、BBS 抽屉都是这一套：拖宽 + 三态吸附 + 键盘 + 记住宽度）——
 * 这里只剩**这一栏自己的参数**。作者 2026-10-13：「左侧那些右侧拖动的宽度……
 * 我的项目其他页面也在用，不要自己复用」（= 别另起一份，用共用的那一套）。
 *
 * 不同的只有两处：贴右缘（`side: 'right'`，往左拖变宽）、默认 292。
 */

import { STORAGE } from '../../api/storageKeys'
import { useDrawerWidth } from '../shared/useDrawerWidth'
import type { DrawerWidth } from '../shared/useDrawerWidth'

export const RAIL_MIN = 240
export const RAIL_MAX = 560
/** 默认宽度，也是吸附点（与左抽屉的 280 不同：这一栏装的是"键 + 值"两列） */
export const RAIL_SNAP = 292

export type BbsRail = DrawerWidth

export function useBbsRail(): BbsRail {
  return useDrawerWidth({
    storageKey: STORAGE.clientBbsRailW,
    side: 'right',
    snap: RAIL_SNAP,
    min: RAIL_MIN,
    max: RAIL_MAX,
    label: '详情栏',
  })
}
