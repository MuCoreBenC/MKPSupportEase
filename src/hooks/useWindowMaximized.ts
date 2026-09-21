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
