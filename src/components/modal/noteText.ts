/*
 * 提示的**文字处理**（不碰 JSX，单独一个文件）。
 *
 * 为什么单放一个：`HeadNote.tsx` 只该导出组件，混着导出函数会让
 * `react-refresh/only-export-components` 报"这个文件不能热更新"。
 */

/**
 * 把**多条**提示压成一行（标题右边那一格只放得下一行）。
 *
 * 一条就原样；多条给 `N 条 · 第一条 …`，全文仍在 `title` 上。
 *
 * 明细（每条一行那种列表）**不归这里** —— 它该留在框身里，那是"要读全"的东西，
 * 而这一格回答的是"有没有事、什么事"。
 */
export function summarizeNotes(list: string[]): string {
  if (list.length <= 1) return list[0] ?? ''
  return `${list.length} 条 · ${list[0]} …`
}
