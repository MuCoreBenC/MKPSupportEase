/*
 * **越界那一屏的俯视图**（2026-11-19）。
 *
 * 画的还是擦料塔那一栏那块画布（`CollisionMap`）：同一份板规格、同一套换算 ——
 * 区别只有三条，全部由 `boundary` 那格事实说：
 *
 *   · 模型是**每块一条轮廓**（内核按占用格连通块切好的凸包），不是整盘一个框；
 *   · 凸出「允许范围」的那部分标红（裁一条"范围之外"的路径）；
 *   · 越界的块各一支白色箭头：从这块的中心指向挪到位的位置（长度就是建议位移）。
 *
 * 报告页「结论」组与处理完成弹窗的「说明」页**共用这一个** —— 作者原话：
 * 「他们两个就用同一个啊，就是复用的，不用说额外再多做什么」。
 */
import type { ReactNode } from 'react'

import type { ReportBoundary, ReportBoundaryPiece, ReportCollision, ReportModelMap } from '../../api'
import { useT } from '../../i18n'
import CollisionMap from './CollisionMap'

export default function BoundaryPlate({
  boundary,
  modelMap,
  machineType,
  height = 132,
  layout = 'stack',
  numbered = false,
  caption,
}: {
  boundary: ReportBoundary
  /** 板上那块可打印区（映射基准）；拿判据范围当基准会整体平移，没有就交给板规格 */
  modelMap?: ReportModelMap | null
  machineType: string | null
  /** 行内缩略图的高度（px）；放大层不认它 */
  height?: number
  /** `row` = 图在左、图例与说明在右（弹窗说明页）；默认图下摆一行图例（报告页） */
  layout?: 'stack' | 'row'
  /** 给越界块编 1、2、3…（与右栏逐条文案同一份排序；报告页不传） */
  numbered?: boolean
  /** `row` 档右栏里图例上面那几句（报告页不传） */
  caption?: ReactNode
}) {
  const tr = useT()
  const pieces: ReportBoundaryPiece[] = boundary.pieces ?? []
  /* 没有几何（旧记录 / 读不出模型点）⇒ 整块不摆，不画一张编出来的图 */
  if (pieces.length === 0) return null

  const collision: ReportCollision = {
    ...(modelMap?.printable !== undefined ? { printable: modelMap.printable } : null),
    ...(modelMap?.forbiddenZones !== undefined
      ? { forbiddenZones: modelMap.forbiddenZones }
      : null),
  }

  return (
    <CollisionMap
      collision={collision}
      machineType={machineType}
      noTower
      height={height}
      label={tr('report.boundaryMap.label')}
      allowed={boundary.limit ?? null}
      pieces={pieces}
      /* 笔尖那一层（与模型重叠处不画）：两种虚线分开，模型仍按切片器所见的位置 */
      pen={boundary.pen ?? null}
      layout={layout}
      numbered={numbered}
    >
      {caption}
    </CollisionMap>
  )
}
