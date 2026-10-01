/*
 * BBS 抽屉的宽度状态（BBS 自己那份参数）。
 *
 * 拖宽那套逻辑是全仓共享的 `shared/useDrawerWidth`（参数页的修改历史抽屉同一套手感：
 * 拖宽 + 三态吸附 + 键盘 + 记住宽度），这里只剩**BBS 自己的参数** —— 贴左缘、默认 280、
 * 键名沿用老键（老用户拖过的宽度不作废）。行为与对外形状（BbsDrawer）和提取前一致。
 */

import { STORAGE } from '../../api/storageKeys'
import { useDrawerWidth } from '../shared/useDrawerWidth'
import type { DrawerWidth } from '../shared/useDrawerWidth'

export const DRAWER_MIN = 200
export const DRAWER_MAX = 520
/** 默认宽度，也是吸附点 */
export const DRAWER_SNAP = 280

export type BbsDrawer = DrawerWidth

export function useBbsDrawer(): BbsDrawer {
  return useDrawerWidth({
    storageKey: STORAGE.clientBbsDrawerW,
    side: 'left',
    snap: DRAWER_SNAP,
    min: DRAWER_MIN,
    max: DRAWER_MAX,
    label: '预设列表',
  })
}