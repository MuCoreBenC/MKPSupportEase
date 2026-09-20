/** 目标平台。预览器里跑，不做自动探测——探到的是本机而非目标平台。 */
export type Platform = 'windows' | 'macos'

export const PLATFORMS: { id: Platform; label: string }[] = [
  { id: 'windows', label: 'Windows' },
  { id: 'macos', label: 'macOS' },
]

export const DEFAULT_PLATFORM: Platform = 'windows'

/** 各平台的窗口圆角，供预览器的窗口框使用 */
export const WINDOW_RADIUS: Record<Platform, number> = {
  windows: 8,
  macos: 10,
}
