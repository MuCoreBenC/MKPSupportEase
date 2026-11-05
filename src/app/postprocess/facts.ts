/*
 * 结果态那几个「取什么值」的判据（纯函数，便于回归）。
 *
 * 2026-11-04 两处口径：
 *   · 作者：「这个正常的处理失败，那个预估打印时间是不需要显示的了」—— 后来补了一句
 *     「我说的失败不显示，是那个预估打印时间字段还在，只是右侧的参数是用那个横杠-」：
 *     **行要留着**，失败 / 取消时右侧的值是「—」，不拿切片器自报的那句老话顶上；
 *     成功那趟照旧摆（内核重算的，没算出来前先摆切片器自报的）。
 *   · 弹窗「去设置」后收成右上角胶囊，**直到用户点「关闭」才消失**（不套 60 秒过期）。
 *
 * ★ 为什么单独成件：这两件事原本都住在组件文件里（`PostProcessModal.tsx` /
 *   `PostProcessChip.tsx`），判据塞进去就只能靠肉眼复核；抽成纯函数才写得了自动化回归，
 *   而"失败时给横杠""停驻不过期"正好是最容易在后续改版里被顺手改回来的。
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
