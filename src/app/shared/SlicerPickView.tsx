/*
 * 「用哪个打开」那一行 + 「仅这一次」勾选（摆进打开模型的模态框正文里）。
 * 纯展示件 —— 状态与"写不写回默认"的口径在 `shared/slicer.ts`（useSlicerPick /
 * settleSlicerPick），单独拆一个文件是为了 react-refresh（一个文件只许出一个组件）。
 */

import { SLICER_CHOICES, type SlicerId } from './slicer'
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
  return (
    <div className={s.wrap} aria-label="用哪个打开">
      <label className={s.row}>
        <span className={s.label}>用哪个打开</span>
        <select
          className={s.sel}
          value={value}
          onChange={(e) => onChange(e.target.value as SlicerId)}
        >
          {SLICER_CHOICES.map((c) => (
            <option key={c.id} value={c.id}>
              {c.label}
            </option>
          ))}
        </select>
      </label>
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
