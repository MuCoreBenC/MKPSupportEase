/*
 * 设置页「校准与补偿」那一块（2026-10-17）。
 *
 * 五行：跳过震动校准检测 / 开始阶段收笔 / 收笔抬升高度 / Z 轴停留时间 / XY 轴停留时间。
 *
 * ★★ 这几行**真的改变切出来的 G-code**（2026-10-17 之前它们是写死在管线里的常量，
 * 设置页改不动）。现在它们经 `runtime::postproc_prefs::apply_to_ir` 写进 `ir.prefs`，
 * 由管线读。所以这里的说明文字必须**说清后果** —— 用户要知道打开它会发生什么。
 *
 * ★ 数字框的量程来自后端（`prefs.limits`），**不在这里另写一份**：
 * 两处各写一份的话，迟早会出现"界面允许 30 秒、管线只认到 20 秒"这种漂移。
 * 收笔抬升高度（2026-10-21 作者：「抬得有点高，给个设置」）同理 ——
 * 管线对越界值是回落到 20 而不是夹取，量程对不上就是静默偏差。
 *
 * ★ 文案走字典（2026-10-25）：数字框的 `label` 也是可访问名，同样得跟着语言换。
 */

import { useState } from 'react'

import { useT } from '../../i18n'
import { usePostprocPrefs } from '../state/usePostprocPrefs'
import NumberField from '../../components/field/NumberField'
import { Switch } from '../../components/switch'
import { Tip } from '../../components/tooltip'
import s from './PageSettings.module.css'

export default function CalibSection() {
  const tr = useT()
  const { prefs, set } = usePostprocPrefs()
  const [err, setErr] = useState<string | null>(null)

  const on = (why: string) => setErr(why)

  if (prefs === null) {
    return (
      <section className={s.section} aria-label={tr('settings.calib.title')}>
        <h2 className={s.secTitle}>{tr('settings.calib.title')}</h2>
        <p className={s.fieldNote}>{tr('common.loading')}</p>
      </section>
    )
  }

  const r = prefs.resolved
  const { zDwell, xyDwell, zLift } = prefs.limits

  return (
    <section className={s.section} aria-label={tr('settings.calib.title')}>
      <h2 className={s.secTitle}>{tr('settings.calib.title')}</h2>

      <div className={`${s.field} ${s.fieldRow}`}>
        <div className={s.fieldText}>
          <div className={s.fieldTitle}>
            {tr('settings.calib.skipVibration')}
            <Tip text={tr('settings.calib.skipVibrationNote')} />
          </div>
        </div>
        <Switch
          checked={r.skipVibrationCalibration}
          ariaLabel={tr('settings.calib.skipVibration')}
          onChange={(next) => void set({ skipVibrationCalibration: next }, on)}
        />
      </div>

      <div className={`${s.field} ${s.fieldRow}`}>
        <div className={s.fieldText}>
          <div className={s.fieldTitle}>
            {tr('settings.calib.beginRetract')}
            <Tip text={tr('settings.calib.beginRetractNote')} />
          </div>
        </div>
        <Switch
          checked={r.beginStageRetractEnabled}
          ariaLabel={tr('settings.calib.beginRetract')}
          onChange={(next) => void set({ beginStageRetractEnabled: next }, on)}
        />
      </div>

      {/* 只在收笔开着时才问抬多高：关着的时候配一个不生效的数等于骗人
          （与热床涂胶那一块"只在开着时才问涂哪儿"同一条规矩） */}
      {r.beginStageRetractEnabled && (
        <div className={`${s.field} ${s.fieldRow} ${s.rowSub}`}>
          <div className={s.fieldText}>
            <div className={s.fieldTitle}>
              {tr('settings.calib.lift')}
              <Tip text={tr('settings.calib.liftNote', { min: zLift.min, max: zLift.max })} />
            </div>
          </div>
          <span className={s.numBox}>
            <NumberField
              label={tr('settings.calib.lift')}
              unit="mm"
              value={r.beginStageRetractZLiftMm}
              min={zLift.min}
              max={zLift.max}
              step={1}
              onChange={(next) => void set({ beginStageRetractZLiftMm: next }, on)}
            />
          </span>
        </div>
      )}

      <div className={`${s.field} ${s.fieldRow} ${s.rowSub}`}>
        <div className={s.fieldText}>
          <div className={s.fieldTitle}>
            {tr('settings.calib.zDwell')}
            <Tip text={tr('settings.calib.zDwellNote', { min: zDwell.min, max: zDwell.max })} />
          </div>
        </div>
        <span className={s.numBox}>
          <NumberField
            label={tr('settings.calib.zDwell')}
            unit={tr('settings.calib.seconds')}
            value={r.zCalibrationDwellSec}
            min={zDwell.min}
            max={zDwell.max}
            step={1}
            onChange={(next) => void set({ zCalibrationDwellSec: next }, on)}
          />
        </span>
      </div>

      <div className={`${s.field} ${s.fieldRow} ${s.rowSub}`}>
        <div className={s.fieldText}>
          <div className={s.fieldTitle}>
            {tr('settings.calib.xyDwell')}
            <Tip text={tr('settings.calib.xyDwellNote', { min: xyDwell.min, max: xyDwell.max })} />
          </div>
        </div>
        <span className={s.numBox}>
          <NumberField
            label={tr('settings.calib.xyDwell')}
            unit={tr('settings.calib.seconds')}
            value={r.xyCalibrationDwellSec}
            min={xyDwell.min}
            max={xyDwell.max}
            step={1}
            onChange={(next) => void set({ xyCalibrationDwellSec: next }, on)}
          />
        </span>
      </div>

      {err !== null && <p className={`${s.note} ${s.noteBad}`}>{err}</p>}
    </section>
  )
}
