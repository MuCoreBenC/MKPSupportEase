/*
 * 校准模型的三档版本（2026-11-12）。
 *
 * 「打开模型」那个弹窗在校准页与首页各有一个入口 —— 两处**必须**是同一份清单，
 * 否则同一块板在两页会打开不同的 3mf（首页曾经就落后一档：只有原版）。
 *
 * `id` 是契约：直接喂给 `api.openModel`，映射在 Rust 的 `MODEL_DEFS` 里
 * （`combined` / `z-ext` / `z` / `xy-ext` / `xy` / `test-models`）。
 * `note` 是下拉选项右侧那列小字 —— 名字保持短，说明放那里（触发器里是 `nowrap + ellipsis`）。
 */

import type { StringKey } from '../../i18n'

export type CalibVersion = {
  /** `api.openModel` 认的 id */
  id: string
  /** 短名（`原版` / `改版` / `二合一`） */
  label: StringKey
  /** 一句话说明；不写就是没有 */
  note?: StringKey
}

/** Z 板三档：二合一（默认）/ 改版 / 原版。 */
export const Z_VERSIONS: readonly CalibVersion[] = [
  { id: 'combined', label: 'calib.board.combined', note: 'calib.board.combinedNote' },
  { id: 'z-ext', label: 'calib.board.ext', note: 'calib.board.extNote' },
  { id: 'z', label: 'calib.board.orig' },
]

/** XY 板三档：同一份二合一，另两档是 XY 自己的原版与改版。 */
export const XY_VERSIONS: readonly CalibVersion[] = [
  { id: 'combined', label: 'calib.board.combined', note: 'calib.board.combinedNote' },
  { id: 'xy-ext', label: 'calib.board.ext', note: 'calib.board.extNote' },
  { id: 'xy', label: 'calib.board.orig' },
]

/** 默认选哪一档（三档都是二合一打头，这里写明"默认就是它"这件事）。 */
export const DEFAULT_VERSION_ID = 'combined'
