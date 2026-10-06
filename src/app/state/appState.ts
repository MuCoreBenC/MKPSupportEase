/*
 * AppState 客户端（`docs/APP-STATE.md` §7）：应用持久化状态在前端的**唯一读取/订阅口**。
 *
 * 后端把使用中指针 / 草稿 / 数据源统一存进 `run/app-state.json`（`runtime/app_state.rs`），
 * 任何写命令成功都会 emit `app-state-changed`。这里只做三件事：
 *
 *   ① 全应用只有这里调 `api.getActivePreset()`（唯一读取入口）；
 *   ② 接住后端的变更事件重读整份状态（"改了就推"，页面不点任何东西也跟着变）；
 *   ③ 用 `useSyncExternalStore` 把快照发给订阅的页面 —— 页面**不持副本、不回页签对账**。
 *
 * 写这一侧没有"入口"可言：状态变更都是一次业务动作（应用 / 撤销 / 改名 / 删除…），
 * 各自的命令成功后调 `appStateMutated()` 兜底刷新（浏览器 mock 没有事件，靠它；
 * 桌面端事件与它都只会触发一次重读，`inflight` 去重）。换来的 invariant：
 * 任何一条改状态的路径走完，所有订阅页面看到的是同一份快照。
 *
 * 本轮接入的格是 **activePreset**（唯一被多个页面共享的一格）；draft / presetSource
 * 等被多页共享时按同一模式扩到这里，不为单页的状态另开读取口。
 */

import { useSyncExternalStore } from 'react'

import { api, listen } from '../../api'
import type { ActivePreset } from '../../api/contract'
import { inTauri } from '../window'

/** 与 Rust 侧 `ipc::APP_STATE_EVENT` 同名（两边没有编译器，靠这里对齐） */
const APP_STATE_EVENT = 'app-state-changed'

let snapshot: ActivePreset | null = null
let loaded = false
let inflight: Promise<void> | null = null
const listeners = new Set<() => void>()

function notify(): void {
  for (const listener of listeners) listener()
}

/** 重读 AppState（唯一的那次 IPC 读），落地后广播给订阅者 */
export function appStateMutated(): void {
  if (inflight !== null) return
  inflight = api
    .getActivePreset()
    .then((entry) => {
      snapshot = entry
    })
    .catch(() => {
      snapshot = null
    })
    .finally(() => {
      loaded = true
      inflight = null
      notify()
    })
}

let eventBound = false

/** 订阅快照。首个订阅者出现时补一次初始读取，并把后端事件接上（一次性，应用生命周期） */
function subscribe(listener: () => void): () => void {
  listeners.add(listener)
  if (!loaded && inflight === null) appStateMutated()
  if (inTauri && !eventBound) {
    eventBound = true
    void listen(APP_STATE_EVENT, () => appStateMutated())
  }
  return () => {
    listeners.delete(listener)
  }
}

function getSnapshot(): ActivePreset | null {
  return snapshot
}

/**
 * 正在使用的那一份（AppState 的 activePreset 格）。null = 还没用任何一份。
 *
 * 页面只读这个 hook —— 不保存副本、不在"回到本页"时重新读、不对账：
 * 底账变了（应用 / 撤销 / 改名 / 删除，包括删除使用中那份的连带撤指针）事件会推过来，
 * 所有订阅页面同帧换账。
 */
export function useActivePreset(): ActivePreset | null {
  return useSyncExternalStore(subscribe, getSnapshot)
}

/**
 * 首读落地了吗。挂载初期快照还在路上（null 只代表"还没读到"，
 * 不代表"没有已应用"）——要区分这两种 null 的页面用它把关。
 */
export function useActivePresetReady(): boolean {
  return useSyncExternalStore(subscribe, getReady)
}

function getReady(): boolean {
  return loaded
}

/** 一次性拿当前快照（首读落地即返回）。挂载时"默认落在正在使用那一份"的页面用 */
export function activePresetSnapshot(): Promise<ActivePreset | null> {
  if (loaded) return Promise.resolve(snapshot)
  if (inflight === null) appStateMutated()
  return (inflight ?? Promise.resolve()).then(() => snapshot)
}

/**
 * 切换到某台机型的某个版本 = 把它变成「正在使用」（作者 2026-10-06 真机反馈：
 * 不能只有预设页点「应用」才算切换 —— 首页三级选齐一台、抽屉里点一份、
 * 参数页换组合，都要真切换，四个页面同账）。
 *
 * 两条守则：
 * - **底账已经指着这个 combo 时不动** —— 正在用的可能是「我的文件」，
 *   随手浏览回来不该把它顶成官方底稿；
 * - **应用不了（还没下载 / 内容与目录漂了）就保持原账**：页面选择仍成立，
 *   「未下载 / 内容异常」的三态如实显示 —— 不弹错误打断浏览，也不编一句"切换成功"。
 */
export function activateCombo(
  model: string | null,
  variant: string | null,
  fileOf: (machineId: string, versionId: string) => string | null,
  active: ActivePreset | null,
): void {
  if (model === null || variant === null || model === '' || variant === '') return
  if (active !== null && active.machineId === model && active.versionId === variant) return
  /*
   * 浏览器预览（mock）里没有交付字节 —— 官方线的应用在那里**如实拒**
   * （NotImplementedError 的技术形式进控制台，那是给显式按钮的提示通道）。
   * 浏览行为不去触发它：选择仍然成立、底账保持原样，与"应用不了"同一条守则。
   */
  if (!inTauri) return
  const fileName = fileOf(model, variant)
  if (fileName === null) return
  void api
    .applyActivePreset(fileName, 'official')
    .then(() => appStateMutated())
    .catch(() => {
      console.debug(`[appState] ${fileName} 这一步应用不了（多半还没下载）—— 底账保持原样`)
    })
}
