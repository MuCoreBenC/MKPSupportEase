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
  /*
   * `null` = 这一趟还不知道用户选了哪档（app-state 还没落地）。这时**沿用上次记下的**
   * —— 不能直接按系统深浅落，那会把首帧脚本已经放好的浅色又翻成深色（作者录屏里
   * 「先是深色、再变浅色」正是这条路径）。
   */
  const dark = resolveDark(pref ?? readStoredTheme())
  document.documentElement.dataset.theme = dark ? 'dark' : 'light'
  /*
   * 记一份给**首屏骨架**用（2026-10-31 作者录屏：骨架先按系统深色画出来，读了 app-state
   * 才翻成浅色）。`index.html` 的 head 里那段内联脚本同步读这个键 —— 那是首帧唯一来得及
   * 的来源，不用等任何 IPC。
   *
   * 只在**知道用户真值**时写（`pref !== null`）：`applyTheme(null)` 是"还不知道"的兜底，
   * 拿它去覆盖记录，等于把系统档当成用户档存下来。
   */
  if (pref === null) return
  try {
    /* 存**档位**（含 `system`），不是存这一次的解析结果 —— 存结果的话，
       系统在两次启动之间换了深浅（白天/夜里），首帧就会按上一次的结果画。 */
    localStorage.setItem('mkpse.boot.theme', pref)
  } catch {
    /* 存储被禁（隐私模式）：那就让骨架按系统深浅走 */
  }
}

/** 首屏骨架读的那一份（`index.html` 的 head 脚本用同一处写入的值） */
export function readStoredTheme(): ThemePref | null {
  try {
    const v = localStorage.getItem('mkpse.boot.theme')
    return v === 'dark' || v === 'light' || v === 'system' ? v : null
  } catch {
    return null
  }
}
