/*
 * 「0 = 自动」那一格的显示判据。
 *
 * 三件事分开验：
 *   ① 算得出就写数（`自动（6.50 mm）`），算不出就只写「自动」；
 *   ② 数从注册表登记的 `autoValue` 来 —— 取不取绝对值、加多少都照登记走；
 *   ③ 说的是**历史值**时不拿今天的来源值顶（不给 `lookup` 就只写「自动」）。
 */

import { describe, expect, it } from 'vitest'
import type { TextDef } from './valueText'
import { valueText } from './valueText'
import type { Tr } from '../../i18n'

/** 只认这三条键的假 tr：别的键原样回显，免得测试依赖整本字典 */
const tr: Tr = ((key: string, vars?: Record<string, unknown>) => {
  if (key === 'params.row.auto') return '自动'
  if (key === 'params.row.autoWith') return `自动（${String(vars?.value)}）`
  if (key === 'params.value.empty') return '空'
  return key
}) as Tr

function def(over: Partial<TextDef['field']> = {}): TextDef {
  return {
    unit: 'mm',
    field: {
      key: 'toolhead.MKP_extrude',
      label: '挤出长度',
      control: 'number',
      unit: 'mm',
      step: 0.1,
      zeroMeansAuto: true,
      autoValue: { from: 'toolhead.MKP_retract', abs: true, offset: 0.5 },
      ...over,
    },
  }
}

/** 那条被跟随的参数现在是多少 */
const retract = (raw: string) => (key: string) => (key === 'toolhead.MKP_retract' ? raw : '')

describe('valueText · 0 = 自动', () => {
  it('算得出就写数：|回抽| + 0.5，按 mm 两位', () => {
    expect(valueText(def(), '0', tr, retract('-6'))).toBe('自动（6.50 mm）')
    expect(valueText(def(), '0', tr, retract('-6.25'))).toBe('自动（6.75 mm）')
  })

  it('非零就写原值，不进自动那一支', () => {
    expect(valueText(def(), '2.5', tr, retract('-6'))).toBe('2.50 mm')
  })

  it('算不出（没有来源解析器 / 来源为空 / 说不清）就只写「自动」', () => {
    expect(valueText(def(), '0', tr)).toBe('自动')
    expect(valueText(def(), '0', tr, retract(''))).toBe('自动')
    expect(valueText(def(), '0', tr, retract('abc'))).toBe('自动')
    expect(valueText(def({ autoValue: undefined }), '0', tr, retract('-6'))).toBe('自动')
  })

  it('取不取绝对值、加多少都照注册表登记走', () => {
    expect(
      valueText(def({ autoValue: { from: 'toolhead.MKP_retract' } }), '0', tr, retract('-6')),
    ).toBe('自动（-6.00 mm）')
    expect(
      valueText(
        def({ autoValue: { from: 'toolhead.MKP_retract', abs: true, offset: 1.25 } }),
        '0',
        tr,
        retract('-6'),
      ),
    ).toBe('自动（7.25 mm）')
  })
})
