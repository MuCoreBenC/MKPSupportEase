/*
 * 调色板：颜色一律从 `style.css` 的 CSS 变量取，所以「跟着系统深浅色走」只实现一次
 * —— 媒体查询在 CSS 里，这里只负责把当前那套值交给 canvas 与 WebGL。
 *
 * 缓存到下一次主题切换：`getComputedStyle` 每次都要样式计算，2D 每帧读一遍不值当。
 * 系统换主题时 `matchMedia` 会响，清缓存并通知各处重画。
 */

export interface Palette {
  bg: string
  bg1: string
  bg2: string
  line: string
  text: string
  dim: string
  amber: string
  teal: string
  blue: string
  red: string
  /** 空走的颜色（带透明度） */
  up: string
  /** 网格线的颜色（带透明度） */
  grid: string
}

let cached: Palette | null = null

function readVar(name: string, fallback: string): string {
  const v = getComputedStyle(document.documentElement).getPropertyValue(name).trim()
  return v === '' ? fallback : v
}

export function palette(): Palette {
  if (cached !== null) return cached
  cached = {
    bg: readVar('--bg', '#0e1116'),
    bg1: readVar('--bg-1', '#151a21'),
    bg2: readVar('--bg-2', '#1c232c'),
    line: readVar('--line', '#232a33'),
    text: readVar('--text', '#e6edf3'),
    dim: readVar('--dim', '#8b949e'),
    amber: readVar('--amber', '#f0a020'),
    teal: readVar('--teal', '#2bb6a8'),
    blue: readVar('--blue', '#4c8dff'),
    red: readVar('--red', '#e5484d'),
    up: readVar('--up', 'rgba(139, 148, 158, 0.30)'),
    grid: readVar('--grid', 'rgba(35, 42, 51, 0.9)'),
  }
  return cached
}

/** CSS 颜色 → WebGL 用的 `0xRRGGBB`；读不出就退回给定值 */
export function hex(css: string, fallback: number): number {
  const m = /^#([0-9a-f]{6})$/i.exec(css.trim())
  return m === null ? fallback : Number.parseInt(m[1] as string, 16)
}

/** 系统深浅色切换时清缓存并回调（回调里负责重画） */
export function onThemeChange(cb: () => void): void {
  const mq = window.matchMedia('(prefers-color-scheme: dark)')
  mq.addEventListener('change', () => {
    cached = null
    cb()
  })
}
