/**
 * 机型选择的三级选项 —— 品牌。
 *
 * 界面结构数据：这三级（品牌 / 机型 / 打印件版本）决定选择器长什么样、有几列，
 * 属于应用本身。真正会被 Rust 接管的是"选完之后拿到哪份预设"（见 src/api/mock.ts）。
 *
 * `Option` 是三级共用的形状，所以定义在这里，models.ts / variants.ts 从这里引。
 */
export interface Option {
  id: string
  label: string
  badge?: string
  badgeTone?: 'new' | 'hot' | 'rec'
  note?: string
}

export const brands: Option[] = [{ id: 'bambu', label: '拓竹 (Bambu Lab)' }]
