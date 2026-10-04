/*
 * 擦料塔地图（A43 移植）—— 从参考程序 mkpsupporte 的参数页移植（那一稿叫 TowerMapPanel）。
 *
 * 它在「擦料塔位置与打印」这张卡里替下全部参数行的排布：
 *   · 左边一块按机型画的热床图 —— 有板规格的机型铺真板轮廓（`Plate`，随 catalog 下发），
 *     可打印区按 frame 叠在板上（真实余量：A1 背缘 0.5mm、mini 四边 2mm），前缘把手露在
 *     可打印区下方；没有板规格的机型整层不出。6×6 网格、四周一圈改不进去的边缘留白。
 *     frame 来自官方板件模型，(20,20) 的落点与真机一致。
 *   · 右边一列是**这一卡的全部参数行**（含 X/Y 坐标，挤出倍率、刮擦速度）——
 *     行本体由调用方传进来（就是普通的 ParamRow，带 ⓘ 展开详情、还原 chip、搜索高亮），
 *     这里只管摆位：行与地图并排。
 *
 * 坐标系：机器坐标 x 向右、y 向上，原点在可打印区**前左角**；画布的 y 向下（svg 惯例），
 * 两者的换算都收在 bedPointOf / placeStyle 两处。
 *
 * 没移植的两样（参考程序有、这一稿刻意不要）：滚出视口后的悬浮小窗（PIP）、右键菜单
 * —— 回到原位由行上的还原 chip 承担。
 */

import { useCallback, useId, useMemo, useRef, useState } from 'react'
import type { ReactNode, PointerEvent as ReactPointerEvent } from 'react'
import TowerCoreSvg from './TowerCoreSvg'
import type { TowerCoreProps } from './TowerCoreSvg'
import type { Plate } from '../../api'
import s from './TowerMap.module.css'

interface Props {
  /** 热床可打印区尺寸 mm（机器坐标：x 向右、y 向上，原点前左角） */
  bedW: number
  bedD: number
  /** 边缘留白 mm —— 塔推不进去的那一圈 */
  edgeZone: number
  /** 统一包围盒边长 mm（塔体 + 外围结构的最大扩展，算法在调用方） */
  towerSize: number
  /** 包围盒左下角，机器坐标 mm（含拖拽期间的本地值） */
  x: number
  y: number
  savedX: number
  savedY: number
  /** 塔身（brim / 斜肋 / 护套随参数实时变） */
  core: TowerCoreProps
  /** 这台机型的打印板轮廓；没有板规格的机型不给（画布退回圆角矩形） */
  plate?: Plate
  /** 这一卡的全部参数行（含 X/Y）—— 摆在地图右侧，本体是普通 ParamRow */
  side: ReactNode
  /** 一次拖拽手势的落点提交 —— 调用方走 apply，一次 = 一条撤销 */
  onCommit: (x: number, y: number) => void
}

/** 位置落在 0.1mm —— 拖拽是连续量，不收个位小数的话撤销清单里会出现一长串 */
const round1 = (n: number) => Math.round(n * 10) / 10

