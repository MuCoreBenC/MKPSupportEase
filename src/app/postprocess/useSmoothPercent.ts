/*
 * 平滑进度：把"每 1% 一跳"的进度事件插成**一直在走**的一根条。
 *
 * 作者口径（三次合起来）：
 * - 「一顿一顿的我不喜欢，不够平滑」（2026-10-14）
 * - 「数字也卡顿，进度条也一顿一顿的」（同日晚，真机录屏）
 * - 「他会在那几个关键的地方停顿……比如停在 0、3、十几、54 这些地方」——
 *   那些正是**每一步的起点**（那一步还没发第一个进度事件，`fractionInStep = null`，
 *   `percentOf` 只能给该步起点）⇒ 条子干等。
 * - 以及早前定死的「进度条冲到 95% 再卡住那种错觉，就是虚报出来的」（`steps.ts`）。
 *
 * # 业界怎么解（三类，本文件用的是 ②+③ 的组合）
 *
 * ① **纯爬（creep）**：`nprogress` / `pace.js` / 浏览器加载条 —— 只按时间往前爬，
 *    适合"没有真实进度"的场合；
 * ② **速率外推（rate extrapolation）**：用**最近观测到的真实速率**往前推 ——
 *    `tqdm` 的 `smoothing=0.3`（EMA 平滑）与 Chrome 下载条、iOS 的
 *    `UIProgressView` + 剩余时间估计都是这一路；
 * ③ **上限封顶（lookahead cap）**：外推**只许比事件值超前 [#LOOKAHEAD] 个百分点**，
 *    这就是"不虚报"的那道线 —— `progress-estimator`（bvaughn）与 VS Code 的
 *    `withProgress` 都是"真实值为下限、估算为上限"的两段式。
 *
 * 本项目还有一个别处没有的升级口（**还没用**）：执行记录里存着每一步的
 * `StepTiming.elapsed_ms`，将来可以拿"这一步通常多久"（`progress-estimator` 的思路）
 * 替换这里的即时速率 —— 首次运行没有历史时仍是本文件的速率外推。
 *
 * # 为什么**不用 React state**（第二版改掉的）
 *
 * 第一版每帧 `setShown(v)` 让 React 重渲 —— **真机 WKWebView 上就成了"一顿一顿"**：
 * React 18 的调度器会把高频 setState **合并 / 延后**（低优先级更新被抽样）。
 * 现在改成**命令式直写 DOM**：rAF 里只 `textContent =` 与 `style.width =`，
 * 零 React 渲染；React 重渲（倒计时每秒 tick）后由一条无依赖 effect 补写回当前值。
 */

import { useCallback, useEffect, useRef } from 'react'
import type { RefObject } from 'react'

import { STORAGE } from '../../api/storageKeys'
import { STEPS } from './steps'

/*
 * 外推上限（"不虚报"的那道线）—— **不是死数，而是"等得越久、允许越松"**：
 *
 *   允许超前 = LOOKAHEAD_BASE + 已等待秒数 × LOOKAHEAD_GROW_PER_SEC，硬顶 LOOKAHEAD_MAX
 *
 * 为什么不做成死的 3 点（上一版）：「步骤之间的衔接」那一档里，等 1 秒和等 10 秒
 * 得到的余量一样 —— 走满 3 点就死停，看起来还是"卡住"。改成随等待放开之后：
 * 观感是"**一直在往前磨**"，而硬顶保证了它仍然**不撒谎**（真实事件一到立刻回去对齐）。
 *
 * ★ 2026-10-14 晚作者两句话把参数再推了一档：
 *   「56 那个地方还可以多磨一点，我看到他磨到 58 就停了」⇒ MAX 6 → 9、放开速度 0.8 → 1.5；
 *   「一开始 0 那个地方等得比较久」⇒ 兜底爬升 1.2 → 3（开场没有任何速率信息时的下限）。
 */
