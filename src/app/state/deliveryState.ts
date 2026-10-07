/*
 * **投递面代次**（delivery revision）：应用里**任何一处**把交付文件写进 / 移出本机
 * （下载 / 更新 / 删除 / 删归档）之后广播一声"投递面变了"；读这些数据的页面把它
 * 放进 effect 依赖里，跨页刷新。
 *
 * # 为什么要有它（2026-10-07 真机连续两刀挣来的）
 *
 * 外壳的页签 2026-10-05 起是「常驻 + 切显示」（`src/app/App.tsx`）——页面**不重挂载**，
 * 只在首次进入读一次数据；而写投递面的动作却发生在**别的页面**里：
 *
 *   预设页「下载 / 更新 / 删除」   → 首页那张「下载并应用」按钮的状态会陈旧；
 *   首页「下载并应用」            → 预设页的本地表会陈旧（第一刀，已修）；
 *   （BBS 页的「已交付」同一回事。）
 *
 * 第一刀拿「使用中指针」（AppState）当信号 —— 它只覆盖"下载并应用"这一类（必写
 * active），**覆盖不了"只下载不应用"**（预设页的下载不写 active）。这一刀换成直说：
 * **写完投递面就 `deliveryMutated()`**，读的页面自己重读。
 *
 * # 它不是什么（边界，别长歪）
 *
 * - **不是状态容器**：只有一个自增代次，一个投递数据都不存 —— 数据仍由各页面按需读
 *   （`getDownloadedFiles` 等），这里只回答"要不要重读"；
 * - **不是全局状态入口**：那是 AppState（`docs/APP-STATE.md`）的方向 —— 将来收的时候
 *   把这个代次并进去，消费者不变；
 * - **丢一次通知 = 页面回到陈旧数据**，所以**凡写投递面处，写完就发**（成功与否都发：
 *   批量里成了一半也是变了）。
 *
 * # 与 AppState 的分工
 *
 *   谁在变       发什么            典型消费者
 *   使用中指针    appStateMutated  横幅 / 首页按钮的「已应用」那一态 / 预设页的应用列
 *   投递面       deliveryMutated   首页按钮的「缺 / 漂 / 全齐」 / 预设页本地表 / BBS 交付面
 */

import { useSyncExternalStore } from 'react'

let revision = 0
const listeners = new Set<() => void>()

/**
 * 投递面变了 —— 下载 / 更新 / 删除 / 删归档把盘写完（或成了一半）之后调。
 * 同步广播：订阅页面的 effect 从依赖里收到新代次、自己重读。
 */
export function deliveryMutated(): void {
  revision += 1
  for (const listener of listeners) listener()
}

/** 投递面代次。放进 effect 依赖里：变了就重读你自己那一路 */
export function useDeliveryRevision(): number {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener)
      return () => {
        listeners.delete(listener)
      }
    },
    () => revision,
  )
}
