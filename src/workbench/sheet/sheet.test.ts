import { describe, expect, it } from 'vitest'

import type { Cell, Col, Desk, Row } from '../api'
import {
  applyDraftToDesk,
  cellKeyOf,
  dirtyOf,
  draftAfterWrite,
  rawOf,
  savedRawOf,
  settleDraft,
  shownRawOf,
  type DraftEntry,
} from './draftOverlay'
import { WriteQueue, type QueueEntry } from './writeQueue'

/* ---------- 夹具：一列（A1 机型基底）× 一行 ---------- */

const setValue = (raw: unknown, rest: unknown, dirty = false): Cell =>
  ({
    kind: 'value',
    text: String(raw ?? ''),
    lines: null,
    origin: 'factory',
    originLabel: '',
    originExplain: '',
    own: false,
    dirty,
    editable: true,
    reason: null,
    blocked: [],
    blockedNote: null,
    blockedHint: null,
    jumpTo: null,
    differs: false,
    diffTip: null,
    raw,
    rest,
    restPinned: rest !== null,
    /* 夹具不管徽章：这几条判据盯的是值与脏，见 draftOverlay 头注 */
    badge: null,
    deprecated: false,
    deadChoices: [],
  }) as Cell

const rowOf = (key: string, cell: Cell): Row => ({
  key,
  label: key,
  desc: '',
  unit: null,
  sectionId: 's',
  sectionLabel: 'S',
  tabId: null,
  depth: 0,
  parentKey: null,
  parentLabel: null,
  parentNote: null,
  controlNote: null,
  gcode: false,
  deprecated: false,
  impact: null,
  cells: [cell],
})

const cols: Col[] = [
  {
    key: 'A1',
    machineId: 'A1',
    versionUid: null,
    level: 'machine',
    machine: 'A1',
    label: '机型基线',
    items: 0,
  },
]

const deskOf = (...rows: Row[]): Desk =>
  ({
    machineId: 'A1',
    machine: 'A1',
    uid: null,
    cols,
    cur: 0,
    total: rows.length,
    groups: [
      {
        sectionId: 's',
        label: 'S',
        count: rows.length,
        offNote: null,
        items: rows.map((row) => ({ row, children: [] })),
      },
    ],
  }) as unknown as Desk

const K = cellKeyOf('A1', null, 'speed')

/* ---------- 值 → 原文 ---------- */

describe('rawOf —— 值到 TOML 原文的唯一转换点', () => {
  it('null / undefined 落成空串（**不是** "null"）', () => {
    /* 旧页写 `String(cell.rest)`，rest 为 null 时得到 "null"，
       数字控件解析失败回落成 0 —— 作者看到的「点 ↩ 70 变 0」就是这一条 */
    expect(rawOf(null)).toBe('')
    expect(rawOf(undefined)).toBe('')
  })

  it('数字 / 布尔 / 字符串按字面', () => {
    expect(rawOf(65)).toBe('65')
    expect(rawOf(0.15)).toBe('0.15')
    expect(rawOf(true)).toBe('true')
    expect(rawOf('tower')).toBe('tower')
  })
})

/* ---------- 写完之后本地挂什么 ---------- */

describe('draftAfterWrite —— 「跟随基底」不钉值（2026-11-03 作者报的那条）', () => {
  it('写值：钉我的原文', () => {
    expect(draftAfterWrite('90')).toEqual({ raw: '90' })
  })

  it('跟随基底（null）：**什么也不挂** —— 值由后端同步成基底那个数', () => {
    /* 以前挂的是 `rawOf(cell.rest)`（保存值）：后端写进去的是基底值，二者
       永远不相等 ⇒ `settleDraft` 收不掉 ⇒ 徽章写着「跟随基底」，数字却停在
       三步以前那个，改基底一动不动。 */
    expect(draftAfterWrite(null)).toBeNull()
  })
})

/* ---------- 保存值：三层算完的有效值 ---------- */

