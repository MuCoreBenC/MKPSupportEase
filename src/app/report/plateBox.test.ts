import { describe, expect, it } from 'vitest'

import { outlineBounds, shouldShowMeasuredBox } from './plateBox'

/* 2026-11-24 真机那条：轮廓 100.5…158 × 101…158，实测 101.483…157.063 × 101.8…157.38 */
const realOutline: [number, number][][] = [
  [
    [100.5, 101.0],
    [158.0, 101.0],
    [158.0, 118.0],
    [136.0, 120.5],
    [119.5, 119.5],
    [120.5, 136.5],
    [118.0, 158.0],
    [100.5, 158.0],
  ],
]
const realMeasured: [number, number, number, number] = [101.483, 157.063, 101.8, 157.38]

describe('校准画布上的实测包围盒画不画', () => {
  it('轮廓外接框取全部点', () => {
    expect(outlineBounds(realOutline)).toEqual({
      xMin: 100.5,
      xMax: 158,
      yMin: 101,
      yMax: 158,
    })
  })

  it('真机那条：两者差不到 1mm ⇒ 只画轮廓，不画框', () => {
    expect(shouldShowMeasuredBox(realMeasured, realOutline)).toBe(false)
  })

  it('差得多（收缩 / 轮廓被裁）⇒ 框要留着，两条线说的是不同的事', () => {
    expect(shouldShowMeasuredBox([90, 150, 95, 150], realOutline)).toBe(true)
  })

  it('没有轮廓（旧记录 / 空）⇒ 框是图上唯一的几何，照画', () => {
    expect(shouldShowMeasuredBox(realMeasured, [])).toBe(true)
  })
})
