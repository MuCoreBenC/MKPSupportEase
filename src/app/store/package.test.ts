import { describe, expect, it } from 'vitest'

import { instantText, relativePart } from './package'

/*
 * 时间显示的唯一出口。判据只钉两件事：
 *   · 绝对时间不出现 T / Z 与秒（`instantText`）；
 *   · 相对时间的分档与边界（`relativePart`），且 `now` 由调用方给，不读系统时钟。
 */

/** 本地时区构造的 ISO：断言与运行机器时区无关 */
function localIso(y: number, mo: number, d: number, hh: number, mi: number): string {
  return new Date(y, mo - 1, d, hh, mi).toISOString()
}

describe('instantText', () => {
  it('ISO 折成本地时间，到分钟，不带 T / Z / 秒', () => {
    const out = instantText(localIso(2026, 11, 12, 21, 21))
    expect(out).toBe('2026-11-12 21:21')
    expect(out).not.toMatch(/[TZ]/)
    expect(out).not.toMatch(/:\d{2}:\d{2}/)
  })

  it('纯日期只给日期，不编一个 00:00', () => {
    expect(instantText('2026-08-26')).toBe('2026-08-26')
  })

  it('老的本机时刻没有年份就只给月-日', () => {
    expect(instantText('9/30 16:12')).toBe('09-30 16:12')
  })

  it('没有值就是没有值；认不出的原样返回，不猜', () => {
    expect(instantText(undefined)).toBeUndefined()
    expect(instantText('不是时间')).toBe('不是时间')
  })
})

describe('relativePart', () => {
  const at = '2026-11-13T00:00:00Z'
  const now = Date.parse(at)

  it('不到一分钟算刚刚', () => {
    expect(relativePart(at, now + 30_000)).toEqual({ kind: 'now' })
  })

  it('分钟 / 小时 / 天 / 月各自分档', () => {
    expect(relativePart(at, now + 90_000)).toEqual({ kind: 'minute', value: 1 })
    expect(relativePart(at, now + 2 * 3600_000)).toEqual({ kind: 'hour', value: 2 })
    expect(relativePart(at, now + 3 * 86400_000)).toEqual({ kind: 'day', value: 3 })
    expect(relativePart(at, now + 40 * 86400_000)).toEqual({ kind: 'month', value: 1 })
  })

  it('超过一年退回绝对时间（返回 null）', () => {
    expect(relativePart(at, now + 366 * 86400_000)).toBeNull()
  })

  it('未来时刻不算"多久之前"——时钟偏差不该显示成负数', () => {
    expect(relativePart(at, now - 60_000)).toBeNull()
  })

  it('纯日期与老值没有时区信息，一律退回绝对时间', () => {
    expect(relativePart('2026-11-12', now)).toBeNull()
    expect(relativePart('9/30 16:12', now)).toBeNull()
  })

  it('没值 / 认不出的串不编相对值', () => {
    expect(relativePart(undefined, now)).toBeNull()
    expect(relativePart('2026-13-99T99:99:99Z', now)).toBeNull()
  })
})
