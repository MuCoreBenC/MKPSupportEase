/*
 * 通用右键菜单的出口（A34）。
 *
 * 用法（三步）：
 *
 * ```tsx
 * const menu = useContextMenu<Row>()               // 1. 一个列表一个
 * <tr {...menu.triggerProps(row)}>…</tr>           // 2. 摊在行上（自带 tabIndex）
 * <ContextMenu at={menu.at} entries={entriesOf(menu.target)} onClose={menu.close} />
 * ```
 *
 * 两条硬约定：
 *
 *   1. **宿主要包在 `<FieldLayer>` 里**（浮层的落点与翻边判据都取自它）。没包的话菜单不显形。
 *   2. **触发只有右键**：鼠标右键 / 触屏长按（浏览器自己转成 contextmenu）/ Shift+F10 与菜单键。
 *      不做行尾的 ⋯ 按钮 —— 同一件事两个入口只会让人猜哪个才是真的。
 *
 * 为什么不用 `fixed`：`src/App.module.css` 的 `.window` 有一个永不为 none 的 transform，
 * 它是 fixed 的包含块，#33 那两个点不动的浮层就是这么坏的。坐标一律换算成层内坐标。
 */

export { default as ContextMenu } from './ContextMenu'
export { useContextMenu } from './useContextMenu'
export { isSeparator } from './types'
export type {
  ContextMenuEntry,
  ContextMenuItem,
  ContextMenuPoint,
  ContextMenuSeparator,
} from './types'
export type { ContextMenuApi } from './useContextMenu'
