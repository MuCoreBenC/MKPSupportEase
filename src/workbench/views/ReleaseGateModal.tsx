/**
 * **发布软件版本**的闸 → 回执 → 历史（第四刀）。
 *
 * 与 [`PublishGateModal`]（发布预设那道闸）是**两道闸**：这里看的是"能不能发一个
 * 新的安装包"（分支 / 工作区 / 版本号一致 / tag 有没有被占 / 平台支不支持软件 Release），
 * 那边看的是"预设交出去会不会缺东西"。判定**整份来自 Rust**（`wb_release_preflight`
 * 与内核同一个函数）—— 这一屏没有第二套检查。
 *
 * # 两段视图
 *
 * ```text
 * 闸   逐项结果（红的染整行）→ 全绿才亮「确认发布」
 * 回执 阶段链：版本号 → 提交 → PR → 合并 → tag → 构建 → Release → release.json 的 PR
 * ```
 *
 * 发布历史**不在框里**（2026-10-07 作者要的：历史放在外面，不塞模态框）——
 * 它摆在页面「软件版本」卡上（[`BuildPage`]），这里发完关框就能看见。
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
  ReleasePreflight,
  ReleaseTxReport,
} from '../api'
import ModalC14 from '../c14/ModalC14'
import { toasts } from '../c14/toast'
import { releaseStageText as STAGE_TEXT } from '../c14/labels'
import c from '../c14.module.css'
import s from './ReleaseGateModal.module.css'

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

interface Props {
  onClose: () => void
  /** 当前安装的版本（② 卡已经取过，这里只用来显示"当前 → 目标"） */
  currentVersion: string | null
}

