/*
 * 设置页「校准与补偿」那一块（2026-10-17）。
 *
 * 四行：跳过震动校准检测 / 开始阶段收笔 / Z 轴停留时间 / XY 轴停留时间。
 *
 * ★★ 这四行**真的改变切出来的 G-code**（2026-10-17 之前它们是写死在管线里的常量，
 * 设置页改不动）。现在它们经 `runtime::postproc_prefs::apply_to_ir` 写进 `ir.prefs`，
 * 由管线读。所以这里的说明文字必须**说清后果** —— 用户要知道打开它会发生什么。
 *
 * ★ 数字框的量程来自后端（`prefs.limits`），**不在这里另写一份**：
 * 两处各写一份的话，迟早会出现"界面允许 30 秒、管线只认到 20 秒"这种漂移。
 */

import { useState } from 'react'

import { usePostprocPrefs } from '../state/usePostprocPrefs'
import NumberField from '../../components/field/NumberField'
import { Switch } from '../../components/switch'
import s from './PageSettings.module.css'

export default function CalibSection() {
  const { prefs, set } = usePostprocPrefs()
  const [err, setErr] = useState<string | null>(null)

  const on = (why: string) => setErr(why)

  if (prefs === null) {
    return (
      <section className={s.section} aria-label="校准与补偿">
        <h2 className={s.secTitle}>校准与补偿</h2>
        <p className={s.fieldNote}>正在读取…</p>
      </section>
    )
  }

  const r = prefs.resolved
  const { zDwell, xyDwell } = prefs.limits

  return (
    <section className={s.section} aria-label="校准与补偿">
      <h2 className={s.secTitle}>校准与补偿</h2>

      <div className={`${s.field} ${s.fieldRow}`}>
        <div className={s.fieldText}>
          <div className={s.fieldTitle}>跳过震动校准检测</div>
          <p className={s.fieldNote}>
            跳过 G-code 中的震动补偿校准检测步骤（M970/M974），使用打印机已保存的校准数据，
            减少打印准备时间
          </p>
        </div>
        <Switch
          checked={r.skipVibrationCalibration}
          ariaLabel="跳过震动校准检测"
          onChange={(next) => void set({ skipVibrationCalibration: next }, on)}
        />
      </div>

      <div className={`${s.field} ${s.fieldRow}`}>
        <div className={s.fieldText}>
          <div className={s.fieldTitle}>开始阶段收笔</div>
          <p className={s.fieldNote}>
            在 G-code 处理后开头插入工具头收起序列（抬升 Z{' '}
            {r.beginStageRetractZLiftMm}mm 并收笔），确保打印开始前工具头已卸载
          </p>
        </div>
        <Switch
          checked={r.beginStageRetractEnabled}
          ariaLabel="开始阶段收笔"
          onChange={(next) => void set({ beginStageRetractEnabled: next }, on)}
        />
      </div>

      <div className={`${s.field} ${s.fieldRow}`}>
        <div className={s.fieldText}>
          <div className={s.fieldTitle}>Z 轴校准停留时间</div>
          <p className={s.fieldNote}>
            Z 校准每一层测量后的暂停时间（{zDwell.min}-{zDwell.max} 秒）
          </p>
        </div>
        <span className={s.numBox}>
          <NumberField
            label="Z 轴校准停留时间"
            unit="秒"
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
          <div className={s.fieldTitle}>XY 轴校准停留时间</div>
          <p className={s.fieldNote}>
            XY 校准每一条线测量后的暂停时间（{xyDwell.min}-{xyDwell.max} 秒）
          </p>
        </div>
        <span className={s.numBox}>
          <NumberField
            label="XY 轴校准停留时间"
            unit="秒"
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
