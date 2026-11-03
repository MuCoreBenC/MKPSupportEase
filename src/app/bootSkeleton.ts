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
 * # 为什么要一个最短停留时间（2026-11-03）
 *
 * 作者：「我希望加载完就是正常显示的，骨架屏时间可以久一点」「不稳定触发的开屏跳动」。
 *
 * 收骨架的时刻由**图片解码完**（`PageHome` 的 `heroReady`）决定，几百毫秒到一两秒
 * 不等 —— 快的时候骨架只露 300ms 上下，一闪而过，反而比多等一会儿更像"抖了一下"。
 * 所以这里压一个**下限**：不到 `MIN_VISIBLE_MS` 就先把这次收掉记下来，到点再收。
 * 慢的时候（真等了几秒）这个下限不起作用，收的时机与以前逐字一样。
 *
 * ★ 只碰 `data-done` 这一个属性，判据与样式都在 `index.html`（`#boot[data-done]`），
 *   这里不复制一套"该怎么藏"的定义。重复调用是幂等的（StrictMode 会调两次）。
 */

const BOOT_ID = 'boot'

/**
 * 骨架至少露这么久（毫秒）。取 600 是为了一眼看清"这是什么窗口"又不至于等：
 * 冷启动真正要等的那些 IPC 通常就在这几百毫秒里跑完，视觉上停在骨架而不是闪一下。
 */
const MIN_VISIBLE_MS = 600

/** 骨架出现的时刻（模块加载那一刻它已经在屏幕上了，见文件头） */
let shownAt = performance.now()

/** 待执行的"到点再收"（重复调用先清掉，不叠定时器） */
let timer = 0

/** 最近一次请求是"收"还是"放" —— 定时器到点时用它确认没被后来的 `show` 反悔 */
let wantHide = false

function node(): HTMLElement | null {
  return document.getElementById(BOOT_ID)
}

/** 显示首屏骨架（重试时用）—— 没有那个节点（例如引导窗口）就什么都不做 */
export function showBootSkeleton(): void {
  const el = node()
  if (el === null) return
  wantHide = false
  window.clearTimeout(timer)
  timer = 0
  shownAt = performance.now()
  delete el.dataset.done
}

/** 收掉首屏骨架。**必须在首页真正画出来之后**调（见调用处那条注释），否则会露出半成品 */
export function hideBootSkeleton(): void {
  const el = node()
  if (el === null) return
  wantHide = true
  window.clearTimeout(timer)
  const left = MIN_VISIBLE_MS - (performance.now() - shownAt)
  if (left <= 0) {
    el.dataset.done = ''
    timer = 0
    return
  }
  timer = window.setTimeout(() => {
    timer = 0
    const target = node()
    if (target === null || !wantHide) return
    target.dataset.done = ''
  }, left)
}
