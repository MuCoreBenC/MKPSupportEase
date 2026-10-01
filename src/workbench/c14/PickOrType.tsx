/*
 * 下拉 + 自定义（C14 原样移植）。
 *
 * 品牌、图标这类格子的性质是「大概率是已经用过的那几个，但必须允许新的」。
 * 纯下拉会挡住新品牌，纯输入框就得靠人想。所以：`SelectField` 列已有值，
 * 末项固定是「自定义…」，选中它就把这一格换成输入框。
 * **不改共用件** —— `SelectField` 本身不支持自由输入，这里是组合出来的。
 */

import { useState } from 'react'
import { TextField } from '../components/field'
import SelectField from './field/SelectField'
import s from '../c14.module.css'

/** 用一个不可能撞车的值当「自定义…」的标记 */
const CUSTOM = '\u0000custom'

interface Props {
  label: string
  options: string[]
  value: string
  /** 值为空时下拉上显示什么（如「未填」） */
  emptyLabel?: string
  onChange: (next: string) => void
}

export default function PickOrType({ label, options, value, emptyLabel, onChange }: Props) {
  /* 当前值不在清单里（刚手打进去的），那就直接停在输入模式，别让人以为它被改掉了 */
  const [typing, setTyping] = useState(false)

  if (typing) {
    return (
      <span className={s.addRow}>
        <TextField
          value={value}
          label={label}
          onChange={(next) => {
            onChange(next)
            setTyping(false)
          }}
        />
        <button type="button" className={`${s.btn} ${s.btnSm}`} onClick={() => setTyping(false)}>
          回到选择
        </button>
      </span>
    )
  }

  return (
    <SelectField
      label={label}
      value={value}
      options={[
        ...(value ? [] : [{ value: '', label: emptyLabel ?? '未填' }]),
        ...options.map((o) => ({ value: o, label: o })),
        { value: CUSTOM, label: '自定义…', note: '打一个新的' },
      ]}
      onChange={(next) => {
        if (next === CUSTOM) {
          setTyping(true)
          return
        }
        onChange(next)
      }}
    />
  )
}
