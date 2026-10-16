/**
 * 工作台的主题（2026-10-16「照客户端的样子给工作台也做一套深色」）。
 *
 * # 解析那一句不在这儿写
 *
 * 「档位 + 系统深浅 ⇒ 到底是不是深色」取 `src/app/state/theme` 里那唯一一份 ——
 * 客户端主窗口、引导窗口、**工作台**三扇窗共用同一句判据。各写一遍就是两个不同源
 * 的判据，迟早对不上（引导窗口那一档已经出过一次）。
 *
 * # 为什么工作台必须自己落一次 `data-theme`
 *
 * 这扇窗**不跑 appState**（`main.tsx` 与 `src/app/App` 是两棵不相干的树），
 * 所以客户端那条「读 AppState 的 theme 格 → 落 `<html data-theme>`」在这儿
 * **根本不会发生**。不落的话全窗只认系统深浅：人在设置里点了「深色」而 macOS
 * 是浅色，工作台就还是白的 —— 与客户端对不上（引导窗口 2026-10-15 出的正是这一档）。
 *
 * # 档位存在哪
 *
 * 命令就是客户端那两条（`get_theme` / `set_theme`，`ipc::prefs`），**但落点是工作台
 * 自己那棵数据根**（identifier = `SupportEase-Workbench`）：两个应用各记各的一档，
 * 不跨 identifier 去读对方那棵树。
 */
import { useSyncExternalStore } from 'react'

import type { ThemePref } from '../api/contract'
import { applyTheme, prefersDark, resolveDark } from '../app/state/theme'
import { wb } from './api'

/** 盘上那一档。`null` = 还没选过 ⇒ 按「跟随系统」 */
let pref: ThemePref | null = null

const listeners = new Set<() => void>()

function subscribe(listener: () => void): () => void {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

/** 现在到底是不是深色 —— **解析完的真值**（档位 + 系统深浅）。给"要拿真值去画"的读者 */
export function useResolvedDark(): boolean {
  return useSyncExternalStore(subscribe, () => resolveDark(pref))
}

/** 盘上那一档。给设置页显示"选的是哪一档" */
export function useThemePref(): ThemePref | null {
  return useSyncExternalStore(subscribe, () => pref)
}

/** 落一次：写 `<html data-theme>` 并广播给订阅者 */
function land(next: ThemePref | null): void {
  pref = next
  applyTheme(next)
  for (const l of listeners) l()
}

/**
 * 开场那一笔：**先按「跟随系统」落一次**（首帧不等 IPC —— 工作台窗口是首帧之后
 * 才 `show()` 的，为一条偏好把整棵模块树压后一分钟不值），再拿盘上那一档纠正。
 *
 * 读不到（桩 / 首次启动）就停在「跟随系统」—— `null` 正是那一档。
 */
export function initTheme(): void {
  applyTheme(pref)
  void wb
    .theme()
    .then(land)
    .catch(() => undefined)

  /* 跟随系统那一档：系统深浅变了要跟着翻 —— 用户点了浅 / 深则不理会（真值在他那一档里） */
  prefersDark.addEventListener('change', () => {
    if (pref === null || pref === 'system') {
      applyTheme(pref)
      for (const l of listeners) l()
    }
  })
}

/**
 * 换档：**真值由后端答**（它落盘、它顺手把原生窗口那一层也换掉），
 * 回来了再落一次界面 —— 界面上显示的永远是盘上那一格。
 */
export async function setTheme(next: ThemePref): Promise<ThemePref> {
  const stored = await wb.setTheme(next)
  land(stored)
  return stored
}
