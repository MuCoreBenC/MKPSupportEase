/*
 * 圆钮开关的出口。
 *
 * 宿主一律从这里 import（`import { Switch } from '../../components/switch'`）——
 * 别直接指向 `Switch.tsx`，将来换实现才只用改这一处。
 *
 * 与 `components/field/SegmentedField` 的 `variant="switch"` 是**两件东西**：
 * 那一件是"关闭 | 开启"两段胶囊（参数表在用），这一件是设置页那种圆钮滑块。
 * 别为了让它们一样而改任何一边，见 `Switch.tsx` 头注。
 */

export { default as Switch } from './Switch'
export type { SwitchProps } from './Switch'
