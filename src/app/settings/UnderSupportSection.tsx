/*
 * 设置页「下方支撑余量（实验）」那一块（2026-11-17）。
 *
 * 判据先在试验台验过（`docs/GLUE-UNDER-SUPPORT-2026-11-16.md`）：
 * 涂胶段离**笔下方那层支撑的边界** ≥ 阈值 ⇒ 这一段不抬高；不够就按原判
 * （边缘带）处理。承托面取上一层的支撑材料（Support / 界面 / 过渡 / 熨烫），
 * 被围住的内孔填平 —— 孔里是零件本体，笔走过去有东西托着。
 *
 * ★ 这一行**不是预设参数**：它写进 `postproc` 偏好格，不进参数表。
 *   作者 2026-11-17：「不改字段了，直接在设置页面加……后续好用我才做进修改参数里面」。
 * ★ 默认关：关掉时一行都不攒、一个字节都不改（内核那边 `under_support_enabled`
 *   为假时不收集支撑，也不建网格）。
 */

import { useState } from 'react'

import { useT } from '../../i18n'
import NumberField from '../../components/field/NumberField'
import { Switch } from '../../components/switch'
import { Tip } from '../../components/tooltip'
import { usePostprocPrefs } from '../state/usePostprocPrefs'
import { SectionHead } from './SectionHead'
import s from './PageSettings.module.css'

/**
 * `nested`：挂在「后处理」大类底下（与校准、热床涂胶同一层）——
 * 标题降成小标题、上下留白收一档。
 */
export default function UnderSupportSection({ nested = false }: { nested?: boolean }) {
  const tr = useT()
  const { prefs, set } = usePostprocPrefs()
  const [err, setErr] = useState<string | null>(null)

  const on = (why: string) => setErr(why)

  const head = (
    <SectionHead
      icon="target"
      level={nested ? 3 : 2}
      title={tr('settings.under.title')}
      desc={tr('settings.under.desc')}
    />
  )

  if (prefs === null) {
    return (
      <section className={s.section} aria-label={tr('settings.under.title')}>
        {head}
        <p className={s.fieldNote}>{tr('common.loading')}</p>
      </section>
    )
  }

  const r = prefs.resolved
  const limit = prefs.limits.underSupport

  return (
    <section className={s.section} aria-label={tr('settings.under.title')}>
      {head}

      <div className={`${s.field} ${s.fieldRow}`}>
        <div className={s.fieldText}>
          <div className={s.fieldTitle}>
            {tr('settings.under.enable')}
            <Tip text={tr('settings.under.enableNote')} />
          </div>
        </div>
        <Switch
          checked={r.underSupportEnabled}
          ariaLabel={tr('settings.under.enable')}
          onChange={(next) => void set({ underSupportEnabled: next }, on)}
        />
      </div>

      {/* 只在开关开着时才问阈值：关着还摆一个不生效的数等于骗人 */}
      {r.underSupportEnabled && (
        <div className={`${s.field} ${s.fieldRow} ${s.rowSub}`}>
          <div className={s.fieldText}>
            <div className={s.fieldTitle}>
              {tr('settings.under.margin')}
              <Tip text={tr('settings.under.marginNote', { min: limit.min, max: limit.max })} />
            </div>
          </div>
          <span className={s.numBox}>
            <NumberField
              label={tr('settings.under.margin')}
              unit="mm"
              value={r.underSupportMm}
              min={limit.min}
              max={limit.max}
              step={0.05}
              onChange={(next) => void set({ underSupportMm: next }, on)}
            />
          </span>
        </div>
      )}

      {err !== null && <p className={s.fieldNote}>{err}</p>}
    </section>
  )
}
