/*
 * 后处理在后台跑 —— **标题栏那一枚小胶囊**（图标 + 百分比）。
 *
 * # 它从哪儿来
 *
 * 原来是页签下面的一条横条（`PostProcessStrip`，in-flow）：「后台运行」收起来之后
 * 你要有入口回得去。但它是**参与排版**的 —— 一出现整页往下推，作者 2026-10-17：
 * 「出现这个会把页面往下我不喜欢」「要不然就复用更新的进度的，用不同的图标？
 * 不用圆环，带动画的图标，处理中的感觉就可以，进度就用百分比」。
 *
 * ⇒ 现在住进标题栏（`TopTabs` 里、更新指示器左边），**不占内容流、不推任何一页**；
 *   点一下 = 把那一屏叫回来（`reopen`，与原来那条「看进度」同一个出口）。
 *
 * # 它什么时候在
 *
 * 与原来那条窄条**同一条判据**：这一趟真在跑（`run` 有、`finished` 还没有）**并且**
 * 用户把那一屏收起来了（`dismissed`）。那一屏自己开着的时候不需要它 —— 进度就在眼前。
 *
 * # 为什么是三颗点，不是转圈
 *
 * 作者：「不用圆环」「处理中的感觉就可以」。转圈在别处已经用过（更新那枚环），
 * 再来一个就分不清谁是谁；三颗依次起伏的点是"还在动"最省话的说法，也不容易
 * 和"加载中"混成同一件事。
 *
 * ★ 动效走 WAAPI（`element.animate()`），不走 CSS animation —— WKWebView 上
 *   CSS 动画不播（同 `HANDOVER.md` §4）。`prefers-reduced-motion` 时干脆不动。
 */

import { useEffect, useRef, useState } from 'react'

import TitlebarChip from '../components/TitlebarChip'
import { useT } from '../../i18n'
import { percentOf, stepName } from './steps'
import type { PostProcessRunView } from './usePostProcessRun'
import s from './PostProcessChip.module.css'

/** 一颗点起伏一次的时长（毫秒）与相邻两颗的错峰 */
const DOT_MS = 900
const DOT_STAGGER = 140

function Dots() {
  const hostRef = useRef<HTMLSpanElement | null>(null)

  useEffect(() => {
    const host = hostRef.current
    if (host === null) return
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return

    const dots = Array.from(host.children) as HTMLElement[]
    const anims = dots.map((dot, i) =>
      dot.animate(
        [
          { transform: 'translateY(0)', opacity: 0.35 },
          { transform: 'translateY(-3px)', opacity: 1, offset: 0.35 },
          { transform: 'translateY(0)', opacity: 0.35 },
        ],
        {
          duration: DOT_MS,
          delay: i * DOT_STAGGER,
          iterations: Infinity,
          easing: 'ease-in-out',
          /* 错峰那一段也要按 0% 那一帧画（否则第二、三颗要等各自的 delay 过了才动，
             前 140ms 会保持不透明，看起来像"先亮一下再开始"） */
          fill: 'both',
        },
      ),
    )
    return () => anims.forEach((a) => a.cancel())
  }, [])

  return (
    <span ref={hostRef} className={s.dots}>
      <i />
      <i />
      <i />
    </span>
  )
}

