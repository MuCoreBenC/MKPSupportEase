import { getCurrentWindow } from '@tauri-apps/api/window'

/** @tauri-apps/api 2.11 没导出这个类型，自己定义一份 */
export type ResizeDirection = 'East' | 'North' | 'NorthEast' | 'NorthWest' | 'South' | 'SouthEast' | 'SouthWest' | 'West'

/**
 * 窗口动作 —— 标题栏那几颗按钮、拖窗、改大小真正要干的事。
 *
 * 在浏览器里（没有 Tauri）一律空转：调 `getCurrentWindow()` 会去摸 `__TAURI_INTERNALS__`，
 * 摸不到就抛。前端两条运行路径都要能跑，所以这里统一挡一道，而不是让每个调用点自己 try。
 *
 * macOS 上最小化 / 缩放 / 关闭其实用不到：系统的交通灯接管了（见 `tauri.conf.json` 的
 * `titleBarStyle: "Overlay"`）。它们是给 Windows 用的 —— 那边没有 overlay 这回事，
 * 装饰整个关掉（`tauri.windows.conf.json`），三颗键由我们自己画、由这里接上真实动作。
 *
 * 拖窗与改大小两平台都走这里：`data-tauri-drag-region` 已经不用了（原因见
 * `useTitlebarDrag.ts` 的注释），系统那圈 resize 边框又落在可见窗口之外（见 `ResizeEdges`）。
 */
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

/**
 * 把拖窗交给系统。
 *
 * 调用时机很关键，由 `useTitlebarDrag` 决定：**必须等鼠标真的移动之后**。
 * 一按下就调的话，系统立刻进入模态拖窗循环、吃掉这次的 mouseup，
 * 双击就凑不出来了。
 */
export const winStartDragging = () => act('startDragging')

/** 从某条边 / 某个角开始改窗口大小。带参数，所以不走 `act` */
export async function winStartResize(dir: ResizeDirection) {
  if (!inTauri) return
  try {
    await getCurrentWindow().startResizeDragging(dir)
  } catch (e) {
    console.error(`[window] startResizeDragging(${dir}) 失败`, e)
  }
}
