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

import { useCallback, useEffect, useRef, useState } from 'react'

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

/**
 * 静默档下，结论那枚胶囊在标题栏留多久（毫秒）。
 *
 * 作者 2026-10-17：「右上角的一分钟后自动消失」。一分钟够他扫一眼"这一盘成没成"，
 * 又不至于一直挂在那儿像没处理完。
 */
export const SILENT_KEEP_MS = 60_000

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
  /**
   * **静默档**（设置页「后处理模式」选了静默）。
   *
   * `true` 时这一屏**永远不会自己弹出来**：不是"弹了再收"，是压根不弹 ——
   * 用户选静默要的就是"别拿窗口打扰我"。只有他点右上角那枚状态胶囊才会把它叫回来
   * （`reopen` 仍然有效，那是一次主动动作，不算打扰）。
   *
   * ★ 与 `dismissed` 的区别：`dismissed` 是"用户关过这一屏"，`silent` 是"这一档根本不许弹"。
   *   两者都不弹，但语义不同 —— 静默档下 `dismissed` 始终为 `true`（由这里统一置位）。
   */
  silent: boolean
  /**
   * 静默档下结论那枚胶囊**留到哪一刻**（unix 毫秒；`null` = 不留 / 不是静默档）。
   *
   * 结论到达时定下 `now + SILENT_KEEP_MS`（60 秒）。到点胶囊自己消失
   * （作者 2026-10-17：「右上角的一分钟后自动消失」）。
   * 用户点过胶囊（`reopen` → 把那一屏叫出来）就算"看过了"，立刻清掉 —— 不再等满一分钟。
   */
  quietUntil: number | null
}

