/*
 * 「用哪个打开」那一行 + 「仅这一次」勾选（摆进打开模型的模态框正文里）。
 * 纯展示件 —— 状态与"写不写回默认"的口径在 `shared/slicer.ts`（useSlicerPick /
 * settleSlicerPick），单独拆一个文件是为了 react-refresh（一个文件只许出一个组件）。
 *
 * 选项 = `slicerOptionsOf`（内置档 + 已指定的自定义应用）—— 与设置页「切片」
 * 那格**同一份**；框里不摆「其他应用…」（从磁盘挑在设置页做，见 slicer.ts 头注释）。
 *
 * ★ 2026-10-13 第三刀：这只原生 `<select>` 与设置页那格一起换成共用件 `SelectField`
 * （`size="sm"` —— 框里是小字档）。原来那层 `<label>` 跟着退成 `<div>`：
 * 触发器是 `<button>`，塞在 label 里点"仅这一次"那行的字会连带触发它。
 */

import { SelectField } from '../../components/field'
import { slicerOptionsOf, useSlicerAppPref, type SlicerId } from './slicer'
import s from './slicer.module.css'

export function SlicerPickView({
  value,
  once,
  onChange,
  onOnceChange,
}: {
  value: SlicerId
  once: boolean
  onChange: (id: SlicerId) => void
  onOnceChange: (v: boolean) => void
}) {
  const [app] = useSlicerAppPref()
  /* 'custom' 但记录不在（清过缓存）：按跟随系统显示 —— 值不在选项里时触发器显示原值，更难懂 */
  const shown = value === 'custom' && app === null ? 'system' : value
  return (
    <div className={s.wrap} aria-label="用哪个打开">
      <div className={s.row}>
        <span className={s.label}>用哪个打开</span>
        <SelectField
          label="用哪个打开"
          size="sm"
          options={slicerOptionsOf(app)}
          value={shown}
          onChange={(v) => onChange(v as SlicerId)}
        />
      </div>
      <label className={s.check}>
        <input
          type="checkbox"
          checked={once}
          onChange={(e) => onOnceChange(e.target.checked)}
        />
        <span>仅这一次用选定的（默认不写回设置）</span>
      </label>
    </div>
  )
}
