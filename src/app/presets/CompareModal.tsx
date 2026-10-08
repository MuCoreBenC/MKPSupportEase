/*
 * 参数对比台（2026-10-08）。
 *
 * # 它是什么，不是什么是
 *
 * **不是文本 diff，不是三方合并，不是"更新机制"** —— 它是一个**独立的常驻工具**：
 * 把 2~3 份**用户自己的本地预设**的参数并排摆出来，用参数页同款控件直接看、直接改。
 *
 *   · 数据源只有「我的预设」（`presets-mine/`）—— **永不出现 baseline**：
 *     隐藏基准是系统内部的东西，它不进这张台子（作者 2026-10-08 裁决）；
 *   · 选中的所有列**都可编辑**（都是用户自己的文件），保存各写各的、互不覆盖；
 *   · 差异只在**底层比较**（TOML 原文值），展示在参数 UI 层 —— 用户看到的是
 *     「开关 / 数字 / 下拉 / 文本」，不是 `enable = true`。
 *
 * # 「采用此值」是显式动作
 *
 * 不做"点一下某个值就神秘地把别处改了"。差异行上每一格旁边有一枚**显式的**
 * 「采用此值」按钮，它做一件事：**把这一格的值复制到本行其它文件**（标题里点名是哪几份）。
 * 复制只是填进草稿，真正落盘要按底部的「保存修改」。
 *
 * # 读不出来不打掉整个框
 *
 * 某一列读不出来（不是 TOML / 编码不对）时，那一列如实说一句人话，不给会报错的控件 ——
 * 其余几列照常能比能改。
 */

import { Fragment, useCallback, useEffect, useMemo, useState } from 'react'
import { api, errorText } from '../../api'
import type { CatalogRegistry, ParamEdit, PresetParamValues, UserPresetFile } from '../../api'
import { FieldControl } from '../../components/field'
import type { FieldSchema } from '../../components/field'
import { Modal } from '../../components/modal'
import s from './CompareModal.module.css'

/** 最多同时比几份 —— 再多就摆不开了，也不符合"几个版本放一起看"这个用法 */
const MAX_PICKED = 3

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

/** 注册表 → 这台子要的行（分组中文名与顺序来自 catalog，客户端不排一遍） */
function rowsOf(registry: CatalogRegistry): CompareRow[] {
  const sectionLabel = new Map<string, string>()
  const sectionOrder = new Map<string, number>()
  for (const tab of registry.tabs) {
    for (const sec of tab.sections) {
      sectionLabel.set(sec.id, sec.label)
      sectionOrder.set(sec.id, sec.order)
    }
  }
  return registry.params
    .map((p): CompareRow => {
      const groupId = p.layout.sectionId
      return {
        key: p.key,
        label: p.label,
        unit: p.unit,
        groupId,
        groupLabel: sectionLabel.get(groupId) ?? groupId,
        order: (sectionOrder.get(groupId) ?? 0) * 1000 + p.layout.order,
        field: {
          key: p.key,
          label: p.label,
          desc: p.desc,
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
            label: c.label,
            deprecated: c.deprecated === true ? true : undefined,
          })),
        },
      }
    })
    .sort((a, b) => a.order - b.order)
}

interface Props {
  open: boolean
  /** 可比较的候选：**用户自己的预设**（读得出来的 MKP 预设） */
  files: UserPresetFile[]
  host?: HTMLElement | null
  onClose: () => void
  /** 保存成功后给页面一句话（走页面那条提示条） */
  onSaved: (note: string) => void
}

