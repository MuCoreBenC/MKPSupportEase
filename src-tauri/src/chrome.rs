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
//! 不生效（试过）。灯的位置（2026-10-17 起）用 **Auto Layout 约束钉死**（见
//! `reposition_traffic_lights`）—— 逐颗改 `standardWindowButton(...).frame` 的老路
//! 在 resize 的每一次布局里都会先被 AppKit 摆回去、再被我们的补摆拉回来，
//! 拖窗过程中一帧错一下（作者报的「改变窗口大小的时候红绿灯在变化位置」就是这个）。
//!
//! **圆角**：这是 tauri#14165。窗口的原生边框是圆的，但 WKWebView 没有被裁 ——
//! 它把内容画进圆角外面，看上去就是直角。**现在自己裁**（2026-10-15 第六版）：
//! 给 `window.contentView` 挂 `CALayer` 圆角 + `masksToBounds`，**只裁不铺色**，
//! 而且**半径必须等于窗口自己的圆角**（见 `MAIN_RADIUS`，量出来是 13）。
//!
//! ★★★ **关键是"两个人都在裁，半径必须一样"**：窗口本身（AppKit 的 `NSThemeFrame`）
//! 也裁一个圆角，而且它那个我们改不了。两个半径不一致时，中间那圈夹缝会露出
//! `NSThemeFrame` 的底色 —— 按**系统外观**渲染，于是"档位深色 + 系统浅色"时
//! 深色界面的四角挂一圈白（作者报的「边边角角有东西」「好像有两层」就是这个）。
//!
//! ★ 走过的弯路（三版，别再回去）：
//!
//! ① `window-vibrancy` 的 liquid glass —— 那层玻璃在我们这套界面上什么也不干
//!    （webview 不透明、盖住整窗），只在圆角缝那 1~2px 露出来，按系统外观渲染成白。
//! ② "自己裁 + 自己铺色" —— 白光盖住了，但同一块色铺在两层上、两层的颜色源不同
//!    ⇒ 切主题那一瞬间露"两层"。
//! ③ `titlebarAppearsTransparent = true` —— 更糟（整窗露白），且动不了那条外弧。
//!
//! ⇒ 正解只有一条：**半径对齐 + 只裁不画**。

use tauri::{Runtime, WebviewWindow};

#[cfg(target_os = "macos")]
use objc2_app_kit::NSView;

/// 主窗口圆角。
///
/// ★★★ 2026-10-15 第六版：**这个值不是"我们喜欢的值"，是"窗口自己的值"** —— `13.0`。
///
/// 前两版走过 26 → 20，都是按"观感"挑的；但窗口本身（AppKit 的 `NSThemeFrame`）
/// 也在裁圆角，而且**它那个我们改不了**。两个半径对不上时，中间那圈夹缝就会
/// 露出 `NSThemeFrame` 的底色（按系统外观渲染 ⇒ 深色界面上一圈白）。
/// 量法：单窗截图 `screencapture -x -o -l <windowid>`，取左上角**外弧**
/// （透明→非透明 的边界）逐行首个非零像素 —— y=0 处 x=26（2x）⇒ **逻辑半径 = 13**。
///
/// ⇒ 现在取 13 = 与窗口对齐，两条弧重合，夹缝消失。
/// ★ 换这个值前请先重新量：它是**系统的**，可能随 macOS 版本变（`NSThemeFrame` 的圆角
/// 是私有实现，没有公开常量可读）。
/// ★ 改这个值要连着看两处：圆角本身（`contentView` 的 CALayer）与标题栏那条分隔的
/// 渐隐端头（`TopTabs.module.css` 里 mask 的 4%）。
#[cfg(target_os = "macos")]
const MAIN_RADIUS: f64 = 13.0;

