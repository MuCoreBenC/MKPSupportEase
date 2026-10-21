import { useEffect, useState } from 'react'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { inTauri } from '../app/window'

/**
 * 窗口是否已最大化。只有 Windows 的自绘最大化键用得到 —— 它要在
 * 「方框」与「还原双叠框」之间换图标，跟系统上其它软件一致。
 *
 * 用 onResized 而不是轮询：最大化 / 还原 / 拖拽 Snap / 双击标题栏
 * 都会走 resize 事件，一个订阅覆盖全部入口。浏览器路径下直接返回 false。
 *
 * ★ **拖窗那一路要合并**（2026-10-21）：`Resized` 是逐帧来的，原来每一帧都发一次
 *   `isMaximized` 的 IPC —— 拖一次边能有几十趟往返，全排在"正在重绘"的那条主线程上。
 *   现在改成**尾随 120ms**：拖动过程中不问，手停下来才问一次。
 *   图标换得晚 120ms 看不出来（最大化 / 还原本来也不是"跟手"的动作），
 *   而拖动期间少几十趟 IPC。
 */
const SETTLE_MS = 120

export function useWindowMaximized(): boolean {
  const [maximized, setMaximized] = useState(false)

  useEffect(() => {
    if (!inTauri) return

    const win = getCurrentWindow()
    let alive = true
    let timer: number | null = null

    const sync = () => {
      win
        .isMaximized()
        .then((v) => {
          if (alive) setMaximized(v)
        })
        .catch(() => {
          /* 查询失败就维持上一个值，不值得打断界面 */
        })
    }

    /** 等这一串 resize 停下来再问一次 */
    const later = () => {
      if (timer !== null) window.clearTimeout(timer)
      timer = window.setTimeout(() => {
        timer = null
        sync()
      }, SETTLE_MS)
    }

    /* 首帧那次立刻问：图标不能等 120ms 才从"方框"变成"双叠框" */
    sync()
    const unlisten = win.onResized(later)

    return () => {
      alive = false
      if (timer !== null) window.clearTimeout(timer)
      unlisten.then((off) => off()).catch(() => {})
    }
  }, [])

  return maximized
}
