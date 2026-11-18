/*
 * 「恢复默认」——整页最后一个动作。
 *
 * # 它恢复的是哪一份
 *
 * 后处理那 15 个偏好（`[Prefs]` 的 11 项 + 通知 / 显示方式 / 置顶 / 强制校准）全部置回
 * `null`（= 没设过）。后端 `resolved()` 拿 `null` 去读**出厂值**，而那份出厂值现在是可配的
 * （工作台「默认配置」页写 `presets/client-defaults.toml`）。所以这一颗是「老用户套用新默认值」
 * 的入口：默认值只影响新装，已经装好的得自己点一下。
 *
 * ★ 不写死任何数：写死就等于把「恢复默认」变成「恢复成上一版」，与
 *   `DebugPrefsModal` 那颗的取舍同一条理。
 *
 * # 为什么先弹框
 *
 * 它一次改满屏开关，点下去之前得让人看见**到底哪几项会变**。框里用的是
 * `lineDiff` 那张左右对比（与预设的「与默认值对比」同一个算法、同一套红绿）：
 * 左边是盘上现在显式设过的值，右边一律「出厂」—— 红的就是会被去掉的那几条。
 *
 * # 为什么在页面最后
 *
 * 它不属于任何一组（一次改完 11 项），挂在「后处理」组里会像那一组的附属开关。
 */

import { useMemo, useState } from 'react'

import type { PostprocPrefsView } from '../../api/contract'
import { Modal } from '../../components/modal'
import { Tip } from '../../components/tooltip'
import { useT } from '../../i18n'
import type { StringKey, Tr } from '../../i18n'
import { usePostprocPrefs } from '../state/usePostprocPrefs'
import { Btn } from '../ui/Controls'
import ts from '../presets/TextDiffModal.module.css'
import { lineDiff } from '../presets/textDiff'
import { SectionHead } from './SectionHead'
import s from './PageSettings.module.css'

/**
 * 「恢复默认」的那一份补丁：全给 `null`，由后端按出厂重算。
 *
 * 键 = 「有出厂值的那 15 项」（`[Prefs]` 的 11 项 + 通知 / 显示方式 / 置顶 / 强制校准）；
 * 漏一个就是「那一项恢复不了」。
 */
const RESET_PATCH = {
  notifyOnFinish: null,
  uiMode: null,
  autoTopOnAsk: null,
  forceCalibrationMode: null,
  glueBottomEnabled: null,
  underSupportEnabled: null,
  underSupportMm: null,
  skipVibrationCalibration: null,
  beginStageRetractEnabled: null,
  beginStageRetractZLiftMm: null,
  zCalibrationDwellSec: null,
  xyCalibrationDwellSec: null,
  bedGlueEnabled: null,
  bedGlueAllModels: null,
  autoTowerPosition: null,
} as const

/** 对比表里的 15 行（顺序与设置页出现顺序一致；标签走文案表，与设置页同一句） */
const ITEMS: { key: keyof PostprocPrefsView['factory']; label: StringKey }[] = [
  { key: 'notifyOnFinish', label: 'settings.postproc.notify' },
  { key: 'uiMode', label: 'settings.postproc.mode' },
  { key: 'autoTopOnAsk', label: 'settings.postproc.topOnAsk' },
  { key: 'forceCalibrationMode', label: 'settings.calib.forceCalib' },
  { key: 'skipVibrationCalibration', label: 'settings.calib.skipVibration' },
  { key: 'beginStageRetractEnabled', label: 'settings.calib.beginRetract' },
  { key: 'beginStageRetractZLiftMm', label: 'settings.calib.lift' },
  { key: 'zCalibrationDwellSec', label: 'settings.calib.zDwell' },
  { key: 'xyCalibrationDwellSec', label: 'settings.calib.xyDwell' },
  { key: 'underSupportEnabled', label: 'settings.under.enable' },
  { key: 'underSupportMm', label: 'settings.under.margin' },
  { key: 'glueBottomEnabled', label: 'settings.postproc.glueBottom' },
  { key: 'bedGlueEnabled', label: 'settings.glue.enable' },
  { key: 'bedGlueAllModels', label: 'settings.glue.allModels' },
  { key: 'autoTowerPosition', label: 'settings.postproc.tower' },
]

/** 三档的说法与设置页那一排**同一套字**（`settings.postproc.towerMode*`），不另写一份 */
const TOWER_LABEL: Record<string, StringKey> = {
  ask: 'settings.postproc.towerModeAsk',
  auto: 'settings.postproc.towerModeAuto',
  off: 'settings.postproc.towerModeOff',
}

