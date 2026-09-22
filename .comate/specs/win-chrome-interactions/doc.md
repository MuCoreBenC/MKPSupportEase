# Windows 窗口交互三修：光标、双击缩放、边缘拖拽命中区

三件事都属于「自绘窗口框架（`decorations: false`）没补齐系统原本提供的行为」，
放同一轮做。它们互相有耦合：修双击必须接管拖动，接管拖动就要自己决定光标。

---

## 1. 标题栏光标是小手

`TopTabs.module.css:14` 写了 `cursor: grab`，注释说这是刻意的
（「空白处抓手、按钮处小手」）。但系统上其它软件的标题栏是**普通箭头**——
Windows 资源管理器、VS Code、Edge、Chrome 全是 `default`。
`grab` 是「这块内容可以被我拖走」的语义（画布、卡片排序），
标题栏是「窗口本体」，不是内容。

改成 `cursor: default`。注意不能只是删掉这一行：`.bar` 不写的话会继承，
而 `body` 没设 cursor 时浏览器默认是 `auto`，在文本上会变成 I 形光标——
标题栏有「MKP」和页签文字，`auto` 会在字上变 I。所以要显式写 `default`。

## 2. 双击最大化极难触发

**这不是阈值问题，是事件被吞了。** 根因在 Tauri 自己注入的
`drag.js`（已逐行读过 `tauri-2.11.6/src/window/scripts/drag.js`）：

```js
document.addEventListener('mousedown', (e) => {
  if (e.button === 0 && (e.detail === 1 || e.detail === 2) && isDragRegion(e.composedPath())) {
    e.preventDefault()
    e.stopImmediatePropagation()
    const cmd = e.detail === 2 ? 'internal_toggle_maximize' : 'start_dragging'
    window.__TAURI_INTERNALS__.invoke('plugin:window|' + cmd)
  }
})
```

关键是第一次 `mousedown`（`detail === 1`）就**立刻**发 `start_dragging`。
这个命令在 Windows 侧走的是 `ReleaseCapture()` + `WM_NCLBUTTONDOWN(HTCAPTION)`，
系统随即进入**模态拖窗循环**。循环期间 WebView2 收不到正常的鼠标消息，
那一次的 `mouseup` 就丢了——Chromium 眼里这次点击没有闭合，
`detail` 的连击计数被打断，第二次 `mousedown` 很可能又是 `detail === 1`。

于是只有在「IPC 往返 + 系统进入拖窗循环」之前就把第二次点击打完，才凑得出
`detail === 2`。这就是你感觉到的「必须点得非常快」。
`e.stopImmediatePropagation()` 还顺手让我们自己挂的 React `onDoubleClick`
永远收不到事件，所以也不是加个 handler 就能绕过。

注意 macOS 分支是**另一套**：`drag.js` 在 mac 上 `detail === 2` 时直接 `return`，
改到 `mouseup` 里判，完全不碰 `start_dragging`——所以 mac 上双击一直是好的。
又一处「mac 正常、Windows 坏」。

### 解法：自己接管拖动，把 `startDragging` 推迟到鼠标真的移动之后

去掉所有 `data-tauri-drag-region`（让 `drag.js` 彻底不参与），改成：

- `mousedown` 只**记下起点并挂临时监听**，不发任何 IPC；
- 指针移动超过 4px 才发 `startDragging()`——这时才真的要拖窗；
- 没移动就松手：什么都没发生，点击序列完整闭合，Chromium 正常累加 `detail`；
- 于是原生 `dblclick` 事件能正常触发，接上 `winToggleMaximize()`。

**双击阈值因此等于系统的双击速度**（Chromium 在 Windows 上取
`GetDoubleClickTime()`，默认 500ms，跟控制面板里的设置联动）——
也就是「和别的软件一样」，而不是我们自己定一个数。
4px 的移动门槛也顺带修掉一个小毛病：现在手抖 1px 就会触发拖窗，窗口会跳一下。

> 如果你试过之后觉得 500ms 还是紧，我再换成自己计时的阈值（比如 700ms），
> 但那会和系统其它软件不一致，所以默认先跟系统。

