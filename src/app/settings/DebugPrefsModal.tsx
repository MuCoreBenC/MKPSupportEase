import { Fragment, useRef, useState } from 'react'
import { createPortal } from 'react-dom'

import Modal from '../../components/modal/Modal'
import NumberField from '../../components/field/NumberField'
import { Switch } from '../../components/switch'
import { Tip } from '../../components/tooltip'
import { useModalHeadSlot } from '../../components/modal/modalHeadSlot'
import { Btn } from '../ui/Controls'
import { useT } from '../../i18n'
import { usePostprocPrefs } from '../state/usePostprocPrefs'
import type { ResolvedPostprocPrefs } from '../../api/contract'
import {
  FACTORY_ANCHORS,
  fmt,
  loadPresets,
  removePreset,
  savePreset,
  summarize,
  type DebugPreset,
} from './debugPresets'
import s from './PageSettings.module.css'

/**
 * 调试模式的设置窗。
 *
 * ★ 尺寸 `size="lg"`：`sm` / `md` 的框身是 `overflow: visible`，内容一高就被
 *   `.box` 裁掉。`lg` 的框身本来就会滚。
 *
 * ★ 布局：左栏 = 开关 + 代码预览；右栏 = 输入框。代码块 `flex: 1 1 0` 吃掉
 *   左下空白，且高度不随内容变（开关一开一关框不跳）。
 *
 * ★ 存档（2026-11-11 作者）：「点一下保存就记一条，方便我用不同的」。
 *   入口在标题栏 `×` 左边那颗「记录」，点开**只换右栏**—— 左栏的代码预览留着，
 *   点某一条就能立刻看到它会生成什么。
 */
