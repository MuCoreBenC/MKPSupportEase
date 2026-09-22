# Summary: Windows 窗口交互三修

## 完成的改动

### 1. 标题栏光标：grab → default
`.bar` 的 `cursor: grab` 改为 `cursor: default`。系统上其它软件标题栏都是箭头，`grab` 语义是"这块内容可以被拖走"，不适用于窗口本体。

### 2. 双击最大化：不再依赖 Tauri 的 drag.js
根因是 `drag.js` 在 Windows 上第一次 `mousedown` 就立刻发 `start_dragging`，系统进入模态拖窗循环吃掉 `mouseup`，Chromium 连击计数被打断。

新增 `useTitlebarDrag` hook，删掉全部 6 处 `data-tauri-drag-region="deep"`：
- `mousedown` 只记起点并挂临时监听
- 移动超过 4px 才发 `startDragging()`
- 没移动就松手 → 点击序列完整闭合 → 原生 `dblclick` 正常触发
- 双击间隔等于系统 `GetDoubleClickTime()`（默认 500ms）

### 3. 边缘 resize 命中区
新增 `ResizeEdges` 组件：8 条透明命中条贴在 `.shell` 内侧（边 5px、角 12px），`mousedown` 调 `startResizeDragging(dir)`。加上系统外侧 8px，抓取带约 13px 跨在可见边上。仅 Windows + Tauri 下渲染。

## 改动文件清单

| 文件 | 改动 |
| --- | --- |
| `src/app/components/TopTabs.module.css` | `cursor: default`；`.controls` z-index 61→91 |
| `src/app/window.ts` | 新增 `winStartDragging`、`winStartResize`、`ResizeDirection` 类型 |
| `src/app/useTitlebarDrag.ts` | **新增**，拖动/双击事件逻辑 |
| `src/app/components/TopTabs.tsx` | 删 6 处 drag-region，挂 useTitlebarDrag |
| `src/app/components/ResizeEdges.tsx` | **新增**，8 向 resize 命中条 |
| `src/app/components/ResizeEdges.module.css` | **新增**，定位与光标 |
| `src/app/App.tsx` | Windows + Tauri 下渲染 ResizeEdges |
| `src-tauri/capabilities/default.json` | 增 `allow-start-resize-dragging`，改写 description |
| `docs/ARCHITECTURE.md` | §7 第 ③ 条整条重写 |

## 验证结果

- `npm run lint`：eslint + stylelint 全绿（ResizeEdges CSS 首次因单行多声明报错，已改为多行格式）
- `npm run build`：tsc + vite build 通过（103 modules，849ms）
- `ResizeDirection` 类型：`@tauri-apps/api` 2.11 未导出，自己定义了一份
