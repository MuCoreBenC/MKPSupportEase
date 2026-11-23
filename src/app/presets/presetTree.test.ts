import { describe, expect, it } from 'vitest'

import {
  branchKept,
  branchMatches,
  branchOf,
  familyLayoutOf,
  isMainCopy,
  slicerCloudActionOf,
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

/*
 * 族视图（A）：母本在前、副本缩进。
 *
 * 判据只认 `originPath` —— 副本改名（`夜班.toml`）也还得挂在 `A1.toml` 下面；
 * 这是 `copyRankOf` 那套按名字猜做不到的。
 */
describe('族视图（A）', () => {
  const row = (fileName: string, origin: string | null = null, pinned = false) => ({
    rowKey: `local::${fileName}`,
    fileName,
    path: fileName,
    pinned,
    originPath: origin,
    originName: origin,
  })

  it('改过名的副本照样挂在母本下面，块首有分隔、末行收口', () => {
    const out = familyLayoutOf([row('A1.toml'), row('夜班.toml', 'A1.toml')])
    expect(out.map((x) => [x.row.fileName, x.meta?.depth, x.meta?.tagName])).toEqual([
      ['A1.toml', 0, null],
      ['夜班.toml', 1, null],
    ])
    expect(out[0].meta?.start).toBe(true)
    expect(out[0].meta?.last).toBe(false)
    expect(out[1].meta?.last).toBe(true)
  })

  it('母本不在可见行里：副本自己当块首，并挂母本签', () => {
    const out = familyLayoutOf([row('客户演示版.toml', 'A1.toml')])
    expect(out[0].meta?.lead).toBe(true)
    expect(out[0].meta?.tagName).toBe('A1')
  })

  it('置顶的副本脱开母本单独成块；母本留在原处', () => {
    /* 输入序 = 排好序的（置顶的在最前） */
    const out = familyLayoutOf([row('夜班.toml', 'A1.toml', true), row('A1.toml')])
    expect(out.map((x) => x.row.fileName)).toEqual(['夜班.toml', 'A1.toml'])
    expect(out[0].meta?.tagName).toBe('A1')
    expect(out[0].meta?.start).toBe(false)
    expect(out[1].meta?.start).toBe(false)
  })

  it('母本也置顶时，置顶的副本仍跟着母本（不拆成两块）', () => {
    const out = familyLayoutOf([row('A1.toml', null, true), row('夜班.toml', 'A1.toml', true)])
    expect(out.map((x) => x.row.fileName)).toEqual(['A1.toml', '夜班.toml'])
    expect(out[1].meta?.depth).toBe(1)
  })

  it('副本的副本只缩进一级，名字不带母本名时挂签；老 copy 名不挂', () => {
    const out = familyLayoutOf([
      row('A1.toml'),
      row('A1 copy.toml', 'A1.toml'),
      row('我的夜班.toml', 'A1 copy.toml'),
      row('A1 copy copy.toml', 'A1 copy.toml'),
    ])
    const mine = out.find((x) => x.row.fileName === '我的夜班.toml')
    const old = out.find((x) => x.row.fileName === 'A1 copy copy.toml')
    expect(mine?.meta?.depth).toBe(2)
    expect(mine?.meta?.tagName).toBe('A1 copy')
    /* 每根枝自己收口：老 copy 是 A1 copy 的最后一个孩子 */
    expect(mine?.meta?.last).toBe(false)
    expect(old?.meta?.depth).toBe(2)
    expect(old?.meta?.tagName).toBeNull()
    expect(old?.meta?.last).toBe(true)
  })
})

/*
 * 切片器档云端行的那一个动作。
 *
 * 这一档没有 MKP 的 `localState` 那本账；从前菜单直接问 `updateStatusOf`，于是落到
 * "状态未知"的兜底上，念成「云端较旧」并把下载禁掉（同一行操作列却给着「下载」）。
 */
describe('切片器云端行的动作', () => {
  it('本机没有 ⇒ 下载', () => {
    expect(slicerCloudActionOf({ presetState: 'missing' })).toEqual({
      action: 'missing',
      gotIt: false,
      older: false,
    })
  })

  it('下了且与目录一致 ⇒ 不给动作（已下载）', () => {
    expect(slicerCloudActionOf({ presetState: 'ok' })).toEqual({
      action: 'none',
      gotIt: true,
      older: false,
    })
  })

  it('盘上是旧版 / 认不出 ⇒ 更新', () => {
    expect(slicerCloudActionOf({ presetState: 'old' }).action).toBe('update')
    expect(slicerCloudActionOf({ presetState: 'tampered' }).action).toBe('update')
  })

  it('云端比本机旧 ⇒ 不给动作，也不许念成"可以下载"', () => {
    const r = slicerCloudActionOf({ presetState: 'old', basedOn: 'newer' })
    expect(r.action).toBe('none')
    expect(r.older).toBe(true)
  })

  it('没有 presetState 时看 downloaded 那一位', () => {
    expect(slicerCloudActionOf({ downloaded: true }).gotIt).toBe(true)
    expect(slicerCloudActionOf({ downloaded: false }).action).toBe('missing')
  })
})
