//! **切片器那扇窗，被谁盖住了** —— 用 CoreGraphics 的窗口列表问系统要答案。
//!
//! # 为什么需要它（2026-10-15 作者）
//!
//! 作者原话：「其他软件盖住了 BBS，它还是这个还是一样的显示，怎么办？有没有可能
//! 我们知道 BBS 的**层级**在哪，就把这一个贴着它的层级……这是系统的事情吗」。
//!
//! ★ **是系统的事情，而且系统给了只读的答案**：`CGWindowListCopyWindowInfo` 返回
//! **按层序从前往后排好的**窗口列表，每项带 **归属 pid / 层级 / 矩形**。
//! 只读这三样**不需要任何授权** —— 要读窗口的**名字/内容**才需要"屏幕录制"，
//! 我们一件都不读。
//!
//! # 为什么不用 AX 的"最前是谁"（`AXFocusedApplication`）
//!
//! - 那是**焦点**，不是**层级** —— 一个窗口压上来、焦点却没变时，它一点反应都没有
//!   （作者遇到的正是这个：盖住了，卡片还挂着）；
//! - 它要**打给别的进程**（同步 IPC，切片器一忙就卡）；
//! - 而这里**全在本地**（窗口服务器），快得多，也顺便把"最小化 / 隐藏"一起答了
//!   （`OnScreenOnly` 的列表里根本就没有被最小化的窗）。
//!
//! # 判据
//!
//! 在列表里找切片器那扇**普通窗**（`ownerPID == 切片器` 且 `layer == 0`），
//! 再看它**前面**那些窗口里有没有「别的 App + 普通层级 + 与它重叠」的 ⇒ 被盖住。
//!
//! ★ **只算 `layer == 0`**：我们自己的引导卡片是浮层（`always_on_top` ⇒ layer 3），
//! 天然被排除；别的 App 的浮动面板 / 提示条也不该算"盖住"。
//! ★ 重叠面积要**占切片器窗口的两成以上**才算（`COVERED_ENOUGH`）：一个小浮窗压到
//! 角上不该让卡片闪。
//!
//! # 兜底
//!
//! 拿不到窗口列表（系统不给 / 以后收紧了这个接口）时返回 `None`，调用方
//! （`ipc::guide::decide`）会**退回 AX 那套口径**，而不是瞎猜。
//!
//! ★ **非 macOS 也走这条兜底**：CoreGraphics 只有 macOS 有，其它平台 `slicer_window`
//! 恒 `None` —— 于是那边一切照旧按 AX 口径走（本就没有"层序"这一说）。

/// 被别的 App 盖住多少才算"看不见了"。
///
/// ★ 为什么不是"压到一点就算"：一个小浮窗、一条通知压到角上，那块板型控件照样看得见
/// —— 那种时候把卡片收掉反而莫名其妙。两成是个保守的界。
/// ★ 全平台同一个数：macOS 在 `ipc::guide::decide` 里比，Windows 在 `ax::win::state`
/// 里比 —— 单一真值，改这里就够。
pub const COVERED_ENOUGH: f64 = 0.2;

/// 切片器窗口在系统层序里的处境。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlicerWindow {
    /// 它在屏上吗（`false` = 最小化 / 隐藏 / 已经不在跑了）
    pub on_screen: bool,
    /// 被**别的 App 的普通窗**盖住的面积占比（0.0~1.0）
    pub covered: f64,
    /// 它那扇窗的屏幕矩形（**取整**后）。
    ///
    /// ★ 位置**顺手就带出来了**（同一次窗口列表里就有），于是"拖动就跟随"拿它当
    /// "变没变"的键时，**一个 AX 调用都不用发** —— 那才是真省（也是省在刀刃上：
    /// AX 是同步 IPC，切片器一忙会卡好几秒）。
    pub rect: Option<crate::ax::RectDto>,
}