/// 红绿灯**灯心**相对窗口左上角的距离（左 = 上 = 19）。
///
/// 作者 2026-10-15：「它是左边距和上边距留的一样高，一样的距离」+ 口径确认：
/// **以灯心为基准**（不是灯组外缘）。
///
/// ★★★ 2026-10-15 第七版：**从"基准 + 偏移"改成"绝对定位"**。
///
/// 原来写的是 `LIGHTS_NUDGE = (-8, -7)`（AppKit 默认位置再挪一点），实测下来
/// （整屏截图 + 按颜色认灯，量的是**灯心到窗口左上**）：
/// ```text
///   52 档（旧）：灯心距左 17.75 / 距上 32    ← 左 ≠ 上，作者说"不一样"
///   38 档（新）：灯心距左 19    / 距上 19    ← 正中间，左 = 上
/// ```
/// 为什么改成绝对值：① 作者要的是"左 = 上"这个**关系**，相对偏移要凑出相等，
/// 得先知道基准是什么（而基准会随 toolbar 档位变）；② 标题栏 52 → 38 之后，
/// AppKit 摆的位置也跟着变，相对量每换一档都要重算一遍。
/// 直接给绝对值反而**少一个变量**：只要标题栏高度定了，这两个数就定了。
///
/// ★ `19` 的来历：标题栏 38 高 ⇒ 竖直居中的灯心就是 `38 / 2 = 19`。
/// "左 = 上"和"在标题栏里竖直居中"在这里**是同一个数**，不用二选一。
///
/// ★★ AppKit 会在 **resize / 进出全屏 / 切外观**时重摆这些按钮 ⇒ 不能只设一次。
/// 第九版起用 **Auto Layout 约束**钉死（约束本身就是布局的一部分，AppKit 每次布局
/// 自己按约束摆，resize 过程中不再错位）；本函数在按钮被重建时重钉（见 `LIGHTS_PINS`）。
/// 调用点见 `lib.rs` 的窗口事件那一段与 `set_theme`。
#[cfg(target_os = "macos")]
const LIGHTS_CENTER: f64 = 19.0;

/// 红绿灯的**灯径**（AppKit 在统一工具栏这一档给的是 14）。
///
/// ★ 量出来的（`standardWindowButton(...).frame()` 的 `size`），不是猜的 ——
/// "灯心 → frame 原点"要减掉半径，所以它必须准。AppKit 若改了灯径，这里要跟着改
/// （症状：灯整体偏半个差值，且**三颗一起偏**）。
#[cfg(target_os = "macos")]
const LIGHTS_DIAMETER: f64 = 14.0;

/// 三颗灯之间的**灯心间距**（实测 23：红 17.75 → 黄 40.75 → 绿 63.75）。
#[cfg(target_os = "macos")]
const LIGHTS_PITCH: f64 = 23.0;

/// 三颗灯的**基准 frame**（第一次调用时量下来）—— 现在**只用来取灯径**，
/// 以及"AppKit 到底把灯摆在哪"这个事实的留档（诊断用）。
#[cfg(target_os = "macos")]
static LIGHTS_BASE: std::sync::Mutex<Option<[(f64, f64); 3]>> = std::sync::Mutex::new(None);

/// 已钉上的约束记录（第九版）。
///
/// `buttons` = 钉的时候那三颗按钮的指针，`constraints` = 我们建的三条横向约束的指针
/// （每颗一条就够作证；竖直那三条与它们同生共死）。
///
/// ★ 记指针而不是"装没装"一个布尔：**进出全屏时 AppKit 可能整棵标题栏视图树重建** ——
/// 按钮是新对象、旧约束随旧视图一起没了，这时必须重钉。
/// 指针一样（还是那三颗）且容器里还找得到我们的约束，就什么都不做。
#[cfg(target_os = "macos")]
struct LightsPins {
    buttons: [usize; 3],
    constraints: [usize; 3],
}

#[cfg(target_os = "macos")]
static LIGHTS_PINS: std::sync::Mutex<Option<LightsPins>> = std::sync::Mutex::new(None);

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