export default function DebugPrefsModal({ open, onClose }: { open: boolean; onClose: () => void }) {
  const tr = useT()
  const { prefs, set } = usePostprocPrefs()
  const [panel, setPanel] = useState<'edit' | 'list' | 'anchors'>('edit')
  const [presets, setPresets] = useState<DebugPreset[]>(() => loadPresets())
  /* 打开那一刻的样子：「放弃」按它还原 */
  const snapshot = useRef<ResolvedPostprocPrefs | null>(null)
  const editorRef = useRef<HTMLTextAreaElement | null>(null)

  const r = prefs?.resolved
  if (r === undefined || r === null) return null

  /* 每次从关到开都重新记一份（关着的时候 prefs 还在变，不能只记第一次） */
  if (open && snapshot.current === null) snapshot.current = r
  if (!open && snapshot.current !== null) snapshot.current = null

  /* 高级模式那段代码的毛病（空 = 没问题）：只认 G / M / L / 注释 / 空行 */
  const codeIssue = r.debugCustomGcode === null ? null : firstBadLine(r.debugCustomGcode)

  const on = r.debugModeEnabled
  const yOn = on && r.debugMoveY

  /** 把一条存档套回当前参数（只改界面上的值，仍然要按「完成」才离开） */
  const applyPreset = (p: DebugPreset) => {
    void set(
      {
        debugWaitX: p.waitX,
        debugWaitY: p.waitY,
        debugMountX: p.mountX,
        debugMountY: p.mountY,
        debugUnmountX: p.unmountX,
        debugUnmountY: p.unmountY,
        debugSpeed: p.speed,
        debugMoveY: p.moveY,
      },
      () => {},
    )
  }

  const keep = () => {
    setPresets(
      savePreset({
        waitX: r.debugWaitX,
        waitY: r.debugWaitY,
        mountX: r.debugMountX,
        mountY: r.debugMountY,
        unmountX: r.debugUnmountX,
        unmountY: r.debugUnmountY,
        speed: r.debugSpeed,
        moveY: r.debugMoveY,
      }),
    )
    setPanel('list')
  }

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
          {/* 校验没过时把毛病摆在这儿；没事就摆那句说明 */}
          <span className={codeIssue === null ? s.footNote : `${s.footNote} ${s.footWarn}`}>
            {codeIssue ?? tr('settings.calib.debugKeepNote')}
          </span>
          {panel === 'list' ? (
            <Btn onClick={() => setPanel('edit')}>{tr('settings.calib.debugBack')}</Btn>
          ) : (
            <>
              <Btn
                onClick={() => {
                  /* 放弃 = 还原到打开那一刻，然后关掉 */
                  if (snapshot.current !== null) void set(snapshot.current, () => {})
                  onClose()
                }}
              >
                {tr('settings.calib.debugDiscard')}
              </Btn>
              <Btn
                variant="primary"
                disabled={codeIssue !== null}
                onClick={() => {
                  /* 保存 = 记一条存档再关（作者：保存就直接当那条记录） */
                  keep()
                  onClose()
                }}
              >
                {tr('settings.calib.debugSave')}
              </Btn>
            </>
          )}
        </>
      }
    >
      <HeadRecordsButton
        count={presets.length}
        active={panel === 'list'}
        onClick={() => setPanel(panel === 'list' ? 'edit' : 'list')}
      />
      <HeadAnchorsButton
        active={panel === 'anchors'}
        onClick={() => setPanel(panel === 'anchors' ? 'edit' : 'anchors')}
      />

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
          <SwitchRow
            label={tr('settings.calib.debugAdvanced')}
            tip={tr('settings.calib.debugAdvancedNote')}
            checked={r.debugCustomGcode !== null}
            disabled={!on}
            onChange={(next) =>
              void set(
                { debugCustomGcode: next ? previewLines(r).join('\n') : null },
                () => {},
              )
            }
          />
          <div className={s.resetRow}>
            <button
              type="button"
              className={s.textBtn}
              onClick={() => void set(RESET_PATCH, () => {})}
            >
              {tr('settings.calib.debugReset')}
            </button>
          </div>
          {panel === 'anchors' ? (
            <AnchorTable />
          ) : r.debugCustomGcode !== null ? (
            /* 高级模式：整段自己写。打开那一刻已经把当前生成的那份灌进来，改的是它 */
            <>
              <textarea
                ref={editorRef}
                className={s.codeEditor}
                spellCheck={false}
                value={r.debugCustomGcode}
                onChange={(e) => void set({ debugCustomGcode: e.target.value }, () => {})}
              />
              <div className={s.snippetBar}>
                <span className={s.snippetLabel}>{tr('settings.calib.debugInsert')}</span>
                {([
                  { key: 'settings.calib.debugSnipWait3', code: 'G4 S3' },
                  { key: 'settings.calib.debugSnipWait5', code: 'G4 S5' },
                ] as const).map((snip) => (
                  <button
                    key={snip.key}
                    type="button"
                    className={s.snippetBtn}
                    onClick={() => insertSnippet(editorRef, r, snip.code, set)}
                  >
                    {tr(snip.key)}
                  </button>
                ))}
                <button
                  type="button"
                  className={s.snippetBtn}
                  onClick={() => insertSnippet(editorRef, r, backToWait(r), set)}
                >
                  {tr('settings.calib.debugSnipPark')}
                </button>
              </div>
            </>
          ) : (
            <CodePreview r={r} />
          )}
        </div>

        <div className={s.debugParams}>
          {panel === 'list' ? (
            <PresetList
              presets={presets}
              onUse={applyPreset}
              onDelete={(id) => setPresets(removePreset(id))}
              onKeep={keep}
            />
          ) : (
            <>
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
                label={tr('settings.calib.debugRepeat')}
                unit={tr('settings.calib.debugRepeatUnit')}
                value={r.debugRepeat}
                min={1}
                max={5}
                step={1}
                disabled={!on}
                onChange={(next) => void set({ debugRepeat: Math.round(next) }, () => {})}
              />
              <SwitchRow
                label={tr('settings.calib.debugUseTravel')}
                tip={tr('settings.calib.debugUseTravelNote')}
                checked={r.debugUseTravelSpeed}
                disabled={!on}
                onChange={(next) => void set({ debugUseTravelSpeed: next }, () => {})}
              />
              <NumRow
                label={tr('settings.calib.debugTravelSpeed')}
                unit="mm/min"
                value={r.debugTravelSpeed}
                min={100}
                max={60000}
                step={100}
                disabled={!on || !r.debugUseTravelSpeed}
                onChange={(next) => void set({ debugTravelSpeed: next }, () => {})}
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
            </>
          )}
        </div>
      </div>
    </Modal>
  )
}

/**
 * 标题栏 `×` 左边那颗「记录」。
 *
 * 借的是项目现成的**动作槽**（`modalHeadSlot.ts`，「修改历史」就住那儿）——
 * 不自己往标题里塞一个绝对定位的按钮。
 */
function HeadRecordsButton({
  count,
  active,
  onClick,
}: {
  count: number
  active: boolean
  onClick: () => void
}) {
  const tr = useT()
  const slot = useModalHeadSlot()
  const node = (
    <button
      type="button"
      className={active ? `${s.headBtn} ${s.headBtnOn}` : s.headBtn}
      aria-pressed={active}
      onClick={onClick}
    >
      {tr('settings.calib.debugRecords')}
      {count > 0 && <span className={s.headCount}>{count}</span>}
    </button>
  )
  return slot === null ? node : createPortal(node, slot)
}

