/*
 * 数据多语言挑选的回归判据（2026-10-31）。
 *
 * # 它守的是什么
 *
 * 作者真机截图：日文界面下首页/预设列表仍是中文。查下来断点在**后端出站 DTO**
 * （`VersionDto` 漏了 `nameJa`，serde 静默丢弃）—— 那条已经由 Rust 侧
 * `dto_builders_read_the_catalog_and_nothing_else` 钉住「数据有没有传出去」。
 *
 * 这里守的是另一半：**传出来了，界面有没有挑对**。
 *   · ja 界面用 `nameJa`，en 用 `nameEn`，zh-TW 用 `nameTw`；
 *   · 缺那一册/空串/只有空白 ⇒ 回落中文（不是显示空、也不是显示别的语言）；
 *   · zh-CN 界面**永远**中文（哪怕英文册有值）—— 否则中文用户会看到英文。
 *
 * 判据只吃纯函数，不起 DOM。
 */
import { describe, expect, it } from 'vitest'

import { bi, localizeMachines, multiLang } from './dataLabel'

type V = {
  id: string
  name: string
  nameEn?: string | null
  nameTw?: string | null
  nameJa?: string | null
  tag?: string | null
  tagEn?: string | null
  tagTw?: string | null
  tagJa?: string | null
  description?: string | null
  descriptionEn?: string | null
  descriptionTw?: string | null
  descriptionJa?: string | null
}

/** 与真目录同形的版本：三册都填了 */
const full: V = {
  id: 'STANDARD',
  name: '标准版',
  nameEn: 'Standard',
  nameTw: '標準版',
  nameJa: 'スタンダード版',
  tag: '推荐',
  tagEn: 'Recommended',
  tagTw: '推薦',
  tagJa: 'おすすめ',
  description: '标准配置，适合大多数用户',
  descriptionEn: 'Standard configuration for most users',
  descriptionTw: '標準配置，適合大多數使用者',
  descriptionJa: '標準構成，ほとんどのユーザーに適しています',
}

/** 只译了中文的版本（老数据的形状） */
const onlyZh: V = { id: 'LITE', name: 'lite版' }

const wrap = (v: V) => [{ id: 'A1', versions: [v] }]
const pick = (v: V, locale: 'zh-CN' | 'zh-TW' | 'en' | 'ja') =>
  localizeMachines(wrap(v), locale)[0].versions[0]

describe('multiLang：按界面语言挑一册', () => {
  it('ja 挑日文册，en 挑英文册，zh-TW 挑繁體册', () => {
    const t = { En: 'Standard', Tw: '標準版', Ja: 'スタンダード版' }
    expect(multiLang('标准版', t, 'ja')).toBe('スタンダード版')
    expect(multiLang('标准版', t, 'en')).toBe('Standard')
    expect(multiLang('标准版', t, 'zh-TW')).toBe('標準版')
  })

  it('zh-CN 永远中文 —— 英文册有值也不许串语言', () => {
    expect(multiLang('标准版', { En: 'Standard', Ja: 'スタンダード版' }, 'zh-CN')).toBe('标准版')
  })

  it('缺册、空串、null 一律回落中文', () => {
    expect(multiLang('标准版', {}, 'ja')).toBe('标准版')
    expect(multiLang('标准版', { Ja: '' }, 'ja')).toBe('标准版')
    expect(multiLang('标准版', { Ja: null }, 'ja')).toBe('标准版')
    // 只有英文册时，日文界面回落中文而不是英文（不给"半懂"的名字）
    expect(multiLang('标准版', { En: 'Standard' }, 'ja')).toBe('标准版')
  })
})

describe('localizeMachines：三个界面各显示各的语言', () => {
  it('日文界面：名字/角标/描述全部换成日文', () => {
    const v = pick(full, 'ja')
    expect(v.name).toBe('スタンダード版')
    expect(v.tag).toBe('おすすめ')
    expect(v.description).toBe('標準構成，ほとんどのユーザーに適しています')
  })

  it('英文界面：换成英文', () => {
    const v = pick(full, 'en')
    expect(v.name).toBe('Standard')
    expect(v.tag).toBe('Recommended')
    expect(v.description).toBe('Standard configuration for most users')
  })

  it('繁体界面：换成繁體', () => {
    const v = pick(full, 'zh-TW')
    expect(v.name).toBe('標準版')
    expect(v.tag).toBe('推薦')
  })

  it('中文界面：原样不动（连对象都还是那一份）', () => {
    const machines = wrap(full)
    expect(localizeMachines(machines, 'zh-CN')).toBe(machines)
  })

  it('没译的版本在日文界面回落中文，不显示空', () => {
    const v = pick(onlyZh, 'ja')
    expect(v.name).toBe('lite版')
    // 原对象根本没有 `tag` 这个键 —— 摊开后仍是 undefined（不是空串、也不是 null）
    expect(v.tag).toBeUndefined()
  })

  it('版本 id 不被改动（它是身份，不是文案）', () => {
    expect(pick(full, 'ja').id).toBe('STANDARD')
  })
})

describe('bi：两册择一的老口径不受影响', () => {
  it('en 空串时回落中文', () => {
    expect(bi('标准版', '', 'en')).toBe('标准版')
    expect(bi('标准版', undefined, 'en')).toBe('标准版')
    expect(bi('标准版', 'Standard', 'en')).toBe('Standard')
    expect(bi('标准版', 'Standard', 'zh-CN')).toBe('标准版')
  })
})
