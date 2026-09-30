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
 *   图标跟着换小房子。路由 id 仍是 `machine`（不改 id：改它会连带动 `App.tsx` 的落地页与
 *   `TopTabs` 的图标表，而这个 id 只在这两处出现，没有第二个消费者）。
 * - 设置换真齿轮：原来那条 `settings` 图标是小圆 + 长辐条，迷你档看着像太阳。
 *
 * 「客户端接发布包」这一轮加了两条 —— 顺序照试验场 A40 的顶栏：
 * - **同步**（`sync`）：说明书自动同步 + 「获取预设」。原来叫「数据包」，那是「让我手动下载
 *   JSON」的语义，而说明书是自动同步的（T7.3 把这一页改名的理由）。
 * - **BBS 预设**（`bbs`）：Bambu Studio 工艺预设查看器。它有两个入口，另一个是预设页右键
 *   菜单里那一项（跨页带目标文件名，见 `App.tsx` 的 `pendingBbs`）。
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
  { id: 'sync', label: '同步' },
  { id: 'bbs', label: 'BBS 预设' },
  { id: 'report', label: '报告' },
  { id: 'settings', label: '设置', icon: 'gear' },
]
