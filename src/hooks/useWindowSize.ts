import { useEffect, useState } from 'react'

/**
 * 逻辑窗口尺寸。
 *
 * 大图的尺寸与微移曲线以「窗口宽高」为横轴（见 `src/app/heroCurves.ts`）。试验场里这个值
 * 由预览器外壳量出来再上报进 devStore —— 那里的"窗口"是页面里的一个模拟窗口。
 * 产品里窗口就是窗口：Tauri 的原生窗口 = 视口，所以直接读 `innerWidth/innerHeight`。
 *
 * 用 `resize` 而不是 ResizeObserver：要的是窗口尺寸，不是某个元素的尺寸。
 */
export function useWindowSize() {
  const [size, setSize] = useState(() => ({
    w: typeof window === 'undefined' ? 0 : window.innerWidth,
    h: typeof window === 'undefined' ? 0 : window.innerHeight,
  }))

  useEffect(() => {
    const onResize = () => setSize({ w: window.innerWidth, h: window.innerHeight })
    onResize()
    window.addEventListener('resize', onResize)
    return () => window.removeEventListener('resize', onResize)
  }, [])

  return size
}
