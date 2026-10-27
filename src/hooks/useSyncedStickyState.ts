/*
 * 与 `useStickyState` 同口径（localStorage + JSON），但**同一文档里的多份实例互相同步** ——
 * 外壳的页签是常驻挂载的：在一处写、另一处那份 state 不会重新挂载，纯 `useStickyState`
 * 会显示旧值（"在参数页关掉了提示、切到设置页却发现还开着"）。写的时候顺手广播一个
 * 自定义事件，`storage` 事件兜底跨标签页的场景。
 *
 * 从一个实例里搬出来的共用件（2026-10-17）—— 原来只长在 `app/shared/slicer.ts` 里，
 * 现在切片偏好与「引导提示」开关都走这一份（同名同事件，两处不再各写一套）。
 */

import { useCallback, useEffect, useState } from 'react'

export function useSyncedStickyState<T>(
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
