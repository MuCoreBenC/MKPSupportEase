# Windows 窗口交互三修：光标、双击缩放、边缘拖拽命中区

- [x] Task 1: 标题栏光标改成箭头
    - 1.1: `TopTabs.module.css` 的 `.bar` 把 `cursor: grab` 改为 `cursor: default`
    - 1.2: 更新那条注释——原注释说的「空白处抓手」不再成立

- [x] Task 2: window.ts 补拖窗与改大小两个动作
    - 2.1: `act()` 的联合类型加入 `'startDragging'`，导出 `winStartDragging`
    - 2.2: 新增 `winStartResize(dir: ResizeDirection)`，单独 try/catch（带参，不走 `act`）

- [x] Task 3: 新增 useTitlebarDrag，接管拖动与双击
    - 3.1: 创建 `src/app/useTitlebarDrag.ts`，定义 `DRAG_THRESHOLD_PX = 4`
    - 3.2: 实现 `onMouseDown`：左键 + 非交互元素才记起点，挂 mousemove/mouseup/blur
    - 3.3: 移动超过阈值才 `winStartDragging()`，并立即卸监听
    - 3.4: 实现 `onDoubleClick`：非交互元素时 `winToggleMaximize()`
    - 3.5: `useEffect` 卸载时清理残留监听（`cleanupRef`）

- [x] Task 4: TopTabs.tsx 换用自己的拖动逻辑
    - 4.1: 删掉全部 6 处 `data-tauri-drag-region="deep"`
    - 4.2: 引入 `useTitlebarDrag`，在 `<header>` 上挂 `onMouseDown` / `onDoubleClick`
    - 4.3: 更新窗口键那段注释——原注释解释的是 drag-region 的命中规则，已过期

- [x] Task 5: 新增 ResizeEdges 组件
    - 5.1: 创建 `src/app/components/ResizeEdges.module.css`，8 条命中条定位 + 光标，`z-index: 90`
    - 5.2: 创建 `src/app/components/ResizeEdges.tsx`，遍历 8 个方向渲染
    - 5.3: `mousedown` 时 `preventDefault` 并调 `winStartResize(dir)`

- [x] Task 6: 调整 z-index 层级并挂载 ResizeEdges
    - 6.1: `TopTabs.module.css` 的 `.controls` z-index 从 61 提到 91（压过命中条与模态遮罩）
    - 6.2: `App.tsx` 引入 `inTauri` 与 `ResizeEdges`，在 `.shell` 内 `<main>` 之后条件渲染

- [x] Task 7: 补 ACL 权限
    - 7.1: `capabilities/default.json` 增加 `core:window:allow-start-resize-dragging`
    - 7.2: 改写 description 里关于 `allow-start-dragging` 的说明（不再是给 drag-region 用的）

- [x] Task 8: 验证——lint + build
    - 8.1: 运行 `npm run lint`
    - 8.2: 运行 `npm run build`
    - 8.3: 确认 tauri dev 的 Rust 侧因 capability 变更自动重编译成功

- [x] Task 9: 更新 ARCHITECTURE.md §7 第 ③ 条
    - 9.1: 整条重写为「为什么自己接管标题栏拖动」，并补上 resize 命中区那圈的说明
