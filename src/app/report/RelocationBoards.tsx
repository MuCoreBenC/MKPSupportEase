/*
 * **报告页里的两幅板子**（2026-10-18）。
 *
 * 作者：「成功挪塔的也可以显示，画出来碰撞位置，就是把后处理的那两个板子也做进报告页面」。
 * 于是这里把后处理那一屏的「现在 / 挪后」搬过来 —— 同一份 `CollisionMap`、同一套画法：
 *
 *   · 左「现在」：挪之前的事实（冲突点、模型框、塔框）+ **画布内白色塔首尾箭头**
 *     （塔中心 → 目标塔中心，长度 = 真实位移）+ **目标虚框**；
 *   · 右「挪后」：同一份事实只把塔框平移过去，冲突点清空，**不画箭头**；
 *   · 两幅之间一支**虚线绿色箭头**（"接下来这一幅"，与后处理那一屏同一条口径）。
 *
 * ★ 事实从 `relocation.before` 来（成功那趟 `collision` 是 `null` —— 撞了就走不到成功
 *   出口，事实必须跟着记录走）。`to == null`（没挪）就只画左幅、不画箭头与虚框。
 */
import type { ReportCollision, ReportBox4 } from '../../api/contract'
import CollisionMap from './CollisionMap'
import s from './RelocationBoards.module.css'

/** 两幅板子的目标高度（px）—— 与后处理那一屏同一口 104，报告页里略大一点好读 */
const MAP_H = 132

/** 塔框中心（机器坐标）—— 白色那支箭头的起点 */
function centerOf(box: ReportBox4 | undefined): [number, number] | null {
  if (box?.xMin === undefined || box.xMax === undefined) return null
  return [(box.xMin + box.xMax) / 2, ((box.yMin ?? 0) + (box.yMax ?? 0)) / 2]
}

/** 挪后那一幅：塔框整体平移，冲突点与建议清空（塔已经挪开了） */
function shifted(c: ReportCollision, dx: number, dy: number): ReportCollision {
  const box = c.towerBbox
  if (box == null) return { ...c, conflictPoints: [], conflictCount: 0, adviceText: '' }
  return {
    ...c,
    towerBbox: {
      xMin: (box.xMin ?? 0) + dx,
      xMax: (box.xMax ?? 0) + dx,
      yMin: (box.yMin ?? 0) + dy,
      yMax: (box.yMax ?? 0) + dy,
    },
    conflictPoints: [],
    conflictCount: 0,
    adviceText: '',
  }
}

export default function RelocationBoards({
  before,
  from,
  to,
  machineType,
}: {
  /** 挪之前的碰撞事实（`relocation.before`） */
  before: ReportCollision
  /** 塔左下角，挪之前 */
  from: [number, number]
  /** 塔左下角，挪之后；`null` = 没挪 */
  to: [number, number] | null
  machineType: string | null
}) {
  const dx = to !== null ? to[0] - from[0] : 0
  const dy = to !== null ? to[1] - from[1] : 0
  const moved = to !== null ? shifted(before, dx, dy) : null

  /* 箭头与目标虚框：与后处理那一屏同一套算法（同一个位移算出来，所以准） */
  const center = centerOf(before.towerBbox)
  const arrow =
    to !== null && center !== null ? { from: center, vec: [dx, dy] as [number, number] } : null
  const ghost =
    to !== null && before.towerBbox != null
      ? {
          xMin: (before.towerBbox.xMin ?? 0) + dx,
          xMax: (before.towerBbox.xMax ?? 0) + dx,
          yMin: (before.towerBbox.yMin ?? 0) + dy,
          yMax: (before.towerBbox.yMax ?? 0) + dy,
        }
      : null

  return (
    <div className={s.pair}>
      <div className={s.fig}>
        <CollisionMap
          collision={before}
          machineType={machineType}
          arrow={arrow}
          ghost={ghost}
          hideLegend
          height={MAP_H}
        />
        <span className={s.tag}>现在</span>
      </div>
      {moved !== null && (
        <>
          {/* 板间那支：虚线 + 主色，和画布里那支白实线箭分开（同一个口径） */}
          <span className={s.arrow} aria-hidden="true">
            <svg viewBox="0 0 44 18" width="44" height="18" fill="none">
              <path
                d="M2 9h33"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeDasharray="5 4"
              />
              <path
                d="M32.5 3.4 41 9l-8.5 5.6"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </svg>
          </span>
          <div className={s.fig}>
            <CollisionMap
              collision={moved}
              machineType={machineType}
              hideArrow
              hideLegend
              height={MAP_H}
            />
            <span className={s.tag}>挪后</span>
          </div>
        </>
      )}
    </div>
  )
}
