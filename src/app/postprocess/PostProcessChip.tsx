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

import { useEffect, useRef } from 'react'

import TitlebarChip from '../components/TitlebarChip'
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
  const { run, dismissed, reopen } = view
  if (run === null || !dismissed || run.finished !== null) return null

  const percent =
    run.progress === null ? 0 : percentOf(run.progress.step, run.progress.fractionInStep)
  const pct = Math.round(percent)
  const phase = run.progress === null ? '准备中' : stepName(run.progress.step)

  return (
    <TitlebarChip
      testId="postprocess-chip"
      icon={<Dots />}
      label={`${pct}%`}
      title={`后处理在后台跑：${phase} · ${pct}%（点击看进度）`}
      ariaLabel={`后处理在后台跑：${phase}，${pct}%，点击看进度`}
      onClick={reopen}
    />
  )
}
