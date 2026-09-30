/*
 * 字段共用件的类型（#33）。
 *
 * # 这个文件的边界
 *
 * 共用件**不认识任何一稿的数据层**：它不知道 A1MF 的注册表、不知道配方的机型/版本/来源，
 * 也不知道草稿与撤销栈。它只认识「一个字段长什么样」。所以这里没有 `value` 字段 ——
 * 当前值由宿主拿着，每次渲染传进来（见 `FieldControl` 的 `raw`）。
 *
 * # 为什么不写翻译层
 *
 * `v030/paramsA1MF` 的 `FieldDef` 结构上已经满足 `FieldSchema`（键名与类型逐个对得上，
 * 它的 `Control` 是 `FieldControlType` 的子集）。#29 那一稿「每行先翻译成 ParamRowItem 才画得出来」
 * 的代价是来源那一列被压成一句话，所以这一稿**直接把 FieldDef 当 FieldSchema 用**，
 * 兼容性由 `v033/recipeSourceV033.ts` 里一句 `satisfies` 在编译期盯着。
 */

/** 五种控件形态。`gcode` 不由共用件画（多行文本塞不进一行，也塞不进格子），交宿主给块 */
export type FieldControlType = 'number' | 'switch' | 'choice' | 'text' | 'gcode'

export interface FieldOption {
  value: string
  label: string
  /** 选项右边那一列小字（「3 个版本共用」/「2 处自有」）。下拉与分段都支持 */
  note?: string
  /**
   * 这一档已经弃用（C13）。**调用方说了算，共用件只负责画**：分段与下拉都给
   * 标签划一条线（其余照旧可点 —— 能不能落盘是调用方那道闸的事）。
   *
   * 与 `NumberField` 的 `focusOnBoxClick` 同一条办法：可选、默认关，
   * 不设这个字段的稿一个像素不动。
   */
  deprecated?: boolean
}

export interface FieldSchema {
  /** TOML 路径，如 `toolhead.offset.x` */
  key: string
  label: string
  desc?: string
  control: FieldControlType
  unit?: string
  min?: number
  max?: number
  step?: number
  choices?: FieldOption[]
}

/**
 * 控件摆在哪里。
 *
 *   row   一行一个控件（参数页的卡片）—— 横向摆得开，枚举直接摊成分段按钮
 *   cell  密表里的一个格子（配方工作台）—— 摆不开，枚举收进下拉，单位是独立列
 *
 * 这个取值只影响**形态**，不影响值的含义。判断写在 `FieldControl` 一处，
 * 两个页面都不再拥有「这个字段画成什么」的决定权。
 */
export type FieldForm = 'row' | 'cell'