export default function ReleaseGateModal({ onClose, currentVersion }: Props) {
  const [tab, setTab] = useState<'gate' | 'receipt'>('gate')
  /** 闸的结果；null = 还没算回来 */
  const [pre, setPre] = useState<ReleasePreflight | null>(null)
  const [error, setError] = useState<string | null>(null)
  /**
   * 错误的**详情**（Rust 的 `detail`：git 的 stderr、平台回的那句话）。
   *
   * ★ 只说标题等于没说 —— 2026-10-07 真机就卡在这儿：界面上只剩一句
   * "git push 失败了"，而真正的原因（本地 main 陈旧 → 非快进 → 本地闸③）躺在
   * detail 里，得去翻日志才看得见。两行都要摆出来。
   */
  const [errorDetail, setErrorDetail] = useState<string | null>(null)
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

  /** 记一条错误：`message` 是标题，`detail` 是"为什么" */
  const fail = (e: unknown) => {
    if (isAppError(e)) {
      setError(e.message)
      setErrorDetail(e.detail ?? null)
    } else {
      setError(String(e))
      setErrorDetail(null)
    }
  }

  const clearError = () => {
    setError(null)
    setErrorDetail(null)
  }

  /** 跑一遍闸。**只读** —— 后端那条命令一个字节都不写，"重新检查"点几次都没副作用 */
  const run = useCallback(async () => {
    clearError()
    try {
      const p = await wb.releasePreflight(version.trim() === '' ? null : version.trim())
      setPre(p)
      if (version.trim() === '') setVersion(p.currentVersion)
    } catch (e) {
      setPre(null)
      fail(e)
    }
  }, [version])

  useEffect(() => {
    void run()
    // ★ 只在**打开时**跑一次：`version` 一变就重跑会把人正在输入的东西冲掉
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  /** 真发：一次手势走完内核那八步（**以分钟计**：要构建并上传安装包） */
  const release = async () => {
    setBusy(true)
    clearError()
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
      fail(e)
    } finally {
      setBusy(false)
    }
  }

  const openUrl = async (url: string) => {
    if (url === '') return
    try {
      await wb.openExternal(url)
    } catch (e) {
      fail(e)
    }
  }

  /**
   * run-env 拦下时的一键解法：杀掉本进程自己的 dev 监视器（后端先验明正身再动手，
   * 只杀 CLI 本尊 —— 窗口与 vite 都活着）→ 自动重跑闸。PID 以最近一次预检为准。
   */
  const killWatcher = async () => {
    if (pre?.devWatcherPid == null) return
    setBusy(true)
    clearError()
    try {
      await wb.killDevWatcher(pre.devWatcherPid)
      await run()
    } catch (e) {
      fail(e)
    } finally {
      setBusy(false)
    }
  }

  /**
   * **复制发布提示词**（2026-10-07 作者要的"傻瓜化"出口）：把"这一版要怎么发"整成一段
   * 任务书贴给 AI —— 版本号、更新说明、发版纪律、产物名、`release.json` 落点全在里面，
   * 它照着仓库自己的发版路径做，人不必在这颗按钮上冒险。
   *
   * 文本由 Rust 生成（事实只有一处），这里只负责取回来 + 放进剪贴板。
   */
  const copyPrompt = async () => {
    setBusy(true)
    clearError()
    try {
      const text = await wb.releasePrompt(
        version.trim() === '' ? null : version.trim(),
        notes.trim(),
      )
      await navigator.clipboard.writeText(text)
      toasts.push('发布提示词已复制 —— 贴给 AI 即可（任务书里带着版本号、更新说明与发版纪律）')
    } catch (e) {
      fail(e)
    } finally {
      setBusy(false)
    }
  }

  /** 错误块：标题 + 详情 —— 三段视图共用（详情就是"为什么"，不许只给标题） */
  const errorBox =
    error === null ? null : (
      <div role="alert">
        <p className={s.error}>{error}</p>
        {errorDetail !== null && errorDetail.trim() !== '' && (
          <pre className={s.errorDetail}>{errorDetail.trim()}</pre>
        )}
      </div>
    )

  /* ---------- 回执 ---------- */

  if (tab === 'receipt' && report !== null) {
    const steps = receiptSteps(report)
    return (
      <ModalC14
        open
        size="xl"
        title="发布回执 · 软件版本"
        subtitle={`${STAGE_TEXT[report.stage] ?? report.stage} · ${report.tag}`}
        closeOnScrim={false}
        onClose={onClose}
        footer={
          <div className={s.footCol}>
            <span className={s.footNote}>{report.summary}</span>
            <div className={s.footBtns}>
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
            </div>
          </div>
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
        {errorBox}
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
      size="xl"
      title="发布软件版本"
      subtitle={subtitle}
      closeOnScrim={false}
      onClose={onClose}
      footer={
        /*
         * 底部**两行**（2026-10-07）：提示独占一行、按钮另起一行 —— 之前五个按钮和
         * 一句长提示挤同一行，按钮把提示挤成竖排的字，footer 被顶得老高（作者：
         * "按钮太多了，才导致文字变得这么长这么高"）。宽的那档（xl）也是给它的。
         */
        <div className={s.footCol}>
          <span className={s.footNote}>
            {pre === null
              ? '闸还没跑完 —— 跑完才谈得上发不发'
              : pre.canRelease
                ? '全绿才发：这一趟会切 main、打 tag、构建并上传安装包'
                : '有项没过：修完点「重新检查」'}
          </span>
          <div className={s.footBtns}>
            <button
              type="button"
              className={c.btn}
              disabled={busy}
              title="把这—版要怎么发（版本号 / 更新说明 / 发版纪律 / 产物名）生成一段任务书并复制 —— 贴给 AI，由它照仓库自己的发版路径做"
              onClick={() => void copyPrompt()}
            >
              复制发布提示词
            </button>
            <button type="button" className={c.btn} onClick={onClose} disabled={busy}>
              取消
            </button>
            <button type="button" className={c.btn} onClick={() => void run()} disabled={busy}>
              重新检查
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
          </div>
        </div>
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
                  className={`${c.btn} ${c.btnDanger} ${s.rowAction}`}
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