/// 把窗口裁成圆角（**只裁，不画**）。
///
/// ★★★ 2026-10-15 第六版：**半径必须与窗口自己的圆角一致**（`MAIN_RADIUS = 13`）。
/// 这一条是作者第三次追问「四个角都有…本来就自身有一个圆角，然后我们又画了一个圆角」
/// 的真正答案 —— **两个人都在裁，而且半径不一样**：
///
/// ```text
///   窗口本身（AppKit 的 NSThemeFrame）  → 圆角 13 逻辑点（量出来的）
///   我们的 contentView.layer           → 圆角 MAIN_RADIUS
/// ```
///
/// 视图树的实况（应用内 `dump_views` 打出来的，量完已删）：
/// ```text
///   NSKVONotifying_NSThemeFrame  900x640  corner=0.0  masks=true  borderColor=有值
///     └─ contentView(=WryWebViewParent)  900x640  corner=20  masks=true  bg=false
///          └─ WryWebView  corner=0  masks=false
///                └─ WKFlippedView  corner=0  masks=true
/// ```
/// `NSThemeFrame` 的 `corner=0` 但 `masks=true` —— 它的圆角**不在 layer 上**，
/// 是 AppKit 自己画的窗口形状。⇒ 半径 20 的头几年，**半径 13~20 之间那一圈窗口还是"满"的**，
/// 露出来的就是 AppKit 那层的底色（系统外观 ⇒ 深色界面上一圈白）。这就是作者说的"两层"。
///
/// ★ 为什么不是"我们画多了"：我们的圆角**比窗口的小**时同理（会露出我们的层），
/// 所以只能**对齐**，不能靠调颜色盖（颜色归 CSS，见下）。
///
/// ★★ 走过的弯路（别再回去），三条：
///
/// - 其一：借 `window-vibrancy` 的 liquid glass 裁角 —— 玻璃在深色界面上按**系统外观**渲染成白；
/// - 其二："自己裁 + 自己铺色"（`window.backgroundColor` + `contentView.layer.backgroundColor`
///   加兄弟层三处刷成界面底）—— 白光盖住了，但**同一块色铺在两层上**，外层按**窗口外观**、
///   内层按**用户档位** ⇒ 切主题那一瞬间两层一浅一深；
///
/// - 其三：试过 `titlebarAppearsTransparent = true` —— **更糟**（AppKit 不再画深色底 ⇒ 整窗露白），
///   而且它**动不了那条外弧**（量过：改前改后外弧逐行像素一模一样）。
///
/// ⇒ 现在的口径：**只裁、不铺色**（配色全归 CSS），而且**半径与窗口对齐**。
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
fn paint_corner_fill(base: &NSView, dark: bool) {
    use objc2_quartz_core::kCACornerCurveContinuous;

    let _ = dark;

    // SAFETY: window / contentView / layer 都是 AppKit 的纯读方法，窗口存活期间有效。
    let Some(window) = base.window() else {
        eprintln!("[chrome] 拿不到窗口，圆角跳过");
        return;
    };
    /* ★ 显式**不**设底色：不透明窗口的底由界面的 CSS 画满整窗，这里碰它就是
    在给"两层色不同步"留门（见上面那段注释）。 */
    let Some(content) = window.contentView() else {
        return;
    };
    content.setWantsLayer(true);
    if let Some(layer) = content.layer() {
        layer.setCornerRadius(MAIN_RADIUS);
        layer.setCornerCurve(unsafe { kCACornerCurveContinuous });
        layer.setMasksToBounds(true);
    }
}

