/*
 * **本机文件面代次**（delivery revision）：应用里**任何一处**把本机文件写进 / 移出
 * （交付文件的下载 / 更新 / 删除 / 删归档；用户副本的另存 / 改归属）之后广播一声
 * "文件面变了"；读这些数据的页面把它放进 effect 依赖里，跨页刷新。
 *
 * 口径 2026-10-10 放宽过一次（原来只覆盖"交付文件"）：首页「复制后处理脚本」的
 * `--Toml` 把**我那一份**（`presets-mine/…`）也算进候选，而写它的人在预设页 ——
 * 不广播，首页就还拿着上一份的落点。名字与边界没变：仍是"盘上摆布变了、读的人
 * 自己重读"的一条代次，不存任何数据。
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
 *   本机文件面    deliveryMutated   首页按钮的「缺 / 漂 / 全齐」 / 预设页本地表 / BBS 交付面 /
 *                                  首页「复制后处理脚本」（`--Toml` 的候选）
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

/* ---------- 数据源代次（2026-10-08） ---------- */

let sourceRevision = 0
const sourceListeners = new Set<() => void>()

/**
 * **数据源换过了**（设置页保存成功那一刻调）。
 *
 * # 为什么单开一条代次，不并进 `deliveryMutated`
 *
 * 换源与"投递面变了"是两件事：前者换的是**去哪儿取数**，后者换的是**盘上有什么**。
 * 换源之后要做的动作也更重一层 —— 不只是重读，还要**允许后台再检查一次目录**
 * （`checkBootstrapOnce` 的"本次运行只一次"要被复位），否则用户改完地址看到的是
 * 旧源的数据，而验收标准是**保存即生效、不许要求重启**。
 *
 * 与 `deliveryMutated` 同一条边界：只广播一个代次，不存任何数据。
 */
export function sourceMutated(): void {
  sourceRevision += 1
  for (const listener of sourceListeners) listener()
}

/** 数据源代次。放进 effect 依赖里：变了就按新地址重来一遍 */
export function useSourceRevision(): number {
  return useSyncExternalStore(
    (listener) => {
      sourceListeners.add(listener)
      return () => {
        sourceListeners.delete(listener)
      }
    },
    () => sourceRevision,
  )
}