/// 切片器窗口的层级处境。`None` = 问不到（调用方退回 AX 口径）。
pub fn slicer_window(bbs_pid: i32) -> Option<SlicerWindow> {
    #[cfg(target_os = "macos")]
    {
        mac::slicer_window(bbs_pid)
    }
    #[cfg(target_os = "windows")]
    {
        win::slicer_window(bbs_pid)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        // ★ CoreGraphics 的窗口列表只有 macOS 有 —— 别处恒 `None`，
        // 调用方（`ipc::guide::decide`）会退回 AX 那条口径。
        let _ = bbs_pid;
        None
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use std::ffi::c_void;

    type CFArrayRef = *const c_void;
    type CFDictionaryRef = *const c_void;
    type CFStringRef = *const c_void;
    type CFTypeRef = *const c_void;

    #[repr(C)]
    #[derive(Clone, Copy, Default, Debug, PartialEq)]
    struct CGPoint {
        x: f64,
        y: f64,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default, Debug, PartialEq)]
    struct CGSize {
        width: f64,
        height: f64,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default, Debug, PartialEq)]
    struct CGRect {
        origin: CGPoint,
        size: CGSize,
    }

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        /// 窗口列表（**从前往后**）。这就是"层级"的来源。
        fn CGWindowListCopyWindowInfo(option: u32, relative_to: u32) -> CFArrayRef;
        /// 把 `kCGWindowBounds` 那个字典直接变成 `CGRect`（省掉自己拆字典）
        fn CGRectMakeWithDictionaryRepresentation(dict: CFDictionaryRef, out: *mut CGRect) -> bool;
        /// 字典里的三个键（CoreGraphics 导出的**数据符号** —— 用它们比猜字符串稳）
        static kCGWindowOwnerPID: CFStringRef;
        static kCGWindowLayer: CFStringRef;
        static kCGWindowBounds: CFStringRef;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFArrayGetCount(array: CFArrayRef) -> isize;
        fn CFArrayGetValueAtIndex(array: CFArrayRef, idx: isize) -> *const c_void;
        fn CFDictionaryGetValue(dict: CFDictionaryRef, key: CFTypeRef) -> CFTypeRef;
        fn CFNumberGetValue(number: CFTypeRef, ty: i64, out: *mut c_void) -> bool;
        /* 释放一律走 [`crate::ax::cf_release`]（空安全）—— 这里不另开一个出口 */
    }

    /// `kCGWindowListOptionOnScreenOnly`（只列屏上的 ⇒ 最小化的自然不在里面）
    const ON_SCREEN_ONLY: u32 = 1;
    /// `kCGNullWindowID`
    const NULL_WINDOW: u32 = 0;
    /// `kCFNumberSInt32Type` / `kCFNumberSInt64Type`（见 CFNumber.h）
    const SINT32: i64 = 3;
    const SINT64: i64 = 4;

    /// 普通窗口层（`NS_NORMAL_WINDOW_LEVEL`）。我们自己的卡片是浮层，不是这一层。
    const NORMAL_LAYER: i64 = 0;

    pub(super) fn slicer_window(bbs_pid: i32) -> Option<super::SlicerWindow> {
        unsafe {
            let list = CGWindowListCopyWindowInfo(ON_SCREEN_ONLY, NULL_WINDOW);
            if list.is_null() {
                /* 问不到（系统不给）—— 交给调用方退回 AX 口径 */
                return None;
            }
            let count = CFArrayGetCount(list);
            if count <= 0 {
                crate::ax::cf_release(list);
                return None;
            }

            /* ① 切片器那扇普通窗：它的矩形 + 它在列表里的位置 */
            let mut bbs: Option<(isize, CGRect)> = None;
            for i in 0..count {
                let dict = CFArrayGetValueAtIndex(list, i) as CFDictionaryRef;
                if pid_of(dict) != Some(bbs_pid) || layer_of(dict) != Some(NORMAL_LAYER) {
                    continue;
                }
                if let Some(r) = bounds_of(dict) {
                    bbs = Some((i, r));
                    break;
                }
            }
            let Some((bbs_index, bbs_rect)) = bbs else {
                /* 屏上没有它的普通窗 = 最小化 / 隐藏 / 退出了 */
                crate::ax::cf_release(list);
                return Some(super::SlicerWindow {
                    on_screen: false,
                    covered: 0.0,
                    rect: None,
                });
            };
            /* ★ 取整：和"变没变"的键对齐（浮点零头抖动会让每一跳都判"变了"） */
            let rect = crate::ax::RectDto {
                x: bbs_rect.origin.x.round(),
                y: bbs_rect.origin.y.round(),
                width: bbs_rect.size.width.round(),
                height: bbs_rect.size.height.round(),
            };

            /* ② 看它**前面**那些：别的 App 的普通窗，与它重叠了多少 */
            let area = bbs_rect.size.width * bbs_rect.size.height;
            let mut covered = 0.0_f64;
            if area > 0.0 {
                for i in 0..bbs_index {
                    let dict = CFArrayGetValueAtIndex(list, i) as CFDictionaryRef;
                    if layer_of(dict) != Some(NORMAL_LAYER) {
                        /* 浮层（含我们自己的卡片）不算"盖住" */
                        continue;
                    }
                    if pid_of(dict) == Some(bbs_pid) {
                        /* 切片器自己别的窗 */
                        continue;
                    }
                    let Some(r) = bounds_of(dict) else { continue };
                    covered += overlap_area(r, bbs_rect) / area;
                }
            }
            crate::ax::cf_release(list);

            /*
             * ★ **量化成 1/20 步长**再交出去：这个值会被拿去当"变没变"的键，
             * 浮点的零头抖动会让每一跳回读都判成"变了" ⇒ 每 700ms 重扫一次控件树。
             * 而判据只需要"有没有过两成"这个量级的分辨率。
             */
            let covered = (covered.min(1.0) * 20.0).floor() / 20.0;
            Some(super::SlicerWindow {
                on_screen: true,
                covered,
                rect: Some(rect),
            })
        }
    }

    /// 这项窗口的归属进程。
    fn pid_of(dict: CFDictionaryRef) -> Option<i32> {
        let v = unsafe { CFDictionaryGetValue(dict, kCGWindowOwnerPID) };
        let mut out: i32 = 0;
        let ok = unsafe { CFNumberGetValue(v, SINT32, &mut out as *mut _ as *mut c_void) };
        ok.then_some(out)
    }

    /// 这项窗口的层级（0 = 普通窗）。
    fn layer_of(dict: CFDictionaryRef) -> Option<i64> {
        let v = unsafe { CFDictionaryGetValue(dict, kCGWindowLayer) };
        let mut out: i64 = 0;
        let ok = unsafe { CFNumberGetValue(v, SINT64, &mut out as *mut _ as *mut c_void) };
        ok.then_some(out)
    }

    /// 这项窗口的矩形。
    fn bounds_of(dict: CFDictionaryRef) -> Option<CGRect> {
        let v = unsafe { CFDictionaryGetValue(dict, kCGWindowBounds) };
        if v.is_null() {
            return None;
        }
        let mut r = CGRect::default();
        let ok = unsafe { CGRectMakeWithDictionaryRepresentation(v as CFDictionaryRef, &mut r) };
        ok.then_some(r)
    }

    /// 两个矩形的重叠面积。
    fn overlap_area(a: CGRect, b: CGRect) -> f64 {
        let w =
            (a.origin.x + a.size.width).min(b.origin.x + b.size.width) - a.origin.x.max(b.origin.x);
        let h = (a.origin.y + a.size.height).min(b.origin.y + b.size.height)
            - a.origin.y.max(b.origin.y);
        if w <= 0.0 || h <= 0.0 {
            0.0
        } else {
            w * h
        }
    }
}

