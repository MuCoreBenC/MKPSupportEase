/*
 * 成员编辑框的草稿数学：加入 / 取消 / 移出 / 恢复 / 候选过滤。
 * 只测纯状态转移 —— 框里的改动在确认之前不碰 DOM，也不发 IPC。
 */

import { describe, expect, it } from 'vitest'

import {
  applyMoves,
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

  it('换位净位移摊回显示顺序，到头就停', () => {
    const rows = [{ id: 'a' }, { id: 'b' }, { id: 'c' }]
    expect(applyMoves(rows, [{ sectionId: 'c', steps: -2 }]).map((r) => r.id)).toEqual([
      'c', 'a', 'b',
    ])
    expect(applyMoves(rows, [{ sectionId: 'a', steps: 9 }]).map((r) => r.id)).toEqual([
      'b', 'c', 'a',
    ])
    expect(applyMoves(rows, [{ sectionId: 'a', steps: -1 }]).map((r) => r.id)).toEqual([
      'a', 'b', 'c',
    ])
    expect(applyMoves(rows, []).map((r) => r.id)).toEqual(['a', 'b', 'c'])
    expect(rows.map((r) => r.id)).toEqual(['a', 'b', 'c'])
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
