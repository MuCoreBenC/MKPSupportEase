/*
 * 设置页「后处理模式」那一块（2026-10-17）。
 *
 * 四行：跑完发不发系统通知 / 静默还是 GUI / 支撑面底面涂胶 / 重置后处理状态。
 *
 * 组件单独一个文件是有原因的：`PageSettings.tsx` 已经一千多行，再往里塞四块会
 * 变成谁都不敢动的大泥球。这一块只管**自己那四行** —— 偏好读写走
 * `usePostprocPrefs`，重置走一条命令，别的一概不碰。
 */

import { useState } from 'react'

import { api, errorText } from '../../api'
import { usePostprocPrefs } from '../state/usePostprocPrefs'
import { Switch } from '../../components/switch'
import { Btn } from '../ui/Controls'
import s from './PageSettings.module.css'

/** 静默 / GUI 两档的口径（与后端 `PostprocUiMode` 一一对应，**id 是契约**） */
const UI_MODES = [
  {
    id: 'silent' as const,
    label: '静默',
    hint: '后台静默运行，不显示任何窗口',
    icon: <SilentIcon />,
  },
  {
    id: 'gui' as const,
    label: 'GUI 界面',
    hint: '在独立窗口中显示处理进度和详细信息',
    icon: <WindowIcon />,
  },
]

export default function PostprocModeSection() {
  const { prefs, set } = usePostprocPrefs()
  const [err, setErr] = useState<string | null>(null)
  const [resetNote, setResetNote] = useState<string | null>(null)
  const [resetBusy, setResetBusy] = useState(false)

  const on = (why: string) => setErr(why)

  /** 偏好还没读回来：不画开关（画一个"看起来是关的"开关会骗人） */
  if (prefs === null) {
    return (
      <section className={s.section} aria-label="后处理模式">
        <h2 className={s.secTitle}>后处理模式</h2>
        <p className={s.fieldNote}>正在读取…</p>
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
        setResetNote(`没重置成：${errorText(e)}`)
        setResetBusy(false)
      },
    )
  }

  return (
    <section className={s.section} aria-label="后处理模式">
      <h2 className={s.secTitle}>后处理模式</h2>

      <div className={`${s.field} ${s.fieldRow}`}>
        <div className={s.fieldText}>
          <div className={s.fieldTitle}>处理完成发送系统通知</div>
          <p className={s.fieldNote}>处理完成后发送系统通知，点通知可打开处理结果</p>
        </div>
        <Switch
          checked={r.notifyOnFinish}
          ariaLabel="处理完成发送系统通知"
          onChange={(next) => void set({ notifyOnFinish: next }, on)}
        />
      </div>

      <div className={s.field}>
        <div className={s.fieldTitle}>这一趟怎么显示</div>
        <p className={s.fieldNote}>
          静默模式照样跑完、照样改 G-code，只是不弹窗给你看进度。
        </p>

        <div className={s.modeGrid} role="radiogroup" aria-label="后处理界面形态">
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
                <span className={s.modeLabel}>{m.label}</span>
                <span className={s.modeHint}>{m.hint}</span>
              </button>
            )
          })}
        </div>
      </div>

      <div className={`${s.field} ${s.fieldRow}`}>
        <div className={s.fieldText}>
          <div className={s.fieldTitle}>支撑面底面涂胶</div>
          <p className={s.fieldNote}>在支撑面下方一层也执行涂胶（比主涂胶低一个层高）</p>
        </div>
        <Switch
          checked={r.glueBottomEnabled}
          ariaLabel="支撑面底面涂胶"
          onChange={(next) => void set({ glueBottomEnabled: next }, on)}
        />
      </div>

      <div className={`${s.field} ${s.fieldRow}`}>
        <div className={s.fieldText}>
          <div className={s.fieldTitle}>智能塔位置</div>
          <p className={s.fieldNote}>
            模型与擦料塔相撞时问一句：自动把塔挪到不撞的位置，还是你自己回切片器挪模型。
            选自动挪塔只改这一趟，预设文件不动。
          </p>
        </div>
        <Switch
          checked={r.autoTowerPosition}
          ariaLabel="智能塔位置"
          onChange={(next) => void set({ autoTowerPosition: next }, on)}
        />
      </div>

      <div className={`${s.field} ${s.fieldRow}`}>
        <div className={s.fieldText}>
          <div className={s.fieldTitle}>重置后处理状态</div>
          <p className={s.fieldNote}>
            钩子卡在等答复、界面这一屏还是上次的时点这里。已经跑完的处理不会被改。
          </p>
        </div>
        <Btn variant="danger" disabled={resetBusy} onClick={doReset}>
          {resetBusy ? '正在重置…' : '重置'}
        </Btn>
      </div>

      {resetNote !== null && <p className={s.note}>{resetNote}</p>}
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