/// 让窗口圆角化。`dark` = 用户选的那一档（**只用于对数** —— 原生侧现在一个颜色都不画）。
///
/// ★★★ 2026-10-15 第六版：**半径对齐 + 只裁不画**。两条缺一不可，前面几版各只做了一半：
///
/// - 只"只裁不画"、半径却比窗口大（20 vs 13）⇒ 夹缝露白（作者第三次报）；
/// - 只"铺色盖住"、半径仍不对 ⇒ 夹缝里的白被我们的色衬出来，看着像"两层"。
///
/// ★ 另一半的教训是"同一件事两个数据源"：第四版为了让白消失，把"界面底"这个颜色
/// 在原生侧又算了一遍（`resolved_dark` + 92%/97% 压深），并铺在两层上
/// （`window.backgroundColor` 与 `contentView.layer.backgroundColor`）——
/// CSS 一层、原生一层、AppKit 自己还按**窗口外观**刷一层，切主题时三层先后落地。
/// ⇒ 现在原生侧**不参与配色**：主窗口不透明（`transparent: false`），
/// 界面底由 CSS 画满整扇窗，这里只把 `contentView` 裁成**与窗口同半径**的圆角。
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
pub fn apply_native_corners<R: Runtime>(win: &WebviewWindow<R>, dark: bool) {
    use objc2_app_kit::NSView;

    let res = win.with_webview(move |webview| {
        let ptr = webview.inner();
        if ptr.is_null() {
            eprintln!("[chrome] webview 指针是空的，圆角保持直角");
            return;
        }
        // SAFETY: macOS 上 Tauri 给的是这扇窗的 WKWebView，而它是 NSView 的子类；
        // 指针在窗口存活期间有效，这里只借用到闭包结束。
        let view: &NSView = unsafe { &*ptr.cast::<NSView>() };
        paint_corner_fill(view, dark);
    });

    if let Err(err) = res {
        eprintln!("[chrome] 拿不到 webview（{err}），跳过圆角");
    }
}

/// **换主题之后重跑一遍圆角那一步**。
///
/// 两条路都会走到这儿：① 用户在设置页换档（`ipc::prefs::set_theme`）；
/// ② 档位是"跟随系统"时系统深浅变了（`WindowEvent::ThemeChanged`）。
///
/// ★★ 第五版起这里其实**没什么颜色要换**了 —— 配色归 CSS，`paint_corner_fill`
/// 只保证"圆角还在、还裁着"。留着它是因为**窗口还被裁**这件事是幂等的、廉价的，
/// 而且 AppKit 在切外观时会不会把 layer 的圆角弄掉没有保证；真丢了用户看到的是直角。
/// （作者报的"切主题那一瞬间两层色"正是**在它身上**发生的：原来它换的是颜色。）
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
pub fn repaint_corner_fill<R: Runtime>(win: &WebviewWindow<R>, dark: bool) {
    let res = win.with_webview(move |webview| {
        let ptr = webview.inner();
        if ptr.is_null() {
            return;
        }
        // SAFETY: 同 `apply_native_corners`（webview 的 NSView，窗口存活期间有效）。
        let view: &NSView = unsafe { &*ptr.cast::<NSView>() };
        paint_corner_fill(view, dark);
    });
    if let Err(err) = res {
        eprintln!("[chrome] 换主题后重跑圆角失败（{err}）");
    }
}

