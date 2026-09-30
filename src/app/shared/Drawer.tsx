/*
 * 右抽屉的外壳。
 *
 * # 一律 absolute，不用 fixed
 *
 * 窗口外壳上只要有一层 `transform`，它就成了 `position: fixed` 的包含块：
 * 浮层量的是视口坐标、却被当成局部坐标用，偏移一百多像素之后再被 `overflow: hidden`
 * 裁掉 —— 试验场里那两个「点了没反应」的浮层就栽在这上面。
 * 所以抽屉与浮层全部 absolute，挂在页面自己的定位祖先上（页面根节点是 relative）。
 *
 * # 遮罩只盖内容区
 *
 * 由调用方决定挂在哪一层：参数页把它挂在 `.main` 里，于是底栏与窗口右下角那个 resize 手柄
 * 还点得到 —— 这是特意调过的，别改成整页遮罩。
 */

import { useEffect } from 'react'
import type { ReactNode } from 'react'
import Icon from '../shell/icons'
import type { DrawerWidth } from './useDrawerWidth'
import s from './Drawer.module.css'

/**
 * 悬浮抽屉与窗口左缘之间必须留出的空隙。
 *
 * 真软件里窗口最左、最下那几条边是**拖拽改窗口尺寸**的热区（Wails / Electron 的
 * resize 边框），而抽屉自己的左缘也是一条拖宽热区（`.grip`）—— 抽屉贴到窗口左缘，
 * 这两条就叠在一起：想拖窗口会变成拖抽屉。作者：「不要到最最左边，要给一定的空间，
 * 不然它最左边是拖这个软件的，跟它自己的这个宽度就冲突了」。
 *
 * 56px：比两条热区加起来宽得多，一眼也看得出抽屉没贴着边。
 * 由 `.hold` 与这里的宽度上限共同保证 —— 抽屉再宽也要先让出这一段。
 */
const EDGE_GAP = 56

interface Props {
  open: boolean
  title: string
  /** 标题下面那行小字 */
  subtitle?: string
  /** 标题右边的附加动作（如「固定在右侧」），关闭按钮由抽屉自己画 */
  actions?: ReactNode
  /** `pinned` = 占住右侧一条、不带遮罩；`float` = 盖在内容上、带遮罩 */
  mode?: 'float' | 'pinned'
  /**
   * 悬浮抽屉的受控宽度（px）+ 左缘拖宽热区（`shared/useDrawerWidth` 的返回值）。
   * 传了才启用：面板宽度跟着走、左缘多出一条 6px 的拖宽热区。不传 = 维持默认 380px。
   */
  width?: number
  drag?: DrawerWidth
  footer?: ReactNode
  onClose: () => void
  children: ReactNode
}

export default function Drawer({
  open,
  title,
  subtitle,
  actions,
  mode = 'float',
  width,
  drag,
  footer,
  onClose,
  children,
}: Props) {
  useEffect(() => {
    if (!open) return
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    document.addEventListener('keydown', onKey)
    return () => document.removeEventListener('keydown', onKey)
  }, [open, onClose])

  if (!open) return null

  const panel = (
    <aside
      className={mode === 'pinned' ? s.pinned : s.panel}
      role="dialog"
      aria-label={title}
      style={mode === 'float' && width != null ? { width: '100%' } : undefined}
    >
      {mode === 'float' && drag !== undefined && (
        <span className={s.grip} {...drag.gripProps} />
      )}
      <header className={s.head}>
        <span className={s.headText}>
          <strong className={s.title}>{title}</strong>
          {subtitle !== undefined && <span className={s.sub}>{subtitle}</span>}
        </span>
        <span className={s.headActs}>
          {actions}
          <button type="button" className={s.close} aria-label="关闭" onClick={onClose}>
            <Icon name="close" size={15} />
          </button>
        </span>
      </header>

      <div className={s.body}>{children}</div>

      {footer !== undefined && <footer className={s.foot}>{footer}</footer>}
    </aside>
  )

  if (mode === 'pinned') return panel

  return (
    <div className={s.scrim} role="presentation" onClick={onClose}>
      <div
        className={s.hold}
        role="presentation"
        onClick={(e) => e.stopPropagation()}
        style={
          width != null ? { width: `min(${width}px, calc(100% - ${EDGE_GAP}px))` } : undefined
        }
      >
        {panel}
      </div>
    </div>
  )
}
