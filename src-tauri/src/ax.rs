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
//!
//! # 平台
//!
//! - **macOS**：Accessibility（`mod mac`），如上；
//! - **Windows**（2026-10-19，`mod win`）：不需要任何授权 —— 主窗按 pid
//!   `EnumWindows` 找，「板子类型」缩略图是**真实子窗口**（class `Static`），
//!   `EnumChildWindows` 直接枚举到，判据是 Windows 自己实测的一组数
//!   （48×48 @ y=120/123 逻辑像素，与 macOS 的 50×50 @ y=115/117 同族不同值）；
//! - 别的平台没有这一层：照实说"这一台上没有"，不给假坐标。

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
    #[cfg(target_os = "windows")]
    {
        win::state(pid)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
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
///
/// Windows 上按**进程名**认（`bambu-studio.exe`，Toolhelp 快照，只读）；
/// 多开时取第一个"有主窗口"的那个 —— 引导卡片只能跟着一扇窗走。
pub fn slicer_pid() -> Option<i32> {
    #[cfg(target_os = "macos")]
    {
        mac::running_pid()
    }
    #[cfg(target_os = "windows")]
    {
        win::running_pid()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
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
    #[cfg(target_os = "windows")]
    {
        win::activate()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
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
    #[cfg(target_os = "windows")]
    {
        win::raise()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        /* 别处没有 AX：抬不动就照实说，卡片仍会摆出去（用户自己切一下窗口） */
        false
    }
}

/* ---------- 挑选规则（macOS 的判据；Windows 那组在 `mod win` 里另有一套） ---------- */

/// 缩略图的边长：板型与耗材色块都是 50×50 这一族。
#[cfg(target_os = "macos")]
const THUMB_SIDE: f64 = 50.0;
/// 缩略图相对窗口顶部的 y 落在这一段内（实测 115 / 117，留一点余量）。
#[cfg(target_os = "macos")]
const THUMB_Y_MIN: f64 = 100.0;
#[cfg(target_os = "macos")]
const THUMB_Y_MAX: f64 = 140.0;
/// 边长与这个数差多少还算"是 50"（坐标是浮点，别用严格相等）。
#[cfg(target_os = "macos")]
const SIDE_EPS: f64 = 1.0;

/// 从窗口的直系子元素里挑出「板子类型」那一个。
///
/// 入参是**相对窗口**的坐标（外面先把绝对坐标减掉窗口原点）—— 因为挑选用的是
/// 相对位置（稳定），而给回界面的是绝对坐标（能直接画）。
///
/// 命中多个时取 **x 较大**的那个。
/// ★ 只有 macOS 的挑选用它（Windows 的判据不同，见 `mod win` 的 `pick_bed_type_win`）。
#[cfg(target_os = "macos")]
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
    #[cfg(target_os = "windows")]
    {
        win::locate()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        GuideTargetDto {
            ready: false,
            window: None,
            target: None,
            /* 屏幕引导走的是 macOS 的 Accessibility 与 Windows 的 Win32 窗口枚举；
            别处没有这一层，照实说"这一台上没有"，而不是给一组假的坐标 */
            reason: Some("屏幕引导只在 macOS / Windows 上有（要读切片器的窗口坐标）".to_owned()),
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
        /*
         * ★ 矩形**取整**再拿去当"变没变"的键：AX 报的是浮点，同一扇窗两次读到的值
         * 可能差一丝（子点抖动），不取整就会**每一跳回读都判成"变了"** ⇒ 每 350ms
         * 重扫一次控件树 —— 那才是真能把主线程和切片器都拖慢的那种"轮询"。
         * 取整后的精度对"卡片摆哪"完全够（差半点的位置没人看得出来）。
         */
        let rect = position_of(win).zip(size_of(win)).map(|(p, s)| RectDto {
            x: p.x.round(),
            y: p.y.round(),
            width: s.width.round(),
            height: s.height.round(),
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

/* 这些测试钉的是 **macOS** 的判据（50×50 / y 落段）；Windows 那组的判据测试在
`mod win::tests` 里（48×48 / class 过滤），两边判据不同，别混。 */
#[cfg(all(test, target_os = "macos"))]
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

/*
 * ★★★ **Windows 那一半**（2026-10-19）：`#[cfg(windows)]`，与 `mod mac` 完全对称。
 *
 * 可行性是**实测出来的**（探针 `scripts/probes/uia-probe.ps1` / `childwin-probe.ps1`，
 * 2026-10-19，BBS `02.08.02.60` @ Windows 125% 缩放）：
 *
 * - 主窗口 class `wxWindowNR`（wxWidgets），`EnumWindows` 按 pid 就能找到，
 *   **不需要任何权限**（macOS 的辅助功能授权那一关在 Windows 上不存在）；
 * - 「板子类型」缩略图是**真实子窗口**（class `Static`），`EnumChildWindows` 直接枚举到，
 *   连 UIA / COM 都不用引 —— 实测两个 60×60（物理像素），相对窗口 y = 150 / 154；
 * - 折算逻辑像素（DPI 120 ÷ 96 = 1.25）：**48×48，y = 120 / 123** ——
 *   与 macOS（50×50，y = 115 / 117）同族但数值不同 ⇒ 判据**单独定常数**（见下），
 *   不复用 `pick_bed_type`（那边 50±1 会把 48 挡在门外）；
 * - 窗口里还有**别的**方形子窗口（class `wxWindowNR` 的 50×50 / 63×63），
 *   所以 Windows 判据比 macOS 多一条：**class 必须是 `Static`**。
 */
#[cfg(target_os = "windows")]
mod win {
    use super::{GuideTargetDto, RectDto, SlicerState, SlicerVisibility};
    use std::sync::Once;

    /* ---------- 判据（逻辑像素；实测 48×48 @ y=120/123，别再用 macOS 那组数） ---------- */

    /// 缩略图边长（逻辑像素）。实测 48（物理 60 @ 125%）；给 ±6 容差盖住 DPI 取整的毛边。
    const THUMB_SIDE: f64 = 48.0;
    /// 边长容差。
    const SIDE_EPS: f64 = 6.0;
    /// 相对窗口顶部的 y 落在这一段（逻辑像素）。实测 120 / 123（macOS 是 115 / 117）。
    const THUMB_Y_MIN: f64 = 100.0;
    const THUMB_Y_MAX: f64 = 145.0;
    /// 正方形的判定毛边（逻辑像素）。
    const SQUARE_EPS: f64 = 4.0;
    /// 缩略图的子窗口 class。**必须过滤**：窗口里还有 `wxWindowNR` 的方形子窗口
    /// （50×50 @ y=510、63×63 @ y=374），不过滤就会指错。
    const THUMB_CLASS: &str = "Static";

    /// 切片器进程名（小写比较）。实测 `bambu-studio.exe`；Beta 与旧版命名一并列上。
    const BBS_IMAGES: [&str; 3] = [
        "bambu-studio.exe",
        "bambu-studio-beta.exe",
        "bambustudio.exe",
    ];

    /* ---------- Win32 FFI（手工声明，与 mac 侧手工声明 CF 同一风格） ---------- */

    type Hwnd = isize;

    #[repr(C)]
    #[derive(Default, Clone, Copy, Debug)]
    struct RectPx {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    impl RectPx {
        fn width(&self) -> i32 {
            self.right - self.left
        }
        fn height(&self) -> i32 {
            self.bottom - self.top
        }
    }

    #[repr(C)]
    struct ProcessEntry {
        dw_size: u32,
        cnt_usage: u32,
        th32_process_id: u32,
        th32_default_heap_id: usize,
        th32_module_id: u32,
        cnt_threads: u32,
        th32_parent_process_id: u32,
        pc_pri_class_base: i32,
        dw_flags: u32,
        sz_exe_file: [u16; 260],
    }

    /* 全零对这块 POD 是合法初值（Toolhelp 要求调用方先把 dw_size 填成 sizeof）；
    `[u16; 260]` 没有 Default，只能自己写。 */
    impl Default for ProcessEntry {
        fn default() -> Self {
            unsafe { std::mem::zeroed() }
        }
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> isize;
        fn Process32FirstW(snapshot: isize, entry: *mut ProcessEntry) -> i32;
        fn Process32NextW(snapshot: isize, entry: *mut ProcessEntry) -> i32;
        fn CloseHandle(handle: isize) -> i32;
    }

    #[link(name = "user32")]
    extern "system" {
        fn EnumWindows(cb: Option<extern "system" fn(Hwnd, isize) -> i32>, lparam: isize) -> i32;
        fn EnumChildWindows(
            parent: Hwnd,
            cb: Option<extern "system" fn(Hwnd, isize) -> i32>,
            lparam: isize,
        ) -> i32;
        fn GetWindowThreadProcessId(hwnd: Hwnd, out_pid: *mut u32) -> u32;
        fn IsWindowVisible(hwnd: Hwnd) -> i32;
        fn IsIconic(hwnd: Hwnd) -> i32;
        fn GetWindowRect(hwnd: Hwnd, out: *mut RectPx) -> i32;
        fn GetClassNameW(hwnd: Hwnd, buf: *mut u16, max: i32) -> i32;
        fn GetWindowTextLengthW(hwnd: Hwnd) -> i32;
        fn GetDpiForWindow(hwnd: Hwnd) -> u32;
        fn SetForegroundWindow(hwnd: Hwnd) -> i32;
        fn ShowWindow(hwnd: Hwnd, cmd: i32) -> i32;
        fn SetProcessDPIAware() -> i32;
    }

    /// `CreateToolhelp32Snapshot` 的 `TH32CS_SNAPPROCESS`。
    const SNAP_PROCESS: u32 = 0x2;
    /// `ShowWindow` 的 `SW_RESTORE`。
    const SW_RESTORE: i32 = 9;

    /*
     * ★★ **本进程必须 DPI 感知**，`GetWindowRect` 才给物理像素：
     * - 给用户的 exe：tauri 的清单里已经声明了 per-monitor 感知，这一下是 no-op；
     * - cargo test / 探针那种裸进程：默认不感知，坐标会被虚拟化 ⇒ 判据全歪。
     * 进程级设置，`Once` 保证只设一次；已设过时系统返回 0，无害。
     */
    fn ensure_dpi_aware() {
        static ONCE: Once = Once::new();
        ONCE.call_once(|| unsafe {
            SetProcessDPIAware();
        });
    }

    /* ---------- 基础读取 ---------- */

    /// 窗口属于哪个进程。
    fn pid_of(hwnd: Hwnd) -> u32 {
        let mut pid: u32 = 0;
        unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
        pid
    }

    /// 窗口矩形（物理像素）。读不到给 `None`。
    fn rect_of(hwnd: Hwnd) -> Option<RectPx> {
        let mut r = RectPx::default();
        let ok = unsafe { GetWindowRect(hwnd, &mut r) };
        (ok != 0).then_some(r)
    }

    /// 窗口有没有标题（主窗口的判据之一：工具窗 / 隐藏窗大多没标题）。
    fn has_title(hwnd: Hwnd) -> bool {
        unsafe { GetWindowTextLengthW(hwnd) > 0 }
    }

    fn class_of(hwnd: Hwnd) -> Option<String> {
        let mut buf = [0u16; 256];
        let n = unsafe { GetClassNameW(hwnd, buf.as_mut_ptr(), 256) };
        (n > 0).then(|| String::from_utf16_lossy(&buf[..n as usize]))
    }

    /// 这扇窗的 DPI（缩放 = dpi / 96）。读不到按 96 处理（当 1.0 用，别除零）。
    fn scale_of(hwnd: Hwnd) -> f64 {
        let dpi = unsafe { GetDpiForWindow(hwnd) };
        if dpi == 0 {
            1.0
        } else {
            f64::from(dpi) / 96.0
        }
    }

    /// 物理像素矩形 → 逻辑像素矩形（除以窗口的缩放）。
    fn to_logical(r: RectPx, scale: f64) -> RectDto {
        RectDto {
            x: f64::from(r.left) / scale,
            y: f64::from(r.top) / scale,
            width: f64::from(r.width()) / scale,
            height: f64::from(r.height()) / scale,
        }
    }

    /// `EnumWindows` 的过场：pid 的每一扇顶层窗（有标题、rect 读得到的）。
    struct MainWinCtx {
        want: u32,
        found: Vec<(Hwnd, bool, f64)>,
    }

    /// 一个 pid 的**主窗口**。
    ///
    /// ★★ **不能"z 序第一个有标题的就当主窗"**（实机验证踩到的坑，2026-10-19）：
    /// BBS 同一个 pid 下还有**别的**有标题的顶层窗（实测一扇 20×20 的小窗排在最前），
    /// 按层序先碰到的就是它 ⇒ 定位全部落空。判据三层收紧：
    ///
    /// 1. **可见**（`IsWindowVisible`）；
    /// 2. **有标题**；
    /// 3. 在里面挑**面积最大**的、且优先 class `wxWindowNR`（BBS 主框是 wxWidgets
    ///    的窗；面积兜住"多开时另一份 BBS 的最小化窗"之类的杂项）。
    ///
    /// ★ 多开时每个进程各有一扇主框；**最小化的也算**（还原后就是它）——
    /// 卡片这时本来就该收起来，但"窗口在哪"还是要答得上来。
    fn main_window_of(pid: u32) -> Option<Hwnd> {
        ensure_dpi_aware();
        let mut ctx = MainWinCtx {
            want: pid,
            found: Vec::new(),
        };
        unsafe {
            EnumWindows(
                Some(enum_main_window_cb),
                std::ptr::addr_of_mut!(ctx) as isize,
            );
        }
        /* 优先 wxWindowNR（BBS 主框），里面再取面积最大的；没有才退到"面积最大的有标题窗" */
        let best = |wx_only: bool| {
            ctx.found
                .iter()
                .filter(|(_, is_wx, _)| !wx_only || *is_wx)
                .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(h, _, _)| *h)
        };
        best(true).or_else(|| best(false))
    }

    extern "system" fn enum_main_window_cb(hwnd: Hwnd, lparam: isize) -> i32 {
        let Some(ctx) = (unsafe { (lparam as *mut MainWinCtx).as_mut() }) else {
            return 0;
        };
        if pid_of(hwnd) != ctx.want || !has_title(hwnd) || unsafe { IsWindowVisible(hwnd) } == 0 {
            return 1;
        }
        let Some(r) = rect_of(hwnd) else {
            return 1;
        };
        let is_wx = class_of(hwnd).is_some_and(|c| c.eq_ignore_ascii_case("wxWindowNR"));
        let area = f64::from(r.width()) * f64::from(r.height());
        ctx.found.push((hwnd, is_wx, area));
        1
    }

    /* ---------- 进程 ---------- */

    /// 在跑的切片器进程（按进程名认，见 [`BBS_IMAGES`]）。
    ///
    /// ★ **必须挑"有主窗口"的那个**：BBS 会起好几个同名进程（实测三份），
    /// 其中只有带工程窗口的才是用户看着的那个；没有窗口的挑了也定位不了。
    pub(super) fn running_pid() -> Option<i32> {
        ensure_dpi_aware();
        let snapshot = unsafe { CreateToolhelp32Snapshot(SNAP_PROCESS, 0) };
        if snapshot == -1 {
            return None;
        }
        let mut entry = ProcessEntry {
            dw_size: std::mem::size_of::<ProcessEntry>() as u32,
            ..Default::default()
        };
        let mut found: Option<i32> = None;
        unsafe {
            if Process32FirstW(snapshot, &mut entry) != 0 {
                loop {
                    let len = entry
                        .sz_exe_file
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(entry.sz_exe_file.len());
                    let name = String::from_utf16_lossy(&entry.sz_exe_file[..len]);
                    if BBS_IMAGES.iter().any(|b| name.eq_ignore_ascii_case(b))
                        && main_window_of(entry.th32_process_id).is_some()
                    {
                        found = Some(entry.th32_process_id as i32);
                        break;
                    }
                    if Process32NextW(snapshot, &mut entry) == 0 {
                        break;
                    }
                }
            }
            CloseHandle(snapshot);
        }
        found
    }

    /* ---------- 状态（卡片该不该露脸） ---------- */

    pub(super) fn state(pid: i32) -> SlicerState {
        ensure_dpi_aware();
        let Some(hwnd) = main_window_of(pid as u32) else {
            return SlicerState {
                rect: None,
                visibility: SlicerVisibility::Gone,
            };
        };
        let rect = rect_of(hwnd).map(|r| to_logical(r, scale_of(hwnd)));
        let visible = unsafe { IsWindowVisible(hwnd) != 0 && IsIconic(hwnd) == 0 };
        /*
         * ★ "被压住了"与 macOS 同一条口径：问 [`crate::zorder`]（Windows 那半是
         * `EnumWindows` 的 z 序，全在本地，比 macOS 的 CG 还轻）。问不到就 fail-open
         * 当作露脸 —— 这是一张"指路"的卡，判断不了别让它凭空消失。
         */
        let visibility = if !visible {
            SlicerVisibility::Behind
        } else {
            match crate::zorder::slicer_window(pid) {
                Some(z) if z.covered >= crate::zorder::COVERED_ENOUGH => SlicerVisibility::Behind,
                Some(z) if !z.on_screen => SlicerVisibility::Behind,
                _ => SlicerVisibility::Front,
            }
        };
        SlicerState { rect, visibility }
    }

    /* ---------- 定位 ---------- */

    /// 一条子窗口候选：class 是不是 `Static`、相对窗口的矩形（**逻辑像素**）、绝对逻辑 x。
    struct Candidate {
        is_static: bool,
        rel: RectDto,
        abs_x: f64,
    }

    /// 挑「板子类型」：class `Static` + 正方形 + 边长 48 一族 + y 落段，命中取 x 较大。
    ///
    /// ★ 与 macOS 的 [`super::pick_bed_type`] 同一个思路（**遍历找特征，不记偏移**），
    /// 但判据是 Windows 自己的（48 不是 50；多一条 class 过滤）。
    fn pick_bed_type_win(candidates: &[Candidate]) -> Option<&Candidate> {
        candidates
            .iter()
            .filter(|c| {
                c.is_static
                    && (c.rel.width - c.rel.height).abs() <= SQUARE_EPS
                    && (c.rel.width - THUMB_SIDE).abs() <= SIDE_EPS
                    && c.rel.y >= THUMB_Y_MIN
                    && c.rel.y <= THUMB_Y_MAX
            })
            .max_by(|a, b| {
                a.abs_x
                    .partial_cmp(&b.abs_x)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// `EnumChildWindows` 的过场：候选收集器 + 窗口矩形（物理）+ 缩放。
    struct ChildCtx {
        candidates: Vec<Candidate>,
        win_px: RectPx,
        scale: f64,
    }

    pub(super) fn locate() -> GuideTargetDto {
        ensure_dpi_aware();
        let Some(pid) = running_pid() else {
            return refused("没看到切片器在跑 —— 先打开 Bambu Studio，再点「查看引导」");
        };
        let Some(hwnd) = main_window_of(pid as u32) else {
            return refused("切片器没有打开的工程窗口");
        };
        let Some(win_px) = rect_of(hwnd) else {
            return refused("读不到窗口位置");
        };
        let scale = scale_of(hwnd);
        let window = to_logical(win_px, scale);

        let mut ctx = ChildCtx {
            candidates: Vec::new(),
            win_px,
            scale,
        };
        unsafe {
            EnumChildWindows(
                hwnd,
                Some(enum_child_cb),
                std::ptr::addr_of_mut!(ctx) as isize,
            );
        }

        match pick_bed_type_win(&ctx.candidates) {
            None => refused(
                "在窗口里没找到板子类型的位置（切片器界面换过了？）—— 这一版的引导暂时指不了",
            ),
            Some(c) => GuideTargetDto {
                ready: true,
                window: Some(window),
                target: Some(RectDto {
                    x: c.abs_x,
                    y: window.y + c.rel.y,
                    width: c.rel.width,
                    height: c.rel.height,
                }),
                reason: None,
            },
        }
    }

    extern "system" fn enum_child_cb(hwnd: Hwnd, lparam: isize) -> i32 {
        let Some(ctx) = (unsafe { (lparam as *mut ChildCtx).as_mut() }) else {
            return 0;
        };
        let Some(child_px) = rect_of(hwnd) else {
            return 1;
        };
        let is_static = class_of(hwnd).is_some_and(|c| c.eq_ignore_ascii_case(THUMB_CLASS));
        ctx.candidates.push(Candidate {
            is_static,
            abs_x: f64::from(child_px.left) / ctx.scale,
            rel: RectDto {
                x: f64::from(child_px.left - ctx.win_px.left) / ctx.scale,
                y: f64::from(child_px.top - ctx.win_px.top) / ctx.scale,
                width: f64::from(child_px.width()) / ctx.scale,
                height: f64::from(child_px.height()) / ctx.scale,
            },
        });
        1
    }

    /// 指不了时的那句人话（与 mac 侧同一个形状：不编坐标）。
    fn refused(reason: &str) -> GuideTargetDto {
        GuideTargetDto {
            ready: false,
            window: None,
            target: None,
            reason: Some(reason.to_owned()),
        }
    }

    /* ---------- 抬窗（本模块唯一的"写"，写的只是层序） ---------- */

    pub(super) fn raise() -> bool {
        let Some(hwnd) = running_pid().and_then(|pid| main_window_of(pid as u32)) else {
            return false;
        };
        unsafe {
            /* 最小化着就先还原 —— 还原那一下通常顺带把窗抬起来 */
            if IsIconic(hwnd) != 0 {
                ShowWindow(hwnd, SW_RESTORE);
            }
            SetForegroundWindow(hwnd) != 0
        }
    }

    /// Windows 没有"窗 / 整个 App"那层区分（macOS 的 `AXRaise` vs `AXFrontmost`），
    /// 与 [`raise`] 同一件事 —— 保留两个名字是为了让 `ipc::guide` 的两处调用各取所需。
    pub(super) fn activate() -> bool {
        raise()
    }

    /* ---------- 测试：判据落在探针实测的那组数上 ---------- */

    #[cfg(test)]
    mod tests {
        use super::*;

        fn cand(is_static: bool, x: f64, y: f64, side: f64) -> Candidate {
            Candidate {
                is_static,
                rel: RectDto {
                    x,
                    y,
                    width: side,
                    height: side,
                },
                abs_x: x,
            }
        }

        /// 实测那一对（125% 缩放折算：48×48 @ y=120/123）⇒ 取 x 较大的那个
        #[test]
        fn it_picks_the_right_hand_thumb() {
            let cands = [
                cand(true, 98.4, 120.0, 48.0),
                cand(true, 268.8, 123.2, 48.0),
            ];
            let got = pick_bed_type_win(&cands).map(|c| c.abs_x);
            assert_eq!(got, Some(268.8));
        }

        /// class 不是 `Static` 的不算 —— 窗口里真有 `wxWindowNR` 的方形子窗口
        /// （50×50 @ y=510、63×63 @ y=374，见 childwin-probe 的读数）
        #[test]
        fn non_static_squares_are_not_candidates() {
            let cands = [cand(false, 228.0, 408.0, 50.0)];
            assert!(pick_bed_type_win(&cands).is_none());
        }

        /// y 不在落段的也不算
        #[test]
        fn thumbs_outside_the_band_are_not_candidates() {
            let cands = [cand(true, 98.4, 400.0, 48.0)];
            assert!(pick_bed_type_win(&cands).is_none());
        }

        /// 物理像素 → 逻辑像素的换算要与 125% 实测对上（60 phys → 48）
        #[test]
        fn physical_to_logical_matches_the_probe() {
            let scale = 1.25;
            assert!((THUMB_SIDE - f64::from(60) / scale).abs() <= SIDE_EPS);
        }

        /// `THUMB_CLASS` 的大小写比较要吃得住系统报回来的任意大小写
        #[test]
        fn class_comparison_is_case_insensitive() {
            assert!("STATIC".eq_ignore_ascii_case(THUMB_CLASS));
            assert!("static".eq_ignore_ascii_case(THUMB_CLASS));
        }

        /// **实机验证**（`#[ignore]`，本机正跑着 Bambu Studio 时用
        /// `cargo test -- --ignored` 跑）：定位要指得上，且命中的是
        /// 「边长 48 一族 + y 落段」的那个缩略图。判据一变这里先红。
        #[test]
        #[ignore = "要本机在跑 Bambu Studio（实机验证，CI 上没有切片器）"]
        fn live_locate_hits_the_bed_type_thumb() {
            let got = locate();
            assert!(got.ready, "实机定位失败：{:?}", got.reason);
            let (w, t) = (got.window.expect("窗口矩形"), got.target.expect("目标"));
            assert!(
                (t.width - THUMB_SIDE).abs() <= SIDE_EPS,
                "命中宽度 {} 不是 {} 一族（判据歪了）",
                t.width,
                THUMB_SIDE
            );
            assert!(
                t.y >= THUMB_Y_MIN && t.y <= THUMB_Y_MAX,
                "命中 y={} 不在落段 [{}, {}]",
                t.y,
                THUMB_Y_MIN,
                THUMB_Y_MAX
            );
            assert!(
                t.x > w.x && t.y > w.y,
                "目标 ({},{}) 应落在窗口 ({},{}) 之内",
                t.x,
                t.y,
                w.x,
                w.y
            );
        }

        /// **实机验证（状态）**：状态要答得上来 —— 窗口矩形在、可见性不是 Gone
        /// （三个 BBS 实例叠着时可能是 Behind，那是真实处境，不算失败）。
        #[test]
        #[ignore = "要本机在跑 Bambu Studio（实机验证，CI 上没有切片器）"]
        fn live_state_answers_rect_and_visibility() {
            let Some(pid) = running_pid() else {
                panic!("没找到在跑的 Bambu Studio");
            };
            let st = state(pid);
            assert!(st.rect.is_some(), "窗口矩形读不到");
            assert_ne!(
                st.visibility,
                SlicerVisibility::Gone,
                "进程在跑、主窗找到了，可见性不该是 Gone"
            );
        }
    }
}