/** 标题栏「默认点位」：把**左栏**从代码切成点位表（右栏仍是输入框，方便照着改） */
function HeadAnchorsButton({ active, onClick }: { active: boolean; onClick: () => void }) {
  const tr = useT()
  const slot = useModalHeadSlot()
  const node = (
    <button
      type="button"
      className={active ? `${s.headBtn} ${s.headBtnOn}` : s.headBtn}
      aria-pressed={active}
      onClick={onClick}
    >
      {tr('settings.calib.debugAnchors')}
    </button>
  )
  return slot === null ? node : createPortal(node, slot)
}

/** 左栏的「出厂点位」参照表：点「默认点位」才出来，默认位置是代码预览 */
function AnchorTable() {
  const tr = useT()
  return (
    <div className={s.anchorWrap}>
      <p className={s.anchorLegend}>{tr('settings.calib.debugAnchorLegend')}</p>
      <div className={s.anchorRow}>
        <span className={s.anchorHead}>{tr('settings.calib.debugAnchorVersion')}</span>
        <span className={s.anchorHead}>{tr('settings.calib.debugMountPath')}</span>
        <span className={s.anchorHead}>{tr('settings.calib.debugUnmountPath')}</span>
      </div>
      {FACTORY_ANCHORS.map((g) => (
        <Fragment key={g.machine}>
          {/* 组头：机型只写这一次，右边的「开始收笔」按机型，组内各版本相同 */}
          <div className={s.anchorGroup}>
            <span className={s.anchorGroupName}>{g.machine}</span>
            <span className={s.anchorGroupNote}>
              {tr('settings.calib.debugAnchorStart')} {g.startRetract}
            </span>
          </div>
          {g.rows.map((row) => (
            <div key={row.version || 'base'} className={s.anchorRow}>
              <span className={s.anchorVersion}>{row.version || '—'}</span>
              <span className={s.anchorNum}>{row.mountPath}</span>
              <span className={s.anchorNum}>{row.unmountPath}</span>
            </div>
          ))}
        </Fragment>
      ))}
      <p className={s.presetHint}>{tr('settings.calib.debugAnchorNote')}</p>
    </div>
  )
}

/** 右栏换成存档列表：点一条就套用，左边代码立刻变 */
function PresetList({
  presets,
  onUse,
  onDelete,
  onKeep,
}: {
  presets: DebugPreset[]
  onUse: (p: DebugPreset) => void
  onDelete: (id: string) => void
  onKeep: () => void
}) {
  const tr = useT()
  if (presets.length === 0) {
    return (
      <div className={s.presetEmpty}>
        <p>{tr('settings.calib.debugNoPresets')}</p>
        <Btn onClick={onKeep}>{tr('settings.calib.debugSavePreset')}</Btn>
      </div>
    )
  }
  return (
    <>
      <div className={s.presetList}>
        {presets.map((p) => (
          <div key={p.id} className={s.presetRow}>
            <button type="button" className={s.presetUse} onClick={() => onUse(p)}>
              <span className={s.presetWhen}>{when(p.at)}</span>
              <span className={s.presetWhat}>{summarize(p)}</span>
            </button>
            <button
              type="button"
              className={s.presetDel}
              aria-label={tr('settings.calib.debugDeletePreset')}
              onClick={() => onDelete(p.id)}
            >
              ✕
            </button>
          </div>
        ))}
      </div>
      <p className={s.presetHint}>{tr('settings.calib.debugPresetLimit')}</p>
    </>
  )
}

