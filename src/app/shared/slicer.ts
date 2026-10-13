/*
 * 切片软件的选择（2026-10-13 作者口径）。
 *
 * # 默认用哪个（设置页那一格）
 *
 * 纯前端偏好 —— 落 localStorage（`STORAGE.clientSlicer`），不走 AppState：
 * 它不进后端账本，壳（Tauri）那边将来接 `openModel` 时**由前端把这个值随调用带给它**。
 *
 * # 打开模型的模态框
 *
 * 「打开模型」确认框里摆一个「用哪个打开」下拉 + 「仅这一次用选定的」勾选：
 * - 不勾：确认时**把选择写回默认**（下次设置页里看到的就是它）；
 * - 勾了：只用这一次，默认不动。
 * 状态住在页面里（两个页面各一份，弹窗开着才有意义），`useSlicerPick` 负责弹窗
 * 每次打开时把选择重置回当前默认 —— 上一次的「仅这一次」不该渗到下一次。
 */

import { useCallback, useEffect, useState } from 'react'
import { STORAGE } from '../../api/storageKeys'

/** 三档，**id 是给壳的契约**（跟随系统 = 交给系统关联程序，不指定具体应用） */
export type SlicerId = 'system' | 'bambu' | 'orca'

export const SLICER_CHOICES: { id: SlicerId; label: string; hint: string }[] = [
  { id: 'system', label: '跟随系统默认', hint: '用 .3mf 关联的程序打开（没选过时就是这个）' },
  { id: 'bambu', label: 'Bambu Studio', hint: '总是交给 Bambu Studio' },
  { id: 'orca', label: 'OrcaSlicer', hint: '总是交给 OrcaSlicer' },
]

/** 默认那一份（设置页写，打开模型读）。值不合法时回落到跟随系统 */
export function useSlicerPref() {
  return useSyncedStickyState<SlicerId>(STORAGE.clientSlicer, 'system', isSlicerId)
}

function isSlicerId(v: unknown): v is SlicerId {
  return v === 'system' || v === 'bambu' || v === 'orca'
}

/**
 * 与 `useStickyState` 同口径，但**同一文档里的多份实例互相同步** ——
 * 外壳的页签是常驻挂载的，设置页那份 state 在模态框里「不勾仅这一次」写回默认时
 * 不会重新挂载，纯 useStickyState 会显示旧值。写的时候顺手广播一个自定义事件，
 * storage 事件兜底跨标签页的场景。
 */
function useSyncedStickyState<T extends string>(
  key: string,
  initial: T,
  is: (v: unknown) => v is T,
): [T, (v: T) => void] {
  const EVT = `mkp:synced-sticky:${key}`
  const [value, setValue] = useState<T>(() => {
    try {
      const raw = localStorage.getItem(key)
      if (raw === null) return initial
      const v = JSON.parse(raw) as unknown
      return is(v) ? v : initial
    } catch {
      return initial
    }
  })
  useEffect(() => {
    const on = (e: Event) => {
      const detail = (e as CustomEvent<T>).detail
      if (is(detail)) setValue(detail)
    }
    const onStorage = (e: StorageEvent) => {
      if (e.key !== key) return
      try {
        const v = e.newValue === null ? initial : (JSON.parse(e.newValue) as unknown)
        setValue(is(v) ? v : initial)
      } catch {
        setValue(initial)
      }
    }
    window.addEventListener(EVT, on)
    window.addEventListener('storage', onStorage)
    return () => {
      window.removeEventListener(EVT, on)
      window.removeEventListener('storage', onStorage)
    }
    /* `is` 是模块级纯函数，不进依赖 —— 关掉这条警示 */
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [EVT, key, initial])
  const set = useCallback(
    (v: T) => {
      try {
        localStorage.setItem(key, JSON.stringify(v))
      } catch {
        /* 无痕模式：存不了就算了 */
      }
      window.dispatchEvent(new CustomEvent(EVT, { detail: v }))
      setValue(v)
    },
    [EVT, key],
  )
  return [value, set]
}

/** 模态框里的瞬时选择：每次打开重置为当前默认、「仅这一次」清掉 */
export function useSlicerPick(defaultSlicer: SlicerId, open: boolean) {
  const [value, setValue] = useState<SlicerId>(defaultSlicer)
  const [once, setOnce] = useState(false)
  useEffect(() => {
    if (open) {
      setValue(defaultSlicer)
      setOnce(false)
    }
  }, [open, defaultSlicer])
  return { value, setValue, once, setOnce }
}

/** 确认打开时的两件事：不勾「仅这一次」就把选择写成新默认；返回真正要用的那一份 */
export function settleSlicerPick(
  pick: { value: SlicerId; once: boolean },
  setDefault: (id: SlicerId) => void,
): SlicerId {
  if (!pick.once) setDefault(pick.value)
  return pick.value
}