const LOOKAHEAD_BASE = 3
const LOOKAHEAD_MAX = 9
/** 每等 1 秒，上限多放开多少点 */
const LOOKAHEAD_GROW_PER_SEC = 1.5
/** 速度估计的 EMA 系数（新样本占比）；越小越稳、越大越跟手 */
const SPEED_EMA = 0.45
/**
 * 估速的采样间隔（秒）：攒够这么久才采一个样本。**不能太大** ——
 * 小文件跑完只要 1~2 秒，0.25 秒才采一次的话，学到"真实 68 点/秒"已经跑完了
 * （2026-10-14 实测：done 时条子才 31、事件已经 100）。
 * 0.1 秒：开场那几步（读文件 / 读预设，几十毫秒一步）也够采到一两个样本，
 * 免得"没有速率信息 ⇒ 落到兜底爬升"这种来源突变造成的顿挫。
 */
const SPEED_MIN_DT = 0.1
/** 没有任何速率信息时的兜底爬升（百分点/秒）—— 开场那几步用 */
const CREEP_PER_SEC = 3
/*
 * 逼近系数（每秒）—— "比例项"的斜率：差距越大走得越快，`1 / APPROACH` 秒走掉 63%。
 *
 * ★ 2026-10-14 晚第二版（作者："开头的动作有点顿，后半部分均匀了"）：
 * 原来写 10（0.1 秒走掉 63%）—— 对"差距 80 点的收尾"是"0.3 秒的漂亮尾巴"，
 * 但**开场那几步差距只有 1~2 点**时，它 0.1 秒就把差距走光了 ⇒ 条子"窜一下、停住"，
 * 而那一步的事件可能还要等 0.5 秒 ⇒ 就是作者说的"顿"。
 * 改成 1.2（0.83 秒走掉 63%）：开场那几步变成"缓缓铺过去"，与后半部分同一节奏。
 * 代价：大文件收尾（差距约 15 点）从 0.3 秒拉长到约 1 秒 —— 收尾本来就有
 * `SETTLE_PER_SEC` 限速（25 点/秒），这里只是让它更从容，观感反而统一。
 */
const APPROACH = 1.2
/**
 * 追赶速度上限（百分点/秒）—— 差距大时不许"一跳"，要**滑**过去。
 *
 * 作者 2026-10-14：「现在容易从 3 或者 8 直接跳到 11 不够平滑」。根因是逼近原来是
 * **纯比例**（`差距 × APPROACH`）：差距 8 点时起步就是 80 点/秒，60fps 一帧 1.3 点，
 * 三帧跳完 —— 数学上是"指数收敛"，观感上就是"跳"。
 * 现在"比例 + 限速"两者取小：差距小按比例自然收尾（那 0.3 秒的漂亮尾巴还在），
 * 差距大按这个速度滑过去（8 点 ≈ 0.67 秒）。真实速率比它快时按真实速率放开，不拖后腿。
 */
const CHASE_PER_SEC = 12
/**
 * 追赶速度的**绝对**上限（百分点/秒）—— 防止"一次大跳变把估计速率带爆"从而绕过上面那条限速：
 * 换步后的第一个事件可能是一次 8 点的跳变，`dv/dt` 一瞬就是几十点/秒，`rate × 1.2` 跟着涨，
 * 限速就形同虚设了。
 *
 * 80（不是 40）：**要盖过"小文件的事件速度"** —— 实测内存里跑一个小切片，
 * 事件推进能到 68+ 点/秒（1~2 秒整个跑完）。天花板压在 40 的话条子永远追不上，
 * 完成时还挂在 60 多（2026-10-14 作者："有的文件小的，60 多就突然完成了"）。
 */
const MAX_CHASE_PER_SEC = 80
/** 到位判据（百分点） */
const EPS = 0.05

/**
 * 收尾（`done` 之后）的比例系数（每秒）—— **收尾单独一套，不跟过程的 `APPROACH`**。
 *
 * 这一轮的口径（作者三句话定下来的）：
 *   · 「动画慢，起码 3 秒」→ 试过限速 25 点/秒；
 *   · 「不用限制 3 秒 … 动画自然均匀」；
 *   · 「收尾慢了，100% 停太久」→「**这个限制感觉是限制了收尾**」——
 *     限速（哪怕是 60 点/秒）终究是"按住它走"，有黏滞感。**收尾要的是指数冲线**：
 *     起步快、尾巴自然收敛，跟过程用的是同一种数学，只是系数大。
 * 8（每秒）：0.125 秒走掉 63% 的差距 —— 87 → 100 约 0.3 秒、60 → 100 约 0.5 秒，
 * 是"利落的一冲"，不是"被限速拖着"。
 */
