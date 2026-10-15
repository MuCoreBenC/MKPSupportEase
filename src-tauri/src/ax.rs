//! **屏幕引导的取数口**：找到切片器（Bambu Studio）的窗口，在窗口里找到
//! 「板子类型」那个控件，把它的**屏幕坐标**给回界面。
//!
//! # 它只回答"在哪"，不做任何操作
//!
//! **只读**（没有任何 AX 写动作、不模拟点击、不碰别的进程的内存）—— 引导层
//! 拿到的只是一组坐标，箭头怎么画、什么时候推进，都是界面那一侧的事。
//! 这与"改 BBS 内存"那条路是两回事：那条路本机被 entitlement 封死（见
//! `docs/INVESTIGATION-BBS-INVALIDATE.md` §6），这条路只用公开 API。
//!
//! # 为什么是"遍历找特征"，不是"记住一个偏移"
//!
//! 切片器窗口里 **50×50 的缩略图**有两个（一个是耗材色块，一个是板型），
//! 实测（同一进程、两种窗口尺寸）：
//!
//! ```text
//!   窗口 1440×786 → 相对窗口 (61,115) / (200,117)
//!   窗口 1000×652 → 相对窗口 (32,115) / (142,117)
//! ```
//!
//! 看出两件事：
//! - **相对 y 稳定**（115 / 117）—— 可以当特征；
//! - **相对 x 会变**（左栏宽度跟着窗口走）—— 所以 x **必须实时读**，
//!   "窗口原点 + 固定偏移"在窗口一变就指错（差 30~60 点）。
//!
//! ⇒ 判据定成：`尺寸 50×50` 且 `相对窗口 y 落在 100~140`，命中里取 x 较大的那个。
//! （取 x 较大 = 板型那一个；这是 2026-10-15 一次实测的读数，换了界面要重测。）
//!
//! # 「窗口动了 / 被压住了 / 最小化了」—— 靠 AX 推送，不轮询
//!
//! 作者 2026-10-15 问的三件事（拖动就跟随、被别的软件压住就收起来、最小化就消失）
//! 全是**同一个信号源**：macOS 的 **AX 通知**。`AXObserverCreate` 注册
//! `kAXWindowMovedNotification` / `kAXWindowResizedNotification` /
//! `kAXWindowMiniaturizedNotification` / `kAXWindowDeminiaturizedNotification` /
//! `kAXFocusedWindowChangedNotification`，挂到主线上的 run loop —— 切片器自己
//! **主动 push** 过来，我们**一个定时器都不用起**（这正是作者说的"不要频繁的轮询"）。
//!
//! ★ 通知的载荷里没有坐标，所以收到之后仍然调一次 [`locate_bed_type`] ——
//! 那是"被通知了才读"，不是"隔一会儿读一次"，与轮询是两回事。
//!
//! # 权限
//!
//! macOS 上读别的进程的控件要**辅助功能授权**。没授权时这里**照实报**
//! （`ready: false` + 一句人话），**不编坐标** —— 指错位置比不指更糟。

use serde::Serialize;

/// 屏幕坐标下的一个矩形（左上角 + 宽高，单位是**点**）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct RectDto {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// 「板子类型」控件在哪。形状与 `ipc::PostProcessCommandDto` 同一套规矩：
/// `ready` 决定能不能指，指不了给一句人话（`reason`），**不编坐标**。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuideTargetDto {
    /// 两个条件都成立才为真：拿到窗口、且窗口里找到了那个控件
    pub ready: bool,
    /// 切片器窗口的屏幕矩形（引导层拿它判"窗口动没动"）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window: Option<RectDto>,
    /// 「板子类型」控件的屏幕矩形（箭头指它）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<RectDto>,
    /// 指不了时的一句人话（为什么 + 下一步做什么）；能指时没有
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// 切片器窗口**这一刻的状态** —— 决定引导卡片该不该露脸。
///
/// 判据只有一条：**它是不是当前最前的那扇窗**。压在别人后面 / 最小化 / 被隐藏
/// ⇒ 用户这会儿看不到那个板型控件，卡片留着就是"指着空气"。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SlicerVisibility {
    /// 在最前，卡片该露脸
    Front,
    /// 还在跑，但被压在后面 / 最小化了 —— 卡片该收起来
    Behind,
    /// 已经不在跑了（切成器退了）—— 卡片该收起来
    Gone,
}

