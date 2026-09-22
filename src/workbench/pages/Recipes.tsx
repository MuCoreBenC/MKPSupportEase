/**
 * 配方编辑页。
 *
 * 这一页要同时说清三件事，所以每个字段行上有三处信息：
 * - **值**：可编辑
 * - **来源**：继承自机型基底，还是本版覆盖
 * - **清除覆盖后会变成什么**：只有覆盖行才有，避免"删了覆盖才知道基底是多少"
 *
 * 编辑目标有两个，用一组单选切换，**刻意不做成同一张表里混着改**：
 * - 改版本 → 创建/更新版本覆盖，不动机型基底
 * - 改机型基底 → 只动基底，会影响所有继承该字段的版本
 *
 * 混在一起改是最容易出事的操作（"我只想调这个版本"却改到了全机型），所以界面上
 * 当前在改哪一层必须一眼看见。Rust 侧也把两条写入路径分开了，类型上互相碰不到。
 */
import { useCallback, useEffect, useMemo, useState } from 'react'

import {
  describeError,
  wb,
  type DraftView,
  type EffectiveView,
  type FieldDef,
  type Params,
  type ParamValue,
  type Registry,
} from '../api'
import type { Selection } from '../App'

type Target = 'version' | 'base'

export function RecipesPage({
  registry,
  selection,
  onTreeChanged,
}: {
  registry: Registry | null
  selection: Selection
  onTreeChanged: () => void
}) {
  const [view, setView] = useState<EffectiveView | null>(null)
  const [target, setTarget] = useState<Target>('version')
  const [edits, setEdits] = useState<Params>({})
  const [displayName, setDisplayName] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [saved, setSaved] = useState<string | null>(null)
  /** 草稿已自动恢复时的提示 */
  const [restoredAt, setRestoredAt] = useState<string | null>(null)
  /** 草稿与当前配方对不上时，等人来选。**不静默套用也不静默丢弃** */
  const [stale, setStale] = useState<DraftView | null>(null)

  /** 从后端那份重置本地编辑态。保存成功后也走这里 —— 保证屏幕上的东西就是盘上的东西 */
  const adopt = useCallback((v: EffectiveView, keepTarget: Target) => {
    setView(v)
    setDisplayName(v.version.displayName)
    setEdits(keepTarget === 'version' ? { ...v.version.overrides } : { ...v.machine.base })
  }, [])

  useEffect(() => {
    setError(null)
    setSaved(null)
    setRestoredAt(null)
    setStale(null)
    void (async () => {
      try {
        const v = await wb.effective(selection.machineId, selection.versionId)
        adopt(v, target)

        // 草稿：关掉再开还在那个状态
        const di =
          target === 'version'
            ? await wb.draft(selection.machineId, selection.versionId)
            : await wb.baseDraft(selection.machineId)
        if (di.draft) {
          if (di.matchesCurrent) {
            setEdits({ ...di.draft.overrides })
            setRestoredAt(di.draft.savedAt)
          } else {
            setStale(di)
          }
        }
      } catch (e) {
        setView(null)
        setError(describeError(e))
      }
    })()
    // target 变化时也要重置 edits，所以它在依赖里
  }, [selection.machineId, selection.versionId, target, adopt])

  const original: Params = useMemo(() => {
    if (!view) return {}
    return target === 'version' ? view.version.overrides : view.machine.base
  }, [view, target])

  const dirty =
    JSON.stringify(edits) !== JSON.stringify(original) ||
    (view ? displayName !== view.version.displayName : false)

  const sections = useMemo(() => groupBySection(registry), [registry])

  /* 参数一改就写草稿（防抖 400ms）。
     **草稿只记参数值，不记版本显示名** —— 显示名是一个输入框的内容，
     丢了重打一遍就行；参数值可能是调了半小时的结果。这个不对称是刻意的，
     不是漏了。 */
  const draftToken = view ? (target === 'version' ? view.effective.hash : view.machineHash) : null
  useEffect(() => {
    if (!view || !draftToken || !dirty || stale) return
    const timer = setTimeout(() => {
      void (async () => {
        try {
          if (target === 'version') {
            await wb.saveDraft({
              machineId: selection.machineId,
              versionId: selection.versionId,
              baseHash: draftToken,
              overrides: edits,
            })
          } else {
            await wb.saveBaseDraft({
              machineId: selection.machineId,
              baseHash: draftToken,
              overrides: edits,
            })
          }
        } catch (e) {
          // 草稿写失败不该打断编辑，但也不能装作没发生
          setError(`草稿没写上：${describeError(e)}`)
        }
      })()
    }, 400)
    return () => clearTimeout(timer)
  }, [edits, dirty, stale, view, draftToken, target, selection.machineId, selection.versionId])

  const dropDraft = async () => {
    if (target === 'version') {
      await wb.clearDraft(selection.machineId, selection.versionId)
    } else {
      await wb.clearBaseDraft(selection.machineId)
    }
  }

  const discard = async () => {
    if (!view) return
    adopt(view, target)
    setRestoredAt(null)
    setStale(null)
    try {
      await dropDraft()
    } catch (e) {
      setError(describeError(e))
    }
  }

  const save = async () => {
    if (!view) return
    setError(null)
    setSaved(null)
    try {
      if (target === 'version') {
        const next = await wb.saveVersion({
          machineId: selection.machineId,
          versionId: selection.versionId,
          displayName,
          overrides: edits,
          expectedHash: view.effective.hash,
        })
        adopt(next, target)
        onTreeChanged()
      } else {
        await wb.saveMachineBase({
          machineId: selection.machineId,
          base: edits,
          expectedMachineHash: view.machineHash,
        })
        // 基底一改，所有版本的有效配方都可能变，所以整份重读
        const next = await wb.effective(selection.machineId, selection.versionId)
        adopt(next, target)
        onTreeChanged()
      }
      setSaved('配方已保存。注意：这还没有生成 TOML。')
      setRestoredAt(null)
      setStale(null)
    } catch (e) {
      setError(describeError(e))
    }
  }

  if (error && !view) return <p className="wb-banner wb-banner--bad">{error}</p>
  if (!view || !registry) return <p className="wb-placeholder">读取中…</p>

  return (
    <div className="wb-stack">
      <div className="wb-row">
        <label className="wb-field-inline">
          版本显示名
          <input
            className="wb-input"
            value={displayName}
            onChange={(e) => setDisplayName(e.target.value)}
            disabled={target === 'base'}
          />
        </label>
        <span className="wb-hint wb-mono">
          {view.machine.id} / {view.version.id} · 指纹 {view.effective.hash.slice(0, 12)}
        </span>
      </div>

      <div className="wb-row">
        <span className="wb-hint">编辑目标</span>
        <label className="wb-radio">
          <input
            type="radio"
            checked={target === 'version'}
            onChange={() => setTarget('version')}
          />
          这个版本的覆盖值
        </label>
        <label className="wb-radio">
          <input type="radio" checked={target === 'base'} onChange={() => setTarget('base')} />
          机型基底（影响所有继承该字段的版本）
        </label>
      </div>

      {target === 'base' && (
        <div className="wb-banner wb-banner--warn">
          现在改的是 {view.machine.displayName} 的基底。所有没覆盖这些字段的版本都会跟着变。
        </div>
      )}

      {/* 三件事分别指示，不合并成一句"已同步"：草稿恢复 ≠ 配方已保存 ≠ TOML 已生成 */}
      {restoredAt && (
        <div className="wb-banner">
          有未保存的修改已恢复（草稿写于 {restoredAt}）。它**还没有保存**，也没有生成 TOML。
        </div>
      )}

      {stale && stale.draft && (
        <div className="wb-banner wb-banner--warn">
          找到一份草稿（写于 {stale.draft.savedAt}），但这份配方在那之后被改过，
          两者对不上。要用哪一份？
          <span className="wb-row">
            <button
              type="button"
              className="wb-btn"
              onClick={() => {
                setEdits({ ...stale.draft!.overrides })
                setRestoredAt(stale.draft!.savedAt)
                setStale(null)
              }}
            >
              套用草稿
            </button>
            <button type="button" className="wb-btn" onClick={() => void discard()}>
              丢弃草稿
            </button>
          </span>
        </div>
      )}

      {view.unconfigured && (
        <div className="wb-banner wb-banner--warn">
          这个版本还没写配方（机型基底和版本覆盖都是空的）。这不是"生成失败"，是还没开始写。
        </div>
      )}

      {view.effective.orphans.length > 0 && (
        <div className="wb-banner wb-banner--warn">
          有 {view.effective.orphans.length} 个覆盖值在当前机型上不适用（基底里没有这些字段）：
          <span className="wb-mono"> {view.effective.orphans.join('、')}</span>
          。值原样留着不丢，但不会写进 TOML。
        </div>
      )}

      {view.effective.unknownKeys.length > 0 && (
        <div className="wb-banner wb-banner--bad">
          有 {view.effective.unknownKeys.length} 个覆盖用了字段定义里没有的 key：
          <span className="wb-mono"> {view.effective.unknownKeys.join('、')}</span>
          。这多半是手改过文件或字段被删了。
        </div>
      )}

      {sections.map(([section, fields]) => (
        <section key={section} className="wb-section">
          <h3 className="wb-section-title">{section}</h3>
          <table className="wb-table">
            <thead>
              <tr>
                <th>字段</th>
                <th>值</th>
                <th>来源</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {fields.map((f) => {
                const applicable = target === 'base' || f.key in view.machine.base
                const eff = view.effective.values[f.key]
                const edited = f.key in edits
                const shown: ParamValue | undefined = edited
                  ? edits[f.key]
                  : target === 'base'
                    ? undefined
                    : eff?.value

                return (
                  <tr key={f.key} className={applicable ? undefined : 'wb-row--na'}>
                    <td>
                      <div>{f.label}</div>
                      <div className="wb-hint wb-mono">{f.key}</div>
                    </td>
                    <td>
                      <ValueInput
                        field={f}
                        value={shown}
                        onChange={(v) =>
                          setEdits((prev) =>
                            v === undefined
                              ? omit(prev, f.key)
                              : { ...prev, [f.key]: v },
                          )
                        }
                      />
                      {f.unit && <span className="wb-hint"> {f.unit}</span>}
                    </td>
                    <td>
                      {target === 'base' ? (
                        <span className="wb-tag">{edited ? '基底有值' : '基底未设'}</span>
                      ) : edited ? (
                        <span className="wb-tag wb-tag--own">本版覆盖</span>
                      ) : eff ? (
                        <span className="wb-tag">继承自 {view.machine.displayName}</span>
                      ) : (
                        <span className="wb-tag wb-tag--na">该机型不适用</span>
                      )}
                      {/* 覆盖行显示"清掉会回到多少"，免得删了才知道 */}
                      {target === 'version' && edited && f.key in view.machine.base && (
                        <div className="wb-hint wb-mono">
                          清除后 → {format(view.machine.base[f.key])}
                        </div>
                      )}
                    </td>
                    <td>
                      {edited && (
                        <button
                          type="button"
                          className="wb-link"
                          onClick={() => setEdits((prev) => omit(prev, f.key))}
                        >
                          {target === 'base' ? '从基底移除' : '清除覆盖'}
                        </button>
                      )}
                    </td>
                  </tr>
                )
              })}
            </tbody>
          </table>
        </section>
      ))}

      <div className="wb-row wb-sticky-actions">
        <button type="button" className="wb-btn wb-btn--primary" disabled={!dirty} onClick={save}>
          保存
        </button>
        <button
          type="button"
          className="wb-btn"
          disabled={!dirty}
          onClick={() => void discard()}
        >
          放弃修改
        </button>
        {dirty && <span className="wb-hint">有未保存的修改</span>}
        {saved && <span className="wb-hint wb-hint--ok">{saved}</span>}
        {error && <span className="wb-hint wb-hint--bad">{error}</span>}
      </div>
    </div>
  )
}

