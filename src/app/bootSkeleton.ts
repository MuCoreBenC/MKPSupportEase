/*
 * 首屏骨架（`index.html` 里那个 `#boot`）的**唯一收放口**（2026-10-31）。
 *
 * # 它为什么住在应用外面
 *
 * 骨架是**接线之前**就能看见的东西：窗口一出现，WebView 拿到的就是那份 HTML，
 * 不用等任何 JS / CSS Module / IPC。所以它必须写在 `index.html` 里；而它要在
 * "首页还没决定该显示什么"这段时间一直留在屏幕上，就**不能**住在 `#root` 里
 * （React 的第一次提交会把容器清空 —— 那正是"骨架 → 一个 Logo → 真界面"三段跳的由来，
 * 作者 2026-10-31：「为什么是先出的骨架屏，然后再出了一个什么 MKP 三个字？
 * 别说 MKP 了，就直接用骨架」）。
 *
 * 现在 `#boot` 是 `#root` 的**兄弟**：应用在它底下长好，长好了（或明确失败了）
 * 再由这里给它挂上 `data-done` 收掉。收掉的那一刻露出来的就是终态版面。
 *
 * # 为什么还要 `show`
 *
 * 失败态那颗「重试」会把 `attempted` 拨回未读 —— 那一刻骨架得回来（否则就是一扇
 * 空窗）。所以收放成对：就绪 / 失败 → 收；重试 → 放。
 *
 * ★ 只碰 `data-done` 这一个属性，判据与样式都在 `index.html`（`#boot[data-done]`），
 *   这里不复制一套"该怎么藏"的定义。重复调用是幂等的（StrictMode 会调两次）。
 */

const BOOT_ID = 'boot'

/** 显示首屏骨架（重试时用）—— 没有那个节点（例如引导窗口）就什么都不做 */
export function showBootSkeleton(): void {
  const el = document.getElementById(BOOT_ID)
  if (el === null) return
  delete el.dataset.done
}

/** 收掉首屏骨架。**必须在首页真正画出来之后**调（见调用处那条注释），否则会露出半成品 */
export function hideBootSkeleton(): void {
  const el = document.getElementById(BOOT_ID)
  if (el === null) return
  el.dataset.done = ''
}
