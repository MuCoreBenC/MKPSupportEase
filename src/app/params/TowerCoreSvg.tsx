/*
 * 塔身绘制（A43 移植）—— 从参考程序 mkpsupporte 逐字移植（参数页「擦料塔位置」的塔块）。
 *
 * 原件在共享包 @mkp/param-layout 的 TowerCoreSVG（它自己又是从 Supporte 抽的，
 * 底层的 ribSection / roundingPolygon / offsetPolygon 三个算法则是从 Go 后端
 * internal/tower/polygon.go 移植到前端的）。算法一行没动 —— 画出来的形状必须和
 * 真程序一致。
 *
 * 颜色也照原样保留（塔身 #fcd34d / 外围绿 #10b981）：那是那台程序里塔的既定长相，
 * 不在这一稿重新设计。
 */

import { useMemo } from 'react'

interface Pt {
  x: number
  y: number
}

function removeDuplicatePoints(points: Pt[], epsilon: number): Pt[] {
  if (points.length === 0) return points
  const result: Pt[] = [points[0]]
  for (let i = 1; i < points.length; i++) {
    const prev = result[result.length - 1]
    if (prev === undefined) break
    const dx = points[i].x - prev.x
    const dy = points[i].y - prev.y
    if (Math.sqrt(dx * dx + dy * dy) > epsilon) {
      result.push(points[i])
    }
  }
  return result
}

/** 等距偏移（miter）：Go 后端 polygonOffset 的移植。向内偏移非凸多边形不会自相交 */
function offsetPolygon(polygon: Pt[], offset: number): Pt[] {
  const n = polygon.length
  if (n < 3 || offset === 0) return polygon

  const edges: { ax: number; ay: number; bx: number; by: number }[] = []
  for (let i = 0; i < n; i++) {
    const curr = polygon[i]
    const next = polygon[(i + 1) % n]
    if (curr === undefined || next === undefined) break
    const dx = next.x - curr.x
    const dy = next.y - curr.y
    const len = Math.hypot(dx, dy)
    if (len < 1e-10) {
      edges.push({ ax: curr.x, ay: curr.y, bx: next.x, by: next.y })
      continue
    }
    const nx = dy / len
    const ny = -dx / len
    edges.push({
      ax: curr.x + nx * offset,
      ay: curr.y + ny * offset,
      bx: next.x + nx * offset,
      by: next.y + ny * offset,
    })
  }

  const res: Pt[] = []
  for (let i = 0; i < n; i++) {
    const prev = (i - 1 + n) % n
    const pe = edges[prev]
    const ce = edges[i]
    if (pe === undefined || ce === undefined) break
    const d1x = pe.bx - pe.ax
    const d1y = pe.by - pe.ay
    const d2x = ce.bx - ce.ax
    const d2y = ce.by - ce.ay
    const cross = d1x * d2y - d1y * d2x
    if (Math.abs(cross) < 1e-10) {
      res.push({ x: (pe.bx + ce.ax) / 2, y: (pe.by + ce.ay) / 2 })
      continue
    }
    const t = ((ce.ax - pe.ax) * d2y - (ce.ay - pe.ay) * d2x) / cross
    res.push({ x: pe.ax + t * d1x, y: pe.ay + t * d1y })
  }
  return res
}

/**
 * 圆角多边形：Go 后端 roundingPolygon 的移植。不分内外凸凹统一 roundingDistance，
 * 角度阈值 angleTolDeg 以下的拐角才做圆角。
 */
