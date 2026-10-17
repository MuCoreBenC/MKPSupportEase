/*
 * 「保存之后的引导提示，要不要弹」这一个**共用开关**（2026-10-17 作者）。
 *
 * # 它管的是什么
 *
 * 保存成功之后那一屏「参数已保存 —— 切片器不会自己知道参数变了」（`ResliceNotice`）。
 * 参数页与校准页是**同一句话的两个入口**（同一份组件），作者要的是"一处关、处处关"：
 * 提示里的「不再显示」与设置页「引导」那一格**写的是同一个键**
 * （`STORAGE.clientShowGuides`），不是各记各的。
 *
 * # 两个入口，两种用法
 *
 *   提示里的「不再显示」   勾上 = 关（读/写都走 `useGuidesPref`，多实例同步）
 *   设置页「引导」那一格   开关 = 同一个键（作者：想再看就解开）
 *
 * 保存流程（`useParams` / `useCalibration`）在**决定弹不弹的那一刻**用 `guidesEnabled()`
 * 读盘上真值 —— 不走 hook：那一刻要的是"现在到底开不开"，不是"我这份 state 记的是啥"，
 * 免得页签常驻挂载下读到一份旧值。
 *
 * # 没设过 = 显示
 *
 * 默认是"问一句"（`true`）：只有用户明确关掉才不弹。盘上没有这个键 / 值是坏的时候
 * 一律按"显示"处理 —— 别让一个读坏的值把提示永久关掉。
 */

import { STORAGE } from '../../api/storageKeys'
import { useSyncedStickyState } from '../../hooks/useSyncedStickyState'

function isBool(v: unknown): v is boolean {
  return typeof v === 'boolean'
}

/** 现在要不要弹这类引导提示（**读盘上真值**，没设过 / 读坏了 = 显示） */
export function guidesEnabled(): boolean {
  try {
    const raw = localStorage.getItem(STORAGE.clientShowGuides)
    if (raw === null) return true
    const v = JSON.parse(raw) as unknown
    return typeof v === 'boolean' ? v : true
  } catch {
    return true
  }
}

/**
 * 同一个开关的**可订阅**版本（提示里的勾选、设置页那一格都用它）：
 * 写的时候广播一声，于是另一处已经挂着的实例会跟着变 —— 页签是常驻挂载的，
 * 不广播就会"这边关了、那边还显示开着"。
 */
export function useGuidesPref(): [boolean, (v: boolean) => void] {
  return useSyncedStickyState<boolean>(STORAGE.clientShowGuides, true, isBool)
}
