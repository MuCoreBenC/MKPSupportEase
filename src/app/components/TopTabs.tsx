import { useLayoutEffect, useRef, useState } from 'react'

import Icon from '../../components/Icon'
import UpdateIndicator from './UpdateIndicator'
import { useWindowMaximized } from '../../hooks/useWindowMaximized'
import { useTitlebarDrag } from '../useTitlebarDrag'
import { winClose, winMinimize, winToggleMaximize } from '../window'
import type { Density } from '../../hooks/useDensity'
import type { Platform } from '../../hooks/usePlatform'
import s from './TopTabs.module.css'

interface Tab {
  id: string
  label: string
}

interface TopTabsProps {
  tabs: Tab[]
  active: string
  onChange: (id: string) => void
  density: Density
  platform: Platform
  /** 开启后页签间距/字号按容器宽度连续缩放，不再随密度档位跳变 */
  fluid?: boolean
  /** 传了就只显示这一行标题，页签条整个不渲染 —— 报告这类全屏子视图用 */
  title?: string
}


/* macOS 上交通灯由系统画在我们这条标题栏上（titleBarStyle: "Overlay"），
   这里只留出它占的那块地，别让 MKP 字样压在下面。
   77 是 AppKit 量出来的：装了统一工具栏之后红绿灯整组右缘 79、第一个 toolbar item 左缘 97，
   减掉标题栏自己的左内衬 16 与 gap 之后剩这么宽。60 / 72 都偏窄。 */
const MAC_LIGHTS_INSET = 77

/*
 * **mini 档不再收图标**（作者 2026-10-04：600 宽下七个文字页签放得下，「不用变成
 * 图标」）—— 曾经 mini 档整条换成图标页签（TAB_ICONS 表 + `icon` 口子，给试验稿
 * 换图标用），把中段宽度还给拖动区；文字版实测只差几十像素，图标版退役，
 * 任何档都是文字页签。真不够宽时 `.tabs` 自己横向滑（样式里留着的兜底）。
 */

export default function TopTabs({
  tabs,
  active,
  onChange,
  density,
  platform,
  fluid,
  title,
}: TopTabsProps) {

  const isMac = platform === 'macos'
  const maximized = useWindowMaximized()
  const drag = useTitlebarDrag()

  /*
   * 滑动下划线（2026-10-05）：原来 active 页签的 ::after 直接出现/消失，快速换
   * 页签时下划线在两处「眨」。现在是一条常驻指示条，换页签时从旧位滑到新位。
   * 几何全部量 DOM：左右内衬直接读页签自己的 padding（compact / mini / fluid
   * 三档差异就在那儿，量出来自动跟随），页签条任何尺寸变化（拉窗、换密度、
   * 字体就绪、active 加粗引起的 1px 重排）都由 ResizeObserver 重测。
   * `latest` 让 RO 与换页签两条路都调到最新一次渲染的测量函数，不用重订 RO。
   */
  const navRef = useRef<HTMLElement | null>(null)
  const tabEls = useRef(new Map<string, HTMLButtonElement>())
  const [ind, setInd] = useState<{ x: number; w: number } | null>(null)
  const latest = useRef<() => void>(() => {})

  const syncIndicator = () => {
    const nav = navRef.current
    const el = tabEls.current.get(active)
    if (!nav || !el) return
    const navLeft = nav.getBoundingClientRect().left
    const rect = el.getBoundingClientRect()
    const padX = parseFloat(getComputedStyle(el).paddingLeft) || 0
    setInd({ x: rect.left - navLeft + padX, w: rect.width - padX * 2 })
  }

  useLayoutEffect(() => {
    latest.current = syncIndicator
  })

  useLayoutEffect(() => {
    latest.current()
  }, [active])

  useLayoutEffect(() => {
    const nav = navRef.current
    if (!nav) return
    const ro = new ResizeObserver(() => latest.current())
    ro.observe(nav)
    for (const el of tabEls.current.values()) ro.observe(el)
    return () => ro.disconnect()
  }, [])

  const wideStrip = (
    <nav ref={navRef} className={s.tabs} aria-label="主页签">
      {tabs.map((t) => {
        const on = t.id === active
        return (
          <button
            key={t.id}
            type="button"
            className={s.tab}
            data-on={on}
            aria-current={on ? 'page' : undefined}
            onClick={() => onChange(t.id)}
            ref={(el) => {
              if (el) tabEls.current.set(t.id, el)
              else tabEls.current.delete(t.id)
            }}
          >
            {t.label}
          </button>
        )
      })}
      {ind && (
        <span
          aria-hidden="true"
          className={s.indicator}
          style={{ width: ind.w, transform: `translateX(${ind.x}px)` }}
        />
      )}
    </nav>
  )

  /* 有 title 时页签条整个不渲染：报告态标题栏只剩品牌 + 页名 + 窗口键，中段全是拖动区 */
  const tabStrip = title ? <span className={s.title}>{title}</span> : wideStrip

  return (
    <header
      className={s.bar}
      onMouseDown={drag.onMouseDown}
      onDoubleClick={drag.onDoubleClick}
      data-density={density}
      data-platform={platform}
      data-fluid={fluid ? 'true' : undefined}
    >

      {/* 系统交通灯的地盘：只占位，不画东西 —— 画的那三颗在系统那一层 */}
      {isMac && <span
          className={s.lightsInset}
          style={{ width: MAC_LIGHTS_INSET }}
          aria-hidden="true"
        />}

      <span className={s.logo}>MKP</span>

      {tabStrip}

      {/*
        下载指示器（2026-10-05 第五刀）：**在页签与窗口键之间**，任何页面都看得见。
        它**自己取数据**（不靠 props 层层传）：`idle` 时**什么都不渲染**，
        所以首屏不因为它多等一毫秒（铁律 2：云端不参与首屏）。
      */}
      <UpdateIndicator onInstalled={() => window.location.reload()} />

      {/* Windows 侧的窗口键：撑满标题栏全高、贴到右上角、彼此无缝。
          按钮是 <button>，useTitlebarDrag 里 closest('button') 会跳过它们，
          不会误把点击窗口键当成拖窗或双击缩放 */}
      {!isMac && (
        <div className={s.controls}>
          <button type="button" className={s.ctrl} aria-label="最小化" onClick={winMinimize}>
            <Icon name="min" size={11} strokeWidth={1.5} />
          </button>
          <button
            type="button"
            className={s.ctrl}
            aria-label={maximized ? '还原' : '最大化'}
            onClick={winToggleMaximize}
          >
            <Icon name={maximized ? 'restore' : 'max'} size={11} strokeWidth={1.5} />
          </button>
          <button type="button" className={`${s.ctrl} ${s.close}`} aria-label="关闭" onClick={winClose}>
            <Icon name="close" size={11} strokeWidth={1.5} />
          </button>
        </div>
      )}
    </header>
  )
}
