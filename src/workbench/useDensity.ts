/**
 * 密度四档：`ultra` / `wide` / `compact` / `mini`（doc §7 视觉规范）。
 *
 * # 量容器实测宽度，不用 media query
 *
 * media query 量的是**视口**。工作台是第二个窗口，用户会把它拉到半屏；
 * 而更要紧的是同一组组件以后可能被放进一个 400px 的预览格子里 ——
 * 那时候视口还是 1920，而组件只有 400 可用。`ResizeObserver` 量的是容器，
 * 所以两种情况都判得对。
 *
 * 结果写到根节点的 `data-density`，CSS 里 `[data-density='mini'] …` 生效。
 * 之所以走属性而不是给每个组件传 prop：降级是**布局**的事，
 * 传 prop 会让每个组件都要认识"现在是第几档"，而它们只需要认识 CSS。
 *
 * # mini 档纪律：不许隐藏任何可用操作
 *
 * 只能改布局方向、隐藏说明文字、把横排改竖排。`display: none` 掉一个按钮，
 * 用户在小窗口里就永远做不了那件事，而他看不出来是被藏了还是没有这个功能。
 */
import { useEffect, useState, type RefObject } from 'react'

export type Density = 'ultra' | 'wide' | 'compact' | 'mini'

/** 断点照参考实现那一份。**比较的是容器宽度**，不是视口 */
export const DENSITY_BREAKPOINTS = { ultra: 1600, wide: 1080, compact: 640 } as const

export function densityOf(width: number): Density {
  if (width >= DENSITY_BREAKPOINTS.ultra) return 'ultra'
  if (width >= DENSITY_BREAKPOINTS.wide) return 'wide'
  if (width >= DENSITY_BREAKPOINTS.compact) return 'compact'
  return 'mini'
}

/**
 * 盯住一个容器，返回它当前属于哪一档，并把档位写到它的 `data-density` 上。
 *
 * 初值取 `wide` 而不是 `mini`：首帧拿不到宽度，从 mini 起跳会让界面在开窗那一瞬
 * 闪一次上下堆叠的布局
 */
export function useDensity(ref: RefObject<HTMLElement | null>): Density {
  const [density, setDensity] = useState<Density>('wide')

  useEffect(() => {
    const host = ref.current
    if (!host) return

    const apply = (width: number) => {
      const next = densityOf(width)
      host.dataset.density = next
      // 只在真的换档时 setState —— 拖窗口会每帧回调，不拦的话整棵树每帧重渲染
      setDensity((prev) => (prev === next ? prev : next))
    }

    apply(host.getBoundingClientRect().width)
    const ro = new ResizeObserver((entries) => {
      for (const e of entries) apply(e.contentRect.width)
    })
    ro.observe(host)
    return () => ro.disconnect()
  }, [ref])

  return density
}
