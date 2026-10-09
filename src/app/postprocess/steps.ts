/*
 * 后处理 12 步的**显示口径**：中文名 + 全局百分比权重。
 *
 * 为什么住界面：内核只给稳定 id（`input` / `pass1`…）与**步内**比例 ——
 * "怎么把它读成给人看的进度"是界面的事（边界写在 `crates/postprocess/src/pipeline/progress.rs`
 * 的模块头：显示名与全局百分比**只允许存在于 UI 一处**）。
 *
 * 权重是**体感**不是测量：两次 pass 是绝对大头（一条 G-code 逐行过两遍），
 * 其余是解析与收尾。内核不报耗时分布，这里也不假装知道它。
 */

export interface StepView {
  id: string
  name: string
  /** 全局权重（百分比）—— 十二项加起来恰好 100 */
  weight: number
}

export const STEPS: StepView[] = [
  { id: 'input', name: '读输入', weight: 1 },
  { id: 'config', name: '读预设', weight: 1 },
  { id: 'machine-detect', name: '认机型', weight: 1 },
  { id: 'parse', name: '解析 G-code', weight: 2 },
  { id: 'support', name: '找支撑面', weight: 5 },
  { id: 'collision', name: '避让检查', weight: 5 },
  { id: 'calib-check', name: '校准检查', weight: 1 },
  { id: 'pass1', name: '第一遍处理', weight: 38 },
  { id: 'pass2', name: '第二遍处理', weight: 38 },
  { id: 'calibration', name: '校准处理', weight: 3 },
  { id: 'write', name: '写回文件', weight: 4 },
  { id: 'printtime', name: '估算打印时间', weight: 1 },
]

/**
 * 阶段 id → 中文名。**认不出的 id 照原样显示** —— 内核加了新步骤时，
 * 界面宁可显示一个英文 id，也不装作认识它（那会显示成错的阶段名）。
 */
export function stepName(id: string): string {
  return STEPS.find((s) => s.id === id)?.name ?? id
}

/**
 * 全局百分比（`0..100`）。
 *
 * `fractionInStep = null`（这一步此刻给不出比例）⇒ 算这一步的**起点** ——
 * 宁可少走一点也不虚报：进度条冲到 95% 再卡住那种错觉，就是虚报出来的。
 */
export function percentOf(id: string, fractionInStep: number | null): number {
  const at = STEPS.findIndex((s) => s.id === id)
  if (at < 0) return 0
  let done = 0
  for (let i = 0; i < at; i += 1) done += STEPS[i].weight
  return Math.min(100, done + STEPS[at].weight * (fractionInStep ?? 0))
}
