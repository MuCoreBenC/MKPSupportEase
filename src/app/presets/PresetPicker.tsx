/*
 * 机型那一个下拉。
 *
 * ```
 * 机型 [ A1 ▾ ]
 * ```
 *
 * # 版本单选整块删掉了
 *
 * 这里曾经还有一条「版本 ⦿ 标准版 ○ 快拆版6月以前 ○ 快拆版260628」。作者的原话：
 * 「就这么直接显示在这了，很糟糕，甚至也没有全部显示的，如果有很多的版本呢？还是都列出来
 * 占位置吗，我觉得版本都没必要筛选吧，机型可以筛选」。
 *
 * 根因是**筛选器的宽度随选项个数线性增长**：6 个版本就摆不下，实测那一行只画出了 3 个。
 * 而版本是「这个文件的属性」，机型才是「我这台机器关我什么事」的边界。
 * 所以版本降成了表里的一列（`PresetTable` 的第 2 列），这里只剩机型。
 *
 * # 「全部机型」是一档，不是一个复选框
 *
 * 值用**空串**表示（`machineId: ''`）。它顺手补上了之前那个缺口：在 A1 下看不到
 * A1 mini 的本机文件 —— 本机磁盘上的东西不该被一个机型选择藏起来。
 *
 * 下拉用共用件（`components/field/SelectField`），不自造控件。
 * 这一条整个是工具条里的一段，自己不画横带（外框与 padding 在 `PagePresets` 那一层）。
 */

import { SelectField } from '../../components/field'
import type { Machine } from '../../api'
import s from './PresetPicker.module.css'

interface Props {
  machines: Machine[]
  /** 当前机型。**空串 = 「全部机型」** */
  machineId: string
  onPick: (machineId: string) => void
}

/** 「全部机型」那一档的值。空串而不是 `'ALL'`：机型 id 里不可能有空串，不会撞 */
const ALL = ''

export default function PresetPicker({ machines, machineId, onPick }: Props) {
  return (
    <span className={s.picker}>
      <span className={s.legend}>机型</span>
      <SelectField
        label="机型"
        options={[
          /* 「全部机型」排第一档，小字写清它是几台 —— 不是一个特殊状态，就是一档 */
          { value: ALL, label: '全部机型', note: `${machines.length} 台` },
          ...machines.map((m) => ({ value: m.id, label: m.display, note: m.id })),
        ]}
        value={machineId}
        onChange={onPick}
      />
    </span>
  )
}
