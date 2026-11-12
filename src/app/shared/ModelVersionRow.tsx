/*
 * 「打开模型」弹窗里那一行**模型版本**（2026-11-12）。
 *
 * # 为什么抽出来
 *
 * 校准页与首页各有一个「打开模型」入口，弹窗本身是两份（文案键不同），但版本这一行
 * 必须一模一样 —— 首页原来根本没有这一行，于是它永远只开原版。抽成一件之后
 * 两页说的是同一份清单（`calibVersions.ts`），不会再漂。
 *
 * # 为什么是下拉而不是平铺按钮
 *
 * 三档（二合一 / 改版 / 原版）平铺会把那一行撑成两行。下拉用共用件 `SelectField`
 * （与设置页、BBS 抽屉同一件）—— 本仓 2026-10-13 起就没有原生 `<select>` 了。
 *
 * 名字保持短（`原版` / `改版` / `二合一`），说明走选项右侧的 `note`：
 * 触发器里是 `nowrap + ellipsis`，把「名字 — 长描述」拼进去会被截断且看不见全文。
 */

import { SelectField } from '../../components/field'
import { useT } from '../../i18n'
import type { CalibVersion } from './calibVersions'
import s from './ModelVersionRow.module.css'

export default function ModelVersionRow({
  versions,
  value,
  onChange,
}: {
  /** 可以不传：测试模型这类只有一份的没有版本可选（传 `undefined` 就当单版处理） */
  versions?: readonly CalibVersion[]
  value: string
  onChange: (id: string) => void
}) {
  const tr = useT()
  /* 单版 / 没有版本清单都不出这一行 */
  if (versions === undefined || versions.length < 2) return null
  return (
    <div className={s.row}>
      <span className={s.label}>{tr('calib.board.modelVersion')}</span>
      <SelectField
        label={tr('calib.board.modelVersion')}
        value={value}
        options={versions.map((v) => ({
          value: v.id,
          label: tr(v.label),
          ...(v.note === undefined ? {} : { note: tr(v.note) }),
        }))}
        onChange={onChange}
      />
    </div>
  )
}
