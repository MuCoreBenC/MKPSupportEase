import { rmSync } from 'node:fs'
import { resolve } from 'node:path'
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { bbsFs } from './tools/dev-server/bbsFs.mjs'

/*
 * 已经接进 catalog、走「下载管道」的资产，**不再随客户端产物分发**。
 *
 * 以前 `public/` 里有什么就进什么包 —— 于是"开发仓库里有这份文件"变成了
 * "用户安装包里也带一份"。第三圈起它们归 catalog 管（带 SHA / 大小、按需下载进 `mkp/`），
 * 包里那份副本就成了第二个真源：**盘上哪份是对的，从此有两个答案。**
 *
 * 这里只摘**已经接进管道的那几类**（接一类加一行），没接的（整机图那一类 UI 装饰）
 * 暂时还在包里 —— 等它们也接进来，这份清单就是空的时候，`public/` 那整个目录
 * 也可以搬走。判据 1（`scripts/check-bundle.mjs`）盯着这件事：清单里有的却出现在产物里就红。
 */
const DELIVERED_ASSET_DIRS = ['bbs']

/** 从客户端构建产物里摘掉上面那几类资产的副本。**工作台构建不摘** —— 那是开发态工具 */
function dropDeliveredAssets(enabled: boolean) {
  return {
    name: 'drop-delivered-assets',
    apply: 'build' as const,
    closeBundle() {
      if (!enabled) return
      for (const dir of DELIVERED_ASSET_DIRS) {
        rmSync(resolve('dist/assets', dir), { recursive: true, force: true })
      }
    },
  }
}

/*
 * 两个插件：react() 与 bbsFs()。
 *
 * `bbsFs()` 是 BBS 预设页的数据源 —— `GET /api/bbs/presets` 实时读本机 BBS 目录
 * （本仓不打包那 285 个预设快照，见 C15-A40-PORT-PLAN §6-2）。它**只读**、只在 serve 期
 * 存在（dev 与 preview 都挂，验收走的是 preview），`vite build` 的产物里没有它。
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
 *
 *   工作台模式（workbench）另起 5322：两份 frontend 同源不同页，若共用 5321，
 *   只要工作台的 vite 还在跑，`npm run tauri dev` 就必然撞上 `Port 5321 is in use`。
 *   分开后两边可同时开，改 5322 同样要同步 src-tauri/tauri.workbench.conf.json 的 devUrl。
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
    plugins: [react(), bbsFs(), dropDeliveredAssets(!withWorkbench)],

    build: {
      rollupOptions: { input },
    },

    server: {
      host: process.env.TAURI_DEV_HOST ?? false,
      port: withWorkbench ? 5322 : 5321,
      strictPort: true,
      open: false,
    },

    // Tauri 自己的输出已经够吵，Vite 不要再清屏把它的报错冲掉
    clearScreen: false,
  }
})
