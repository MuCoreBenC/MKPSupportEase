import { createContext, useContext } from 'react'

export type DeckLayer = 'plane' | 'card' | 'exit'
export type DeckPhase = 'idle' | 'in' | 'out'

export interface DeckLayerInfo {
  /** 这一份 sheet 现在被渲染在哪一层：当前页 / 右侧露出卡 / 正在退出的卡 */
  layer: DeckLayer
  phase: DeckPhase
}

/**
 * 同一份 sheet 会被 SlideDeck 同时渲染在两层上（当前页 + 下一页的露出卡）。
 * 页面内容原来只能靠 CSS 的 [data-layer] 分辨自己在哪一层 —— 纯样式够用，
 * 但"露出态换一种布局"这种事 CSS 做不到（flex-direction 不可补间），得让组件知道。
 *
 * 默认值是 plane/idle：不在 deck 里的组件照常渲染，不用判空。
 */
const DeckLayerContext = createContext<DeckLayerInfo>({ layer: 'plane', phase: 'idle' })

export default DeckLayerContext

export function useDeckLayer(): DeckLayerInfo {
  return useContext(DeckLayerContext)
}
