import { describe, expect, it } from 'vitest'

import type { ReportRaster } from '../../api'
import { rasterScreenRect } from './rasterRect'

/* 一台 256×256 的板，画布按 1mm = 1px，y 翻到屏幕方向（与 `bedPt` 同一条） */
const bedPt = (x: number, y: number): [number, number] => [x, 256 - y]

const raster: ReportRaster = {
  xMin: 10,
  yMin: 20,
  cellMm: 0.2,
  width: 50,
  height: 25,
  pngBase64: '',
}

describe('走线投影的贴图矩形', () => {
  it('宽高按「像素数 × 格距」算', () => {
    const r = rasterScreenRect(raster, bedPt)
    expect(r.w).toBeCloseTo(10, 6) // 50 × 0.2
    expect(r.h).toBeCloseTo(5, 6) // 25 × 0.2
  })

  it('左上角 = 机器坐标的上沿（y 翻转）', () => {
    const r = rasterScreenRect(raster, bedPt)
    expect(r.x).toBeCloseTo(10, 6)
    expect(r.y).toBeCloseTo(256 - (20 + 5), 6)
  })

  it('贴图底边落在 yMin 上（图片不上下颠倒）', () => {
    const r = rasterScreenRect(raster, bedPt)
    expect(r.y + r.h).toBeCloseTo(256 - 20, 6)
  })
})
