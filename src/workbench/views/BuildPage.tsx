/**
 * 「生成」视角 —— doc §4.2 的「检查与生成」+「发布中心」两行，落在同一个视角。
 *
 * # 四段同时可见，但约束是流水线的（14.2）
 *
 * 检查 → 生成 → 基线 → 发布。后端已经有三道硬闸（`wb_generate` / `wb_publish`
 * 开头的 `inspect` + `first_block`、残留清零才许发布），这一页的按钮禁用
 * **只是把后端会拒绝的事提前挡住**，不是第二套判定 —— 所以禁用理由一律
 * 引后端给的话（`words.disabled.*` / `IssueReport` / `artifact` 状态），不自己编。
 *
 * # 基线同步是显式写入（14.9 的裁决，2026-09-24 锁定交互）
 *
 * 只读 diff → 分类展示（将覆盖 / 将新增；产物名单是编译进二进制的固定九份，
 * **没有"从基线删除"的候选**）→ 人点确认 → 执行同步 → **重读 diff**。
 * 同步后的状态以重读的对照结果为准，不显示预设的成功话术。
 */
import { useCallback, useEffect, useState } from 'react'

import {
  isAppError,
  wb,
  type BaselineDiffEntry,
  type BookView,
  type BuildScope,
  type GenerateReport,
  type IssueReport,
  type Patch,
  type PublishReport,
  type Words,
} from '../api'

interface Props {
  book: BookView
  words: Words
  /** 生成记录（`markBuilt`）必须走唯一写入口 `wb_apply_draft` */
  onApply: (label: string, patches: Patch[]) => Promise<unknown>
  /** 发布 / 清残留这类直接命令之后，让外壳重取 book */
  onView: () => Promise<void>
}