function roundingPolygon(polygon: Pt[], roundingDistance: number, angleTolDeg: number): Pt[] {
  const n = polygon.length
  if (n < 3) return polygon

  const angleTolRad = (angleTolDeg * Math.PI) / 180
  const cosAngleTol = Math.abs(Math.cos(angleTolRad))
  const arcSegments = 8

  const result: Pt[] = []

  for (let i = 0; i < n; i++) {
    const a = polygon[(i - 1 + n) % n]
    const b = polygon[i]
    const c = polygon[(i + 1) % n]
    if (a === undefined || b === undefined || c === undefined) break

    const abLen = Math.hypot(a.x - b.x, a.y - b.y)
    const bcLen = Math.hypot(b.x - c.x, b.y - c.y)

    if (abLen < 0.001 || bcLen < 0.001) {
      result.push(b)
      continue
    }

    const abx = (b.x - a.x) / abLen
    const aby = (b.y - a.y) / abLen
    const bcx = (c.x - b.x) / bcLen
    const bcy = (c.y - b.y) / bcLen

    let cosangle = abx * bcx + aby * bcy
    cosangle = Math.max(-1, Math.min(1, cosangle))

    const isCCW = abx * bcy - aby * bcx > 0

    if (Math.abs(cosangle) < cosAngleTol) {
      const halfAngle = Math.acos(cosangle) / 2
      if (halfAngle < 0.001) {
        result.push(b)
        continue
      }

      const realRoundingDis = Math.min(roundingDistance, Math.min(abLen / 2.1, bcLen / 2.1))

      const leftX = b.x - abx * realRoundingDis
      const leftY = b.y - aby * realRoundingDis
      const rightX = b.x + bcx * realRoundingDis
      const rightY = b.y + bcy * realRoundingDis

      let dirX = rightX - leftX
      let dirY = rightY - leftY
      const dirLen = Math.hypot(dirX, dirY)
      if (dirLen < 0.001) {
        result.push(b)
        continue
      }
      dirX /= dirLen
      dirY /= dirLen

      let rotX = -dirY
      let rotY = dirX
      if (!isCCW) {
        rotX = -rotX
        rotY = -rotY
      }

      const dis = realRoundingDis / Math.sin(halfAngle)
      const ccx = b.x + rotX * dis
      const ccy = b.y + rotY * dis
      const radius = Math.hypot(leftX - ccx, leftY - ccy)

      let polarStartTheta = Math.atan2(leftY - ccy, leftX - ccx)
      if (polarStartTheta < 0) polarStartTheta += 2 * Math.PI
      let polarEndTheta = Math.atan2(rightY - ccy, rightX - ccx)
      if (polarEndTheta < 0) polarEndTheta += 2 * Math.PI

      let angleRadians = polarEndTheta - polarStartTheta
      if (angleRadians < 0 && isCCW) {
        angleRadians += 2 * Math.PI
      } else if (angleRadians > 0 && !isCCW) {
        angleRadians -= 2 * Math.PI
      }

      for (let j = 0; j < arcSegments; j++) {
        let curAngle = polarStartTheta + (j / arcSegments) * angleRadians
        if (curAngle > 2 * Math.PI) curAngle -= 2 * Math.PI
        else if (curAngle < 0) curAngle += 2 * Math.PI
        result.push({ x: ccx + radius * Math.cos(curAngle), y: ccy + radius * Math.sin(curAngle) })
      }
      result.push({ x: rightX, y: rightY })
    } else {
      result.push(b)
    }
  }
  return result
}

/** 纯净的基础直角多边形：Go 后端 ribSection 的移植（filletWall=false 版本） */
function ribSectionPointsRaw(width: number, depth: number, ribLength: number, ribWidth: number): Pt[] {
  const theta = Math.atan(width / depth)
  const costheta = Math.cos(theta)
  const sintheta = Math.sin(theta)
  const w = ribWidth / 2
  const diag = Math.sqrt(width * width + depth * depth)
  const l = (ribLength - diag) / 2

  const dir1X = width / diag
  const dir1Y = depth / diag
  const perp1X = -dir1Y
  const perp1Y = dir1X

  const dir2X = -width / diag
  const dir2Y = depth / diag
  const perp2X = -dir2Y
  const perp2Y = dir2X

  const res: Pt[] = [
    { x: 0, y: 0 + w / sintheta },
    { x: 0 - dir1X * l + perp1X * w, y: 0 - dir1Y * l + perp1Y * w },
    { x: 0 - dir1X * l - perp1X * w, y: 0 - dir1Y * l - perp1Y * w },
    { x: 0 + w / costheta, y: 0 },
    { x: width - w / costheta, y: 0 },
    { x: width - dir2X * l + perp2X * w, y: 0 - dir2Y * l + perp2Y * w },
    { x: width - dir2X * l - perp2X * w, y: 0 - dir2Y * l - perp2Y * w },
    { x: width, y: 0 + w / sintheta },
    { x: width, y: depth - w / sintheta },
    { x: width + dir1X * l - perp1X * w, y: depth + dir1Y * l - perp1Y * w },
    { x: width + dir1X * l + perp1X * w, y: depth + dir1Y * l + perp1Y * w },
    { x: width - w / costheta, y: depth },
    { x: 0 + w / costheta, y: depth },
    { x: 0 + dir2X * l - perp2X * w, y: depth + dir2Y * l - perp2Y * w },
    { x: 0 + dir2X * l + perp2X * w, y: depth + dir2Y * l + perp2Y * w },
    { x: 0, y: depth - w / sintheta },
  ]
  return removeDuplicatePoints(res, 0.001)
}

// ========== 组件 ==========

export interface TowerCoreProps {
  outerVal: 'brim' | 'rib' | 'sheath'
  isRibVisible: boolean
  ribWidthMm: number
  ribExtraLengthMm: number
  ribFilletWall: boolean
  /** viewBox 左上角（负的扩展量像素）—— 三种外围结构共用同一张画布 */
  vbOffset: number
  vbSize: number
  /** mm → px 的比例（塔身固定画成 44px） */
  vm: number
  brimLoopCount: number
  /** 宿主给 svg 定尺寸用（module class 不能跨文件猜名字） */
  className?: string
}