## 3. 边缘拖拽命中区偏到窗口外面

`tauri.windows.conf.json` 关掉了 `decorations`，窗口仍保留 `WS_THICKFRAME`，
所以系统给的那条不可见 resize 边框还在——它在**可见窗口之外**约
`SM_CXSIZEFRAME + SM_CXPADDEDBORDER`（Win11 默认 8px）。
有标题栏装饰的窗口，这 8px 落在系统画的边框与阴影里，手感上「就在边上」；
我们的窗口这 8px 全在画面之外，所以你必须把鼠标移出窗口才拿到 resize 光标。

这个外圈**不能从前端移进来**（要改就得在 Rust 侧接 `WM_NCHITTEST`）。
但可以在窗口**里侧**再补一圈自己的命中区，两边一夹，
抓取带就正好压在可见边上——这也是 `startResizeDragging` 这个 API 存在的理由。

新增 `ResizeEdges` 组件：8 条透明命中条（4 边 + 4 角），贴在 `.shell` 内侧，
各自带正确的 resize 光标，`mousedown` 调
`getCurrentWindow().startResizeDragging(dir)`。

尺寸：边 5px、角 12×12。加上系统外圈 8px，实际抓取带约 13px 跨在边界上。

## 4. 受影响文件

| 文件 | 改动类型 | 具体位置 |
| --- | --- | --- |
| `g:\project\MKPSupportEase\src\app\components\TopTabs.module.css` | 改 | `.bar` 的 `cursor: grab` → `default`（14 行）；`.controls` 的 z-index 61 → 91 |
| `g:\project\MKPSupportEase\src\app\components\TopTabs.tsx` | 改 | 删掉 6 处 `data-tauri-drag-region`；`<header>` 挂 `onMouseDown` / `onDoubleClick` |
| `g:\project\MKPSupportEase\src\app\window.ts` | 改 | 新增 `winStartDragging`、`winStartResize`，`act()` 扩展为支持带参命令 |
| `g:\project\MKPSupportEase\src\app\useTitlebarDrag.ts` | **新增** | 标题栏拖动/双击的事件逻辑（从组件里拆出来，TopTabs 只负责挂） |
| `g:\project\MKPSupportEase\src\app\components\ResizeEdges.tsx` | **新增** | 8 条边缘命中条 |
| `g:\project\MKPSupportEase\src\app\components\ResizeEdges.module.css` | **新增** | 命中条定位与光标 |
| `g:\project\MKPSupportEase\src\app\App.tsx` | 改 | Windows + Tauri 下渲染 `<ResizeEdges />` |
| `g:\project\MKPSupportEase\src-tauri\capabilities\default.json` | 改 | 增 `core:window:allow-start-resize-dragging`；`allow-start-dragging` 的说明要改（不再是给 `data-tauri-drag-region` 用的） |
| `g:\project\MKPSupportEase\docs\ARCHITECTURE.md` | 改 | §7 第 ③ 条整条重写——`data-tauri-drag-region` 已经不用了，那段说明会误导后人 |

## 5. 实现细节

### 5.1 window.ts：补两个动作

现在的 `act()` 只支持无参命令。加一个带参的分支：

```ts
import { getCurrentWindow } from '@tauri-apps/api/window'
import type { ResizeDirection } from '@tauri-apps/api/window'

export const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

async function act(name: 'minimize' | 'toggleMaximize' | 'close' | 'startDragging') {
  if (!inTauri) return
  try {
    await getCurrentWindow()[name]()
  } catch (e) {
    console.error(`[window] ${name} 失败`, e)
  }
}

export const winMinimize = () => act('minimize')
export const winToggleMaximize = () => act('toggleMaximize')
export const winClose = () => act('close')

/** 把拖窗交给系统。调用时机由 useTitlebarDrag 决定 —— 必须等鼠标真的移动了 */
export const winStartDragging = () => act('startDragging')

/** 从某条边/某个角开始改窗口大小 */
export async function winStartResize(dir: ResizeDirection) {
  if (!inTauri) return
  try {
    await getCurrentWindow().startResizeDragging(dir)
  } catch (e) {
    console.error('[window] startResizeDragging 失败', e)
  }
}
```

