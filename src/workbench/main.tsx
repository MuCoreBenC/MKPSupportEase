/**
 * 工作台前端入口。
 *
 * 与客户端的 src/main.tsx 是**两个互不相干的入口**：不共用 App、不共用 store、
 * 不共用全局样式（工作台是密集表格界面，客户端是展示界面，两套视觉诉求不一样）。
 * 唯一共用的是 src/styles/tokens.css 里的色值与间距变量 —— 那是设计系统，不是业务。
 *
 * 挂载点 id 是 `workbench-root` 而不是 `root`：两份 html 各自独立，撞名只会在
 * 排查问题时制造"我到底在看哪个页面"的困惑。
 */
import React from 'react'
import ReactDOM from 'react-dom/client'

import './tokens.css'
import './workbench.css'
import './c14tokens.css'
import { WorkbenchApp } from './App'

/* 开发桩：浏览器里没有 Tauri IPC 时装上（见 dev/mockBackend.ts 的文件头）。
   真机 / 生产构建不装 —— import.meta.env.DEV 为 false 时这段连同模块都被摇掉 */
if (import.meta.env.DEV && !('__TAURI_INTERNALS__' in window)) {
  const { installMockBackend } = await import('./dev/mockBackend')
  installMockBackend()
}

const host = document.getElementById('workbench-root')
if (!host) {
  throw new Error('找不到 #workbench-root —— workbench.html 与本文件对不上了')
}

/* TEMP-DEBUG: 启动错误直接写到页面上（排查完删掉） */
window.addEventListener('error', (e) => {
  host.textContent = `BOOT ERROR: ${e.message}\n${String(e.error?.stack ?? '').slice(0, 2400)}`
})
window.addEventListener('unhandledrejection', (e) => {
  host.textContent = `BOOT REJECTION: ${String(e.reason).slice(0, 600)}`
})

class Boundary extends React.Component<{ children: React.ReactNode }, never> {
  componentDidCatch(error: unknown, info: { componentStack?: string }) {
    const host = document.getElementById('workbench-root')
    if (host) {
      const div = document.createElement('pre')
      div.textContent =
        'BOUNDARY: ' + String(error) + '\n' + (info.componentStack ?? '')
      host.appendChild(div)
    }
  }
  render() {
    return this.props.children
  }
}

ReactDOM.createRoot(host).render(
  <React.StrictMode>
    <Boundary>
      <WorkbenchApp />
    </Boundary>
  </React.StrictMode>,
)