export default function TowerCoreSvg({
  outerVal,
  isRibVisible,
  ribWidthMm,
  ribExtraLengthMm,
  ribFilletWall,
  vbOffset,
  vbSize,
  vm,
  brimLoopCount,
  className,
}: TowerCoreProps) {
  const coreSize = 44
  const center = coreSize / 2
  const AUX_COLOR = '#10b981'
  const AUX_STROKE = '#047857'
  const coreFill = '#fcd34d'
  const coreStroke = '#d97706'

  const expansionPx = -vbOffset

  const ribPolygons = useMemo(() => {
    if (!isRibVisible) return []

    const baseTowerBodyMm = 19.58
    const baseRibWidth = Math.min(ribWidthMm, baseTowerBodyMm / 2)
    const baseRibLength = baseTowerBodyMm * Math.SQRT2 + ribExtraLengthMm * 2
    const rawBasePts = ribSectionPointsRaw(baseTowerBodyMm, baseTowerBodyMm, baseRibLength, baseRibWidth)

    const lineSpacingMm = 0.8
    const loopCount = 5

    /*
     * 对齐 Go 后端 rounding 的思路，方向改成从内往外：
     * 最内层先做一次圆角，再向外逐层偏移 —— 偏移天然同心，相邻线圈无缝隙。
     */
    const innerFilletPts = ribFilletWall ? roundingPolygon(rawBasePts, 2.0, 30.0) : rawBasePts

    const polys: string[] = []
    let currentPts = innerFilletPts
    for (let i = 0; i < loopCount; i++) {
      const cx = baseTowerBodyMm / 2
      const cy = baseTowerBodyMm / 2
      const polyStr = currentPts
        .map((p) => {
          const px = coreSize / 2 + (p.x - cx) * vm
          const py = coreSize / 2 - (p.y - cy) * vm
          return `${px.toFixed(2)},${py.toFixed(2)}`
        })
        .join(' ')
      polys[i] = polyStr
      if (i < loopCount - 1) {
        currentPts = removeDuplicatePoints(offsetPolygon(currentPts, lineSpacingMm), 0.01)
      }
    }
    return polys
  }, [isRibVisible, ribWidthMm, ribExtraLengthMm, ribFilletWall, vm])

  return (
    <svg
      viewBox={`${vbOffset} ${vbOffset} ${vbSize} ${vbSize}`}
      className={className}
      preserveAspectRatio="xMidYMid meet"
    >
      {/* 最底层的 brim（裙边线圈） */}
      {outerVal === 'brim' && (
        <g fill="none" stroke={AUX_COLOR} strokeWidth={1.5} opacity={0.5}>
          {Array.from({ length: brimLoopCount }).map((_, i) => {
            const inset = (i / (brimLoopCount - 1)) * expansionPx
            return (
              <rect
                key={i}
                x={vbOffset + inset}
                y={vbOffset + inset}
                width={vbSize - inset * 2}
                height={vbSize - inset * 2}
                rx={0}
              />
            )
          })}
        </g>
      )}

      {/* 斜肋线圈层 */}
      {isRibVisible && ribPolygons.length > 0 && (
        <g>
          {ribPolygons[4] !== undefined && (
            <polygon
              points={ribPolygons[4]}
              fill="none"
              stroke="#11e6a2ff"
              strokeWidth={6}
              strokeLinejoin="round"
              opacity={0.15}
            />
          )}
          {ribPolygons.map((pts, i) => {
            const colors = ['#cb740fff', '#f8bc77ff', '#fbd34cff', '#3ca780c2', '#2fcc8d9f']
            const color = colors[i]
            if (color === undefined) return null
            return (
              <polygon key={i} points={pts} fill="none" stroke={color} strokeWidth={2.2} strokeLinejoin="round" />
            )
          })}
        </g>
      )}

      {/* 护套层 */}
      {outerVal === 'sheath' && (
        <g fill={AUX_COLOR} stroke={AUX_STROKE} strokeWidth={2}>
          <rect x={vbOffset} y={vbOffset} width={vbSize} height={vbSize} rx={0} />
          <line x1={vbOffset} y1={vbOffset} x2={0} y2={0} />
          <line x1={vbOffset + vbSize} y1={vbOffset} x2={coreSize} y2={0} />
          <line x1={vbOffset} y1={vbOffset + vbSize} x2={0} y2={coreSize} />
          <line x1={vbOffset + vbSize} y1={vbOffset + vbSize} x2={coreSize} y2={coreSize} />
        </g>
      )}

      {/* 塔身（保持原件的顶层渲染顺序）。斜肋模式下四个角加 1.5px 微圆角藏住尖刺 */}
      <g fill={coreFill} stroke={coreStroke} strokeWidth={2}>
        <rect x={0} y={0} width={coreSize} height={coreSize} rx={outerVal === 'rib' && isRibVisible ? 1.5 : 0} />
      </g>
      <g stroke={coreStroke} strokeWidth={1.5}>
        <line x1={center} y1={0} x2={center} y2={coreSize} />
        <line x1={0} y1={center} x2={coreSize} y2={center} />
      </g>
      <text x={center} y={center + 5} fontSize="15" fontWeight="bold" fill="#92400e" textAnchor="middle">
        塔
      </text>
    </svg>
  )
}
