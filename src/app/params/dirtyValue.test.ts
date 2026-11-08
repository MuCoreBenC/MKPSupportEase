import { describe, expect, it } from 'vitest'

import { isDirtyValue } from './dirtyValue'

describe('isDirtyValue —— 「脏」只看值，不看键在不在', () => {
  it('没有草稿 ⇒ 不脏', () => {
    expect(isDirtyValue(undefined, '1.2')).toBe(false)
  })

  it('草稿与保存值不同 ⇒ 脏', () => {
    expect(isDirtyValue('1.3', '1.2')).toBe(true)
  })

  it('草稿与保存值相同 ⇒ 不脏（改回原值当场就不脏，不必等 apply 撤键）', () => {
    expect(isDirtyValue('1.2', '1.2')).toBe(false)
  })

  it('空串与原值不同照样算脏（清空也是改过）', () => {
    expect(isDirtyValue('', '1.2')).toBe(true)
  })
})
