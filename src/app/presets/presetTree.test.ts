import { describe, expect, it } from 'vitest'

import {
  branchKept,
  branchMatches,
  branchOf,
  isMainCopy,
  updateStatusOf,
} from './presetTree'

/*
 * 分支隔离的三条判据。
 *
 * 模型：每个分支的下载各落一个文件夹；"我这儿有没有这一支的"只看同分支的那几份。
 * 其它分支的文件仍在盘上（切筛选看得到），但**不冒充**当前这一支已有。
 */
describe('分支隔离', () => {
  it('从来源记录里取出分支名；没有 ref（自建源）给 null', () => {
    expect(branchOf({ sourceRef: 'github#o/r@main' })).toBe('main')
    expect(branchOf({ sourceRef: 'github#o/r@feat/pen' })).toBe('feat/pen')
    expect(branchOf({ sourceRef: 'custom#127.0.0.1:8787@-' })).toBeNull()
    expect(branchOf({ sourceRef: null })).toBeNull()
    expect(branchOf(null)).toBeNull()
  })

  it('两个分支互不认领；没有来源的老副本照旧参与', () => {
    expect(branchMatches('main', 'delivery')).toBe(false)
    expect(branchMatches('main', 'main')).toBe(true)
    /* 导入 / 老副本没有来源记录 —— 不属于任何一支，不因分支不同被排除 */
    expect(branchMatches(null, 'main')).toBe(true)
    /* 当前源没有 ref（自建源）⇒ 不隔离 */
    expect(branchMatches('main', null)).toBe(true)
  })

  it('本地表三档：当前分支带「没有来源」/ 全部 / 没有来源', () => {
    /* 当前分支：自己这一支留着，别的分支藏起来 */
    expect(branchKept('main', true, 'current', 'main')).toBe(true)
    expect(branchKept('delivery', true, 'current', 'main')).toBe(false)
    /* 但"没有来源"的不能被藏 —— 藏了等于说用户那份不见了 */
    expect(branchKept(null, false, 'current', 'main')).toBe(true)

    expect(branchKept('delivery', true, 'all', 'main')).toBe(true)
    expect(branchKept(null, false, 'all', 'main')).toBe(true)

    expect(branchKept(null, false, 'unfiled', 'main')).toBe(true)
    expect(branchKept('main', true, 'unfiled', 'main')).toBe(false)
  })

  it('自建源：记着来源但没有分支，归「当前」，不进「没有来源」', () => {
    /* `custom#127.0.0.1:8787@-` → ref 是 null，但来源记录在 */
    expect(branchKept(null, true, 'unfiled', null)).toBe(false)
    expect(branchKept(null, true, 'current', null)).toBe(true)
  })
})

/*
 * 「动作 × 决定 → 状态」的前端半边（规格表见 `docs/PRESET-UPDATE-SPEC.md`）。
 *
 * 状态只有一个出口：`updateStatusOf`。表里每一行都对着它的一档 ——
 * 改行为必须先改表，表与这里对不上就是红。
 */
describe('云端行的状态', () => {
  const cases: {
    why: string
    row: Parameters<typeof updateStatusOf>[0]
    action: 'download' | 'update' | 'none'
  }[] = [
    { why: '本机没有这一份', row: { localState: 'none' }, action: 'download' },
    { why: '本地基于旧版', row: { localState: 'stale', basedOn: 'outdated' }, action: 'update' },
    {
      why: '发布前进了但这一份没变（允许重新下载补齐）',
      row: { localState: 'stale', basedOn: 'unchanged' },
      action: 'update',
    },
    { why: '已是最新', row: { localState: 'has', basedOn: 'current' }, action: 'none' },
    {
      why: '别的分支下载的那份（分支隔离后不再认领）',
      row: { localState: 'stale', basedOn: 'elsewhere' },
      action: 'none',
    },
    {
      why: '云端比本机旧（不给更新，那是往回带）',
      row: { localState: 'stale', basedOn: 'newer' },
      action: 'none',
    },
    {
      why: '目录自相矛盾',
      row: { localState: 'stale', basedOn: 'inconsistent' },
      action: 'none',
    },
    {
      why: '说不清版本关系',
      row: { localState: 'stale', basedOn: 'unverified' },
      action: 'none',
    },
    {
      why: '用户自己拷出来的独立一份',
      row: { localState: 'stale', basedOn: 'outdated', detached: true },
      action: 'none',
    },
    { why: '切片器档没有这一档判定', row: {}, action: 'none' },
  ]

  for (const c of cases) {
    it(c.why, () => {
      expect(updateStatusOf(c.row).action).toBe(c.action)
    })
  }
})

describe('主份与副本', () => {
  it('下载建的那份是主份；复制 / 导入不是', () => {
    expect(isMainCopy({ provenanceKind: 'download' })).toBe(true)
    expect(isMainCopy({ provenanceKind: 'copy' })).toBe(false)
    expect(isMainCopy({ provenanceKind: 'import' })).toBe(false)
  })

  it('没有来路账的老文件当主份（否则云端会显示"未下载"）', () => {
    expect(isMainCopy({ provenanceKind: null })).toBe(true)
    expect(isMainCopy({})).toBe(true)
  })
})
