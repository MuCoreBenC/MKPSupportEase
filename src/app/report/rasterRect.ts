/*
 * 走线投影那张 PNG 该贴在画布的哪一块。
 *
 * 内核给的是机器坐标（左下角 + 一格多少毫米 + 像素数）；画布用的是 `bedPt` 换算后的
 * 板坐标（y 已经翻到屏幕方向）。这里只做这一件事，单独成函数才测得动。
 */
import type { ReportRaster } from '../../api'

export interface RasterRect {
  x: number
  y: number
  w: number
  h: number
}

/**
 * `bedPt` 是画布那套「机器坐标 → 板坐标」的换算（含 y 翻转）。
 * 图片的 `y` 是左上角：机器坐标的上沿 = `yMin + height * cellMm`。
 */
export function rasterScreenRect(
  raster: ReportRaster,
  bedPt: (x: number, y: number) => [number, number],
): RasterRect {
  const wMm = raster.width * raster.cellMm
  const hMm = raster.height * raster.cellMm
  const [x, y] = bedPt(raster.xMin, raster.yMin + hMm)
  const [x2, y2] = bedPt(raster.xMin + wMm, raster.yMin)
  return { x, y, w: x2 - x, h: y2 - y }
}
