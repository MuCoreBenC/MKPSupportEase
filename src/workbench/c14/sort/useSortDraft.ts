/*
 * 排序草稿层。
 *
 * 排序框里的一切动作都只改这一份本地次序：拖动、输入位次、撤销、重做、恢复。
 * 确认之前一个字节都不写盘 —— 「点了取消就当没动过」是这一层的全部意义。
 *
 * 位次输入与拖拽必须落到同一套数学（都走 move()），否则两条路会算出两种次序。
 * 次序只记 key，行内容与落盘命令都由调用方给，这一层不认任何后端接口。
 */

import { useCallback, useRef, useState } from 'react'

export interface SortDraft {
  /** 当前次序（key） */
  order: string[]
  /** 把第 `from` 行挪到第 `to` 行（两者都是落地后的下标） */
  move: (from: number, to: number) => void
  /** 直接把某一条放到第 n 位（1 起）。超出范围按两端夹住 */
  setPosition: (key: string, n: number) => void
  undo: () => void
  redo: () => void
  /** 恢复 = 回到打开时那一刻的次序 */
  reset: () => void
  /** 与打开时不同就算脏（撤销回原样也会变回不脏） */
  dirty: boolean
  canUndo: boolean
  canRedo: boolean
}

interface State {
  order: string[]
  past: string[][]
  future: string[][]
}

/** 历史封顶：一屏几十行，撤到底也就几十步 */
const HISTORY_MAX = 100

const same = (a: string[], b: string[]): boolean =>
  a.length === b.length && a.every((k, i) => k === b[i])

export function useSortDraft(initial: string[]): SortDraft {
  /* 打开那一刻的次序 = 「恢复」要回到的地方。只认第一次，之后不再跟着 props 变 */
  const initialRef = useRef<string[]>([...initial])
  const [state, setState] = useState<State>(() => ({
    order: [...initial],
    past: [],
    future: [],
  }))

  const apply = useCallback((next: string[]) => {
    setState((prev) => {
      /* 与现值一样就不占一步历史（拖回原位、输入同一个数字） */
      if (same(prev.order, next)) return prev
      return {
        order: next,
        past: [...prev.past, prev.order].slice(-HISTORY_MAX),
        future: [],
      }
    })
  }, [])

  const move = useCallback(
    (from: number, to: number) => {
      setState((prev) => {
        if (from === to || from < 0 || to < 0) return prev
        if (from >= prev.order.length || to >= prev.order.length) return prev
        const next = [...prev.order]
        const [moved] = next.splice(from, 1)
        if (moved === undefined) return prev
        next.splice(to, 0, moved)
        if (same(prev.order, next)) return prev
        return { order: next, past: [...prev.past, prev.order].slice(-HISTORY_MAX), future: [] }
      })
    },
    [],
  )

  const setPosition = useCallback(
    (key: string, n: number) => {
      setState((prev) => {
        const from = prev.order.indexOf(key)
        if (from < 0) return prev
        const to = Math.min(Math.max(Math.round(n), 1), prev.order.length) - 1
        if (to === from) return prev
        const next = [...prev.order]
        const [moved] = next.splice(from, 1)
        if (moved === undefined) return prev
        next.splice(to, 0, moved)
        return { order: next, past: [...prev.past, prev.order].slice(-HISTORY_MAX), future: [] }
      })
    },
    [],
  )

  const undo = useCallback(() => {
    setState((prev) => {
      const back = prev.past[prev.past.length - 1]
      if (back === undefined) return prev
      return {
        order: back,
        past: prev.past.slice(0, -1),
        future: [prev.order, ...prev.future].slice(0, HISTORY_MAX),
      }
    })
  }, [])

  const redo = useCallback(() => {
    setState((prev) => {
      const [next, ...rest] = prev.future
      if (next === undefined) return prev
      return {
        order: next,
        past: [...prev.past, prev.order].slice(-HISTORY_MAX),
        future: rest,
      }
    })
  }, [])

  const reset = useCallback(() => {
    apply([...initialRef.current])
  }, [apply])

  return {
    order: state.order,
    move,
    setPosition,
    undo,
    redo,
    reset,
    dirty: !same(state.order, initialRef.current),
    canUndo: state.past.length > 0,
    canRedo: state.future.length > 0,
  }
}
