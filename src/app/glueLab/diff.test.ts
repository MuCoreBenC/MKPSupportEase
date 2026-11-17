/*
 * 行级对齐：钉住「插入整段 → 全部新增后接回原文」「删几行 → 全部删除后接回」
 * 「同位置数值改写 → 已改」这三条，别的一律用计数与映射核对。
 */

import { describe, expect, it } from 'vitest'

import { DIFF_ADDED, DIFF_CHANGED, DIFF_REMOVED, DIFF_SAME, alignLines } from './diff'

const lines = (n: number, step = 1): string[] =>
  Array.from({ length: n }, (_, i) => `G1 X${i * step} Y${i * step} E0.${i}`)

describe('alignLines', () => {
  it('一模一样：逐行对上，没有差异', () => {
    const text = lines(20)
    const diff = alignLines(text, text)
    for (let i = 0; i < text.length; i += 1) {
      expect(diff.origOf[i]).toBe(i)
      expect(diff.afterOf[i]).toBe(i)
      expect(diff.afterKind[i]).toBe(DIFF_SAME)
    }
    expect(diff.stats).toEqual({ added: 0, removed: 0, changed: 0 })
  })

  it('中间插一整段：插入行全记新增，后面接回原行', () => {
    const before = lines(20)
    const block = Array.from({ length: 300 }, (_, i) => `; glue ${i}`)
    const after = [...before.slice(0, 10), ...block, ...before.slice(10)]

    const diff = alignLines(before, after)
    expect(diff.stats.added).toBe(300)
    expect(diff.stats.removed).toBe(0)
    for (let k = 0; k < 300; k += 1) {
      expect(diff.afterKind[10 + k]).toBe(DIFF_ADDED)
      expect(diff.origOf[10 + k]).toBe(-1)
    }
    expect(diff.origOf[10 + 300]).toBe(10)
    expect(diff.origOf[after.length - 1]).toBe(19)
    expect(diff.afterOf[19]).toBe(after.length - 1)
  })

  it('删掉几行：原件侧记删除，产物侧直接接上', () => {
    const before = lines(30)
    const after = [...before.slice(0, 12), ...before.slice(15)]
    const diff = alignLines(before, after)

    expect(diff.stats.removed).toBe(3)
    expect(diff.stats.added).toBe(0)
    for (let k = 12; k < 15; k += 1) expect(diff.beforeKind[k]).toBe(DIFF_REMOVED)
    expect(diff.afterOf[15]).toBe(12)
    expect(diff.origOf[12]).toBe(15)
  })

  it('同位置数值改写：两边都记已改，映射仍旧对得上', () => {
    const before = ['G1 X1 Y1 E0.10000', 'G1 X2 Y2 E0.20000', 'G1 X3 Y3 E0.30000']
    const after = ['G1 X1 Y1 E0.10000', 'G1 X2 Y2 E0.20000', 'G1 X3 Y3 E0.30000']
    const diff = alignLines(before, after)
    expect(diff.stats.changed).toBe(0)

    const changedAfter = ['G1 X1 Y1 E0.10000', 'G1 X2 Y2 E0.20000', 'G1 X3 Y3 E0.33333']
    const d2 = alignLines(before, changedAfter)
    expect(d2.beforeKind[2]).toBe(DIFF_CHANGED)
    expect(d2.afterKind[2]).toBe(DIFF_CHANGED)
    expect(d2.origOf[2]).toBe(2)
    expect(d2.stats.changed).toBe(1)
  })

  it('重复行很多时插入仍能接回（`G1 Z0.2` 这种）', () => {
    const before = [
      ...Array.from({ length: 40 }, () => 'G1 Z0.2'),
      'G1 X1 Y1 E0.1',
      ...Array.from({ length: 40 }, () => 'G1 Z0.2'),
      'G1 X9 Y9 E0.9',
    ]
    const after = [
      ...before.slice(0, 41),
      ';MKP_BEGIN paint ev=1 z=0.2',
      'G1 X5 Y5',
      ';MKP_END paint',
      ...before.slice(41),
    ]
    const diff = alignLines(before, after)
    expect(diff.stats.added).toBe(3)
    expect(diff.origOf[44]).toBe(41)
    expect(diff.origOf[after.length - 1]).toBe(81)
  })

  it('空文件：两侧都空、只缺一侧都不抛错', () => {
    const none = alignLines([], [])
    expect(none.origOf.length).toBe(0)
    expect(none.stats).toEqual({ added: 0, removed: 0, changed: 0 })

    const onlyAfter = alignLines([], ['G1 X1 Y1'])
    expect(onlyAfter.afterKind[0]).toBe(DIFF_ADDED)
    expect(onlyAfter.origOf[0]).toBe(-1)
    expect(onlyAfter.anchorOf[0]).toBe(-1)

    const onlyBefore = alignLines(['G1 X1 Y1'], [])
    expect(onlyBefore.beforeKind[0]).toBe(DIFF_REMOVED)
    expect(onlyBefore.stats.removed).toBe(1)
  })

  it('新增行沿用上一处匹配的位置（原件停在哪）', () => {
    const before = lines(10)
    const after = [before[0] as string, '; 新行', before[1] as string]
    const diff = alignLines(before, after)
    expect(diff.origOf[1]).toBe(-1)
    expect(diff.anchorOf[1]).toBe(0)
    expect(diff.anchorOf[2]).toBe(1)
  })
})
