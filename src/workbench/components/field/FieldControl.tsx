/*
 * 唯一的分派点（#33）。
 *
 * # 这个文件存在的全部理由
 *
 * 「字段类型 → 用哪个控件」这件事**只允许在这里写一次**。
 * #32 是写了两次：参数页 `ParamFieldRowV032` 里一个 switch（枚举画成分段按钮），
 * 工作台 `RecipeRowV032` 里三个 if（枚举画成下拉、文本自己内联了一个 input）。
 * 结果就是「开关和文本框感觉不一样」—— 因为它们真的是两套组件。
 *
 * # 形态的决定权也在这里
 *
 * 同一个 `choice` 字段，一行一控件时摊成分段按钮（摆得开、少一次点击）、密表格子里收进下拉
 * （摆不开），这两个判断都对。错的是让两个页面各自持有这个决定权。
 * 所以这里收一个 `form: 'row' | 'cell'`，页面只说「我是哪种场合」，不说「画成什么」。
 *
 * # 值的边界
 *
 * 进来的 `raw` 是 TOML 原文字符串，出去的也是。控件内部说强类型（boolean / number），
 * 转换走 `value.ts`，**只在这个文件里发生**。
 */

import type { ReactNode } from 'react'
import NumberField from './NumberField'
import SegmentedField from './SegmentedField'
import SelectField from './SelectField'
import TextField from './TextField'
import type { FieldForm, FieldSchema } from './types'
import { boolOf, decimalsOf, isNum, numOf, rawOfBool, rawOfNum } from './value'

interface Props {
  field: FieldSchema
  /** 当前值：TOML 原文 */
  raw: string
  onChange: (next: string) => void
  form: FieldForm
  /** 被父项条件关掉、或这一屏只读：看得见、改不动 */
  disabled?: boolean
  /** 数字控件：点框内任何一处都进编辑态（透传给 `NumberField`，见它那里的说明） */
  focusOnBoxClick?: boolean
  /**
   * `gcode` 交给宿主画。
   *
   * 多行 G-code 既塞不进一个格子，也塞不进一行 —— 参数页给的是整块编辑器，
   * 工作台给的是「N 行 · 点开」。共用件不该替它们选一种。
   */
  renderBlock?: (field: FieldSchema) => ReactNode
}

/** 分段最多摆几段，超过就改下拉。与参考项目 ParamSegmented 的 MAX_INLINE 同一个值 */
const MAX_INLINE = 5

export default function FieldControl({
  field,
  raw,
  onChange,
  form,
  disabled = false,
  focusOnBoxClick = false,
  renderBlock,
}: Props): ReactNode {
  const cell = form === 'cell'

  switch (field.control) {
    case 'switch':
      return (
        <SegmentedField
          variant="switch"
          label={field.label}
          checked={boolOf(raw)}
          disabled={disabled}
          dense={cell}
          onChange={(on) => onChange(rawOfBool(on))}
        />
      )

    case 'choice': {
      const options = field.choices ?? []
      /*
       * 摆成哪副样子：row 场合调用方指定了（定义里的 `uiComponent`：分段 / 下拉）
       * 就听它的；没指定（老稿不认识这个字段）按选项数自判。**cell 场合永远收进
       * 下拉** —— 那是格子，摆不开，指定了也不摊。
       * 「我选的是下拉却画成分段」就是从前漏传这个字段开始的（作者的实测）。
       */
      const inline = cell
        ? false
        : field.choiceLayout
          ? field.choiceLayout === 'inline'
          : options.length <= MAX_INLINE
      return inline ? (
        <SegmentedField
          variant="segmented"
          label={field.label}
          options={options}
          value={raw}
          disabled={disabled}
          onChange={onChange}
        />
      ) : (
        <SelectField
          label={field.label}
          options={options}
          value={raw}
          disabled={disabled}
          size={cell ? 'sm' : 'md'}
          onChange={onChange}
        />
      )
    }

    case 'number':
      return (
        <NumberField
          label={field.label}
          value={numOf(raw)}
          step={field.step ?? 1}
          min={field.min}
          max={field.max}
          decimals={decimalsOf(field.step)}
          /* 密表里单位是独立一列，塞两遍是重复信息 —— 保留 #32 这条判断 */
          unit={cell ? undefined : field.unit}
          disabled={disabled}
          invalid={!isNum(raw)}
          size={cell ? 'sm' : 'md'}
          focusOnBoxClick={focusOnBoxClick}
          onChange={(n) => onChange(rawOfNum(n, field.step))}
        />
      )

    case 'text':
      return (
        <TextField
          label={field.label}
          value={raw}
          disabled={disabled}
          clickToEdit={cell}
          onChange={onChange}
        />
      )

    case 'gcode':
      return renderBlock?.(field) ?? null
  }
}
