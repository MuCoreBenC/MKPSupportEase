import { describe, expect, it } from 'vitest'

import { arrowPathOf, minArrowLength, pieceMoveLength } from './arrowGeometry'

describe('箭头的形状与最小可见长度', () => {
  it('最小可见长度按画布宽的 12% 走，并有一个下限', () => {
    expect(minArrowLength(180)).toBeCloseTo(21.6, 5)
    expect(minArrowLength(10)).toBe(8)
  })

  it('3mm 的位移按真长只有几像素 —— 这就是用户看不到箭头的那支', () => {
    const tiny = arrowPathOf(3, 180)
    expect(tiny).toContain('L 3 0')
    /* 补到最小可见长度后，箭身长度差了 7 倍 */
    expect(minArrowLength(180) / 3).toBeGreaterThan(7)
  })

  it('补到最小可见长度之后能画出完整的一支', () => {
    const path = arrowPathOf(minArrowLength(180), 180)
    expect(path).not.toBeNull()
    expect(path).toContain('L')
  })

  it('长位移照真长画', () => {
    const path = arrowPathOf(60, 180)
    expect(path).toContain('L 60 0')
  })

  it('位移长度就是 shift 的模', () => {
    expect(pieceMoveLength({ shift: [0, -3] })).toBeCloseTo(3, 6)
    expect(pieceMoveLength({ shift: [3, 4] })).toBeCloseTo(5, 6)
    expect(pieceMoveLength({})).toBe(0)
  })
})
