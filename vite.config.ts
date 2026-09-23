import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

/*
 * 只有 react() 一个插件。
 *
 * 试验场那份还挂着 calibFs() 与 curvesFs() 两个 dev-server 插件 —— 它们提供
 * `/__calib/write`、`/__curves/write` 两个**无鉴权的写盘端点**，是标定工作台与
 * 调参面板的后端。工作台留在试验场，这两个端点绝不进产品仓。
 *
 * server 的三条也都是刻意的，别"顺手修掉"：
 * - strictPort: true —— 没有它时 5321 被占就静默跳 5322、5323…，每跑一次 dev 就多一个
 *   没人知道的 vite 进程。**报 `Port 5321 is in use` 是它在正常工作**：说明已经有服务在跑，
 *   直接用 http://localhost:5321/。Tauri 的 devUrl 写死这个端口，顺延了就连不上。
 *
 * - port: 5321 —— 刻意避开 5173~5180（Vite 默认及其顺延区，同机其它项目大概率占着）
 *   和 5432（PostgreSQL 默认）。改这个值必须同步改 src-tauri/tauri.conf.json 的 devUrl，
 *   两处不一致 = Tauri 窗口白屏。
 * - open: false —— Tauri 会开自己的原生窗口，再开一个浏览器标签是多余的。
 * - host —— 只在 Tauri 需要时绑网卡（`TAURI_DEV_HOST` 由 tauri dev 在真机调试时注入），
 *   平时不绑，避免把 dev server 暴露给同网段。这与试验场的 `host: true` 是刻意的差别。
 *
 * ---- 第二个入口：后厨工作台（B03）----
 *
 * `workbench.html` **只在工作台构建里进 input**。默认 `npm run build` 的产物里不存在这一页，
 * 所以给用户的包里找不到它 —— 这不是"藏起来"，是没编进去，与 Rust 侧的
 * `#[cfg(feature = "workbench")]` 是两道独立的闸。
 *
 * 开关看两处，任一成立即开：
 * - `--mode workbench`（跨平台，npm script 用的是这个）
 * - `BUILD_WORKBENCH=1`（doc 里写的形式，手动跑命令时方便；Windows 的 cmd/PowerShell
 *   没法在命令前置赋值环境变量，所以不能只靠它）
 */
export default defineConfig(({ mode }) => {
  const withWorkbench = mode === 'workbench' || process.env.BUILD_WORKBENCH === '1'

  /* 先只放客户端那一页，再按开关补第二页。
     写成"两个字面量取其一"的三元式会让 TS 推出带 `workbench?: undefined` 的联合类型，
     和 rollup 的 `Record<string, string>` 对不上 —— 这里用逐步补键，不是啰嗦 */
  const input: Record<string, string> = { index: 'index.html' }
  if (withWorkbench) {
    input.workbench = 'workbench.html'
  }

  return {
    plugins: [react()],

    build: {
      rollupOptions: { input },
    },

    server: {
      host: process.env.TAURI_DEV_HOST ?? false,
      port: 5321,
      strictPort: true,
      open: false,
    },

    // Tauri 自己的输出已经够吵，Vite 不要再清屏把它的报错冲掉
    clearScreen: false,
  }
})
