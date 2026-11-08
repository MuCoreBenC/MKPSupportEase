/*
 * 提示的**文字处理**（不碰 JSX，单独一个文件）。
 *
 * 为什么单放一个：`HeadNote.tsx` 只该导出组件，混着导出函数会让
 * `react-refresh/only-export-components` 报"这个文件不能热更新"。
 */

import { t } from '../../i18n'

/**
 * 把**多条**提示压成一行（标题右边那一格只放得下一行）。
 *
 * 一条就原样；多条给 `N 条 · 第一条 …`，全文仍在 `title` 上。
 *
 * 明细（每条一段那种）**不归这里** —— 要读全的东西住上面那张 hero 卡
 * （`NoticeHero`：卡高定死、内容在卡里滚），不在框身里另起一块；
 * 这一格回答的是"有没有事、什么事"。
 */
export function summarizeNotes(list: string[]): string {
  if (list.length <= 1) return list[0] ?? ''
  return t('notes.more', { count: list.length, first: list[0] ?? '' })
}
