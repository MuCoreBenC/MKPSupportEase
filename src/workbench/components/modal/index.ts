/*
 * 居中模态框的出口（A34）。
 *
 * ```tsx
 * <Modal open={x !== null} title="文件详情" size="sm" onClose={() => setX(null)}>
 *   …
 * </Modal>
 * ```
 *
 * 两条必须知道的：
 *
 *   1. **挂在哪 = 遮罩盖到哪**。它是 `absolute inset:0`，所以要挂在内容区
 *      （有定位祖先的那一层）里，别挂在整页 —— 底栏和 resize 手柄还得点得着。
 *   2. **有未保存内容的框传 `closeOnScrim={false}`**，否则手滑点一下就把人家改的丢了。
 *
 * 用抽屉还是模态框，判据是「关掉之后还要不要回来」：编辑历史那种要回来的用右抽屉，
 * 看完就关的用这个。
 */

export { default as Modal } from './Modal'
export type { ModalProps } from './Modal'
export { modalShortcutGate } from './shortcutGate'
