/*
 * 选择器候选的形状（2026-11-14）。
 *
 * 原来是 `GroupPicker`（浮层下拉）的输入类型；那个组件随「所有下拉都搬进模态框」
 * 一起退役，形状本身留着 —— `PickerModal` 与「编辑定义」拼候选都按它来。
 *
 * 分两层是刻意的：**组标题 + 组内若干行**。`title: null` 的组不画标题
 * （「不属于谁」「无条件」那两条就是它）。
 */

import type { ReactNode } from 'react'

export interface GroupedOption {
  value: string
  /** 显示名 */
  label: string
  /** 名字后面那截浅色旁注（键名 / 位置 …）。没有就不画 */
  by?: string
  /** 行右侧的小字。**禁用的理由也写这儿**，不许只灰着不解释 */
  note?: string
  disabled?: boolean
  /** 行尾额外的次级按钮（排序框里的 ↑↓ 用的就是这一格） */
  extra?: ReactNode
}

export interface GroupedGroup {
  /** 组标题（页签名 / 分组名）。`null` = 不画标题，直接排选项 */
  title: string | null
  options: GroupedOption[]
}
