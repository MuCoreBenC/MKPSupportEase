/*
 * 主题档位 →「现在到底是不是深色」。**唯一的解析处**。
 *
 * # 为什么单独一个文件（2026-10-15）
 *
 * 这句话有**两个读者**，而它们住在**两个不同的窗口**里：
 *
 * - **主窗口**：`state/appState.ts` 读 AppState 的 theme 格 + 这里的解析 ⇒ 落
 *   `<html data-theme>`，全站 token 跟着翻；
 * - ★ **引导窗口**（`?guide=1`）：它**不跑 appState** —— 见 `main.tsx`，那扇窗是
 *   切片器旁边现开的一张小卡片，把整个状态链拖起来就慢一拍了。所以它得**自己落一次**
 *   `data-theme`；不落的话那张卡片只认**系统**深浅：用户在设置里选了「深色」而
 *   macOS 是浅色时，卡片会是白的（2026-10-15 作者报的就是这一档漏）。
 * - ★ **工作台窗口**（`workbench.html`，2026-10-16 做深色时补上）：同样**不跑 appState**
 *   （它与客户端是两棵不相干的树），同样得自己落一次 —— 见 `src/workbench/theme.ts`。
 *   它有自己的设置页「外观」，那一档存在**工作台自己那棵数据根**里。
 *
 * 三处各写一遍解析 = 三个不同源的判据，迟早对不上 ⇒ 收在这里，谁要谁来取。
 *
 * # 规则只有一条
 *
 * 「跟随系统」由 `prefers-color-scheme` 解析 —— **那是 webview 的真值**，
 * 程序不猜操作系统此刻是深是浅（`theme = 'system'` 只记"用户选了跟随"）。
 */

import type { ThemePref } from '../../api/contract'

/** 系统此刻是不是深色（webview 的真值）。 */
export const prefersDark = window.matchMedia('(prefers-color-scheme: dark)')

/** 档位 + 系统深浅 ⇒ 到底是不是深色。`null` = 用户还没选过 ⇒ 按「跟随系统」。 */
export function resolveDark(pref: ThemePref | null): boolean {
  return pref === 'dark' || (pref !== 'light' && prefersDark.matches)
}

/**
 * 把解析结果落到 **`<html data-theme>`** —— 全站 token 只认这一个属性。
 *
 * ★ 挂在 `<html>` 上而不是某个组件：① 得盖住挂到 `body` 上的东西（遮罩 / 浮层）；
 * ② `body` 自己的底色在 `styles/global.css` 里就吃 token，只有 `<html>` 能一次盖住。
 */
export function applyTheme(pref: ThemePref | null): void {
  document.documentElement.dataset.theme = resolveDark(pref) ? 'dark' : 'light'
}
