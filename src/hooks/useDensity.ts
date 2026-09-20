import { useEffect, useState } from 'react'
import type { RefObject } from 'react'

/** 界面密度档位。ultra=超宽屏，wide=宽屏完整，compact=日常窗口，mini=角落小窗 */
export type Density = 'ultra' | 'wide' | 'compact' | 'mini'

export const DENSITY_BREAKPOINTS = { ultra: 1600, wide: 1080, compact: 640 } as const

export function pickDensity(width: number): Density {
  if (width >= DENSITY_BREAKPOINTS.ultra) return 'ultra'
  if (width >= DENSITY_BREAKPOINTS.wide) return 'wide'
  if (width >= DENSITY_BREAKPOINTS.compact) return 'compact'
  return 'mini'
}

/**
 * 观测容器实测宽度得出密度档位。
 * 用 ResizeObserver 而非 @media，使同一组件能在任意尺寸容器内正确渲染，
 * 便于在一屏内并排预览三种形态。
 */
export function useDensity(ref: RefObject<HTMLElement>): Density {
  const [density, setDensity] = useState<Density>('wide')

  useEffect(() => {
    const el = ref.current
    if (!el) return

    const apply = (width: number) => {
      const next = pickDensity(width)
      setDensity((prev) => (prev === next ? prev : next))
    }

    apply(el.getBoundingClientRect().width)

    const ro = new ResizeObserver((entries) => {
      apply(entries[0].contentRect.width)
    })
    ro.observe(el)
    return () => ro.disconnect()
  }, [ref])

  return density
}
