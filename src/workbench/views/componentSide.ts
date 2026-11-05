/*
 * 组件卡里那块画布在卡片的哪一边（2026-11-01）。
 *
 * 客户端各组件自己决定「图在左还是参数在左」，工作台只能照抄一张小表：
 * 值来自客户端各组件自己的排法，不是猜的 —— 新组件有画布了，加一行即可。
 *
 * 放在单独的 `.ts` 里而不是跟着组件文件：那边只管画，`react-refresh` 不喜欢
 * 一个文件同时导出组件和常量。
 */

const COMPONENT_SIDE: Record<string, 'left' | 'right'> = {
  /* 客户端 `tower-map`：左边是可拖的画布，右边是 X/Y/挤出/速度四行 */
  'tower-map': 'left',
}

/** 这个组件有没有画布、画布在哪一侧；`null` = 它只是张普通卡，不用占位 */
export function slotSideOf(component: string | null): 'left' | 'right' | null {
  if (component === null) return null
  return COMPONENT_SIDE[component] ?? null
}
