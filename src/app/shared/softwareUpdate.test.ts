import { describe, expect, it } from 'vitest'

import type { UpdateState } from '../../api/contract'
import { updateChipIntent } from './softwareUpdate'

/*
 * 标题栏胶囊的状态表（docs/SOFTWARE-UPDATE-SPEC.md §3）。
 *
 * 重点是两件事：
 *  ① `hasUpdate` 根本不在入参里 —— 关掉自动检查 / 换渠道之后它仍可能是 true，
 *     拿它判胶囊就会出现"关了还一直在"；
 *  ② 进度态永远压过"有新版"，忽略只压"有新版"那一态。
 */
describe('标题栏更新胶囊', () => {
  const idle: UpdateState = { state: 'idle' }
  const downloading: UpdateState = {
    state: 'downloading',
    received: 1,
    total: 2,
  }

  it('查不到新版就不渲染，哪怕设置页手检查还有 hasUpdate', () => {
    expect(updateChipIntent(idle, undefined, undefined)).toBe('none')
    expect(updateChipIntent(idle, null, null)).toBe('none')
    expect(updateChipIntent(idle, '', null)).toBe('none')
  })

  it('自动检查发现一版 ⇒ 可更新；忽略同一版 ⇒ 不渲染', () => {
    expect(updateChipIntent(idle, '0.0.12-preview.1', null)).toBe('pending')
    expect(updateChipIntent(idle, '0.0.12-preview.1', '0.0.12-preview.1')).toBe('none')
    /* 忽略只压那一版；云端发新版后自动恢复 */
    expect(updateChipIntent(idle, '0.0.12-preview.2', '0.0.12-preview.1')).toBe('pending')
  })

  it('有下载任务时只认进度，忽略与 pending 都不改它', () => {
    expect(updateChipIntent(downloading, undefined, undefined)).toBe('progress')
    expect(updateChipIntent(downloading, '0.0.12', '0.0.12')).toBe('progress')
    for (const state of ['paused', 'ready', 'failed'] as const) {
      const s: UpdateState =
        state === 'paused'
          ? { state, received: 1, total: 2 }
          : state === 'ready'
            ? { state, path: '/x', size: 1, version: '0.0.12', channel: 'stable' }
            : { state, reason: '离线' }
      expect(updateChipIntent(s, '0.0.12', '0.0.12')).toBe('progress')
    }
  })

  it('取消 / 空闲不算进度（与设置页的下载面板判据一致）', () => {
    expect(updateChipIntent({ state: 'cancelled' }, '0.0.12', null)).toBe('pending')
  })
})
