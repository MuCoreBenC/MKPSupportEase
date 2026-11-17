/**
 * 发布历史 —— **三本账并列**（2026-11-05 作者：「什么时候发布了什么，我都不记得」）。
 *
 * 1. **交付预设**：`<appDataDir>/publish-history.json`，一次事务一条回执；
 * 2. **软件发版**：`release-history.json`，打了 tag / 出了安装包 / 写了 release.json 那条链；
 * 3. **本分支最近提交**：`git log`（只读、本机仓库视图、最多 20 条）——
 *    用来回答「AI 在 CI 里替我做了什么」。
 *
 * 三本账**各自空态、各自报错**：一本读不出来不许假装另外两本也没有。
 *
 * # 状态是快照，刷新是手动（作者 2026-10-04 定死）
 *
 * 记录里的 PR/MR 状态是**写入那一刻**的（合并之后由后端把新状态写回那一条）。
 * 要看现在走到哪，点那一条的「刷新」—— 走 `wb_publish_status` 回读一次。
 * **不做自动轮询**：状态是"看一看"，不是常驻任务。
 *
 * # 它不是什么
 *
 * 不是发布配置（那是设置里的「发布账户」）、不是审计账本（最多留 100 条）。
 * 删掉这些文件不影响发布能力，只丢展示 —— 坏了会**如实报错**，不静默当"没发过"。
 * 第三栏尤其是**本机**的提交列表：没推上去 / 没合进主线的也在里面。
 */

import { useEffect, useState } from 'react'

import { isAppError, wb } from '../api'
import type { CommitRow, PublishHistory, PublishRecord, ReleaseHistory, RemoteReview } from '../api'
import ModalC14 from '../c14/ModalC14'
import c from '../c14.module.css'
import h from './HistoryModal.module.css'

/** 阶段名（后端 `PublishStage::wire_name`）→ 一句人话。认不出就原样显示（不猜） */
const STAGE_TEXT: Record<string, string> = {
  blockedAudit: '停在发布检查',
  generated: '生成完成（未提交）',
  committed: '已提交（未推送）',
  pushed: '已推送（未建 PR）',
  reviewOpened: '已建 PR',
  statusRead: '发布完成',
}

const CHECKS_TEXT: Record<RemoteReview['checks'], string> = {
  pending: '运行中',
  passed: '通过',
  failed: '失败',
  none: '无检查',
  unknown: '认不出',
}

/** 时间：今天 / 昨天 / 日期 + HH:mm。后端记的是 ISO 串，给人看要转本机时区 */
function whenText(iso: string): string {
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return iso
  const p = (n: number) => String(n).padStart(2, '0')
  const hm = `${p(d.getHours())}:${p(d.getMinutes())}`
  const sameDay = (a: Date, b: Date) =>
    a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate()
  const today = new Date()
  if (sameDay(d, today)) return `今天 ${hm}`
  const y = new Date(today)
  y.setDate(y.getDate() - 1)
  if (sameDay(d, y)) return `昨天 ${hm}`
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${hm}`
}

/** 这一条在列表里的 key（时间 + PR 号；同一秒两条的概率可以忽略） */
function keyOf(r: PublishRecord): string {
  return `${r.at}#${r.review?.number ?? '-'}`
}

interface Props {
  onClose: () => void
}

