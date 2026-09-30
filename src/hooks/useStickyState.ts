import { useEffect, useState } from 'react'
import type { Dispatch, SetStateAction } from 'react'

/** localStorage 持久化的 state；存储不可用时静默退回内存 state */
export function useStickyState<T>(
  key: string,
  initial: T
): [T, Dispatch<SetStateAction<T>>] {
  const [value, setValue] = useState<T>(() => {
    try {
      const raw = localStorage.getItem(key)
      return raw === null ? initial : (JSON.parse(raw) as T)
    } catch {
      return initial
    }
  })

  useEffect(() => {
    try {
      localStorage.setItem(key, JSON.stringify(value))
    } catch {
      /* 无痕模式 / 配额满：忽略，仅内存生效 */
    }
  }, [key, value])

  return [value, setValue]
}