/*
 * ★★★ **Windows 那一半**（2026-10-19）：`#[cfg(windows)]`。
 *
 * CoreGraphics 的窗口列表在 Windows 上的对应物就是 **`EnumWindows` 的 z 序**
 * （文档明确：按"top of the Z order 到 bottom"的顺序返回）。判据与 mac 侧逐条对应：
 *
 * | macOS | Windows |
 * | --- | --- |
 * | `ownerPID == 切片器` 且 `layer == 0` | `pid == 切片器` 且**有标题**（工具窗没有） |
 * | `layer != 0` 的浮层不算"盖住" | `WS_EX_TOPMOST` 的不算（我们自己的卡片是 `always_on_top`，天然被排除） |
 * | 屏上没有 = 最小化 / 隐藏 | `IsWindowVisible == 0` 或 `IsIconic != 0` |
 * | 重叠 ≥ `COVERED_ENOUGH`（两成） | 同一个数，那边算完这边直接比 |
 *
 * ★ **同进程的窗口要跳过**（与 mac 侧"切片器自己别的窗"同一条）：多开时另一份
 * BBS 是**另一个 pid**，照算"盖住" —— 那是真实处境：它的窗真的压在上面。
 */
#[cfg(target_os = "windows")]
mod win {
    type Hwnd = isize;

