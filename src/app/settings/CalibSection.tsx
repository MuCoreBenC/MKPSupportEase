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
import DebugPrefsModal from './DebugPrefsModal'
import { SectionHead } from './SectionHead'
import s from './PageSettings.module.css'

/**
 * `nested`：挂在「后处理」大类底下（2026-11-09）—— 标题降成小标题（`h3`）、
 * 上下留白收一档，读起来是那个大类里的一段，不是并列的又一章。
 */
export default function CalibSection({ nested = false }: { nested?: boolean }) {
  const tr = useT()
  const { prefs, set } = usePostprocPrefs()
  const [err, setErr] = useState<string | null>(null)
  /** 调试模式那扇窗开着没 */
  const [debugOpen, setDebugOpen] = useState(false)

  const on = (why: string) => setErr(why)

  const head = (
    <SectionHead
      icon="crosshair"
      level={nested ? 3 : 2}
      title={tr('settings.calib.title')}
      desc={tr('settings.calib.desc')}
    />
  )

  if (prefs === null) {
    return (
      <section className={s.section} aria-label={tr('settings.calib.title')}>
        {head}
        <p className={s.fieldNote}>{tr('common.loading')}</p>
      </section>
    )
  }

  const r = prefs.resolved
  const { zDwell, xyDwell, zLift } = prefs.limits

  return (
    <section className={s.section} aria-label={tr('settings.calib.title')}>
      {head}

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
          （与热床涂胶那一块"只在开着时才问涂哪儿"同一条规矩）。

          ★ 缩进的唯一判据（2026-11-09）：**被上面那颗开关放出来的那一格**才缩进。
            这一格是「开始阶段收笔」派生的数字，缩进正对；下面两条停留时间是并列的
            参数，谁也不属于，所以不缩进。 */}
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

      {/* 调试模式：这一行只显示开关状态，点它进那扇窗改细节。
          选项只会越来越多（驻点、落笔位、收笔位、速度、测哪个…），
          摊在设置页会把这一屏撑满，而它是偶尔用一次的临时状态。 */}
      <div className={`${s.field} ${s.fieldRow}`}>
        <div className={s.fieldText}>
          <div className={s.fieldTitle}>
            {tr('settings.calib.debugMode')}
            <Tip text={tr('settings.calib.debugModeNote')} />
          </div>
        </div>
        <button type="button" className={s.textBtn} onClick={() => setDebugOpen(true)}>
          {tr(r.debugModeEnabled ? 'settings.calib.debugOn' : 'settings.calib.debugOff')}
        </button>
      </div>

      {/* `key` 跟着开/关走：每次打开都是新实例，`panel` 自然回到编辑那一屏。
          之前靠 `useEffect` 里 `setPanel('edit')`，要等 prefs 异步回来那一帧才生效，
          作者看到的就是「打开是记录页、点返回没反应、几秒后才好」 */}
      <DebugPrefsModal
        key={debugOpen ? 'open' : 'closed'}
        open={debugOpen}
        onClose={() => setDebugOpen(false)}
      />

      {/* 停留时间是并列的参数（不是哪一行的派生值）⇒ 不缩进 */}
      <div className={`${s.field} ${s.fieldRow}`}>
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

      <div className={`${s.field} ${s.fieldRow}`}>
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
