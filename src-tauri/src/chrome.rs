//! 窗口外观：原生圆角与原生红绿灯。
//!
//! 两件事都不是 CSS 能做的，走法是从 `neteasemusicc` 那个项目抄来的（它在这两点上是对的）：
//!
//! **红绿灯位置**：Tauri 的 `titleBarStyle: "Overlay"` 只给窗口加了 `fullSizeContentView`，
//! **没有**真正的 `NSToolbar`。同一套 styleMask 下 AppKit 摆灯的两档实测值：
//!
//! | 配置 | 标题栏高 | close 圆心 | min | zoom |
//! | --- | --- | --- | --- | --- |
//! | 只有 fullSizeContentView | 32 | (16, 16) | (39, 16) | (62, 16) |
//! | 带 NSToolbar `.unified` | 52 | (26, 26) | (49, 26) | (72, 26) |
//!
//! 我们的标题栏是 52px（`--titlebar-h`），所以要装一个**空**工具栏 —— 不放任何 item，
//! 控件全由 WebView 画，只借它的布局结果。`trafficLightPosition` 这条配置在 Overlay 下
//! 不生效（试过），而逐颗改 `standardWindowButton(...).frame` 要在 resize / 进出全屏 /
//! 切外观时各补摆一次，每次补摆都有一帧错位 —— 交回给系统算才是稳的。
//!
//! **圆角**：这是 tauri#14165。窗口的原生边框是圆的，但 WKWebView 没有被裁 ——
//! 它把内容画进圆角外面，看上去就是直角。`windowEffects.radius` 只圆了底下那层
//! NSVisualEffectView，盖在上面的 webview 依旧没裁。解法是把 webview 从窗口 contentView
//! 摘下来、挂进一层玻璃视图里，**圆角由 AppKit 裁**，和系统其它窗口一样。
//! `opaque(true)` 是关键：这层玻璃只给底色带上环境色与高光，不负责透出桌面 ——
//! 我们的界面本身是不透明的浅色渐变，传 false 会让背后的窗口整片读进来。
//!
//! 低于 macOS 26 的系统上 `apply_liquid_glass` 会返回 `UnsupportedPlatformVersion`
//! （crate 自己比 `NSAppKitVersionNumber`），那时就只打一条日志、不退回毛玻璃 ——
//! 毛玻璃会把界面变成半透明，那是比直角更大的观感改变。

use tauri::{Runtime, WebviewWindow};

/// 主窗口圆角。Tahoe 带工具栏的窗口就是 26
#[cfg(target_os = "macos")]
const MAIN_RADIUS: f64 = 26.0;

/// 让 AppKit 按「统一工具栏」那一档摆红绿灯。失败只打日志 —— 灯偏 10px 是外观问题，
/// 不该让启动挂掉。
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
pub fn install_unified_toolbar<R: Runtime>(win: &WebviewWindow<R>) {
    use objc2::{MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{NSTitlebarSeparatorStyle, NSToolbar, NSWindow, NSWindowToolbarStyle};
    use objc2_foundation::NSString;

    let Ok(ptr) = win.ns_window() else {
        eprintln!("[chrome] 拿不到 NSWindow，红绿灯保持 AppKit 默认（会偏 10px）");
        return;
    };
    if ptr.is_null() {
        return;
    }
    // 只能在主线程碰 NSWindow / NSToolbar。setup 里就是主线程，但别赌
    let Some(mtm) = MainThreadMarker::new() else {
        eprintln!("[chrome] 不在主线程，跳过装工具栏");
        return;
    };

    // SAFETY: Tauri 在 macOS 上给的就是这扇窗的 NSWindow，指针在窗口存活期间有效，
    // 这里只借用到本函数结束。
    let window: &NSWindow = unsafe { &*ptr.cast::<NSWindow>() };

    let identifier = NSString::from_str("supportease-unified");
    let toolbar = NSToolbar::initWithIdentifier(NSToolbar::alloc(mtm), &identifier);
    toolbar.setAllowsUserCustomization(false);
    window.setToolbar(Some(&toolbar));
    window.setToolbarStyle(NSWindowToolbarStyle::Unified);
    // AppKit 自己那条标题栏分隔线关掉：我们的标题栏底下那条线是 TopTabs 自己画的
    window.setTitlebarSeparatorStyle(NSTitlebarSeparatorStyle::None);
}

/// 把 webview 挂进玻璃层，让 AppKit 裁圆角。
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
pub fn apply_native_corners<R: Runtime>(win: &WebviewWindow<R>) {
    use objc2_app_kit::NSView;
    use window_vibrancy::{apply_liquid_glass, LiquidGlassOptions, NSGlassEffectViewStyle};

    let target = win.clone();
    let res = win.with_webview(move |webview| {
        let ptr = webview.inner();
        if ptr.is_null() {
            eprintln!("[chrome] webview 指针是空的，圆角保持直角");
            return;
        }
        // SAFETY: macOS 上 Tauri 给的是这扇窗的 WKWebView，而它是 NSView 的子类；
        // 指针在窗口存活期间有效，这里只借用到闭包结束。
        let view: &NSView = unsafe { &*ptr.cast::<NSView>() };

        let options = LiquidGlassOptions::new(NSGlassEffectViewStyle::Regular)
            .radius(MAIN_RADIUS)
            .opaque(true)
            .content_view(view);

        if let Err(err) = apply_liquid_glass(&target, options) {
            eprintln!("[chrome] 上原生玻璃失败（{err:?}）—— 圆角会是直角，其余不受影响");
        }
    });

    if let Err(err) = res {
        eprintln!("[chrome] 拿不到 webview（{err}），跳过圆角");
    }
}

/// 非 macOS：这两件事都不存在（Windows 侧的窗口键是我们自己画的，见 TopTabs）
#[cfg(not(target_os = "macos"))]
pub fn install_unified_toolbar<R: Runtime>(_win: &WebviewWindow<R>) {}

#[cfg(not(target_os = "macos"))]
pub fn apply_native_corners<R: Runtime>(_win: &WebviewWindow<R>) {}