describe('savedRawOf —— 保存值不是"文件里那格"', () => {
  it('文件里空着、继承来的值照样算得出（作者 2026-11-02 问的就是这个）', () => {
    /* BASE 里 `A1:STANDARD` 没钉 ⇒ `rest` 由后端算成机型基底 / 出厂默认 */
    expect(savedRawOf(setValue(90, 65))).toBe('65')
  })

  it('这一项在这台机型上不适用（rest = null）⇒ 空串', () => {
    expect(savedRawOf(setValue(90, null))).toBe('')
  })
})

/* ---------- 脏：唯一判据是"值不一样" ---------- */

describe('dirtyOf —— 值 / 保存值 / 脏 三件事各归各位', () => {
  it('显示值 = 保存值 ⇒ 不脏（**哪怕后端那个 dirty 是 true**）', () => {
    /* 2026-11-02 截图：值 65、保存值 65，却标着「已修改」而右上角是 0。
       两个源各说各话 —— 显示只认值。 */
    const cell = setValue(65, 65, true)
    expect(dirtyOf(cell, undefined)).toBe(false)
  })

  it('改成别的值 ⇒ 脏', () => {
    expect(dirtyOf(setValue(90, 65), undefined)).toBe(true)
  })

  it('草稿里是别的值 ⇒ 脏（不等后端）', () => {
    expect(dirtyOf(setValue(65, 65), { raw: '90' })).toBe(true)
  })

  it('改回保存值 ⇒ 当场不脏（徽章立刻灭，不必等保存）', () => {
    expect(dirtyOf(setValue(65, 65, true), { raw: '65' })).toBe(false)
  })

  it('删键挂回继承（草稿空串）⇒ 与保存值不同就算脏', () => {
    expect(dirtyOf(setValue(65, 65), { raw: '' })).toBe(true)
    /* 保存值本来就是空（不适用）时不算脏 */
    expect(dirtyOf(setValue(65, null), { raw: '' })).toBe(false)
  })
})

/* ---------- 草稿叠到一屏上 ---------- */

describe('applyDraftToDesk —— 显示只看草稿，脏只看值', () => {
  it('改过的格子：显示我的值', () => {
    const desk = deskOf(rowOf('speed', setValue(65, 65)))
    const out = applyDraftToDesk(desk, new Map([[K, { raw: '90' }]]))!
    const cell = out.groups[0].items[0].row.cells[0]
    expect(cell.raw).toBe('90')
    expect(cell.text).toBe('90')
    expect(cell.dirty).toBe(true)
  })

  it('没有草稿、后端 dirty 又是错的 ⇒ 也要纠正回来', () => {
    const desk = deskOf(rowOf('speed', setValue(65, 65, true)))
    const out = applyDraftToDesk(desk, new Map())!
    expect(out.groups[0].items[0].row.cells[0].dirty).toBe(false)
  })

  it('什么都没变 ⇒ 原样返回（引用不变，不重渲染）', () => {
    const desk = deskOf(rowOf('speed', setValue(65, 65)))
    expect(applyDraftToDesk(desk, new Map())).toBe(desk)
  })

  it('枚举那种非数字值：原文换了、label 仍用后端那份', () => {
    const cell = setValue('tower', 'tower')
    cell.text = '擦料塔'
    const desk = deskOf(rowOf('wipe', cell))
    const out = applyDraftToDesk(desk, new Map([[cellKeyOf('A1', null, 'wipe'), { raw: 'wipe' }]]))!
    const got = out.groups[0].items[0].row.cells[0]
    expect(got.raw).toBe('wipe')
    expect(got.text).toBe('擦料塔')
  })
})

/* ---------- 后端跟上之后收掉草稿 ---------- */

