import { useCallback, useEffect, useRef, useState } from 'react'
import type { ArtKind, HeroArt } from './heroArt'

export interface ArtLayer {
  id: number
  src: string
  kind: ArtKind
  /** true = 这一层还没播过淡入 */
  fresh: boolean
}

/**
 * 大图的层状态放在页面层，不放在 HeroFade 里。
 *
 * 卡片位与退出层会各挂一份同样的内容，页面本身却一直活着。状态留在组件内的话，
 * 每次重新挂载都会重播淡入 —— 那就是展开 / 折叠时闪的那一下。
 * 放到页面层之后，"播过了"这件事能跨重挂留住，只有真的换图才会再淡入一次。
 */
export function useArtLayers(art: HeroArt | null) {
  const seq = useRef(0)
  const [layers, setLayers] = useState<ArtLayer[]>(() =>
    art ? [{ id: 0, fresh: false, ...art }] : [],
  )

  const src = art?.src ?? null
  const kind = art?.kind ?? null

  useEffect(() => {
    setLayers((prev) => {
      if (!src || !kind) return prev.length === 0 ? prev : []
      const cur = prev[prev.length - 1]
      if (cur && cur.src === src) return prev
      seq.current += 1
      const next: ArtLayer = { id: seq.current, fresh: true, src, kind }
      // 最多两层：一层在淡出、一层在淡入
      return cur ? [cur, next] : [next]
    })
  }, [src, kind])

  /** 淡入播完：旧层退场，新层标记为已播 */
  const settle = useCallback((id: number) => {
    setLayers((prev) => {
      const kept = prev.filter((l) => l.id === id)
      if (kept.length === 0) return prev
      return kept.map((l) => (l.fresh ? { ...l, fresh: false } : l))
    })
  }, [])

  /** 淡出播完 / 加载失败：直接丢掉这一层 */
  const drop = useCallback((id: number) => {
    setLayers((prev) => prev.filter((l) => l.id !== id))
  }, [])

  return { layers, settle, drop }
}