/// 把三颗灯**用 Auto Layout 约束钉到灯心距左 = 距上 = `LIGHTS_CENTER`**（第九版，幂等）。
///
/// ★ 只在装了统一工具栏之后调用有意义（那一档的灯径/间距才是量过的那组）。
/// 只打日志不中止 —— 灯偏几像素是外观问题，不该影响别的。
///
/// ★★★ **为什么是约束而不是补摆 frame**（2026-10-17，作者报「改变窗口大小的时候
/// 红绿灯在变化位置」）：AppKit 在 resize 的**每一次布局**都把灯摆回它的默认位置，
/// 而我们的 `Resized` 事件只在布局之后才补 —— 于是拖窗过程中灯先跳回去再被拉回来，
/// 一帧错一下。约束是**布局的一部分**：装上之后 AppKit 每次布局自己就按约束摆灯，
/// 没有"跳回去"那一步，resize / 进出全屏 / 切外观都不用我们再管。
/// （重钉只发生在按钮被 AppKit 重建之后，见 `LIGHTS_PINS`。）
///
/// ★★★ **坐标系**：`standardWindowButton(...).frame()` 是 **NSView 坐标（原点在左下）**，
/// 而我们要的是"距窗口上边"。这里用按钮自己的 `superview`（标题栏那一层）来翻，
/// **不用窗口高度** —— 第一版用了窗口高度，算出"灯心距上 607"这种一眼假的值
/// （按钮的 y 是相对 superview 的，不是相对整窗）。约束从**按钮**出发锚到容器的
/// `topAnchor` / `leadingAnchor`（方向语义 = a = b + c，见函数体里的注释），连换算都省了。
///
/// ★★★ **每颗灯四条约束：上 · 左 · 宽 · 高**。少了尺寸那两条，关掉 autoresizing 之后
/// 按钮的 frame 由布局引擎重算 ⇒ 没有尺寸约束就**塌成 0×0 ⇒ 红绿灯整组不见了**
/// （2026-10-17 第一版就是这个症状，灯"消失"）。
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
pub fn reposition_traffic_lights<R: Runtime>(win: &WebviewWindow<R>) {
    use objc2::{rc::Retained, MainThreadMarker};
    use objc2_app_kit::{NSButton, NSWindow, NSWindowButton};
    use objc2_foundation::NSPoint;

    let Ok(ptr) = win.ns_window() else {
        return;
    };
    if ptr.is_null() {
        return;
    }
    // 动窗口必须在主线程（`set_theme` 命令跑在 tokio 线程上）
    if MainThreadMarker::new().is_none() {
        let target = win.clone();
        if let Err(err) = win.run_on_main_thread(move || reposition_traffic_lights(&target)) {
            eprintln!("[chrome] 挪红绿灯：派回主线程失败（{err}）");
        }
        return;
    }
    // SAFETY: Tauri 在 macOS 上给的就是这扇窗的 NSWindow，指针在窗口存活期间有效。
    let window: &NSWindow = unsafe { &*ptr.cast::<NSWindow>() };
    let buttons = [
        NSWindowButton::CloseButton,
        NSWindowButton::MiniaturizeButton,
        NSWindowButton::ZoomButton,
    ];
    let lights: Vec<Retained<NSButton>> = buttons
        .iter()
        .filter_map(|b| window.standardWindowButton(*b))
        .collect();
    if lights.len() != 3 {
        eprintln!("[chrome] 只找到 {} 颗红绿灯，跳过挪位", lights.len());
        return;
    }

    let mut base = LIGHTS_BASE.lock().unwrap_or_else(|e| e.into_inner());
    if base.is_none() {
        *base = Some(std::array::from_fn(|i| {
            let f = lights[i].frame();
            (f.origin.x, f.origin.y)
        }));
    }

    /* 约束要挂在**按钮与参照视图的公共祖先**上。参照视图用 `contentView`
    （= WryWebViewParent，铺满整窗）：★★ 标题栏容器（按钮的 superview）自己的
    高度/位置**不可靠** —— 实测同一份代码下它一会 45 一会 90，顶边还会伸到窗口
    上缘之外（第三版锚它，灯整体高出 17pt、被窗口上缘裁掉半截）。
    窗口顶（contentView 的 top）才是稳定参照。 */
    let Some(content) = window.contentView() else {
        return;
    };
    // SAFETY: superview() 是纯读；视图在窗口存活期间有效。
    let Some(frame_view) = (unsafe { content.superview() }) else {
        return;
    };
    /* 已经钉过（还是那三颗按钮、我们的约束还在宿主里）就什么都不做 ——
    后面 lib.rs 那几个事件调用点只是"按钮被重建后补钉"的保险，平时都是空转。 */
    let pins_now = [
        Retained::as_ptr(&lights[0]) as usize,
        Retained::as_ptr(&lights[1]) as usize,
        Retained::as_ptr(&lights[2]) as usize,
    ];
    let present: Vec<usize> = frame_view
        .constraints()
        .iter()
        .map(|c| Retained::as_ptr(&c) as usize)
        .collect();
    let already = {
        let guard = LIGHTS_PINS.lock().unwrap_or_else(|e| e.into_inner());
        match &*guard {
            Some(p) => p.buttons == pins_now && p.constraints.iter().all(|c| present.contains(c)),
            None => false,
        }
    };
    if already {
        return;
    }

    let radius = LIGHTS_DIAMETER / 2.0;
    let mut pinned_constraints: [usize; 3] = [0; 3];
    for (i, btn) in lights.iter().enumerate() {
        /* 灯心 → frame 原点：横竖各减一个半径（灯是圆的，宽高都是 LIGHTS_DIAMETER）。
        ① 先把 frame 摆到目标位置 —— 约束要到下一次 layout 才生效，先摆一帧免得闪。 */
        let x = LIGHTS_CENTER + LIGHTS_PITCH * i as f64 - radius;
        btn.setFrameOrigin(NSPoint::new(
            x,
            content.frame().size.height - LIGHTS_CENTER - radius,
        ));
        btn.setTranslatesAutoresizingMaskIntoConstraints(false);
        /*
         * ② **四条**等号约束（必需优先级）：上边 / 左边 / 宽 / 高。
         *
         * ★★★ **尺寸那两条必须给**（第一版只给了位置 ⇒ 布局引擎把按钮
         * **塌成 0×0 ⇒ 红绿灯整组不见了**）。"位置 + 尺寸"四条一起，几何才是唯一的。
         *
         * ★★★ **方向别搞反**：`a.constraintEqualToAnchor_constant(b, c)` 的语义是
         * **a = b + c**（self 是被约束的那个）。这里从**按钮**出发钉到 contentView：
         *   btn.centerY = content.top + 19      （灯心距窗口顶 19）
         *   btn.leading = content.leading + x   （灯心距窗左 19 + 23·i，减半径）
         * 第二版把 self/anchor 写反 ⇒ x 变成 −12、整组灯被裁到窗口左缘外
         * （日志抓到的 `x=-12.00 y=64.00`；纵向碰巧没露馅，别指望那个运气）。
         */
        let top = btn
            .centerYAnchor()
            .constraintEqualToAnchor_constant(&content.topAnchor(), LIGHTS_CENTER);
        let lead = btn
            .leadingAnchor()
            .constraintEqualToAnchor_constant(&content.leadingAnchor(), x);
        let w = btn.widthAnchor().constraintEqualToConstant(LIGHTS_DIAMETER);
        let h = btn
            .heightAnchor()
            .constraintEqualToConstant(LIGHTS_DIAMETER);
        frame_view.addConstraint(&top);
        frame_view.addConstraint(&lead);
        frame_view.addConstraint(&w);
        frame_view.addConstraint(&h);
        pinned_constraints[i] = Retained::as_ptr(&lead) as usize;
    }
    /* 钉完立刻跑一次布局并留下实测几何 —— 灯"在不在、在哪"这个事实只认这一行，
    不猜（诊断用：真机上 `[chrome] 红绿灯钉完` 这一行要说 x=12 / y=h−26 / 14×14 那组数）。 */
    frame_view.layoutSubtreeIfNeeded();
    if let Some(f) = lights.first() {
        let got = f.frame();
        eprintln!(
            "[chrome] 红绿灯钉完：第一颗 x={:.2} y={:.2} {:.2}×{:.2}",
            got.origin.x, got.origin.y, got.size.width, got.size.height,
        );
    }
    *LIGHTS_PINS.lock().unwrap_or_else(|e| e.into_inner()) = Some(LightsPins {
        buttons: pins_now,
        constraints: pinned_constraints,
    });
}

/// 非 macOS：这两件事都不存在（Windows 侧的窗口键是我们自己画的，见 TopTabs）
#[cfg(not(target_os = "macos"))]
pub fn install_unified_toolbar<R: Runtime>(_win: &WebviewWindow<R>) {}

#[cfg(not(target_os = "macos"))]
pub fn reposition_traffic_lights<R: Runtime>(_win: &WebviewWindow<R>) {}

#[cfg(not(target_os = "macos"))]
pub fn apply_native_corners<R: Runtime>(_win: &WebviewWindow<R>, _dark: bool) {}

#[cfg(not(target_os = "macos"))]
pub fn repaint_corner_fill<R: Runtime>(_win: &WebviewWindow<R>, _dark: bool) {}
