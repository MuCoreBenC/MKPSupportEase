/**
 * **发布软件版本**的闸 → 回执 → 历史（第四刀）。
 *
 * 与 [`PublishGateModal`]（发布预设那道闸）是**两道闸**：这里看的是"能不能发一个
 * 新的安装包"（分支 / 工作区 / 版本号一致 / tag 有没有被占 / 平台支不支持软件 Release），
 * 那边看的是"预设交出去会不会缺东西"。判定**整份来自 Rust**（`wb_release_preflight`
 * 与内核同一个函数）—— 这一屏没有第二套检查。
 *
 * # 三段视图
 *
 * ```text
 * 闸   逐项结果（红的染整行）→ 全绿才亮「确认发布」
 * 回执 阶段链：版本号 → 提交 → PR → 合并 → tag → 构建 → Release → release.json 的 PR
 * 历史 release-history.json（**与发布预设那本账分开**），最新在前
 * ```
 *
 * # 两条必须说清的话（不藏在说明文字里）
 *
 * 1. **这一趟会切本机的当前分支到 main 并在其 tip 上打 tag** —— 用独立的警告条说；
 * 2. **`release.json` 那个 PR 要人合并** —— 合并之后客户端才看得到新版本
 *    （GitHub raw 只吐 main 上的东西）。回执里给它单独一格。
 */
import { useCallback, useEffect, useState } from 'react'

import { isAppError, wb } from '../api'
import type {
  PreflightItem,
  ReleaseHistory,
  ReleasePreflight,
  ReleaseRecord,
  ReleaseTxReport,
} from '../api'
import ModalC14 from '../c14/ModalC14'
import c from '../c14.module.css'
import s from './ReleaseGateModal.module.css'

/** 阶段的线上名 → 人话（与后端 `ReleaseStage::wire_name()` 逐字对应） */
const STAGE_TEXT: Record<string, string> = {
  blockedPreflight: '停在预检',
  ready: '预检通过',
  versionBumped: '版本号已推进',
  committed: '已提交',
  pushed: '已推送',
  reviewOpened: 'PR 已建',
  merged: '已合并',
  tagged: 'tag 已打',
  built: '安装包已构建',
  releaseCreated: 'Release 已建',
  assetUploaded: '安装包已上传',
  infoCommitted: 'release.json 已提交',
  infoPushed: 'release.json 已推送',
  infoReviewOpened: 'release.json 的 PR 已建',
}

/** 回执的阶段链：按"走到了哪"的顺序排，走到的是 done、没走到的是 todo */
const STAGE_ORDER: string[] = [
  'versionBumped',
  'committed',
  'pushed',
  'reviewOpened',
  'merged',
  'tagged',
  'built',
  'releaseCreated',
  'assetUploaded',
  'infoCommitted',
  'infoPushed',
  'infoReviewOpened',
]

const STAGE_LABEL: Record<string, string> = {
  versionBumped: '版本号',
  committed: '提交',
  pushed: '推送',
  reviewOpened: 'PR',
  merged: '合并',
  tagged: 'tag',
  built: '构建',
  releaseCreated: 'Release',
  assetUploaded: '上传',
  infoCommitted: '信息提交',
  infoPushed: '信息推送',
  infoReviewOpened: '信息 PR',
}

function receiptSteps(report: ReleaseTxReport): { key: string; label: string; note: string; state: string }[] {
  const at = STAGE_ORDER.indexOf(report.stage)
  return STAGE_ORDER.map((key, i) => ({
    key,
    label: STAGE_LABEL[key] ?? key,
    state: i < at ? 'done' : i === at ? 'done' : 'todo',
    note:
      key === 'reviewOpened' && report.review !== null
        ? `#${report.review.number} ${report.review.title}`
        : key === 'infoReviewOpened' && report.infoReview !== null
          ? `#${report.infoReview.number}`
          : key === 'tagged'
            ? report.tag
            : key === 'built' && report.artifact !== null
              ? `${report.artifact.name}（${(report.artifact.size / 1024 / 1024).toFixed(1)} MB）`
              : i === at
                ? '走到这里'
                : '',
  }))
}

