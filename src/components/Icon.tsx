import css from './Icon.module.css'

export type IconName =
  | 'home'
  | 'files'
  | 'params'
  | 'presets'
  | 'history'
  | 'settings'
  | 'gear'
  | 'folder'
  | 'target'
  | 'move'
  | 'doc'
  | 'chevron'
  | 'bolt'
  | 'clock'
  | 'drop'
  | 'crosshair'
  | 'thermo'
  | 'bed'
  | 'fan'
  | 'check'
  | 'printer'
  | 'menu'
  | 'min'
  | 'max'
  | 'close'

const paths: Record<IconName, string> = {
  home: 'M3 10.5 12 3l9 7.5V20a1 1 0 0 1-1 1h-5v-6H9v6H4a1 1 0 0 1-1-1z',
  files: 'M4 6a2 2 0 0 1 2-2h4l2 2h6a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2z',
  params: 'M4 7h10M18 7h2M4 17h4M12 17h8M4 12h6M14 12h6M16 5v4M10 10v4M10 15v4',
  presets: 'M5 4h14a1 1 0 0 1 1 1v15l-4-2-4 2-4-2-4 2V5a1 1 0 0 1 1-1z',
  history: 'M12 7v5l3.5 2M21 12a9 9 0 1 1-3.2-6.9M21 3v4h-4',
  settings:
    'M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM12 2v3M12 19v3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M2 12h3M19 12h3M4.9 19.1 7 17M17 7l2.1-2.1',
  /*
   * 真齿轮：齿廓是轮廓本身的一条闭合多边形，不是挂在圆外的辐条。
   *
   * 上一版画成"圆 + 8 条短线"，线帽一圆就还是太阳 —— 辐条与圆之间有缝，
   * 眼睛读到的是"圆心发光"。这一版没有缝：6 颗梯形齿，齿顶 R9.0、齿根 r6.3，
   * 齿顶弦 28°、齿根弦 16°，齿数压到 6 是为了 17px 下每颗齿还有 1.7px 以上的实体宽度。
   * 中心另画一个 r2.6 的轴孔，与齿根之间留 3.7 的空当，1.7 的描边不会糊在一起。
   *
   * 坐标是算出来的（中心 12,12，按角度取点），别手改单个数 —— 改了齿就不等分。
   * 老稿还用着 settings 那条（小圆 + 长辐条），不去动它。
   */
  gear:
    'M20.73 9.82 20.73 14.18 17.84 14.36 16.96 15.88 18.25 18.47 14.48 20.65 12.88 18.24 11.12 18.24 9.52 20.65 5.75 18.47 7.04 15.88 6.16 14.36 3.27 14.18 3.27 9.82 6.16 9.64 7.04 8.12 5.75 5.53 9.52 3.35 11.12 5.76 12.88 5.76 14.48 3.35 18.25 5.53 16.96 8.12 17.84 9.64ZM12 9.4a2.6 2.6 0 1 0 0 5.2 2.6 2.6 0 0 0 0-5.2z',
  folder: 'M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z',
  target: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM12 16a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM12 13v-1',
  move: 'M12 3v18M3 12h18M12 3 9 6M12 3l3 3M12 21l-3-3M12 21l3-3M3 12l3-3M3 12l3 3M21 12l-3-3M21 12l-3 3',
  doc: 'M6 3h8l4 4v14H6zM14 3v4h4M9 12h6M9 16h4',
  chevron: 'M9 6l6 6-6 6',
  bolt: 'M13 2 5 14h5l-1 8 8-12h-5z',
  clock: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM12 7v5l3.5 2',
  drop: 'M12 3s6 6.5 6 10.5A6 6 0 0 1 6 13.5C6 9.5 12 3 12 3z',
  crosshair: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM12 3v4M12 17v4M3 12h4M17 12h4',
  thermo: 'M14 14.8V5a2 2 0 1 0-4 0v9.8a4 4 0 1 0 4 0zM12 9v6',
  bed: 'M4 16h16M6 16V9l6-4 6 4v7M9 16v-4h6v4',
  fan: 'M12 12a4 4 0 0 0 4-4c0-2-1.8-3-4-3S8 6 8 8M12 12a4 4 0 0 0-4 4c0 2 1.8 3 4 3s4-1 4-3M12 12a4 4 0 0 0 2 3.5c1.7 1 3.5.6 4.6-1.3M12 12a4 4 0 0 0-2-3.5C8.3 7.5 6.5 7.9 5.4 9.8',
  check: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM8.5 12.5l2.5 2.5 4.5-5',
  printer: 'M7 9V4h10v5M7 18H5a1 1 0 0 1-1-1v-6a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v6a1 1 0 0 1-1 1h-2M7 14h10v6H7z',
  min: 'M5 12h14',
  max: 'M6 6h12v12H6z',
  close: 'M6 6l12 12M18 6 6 18',
  menu: 'M4 7h16M4 12h16M4 17h16',
}

interface IconProps {
  name: IconName
  /** 像素尺寸；省略则跟随 --icon 变量 */
  size?: number
  strokeWidth?: number
  className?: string
}

export default function Icon({ name, size, strokeWidth = 1.7, className }: IconProps) {
  return (
    <svg
      className={className ? `${css.icon} ${className}` : css.icon}
      style={size ? { width: size, height: size } : undefined}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      <path d={paths[name]} />
    </svg>
  )
}