/* ---------- 小件 ---------- */

function ValueInput({
  field,
  value,
  onChange,
}: {
  field: FieldDef
  value: ParamValue | undefined
  onChange: (v: ParamValue | undefined) => void
}) {
  if (field.valueType === 'bool') {
    return (
      <input
        type="checkbox"
        checked={value === true}
        onChange={(e) => onChange(e.target.checked)}
      />
    )
  }

  if (field.valueType === 'gcode') {
    return (
      <textarea
        className="wb-textarea wb-mono"
        rows={4}
        value={value === undefined ? '' : String(value)}
        onChange={(e) => onChange(e.target.value)}
      />
    )
  }

  if (field.valueType === 'float' || field.valueType === 'int') {
    return (
      <input
        className="wb-input wb-input--num"
        type="number"
        step={field.step ?? (field.valueType === 'int' ? 1 : 'any')}
        min={field.min}
        max={field.max}
        value={value === undefined ? '' : String(value)}
        onChange={(e) => {
          const t = e.target.value
          if (t === '') {
            onChange(undefined)
            return
          }
          const n = Number(t)
          // NaN 不往上传：把"输入中间态"当成值写进配方，会在 hash 上留下垃圾
          if (!Number.isNaN(n)) onChange(field.valueType === 'int' ? Math.round(n) : n)
        }}
      />
    )
  }

  return (
    <input
      className="wb-input"
      value={value === undefined ? '' : String(value)}
      onChange={(e) => onChange(e.target.value)}
    />
  )
}

function groupBySection(registry: Registry | null): [string, FieldDef[]][] {
  if (!registry) return []
  const map = new Map<string, FieldDef[]>()
  for (const f of registry.fields.slice().sort((a, b) => a.order - b.order)) {
    const list = map.get(f.section) ?? []
    list.push(f)
    map.set(f.section, list)
  }
  return [...map.entries()]
}

function omit(p: Params, key: string): Params {
  const next = { ...p }
  delete next[key]
  return next
}

function format(v: ParamValue | undefined): string {
  if (v === undefined) return '（无）'
  if (typeof v === 'string') return v.includes('\n') ? `${v.split('\n')[0]} …` : v
  return String(v)
}
