/*
 * 切片软件的选择（2026-10-13 作者口径）。
 *
 * # 默认用哪个（设置页那一格）
 *
 * 纯前端偏好 —— 落 localStorage（`STORAGE.clientSlicer`），不走 AppState：
 * 它不进后端账本，壳（Tauri）那边将来接 `openModel` 时**由前端把这个值随调用带给它**。
 *
 * # 打开模型的模态框
 *
 * 「打开模型」确认框里摆一个「用哪个打开」下拉 + 「仅这一次用选定的」勾选：
 * - 不勾：确认时**把选择写回默认**（下次设置页里看到的就是它）；
 * - 勾了：只用这一次，默认不动。
 * 状态住在页面里（两个页面各一份，弹窗开着才有意义），`useSlicerPick` 负责弹窗
 * 每次打开时把选择重置回当前默认 —— 上一次的「仅这一次」不该渗到下一次。
 */

import { useEffect, useState } from 'react'
import { api } from '../../api'
import type { SlicerApp } from '../../api/contract'
import { STORAGE } from '../../api/storageKeys'
/* 多实例同步的 sticky 偏好：切片这一格与「引导提示」开关共用这一份（见 `hooks/`） */
import { useSyncedStickyState } from '../../hooks/useSyncedStickyState'

/** 四档，**id 是给壳的契约**（跟随系统 = 交给系统关联程序，不指定具体应用） */
export type SlicerId = 'system' | 'bambu' | 'orca' | 'custom'

export const SLICER_CHOICES: { id: SlicerId; label: string; hint: string }[] = [
  { id: 'system', label: '跟随系统默认', hint: '用 .3mf 关联的程序打开（没选过时就是这个）' },
  { id: 'bambu', label: 'Bambu Studio', hint: '总是交给 Bambu Studio' },
  { id: 'orca', label: 'OrcaSlicer', hint: '总是交给 OrcaSlicer' },
]

/**
 * 「跟随系统默认」那一档。**单独取出来是为了别的代码不靠下标说话** —— 见
 * [`systemSlicerLine`]：问不到应用名时回落的就是它自己的 hint（那档的说明只有一个出处，
 * 就是上面那张表）。
 */
const SYSTEM_CHOICE = SLICER_CHOICES[0]

/**
 * 设置页下拉里那条**动作项**（不是真的档位）：选中它 = 去磁盘挑一个（走
 * `api.pickSlicerApp`）。与任何 `SlicerId` 都不同名，调用点就能一眼认出
 * "这是动作、不是值" —— 动作不写盘，挑中才落（`PageSettings` 的 `pickSlicer`）。
 */
export const SLICER_CHOOSE = '__choose'

/**
 * 下拉里列出来的档：内置几档 +（按路径指定过就多一条）自定义应用。
 *
 * 形状 = 共用件 `SelectField` 的 `FieldOption`（`{ value, label, note }`，
 * `note` 是弹层选项右边那列小字）。**设置页与「打开模型」框共用同一份** ——
 * 两处列的东西必须一模一样，不然模态框的下拉认不出"当前默认就是自定义应用"。
 *
 * 自定义那条 label 只写应用名（"（你指定的）"由 note 承担）；完整路径太长，
 * 不进弹层，摆在下拉下面那行（见 `PageSettings`）。
 *
 * `systemApp`（2026-10-16）= 「跟随系统默认」**在这台机器上指谁**：问到就把名字写进
 * 那条的 note，问不到（`null`）就用原来那句话 —— 见 [`systemSlicerLine`]。
 */
export function slicerOptionsOf(
  app: SlicerApp | null,
  systemApp: SlicerApp | null,
): { value: SlicerId; label: string; note: string }[] {
  const base = SLICER_CHOICES.map((c) => ({
    value: c.id,
    label: c.label,
    note: c.id === 'system' ? systemSlicerLine(systemApp) : c.hint,
  }))
  if (app === null) return base
  return [...base, { value: 'custom', label: app.name, note: '你指定的' }]
}

/**
 * 「跟随系统默认」**在这台机器上**是什么意思（作者 2026-10-16：那一档名字里没有
 * 应用名，看不出它指谁）。三处说同一句：下拉那条 note、设置页下面那行小字、
 * 「打开模型」框里那行 —— 同一句话只写一次，免得三处各漂一处。
 *
 * 问不到应用名（`null`：这台机器没关联任何程序 / 这一次问不成）就回落**那一档自己的
 * 说明**：界面照旧显示原来那句，**不编一个名字顶上**。
 */
