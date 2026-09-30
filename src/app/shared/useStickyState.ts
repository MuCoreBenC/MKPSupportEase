/*
 * 切走再切回来不丢选择。
 *
 * # 为什么需要它
 *
 * 应用外壳按 tab 用 switch 渲染页面 —— 切到别的 tab，这一页就**卸载**了，
 * useState 里的东西一并没了。于是「展开了 A1 mini、筛成只看可选、看到一半去参数页对一眼」
 * 回来之后全部复位，列表还滚回顶上。三层分组的列表尤其明显：每次回来都要重新展开一遍。
 *
 * 真正的解法是把外壳改成常驻挂载（`hidden` 而不是卸载），但那要动外壳的渲染方式，
 * 先不动。这里用最小的一块：**把值记在模块作用域**，页面重新挂载时读回来。
 *
 * # 边界
 *
 * - 只记「这次会话里我看到哪」这类界面状态（展开 / 范围 / 搜索词 / 当前机型）。
 *   **不要**用它记草稿或未保存的改动 —— 那种东西必须显式提示，不能靠一个静默的缓存活着。
 * - 刷新页面就没了（不落 localStorage）：它只是在补「切 tab 不该失忆」，不是持久化。
 * - setter 只收**算好的下一个值**，不收 updater 函数 —— 少一种写法，就少一处
 *   「缓存里的值和 state 不同步」的可能。
 */

import { useCallback, useState } from 'react'

const memory = new Map<string, unknown>()

export function useStickyState<T>(key: string, initial: T): [T, (next: T) => void] {
  const [value, setValue] = useState<T>(() => (memory.has(key) ? (memory.get(key) as T) : initial))

  const set = useCallback(
    (next: T) => {
      memory.set(key, next)
      setValue(next)
    },
    [key],
  )

  return [value, set]
}