/** 本地时区的时间戳（后端写的是 UTC ISO 串，给人看要转本机时区） */
function localStamp(iso: string): string {
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return iso
  const p = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`
}

interface Props {
  onClose: () => void
  /** 当前安装的版本（② 卡已经取过，这里只用来显示"当前 → 目标"） */
  currentVersion: string | null
}

export default function ReleaseGateModal({ onClose, currentVersion }: Props) {
  const [tab, setTab] = useState<'gate' | 'receipt' | 'history'>('gate')
  /** 闸的结果；null = 还没算回来 */
  const [pre, setPre] = useState<ReleasePreflight | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  /** 回执：非 null = 这一屏在收尾视图里 */
  const [report, setReport] = useState<ReleaseTxReport | null>(null)
  /**
   * 目标版本号。**默认沿用当前真值**（作者：版本号只有一处真值，人改一次就够）——
   * 要发新版才改这一格。空 = 沿用。
   */
  const [version, setVersion] = useState('')
  /** 一句话说明：进 Release 正文与 `release.json` 的 notes */
  const [notes, setNotes] = useState('')
  const [history, setHistory] = useState<ReleaseHistory | null>(null)

  /** 跑一遍闸。**只读** —— 后端那条命令一个字节都不写，"重新检查"点几次都没副作用 */
  const run = useCallback(async () => {
    setError(null)
    try {
      const p = await wb.releasePreflight(version.trim() === '' ? null : version.trim())
      setPre(p)
      if (version.trim() === '') setVersion(p.currentVersion)
    } catch (e) {
      setPre(null)
      setError(isAppError(e) ? e.message : String(e))
    }
  }, [version])

  useEffect(() => {
    void run()
    // ★ 只在**打开时**跑一次：`version` 一变就重跑会把人正在输入的东西冲掉
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const openTab = async (next: typeof tab) => {
    setTab(next)
    if (next === 'history' && history === null) {
      try {
        setHistory(await wb.releaseHistory())
      } catch (e) {
        setError(isAppError(e) ? e.message : String(e))
      }
    }
  }

  /** 真发：一次手势走完内核那八步（**以分钟计**：要构建并上传安装包） */
  const release = async () => {
    setBusy(true)
    setError(null)
    try {
      const rep = await wb.releaseSoftware({
        version: version.trim() === '' ? null : version.trim(),
        notes: notes.trim(),
        merge: true,
      })
      setReport(rep)
      setTab('receipt')
    } catch (e) {
      // 发挂了就把闸留在原地 —— 回执是给"走到了"用的，别拿它兜错误
      setError(isAppError(e) ? e.message : String(e))
    } finally {
      setBusy(false)
    }
  }

  const openUrl = async (url: string) => {
    if (url === '') return
    try {
      await wb.openExternal(url)
    } catch (e) {
      setError(isAppError(e) ? e.message : String(e))
    }
  }

  /**
   * run-env 拦下时的一键解法：杀掉本进程自己的 dev 监视器（后端先验明正身再动手，
   * 只杀 CLI 本尊 —— 窗口与 vite 都活着）→ 自动重跑闸。PID 以最近一次预检为准。
   */
  const killWatcher = async () => {
    if (pre?.devWatcherPid == null) return
    setBusy(true)
    setError(null)
    try {
      await wb.killDevWatcher(pre.devWatcherPid)
      await run()
    } catch (e) {
      setError(isAppError(e) ? e.message : String(e))
    } finally {
      setBusy(false)
    }
  }

  /* ---------- 历史 ---------- */

  if (tab === 'history') {
    return (
      <ModalC14
        open
        size="lg"
        title="软件版本发布历史"
        subtitle={history === null ? '正在读……' : `${history.records.length} 条 · 最新在前`}
        closeOnScrim={false}
        onClose={onClose}
        footer={
          <>
            <span className={s.footNote}>★ 这是「发布软件版本」那本账 —— 发布预设的回执在另一本里</span>
            <span className={c.grow} />
            <button type="button" className={c.btn} onClick={() => void openTab('gate')}>
              回闸
            </button>
            <button type="button" className={c.btn} onClick={onClose}>
              关闭
            </button>
          </>
        }
      >
        {error !== null && (
          <p className={s.error} role="alert">
            {error}
          </p>
        )}
        {history !== null && history.records.length === 0 ? (
          <p className={s.empty}>还没有发过软件版本。</p>
        ) : (
          <div className={s.hist}>
            {(history?.records ?? []).map((r: ReleaseRecord, i) => (
              <div key={`${r.tag}-${i}`} className={s.histRow}>
                <div className={s.histHead}>
                  <span className={s.histTag}>{r.tag}</span>
                  <span className={s.histAt}>{localStamp(r.at)}</span>
                  <span className={s.histAt}>{STAGE_TEXT[r.stage] ?? r.stage}</span>
                </div>
                <span className={s.histNote}>{r.summary}</span>
              </div>
            ))}
          </div>
        )}
      </ModalC14>
    )
  }

  /* ---------- 回执 ---------- */

  if (tab === 'receipt' && report !== null) {
    const steps = receiptSteps(report)
    return (
      <ModalC14
        open
        size="lg"
        title="发布回执 · 软件版本"
        subtitle={`${STAGE_TEXT[report.stage] ?? report.stage} · ${report.tag}`}
        closeOnScrim={false}
        onClose={onClose}
        footer={
          <>
            <span className={s.footNote}>{report.summary}</span>
            <span className={c.grow} />
            <button type="button" className={c.btn} onClick={() => void openTab('history')}>
              历史
            </button>
            <button type="button" className={c.btn} onClick={onClose}>
              关闭
            </button>
            {report.infoReview !== null && report.infoReview.url !== '' && (
              <button
                type="button"
                className={`${c.btn} ${c.btnPrimary}`}
                title="★ 合并它之后客户端才看得到新版本"
                onClick={() => void openUrl(report.infoReview?.url ?? '')}
              >
                去合并 release.json 的 PR
              </button>
            )}
          </>
        }
      >
        <div className={s.chain} role="list">
          {steps.map((st) => (
            <div key={st.key} className={s.step} data-state={st.state} role="listitem">
              <span className={s.stepMark}>{st.state === 'done' ? '✓' : '·'}</span>
              <b className={s.stepLabel}>{st.label}</b>
              <span className={s.stepNote}>{st.note}</span>
            </div>
          ))}
        </div>

        {report.release !== null && report.release.url !== '' && (
          <p className={s.prLine}>
            Release：<span className={c.mono}>{report.release.url}</span>
          </p>
        )}
        {report.infoReview !== null && report.infoReview.url !== '' && (
          <p className={s.prLine}>
            release.json 的 PR：<span className={c.mono}>{report.infoReview.url}</span>
          </p>
        )}
        {error !== null && (
          <p className={s.error} role="alert">
            {error}
          </p>
        )}
      </ModalC14>
    )
  }

  /* ---------- 闸 ---------- */

  const items: PreflightItem[] = pre?.items ?? []
  const failed = items.filter((i) => i.status === 'fail').length

  const subtitle =
    pre === null
      ? error === null
        ? '正在跑检查……'
        : '没跑起来'
      : pre.canRelease
        ? `可以发 ${pre.tag}`
        : `${failed} 项拦住发版`

  return (
    <ModalC14
      open
      size="lg"
      title="发布软件版本"
      subtitle={subtitle}
      closeOnScrim={false}
      onClose={onClose}
      footer={
        <>
          <span className={s.footNote}>
            {pre === null
              ? '闸还没跑完 —— 跑完才谈得上发不发'
              : pre.canRelease
                ? '全绿才发：这一趟会切 main、打 tag、构建并上传安装包'
                : '有项没过：修完点「重新检查」'}
          </span>
          <span className={c.grow} />
          <button type="button" className={c.btn} onClick={onClose} disabled={busy}>
            取消
          </button>
          <button type="button" className={c.btn} onClick={() => void run()} disabled={busy}>
            重新检查
          </button>
          <button type="button" className={c.btn} onClick={() => void openTab('history')} disabled={busy}>
            历史
          </button>
          <button
            type="button"
            className={`${c.btn} ${c.btnPrimary}`}
            disabled={!pre?.canRelease || busy}
            title={pre?.canRelease ? undefined : '闸没全绿 —— 这颗按钮不亮'}
            onClick={() => void release()}
          >
            {busy ? '发布中…（要构建并上传安装包，几分钟）' : '确认发布'}
          </button>
        </>
      }
    >
      {/*
       * ★ 危险动作单独一块说（作者的"破坏性命令要讲明白"）：
       * 这一趟会**切换本机的当前分支**到 main 并在其 tip 上打 tag。
       */}
      <div className={s.warn}>
        <div className={s.warnTitle}>这一趟会动本机的工作区</div>
        切到 main → 在 main 的 tip 上打 tag（squash 会重写提交，tag 必须打在 main 上）→
        构建安装包（macOS → dmg / Windows → NSIS）→ 建 Release 并上传 → 写 release.json
        并开第二个 PR。
        <br />
        ★ 这一版**已经发过**时只补**本平台**的包：不重打 tag、不开 PR、不动版本号 ——
        同一个版本可以同时挂 macOS 与 Windows 的安装包（同一个平台不重发）。
        <br />
        ★ release.json 那个 PR 要**你自己合并** —— 合并之后客户端才看得到新版本。
      </div>

      <div className={s.form}>
        <div className={s.field}>
          <span className={s.fieldLabel}>目标版本</span>
          <input
            className={`${s.input} ${c.mono}`}
            value={version}
            disabled={busy}
            placeholder={currentVersion ?? '沿用当前版本'}
            onChange={(e) => setVersion(e.target.value)}
            onBlur={() => void run()}
          />
          <span className={s.fieldLabel}>一句话说明</span>
          <input
            className={s.input}
            value={notes}
            disabled={busy}
            placeholder="会成为 Release 正文"
            onChange={(e) => setNotes(e.target.value)}
          />
        </div>
      </div>

      {error !== null && (
        <p className={s.error} role="alert">
          {error}
        </p>
      )}

      <div className={s.list}>
        {items.map((it) => (
          <div key={it.id} className={`${s.row} ${it.status === 'fail' ? s.rowFail : ''}`}>
            <span className={`${s.st} ${it.status === 'pass' ? s.stPass : s.stFail}`}>
              {it.status === 'pass' ? '过' : '拦'}
            </span>
            <span className={s.body}>
              <span className={s.head}>
                <span className={s.name}>{it.label}</span>
                <span className={s.id}>{it.id}</span>
              </span>
              <span className={s.details}>{it.detail}</span>
              {it.id === 'run-env' && it.status === 'fail' && pre?.devWatcherPid != null && (
                <button
                  type="button"
                  className={`${c.btn} ${c.btnDanger}`}
                  disabled={busy}
                  title="只杀 tauri dev 的 CLI 监视进程 —— 本窗口与前端 HMR 都不受影响"
                  onClick={() => void killWatcher()}
                >
                  杀掉 dev 监视进程（PID {pre.devWatcherPid}）
                </button>
              )}
            </span>
          </div>
        ))}
        {pre === null && error === null && <p className={s.pending}>正在跑检查……</p>}
      </div>
    </ModalC14>
  )
}
