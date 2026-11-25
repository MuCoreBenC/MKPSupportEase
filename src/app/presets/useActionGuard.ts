/*
 * 动作在途守卫：同一动作、同一目标，任一时刻只跑一次。
 *
 * 行内按钮、右键菜单、详情卡是同一个动作的多个入口，它们共用一把锁：
 * `busy` 给界面读禁用态，`inFlight` 是同步账，挡住 React 下一次渲染之前的连点。
 * 键是 `动作:目标`，所以同一行上的「下载」与「使用」互不阻塞。
 */

import { useCallback, useRef, useState } from 'react'

/** 守卫键：动作限定的目标标识 */
export function actionKey(action: string, target: string): string {
  return `${action}:${target}`
}

/**
 * 同步在途账。`enter` 立即占位，不等渲染；`leave` 注销。
 * 抽成独立类是为了让判据能在单测里直接跑，不必起组件。
 */
export class GuardLedger {
  private readonly entries = new Set<string>()

  enter(key: string): boolean {
    if (this.entries.has(key)) return false
    this.entries.add(key)
    return true
  }

  leave(key: string): void {
    this.entries.delete(key)
  }

  has(key: string): boolean {
    return this.entries.has(key)
  }
}

export interface ActionGuard {
  /** 正在执行的 `${action}:${target}`，按钮与菜单项的禁用态读它 */
  readonly busy: ReadonlySet<string>
  isBusy: (action: string, target: string) => boolean
  /** 同步占位；同一键重复进入返回 false，调用方据此静默丢掉这一下 */
  enter: (action: string, target: string) => boolean
  /** 释放；与 enter 成对，失败路径也要走到 */
  leave: (action: string, target: string) => void
}

export function useActionGuard(): ActionGuard {
  const ledger = useRef(new GuardLedger())
  const [busy, setBusy] = useState<ReadonlySet<string>>(() => new Set<string>())

  const enter = useCallback((action: string, target: string): boolean => {
    const key = actionKey(action, target)
    if (!ledger.current.enter(key)) return false
    setBusy((cur) => {
      if (cur.has(key)) return cur
      const next = new Set(cur)
      next.add(key)
      return next
    })
    return true
  }, [])

  const leave = useCallback((action: string, target: string): void => {
    const key = actionKey(action, target)
    ledger.current.leave(key)
    setBusy((cur) => {
      if (!cur.has(key)) return cur
      const next = new Set(cur)
      next.delete(key)
      return next
    })
  }, [])

  const isBusy = useCallback(
    (action: string, target: string) => busy.has(actionKey(action, target)),
    [busy],
  )

  return { busy, isBusy, enter, leave }
}
