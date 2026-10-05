/*
 * **标题栏的下载指示器 + 详情面板**（2026-10-05 第五刀）。
 *
 * 作者要的是"像 Trae / WorkBuddy 那样"：**状态栏一个小图标、环形进度、点开是详情面板
 * （下载 / 暂停 / 取消）、下载完出现「重启并安装」**。所以：
 *
 * - **常驻标题栏**（不在设置页里）—— 用户在哪一页都看得见它在下载；
 * - **没有任务时什么都不占**（`state = idle` 直接不渲染，不留一个灰图标占位）；
 * - 进度是**事件**推来的（`software-update-progress`），状态是**快照**问来的
 *   （`updateInfo`）—— 界面重新挂上（切窗口、刷新）问一次就知道现在怎样。
 *
 * # 为什么不在首屏问
 *
 * 铁律 2：云端不参与首屏。所以这个组件**不自己问**"有没有新版"，
 * 只在**有任务时**订阅事件 + 显示；`updateInfo` 由设置页问过一次顺手带回来
 * （`PageSettings` 把 `hasUpdate` 写进 store），这里读那份。
 */
import { useCallback, useEffect, useState } from 'react'

import { api, errorText, listen } from '../../api'
import type { UpdateInfo, UpdateState } from '../../api/contract'
import s from './UpdateIndicator.module.css'

/** 进度环那一圈（SVG 直径，stroke 居中画在半径上） */
const RING = 13
const CIRC = 2 * Math.PI * RING

/** 字节数 → 人话（MB / GB；`total = 0` 就是"总量未知"） */
function mb(v: number): string {
  if (v >= 1024 * 1024 * 1024) return `${(v / 1024 / 1024 / 1024).toFixed(2)} GB`
  if (v >= 1024 * 1024) return `${(v / 1024 / 1024).toFixed(1)} MB`
  return `${Math.max(1, Math.round(v / 1024))} KB`
}

/** 进度环那一格显示什么（**百分比优先**；总量未知时给"已下多少"） */
export function progressLabel(state: UpdateState): string {
  switch (state.state) {
    case 'downloading':
    case 'paused':
      return state.total > 0
        ? `${Math.round((state.received / state.total) * 100)}%`
        : mb(state.received)
    case 'ready':
      return '可安装'
    case 'failed':
      return '失败'
    default:
      return ''
  }
}

/** 这一态要不要在标题栏露脸（**没任务就不占位**） */
export function isVisible(state: UpdateState): boolean {
  return state.state === 'downloading' || state.state === 'paused' || state.state === 'ready' || state.state === 'failed'
}

interface Props {
  /** 装好了（退出后重启的账变了） */
  onInstalled: (version: string) => void
}

