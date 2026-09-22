# Windows 标题栏窗口键：接上真实动作 + 改成撑满式

两件事绑在一起做，因为它们改的是同一个 DOM 区域（`TopTabs` 右上角那三颗键）：

1. **修 bug**：Windows 上最小化 / 最大化 / 关闭点了没反应（macOS 正常）。
2. **改样式**：把三颗小圆角按钮改成 Windows 原生那种「撑满标题栏高度、贴到窗口右上角、
   彼此无缝」的 caption button，解决「不好看」与更关键的「不好点」。

---

## 1. 根因：ACL 权限缺失，不是 CSS，也不是事件没绑

三颗键的 `onClick` 是接上的（`src/app/window.ts`），走的是 Tauri core 命令
`plugin:window|minimize` / `toggle_maximize` / `close`。而 `src-tauri/capabilities/default.json`
只授了两条：

```json
"permissions": ["core:default", "core:window:allow-start-dragging"]
```

`core:default` 里包含的 `core:window:default` **是一档只读权限**。这不是推测——
本机 `npm run tauri dev` 构建后生成的
`src-tauri/gen/schemas/acl-manifests.json` 里 `core:window.default_permission.permissions`
逐条如下（已核对，共 28 条）：

```
allow-get-all-windows, allow-scale-factor, allow-inner-position, allow-outer-position,
allow-inner-size, allow-outer-size, allow-is-fullscreen, allow-is-minimized,
allow-is-maximized, allow-is-focused, allow-is-decorated, allow-is-resizable,
allow-is-maximizable, allow-is-minimizable, allow-is-closable, allow-is-visible,
allow-is-enabled, allow-title, allow-current-monitor, allow-primary-monitor,
allow-monitor-from-point, allow-available-monitors, allow-cursor-position, allow-theme,
allow-is-always-on-top, allow-activity-name, allow-scene-identifier,
allow-internal-toggle-maximize
```

**全是 `is-*` / `get-*` 查询类，加一条 `allow-internal-toggle-maximize`
（那是 drag region 双击缩放专用，前端 API 调不到）。**
真正动手的 `allow-minimize` / `allow-maximize` / `allow-unmaximize` /
`allow-toggle-maximize` / `allow-close` 一条都不在里面。IPC 在 Rust 侧被 ACL 拒掉，
前端拿到一个 rejected promise。

**为什么 macOS 没暴露**：`TopTabs.tsx:137` 是 `{!isMac && (...)}` —— mac 上这三颗键
根本不渲染，最小化 / 缩放 / 关闭全由系统交通灯接管（`titleBarStyle: "Overlay"` +
`chrome.rs` 的空 `NSToolbar`），完全不经过 IPC。所以「mac 能用、Windows 不能用」
正好对应这个分支结构。

**为什么连报错都看不见**：`src/app/window.ts:20-22` 把异常 `catch` 成了
`console.error`。报错只落在 devtools console，界面上是纯粹的「点了没反应」。

> 讽刺的是 `default.json` 的 description 里作者已经写明了
> 「`core:default` 里不含 `allow-start-dragging`，缺了没有任何报错」——
> 同一个坑踩了两次，第二次只是因为 mac 上不走这条路径而一直没被发现。

`docs/ARCHITECTURE.md` §9 第 186-187 行还写着「那三颗是假控件、尚未接
`getCurrentWindow().minimize()`、Tauri 窗口目前带系统装饰」，这三句现在都是过期的
（`window.ts` 已接、`tauri.windows.conf.json` 已 `decorations: false`），顺手改掉。

## 2. 视觉方案：Windows 11 caption button 规格

现状（`TopTabs.module.css:183-211`）：`.ctrl` 是 `30×26` + `border-radius: 6px`，
外层 `.controls` 有 `gap: 2px`，再加 `.bar` 的 `padding-right: 12px`。
结果是三块小色片浮在 52px 高的标题栏中间，四周全是死区——
命中面积 30×26=780px²，而标题栏能给的是 46×51=2346px²，**浪费了三分之二**。
右上角那个「把鼠标甩到屏幕角落就能点到关闭」的 Fitts 定律红利也拿不到。

目标规格（对齐 Windows 11 / VS Code / Edge 的做法）：

| 项 | 现在 | 改成 |
| --- | --- | --- |
| 单键宽 | 30px | `46px`（mini 档 `40px`） |
| 单键高 | 26px | `100%`（撑满标题栏内容高，即 51px） |
| 圆角 | 6px | 0 |
| 键间距 | 2px | 0（无缝） |
| 右侧留白 | 12px | 0（贴窗口右边缘） |
| 悬停底色 | `--surface-sunken` | `rgb(16 24 32 / 6%)`，按下 `11%` |
| 关闭悬停 | `#e5484d` | `#e5484d`（保留），按下 `#cf3c41` |
| 最大化图标 | 恒为方框 | 已最大化时换「还原」双叠框 |