export default function CompareModal({ open, files, host = null, onClose, onSaved }: Props) {
  const [picked, setPicked] = useState<string[]>([])
  const [loaded, setLoaded] = useState<Record<string, PresetParamValues>>({})
  const [draft, setDraft] = useState<Record<string, Record<string, string>>>({})
  const [registry, setRegistry] = useState<CatalogRegistry | null>(null)
  const [busy, setBusy] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)

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
        if (alive) setProblem('读不到参数注册表 —— 对比台起不来')
      },
    )
    return () => {
      alive = false
    }
  }, [open])

  /* 每次打开：默认选前两份（用户什么都不点也能直接看到一张对照表） */
  useEffect(() => {
    if (!open) return
    setProblem(null)
    setDraft({})
    setPicked((cur) => {
      const alive = cur.filter((p) => files.some((f) => f.path === p))
      if (alive.length >= 2) return alive
      return files.slice(0, Math.min(2, MAX_PICKED)).map((f) => f.path)
    })
  }, [open, files])

  const rows = useMemo(() => (registry === null ? [] : rowsOf(registry)), [registry])

  /* 选中的每一列各读一次参数（读不出来的那一列照实存 `problem`，不打掉整张台子） */
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

  const valueAt = useCallback(
    (path: string, key: string): string | undefined =>
      draft[path]?.[key] ?? loaded[path]?.values[key],
    [draft, loaded],
  )

  const dirtyOf = useCallback((path: string) => Object.keys(draft[path] ?? {}).length, [draft])

  const dirtyTotal = picked.reduce((n, p) => n + dirtyOf(p), 0)

  const togglePick = (path: string) => {
    setProblem(null)
    setPicked((cur) => {
      if (cur.includes(path)) return cur.filter((p) => p !== path)
      if (cur.length >= MAX_PICKED) return cur
      return [...cur, path]
    })
  }

  /** 用户直接改某一格的值 */
  const edit = (path: string, key: string, next: string) => {
    setDraft((cur) => ({ ...cur, [path]: { ...(cur[path] ?? {}), [key]: next } }))
  }

  /** **采用此值**：把这一格的值复制到本行其它文件（显式动作，不是点值就改对面） */
  const adopt = (key: string, from: string, value: string) => {
    setDraft((cur) => {
      const next = { ...cur }
      for (const path of picked) {
        if (path === from) continue
        /* 目标那份没有这个字段就跳过（后端只改已存在的键，不该发一条必定失败的改动） */
        if (loaded[path]?.values[key] === undefined) continue
        next[path] = { ...(next[path] ?? {}), [key]: value }
      }
      return next
    })
  }

  const adoptTargets = (key: string, from: string): string[] =>
    picked.filter((p) => p !== from && loaded[p]?.values[key] !== undefined)

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
              problem: '保存后重读失败 —— 关掉重开一次看盘上的真实值',
            }),
          ),
        ),
      )
      setLoaded(Object.fromEntries(again.map((v) => [v.path, v])))
      setDraft({})
      setBusy(false)
      if (failed.length === 0) {
        onSaved(`已保存 ${targets.map(nameOf).join('、')} 的参数改动`)
        onClose()
      } else {
        setProblem(`有 ${failed.length} 份没保存成：${failed.join('；')}`)
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

  const footer = (
    <>
      <span className={s.footNote}>
        {dirtyTotal === 0
          ? '改动只落在这一屏；按「保存修改」才写回各自那份文件'
          : `${dirtyTotal} 处改动待保存`}
      </span>
      <button type="button" className={s.ghost} onClick={onClose} disabled={busy}>
        关闭
      </button>
      <button
        type="button"
        className={s.primary}
        onClick={save}
        disabled={busy || dirtyTotal === 0}
        title="把改动写回各自那份「我的预设」（同一路径，不产生第二份；注释与键序一个字节不动）"
      >
        {busy ? '保存中…' : '保存修改'}
      </button>
    </>
  )

  return (
    <Modal
      open={open}
      title="参数对比"
      subtitle="选 2~3 份「我的预设」，同一项并排看；值可以直接改，「采用此值」把它复制到本行其它几份"
      size="lg"
      host={host}
      closeOnScrim={dirtyTotal === 0}
      closeTitle={dirtyTotal === 0 ? '关掉对比台' : '有未保存的改动 —— 先保存或放弃'}
      onClose={onClose}
      footer={footer}
    >
      <div className={s.pick}>
        {files.length === 0 && (
          <span className={s.note}>
            还没有「我的预设」可以对比 —— 先在资源库里另存一份自己的（官方那一份是模板，不进这张台子）。
          </span>
        )}
        {files.map((f) => {
          const on = picked.includes(f.path)
          const full = !on && picked.length >= MAX_PICKED
          return (
            <button
              key={f.path}
              type="button"
              className={on ? `${s.pickItem} ${s.pickOn}` : s.pickItem}
              disabled={full}
              title={full ? `最多同时比 ${MAX_PICKED} 份` : f.path}
              onClick={() => togglePick(f.path)}
            >
              {f.fileName}
            </button>
          )
        })}
      </div>

      {problem !== null && <p className={s.problem}>{problem}</p>}

      {picked.length < 2 ? (
        <p className={s.note}>至少选两份才能对比。</p>
      ) : (
        <div className={s.grid}>
          <table className={s.table}>
            <thead>
              <tr>
                <th className={s.headKey}>参数</th>
                {picked.map((path) => (
                  <th key={path} className={s.headCol}>
                    <span className={s.headName}>{nameOf(path)}</span>
                    <span className={s.headSub}>
                      {loaded[path]?.problem !== undefined && loaded[path]?.problem !== null
                        ? loaded[path]?.problem
                        : dirtyOf(path) > 0
                          ? '有未保存修改'
                          : '未改动'}
                    </span>
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {groups.map((g) => (
                <Fragment key={g.id}>
                  <tr className={s.groupRow}>
                    <td colSpan={picked.length + 1}>{g.label}</td>
                  </tr>
                  {g.rows.map((row) => {
                    const values = picked.map((p) => valueAt(p, row.key))
                    const present = values.filter((v): v is string => v !== undefined)
                    const differs = present.length > 1 && new Set(present).size > 1
                    return (
                      <tr key={row.key} className={differs ? `${s.row} ${s.rowDiff}` : s.row}>
                        <td className={s.keyCell}>
                          <span className={s.keyLabel}>
                            {row.label}
                            {row.unit !== undefined && row.unit !== '' && (
                              <span className={s.unit}>{row.unit}</span>
                            )}
                          </span>
                          <span className={s.keyPath}>{row.key}</span>
                        </td>
                        {picked.map((path, i) => {
                          const value = values[i]
                          const targets = adoptTargets(row.key, path)
                          return (
                            <td key={path} className={s.cell}>
                              {value === undefined ? (
                                <span className={s.absent} title="这一份里没有这个参数（不是空值）">
                                  —
                                </span>
                              ) : row.field.control === 'gcode' ? (
                                <span className={s.gcode} title={value}>
                                  {value.split('\n')[0] || '（空）'}
                                  {value.includes('\n') && <span className={s.more}>…</span>}
                                </span>
                              ) : (
                                <FieldControl
                                  field={row.field}
                                  raw={value}
                                  form="cell"
                                  onChange={(next) => edit(path, row.key, next)}
                                />
                              )}
                              {differs && value !== undefined && targets.length > 0 && (
                                <button
                                  type="button"
                                  className={s.adopt}
                                  title={`把这一格的值复制到：${targets.map(nameOf).join('、')}`}
                                  onClick={() => adopt(row.key, path, value)}
                                >
                                  采用此值
                                </button>
                              )}
                            </td>
                          )
                        })}
                      </tr>
                    )
                  })}
                </Fragment>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </Modal>
  )
}
