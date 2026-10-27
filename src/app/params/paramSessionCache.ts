/*
 * 参数页那几层的**会话缓存**（关掉模态框再进来接着改）—— 2026-10-19。
 *
 * 照 `src/app/presets/PagePresets.tsx` 的 `textHistCache` 抄的（兄弟链「编辑文件」抽屉
 * 早就靠一份会话缓存把同一个缝堵上了）。
 *
 * # 它存什么、不存什么
 *
 * 只存**这一屏自己的内存状态**，也就是**别处都没有的那两样**：
 *
 *   · **草稿层** —— 改了还没保存的那几项（`useParams` 的 `draft`）；
 *   · **撤销 / 重做栈**（`past` / `future`）。
 *
 * **不存**（那两样已经有主，存了会打架）：
 *
 *   · 「已保存」那一层 —— 真相在盘上那份文件里（`savedValueOf` 去读它）；
 *   · **修改历史 `log` / 序号 `seq`** —— 后端**按文件持久化**（`get_preset_history` /
 *     `put_preset_history`，`useParams` 里"读那本 / 写那本"两个 effect），
 *     打开时自己会读回来（作者 2026-10-17 那条"改过 → 关掉 → 再打开还在"）。
 *     这一层再存一份，两条读会互相盖。
 *
 * # 为什么需要它
 *
 * 作者 2026-10-19 实测：进编辑模式改了几处、**不保存直接关掉**，下次进来**改动看不见** ——
 * 盘上草稿还在（`run/draft-preset.json`），可这一屏的 `draft` 从 `{}` 起（`useParams` 里
 * 那个 state 只在 `save` / 切 combo 时清、**从不 seed**），`valueOf` 于是落回盘上那份文件的旧值。
 * 这一层把那份内存状态按"编辑目标"记着，重开时接上。
 *
 * # 有效性只认一条
 *
 * **后端答 `reused`**（`beginPresetEdit`：按"同一份"认到草稿才算接着改）。
 * `reused === false` ⇒ 草稿是这趟新建的、或已被别处作废（正文链保存会把参数草稿作废，
 * 见契约 `PresetTextDraft`）⇒ 旧快照对不上手，整条丢掉。
 * 与 `PagePresets` 那句「对不上（草稿被保存 / 被别处动过）就丢掉」同一条纪律。
 *
 * # 边界
 *
 * **换台机器 / 重启应用之后这条缓存不在了**（进程内存里没有）—— 那是 2026-10-19
 * 补上的另一半：`useParams` 发现"后端答 `reused`、缓存却空"时，向后端要
 * **草稿与来源的参数级差异**（`api.readPresetDraftChanges()`）把草稿层接回来。
 * 所以重启之后照样看得见"上次改了哪几项"；接不回来的是**撤销 / 重做栈**
 * （那是操作序列，后端不存），以及"这几项是怎么一步步改成的"。
 */

import type { EditStep } from './useParams'

export interface ParamSession {
  /** 草稿层：改了还没保存的那几项（key → 新原文） */
  draft: Record<string, string>
  past: EditStep[]
  future: EditStep[]
}

const cache = new Map<string, ParamSession>()

/**
 * 编辑目标的记号：两条线各自的钥匙。
 * 与后端 `draft_subject`（`src-tauri/src/ipc/local.rs`）同一套：用户线认路径、云端线认文件名。
 */
export const paramTargetKey = (origin: string, path: string | null, fileName: string): string =>
  `${origin}:${path ?? fileName}`

export function readParamSession(key: string): ParamSession | null {
  return cache.get(key) ?? null
}

export function writeParamSession(key: string, session: ParamSession): void {
  cache.set(key, session)
}

/** 保存（草稿被提交掉了）之后作废这条缓存 —— 再进来该是一台干净的台 */
export function clearParamSession(key: string): void {
  cache.delete(key)
}
