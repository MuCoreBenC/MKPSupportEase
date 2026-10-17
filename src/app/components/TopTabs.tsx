import { useCallback, useLayoutEffect, useRef, useState } from 'react'

import Icon from '../../components/Icon'
import UpdateIndicator from './UpdateIndicator'
import PostProcessChip from '../postprocess/PostProcessChip'
import { useActivePreset } from '../state/appState'
/* 「复制后处理脚本」走全站那一份（不再自己调 api / 自己写通知）—— 见那个文件的文件头 */
import { copyPostProcessScript } from '../postprocess/copyScript'
import type { PostProcessRunView } from '../postprocess/usePostProcessRun'
import { useDevPreview } from '../dev/useDevPreview'
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
  /**
   * 后处理那一趟（后台跑着的时候标题栏要露一枚小胶囊）。
   *
   * ★ 2026-10-17：这条身份原来住在**页签下面一条 in-flow 的横条**里，一出现就把整页
   *   往下推（作者：「出现这个会把页面往下我不喜欢」）。现在与更新指示器并排，
   *   不占内容流 —— 这一格由外壳（`App.tsx`）喂进来，标题栏自己不去读那一趟。
   */
  postProcess?: PostProcessRunView
}


/*
 * macOS 上交通灯由**系统/我们自己摆在窗口图层上**（不在 DOM 里，见 `chrome.rs` 的
 * `reposition_traffic_lights`），这里只留出它占的那块地，别让 MKP 字样压在下面。
 *
 * ★★★ 2026-10-15 第七版：**78**（标题栏 52 → 38 之后重量的）。
 *
 * 这句话现在的语义是「**从灯组右缘到 MKP 字样左缘**要留多宽」—— 直接量出来的：
 * ```text
 *   绿灯右缘 70.5  ·  MKP 文字左缘 108   ⇒  差 37.5
 * ```
 * 但 `.lightsInset` 是个**参与 flex 的占位块**，它的宽度要先被 `.bar` 的左内衬
 * （macOS 档 14）与 `gap`（16）吃掉一部分，所以：
 * ```text
 *   14（padding-left） + INSET + 16（gap） = 108（MKP 左缘）  ⇒  INSET = 78
 * ```
 * ★ 写成"差 37.5"而不是 78 是不行的 —— 那 37.5 是**视觉缝隙**，而这里要的是
 * **占位块宽**；两者差一个左内衬 + 一个 gap。作者说过"左右不一致时先量清在比哪两个数"，
 * 这里同理：**先分清"视觉缝隙"与"参与布局的宽度"**。
 *
 * ★ 老的 77 是 52 档下按另一组几何凑的（当时绿灯右缘 79、MKP 左缘按 toolbar 的
 * 第一个 item 算到 97）。那组数随标题栏高度一起变了，所以必须重新量。
 *
 * ★★★ 2026-10-16：**78 → 66**。作者：「这个字再往左靠一点，靠近那个红绿灯」。
 * ⇒ 视觉缝隙从 37.5 收到 **25.5**（绿灯右缘 70.5 + 25.5 = 96 = 新 MKP 左缘）。
 * 别收到 20 以下 —— 红绿灯与字之间要有"看得出来的呼吸"，挨太近会被读成
 * "字压在灯上"（尤其绿灯和小写字母混在一起时）。
 */
const MAC_LIGHTS_INSET = 66

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
  postProcess,
}: TopTabsProps) {

  const isMac = platform === 'macos'
  const maximized = useWindowMaximized()
  const drag = useTitlebarDrag()
  /* 开发台：点「软件更新提示」时把下载指示器摆成"正在下载"那一态（正式构建里恒 false） */
  const { on: devUpdate } = useDevPreview('update-prompt')

  /*
   * `MKP` 字标 = 「复制后处理脚本」的**快捷入口**（2026-10-16 作者）。
   *
   * 作者：「到时候我知道这个图标啊，到时候可以点击点击了就是复制那个脚本」+「让用户自己
   * 摸索到就算了」⇒ 不做引导层、不加常驻说明，只给**可按压的观感**（悬停微亮、按下微沉，
   * 见 CSS）+ 点完在右上角发一条通知。
   *
   * 复制的对象是**当前已应用的那一份**（`useActivePreset`）—— 与首页页脚那颗
   * 「复制脚本」判据一致（那份才"真在盘上"）。没应用任何一份时**不装死**：
   * 发一条说明原因的通知（不是灰掉 —— 作者否过灰态：「用户根本不知道是禁用」）。
   *
   * ★ 2026-10-16：从"自己管一颗 toast"改成调**全站通知系统**（`notify`）。
   *   原来那套（`TitlebarToast` + 本地 state）**已删** —— 它和更新面板各写了一套材质，
   *   深色适配就漏了一处（作者报的"深色下发白"）。现在**只有一套**，改主题只改一处。
   */
  const activePreset = useActivePreset()

  const copyScript = useCallback(() => {
    const ap = activePreset
    if (ap === null) {
      /* 这一段话归 `copyPostProcessScript` 里那句（传空串就走那一支），这里只负责"没有" */
      void copyPostProcessScript({ machineId: '', versionId: '' })
      return
    }
    /*
     * ★ 走**全站那一份**（`copyPostProcessScript`）—— 与预设页右键那一项、与首页那颗
     *   同一个出口、同一句话（作者 2026-10-16：「左上角的 mkp 点击也是，显示一样的」）。
     *   原来这里自己调 `api.getPostProcessCommand` + `clipboard` + 自己写通知，
     *   与右键那一项就是两套话术 —— 那种"同一件事写两遍"正是深色下漏改的源头。
     */
    void copyPostProcessScript({
      machineId: ap.machineId,
      versionId: ap.versionId,
      fileName: ap.fileName,
    })
  }, [activePreset])

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

      {/*
        `MKP` 字标：既是品牌标记，也是「复制后处理脚本」的快捷入口（见 `copyScript`）。
        ★ 用 `<button>` 而不是给它挂 onClick 的 `<span>`：键盘可达、语义正确，
        且 `user-select: none`（全站那层）本来就不让选它，不冲突。
        ★ 悬停/按压的观感全在 CSS（`data-clickable`），这里不管。
      */}
      <button
        type="button"
        className={s.logo}
        data-clickable="true"
        title="把这一份的后处理脚本复制到剪贴板"
        onClick={copyScript}
      >
        MKP
      </button>

      {tabStrip}

      {/*
        后处理在后台跑的那枚小胶囊（2026-10-17）：与更新指示器**同一种身子、不同图标**。
        它只在「用户把那一屏收起来了、而那一趟还在跑」时出现；
        `run = null` / 已经出结论时什么都不渲染（结论会把那一屏自己叫回来）。
        ★ 它不是流程里的一条、不占内容流 —— 出现与消失都不推任何一页。
      */}
      {postProcess && <PostProcessChip view={postProcess} />}

      {/*
        下载指示器（2026-10-05 第五刀）：**在页签与窗口键之间**，任何页面都看得见。
        它**自己取数据**（不靠 props 层层传）：`idle` 时**什么都不渲染**，
        所以首屏不因为它多等一毫秒（铁律 2：云端不参与首屏）。
      */}
      <UpdateIndicator
        onInstalled={() => window.location.reload()}
        /* 开发台：点「软件更新提示」时摆出"正在下载"那一态（正式构建里恒 undefined） */
        devPreview={devUpdate ? { state: 'downloading', received: 41_943_040, total: 67_108_864 } : undefined}
      />

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
