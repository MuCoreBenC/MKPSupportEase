/*
 * 下方支撑余量（lab 试验判据）。
 *
 * 回答一件事：一条涂胶线落下去，笔下面是实的还是空的、离实的边界还有多远。
 * 取「动作之前最后打印的那一层」的支撑材料铺成覆盖图（被围住的孔填平），
 * 再对每条移动取样，量到外边界的最小距离。
 *
 * 与 `band.ts` 的「边缘带」并列、不覆盖它：边缘带看离涂胶区域边界多远，
 * 这里看离下方材料边界多远。判据口径见 docs/GLUE-UNDER-SUPPORT-2026-11-16.md。
 */

import { EdgeField } from './band'
import type { Stroke } from './band'
import type { Point, PrintModel, PrintKind } from './parse'

/** 认作承托面的类别（支撑材料；不含零件本体） */
export const UNDER_KINDS: readonly PrintKind[] = ['support', 'supportFace', 'ironing']

/**
 * 判「孔里有没有料」用的类别：整层的挤出（支撑 + 零件本体）。
 *
 * 支撑铺覆盖区；这一份只回答「这个孔里是不是真有东西」。有料才填平，
 * 真空气孔留着 —— 涂胶走过去照样按贴边处理。
 */
export const UNDER_MATERIAL_KINDS: readonly PrintKind[] = [
  'outer',
  'inner',
  'fill',
  'support',
  'supportFace',
  'ironing',
]

export interface UnderCoverage {
  field: EdgeField
  /** 每格有没有料（1 = 有）；画覆盖层用，建一次就够 */
  mask: Uint8Array
  /** 承托层自己的层高（mm） */
  layerZ: number
  minX: number
  minY: number
  maxX: number
  maxY: number
}

/** 把一层支撑的线段铺成覆盖图。点太少或退化成一条线时返回 `null` */
export function buildUnder(
  strokes: readonly Stroke[],
  material: readonly Stroke[],
  layerZ: number,
): UnderCoverage | null {
  const field = EdgeField.build(strokes, { material })
  if (field === null) return null
  return {
    field,
    mask: field.coverageMask(),
    layerZ,
    minX: field.x0,
    minY: field.y0,
    maxX: field.x0 + field.nx * field.cellMm,
    maxY: field.y0 + field.ny * field.cellMm,
  }
}

/** 小于 `z` 的最近一层在 `print.layers` 里的下标；一层都没有时返回 -1 */
export function layerIndexBelow(print: PrintModel, z: number): number {
  let best = -1
  for (let i = 0; i < print.layers.length; i += 1) {
    if ((print.layers[i] as number) < z - 1e-9) best = i
  }
  return best
}

/**
 * 取某一层里指定类别的全部线段（跨对象）。
 *
 * `bounds[kind][i] … bounds[kind][i+1]` 是这一层的段数区间，乘 6 是顶点数区间
 * （一段两个端点 × xyz）；同一层可能有好几个对象，全部并进来。
 */
export function layerStrokes(
  print: PrintModel,
  layerIndex: number,
  kinds: readonly PrintKind[],
): Stroke[] {
  const out: Stroke[] = []
  if (layerIndex < 0) return out
  for (const obj of print.objects) {
    for (const kind of kinds) {
      const data = obj.kinds[kind]
      const edge = obj.bounds[kind]
      const from = (edge[layerIndex] ?? 0) * 6
      const to = (edge[layerIndex + 1] ?? 0) * 6
      for (let i = from; i + 5 < to; i += 6) {
        out.push({
          from: { x: data[i] as number, y: data[i + 1] as number },
          to: { x: data[i + 3] as number, y: data[i + 4] as number },
        })
      }
    }
  }
  return out
}

/** 这个点到覆盖外边界的余量（mm）：>0 在里面，0 贴边或落在外面 */
export function marginAt(cov: UnderCoverage, p: Point): number {
  return cov.field.faceId(p) === 0 ? 0 : cov.field.edgeDistance(p)
}

/** 一条移动的最小余量：按 0.2mm 采样，两端都算 */
export function minMarginOfMove(cov: UnderCoverage, from: Point, to: Point): number {
  const len = Math.hypot(to.x - from.x, to.y - from.y)
  const steps = Math.max(1, Math.min(200, Math.ceil(len / 0.2)))
  let best = Number.POSITIVE_INFINITY
  for (let k = 0; k <= steps; k += 1) {
    const t = k / steps
    const d = marginAt(cov, { x: from.x + (to.x - from.x) * t, y: from.y + (to.y - from.y) * t })
    if (d < best) best = d
  }
  return Number.isFinite(best) ? best : 0
}
