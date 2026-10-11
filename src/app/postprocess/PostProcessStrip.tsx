/*
 * 后台跑着那条窄条 —— 只在**用户把那一屏收起来了**、而那一趟还在跑的时候出现。
 *
 * # 为什么必须有它
 *
 * 「后台运行」如果只是把模态框藏掉，用户就再没有入口看进度、也没法按停止 ——
 * 那不叫后台运行，叫弄丢了。窄条与导入结果条同一个位置、同一套长相（in-flow、
 * 不遮内容、不自己消失），点一下把那一屏叫回来。
 *
 * 跑完之后窄条自己消失：那一屏会被**叫回来**摆结论（旧世代那条「结论留在屏上」，
 * 见 `usePostProcessRun` 里 `finished` 那一条订阅）—— 所以窄条只管"跑着"这一段。
 */

import { percentOf, stepName } from './steps'
import type { PostProcessRunView } from './usePostProcessRun'
import s from './PostProcessStrip.module.css'

export default function PostProcessStrip({ view }: { view: PostProcessRunView }) {
  const { run, dismissed, reopen } = view
  if (run === null || !dismissed || run.finished !== null) return null

  const percent =
    run.progress === null ? 0 : percentOf(run.progress.step, run.progress.fractionInStep)

  return (
    <div className={s.strip} role="status">
      <span className={s.dot} aria-hidden="true" />
      <span className={s.text}>
        后处理在后台跑：{run.progress === null ? '准备中' : stepName(run.progress.step)}
      </span>
      <span className={s.pct}>{Math.round(percent)}%</span>
      <button type="button" className={s.open} onClick={reopen}>
        看进度
      </button>
    </div>
  )
}