export default function UpdateIndicator({ onInstalled }: Props) {
  const [state, setState] = useState<UpdateState>({ state: 'idle' })
  /** 有新版吗 + 那个安装包（`null` = 还没问 / 问不到） */
  const [info, setInfo] = useState<UpdateInfo | null>(null)
  const [open, setOpen] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  /* 进度事件：后端每 120ms 推一次同一个 `UpdateState` 形状 */
  useEffect(() => {
    let alive = true
    const stop = listen<UpdateState>('software-update-progress', (next) => {
      if (alive) setState(next)
    })
    return () => {
      alive = false
      void stop
    }
  }, [])

  /* 问一次"有没有新版可下"。
     ★ **挂载后延迟 1.2s 再问**：首屏不等它（铁律 2 —— 云端不参与首屏），
     而它问回来的也只是"要不要在标题栏露一个图标"，晚一点毫无损失。 */
  useEffect(() => {
    const t = setTimeout(() => {
      void api
        .updateInfo()
        .then(setInfo)
        .catch(() => setInfo(null))
    }, 1200)
    return () => clearTimeout(t)
  }, [])

  const idle: UpdateState = { state: 'idle' }
  const shown = state.state === 'idle' ? idle : state

  const act = useCallback(async (fn: () => Promise<void>) => {
    setBusy(true)
    setError(null)
    try {
      await fn()
    } catch (e) {
      setError(errorText(e))
    } finally {
      setBusy(false)
    }
  }, [])

  /* 装完重启：真机上本进程会退出；演示里只是把账记上 */
  const install = () =>
    act(async () => {
      const latest = info?.latestVersion ?? ''
      await api.installUpdate()
      onInstalled(latest)
    })

  if (!isVisible(shown)) return null

  const ring =
    shown.state === 'downloading' || shown.state === 'paused'
      ? shown.total > 0
        ? (shown.received / shown.total) * CIRC
        : CIRC * 0.25 /* 总量未知：给一小段"还在动"的暗示，不假装知道多少 */
      : shown.state === 'ready'
        ? CIRC
        : CIRC * 0.15

  return (
    <div className={s.wrap} data-testid="update-indicator">
      <button
        type="button"
        className={s.icon}
        title={progressLabel(shown)}
        aria-label={`软件更新：${progressLabel(shown)}`}
        onClick={() => setOpen((v) => !v)}
        disabled={busy}
      >
        <svg viewBox="0 0 32 32" className={s.ring} aria-hidden="true">
          <circle cx="16" cy="16" r={RING} className={s.track} />
          <circle
            cx="16"
            cy="16"
            r={RING}
            className={s.bar}
            strokeDasharray={CIRC}
            strokeDashoffset={CIRC - ring}
          />
        </svg>
        <span className={s.label}>{progressLabel(shown)}</span>
      </button>

      {open && (
        <div className={s.panel} role="dialog" aria-label="软件更新">
          <div className={s.head}>
            <b>软件更新</b>
            <button type="button" className={s.close} onClick={() => setOpen(false)} aria-label="关闭">
              ×
            </button>
          </div>

          {shown.state === 'downloading' && (
            <p className={s.line}>
              正在下载 {mb(shown.received)}
              {shown.total > 0 ? ` / ${mb(shown.total)}` : ''}
            </p>
          )}
          {shown.state === 'paused' && (
            <p className={s.line}>已暂停（{mb(shown.received)}）</p>
          )}
          {shown.state === 'ready' && <p className={s.line}>下载完成，可以重启安装。</p>}
          {shown.state === 'failed' && <p className={`${s.line} ${s.bad}`}>下载失败：{shown.reason}</p>}
          {shown.state === 'cancelled' && <p className={s.line}>已取消。</p>}

          <div className={s.actions}>
            {shown.state === 'downloading' && (
              <>
                <button type="button" className={s.btn} disabled={busy} onClick={() => void act(() => api.pauseUpdate())}>
                  暂停
                </button>
                <button type="button" className={s.btn} disabled={busy} onClick={() => void act(() => api.cancelUpdate())}>
                  取消
                </button>
              </>
            )}
            {shown.state === 'paused' && (
              <>
                <button type="button" className={s.btn} disabled={busy} onClick={() => void act(() => api.resumeUpdate())}>
                  继续
                </button>
                <button type="button" className={s.btn} disabled={busy} onClick={() => void act(() => api.cancelUpdate())}>
                  取消
                </button>
              </>
            )}
            {shown.state === 'ready' && (
              <button type="button" className={`${s.btn} ${s.btnPrimary}`} disabled={busy} onClick={() => void install()}>
                重启并安装
              </button>
            )}
            {shown.state === 'failed' && (
              <button
                type="button"
                className={s.btn}
                disabled={busy}
                onClick={() => void act(() => api.startUpdate())}
              >
                重试
              </button>
            )}
          </div>

          {/* 「下载这一版」：只有 release.json 给了 asset 才有得下 */}
          {info?.hasUpdate === true && info.asset !== undefined && shown.state !== 'ready' && (
            <button
              type="button"
              className={`${s.btn} ${s.btnWide}`}
              disabled={busy || shown.state === 'downloading' || shown.state === 'paused'}
              onClick={() => void act(() => api.startUpdate())}
            >
              下载 {info.latestVersion}（{mb(info.asset.size)}）
            </button>
          )}

          {error !== null && (
            <p className={s.bad} role="alert">
              {error}
            </p>
          )}
        </div>
      )}
    </div>
  )
}
