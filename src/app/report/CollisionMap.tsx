/*
 * **碰撞位置 · 热床俯视图**（2026-10-18）。
 *
 * 碰撞失败那一趟的记录里带着一份几何（`_meta.json` 的 `collision`），这一屏就是把它
 * 画出来：热床（可打印区）+ 淡网格、**擦料塔**（按参数算出真实形状：塔身 / 斜肋 /
 * 护套，与参数页同一套算法）、**模型整体包围盒**、**冲突点**（红），外加一条图例。
 *
 * 三条定死的口径：
 *   1. **只读**：拖拽 / 编辑一概没有 —— 这一屏是"事后复盘"，不是调参器（调参在参数页）。
 *   2. **判据框与真实形状一起画**：塔外面那圈虚线框是**碰撞判据按哪块算**的包围盒，
 *      里面那块是塔的真实长相。只画一个的话，用户会问"图上明明没碰到啊"。
 *   3. **没给就不摆**：几何缺一格（`printable` / `towerBBox`）就整块不画 ——
 *      宁可少一张图，不画一张编出来的图（与报告页"没给就不摆"同一条）。
 *
 * 坐标系：机器坐标 x 向右、y 向上，原点在可打印区左下角；svg 的 y 向下 ——
 * 换算全收在 `px` / `py` 两处，别的地方不出现翻转。
 */
import TowerCoreSvg from '../params/TowerCoreSvg'
import type { ReportBox4, ReportCollision } from '../../api'
import s from './CollisionMap.module.css'

interface Props {
  collision: ReportCollision
}

/** 塔身固定画成 44px（与参数页同一个数 —— 两边比例一致，形状才是同一个形状） */
const TOWER_CORE_PX = 44
/** 参数页那张塔图的裙边圈数（同一套算法，同一个数） */
const BRIM_LOOP_COUNT = 5
/** 网格几等分（照参数页那张热床图的观感） */
const GRID = 6

/** 一个框的四至（缺一格就当没有 —— 不许拿 0 顶替） */
function boxOf(b: ReportBox4 | undefined): { x: number; y: number; w: number; h: number } | null {
  if (b === undefined) return null
  const { xMin, xMax, yMin, yMax } = b
  if (xMin === undefined || xMax === undefined || yMin === undefined || yMax === undefined) {
    return null
  }
  const w = xMax - xMin
  const h = yMax - yMin
  if (!(w > 0) || !(h > 0)) return null
  return { x: xMin, y: yMin, w, h }
}

