/**
 * 预设 / 资源管理：左边 MKP TOML，右边 BBS JSON。
 *
 * 两栏刻意**不共用编辑流程**（doc §10）：
 * - TOML 由开发配方生成，只能重烤，有生成状态
 * - BBS 是外部导入，只能收货/贴标/决定配给谁，没有"生成状态"这回事
 *
 * 状态那一列是每个版本各自的，不是全店一个 —— 因为你不是每次都全量生成。
 * 用词是「已生成」不是「已同步」：工作台只知道本地产物对不对应当前配方，
 * 不知道云端发布了没、客户端更新了没。
 */
import { useCallback, useEffect, useState } from 'react'

import {
  describeError,
  wb,
  type BbsFile,
  type MachineNode,
  type VersionStatus,
} from '../api'

export function ResourcesPage({
  tree,
  onChanged,
}: {
  tree: MachineNode[]
  onChanged: () => void
}) {
  const [status, setStatus] = useState<VersionStatus[]>([])
  const [files, setFiles] = useState<BbsFile[]>([])
  const [msg, setMsg] = useState<string | null>(null)
  const [err, setErr] = useState<string | null>(null)
  const [importId, setImportId] = useState('')
  const [importPath, setImportPath] = useState('')

  const reload = useCallback(async () => {
    try {
      setStatus(await wb.status())
      setFiles(await wb.bbsList())
    } catch (e) {
      setErr(describeError(e))
    }
  }, [])

  useEffect(() => {
    void reload()
  }, [reload])

  const act = async (label: string, fn: () => Promise<string>) => {
    setMsg(null)
    setErr(null)
    try {
      setMsg(await fn())
      await reload()
      onChanged()
    } catch (e) {
      setErr(`${label}：${describeError(e)}`)
    }
  }

  return (
    <div className="wb-stack">
      {msg && <div className="wb-banner">{msg}</div>}
      {err && <div className="wb-banner wb-banner--bad">{err}</div>}

      <section className="wb-section">
        <h3 className="wb-section-title">MKP TOML（由开发配方生成）</h3>
        <table className="wb-table">
          <thead>
            <tr>
              <th>版本</th>
              <th>presetId</th>
              <th>状态</th>
              <th>上次成功生成</th>
              <th>产物</th>
              <th>最低客户端</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {status.map((s) => (
              <tr key={`${s.machineId}/${s.versionId}`}>
                <td>
                  {s.machineName} / {s.versionName}
                </td>
                <td className="wb-mono">{s.presetId}</td>
                <td>
                  <StateTag s={s} />
                  {s.lastFailure && (
                    <div className="wb-hint wb-hint--bad">
                      上次生成失败：{s.lastFailure.reason}（旧产物没被动过）
                    </div>
                  )}
                  {s.orphanCount > 0 && (
                    <div className="wb-hint">{s.orphanCount} 个覆盖在本机型不适用</div>
                  )}
                </td>
                <td className="wb-mono">{s.lastGeneratedAt ?? '—'}</td>
                <td>
                  {!s.outputPresent ? (
                    <span className="wb-tag wb-tag--warn">文件不在</span>
                  ) : s.outputMatches ? (
                    <span className="wb-tag">字节一致</span>
                  ) : (
                    <span className="wb-tag wb-tag--warn">被改过</span>
                  )}
                </td>
                <td className="wb-mono">{s.minClientVersion ?? '—'}</td>
                <td>
                  <button
                    type="button"
                    className="wb-link"
                    onClick={() =>
                      void act('生成', async () => {
                        const r = await wb.generateOne(s.machineId, s.versionId)
                        if (r.failed.length > 0) return `没成：${r.failed[0].reason}`
                        const o = r.generated[0]
                        return o.unchanged
                          ? '产物字节没变（所以没重写文件）。'
                          : `已生成，最低客户端 ${o.minClientVersion}。注意：这还没有发布。`
                      })
                    }
                  >
                    生成
                  </button>
                  {s.lastGeneratedAt && (
                    <button
                      type="button"
                      className="wb-link"
                      onClick={() =>
                        void act('恢复配方', async () => {
                          const r = await wb.restoreRecipe(s.machineId, s.versionId)
                          const base =
                            r.baseDiffers.length > 0
                              ? ` 机型基底有 ${r.baseDiffers.length} 处与当时不同，**没有**恢复（它是共享的）。`
                              : ''
                          return `已恢复 ${r.restoredOverrides} 个覆盖值到 ${r.snapshotGeneratedAt} 那一版。${base}恢复不等于重新生成 —— 还要点一下生成。`
                        })
                      }
                    >
                      恢复上次生成时的配方
                    </button>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>

      <section className="wb-section">
        <h3 className="wb-section-title">BBS JSON（外部资源，只收录不编辑）</h3>

        <div className="wb-row">
          <input
            className="wb-input"
            placeholder="id（如 BBS-01）"
            value={importId}
            onChange={(e) => setImportId(e.target.value)}
          />
          <input
            className="wb-input"
            placeholder="源文件的完整路径"
            value={importPath}
            onChange={(e) => setImportPath(e.target.value)}
          />
          <button
            type="button"
            className="wb-btn"
            disabled={!importId.trim() || !importPath.trim()}
            onClick={() =>
              void act('收录', async () => {
                await wb.bbsImport(importId.trim(), importPath.trim())
                setImportId('')
                setImportPath('')
                return '已收录，字节原样存放。它默认是「仅归档」—— 要交付得先加进菜单。'
              })
            }
          >
            收录
          </button>
          <span className="wb-hint">
            粘路径而不是弹文件框：本仓库没装 tauri 的 dialog 插件，为工作台单独引一个会连带
            改客户端的权限清单。
          </span>
        </div>

        {files.length === 0 ? (
          <p className="wb-placeholder">
            仓库里还没有 BBS 文件。（`mkpse-presets/presets/bbs/` 当前实测也是空的，
            所以这里不放假样本。）
          </p>
        ) : (
          <table className="wb-table">
            <thead>
              <tr>
                <th>id</th>
                <th>三态</th>
                <th>谁在用</th>
                <th>大小</th>
                <th>JSON</th>
              </tr>
            </thead>
            <tbody>
              {files.map((f) => (
                <tr key={f.id}>
                  <td className="wb-mono">{f.id}</td>
                  <td>
                    <span
                      className={
                        f.state === 'archivedOnly' ? 'wb-tag wb-tag--na' : 'wb-tag wb-tag--own'
                      }
                    >
                      {f.state === 'assigned'
                        ? '已分配'
                        : f.state === 'optional'
                          ? '可选'
                          : '仅归档'}
                    </span>
                  </td>
                  <td className="wb-mono">{f.usedBy.join('、') || '—'}</td>
                  <td className="wb-mono">{f.size}</td>
                  <td>
                    {f.parses ? (
                      <span className="wb-tag">可解析</span>
                    ) : (
                      <span className="wb-tag wb-tag--warn">解析不通</span>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}

        <h4 className="wb-section-title">机型默认清单</h4>
        {tree.map((m) => (
          <MachineBbs
            key={m.id}
            machine={m}
            files={files}
            onSave={(ids) =>
              void act('设置机型默认 BBS', async () => {
                await wb.setMachineBbs(m.id, ids)
                return `${m.displayName} 的默认 BBS 已更新。没覆盖的版本跟着变。`
              })
            }
          />
        ))}
      </section>
    </div>
  )
}

function StateTag({ s }: { s: VersionStatus }) {
  if (s.state === 'unconfigured') {
    return <span className="wb-tag wb-tag--warn">未配置（还没写配方）</span>
  }
  if (s.state === 'stale') return <span className="wb-tag wb-tag--warn">待生成</span>
  return <span className="wb-tag">已生成</span>
}

function MachineBbs({
  machine,
  files,
  onSave,
}: {
  machine: MachineNode
  files: BbsFile[]
  onSave: (ids: string[]) => void
}) {
  const [picked, setPicked] = useState<string[]>([])
  return (
    <div className="wb-row">
      <span className="wb-hint">{machine.displayName}</span>
      {files.length === 0 && <span className="wb-hint">（没有可选的 BBS）</span>}
      {files.map((f) => (
        <label key={f.id} className="wb-radio">
          <input
            type="checkbox"
            checked={picked.includes(f.id)}
            onChange={(e) =>
              setPicked(e.target.checked ? [...picked, f.id] : picked.filter((x) => x !== f.id))
            }
          />
          {f.id}
        </label>
      ))}
      {files.length > 0 && (
        <button type="button" className="wb-btn" onClick={() => onSave(picked)}>
          设为默认
        </button>
      )}
    </div>
  )
}
