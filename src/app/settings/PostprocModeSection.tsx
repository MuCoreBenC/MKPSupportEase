/*
 * 设置页「后处理模式」那一块（2026-10-17）。
 *
 * 四行：跑完发不发系统通知 / 静默还是 GUI / 支撑面底面涂胶 / 重置后处理状态。
 *
 * 组件单独一个文件是有原因的：`PageSettings.tsx` 已经一千多行，再往里塞四块会
 * 变成谁都不敢动的大泥球。这一块只管**自己那四行** —— 偏好读写走
 * `usePostprocPrefs`，重置走一条命令，别的一概不碰。
 */

import { useState, type ReactNode } from 'react'

import { api, errorText } from '../../api'
import { msg, useText, useT, type StringKey, type Text } from '../../i18n'
import { usePostprocPrefs } from '../state/usePostprocPrefs'
import { Switch } from '../../components/switch'
import { Btn } from '../ui/Controls'
import s from './PageSettings.module.css'

/**
 * 静默 / GUI 两档的口径（与后端 `PostprocUiMode` 一一对应，**id 是契约**）。
 *
 * ★ `label` / `hint` 是**字典键**（2026-10-25）：这张表是模块级常量，放句子会被
 *   求值一次冻住 —— 渲染时用 `tr()` 取当前语言那份。
 */
const UI_MODES: { id: 'silent' | 'gui'; label: StringKey; hint: StringKey; icon: ReactNode }[] = [
  {
    id: 'silent',
    label: 'settings.postproc.silent',
    hint: 'settings.postproc.silentHint',
    icon: <SilentIcon />,
  },
  {
    id: 'gui',
    label: 'settings.postproc.gui',
    hint: 'settings.postproc.guiHint',
    icon: <WindowIcon />,
  },
]

export default function PostprocModeSection() {
  const tr = useT()
  const say = useText()
  const { prefs, set } = usePostprocPrefs()
  const [err, setErr] = useState<string | null>(null)
  /** 成功时是后端给的落点原话，失败时是存键的那句 —— 两种都住 [`Text`] */
  const [resetNote, setResetNote] = useState<Text | null>(null)
  const [resetBusy, setResetBusy] = useState(false)

  const on = (why: string) => setErr(why)

  /** 偏好还没读回来：不画开关（画一个"看起来是关的"开关会骗人） */
  if (prefs === null) {
    return (
      <section className={s.section} aria-label={tr('settings.postproc.title')}>
        <h2 className={s.secTitle}>{tr('settings.postproc.title')}</h2>
        <p className={s.fieldNote}>{tr('common.loading')}</p>
      </section>
    )
  }

  const r = prefs.resolved

  const doReset = () => {
    setResetBusy(true)
    setResetNote(null)
    void api.resetPostProcessState().then(
      (line) => {
        setResetNote(line)
        setResetBusy(false)
      },
      (e: unknown) => {
        setResetNote(msg('settings.postproc.resetFailed', { detail: errorText(e) }))
        setResetBusy(false)
      },
    )
  }

  return (
    <section className={s.section} aria-label={tr('settings.postproc.title')}>
      <h2 className={s.secTitle}>{tr('settings.postproc.title')}</h2>

      <div className={`${s.field} ${s.fieldRow}`}>
        <div className={s.fieldText}>
          <div className={s.fieldTitle}>{tr('settings.postproc.notify')}</div>
          <p className={s.fieldNote}>{tr('settings.postproc.notifyNote')}</p>
        </div>
        <Switch
          checked={r.notifyOnFinish}
          ariaLabel={tr('settings.postproc.notify')}
          onChange={(next) => void set({ notifyOnFinish: next }, on)}
        />
      </div>

      <div className={s.field}>
        <div className={s.fieldTitle}>{tr('settings.postproc.mode')}</div>
        <p className={s.fieldNote}>{tr('settings.postproc.modeNote')}</p>

        <div className={s.modeGrid} role="radiogroup" aria-label={tr('settings.postproc.modeAria')}>
          {UI_MODES.map((m) => {
            const picked = r.uiMode === m.id
            return (
              <button
                key={m.id}
                type="button"
                role="radio"
                aria-checked={picked}
                className={`${s.modeCard} ${picked ? s.modeCardOn : ''}`}
                onClick={() => void set({ uiMode: m.id }, on)}
              >
                <span className={s.modeIcon} aria-hidden="true">
                  {m.icon}
                </span>
                <span className={s.modeLabel}>{tr(m.label)}</span>
                <span className={s.modeHint}>{tr(m.hint)}</span>
              </button>
            )
          })}
        </div>
      </div>

      <div className={`${s.field} ${s.fieldRow}`}>
        <div className={s.fieldText}>
          <div className={s.fieldTitle}>{tr('settings.postproc.glueBottom')}</div>
          <p className={s.fieldNote}>{tr('settings.postproc.glueBottomNote')}</p>
        </div>
        <Switch
          checked={r.glueBottomEnabled}
          ariaLabel={tr('settings.postproc.glueBottom')}
          onChange={(next) => void set({ glueBottomEnabled: next }, on)}
        />
      </div>

      <div className={`${s.field} ${s.fieldRow}`}>
        <div className={s.fieldText}>
          <div className={s.fieldTitle}>{tr('settings.postproc.tower')}</div>
          <p className={s.fieldNote}>{tr('settings.postproc.towerNote')}</p>
        </div>
        <Switch
          checked={r.autoTowerPosition}
          ariaLabel={tr('settings.postproc.tower')}
          onChange={(next) => void set({ autoTowerPosition: next }, on)}
        />
      </div>

      <div className={`${s.field} ${s.fieldRow}`}>
        <div className={s.fieldText}>
          <div className={s.fieldTitle}>{tr('settings.postproc.reset')}</div>
          <p className={s.fieldNote}>{tr('settings.postproc.resetNote')}</p>
        </div>
        <Btn variant="danger" disabled={resetBusy} onClick={doReset}>
          {resetBusy ? tr('settings.postproc.resetting') : tr('settings.postproc.resetAction')}
        </Btn>
      </div>

      {resetNote !== null && <p className={s.note}>{say(resetNote)}</p>}
      {err !== null && <p className={`${s.note} ${s.noteBad}`}>{err}</p>}
    </section>
  )
}

/* ---- 两枚小图标（12px，只描边不填充，与这一页的安静程度一致） ---- */

function SilentIcon() {
  return (
    <svg viewBox="0 0 16 16" width="16" height="16" fill="none" aria-hidden="true">
      <path
        d="M2.5 6v4a.5.5 0 0 0 .5.5h1.6L8 13V3L4.6 5.5H3a.5.5 0 0 0-.5.5Z"
        stroke="currentColor"
        strokeWidth="1.2"
        strokeLinejoin="round"
      />
      <path d="m11 6 3 4m0-4-3 4" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
    </svg>
  )
}

function WindowIcon() {
  return (
    <svg viewBox="0 0 16 16" width="16" height="16" fill="none" aria-hidden="true">
      <rect x="2" y="3.2" width="12" height="9.6" rx="1.4" stroke="currentColor" strokeWidth="1.2" />
      <path d="M2 6.2h12" stroke="currentColor" strokeWidth="1.2" />
      <circle cx="4.2" cy="4.7" r="0.55" fill="currentColor" />
    </svg>
  )
}
