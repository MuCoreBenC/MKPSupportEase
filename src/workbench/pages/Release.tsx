/**
 * 生成 / 出货检查 / 发布 —— **三个动作，界面上分开**（doc §13）。
 *
 * 顺序是刻意的，而且每一步的按钮都单独存在：
 * 1. 生成待更新项（默认按钮；「全部重新生成」是次要入口）
 * 2. 出货检查（阻断项清零才能发布）
 * 3. 发布
 *
 * 界面上反复说的一句话：**保存 ≠ 生成 ≠ 发布 ≠ 客户端更新。**
 * 把它们合成一个"一键同步"会让人分不清现在顾客手里是哪一版。
 */
import { useState } from 'react'

import {
  describeError,
  wb,
  type GenReport,
  type PreflightReport,
  type PublishReport,
} from '../api'

export function ReleasePage({ onChanged }: { onChanged: () => void }) {
  const [gen, setGen] = useState<GenReport | null>(null)
  const [pre, setPre] = useState<PreflightReport | null>(null)
  const [pub, setPub] = useState<PublishReport | null>(null)
  const [err, setErr] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const run = async <T,>(fn: () => Promise<T>, set: (v: T) => void) => {
    setBusy(true)
    setErr(null)
    try {
      set(await fn())
      onChanged()
    } catch (e) {
      setErr(describeError(e))
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="wb-stack">
      {err && <div className="wb-banner wb-banner--bad">{err}</div>}

      <div className="wb-banner">
        四件事是分开的：<strong>配方存了 ≠ TOML 生成了 ≠ 发布了 ≠ 客户端更新了</strong>。
        这一页只管中间两件；客户端什么时候更新，工作台不知道也不假装知道。
      </div>

      <section className="wb-section">
        <h3 className="wb-section-title">第一步：生成</h3>
        <div className="wb-row">
          <button
            type="button"
            className="wb-btn wb-btn--primary"
            disabled={busy}
            onClick={() => void run(() => wb.generateStale(), setGen)}
          >
            生成待更新项
          </button>
          <button
            type="button"
            className="wb-btn"
            disabled={busy}
            onClick={() => void run(() => wb.generateAll(), setGen)}
          >
            全部重新生成
          </button>
          <span className="wb-hint">
            默认只生成配方变了的那些。没改的产物字节不变 —— 这样用户端不会莫名要更新。
          </span>
        </div>

        {gen && (
          <div className="wb-stack">
            <div className={gen.failed.length > 0 ? 'wb-banner wb-banner--warn' : 'wb-banner'}>
              成功 {gen.generated.length} 个（其中 {gen.unchangedCount} 个字节没变），
              失败 {gen.failed.length} 个。
              {gen.failed.length > 0 && ' 失败的那些，旧产物没被动过。'}
            </div>
            {gen.generated.length > 0 && (
              <table className="wb-table">
                <thead>
                  <tr>
                    <th>presetId</th>
                    <th>产物</th>
                    <th>最低客户端</th>
                    <th>字节</th>
                  </tr>
                </thead>
                <tbody>
                  {gen.generated.map((o) => (
                    <tr key={o.presetId}>
                      <td className="wb-mono">{o.presetId}</td>
                      <td className="wb-mono">{o.outputRel}</td>
                      <td className="wb-mono">
                        {o.minClientVersion}
                        {o.unsupported.length > 0 && (
                          <div className="wb-hint">
                            {o.unsupported.length} 个支持期内的客户端用不了它
                          </div>
                        )}
                      </td>
                      <td>{o.unchanged ? '没变' : `${o.size} B`}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
            {gen.failed.map((f) => (
              <div key={`${f.machineId}/${f.versionId}`} className="wb-banner wb-banner--bad">
                {f.machineId}/{f.versionId}：{f.reason}
              </div>
            ))}
          </div>
        )}
      </section>

      <section className="wb-section">
        <h3 className="wb-section-title">第二步：出货检查</h3>
        <div className="wb-row">
          <button
            type="button"
            className="wb-btn"
            disabled={busy}
            onClick={() => void run(() => wb.preflight(), setPre)}
          >
            检查
          </button>
          <span className="wb-hint">
            「无法校验」算阻断，不算通过 —— 否则兼容性就是靠运气。
          </span>
        </div>

        {pre && (
          <div className="wb-stack">
            <div
              className={
                pre.canPublish ? 'wb-banner' : 'wb-banner wb-banner--bad'
              }
            >
              阻断 {pre.blocking} 条，提示 {pre.warnings} 条。
              {pre.canPublish ? ' 可以发布。' : ' 阻断项清零才能发布。'}
            </div>
            {pre.findings.length > 0 && (
              <table className="wb-table">
                <thead>
                  <tr>
                    <th>级别</th>
                    <th>类别</th>
                    <th>说明</th>
                  </tr>
                </thead>
                <tbody>
                  {pre.findings.map((f, i) => (
                    <tr key={i} className={f.severity === 'warning' ? 'wb-row--warn' : undefined}>
                      <td>
                        <span
                          className={
                            f.severity === 'blocking' ? 'wb-tag wb-tag--warn' : 'wb-tag'
                          }
                        >
                          {f.severity === 'blocking' ? '阻断' : '提示'}
                        </span>
                      </td>
                      <td>{categoryLabel(f.category)}</td>
                      <td>
                        {f.message}
                        {f.detail && <div className="wb-hint">{f.detail}</div>}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        )}
      </section>

      <section className="wb-section">
        <h3 className="wb-section-title">第三步：发布</h3>
        <div className="wb-row">
          <button
            type="button"
            className="wb-btn wb-btn--primary"
            disabled={busy || !pre?.canPublish}
            onClick={() => void run(() => wb.publish(), setPub)}
          >
            发布到 dist-presets
          </button>
          <span className="wb-hint">
            先检查再发布。这个按钮禁着只是提醒 —— 真正的判定在 Rust 侧，
            有阻断项时命令本身就会拒绝。
          </span>
        </div>

        {pub && (
          <div className="wb-stack">
            <div className="wb-banner">
              已发布 {pub.presetCount} 个预设、{pub.bbsCount} 个 BBS、{pub.bundleCount} 个套餐
              （{pub.publishedAt}）。
              <strong>开发配方 JSON 不在发布内容里。</strong>
              发布完成不等于客户端已经更新。
            </div>
            <table className="wb-table">
              <thead>
                <tr>
                  <th>#</th>
                  <th>写盘顺序</th>
                </tr>
              </thead>
              <tbody>
                {pub.files.map((f, i) => (
                  <tr key={f}>
                    <td>{i + 1}</td>
                    <td className="wb-mono">
                      {f}
                      {f === 'catalog.json' && (
                        <span className="wb-hint">
                          {' '}
                          ← 最后一个。先写它，客户端就可能读到"目录说有、文件还没到"
                        </span>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>
    </div>
  )
}

function categoryLabel(c: PreflightReport['findings'][number]['category']): string {
  switch (c) {
    case 'recipe':
      return '配方与产物'
    case 'reference':
      return '引用完整性'
    case 'compatibility':
      return '兼容性'
    case 'inventory':
      return '资源盘点'
  }
}
