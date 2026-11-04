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
import type { TowerRelocateMode } from '../../api/contract'
import { msg, useText, useT, type StringKey, type Text } from '../../i18n'
import { usePostprocPrefs } from '../state/usePostprocPrefs'
import { Switch } from '../../components/switch'
import { Tip } from '../../components/tooltip'
import { Btn } from '../ui/Controls'
import { SETTINGS_TARGET_TOWER } from './targets'
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

/**
 * 「智能塔位置」三档（与后端 `autoTowerPosition` 的线上字符串**逐字对应**，id 是契约）。
 *
 * 顺序按"人遇到的频率"排：安装后默认的「每次提问」放第一颗，用户一眼对得上现在的状态。
 * ★ 措辞（2026-11-04 作者：「我觉得问这个字不好听」）：用**每次提问**而不是"问"——
 *   与系统权限那种"每次询问 / 总是允许 / 不允许"同一个语汇，读起来是一件**事**，
 *   不是一句问话。
 */
const TOWER_MODES: {
  id: TowerRelocateMode
  label: StringKey
  hint: StringKey
}[] = [
  {
    id: 'ask',
    label: 'settings.postproc.towerModeAsk',
    hint: 'settings.postproc.towerAskHint',
  },
  {
    id: 'auto',
    label: 'settings.postproc.towerModeAuto',
    hint: 'settings.postproc.towerOnHint',
  },
  {
    id: 'off',
    label: 'settings.postproc.towerModeOff',
    hint: 'settings.postproc.towerOffHint',
  },
]

/**
 * 「智能塔位置」那一行是**深链落点**（2026-11-04）。
 *
 * 这一行只负责挂一个 id（常量住在 `settings/targets.ts`）：滚过去、铺绿底、停 6 秒
 * 淡出全部由 `components/scroll/locate` 一件办完。这里不再接 `flash` prop、也不挂
 * `data-flash` —— 高亮是共用件直接演在这行自己的背景上，组件不参与时序。
 */
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
          <div className={s.fieldTitle}>
            {tr('settings.postproc.notify')}
            <Tip text={tr('settings.postproc.notifyNote')} />
          </div>
        </div>
        <Switch
          checked={r.notifyOnFinish}
          ariaLabel={tr('settings.postproc.notify')}
          onChange={(next) => void set({ notifyOnFinish: next }, on)}
        />
      </div>

      {/* 2026-10-30：两张模式卡 → 一行分段（与主题/语言/数据源同一套）。
          卡的说明不再铺在页面上，挪进 `Tip`（悬停看那颗 `?`）——
          2026-11-04 从手写 `data-tip` 改回来：这一页每一行标题都有一颗 `?`，
          光挂 `data-tip` 只有悬停才出气泡，视觉上像"这一行没说明"。 */}
      <div className={`${s.field} ${s.fieldRow}`}>
        <div className={s.fieldText}>
          <div className={s.fieldTitle}>
            {tr('settings.postproc.mode')}
            <Tip text={tr('settings.postproc.modeNote')} />
          </div>
        </div>
        <div className={s.seg} role="radiogroup" aria-label={tr('settings.postproc.modeAria')}>
          {UI_MODES.map((m) => (
            <button
              key={m.id}
              type="button"
              role="radio"
              aria-checked={r.uiMode === m.id}
              data-tip={tr(m.hint)}
              onClick={() => void set({ uiMode: m.id }, on)}
            >
              {tr(m.label)}
            </button>
          ))}
        </div>
      </div>

      <div className={`${s.field} ${s.fieldRow}`}>
        <div className={s.fieldText}>
          <div className={s.fieldTitle}>
            {tr('settings.postproc.glueBottom')}
            <Tip text={tr('settings.postproc.glueBottomNote')} />
          </div>
        </div>
        <Switch
          checked={r.glueBottomEnabled}
          ariaLabel={tr('settings.postproc.glueBottom')}
          onChange={(next) => void set({ glueBottomEnabled: next }, on)}
        />
      </div>

      {/* 智能塔位置（2026-11-04 第二版：**分段选择器**，三档并列）
          作者：「它怎么还是开关呀，它应该像这个怎么显示啊…… 用那个选择切换器啊」——
          三档是并列的选择，不是"开/关"两个方向，所以与上一行「这一趟怎么显示」
          同一个控件、同一套排版（`UI_MODES` 那张表的弟弟）。

          ★ 标题旁边那颗 `?`（2026-11-04 作者：「为什么不能放？缺少了？」）：
            这一页**每一行标题都有一颗 `?`**，挨着的是上一行「支撑面底面涂胶」——
            那一行少一颗，读起来就像"这一项没有说明"。走同一个 `Tip` 组件
            （它自己会带 `?` 圈并把句子写进 `data-tip`），别手写 `data-tip` 顶替。 */}
      <div
        className={`${s.field} ${s.fieldRow}`}
        /* 报告页 / 后处理弹窗那条「去设置」的落点（唯一出处：settings/targets.ts） */
        id={SETTINGS_TARGET_TOWER}
      >
        <div className={s.fieldText}>
          <div className={s.fieldTitle}>
            {tr('settings.postproc.tower')}
            <Tip text={tr('settings.postproc.towerNote')} />
          </div>
        </div>
        <div className={s.seg} role="radiogroup" aria-label={tr('settings.postproc.towerAria')}>
          {TOWER_MODES.map((m) => (
            <button
              key={m.id}
              type="button"
              role="radio"
              aria-checked={r.autoTowerPosition === m.id}
              data-tip={tr(m.hint)}
              onClick={() => void set({ autoTowerPosition: m.id }, on)}
            >
              {tr(m.label)}
            </button>
          ))}
        </div>
      </div>

      {/* 次级行：附属动作，往右让一格、字灰一档 */}
      <div className={`${s.field} ${s.fieldRow} ${s.rowSub}`}>
        <div className={s.fieldText}>
          <div className={s.fieldTitle}>
            {tr('settings.postproc.reset')}
            <Tip text={tr('settings.postproc.resetNote')} />
          </div>
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
