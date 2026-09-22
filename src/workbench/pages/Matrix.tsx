/**
 * 参数矩阵与批量编辑。
 *
 * 首次打开只显示**当前机型**的版本（doc §9.8）：一上来就把整个仓库摊开，
 * 真正要比较的两列会被淹掉。之后恢复上次的筛选，另给「显示全部」的入口。
 * 筛选状态存在 `.draft/matrix-view.json` —— 本机状态，不入库。
 *
 * 批量编辑的界面顺序是刻意的，四步一步都不能省：
 * **显式勾列 → 选字段 → 输入新值 → 预览影响范围 → 确认**。
 * 没有"一键应用到所有机型"这种入口。
 */
import { useCallback, useEffect, useMemo, useState } from 'react'

import {
  describeError,
  wb,
  type BulkEffect,
  type BulkTarget,
  type MachineNode,
  type Matrix,
  type MatrixCell,
  type ParamValue,
  type Registry,
} from '../api'
import type { Selection } from '../App'

const VIEW_STATE = 'matrix-view'

interface ViewState {
  machineFilter: string[]
  /** 也勾了哪些机型的"基底"作为批量目标 */
  baseTargets: string[]
}

export function MatrixPage({
  tree,
  registry,
  selection,
}: {
  tree: MachineNode[]
  registry: Registry | null
  selection: Selection | null
}) {
  const [matrix, setMatrix] = useState<Matrix | null>(null)
  const [filter, setFilter] = useState<string[] | null>(null)
  const [picked, setPicked] = useState<string[]>([])
  const [baseTargets, setBaseTargets] = useState<string[]>([])
  const [fieldKey, setFieldKey] = useState('')
  const [raw, setRaw] = useState('')
  const [effects, setEffects] = useState<BulkEffect[] | null>(null)
  const [msg, setMsg] = useState<string | null>(null)
  const [err, setErr] = useState<string | null>(null)

  /* 首次：读上次的筛选；没有就用"当前机型"。
     刻意不默认全部 —— 见本文件头注 */
  useEffect(() => {
    void (async () => {
      try {
        const saved = await wb.viewState<ViewState>(VIEW_STATE)
        if (saved?.machineFilter) {
          setFilter(saved.machineFilter)
          setBaseTargets(saved.baseTargets ?? [])
        } else {
          setFilter(selection ? [selection.machineId] : [])
        }
      } catch (e) {
        setErr(describeError(e))
        setFilter([])
      }
    })()
    // 只在挂载时读一次：之后筛选由用户操作决定，不该被选中项变化覆盖
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const reload = useCallback(async (f: string[]) => {
    try {
      setMatrix(await wb.matrix(f))
    } catch (e) {
      setErr(describeError(e))
    }
  }, [])

  useEffect(() => {
    if (filter === null) return
    void reload(filter)
    void wb
      .saveViewState(VIEW_STATE, { machineFilter: filter, baseTargets } satisfies ViewState)
      .catch(() => {
        /* 视图状态存不上不影响用，静默 —— 它不是数据 */
      })
  }, [filter, baseTargets, reload])

  const fields = useMemo(
    () => (registry ? registry.fields.slice().sort((a, b) => a.order - b.order) : []),
    [registry],
  )

  const targets: BulkTarget[] = useMemo(() => {
    const t: BulkTarget[] = picked.map((k) => {
      const [machineId, versionId] = k.split('/')
      return { kind: 'versionOverride', machineId, versionId }
    })
    for (const m of baseTargets) t.push({ kind: 'machineBase', machineId: m })
    return t
  }, [picked, baseTargets])

  const parsed: ParamValue | null = useMemo(() => {
    const f = fields.find((x) => x.key === fieldKey)
    if (!f || raw === '') return null
    if (f.valueType === 'bool') return raw === 'true'
    if (f.valueType === 'float' || f.valueType === 'int') {
      const n = Number(raw)
      return Number.isNaN(n) ? null : f.valueType === 'int' ? Math.round(n) : n
    }
    return raw
  }, [fields, fieldKey, raw])

  const run = async (apply: boolean) => {
    if (!fieldKey || parsed === null || targets.length === 0) return
    setErr(null)
    setMsg(null)
    try {
      const r = apply
        ? await wb.bulkApply(fieldKey, parsed, targets)
        : await wb.bulkPreview(fieldKey, parsed, targets)
      setEffects(r)
      if (apply) {
        const wrote = r.filter((e) => e.kind !== 'noChange' && e.kind !== 'notApplicable').length
        setMsg(`已改 ${wrote} 处（跳过 ${r.length - wrote} 处：值没变或该机型不适用）。还没有生成 TOML。`)
        if (filter) await reload(filter)
      }
    } catch (e) {
      setErr(describeError(e))
    }
  }

  if (filter === null) return <p className="wb-placeholder">读取中…</p>

  return (
    <div className="wb-stack">
      <div className="wb-row">
        <span className="wb-hint">显示机型</span>
        {tree.map((m) => (
          <label key={m.id} className="wb-radio">
            <input
              type="checkbox"
              checked={filter.includes(m.id)}
              onChange={(e) =>
                setFilter(
                  e.target.checked ? [...filter, m.id] : filter.filter((x) => x !== m.id),
                )
              }
            />
            {m.displayName}
          </label>
        ))}
        <button type="button" className="wb-btn" onClick={() => setFilter(tree.map((m) => m.id))}>
          显示全部版本
        </button>
        {selection && (
          <button type="button" className="wb-btn" onClick={() => setFilter([selection.machineId])}>
            只看当前机型
          </button>
        )}
      </div>

      {msg && <div className="wb-banner">{msg}</div>}
      {err && <div className="wb-banner wb-banner--bad">{err}</div>}

      {matrix && (
        <div className="wb-scroll-x">
          <table className="wb-table">
            <thead>
              <tr>
                <th>字段</th>
                {matrix.columns.map((c) => {
                  const k = `${c.machineId}/${c.versionId}`
                  return (
                    <th key={k}>
                      <label className="wb-radio">
                        <input
                          type="checkbox"
                          checked={picked.includes(k)}
                          onChange={(e) =>
                            setPicked(
                              e.target.checked
                                ? [...picked, k]
                                : picked.filter((x) => x !== k),
                            )
                          }
                        />
                        {c.machineName} / {c.versionName}
                      </label>
                      {c.unconfigured && <div className="wb-tag wb-tag--warn">未配置</div>}
                    </th>
                  )
                })}
              </tr>
              <tr>
                <th className="wb-hint">机型基底（作为批量目标）</th>
                {matrix.columns.map((c) => {
                  const first =
                    matrix.columns.findIndex((x) => x.machineId === c.machineId) ===
                    matrix.columns.indexOf(c)
                  return (
                    <th key={`base-${c.machineId}/${c.versionId}`}>
                      {first && (
                        <label className="wb-radio">
                          <input
                            type="checkbox"
                            checked={baseTargets.includes(c.machineId)}
                            onChange={(e) =>
                              setBaseTargets(
                                e.target.checked
                                  ? [...baseTargets, c.machineId]
                                  : baseTargets.filter((x) => x !== c.machineId),
                              )
                            }
                          />
                          {c.machineName} 基底
                        </label>
                      )}
                    </th>
                  )
                })}
              </tr>
            </thead>
            <tbody>
              {matrix.rows.map((r) => (
                <tr key={r.key}>
                  <td>
                    <div>{r.label}</div>
                    <div className="wb-hint wb-mono">
                      {r.key}
                      {r.unit ? ` (${r.unit})` : ''}
                    </div>
                  </td>
                  {r.cells.map((cell, i) => (
                    <td
                      key={`${r.key}-${matrix.columns[i].machineId}/${matrix.columns[i].versionId}`}
                      className={cellClass(cell)}
                    >
                      {fmt(cell.value)}
                      {cell.origin === 'override' && (
                        <span className="wb-tag wb-tag--own">覆盖</span>
                      )}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <section className="wb-section">
        <h3 className="wb-section-title">批量修改</h3>
        <div className="wb-row">
          <select
            className="wb-input"
            value={fieldKey}
            onChange={(e) => {
              setFieldKey(e.target.value)
              setEffects(null)
            }}
          >
            <option value="">选字段…</option>
            {fields.map((f) => (
              <option key={f.key} value={f.key}>
                {f.label}（{f.key}）
              </option>
            ))}
          </select>
          <input
            className="wb-input"
            placeholder="新值（布尔填 true / false）"
            value={raw}
            onChange={(e) => {
              setRaw(e.target.value)
              setEffects(null)
            }}
          />
          <button
            type="button"
            className="wb-btn"
            disabled={!fieldKey || parsed === null || targets.length === 0}
            onClick={() => void run(false)}
          >
            预览影响范围
          </button>
          <span className="wb-hint">
            已勾 {picked.length} 个版本 + {baseTargets.length} 个基底
          </span>
        </div>

        {targets.length === 0 && (
          <p className="wb-hint">
            先在表头勾选目标列。**没有"一键应用到所有机型"** —— 跨机型也要一列一列勾。
          </p>
        )}

        {effects && (
          <div className="wb-stack">
            <table className="wb-table">
              <thead>
                <tr>
                  <th>目标</th>
                  <th>现在</th>
                  <th>之后</th>
                  <th>会发生什么</th>
                </tr>
              </thead>
              <tbody>
                {effects.map((e, i) => (
                  <tr key={i} className={e.kind === 'notApplicable' ? 'wb-row--na' : undefined}>
                    <td>{e.label}</td>
                    <td className="wb-mono">{fmt(e.before)}</td>
                    <td className="wb-mono">{fmt(e.after)}</td>
                    <td>{explainBulk(e)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
            <div className="wb-row">
              <button
                type="button"
                className="wb-btn wb-btn--primary"
                onClick={() => void run(true)}
              >
                确认应用
              </button>
              <button type="button" className="wb-btn" onClick={() => setEffects(null)}>
                取消
              </button>
            </div>
          </div>
        )}
      </section>
    </div>
  )
}

function explainBulk(e: BulkEffect): string {
  switch (e.kind) {
    case 'createsOverride':
      return '这个版本原来继承基底，改完会多出一条覆盖'
    case 'updatesOverride':
      return '更新已有的覆盖'
    case 'updatesBase':
      return '改机型基底（所有继承该字段的版本都会跟着变）'
    case 'noChange':
      return '值没变，跳过'
    case 'notApplicable':
      return '该机型基底里没这个字段，跳过（否则会造出一条不适用的覆盖）'
  }
}

function cellClass(cell: MatrixCell): string | undefined {
  if (cell.origin === undefined) return 'wb-row--na'
  return undefined
}

function fmt(v: ParamValue | undefined): string {
  if (v === undefined) return '—'
  if (typeof v === 'string') return v.includes('\n') ? `${v.split('\n')[0]} …` : v
  return String(v)
}
