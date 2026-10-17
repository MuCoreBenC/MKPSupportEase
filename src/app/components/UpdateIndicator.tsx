/*
 * **标题栏的下载指示器 + 详情面板**（2026-10-05 第五刀）。
 *
 * 作者要的是"像 Trae / WorkBuddy 那样"：**状态栏一个小图标、点开是详情面板
 * （下载 / 暂停 / 取消）、下载完出现「重启并安装」**。
 *
 * 进度**原来是圆环**（`stroke-dashoffset` 画一圈），2026-10-17 作者改口：
 * 「更新的也可以不用圆环，用下载的动态的图标」—— 于是那一格换成
 * **一枚会往下跑的下载箭头 + 百分比**（WAAPI 驱动，见 `DownloadIcon`）：
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
import { useCallback, useEffect, useRef, useState } from 'react'

import { api, errorText, listen } from '../../api'
import { dismiss, notify, update, type Notice, type NotifyAction } from '../notify/notify'
import type { UpdateInfo, UpdateState } from '../../api/contract'
import TitlebarChip from './TitlebarChip'

/**
 * 下载那一枚（16×16）：**托盘不动，箭头往下走** —— "正在把东西取回来"最省话的动法。
 *
 * ★ 动效走 WAAPI（`element.animate()`），不走 CSS animation：WKWebView 上 CSS 动画
 *   不播（`app/postprocess/HANDOVER.md` §4）。暂停时停在原位、失败时干脆不动。
 * ★ 淡入淡出收在关键帧里（末尾 opacity 归 0 再从头起），循环那一瞬不跳。
 */
function DownloadIcon({ active }: { active: boolean }) {
  const arrowRef = useRef<SVGGElement | null>(null)

  useEffect(() => {
    const el = arrowRef.current
    if (el === null || !active) return
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return

    const anim = el.animate(
      [
        { transform: 'translateY(-2px)', opacity: 0 },
        { transform: 'translateY(-1px)', opacity: 1, offset: 0.25 },
        { transform: 'translateY(2px)', opacity: 1, offset: 0.7 },
        { transform: 'translateY(2.6px)', opacity: 0 },
      ],
      { duration: 1100, iterations: Infinity, easing: 'ease-in-out', fill: 'both' },
    )
    return () => anim.cancel()
  }, [active])

  return (
    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth={1.5} aria-hidden="true">
      <g ref={arrowRef}>
        <path d="M8 2.6v7.6M5 7.4 8 10.4l3-3" strokeLinecap="round" strokeLinejoin="round" />
      </g>
      <path
        d="M4 11.6v1.4a1 1 0 0 0 1 1h6a1 1 0 0 0 1-1v-1.4"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  )
}

/** 字节数 → 人话（MB / GB；`total = 0` 就是"总量未知"） */
function mb(v: number): string {
  if (v >= 1024 * 1024 * 1024) return `${(v / 1024 / 1024 / 1024).toFixed(2)} GB`
  if (v >= 1024 * 1024) return `${(v / 1024 / 1024).toFixed(1)} MB`
  return `${Math.max(1, Math.round(v / 1024))} KB`
}

