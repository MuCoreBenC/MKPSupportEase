/**
 * 发布闸 + 发布回执 —— 点【发布】之前先过闸，点完**就地**把结果留在原地。
 *
 * # 它是什么
 *
 * 两段视图，同一屏：
 *
 * ```text
 * ① 发布闸（点【发布】先到这里）
 *    十五项检查逐项打勾，**全绿才允许往下走**（作者定的硬规矩：任何一项 Blocker 红了，
 *    绝对不进 commit / push / PR）。
 *
 * ② 发布回执（人点了「确认发布」之后就地切过来）
 *    阶段链：发布检查 → 生成 → 提交 <sha> → 推送 → PR #27 → CI → 合并
 *    + PR 地址（可点，交给系统浏览器）+【合并】（squash，二次确认）
 * ```
 *
 * ★ 界面在这里**只负责画，不负责判**：闸的整份结果来自 `wb_publish_audit`，
 * 回执的每个阶段来自 `wb_publish` 返回的 `PublishTxReport`（Rust 的
 * `audit::publish_audit` / `publish_tx::run` —— 与 `cargo test` 判据调的是同一批函数）。
 * 前端**不许**自己再实现一套检查：那正是这把刀要治的老病根 —— 两边各算一遍，
 * 各自看着都对，然后发布了不该发布的东西。
 *
 * # 合并的口径（作者 2026-10-04 拍）
 *
 * 一律 **squash**；**不强制等 CI** —— CI 没跑完 / 已经红了都在这里**说清楚再二次确认**，
 * 决定权在人（后端只照做，不重复设闸）。合完回读一份真状态，`Merged` 就是 `Merged`。
 *
 * # 左边那列状态
 *
 * `Skipped` 画成**虚线的灰**，不画成绿勾 —— 它说的是「这一项今天还没跑」。
 * 把"没实现"画成通过，比画成红叉更危险（作者定的分档口径）。
 */

import { useCallback, useEffect, useState } from 'react'

import { isAppError, wb } from '../api'
import type {
  AuditItem,
  AuditStatus,
  PublishAudit,
  PublishStage,
  PublishTxReport,
  RemoteReview,
} from '../api'
import ModalC14 from '../c14/ModalC14'
import c from '../c14.module.css'
import s from './PublishGateModal.module.css'

/** 四态各画成一个词。`pass` 是"跑过了、没问题"，`skipped` 是"今天没跑"—— 两件事 */
const STATUS_TEXT: Record<AuditStatus, string> = {
  pass: '通过',
  fail: '未通过',
  warn: '待留意',
  skipped: '未实现',
}

const STATUS_CLASS: Record<AuditStatus, string> = {
  pass: s.stPass,
  fail: s.stFail,
  warn: s.stWarn,
  skipped: s.stSkipped,
}

/** 一行底色：红的压过黄的，虚线留给"没跑" */
function rowClass(i: AuditItem): string {
  if (i.status === 'fail') return `${s.row} ${s.rowFail}`
  if (i.status === 'warn') return `${s.row} ${s.rowWarn}`
  if (i.status === 'skipped') return `${s.row} ${s.rowSkipped}`
  return s.row
}

/** 排序：红的、黄的在前（要人先看），其余照后端给的编号顺序 */
function rankOf(i: AuditItem): number {
  if (i.status === 'fail') return 0
  if (i.status === 'warn') return 1
  return 2
}

/** 一行里的文件清单：最多摊这么多个，多的折成一句 */
const FILE_CAP = 6

/* ---------- 回执：阶段链 ---------- */

/** 阶段名次 —— 回执按它把每一格画成"过了 / 停在这 / 还没到" */
const STAGE_RANK: Record<PublishStage, number> = {
  blockedAudit: 0,
  generated: 1,
  committed: 2,
  pushed: 3,
  reviewOpened: 4,
  statusRead: 5,
}

type StepState = 'done' | 'stop' | 'now' | 'todo'

interface Step {
  key: string
  label: string
  note: string
  state: StepState
}

