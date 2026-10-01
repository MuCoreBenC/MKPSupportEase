/*
 * 字段共用件的出口（#33）。
 *
 * 宿主一律从这里 import。控件之间可以互相引用（`SelectField` 用 `FieldPopover`），
 * 但宿主不该直接指向某个实现文件 —— 那样将来换实现就得改一堆 import。
 *
 * 目录里的东西只有两类：
 *
 *   控件      SegmentedField / NumberField / SelectField / TextField
 *   支撑件    FieldControl（唯一分派点）、FieldLayer + FieldPopover（浮层）、
 *             types（字段长什么样）、value（TOML 原文 ⇄ 强类型）
 */

export type { FieldControlType, FieldForm, FieldOption, FieldSchema } from './types'
export type { FieldRaw } from './value'
export { boolOf, decimalsOf, isNum, numOf, rawOfBool, rawOfNum } from './value'

export { default as FieldLayer } from './FieldLayer'
export { useFieldLayer } from './fieldLayerContext'
export type { FieldLayerApi } from './fieldLayerContext'
export { default as FieldPopover } from './FieldPopover'

export { default as SegmentedField } from './SegmentedField'
export { default as NumberField } from './NumberField'
export { default as SelectField } from './SelectField'
export { default as TextField } from './TextField'
export { default as FieldControl } from './FieldControl'
