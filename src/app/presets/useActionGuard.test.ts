import { describe, expect, it } from 'vitest'

import { actionKey, GuardLedger } from './useActionGuard'

describe('动作在途守卫', () => {
  it('同一动作同一目标只放进第一次，重复进入被挡', () => {
    const ledger = new GuardLedger()
    expect(ledger.enter('download:a.toml')).toBe(true)
    expect(ledger.enter('download:a.toml')).toBe(false)
  })

  it('不同动作互不挡，不同目标互不挡', () => {
    const ledger = new GuardLedger()
    expect(ledger.enter('download:a.toml')).toBe(true)
    expect(ledger.enter('use:a.toml')).toBe(true)
    expect(ledger.enter('download:b.toml')).toBe(true)
    expect(ledger.has('download:a.toml')).toBe(true)
    expect(ledger.has('use:a.toml')).toBe(true)
    expect(ledger.has('download:b.toml')).toBe(true)
  })

  it('释放后可以再次进入；失败路径释放同样生效', () => {
    const ledger = new GuardLedger()
    expect(ledger.enter('download:a.toml')).toBe(true)
    ledger.leave('download:a.toml')
    expect(ledger.has('download:a.toml')).toBe(false)
    expect(ledger.enter('download:a.toml')).toBe(true)
  })

  it('释放一个未占住的键不影响别的键', () => {
    const ledger = new GuardLedger()
    ledger.enter('download:a.toml')
    ledger.enter('download:b.toml')
    ledger.leave('download:c.toml')
    expect(ledger.has('download:a.toml')).toBe(true)
    expect(ledger.has('download:b.toml')).toBe(true)
  })

  it('键由动作与目标拼成，目标里的冒号不歧义到动作', () => {
    expect(actionKey('download', 'a.toml')).toBe('download:a.toml')
    expect(actionKey('use', 'presets-local/a.toml')).toBe('use:presets-local/a.toml')
  })
})
