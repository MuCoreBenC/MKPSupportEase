/**
 * 菜单与套餐 —— **两份文件，分开管**（doc §11）。
 *
 * - 菜单决定客户端能看到、能下载什么。不在菜单上的文件，客户端完全看不到也下不了。
 * - 套餐定义"一份交付里装什么"：0..N 个预设 + 0..N 个 BBS，四种组合都合法。
 *
 * 这一页上改显示名、上下架、调套餐，**都不需要升级客户端**（doc §5）：
 * 客户端按 presetId 认身份，能力定义里不枚举预设文件名。
 */
import { useCallback, useEffect, useState } from 'react'

import {
  describeError,
  wb,
  type BbsFile,
  type Bundles,
  type Catalog,
  type VersionStatus,
} from '../api'

export function CatalogPage({ onChanged }: { onChanged: () => void }) {
  const [catalog, setCatalog] = useState<Catalog | null>(null)
  const [bundles, setBundles] = useState<Bundles | null>(null)
  const [status, setStatus] = useState<VersionStatus[]>([])
  const [files, setFiles] = useState<BbsFile[]>([])
  const [msg, setMsg] = useState<string | null>(null)
  const [err, setErr] = useState<string | null>(null)
  const [newBundle, setNewBundle] = useState('')

  const reload = useCallback(async () => {
    try {
      setCatalog(await wb.catalog())
      setBundles(await wb.bundles())
      setStatus(await wb.status())
      setFiles(await wb.bbsList())
    } catch (e) {
      setErr(describeError(e))
    }
  }, [])

  useEffect(() => {
    void reload()
  }, [reload])

  const save = async (label: string, fn: () => Promise<string>) => {
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

  if (!catalog || !bundles) return <p className="wb-placeholder">读取中…</p>

  const listedOf = (machineId: string, versionId: string) =>
    catalog.presets.find((p) => p.machine === machineId && p.version === versionId)

  return (
    <div className="wb-stack">
      {msg && <div className="wb-banner">{msg}</div>}
      {err && <div className="wb-banner wb-banner--bad">{err}</div>}

      <div className="wb-banner">
        仓库里有 ≠ 客户端能看到。只有进了这份菜单的资源才对客户端提供 ——
        没上架的文件，客户端完全看不到也下不了（但仓库是开源的，别人能来后厨看见它）。
      </div>

      <section className="wb-section">
        <h3 className="wb-section-title">预设上架（菜单）</h3>
        <table className="wb-table">
          <thead>
            <tr>
              <th>版本</th>
              <th>上架</th>
              <th>presetId（身份，尽量不改）</th>
              <th>显示名（随时可改）</th>
              <th>可单独下载</th>
              <th>最低客户端</th>
            </tr>
          </thead>
          <tbody>
            {status.map((s) => {
              const entry = listedOf(s.machineId, s.versionId)
              return (
                <tr key={`${s.machineId}/${s.versionId}`}>
                  <td>
                    {s.machineName} / {s.versionName}
                  </td>
                  <td>
                    <input
                      type="checkbox"
                      checked={!!entry}
                      onChange={(e) =>
                        void save('上下架', async () => {
                          const next = { ...catalog, presets: [...catalog.presets] }
                          if (e.target.checked) {
                            next.presets.push({
                              presetId: s.presetId,
                              machine: s.machineId,
                              version: s.versionId,
                              displayName: `${s.machineName} ${s.versionName}`,
                              resource: `presets/${s.presetId}.toml`,
                              standalone: true,
                            })
                          } else {
                            next.presets = next.presets.filter(
                              (p) => !(p.machine === s.machineId && p.version === s.versionId),
                            )
                          }
                          await wb.saveCatalog(next)
                          return e.target.checked
                            ? '已上架。还没发布 —— 发布是另一个动作。'
                            : '已下架。客户端下次拉目录就看不到它了。'
                        })
                      }
                    />
                  </td>
                  <td className="wb-mono">{entry?.presetId ?? s.presetId}</td>
                  <td>
                    {entry ? (
                      <input
                        className="wb-input"
                        defaultValue={entry.displayName}
                        onBlur={(e) => {
                          if (e.target.value === entry.displayName) return
                          void save('改显示名', async () => {
                            const next = {
                              ...catalog,
                              presets: catalog.presets.map((p) =>
                                p.presetId === entry.presetId
                                  ? { ...p, displayName: e.target.value }
                                  : p,
                              ),
                            }
                            await wb.saveCatalog(next)
                            return '显示名改了。presetId 没动，也不需要升级客户端。'
                          })
                        }}
                      />
                    ) : (
                      <span className="wb-hint">（未上架）</span>
                    )}
                  </td>
                  <td>
                    {entry && (
                      <input
                        type="checkbox"
                        checked={entry.standalone}
                        onChange={(e) =>
                          void save('改单售', async () => {
                            const next = {
                              ...catalog,
                              presets: catalog.presets.map((p) =>
                                p.presetId === entry.presetId
                                  ? { ...p, standalone: e.target.checked }
                                  : p,
                              ),
                            }
                            await wb.saveCatalog(next)
                            return '改好了。'
                          })
                        }
                      />
                    )}
                  </td>
                  <td className="wb-mono">{s.minClientVersion ?? '—'}</td>
                </tr>
              )
            })}
          </tbody>
        </table>
      </section>

      <section className="wb-section">
        <h3 className="wb-section-title">BBS 上架</h3>
        {files.length === 0 ? (
          <p className="wb-placeholder">仓库里还没有 BBS 文件。</p>
        ) : (
          <table className="wb-table">
            <thead>
              <tr>
                <th>id</th>
                <th>上架</th>
                <th>形态</th>
              </tr>
            </thead>
            <tbody>
              {files.map((f) => {
                const e = catalog.bbs.find((b) => b.bbsId === f.id)
                return (
                  <tr key={f.id}>
                    <td className="wb-mono">{f.id}</td>
                    <td>
                      <input
                        type="checkbox"
                        checked={!!e}
                        onChange={(ev) =>
                          void save('BBS 上下架', async () => {
                            const next = { ...catalog, bbs: [...catalog.bbs] }
                            if (ev.target.checked) {
                              next.bbs.push({
                                bbsId: f.id,
                                displayName: f.id,
                                resource: `bbs/${f.id}.json`,
                                offering: 'optional',
                              })
                            } else {
                              next.bbs = next.bbs.filter((b) => b.bbsId !== f.id)
                            }
                            await wb.saveCatalog(next)
                            return ev.target.checked
                              ? '已上架。'
                              : '已下架 —— 它回到「仅归档」，客户端看不到了。'
                          })
                        }
                      />
                    </td>
                    <td>
                      {e ? (
                        <select
                          className="wb-input"
                          value={e.offering}
                          onChange={(ev) =>
                            void save('改形态', async () => {
                              const next = {
                                ...catalog,
                                bbs: catalog.bbs.map((b) =>
                                  b.bbsId === f.id
                                    ? { ...b, offering: ev.target.value as 'assigned' | 'optional' }
                                    : b,
                                ),
                              }
                              await wb.saveCatalog(next)
                              return '改好了。'
                            })
                          }
                        >
                          <option value="assigned">已分配（随套餐交付）</option>
                          <option value="optional">可选（用户可额外下载）</option>
                        </select>
                      ) : (
                        <span className="wb-tag wb-tag--na">仅归档</span>
                      )}
                    </td>
                  </tr>
                )
              })}
            </tbody>
          </table>
        )}
      </section>

      <section className="wb-section">
        <h3 className="wb-section-title">套餐（0..N 预设 + 0..N BBS）</h3>
        <div className="wb-row">
          <input
            className="wb-input"
            placeholder="套餐 id"
            value={newBundle}
            onChange={(e) => setNewBundle(e.target.value)}
          />
          <button
            type="button"
            className="wb-btn"
            disabled={!newBundle.trim()}
            onClick={() =>
              void save('建套餐', async () => {
                await wb.saveBundles({
                  bundles: [
                    ...bundles.bundles,
                    {
                      bundleId: newBundle.trim(),
                      displayName: newBundle.trim(),
                      presets: [],
                      bbs: [],
                    },
                  ],
                })
                setNewBundle('')
                return '建好了。空套餐也是合法的 —— 装几样由你定。'
              })
            }
          >
            新建套餐
          </button>
        </div>

        {bundles.bundles.map((b) => (
          <div key={b.bundleId} className="wb-stack">
            <div className="wb-row">
              <strong>{b.displayName}</strong>
              <span className="wb-hint wb-mono">{b.bundleId}</span>
              <button
                type="button"
                className="wb-link"
                onClick={() =>
                  void save('删套餐', async () => {
                    await wb.saveBundles({
                      bundles: bundles.bundles.filter((x) => x.bundleId !== b.bundleId),
                    })
                    return '删了。'
                  })
                }
              >
                删除
              </button>
            </div>
            <div className="wb-row">
              <span className="wb-hint">预设</span>
              {catalog.presets.length === 0 && <span className="wb-hint">（还没有上架的预设）</span>}
              {catalog.presets.map((p) => (
                <label key={p.presetId} className="wb-radio">
                  <input
                    type="checkbox"
                    checked={b.presets.includes(p.presetId)}
                    onChange={(e) =>
                      void save('改套餐内容', async () => {
                        await wb.saveBundles({
                          bundles: bundles.bundles.map((x) =>
                            x.bundleId === b.bundleId
                              ? {
                                  ...x,
                                  presets: e.target.checked
                                    ? [...x.presets, p.presetId]
                                    : x.presets.filter((i) => i !== p.presetId),
                                }
                              : x,
                          ),
                        })
                        return '改好了。'
                      })
                    }
                  />
                  {p.displayName}
                </label>
              ))}
            </div>
            <div className="wb-row">
              <span className="wb-hint">BBS</span>
              {files.length === 0 && <span className="wb-hint">（仓库里没有 BBS）</span>}
              {files.map((f) => (
                <label key={f.id} className="wb-radio">
                  <input
                    type="checkbox"
                    checked={b.bbs.includes(f.id)}
                    onChange={(e) =>
                      void save('改套餐内容', async () => {
                        await wb.saveBundles({
                          bundles: bundles.bundles.map((x) =>
                            x.bundleId === b.bundleId
                              ? {
                                  ...x,
                                  bbs: e.target.checked
                                    ? [...x.bbs, f.id]
                                    : x.bbs.filter((i) => i !== f.id),
                                }
                              : x,
                          ),
                        })
                        return '改好了。'
                      })
                    }
                  />
                  {f.id}
                </label>
              ))}
            </div>
          </div>
        ))}
      </section>
    </div>
  )
}