几个刻意的取舍：

- **红色沿用仓库的 `#e5484d`，不换成 Windows 标准的 `#c42b1c`。** 色块面积从 780px²
  涨到 2346px² 之后 `#e5484d` 确实更跳，但这是本仓库唯一的危险红，为了「更像 Windows」
  引入第二个红不值得。要换是设计决定，留给你一句话推翻。
- **撑满是「撑满内容盒」，不含 `.bar` 底部那条 1px 分隔线**（`box-sizing: border-box`
  下 `.bar` 内容高 = 52 - 1 = 51）。按钮停在细线上沿，看起来是有意为之，
  而不是把线压掉一截。
- **焦点环改成内描边**。`global.css:33-37` 给所有按钮设了
  `outline-offset: 2px`，贴边之后这个外扩 2px 会被窗口右/上边缘裁掉。
  `.ctrl:focus-visible { outline-offset: -2px }` 覆盖掉。
- **不加 Windows 11 的 Snap Layouts（悬停最大化键弹出布局菜单）。** 那需要 Rust 侧
  接 `WM_NCHITTEST` 返回 `HTMAXBUTTON`，是另一个量级的改动，本轮不做。

## 3. 拖动区不受影响（已验证逻辑，不需要改）

`.bar` 上挂着 `data-tauri-drag-region="deep"`，按钮是它的子节点。Tauri 的 drag.js
在 `mousedown` 里判的是 **`e.target` 自己**有没有这个属性（不是 `closest()`），
点在 `<button>` / 内部 `<svg>` 上时 target 不是 header，不会触发拖窗。所以
「撑满之后整条右上角变成按钮」不会把拖动吃掉，也不会让按钮变成拖把手。

顺带把 `.controls` 那个 `<div>` 上的 `data-tauri-drag-region="deep"` **去掉**：
撑满之后这个容器已被三颗按钮完全铺满，属性永远命中不到，留着只是误导
（且它会让人以为按钮区可拖）。

## 4. 受影响文件

| 文件 | 改动类型 | 具体位置 |
| --- | --- | --- |
| `g:\project\MKPSupportEase\src-tauri\capabilities\default.json` | 改 | `permissions` 数组增 5 条；description 补一段 |
| `g:\project\MKPSupportEase\src\app\components\TopTabs.module.css` | 改 | `.bar[data-platform='windows']` 新增；重写 `.controls`（183-191）与 `.ctrl` / `.close`（193-211） |
| `g:\project\MKPSupportEase\src\app\components\TopTabs.tsx` | 改 | 三颗键那段（137-149）：去掉容器上的 drag-region、最大化键按状态换图标与 aria-label |
| `g:\project\MKPSupportEase\src\app\window.ts` | 改 | 导出 `inTauri`（供新 hook 复用，避免第三份重复探测） |
| `g:\project\MKPSupportEase\src\hooks\useWindowMaximized.ts` | **新增** | 订阅窗口尺寸变化，返回 `isMaximized` |
| `g:\project\MKPSupportEase\src\components\Icon.tsx` | 改 | `IconName` 增 `'restore'`，`paths` 增一条 |
| `g:\project\MKPSupportEase\docs\ARCHITECTURE.md` | 改 | §7 新增第 ④ 条（窗口动作权限）；§9 删掉过期的「假控件」那两句 |

## 5. 实现细节

### 5.1 capabilities/default.json

```json
  "permissions": [
    "core:default",
    "core:window:allow-start-dragging",
    "core:window:allow-minimize",
    "core:window:allow-maximize",
    "core:window:allow-unmaximize",
    "core:window:allow-toggle-maximize",
    "core:window:allow-close"
  ]
```

- `allow-toggle-maximize` 是 `winToggleMaximize()` 直接要的那条；
  `allow-maximize` / `allow-unmaximize` 一起授是因为 Tauri 的 `toggleMaximize`
  在部分路径上会落到这两个命令，只授 toggle 会留下一半失效的隐患。
- **不授** `allow-destroy` / `allow-hide`：`close` 走的是正常关闭流程（会触发
  `CloseRequested`，将来要加「有未保存改动」拦截就靠它），`destroy` 是绕过拦截的暴力关闭，
  前端不该有这个能力。
- description 里把「为什么 `core:default` 不够」写清楚，措辞跟已有那段
  `allow-start-dragging` 的说明对齐——这个仓库的注释风格是「写判据和踩过的坑」，不是写它是什么。

### 5.2 TopTabs.module.css