/** `uiMode` 两档同上（`settings.postproc.silent` / `gui`） */
const UI_MODE_LABEL: Record<string, StringKey> = {
  silent: 'settings.postproc.silent',
  gui: 'settings.postproc.gui',
}

/** 一格的值怎么写成一行 */
function valueText(tr: Tr, v: boolean | number | string | null | undefined, dash: string): string {
  if (v === null || v === undefined) return dash
  if (typeof v === 'boolean') return v ? '开' : '关'
  if (typeof v === 'number') return Number.isInteger(v) ? String(v) : v.toFixed(2)
  if (typeof v === 'string') {
    const key = TOWER_LABEL[v] ?? UI_MODE_LABEL[v]
    return key === undefined ? v : tr(key)
  }
  return String(v)
}

export default function ResetPostprocDefaults({ host }: { host: HTMLElement | null }) {
  const tr = useT()
  const { prefs, set } = usePostprocPrefs()
  const [open, setOpen] = useState(false)
  const [err, setErr] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  /* 值理论上不会缺；缺了给个短横，不编一个「出厂」糊过去 */
  const dash = '—'

  /*
   * 两段正文喂给同一个 `lineDiff`：左边是盘上现在显式设过的值，右边一律出厂。
   * 于是「没设过」的项两边一样（灰的），「设过」的项是 change（红绿并排）。
   */
  const rows = useMemo(() => {
    if (prefs === null) return []
    const left = ITEMS.map(
      (it) => `${tr(it.label)} = ${valueText(tr, prefs.resolved[it.key], dash)}`,
    ).join('\n')
    const right = ITEMS.map(
      (it) => `${tr(it.label)} = ${valueText(tr, prefs.factory[it.key], dash)}`,
    ).join('\n')
    return lineDiff(left, right)
  }, [prefs, dash, tr])

  const changed = rows.filter((r) => r.kind !== 'same').length

  const doReset = (): void => {
    setBusy(true)
    setErr(null)
    void set(RESET_PATCH, setErr).then(() => {
      setBusy(false)
      setOpen(false)
    })
  }

  return (
    <div className={s.section}>
      <section aria-label={tr('settings.resetDefaults.title')}>
        <SectionHead
          icon="target"
          level={2}
          title={tr('settings.resetDefaults.title')}
          desc={tr('settings.resetDefaults.desc')}
        />

        <div className={`${s.field} ${s.fieldRow}`}>
          <div className={s.fieldText}>
            <div className={s.fieldTitle}>
              {tr('settings.resetDefaults.row')}
              <Tip text={tr('settings.resetDefaults.rowNote')} />
            </div>
          </div>
          <Btn
            variant="default"
            disabled={prefs === null || busy}
            onClick={() => {
              setErr(null)
              setOpen(true)
            }}
          >
            {tr('settings.resetDefaults.action')}
          </Btn>
        </div>

        {err !== null && <p className={`${s.note} ${s.noteBad}`}>{err}</p>}
      </section>

      <Modal
        open={open}
        host={host}
        size="lg"
        title={tr('settings.resetDefaults.modalTitle')}
        subtitle={tr('settings.resetDefaults.modalDesc')}
        onClose={() => setOpen(false)}
        footer={
          <>
            <span className={ts.footLeft} />
            <span className={ts.footActs}>
              <Btn variant="ghost" disabled={busy} onClick={() => setOpen(false)}>
                {tr('settings.resetDefaults.cancel')}
              </Btn>
              <Btn variant="primary" disabled={busy || changed === 0} onClick={doReset}>
                {busy ? tr('settings.resetDefaults.busy') : tr('settings.resetDefaults.action')}
              </Btn>
            </span>
          </>
        }
      >
        {prefs === null ? (
          <p className={s.fieldNote}>{tr('common.loading')}</p>
        ) : changed === 0 ? (
          <p className={s.fieldNote}>{tr('settings.resetDefaults.empty')}</p>
        ) : (
          <div className={ts.diff} role="table" aria-label={tr('settings.resetDefaults.modalTitle')}>
            {rows.map((r, i) => (
              <div key={`${r.leftNo ?? 'x'}-${r.rightNo ?? 'y'}-${i}`} className={ts.row} data-kind={r.kind}>
                <span className={ts.no}>{r.leftNo ?? ''}</span>
                <span className={ts.code} data-side="left">
                  {r.left ?? ''}
                </span>
                <span className={ts.no}>{r.rightNo ?? ''}</span>
                <span className={ts.code} data-side="right">
                  {r.right ?? ''}
                </span>
              </div>
            ))}
          </div>
        )}
      </Modal>
    </div>
  )
}
