import type { IconName } from '../../components/Icon'

/**
 * 主页签 —— 界面结构数据，属于应用本身，不属于 API mock。
 *
 * 试验场里这份表活在 `src/mock/mkpFull.ts` 的 `tabs`，再由 v023 的 `AppV023.tsx` 做两处覆盖。
 * 那边不敢直接改 mock：同一份 `tabs` 被 #5…#21 全部老稿引用，改了老稿的标题栏就跟着变、
 * 回头对比稿子不算数。产品仓只有一个界面，所以覆盖直接落到这张表里，不再分两层。
 *
 * 保留的两处覆盖：
 * - 第一页叫「首页」不叫「机型」：它已经是五步向导的入口与总览，"机型"只是第一步要选的东西；
 *   图标跟着换小房子。路由 id 仍是 `machine`。
 * - 设置换真齿轮：原来那条 `settings` 图标是小圆 + 长辐条，迷你档看着像太阳。
 */
export interface Tab {
  id: string
  label: string
  /** 迷你档用的图标；不传则由 TopTabs 按 id 查它自己那张表 */
  icon?: IconName
}

export const tabs: Tab[] = [
  { id: 'machine', label: '首页', icon: 'home' },
  { id: 'preset', label: '预设' },
  { id: 'calib', label: '校准' },
  { id: 'params', label: '参数' },
  { id: 'report', label: '报告' },
  { id: 'settings', label: '设置', icon: 'gear' },
]
