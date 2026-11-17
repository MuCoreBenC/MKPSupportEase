/*
 * 线距实测（试验台口径）。
 *
 * 带宽 = N × 线距，而线距不是常数：喷嘴直径、`glue_sparse_ratio`、切片器怎么排
 * 都会改它。把 0.4 写死，换台机器就差一倍 —— 所以从这一层的路径自己量。
 *
 * 怎么量：并排两条涂胶线的垂直间距就是线距。取**相邻的两条长移动**，方向大致
 * 平行（|cos| > 0.7）、垂直间距落在 (0.1, 3.0)mm、连接段不跨远，就记一次样本，
 * 最后取中位数。蛇形填充的相邻两行、环形轮廓的相邻两圈都落进这条判据。
 *
 * 同一行被切成好几条首尾相接的移动（真产物就是这样）时垂直间距约为 0，被 0.1mm
 * 的下限挡掉，不会把线距拉低。样本不足 `MIN_SAMPLES` 条就回落名义线距，并把
 * `fallback` 报给界面 —— 界面上「实测 / 回落」必须能看出来。
 */

import type { LabMove, Point } from './parse'

/** 名义线距（mm）：0.4 喷嘴并排涂胶线的常见间距，只在测不出时用 */
export const NOMINAL_PITCH_MM = 0.4
const MIN_PITCH_MM = 0.15
const MAX_PITCH_MM = 2.0
/** 参与比较的移动至少这么长，短移动的方向不可靠 */
const MIN_LONG_MM = 1.2
/** 两条长移动之间允许隔多远（蛇形的一次折返就是一个线距） */
const MAX_GAP_MM = 3.0
const MIN_SAMPLES = 6

export interface PitchEstimate {
  mm: number
  /** 命中样本数 */
  samples: number
  /** 样本不够，`mm` 是回落的名义值 */
  fallback: boolean
}

interface Stroke {
  start: Point
  end: Point
  dir: Point
  len: number
}

function stroke(m: LabMove): Stroke {
  const dx = m.to.x - m.from.x
  const dy = m.to.y - m.from.y
  const len = Math.hypot(dx, dy)
  return {
    start: m.from,
    end: m.to,
    dir: len > 0 ? { x: dx / len, y: dy / len } : { x: 0, y: 0 },
    len,
  }
}

/** 点 `p` 到过 `s.start`、方向 `s.dir` 的直线的距离（`s.dir` 已归一） */
function lineDistance(s: Stroke, p: Point): number {
  return Math.abs((p.x - s.start.x) * s.dir.y - (p.y - s.start.y) * s.dir.x)
}

/**
 * 这一层的线距（mm）。`moves` 传落笔移动即可，空走不参与。
 *
 * 判据只看几何，不看预设：同一份产物换个喷嘴重切，量出来的值就跟着变。
 */
export function estimatePitch(moves: readonly LabMove[]): PitchEstimate {
  const samples: number[] = []
  let prev: Stroke | null = null

  for (const m of moves) {
    const cur = stroke(m)
    if (cur.len < MIN_LONG_MM) continue
    if (prev) {
      const gap = Math.hypot(cur.start.x - prev.end.x, cur.start.y - prev.end.y)
      const cos = prev.dir.x * cur.dir.x + prev.dir.y * cur.dir.y
      if (gap <= MAX_GAP_MM && Math.abs(cos) > 0.7) {
        const perp = lineDistance(prev, cur.start)
        if (perp > 0.1 && perp < MAX_GAP_MM) samples.push(perp)
      }
    }
    prev = cur
  }

  if (samples.length < MIN_SAMPLES) {
    return { mm: NOMINAL_PITCH_MM, samples: samples.length, fallback: true }
  }
  samples.sort((a, b) => a - b)
  const mid = samples.length >> 1
  const median =
    samples.length % 2 === 0 ? (samples[mid - 1] + samples[mid]) / 2 : (samples[mid] as number)
  return {
    mm: Math.min(MAX_PITCH_MM, Math.max(MIN_PITCH_MM, median)),
    samples: samples.length,
    fallback: false,
  }
}