### 5.2 useTitlebarDrag.ts（新增）

```ts
import { useCallback, useEffect, useRef } from 'react'
import { winStartDragging, winToggleMaximize } from './window'

/** 超过这个距离才算「在拖窗」，不到就当普通点击 —— 手抖 1px 不该让窗口跳 */
const DRAG_THRESHOLD_PX = 4

/**
 * 标题栏的拖动与双击缩放。
 *
 * 为什么不用 `data-tauri-drag-region`：Tauri 的 drag.js 在第一次 mousedown 就发
 * start_dragging，系统随即进入模态拖窗循环、吃掉这次的 mouseup，Chromium 的连击
 * 计数被打断 —— 表现是双击最大化要点得极快才灵。它还 stopImmediatePropagation，
 * 连我们自己挂 onDoubleClick 都收不到。
 *
 * 这里的做法是把 startDragging 推迟到指针真的移动之后：没移动就什么都不发，
 * 点击序列完整闭合，原生 dblclick 正常触发，阈值等于系统双击速度。
 */
export function useTitlebarDrag() {
  const cleanupRef = useRef<(() => void) | null>(null)

  useEffect(() => () => cleanupRef.current?.(), [])

  const onMouseDown = useCallback((e: React.MouseEvent<HTMLElement>) => {
    if (e.button !== 0) return
    // 按钮、链接、输入框自己处理点击，不参与拖窗
    if ((e.target as HTMLElement).closest('button, a, input, select, textarea')) return

    const startX = e.clientX
    const startY = e.clientY

    const onMove = (ev: MouseEvent) => {
      if (Math.abs(ev.clientX - startX) < DRAG_THRESHOLD_PX
        && Math.abs(ev.clientY - startY) < DRAG_THRESHOLD_PX) return
      cleanup()
      void winStartDragging()
    }

    // 系统接管拖窗后 mouseup 可能收不到，blur 是兜底 —— 不清理会留下常驻监听
    const cleanup = () => {
      window.removeEventListener('mousemove', onMove)
      window.removeEventListener('mouseup', cleanup)
      window.removeEventListener('blur', cleanup)
      cleanupRef.current = null
    }

    cleanupRef.current = cleanup
    window.addEventListener('mousemove', onMove)
    window.addEventListener('mouseup', cleanup)
    window.addEventListener('blur', cleanup)
  }, [])

  const onDoubleClick = useCallback((e: React.MouseEvent<HTMLElement>) => {
    if ((e.target as HTMLElement).closest('button, a, input, select, textarea')) return
    void winToggleMaximize()
  }, [])

  return { onMouseDown, onDoubleClick }
}
```

放在 `src/app/` 而不是 `src/hooks/`：它和 `window.ts` 是一组（窗口 chrome 的行为），
`src/hooks/` 里那几个（`useDensity` / `usePlatform` / `useWindowMaximized`）是通用探测。

### 5.3 TopTabs.tsx

删掉全部 6 处 `data-tauri-drag-region="deep"`（header、两个 `<nav>`、`.title`、
`.logo`、`.lightsInset`），换成 header 上两个 handler：

```tsx
const drag = useTitlebarDrag()

// ...

<header
  className={s.bar}
  onMouseDown={drag.onMouseDown}
  onDoubleClick={drag.onDoubleClick}
  data-density={density}
  data-platform={platform}
  data-fluid={fluid ? 'true' : undefined}
>
```

页签 `<button>` 不会被误当成拖动起点：handler 里先 `closest('button, …')` 挡掉。
比 `drag.js` 的 `isDragRegion` 更简单，因为我们只有这一条标题栏，
不需要「deep / self / false」三档语义。

### 5.4 ResizeEdges.tsx（新增）