export function usePostProcessRun(silent = false): PostProcessRunView {
  const [run, setRun] = useState<PostProcessRun | null>(null)
  /* 静默档在回调里也要读（事件回调只装一次，不能闭包旧的 `silent`） */
  const silentRef = useRef(silent)
  silentRef.current = silent
  /*
   * `dismissed` 记的是"用户主动关过这一屏"（GUI 档那颗「后台运行」）。
   *
   * ★★ 不拿 `silent` 初始化它 —— `useState(silent)` 只吃**第一次渲染**的值，
   *   而偏好是异步读回来的：首帧 `silent` 必然是 `false`，于是静默档永远不生效
   *   （2026-10-17 实测：设了静默，模态框照样弹）。静默档对"弹不弹"的答案是
   *   **派生的**，见下面的 `effectiveDismissed`。
   */
  const [dismissed, setDismissed] = useState(false)
  /* 结论到达那条回调要读"用户收起来过没有" —— 回调只装一次，不能闭包旧的 `dismissed` */
  const dismissedRef = useRef(false)
  dismissedRef.current = dismissed
  const [sinceMs, setSinceMs] = useState<number | null>(null)
  /* 静默档：结论胶囊留到哪一刻（见 `quietUntil` 的注释） */
  const [quietUntil, setQuietUntil] = useState<number | null>(null)
  /*
   * 静默档下用户**主动点开过**那一屏。
   *
   * 静默档让 `dismissed` 恒 `true`（不许自己弹），但"点胶囊要看"是一次主动动作，
   * 那时得让那一屏真出来 —— 于是记这一笔把它顶回来。
   * 新一趟开场 / 结论到达都清掉：下一次仍旧不许自己弹。
   */
  const [forcedOpen, setForcedOpen] = useState(false)

  /*
   * 这一趟**界面到底看见过没有**（2026-10-18）。
   *
   * 首帧两条路（读快照 / 订阅事件）谁先到全看运气，光靠"读回来的快照是 null"
   * 分不清两件事：① 这一趟还没来（正常：用户自己打开的程序）② 开场那一条落在
   * 缝里丢了（那时窗口已经被抬起来，屏上却没有那一屏 —— 作者真机这回看到的就是它）。
   * 于是"订阅挂上之后"再问一次后端：它手里有这一趟就整份重推一遍。
   */
  const sawRun = useRef(false)

  useEffect(() => {
    let alive = true
    const offs: (() => void)[] = []

    const snapshotDone = api
      .getPostProcessRun()
      .then((snap) => {
        if (!alive) return
        if (snap !== null) sawRun.current = true
        setRun((prev) => prev ?? snap)
        /*
         * ★★ 首帧那两条路的收尾（2026-10-17，作者真机踩到："没有出现选择的模态框，
         *   超时了就停止了"）。
         *
         * 钩子那边「推问句 → 开始等答复」之间没有间隙，所以"快照"与"订阅事件"谁先到
         * 全看运气：订阅晚一步，快照里那份问句就永远不会以事件形式到达，而 `bind` 那几条
         * 一律 `prev === null ? prev : …` 也不建场 ⇒ 界面一片空白地等，直到钩子超时
         * 按"不跑"处理（日志里只有一条 `cancelled=true`）。
         *
         * 补推一次就够：它只重新 emit，问句本来就还在后端手里、钩子还在等。
         */
        if (snap !== null && snap.question !== null && !snap.questionEmitted) {
          void api.replayPostProcessQuestion().catch(() => {
            /* 补推失败不致命：钩子那侧超时照样会把结论送来 */
          })
        }
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
    const startedReady = listen<PostProcessStarted>(STARTED_EVENT, (started: PostProcessStarted) => {
      if (!alive) return
      sawRun.current = true
      setRun({
        presetName: started.presetName,
        gcodeName: started.gcodeName,
        facts: null,
        question: null,
        progress: null,
        finished: null,
        /* 开发台演出来的那一趟带 `preview: true` —— 界面据此禁掉真业务按钮 */
        preview: started.preview,
        /* 新一趟还没有问句，自然也就没有"推过没有" */
        questionEmitted: false,
      })
      /* 新一趟该露脸 —— 但静默档不露（`effectiveDismissed` 已经处理，见下面）。
         这里仍然写 `false`：它是"用户没关过这一趟"的事实，与档位无关。 */
      setDismissed(false)
      setSinceMs(Date.now())
      /* 新一趟开始：上一趟那枚"已完成"的账清掉，静默档下也重新"收着" */
      setQuietUntil(null)
      setForcedOpen(false)
    }).then((off) => {
      if (alive) offs.push(off)
      else off()
    })

    /*
     * **收尾**：快照读完了、订阅也挂上了，两件事都落定之后再回头看一眼 ——
     * 这一趟还是没看见，就请后端把整份重推一遍（2026-10-18）。
     *
     * 只补"落在缝里"的那一次：后端手里没有这一趟就什么都不推，
     * 于是"用户自己打开程序"这种日常情形不会凭空多出一屏。
     */
    void Promise.all([snapshotDone, startedReady]).then(() => {
      if (!alive || sawRun.current) return
      void api.replayPostProcessRun().catch(() => {
        /* 补推失败不致命：后面每一条进度仍是正常事件，只是这一趟的开场没接上 */
      })
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
      /*
       * 跑完了：**要不要把那一屏叫回来**？
       *
       * 2026-10-17 改口径（作者：「点击后台处理之后就不要弹出来了吧，就像静默的一样，
       * 就这一次，在右上角显示已完成就可以…静默的他们的逻辑都一样复用的，
       * 一分钟自动消失…如果点击了出现了模态框就消失」）：
       *
       *   这一趟**用户自己收起来过**（GUI 档点了「后台运行」）⇒ 跑完也**不弹**，
       *   与静默档完全同一条路 —— 右上角那枚胶囊换成「已完成」，60 秒后自己消失。
       *
       * 原来这里 GUI 档一律 `setDismissed(false)`（"点过后台运行的人，等的是结论"）。
       * 那条在"用户压根不想看"的时候是反的 —— 他刚说了别打扰我，跑完又把窗顶到脸上。
       *
       * ★ 静默档与"手动收起"合流成**同一个判据**：`silent || dismissed`。
       *   于是 `PostProcessChip` 那一侧不用分两套逻辑，只认 `quietUntil` 有没有值。
       */
      if (!alive) return
      if (silentRef.current || dismissedRef.current) {
        setQuietUntil(Date.now() + SILENT_KEEP_MS)
        /* 跑完也不许自己弹出来（哪怕中途点开看过） */
        setForcedOpen(false)
      } else setDismissed(false)
    }).then((off) => {
      if (alive) offs.push(off)
      else off()
    })

    return () => {
      alive = false
      for (const off of offs) off()
    }
  }, [])

  /*
   * 「后台运行」那颗按钮（以及 Esc）：**把这一屏收起来**。
   *
   * ★★ 必须同时清掉 `forcedOpen`（2026-10-17）：静默档下，用户点了胶囊把那屏翻出来、
   *   看了一半再点「后台运行」——  只设 `dismissed` 是**收不回去的**
   *   （`effectiveDismissed = dismissed || (silent && !forcedOpen)`，`forcedOpen`
   *   还挂着 true，模态框照样渲染）。两个状态要一起翻，否则"收起来"这条路断一半。
   *
   * 来回切（点开 → 后台 → 再点开 → 再后台）着走没问题：两个开关各归各位，
   * 每一步都是幂等的。
   */
  const dismiss = useCallback(() => {
    setDismissed(true)
    setForcedOpen(false)
    /* 收起来之后不再"留着结论"—— 结论出来时会重新定一次 60 秒 */
    setQuietUntil(null)
  }, [])

  /*
   * 点胶囊 = 用户主动要看 ⇒ 立刻把那一屏叫回来，那枚「已完成」不再留着等满一分钟。
   *
   * ★ `forcedOpen` 只顶开"这一下"，不是永久开关：它要在
   *   · 用户再点「后台运行」（`dismiss` 里清）、
   *   · 下一趟开场（`Hello` 里清）、
   *   · 结论到达（`Finished` 里清）
   *   这三处复位。否则静默档一旦点开过，之后就再也收不回去了。
   */
  const reopen = useCallback(() => {
    setDismissed(false)
    setForcedOpen(true)
    setQuietUntil(null)
  }, [])

  /*
   * 对外那个 `dismissed` = **"这一屏现在该不该收着"**：
   *   用户关过（`dismissed`）**或** 静默档（`silent`）。
   *
   * 静默档下它恒为 `true` ⇒ 模态框恒不渲染；而 `reopen` 仍然有效 ——
   * 用户点右上角那枚胶囊是主动动作，那时把这一屏叫出来是对的。
   * 为了让 `reopen` 在静默档下也能生效，`reopen` 里额外记一笔 `forcedOpen`。
   */
  const effectiveDismissed = dismissed || (silent && !forcedOpen)

  return {
    run,
    dismissed: effectiveDismissed,
    sinceMs,
    dismiss,
    reopen,
    silent,
    quietUntil,
  }
}
