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
 * - strictPort: true —— 没有它时 5178 被占就静默跳 5179、5180…，每跑一次 dev 就多一个
 *   没人知道的 vite 进程。**报 `Port 5178 is in use` 是它在正常工作**：说明已经有服务在跑，
 *   直接用 http://localhost:5178/。Tauri 的 devUrl 写死这个端口，顺延了就连不上。
 * - open: false —— Tauri 会开自己的原生窗口，再开一个浏览器标签是多余的。
 * - host —— 只在 Tauri 需要时绑网卡（`TAURI_DEV_HOST` 由 tauri dev 在真机调试时注入），
 *   平时不绑，避免把 dev server 暴露给同网段。这与试验场的 `host: true` 是刻意的差别。
 */
export default defineConfig({
  plugins: [react()],

  server: {
    host: process.env.TAURI_DEV_HOST ?? false,
    port: 5178,
    strictPort: true,
    open: false,
  },

  // Tauri 自己的输出已经够吵，Vite 不要再清屏把它的报错冲掉
  clearScreen: false,
})
