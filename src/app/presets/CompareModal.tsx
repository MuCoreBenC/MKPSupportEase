/*
 * 参数对比台 · v4（2026-10-16 重做，定稿稿 = frontend-lab/lab/preset-compare-gallery.html）。
 *
 * # 交互（作者定的三条）
 *
 *   ① **默认只读** —— 进台值是纯文本（与工作台参数台一致），控件不出现；
 *      底栏右下「进入编辑模式」才进编辑态。
 *      ★ 2026-10-16 统一：这颗原来在**标题栏**（ModalHeadSlot，× 旁边），
 *      作者后来的口径是「进入编辑模式的那个按钮位置也统一一下」——挪到底栏右下，
 *      与「编辑文件」模态框、修改参数弹窗三处同位。
 *   ② 编辑态才有控件（`FieldControl` form="cell"，与修改参数弹窗**同一份共用件**——
 *      作者：「对比的和修改参数的控件可以复用」）+ 底栏 撤销/重做/放弃并退出/保存。
 *      「放弃并退出」= **作废这一次没保存的改动**、回只读（2026-10-16 作者立的口径，
 *      三个模态框统一；原来是「取消」：丢改动但留在编辑态。词从「退出编辑模式」
 *      改到「放弃并退出」是当天第二版的裁决 —— 见 `EDIT_TEXT.exitMode`）。
 *   ③ 候选**不摊一排 chips**（作者：「他就不该显示那么多 toml，一般只是两个对比的，最多三个」）
 *      —— 右键带进来的两份直接坐列，「＋ 加一份对比」的弹层里才列候选（最多 3 份）。
 *
 * # 排版（gallery 两轮换来的两条铁律）
 *
 *   · **grid 锁列**，不用 `<table>`：mono 长键名（toolhead.first_pen_revitalization_flag）
 *     在 table-layout:fixed 里会把格子挤穿、键名窜进值列（第一轮翻车实录）。
 *     键名列 word-break 断行、值列控件 flex-shrink:0 永不折行。
 *   · 骨架参考工作台「参数台」：搜索参数 → 「仅显示差异 N」→ 分组 chips → 状态徽标列。
 *
 * 撤销/重做是**这一台的本地栈**（与参数页的 useParams 无关 —— 那边编辑的是一份，
 * 这里是 2~3 份各一份草稿）：一次手势 = 一条 UndoEntry（「采用此值」是一批 = 一条）。
 */

import { Fragment, useCallback, useEffect, useMemo, useRef, useState } from 'react'
import type { CSSProperties, ReactNode } from 'react'
import { api, errorText } from '../../api'
/* `t` 只给**事件回调**用（保存成功/失败的句子在事发那一刻取，语言是对的）；
   渲染期的字一律走 `tr`，否则切语言不重渲染 */
import { msg, t, useLocale, useText, useT, type Locale, type Text, type Tr } from '../../i18n'
import type { CatalogRegistry, ParamEdit, PresetParamValues, LocalPresetFile } from '../../api'
import { bi } from '../shared/dataLabel'
import { FieldControl } from '../../components/field'
import type { FieldSchema } from '../../components/field'
import { boolOf } from '../../components/field/value'
import Icon from '../shell/icons'
import { Modal } from '../../components/modal'
/* 两态框架那两颗按钮的词（与「编辑文件」/「修改参数」**同一个源**，见那一份的 `EDIT_TEXT`） */
import { EDIT_TEXT } from './presetTree'
import s from './CompareModal.module.css'

/** 最多同时比几份 —— 再多就摆不开了，也不符合"几个版本放一起看"这个用法 */
const MAX_PICKED = 3

/** 一条手势：一次编辑一格；「采用此值」是一批。`before === undefined` = 原来没有这一格 */
interface UndoEntry {
  path: string
  key: string
  before: string | undefined
  after: string
}

/** 列身份色：轴标三色（tokens 现成的 —— 校准页 / 报告页同源） */
const COL_COLORS = ['var(--accent)', 'var(--axis-y)', 'var(--axis-z)']

/** 一个字段在这张台上长什么样（分组 + 控件 + 顺序） */
interface CompareRow {
  key: string
  label: string
  unit?: string
  field: FieldSchema
  groupId: string
  groupLabel: string
  /** 排序：先按分组顺序，再按字段自己声明的组内顺序 */
  order: number
}

