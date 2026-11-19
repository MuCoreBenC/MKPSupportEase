/**
 * 「默认配置」页 —— 改**客户端新装时的出厂默认值**。
 *
 * # 它改的是哪一份
 *
 * `presets/client-defaults.toml`。那份是唯一源：
 *   · `[Prefs]` 段那 11 项 —— 内核 `PrefsIr::default()` 与客户端 `PostprocPrefs::resolved()` 都读它；
 *   · `[Client]` 段 —— 主题 / 语言 / 切片器 / 引导提示 / 通知 / 显示方式 / 预设页那三项 …
 *     （客户端 `runtime::client_defaults` 读它）。
 * 这里改完**不影响已装好的客户端**，要重新构建才进包 —— 顶上那句话就是为这个。
 *
 * # 为什么能撤销、能保存
 *
 * 写入走 `with_struct_ctx`（一个文件、一条撤销点），落的是影子树；
 * 外壳的「保存改动」才把影子树写进真仓库。这一页不画自己的保存按钮。
 *
 * # 分组照客户端的设置页
 *
 * 卡片顺序、卡片标题、行标签、选项用词**全部照客户端**（`src/app/settings/**`）：
 * 同一个设置在两边叫两个名字时，配默认值的人对不上号（作者 2026-11-18 点过名）。
 * 客户端里那些**没有偏好**的入口（备份与恢复、最近删除、打开日志目录、导出日志…）
 * 不在这页上 —— 它们不是"能配的默认值"。
 */
import { useCallback, useEffect, useState } from 'react'

import { isAppError, wb } from '../api'
import type { DefaultsField, DefaultsView } from '../api'
import s from './DefaultsPage.module.css'

interface Props {
  /** 外壳的「后端状态变过了」计数：撤销 / 重做 / 保存之后 +1，页面据此重取 */
  tick: number
  /** 写完一笔就重取整本与事务状态，页头「保存改动 · N 处」才跟得上 */
  onBookRefresh: () => void
}

/** 卡里的一小节：同一张卡换一层小标题（与客户端那几张卡的次级分组对应） */
interface Section {
  /** 空串 = 不分小节 */
  note: string
  keys: string[]
}

interface EditGroup {
  id: string
  title: string
  desc: string
  sections: Section[]
}

/**
 * 卡片顺序 = 客户端设置页的正文顺序：外观与语言 → 切片与引导 → 后处理（四张）→
 * 预设 → 数据与维护 → 软件更新。键名是 `presets/client-defaults.toml` 里那个键。
 */
const GROUPS: EditGroup[] = [
  {
    id: 'appearance',
    title: '外观与语言',
    desc: '主题与界面语言',
    sections: [{ note: '', keys: ['Theme', 'Locale'] }],
  },
  {
    id: 'slicer',
    title: '切片与引导',
    desc: '打开模型用哪个应用',
    sections: [{ note: '', keys: ['Slicer', 'GuideShow'] }],
  },
  {
    id: 'postproc',
    title: '后处理',
    desc: '完成后的通知与显示',
    sections: [{ note: '', keys: ['NotifyOnFinish', 'UiMode', 'AutoTopOnAsk', 'GlueBottomEnabled', 'AutoTowerPosition'] }],
  },
  {
    id: 'calib',
    title: '校准与补偿',
    desc: '会改写 G-code',
    sections: [
      {
        note: '',
        keys: [
          'ForceCalibrationMode',
          'SkipVibrationCalibration',
          'BeginStageRetractEnabled',
          'BeginStageRetractZLiftMm',
          'CalibrationSafeLiftMm',
          'ZCalibrationDwellSec',
          'XyCalibrationDwellSec',
        ],
      },
    ],
  },
  {
    id: 'under',
    title: '下方支撑余量',
    desc: '笔下面有支撑、且离边够远时就不抬',
    sections: [{ note: '', keys: ['UnderSupportEnabled', 'UnderSupportMm'] }],
  },
  {
    id: 'glue',
    title: '热床预涂胶',
    desc: '可能划伤打印板',
    sections: [{ note: '', keys: ['BedGlueEnabled', 'BedGlueAllModels'] }],
  },
  {
    id: 'presets',
    title: '预设',
    desc: '下载、复制与脚本的默认行为',
    sections: [
      {
        note: '',
        keys: ['AutoUseAfterDownload', 'CopiesFollowCloud', 'AutoCopyScript'],
      },
    ],
  },
  {
    id: 'history',
    title: '数据与维护',
    desc: '执行记录怎么清',
    sections: [{ note: '', keys: ['AutoCleanHistory'] }],
  },
  {
    id: 'update',
    title: '软件更新',
    desc: '检查新版本的时机',
    sections: [{ note: '', keys: ['AutoCheckUpdate'] }],
  },
]

/** 数字框各自的单位与步进 —— 与客户端设置页那几个框一致 */
const NUM_META: Record<string, { unit: string; step: number; min: number; max: number }> = {
  UnderSupportMm: { unit: 'mm', step: 0.05, min: 0, max: 2 },
  BeginStageRetractZLiftMm: { unit: 'mm', step: 1, min: 5, max: 30 },
  CalibrationSafeLiftMm: { unit: 'mm', step: 1, min: 6, max: 20 },
  ZCalibrationDwellSec: { unit: '秒', step: 1, min: 1, max: 20 },
  XyCalibrationDwellSec: { unit: '秒', step: 1, min: 1, max: 3 },
}

