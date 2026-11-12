/*
 * 「预估打印时间」的取值（2026-11-04 作者两句话定的口径）：
 * **行永远在**；失败 / 取消时右侧是「—」（不拿切片器自报的老话顶上），成功才给数。
 *
 * 判据只看结论本身：失败时 `result` 里没有重算的秒数，而开场事实里那句
 * 「切片器自报 1 小时 58 分」是**输入头老话**，不该冒充这一趟的结果。
 */
import { describe, expect, it } from 'vitest'

import {
  SETTINGS_FOLD_MS,
  estimatedPrintTimeSeconds,
  postProcessScene,
  postProcessSettingsFold,
  showsPostProcessChip,
} from './facts'
import type { ChipState, SettingsFoldInput } from './facts'
import type {
  PostProcessFinished,
  PostProcessQuestion,
  PostProcessRun,
} from '../../api/contract'

function finished(over: Partial<PostProcessFinished>): PostProcessFinished {
  return {
    ok: true,
    cancelled: false,
    message: '',
    ...over,
  } as PostProcessFinished
}

describe('结果页的预估打印时间', () => {
  const secs = (f: PostProcessFinished, over: { result?: number; slicer?: number } = {}) =>
    estimatedPrintTimeSeconds({
      finished: f,
      resultSeconds: over.result,
      slicerSeconds: over.slicer,
    })

  it('成功：内核重算的优先', () => {
    expect(secs(finished({ ok: true }), { result: 7080, slicer: 9999 })).toBe(7080)
  })

  it('成功但还没算出来：先摆切片器自报的那格顶着', () => {
    /* 打印时间是后到的（`postprocess-print-time` 事件），到之前不能空着 */
    expect(secs(finished({ ok: true }), { slicer: 9999 })).toBe(9999)
    expect(secs(finished({ ok: true }))).toBeUndefined()
  })

  it('正常处理失败：行还在，右侧给「—」', () => {
    expect(secs(finished({ ok: false }))).toBeUndefined()
  })

  it('取消：也是「—」', () => {
    expect(secs(finished({ ok: false, cancelled: true }))).toBeUndefined()
  })

  it('回放里的失败记录同一份判据（不另立一套）', () => {
    /* 回放的 finished 由 `PageReport.replayRunOf` 按 `d.summary.ok` 造出来 */
    const replay = finished({ ok: false, message: 'E_GCODE_COLLISION_001' })
    expect(secs(replay, { slicer: 7080 })).toBeUndefined()
  })

  it('失败屏带着「切片器自报时长」也不许顶上 —— 这正是原来那句「1 小时 58 分」的来源', () => {
    expect(secs(finished({ ok: false, result: { printTimeSeconds: 7080 } }), { slicer: 7080 })).toBeUndefined()
  })
})

/* 「去设置」停驻：胶囊不过期、点开后收起、点关闭才真消失（2026-11-04） */
describe('后处理胶囊', () => {
  const done = { finished: { ok: false } } as PostProcessRun
  const running = { finished: null } as PostProcessRun
  const state = (over: Partial<ChipState> = {}): ChipState => ({
    run: done,
    dismissed: true,
    quietUntil: null,
    expired: false,
    ...over,
  })

  it('这一趟还没来过 ⇒ 不摆', () => {
    expect(showsPostProcessChip(state({ run: null }))).toBe(false)
  })

  it('跑着并收起 ⇒ 三颗点那枚照旧', () => {
    expect(showsPostProcessChip(state({ run: running }))).toBe(true)
  })

  it('跑完但没「留一会儿」的账 ⇒ 不摆（GUI 档结论正摆在屏上）', () => {
    expect(showsPostProcessChip(state())).toBe(false)
  })

  it('静默档 60 秒计时内 ⇒ 摆，到点自己消失', () => {
    expect(showsPostProcessChip(state({ quietUntil: 123 }))).toBe(true)
    expect(showsPostProcessChip(state({ quietUntil: 123, expired: true }))).toBe(false)
  })

  it('停驻的失败结论 ⇒ 不看 60 秒过期，胶囊常驻', () => {
    /* 这正是作者要的：「等用户点了这个关闭的时候它才消失」 */
    expect(
      showsPostProcessChip(state({ parked: true, expired: true, quietUntil: null })),
    ).toBe(true)
  })

  it('点胶囊把模态框翻回来 ⇒ 胶囊自己收起，不许两处同框', () => {
    expect(showsPostProcessChip(state({ parked: true, dismissed: false }))).toBe(false)
  })

  it('用户点「关闭」 ⇒ 停驻标志清掉，胶囊真消失', () => {
    expect(showsPostProcessChip(state({ parked: false }))).toBe(false)
  })
})

/* 设置页上「完成」折成胶囊的时机（2026-11-13） */
describe('设置页上的后处理那一屏', () => {
  const fold = (over: Partial<SettingsFoldInput> = {}) =>
    postProcessSettingsFold({
      scene: 'finished',
      dismissed: false,
      succeeded: true,
      onSettings: true,
      prevOnSettings: true,
      prevScene: 'opening',
      ...over,
    })

  it('人停在设置页、成功刚出来 ⇒ 先摆 10 秒再折', () => {
    expect(fold()).toBe('grace')
    expect(SETTINGS_FOLD_MS).toBe(10_000)
  })

  it('失败 / 取消刚出来 ⇒ 只有成功有宽限，当场折', () => {
    expect(fold({ succeeded: false })).toBe('now')
  })

  it('失败那屏点开看错误码 ⇒ 一直开着，不许又被折回去', () => {
    expect(fold({ succeeded: false, prevScene: 'finished' })).toBe('none')
  })

  it('从别的页切进设置页 ⇒ 立刻折', () => {
    expect(fold({ prevOnSettings: false, prevScene: 'finished' })).toBe('now')
  })

  it('折了之后点胶囊打开 ⇒ 一直开着，不再计时', () => {
    /* 点开胶囊不换幕（上一帧已经是 finished），不许重新数那 10 秒 */
    expect(fold({ prevScene: 'finished' })).toBe('none')
  })

  it('跑着 / 机型不匹配 / 点位测试 ⇒ 不动它（照常盖在设置页上）', () => {
    expect(fold({ scene: 'opening' })).toBe('none')
    expect(fold({ scene: 'question' })).toBe('none')
  })

  it('已经收起 / 不在设置页 ⇒ 不动它', () => {
    expect(fold({ dismissed: true })).toBe('none')
    expect(fold({ onSettings: false, prevOnSettings: false })).toBe('none')
  })

  it('幕的判据：跑着 / 等答复 / 完成', () => {
    const run = (over: Partial<PostProcessRun>): PostProcessRun =>
      ({ finished: null, question: null, ...over }) as PostProcessRun
    const q = { questionKind: 'machine-mismatch' } as PostProcessQuestion
    expect(postProcessScene(run({}))).toBe('opening')
    expect(postProcessScene(run({ question: q }))).toBe('question')
    expect(
      postProcessScene(run({ finished: { ok: true } as PostProcessFinished })),
    ).toBe('finished')
    expect(postProcessScene(null)).toBeNull()
  })
})