describe('settleDraft —— 草稿只在"后端还没跟上"时存在', () => {
  it('后端已经是草稿那个值 ⇒ 删掉那一条', () => {
    const desk = deskOf(rowOf('speed', setValue(90, 65)))
    const draft = new Map<string, DraftEntry>([[K, { raw: '90' }]])
    expect(settleDraft(desk, draft).size).toBe(0)
  })

  it('后端还是旧值 ⇒ 留着（否则显示会跳回旧值）', () => {
    const desk = deskOf(rowOf('speed', setValue(65, 65)))
    const draft = new Map<string, DraftEntry>([[K, { raw: '90' }]])
    expect(settleDraft(desk, draft).size).toBe(1)
  })

  it('一条都没删 ⇒ 返回同一个引用', () => {
    const desk = deskOf(rowOf('speed', setValue(65, 65)))
    const draft = new Map<string, DraftEntry>([[K, { raw: '90' }]])
    expect(settleDraft(desk, draft)).toBe(draft)
  })
})

describe('shownRawOf', () => {
  it('有草稿用草稿，没有就用后端那份', () => {
    const cell = setValue(65, 65)
    expect(shownRawOf(cell, { raw: '90' })).toBe('90')
    expect(shownRawOf(cell, undefined)).toBe('65')
  })
})

/* ---------- 写入队列：单飞 + 排空 + 失败回滚 ---------- */

/** 每条请求都挂在外面，由测试决定什么时候回来 */
function gate() {
  const waiting: { entry: QueueEntry; resolve: () => void; reject: (e: unknown) => void }[] = []
  const send = (entry: QueueEntry) =>
    new Promise<void>((resolve, reject) => {
      waiting.push({ entry, resolve, reject })
    })
  return { waiting, send }
}

const patch = { kind: 'setValue', level: 'machine', owner: 'A1', key: 'k', value: 1 } as never

describe('WriteQueue —— 同一时刻只有一条在飞', () => {
  it('连发三条，只有第一条出去；落了才发下一条', async () => {
    const g = gate()
    const q = new WriteQueue({ send: g.send, onFail: () => undefined })
    q.enqueue('a', 'A', patch)
    q.enqueue('b', 'B', patch)
    q.enqueue('c', 'C', patch)
    expect(g.waiting).toHaveLength(1)

    g.waiting[0].resolve()
    await Promise.resolve()
    await Promise.resolve()
    expect(g.waiting).toHaveLength(2)

    g.waiting[1].resolve()
    await Promise.resolve()
    await Promise.resolve()
    g.waiting[2].resolve()
    await q.drain(1000)
    expect(q.pendingCount).toBe(0)
  })

  it('同一格连改：只留最新一条', async () => {
    const g = gate()
    const q = new WriteQueue({ send: g.send, onFail: () => undefined })
    q.enqueue('a', 'A', patch)
    q.enqueue('b', 'B1', patch)
    q.enqueue('b', 'B2', patch)
    g.waiting[0].resolve()
    await Promise.resolve()
    await Promise.resolve()
    expect(g.waiting).toHaveLength(2)
    expect(g.waiting[1].entry.label).toBe('B2')
  })

  it('失败：报给上层（界面据此把那只格子退回保存值）', async () => {
    const g = gate()
    const failed: string[] = []
    const q = new WriteQueue({ send: g.send, onFail: (k) => failed.push(k) })
    q.enqueue('a', 'A', patch)
    g.waiting[0].reject(new Error('boom'))
    await q.drain(1000)
    expect(failed).toEqual(['a'])
  })

  it('放弃之后迟到的那条**不报错**（用户已经放弃了这一份）', async () => {
    const g = gate()
    const failed: string[] = []
    const q = new WriteQueue({ send: g.send, onFail: (k) => failed.push(k) })
    q.enqueue('a', 'A', patch)
    q.cancelAll()
    g.waiting[0].reject(new Error('boom'))
    await q.drain(1000)
    expect(failed).toEqual([])
  })

  it('排不空就拒绝（保存前必须拿到这个错）', async () => {
    const g = gate()
    const q = new WriteQueue({ send: g.send, onFail: () => undefined })
    q.enqueue('a', 'A', patch)
    await expect(q.drain(20)).rejects.toThrow()
  })
})