/// 切片器**此刻的状态**（[`slicer_state`] 的返回值）。
///
/// ★ 两个字段一起读、一起比：`ipc::guide::sync` 拿它当"变没变"的键 ——
/// **一次比较就把三类变化都罩住**（拖动了 / 被压住了 / 最小化了）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlicerState {
    /// 切片器窗口的屏幕矩形（读不到就是 `None`）
    pub rect: Option<RectDto>,
    pub visibility: SlicerVisibility,
}

/// 切片器**此刻的两件事**：窗口在哪 → 卡片该摆哪；它在不在最前 → 卡片该不该露脸。
///
/// ★★★ 为什么把这两件事**合成一次读**（2026-10-15 作者："拖动了它没有跟随"）：
///
/// 拖动跟随**不能只靠 AX 通知** —— 切片器是 Qt 应用，实测拖动过程中**不发**
/// `AXWindowMoved`（而最小化 / 还原 / 换焦点那几条是发的）。所以位置这一条要有个
/// **很轻的定时回读**兜底（见 `ax_watch` 的 `TICK`），而那必须便宜：
/// **一个来回**把"窗口矩形 + 最前是谁 + 最小化了没有"都读出来，一次也不去重扫
/// 切片器的控件树（重扫是几十次 AX 调用，那才是作者说的"不要频繁的轮询"）。
pub fn slicer_state(pid: i32) -> SlicerState {
    #[cfg(target_os = "macos")]
    {
        mac::state(pid)
    }
    #[cfg(not(target_os = "macos"))]
    {
        /* 别处没有这一层：照实说"看不见"，让卡片收起来，而不是假装它在最前 */
        let _ = pid;
        SlicerState {
            rect: None,
            visibility: SlicerVisibility::Gone,
        }
    }
}

