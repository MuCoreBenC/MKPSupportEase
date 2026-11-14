/*
 * 成员编辑框的草稿数学：加入 / 取消 / 移出 / 最终分组次序 / 候选过滤。
 * 只测纯状态转移 —— 框里的改动在确认之前不碰 DOM，也不发 IPC。
 */

import { describe, expect, it } from 'vitest'

import {
  moveInOrder,
  removeMember,
  sameKeys,
  toggleMember,
  usableSections,
} from './useMembersDraft'

describe('成员草稿', () => {
  it('加入与取消落在同一份序列，且不改原数组', () => {
    const keys = ['a', 'b']
    const added = toggleMember(keys, 'c')
    expect(added).toEqual(['a', 'b', 'c'])
    expect(toggleMember(added, 'c')).toEqual(keys)
    expect(keys).toEqual(['a', 'b'])
    expect(added).not.toBe(keys)
  })

  it('移出只摘掉那一条，没有的那条不动', () => {
    const keys = ['a', 'b', 'c']
    expect(removeMember(keys, 'b')).toEqual(['a', 'c'])
    expect(removeMember(keys, 'z')).toEqual(keys)
  })

  it('恢复与脏判据都对着打开时那一序', () => {
    const initial = ['a', 'b']
    expect(sameKeys(initial, ['a', 'b'])).toBe(true)
    expect(sameKeys(initial, ['b', 'a'])).toBe(false)
    const edited = removeMember(toggleMember(initial, 'c'), 'a')
    expect(sameKeys(edited, initial)).toBe(false)
    expect(sameKeys([...initial], initial)).toBe(true)
  })

  it('加入 → 移出 → 放回的一串动作落在同一份序列', () => {
    const initial = ['a', 'b']
    let keys = toggleMember(initial, 'c')
    expect(keys).toEqual(['a', 'b', 'c'])
    keys = removeMember(keys, 'a')
    expect(keys).toEqual(['b', 'c'])
    keys = toggleMember(keys, 'a')
    expect(keys).toEqual(['b', 'c', 'a'])
    expect(removeMember(keys, 'c')).toEqual(['b', 'a'])
    expect(sameKeys(initial, ['a', 'b'])).toBe(true)
  })

  it('分组最终次序：移到目标位，越界与同位置原样退回', () => {
    const order = ['a', 'b', 'c']
    expect(moveInOrder(order, 2, 0)).toEqual(['c', 'a', 'b'])
    expect(moveInOrder(order, 0, 2)).toEqual(['b', 'c', 'a'])
    expect(moveInOrder(order, 1, 1)).toBe(order)
    expect(moveInOrder(order, -1, 0)).toBe(order)
    expect(moveInOrder(order, 0, 9)).toBe(order)
    expect(order).toEqual(['a', 'b', 'c'])
  })

  it('组件卡与整块卡不进候选', () => {
    const sections = [
      { id: 'normal', component: null, standalone: false },
      { id: 'card', component: 'tower-map', standalone: false },
      { id: 'fixed', component: null, standalone: true },
    ]
    expect(usableSections(sections).map((x) => x.id)).toEqual(['normal'])
  })
})
