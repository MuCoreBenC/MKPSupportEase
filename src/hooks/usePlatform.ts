import type { StringKey } from '../i18n'

/**
 * 目标平台。
 *
 * 试验场那份是给预览器用的常量（平台下拉 + 模拟窗口的圆角），因为在浏览器里"探到的是本机
 * 而非目标平台"。产品里跑在真窗口里，本机就是目标平台，所以这里改成真探测。
 *
 * 用途只有一个：标题栏该把窗口按钮画在左边（macOS 交通灯）还是右边（Windows）。
 * 将来接 `@tauri-apps/plugin-os` 可以拿到更准的值，但那要等 Rust 侧起来；
 * UA 判断在这件事上够用，而且浏览器调试路径也能工作。
 */
export type Platform = 'windows' | 'macos'

export function detectPlatform(): Platform {
  if (typeof navigator === 'undefined') return 'macos'
  return /mac/i.test(navigator.userAgent) ? 'macos' : 'windows'
}

/** 平台在进程里不会变 ⇒ 模块级算一次，各处别再各自 `detectPlatform()` 一遍 */
export const PLATFORM: Platform = detectPlatform()

/*
 * ---------- 键位说法（2026-10-30）----------
 *
 * 作者：「只是担心没有按平台显示，看看还有什么地方没有按平台的……参考别人成熟的方案
 * 做一下自动平台区分显示，还有快捷键也是」。
 *
 * 成熟做法（VS Code / Figma / Slack 同一套）：
 *   · **显示**用符号：mac `⌘ ⇧ ⌥` 连写（`⇧⌘Z`），Windows `Ctrl+Shift+Alt` 加号连接；
 *   · **组合整体按平台变**，不是只换修饰键 —— 重做 mac 是 `⇧⌘Z`、Windows 是 `Ctrl+Y`，
 *     两边结构都不一样，拼修饰键一定拼错；
 *   · **无障碍属性与显示分开**：`aria-keyshortcuts` 是给读屏软件的，只认
 *     `Meta+` / `Control+` 这类键名，符号读不出来。
 *
 * ★ 只管**说出来的那串字**。键位绑定不动（那些地方一直是 `metaKey || ctrlKey` 双认），
 *   所以这次零功能风险，改的只是"话对不对"。
 */

export const MOD_KEY = PLATFORM === 'windows' ? 'Ctrl' : '⌘'
export const ALT_KEY = PLATFORM === 'windows' ? 'Alt' : '⌥'
export const SHIFT_KEY = PLATFORM === 'windows' ? 'Shift' : '⇧'

export type ComboName = 'search' | 'save' | 'undo' | 'redo' | 'wrap'

/** 界面上出现的那一串组合键（给人看） */
export function comboOf(name: ComboName): string {
  switch (name) {
    case 'search':
      return PLATFORM === 'windows' ? `${MOD_KEY}+F` : `${MOD_KEY}F`
    case 'save':
      return PLATFORM === 'windows' ? `${MOD_KEY}+S` : `${MOD_KEY}S`
    case 'undo':
      return PLATFORM === 'windows' ? `${MOD_KEY}+Z` : `${MOD_KEY}Z`
    /* 重做：mac 是 ⇧⌘Z（Shift 在前 ⌘ 在后，与 VS Code 一致），Windows 是 Ctrl+Y */
    case 'redo':
      return PLATFORM === 'windows' ? `${MOD_KEY}+Y` : `${SHIFT_KEY}${MOD_KEY}Z`
    case 'wrap':
      return PLATFORM === 'windows' ? `${ALT_KEY}+Z` : `${ALT_KEY}Z`
  }
}

/** `aria-keyshortcuts` 那一串（给读屏软件，只认键名，不认符号） */
export function ariaOf(name: ComboName): string {
  switch (name) {
    case 'search':
      return PLATFORM === 'windows' ? 'Control+F' : 'Meta+F'
    case 'save':
      return PLATFORM === 'windows' ? 'Control+S' : 'Meta+S'
    case 'undo':
      return PLATFORM === 'windows' ? 'Control+Z' : 'Meta+Z'
    case 'redo':
      return PLATFORM === 'windows' ? 'Control+Y' : 'Meta+Shift+Z'
    case 'wrap':
      return 'Alt+Z'
  }
}

/*
 * 「打开所在文件夹」那颗的文案键 —— mac 说访达、Windows 说文件夹。
 * 底下是 `opener().reveal_item_in_dir()`（选中该文件），不是"打开目录"。
 */
export const REVEAL_LABEL_KEY: StringKey =
  PLATFORM === 'windows' ? 'presets.page.revealWindows' : 'presets.page.revealMac'

/*
 * 页脚「没有鼠标就…」那一句：Windows 有 Shift+F10 和菜单键，mac 两样都没有
 * ⇒ mac 写 Ctrl+点按（作者 2026-10-30 追问后定的两条）。
 */
export const FOOT_MENU_HINT_KEY: StringKey =
  PLATFORM === 'windows' ? 'presets.page.menuKey.win' : 'presets.page.menuKey.mac'
