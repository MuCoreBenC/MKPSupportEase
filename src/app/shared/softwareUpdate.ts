/*
 * **一份更新快照，所有 UI 都读它**（2026-11-22 统一状态那一刀）。
 *
 * # 收掉了什么
 *
 * 从前这里有"两份事实"：设置页自己问 `checkSoftwareUpdate` 存一份、标题栏自己问
 * `updateInfo` 再存一份，两边都不会因为对方的变化而刷新 —— 于是同一屏能出现
 * 「设置页说已是最新、标题栏挂着发现 0.0.13」。
 *
 * 现在只有一份：后端 `run/update-state.json` 的快照，经 `software-update-state`
 * 事件与 `getUpdateSnapshot()` 到达这里，组件全部 `useSoftwareUpdate()`。
 *
 * # 三件事仍分开放
 *
 * - 业务事实（候选 / 检查记录 / 忽略 / 上次安装结果）：后端快照；
 * - 下载进度（downloading / paused / ready）：`software-update-progress` 事件，
 *   只活在本次进程；
 * - 旧 localStorage 的忽略记录：一次性迁移握手（[`migrateIgnoredOnce`]），
 *   **只有后端确认后才删键**。
 *
 * # 为什么不在挂载时自动问
 *
 * 铁律 2：云端不参与首屏。谁需要谁调 `refreshSoftwareUpdate()`（标题栏那 1.2s 的延迟
 * 照旧，设置页是用户点进去才问）。
 */

import { useEffect, useSyncExternalStore } from 'react'

import { api, listen } from '../../api'
import { STORAGE } from '../../api/storageKeys'
import type { UpdateInfo, UpdateState } from '../../api/contract'

let snapshot: UpdateInfo | null = null
const listeners = new Set<() => void>()

