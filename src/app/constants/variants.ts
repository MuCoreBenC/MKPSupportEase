import type { Option } from './machines'

/**
 * 打印件版本（快拆件的版本）。
 * id 同时是取预设的键：`api.getPreset(variantId)`，见 src/api/mock.ts 的 presetIndex。
 */
export const variants: Option[] = [
  { id: 'std', label: '标准版', badge: '推荐', badgeTone: 'rec' },
  { id: 'fast-old', label: '快拆版6月以前', badge: '热门', badgeTone: 'hot' },
  { id: 'fast-260628', label: '快拆版260628', badge: '最新', badgeTone: 'new' },
]