export default function CollisionMap({ collision }: Props) {
  const bed = boxOf(collision.printable)
  const tower = boxOf(collision.towerBBox)
  const model = boxOf(collision.modelBBox)

  /* 热床都没有（老记录 / 几何缺）⇒ 整块不画，交给上一层的空白态 */
  if (bed === null || tower === null) return null

  /* 画布就用可打印区这一框：模型 / 塔 / 冲突点全按同一套 mm → viewBox 单位 */
  const px = (x: number): number => x - bed.x
  /** 机器 y 向上 → svg y 向下（唯一的一处翻转） */
  const py = (y: number): number => bed.h - (y - bed.y)

  const towerShape = collision.tower ?? null
  const bodyMm = towerShape?.bodyMm ?? tower.w
  /*
   * 塔图那张 svg 自己带"外围结构往外扩多少"的画布：扩展量取**判据框与塔身之差的一半**
   * —— 于是画出来的外沿正好压在虚线判据框上（两件事是同一件事实的两个说法）。
   */
  const expand = Math.max(0, (Math.min(tower.w, tower.h) - bodyMm) / 2)
  const vm = TOWER_CORE_PX / (bodyMm > 0 ? bodyMm : 1)
  const vbOffset = -expand * vm
  const vbSize = TOWER_CORE_PX + expand * vm * 2

  /* 冲突点：落进可打印区才画（框外的点画出来只会误导 —— 那不是撞的地方） */
  const points = (collision.conflictPoints ?? []).filter(
    ([x, y]) => x >= bed.x && x <= bed.x + bed.w && y >= bed.y && y <= bed.y + bed.h,
  )
  const dotR = Math.max(0.8, bed.w / 240)

  return (
    <div className={s.wrap}>
      <div className={s.map} style={{ aspectRatio: `${bed.w} / ${bed.h}` }}>
        {/* 热床 + 网格 + 模型框 + 禁区 + 冲突点：一整张 svg，单位 = mm */}
        <svg
          className={s.svg}
          viewBox={`0 0 ${bed.w} ${bed.h}`}
          preserveAspectRatio="none"
          aria-hidden="true"
        >
          <rect
            className={s.bed}
            x={0}
            y={0}
            width={bed.w}
            height={bed.h}
            rx={Math.min(bed.w, bed.h) / 40}
          />
          {/* 淡网格：给"挪多少"一个尺度感（6 等分，与参数页同一观感） */}
          <g className={s.grid}>
            {Array.from({ length: GRID - 1 }, (_, i) => {
              const at = ((i + 1) * bed.w) / GRID
              return <line key={`v${i}`} x1={at} y1={0} x2={at} y2={bed.h} />
            })}
            {Array.from({ length: GRID - 1 }, (_, i) => {
              const at = ((i + 1) * bed.h) / GRID
              return <line key={`h${i}`} x1={0} y1={at} x2={bed.w} y2={at} />
            })}
          </g>

          {/* 机型额外禁区（通常没有；有就画 —— "塔落在禁区里"那一档靠它说清） */}
          {(collision.forbiddenZones ?? []).map((zone, i) => {
            if (zone.length < 3) return null
            return (
              <polygon
                key={`z${i}`}
                className={s.zone}
                points={zone.map(([x, y]) => `${px(x)},${py(y)}`).join(' ')}
              />
            )
          })}

          {/* 模型整体包围盒（另一种颜色 + 虚线：一眼和塔分开） */}
          {model !== null && (
            <rect
              className={s.model}
              x={px(model.x)}
              y={py(model.y + model.h)}
              width={model.w}
              height={model.h}
            />
          )}

          {/* 判据框：碰撞按这一块算（虚线，压在塔的真实形状外沿上） */}
          <rect
            className={s.towerBox}
            x={px(tower.x)}
            y={py(tower.y + tower.h)}
            width={tower.w}
            height={tower.h}
          />

          {/* 冲突点 */}
          <g className={s.points}>
            {points.map(([x, y], i) => (
              <circle key={i} cx={px(x)} cy={py(y)} r={dotR} />
            ))}
          </g>
        </svg>

        {/* 塔的真实形状（塔身 / 斜肋 / 护套）—— 与参数页同一张 svg，只是换了个落点 */}
        {towerShape !== null && (
          <div
            className={s.tower}
            style={{
              left: `${(px(tower.x) / bed.w) * 100}%`,
              top: `${(py(tower.y + tower.h) / bed.h) * 100}%`,
              width: `${(tower.w / bed.w) * 100}%`,
              height: `${(tower.h / bed.h) * 100}%`,
            }}
          >
            <TowerCoreSvg
              outerVal={
                towerShape.outerStructure === 'rib' || towerShape.outerStructure === 'sheath'
                  ? towerShape.outerStructure
                  : 'brim'
              }
              isRibVisible={
                towerShape.outerStructure === 'rib' && (towerShape.ribWidth ?? 0) > 0
              }
              ribWidthMm={towerShape.ribWidth ?? 0}
              ribExtraLengthMm={towerShape.ribExtraLength ?? 0}
              ribFilletWall={towerShape.ribFilletWall ?? false}
              vbOffset={vbOffset}
              vbSize={vbSize}
              vm={vm}
              brimLoopCount={BRIM_LOOP_COUNT}
              className={s.core}
            />
          </div>
        )}
      </div>

      {/* 图例：三个色块 + 名字（不标数字 —— 数字在「建议」那组里） */}
      <ul className={s.legend}>
        <li>
          <span className={s.swTower} aria-hidden="true" />
          擦料塔
        </li>
        <li>
          <span className={s.swModel} aria-hidden="true" />
          模型包围盒
        </li>
        <li>
          <span className={s.swHit} aria-hidden="true" />
          冲突点
          {collision.conflictCount !== undefined && collision.conflictCount > 0 && (
            <span className={s.legendNum}>{collision.conflictCount} 个</span>
          )}
        </li>
      </ul>
    </div>
  )
}