/**
 * 选项用词：**逐字照客户端**（`src/i18n/zh-CN.ts`）。
 *
 * 判据是"同一个人在两边看到的是同一个词"：塔位在客户端叫「每次提问 / 自动挪塔 / 不处理」，
 * 这里就不能写「问一句 / 不挪」。
 */
const CHOICES: Record<string, { value: string; label: string }[]> = {
  Theme: [
    { value: 'system', label: '跟随系统' },
    { value: 'light', label: '浅色' },
    { value: 'dark', label: '深色' },
  ],
  Locale: [
    { value: 'system', label: '跟随系统' },
    { value: 'zh-CN', label: '简体中文' },
    { value: 'zh-TW', label: '繁體中文' },
    { value: 'ja', label: '日本語' },
    { value: 'en', label: 'English' },
  ],
  Slicer: [
    { value: 'system', label: '跟随系统默认' },
    { value: 'bambu', label: 'Bambu Studio' },
    { value: 'orca', label: 'OrcaSlicer' },
  ],
  UiMode: [
    { value: 'silent', label: '静默' },
    { value: 'gui', label: 'GUI 界面' },
  ],
  AutoTowerPosition: [
    { value: 'ask', label: '每次提问' },
    { value: 'auto', label: '自动挪塔' },
    { value: 'off', label: '不处理' },
  ],
}

export default function DefaultsPage({ tick, onBookRefresh }: Props) {
  const [view, setView] = useState<DefaultsView | null>(null)
  const [err, setErr] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const load = useCallback(() => {
    wb.defaultsView()
      .then((next) => {
        setView(next)
        setErr(null)
      })
      .catch((e: unknown) => {
        setErr(isAppError(e) ? e.message : String(e))
      })
  }, [])

  useEffect(load, [load, tick])

  const change = (key: string, value: string | number | boolean): void => {
    if (busy) return
    setBusy(true)
    wb.defaultsApply(key, value)
      .then((next) => {
        setView(next)
        setErr(null)
        /* 让外壳重取事务状态：页头那个数、撤销栈都得跟上 */
        onBookRefresh()
      })
      .catch((e: unknown) => {
        setErr(isAppError(e) ? e.message : String(e))
      })
      .finally(() => setBusy(false))
  }

  const fieldOf = (key: string): DefaultsField | null =>
    view?.fields.find((f) => f.key === key) ?? null

  const row = (key: string) => {
    const field = fieldOf(key)
    if (field === null) {
      return (
        <div key={key} className={`${s.row} ${s.readonly}`}>
          <span className={s.label}>
            {key}
            <span className={s.onlyClient}>（这份文件里没有这一项）</span>
          </span>
        </div>
      )
    }
    return (
      <div key={key} className={s.row}>
        <span className={s.label}>
          {field.label}
          <span className={s.key}>{key}</span>
        </span>
        <span className={s.control}>{control(field)}</span>
      </div>
    )
  }

  const control = (field: DefaultsField) => {
    if (typeof field.value === 'boolean') {
      return (
        <button
          type="button"
          role="switch"
          aria-checked={field.value}
          aria-label={field.label}
          className={s.switch}
          disabled={busy}
          onClick={() => change(field.key, !field.value)}
        />
      )
    }
    if (typeof field.value === 'number') {
      const meta = NUM_META[field.key]
      return (
        <>
          <input
            type="number"
            className={s.num}
            aria-label={field.label}
            value={field.value}
            step={meta?.step ?? 1}
            min={meta?.min}
            max={meta?.max}
            disabled={busy}
            onChange={(e) => {
              const next = Number(e.currentTarget.value)
              if (Number.isFinite(next)) change(field.key, next)
            }}
          />
          {meta ? <span className={s.unit}>{meta.unit}</span> : null}
        </>
      )
    }
    return (
      <select
        className={s.select}
        aria-label={field.label}
        value={field.value}
        disabled={busy}
        onChange={(e) => change(field.key, e.currentTarget.value)}
      >
        {(CHOICES[field.key] ?? []).map((c) => (
          <option key={c.value} value={c.value}>
            {c.label}
          </option>
        ))}
      </select>
    )
  }

  return (
    <div className={s.page}>
      <p className={s.intro}>
        改的是 <b>新装默认值</b>：已经装好的客户端不变，重新构建后才进包；老用户可在客户端设置里点「恢复默认」套用。
      </p>

      {err !== null && (
        <p className={s.err} role="alert">
          {err}
        </p>
      )}

      {view === null && err === null && <p className={s.empty}>正在读 {`presets/client-defaults.toml`}…</p>}

      {GROUPS.map((g) => (
        <section key={g.id} className={s.group} aria-label={g.title}>
          <header className={s.groupHead}>
            <h2 className={s.groupTitle}>{g.title}</h2>
            <span className={s.groupDesc}>{g.desc}</span>
          </header>
          <div className={s.body}>
            {g.sections.map((sec) => (
              <div key={`${g.id}-${sec.note || 'main'}`}>
                {sec.note !== '' && <div className={s.sectionNote}>{sec.note}</div>}
                {sec.keys.map((k) => row(k))}
              </div>
            ))}
          </div>
        </section>
      ))}
    </div>
  )
}
