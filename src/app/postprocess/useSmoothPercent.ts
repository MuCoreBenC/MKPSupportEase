/*
 * 平滑进度：把"每 1% 一跳"的进度事件插成连续走的一根条。
 *
 * 作者三次口径合起来的一句话：**要平滑，但不要骗人** ——
 * 「一顿一顿的我不喜欢，不够平滑」（2026-10-14）、
 * 「数字也卡顿，进度条也一顿一顿的」（同日晚，真机录屏）、
 * 以及早前定的「进度条冲到 95% 再卡住那种错觉，就是虚报出来的」（`steps.ts`）。
 *
 * # 为什么**不用 React state**（第二版改掉的地方）
 *
 * 第一版每帧 `setShown(v)` 让 React 重渲。**在真机 WKWebView 上这就成了"一顿一顿"**：
 * React 18 的调度器会把高频 setState **合并 / 延后**（尤其在有并发特性的低优先级更新上），
 * 帧被抽样丢掉 ⇒ 数字与条子看着一跳一跳；再叠上"每帧重渲一棵子树"的开销，更明显。
 *
 * 所以这里改成**命令式直写 DOM**：rAF 里只 `textContent =` 与 `style.width =`
 * （两个属性赋值，零 React 渲染、零 diff）。React 重渲（比如倒计时每秒 tick）之后，
 * 用一条**无依赖 effect** 把当前值重新铺回去（廉价，一次赋值）。
 *
 * # 三条口径（没变）
 *
 * 1. **不虚报**：蠕动最多比目标超前 [`CREEP_CAP`] 个百分点，且永不越过 100；
 * 2. **到位就停**：追上目标、或蠕动到顶，就**不再 requestAnimationFrame**（常态零 rAF）；
 * 3. **收尾有交代**：`done` 置位后目标直接给 100 —— 条子把最后几个点走完，
 *    而不是 95% 戛然而止（切换结果态的时机由调用方按 [`SETTLE_MS`] 兜）。
 */

import { useEffect, useRef } from 'react'
import type { RefObject } from 'react'

/** 蠕动速度（百分点 / 秒）：事件稀疏时条子仍在动，但不越过下面的上限 */
const CREEP_PER_SEC = 1.2
/** 蠕动上限：最多比目标超前这么多百分点 —— 这就是"不虚报"的那道线 */
const CREEP_CAP = 1.5
/** 逼近系数（每秒）：越大越紧跟。10 ≈ 0.3 秒走完 95% 的差距 */
const APPROACH = 10
/** 到位判据（百分点） */
const EPS = 0.05

/** 调用方等收尾用的时长（ms）：与逼近系数配（95 → 100 约 0.3s） */
export const SETTLE_MS = 420

/**
 * @param target 事件给的**真实**百分比（`percentOf(step, fractionInStep)`）
 * @param done 跑完没有 —— 置位后目标变 100（收尾）
 * @returns 两个 ref：数字那格（写 textContent）与条子填充那格（写宽度）。
 *          **不要**再把它的值放进 JSX 的 children / style —— 那会被 React 重渲覆盖，
 *          由下面那条无依赖 effect 负责在每次渲染后补写。
 */
export function useSmoothPercent(
  target: number,
  done: boolean,
): { numRef: RefObject<HTMLSpanElement>; fillRef: RefObject<HTMLSpanElement> } {
  const numRef = useRef<HTMLSpanElement>(null)
  const fillRef = useRef<HTMLSpanElement>(null)
  const shownRef = useRef(target)
  const targetRef = useRef(target)
  const doneRef = useRef(done)
  targetRef.current = target
  doneRef.current = done

  /* 把当前值铺到 DOM 上（两个属性赋值，不碰 React） */
  const paint = (v: number) => {
    if (numRef.current !== null) numRef.current.textContent = `${Math.round(v)}%`
    if (fillRef.current !== null) {
      fillRef.current.style.width = `${Math.min(100, Math.max(0, v))}%`
    }
  }

  /* 每次渲染后补写一次：React 重渲会把 JSX 里的初值铺回来（例：倒计时每秒 tick） */
  useEffect(() => {
    paint(shownRef.current)
  })

  useEffect(() => {
    let raf = 0
    let last = performance.now()

    const tick = (now: number) => {
      const dt = Math.min(0.12, (now - last) / 1000)
      last = now

      const want = Math.min(100, doneRef.current ? 100 : targetRef.current)
      const cur = shownRef.current
      /*
       * 两种走法，分清楚（第一版把它们揉在一起，蠕动被逼近系数又乘了一次 ⇒
       * 起点那几步实际只爬到 0.2%/s，"零的地方会顿"——实测采样抓出来的）：
       *
       * - **追事件值**（`want` 在 cur 前面）：逐帧指数逼近，帧间差值连续；
       * - **纯蠕动**（`want` 已经到过、还没来新事件）：直接用蠕动速度
       *   （CREEP_PER_SEC 自己已限过速，再乘逼近系数就变龟速了）。
       */
      const next =
        want > cur
          ? cur + (want - cur) * Math.min(1, APPROACH * dt)
          : Math.min(want + CREEP_CAP, cur + CREEP_PER_SEC * dt)
      const value = Math.min(100, Math.max(cur, next))

      shownRef.current = value
      paint(value)

      /* 停帧：追上目标（贴住）或蠕动到顶 ⇒ 不再要下一帧（常态零 rAF） */
      const settled = Math.abs(value - want) < EPS || value >= want + CREEP_CAP - EPS
      if (!settled) raf = requestAnimationFrame(tick)
    }

    raf = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(raf)
  }, [target, done])

  return { numRef, fillRef }
}