/**
 * 注册表 → 这台子要的行（分组名与顺序来自 catalog，客户端不排一遍）。
 *
 * **第二种读取路径**：对比台不经过 `useParams`，自己从 `getRuntimeCatalog` 摊一遍。
 * 双语挑一册因此必须走同一个工具（`dataLabel`）—— 2026-10-26 就是这里漏了，
 * 英文界面上整片中文。
 */
function rowsOf(registry: CatalogRegistry, locale: Locale): CompareRow[] {
  const sectionLabel = new Map<string, string>()
  const sectionOrder = new Map<string, number>()
  for (const tab of registry.tabs) {
    for (const sec of tab.sections) {
      sectionLabel.set(sec.id, bi(sec.label, sec.labelEn, locale))
      sectionOrder.set(sec.id, sec.order)
    }
  }
  return registry.params
    .map((p): CompareRow => {
      const groupId = p.layout.sectionId
      return {
        key: p.key,
        label: bi(p.label, p.labelEn, locale),
        unit: p.unit,
        groupId,
        groupLabel: sectionLabel.get(groupId) ?? groupId,
        order: (sectionOrder.get(groupId) ?? 0) * 1000 + p.layout.order,
        field: {
          key: p.key,
          label: bi(p.label, p.labelEn, locale),
          desc: bi(p.desc, p.descEn, locale),
          control:
            p.uiComponent === 'number'
              ? 'number'
              : p.uiComponent === 'switch'
                ? 'switch'
                : p.uiComponent === 'segmented' || p.uiComponent === 'select'
                  ? 'choice'
                  : p.uiComponent === 'gcode'
                    ? 'gcode'
                    : 'text',
          unit: p.unit,
          min: p.min,
          max: p.max,
          step: p.step,
          choices: p.choices?.map((c) => ({
            value: c.value,
            label: bi(c.label, c.labelEn, locale),
            deprecated: c.deprecated === true ? true : undefined,
          })),
        },
      }
    })
    .sort((a, b) => a.order - b.order)
}

/** 只读态的纯文本值（编辑态才出现控件 —— 作者：「默认是不能改参数的」） */
function roTextOf(row: CompareRow, raw: string, tr: Tr): string {
  switch (row.field.control) {
    case 'switch':
      return boolOf(raw) ? tr('presets.cmp.on') : tr('presets.cmp.off')
    case 'choice':
      return row.field.choices?.find((c) => c.value === raw)?.label ?? raw
    case 'gcode':
      return tr('presets.cmp.lines', {
        count: raw.split('\n').filter((l) => l.trim() !== '').length,
      })
    default:
      return raw
  }
}

interface Props {
  open: boolean
  /** 可比较的候选：**用户自己的预设**（读得出来的 MKP 预设） */
  files: LocalPresetFile[]
  /**
   * **这一次打开要直接选中的两份**（path）—— 入口是右键两下选：
   * 右键第一份点「参数对比」、右键第二份点「与「X」对比」，那两份由这里带进来、
   * 开台即选中（就是列头坐定的 ①②）。只在打开那一下生效。
   */
  preselect?: string[]
  host?: HTMLElement | null
  onClose: () => void
  /** 保存成功后给页面一句话（走页面那条提示条） */
  onSaved: (note: string) => void
}