const SETTLE_APPROACH = 8
/**
 * 调用方在"条子到 100"之后停留的时长（ms）。
 * 420 → 150：作者「100% 停太久了」—— 那 420 是当初给"95% 一闪而过"留的看清时间，
 * 现在条子自己会报告"走到了"，只在 100% 上点一下头就走（150ms ≈ 两帧多，看清又不黏）。
 */
export const SETTLE_MS = 150

/** 某一步的全局权重（百分点跨度）—— 用来把"整步耗时"折成"每百分点耗时" */
function weightOf(id: string | null): number {
  if (id === null) return 0
  return STEPS.find((s) => s.id === id)?.weight ?? 0
}

/**
 * @param target 事件给的**真实**百分比（`percentOf(step, fractionInStep)`）
 * @param done 跑完没有 —— 置位后目标变 100（收尾）
 * @param step 当前步的**稳定 id**（`run.progress?.step`）—— 跨步衔接要用它：
 *             上一步的"整步平均节奏"是下一步最好的先验（`parse/pass1/pass2/write`
 *             都是逐行扫，**耗时 ∝ 文件大小** ⇒ 大小差异自动被这条节奏吸收）。
 * @returns 两个 ref：数字那格（写 textContent）与条子填充那格（写宽度）。
 *          **不要**再把它的值放进 JSX 的 children / style —— 那会被 React 重渲覆盖，
 *          由下面那条无依赖 effect 负责在每次渲染后补写。
 */
