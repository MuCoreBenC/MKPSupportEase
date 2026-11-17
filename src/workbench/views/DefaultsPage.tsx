/**
 * 「默认配置」页 —— 改**客户端新装时的出厂默认值**。
 *
 * # 它改的是哪一份
 *
 * `presets/client-defaults.toml` 的 `[Prefs]` 段。那份是唯一源：内核
 * `PrefsIr::default()` 与客户端 `PostprocPrefs::resolved()` 都 `include_str!` 它。
 * 所以这里改完**不影响已装好的客户端**，要重新构建才进包 —— 顶上那句话就是为这个。
 *
 * # 为什么能撤销、能保存
 *
 * 写入走 `with_struct_ctx`（一个文件、一条撤销点），落的是影子树；
 * 外壳的「保存改动」才把影子树写进真仓库。这一页不画自己的保存按钮。
 *
 * # 为什么只做两组的控件
 *
 * 那 11 个键是「真改变切出来 G-code」的全部。外观 / 切片 / 备份 / 高级 / 引导图
 * 是客户端私有偏好，不在这个文件里 —— 照客户端顺序摆出来、标一句「只在客户端设置」，
 * 不假装它们能在这里改。
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

/** 卡里的一小节：同一张卡换一层小标题（校准动作 / 涂胶与塔） */
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
 * 可编辑的两张卡。**顺序照客户端设置页**：校准与补偿在前、下方支撑余量在后。
 *
 * 11 个键里 `GlueBottomEnabled` / `BedGlueEnabled` / `BedGlueAllModels` /
 * `AutoTowerPosition` 在客户端不在「校准与补偿」那一段，这里收进同一张卡的第二小节
 * —— 它们同属「影响产物、但不是下方支撑余量」，单独占一张卡会把客户端那 7 组的顺序打乱。
 */
const EDIT_GROUPS: EditGroup[] = [
  {
    id: 'calib',
    title: '校准与补偿',
    desc: '切出来之前那些校准动作、收笔与停留时间。',
    sections: [
      {
        note: '',
        keys: [
          'SkipVibrationCalibration',
          'BeginStageRetractEnabled',
          'BeginStageRetractZLiftMm',
          'ZCalibrationDwellSec',
          'XyCalibrationDwellSec',
        ],
      },
      {
        note: '涂胶与塔',
        keys: ['GlueBottomEnabled', 'BedGlueEnabled', 'BedGlueAllModels', 'AutoTowerPosition'],
      },
    ],
  },
  {
    id: 'under',
    title: '下方支撑余量',
    desc: '笔下方支撑够宽时不让它抬；贴合实验判据。',
    sections: [{ note: '', keys: ['UnderSupportEnabled', 'UnderSupportMm'] }],
  },
]

/** 只读的五组：照客户端顺序排，标一句「只在客户端设置」 */
const READONLY_GROUPS: { id: string; title: string; desc: string; rows: string[] }[] = [
  {
    id: 'appearance',
    title: '外观',
    desc: '主题、语言与界面密度。',
    rows: ['主题', '语言', '界面密度'],
  },
  {
    id: 'slicer',
    title: '切片',
    desc: '切片器程序与预设文件的位置。',
    rows: ['切片器程序', '预设文件目录'],
  },
  {
    id: 'backup',
    title: '备份',
    desc: '备份目录与手动备份。',
    rows: ['备份目录', '自动备份'],
  },
  {
    id: 'advanced',
    title: '高级',
    desc: '预设数据源、调试偏好与诊断入口。',
    rows: ['预设数据源', '调试偏好', '打开日志目录'],
  },
  {
    id: 'guide',
    title: '引导图',
    desc: '引导图资源与生成。',
    rows: ['引导图目录', '生成引导图'],
  },
]

/** 数字框各自的单位与步进 —— 与客户端设置页那几个框一致 */
const NUM_META: Record<string, { unit: string; step: number; min: number; max: number }> = {
  UnderSupportMm: { unit: 'mm', step: 0.05, min: 0, max: 2 },
  BeginStageRetractZLiftMm: { unit: 'mm', step: 1, min: 5, max: 30 },
  ZCalibrationDwellSec: { unit: '秒', step: 1, min: 1, max: 20 },
  XyCalibrationDwellSec: { unit: '秒', step: 1, min: 1, max: 3 },
}

const TOWER_MODES = [
  { value: 'ask', label: '问一句' },
  { value: 'auto', label: '自动挪塔' },
  { value: 'off', label: '不挪' },
]

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
        {TOWER_MODES.map((m) => (
          <option key={m.value} value={m.value}>
            {m.label}
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

      {EDIT_GROUPS.map((g) => (
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

      {READONLY_GROUPS.map((g) => (
        <section key={g.id} className={s.group} aria-label={g.title}>
          <header className={s.groupHead}>
            <h2 className={s.groupTitle}>{g.title}</h2>
            <span className={s.groupDesc}>{g.desc}</span>
          </header>
          <div className={s.body}>
            {g.rows.map((r) => (
              <div key={r} className={`${s.row} ${s.readonly}`}>
                <span className={s.label}>{r}</span>
                <span className={s.control}>
                  <span className={s.onlyClient}>只在客户端设置</span>
                </span>
              </div>
            ))}
          </div>
        </section>
      ))}
    </div>
  )
}
