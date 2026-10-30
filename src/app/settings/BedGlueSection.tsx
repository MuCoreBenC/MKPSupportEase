/*
 * 设置页「热床预涂胶」那一块（2026-10-17）—— **危险操作**，整块用警告色。
 *
 * 为什么单独一个文件：它的样子与别的块不一样（警告底 + 三角图标 + 内嵌风险提示），
 * 混在别的块里会让"哪一块是危险的"这件事靠读文案才知道。
 * 作者的原话：「可能导致笔尖划伤打印板，建议不要开启，除非您清楚了解风险」。
 *
 * ★ 这块**真的会往 G-code 里插东西**（`pipeline/bed_glue.rs`，从旧程序移植过来的）：
 *   在自动调平后、检测条前降温到 30°C → 换笔 → 涂胶 → 收笔，然后把热床升回原温度。
 *   校准件那一趟不涂（没有擦料塔，路径会跑出校准区域），这是内核里的硬边界。
 *
 * ★ 文案走字典（2026-10-25）：风险那句是全页最该被读懂的一句，英文版逐句对齐。
 */

import { useState } from 'react'

import { useT } from '../../i18n'
import { usePostprocPrefs } from '../state/usePostprocPrefs'
import { Switch } from '../../components/switch'
import { Tip } from '../../components/tooltip'
import s from './PageSettings.module.css'

export default function BedGlueSection() {
  const tr = useT()
  const { prefs, set } = usePostprocPrefs()
  const [err, setErr] = useState<string | null>(null)

  const on = (why: string) => setErr(why)

  if (prefs === null) {
    return (
      <section className={`${s.section} ${s.dangerBlock}`} aria-label={tr('settings.glue.title')}>
        <h2 className={`${s.secTitle} ${s.dangerTitle}`}>
          <WarnIcon />
          {tr('settings.glue.title')}
        </h2>
        <p className={s.fieldNote}>{tr('common.loading')}</p>
      </section>
    )
  }

  const r = prefs.resolved

  return (
    <section className={`${s.section} ${s.dangerBlock}`} aria-label={tr('settings.glue.title')}>
      <h2 className={`${s.secTitle} ${s.dangerTitle}`}>
        <WarnIcon />
        {tr('settings.glue.title')}
      </h2>

      {/* 风险提示：与下面那个开关分开摆 —— 先让人看见"这是什么性质的事"，再看见开关 */}
      <p className={s.dangerNote}>
        <span className={s.dangerNoteHead}>
          <WarnIcon />
          {tr('common.danger')}
        </span>
        {tr('settings.glue.risk')}
      </p>

      <div className={`${s.field} ${s.fieldRow}`}>
        <div className={s.fieldText}>
          <div className={`${s.fieldTitle} ${s.dangerTitle}`}>
            {tr('settings.glue.enable')}
            <Tip text={tr('settings.glue.enableNote')} />
          </div>
        </div>
        <Switch
          checked={r.bedGlueEnabled}
          ariaLabel={tr('settings.glue.enable')}
          onChange={(next) => void set({ bedGlueEnabled: next }, on)}
        />
      </div>

      {/* 只在开着时才问"涂哪儿"：关着的时候问等于让用户配一个不生效的东西 */}
      {r.bedGlueEnabled && (
        <div className={`${s.field} ${s.fieldRow}`}>
          <div className={s.fieldText}>
            <div className={s.fieldTitle}>
              {tr('settings.glue.allModels')}
              <Tip text={tr('settings.glue.allModelsNote')} />
            </div>
          </div>
          <Switch
            checked={r.bedGlueAllModels}
            ariaLabel={tr('settings.glue.allModels')}
            onChange={(next) => void set({ bedGlueAllModels: next }, on)}
          />
        </div>
      )}

      {err !== null && <p className={`${s.note} ${s.noteBad}`}>{err}</p>}
    </section>
  )
}

/** 三角警示图标（14px，`currentColor` 跟着块的字色走） */
function WarnIcon() {
  return (
    <svg viewBox="0 0 16 16" width="13" height="13" fill="none" aria-hidden="true">
      <path
        d="M8 2.4 14.2 13H1.8L8 2.4Z"
        stroke="currentColor"
        strokeWidth="1.2"
        strokeLinejoin="round"
      />
      <path d="M8 6.4v3" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
      <circle cx="8" cy="11.2" r="0.6" fill="currentColor" />
    </svg>
  )
}
