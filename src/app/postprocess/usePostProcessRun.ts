/*
 * 钩子那一趟的唯一客户端（那一屏模态框读它）。
 *
 * # 两条腿：先读快照，再挂事件
 *
 * 窗口起来时那一趟可能**已经跑到一半、甚至已经跑完**（早期的失败尤其快 ——
 * 参数错 / 预设读不出来都是毫秒级）。只挂事件的话，那一屏会永远停在"准备中"，
 * 而进程其实早就退了。所以：
 *
 *   1. 首帧读一次 `getPostProcessRun()`（全量快照）；
 *   2. 之后每条事件只换掉快照里的那一格。
 *
 * 快照**不许盖掉**事件已经写进去的东西（读是异步的，事件可能更快）：用 `prev ?? snap`。
 *
 * 事件名与 Rust 侧 `hook_ui` 那三个常量同名 —— 两边没有编译器，靠这条注释对齐。
 */

import { useEffect, useState } from 'react'

import { api, listen } from '../../api'
import type {
  PostProcessFinished,
  PostProcessProgress,
  PostProcessQuestion,
  PostProcessRun,
} from '../../api/contract'

const PROGRESS_EVENT = 'postprocess-progress'
const QUESTION_EVENT = 'postprocess-question'
const FINISHED_EVENT = 'postprocess-finished'

export function usePostProcessRun(): PostProcessRun | null {
  const [run, setRun] = useState<PostProcessRun | null>(null)

  useEffect(() => {
    let alive = true
    const offs: (() => void)[] = []

    void api
      .getPostProcessRun()
      .then((snap) => {
        if (!alive) return
        setRun((prev) => prev ?? snap)
      })
      .catch((err: unknown) => {
        // 读不到快照 = 这一屏什么都显示不了；但**不装成"没有在跑"**，日志里留一条
        console.error('[postprocess] 读钩子那一趟的快照失败', err)
      })

    const bind = <T,>(event: string, apply: (prev: PostProcessRun, payload: T) => PostProcessRun) => {
      void listen<T>(event, (payload) => {
        if (!alive) return
        setRun((prev) => (prev === null ? prev : apply(prev, payload)))
      }).then((off) => {
        // 订阅的回执比卸载晚到时，就地退订（否则监听器留在后端没人收）
        if (alive) offs.push(off)
        else off()
      })
    }

    bind<PostProcessProgress>(PROGRESS_EVENT, (prev, progress) => ({ ...prev, progress }))
    bind<PostProcessQuestion>(QUESTION_EVENT, (prev, question) => ({ ...prev, question }))
    bind<PostProcessFinished>(FINISHED_EVENT, (prev, finished) => ({
      ...prev,
      finished,
      // 结论已出：那一问不可能还有人在等，界面别再挂着它
      question: null,
    }))

    return () => {
      alive = false
      for (const off of offs) off()
    }
  }, [])

  return run
}
