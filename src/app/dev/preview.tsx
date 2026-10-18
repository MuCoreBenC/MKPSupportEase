/*
 * **开发台**：客户端这一侧的接收口（作者 2026-10-14）。
 *
 * # 它做什么
 *
 * 工作台「开发台」点一个场景 → 发一条指令 → 客户端窗口显示**对应那一格真实界面**。
 * 这个 Provider 收下那条指令（Tauri 事件 `dev-preview`），把 `scene` 摆进 context；
 * **持有那一格界面的组件**用 [`useDevPreview`] 认领自己的 `scene`，认领到就把那一格打开。
 *
 * ```text
 * 工作台「开发台」──invoke──► wb_dev_trigger ──loopback──► 客户端进程
 *                                                              │ hook_ui::run_preview
 *                                       （后处理那一组：演出那一屏）
 *                                       （其余网格：emit `dev-preview`）
 *                                                              ▼
 *                                                    本 Provider（按层类归类）
 *                                                              │
 *                                     useDevPreview('rename') 认领 → 打开「重命名」那一格
 * ```
 *
 * # 四条纪律
 *
 * 1. **不发真业务、不落记录**：认领只是"把那一格打开"，里面的动作照旧是用户自己点才发生。
 * 2. **关闭就是关闭**：`close(slug)` 把那一格清掉；工作台再点就是新的一条指令
 *    （`nOnce` 换一个数 —— 同一格能反复打开，不会被"已经开着"挡住）。
 * 3. **同类互斥、异类并存**（作者 2026-10-14）：按 [`DevSlot`] 归类，**只顶掉自己那一类**里
 *    的上一格 —— 于是"更新面板 + 参数对比台 + 提示条"能同时摆着看，而连点两次「重命名」
 *    是第二个顶掉第一个（真机上这些框本来也是一次一个）。
 * 4. **不在产品逻辑里留钩子**：这一层只在**开发构建**里才收到事件（正式包里没有
 *    `wb_dev_trigger`，也没有谁会 `emit('dev-preview')`）—— 事件收不到时它就是恒 `false`
 *    的 context，对真机零影响。
 *
 * 生产代码里只有**极薄**的一层接线：持有者加一行 `useDevPreview('<slug>')`。
 * 正常业务逻辑一行不改。
 */

import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import type { ReactNode } from 'react'

import { listen } from '../../api'
import { DEV_PREVIEW_EVENT, slotOf } from './scene'
import type { DevSlot, Occupied, Slot } from './scene'
import { DevPreviewContext } from './context'
import { DEV_FAULT_EVENT, setArmedFault } from './fault'
import type { DevFaultDto } from './fault'

export function DevPreviewProvider({ children }: { children: ReactNode }) {
  /* 每一层类摆着哪一格 —— **同类互斥**就落在这儿（后到的覆盖自己那一类） */
  const [bySlot, setBySlot] = useState<ReadonlyMap<DevSlot, Occupied>>(new Map())
  /* 次数：同一格再点一次也要能重开，所以每次指令 +1（不是每类 +1） */
  const nonceRef = useRef(0)

  useEffect(() => {
    let alive = true
    let off: (() => void) | null = null
    void listen<{ scene: string }>(DEV_PREVIEW_EVENT, (payload) => {
      if (!alive) return
      const s = slotOf(payload.scene)
      nonceRef.current += 1
      const occupied: Occupied = { scene: payload.scene, nOnce: nonceRef.current }
      setBySlot((prev) => {
        const next = new Map(prev)
        /* ★ 只顶掉**自己那一类**里的上一格；别的类原样留着（异类并存） */
        next.set(s, occupied)
        return next
      })
    }).then((un) => {
      if (alive) off = un
      else un()
    })
    return () => {
      alive = false
      off?.()
    }
  }, [])

  /*
   * ★ 2026-10-19：开发台现在发**两类**指令，这一层都收：
   *   ① `dev-preview`  —— 打开一格界面（上面那个 effect）
   *   ② `dev-fault`    —— 给某个 API 方法装一条错（下面这个 effect）
   *
   * 两者是相反的两件事：一个动界面、一个动后端。装了故障之后**所有**走那个方法的
   * 界面都会真的报错（真组件 / 真按钮 / 真错误码）—— 见 `fault.ts` 抬头。
   */
  useEffect(() => {
    let alive = true
    let off: (() => void) | null = null
    void listen<DevFaultDto>(DEV_FAULT_EVENT, (payload) => {
      if (!alive) return
      setArmedFault(
        payload.method === null || payload.error === null
          ? null
          : { method: payload.method, error: payload.error },
      )
    }).then((un) => {
      if (alive) off = un
      else un()
    })
    return () => {
      alive = false
      off?.()
    }
  }, [])

  /** 关掉**一格**：只清它所在的那一类（别类的还摆着） */
  const close = useCallback((scene: string) => {
    const s = slotOf(scene)
    setBySlot((prev) => {
      /* 只关"还是它"的那一类 —— 若这一格已经被同类的新格顶掉了，就别误清 */
      if (prev.get(s)?.scene !== scene) return prev
      const next = new Map(prev)
      next.delete(s)
      return next
    })
  }, [])

  const value = useMemo<Slot>(() => ({ bySlot, close }), [bySlot, close])

  return <DevPreviewContext.Provider value={value}>{children}</DevPreviewContext.Provider>
}