```tsx
import { winStartResize } from '../window'
import type { ResizeDirection } from '@tauri-apps/api/window'
import s from './ResizeEdges.module.css'

/* 8 个方向与各自的 class。顺序无所谓 —— 角在后面，DOM 顺序让它盖在边上面 */
const EDGES: { dir: ResizeDirection; cls: string }[] = [
  { dir: 'North', cls: s.n }, { dir: 'South', cls: s.s },
  { dir: 'West', cls: s.w }, { dir: 'East', cls: s.e },
  { dir: 'NorthWest', cls: s.nw }, { dir: 'NorthEast', cls: s.ne },
  { dir: 'SouthWest', cls: s.sw }, { dir: 'SouthEast', cls: s.se },
]

/**
 * 窗口内侧的一圈 resize 命中区。
 *
 * 系统那条不可见 resize 边框在**可见窗口之外** 8px（decorations: false 之后
 * 它没有可见边框可以落脚），所以不补这一圈的话，必须把鼠标移出窗口才抓得到边。
 * 前端改不了外圈（那要 Rust 侧接 WM_NCHITTEST），但补内侧一圈之后
 * 抓取带就跨在边界上，和系统其它软件的手感一致。
 */
export default function ResizeEdges() {
  return (
    <>
      {EDGES.map(({ dir, cls }) => (
        <div
          key={dir}
          className={`${s.edge} ${cls}`}
          onMouseDown={(e) => {
            if (e.button !== 0) return
            e.preventDefault()
            void winStartResize(dir)
          }}
        />
      ))}
    </>
  )
}
```

### 5.5 ResizeEdges.module.css（新增）

```css
/* 边 5px、角 12px。加上系统那圈 8px，抓取带约 13px 跨在可见边上。
   z-index 90 压过模态遮罩（ui.module.css 的 .scrim 是 80）—— 开着弹窗也该能改大小；
   标题栏那三颗窗口键是 91，比这里高，所以右上角不会被命中条抢掉点击。 */
.edge {
  position: absolute;
  z-index: 90;
}

.n { top: 0; left: 12px; right: 12px; height: 5px; cursor: ns-resize }
.s { bottom: 0; left: 12px; right: 12px; height: 5px; cursor: ns-resize }
.w { left: 0; top: 12px; bottom: 12px; width: 5px; cursor: ew-resize }
.e { right: 0; top: 12px; bottom: 12px; width: 5px; cursor: ew-resize }

.nw { top: 0; left: 0; width: 12px; height: 12px; cursor: nwse-resize }
.se { bottom: 0; right: 0; width: 12px; height: 12px; cursor: nwse-resize }
.ne { top: 0; right: 0; width: 12px; height: 12px; cursor: nesw-resize }
.sw { bottom: 0; left: 0; width: 12px; height: 12px; cursor: nesw-resize }
```

### 5.6 App.tsx

```tsx
{inTauri && PLATFORM === 'windows' && <ResizeEdges />}
```

放在 `.shell` 里、`<main>` 之后，DOM 顺序靠后配合 z-index 90 更稳。
macOS 不渲染：那边窗口有系统边框，resize 由系统管；
而且 `apply_liquid_glass` 把角切圆了，方角命中条会露在圆角外面。

### 5.7 capabilities/default.json

```json
    "core:window:allow-start-dragging",
    "core:window:allow-start-resize-dragging",
```

`allow-start-dragging` 保留，但用途变了：以前是给 `data-tauri-drag-region` 用的，
现在是给 `useTitlebarDrag` 手动调 `startDragging()` 用的。description 里要改这句，
不然下一个人会以为删掉 drag-region 之后这条权限就没用了。

### 5.8 ARCHITECTURE.md §7 第 ③ 条

现在那条写的是：

> **③ `data-tauri-drag-region` 需要 `core:window:allow-start-dragging` 权限。**
> …用 `data-tauri-drag-region="deep"`（整棵子树可拖，Tauri ≥ 2.11 支持）。
> 双击缩放不用自己监听：Tauri 的 drag.js 已经在 `mouseup(detail === 2)` 时发
> `internal_toggle_maximize`。

