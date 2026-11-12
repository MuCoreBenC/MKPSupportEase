/*
 * 设置页「热床预涂胶」那一块（2026-10-17）—— **危险操作**，整张卡用警告色。
 *
 * 为什么单独一个文件：它的样子与别的块不一样（橙色卡 + 风险句作卡片说明），
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
import { SectionHead } from './SectionHead'
import s from './PageSettings.module.css'

/**
 * `nested`：挂在「后处理」大类底下（2026-11-09）—— 标题降成小标题（`h3`）、
 * 上下留白收一档。橙色（危险）这层含义不变，只是不再是一整章。
 */
export default function BedGlueSection({ nested = false }: { nested?: boolean }) {
  const tr = useT()
  const { prefs, set } = usePostprocPrefs()
  const [err, setErr] = useState<string | null>(null)

  const on = (why: string) => setErr(why)

  const box = `${s.section} ${s.dangerBlock}`
  /*
   * 风险句就是这张橙卡的说明（2026-11-10，照参考稿）—— 不再单独占一段、
   * 也不再另起一个「危险操作」小标题：整张卡是橙的，这一句就是"为什么危险"。
   */
  const head = (
    <SectionHead
      icon="drop"
      level={nested ? 3 : 2}
      title={tr('settings.glue.title')}
      desc={tr('settings.glue.risk')}
    />
  )

  if (prefs === null) {
    return (
      <section className={box} aria-label={tr('settings.glue.title')}>
        {head}
        <p className={s.fieldNote}>{tr('common.loading')}</p>
      </section>
    )
  }

  const r = prefs.resolved

  return (
    <section className={box} aria-label={tr('settings.glue.title')}>
      {head}

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

      {/* 只在开着时才问"涂哪儿"：关着的时候问等于让用户配一个不生效的东西。
          ★ 它由上面那颗开关放出来 ⇒ 缩进（缩进的唯一判据，见 `CalibSection`）。 */}
      {r.bedGlueEnabled && (
        <div className={`${s.field} ${s.fieldRow} ${s.rowSub}`}>
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
