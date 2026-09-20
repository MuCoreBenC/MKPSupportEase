import Icon from '../../components/Icon'
import type { IconName } from '../../components/Icon'
import type { Density } from '../../hooks/useDensity'
import type { Platform } from '../../hooks/usePlatform'
import s from './TopTabs.module.css'

interface Tab {
  id: string
  label: string
  /**
   * 迷你档要用的图标。不传就按 id 查下面那张 TAB_ICONS —— 老稿全都不传，走原路。
   * 这个口子是给"同一个 id、这一稿想换个图标"用的（v022 的设置换成真齿轮就走这里），
   * 改 TAB_ICONS 会把 #5…#21 的标题栏一起改掉。
   */
  icon?: IconName
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


const lights = [
  { id: 'close', label: '关闭' },
  { id: 'min', label: '最小化' },
  { id: 'zoom', label: '缩放' },
]

/**
 * 迷你档用图标代替文字，好把标题栏中段的宽度还给「拖窗口」。
 * 查不到的 id 退回显示文字 —— 任何稿传进来的自定义页签都不会变成空方块。
 */
const TAB_ICONS: Record<string, IconName> = {
  machine: 'printer',
  preset: 'presets',
  calib: 'crosshair',
  params: 'params',
  report: 'doc',
  settings: 'settings',
}

export default function TopTabs({
  tabs,
  active,
  onChange,
  density,
  platform,
  fluid,
  title,
}: TopTabsProps) {

  const isMini = density === 'mini'
  const isMac = platform === 'macos'

  const iconStrip = (
    <nav className={s.iconTabs} aria-label="主页签">
      {tabs.map((t) => {
        const on = t.id === active
        const icon = t.icon ?? TAB_ICONS[t.id]
        return (
          <button
            key={t.id}
            type="button"
            className={icon ? s.iconTab : s.tab}
            data-on={on}
            aria-current={on ? 'page' : undefined}
            aria-label={icon ? t.label : undefined}
            title={t.label}
            onClick={() => onChange(t.id)}
          >
            {icon ? <Icon name={icon} size={17} /> : t.label}
          </button>
        )
      })}
    </nav>
  )

  const wideStrip = (
    <nav className={s.tabs} aria-label="主页签">
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
          >
            {t.label}
          </button>
        )
      })}
    </nav>
  )

  /* 有 title 时页签条整个不渲染：报告态标题栏只剩品牌 + 页名 + 窗口键，中段全是拖动区 */
  const tabStrip = title ? (
    <span className={s.title}>{title}</span>
  ) : isMini ? (
    iconStrip
  ) : (
    wideStrip
  )

  return (
    <header
      className={s.bar}
      data-density={density}
      data-platform={platform}
      data-fluid={fluid ? 'true' : undefined}
    >

      {isMac && (
        <div className={s.lights}>
          {lights.map((l) => (
            <button
              key={l.id}
              type="button"
              className={s.light}
              data-kind={l.id}
              aria-label={l.label}
            />
          ))}
        </div>
      )}

      <span className={s.logo}>MKP</span>

      {tabStrip}

      {!isMac && (
        <div className={s.controls}>
          <button type="button" className={s.ctrl} aria-label="最小化">
            <Icon name="min" size={14} />
          </button>
          <button type="button" className={s.ctrl} aria-label="最大化">
            <Icon name="max" size={12} />
          </button>
          <button type="button" className={`${s.ctrl} ${s.close}`} aria-label="关闭">
            <Icon name="close" size={14} />
          </button>
        </div>
      )}
    </header>
  )
}
