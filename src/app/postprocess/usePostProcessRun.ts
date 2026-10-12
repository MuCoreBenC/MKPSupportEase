/*
 * 钩子那一趟的唯一客户端（那一屏模态框读它）。
 *
 * # 两条腿：先读快照，再挂事件
 *
 * 窗口起来时那一趟可能**已经跑到一半、甚至已经跑完**（早期的失败尤其快 ——
 * 参数错 / 预设读不出来都是毫秒级）。只挂事件的话，那一屏会永远停在"准备中"，
 * 而钩子进程其实早就退了。所以：
 *
 *   1. 首帧读一次 `getPostProcessRun()`（全量快照）；
 *   2. 之后每条事件只换掉快照里的那一格。
 *
 * 快照**不许盖掉**事件已经写进去的东西（读是异步的，事件可能更快）：用 `prev ?? snap`。
 *
 * # 窗口是常驻的
 *
 * 钩子跑完就退，但**这一屏不自动关**、窗口也留着（下次切片复用同一个窗口）——
 * 所以"用户关掉这一屏"得记一笔（`dismissed`），否则下一次读快照又把它顶回来。
 * 只有 `postprocess-started`（新一趟的开场）才把那一笔清掉。
 *
 * 事件名住 `./events`（与 Rust 侧 `hook_ui` 那几个常量同名的那一份）——
 * **报告页也认 `finished` 那一条**（跑完自己重读一遍），所以字面量只许有那一处。
 */

import { useCallback, useEffect, useState } from 'react'

import { api, listen } from '../../api'
import type {
  PostProcessFacts,
  PostProcessFinished,
  PostProcessPrintTime,
  PostProcessProgress,
  PostProcessQuestion,
  PostProcessRun,
  PostProcessStarted,
} from '../../api/contract'

import {
  POSTPROCESS_FACTS as FACTS_EVENT,
  POSTPROCESS_FINISHED as FINISHED_EVENT,
  POSTPROCESS_PRINT_TIME as PRINT_TIME_EVENT,
  POSTPROCESS_PROGRESS as PROGRESS_EVENT,
  POSTPROCESS_QUESTION as QUESTION_EVENT,
  POSTPROCESS_STARTED as STARTED_EVENT,
} from './events'

export interface PostProcessRunView {
  /** 现在这一趟（**不看用户关没关** —— 后台那条窄条要它）；`null` = 这一趟还没来过 */
  run: PostProcessRun | null
  /** 用户把那一屏收起来了没有（`true` = 后台跑着，界面上只剩一条窄条） */
  dismissed: boolean
  /**
   * 这一趟开场那一刻（界面自己看见 `postprocess-started` 时记的毫秒）。
   *
   * 「预计还需多久」拿它算 —— 窗口开在**半路**时这一格是 `null`（起点没看见），
   * 那一屏就不摆预计时间：编一个比空着糟。
   */
  sinceMs: number | null
  /** 收起那一屏 —— 跑着的时候就是「后台运行」那颗按钮（只影响这一屏；结论还在报告页里） */
  dismiss: () => void
  /** 把那一屏叫回来（点那条窄条） */
  reopen: () => void
}

export function usePostProcessRun(): PostProcessRunView {
  const [run, setRun] = useState<PostProcessRun | null>(null)
  const [dismissed, setDismissed] = useState(false)
  const [sinceMs, setSinceMs] = useState<number | null>(null)

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
        console.error('[postprocess] 读那一趟的快照失败', err)
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

    /*
     * 新一趟开场：把上一趟的结论留着的显示清掉、把"我关过了"那一笔清掉、记下起点。
     *
     * ★★ 这一条**是唯一能从"什么都没有"直接建出这一趟**的事件（其余几条见 `bind`）。
     *   为什么必须这样：**「界面一直开着、钩子后来才来」是最常见的一路** —— 首帧快照
     *   那时候还没有这一趟（`None`），而 `bind` 那几条一律 `prev === null ? prev : …`
     *   （它们手里没有机型 / 文件名，建不出场）。只认"这一趟已经开了"的话，后面每一条
     *   都被丢掉，那一屏永远不会出现。
     *
     *   作者 2026-10-11 实测到的就是这一条：窗口 20:54 起的、钩子 20:58 才来 ——
     *   服务端日志里"钩子来了 / 那一趟结束"一行不少，界面上却什么都没有。
     *
     *   起点也在这里记（不是 setState 外面）：订阅晚于开场时也不丢。
     */
    void listen<PostProcessStarted>(STARTED_EVENT, (started: PostProcessStarted) => {
      if (!alive) return
      setRun({
        presetName: started.presetName,
        gcodeName: started.gcodeName,
        facts: null,
        question: null,
        progress: null,
        finished: null,
      })
      setDismissed(false)
      setSinceMs(Date.now())
    }).then((off) => {
      if (alive) offs.push(off)
      else off()
    })

    /* 事实那一条紧跟在开场后面（左栏那张表），收不到就空着 —— 不编 */
    bind<PostProcessFacts>(FACTS_EVENT, (prev, facts) => ({ ...prev, facts }))
    bind<PostProcessProgress>(PROGRESS_EVENT, (prev, progress) => ({ ...prev, progress }))
    bind<PostProcessQuestion>(QUESTION_EVENT, (prev, question) => ({ ...prev, question }))
    bind<PostProcessFinished>(FINISHED_EVENT, (prev, finished) => ({
      ...prev,
      finished,
      // 结论已出：那一问不可能还有人在等，界面别再挂着它
      question: null,
    }))
    /*
     * 打印时间补算完（延后那一趟）：补进**已经在屏上的那条结论**里。
     *
     * 有界面时钩子不等打印时间（切片器在等退出码）⇒ 结论里那一格先是空的；
     * 常驻进程算完推这一条，那一行就亮起来 —— 用户不用跑去报告页找那个数。
     * 补算失败不发这条（那一格继续空着，不许拿 0 凑数）。
     */
    bind<PostProcessPrintTime>(PRINT_TIME_EVENT, (prev, payload) => {
      if (prev.finished === null) return prev
      return {
        ...prev,
        finished: {
          ...prev.finished,
          result: { ...(prev.finished.result ?? {}), printTimeSeconds: payload.totalSeconds },
        },
      }
    })
    /*
     * 跑完把那一屏叫回来（如果用户正在后台跑）。
     *
     * 这是旧世代那条「**结论留在屏上**」：跑到一半点「后台运行」收起来的人，等的是结论 ——
     * 让他自己去找入口才叫弄丢了。**不是通知**（窗口不抢焦点、也不弹系统横幅）。
     */
    void listen<PostProcessFinished>(FINISHED_EVENT, () => {
      if (alive) setDismissed(false)
    }).then((off) => {
      if (alive) offs.push(off)
      else off()
    })

    return () => {
      alive = false
      for (const off of offs) off()
    }
  }, [])

  const dismiss = useCallback(() => setDismissed(true), [])
  const reopen = useCallback(() => setDismissed(false), [])

  return { run, dismissed, sinceMs, dismiss, reopen }
}
