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
import { getCurrentWindow } from '@tauri-apps/api/window'

import './tokens.css'
import './workbench.css'
import './c14tokens.css'
import { WorkbenchApp } from './App'

/*
 * 测试桩：**只有显式要求才装**（`?mock=1`），这是 2026-10-02 定的口径。
 *
 * 以前是「dev 且没有 Tauri 就自动装」—— 那让"浏览器里打开工作台"看起来能跑，
 * 实际上读的是一份手写夹具，**与真盘上的 workbench/bootstrap.json 毫无关系**：
 * 在终端改了配置文件，界面照旧显示夹具里的旧值，人会以为"改没生效"。
 * 现在默认**一条路**：真机没有 Tauri 就如实报错，不再拿夹具冒充后端。
 *
 * 桩不是产品运行时的能力，是**探针的测试后端**（`scripts/probes/chain.mjs` 等）：
 * 它们跑在 vite preview + 浏览器里，本来就该显式说"我需要桩"。
 *   · `tauri:workbench:dev` / 生产构建 → **永不装**（这段被 `import.meta.env.DEV` 摇掉）
 *   · `vite preview` + `?mock=1`      → 装（只服务探针）
 */
const wantsMock =
  import.meta.env.DEV && new URLSearchParams(window.location.search).get('mock') === '1'
if (wantsMock) {
  const { installMockBackend } = await import('./dev/mockBackend')
  installMockBackend()
}

const host = document.getElementById('workbench-root')
if (!host) {
  throw new Error('找不到 #workbench-root —— workbench.html 与本文件对不上了')
}

/* 全局 error / unhandledrejection 不再接管 #workbench-root：
   ResizeObserver 的良性回路通知（loop completed with undelivered notifications）
   也会以 error 事件的形式冒上来，一接管就把整棵树刷成一行白字。
   真正的渲染错误由下面的 Boundary 兜，启动失败由 App 的 fail() 横幅兜。 */

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

/*
 * ★ **窗口这时还是藏着的**（Rust 侧 `.visible(false)`，见 `workbench::open_window`）——
 * 等两帧过去（React 把首帧画上），再让它露面：作者 2026-10-07 定的是"画面跟窗口
 * 一起出现"，不要"先给一块白板、再等一分钟"。
 *
 * 两帧的原因：React 18 的 render 是调度着提交的，跳过第一帧再 show 最稳 ——
 * 差一帧人看不出来。Rust 侧还挂着 60 秒兜底：这段代码因任何原因没走到，
 * 窗口照样会出现（宁可白板，不能永不露面）。
 *
 * 浏览器里（探针跑 vite preview 时）没有 Tauri，跳过。
 */
if ('__TAURI_INTERNALS__' in window) {
  requestAnimationFrame(() => {
    requestAnimationFrame(() => {
      getCurrentWindow()
        .show()
        .catch(() => {
          /* show 失败没有补救余地（Rust 侧兜底定时器还在），不打断入口 */
        })
    })
  })
}