export function useSmoothPercent(
  target: number,
  done: boolean,
  step: string | null,
  /** 本次处理的 G-code 字节数（`run.facts.fileSizeBytes`）—— 跨运行学习按 MB 归一要用；不知道给 null */
  sizeBytes: number | null,
  /**
   * 条子**走到 100** 时回调一次（不是 done 那一刻 —— 是动画真走完那一刻）。
   * 调用方拿它决定"什么时候切结果态"（这样它不必猜条子还要走多久）。
   */
  onSettled?: () => void,
): { numRef: RefObject<HTMLSpanElement>; fillRef: RefObject<HTMLSpanElement> } {
  const numRef = useRef<HTMLSpanElement>(null)
  const fillRef = useRef<HTMLSpanElement>(null)
  const shownRef = useRef(target)
  const targetRef = useRef(target)
  const doneRef = useRef(done)
  targetRef.current = target
  doneRef.current = done

  /**
   * 最近一次"事件值变化"的时刻与值（外推的基准点；跨步不重置 —— 步骤切换的空档正要用它）。
   * `at = -1` 是刻意的：**本轮挂载时"本步还没收到过事件"**（见下面 `noEventYet` 的判据），
   * 开场那几步才能走进"用先验"的分支（原来初始化成 `performance.now()`，
   * 与 `stepStartRef` 同一时刻 ⇒ `<` 不成立 ⇒ 先验永远用不上）。
   */
  const anchorRef = useRef({ value: target, at: -1 })
  /** 速率（百分点/秒，EMA 平滑）。0 = 还没有信息 ⇒ 走兜底爬升 */
  const speedRef = useRef(0)

  /** 本次文件大小（MB）；0 = 未知或太小 ⇒ 不学（归一分母不能是 0） */
  const sizeMbRef = useRef(0)
  sizeMbRef.current = sizeBytes !== null && sizeBytes > 0 ? sizeBytes / (1024 * 1024) : 0

  /*
   * 跨运行学习（作者 2026-10-14：「可以，跨运行学习」）：
   *
   * `STORAGE.clientPostProcessPace` 里存着"**每一步每 MB 多少毫秒**"。
   * 逐行扫的几步耗时 ∝ 文件大小，所以按 MB 归一之后**换文件大小也能直接用** ——
   * 开场那几步（input / config）本来没有任何速率信息，现在第二次跑就有先验了，
   * 正是作者说的「一开始 0 那个地方等得比较久」的解法。
   * 读失败 / 坏值一律当空表（**这是缓存不是底账**，丢了只是下次重新学）。
   */
  const paceTableRef = useRef<Record<string, number> | null>(null)
  const readPaceTable = useCallback((): Record<string, number> => {
    if (paceTableRef.current !== null) return paceTableRef.current
    let table: Record<string, number> = {}
    try {
      const raw = window.localStorage.getItem(STORAGE.clientPostProcessPace)
      if (raw !== null) {
        const parsed: unknown = JSON.parse(raw)
        if (parsed !== null && typeof parsed === 'object' && !Array.isArray(parsed)) {
          for (const [k, v] of Object.entries(parsed as Record<string, unknown>)) {
            if (typeof v === 'number' && Number.isFinite(v) && v > 0) table[k] = v
          }
        }
      }
    } catch {
      table = {}
    }
    paceTableRef.current = table
    return table
  }, [])

  /*
   * 跨步衔接（作者 2026-10-14：「后续要改进的话，就是得改进那些不同步骤之间的衔接」）：
   *
   * 换步那一刻，本步的第一个事件**还没到** —— 这正是"停在 54 / 十几"那类空档。
   * 此时用**上一步的整步平均节奏**（每百分点多少毫秒）来预测本步：
   * 逐行扫的那几步（`parse`/`pass1`/`pass2`/`write`）耗时都 ∝ 文件大小，
   * 所以"上一步每走 1% 用了多久"就是对下一步最好的先验 —— **大文件小文件都不用另做标定**。
   * 步内一旦有事件，就切回瞬时速率（更跟手）。
   */
  const stepRef = useRef<string | null>(step)
  const stepStartRef = useRef(performance.now())
  /** 上一步的"每百分点耗时"（ms/点）；0 = 还没攒到可信的一次 */
  const paceRef = useRef(0)

  /**
   * 学一格：把"某一步花了 `elapsedMs` 毫秒"按 MB 归一（存**每 MB 毫秒**，换文件大小也能用）、
   * EMA 进表、落盘。**两个调用点**：换步时结算上一步、`done` 时结算最后一步
   * （最后一步后面没有新步来触发结算 —— 不补这一下，`printtime` 永远学不到）。
   * 门槛：这一步得真走了一截（> 400ms）、文件大小得知道（分母不能是 0）。
   */
  const learnStep = useCallback(
    (id: string, elapsedMs: number) => {
      const mb = sizeMbRef.current
      if (mb <= 0.05 || elapsedMs <= 400) return
      const table = readPaceTable()
      const sample = elapsedMs / mb
      const old = table[id]
      table[id] = old === undefined ? sample : old * 0.7 + sample * 0.3
      try {
        window.localStorage.setItem(STORAGE.clientPostProcessPace, JSON.stringify(table))
      } catch {
        /* 存不进去就算了 —— 这是缓存，不是底账 */
      }
    },
    [readPaceTable],
  )

  useEffect(() => {
    if (step === stepRef.current) return
    const now = performance.now()
    const prev = stepRef.current
    const elapsed = now - stepStartRef.current
    const span = weightOf(prev)
    /* 上一步真走了一截（有跨度、且不是零点几秒那种小步）才记节奏 */
    if (prev !== null && span > 0 && elapsed > 400) {
      paceRef.current = elapsed / span
      learnStep(prev, elapsed)
    }
    stepRef.current = step
    stepStartRef.current = now
  }, [step, learnStep])

  /* 条子走到 100 的通知（只发一次；`done` 回落时重置，下一次跑还能再发） */
  const onSettledRef = useRef(onSettled)
  onSettledRef.current = onSettled
  const settledNotifiedRef = useRef(false)
  useEffect(() => {
    if (!done) settledNotifiedRef.current = false
  }, [done])

  /* 收尾：结算最后一步（只学一次 —— 重渲会多次进这条 effect） */
  const learnedFinalRef = useRef(false)
  useEffect(() => {
    if (!done) {
      learnedFinalRef.current = false
      return
    }
    if (learnedFinalRef.current) return
    learnedFinalRef.current = true
    const cur = stepRef.current
    if (cur !== null) learnStep(cur, performance.now() - stepStartRef.current)
  }, [done, learnStep])

  /**
   * 采样基准（只给估速用）—— **与 `anchorRef` 分开**：
   * 上个版本两者合一，于是"间隔太短就挪基准点"把 `dt` 一直清零 ——
   * 事件快的时候（内核每 50ms 报一跳）**速率永远采不到样本**，显示便跟不上事件
   * （2026-10-14 实测：pass2 每 50ms +1.6 点，条子只走 9 点/秒）。
   * 现在：`anchorRef` 跟每次事件走（外推基准必须新鲜），采样只在攒够
   * `SPEED_MIN_DT` 之后做一次、且用"上次采样到这次"的差。
   */
  const sampleRef = useRef({ value: target, at: -1 })

  /* 每次 target 变化：按"距上次采样增了多少、隔了多久"更新速率（样本间隔太短就继续攒） */
  useEffect(() => {
    const now = performance.now()
    const dt = (now - sampleRef.current.at) / 1000
    const dv = target - sampleRef.current.value
    if (dt >= SPEED_MIN_DT && dv > 0) {
      speedRef.current = speedRef.current * (1 - SPEED_EMA) + (dv / dt) * SPEED_EMA
      sampleRef.current = { value: target, at: now }
    } else if (dv < 0) {
      /* 进度倒退（换步重开）：重置采样基准，免得下一段把差算成天文数字 */
      sampleRef.current = { value: target, at: now }
    }
    anchorRef.current = { value: target, at: now }
  }, [target])

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
       * 目标位置 = "事件值 + 速率 × 距上次事件的时间"，并封顶：
       *   · 收尾时封顶就是 100；
       *   · 平时最多比事件值超前 LOOKAHEAD 个百分点（**不虚报**的那道线）。
       * 步骤切换/事件空档里 speed > 0 ⇒ 条子按刚才的真实速度继续走 —— 正是
       * "停在 0 / 3 / 十几 / 54" 那几个关键点的解法。
       */
      /*
       * "本步等了多久" —— 基准取 `max(上一次事件时刻, 本步起点)`：
       *   · 步内：= 距上次事件的时间（外推的时长）；
       *   · 换步后 / 刚挂载：= 本步开始以来的时间（`anchor.at` 还停在上一步或为 -1）。
       * 上一版直接用 `now - anchor.at`：换步后会把"上一步的等待"也算进来（偏大）。
       */
      const since = Math.max(anchorRef.current.at, stepStartRef.current)
      const idle = (now - since) / 1000
      /*
       * 速率来源，优先序（越靠前越可信）：
       *   ① 本步已收到的瞬时速率（`speedRef`，步内事件驱动）；
       *   ② 本次运行**上一步的实测节奏**（`paceRef`）—— 跨步空档用它；
       *   ③ **跨运行学到的"这一步每 MB 多少毫秒"**（表 × 本次文件大小）——
       *      **开场（input / config 那几步）就用它**，正是"0 那个地方等得久"的解；
       *   都没有 ⇒ 0，落到下面的兜底爬升（CREEP_PER_SEC）。
       */
      let rate = 0
      if (anchorRef.current.at < stepStartRef.current) {
        /* 本步一个事件都还没到 */
        rate = paceRef.current > 0 ? 1000 / paceRef.current : 0
        if (rate === 0) {
          const prior = stepRef.current !== null ? readPaceTable()[stepRef.current] : undefined
          const mb = sizeMbRef.current
          const span = weightOf(stepRef.current)
          if (prior !== undefined && mb > 0 && span > 0) {
            /* 表里是"该步每 MB 毫秒" ⇒ 折成"每百分点多少毫秒"再取倒数 */
            const paceMs = (prior * mb) / span
            if (paceMs > 0) rate = 1000 / paceMs
          }
        }
      } else {
        rate = speedRef.current
      }
      const extrapolated = want + rate * idle
      /* 上限随"等了多久"放开（硬顶 LOOKAHEAD_MAX）——见文件头那条常量说明 */
      const cap = Math.min(LOOKAHEAD_MAX, LOOKAHEAD_BASE + idle * LOOKAHEAD_GROW_PER_SEC)
      const ceiling = doneRef.current ? 100 : Math.min(want + cap, 100)

      /*
       * ★★ 滞后**下限**（2026-10-14 晚，作者："有的文件小的，60 多就突然完成了"）——
       *
       * 限速（CHASE_PER_SEC）只管"超前"这一个方向，**滞后没有底线**：
       * 小文件 1~2 秒跑完，事件推进 50+ 点/秒，条子被 12 点/秒限着追 ——
       * 事件到 100 时条子还在 60，`done` 一到（420ms 后切结果态）就被切掉 ⇒
       * 观感就是"**60 多突然完成**"。
       *
       * 现在给滞后划一条线：**最多落后 `lag` 点**，超线就无条件提速追。
       * `lag = max(cap, rate × 0.4)` —— 高速时按"**时间滞后 0.4 秒**"折算成点数
       * （50 点/秒 ⇒ 允许落后 20 点）：观感上仍是"紧跟且平滑"，但绝不会被甩掉。
       */
      const lag = Math.max(cap, rate * 0.4)
      const floor = doneRef.current ? 100 : Math.max(0, want - lag)
      const goal = Math.min(Math.max(extrapolated, floor), ceiling)

      /*
       * 逼近目标 —— **比例 + 限速，两者取小**（见 CHASE_PER_SEC 那条说明）：
       *   · 差距小：比例项说了算，0.3 秒那种平滑尾巴；
       *   · 差距大（换步 / 事件跳变，比如 3 → 11）：限速项说了算，滑过去而不是跳过去；
       *   · 真实速率更快（高速步）：限速跟着 rate 放开，不拖后腿；
       *   · **落后于下限**（高速事件把条子甩了）：按绝对上限 40 点/秒追。
       *
       * ★ 收尾（done）：**冲线走纯比例**（0.3 秒那种速度，从 60 冲到 100 约 0.35 秒）——
       * 收尾要的正是"冲线感"，这里不适用"不许跳"那条（那一帧之后条子就让位给结果态了）。
       *
       * ★ 目标落在身后时（外推没起来 / 刚冲过头）**以兜底速度朝上限爬** ——
       * 上一版写的是 `min(goal, cur + CREEP*dt)`，`goal` 正落在 `cur` 上时会把条子
       * **钉死在原地**（开场 0% 那几秒就是这么"干等"的）。朝 `ceiling` 爬才对。
       */
      const chaseLimit = Math.min(
        MAX_CHASE_PER_SEC,
        Math.max(CHASE_PER_SEC, rate * (cur < floor ? 1.8 : 1.2)),
      )
      const next =
        doneRef.current
          ? /* 收尾：指数冲线（单独的大系数，不设点数限速）—— 见 SETTLE_APPROACH */
            cur + (100 - cur) * Math.min(1, SETTLE_APPROACH * dt)
          : goal > cur
            ? cur +
              Math.min((goal - cur) * Math.min(1, APPROACH * dt), chaseLimit * dt)
            : Math.min(ceiling, cur + CREEP_PER_SEC * dt)
      const value = Math.min(100, Math.max(cur, next))

      shownRef.current = value
      paint(value)

      /*
       * 条子"走到了"⇒ 通知调用方一次（它据此切结果态）。
       * 判据按**显示值**（`Math.round` 显示 100 就算到）而不是 `ceiling - EPS`（99.95）——
       * 指数冲线的尾巴是渐近的，等 99.95 要多花小半秒；用户看见的就是"100%"那一下，
       * 用显示值做判据，切换时刻与眼睛看到的完全一致。
       */
      if (doneRef.current && !settledNotifiedRef.current && Math.round(value) >= Math.round(ceiling)) {
        settledNotifiedRef.current = true
        onSettledRef.current?.()
      }

      /* 停帧：到外推上限就停（新事件来了 useEffect 会重启这个循环） */
      if (value < ceiling - EPS) raf = requestAnimationFrame(tick)
    }

    raf = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(raf)
  }, [target, done, readPaceTable])

  return { numRef, fillRef }
}
