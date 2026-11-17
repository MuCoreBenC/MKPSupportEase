/*
 * 「恢复后处理默认值」那一行（2026-11-17）。
 *
 * # 它恢复的是哪一份
 *
 * 后处理那 11 个偏好全部置回 `null` —— 也就是「没设过」。后端 `resolved()` 拿
 * `null` 去读**出厂值**，而那份出厂值现在是可配的（工作台「默认配置」页写
 * `presets/client-defaults.toml`）。所以这颗按钮是「老用户套用新默认值」的入口：
 * 默认值只影响新装，已经装好的得自己点一下。
 *
 * ★ 不在这里写死任何数：写死就等于把「恢复默认」变成「恢复成上一版」，与
 *   `DebugPrefsModal` 那颗的取舍同一条理。
 *
 * # 为什么只覆盖 11 项
 *
 * 那 11 项是 `PrefsIr` 的全部 —— 真改变切出来 G-code 的那几格。调试动作那组
 * （`debug_*`）有它自己的「恢复默认」，住 `DebugPrefsModal` 里，两处各管各的。
 *
 * # 二次确认
 *
 * 点一下先变「再点一次确认」，三秒不动退回去（与 `DebugPrefsModal` 那颗同一个口径）。
 * 这不是删数据，但它会一次改掉满屏开关 —— 值得一次确认。
 */

import { useState } from 'react'

import { useT } from '../../i18n'
import { usePostprocPrefs } from '../state/usePostprocPrefs'
import { Tip } from '../../components/tooltip'
import { Btn } from '../ui/Controls'
import { SectionHead } from './SectionHead'
import s from './PageSettings.module.css'

const CONFIRM_MS = 3000

/**
 * 「恢复默认」的那一份补丁：全给 `null`，由后端按出厂重算。
 *
 * 键与 `postproc::ir::PrefsIr` 一一对应；漏一个就是「那一项恢复不了」，
 * 多一个会被后端忽略（`setPostprocPrefs` 收的是整格覆盖）。
 */
const RESET_PATCH = {
  glueBottomEnabled: null,
  underSupportEnabled: null,
  underSupportMm: null,
  skipVibrationCalibration: null,
  beginStageRetractEnabled: null,
  beginStageRetractZLiftMm: null,
  zCalibrationDwellSec: null,
  xyCalibrationDwellSec: null,
  bedGlueEnabled: null,
  bedGlueAllModels: null,
  autoTowerPosition: null,
} as const

export default function ResetPostprocDefaults({ nested = false }: { nested?: boolean }) {
  const tr = useT()
  const { prefs, set } = usePostprocPrefs()
  const [err, setErr] = useState<string | null>(null)
  const [armed, setArmed] = useState(false)
  const [busy, setBusy] = useState(false)

  const run = (): void => {
    if (busy) return
    if (!armed) {
      setArmed(true)
      window.setTimeout(() => setArmed(false), CONFIRM_MS)
      return
    }
    setArmed(false)
    setBusy(true)
    setErr(null)
    void set(RESET_PATCH, setErr).finally(() => setBusy(false))
  }

  const label = busy
    ? tr('settings.resetDefaults.busy')
    : armed
      ? tr('settings.resetDefaults.confirm')
      : tr('settings.resetDefaults.action')

  return (
    <section className={s.section} aria-label={tr('settings.resetDefaults.title')}>
      {nested ? (
        <SectionHead
          icon="target"
          level={3}
          title={tr('settings.resetDefaults.title')}
          desc={tr('settings.resetDefaults.desc')}
        />
      ) : (
        <SectionHead
          icon="target"
          level={2}
          title={tr('settings.resetDefaults.title')}
          desc={tr('settings.resetDefaults.desc')}
        />
      )}

      <div className={`${s.field} ${s.fieldRow}`}>
        <div className={s.fieldText}>
          <div className={s.fieldTitle}>
            {tr('settings.resetDefaults.row')}
            <Tip text={tr('settings.resetDefaults.rowNote')} />
          </div>
        </div>
        <Btn variant={armed ? 'danger' : 'default'} disabled={prefs === null || busy} onClick={run}>
          {label}
        </Btn>
      </div>

      {err !== null && <p className={`${s.note} ${s.noteBad}`}>{err}</p>}
    </section>
  )
}
