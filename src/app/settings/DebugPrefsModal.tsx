import Modal from '../../components/modal/Modal'
import NumberField from '../../components/field/NumberField'
import { Switch } from '../../components/switch'
import { Tip } from '../../components/tooltip'
import { Btn } from '../ui/Controls'
import { useT } from '../../i18n'
import { usePostprocPrefs } from '../state/usePostprocPrefs'
import type { ResolvedPostprocPrefs } from '../../api/contract'
import s from './PageSettings.module.css'

/**
 * 调试模式的设置窗。
 *
 * ★ 尺寸走 `size="lg"`：`sm` / `md` 的框身是 `overflow: visible`，内容一高就被
 *   `.box` 裁掉，既看不全也滚不动。`lg` 档的框身本来就 `overflow-y: auto`。
 *
 * ★ 布局（2026-11-11 作者给的具体排法）：
 *     左栏 = 两个开关 + 代码预览（代码吃掉左下角那一大片空白）
 *     右栏 = 带输入框的那几行
 *   上一版是「左参数 / 右代码」，参数 9 行而代码 7 行短一截，右下角就空出一块。
 *
 * ★ 每个坐标都是 (X, Y)。落笔 / 收笔的 Y **不填就跟驻点**（`null` = 跟随）。
 */
export default function DebugPrefsModal({ open, onClose }: { open: boolean; onClose: () => void }) {
  const tr = useT()
  const { prefs, set } = usePostprocPrefs()
  const r = prefs?.resolved
  if (r === undefined || r === null) return null

  const on = r.debugModeEnabled
  /* Y 轴关着 ⇒ 所有 Y 都不能调（X 照旧） */
  const yOn = on && r.debugMoveY

  return (
    <Modal
      open={open}
      size="lg"
      title={tr('settings.calib.debugTitle')}
      subtitle={tr('settings.calib.debugNote')}
      /* 这扇窗从设置页的卡片里打开：不 portal 出去会被卡片裁掉 */
      host={typeof document === 'undefined' ? null : document.body}
      onClose={onClose}
      footer={
        <>
          <span className={s.footNote}>{tr('settings.calib.debugKeepNote')}</span>
          <Btn variant="primary" onClick={onClose}>
            {tr('settings.calib.debugDone')}
          </Btn>
        </>
      }
    >
      <div className={s.debugGrid}>
        <div className={s.debugLeft}>
          <SwitchRow
            label={tr('settings.calib.debugMode')}
            tip={tr('settings.calib.debugModeNote')}
            checked={on}
            onChange={(next) => void set({ debugModeEnabled: next }, () => {})}
          />
          <SwitchRow
            label={tr('settings.calib.debugMoveY')}
            tip={tr('settings.calib.debugMoveYNote')}
            checked={r.debugMoveY}
            disabled={!on}
            onChange={(next) => void set({ debugMoveY: next }, () => {})}
          />
          <CodePreview r={r} />
        </div>

        <div className={s.debugParams}>
          <NumRow
            label={tr('settings.calib.debugWait')}
            value={r.debugWaitX}
            disabled={!on}
            onChange={(next) => void set({ debugWaitX: next }, () => {})}
          />
          <NumRow
            label={tr('settings.calib.debugWaitY')}
            value={r.debugWaitY}
            disabled={!yOn}
            onChange={(next) => void set({ debugWaitY: next }, () => {})}
          />
          <NumRow
            label={tr('settings.calib.debugMount')}
            value={r.debugMountX}
            disabled={!on}
            onChange={(next) => void set({ debugMountX: next }, () => {})}
          />
          <YRow
            label={tr('settings.calib.debugMountY')}
            followLabel={tr('settings.calib.debugFollowWait')}
            value={r.debugMountY}
            waitY={r.debugWaitY}
            disabled={!yOn}
            onFollow={(follow) =>
              void set({ debugMountY: follow ? null : r.debugWaitY }, () => {})
            }
            onChange={(next) => void set({ debugMountY: next }, () => {})}
          />
          <NumRow
            label={tr('settings.calib.debugUnmount')}
            value={r.debugUnmountX}
            disabled={!on}
            onChange={(next) => void set({ debugUnmountX: next }, () => {})}
          />
          <YRow
            label={tr('settings.calib.debugUnmountY')}
            followLabel={tr('settings.calib.debugFollowWait')}
            value={r.debugUnmountY}
            waitY={r.debugWaitY}
            disabled={!yOn}
            onFollow={(follow) =>
              void set({ debugUnmountY: follow ? null : r.debugWaitY }, () => {})
            }
            onChange={(next) => void set({ debugUnmountY: next }, () => {})}
          />
          <NumRow
            label={tr('settings.calib.debugSpeed')}
            unit="mm/min"
            value={r.debugSpeed}
            min={100}
            max={60000}
            step={100}
            disabled={!on}
            onChange={(next) => void set({ debugSpeed: next }, () => {})}
          />
        </div>
      </div>
    </Modal>
  )
}