export function BuildPage({ book, words, onApply, onView }: Props) {
  const [report, setReport] = useState<IssueReport | null>(null)
  const [gen, setGen] = useState<GenerateReport | null>(null)
  const [pub, setPub] = useState<PublishReport | null>(null)
  const [diff, setDiff] = useState<BaselineDiffEntry[] | null>(null)
  const [strays, setStrays] = useState<string[] | null>(null)
  const [confirmSync, setConfirmSync] = useState(false)
  const [syncNote, setSyncNote] = useState<string | null>(null)
  const [previewUid, setPreviewUid] = useState<string>('')
  const [previewToml, setPreviewToml] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [err, setErr] = useState<string | null>(null)

  const fail = (e: unknown) => setErr(isAppError(e) ? `${e.message}${e.detail ? ` —— ${e.detail}` : ''}` : String(e))

  /** 进入这一页先跑一次检查与基线 diff —— 两个都是**只读**命令 */
  const load = useCallback(async () => {
    setBusy(true)
    try {
      const [r, d] = await Promise.all([wb.preflight(), wb.baselineDiff()])
      setReport(r)
      setDiff(d)
    } catch (e) {
      fail(e)
    } finally {
      setBusy(false)
    }
  }, [])

  useEffect(() => {
    void load()
  }, [load])

  const buildable = book.buildRows.filter((r) => r.buildable)
  const toChange = diff?.filter((d) => d.status === 'changed') ?? []
  const toAdd = diff?.filter((d) => d.status === 'missingBaseline') ?? []
  const dirtyBaseline = toChange.length + toAdd.length > 0

  const generate = async (scope: BuildScope, label: string) => {
    setBusy(true)
    setErr(null)
    try {
      const out = await wb.generate(scope)
      setGen(out)
      // 生成记录落进草稿（markBuilt）。不可撤销，`inverse` 为空，不进撤销栈
      await onApply(`生成：${label}`, [out.mark])
    } catch (e) {
      fail(e)
    } finally {
      setBusy(false)
    }
  }

  /** 同步。**能走到这里 = 人已经在确认区点过"确认写入"** */
  const sync = async () => {
    setBusy(true)
    setErr(null)
    try {
      const n = await wb.syncBaseline()
      setConfirmSync(false)
      // 关键一步：同步完成后**重读 diff**。界面上的状态是这次重读的结果，
      // 不是"成功"两个字 —— 哪份还红着，一眼就能看见
      await loadDiff()
      setSyncNote(`已写入 ${n} 份。下面是同步后重读的对照结果。`)
    } catch (e) {
      fail(e)
    } finally {
      setBusy(false)
    }
  }

  const loadDiff = useCallback(async () => {
    try {
      setDiff(await wb.baselineDiff())
    } catch (e) {
      fail(e)
    }
  }, [])

  const publish = async () => {
    setBusy(true)
    setErr(null)
    try {
      const out = await wb.publish()
      setPub(out)
      await onView()
    } catch (e) {
      fail(e)
    } finally {
      setBusy(false)
    }
  }

  const checkStrays = async () => {
    setBusy(true)
    setErr(null)
    try {
      setStrays(await wb.distStrays())
    } catch (e) {
      fail(e)
    } finally {
      setBusy(false)
    }
  }

  const cleanStrays = async () => {
    setBusy(true)
    setErr(null)
    try {
      const n = await wb.cleanDistStrays()
      setStrays(await wb.distStrays())
      setSyncNote(`已把 ${n} 份残留移入回收站。`)
      await onView()
    } catch (e) {
      fail(e)
    } finally {
      setBusy(false)
    }
  }

  const preview = async () => {
    if (!previewUid) return
    setBusy(true)
    setErr(null)
    try {
      setPreviewToml(await wb.previewToml(previewUid))
    } catch (e) {
      fail(e)
    } finally {
      setBusy(false)
    }
  }

  /* 生成按钮的禁用理由：检查没跑 / 有 block / 忙。理由全部来自后端 */
  const genReason =
    report === null
      ? '先跑一次检查'
      : report.blocks > 0
        ? words.disabled.buildBlocked
        : busy
          ? '正在处理'
          : null
  /* 发布按钮的禁用理由：产物不新鲜 / 有 block / 有残留。artifact 是后端算的 */
  const strayCount = strays?.length ?? 0
  const pubReason =
    report !== null && report.blocks > 0
      ? words.disabled.buildBlocked
      : book.artifact !== 'fresh'
        ? (words.artifact[book.artifact].explain ?? '产物不是最新的，先生成')
        : strayCount > 0
          ? `交付目录里有 ${strayCount} 份残留，先清理`
          : busy
            ? '正在处理'
            : null

  return (
    <div className="wb-build">
      {/* ---------- ① 检查 ---------- */}
      <section className="wb-card">
        <header className="wb-card__head">
          <span className="wb-card__title">检查</span>
          <button type="button" className="wb-btn" disabled={busy} onClick={() => void load()}>
            {report ? '重新检查' : '跑检查'}
          </button>
        </header>
        {report ? (
          <>
            <p className="wb-build__counts">
              <b data-tone={report.blocks > 0 ? 'danger' : 'ok'}>阻断 {report.blocks}</b>
              <span>待办 {report.todos}</span>
              <span>提示 {report.hints}</span>
            </p>
            {report.issues.length === 0 ? (
              <p className="wb-todo" data-tone="ok">{report.emptyHint}</p>
            ) : (
              <ul className="wb-issues">
                {report.issues.map((i) => (
                  <li key={i.id} className="wb-issue" data-sev={i.severity}>
                    <span className="wb-issue__sev">
                      {i.severity === 'block' ? '阻断' : i.severity === 'todo' ? '待办' : '提示'}
                    </span>
                    <span className="wb-issue__body">
                      <b>{i.title}</b>
                      <span>{i.detail}</span>
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </>
        ) : (
          <p className="wb-todo">{busy ? '正在检查…' : '还没跑检查。'}</p>
        )}
      </section>

      {/* ---------- ② 生成 ---------- */}
      <section className="wb-card">
        <header className="wb-card__head">
          <span className="wb-card__title">生成</span>
          <span className="wb-mc__headright">
            <button
              type="button"
              className="wb-btn"
              data-tone="primary"
              disabled={genReason !== null}
              title={genReason ?? undefined}
              onClick={() => void generate('stale', '有变化的版本')}
            >
              生成有变化的
            </button>
            <button
              type="button"
              className="wb-btn"
              disabled={genReason !== null}
              title={genReason ?? undefined}
              onClick={() => void generate('all', '全部')}
            >
              全部重新生成
            </button>
          </span>
        </header>
        {genReason && <p className="wb-todo" data-tone="warn">{genReason}</p>}
        <ul className="wb-build__rows">
          {book.buildRows.map((r) => (
            <li key={r.uid} className="wb-build__row">
              <span className="wb-build__who">{r.machine} / {r.name}</span>
              <span
                className="wb-tag"
                data-state={r.state === 'built' ? 'ok' : r.state === 'noResources' ? 'off' : 'warn'}
                title={words.build[r.state].explain ?? undefined}
              >
                {words.build[r.state].label}
              </span>
              {!r.buildable && r.disabledReason && (
                <span className="wb-build__why" title={r.disabledReason}>{r.disabledReason}</span>
              )}
            </li>
          ))}
        </ul>
        {gen && (
          <div className="wb-build__result">
            <p>
              写入 {gen.written.length} 份 · 内容没变跳过重写 {gen.unchanged.length} 份
              {gen.skipped.length > 0 && <> · 跳过 {gen.skipped.length} 份</>}
            </p>
            {gen.written.length > 0 && (
              <p className="wb-mono wb-build__files">{gen.written.join('　')}</p>
            )}
            {gen.skipped.length > 0 && (
              <ul className="wb-build__skips">
                {gen.skipped.map(([name, why]) => (
                  <li key={name}><span className="wb-mono">{name}</span> —— {why}</li>
                ))}
              </ul>
            )}
          </div>
        )}
        {buildable.length > 0 && (
          <div className="wb-build__previewbar">
            <select
              className="wb-ctl"
              data-form="row"
              value={previewUid}
              onChange={(e) => setPreviewUid(e.target.value)}
            >
              <option value="">渲染预览（选一个版本）…</option>
              {buildable.map((r) => (
                <option key={r.uid} value={r.uid}>
                  {r.machine} / {r.name}
                </option>
              ))}
            </select>
            <button type="button" className="wb-btn" disabled={!previewUid || busy} onClick={() => void preview()}>
              渲染
            </button>
          </div>
        )}
        {previewToml !== null && (
          <pre className="wb-build__toml wb-mono">{previewToml}</pre>
        )}
      </section>

      {/* ---------- ③ 对照基线（显式同步） ---------- */}
      <section className="wb-card">
        <header className="wb-card__head">
          <span className="wb-card__title">对照基线</span>
          <button
            type="button"
            className="wb-btn"
            disabled={busy || diff === null || !dirtyBaseline || confirmSync}
            title={
              diff === null
                ? '还没拉到对照清单'
                : dirtyBaseline
                  ? undefined
                  : '九份产物与基线全部一致，没有要同步的'
            }
            onClick={() => setConfirmSync(true)}
          >
            同步基线…
          </button>
        </header>
        <p className="wb-build__baseline-note">
          基线是判据资产。同步是显式写入：看清单 → 确认 → 写 → 重读。
          产物名单固定九份（编译进二进制，有判据盯着），所以没有「从基线删除」的候选。
        </p>
        {syncNote && <p className="wb-todo" data-tone="ok">{syncNote}</p>}
        {confirmSync && diff !== null && (
          <div className="wb-build__confirm">
            <p>
              即将把 <b>{toChange.length + toAdd.length}</b> 份产物写入基线目录
              （覆盖 {toChange.length} 份、新增 {toAdd.length} 份）。
              写之前确认上面的 diff 就是你预期的。
            </p>
            <div className="wb-ver__buttons">
              <button type="button" className="wb-btn" onClick={() => setConfirmSync(false)}>
                再看看
              </button>
              <button
                type="button"
                className="wb-btn"
                data-tone="primary"
                disabled={busy}
                onClick={() => void sync()}
              >
                确认写入 {toChange.length + toAdd.length} 份
              </button>
            </div>
          </div>
        )}
        {diff === null ? (
          <p className="wb-todo">{busy ? '正在读取…' : '还没拉到对照清单。'}</p>
        ) : (
          <table className="wb-table">
            <thead>
              <tr>
                <th>产物</th>
                <th>状态</th>
                <th>产物 sha</th>
                <th>基线 sha</th>
              </tr>
            </thead>
            <tbody>
              {diff.map((d) => (
                <tr key={d.fileName} data-dirty={d.status !== 'same' ? 'yes' : undefined}>
                  <td className="wb-mono">{d.fileName}</td>
                  <td>
                    {d.status === 'same'
                      ? '一致'
                      : d.status === 'changed'
                        ? '将覆盖（与基线不同）'
                        : '将新增（基线缺这份）'}
                  </td>
                  <td className="wb-mono">{d.productSha}</td>
                  <td className="wb-mono">{d.baselineSha ?? '—'}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      {/* ---------- ④ 发布 ---------- */}
      <section className="wb-card">
        <header className="wb-card__head">
          <span className="wb-card__title">发布</span>
          <span className="wb-mc__headright">
            <button type="button" className="wb-btn" disabled={busy} onClick={() => void checkStrays()}>
              {strays === null ? '检查残留' : '重查残留'}
            </button>
            {strays !== null && strayCount > 0 && (
              <button type="button" className="wb-btn" disabled={busy} onClick={() => void cleanStrays()}>
                清理残留（移入回收站）
              </button>
            )}
            <button
              type="button"
              className="wb-btn"
              data-tone="primary"
              disabled={pubReason !== null}
              title={pubReason ?? undefined}
              onClick={() => void publish()}
            >
              发布
            </button>
          </span>
        </header>
        {pubReason && <p className="wb-todo" data-tone="warn">{pubReason}</p>}
        {strays !== null && (
          <div className="wb-build__strays">
            {strays.length === 0 ? (
              <p className="wb-todo" data-tone="ok">交付目录没有残留文件。</p>
            ) : (
              <>
                <p>
                  交付目录里有 <b>{strays.length}</b> 份不在本次交付集合内的文件
                  （消费端会真的下载到它们，发布会被拦下）：
                </p>
                <ul className="wb-build__skips">
                  {strays.map((s) => (
                    <li key={s} className="wb-mono">{s}</li>
                  ))}
                </ul>
              </>
            )}
          </div>
        )}
        {pub && (
          <div className="wb-build__result">
            <p>
              已交付 <b>{pub.files}</b> 份文件到 <span className="wb-mono">{pub.root}</span>
              {pub.minimumClient && <> · 最低客户端 {pub.minimumClient}</>} ·
              待办 {pub.todos} · 提示 {pub.hints}
            </p>
          </div>
        )}
      </section>

      {err && <p className="wb-todo" data-tone="danger">{err}</p>}
    </div>
  )
}