export default function CompareModal({
  open,
  files,
  preselect,
  host = null,
  onClose,
  onSaved,
}: Props) {
  const tr = useT()
  const say = useText()
  /* 数据（参数名/分组名）那一册 —— 与参数页同一个工具，别在这条路上再抄一遍 */
  const locale = useLocale()
  const [picked, setPicked] = useState<string[]>([])
  const [loaded, setLoaded] = useState<Record<string, PresetParamValues>>({})
  const [draft, setDraft] = useState<Record<string, Record<string, string>>>({})
  const [registry, setRegistry] = useState<CatalogRegistry | null>(null)
  const [busy, setBusy] = useState(false)
  /** 这张台子自己的"没成"那句话（存键，渲染时再翻） */
  const [problem, setProblem] = useState<Text | null>(null)

  /* 只读 / 编辑（作者：「默认是不能改参数的，加一个编辑或者修改按钮才能改」） */
  const [editing, setEditing] = useState(false)
  /** 本地撤销栈：一次手势一条（「采用此值」一批也是一条） */
  const [past, setPast] = useState<UndoEntry[][]>([])
  const [future, setFuture] = useState<UndoEntry[][]>([])
  const [query, setQuery] = useState('')
  const [onlyDiff, setOnlyDiff] = useState(false)
  const [groupTab, setGroupTab] = useState<string>('all')
  const [addOpen, setAddOpen] = useState(false)
  const addRef = useRef<HTMLDivElement>(null)

  const nameOf = useCallback(
    (path: string) => files.find((f) => f.path === path)?.fileName ?? path.split('/').pop() ?? path,
    [files],
  )

  /* 打开时读一次注册表（字段定义 + 分组名 —— 与参数页同一份来源） */
  useEffect(() => {
    if (!open) return
    let alive = true
    void api.getRuntimeCatalog().then(
      (c) => {
        if (alive) setRegistry(c.registry)
      },
      () => {
        if (alive) setProblem(msg('presets.cmp.registryFailed'))
      },
    )
    return () => {
      alive = false
    }
  }, [open])

  /*
   * 关掉就收回编辑态与小开关。**草稿的清理在下面「打开那一下」**（`justOpened` 的 first
   * 分支）—— 两处合起来的结果：**下次打开是一台干净的只读台**（草稿不跨开关保留，
   * 所以也永远不会出现"重开时还挂着未保存改动"这一幕）。
   */
  useEffect(() => {
    if (open) return
    setEditing(false)
    setPast([])
    setFuture([])
    setQuery('')
    setOnlyDiff(false)
    setGroupTab('all')
    setAddOpen(false)
  }, [open])

  /*
   * 每次打开：优先用右键带进来的那两份（`preselect`），没有就默认选前两份。
   * `justOpened` 认「打开的那一下」——之后用户自己增删列，不被它顶掉。
   */
  const justOpened = useRef(false)
  useEffect(() => {
    if (!open) {
      justOpened.current = false
      return
    }
    const first = !justOpened.current
    justOpened.current = true
    setProblem(null)
    if (first) {
      setDraft({})
      setEditing(false)
      setPast([])
      setFuture([])
    }
    setPicked((cur) => {
      if (first) {
        const want = (preselect ?? []).filter((p) => files.some((f) => f.path === p))
        if (want.length >= 2) return want.slice(0, MAX_PICKED)
      }
      const alive = cur.filter((p) => files.some((f) => f.path === p))
      if (alive.length >= 2) return alive
      return files.slice(0, Math.min(2, MAX_PICKED)).map((f) => f.path)
    })
  }, [open, files, preselect])

  const rows = useMemo(
    () => (registry === null ? [] : rowsOf(registry, locale)),
    [registry, locale],
  )

  /* 选中的每一列各读一次参数（读不出来的那一列照实存 problem，不打掉整张台子） */
  useEffect(() => {
    if (!open) return
    const missing = picked.filter((p) => loaded[p] === undefined)
    if (missing.length === 0) return
    let alive = true
    void Promise.all(
      missing.map((path) =>
        api.readPresetParams(path).then(
          (v) => v,
          (e: unknown): PresetParamValues => ({
            path,
            fileName: nameOf(path),
            values: {},
            problem: errorText(e),
          }),
        ),
      ),
    ).then((got) => {
      if (!alive) return
      setLoaded((cur) => {
        const next = { ...cur }
        for (const v of got) next[v.path] = v
        return next
      })
    })
    return () => {
      alive = false
    }
  }, [open, picked, loaded, nameOf])

  /* 点弹层外面收「加一份对比」的菜单 */
  useEffect(() => {
    if (!addOpen) return
    const onDown = (e: MouseEvent) => {
      if (addRef.current !== null && !addRef.current.contains(e.target as Node)) setAddOpen(false)
    }
    document.addEventListener('mousedown', onDown)
    return () => document.removeEventListener('mousedown', onDown)
  }, [addOpen])

  const valueAt = useCallback(
    (path: string, key: string): string | undefined =>
      draft[path]?.[key] ?? loaded[path]?.values[key],
    [draft, loaded],
  )

  const dirtyOf = useCallback((path: string) => Object.keys(draft[path] ?? {}).length, [draft])

  const dirtyTotal = picked.reduce((n, p) => n + dirtyOf(p), 0)

  /* ---------- 列的增删（候选只在弹层里出现） ---------- */

  const removePick = (path: string) => {
    if (picked.length <= 2) return
    setPicked((cur) => cur.filter((p) => p !== path))
  }

  const candidates = files.filter((f) => !picked.includes(f.path))

  const addPick = (path: string) => {
    setProblem(null)
    setPicked((cur) => (cur.length >= MAX_PICKED || cur.includes(path) ? cur : [...cur, path]))
    setAddOpen(false)
  }

  /* ---------- 改值（编辑态） + 本地撤销/重做 ---------- */

  /** 记一条手势（batch = 一次手势动过的格子；「采用此值」一批也是一条） */
  const pushUndo = (batch: UndoEntry[]) => {
    setPast((p) => [...p, batch])
    setFuture([])
  }

  const edit = (path: string, key: string, next: string) => {
    const before = valueAt(path, key)
    if (before === next) return
    setDraft((cur) => ({ ...cur, [path]: { ...(cur[path] ?? {}), [key]: next } }))
    pushUndo([{ path, key, before, after: next }])
  }

  /** **采用此值**：把这一格的值复制到本行其它文件（显式动作，不是点值就改对面） */
  const adopt = (key: string, from: string, value: string) => {
    const targets = picked.filter((p) => p !== from && loaded[p]?.values[key] !== undefined)
    if (targets.length === 0) return
    const batch: UndoEntry[] = []
    setDraft((cur) => {
      const next = { ...cur }
      for (const path of targets) {
        batch.push({ path, key, before: next[path]?.[key], after: value })
        next[path] = { ...(next[path] ?? {}), [key]: value }
      }
      return next
    })
    pushUndo(batch)
  }

  const undo = () => {
    const entry = past[past.length - 1]
    if (entry === undefined) return
    setDraft((cur) => {
      const next = { ...cur }
      for (const { path, key, before } of entry) {
        const cell = { ...(next[path] ?? {}) }
        if (before === undefined) delete cell[key]
        else cell[key] = before
        next[path] = cell
      }
      return next
    })
    setPast((p) => p.slice(0, -1))
    setFuture((f) => [...f, entry])
  }

  const redo = () => {
    const entry = future[future.length - 1]
    if (entry === undefined) return
    setDraft((cur) => {
      const next = { ...cur }
      for (const { path, key, after } of entry) {
        next[path] = { ...(next[path] ?? {}), [key]: after }
      }
      return next
    })
    setFuture((f) => f.slice(0, -1))
    setPast((p) => [...p, entry])
  }

  const save = () => {
    const targets = picked.filter((p) => dirtyOf(p) > 0)
    if (targets.length === 0) return
    setBusy(true)
    setProblem(null)
    void (async () => {
      const failed: string[] = []
      for (const path of targets) {
        const edits: ParamEdit[] = Object.entries(draft[path] ?? {}).map(([paramKey, value]) => ({
          paramKey,
          value,
        }))
        const err = await api.savePresetParams(path, edits).then(
          () => null,
          (e: unknown) => errorText(e),
        )
        if (err !== null) failed.push(`${nameOf(path)}：${err}`)
      }
      /* 写完**重读那几列**（与别处同一条规矩：界面显示的必须是盘上那份的真实读数） */
      const again = await Promise.all(
        picked.map((path) =>
          api.readPresetParams(path).then(
            (v) => v,
            (): PresetParamValues => ({
              path,
              fileName: nameOf(path),
              values: {},
              problem: t('presets.cmp.savedReloadFailed'),
            }),
          ),
        ),
      )
      if (!host) {
        /* host 无关：状态照常更新 */
      }
      setLoaded((cur) => {
        const next = { ...cur }
        for (const v of again) next[v.path] = v
        return next
      })
      setBusy(false)
      if (failed.length === 0) {
        onSaved(
          t('presets.cmp.savedOk', {
            names: targets.map(nameOf).join(t('presets.cmp.savedSep')),
          }),
        )
        onClose()
      } else {
        setProblem(
          msg('presets.cmp.savedFailed', {
            count: failed.length,
            names: failed.join(t('presets.cmp.savedFailSep')),
          }),
        )
      }
    })()
  }

  const groups = useMemo(() => {
    const out: { id: string; label: string; rows: CompareRow[] }[] = []
    for (const row of rows) {
      const last = out[out.length - 1]
      if (last !== undefined && last.id === row.groupId) last.rows.push(row)
      else out.push({ id: row.groupId, label: row.groupLabel, rows: [row] })
    }
    return out
  }, [rows])

  /* ---------- 过滤（搜索 / 仅显示差异 / 分组 chips） ---------- */

  const rowDiffers = useCallback(
    (row: CompareRow) => {
      const values = picked.map((p) => valueAt(p, row.key))
      const present = values.filter((v): v is string => v !== undefined)
      return present.length > 1 && new Set(present).size > 1
    },
    [picked, valueAt],
  )

  const rowMatches = useCallback(
    (row: CompareRow) => {
      if (onlyDiff && !rowDiffers(row)) return false
      const kw = query.trim().toLowerCase()
      if (kw === '') return true
      return row.label.toLowerCase().includes(kw) || row.key.toLowerCase().includes(kw)
    },
    [onlyDiff, query, rowDiffers],
  )

  const tabCounts = useMemo(() => {
    const m = new Map<string, number>()
    let all = 0
    for (const row of rows) {
      if (!rowMatches(row)) continue
      m.set(row.groupId, (m.get(row.groupId) ?? 0) + 1)
      all++
    }
    return { m, all }
  }, [rows, rowMatches])

  /* ---------- 底栏（两态，与「编辑文件」/「修改参数」同一套框架） ---------- */

  /*
   * ★ 2026-10-16 统一（作者：「不管是对比还是那个修改参数，他们的那个编辑模式布局
   *   几乎都要一样……进入编辑模式的那个按钮位置也统一一下」）：
   *
   *   只读态   左「只读对照」· 右下「进入编辑模式」（原来这颗在**标题栏**，挪下来）
   *   编辑态   左「未保存的改动（N）」· 右下 撤销 / 重做 / 放弃并退出 / 保存
   *
   * 「放弃并退出」= 回只读态 + **作废未保存的改动**（2026-10-16 作者：「退出编辑模式
   * 就是作废，那些改的都作废，保存了才有」；原来那颗「取消」丢改动但留在编辑态，
   * 随这一刀退休）。两颗按钮的词都取自 `EDIT_TEXT`（三个模态框同一句话）。
   */
  const footer = (
    <>
      {editing ? (
        <>
          <span className={s.footNote}>
            {dirtyTotal === 0 ? (
              tr('presets.cmp.noDirty')
            ) : (
              <>
                {tr('presets.cmp.dirtyCount', { count: dirtyTotal })}
              </>
            )}
          </span>
          <span className={s.footBtns}>
            <button
              type="button"
              className={s.iconBtn}
              disabled={past.length === 0 || busy}
              title={tr('presets.cmp.undoTitle')}
              aria-label={tr('presets.cmp.undo')}
              onClick={undo}
            >
              <Icon name="undo" size={15} />
            </button>
            <button
              type="button"
              className={s.iconBtn}
              disabled={future.length === 0 || busy}
              title={tr('presets.cmp.redo')}
              aria-label={tr('presets.cmp.redo')}
              onClick={redo}
            >
              <Icon name="redo" size={15} />
            </button>
            <button
              type="button"
              className={s.textBtn}
              disabled={busy}
              title={tr(EDIT_TEXT.exitTitle)}
              onClick={() => {
                /*
                 * **那一颗 = 作废**（作者 2026-10-16：「没有保存的话，退出编辑模式
                 * 就是作废，那些改的都作废，保存了才有」）——与「编辑文件」/「修改参数」
                 * 三处同一个口径。本来这台就是"打开那一下清草稿"（见 `justOpened`），
                 * 这一步与它同一条路。
                 */
                setDraft({})
                setPast([])
                setFuture([])
                setEditing(false)
              }}
            >
              {tr(EDIT_TEXT.exitMode)}
            </button>
            <button
              type="button"
              className={s.primary}
              onClick={save}
              disabled={busy || dirtyTotal === 0}
            >
              {busy ? tr('presets.cmp.saving') : tr('presets.cmp.save')}
            </button>
          </span>
        </>
      ) : (
        <>
          <span className={s.footNote}>{tr('presets.cmp.readonlyNote')}</span>
          <button
            type="button"
            className={s.primary}
            disabled={busy || picked.length < 2}
            title={tr('presets.cmp.enterTitle')}
            onClick={() => setEditing(true)}
          >
            {tr(EDIT_TEXT.enterMode)}
          </button>
        </>
      )}
    </>
  )

  const gridStyle = { '--n': picked.length } as CSSProperties

  const body: ReactNode = (
    <>
      {problem !== null && <p className={s.problem}>{say(problem)}</p>}

      {picked.length < 2 ? (
        <p className={s.note}>{tr('presets.cmp.needTwo')}</p>
      ) : (
        <>
          {/* 工具行：搜索 + 仅显示差异 + 计数 + 加一份（候选只在弹层里） */}
          <div className={s.tools}>
            <input
              className={s.search}
              placeholder={tr('presets.cmp.searchPlaceholder')}
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
            <button
              type="button"
              className={onlyDiff ? `${s.onlyDiff} ${s.on}` : s.onlyDiff}
              onClick={() => setOnlyDiff((v) => !v)}
              title={tr('presets.cmp.onlyDiffTitle')}
            >
              <span className={s.sw} />
              {tr('presets.cmp.onlyDiff')} <span className={s.cnt}>{tabCounts.all}</span>
            </button>
            <span className={s.toolsCnt}>
              {tr('presets.cmp.total', { count: tabCounts.all })}
            </span>
            <span className={s.toolsGap} />
            <div className={s.addWrap} ref={addRef}>
              <button
                type="button"
                className={s.addBtn}
                disabled={picked.length >= MAX_PICKED || candidates.length === 0}
                title={
                  picked.length >= MAX_PICKED
                    ? tr('presets.cmp.maxPicked')
                    : tr('presets.cmp.addPick')
                }
                onClick={() => setAddOpen((v) => !v)}
              >
                {tr('presets.cmp.add')}
              </button>
              {addOpen && (
                <div className={s.addMenu}>
                  {candidates.map((f, i) => (
                    <button key={f.path} type="button" onClick={() => addPick(f.path)}>
                      <i
                        className={s.dot}
                        style={{ background: COL_COLORS[(picked.length + i) % COL_COLORS.length] }}
                      />
                      <span>{f.fileName}</span>
                    </button>
                  ))}
                  {candidates.length === 0 && (
                    <button type="button" disabled>
                      <span>{tr('presets.cmp.noMore')}</span>
                    </button>
                  )}
                </div>
              )}
            </div>
          </div>

          {/* 分组 chips（工作台「全部 73 / 偏移 5 …」同构） */}
          <div className={s.tabs}>
            <button
              type="button"
              className={groupTab === 'all' ? `${s.tab} ${s.on}` : s.tab}
              onClick={() => setGroupTab('all')}
            >
              {tr('presets.cmp.allTab')} <span className={s.n}>{tabCounts.all}</span>
            </button>
            {groups.map((g) => (
              <button
                key={g.id}
                type="button"
                className={groupTab === g.id ? `${s.tab} ${s.on}` : s.tab}
                onClick={() => setGroupTab(g.id)}
              >
                {g.label} <span className={s.n}>{tabCounts.m.get(g.id) ?? 0}</span>
              </button>
            ))}
          </div>

          <div className={s.grid} style={gridStyle}>
            {/* 列头 = 槽位：色点 + 文件名 + ✕（两份时钉死，✕ 不出） */}
            <div className={s.thead}>
              <span className={s.hkey}>{tr('presets.cmp.keyHeader')}</span>
              {picked.map((path, i) => (
                <span key={path} className={s.hcol}>
                  <i className={s.dot} style={{ background: COL_COLORS[i] }} />
                  <span className={s.nm} title={nameOf(path)}>
                    {nameOf(path)}
                  </span>
                  <span className={s.ops}>
                    <button
                      type="button"
                      className={s.op}
                      disabled={picked.length <= 2}
                      title={
                        picked.length <= 2
                          ? tr('presets.cmp.removeMin')
                          : tr('presets.cmp.removeTitle')
                      }
                      onClick={() => removePick(path)}
                    >
                      ✕
                    </button>
                  </span>
                  {dirtyOf(path) > 0 && (
                    <span className={s.op} style={{ cursor: 'default', opacity: 1 }} title="">
                      {dirtyOf(path)}
                    </span>
                  )}
                </span>
              ))}
              <span className={s.hst}>{tr('presets.cmp.statusHeader')}</span>
            </div>

            {groups.map((g) => {
              const visible = g.rows.filter(rowMatches)
              if (groupTab !== 'all' && groupTab !== g.id) return null
              if (visible.length === 0) return null
              return (
                <Fragment key={g.id}>
                  <div className={`${s.grow} ${s.groupRow}`}>
                    <span>{g.label}</span>
                  </div>
                  {visible.map((row) => {
                    const differs = rowDiffers(row)
                    return (
                      <div key={row.key} className={`${s.grow}${differs ? ` ${s.diff}` : ''}`}>
                        <span className={s.keyCell}>
                          <span className={s.keyLabel}>
                            {row.label}
                            {row.unit !== undefined && row.unit !== '' && (
                              <span className={s.unit}>{row.unit}</span>
                            )}
                          </span>
                          <span className={s.keyPath}>{row.key}</span>
                        </span>
                        {picked.map((path) => {
                          const value = valueAt(path, row.key)
                          const targets =
                            editing && differs && value !== undefined
                              ? picked.filter(
                                  (p) => p !== path && loaded[p]?.values[row.key] !== undefined,
                                )
                              : []
                          return (
                            <span key={path} className={s.cell}>
                              {value === undefined ? (
                                <span className={s.absent}>{tr('presets.cmp.na')}</span>
                              ) : !editing ? (
                                <span className={s.ro}>
                                  <span className={row.field.control === 'number' ? s.ro : s.ro}>
                                    <span
                                      className={
                                        row.field.control === 'number' || row.field.control === 'text'
                                          ? `${s.ro} ${s.mono}`
                                          : s.ro
                                      }
                                    >
                                      {roTextOf(row, value, tr)}
                                    </span>
                                  </span>
                                </span>
                              ) : row.field.control === 'gcode' ? (
                                <span className={s.gcode}>
                                  {value.split('\n')[0] || tr('presets.cmp.empty')}
                                  {value.includes('\n') && <span className={s.absent}>…</span>}
                                </span>
                              ) : (
                                <FieldControl
                                  field={row.field}
                                  raw={value}
                                  form="cell"
                                  onChange={(next) => edit(path, row.key, next)}
                                />
                              )}
                              {targets.length > 0 && value !== undefined && (
                                <button
                                  type="button"
                                  className={s.adopt}
                                  onClick={() => adopt(row.key, path, value)}
                                >
                                  {tr('presets.cmp.adopt')}
                                </button>
                              )}
                            </span>
                          )
                        })}
                        <span className={s.cell}>
                          <span className={differs ? `${s.badge} ${s.diff}` : `${s.badge} ${s.same}`}>
                            {differs
                              ? tr('presets.cmp.differs')
                              : picked.some((p) => valueAt(p, row.key) === undefined)
                                ? '—'
                                : tr('presets.cmp.same')}
                          </span>
                        </span>
                      </div>
                    )
                  })}
                </Fragment>
              )
            })}
          </div>
        </>
      )}
    </>
  )

  return (
    <Modal
      open={open}
      title={tr('presets.cmp.title')}
      subtitle={
        editing
          ? tr('presets.cmp.subtitleEditing')
          : tr('presets.cmp.subtitleReadonly')
      }
      size="lg"
      host={host}
      /* 点遮罩一律不关（2026-10-15 作者定的全站口径）—— 关走 Esc 或页眉那颗 */
      closeTitle={dirtyTotal === 0 ? tr('presets.cmp.closeClean') : tr('presets.cmp.closeDirty')}
      onClose={onClose}
      footer={footer}
    >
      {body}
    </Modal>
  )
}