三句里有两句现在是**错的**：我们不再用 `data-tauri-drag-region`；
而且 drag.js 在 Windows 上是 `mousedown(detail === 2)`，`mouseup` 那条只有 mac 走。
整条改写成「为什么自己接管拖动」，把上面第 2 节的结论压缩进去。

## 6. 边界条件与异常

| 情况 | 行为 |
| --- | --- |
| 浏览器路径 | `winStartDragging` / `winStartResize` 因 `inTauri` 为假空转；`ResizeEdges` 不渲染；双击标题栏无反应。改样式照常在浏览器里验 |
| macOS | `useTitlebarDrag` 照常挂（拖动与双击都走我们这套，不再依赖 drag.js 的 mac 分支）；`ResizeEdges` 不渲染 |
| 点页签后拖动 | `closest('button')` 挡掉，页签正常切换，不会误触发拖窗 |
| 拖动中松手在窗口外 | 系统接管后我们的 `mouseup` 可能收不到，`blur` 兜底清理；再不行组件卸载时 `cleanupRef` 也会清 |
| 双击窗口键 | 三颗键是 `<button>`，`onDoubleClick` 里 `closest('button')` 挡掉，不会既关窗又缩放 |
| 已最大化时拖边缘 | 系统会拒绝 resize，`startResizeDragging` 返回错误进 console，不影响界面 |
| 最大化时的 resize 命中条 | 仍然渲染。Windows 上最大化窗口的边缘 resize 本来就无效，光标会变但拖不动 —— 与系统一致，不特殊处理 |
| 内容贴边被命中条挡住 | 边只有 5px，且 `.body` 内容都有内衬；幽灵滚动条零宽不受影响 |

## 7. 数据流

拖窗（改后）：

```
mousedown 在标题栏
  → useTitlebarDrag.onMouseDown：记起点，挂 mousemove/mouseup/blur
  → 指针移动 > 4px
      → cleanup() 卸监听 → winStartDragging()
      → IPC 'plugin:window|start_dragging'（权限 allow-start-dragging）
      → 系统进入拖窗循环
  → 没移动就 mouseup
      → cleanup()，不发任何 IPC
      → 点击序列闭合，Chromium 累加 detail
      → 第二次点击在系统双击间隔内 → 原生 dblclick
      → winToggleMaximize() → 'plugin:window|toggle_maximize'
```

改窗口大小：

```
mousedown 在某条命中条
  → winStartResize('SouthEast')
  → IPC 'plugin:window|start_resize_dragging'（权限 allow-start-resize-dragging）
  → 系统进入 resize 循环
```

## 8. 预期结果与验证

1. `npm run lint`、`npm run build` 全绿。
2. `npm run tauri dev` 在原生窗口里逐项验：
   - 标题栏空白处光标是**箭头**，不是小手；页签与窗口键上是小手；
   - 按住标题栏移动 → 窗口跟手；按下不动再松开 → 窗口不跳；
   - **慢速双击**（间隔半秒左右）标题栏 → 最大化；再双击 → 还原；
   - 双击窗口键不会触发缩放；
   - 鼠标移到窗口**可见边缘上**（不用移出去）→ 出现对应的 resize 光标，能拖动改大小；
   - 四个角能斜向改大小；
   - 拖窗、拖边缘之后 devtools 里没有残留的 `[window] … 失败`。
3. 顺手回归一次：三颗窗口键仍然工作、页签仍能切换、标题栏 38px 高度没变。

## 9. 本轮明确不做

- **Rust 侧 `WM_NCHITTEST`**：能把系统那圈 8px 真正移进来，也是 Snap Layouts
  的前置条件，但属于另一个量级（要引 `windows` crate、处理 DPI 与最大化态），另开一轮；
- **自定义双击阈值**：先跟系统的 `GetDoubleClickTime()`，你试过觉得紧再说；
- **拖动时的窗口半透明/吸附预览**：系统自带 Snap，不自己做。

## 10. git

继续在 `fix/win-caption-buttons` 分支上做（同一批 Windows 窗口框架问题），
不另开分支。仍然不会擅自 commit。
