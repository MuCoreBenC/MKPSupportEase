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
