import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { mkpAssets } from './tools/assets/plugin.mjs'

/*
 * 随包资产的路由：**没有别名**（2026-10-03 第二刀撤掉了 `@client-assets`）。
 *
 * 客户端不再 import 交付根 —— 它按台账（`catalog.assets[]` 的 id → `path`）拼
 * `/assets/<path>` 去取，装配步骤把同一棵树铺进 `dist/assets/`（`tools/assets/plugin.mjs`）。
 * 别名是"把生成物当模块引"的那条老路，随显式 import 表一起退场：
 * 留着它只会让下一个人以为客户端还认识具体文件。
 *
 * 这一条也顺带解掉了"生成目录不存在时 `tsc -b` 会不会红"那类问题：
 * 客户端源码不再指向任何生成物。
 */

/*
 * 两个插件：react() 与随包资产流水线 mkpAssets()。
 *
 * ★ 2026-10-13：原来这里还挂着 `bbsFs()`（`/api/bbs/presets` + `/api/bbs/preset`，
 *   只在 serve 期实时读本机 BBS 目录）。**它退役了** —— 工艺页那个「本机工艺」档改走
 *   IPC（`src-tauri/src/ipc/bbs.rs`：打包后的桌面版没有 dev server，只有 IPC 一条路），
 *   浏览器预览那一边由 mock 给一份演示集合（`src/api/mockServer/bbsLocal.ts`）。
 *   试验场那份还留着 —— 那边的预览壳没有 IPC 可走，它仍然是那边唯一的数据源。
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
 * - host —— **平时明确绑 IPv6 回环 `::1`**（`TAURI_DEV_HOST` 由 tauri dev 在真机调试时
 *   注入，那时仍以注入值优先）。
 *
 *   ★★ 2026-10-19：原来写的是 `false`（= `localhost`）。Node 在 Windows 上把 `localhost`
 *   解析成 `::1`，实际也是绑在 ::1；但 **WebView2 的解析器与 Node 不是同一套** ——
 *   devUrl 写 `http://localhost:5321` 时，WebView2 可能先走 `127.0.0.1`，而 `127.0.0.1`
 *   与 `::1` 是两套独立的回环栈（本机实测：未监听的那一侧，连接要白等 **2 秒**才回落）。
 *   首屏是几百个模块请求，每一次都可能吃这一下 —— 白屏时长主要就是这么叠出来的，
 *   而且只在 Windows 上明显（Mac 的回环拒绝是即时的，所以那边"不用等"）。
 *   ⇒ 两处都写死 `::1`（vite 绑它、devUrl 用 `http://[::1]:5321`），不再猜。
 *   浏览器手动打开 `http://localhost:5321` 仍然可用（本机 localhost 就解析到 ::1）。
 *
 *   `::1` 是回环地址，不占网卡，不会暴露给同网段。
 *
 * ---- 第二个入口：工作台（B03）----
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
    plugins: [react(), mkpAssets({ workbench: withWorkbench })],

    build: {
      rollupOptions: { input },
    },

    server: {
      host: process.env.TAURI_DEV_HOST ?? '::1',
      port: withWorkbench ? 5322 : 5321,
      strictPort: true,
      open: false,

      /*
       * ★★ **构建产物不进 watcher**（2026-10-19，Windows 白屏那一刀）。
       *
       * Vite 默认只忽略 `node_modules` 与 `.git`，**不认 cargo 的产物目录**：仓库根的
       * `target/`（workspace 产物，本机几万个文件）与 `target-workbench/` 整棵都在
       * watch 里。Windows 上 chokidar 是逐目录建句柄（macOS 那边是 FSEvents 整树，
       * 开销小得多），实测一个空转的 dev server 句柄能到 **6 万+**、CPU 打满；
       * `GET /` 要等 40~100 秒才回，首屏的模块 transform 全排在它后面 ——
       * 这就是"Windows 白屏比 Mac 久得多"的根因。
       *
       * ★ 正则里**目录本身也要匹配**：chokidar 拿 `…\target`（末尾没有分隔符）来问，
       *   只写 `[\\/]target[\\/]` 会判 false、照样 readdir 进去。末尾的 `($|[\\/])`
       *   就是干这个的。实测加上之后句柄降到 ~1900，`GET /` 从 40s+ 变成 59ms。
       *
       * 这些目录都**不由前端源码消费**：cargo 产物有自己的文件监视（Tauri CLI），
       * `dist` 只在 build 时产生，`client-assets` 由 `mkpAssets` 插件自己盯源目录同步。
       * 忽略它们不会少热更新；不忽略才会。
       */
      watch: {
        ignored: [
          /[\\/](node_modules|\.git|target|target-workbench|dist|dist-presets|client-assets|tmp-shots)($|[\\/])/,
        ],
      },

      /*
       * ★★ **不要全量预热**（2026-10-19 拿掉的；这段历史留着，别再加回来）。
       *
       * 2026-10-07 曾在这里挂 `warmup.clientFiles`，把 `src/` 下所有 ts/tsx/css 全量
       * 预热；动机是"先起 vite → cargo 编译几十秒 → 才开窗口"，让预热藏进编译那段。
       *
       * 那个前提在**增量编译**下不成立：本机实测 `cargo` 增量 0.62 秒就结束，窗口
       * 打开时预热刚起步。预热会把 transform 线程池占满，首屏请求排在几百个模块后面 ——
       * 同一台机器上实测 `GET /` 要等 **43~71 秒**（warmup 跑完才轮到它），比不预热更糟。
       * 于是：
       *   · 首屏的观感交给 `index.html` 里那具内联骨架（窗口一出现就有内容）；
       *   · 首屏的模块量交给 `App.tsx` 的懒加载（五页不再进首屏）；
       *   · 其余模块按需编译 —— 浏览器自己会并发请求，只编真正用到的那些。
       *
       * 真要做"预热"也只该列首屏那十几个入口文件，且**不能**覆盖整棵 `src/`；
       * 维护一张随页面漂移的手工表不值得，先不加。
       */
    },

    // Tauri 自己的输出已经够吵，Vite 不要再清屏把它的报错冲掉
    clearScreen: false,
  }
})
