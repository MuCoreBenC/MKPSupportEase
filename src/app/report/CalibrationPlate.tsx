/*
 * **校准件在板上的位置**（2026-11-23）。
 *
 * 作者：「我希望这个校准模式的话，也是像这种结论一样的出现。这个板子，这个画布显示
 * 它那个包围盒的轮廓」。所以它和越界那张图同一个位置关系（结论字段下面一块板子）、
 * 同一块画布（`CollisionMap`），不另写一套画法。
 *
 * 三条口径：
 *   · 轮廓用 `modelMap.modelOutlines`（**真实外形**，内凹保留）—— L 形件不许画成凸包的
 *     梯形；旧记录没有这一格 ⇒ 退回 `modelPolygons` 的凸包（与从前一致）；
 *   · 外面再圈一圈**实测包围盒**（`detail.calibration.measured`）：档位就是按它认的，
 *     图上要能对上"这一件当时量到多大"；
 *   · 没有 `measured`（失败 / 旧记录）⇒ **整块不摆**，不画一张编出来的图。
 */
import type { ReportBox4, ReportCollision, ReportModelMap } from '../../api'
import { useT } from '../../i18n'
import CollisionMap from './CollisionMap'

export default function CalibrationPlate({
  measured,
  modelMap,
  machineType,
  height = 132,
}: {
  /** 实测挤出包围盒 `[xMin, xMax, yMin, yMax]`（mm，机器坐标） */
  measured: [number, number, number, number]
  /** 板上那块可打印区与模型几何（映射基准）；旧记录可能没有 */
  modelMap?: ReportModelMap | null
  machineType: string | null
  /** 行内缩略图的高度（px）；放大层不认它 */
  height?: number
}) {
  const tr = useT()
  const [x0, x1, y0, y1] = measured
  /* 退化框（量成 0）⇒ 不摆：那一趟没有可画的"在哪" */
  if (!(x1 > x0) || !(y1 > y0)) return null

  const box: ReportBox4 = { xMin: x0, xMax: x1, yMin: y0, yMax: y1 }
  const outlines = modelMap?.modelOutlines ?? []
  const collision: ReportCollision = {
    modelBbox: box,
    ...(modelMap != null
      ? {
          printable: modelMap.printable,
          modelBoxes: modelMap.modelBoxes,
          /* 真实外形优先；旧记录退回凸包（与整理前一致） */
          modelPolygons: outlines.length > 0 ? outlines : modelMap.modelPolygons,
          forbiddenZones: modelMap.forbiddenZones,
        }
      : null),
  }

  return (
    <CollisionMap
      collision={collision}
      machineType={machineType}
      noTower
      height={height}
      label={tr('report.calibMap.label')}
      box={box}
    />
  )
}