const STEP_MARK: Record<StepState, string> = { done: '✓', stop: '!', now: '…', todo: '·' }

/** CI 汇总 → 一个词（不在界面里现场发明第二套档位） */
const CHECKS_TEXT: Record<RemoteReview['checks'], string> = {
  pending: '运行中',
  passed: '通过',
  failed: '失败',
  none: '没有可读的检查',
  unknown: '认不出',
}

/** PR/MR 状态 → 一个词 */
const REVIEW_TEXT: Record<RemoteReview['state'], string> = {
  open: '等待合并',
  merged: '已合并',
  closed: '已关闭',
  unknown: '认不出',
}

/**
 * 把一次事务报告摊成阶段链。**只读报告，不猜**：报告停在哪一步，
 * 后面那几步就是「还没到」；`commit` 为空且产物没变化 ⇒ 如实说"没有可提交的内容"。
 */
function receiptSteps(report: PublishTxReport, review: RemoteReview | null): Step[] {
  const rank = STAGE_RANK[report.stage] ?? 0
  const mk = (key: string, label: string, reached: boolean, note: string): Step => ({
    key,
    label,
    note,
    state: reached ? 'done' : 'todo',
  })

  const steps: Step[] = [
    rank >= 1
      ? {
          key: 'audit',
          label: '发布检查',
          note: `${report.auditPassed} 项全过`,
          state: 'done',
        }
      : {
          key: 'audit',
          label: '发布检查',
          note: `${report.auditFailed} 项没通过 —— 一个字节都没写`,
          state: 'stop',
        },
    mk(
      'generate',
      '生成',
      rank >= 1,
      report.generated > 0 ? `新生成 ${report.generated} 份产物` : '没有待生成的项（产物已是最新）',
    ),
    mk(
      'commit',
      '提交',
      rank >= 2,
      report.committedPaths.length === 0
        ? '产物逐字节没变 —— 没有可提交的内容'
        : (report.commit ?? '已提交'),
    ),
    mk('push', '推送', rank >= 3, report.branch ? `→ ${report.branch}` : '没推（没走到这一步）'),
    mk(
      'review',
      review ? `PR #${review.number}` : 'PR / MR',
      review !== null,
      review ? review.title : '没建（没配发布账户 / 只走到提交）',
    ),
  ]

  if (review !== null) {
    const checks = review.checks
    steps.push({
      key: 'checks',
      label: 'CI',
      note: CHECKS_TEXT[checks],
      state: checks === 'failed' ? 'stop' : checks === 'pending' ? 'now' : 'done',
    })
    steps.push({
      key: 'merge',
      label: '合并',
      note: review.state === 'merged' ? '已合并（squash）' : REVIEW_TEXT[review.state],
      state: review.state === 'merged' ? 'done' : 'todo',
    })
  }
  return steps
}

interface Props {
  onClose: () => void
  /** 闸全绿、人点了「确认发布」之后才走这里。**报错请抛出来**，这一屏会把它显示在原地 */
  onPublish: () => Promise<PublishTxReport>
  /** 带一份"上一次的发布结果"打开 ⇒ 直接进回执视图（② 卡的「查看发布结果」） */
  initialReport?: PublishTxReport | null
  /** 合并成功后把新状态回传给外面（让 ② 卡那份 lastPublish 跟着更新） */
  onMerged?: (review: RemoteReview) => void
}

