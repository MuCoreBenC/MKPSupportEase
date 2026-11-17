/*
 * 「恢复默认」——整页最后一个动作。
 *
 * # 它恢复的是哪一份
 *
 * 后处理那 11 个偏好全部置回 `null`（= 没设过）。后端 `resolved()` 拿 `null` 去读
 * **出厂值**，而那份出厂值现在是可配的（工作台「默认配置」页写
 * `presets/client-defaults.toml`）。所以这一颗是「老用户套用新默认值」的入口：
 * 默认值只影响新装，已经装好的得自己点一下。
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
import { usePostprocPrefs } from '../state/usePostprocPrefs'
import { Btn } from '../ui/Controls'
import ts from '../presets/TextDiffModal.module.css'
import { lineDiff } from '../presets/textDiff'
import { SectionHead } from './SectionHead'
import s from './PageSettings.module.css'

/**
 * 「恢复默认」的那一份补丁：全给 `null`，由后端按出厂重算。
 *
 * 键与 `postproc::ir::PrefsIr` 一一对应；漏一个就是「那一项恢复不了」。
 */
const RESET_PATCH = {
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

/** 对比表里的 11 行（顺序与设置页出现顺序一致） */
const ITEMS: { key: keyof PostprocPrefsView['factory']; label: string }[] = [
  { key: 'skipVibrationCalibration', label: '跳过震动校准检测' },
  { key: 'beginStageRetractEnabled', label: '开始阶段收笔' },
  { key: 'beginStageRetractZLiftMm', label: '收笔抬升高度' },
  { key: 'zCalibrationDwellSec', label: 'Z 轴校准停留时间' },
  { key: 'xyCalibrationDwellSec', label: 'XY 轴校准停留时间' },
  { key: 'underSupportEnabled', label: '启用下方支撑余量' },
  { key: 'underSupportMm', label: '下方支撑余量阈值' },
  { key: 'glueBottomEnabled', label: '支撑面下方一层也涂胶' },
  { key: 'bedGlueEnabled', label: '启用热床预涂胶' },
  { key: 'bedGlueAllModels', label: '热床涂胶覆盖所有模型' },
  { key: 'autoTowerPosition', label: '智能塔位置' },
]

const TOWER_LABEL: Record<string, string> = { ask: '问一句', auto: '自动挪塔', off: '不挪' }

/** 一格的值怎么写成一行 */
function valueText(v: boolean | number | string | null | undefined, dash: string): string {
  if (v === null || v === undefined) return dash
  if (typeof v === 'boolean') return v ? '开' : '关'
  if (typeof v === 'number') return Number.isInteger(v) ? String(v) : v.toFixed(2)
  if (typeof v === 'string') return TOWER_LABEL[v] ?? v
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
    const left = ITEMS.map((it) => `${it.label} = ${valueText(prefs.resolved[it.key], dash)}`).join('\n')
    const right = ITEMS.map((it) => `${it.label} = ${valueText(prefs.factory[it.key], dash)}`).join('\n')
    return lineDiff(left, right)
  }, [prefs, dash])

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
