/*
 * 结论页「擦料塔」这一组摆不摆。
 *
 * 两条判据，缺一不可：
 *   · **校准件那一趟**（`detail.calibration` 有值）⇒ 不摆 —— 那一趟没有塔，
 *     板子图已经让给「结论」里的校准件位置图；
 *   · 其余记录按后端给没给 `detail.tower` 说话：`null` / 缺键（失败 / 旧记录）⇒ 不摆。
 *
 * 这一条判断单独放一处：`buildGroups` 与判据测试读同一份，不在 JSX 里散写条件。
 */
import type { ReportDetail } from '../../api'

export function showsTowerGroup(
  d: Pick<ReportDetail, 'tower' | 'calibration'>,
): boolean {
  /* 校准件那一趟：字段、画布、页签一起让位（`calibration` 只在判成校准件时才写） */
  if (d.calibration != null) return false
  return d.tower != null
}
