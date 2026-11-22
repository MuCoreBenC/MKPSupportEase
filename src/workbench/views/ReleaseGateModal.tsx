/**
 * **这一代发布到哪一步**（发布内核 2.0）—— 只读。
 *
 * 工作台不再有自己的发版路径：发布只有 `npm run ship -- --what stable --confirm`
 * 一条，状态从远端现读（Gitee / GitHub / main 清单 / 正式 tag），谁没齐就补谁。
 *
 * 这一屏能做的事只有三件：看阶段、看「平台 × 源」矩阵、复制那条命令。
 * 危险动作（打 tag / 传包 / 写清单）不在界面上 —— 绕过状态机比少一个按钮危险得多。
 */
import { useCallback, useEffect, useState } from 'react'

import { isAppError, wb } from '../api'
import type { PlatformStage, ReleasePhase, ReleaseStateView } from '../api'
import ModalC14 from '../c14/ModalC14'
import { toasts } from '../c14/toast'
import c from '../c14.module.css'
import s from './ReleaseGateModal.module.css'

const PHASE_TEXT: Record<ReleasePhase, string> = {
  preparing: '准备中',
  frozen: '源码已冻结',
  building: '平台构建中',
  staged: '平台资产已暂存',
  manifestMerged: '清单已合并',
  verified: '核验通过',
  sealed: '已封版',
}

const STAGE_TEXT: Record<PlatformStage, string> = {
  missing: '待构建',
  partial: '部分完成',
  staged: '已暂存',
  published: '已发布',
}

const PHASES: ReleasePhase[] = [
  'preparing',
  'frozen',
  'building',
  'staged',
  'manifestMerged',
  'verified',
  'sealed',
]

interface Props {
  onClose: () => void
  /** 当前安装的版本（只用来显示"当前 → 目标"） */
  currentVersion: string | null
}

export default function ReleaseGateModal({ onClose, currentVersion }: Props) {
  const [state, setState] = useState<ReleaseStateView | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [errorDetail, setErrorDetail] = useState<string | null>(null)

  const load = useCallback(async () => {
    setError(null)
    setErrorDetail(null)
    try {
      setState(await wb.releaseState(null))
    } catch (e) {
      if (isAppError(e)) {
        setError(e.message)
        setErrorDetail(e.detail ?? null)
      } else {
        setError('读不到发布状态')
        setErrorDetail(String(e))
      }
    }
  }, [])

  useEffect(() => {
    void load()
  }, [load])

  const copyCommand = useCallback(async () => {
    try {
      const text = await wb.releasePrompt()
      await navigator.clipboard.writeText(text)
      toasts.push('已复制发布命令')
    } catch {
      toasts.push('复制失败')
    }
  }, [])

  const reached = state ? PHASES.indexOf(state.phase) : -1

  return (
    <ModalC14
      open
      title="发布软件版本"
      subtitle={state ? `这一代：${state.version}` : undefined}
      size="md"
      onClose={onClose}
      footer={
        <div className={s.footBtns}>
          <button type="button" className={c.btn} onClick={onClose}>
            关闭
          </button>
          <button type="button" className={c.btn} onClick={() => void load()}>
            重新读
          </button>
          <button type="button" className={c.btnPrimary} onClick={() => void copyCommand()}>
            复制命令
          </button>
        </div>
      }
    >
      <div className={s.body}>
        <p className={s.footNote}>
          当前安装 {currentVersion ?? '未知'} · 发布只走命令行一条路
        </p>

        {error && (
          <div className={s.error}>
            <div>{error}</div>
            {errorDetail && <div className={s.errorDetail}>{errorDetail}</div>}
          </div>
        )}

        {state && (
          <>
            <div className={s.chain}>
              {PHASES.map((p, i) => (
                <span
                  key={p}
                  className={i <= reached ? s.step : s.pending}
                  title={p}
                >
                  {PHASE_TEXT[p]}
                </span>
              ))}
            </div>

            <div className={s.list}>
              {state.platforms.map((p) => (
                <div key={p.id} className={s.row}>
                  <span className={s.name}>{p.label}</span>
                  <span className={p.giteeDone ? s.stPass : s.rowWarn}>
                    Gitee {p.giteeDone ? '齐' : '缺'}
                  </span>
                  <span className={p.githubDone ? s.stPass : s.rowWarn}>
                    GitHub {p.githubDone ? '齐' : '缺'}
                  </span>
                  <span className={s.st}>{STAGE_TEXT[p.stage]}</span>
                </div>
              ))}
            </div>

            <div className={s.details}>
              <div>
                main 上的清单：
                {state.manifest.complete
                  ? ` ${state.manifest.version} 已含全部平台`
                  : ` ${state.manifest.version ?? '读不到'}（缺 ${state.manifest.missing.length} 份）`}
              </div>
              {state.sourceCommit && <div>冻结源码：{state.sourceCommit.slice(0, 7)}</div>}
              <div>正式 tag：{state.tag} · 暂存 tag：{state.stagingTag}</div>
            </div>

            {state.conflicts.length > 0 && (
              <div className={s.error}>
                {state.conflicts.map((x) => (
                  <div key={x}>{x}</div>
                ))}
              </div>
            )}

            <div className={s.notice}>
              下一步：{state.next}
            </div>
          </>
        )}
      </div>
    </ModalC14>
  )
}
