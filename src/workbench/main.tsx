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
import { WorkbenchApp } from './App'

const host = document.getElementById('workbench-root')
if (!host) {
  throw new Error('找不到 #workbench-root —— workbench.html 与本文件对不上了')
}

ReactDOM.createRoot(host).render(
  <React.StrictMode>
    <WorkbenchApp />
  </React.StrictMode>,
)