export default function HistoryModal({ onClose }: Props) {
  const [history, setHistory] = useState<PublishHistory | null>(null)
  const [releases, setReleases] = useState<ReleaseHistory | null>(null)
  const [commits, setCommits] = useState<CommitRow[] | null>(null)
  /** 三本账各自记自己的错：一本读坏不许连累另外两本的展示 */
  const [errors, setErrors] = useState<{ publish?: string; release?: string; commits?: string }>({})
  /** 正在刷新哪一条（同一时刻只会有一次） */
  const [busy, setBusy] = useState<string | null>(null)
  /** 刷新回来的当下状态（只活在这次打开里；记录本身是快照） */
  const [live, setLive] = useState<Record<string, RemoteReview>>({})

  /** 打开时各读一次（**只读一次** —— 不做轮询） */
  useEffect(() => {
    let alive = true
    const fail = (e: unknown) => (isAppError(e) ? e.message : String(e))
    void (async () => {
      try {
        const got = await wb.publishHistory()
        if (alive) setHistory(got)
      } catch (e) {
        if (alive) setErrors((prev) => ({ ...prev, publish: fail(e) }))
      }
    })()
    void (async () => {
      try {
        const got = await wb.releaseHistory()
        if (alive) setReleases(got)
      } catch (e) {
        if (alive) setErrors((prev) => ({ ...prev, release: fail(e) }))
      }
    })()
    void (async () => {
      try {
        const got = await wb.recentCommits()
        if (alive) setCommits(got)
      } catch (e) {
        if (alive) setErrors((prev) => ({ ...prev, commits: fail(e) }))
      }
    })()
    return () => {
      alive = false
    }
  }, [])

  /** 手动刷新一条：回读那份 PR/MR 的当下状态 */
  const refresh = async (r: PublishRecord, key: string) => {
    if (r.review === null) return
    setBusy(key)
    setErrors((prev) => ({ ...prev, publish: undefined }))
    try {
      const now = await wb.publishStatus(r.review.number)
      setLive((prev) => ({ ...prev, [key]: now }))
    } catch (e) {
      setErrors((prev) => ({
        ...prev,
        publish: isAppError(e) ? e.message : String(e),
      }))
    } finally {
      setBusy(null)
    }
  }

  /** Rust 端 `Option::None` 序列化时**整个字段省掉**（`skip_serializing_if`）—— 到前端是
   *  `undefined`，类型上写的却是 `null`。在这一处归一，下面的判空才成立：那条
   *  "未配置发布账户"的回执就没有 review，`rev !== null` 拦不住 `undefined`，
   *  一打开这里整个界面崩掉（2026-10-05 真实发生过）。 */
  const records: PublishRecord[] = (history?.records ?? []).map((r) =>
    r.review === undefined ? { ...r, review: null } : r,
  )
  const releaseRows = releases?.records ?? []
  const commitRows = commits ?? []

  /**
   * 同一条发布线上**更早的那一号**（`null` = 这是这条线的第一条回执）。
   *
   * 回执是最新在前 ⇒ 往列表后面找第一条同线的号就是上一代。只用来把
   * 「16 → 17」摆出来，不参与任何判定。
   */
  const prevSerialOf = (index: number): number | null => {
    const line = records[index]?.catalogId ?? null
    for (let i = index + 1; i < records.length; i++) {
      const r = records[i]
      if ((r.catalogId ?? null) !== line) continue
      if (r.publishSerial != null && r.publishSerial > 0) return r.publishSerial
    }
    return null
  }

  const sectionError = (which: 'publish' | 'release' | 'commits') =>
    errors[which] !== undefined ? (
      <p className={h.error} role="alert">
        {errors[which]}
      </p>
    ) : null

  return (
    <ModalC14
      open
      size="lg"
      title="发布历史"
      subtitle={
        history === null && errors.publish === undefined
          ? '正在读……'
          : `${records.length} 条预设回执 · ${releaseRows.length} 次软件发版 · ${commitRows.length} 条本分支提交（打开时各读一次，不轮询）`
      }
      onClose={onClose}
      footer={
        <>
          <span className={h.footNote}>
            前两栏住应用数据目录的 <span className={c.mono}>publish-history.json</span> /{' '}
            <span className={c.mono}>release-history.json</span>；第三栏读本机仓库
          </span>
          <span className={c.grow} />
          <button type="button" className={c.btn} onClick={onClose}>
            关闭
          </button>
        </>
      }
    >
      <section className={h.group}>
        <h3 className={h.groupTitle}>① 交付预设发布</h3>
        {sectionError('publish')}
        {history !== null && records.length === 0 && errors.publish === undefined && (
          <p className={h.empty}>还没有预设发布回执 —— 走一次「交付预设」就会出现一条。</p>
        )}
        {records.length > 0 && (
        <div className={`${h.list} ${h.listShort}`} role="list">
          {records.map((r, index) => {
            const key = keyOf(r)
            const rev = live[key] ?? r.review
            const prevSerial = prevSerialOf(index)
            return (
              <div key={key} className={h.row} role="listitem">
                <div className={h.when}>{whenText(r.at)}</div>
                <div className={h.body}>
                  <div className={h.head}>
                    <b>{STAGE_TEXT[r.stage] ?? r.stage}</b>
                    {/* 这条回执住在哪条入口的账上（工作台 / 命令行）—— 合并读之后必须能分清 */}
                    {r.source !== undefined && (
                      <span className={h.pr}>{r.source === 'cli' ? '命令行' : '工作台'}</span>
                    )}
                    {rev !== null && (
                      <>
                        <span className={h.pr}>PR #{rev.number}</span>
                        <span className={rev.state === 'merged' ? h.merged : h.open}>
                          {rev.state === 'merged' ? '✓ 已合并' : rev.state === 'open' ? '等待合并' : rev.state === 'closed' ? '已关闭' : '认不出'}
                        </span>
                        <span className={h.checks}>CI {CHECKS_TEXT[rev.checks]}</span>
                      </>
                    )}
                  </div>
                  <div className={h.meta}>
                    {/* 那一次发的是第几号（2026-10-29）：老记录没有这两格 ⇒ 照实不显示。
                        「16 → 17」一起摆：只看一个数答不了「这次往前走了没有」 */}
                    {r.publishSerial != null && r.publishSerial > 0 && (
                      <span>
                        {r.catalogId ?? '未知'} 第{' '}
                        {prevSerial === null
                          ? `${r.publishSerial} 号`
                          : `${prevSerial} 号 → 第 ${r.publishSerial} 号`}
                      </span>
                    )}
                    {r.commit !== null && <span className={c.mono}>{r.commit}</span>}
                    <span>{r.files} 份产物</span>
                    {r.generated > 0 && <span>新生成 {r.generated} 份</span>}
                    {rev !== null && rev.url !== '' && <span className={c.mono}>{rev.url}</span>}
                  </div>
                  <div className={h.summary}>{r.summary}</div>
                </div>
                <div className={h.acts}>
                  {rev !== null && (
                    <button
                      type="button"
                      className={`${c.btn} ${c.btnSm}`}
                      disabled={busy === key}
                      title="手动回读这一份 PR/MR 的当下状态（不做轮询）"
                      onClick={() => void refresh(r, key)}
                    >
                      {busy === key ? '正在回读…' : '刷新'}
                    </button>
                  )}
                  {rev !== null && rev.url !== '' && (
                    <button
                      type="button"
                      className={c.btn}
                      title="在系统浏览器里打开"
                      onClick={() => void wb.openExternal(rev.url)}
                    >
                      查看 PR
                    </button>
                  )}
                </div>
              </div>
            )
          })}
        </div>
        )}
      </section>

      <section className={h.group}>
        <h3 className={h.groupTitle}>② 软件发版</h3>
        {sectionError('release')}
        {releases !== null && releaseRows.length === 0 && errors.release === undefined && (
          <p className={h.empty}>还没有发过软件版本 —— 走过「发布软件版本」才会留下 tag 与安装包。</p>
        )}
        {releaseRows.length > 0 && (
          <div className={`${h.list} ${h.listShort}`} role="list">
            {releaseRows.map((r) => (
              <div key={`${r.at}#${r.tag}`} className={h.row} role="listitem">
                <div className={h.when}>{whenText(r.at)}</div>
                <div className={h.body}>
                  <div className={h.head}>
                    <b>{r.version}</b>
                    <span className={h.pr}>{r.tag}</span>
                    {r.source !== undefined && (
                      <span className={h.checks}>{r.source === 'cli' ? '命令行' : '工作台'}</span>
                    )}
                    {r.artifact !== null && r.artifact !== undefined && (
                      <span className={h.checks}>{r.artifact.name}</span>
                    )}
                  </div>
                  <div className={h.meta}>
                    {r.commit !== null && <span className={c.mono}>{r.commit}</span>}
                    {r.infoReview !== null && r.infoReview !== undefined && (
                      <span>信息 PR #{r.infoReview.number}</span>
                    )}
                    <span>{r.stage}</span>
                  </div>
                  <div className={h.summary}>{r.summary}</div>
                </div>
              </div>
            ))}
          </div>
        )}
      </section>

      <section className={h.group}>
        <h3 className={h.groupTitle}>③ 本分支最近提交</h3>
        <p className={h.groupNote}>
          本机仓库视图，<b>不代表线上已合并</b>；没推上去的提交也会出现在这里。
        </p>
        {sectionError('commits')}
        {commits !== null && commitRows.length === 0 && errors.commits === undefined && (
          <p className={h.empty}>这个仓库还没有提交。</p>
        )}
        {commitRows.length > 0 && (
          <div className={`${h.list} ${h.listShort}`} role="list">
            {commitRows.map((r) => (
              <div key={`${r.at}#${r.short}`} className={h.row} role="listitem">
                <div className={h.when}>{whenText(r.at)}</div>
                <div className={h.body}>
                  <div className={h.head}>
                    <b className={c.mono}>{r.short}</b>
                    <span>{r.subject}</span>
                  </div>
                  <div className={h.meta}>
                    <span>{r.author}</span>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </section>
    </ModalC14>
  )
}
