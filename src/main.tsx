import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'

import './styles/tokens.css'
import './styles/global.css'

import { applyTheme, readStoredTheme } from './app/state/theme'

/*
 * 入口只做一件事：把该挂的那一棵挂上去。
 *
 * 试验场的入口挂的是预览器外壳（窗口尺寸预设 + 稿号下拉 + 报告开关），那一层不搬 ——
 * 真窗口由 Tauri 管，稿号在产品里不存在。
 *
 * ★ **引导窗口走的是同一个 index.html**（2026-10-15）：那一扇窗是 Rust 侧现开的
 * （`ipc::guide::guide_show`），URL 上带 `?guide=1`。这里按它分流：
 *
 * - 引导窗口只挂 `GuideCard`（一张小卡片），**不把整个 App 拖起来** —— 它是跟着
 *   用户"保存了参数"那一瞬间冒出来的，慢一拍就指不到东西了（动态 import 就是为这个）；
 * - 主窗口照旧挂 App，行为与以前一字不差。
 */
const host = document.getElementById('root')
if (!host) throw new Error('index.html 里没有 #root')

/*
 * ★★ 正式构建里**关掉 WebView 自带的右键菜单**（2026-10-31 作者）。
 *
 * 作者：「正式编译打包发布之后……只有我自己自定义的右键菜单出现，其他的都不出现」。
 * 截图里那一份（Open Image in New Window / Download Image / Copy Image / Copy Subject /
 * Look Up / Share / Inspect Element）是 **macOS 原生菜单**，由 WKWebView 自己弹的，
 * 不是我们画的 —— 只能在 `contextmenu` 上压住默认行为。
 *
 * 判据只有一条：**开发构建放行**（作者要留着 Inspect Element 调试），正式构建拦。
 * 监听放在 capture：页面里任何元素自己的 handler 之前就压住，不必等冒泡到底。
 *
 * ★ 只 `preventDefault`、**不 `stopPropagation`**：自定义菜单（`useContextMenu` 的
 *   `onContextMenu`，住在行上、冒泡阶段）还要靠这一下才能开。
 */
if (import.meta.env.PROD) {
  window.addEventListener('contextmenu', (e) => e.preventDefault(), true)
}

const isGuide = new URLSearchParams(window.location.search).get('guide') === '1'

/*
 * ★★ 引导窗口必须是**真透明**的（2026-10-15）：`global.css` 给 `body` 铺了一层不透明的
 * 渐变 —— 那是**主窗口**的底。引导窗口跟主窗口共用同一份样式，于是整扇窗也被铺满，
 * 卡片旁边那支白箭头就"没有了"。判据挂到 `html` 上，由 `global.css` 里那条
 * `html[data-guide] body { background: none }` 收口。
 */
if (isGuide) document.documentElement.dataset.guide = '1'

/*
 * ★★ 首帧主题（2026-10-28，作者截图：深色系统下"刚打开先白一下"）。
 *
 * `appState` 的 theme 格要等一次 IPC，App 自己还要先 `await import()` —— 这之前
 * `<html>` 上没有 `data-theme`，`:root` 是浅色 token，于是骨架 / 顶栏已经是深色，
 * 底色却是浅的（`index.html` 那具 boot 骨架只认系统深浅，React 一挂上来就换成
 * `global.css` 的 token 底）。
 *
 * 先按**系统**落一次，能把这段白帧抹掉；显式选「浅色 / 深色」的用户会在这里先按
 * 系统走一帧、随后被 appState 覆盖 —— 一帧偏差比整扇窗白闪好。判据仍只有
 * `theme.ts` 的 `resolveDark` 那一份，这里不另写。
 */
applyTheme(readStoredTheme())

void (async () => {
  if (isGuide) {
    /*
     * ★★ **引导窗口得自己落一次 `data-theme`**（2026-10-15）：这一扇窗**不跑 appState**
     * （下面那行注释说了为什么），所以主窗口那条"读 AppState 的 theme 格 → 落
     * `<html data-theme>`"在这儿**根本不会发生**。结果是那张卡片只认**系统**深浅：
     * 用户在设置里选了「深色」而 macOS 还是浅色时，卡片是白的（与主窗口对不上）。
     *
     * ★ 解析那一句不在这儿重写一遍 —— 取 `state/theme` 里那唯一一份（同一句判据
     * 两个窗口共用；各写一遍就是两个不同源的判据，迟早对不上）。
     */
    const [{ api }, { applyTheme }] = await Promise.all([
      import('./api'),
      import('./app/state/theme'),
    ])
    /* 读不到（mock / 首次）就按「跟随系统」—— `null` 正是那一档 */
    applyTheme(await api.getTheme().catch(() => null))

    const { default: GuideCard } = await import('./app/guide/GuideCard')
    createRoot(host).render(<GuideCard />)
    return
  }
  /*
   * ★★ 首帧的**语言与主题**在挂 React 之前就问好（2026-10-31）。
   *
   * 这一段等待**用户看不见**：屏幕上这会儿是 `index.html` 那具骨架，不是业务界面。
   * 换来的是一整类中间态消失 —— 原来 React 一挂上来就按**系统语言**出字
   * （`i18n` 的 `active = resolveLocale(null)`），要等 `appState` 那次 IPC 回来
   * 才换成用户选的那一档：用户把界面语言设成日文、系统却是中文时，启动会先闪一帧中文。
   * 主题也是同一件事（`appState` 的 theme 格同样要一次 IPC）。
   *
   * 判据**不在这里重写**：语言交给 `i18n.resolveLocale`，深浅交给 `state/theme`
   * 的 `applyTheme` —— 与 AppState 落地时调的是同一对函数，只是提前调用一次。
   * 上面那行 `applyTheme(null)` 保留：它兜的是"连 IPC 都还没通"的那一小段（先按系统落一档）。
   *
   * 三个 import 与 `App` 并行发起（App 本来就依赖它们，不会多下载一份）。
   */
  const [{ default: App }, { api }, { applyTheme }, i18n] = await Promise.all([
    import('./app/App'),
    import('./api'),
    import('./app/state/theme'),
    import('./i18n'),
  ])
  const [themePref, localePref, active] = await Promise.all([
    api.getTheme().catch(() => null),
    api.getLocale().catch(() => null),
    api.getActivePreset().catch(() => null),
  ])
  applyTheme(themePref)
  i18n.setActiveLocale(i18n.resolveLocale(localePref))

  /*
   * 骨架中间那台机器按**正在用的机型**换（2026-10-31）。
   *
   * 原来骨架只描了 A1 一份，选了 A1 mini / P1S 的用户刷新时看到的还是 A1 的剪影
   * （作者：「我有的它是 mini 的，但是刷新骨架是这样的，其他的都变成了 A1」）。
   * 五份轮廓都写在 `index.html` 里（各按机型一张，带自己的 viewBox），这里只负责
   * 把机型名写到 `#boot` 上，让那几条 `#boot[data-machine='…']` 的规则挑中一张。
   *
   * 读不到（首次安装 / mock）就**不写**：那种情况骨架停在默认那张 A1 上 —— 不猜。
   */
  const boot = document.getElementById('boot')
  if (boot !== null && active !== null) {
    boot.dataset.machine = active.machineId
  }

  createRoot(host).render(
    <StrictMode>
      <App />
    </StrictMode>,
  )
})()
