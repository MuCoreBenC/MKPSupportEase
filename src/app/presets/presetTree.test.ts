import { describe, expect, it } from 'vitest'

import type { LocalPresetFile } from '../../api/contract'
import {
  familyLayoutOf,
  filterCountsOf,
  isMainCopy,
  keyOfSourceFilter,
  slicerCloudActionOf,
  sourceKept,
  sourceKeyOf,
  sourceLabelOf,
  sourceMatches,
  sourceValueOf,
  updateStatusOf,
} from './presetTree'

/*
 * 来源隔离：本地表只认完整来源身份，不再只看 ref。
 * 同一个 `main` 也可能来自不同仓库、不同自建源地址或目录，默认各自分开。
 */
describe('来源隔离', () => {
  it('取的是完整来源身份；自建源连同地址与路径一起留着', () => {
    expect(sourceKeyOf({ sourceRef: 'github#o/r@main' })).toBe('github#o/r@main')
    expect(sourceKeyOf({ sourceRef: 'gitee#o/r@main' })).toBe('gitee#o/r@main')
    expect(sourceKeyOf({ sourceRef: 'custom#127.0.0.1:8787/delivery@-' })).toBe(
      'custom#127.0.0.1:8787/delivery@-',
    )
    expect(sourceKeyOf({ sourceRef: null })).toBeNull()
    expect(sourceKeyOf(null)).toBeNull()
  })

  it('显示名不露内部名 custom，自建源念地址与路径', () => {
    expect(sourceLabelOf('github#o/r@main')).toBe('GitHub · o/r · main')
    expect(sourceLabelOf('gitee#o/r@main')).toBe('Gitee · o/r · main')
    expect(sourceLabelOf('custom#127.0.0.1:8787/delivery@-')).toBe('127.0.0.1:8787/delivery')
  })

  it('同一个 ref、不同仓库或自建源地址都互不认领；无来源照旧参与', () => {
    const current = 'github#o/r@main'
    expect(sourceMatches(current, current)).toBe(true)
    expect(sourceMatches('github#other/r@main', current)).toBe(false)
    expect(sourceMatches('gitee#o/r@main', current)).toBe(false)
    expect(
      sourceMatches('custom#127.0.0.1:8787/delivery@-', 'custom#127.0.0.1:8788/delivery@-'),
    ).toBe(false)
    expect(sourceMatches(null, current)).toBe(true)
    expect(sourceMatches(current, null)).toBe(true)
  })

  it('四档筛选：当前来源带「没有来源」/ 全部 / 没有来源 / 指定来源', () => {
    const current = 'github#o/r@main'
    const other = 'gitee#o/r@main'
    expect(sourceKept(current, true, 'current', current)).toBe(true)
    expect(sourceKept(other, true, 'current', current)).toBe(false)
    expect(sourceKept(null, false, 'current', current)).toBe(true)

    expect(sourceKept(other, true, 'all', current)).toBe(true)
    expect(sourceKept(null, false, 'all', current)).toBe(true)

    expect(sourceKept(null, false, 'unfiled', current)).toBe(true)
    expect(sourceKept(current, true, 'unfiled', current)).toBe(false)

    expect(sourceKept(other, true, sourceValueOf(other), current)).toBe(true)
    expect(sourceKept(current, true, sourceValueOf(other), current)).toBe(false)
    expect(sourceKept(null, false, sourceValueOf(other), current)).toBe(false)
  })

  it('筛选值把完整身份与三个保留档分开', () => {
    expect(sourceValueOf('github#o/r@main')).toBe('source:github#o/r@main')
    expect(keyOfSourceFilter(sourceValueOf('gitee#o/r@main'))).toBe('gitee#o/r@main')
    expect(keyOfSourceFilter('current')).toBeNull()
    expect(keyOfSourceFilter('all')).toBeNull()
    expect(keyOfSourceFilter('unfiled')).toBeNull()
  })
})

/*
 * 菜单右侧的数字要等于"选了这一档会看到几行"：机型列按当前来源档数，
 * 来源列按当前机型档数。两列都读本地表那套判据，不拿全盘文件或云端目录充数。
 */
describe('筛选菜单计数', () => {
  const current = 'gitee#mucorebenc/mkphelperse@main'
  const other = 'custom#127.0.0.1:8787/delivery@-'
  const file = (
    path: string,
    machineId: string | null,
    sourceRef: string | null,
    kind: 'mkp_preset' | 'bbs_profile' = 'mkp_preset',
  ) =>
    ({
      path,
      fileName: path,
      kind,
      machineId,
      basedOnMachineId: null,
      basedOnFacts: { sourceRef },
    }) as LocalPresetFile
  const files = [
    file('fast1.toml', 'A1 mini', current),
    file('fast2.toml', 'A1 mini', current),
    file('standard.toml', 'A1 mini', current),
    file('other.toml', 'A1 mini', other),
    file('imported.toml', null, null),
    file('a1.toml', 'A1', current),
    file('x1c1.toml', 'X1C', other),
    file('x1c2.toml', 'X1C', other),
    file('bbs.json', 'A1 mini', current, 'bbs_profile'),
  ]
  const opts = {
    machineIds: ['A1 mini', 'A1', 'X1C'],
    machineId: 'A1 mini',
    sourceFilter: 'current' as const,
    currentSource: current,
    kind: 'mkp' as const,
    query: '',
  }

  it('机型数只算当前来源档的行；未标机型的行在每一档下都算', () => {
    expect(filterCountsOf(files, opts).machine).toEqual({ 'A1 mini': 4, A1: 2, X1C: 1 })
    expect(filterCountsOf(files, { ...opts, sourceFilter: 'all' }).machine).toEqual({
      'A1 mini': 5,
      A1: 2,
      X1C: 3,
    })
    expect(
      filterCountsOf(files, { ...opts, sourceFilter: sourceValueOf(other) }).machine,
    ).toEqual({ 'A1 mini': 1, A1: 0, X1C: 2 })
  })

  it('来源数的四档都按当前机型算，全部 / 当前档也带数字', () => {
    expect(filterCountsOf(files, opts).source).toEqual({
      current: 4,
      all: 5,
      unfiled: 1,
      byKey: { [current]: 3, [other]: 1 },
    })
    expect(filterCountsOf(files, { ...opts, machineId: 'X1C' }).source).toEqual({
      current: 1,
      all: 3,
      unfiled: 1,
      byKey: { [other]: 2 },
    })
  })

  it('搜索词与类型档跟着本地表一起收窄', () => {
    expect(filterCountsOf(files, { ...opts, query: 'standard' }).machine['A1 mini']).toBe(1)
    expect(
      filterCountsOf(files, { ...opts, kind: 'slicer', machineId: '' }).source.all,
    ).toBe(1)
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
      why: '不同数据源：不比较版本，但允许下载当前源版本',
      row: { localState: 'stale', basedOn: 'elsewhere' },
      action: 'download',
    },
    {
      why: '云端比本机旧（不给更新，那是往回带）',
      row: { localState: 'stale', basedOn: 'newer' },
      action: 'none',
    },
    {
      why: '同源同号字节不一致：允许重新下载修复',
      row: { localState: 'stale', basedOn: 'inconsistent' },
      action: 'download',
    },
    {
      why: '说不清版本关系：允许重新下载',
      row: { localState: 'stale', basedOn: 'unverified' },
      action: 'download',
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
