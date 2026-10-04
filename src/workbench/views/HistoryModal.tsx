/**
 * 发布历史 —— 本地回执日志（`<appDataDir>/publish-history.json`）。
 *
 * 一次「发布预设」事务留一条：时间 / 走到哪一阶段 / commit / PR 号与状态 / 产物份数 /
 * 一句话结论。**最新在前**（后端保证顺序，这里不重排）。
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
 * 删掉这个文件不影响发布能力，只丢展示 —— 坏了会**如实报错**，不静默当"没发过"。
 */

import { useEffect, useState } from 'react'

import { isAppError, wb } from '../api'
import type { PublishHistory, PublishRecord, RemoteReview } from '../api'
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
  const [error, setError] = useState<string | null>(null)
  /** 正在刷新哪一条（同一时刻只会有一次） */
  const [busy, setBusy] = useState<string | null>(null)
  /** 刷新回来的当下状态（只活在这次打开里；记录本身是快照） */
  const [live, setLive] = useState<Record<string, RemoteReview>>({})

  /** 打开时读一次（**只读一次** —— 不做轮询） */
  useEffect(() => {
    let alive = true
    void (async () => {
      try {
        const got = await wb.publishHistory()
        if (alive) setHistory(got)
      } catch (e) {
        if (alive) setError(isAppError(e) ? e.message : String(e))
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
    setError(null)
    try {
      const now = await wb.publishStatus(r.review.number)
      setLive((prev) => ({ ...prev, [key]: now }))
    } catch (e) {
      setError(isAppError(e) ? e.message : String(e))
    } finally {
      setBusy(null)
    }
  }

  const records = history?.records ?? []

  return (
    <ModalC14
      open
      size="lg"
      title="发布历史"
      subtitle={
        history === null
          ? error === null
            ? '正在读……'
            : '没读出来'
          : `${records.length} 条回执 · 打开时读一次，每条可手动刷新（不轮询）`
      }
      onClose={onClose}
      footer={
        <>
          <span className={h.footNote}>
            回执日志住 <span className={c.mono}>publish-history.json</span>（应用数据目录）；
            删掉只丢展示，不影响发布
          </span>
          <span className={c.grow} />
          <button type="button" className={c.btn} onClick={onClose}>
            关闭
          </button>
        </>
      }
    >
      {error !== null && (
        <p className={h.error} role="alert">
          {error}
        </p>
      )}

      {history !== null && records.length === 0 && (
        <p className={h.empty}>还没有发布记录 —— 发布一次就会出现一条。</p>
      )}

      {records.length > 0 && (
        <div className={h.list} role="list">
          {records.map((r) => {
            const key = keyOf(r)
            const rev = live[key] ?? r.review
            return (
              <div key={key} className={h.row} role="listitem">
                <div className={h.when}>{whenText(r.at)}</div>
                <div className={h.body}>
                  <div className={h.head}>
                    <b>{STAGE_TEXT[r.stage] ?? r.stage}</b>
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
    </ModalC14>
  )
}