export function systemSlicerLine(systemApp: SlicerApp | null): string {
  return systemApp === null
    ? SYSTEM_CHOICE.hint
    : `用 .3mf 关联的程序打开：${systemApp.name}`
}

/**
 * 「跟随系统默认」到底交给谁 —— 问一次壳（`api.getSystemSlicerApp`，问的是系统真值）。
 *
 * 什么时候问：**挂载时**一次（设置页与「打开模型」框各问各的；那个框每次打开都是新挂载
 * ⇒ 每次打开都是新答案）+ **窗口重新拿到焦点时**再问一次 —— 用户完全可能切出去在系统
 * 里换一个关联再回来，而那件事没有任何广播可听（它不是我们改的，是系统改的）。
 *
 * 问不出来 / 还没答上来一律 `null`：调用方按"不知道"渲染（回落原来那句话术，不编）。
 */
export function useSystemSlicerApp(): SlicerApp | null {
  const [app, setApp] = useState<SlicerApp | null>(null)
  useEffect(() => {
    let alive = true
    const read = () => {
      api.getSystemSlicerApp().then(
        (found) => {
          if (alive) setApp(found)
        },
        () => {
          /* 问不出来不是错：那句话术本来就有"没选过时就是这个"的写法，界面不报这一条 */
        },
      )
    }
    read()
    window.addEventListener('focus', read)
    return () => {
      alive = false
      window.removeEventListener('focus', read)
    }
  }, [])
  return app
}

/** 默认那一份（设置页写，打开模型读）。值不合法时回落到跟随系统 */
export function useSlicerPref() {
  return useSyncedStickyState<SlicerId>(STORAGE.clientSlicer, 'system', isSlicerId)
}

/**
 * 从磁盘指定的那个应用（`{ name, path }`）。切回内置档**不删它** ——
 * 下次想切回来不用重挑（与"记住上一次"的偏好同一条口径）。
 */
export function useSlicerAppPref() {
  return useSyncedStickyState<SlicerApp | null>(STORAGE.clientSlicerApp, null, isSlicerApp)
}

function isSlicerId(v: unknown): v is SlicerId {
  return v === 'system' || v === 'bambu' || v === 'orca' || v === 'custom'
}

function isSlicerApp(v: unknown): v is SlicerApp | null {
  if (v === null) return true
  if (typeof v !== 'object') return false
  const o = v as { name?: unknown; path?: unknown }
  return typeof o.name === 'string' && typeof o.path === 'string'
}

/** 模态框里的瞬时选择：每次打开重置为当前默认、「仅这一次」清掉 */
export function useSlicerPick(defaultSlicer: SlicerId, open: boolean) {
  const [value, setValue] = useState<SlicerId>(defaultSlicer)
  const [once, setOnce] = useState(false)
  useEffect(() => {
    if (open) {
      setValue(defaultSlicer)
      setOnce(false)
    }
  }, [open, defaultSlicer])
  return { value, setValue, once, setOnce }
}

/** 确认打开时的两件事：不勾「仅这一次」就把选择写成新默认；返回真正要用的那一份 */
export function settleSlicerPick(
  pick: { value: SlicerId; once: boolean },
  setDefault: (id: SlicerId) => void,
): SlicerId {
  if (!pick.once) setDefault(pick.value)
  return pick.value
}

/**
 * 递给壳的那份「用哪个打开」（`api.openModel` 的 opts）—— 四个调用点都走这里：
 * 1. `settleSlicerPick` 照旧（不勾「仅这一次」就把选择写回默认）；
 * 2. 定的是 `'custom'` 时把**路径**一起带上（壳读不到 webview 的 localStorage）；
 *    记录不在（清过缓存）就回落跟随系统 —— 不递一个壳解析不了的值。
 */
export function slicerArgsOf(
  pick: { value: SlicerId; once: boolean },
  setDefault: (id: SlicerId) => void,
  app: SlicerApp | null,
): { slicer: SlicerId; slicerApp?: string } {
  const picked = settleSlicerPick(pick, setDefault)
  if (picked !== 'custom') return { slicer: picked }
  if (app === null) return { slicer: 'system' }
  return { slicer: 'custom', slicerApp: app.path }
}