```css
/* Windows：窗口键贴到右上角，右内衬归零 —— 撑满是为了好点，不只是为了好看 */
.bar[data-platform='windows'] {
  padding-right: 0;
}

.controls {
  --winctrl-w: 46px;

  display: flex;
  align-self: stretch;   /* 撑满标题栏内容高（52 减去底部 1px 线） */
  flex-shrink: 0;
  margin-left: auto;
}

/* 迷你档窗口窄，46 会把页签挤掉，收到 40 */
.bar[data-density='mini'] .controls {
  --winctrl-w: 40px;
}

.ctrl {
  width: var(--winctrl-w);
  height: 100%;
  display: grid;
  place-items: center;
  color: var(--text-2);
  transition: background 0.12s ease, color 0.12s ease;
}

/* 贴边之后 global.css 那个外扩 2px 的焦点环会被窗口边缘裁掉，改成内描边 */
.ctrl:focus-visible {
  outline-offset: -2px;
}

.ctrl:hover {
  background: rgb(16 24 32 / 6%);
  color: var(--text-1);
}

.ctrl:active {
  background: rgb(16 24 32 / 11%);
}

.close:hover {
  background: #e5484d;
  color: #fff;
}

.close:active {
  background: #cf3c41;
  color: #fff;
}
```

注意 `.ctrl` 不再写 `border-radius`，`.controls` 不再写 `gap` 与 `align-items`
（`align-self: stretch` + 子元素 `height: 100%` 已经够）。

### 5.3 useWindowMaximized.ts（新增）

```ts
import { useEffect, useState } from 'react'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { inTauri } from '../app/window'

/**
 * 窗口是否已最大化。只有 Windows 的自绘最大化键用得到 —— 它要在
 * 「方框」与「还原双叠框」之间换图标，跟系统上其它软件一致。
 *
 * 用 onResized 而不是轮询：最大化 / 还原 / 拖拽 Snap / 双击标题栏
 * 都会走 resize 事件，一个订阅覆盖全部入口。浏览器路径下直接返回 false。
 */
export function useWindowMaximized(): boolean {
  const [maximized, setMaximized] = useState(false)

  useEffect(() => {
    if (!inTauri) return

    const win = getCurrentWindow()
    let alive = true

    const sync = () => {
      win.isMaximized()
        .then((v) => { if (alive) setMaximized(v) })
        .catch(() => { /* 查询失败就维持上一个值，不值得打断界面 */ })
    }

    sync()
    const unlisten = win.onResized(sync)

    return () => {
      alive = false
      unlisten.then((off) => off()).catch(() => {})
    }
  }, [])

  return maximized
}
```

权限上这两个调用都已覆盖：`isMaximized` 用 `allow-is-maximized`（在
`core:window:default` 里），`onResized` 用 `core:event` 的 `allow-listen`
（已核对 `core:event.default_permission` = `allow-listen, allow-unlisten, allow-emit, allow-emit-to`），
**不需要额外授权**。

### 5.4 TopTabs.tsx

`window.ts` 顶部把 `inTauri` 改成导出：

```ts
export const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
```

组件内三颗键那段：

```tsx
const maximized = useWindowMaximized()

// ...

{!isMac && (
  <div className={s.controls}>
    <button type="button" className={s.ctrl} aria-label="最小化" onClick={winMinimize}>
      <Icon name="min" size={11} strokeWidth={1.5} />
    </button>
    <button
      type="button"
      className={s.ctrl}
      aria-label={maximized ? '还原' : '最大化'}
      onClick={winToggleMaximize}
    >
      <Icon name={maximized ? 'restore' : 'max'} size={11} strokeWidth={1.5} />
    </button>
    <button type="button" className={`${s.ctrl} ${s.close}`} aria-label="关闭" onClick={winClose}>
      <Icon name="close" size={11} strokeWidth={1.5} />
    </button>
  </div>
)}
```

图标从 14/12/14 统一收到 11px、描边从 1.7 收到 1.5：按钮面积涨了 3 倍，
图标反而要更小更细才像系统控件——Windows 的 Segoe Fluent 那三个字形就是 10px 量级。
`useWindowMaximized` 无条件调用（hook 不能进条件分支），mac 上它因为
`isMac` 分支不渲染而无人读取，浏览器路径下 `inTauri` 为假直接空转。

### 5.5 Icon.tsx

```ts
  min: 'M5 12h14',
  max: 'M6 6h12v12H6z',
  restore: 'M6 9h9v9H6zM9 9V6h9v9h-3',
  close: 'M6 6l12 12M18 6 6 18',
```