export default function TowerMap({
  bedW,
  bedD,
  edgeZone,
  towerSize,
  x,
  y,
  savedX,
  savedY,
  core,
  plate,
  side,
  onCommit,
}: Props) {
  const mapRef = useRef<HTMLDivElement | null>(null)
  /** 留白带裁剪用的 clipPath id —— useId 带冒号，url(#) 引用里去掉稳妥 */
  const clipId = useId().replace(/:/g, '')
  /** 拖拽期间的本地位置。提交（= 一条撤销）只发生在松手那一刻，拖的过程不进栈 */
  const [dragPos, setDragPos] = useState<{ x: number; y: number } | null>(null)
  /** 按下那刻点击点与塔心的偏移 —— 拖动时保持这个相对位置，塔不跳到手心 */
  const dragOffsetRef = useRef({ x: 0, y: 0 })

  const shown = dragPos ?? { x, y }
  const offSaved = shown.x !== savedX || shown.y !== savedY

  /*
   * 坐标系统：有板规格时以 frame（真可打印区）为准 —— 上游 bedSize 是涂胶/运动口径
   * （A1 是 260×255，比可打印区 256×256 还大），塔的落点、钳位、留白带全都该活在
   * 可打印区里，跟 X/Y 输入框的喷嘴坐标同一套。没板规格的机型退回 bedSize。
   */
  const bed = useMemo(
    () => (plate ? { w: plate.frame.w, h: plate.frame.h } : { w: bedW, h: bedD }),
    [plate, bedW, bedD],
  )
  const mapW = plate?.w ?? bedW
  const mapD = plate?.d ?? bedD
  const frame = plate?.frame ?? { x: 0, y: 0, w: bedW, h: bedD }
  const frameLeft = (frame.x / mapW) * 100
  const frameTop = (frame.y / mapD) * 100
  const frameW = (frame.w / mapW) * 100
  const frameH = (frame.h / mapD) * 100

  /** 塔不能压边缘留白：左下角夹在 [edgeZone, 床 - edgeZone - 塔] 里 */
  const clamp = useCallback(
    (px: number, py: number) => {
      const maxX = Math.max(edgeZone, bed.w - edgeZone - towerSize)
      const maxY = Math.max(edgeZone, bed.h - edgeZone - towerSize)
      return { x: Math.min(Math.max(px, edgeZone), maxX), y: Math.min(Math.max(py, edgeZone), maxY) }
    },
    [bed, edgeZone, towerSize],
  )

  /** 事件坐标 → 机器坐标 mm（画布 y 向下，机器 y 向上，原点 = frame 左下角） */
  const bedPointOf = (e: ReactPointerEvent<HTMLDivElement>) => {
    const el = mapRef.current
    if (el === null) return null
    const rect = el.getBoundingClientRect()
    return {
      x: ((e.clientX - rect.left) / rect.width) * mapW - frame.x,
      y: frame.y + bed.h - ((e.clientY - rect.top) / rect.height) * mapD,
    }
  }

  const onPointerDown = (e: ReactPointerEvent<HTMLDivElement>) => {
    if (e.button !== 0) return
    e.preventDefault()
    const at = bedPointOf(e)
    if (at === null) return
    const target = clamp(at.x - towerSize / 2, at.y - towerSize / 2)
    dragOffsetRef.current = { x: at.x - (target.x + towerSize / 2), y: at.y - (target.y + towerSize / 2) }
    setDragPos(target)
    mapRef.current?.setPointerCapture(e.pointerId)
  }

  const onPointerMove = (e: ReactPointerEvent<HTMLDivElement>) => {
    if (dragPos === null) return
    const at = bedPointOf(e)
    if (at === null) return
    setDragPos(clamp(at.x - towerSize / 2 - dragOffsetRef.current.x, at.y - towerSize / 2 - dragOffsetRef.current.y))
  }

  const endDrag = (e: ReactPointerEvent<HTMLDivElement>) => {
    if (dragPos === null) return
    const final = dragPos
    setDragPos(null)
    try {
      mapRef.current?.releasePointerCapture(e.pointerId)
    } catch {
      /* 捕获已经丢了（比如 pointercancel 先到）就算了 */
    }
    onCommit(round1(final.x), round1(final.y))
  }

  /* 塔方块：大小 = 包围盒 / 可打印区，位移以方块自身尺寸为基准（translate 的 % 是自己的） */
  const placeStyle = (px: number, py: number): React.CSSProperties => ({
    width: `${(towerSize / bed.w) * 100}%`,
    height: `${(towerSize / bed.h) * 100}%`,
    transform: `translate(${(px / towerSize) * 100}%, ${((bed.h - py - towerSize) / towerSize) * 100}%)`,
  })

  return (
    <div className={s.group}>
      <div className={s.mapCol}>
        <div
          ref={mapRef}
          className={plate !== undefined ? s.mapPlate : s.map}
          style={{ aspectRatio: `${mapW} / ${mapD}` }}
          role="application"
          tabIndex={0}
          aria-label={`擦料塔位置：X ${round1(shown.x)}、Y ${round1(shown.y)}，点击或拖动调整`}
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={endDrag}
          onPointerCancel={endDrag}
        >
          {plate !== undefined && (
            <svg className={s.plate} viewBox={`0 0 ${plate.w} ${plate.d}`} aria-hidden>
              <defs>
                {/* 灰圈沿板身圆角走，但 frame 的方角落在圆弧外面 —— evenodd 会把
                    「frame 之内、板身之外」的角料也涂上，必须按板轮廓裁掉 */}
                <clipPath id={clipId}>
                  <path d={plate.path} />
                </clipPath>
              </defs>
              <path d={plate.path} fillRule="evenodd" />
              {/*
                余量灰圈 = evenodd(板身, 可打印区)，clip 在板轮廓上。
                刻意**不含**卡舌 / 把手；圈内沿与 bedFrame 里的边缘留白灰带同色相接。
              */}
              <path
                className={s.ring}
                fillRule="evenodd"
                clipPath={`url(#${clipId})`}
                d={`${plate.bodyPath} M${frame.x},${frame.y}h${frame.w}v${frame.h}h${-frame.w}Z`}
              />
            </svg>
          )}

          {/* 机器坐标区（frame）：按 frame 摆在板上的真实位置，网格 / 留白 / 塔都在这层 */}
          <div
            className={s.bedFrame}
            style={{ left: `${frameLeft}%`, top: `${frameTop}%`, width: `${frameW}%`, height: `${frameH}%` }}
          >
            {/* 边缘留白（机器 y 向上 → svg y 向下，上下两条翻过来画）。
                viewBox 用 frame 尺寸 —— 用 bedSize 会被 preserveAspectRatio 居中缩放。 */}
            <svg viewBox={`0 0 ${frame.w} ${frame.h}`} className={s.zones} aria-hidden>
              {plate !== undefined && (
                <defs>
                  {/* 板轮廓平移到 frame 坐标系：留白带的方角同样要被圆弧裁掉 */}
                  <clipPath id={`${clipId}-z`}>
                    <path d={plate.path} transform={`translate(${-frame.x}, ${-frame.y})`} />
                  </clipPath>
                </defs>
              )}
              <g clipPath={plate !== undefined ? `url(#${clipId}-z)` : undefined}>
                <rect x={0} y={0} width={frame.w} height={edgeZone} />
                <rect x={0} y={frame.h - edgeZone} width={frame.w} height={edgeZone} />
                <rect x={0} y={edgeZone} width={edgeZone} height={frame.h - 2 * edgeZone} />
                <rect x={frame.w - edgeZone} y={edgeZone} width={edgeZone} height={frame.h - 2 * edgeZone} />
              </g>
            </svg>

            {/* 改过之后，已保存的原位置画一个虚线框 */}
            {offSaved && (
              <div className={s.ghost} style={placeStyle(savedX, savedY)}>
                <svg viewBox="0 0 44 44" className={s.core} preserveAspectRatio="xMidYMid meet">
                  <rect x="1" y="1" width="42" height="42" fill="none" stroke="#d97706" strokeWidth="2.5" strokeDasharray="4 3" rx="2" />
                  <line x1="22" y1="1" x2="22" y2="43" stroke="#d97706" strokeWidth="1.2" strokeDasharray="3 3" />
                  <line x1="1" y1="22" x2="43" y2="22" stroke="#d97706" strokeWidth="1.2" strokeDasharray="3 3" />
                  <text x="22" y="27" fontSize="14" fill="#d97706" textAnchor="middle" fontWeight="bold">
                    原位
                  </text>
                </svg>
              </div>
            )}

            <div className={s.tower} style={placeStyle(shown.x, shown.y)}>
              <TowerCoreSvg {...core} className={s.core} />
            </div>
          </div>
        </div>
        <p className={s.hint}>在热床上点击或拖动调整位置</p>
      </div>

      <div className={s.sideCol}>
        <div className={s.rows}>{side}</div>
      </div>
    </div>
  )
}
