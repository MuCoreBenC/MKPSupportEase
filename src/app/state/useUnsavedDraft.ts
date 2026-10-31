/*
 * **客户端唯一的「有没有未保存改动」来源**（2026-10-31）。
 *
 * # 为什么要有它
 *
 * 之前"未保存"是各页面各自算的：预设列表行上有个标记、参数页底栏有一句、
 * 关窗时谁也不拦。同一次编辑在不同地方给出不同答案，用户没法判断"我现在关掉
 * 会不会丢东西"。
 *
 * 现在统一问后端那份草稿摘要（`getPresetDraftSummary`）—— **它才是草稿的真相**，
 * 前端只展示、不重算。`draftState` 的代次（`useDraftRevision`）一到就重读：
 *
 *   · 参数页改值 / 保存 / 放弃 → 发代次；
 *   · 预设页保存正文 / 放弃正文 → 发代次。
 *
 * # 它管什么、不管什么（作者 2026-10-31 的边界）
 *
 * - **只管参数草稿**（改参数、改对比台参数）。保存 = 写盘，放弃 = 不动磁盘。
 * - **不管文件管理操作**：改名、删除、复制、备注、归属是**立即执行**的，
 *   它们不该让界面挂上"未保存"标，也不该在关窗时拦人。
 */
import { useEffect, useState } from 'react'

import { api } from '../../api'
import { useDraftRevision } from './draftState'

export interface UnsavedDraft {
  /** 有未保存的参数改动 */
  dirty: boolean
  /** 改的是哪一份（文件名；没有草稿时为 null） */
  fileName: string | null
  /** 用户线落点（云端线为 null） */
  path: string | null
}

const EMPTY: UnsavedDraft = { dirty: false, fileName: null, path: null }

/**
 * 读一次草稿摘要并跟着代次刷新。
 *
 * 失败按"没有草稿"处理：这一层是提示，不该因为读不到摘要就把界面卡住
 * （真有草稿时参数页那边的保存仍然可用）。
 */
export function useUnsavedDraft(): UnsavedDraft {
  const revision = useDraftRevision()
  const [state, setState] = useState<UnsavedDraft>(EMPTY)

  useEffect(() => {
    let alive = true
    void api.getPresetDraftSummary().then(
      (s: import('../../api').PresetDraftSummary | null) => {
        if (!alive) return
        /* `null` = 压根没有草稿；`dirty: false` = 打开过但一个字没改 —— 两种都不算未保存 */
        setState(
          s !== null && s.dirty
            ? { dirty: true, fileName: s.sourceFileName ?? null, path: s.path ?? null }
            : EMPTY,
        )
      },
      () => {
        if (alive) setState(EMPTY)
      },
    )
    return () => {
      alive = false
    }
  }, [revision])

  return state
}
