/*
 * 排序列表的一行。次序不在这里 —— 由 `order: string[]` 单独给，
 * 行内容按 key 查；换成别的列表（分组 / 可选项）不必改这一层。
 */

export interface SortRow {
  key: string
  label: string
  /** 名字后面那截浅色旁注（键名） */
  by?: string
}