function publish(next: UpdateInfo): void {
  snapshot = next
  for (const listener of listeners) listener()
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

/* ---------------------------------------------------------------------------
 * 订阅：进度（进程态）+ 快照（业务事实）+ 一次性迁移
 * ------------------------------------------------------------------------- */

let watching = false

/** 只在状态真的变了时才换 install 那一格（免得每次进度事件都造新对象）。 */
function withState(base: UpdateInfo, next: UpdateState): UpdateInfo {
  const install =
    next.state === 'ready'
      ? ({ phase: 'ready', version: next.version } as const)
      : ({ phase: 'idle' } as const)
  return { ...base, state: next, install }
}

function watch(): void {
  if (watching) return
  watching = true
  void listen<UpdateState>('software-update-progress', (next) => {
    if (snapshot === null) {
      void refreshSoftwareUpdate()
      return
    }
    publish(withState(snapshot, next))
  })
  /* 唯一的业务事实事件（`runtime::update_check::run_check` 发）。
     事件只报"变了"，payload 是业务快照（不带进程态）；这里重新读一份 DTO 再发出去，
     免得两份形状在界面上打架。 */
  void listen('software-update-state', () => {
    void fetchSnapshot().then((next) => {
      if (next === null) return
      publish(next)
      announcePending(next)
    })
  })
  void migrateIgnoredOnce()
}

/* ---------------------------------------------------------------------------
 * 新待更新 → 通知
 *
 * 通知不再是"后端另发一个事件"：候选第一次出现在快照里、且不是本次启动时的旧账，
 * 就通知一次。这样「有没有新版」只有一个判据，不会出现"事件说发现了、快照说没有"。
 * ------------------------------------------------------------------------- */

const pendingWatchers = new Set<(version: string) => void>()
let lastPending: string | null = null

/** 订阅"新出现的待更新"；返回取消函数。**首次快照不触发**（那是上次留下的账）。 */
export function onNewPending(cb: (version: string) => void): () => void {
  pendingWatchers.add(cb)
  return () => {
    pendingWatchers.delete(cb)
  }
}

/** 事件路径专用：**刚查到**的新待更新才弹（与"启动时读到的旧账"区分开）。 */
function announcePending(next: UpdateInfo): void {
  const pending = next.pendingVersion ?? null
  if (pending !== null && pending !== lastPending) {
    for (const watcher of pendingWatchers) watcher(pending)
  }
  lastPending = pending
}

/* ---------------------------------------------------------------------------
 * 读 / 检查 / 忽略 / 迁移
 * ------------------------------------------------------------------------- */

/**
 * 问一次后端快照。失败**不抛**（离线 / 没配源都算正常）——
 * 上一次的快照保持不动，免得"问不到"把界面擦成"没有新版"。
 */
async function fetchSnapshot(): Promise<UpdateInfo | null> {
  try {
    return await api.getUpdateSnapshot()
  } catch {
    return null
  }
}

export async function refreshSoftwareUpdate(): Promise<UpdateInfo | null> {
  const next = await fetchSnapshot()
  if (next !== null) {
    publish(next)
    /* 读到的账不算"刚发现"：把水位对齐，别让启动时那一条旧账弹通知 */
    lastPending = next.pendingVersion ?? null
  }
  return next
}

/** 手动检查（设置页那颗按钮）：后端事务写盘，这里把新快照发出去。失败照旧抛。 */
export async function checkSoftwareUpdateNow(): Promise<UpdateInfo> {
  const next = await api.checkSoftwareUpdate()
  publish(next)
  lastPending = next.pendingVersion ?? null
  return next
}

/** 忽略某一版：后端落盘，前端换快照。失败只记日志 —— 下次快照会纠正。 */
export async function ignoreUpdateVersion(version: string): Promise<void> {
  try {
    publish(await api.ignoreUpdate(version))
  } catch (e) {
    console.warn('忽略这一版没落到后端：', e)
  }
}

/**
 * 旧 `localStorage` 忽略记录的一次性迁移。
 *
 * 顺序是死的：读旧键 → 交给后端 → **后端确认成功** → 才删旧键。
 * 中途断电最坏是"旧键还在、后端也已有值"，下次重跑幂等，不会丢。
 */
let migrating = false

async function migrateIgnoredOnce(): Promise<void> {
  if (migrating) return
  const old = readIgnoredUpdateVersion()
  if (old === null) return
  migrating = true
  try {
    const next = await api.migrateUpdateIgnored(old)
    publish(next)
    localStorage.removeItem(STORAGE.clientIgnoredUpdateVersion)
  } catch {
    /* 后端没确认就删键 = 忽略记录丢了。留着，下次挂载再试。 */
  } finally {
    migrating = false
  }
}

/* ---------------------------------------------------------------------------
 * 读这一份共享快照
 * ------------------------------------------------------------------------- */

export interface SoftwareUpdateView {
  /** 整份快照（`null` = 还没问过 / 问不到） */
  snapshot: UpdateInfo | null
  /** 下载进度（快照里那一格，方便组件少解一层） */
  state: UpdateState
  /** 稳定引用（模块级函数）—— 放进 effect 依赖不会反复触发 */
  refresh: () => Promise<UpdateInfo | null>
}

const IDLE: UpdateState = { state: 'idle' }

/** 读这一份共享快照（任何组件都可以读，读到的是同一份） */
export function useSoftwareUpdate(): SoftwareUpdateView {
  useEffect(() => {
    watch()
  }, [])
  const snap = useSyncExternalStore(subscribe, () => snapshot)
  return { snapshot: snap, state: snap?.state ?? IDLE, refresh: refreshSoftwareUpdate }
}

/* ---------------------------------------------------------------------------
 * 标题栏胶囊的唯一判据（用户故事见 docs/SOFTWARE-UPDATE-SPEC.md）
 *
 * `hasUpdate` **不在这里**：它回答"有没有新版"（候选非空），
 * 而胶囊回答"要不要现在冒出来"（有候选、且没被忽略）。两件事。
 * ------------------------------------------------------------------------- */
export type UpdateChipIntent = 'progress' | 'pending' | 'none'

export function updateChipIntent(
  state: UpdateState,
  pending: string | null | undefined,
  ignored: string | null | undefined,
): UpdateChipIntent {
  if (
    state.state === 'downloading' ||
    state.state === 'paused' ||
    state.state === 'ready' ||
    state.state === 'failed'
  ) {
    return 'progress'
  }
  if (pending === null || pending === undefined || pending.trim() === '') return 'none'
  if (ignored === pending) return 'none'
  return 'pending'
}

/** 旧键里用户忽略的那一版（读不到 = 没忽略过；迁移完就删） */
export function readIgnoredUpdateVersion(): string | null {
  try {
    const raw = localStorage.getItem(STORAGE.clientIgnoredUpdateVersion)
    return raw === null ? null : (JSON.parse(raw) as string | null)
  } catch {
    return null
  }
}
