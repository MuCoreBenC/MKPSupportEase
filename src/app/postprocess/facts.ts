/*
 * 后处理那一屏的几处判据（纯函数，便于回归）。
 *
 * 三件事：
 *   · 结果态「取什么值」：失败 / 取消时「预估打印时间」右侧给「—」，不拿切片器自报的顶上；
 *   · 「去设置」后收成右上角胶囊，**直到用户点「关闭」才消失**（不套 60 秒过期）；
 *   · 这一屏演到哪一幕（开场 / 问 / 完成）—— 在设置页上「完成」什么时候折成胶囊。
 *
 * ★ 为什么单独成件：这些原本都住在组件文件里（`PostProcessModal.tsx` /
 *   `PostProcessChip.tsx` / `App.tsx`），判据塞进去就只能靠肉眼复核；抽成纯函数才写得了
 *   自动化回归，而"失败时给横杠""停驻不过期""什么时候折成胶囊"正好是最容易在后续改版里
 *   被顺手改回来的。
 */

import type { PostProcessFinished, PostProcessRun } from '../../api/contract'

export interface EstimatedPrintTimeInput {
  finished: PostProcessFinished
  /**
   * 内核**重算**出来的秒数（`result.printTimeSeconds`）—— 成功那趟才有，
   * 而且常常是后到的（`postprocess-print-time` 事件），先摆切片器自报的那格顶着。
   */
  resultSeconds?: number
  /**
   * 开场事实里「切片器自报」的秒数（`facts.slicerPrintTimeSeconds`）——
   * **输入头里的老话**。失败 / 取消那一趟没有重算值，也**不许**拿它顶上
   * （作者截图里失败屏那句「1 小时 58 分」就是这么来的），只能给「—」。
   */
  slicerSeconds?: number
}

/**
 * 「预估打印时间」这一格的秒数；`undefined` = 右侧摆「—」。
 *
 * 判据只看结论：失败 / 取消 ⇒ `undefined`（**行还在**，只是右侧不给数）；
 * 成功 ⇒ 内核重算的优先，还没算出来就先摆切片器自报的。
 *
 * 跑着 / 问句那两扇不归这条管：那里摆的本来就是切片器自报的那一格，且没有结论。
 */
export function estimatedPrintTimeSeconds(input: EstimatedPrintTimeInput): number | undefined {
  const { finished } = input
  if (!finished.ok || finished.cancelled) return undefined
  return input.resultSeconds ?? input.slicerSeconds
}

export interface ChipState {
  run: PostProcessRun | null
  /** `PostProcessRunView.dismissed`（已含静默档派生） */
  dismissed: boolean
  quietUntil: number | null
  /** 60 秒那一档到点了没有（`PostProcessChip.useRunExpiry` 算出来的） */
  expired: boolean
  /** 「去设置」停驻标志；回放 / 预览那份手写 view 没有它 */
  parked?: boolean
}

/**
 * 右上角那枚胶囊摆不摆。
 *
 * 三档：
 *   · 跑着 + 收起         ⇒ 三颗点 + 百分比（老口径，没动）
 *   · 跑完 + 60 秒计时内   ⇒ 结论胶囊，到点自己消失（静默档 / 普通「后台运行」）
 *   · 跑完 + **停驻**      ⇒ 结论胶囊常驻，既不看 `quietUntil` 也不看 `expired`
 *
 * ★ 停驻还要 `dismissed`：用户点胶囊把模态框翻回来时，胶囊自己收起，
 *   不该两处同框。真正撤掉停驻只有 `close`（用户点「关闭」）或下一趟开场。
 */
export function showsPostProcessChip(s: ChipState): boolean {
  if (s.run === null) return false
  if (s.run.finished === null) return s.dismissed
  const parkedAway = s.parked === true && s.dismissed
  return parkedAway || (s.quietUntil !== null && !s.expired)
}

/* ---- 这一屏演的是哪一幕 / 在设置页上怎么折成胶囊 ---- */

/**
 * 后处理这一屏的幕。只分三档：
 *
 *   · `opening`  —— 开场 / 跑着（hero 进度、自动终止倒计时）
 *   · `question` —— 等答复（机型不匹配、点位测试、校准件选择、挪塔）
 *   · `finished` —— 完成（结果四格、警告页、失败页）
 */
export type PostProcessScene = 'opening' | 'question' | 'finished'

/** 这一趟现在演到哪一幕；`null` = 没有这一趟（用户收起来那一笔另算，见 `dismissed`） */
export function postProcessScene(run: PostProcessRun | null): PostProcessScene | null {
  if (run === null) return null
  if (run.finished !== null) return 'finished'
  return run.question === null ? 'opening' : 'question'
}

/**
 * 成功那一趟停在设置页时，结论屏先摆多久再折成右上角胶囊（作者 2026-11-13：「成功之后
 * 10 秒自动折叠成胶囊」）。**只有成功有这 10 秒**；失败 / 取消当场折（作者：「只想给成功」）。
 */
export const SETTINGS_FOLD_MS = 10_000

export interface SettingsFoldInput {
  /** 这一趟现在演到哪一幕 */
  scene: PostProcessScene | null
  /** 这一屏现在收着没有（`PostProcessRunView.dismissed`） */
  dismissed: boolean
  /** 「完成」那一幕是不是成功那趟（`finished.ok`） */
  succeeded: boolean
  /** 此刻在不在设置页 */
  onSettings: boolean
  /** 上一帧在不在设置页（用来认「刚切过来」） */
  prevOnSettings: boolean
  /** 上一帧演的是哪一幕（用来认「结论刚出来」/「点胶囊打开后一直开着」） */
  prevScene: PostProcessScene | null
}

/** `none` = 不动它；`now` = 立刻折；`grace` = 先摆 `SETTINGS_FOLD_MS` 再折 */
export type SettingsFold = 'none' | 'now' | 'grace'

/**
 * 设置页上这一屏该怎么处置（作者 2026-11-13）。
 *
 *   · 不是「完成」那一幕 / 已经收起来了 / 不在设置页 ⇒ `none`
 *     （开场、机型不匹配、点位测试优先级最高，照常盖在设置页上）；
 *   · 从别的页切进设置页 ⇒ `now`（他已经看过那一屏，来这儿是要改开关）；
 *   · 人本来就停在设置页、结果刚出来 ⇒ 成功给 `grace`（先摆 10 秒），
 *     失败 / 取消给 `now`（只有成功有宽限）。
 *
 * ★ 「刚出来」靠上一帧的幕来认：上一帧还不是 `finished` ⇒ 这是到达那一帧。
 *   于是点胶囊（`dismissed` 翻回 false、幕没变）不会被重新计时 —— 打开就一直开着；
 *   这一条对失败那屏同样成立（点开看错误码，不许又被折回去）。
 */
export function postProcessSettingsFold(i: SettingsFoldInput): SettingsFold {
  if (i.scene !== 'finished') return 'none'
  if (i.dismissed) return 'none'
  if (!i.onSettings) return 'none'
  if (!i.prevOnSettings) return 'now'
  if (i.prevScene === 'finished') return 'none'
  return i.succeeded ? 'grace' : 'now'
}
