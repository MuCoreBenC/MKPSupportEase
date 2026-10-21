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
  const { zDwell, xyDwell, zLift } = prefs.limits

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
            在 G-code 处理后开头插入工具头收起序列（先 Z 归位、抬到设定高度、再收笔），
            确保打印开始前工具头已卸载
          </p>
        </div>
        <Switch
          checked={r.beginStageRetractEnabled}
          ariaLabel="开始阶段收笔"
          onChange={(next) => void set({ beginStageRetractEnabled: next }, on)}
        />
      </div>

      {/* 只在收笔开着时才问抬多高：关着的时候配一个不生效的数等于骗人
          （与热床涂胶那一块"只在开着时才问涂哪儿"同一条规矩） */}
      {r.beginStageRetractEnabled && (
        <div className={`${s.field} ${s.fieldRow}`}>
          <div className={s.fieldText}>
            <div className={s.fieldTitle}>收笔抬升高度</div>
            <p className={s.fieldNote}>
              「开始阶段收笔」把工具头抬到的绝对 Z 高度（{zLift.min}-{zLift.max} mm）。
              太高只是一段白跑的空行程，太低则可能与模型或夹具相碰
            </p>
          </div>
          <span className={s.numBox}>
            <NumberField
              label="收笔抬升高度"
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