    #[repr(C)]
    #[derive(Default, Clone, Copy)]
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
        fn area(&self) -> f64 {
            f64::from(self.width()) * f64::from(self.height())
        }
    }

    #[repr(C)]
    struct Entry {
        hwnd: Hwnd,
        pid: u32,
        rect: RectPx,
        visible: bool,
        iconic: bool,
        topmost: bool,
        cloaked: bool,
        has_title: bool,
    }

    #[link(name = "user32")]
    extern "system" {
        fn EnumWindows(cb: Option<extern "system" fn(Hwnd, isize) -> i32>, lparam: isize) -> i32;
        fn GetWindowThreadProcessId(hwnd: Hwnd, out_pid: *mut u32) -> u32;
        fn IsWindowVisible(hwnd: Hwnd) -> i32;
        fn IsIconic(hwnd: Hwnd) -> i32;
        fn GetWindowRect(hwnd: Hwnd, out: *mut RectPx) -> i32;
        fn GetWindowTextLengthW(hwnd: Hwnd) -> i32;
        fn GetWindowLongW(hwnd: Hwnd, index: i32) -> i32;
        fn GetDpiForWindow(hwnd: Hwnd) -> u32;
        fn SetProcessDPIAware() -> i32;
    }

    #[link(name = "dwmapi")]
    extern "system" {
        fn DwmGetWindowAttribute(hwnd: Hwnd, attr: u32, out: *mut i32, size: u32) -> i32;
    }

    /// `DwmGetWindowAttribute` 的 `DWMWA_CLOAKED`。
    const DWMWA_CLOAKED: u32 = 14;

    /*
     * ★★ **DWM 的"隐形窗"必须排除**（2026-10-20）：
     * `IsWindowVisible` 对下面这两类窗**都说是可见的**，而它们用户根本看不见：
     *
     * - **另一个虚拟桌面**上的窗（Windows 的虚拟桌面就是把窗"cloak"掉）；
     * - UWP / 系统组件那些留着句柄但内容不显示的窗（实测就有一扇全屏的这类窗压在
     *   最前面，于是"切片器被盖住 100%" —— 卡片因此一摆出去就被收掉）。
     *
     * ⇒ 拿 `DWMWA_CLOAKED` 问一次；被 cloak 的窗不算"盖住"。
     */
    fn is_cloaked(hwnd: Hwnd) -> bool {
        let mut v: i32 = 0;
        let ok = unsafe { DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &mut v, 4) };
        ok == 0 && v != 0
    }

    /// `GetWindowLongW` 的 `GWL_EXSTYLE` 与其中 `WS_EX_TOPMOST` 那一位。
    const GWL_EXSTYLE: i32 = -20;
    const WS_EX_TOPMOST: i32 = 0x8;

    pub(super) fn slicer_window(bbs_pid: i32) -> Option<super::SlicerWindow> {
        /*
         * ★ 本进程必须 DPI 感知，`GetWindowRect` 才给物理像素（与 `ax::win` 同一个坑：
         * 给用户的 exe 靠 tauri 的清单，cargo test 那种裸进程要自己设）。
         */
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| unsafe {
            SetProcessDPIAware();
        });

        /* ① 一次 `EnumWindows` 拿全表（z 序：最前在前） */
        let mut entries: Vec<Entry> = Vec::new();
        unsafe {
            EnumWindows(Some(collect_cb), std::ptr::addr_of_mut!(entries) as isize);
        }

        /*
         * ② 切片器那扇主窗 —— ★★ **判据问 `ax` 要，这里不自作主张**（2026-10-20）：
         * BBS 一个进程有好几扇**有标题**的顶层窗，排在最前的常常是隐藏的那扇；
         * 这里要是自己"取第一个 pid 对得上的"，就会挑到一扇看不见的窗 ⇒ 判成
         * "不在屏上" ⇒ 卡片刚摆出去就被收掉（作者报的"几秒钟就消失"）。
         */
        let main_hwnd = crate::ax::main_window_of(bbs_pid as u32);
        let bbs_index = main_hwnd.and_then(|h| entries.iter().position(|e| e.hwnd == h));
        let Some(idx) = bbs_index else {
            /* 一个都找不到 = 最小化 / 隐藏 / 已经退了 */
            return Some(super::SlicerWindow {
                on_screen: false,
                covered: 0.0,
                rect: None,
            });
        };
        let bbs = &entries[idx];

        if !bbs.visible || bbs.iconic {
            /* 最小化（或隐藏）＝ 用户这会儿看不到它 —— 卡片该收起来 */
            return Some(super::SlicerWindow {
                on_screen: false,
                covered: 0.0,
                rect: None,
            });
        }

        /* ③ 看它**前面**那些窗：别的进程、非浮层、真在屏上的，与它重叠了多少 */
        let area = bbs.rect.area();
        let mut covered = 0.0_f64;
        /* 盖得最多的那一个（pid, 占比）—— 只为了日志里能说出"是谁" */
        let mut worst: (Option<u32>, f64) = (None, 0.0);
        if area > 0.0 {
            for e in &entries[..idx] {
                if !e.visible || e.iconic || e.topmost || e.cloaked || e.pid == bbs.pid {
                    continue;
                }
                let w = (e.rect.right).min(bbs.rect.right) - (e.rect.left).max(bbs.rect.left);
                let h = (e.rect.bottom).min(bbs.rect.bottom) - (e.rect.top).max(bbs.rect.top);
                if w > 0 && h > 0 {
                    let share = f64::from(w) * f64::from(h) / area;
                    covered += share;
                    /* 记住盖得最多的那一个 —— 卡片被收掉时日志要能说出"是谁盖的" */
                    if share > worst.1 {
                        worst = (Some(e.pid), share);
                    }
                }
            }
        }
        /*
         * ★ **被收掉要说清是谁压的**（2026-10-20 作者报"卡片几秒钟就消失"）：
         * 光有 `covered` 一个数看不出该怪谁（实测就有过"最前面那份 BBS 上方压着
         * 编辑器的全屏窗"这种真·被盖住的情况）。带上 pid，日志一查就知道是
         * 抬窗没抬动（多半是我们自己那扇），还是用户真的切到别的软件去了。
         */
        if covered >= super::COVERED_ENOUGH {
            tracing::info!(
                slicer = bbs_pid,
                covered_by = worst.0,
                covered,
                "切片器窗口被别的窗压住"
            );
        }
        /*
         * ★ 与 mac 侧同一条：**量化成 1/20 步长**再交出去 —— 这个值会被拿去当
         * "变没变"的键，浮点零头抖动会让每一跳回读都判成"变了"。
         */
        let covered = (covered.min(1.0) * 20.0).floor() / 20.0;

        /* ④ 矩形给回逻辑像素（Tauri 的 set_position 用的是逻辑坐标） */
        let scale = {
            let dpi = unsafe { GetDpiForWindow(bbs.hwnd) };
            if dpi == 0 {
                1.0
            } else {
                f64::from(dpi) / 96.0
            }
        };
        let rect = crate::ax::RectDto {
            x: f64::from(bbs.rect.left) / scale,
            y: f64::from(bbs.rect.top) / scale,
            width: f64::from(bbs.rect.width()) / scale,
            height: f64::from(bbs.rect.height()) / scale,
        };
        Some(super::SlicerWindow {
            on_screen: true,
            covered,
            rect: Some(rect),
        })
    }

    extern "system" fn collect_cb(hwnd: Hwnd, lparam: isize) -> i32 {
        let Some(entries) = (unsafe { (lparam as *mut Vec<Entry>).as_mut() }) else {
            return 0;
        };
        let mut pid: u32 = 0;
        unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
        if pid == 0 {
            return 1;
        }
        let mut rect = RectPx::default();
        if unsafe { GetWindowRect(hwnd, &mut rect) } == 0 {
            return 1;
        }
        let style = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) };
        entries.push(Entry {
            hwnd,
            pid,
            rect,
            visible: unsafe { IsWindowVisible(hwnd) != 0 },
            iconic: unsafe { IsIconic(hwnd) != 0 },
            topmost: style & WS_EX_TOPMOST != 0,
            cloaked: is_cloaked(hwnd),
            has_title: unsafe { GetWindowTextLengthW(hwnd) > 0 },
        });
        1
    }
}
