import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'

import './styles/tokens.css'
import './styles/global.css'
import App from './app/App'

/*
 * 入口只做一件事：把 App 挂上去。
 *
 * 试验场的入口挂的是预览器外壳（窗口尺寸预设 + 稿号下拉 + 报告开关），那一层不搬 ——
 * 真窗口由 Tauri 管，稿号在产品里不存在。
 */
const host = document.getElementById('root')
if (!host) throw new Error('index.html 里没有 #root')

createRoot(host).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