/** 标题栏那一格显示什么（**百分比优先**；总量未知时给"已下多少"）—— 只本文件用，不导出 */
function progressLabel(state: UpdateState): string {
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

/** 这一态要不要在标题栏露脸（**没任务就不占位**）—— 只本文件用，不导出 */
function isVisible(state: UpdateState): boolean {
  return state.state === 'downloading' || state.state === 'paused' || state.state === 'ready' || state.state === 'failed'
}

interface Props {
  /** 装好了（退出后重启的账变了） */
  onInstalled: (version: string) => void
  /**
   * 开发台：**不用真事件也摆出某一态**（作者 2026-10-14）。
   *
   * 平时这一格的状态全是事件 / 云端推来的（`software-update-progress` + `updateInfo`），
   * 演示时没办法即兴造一场"正在下载 62%"出来 —— 所以留这一个口子：给了它就用它，
   * 面板默认展开。**正式代码不传它 ⇒ 行为一字不变**。
   *
   * 只影响"摆哪一态"，那几颗按钮（暂停 / 取消 / 重试）照旧调真 api ——
   * 演示时不点它们（点了也只是如实报"现在没有下载"）。
   */
  devPreview?: UpdateState
}

/** `buildActions` 要的那几件事（避免在调用处写一大坨内联函数） */
interface ActionDeps {
  pause: () => void
  resume: () => void
  cancel: () => void
  install: () => void
  retry: () => void
  /** `null` = 现在没有可下的一版（`release.json` 没给 asset） */
  download: (() => void) | null
  latestVersion: string
  assetSize: number
}

/**
 * 按当前状态拼出**该露哪几颗按钮**。
 *
 * ★ 2026-10-16：从原来那段 JSX 里原样搬过来（四态 → 按钮的对应关系一个字没改），
 * 只是产出从 `<button>` 变成 `NotifyAction[]`。
 * ★ `disabled` 只给"正在忙"那一档 —— 原来那几颗的禁用条件也是这个。
 */
function buildActions(
  shown: UpdateState,
  busy: boolean,
  d: ActionDeps,
): NotifyAction[] | undefined {
  switch (shown.state) {
    case 'downloading':
      return [
        { label: '暂停', onClick: d.pause, disabled: busy },
        { label: '取消', onClick: d.cancel, disabled: busy },
      ]
    case 'paused':
      return [
        { label: '继续', onClick: d.resume, disabled: busy },
        { label: '取消', onClick: d.cancel, disabled: busy },
      ]
    case 'ready':
      return [{ label: '重启并安装', onClick: d.install, disabled: busy, primary: true }]
    case 'failed':
      return [{ label: '重试', onClick: d.retry, disabled: busy }]
    case 'cancelled':
      /* 取消之后还能重来一遍（要真有可下的那一版） */
      return d.download !== null
        ? [
            {
              label: `下载 ${d.latestVersion}（${mb(d.assetSize)}）`,
              onClick: d.download,
              disabled: busy,
            },
          ]
        : undefined
    default:
      /* idle：什么也不露（"有没有新版"那件事由指示器那颗图标自己说） */
      return d.download !== null
        ? [
            {
              label: `下载 ${d.latestVersion}（${mb(d.assetSize)}）`,
              onClick: d.download,
              disabled: busy,
            },
          ]
        : undefined
  }
}

export default function UpdateIndicator({ onInstalled, devPreview }: Props) {
  const [state, setState] = useState<UpdateState>({ state: 'idle' })
  /** 有新版吗 + 那个安装包（`null` = 还没问 / 问不到） */
  const [info, setInfo] = useState<UpdateInfo | null>(null)
  const [open, setOpen] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  /* 演示态：面板默认摊开，一眼看到那一屏 */
  const demo = devPreview !== undefined
  useEffect(() => {
    if (demo) setOpen(true)
  }, [demo])

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
  /* 演示态优先（给了它就用它，不看真事件） */
  const shown = devPreview ?? (state.state === 'idle' ? idle : state)

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

  /*
   * ★★★ 2026-10-16：面板从"自绘"改成**推给全站通知系统**（作者定的统一方案）。
   *
   * 三条纪律：
   *  - `sticky`：**不自动消失**（`ttl: 0`）。它有"暂停 / 取消"这种必须点得到的按钮，
   *    飘走了用户就操作不了。要收掉只能点 ×（或状态自己走到终态）。
   *  - **同一条通知就地改名/改进度**（`update`），不每次新开一条 ——
   *    进度事件每 120ms 推一次，新开一条会瞬间糊满右上角。
   *  - 卸载 / 收起时 `dismiss` 收回，不留残影。
   *
   * ★ 进度用 `progress`（0~1）而不是把百分比写进 `text`：通知卡片自己画进度条，
   *   两条通知（更新 + 复制）的形态才真正是同一种。
   */
  const noticeId = useRef<string | null>(null)

  useEffect(() => {
    if (!isVisible(shown) || !open) {
      /* 收起：把这条收回（面板关了就不该继续挂在那儿） */
      if (noticeId.current !== null) {
        dismiss(noticeId.current)
        noticeId.current = null
      }
      return
    }

    const pct =
      (shown.state === 'downloading' || shown.state === 'paused') && shown.total > 0
        ? shown.received / shown.total
        : undefined

    const body: Omit<Notice, 'id'> = {
      kind: error !== null ? 'bad' : shown.state === 'failed' ? 'bad' : shown.state === 'ready' ? 'ok' : 'info',
      title: '软件更新',
      /* ★ 操作失败（`error`，如点"暂停"时后端报错）**优先于**状态文案 ——
         原来它显示在面板底部，现在并入正文，不许静默吞掉。 */
      text:
        error !== null
          ? error
          : shown.state === 'downloading'
            ? `正在下载 ${mb(shown.received)}${shown.total > 0 ? ` / ${mb(shown.total)}` : ''}`
            : shown.state === 'paused'
              ? `已暂停（${mb(shown.received)}）`
              : shown.state === 'ready'
                ? '下载完成，可以重启安装。'
                : shown.state === 'failed'
                  ? `下载失败：${shown.reason}`
                  : shown.state === 'cancelled'
                    ? '已取消。'
                    : '',
      progress: pct,
      /* 不自动走（有按钮）。用户点 × 收掉 */
      ttl: 0,
      actions: buildActions(shown, busy, {
        pause: () => void act(() => api.pauseUpdate()),
        resume: () => void act(() => api.resumeUpdate()),
        cancel: () => void act(() => api.cancelUpdate()),
        install: () => void install(),
        retry: () => void act(() => api.startUpdate()),
        download:
          info?.hasUpdate === true && info.asset !== undefined
            ? () => void act(() => api.startUpdate())
            : null,
        latestVersion: info?.latestVersion ?? '',
        assetSize: info?.asset?.size ?? 0,
      }),
    }

    if (noticeId.current === null) {
      noticeId.current = notify(body)
    } else {
      update(noticeId.current, body)
    }
  }, [shown, open, busy, info, error, act]) // eslint-disable-line react-hooks/exhaustive-deps

  /* 卸载：把这条收回（切窗口 / 换页时不留孤儿通知） */
  useEffect(
    () => () => {
      if (noticeId.current !== null) dismiss(noticeId.current)
    },
    [],
  )

  if (!isVisible(shown)) return null

  return (
    <TitlebarChip
      testId="update-indicator"
      icon={<DownloadIcon active={shown.state === 'downloading'} />}
      label={progressLabel(shown)}
      tone={
        shown.state === 'failed'
          ? 'danger'
          : shown.state === 'paused'
            ? 'muted'
            : 'accent'
      }
      title={`软件更新：${progressLabel(shown)}（点击查看详情）`}
      ariaLabel={`软件更新：${progressLabel(shown)}`}
      onClick={() => setOpen((v) => !v)}
      disabled={busy}
    />
  )
}
