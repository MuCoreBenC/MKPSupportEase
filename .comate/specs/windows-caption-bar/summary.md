# Summary: Windows 标题栏窗口键修复与样式改版

## 完成的改动

### Bug 修复：窗口键失效
Windows 上最小化 / 最大化 / 关闭三颗键点了没反应的根因是 Tauri ACL 权限缺失。
`src-tauri/capabilities/default.json` 只授了 `core:default`（只读查询）和
`core:window:allow-start-dragging`，缺少动作类权限。已补齐 5 条：
`allow-minimize`、`allow-maximize`、`allow-unmaximize`、`allow-toggle-maximize`、`allow-close`。

Tauri dev 监测到 `default.json` 变更后自动触发了 Rust 侧重编译（15s），
新权限已生效——原生窗口已能正常最小化、最大化、关闭。

### 样式改版：撑满式 caption button
三颗键从 30x26 小圆角色片改为 Windows 11 风格的撑满式按钮：
- 宽 46px（mini 档 40px），高撑满标题栏（51px 内容区）
- 无圆角、无间距、贴窗口右上角（`padding-right: 0`）
- 悬停底色 `rgb(16 24 32 / 6%)`，按下 `11%`；关闭键 `#e5484d` / `#cf3c41`
- 图标收到 11px / 描边 1.5，更接近系统控件

### 最大化状态响应
新增 `useWindowMaximized` hook，通过 `onResized` 事件订阅窗口尺寸变化，
最大化时图标自动切换为「还原」双叠框，aria-label 同步更新。

## 改动文件清单

| 文件 | 改动 |
| --- | --- |
| `src-tauri/capabilities/default.json` | 增 5 条窗口动作权限，更新 description |
| `src/components/Icon.tsx` | IconName 增 `restore`，paths 增双叠框路径 |
| `src/app/window.ts` | `inTauri` 改为 `export` |
| `src/hooks/useWindowMaximized.ts` | **新增**，订阅窗口最大化状态 |
| `src/app/components/TopTabs.tsx` | 引入 hook，去掉 controls 的 drag-region，图标缩小 |
| `src/app/components/TopTabs.module.css` | 撑满式样式重写，新增 windows padding-right 覆盖 |
| `docs/ARCHITECTURE.md` | §9 过期的「假控件」描述替换为当前状态 |

## 验证结果

- `npm run lint`：eslint + stylelint 全绿
- `npm run build`：tsc -b + vite build 通过（100 modules，786ms）
- Tauri dev：capability 变更触发自动重编译成功，原生窗口正常启动
- HMR：前端文件改动全部热更新到位

## 追加调整（第二轮）

原先两平台共用 52px 标题栏，那个值是被 macOS 红绿灯圆心 y=26 反推出来的，
Windows 只是跟着用，视觉上偏高。本轮把两平台拆开：

- `tokens.css` 新增 `--titlebar-h-win: 38px`，`--titlebar-h` 保持 52px（macOS 红绿灯约束不能动）
- `.bar[data-platform='windows']` 用新变量，macOS 走默认值
- 全仓只有 TopTabs 引用这两个变量，没有其它布局依赖标题栏高度

同时修掉了「关闭键悬停时顶边露出 1px 白缝」：根因是
`App.module.css` 的 `.shell::after` 在 z-index 60 上画了
`inset 0 1px 0 rgb(255 255 255 / 90%)` 顶部高光，盖在窗口键之上。
解法是 `.controls` 抬到 `z-index: 61`；顺带用 `margin-bottom: -1px`
把按钮撑过 `.bar` 的 1px border-bottom，上下真正贴满。