/** `11-11 20:37`：列表里只要能认出是哪一次，不摆秒 */
function when(at: number): string {
  const d = new Date(at)
  const p = (n: number) => String(n).padStart(2, '0')
  return `${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`
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
/**
 * 「恢复默认」的那一份补丁：全给 `null`，由后端按出厂常量重算。
 *
 * 不在这里写死 20 / 267 / 276 —— 那是 P1S 一档的数，写死就等于把
 * 「恢复默认」变成「恢复成 P1S」，A1 用户点一下反而更错。
 */
const RESET_PATCH = {
  debugWaitX: null,
  debugWaitY: null,
  debugMountX: null,
  debugMountY: null,
  debugUnmountX: null,
  debugUnmountY: null,
  debugSpeed: null,
  debugRepeat: null,
  debugTravelSpeed: null,
  debugUseTravelSpeed: null,
  debugMoveY: null,
  debugCustomGcode: null,
} as const

/**
 * 高级模式那段代码的结构毛病；没问题返回 `null`。
 *
 * 只认 `G` / `M` / `L` 开头、分号注释、空行 —— 别的（`X100` 这种漏了指令名、
 * 手抖多打的字）会在产物里变成一行没人认的垃圾，BBS 打开时那一行被忽略，
 * 作者看到的是"动作没执行"，比直接拦下来难查得多。
 */
function firstBadLine(text: string): string | null {
  const lines = text.split('\n')
  for (let i = 0; i < lines.length; i++) {
    const t = lines[i].trim()
    if (t === '' || t.startsWith(';')) continue
    if (!/^[GML]/i.test(t)) return `第 ${i + 1} 行不认识：${t.slice(0, 24)}`
  }
  return null
}

/** 在光标处插入一段；没有光标（还没点过）就接在末尾 */
function insertSnippet(
  ref: React.RefObject<HTMLTextAreaElement | null>,
  r: ResolvedPostprocPrefs,
  code: string,
  set: ReturnType<typeof usePostprocPrefs>['set'],
): void {
  const el = ref.current
  const cur = r.debugCustomGcode ?? ''
  if (el === null) {
    void set({ debugCustomGcode: `${cur}\n${code}` }, () => {})
    return
  }
  const at = el.selectionStart ?? cur.length
  const next = `${cur.slice(0, at)}${code}${cur.slice(el.selectionEnd ?? at)}`
  void set({ debugCustomGcode: next }, () => {})
  /* 插完把光标挪到插入内容之后，连着插两段也不会叠在一起 */
  requestAnimationFrame(() => {
    const pos = at + code.length
    el.setSelectionRange(pos, pos)
    el.focus()
  })
}

/** 「回驻点」那一条：用当前驻点坐标与驻点速度拼，跟生成器同一口径 */
function backToWait(r: ResolvedPostprocPrefs): string {
  const f = r.debugUseTravelSpeed ? fmt(r.debugTravelSpeed) : fmt(r.debugSpeed)
  const y = r.debugMoveY ? ` Y${fmt(r.debugWaitY)}` : ''
  return `G1 X${fmt(r.debugWaitX)}${y} F${f}`
}

/** 这一段会写进产物的动作（高级模式打开那一刻要拿它当初始文本） */
function previewLines(r: ResolvedPostprocPrefs): string[] {
  /* 与内核的 `num()` 同一口径：整数不带小数点，小数保留原样 */
  const n = (v: number) => fmt(v)
  const my = r.debugMountY ?? r.debugWaitY
  const uy = r.debugUnmountY ?? r.debugWaitY
  const mv = (x: number, y: number) =>
    r.debugMoveY ? `G1 X${n(x)} Y${n(y)} F${n(r.debugSpeed)}` : `G1 X${n(x)} F${n(r.debugSpeed)}`
  /* 到驻点那一档：开了「驻点速度单独调」就用它，否则与落笔同速 */
  const travelF = r.debugUseTravelSpeed ? n(r.debugTravelSpeed) : n(r.debugSpeed)
  const walk = (x: number, y: number) =>
    r.debugMoveY ? `G1 X${n(x)} Y${n(y)} F${travelF}` : `G1 X${n(x)} F${travelF}`

  const lines: string[] = ['G28 X', 'G28 Y', walk(r.debugWaitX, r.debugWaitY)]
  /* 与内核同一个循环：落笔 → 驻点 → 收笔 → 驻点，跑 `repeat` 遍 */
  for (let i = 0; i < Math.max(1, r.debugRepeat); i++) {
    lines.push(mv(r.debugMountX, my))
    lines.push(walk(r.debugWaitX, r.debugWaitY))
    lines.push(mv(r.debugUnmountX, uy))
    lines.push(walk(r.debugWaitX, r.debugWaitY))
  }
  return lines
}

function CodePreview({ r }: { r: ResolvedPostprocPrefs }) {
  const tr = useT()
  const lines = r.debugModeEnabled
    ? previewLines(r)
    : [tr('settings.calib.debugPreviewOff')]

  return (
    /* 不摆标题行：它白占一行，框高还跟着它上下跳 */
    <div className={s.debugCode}>
      <pre className={s.previewCode}>{lines.join('\n')}</pre>
    </div>
  )
}
