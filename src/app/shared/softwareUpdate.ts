/*
 * **一份更新会话，两处入口**（2026-11-06 作者：标题栏那颗指示器与设置页那两张卡"可以合并"）。
 *
 * # 合并的是什么，不是什么
 *
 * 合的是**数据**：以前两边各干各的 —— 设置页自己 `listen`、自己 `updateInfo`；
 * 标题栏也只在自己挂载后 1.2s 问一次 `updateInfo`。于是会出现
 * 「设置页说有新版、标题栏没图标」，反过来也一样（那个 1.2s 快照之后就再也不更新了）。
 *
 * 不合的是**视图**：标题栏要的是"进度一眼可见、任何页面都看得见"，设置页要的是
 * "哪条通道、哪一版、要不要装" —— 两件事，各留各的壳。这里只保证**两边看同一份事实**。
 *
 * # 手法
 *
 * 与 `deliveryState` 同一套（模块级单例 + `useSyncExternalStore`）：一个快照 + 一组订阅。
 * **不是新的一层后端**：每个动作仍是组件里的一次 `api.*`，进度仍由事件推回来，
 * 这里只是不让第二个组件再订一遍、再问一遍。
 *
 * # 为什么不在挂载时自动问
 *
 * 铁律 2：云端不参与首屏。谁需要谁调 `refreshSoftwareUpdate()`（标题栏那 1.2s 的延迟
 * 照旧，设置页是用户点进去才问）。
 */

import { useEffect, useSyncExternalStore } from 'react'

import { api, listen } from '../../api'
import type { UpdateInfo, UpdateState } from '../../api/contract'

interface Snapshot {
  /** 当前这一态（`software-update-progress` 推来的，或 `updateInfo` 快照里带的） */
  state: UpdateState
  /** 最近一次问回来的那份（有新版吗 / 那一版 / 上次安装的账）；`null` = 还没问 */
  info: UpdateInfo | null
}

const IDLE: UpdateState = { state: 'idle' }

let snapshot: Snapshot = { state: IDLE, info: null }
const listeners = new Set<() => void>()

function publish(patch: Partial<Snapshot>): void {
  snapshot = { ...snapshot, ...patch }
  for (const listener of listeners) listener()
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

/* 进度事件**全站只订这一次** */
let watching = false

function watch(): void {
  if (watching) return
  watching = true
  void listen<UpdateState>('software-update-progress', (next) => {
    publish({ state: next })
  })
}

/**
 * 问一次后端：现在算什么状态、有没有可下的一版、上次装成没成。
 *
 * 失败**不抛**（离线 / 没配源都算正常）—— 调用方按 `null` 处理即可；
 * 上一次的快照保持不动，免得"问不到"把界面擦成"没有新版"。
 */
export async function refreshSoftwareUpdate(): Promise<UpdateInfo | null> {
  try {
    const info = await api.updateInfo()
    publish({ info, state: info.state })
    return info
  } catch {
    return null
  }
}

export interface SoftwareUpdateView {
  state: UpdateState
  info: UpdateInfo | null
  /** 稳定引用（模块级函数）—— 放进 effect 依赖不会反复触发 */
  refresh: () => Promise<UpdateInfo | null>
}

/** 读这一份共享快照（任何组件都可以读，读到的是同一份） */
export function useSoftwareUpdate(): SoftwareUpdateView {
  useEffect(() => {
    watch()
  }, [])
  const snap = useSyncExternalStore(subscribe, () => snapshot)
  return { state: snap.state, info: snap.info, refresh: refreshSoftwareUpdate }
}
