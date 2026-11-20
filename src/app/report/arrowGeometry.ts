/*
 * 箭头的**形状**（2026-11-20）。
 *
 * 只算一支沿 +X 的箭身的 path（调用方再 `translate / rotate` 到起点与方向上）——
 * 这样长度、镞宽、最小可见长度全是纯数，可以单独判据，不用起浏览器。
 *
 * 为什么要「最小可见长度」：越界建议的位移可能是 3mm，按真长画出来只有两三个像素，
 * 用户根本看不到箭头（2026-11-20 原话：「我都看不到那个箭头了。箭头到底在哪里？」）。
 * 太短就画到最小长度 —— 真实距离由右侧那句「沿箭头方向移动 3.0 mm」说。
 */
import type { ReportBoundaryPiece } from '../../api'

/** 箭头在画布上的最短可见长度（画布宽的 12%，A1 mini ≈ 21mm） */
export const MIN_ARROW_RATIO = 0.12

/** 一支箭的最短长度 */
export function minArrowLength(canvasW: number): number {
  return Math.max(8, canvasW * MIN_ARROW_RATIO)
}

/**
 * 箭身 path（沿 +X，起点在原点）；长度连镞都放不下就 `null`。
 *
 * 镞与镞宽都按画布宽取，与碰撞那屏同一套比例（小图上不糊、放大后不细）。
 */
export function arrowPathOf(len: number, canvasW: number): string | null {
  const head = Math.min(Math.max(4, canvasW * 0.05), len * 0.45)
  const w0 = Math.min(Math.max(3, canvasW * 0.02), len * 0.3)
  if (len <= head * 1.4) return null
  const w1 = w0 * 0.55
  const hw = head * 1.15
  const body = len - head
  return (
    `M 0 ${-w0 / 2} L ${body} ${-w1 / 2} L ${body} ${-hw / 2} L ${len} 0` +
    ` L ${body} ${hw / 2} L ${body} ${w1 / 2} L 0 ${w0 / 2} Z`
  )
}

/** 这块模型的建议位移长度（mm）—— 箭头按它画（不够长时由调用方补到最小可见长度） */
export function pieceMoveLength(piece: ReportBoundaryPiece): number {
  const s = piece.shift ?? [0, 0]
  return Math.hypot(s[0], s[1])
}