export default function PublishGateModal({
  onClose,
  onPublish,
  initialReport = null,
  onMerged,
}: Props) {
  /** 闸的结果；null = 还没算回来 */
  const [audit, setAudit] = useState<PublishAudit | null>(null)
  /** 算挂了 / 发布挂了 —— 都在这一行说，不另开一个提示框 */
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  /** 回执：非 null = 这一屏在收尾视图里（不再画十五项清单） */
  const [report, setReport] = useState<PublishTxReport | null>(initialReport)
  /** 回执里的 PR/MR —— 「刷新」与「合并」都在这一份上更新（历史是快照，这里看当下） */
  const [review, setReview] = useState<RemoteReview | null>(initialReport?.review ?? null)
  const [refreshBusy, setRefreshBusy] = useState(false)
  const [mergeBusy, setMergeBusy] = useState(false)
  const [confirming, setConfirming] = useState(false)
  /** 回执上的一句状态话（合并完成 / 回读结果） */
  const [notice, setNotice] = useState<string | null>(null)

  /**
   * 跑一遍闸。**只读** —— 后端那条命令一个字节都不写，所以点几次都不会有副作用
   * （「重新检查」那颗按钮就靠这条）。
   */
  const run = useCallback(async () => {
    setError(null)
    try {
      setAudit(await wb.publishAudit())
    } catch (e) {
      setAudit(null)
      setError(isAppError(e) ? e.message : String(e))
    }
  }, [])

  useEffect(() => {
    // 回执视图（带 initialReport 打开）不重跑闸 —— 它已经是过去那次发布的结果
    if (initialReport === null) void run()
  }, [run, initialReport])

  /** 残留清理中（闸里那颗「清理残留」按钮就挂这一态） */
  const [cleanBusy, setCleanBusy] = useState(false)

  /**
   * 清理交付残留 —— **闸里就地给的那颗按钮**（去修提示让人点它，按钮就该在原地）。
   * 走 `workbench/.trash/delivery/<时间戳>/` 回收（可还原），清完自动重新跑闸。
   */
  const cleanStrays = async () => {
    setCleanBusy(true)
    setError(null)
    try {
      await wb.cleanDistStrays()
      await run()
    } catch (e) {
      setError(isAppError(e) ? e.message : String(e))
    } finally {
      setCleanBusy(false)
    }
  }

  const publish = async () => {
    setBusy(true)
    setError(null)
    try {
      const rep = await onPublish()
      // ★ 不再直接关掉：就地把这一屏切成回执 —— "关掉再打开又是新的"正是这一刀要治的
      setReport(rep)
      setReview(rep.review)
      setNotice(null)
    } catch (e) {
      /* 发布挂了就把闸留在原地 —— 回执是给"走到了"用的，别拿它兜错误 */
      setError(isAppError(e) ? e.message : String(e))
    } finally {
      setBusy(false)
    }
  }

  /** 手动回读一份 PR/MR 的当下状态（作者定死：**不做轮询**，点一次读一次） */
  const refresh = async () => {
    if (review === null) return
    setRefreshBusy(true)
    setError(null)
    try {
      const now = await wb.publishStatus(review.number)
      setReview(now)
      setNotice(`已回读：${REVIEW_TEXT[now.state]} · CI ${CHECKS_TEXT[now.checks]}`)
    } catch (e) {
      setError(isAppError(e) ? e.message : String(e))
    } finally {
      setRefreshBusy(false)
    }
  }

  /** 在系统浏览器里打开 PR 地址（后端只放行 http(s)） */
  const openReview = async () => {
    if (review === null || review.url === '') return
    try {
      await wb.openExternal(review.url)
    } catch (e) {
      setError(isAppError(e) ? e.message : String(e))
    }
  }

  /** 真合并：squash；后端不设闸 —— CI 的提醒由这里的二次确认说清 */
  const doMerge = async () => {
    if (review === null) return
    setMergeBusy(true)
    setError(null)
    try {
      const merged = await wb.mergeReview(review.number)
      setReview(merged)
      setConfirming(false)
      setNotice(
        merged.state === 'merged'
          ? `已以 squash 合并 #${merged.number}`
          : `合并请求已发出，平台回读仍是「${REVIEW_TEXT[merged.state]}」`,
      )
      onMerged?.(merged)
    } catch (e) {
      setError(isAppError(e) ? e.message : String(e))
    } finally {
      setMergeBusy(false)
    }
  }

  /* ---------- 回执视图 ---------- */

  if (report !== null) {
    const steps = receiptSteps(report, review)
    const canMerge = review !== null && review.state === 'open'
    const confirmText =
      review === null
        ? ''
        : review.checks === 'pending'
          ? `CI 尚未完成 —— 确定继续合并 #${review.number} 吗？`
          : review.checks === 'failed'
            ? `CI 已经失败 —— 确定继续合并 #${review.number} 吗？`
            : `确定以 squash 合并 #${review.number} 吗？`

    return (
      <ModalC14
        open
        size="lg"
        title="发布回执"
        subtitle={
          report.stage === 'blockedAudit'
            ? '停在发布检查 —— 一个字节都没写'
            : `本轮走到「${steps.filter((x) => x.state === 'done').length}/${steps.length}」步`
        }
        closeOnScrim={false}
        onClose={onClose}
        footer={
          <>
            <span className={s.footNote}>{report.summary}</span>
            <span className={c.grow} />
            {review !== null && (
              <button
                type="button"
                className={c.btn}
                disabled={refreshBusy || mergeBusy}
                title="手动回读一次这个 PR/MR 的当下状态（不做轮询）"
                onClick={() => void refresh()}
              >
                {refreshBusy ? '正在回读…' : '刷新状态'}
              </button>
            )}
            <button type="button" className={c.btn} onClick={onClose} disabled={mergeBusy}>
              关闭
            </button>
            {review !== null && review.url !== '' && (
              <button
                type="button"
                className={c.btn}
                disabled={mergeBusy}
                title="在系统浏览器里打开这一页"
                onClick={() => void openReview()}
              >
                查看 PR
              </button>
            )}
            {canMerge && (
              <button
                type="button"
                className={`${c.btn} ${c.btnPrimary}`}
                disabled={mergeBusy || refreshBusy}
                title="squash 合并进目标分支（合并不可撤销）"
                onClick={() => setConfirming(true)}
              >
                合并 #{review?.number}
              </button>
            )}
          </>
        }
      >
        {notice !== null && <p className={s.notice}>{notice}</p>}
        {error !== null && (
          <p className={s.error} role="alert">
            {error}
          </p>
        )}

        <div className={s.chain} role="list">
          {steps.map((st) => (
            <div key={st.key} className={s.step} data-state={st.state} role="listitem">
              <span className={s.stepMark}>{STEP_MARK[st.state]}</span>
              <b className={s.stepLabel}>{st.label}</b>
              <span className={`${s.stepNote} ${st.key === 'commit' ? c.mono : ''}`}>
                {st.note}
              </span>
            </div>
          ))}
        </div>

        {review !== null && review.url !== '' && (
          <p className={s.prLine}>
            <span className={c.mono}>{review.url}</span>
          </p>
        )}

        {confirming && review !== null && (
          <div className={s.confirm}>
            <div className={s.confirmTitle}>{confirmText}</div>
            <div className={s.confirmNote}>
              squash 合并进 <span className={c.mono}>{review.base || 'main'}</span>
              ，合并不可撤销。CI 状态不是前置条件 —— 但你要知道它现在是什么。
            </div>
            <div className={s.bar}>
              <span className={c.grow} />
              <button
                type="button"
                className={c.btn}
                disabled={mergeBusy}
                onClick={() => setConfirming(false)}
              >
                取消
              </button>
              <button
                type="button"
                className={`${c.btn} ${c.btnPrimary}`}
                disabled={mergeBusy}
                onClick={() => void doMerge()}
              >
                {mergeBusy ? '正在合并…' : '确定合并'}
              </button>
            </div>
          </div>
        )}
      </ModalC14>
    )
  }

  /* ---------- 闸视图 ---------- */

  const items = audit === null ? [] : [...audit.items].sort((a, b) => rankOf(a) - rankOf(b))
  const blockers = audit?.items.filter((i) => i.severity === 'blocker' && i.status === 'fail').length ?? 0
  const warnings = audit?.items.filter((i) => i.status === 'warn').length ?? 0

  const subtitle =
    audit === null
      ? error === null
        ? '正在跑十五项检查……'
        : '没跑起来'
      : audit.canPublish
        ? `十五项全过${warnings > 0 ? ` · ${warnings} 项待留意` : ''}`
        : `${blockers} 项拦住发布`

  return (
    <ModalC14
      open
      size="lg"
      title="发布闸"
      subtitle={subtitle}
      closeOnScrim={false}
      onClose={onClose}
      footer={
        <>
          <span className={s.footNote}>
            {audit === null
              ? '闸还没跑完 —— 跑完才谈得上发不发'
              : audit.canPublish
                ? '任何一项 Blocker 红了就不许往下走 —— 现在是全绿'
                : '有 Blocker 未通过：修完点「重新检查」'}
          </span>
          <span className={c.grow} />
          <button type="button" className={c.btn} onClick={onClose} disabled={busy}>
            取消
          </button>
          <button type="button" className={c.btn} onClick={() => void run()} disabled={busy}>
            重新检查
          </button>
          <button
            type="button"
            className={`${c.btn} ${c.btnPrimary}`}
            disabled={!audit?.canPublish || busy}
            title={audit?.canPublish ? undefined : '闸没全绿 —— 这颗按钮不亮'}
            onClick={() => void publish()}
          >
            {busy ? '发布中…' : '确认发布'}
          </button>
        </>
      }
    >
      {error !== null && (
        <p className={s.error} role="alert">
          {error}
        </p>
      )}

      {audit !== null && (
        <>
          {/* 这一批会让 presets/ 怎么变 —— 数来自后端那一格（只数 presets/ 下面的） */}
          <p className={s.counts}>
            本次发布会让 <span className={c.mono}>presets/</span> 新增{' '}
            <b>{audit.filesAdded}</b> · 改动 <b>{audit.filesChanged}</b> · 删除{' '}
            <b>{audit.filesRemoved}</b> 个文件
            {audit.minVersion !== null && <> · 最低客户端版本 {audit.minVersion}</>}
          </p>

          <div className={s.list} role="list">
            {items.map((i) => (
              <div key={i.id} className={rowClass(i)} role="listitem">
                <span className={`${s.st} ${STATUS_CLASS[i.status]}`}>
                  {STATUS_TEXT[i.status]}
                  {i.severity === 'blocker' && i.status === 'fail' && (
                    <span className={s.sev} title="Blocker：红了就不许发布">
                      拦发布
                    </span>
                  )}
                </span>
                <div className={s.body}>
                  <span className={s.head}>
                    <b className={s.name}>{i.name}</b>
                    <span className={`${c.mono} ${s.id}`}>{i.id}</span>
                  </span>
                  <span className={s.details}>{i.details}</span>
                  {i.affectedFiles.length > 0 && (
                    <span className={s.files}>
                      {i.affectedFiles.slice(0, FILE_CAP).map((f) => (
                        <code key={f} className={`${c.mono} ${s.file}`}>
                          {f}
                        </code>
                      ))}
                      {i.affectedFiles.length > FILE_CAP && (
                        <span className={s.more}>…等 {i.affectedFiles.length} 处</span>
                      )}
                    </span>
                  )}
                  {i.status !== 'pass' && <span className={s.fix}>去修：{i.fixHint}</span>}
                  {i.id === 'delivery/no-strays' && i.status === 'fail' && (
                    <div className={s.fixAction}>
                      <button
                        type="button"
                        className={c.btn}
                        disabled={cleanBusy}
                        onClick={() => void cleanStrays()}
                      >
                        {cleanBusy ? '清理中…' : '清理残留'}
                      </button>
                      <span className={s.fixNote}>
                        走{' '}
                        <span className={c.mono}>workbench/.trash/delivery/&lt;时间戳&gt;/</span>{' '}
                        回收，可还原；清完自动重新检查
                      </span>
                    </div>
                  )}
                </div>
              </div>
            ))}
          </div>
        </>
      )}

      {audit === null && error === null && <p className={s.pending}>正在跑十五项检查……</p>}
    </ModalC14>
  )
}