/** 窗里的开关行：左标题右开关，行间不画分隔线 */
function SwitchRow({
  label,
  tip,
  checked,
  disabled,
  onChange,
}: {
  label: string
  tip: string
  checked: boolean
  disabled?: boolean
  onChange: (next: boolean) => void
}) {
  return (
    <div className={s.modalRow}>
      <span className={s.modalLabel}>
        {label}
        <Tip text={tip} />
      </span>
      <Switch checked={checked} disabled={disabled} ariaLabel={label} onChange={onChange} />
    </div>
  )
}

/** 窗里的 X 数字行 */
function NumRow({
  label,
  unit = 'mm',
  value,
  min = -20,
  max = 320,
  step = 0.1,
  disabled,
  onChange,
}: {
  label: string
  unit?: string
  value: number
  min?: number
  max?: number
  step?: number
  disabled: boolean
  onChange: (next: number) => void
}) {
  return (
    <div className={s.modalRow}>
      <span className={s.modalLabel}>{label}</span>
      <span className={s.numBox}>
        <NumberField
          label={label}
          unit={unit}
          value={value}
          min={min}
          max={max}
          step={step}
          disabled={disabled}
          onChange={onChange}
        />
      </span>
    </div>
  )
}

/**
 * 可以在「跟随驻点」与「自己填」之间切的那一行 Y。
 *
 * `value === null` 就是跟随：框里显示驻点的 Y 并且灰着。
 * 勾掉之后**从驻点的值起步**，不会一下跳回 0。
 */
function YRow({
  label,
  followLabel,
  value,
  waitY,
  disabled,
  onFollow,
  onChange,
}: {
  label: string
  followLabel: string
  value: number | null
  waitY: number
  disabled: boolean
  onFollow: (follow: boolean) => void
  onChange: (next: number) => void
}) {
  const following = value === null
  return (
    <div className={s.modalRow}>
      <span className={s.modalLabel}>{label}</span>
      <span className={s.rowTail}>
        <label className={s.followCheck}>
          <input
            type="checkbox"
            checked={following}
            disabled={disabled}
            onChange={(e) => onFollow(e.target.checked)}
          />
          {followLabel}
        </label>
        <span className={s.numBox}>
          <NumberField
            label={label}
            unit="mm"
            value={following ? waitY : value}
            min={-20}
            max={320}
            step={0.1}
            disabled={disabled || following}
            onChange={onChange}
          />
        </span>
      </span>
    </div>
  )
}

/**
 * 这趟调试会写进产物的那几行（与 `pipeline/debug.rs` 的 `action_block` 同一套口径）。
 *
 * `G28 Y` **总是**出现，哪怕没开「移动 Y」—— 归零是定基准，不是移动。
 */
function CodePreview({ r }: { r: ResolvedPostprocPrefs }) {
  const tr = useT()
  /* 与内核的 `num()` 同一口径：整数不带小数点，小数保留原样 */
  const n = (v: number) => String(Number(v.toFixed(3)))
  const my = r.debugMountY ?? r.debugWaitY
  const uy = r.debugUnmountY ?? r.debugWaitY
  const mv = (x: number, y: number) =>
    r.debugMoveY ? `G1 X${n(x)} Y${n(y)} F${n(r.debugSpeed)}` : `G1 X${n(x)} F${n(r.debugSpeed)}`

  const lines: string[] = []
  if (r.debugModeEnabled) {
    lines.push('G28 X')
    lines.push('G28 Y')
    lines.push(mv(r.debugWaitX, r.debugWaitY))
    lines.push(mv(r.debugMountX, my))
    lines.push(mv(r.debugWaitX, r.debugWaitY))
    lines.push(mv(r.debugUnmountX, uy))
    lines.push(mv(r.debugWaitX, r.debugWaitY))
  } else {
    lines.push(tr('settings.calib.debugPreviewOff'))
  }

  return (
    /* 不摆「将要写入的代码」那行标题：它白占一行，框高还跟着它上下跳 */
    <div className={s.debugCode}>
      <pre className={s.previewCode}>{lines.join('\n')}</pre>
    </div>
  )
}