/// 切片器进程号（没在跑就是 `None`）。装 AX 通知要用它认进程。
pub fn slicer_pid() -> Option<i32> {
    #[cfg(target_os = "macos")]
    {
        mac::running_pid()
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

/// 在切片器**最前那扇窗**上做一件事（`ax_watch` 用它挂移动 / 最小化通知）。
///
/// ★★ **为什么是"回调"而不是"返回句柄"** —— 这是 2026-10-15 那次"点了就闪退"
/// 的教训：AX 元素是**引用计数**的，读出来的东西归 `Owned` 管（一 Drop 就
/// `CFRelease`）。把裸指针从 `Owned` 里抠出来单独返回 = 递出去一个**悬垂指针**，
/// 下一次 AX 调用就崩。所以句柄**不许离开这个回调的作用域**。
///
/// 窗口是**后来才有的**（切片器可能还没开工程）⇒ 取不到返回 `None`，
/// 调用方下次再来（见 `ax_watch::is_complete` 那条补挂的路）。
///
/// 非 macOS 恒 `None`（没有 AX 这一层）。
pub fn with_top_window<R>(pid: i32, f: impl FnOnce(*mut std::ffi::c_void) -> R) -> Option<R> {
    #[cfg(target_os = "macos")]
    {
        mac::with_top_window(pid, f)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (pid, f);
        None
    }
}

/*
 * ★★ 这里**曾经**有个 `top_window_id()`（返回窗口元素的地址，用来判"换了一扇窗没有"）。
 * 删掉的理由：AXUIElement 是**每次读都新给一个对象**，同一个窗口两次读到的指针
 * **不保证相等** ⇒ 那个判断会把"同一扇窗"误判成"换了窗"，于是每来一条通知 / 每一跳
 * 回读都重挂一次监听（churn），而重挂期间正好收不到通知。
 *
 * 现在不需要它了：位置跟随由 `ax_watch` 的细定时回读兜底，最小化 / 被压住由
 * `ax::slicer_state` 每次现读 —— 都不依赖"当初挂在哪个元素上"。
 */

/// 把切片器**整个应用**激活（`AXFrontmost = true`），成功返回 `true`。
///
/// ★★ 为什么除了"抬窗"还要"激活整个应用"（2026-10-15 作者报"我的软件会覆盖，
/// 完全挡住了"）：
///
/// 引导卡片是一扇**新窗**，建它会把**我们这个 App 激活** —— 于是我们的主窗压到
/// 切片器前面。`AXRaise` 只动**那一扇窗**在它自己 App 内部的层序，**不切换当前
/// 最前的 App** ⇒ 光靠它拉不回切片器。`AXFrontmost` 才是"把那个 App 切到最前"，
/// 与用户在 Dock 上点它一下等效。
///
/// ★ 它只改**前台归属**：不动切片器的任何参数、不移动不缩放、不模拟点击。
/// 与我们自己的窗口关系是"谁在最前" —— 这正是要的。
pub fn activate_slicer() -> bool {
    #[cfg(target_os = "macos")]
    {
        mac::activate()
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

/// 把切片器**那一扇窗**抬到最前，成功返回 `true`。
///
/// ★★★ **这是本模块唯一的"写"动作**，而它写的是——**窗口的层序**，不是任何参数。
/// 边界（`ax.rs` 模块头那条"只读"）在这里要写清楚：`kAXRaiseAction` 只改
/// "哪扇窗在前面"，**不改切片器的任何设置**、不移动不缩放、不模拟点击。
///
/// # 为什么非用它不可（2026-10-15 作者报"有时候会把 MKP 置顶了"）
///
/// 原来用的是 `open -a BambuStudio` —— 那是"**激活整个应用**"，macOS 会把那个应用
/// **所有**窗口一起带到前面。于是我们自己的 `main` / 工作台窗口也跟着冒到切片器
/// 上面，用户点了「查看引导」看到的反而是我们这一屏。
///
/// `kAXRaiseAction` 只抬**这一扇窗**，别的应用、别的窗口一律不动。
pub fn raise_slicer() -> bool {
    #[cfg(target_os = "macos")]
    {
        mac::raise()
    }
    #[cfg(not(target_os = "macos"))]
    {
        /* 别处没有 AX：抬不动就照实说，卡片仍会摆出去（用户自己切一下窗口） */
        false
    }
}

/* ---------- 挑选规则（纯函数，两侧共用，判据落在测试里） ---------- */

/// 缩略图的边长：板型与耗材色块都是 50×50 这一族。
const THUMB_SIDE: f64 = 50.0;
/// 缩略图相对窗口顶部的 y 落在这一段内（实测 115 / 117，留一点余量）。
const THUMB_Y_MIN: f64 = 100.0;
const THUMB_Y_MAX: f64 = 140.0;
/// 边长与这个数差多少还算"是 50"（坐标是浮点，别用严格相等）。
const SIDE_EPS: f64 = 1.0;

/// 从窗口的直系子元素里挑出「板子类型」那一个。
///
/// 入参是**相对窗口**的坐标（外面先把绝对坐标减掉窗口原点）—— 因为挑选用的是
/// 相对位置（稳定），而给回界面的是绝对坐标（能直接画）。
///
/// 命中多个时取 **x 较大**的那个。
fn pick_bed_type(candidates: &[(RectDto, f64)]) -> Option<f64> {
    let mut best: Option<f64> = None;
    for (r, abs_x) in candidates {
        let square =
            (r.width - THUMB_SIDE).abs() <= SIDE_EPS && (r.height - THUMB_SIDE).abs() <= SIDE_EPS;
        if !square || r.y < THUMB_Y_MIN || r.y > THUMB_Y_MAX {
            continue;
        }
        match best {
            None => best = Some(*abs_x),
            Some(x) => best = Some(x.max(*abs_x)),
        }
    }
    best
}

/// 定位一次。**只读**。
pub fn locate_bed_type() -> GuideTargetDto {
    #[cfg(target_os = "macos")]
    {
        mac::locate()
    }
    #[cfg(not(target_os = "macos"))]
    {
        GuideTargetDto {
            ready: false,
            window: None,
            target: None,
            /* 屏幕引导走的是 macOS 的 Accessibility；别处没有这一层，
            照实说"这一台上没有"，而不是给一组假的坐标 */
            reason: Some("屏幕引导只在 macOS 上有（要读切片器的窗口坐标）".to_owned()),
        }
    }
}

#[cfg(target_os = "macos")]
extern "C" {
    /* 只声明释放，不声明别的：别的都在 `mod mac` 里紧挨着用处放 */
    fn CFRelease(cf: *const std::ffi::c_void);
}

/// `CFRelease` 的**空安全**版本 —— 这一层的**每一次释放都走它**。
///
/// ★★★ `CFRelease(NULL)` 在 macOS 上是**硬崩**
/// （崩溃报告那句 `*** CFRelease() called with NULL ***`），不是返回错误、也不是 no-op。
/// 而 CoreFoundation / Accessibility 在这一层到处是"失败就给空指针"，
/// 2026-10-15 那次闪退就是这么来的（当时是 `then_some` 把空指针包进了 `Owned`）。
/// 放在模块根是为了让 `mod mac` 内外都能走**同一个**出口 —— 出口一多就会漏。
#[cfg(target_os = "macos")]
pub(crate) fn cf_release(cf: *const std::ffi::c_void) {
    if !cf.is_null() {
        unsafe { CFRelease(cf) }
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use super::{pick_bed_type, GuideTargetDto, RectDto, SlicerState, SlicerVisibility};
    use core_foundation::base::TCFType as _;
    use core_foundation::string::CFString;
    use std::ffi::c_void;

    type AXUIElementRef = *mut c_void;
    type CFStringRef = *const c_void;
    type CFTypeRef = *const c_void;
    type CFArrayRef = *const c_void;
    type AXError = i32;

    /// `AXValueGetValue` 的 `type` 参数：点 / 尺寸（C 枚举的值，见 AXValue.h）
    const K_AX_VALUE_CG_POINT_TYPE: u32 = 1;
    const K_AX_VALUE_CG_SIZE_TYPE: u32 = 2;

    #[repr(C)]
    #[derive(Copy, Clone)]
    struct CGPoint {
        x: f64,
        y: f64,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    struct CGSize {
        width: f64,
        height: f64,
    }

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
        fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
        fn AXUIElementCopyAttributeValue(
            element: AXUIElementRef,
            attribute: CFStringRef,
            out: *mut CFTypeRef,
        ) -> AXError;
        fn AXValueGetValue(value: CFTypeRef, ty: u32, out: *mut c_void) -> bool;
        /// 系统**当前最前**的那个应用（用来判"切片器是不是压在最上面"）
        fn AXUIElementCreateSystemWide() -> AXUIElementRef;
        /// 对元素执行一个动作（我们只用 `kAXRaiseAction`：抬窗，不动参数）
        fn AXUIElementPerformAction(element: AXUIElementRef, action: CFStringRef) -> AXError;
        /// 写属性（我们只用 `AXFrontmost`：把那个 App 切到最前，不动参数）
        fn AXUIElementSetAttributeValue(
            element: AXUIElementRef,
            attribute: CFStringRef,
            value: CFTypeRef,
        ) -> AXError;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFArrayGetCount(array: CFArrayRef) -> isize;
        fn CFArrayGetValueAtIndex(array: CFArrayRef, idx: isize) -> *const c_void;
        /// `AXWindows[i]` 那份是**借来的**（+0），要交出去就先 retain（见 [`top_window`]）
        fn CFRetain(cf: CFTypeRef) -> CFTypeRef;
        fn CFNumberGetValue(number: CFNumberRef, ty: i64, out: *mut c_void) -> bool;
        fn CFBooleanGetValue(b: CFBooleanRef) -> u8;
    }

    type CFNumberRef = CFTypeRef;
    type CFBooleanRef = CFTypeRef;

    /// `CFNumberType` 里的 `kCFNumberSInt32Type`（枚举值，见 CFNumber.h）
    const K_CF_NUMBER_SINT32_TYPE: i64 = 3;

    /* `signal.h` 里的探活调用（`sig = 0` 只做存在性检查，不真的发信号） */
    extern "C" {
        fn kill(pid: i32, sig: i32) -> i32;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        /// `AXFrontmost` 要的那个布尔真值（`kCFBooleanTrue` 是导出的数据符号）
        static kCFBooleanTrue: CFTypeRef;
    }

    /// 把切片器**整个 App** 切到最前（`AXFrontmost`，见 [`super::activate_slicer`]）。
    pub(super) fn activate() -> bool {
        if !unsafe { AXIsProcessTrusted() } {
            return false;
        }
        let Some(pid) = running_pid() else {
            return false;
        };
        unsafe {
            let app = AXUIElementCreateApplication(pid);
            let key = CFString::new("AXFrontmost");
            let ok = AXUIElementSetAttributeValue(
                app,
                key.as_CFTypeRef() as CFStringRef,
                kCFBooleanTrue,
            ) == 0;
            drop(key);
            crate::ax::cf_release(app as CFTypeRef);
            ok
        }
    }

    /// 一个 AX 属性值的**所有权**：用完即释放（这一层读得很勤，别把 CF 对象攒着）。
    struct Owned(CFTypeRef);

    impl Drop for Owned {
        fn drop(&mut self) {
            /*
             * ★★★ **必须挡住 NULL**：`CFRelease(NULL)` 在 macOS 上是**硬崩**
             * （日志里那句 `*** CFRelease() called with NULL ***`），不是返回错误。
             * 见 [`attr`] 里那条注释 —— 这个包装一多，谁都可能不小心包住一个空指针。
             */
            crate::ax::cf_release(self.0)
        }
    }

    /// 属性名直接用字符串（"AXWindows" 这一族本来就是字符串常量）——
    /// 省掉一整排 framework 里的 `extern static` 声明。
    ///
    /// ★★★ **读失败必须给 `None`，绝不能包一个空指针出来**（2026-10-15 闪退那一刀）：
    ///
    /// 原来写的是 `(err == 0 && !out.is_null()).then_some(Owned(out))` ——
    /// **`then_some` 是急切求值**（参数先算、再看条件），所以读失败时照样会构造出
    /// `Owned(null)`，而它当场就被 Drop ⇒ `CFRelease(NULL)` ⇒ **进程被抬走**。
    /// 这条路在"切片器那一刻没有焦点窗"（`AXFocusedWindow` 读不到）时必踩。
    ///
    /// 改成 `if/else`（惰性）之后，`Owned` 只在拿到真东西时才出生。
    fn attr(element: AXUIElementRef, name: &str) -> Option<Owned> {
        unsafe {
            let key = CFString::new(name);
            let raw = key.as_CFTypeRef();
            let mut out: CFTypeRef = std::ptr::null();
            /* key 在这一句里用完就释放；out 归 Owned */
            let err = AXUIElementCopyAttributeValue(element, raw as CFStringRef, &mut out);
            drop(key);
            if err == 0 && !out.is_null() {
                Some(Owned(out))
            } else {
                None
            }
        }
    }

    fn as_point(v: &Owned) -> Option<CGPoint> {
        unsafe {
            let mut p = CGPoint { x: 0.0, y: 0.0 };
            AXValueGetValue(
                v.0,
                K_AX_VALUE_CG_POINT_TYPE,
                &mut p as *mut _ as *mut c_void,
            )
            .then_some(p)
        }
    }

    fn as_size(v: &Owned) -> Option<CGSize> {
        unsafe {
            let mut s = CGSize {
                width: 0.0,
                height: 0.0,
            };
            AXValueGetValue(
                v.0,
                K_AX_VALUE_CG_SIZE_TYPE,
                &mut s as *mut _ as *mut c_void,
            )
            .then_some(s)
        }
    }

    fn refused(reason: impl Into<String>) -> GuideTargetDto {
        GuideTargetDto {
            ready: false,
            window: None,
            target: None,
            reason: Some(reason.into()),
        }
    }

    /// 切片器的进程名（正式版优先，其次 Beta —— 与 `ipc::bbs` 里那一份同口径）。
    const BBS_PROCESSES: [&str; 2] = ["BambuStudio", "BambuStudioBeta"];

    pub(super) fn running_pid() -> Option<i32> {
        for name in BBS_PROCESSES {
            let out = std::process::Command::new("pgrep")
                .arg("-x")
                .arg(name)
                .output()
                .ok()?;
            let pid = String::from_utf8_lossy(&out.stdout)
                .lines()
                .next()
                .and_then(|line| line.trim().parse::<i32>().ok());
            if pid.is_some() {
                return pid;
            }
        }
        None
    }

    /// 找出切片器**最前的那扇窗**（`AXFocusedWindow`，取不到退回 `AXWindows[0]`）。
    ///
    /// ★★★ 返回的是**自己持有一份**的 `Owned`（+1），**不是借来的裸指针** ——
    /// 这是 2026-10-15 那次"点了就闪退"的教训：
    ///
    /// - `Owned` 一 Drop 就 `CFRelease`；
    /// - 把裸指针从 `Owned` 里抠出来单独用，等于递出去一个**悬垂指针**，
    ///   下一次 AX 调用就崩（而那正好发生在 AX 通知的回调里 ⇒ 点一下就闪退）。
    ///
    /// 所以这里统一**返回带所有权的 `Owned`**，谁用谁拿着，用完自然释放。
    /// `AXWindows[i]` 那份是**借来的**（+0）⇒ 先 `CFRetain` 再交出去。
    fn top_window(app: AXUIElementRef) -> Option<Owned> {
        if let Some(w) = attr(app, "AXFocusedWindow") {
            return Some(w);
        }
        let windows = attr(app, "AXWindows")?;
        unsafe {
            if CFArrayGetCount(windows.0 as CFArrayRef) < 1 {
                return None;
            }
            let borrowed = CFArrayGetValueAtIndex(windows.0 as CFArrayRef, 0) as AXUIElementRef;
            if borrowed.is_null() {
                return None;
            }
            CFRetain(borrowed);
            Some(Owned(borrowed as CFTypeRef))
        }
    }

    /// 在切片器那扇窗上做一件事 —— `f` 拿到的是**只在这个调用里有效**的句柄。
    ///
    /// ★★ 为什么是"回调"而不是"返回句柄"：见 [`top_window`] 的注释 —— 句柄**不许
    /// 活着离开这个作用域**（它底下那个 CF 对象归 `win` 管，`win` 一 Drop 就没了）。
    pub(super) fn with_top_window<R>(pid: i32, f: impl FnOnce(AXUIElementRef) -> R) -> Option<R> {
        unsafe {
            let app = AXUIElementCreateApplication(pid);
            let win = top_window(app);
            /* `win` 自己 +1，所以这里释放 app 不影响它 */
            let out = win.as_ref().map(|w| f(w.0 as AXUIElementRef));
            crate::ax::cf_release(app as CFTypeRef);
            out
        }
    }

    /// 抬窗：`kAXRaiseAction`。**只动层序**（见 [`super::raise_slicer`] 的注释）。
    pub(super) fn raise() -> bool {
        if !unsafe { AXIsProcessTrusted() } {
            return false;
        }
        let Some(pid) = running_pid() else {
            return false;
        };
        with_top_window(pid, |win| unsafe {
            /* `AXUIElementPerformAction` 的 action 名就是一个 CFString */
            let action = CFString::new("AXRaise");
            let ok = AXUIElementPerformAction(win, action.as_CFTypeRef() as CFStringRef) == 0;
            drop(action);
            ok
        })
        .unwrap_or(false)
    }

    /// 切片器窗口**现在是不是最前的那一扇**。
    ///
    /// # 判据怎么定（三条，从紧到松）
    ///
    /// 1. `AXUIElementCreateSystemWide` + `AXFocusedApplication` —— **最前的那个应用**
    ///    不是切片器 ⇒ 它被压在后面了 ⇒ [`SlicerVisibility::Behind`]；
    /// 2. 切片器自己那一扇最前的窗（`AXFocusedWindow`，取不到就退回 `AXWindows[0]`）
    ///    的 `AXMinimized` 为真 ⇒ [`SlicerVisibility::Behind`]；
    /// 3. 都对得上 ⇒ [`SlicerVisibility::Front`]。
    ///
    /// ★ 第 1 条是"被别的软件压住"的**唯一**判据 —— 窗口的 `AXPosition`/`AXSize`
    /// 在压住前后**一模一样**，光看几何判不出来。切片器是自绘控件的 Qt 应用，
    /// `AXWindows` 的顺序**不能**当作 z-order 用（实测过：拖动一下顺序就变）。
    ///
    /// ★★ 还有一条**不能漏**的自查：我们自己的引导卡片也是
    /// `always_on_top` 的一扇窗，而它一露脸就抢焦点吗？——**不会**：建窗时
    /// 没给它 `focus()`，它也不接受键盘焦点。所以"最前的应用"仍然是切片器。
    /// 这一条要是哪天变了（比如给卡片加了可输入的控件），这里的判据会跟着失效。
    /// 一次读齐：窗口矩形 + 最前是谁 + 最小化了没有（见 [`super::slicer_state`]）。
    pub(super) fn state(pid: i32) -> SlicerState {
        let none = SlicerState {
            rect: None,
            visibility: SlicerVisibility::Behind,
        };
        if !unsafe { AXIsProcessTrusted() } {
            /* 没授权读不了"谁在最前"，宁可不露脸（指错比不指更糟） */
            return none;
        }
        /*
         * 进程没了直接报 `Gone`：调用方（`ipc::guide::sync`）靠它把 watcher 卸掉。
         * 不探这一下的话，切片器退掉会被当成"被压住了"，而 watcher 就一直挂着一个
         * **失效的 pid** —— 切片器重启之后，监听挂在旧进程上，再也不会响。
         *
         * ★ 用 `kill(pid, 0)`（只探活、不发信号）而不是 `pgrep`：这条路现在由**定时**
         * 回读也走，起子进程是不可接受的（也正是作者说的"不要频繁的轮询"）。
         */
        if !process_alive(pid) {
            return SlicerState {
                rect: None,
                visibility: SlicerVisibility::Gone,
            };
        }

        unsafe {
            /* ① 最前的那个应用是不是切片器 */
            let sys = AXUIElementCreateSystemWide();
            let front_pid = attr(sys, "AXFocusedApplication")
                .and_then(|app| attr(app.0 as AXUIElementRef, "AXPID"))
                .and_then(|v| as_pid(&v));
            crate::ax::cf_release(sys as CFTypeRef);
            let front = !front_pid.is_some_and(|p| p != pid);

            /* ② 那一扇窗：矩形 + 最小化了没有（**借用**这一对 `Owned` 活到读完为止） */
            let app = AXUIElementCreateApplication(pid);
            let focused = attr(app, "AXFocusedWindow");
            let windows = attr(app, "AXWindows");
            let out = match (&focused, &windows) {
                (Some(w), _) => window_state(w.0 as AXUIElementRef, front),
                (None, Some(list)) => {
                    if CFArrayGetCount(list.0 as CFArrayRef) < 1 {
                        /* 连一扇窗都没有 = 切片器刚起来/正在退，当作看不见 */
                        SlicerState {
                            rect: None,
                            visibility: SlicerVisibility::Gone,
                        }
                    } else {
                        let borrowed =
                            CFArrayGetValueAtIndex(list.0 as CFArrayRef, 0) as AXUIElementRef;
                        window_state(borrowed, front)
                    }
                }
                /* 两个属性都读不到：宁可不露脸（指错比不指更糟） */
                (None, None) => none,
            };
            drop(focused);
            drop(windows);
            crate::ax::cf_release(app as CFTypeRef);
            out
        }
    }

    /// 读一个元素的 `AXPosition`。
    fn position_of(el: AXUIElementRef) -> Option<CGPoint> {
        attr(el, "AXPosition").and_then(|v| as_point(&v))
    }

    /// 读一个元素的 `AXSize`。
    fn size_of(el: AXUIElementRef) -> Option<CGSize> {
        attr(el, "AXSize").and_then(|v| as_size(&v))
    }

    /// 一扇窗的状态：矩形 + （最小化没？）＋（最前的是不是它那个 App？）
    ///
    /// `win` 是**借来的**句柄：调用方负责让它的 `Owned` 活过这次调用。
    ///
    /// ★ 矩形**压住也要读**：压住 -> 又切回最前时，判定"变没变"要靠它一起比。
    fn window_state(win: AXUIElementRef, front: bool) -> SlicerState {
        let minimized = attr(win, "AXMinimized")
            .and_then(|v| as_bool(&v))
            .unwrap_or(false);
        let rect = position_of(win).zip(size_of(win)).map(|(p, s)| RectDto {
            x: p.x,
            y: p.y,
            width: s.width,
            height: s.height,
        });
        let visibility = if !front || minimized {
            SlicerVisibility::Behind
        } else {
            SlicerVisibility::Front
        };
        SlicerState { rect, visibility }
    }

    /// `kill(pid, 0)` —— **只探活，不发信号**（libSystem 里就有，不用额外依赖）。
    fn process_alive(pid: i32) -> bool {
        unsafe { kill(pid, 0) == 0 }
    }

    /// `AXPID` 是个 CFNumber，取成 `i32`。
    fn as_pid(v: &Owned) -> Option<i32> {
        unsafe {
            let mut out: i32 = 0;
            let ok = CFNumberGetValue(
                v.0 as CFNumberRef,
                K_CF_NUMBER_SINT32_TYPE,
                &mut out as *mut _ as *mut c_void,
            );
            ok.then_some(out)
        }
    }

    /// `AXMinimized` 是个 CFBoolean。
    fn as_bool(v: &Owned) -> Option<bool> {
        unsafe { Some(CFBooleanGetValue(v.0 as CFBooleanRef) != 0) }
    }

    pub(super) fn locate() -> GuideTargetDto {
        if !unsafe { AXIsProcessTrusted() } {
            return refused(
                "还没有辅助功能权限：打开「系统设置 → 隐私与安全性 → 辅助功能」，把 SupportEase 勾上",
            );
        }

        let Some(pid) = running_pid() else {
            return refused("没看到切片器在跑 —— 先打开 Bambu Studio，再点「查看引导」");
        };

        unsafe {
            let app = AXUIElementCreateApplication(pid);
            let Some(windows) = attr(app, "AXWindows") else {
                return refused("读不到切片器的窗口（它可能正在启动，稍等一下再试）");
            };
            if CFArrayGetCount(windows.0 as CFArrayRef) < 1 {
                return refused("切片器没有打开的工程窗口");
            }
            let win = CFArrayGetValueAtIndex(windows.0 as CFArrayRef, 0) as AXUIElementRef;

            let Some(pos) = attr(win, "AXPosition").and_then(|v| as_point(&v)) else {
                return refused("读不到窗口位置");
            };
            let Some(size) = attr(win, "AXSize").and_then(|v| as_size(&v)) else {
                return refused("读不到窗口大小");
            };
            let window = RectDto {
                x: pos.x,
                y: pos.y,
                width: size.width,
                height: size.height,
            };

            let Some(children) = attr(win, "AXChildren") else {
                return refused("读不到窗口里的控件");
            };
            let count = CFArrayGetCount(children.0 as CFArrayRef);

            let mut candidates: Vec<(RectDto, f64)> = Vec::new();
            for i in 0..count {
                let el = CFArrayGetValueAtIndex(children.0 as CFArrayRef, i) as AXUIElementRef;
                let (Some(p), Some(s)) = (
                    attr(el, "AXPosition").and_then(|v| as_point(&v)),
                    attr(el, "AXSize").and_then(|v| as_size(&v)),
                ) else {
                    continue;
                };
                candidates.push((
                    /* 挑选用相对坐标（稳定），绝对 x 一起带着（最后给界面） */
                    RectDto {
                        x: p.x - window.x,
                        y: p.y - window.y,
                        width: s.width,
                        height: s.height,
                    },
                    p.x,
                ));
            }

            match pick_bed_type(&candidates) {
                None => refused(
                    "在窗口里没找到板子类型的位置（切片器界面换过了？）—— 这一版的引导暂时指不了",
                ),
                Some(abs_x) => {
                    let rel = candidates
                        .iter()
                        .find(|(_, x)| (*x - abs_x).abs() < 0.5)
                        .map(|(r, _)| *r);
                    let Some(rel) = rel else {
                        return refused("挑出来的控件对不上（不该发生）");
                    };
                    GuideTargetDto {
                        ready: true,
                        window: Some(window),
                        target: Some(RectDto {
                            x: abs_x,
                            y: window.y + rel.y,
                            width: rel.width,
                            height: rel.height,
                        }),
                        reason: None,
                    }
                }
            }
        }
    }

    /*
     * ★ 放在 `mod mac` 的**最末尾**：clippy 的 `items_after_test_module`
     * 要求测试模块后面不能再有别的 item。
     */
    #[cfg(test)]
    mod owned_tests {
        use super::Owned;

        /// ★★★ **`Owned` 包着空指针也必须能安全地走完一生**。
        ///
        /// 2026-10-15 那次闪退就是这个：`attr` 里用 `then_some`（**急切求值**）把空指针
        /// 包进了 `Owned`，Drop 时 `CFRelease(NULL)` ⇒ **整个进程被抬走**。
        ///
        /// 这条判据的失败方式很特别：**没挡住的话不是 assert 失败，而是这个测试进程
        /// 直接崩** —— 正好也是"崩"这件事本身的严重性。**别删。**
        #[test]
        fn dropping_an_owned_null_is_harmless() {
            /* 真包一个空指针：`Owned` 的 Drop 必须自己挡住它 */
            drop(Owned(std::ptr::null()));
            /* 顺带确认那个空安全出口也是无害的 */
            crate::ax::cf_release(std::ptr::null());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f64, y: f64, side: f64) -> RectDto {
        RectDto {
            x,
            y,
            width: side,
            height: side,
        }
    }

    /// 实测那一对（相对窗口 (61,115) 与 (200,117)）⇒ 取 x 较大的那个
    #[test]
    fn it_picks_the_right_hand_thumb() {
        let got = pick_bed_type(&[
            (rect(61.0, 115.0, 50.0), 61.0),
            (rect(200.0, 117.0, 50.0), 200.0),
        ]);
        assert_eq!(got, Some(200.0));
    }

    /// 窗口变小后相对 x 会变（(32,115) 与 (142,117)）—— 判据仍然成立
    #[test]
    fn it_still_picks_it_when_the_panel_is_narrower() {
        let got = pick_bed_type(&[
            (rect(32.0, 115.0, 50.0), 32.0),
            (rect(142.0, 117.0, 50.0), 142.0),
        ]);
        assert_eq!(got, Some(142.0));
    }

    /// 不是 50×50 的不算（旁边那些 18×18 的小按钮）
    #[test]
    fn buttons_of_other_sizes_are_not_candidates() {
        let got = pick_bed_type(&[(rect(10.0, 120.0, 18.0), 10.0)]);
        assert_eq!(got, None);
    }

    /// y 不在那一段的也不算（窗口下方那一批控件）
    #[test]
    fn thumbs_outside_the_band_are_not_candidates() {
        let got = pick_bed_type(&[(rect(50.0, 400.0, 50.0), 50.0)]);
        assert_eq!(got, None);
    }

    #[test]
    fn nothing_found_is_none_not_a_guess() {
        assert_eq!(pick_bed_type(&[]), None);
    }
}
