/**
 * 字段定义 —— **只看，不改**。
 *
 * 为什么这一稿不开放编辑（doc §9.8）：这张表不只是界面表头，它决定 TOML 怎么写、
 * 值怎么校验、区间与步进是多少。做成完整编辑器等于在工作台里再造一个配置系统。
 *
 * 所以本页面上**没有任何写入入口** —— 不是禁用一个按钮，是根本没有那个按钮，
 * 也没有对应的命令（Rust 侧只有 `wb_registry` 一个读命令）。
 */
import { useMemo, useState } from 'react'

import type { Registry } from '../api'

export function RegistryPage({ registry }: { registry: Registry | null }) {
  const [q, setQ] = useState('')

  const rows = useMemo(() => {
    if (!registry) return []
    const needle = q.trim().toLowerCase()
    return registry.fields
      .filter(
        (f) =>
          !needle ||
          f.key.toLowerCase().includes(needle) ||
          f.label.toLowerCase().includes(needle) ||
          f.tomlKey.toLowerCase().includes(needle),
      )
      .slice()
      .sort((a, b) => a.order - b.order)
  }, [registry, q])

  if (!registry) return <p className="wb-placeholder">字段定义还没读出来。</p>

  return (
    <div className="wb-stack">
      <div className="wb-row">
        <input
          className="wb-input"
          placeholder="搜字段名、标签或 TOML 键"
          value={q}
          onChange={(e) => setQ(e.target.value)}
        />
        <span className="wb-hint">
          共 {registry.fields.length} 个字段 · schema v{registry.schemaVersion} · 本页只读
        </span>
      </div>

      <table className="wb-table">
        <thead>
          <tr>
            <th>字段</th>
            <th>标签</th>
            <th>类型</th>
            <th>区间</th>
            <th>步进</th>
            <th>单位</th>
            <th>TOML</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((f) => (
            <tr key={f.key}>
              <td className="wb-mono">{f.key}</td>
              <td title={f.desc}>{f.label}</td>
              <td>{f.valueType}</td>
              <td className="wb-mono">
                {f.min === undefined && f.max === undefined ? '—' : `${f.min ?? '−∞'} … ${f.max ?? '+∞'}`}
              </td>
              <td className="wb-mono">{f.step ?? '—'}</td>
              <td>{f.unit ?? '—'}</td>
              <td className="wb-mono">
                [{f.section}] {f.tomlKey}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