`restore` 是两个错位方框：前框 `6,9→15,18`，后框只画露出来的那条 L 形边
（上边 + 右边 + 右下那 3px），不画被前框盖住的部分——不然 11px 下两条边糊成一团。

## 6. 边界条件与异常

| 情况 | 行为 |
| --- | --- |
| 浏览器路径（`npm run dev`） | `inTauri` 为假：三颗键照常渲染并有 hover 反馈，点了空转；`useWindowMaximized` 恒 false。改样式在浏览器里就能验，不用开原生窗 |
| macOS | 整段 `!isMac` 不渲染，CSS 新规则挂在 `[data-platform='windows']` 上，mac 布局零影响 |
| Linux | `detectPlatform()` 只有两个取值，Linux 被判成 `windows`，走自绘键。新授的 5 条权限在 Linux 上同样有效，行为正确 |
| 窗口已最大化时再点 | `toggleMaximize` 走 unmaximize，`allow-unmaximize` 已授 |
| 键盘操作 | 三颗键是真 `<button>`，Tab 可达；焦点环改内描边后不被窗口边缘裁掉 |
| 权限再缺 | `window.ts` 的 `catch` 仍在，报错进 console（保留现状，不改成弹窗——启动期的窗口操作不该有 UI 噪音） |
| `prefers-reduced-motion` | `global.css:87-95` 全局关掉 transition，hover 变成瞬切，符合预期 |

## 7. 数据流

```
用户点「关闭」
  → <button onClick={winClose}>                      TopTabs.tsx
  → act('close') → getCurrentWindow().close()        src/app/window.ts
  → IPC invoke 'plugin:window|close'
  → Rust ACL 校验 capability "default" 的 permissions
      改前：命中不到 allow-close → Err → 前端 catch → console.error → 界面无反应
      改后：命中 core:window:allow-close → tao 关闭窗口
```

最大化状态回流：

```
窗口尺寸变化（点键 / 拖 Snap / 双击标题栏 / 系统快捷键 Win+↑）
  → Tauri 发 tauri://resize 事件
  → win.onResized(sync)                              useWindowMaximized.ts
  → win.isMaximized() → setMaximized(v)
  → 最大化键换 restore 图标与 aria-label
```

## 8. 预期结果与验证

改完必须跑到的几步：

1. `npm run lint` —— eslint + stylelint 全绿。Stylelint 是 `stylelint-config-standard`，
   注意现代颜色记法（本仓库用 `rgb(16 24 32 / 6%)` 这种空格+百分比写法，别写 `rgba()`）。
2. `npm run build` —— `tsc -b` 要过（新 hook 的类型、`IconName` 新成员）。
3. `npm run tauri dev` 在 Windows 原生窗口里逐项点：
   - 最小化 → 窗口进任务栏；
   - 最大化 → 铺满，图标变还原，再点还原回原尺寸；
   - 关闭 → 进程退出；
   - devtools console 里**没有** `[window] ... 失败`；
   - 三颗键之间无缝、贴到窗口右上角、鼠标甩到屏幕右上角就能命中关闭；
   - 标题栏空白处仍可拖窗，双击仍可缩放。
4. `cd src-tauri && cargo test && cargo clippy -- -D warnings` —— 本轮不改 Rust 代码，
   仅确认 capability JSON 改动没让 `tauri-build` 的 ACL 解析报错（identifier 拼错会在
   build 期直接 panic，这是最快的拼写检查）。

## 9. 本轮明确不做

- **Snap Layouts**（悬停最大化键弹 Windows 11 布局菜单）：需要 Rust 侧
  `WM_NCHITTEST` 返回 `HTMAXBUTTON`，另开一轮；
- **`plugin-os` 换掉 UA 平台探测**：`usePlatform.ts` 的注释已经把这件事记为待办，
  和本轮无关，不顺手做；
- **Windows 窗口圆角**：`chrome.rs` 的 `apply_native_corners` 在非 mac 上是空实现，
  Win11 的 `DWMWA_WINDOW_CORNER_PREFERENCE` 是独立议题；
- **`--statusbar-h` 这个死变量**（`tokens.css:49`，全仓无人引用）：与本轮无关，不清理。

## 10. git 约束

仓库有八道本地闸（`scripts/hooks/`，`core.hooksPath` 已生效）：

- 闸⑥要求分支名有 `feat/ fix/ refactor/ chore/ docs/ style/` 前缀 ——
  本轮建议 `fix/win-caption-buttons`（主体是修失效的 bug，样式是搭车）；
- 闸①禁止直接在 main 上提交，闸②禁止 push main。

**克隆下来当前就在 main 上**，所以第一个任务就是切分支，否则后面每次提交都会被闸①拦。
是否提交 / 是否开 PR 由你决定，我不会擅自 commit。
