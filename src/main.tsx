import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'

import './styles/tokens.css'
import './styles/global.css'

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

const isGuide = new URLSearchParams(window.location.search).get('guide') === '1'

/*
 * ★★ 引导窗口必须是**真透明**的（2026-10-15）：`global.css` 给 `body` 铺了一层不透明的
 * 渐变 —— 那是**主窗口**的底。引导窗口跟主窗口共用同一份样式，于是整扇窗也被铺满，
 * 卡片旁边那支白箭头就"没有了"。判据挂到 `html` 上，由 `global.css` 里那条
 * `html[data-guide] body { background: none }` 收口。
 */
if (isGuide) document.documentElement.dataset.guide = '1'

void (async () => {
  if (isGuide) {
    const { default: GuideCard } = await import('./app/guide/GuideCard')
    createRoot(host).render(<GuideCard />)
    return
  }
  const { default: App } = await import('./app/App')
  createRoot(host).render(
    <StrictMode>
      <App />
    </StrictMode>,
  )
})()
