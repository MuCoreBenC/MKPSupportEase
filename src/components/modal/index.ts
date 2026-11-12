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
/*
 * 标题栏借用位（见 `modalHeadSlot.ts`）：
 *   动作 = `createPortal(node, useModalHeadSlot())`      —— 按钮，排在 `×` 左边
 *   回执 = `createPortal(node, useModalHeadNoteSlot())`  —— 一句状态，排在副标题右边
 */
export { useModalHeadNoteSlot, useModalHeadSlot } from './modalHeadSlot'
/*
 * **一句提示的唯一出口**（2026-10-17，作者：「把所有这些提示的，不管是什么的，
 * 都用同一套组件，同一套逻辑复用在标题后面显示」）。
 *
 *   <HeadNote text="…" tone="error" | "note" | "ok" />
 *
 * 别自己在框身里摆 `<p className={s.bad}>` —— 框身按内容撑高，加一行框就长一截，
 * 下面那张表跟着跳。这一格永远一行、超出省略号、**字数不影响框的尺寸**。
 */
export { HeadNote } from './HeadNote'
export type { HeadNoteTone } from './HeadNote'
/*
 * **"有事要说"的那张 hero 卡**（2026-11-08，作者：「这种出错的就显示在 HERO 里面……
 * 这样就不用说上面也显示错误、下面也显示错误」）。
 *
 *   <NoticeHero tone="warn" title="警告 · 1 条" lines={[…]} />
 *
 * 卡高恒 104、内容多了在卡里滚 —— 摆进框身里那些提示才会把模态框撑高，这一张不会。
 * 一屏能翻页时（下面那块有多页）它跟下面那页**各有各的 hero**，见 `PageToggle`。
 */
export { NoticeHero } from './NoticeHero'
export type { NoticeHeroProps, NoticeTone } from './NoticeHero'
/*
 * 模态框**左下角那颗翻页小钮**（「信息」/「说明」/「警告」）—— 钮上写"点它会去哪一页"。
 * 靠左由外面那一块带 `margin-right: auto` 的页脚负责，这一颗自己不带。
 */
export { PageToggle } from './PageToggle'
export type { PageToggleProps } from './PageToggle'
export { summarizeNotes } from './noteText'
