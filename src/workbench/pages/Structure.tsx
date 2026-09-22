/**
 * 结构操作：新建、克隆、改名、换机型、删除、回收站。
 *
 * 为什么和配方编辑分开一页：这些操作改的是**配方本的目录结构**，不是参数值。
 * 混在参数表旁边，"删除"会离"清除覆盖"太近。
 *
 * 三条界面上的硬规矩：
 * - 克隆预填「XX 副本」，可改；**重名当场提示，不自动加后缀**
 * - 换机型必须先看预览再确认；值变了用橙色，**不用红色**（那不是错误）
 * - 彻底删除要二次确认；删除本身进回收站，可还原
 */
import { useCallback, useEffect, useMemo, useState } from 'react'

import {
  describeError,
  wb,
  type MachineNode,
  type MoveChange,
  type MovePreview,
  type ParamValue,
  type TrashEntry,
} from '../api'
import type { Selection } from '../App'

export function StructurePage({
  tree,
  selection,
  onChanged,
  onSelect,
}: {
  tree: MachineNode[]
  selection: Selection | null
  onChanged: () => void
  onSelect: (s: Selection) => void
}) {
  const [msg, setMsg] = useState<string | null>(null)
  const [err, setErr] = useState<string | null>(null)
  const [trash, setTrash] = useState<TrashEntry[]>([])
  const [purging, setPurging] = useState<string | null>(null)

  const reloadTrash = useCallback(async () => {
    try {
      setTrash(await wb.trashList())
    } catch (e) {
      setErr(describeError(e))
    }
  }, [])

  useEffect(() => {
    void reloadTrash()
  }, [reloadTrash])

  const run = async (label: string, fn: () => Promise<string>) => {
    setMsg(null)
    setErr(null)
    try {
      setMsg(await fn())
      onChanged()
      await reloadTrash()
    } catch (e) {
      setErr(`${label}：${describeError(e)}`)
    }
  }

  const current = tree.find((m) => m.id === selection?.machineId)
  const currentVersion = current?.versions.find((v) => v.id === selection?.versionId)

  return (
    <div className="wb-stack">
      {msg && <div className="wb-banner">{msg}</div>}
      {err && <div className="wb-banner wb-banner--bad">{err}</div>}

      <NewMachine onDone={(id, name) => run('新建机型', async () => {
        await wb.createMachine(id, name)
        return `机型 ${id} 已建好。基底是空的 —— 参数要你自己写。`
      })} />

      <NewVersion
        machines={tree}
        defaultMachine={selection?.machineId ?? tree[0]?.id ?? ''}
        onDone={(machineId, id, name) =>
          run('新建版本', async () => {
            await wb.createVersion(machineId, id, name)
            onSelect({ machineId, versionId: id })
            return `${machineId}/${id} 已建好，当前是「未配置」 —— 这是真实状态，不是出错。`
          })
        }
      />

      {selection && currentVersion ? (
        <section className="wb-section">
          <h3 className="wb-section-title">
            当前：{current?.displayName} / {currentVersion.displayName}
          </h3>

          <CloneVersion
            defaultId={`${selection.versionId}-copy`}
            defaultName={`${currentVersion.displayName} 副本`}
            onDone={(newId, newDisplayName) =>
              run('克隆', async () => {
                await wb.cloneVersion({ ...selection, newId, newDisplayName })
                onSelect({ machineId: selection.machineId, versionId: newId })
                return `已克隆为 ${newId}。`
              })
            }
          />

          <RenameVersion
            defaultId={selection.versionId}
            defaultName={currentVersion.displayName}
            onDone={(newId, newDisplayName) =>
              run('改名', async () => {
                await wb.renameVersion({ ...selection, newId, newDisplayName })
                onSelect({ machineId: selection.machineId, versionId: newId })
                return `已改名。菜单里指向它的条目跟着改了，presetId 没动。`
              })
            }
          />

          <MoveVersion
            selection={selection}
            machines={tree}
            onDone={(toMachineId) =>
              run('换机型', async () => {
                await wb.moveVersion(selection.machineId, selection.versionId, toMachineId)
                onSelect({ machineId: toMachineId, versionId: selection.versionId })
                return `已移到 ${toMachineId}。覆盖值一个没丢。`
              })
            }
          />

          <div className="wb-row">
            <button
              type="button"
              className="wb-btn"
              onClick={() =>
                void run('删除', async () => {
                  const r = await wb.trashVersion(selection.machineId, selection.versionId)
                  const detached =
                    r.detachedPresets.length > 0
                      ? ` 同时从菜单与套餐里摘掉了：${r.detachedPresets.join('、')}。`
                      : ''
                  return `已进回收站，可还原。${detached}`
                })
              }
            >
              删除这个版本（进回收站）
            </button>
            <span className="wb-hint">
              进回收站后它的 TOML 立即从菜单里移除，不会还被当成有效交付物。
            </span>
          </div>
        </section>
      ) : (
        <p className="wb-placeholder">左边选一个版本，才能克隆/改名/换机型/删除。</p>
      )}

      <section className="wb-section">
        <h3 className="wb-section-title">回收站</h3>
        {trash.length === 0 ? (
          <p className="wb-placeholder">空的。</p>
        ) : (
          <table className="wb-table">
            <thead>
              <tr>
                <th>版本</th>
                <th>删除时间</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {trash.map((t) => (
                <tr key={t.file}>
                  <td className="wb-mono">
                    {t.machineId} / {t.versionId}
                  </td>
                  <td className="wb-mono">{t.deletedStamp}</td>
                  <td>
                    <button
                      type="button"
                      className="wb-link"
                      onClick={() =>
                        void run('还原', async () => {
                          const v = await wb.restoreFromTrash(t.file)
                          onSelect({ machineId: v.machineId, versionId: v.id })
                          return `已还原 ${v.machineId}/${v.id}。它没有自动上架 —— 要交付得自己加回菜单。`
                        })
                      }
                    >
                      还原
                    </button>
                    {purging === t.file ? (
                      <>
                        <span className="wb-hint"> 确定彻底删除？</span>
                        <button
                          type="button"
                          className="wb-link"
                          onClick={() =>
                            void run('彻底删除', async () => {
                              await wb.purgeFromTrash(t.file)
                              setPurging(null)
                              return '已彻底删除。'
                            })
                          }
                        >
                          确定
                        </button>
                        <button type="button" className="wb-link" onClick={() => setPurging(null)}>
                          取消
                        </button>
                      </>
                    ) : (
                      <button
                        type="button"
                        className="wb-link"
                        onClick={() => setPurging(t.file)}
                      >
                        彻底删除
                      </button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </div>
  )
}

/* ---------- 小表单 ---------- */

function NewMachine({ onDone }: { onDone: (id: string, name: string) => void }) {
  const [id, setId] = useState('')
  const [name, setName] = useState('')
  return (
    <div className="wb-row">
      <span className="wb-hint">新建机型</span>
      <input
        className="wb-input"
        placeholder="id（字母数字 - _）"
        value={id}
        onChange={(e) => setId(e.target.value)}
      />
      <input
        className="wb-input"
        placeholder="显示名"
        value={name}
        onChange={(e) => setName(e.target.value)}
      />
      <button
        type="button"
        className="wb-btn"
        disabled={!id.trim()}
        onClick={() => {
          onDone(id.trim(), name.trim() || id.trim())
          setId('')
          setName('')
        }}
      >
        建
      </button>
    </div>
  )
}

function NewVersion({
  machines,
  defaultMachine,
  onDone,
}: {
  machines: MachineNode[]
  defaultMachine: string
  onDone: (machineId: string, id: string, name: string) => void
}) {
  const [machineId, setMachineId] = useState(defaultMachine)
  const [id, setId] = useState('')
  const [name, setName] = useState('')

  useEffect(() => setMachineId(defaultMachine), [defaultMachine])

  return (
    <div className="wb-row">
      <span className="wb-hint">新建版本</span>
      <select className="wb-input" value={machineId} onChange={(e) => setMachineId(e.target.value)}>
        {machines.map((m) => (
          <option key={m.id} value={m.id}>
            {m.displayName}
          </option>
        ))}
      </select>
      <input
        className="wb-input"
        placeholder="id"
        value={id}
        onChange={(e) => setId(e.target.value)}
      />
      <input
        className="wb-input"
        placeholder="显示名"
        value={name}
        onChange={(e) => setName(e.target.value)}
      />
      <button
        type="button"
        className="wb-btn"
        disabled={!machineId || !id.trim()}
        onClick={() => {
          onDone(machineId, id.trim(), name.trim() || id.trim())
          setId('')
          setName('')
        }}
      >
        建
      </button>
    </div>
  )
}

function CloneVersion({
  defaultId,
  defaultName,
  onDone,
}: {
  defaultId: string
  defaultName: string
  onDone: (id: string, name: string) => void
}) {
  const [id, setId] = useState(defaultId)
  const [name, setName] = useState(defaultName)

  // 选中的版本一换，预填也要换
  useEffect(() => {
    setId(defaultId)
    setName(defaultName)
  }, [defaultId, defaultName])

  return (
    <div className="wb-row">
      <span className="wb-hint">克隆为</span>
      <input className="wb-input" value={id} onChange={(e) => setId(e.target.value)} />
      <input className="wb-input" value={name} onChange={(e) => setName(e.target.value)} />
      <button type="button" className="wb-btn" disabled={!id.trim()} onClick={() => onDone(id.trim(), name.trim())}>
        克隆
      </button>
      <span className="wb-hint">重名会被拒绝（不自动加后缀）</span>
    </div>
  )
}

function RenameVersion({
  defaultId,
  defaultName,
  onDone,
}: {
  defaultId: string
  defaultName: string
  onDone: (id: string, name: string) => void
}) {
  const [id, setId] = useState(defaultId)
  const [name, setName] = useState(defaultName)

  useEffect(() => {
    setId(defaultId)
    setName(defaultName)
  }, [defaultId, defaultName])

  return (
    <div className="wb-row">
      <span className="wb-hint">改名</span>
      <input className="wb-input" value={id} onChange={(e) => setId(e.target.value)} />
      <input className="wb-input" value={name} onChange={(e) => setName(e.target.value)} />
      <button type="button" className="wb-btn" disabled={!id.trim()} onClick={() => onDone(id.trim(), name.trim())}>
        改
      </button>
    </div>
  )
}

function MoveVersion({
  selection,
  machines,
  onDone,
}: {
  selection: Selection
  machines: MachineNode[]
  onDone: (toMachineId: string) => void
}) {
  const others = useMemo(
    () => machines.filter((m) => m.id !== selection.machineId),
    [machines, selection.machineId],
  )
  /* 依赖用"id 串"而不是数组本身：others 每次渲染都是新数组，
     直接放依赖里会让这个 effect 每帧都跑，把刚拉到的预览清掉 */
  const otherIds = useMemo(() => others.map((m) => m.id).join(','), [others])
  const [to, setTo] = useState(others[0]?.id ?? '')
  const [preview, setPreview] = useState<MovePreview | null>(null)
  const [err, setErr] = useState<string | null>(null)

  useEffect(() => {
    setPreview(null)
    setTo(otherIds.split(',')[0] ?? '')
  }, [selection.machineId, selection.versionId, otherIds])

  return (
    <div className="wb-stack">
      <div className="wb-row">
        <span className="wb-hint">换机型到</span>
        <select className="wb-input" value={to} onChange={(e) => setTo(e.target.value)}>
          {others.map((m) => (
            <option key={m.id} value={m.id}>
              {m.displayName}
            </option>
          ))}
        </select>
        <button
          type="button"
          className="wb-btn"
          disabled={!to}
          onClick={() => {
            setErr(null)
            void wb
              .previewMove(selection.machineId, selection.versionId, to)
              .then(setPreview)
              .catch((e) => setErr(describeError(e)))
          }}
        >
          先看变化
        </button>
      </div>

      {err && <div className="wb-banner wb-banner--bad">{err}</div>}

      {preview && (
        <div className="wb-stack">
          {preview.nameTaken && (
            <div className="wb-banner wb-banner--bad">
              {preview.toMachine} 下已经有同名版本，换不过去 —— 先给它改个名。
            </div>
          )}
          <div className="wb-banner wb-banner--warn">
            {preview.fromMachine} → {preview.toMachine}，共 {preview.changes.length} 处变化。
            <strong>橙色表示值变了，不是出错。</strong>
          </div>
          {preview.changes.length > 0 && (
            <table className="wb-table">
              <thead>
                <tr>
                  <th>字段</th>
                  <th>现在</th>
                  <th>之后</th>
                  <th>说明</th>
                </tr>
              </thead>
              <tbody>
                {preview.changes.map((c) => (
                  <tr key={c.key} className={c.kind === 'overrideKept' ? undefined : 'wb-row--warn'}>
                    <td>
                      <div>{c.label}</div>
                      <div className="wb-hint wb-mono">{c.key}</div>
                    </td>
                    <td className="wb-mono">{fmt(c.from)}</td>
                    <td className="wb-mono">{fmt(c.to)}</td>
                    <td>{explain(c)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
          <div className="wb-row">
            <button
              type="button"
              className="wb-btn wb-btn--primary"
              disabled={preview.nameTaken}
              onClick={() => onDone(to)}
            >
              确认换机型
            </button>
            <button type="button" className="wb-btn" onClick={() => setPreview(null)}>
              取消
            </button>
          </div>
        </div>
      )}
    </div>
  )
}

function explain(c: MoveChange): string {
  switch (c.kind) {
    case 'inheritedChanged':
      return '继承值变了（新机型的基底不同）'
    case 'overrideKept':
      return '本版覆盖，原样保留'
    case 'becomesInapplicable':
      return '新机型基底里没这个字段 → 值留着，但不写进 TOML'
    case 'becomesApplicable':
      return '新机型才有这个字段 → 从此可用'
  }
}

function fmt(v: ParamValue | undefined): string {
  if (v === undefined) return '（无）'
  if (typeof v === 'string') return v.includes('\n') ? `${v.split('\n')[0]} …` : v
  return String(v)
}