export default function PostProcessChip({ view }: { view: PostProcessRunView }) {
  const tr = useT()
  const { run, dismissed, reopen, quietUntil } = view
  /*
   * 「这枚胶囊在不在」—— **两档同一套逻辑**（2026-10-17 作者：
   * 「静默的他们的逻辑都一样复用的，一分钟自动消失，除非遇到下一个又刷新一分钟的，
   * 如果点击了出现了模态框就消失」）。
   *
   *   · **跑着**：`dismissed`（那一屏没在眼前 —— 静默档恒如此，GUI 档点了「后台运行」）
   *     ⇒ 显示三颗点 + 百分比；
   *   · **跑完**：`quietUntil` 有值 ⇒ 显示结论（已完成 / 没成），到点自己消失；
   *   · **点它**（`reopen`）⇒ 那一屏出来，`quietUntil` 立刻清掉，胶囊随之不见。
   *
   * ★ 60 秒的账记在 `usePostProcessRun` 的 `quietUntil`（结论到达那一刻定下），
   *   不是这里"挂载起数 60 秒" —— 那样窗口重开一次就重新计一遍，胶囊永远不消失。
   * ★ 下一趟开场会把 `quietUntil` 清成 `null`，于是"又刷新一分钟"这件事由
   *   **下一趟的结论到达**重新计时，不需要这里自己累加。
   */
  const expired = useRunExpiry(run, quietUntil)

  if (run === null) return null
  if (run.finished === null) {
    /* 跑着：只在那一屏没摆在眼前时才需要这一枚 */
    if (!dismissed) return null
  } else if (quietUntil === null || expired) {
    /* 跑完了但没有"留一会儿"的账（GUI 档下那一屏正在显示结论）⇒ 不摆 */
    return null
  }

  /* ---- 跑着：三颗点 + 百分比 ---- */
  if (run.finished === null) {
    const percent =
      run.progress === null ? 0 : percentOf(run.progress.step, run.progress.fractionInStep)
    const pct = Math.round(percent)
    const phase =
      run.progress === null ? tr('postproc.step.preparing') : stepName(tr, run.progress.step)

    return (
      <TitlebarChip
        testId="postprocess-chip"
        icon={<Dots />}
        label={`${pct}%`}
        title={tr('postproc.chip.runningTitle', { phase, pct })}
        ariaLabel={tr('postproc.chip.runningAria', { phase, pct })}
        onClick={reopen}
      />
    )
  }

  /* ---- 跑完了：一句话说结果，悬停看原因，点一下还能翻出那一屏 ---- */
  const ok = run.finished.ok
  return (
    <TitlebarChip
      testId="postprocess-chip"
      icon={<OutcomeGlyph ok={ok} />}
      label={ok ? tr('postproc.chip.done') : tr('postproc.chip.notDone')}
      tone={ok ? 'accent' : 'danger'}
      title={tr(ok ? 'postproc.chip.okTitle' : 'postproc.chip.notOkTitle', {
        message: run.finished.message,
      })}
      ariaLabel={tr(ok ? 'postproc.chip.okAria' : 'postproc.chip.notOkAria', {
        message: run.finished.message,
      })}
      onClick={reopen}
    />
  )
}

/**
 * 那颗胶囊**过期了没有**（过期 = 该消失了）。
 *
 * 两档共用（静默档 / GUI 档点过「后台运行」）—— 判据只看 `quietUntil` 有没有到点，
 * **不再分档**（作者：「静默的他们的逻辑都一样复用的」）。
 *
 * 到点那一刻自己刷一次，胶囊随之退场；`quietUntil` 变了（下一趟的结论到了）
 * 就重新排一次定时器 —— 那正是"又刷新一分钟"。
 */
function useRunExpiry(run: PostProcessRunView['run'], quietUntil: number | null): boolean {
  const [, force] = useState(0)
  const live = run !== null && run.finished !== null && quietUntil !== null

  useEffect(() => {
    if (!live || quietUntil === null) return
    const left = quietUntil - Date.now()
    if (left <= 0) return
    const t = window.setTimeout(() => force((n) => n + 1), left)
    return () => window.clearTimeout(t)
  }, [live, quietUntil])

  if (!live) return false
  return Date.now() >= quietUntil!
}

/** 结论那一枚小图标：成功是对勾、失败是感叹号（与 `tone` 同一套语义） */
function OutcomeGlyph({ ok }: { ok: boolean }) {
  return ok ? (
    <svg viewBox="0 0 16 16" width="14" height="14" fill="none" aria-hidden="true">
      <path
        d="m3.6 8.4 2.8 2.8 6-6.4"
        stroke="currentColor"
        strokeWidth="1.8"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  ) : (
    <svg viewBox="0 0 16 16" width="14" height="14" fill="none" aria-hidden="true">
      <circle cx="8" cy="8" r="6.2" stroke="currentColor" strokeWidth="1.4" />
      <path d="M8 4.8v4" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" />
      <circle cx="8" cy="11.2" r="0.7" fill="currentColor" />
    </svg>
  )
}
