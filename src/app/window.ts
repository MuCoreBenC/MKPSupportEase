import { getCurrentWindow } from '@tauri-apps/api/window'

/**
 * 窗口动作 —— 标题栏那几颗按钮真正要干的事。
 *
 * 在浏览器里（没有 Tauri）一律空转：调 `getCurrentWindow()` 会去摸 `__TAURI_INTERNALS__`，
 * 摸不到就抛。前端两条运行路径都要能跑，所以这里统一挡一道，而不是让每个调用点自己 try。
 *
 * macOS 上这几个函数其实用不到：系统的交通灯接管了最小化 / 缩放 / 关闭（见
 * `tauri.conf.json` 的 `titleBarStyle: "Overlay"`）。它们是给 Windows 用的 ——
 * 那边没有 overlay 这回事，装饰整个关掉（`tauri.windows.conf.json`），三颗键由我们自己画、
 * 由这里接上真实动作。
 */
const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

async function act(name: 'minimize' | 'toggleMaximize' | 'close') {
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
