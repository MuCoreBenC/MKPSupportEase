import { describe, expect, it } from 'vitest'

import { formulaOf } from './calibLift'

/*
 * 算式与内核 `calibration_safe_z` 是同一个加法：
 *
 *   挂 / 收笔那句 `G1 Z` = 首层 + 笔尖偏移 + 校准抬升高度
 *
 * 设置页把它摊开给人看，所以这里钉住三件事：加数顺序、尾零的处理、
 * 以及"改抬升量时等号右边跟着变"（作者 2026-11-19 的原话）。
 */
describe('校准抬升高度的算式', () => {
  it('首层 + 笔偏移 + 抬升量 = 挂收笔那句 G1 Z', () => {
    expect(formulaOf({ firstLayer: 0.2, penOffset: 4.1 }, 10)).toBe('0.2 + 4.1 + 10 = 14.3 mm')
  })

  it('改抬升量只换第三项与结果', () => {
    expect(formulaOf({ firstLayer: 0.2, penOffset: 4.1 }, 12)).toBe('0.2 + 4.1 + 12 = 16.3 mm')
    expect(formulaOf({ firstLayer: 0.2, penOffset: 4.1 }, 20)).toBe('0.2 + 4.1 + 20 = 24.3 mm')
  })

  it('笔偏移是 0 或整数时不留尾零', () => {
    expect(formulaOf({ firstLayer: 0.2, penOffset: 0 }, 10)).toBe('0.2 + 0 + 10 = 10.2 mm')
    expect(formulaOf({ firstLayer: 0.2, penOffset: 4 }, 10)).toBe('0.2 + 4 + 10 = 14.2 mm')
  })

  it('浮点尾巴不进算式（0.1 + 0.2 那种）', () => {
    expect(formulaOf({ firstLayer: 0.3, penOffset: 1.1 }, 6)).toBe('0.3 + 1.1 + 6 = 7.4 mm')
  })
})
