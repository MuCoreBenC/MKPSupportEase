import Modal from '../../components/modal/Modal'
import NumberField from '../../components/field/NumberField'
import { Switch } from '../../components/switch'
import { Tip } from '../../components/tooltip'
import { useT } from '../../i18n'
import { usePostprocPrefs } from '../state/usePostprocPrefs'
import s from './PageSettings.module.css'

/**
 * 调试模式的设置窗（2026-11-11）。
 *
 * 为什么从「设置页一行开关」改成「一颗按钮 + 这扇窗」：调试能调的东西会越加越多
 * （驻点 X/Y、落笔位、收笔位、速度、测不测落笔、Z 要不要归零…）。摊在设置页里
 * 那一屏会被它撑满，而它其实是**偶尔用一次的临时状态**，不该占常驻版面。
 *
 * 外面那行只留开关状态（「已开启 / 未开启」），要看细节、要改数，点进这里。
 */
export default function DebugPrefsModal({ open, onClose }: { open: boolean; onClose: () => void }) {
  const tr = useT()
  const { prefs, set } = usePostprocPrefs()
  const r = prefs?.resolved
  if (r === undefined || r === null) return null

  return (
    <Modal
      open={open}
      title={tr('settings.calib.debugTitle')}
      subtitle={tr('settings.calib.debugNote')}
      onClose={onClose}
      size="sm"
    >
      <div className={s.debugBody}>
        <div className={`${s.field} ${s.fieldRow}`}>
          <div className={s.fieldText}>
            <div className={s.fieldTitle}>
              {tr('settings.calib.debugMode')}
              <Tip text={tr('settings.calib.debugModeNote')} />
            </div>
          </div>
          <Switch
            checked={r.debugModeEnabled}
            ariaLabel={tr('settings.calib.debugMode')}
            onChange={(next) => void set({ debugModeEnabled: next }, () => {})}
          />
        </div>

        {/* 关着的时候不摆那一排不生效的数：改了也不会进产物 */}
        {r.debugModeEnabled && (
          <>
            <NumRow
              label={tr('settings.calib.debugWait')}
              unit="mm"
              value={r.debugWaitX}
              min={-20}
              max={320}
              step={0.1}
              onChange={(next) => void set({ debugWaitX: next }, () => {})}
            />
            <NumRow
              label={tr('settings.calib.debugMount')}
              unit="mm"
              value={r.debugMountX}
              min={-20}
              max={320}
              step={0.1}
              onChange={(next) => void set({ debugMountX: next }, () => {})}
            />
            <NumRow
              label={tr('settings.calib.debugUnmount')}
              unit="mm"
              value={r.debugUnmountX}
              min={-20}
              max={320}
              step={0.1}
              onChange={(next) => void set({ debugUnmountX: next }, () => {})}
            />
            <NumRow
              label={tr('settings.calib.debugSpeed')}
              unit="mm/min"
              value={r.debugSpeed}
              min={100}
              max={60000}
              step={100}
              onChange={(next) => void set({ debugSpeed: next }, () => {})}
            />
            <p className={s.fieldNote}>{tr('settings.calib.debugKeepNote')}</p>
          </>
        )}
      </div>
    </Modal>
  )
}

/** 一行「标题 + 数字框」，与设置页那些行同一套间距 */
function NumRow({
  label,
  unit,
  value,
  min,
  max,
  step,
  onChange,
}: {
  label: string
  unit: string
  value: number
  min: number
  max: number
  step: number
  onChange: (next: number) => void
}) {
  return (
    <div className={`${s.field} ${s.fieldRow} ${s.rowSub}`}>
      <div className={s.fieldText}>
        <div className={s.fieldTitle}>{label}</div>
      </div>
      <span className={s.numBox}>
        <NumberField
          label={label}
          unit={unit}
          value={value}
          min={min}
          max={max}
          step={step}
          onChange={onChange}
        />
      </span>
    </div>
  )
}
