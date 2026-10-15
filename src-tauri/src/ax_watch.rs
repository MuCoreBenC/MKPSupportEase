//! **切片器窗口的跟随信号**（2026-10-15）：引用引导卡片该不该露脸、该挪到哪。
//!
//! # 为什么不是一个定时器
//!
//! 作者原话：「有没有可能做到，只在 BBS 置顶的时候显示，BBS 被其他软件压住或者
//! 最小化之后消失？……而且能不能做到 BBS 拖动了之后位置变化了，就自动跟随？
//! **不要频繁的轮询**」。
//!
//! 这三件事在 macOS 上**是同一个东西**：Accessibility 的**通知**。
//! `AXObserverCreate(pid, 回调, &observer)` 注册下面五条，再把 observer 的
//! run loop source 挂到主线程 —— 切片器自己**主动 push**，我们一个定时器都不起：
//!
//! | 通知 | 它在说什么 |
//! | --- | --- |
//! | `kAXWindowMovedNotification` | 窗口被拖动了 ⇒ 重新读坐标、把卡片挪过去 |
//! | `kAXWindowResizedNotification` | 窗口被拉大缩小 ⇒ 同上 |
//! | `kAXWindowMiniaturizedNotification` | 最小化了 ⇒ **收起卡片** |
//! | `kAXWindowDeminiaturizedNotification` | 还原了 ⇒ 重新判断该不该露脸 |
//! | `kAXFocusedWindowChangedNotification` | 换了一扇最前的窗 ⇒ 它是"被压在后面了吗"的判据 |
//!
//! ★ **通知的载荷里没有坐标**，所以收到之后仍然要 [`crate::ax::locate_bed_type`]
//! 读一次。那是"被通知了才读"，不是"隔一会儿读一次" —— 与轮询是两回事。
//!
//! # 生命周期
//!
//! 一个**单例** watcher：卡片露脸时装上（[`start`]）、收起来时卸掉（[`stop`]）。
//! 装的时候顺手读一次当前可见性（订阅之前发生过的事不会补发）。
//!
//! # 回调必须回到主线程
//!
//! AX 回调是在**它自己的**线程上叫起来的，而 `get_webview_window` / `set_position`
//! 这些 Tauri 动作只能在主线程做。所以这里不做任何窗口操作，只把"该重算一次"这件事
//! **转成一条主线程任务**（`run_on_main_thread`）—— 真正的搬运在 [`crate::ipc::guide`]。
//!
//! # 边界（与 `ax.rs` 同一条）
//!
//! **只读**：只注册通知、只读坐标，没有任何 AX 写动作、不模拟点击、不改切片器参数。

#![cfg(target_os = "macos")]

use std::os::raw::c_void;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use core_foundation::base::TCFType as _;
use core_foundation::string::CFString;
use tauri::AppHandle;

type AXUIElementRef = *mut c_void;
type AXObserverRef = *mut c_void;
type CFStringRef = *const c_void;
type CFTypeRef = *const c_void;
type CFRunLoopRef = *mut c_void;
type CFRunLoopSourceRef = *mut c_void;
type AXError = i32;

/// 回调的签名：`AXObserverCallback(AXObserverRef, AXUIElementRef, CFStringRef, void*)`
type AXObserverCallback = extern "C" fn(AXObserverRef, AXUIElementRef, CFStringRef, *mut c_void);

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXObserverCreate(pid: i32, cb: AXObserverCallback, out: *mut AXObserverRef) -> AXError;
    fn AXObserverAddNotification(
        observer: AXObserverRef,
        element: AXUIElementRef,
        notification: CFStringRef,
        refcon: *mut c_void,
    ) -> AXError;
    fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    /* ★ 释放**不在这里声明**：一律走 [`crate::ax::cf_release`]（空安全）。
    少一个出口就少一处 `CFRelease(NULL)` 的机会。 */
    fn CFRunLoopGetMain() -> CFRunLoopRef;
    fn CFRunLoopAddSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: CFStringRef);
    fn CFRunLoopRemoveSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: CFStringRef);
}

/// 收到通知之后做什么 —— 由 `ipc::guide` 在装 watcher 时给（避免这一层反向依赖 IPC）。
///
/// ★ 它**已经在主线程上**（回调只负责投递，见 [`on_event`]），所以实现里可以直接
/// 碰窗口、读 SQLite、起进程。
pub type OnChange = fn(&AppHandle);

/// 装好的 watcher（单例）。卸的时候要留着这几个指针才能摘干净。
struct Watcher {
    observer: AXObserverRef,
    source: CFRunLoopSourceRef,
    app: AXUIElementRef,
    pid: i32,
}

/* AX 的这几个指针只在主线程那条 run loop 上用，`Send` 是安全的（我们不做跨线程释放） */
unsafe impl Send for Watcher {}

static WATCHER: OnceLock<Mutex<Option<Watcher>>> = OnceLock::new();

fn slot() -> &'static Mutex<Option<Watcher>> {
    WATCHER.get_or_init(|| Mutex::new(None))
}

/// 收到通知时要跳回的上下文（`refcon`）。全局一份 —— 同时只该有一个 watcher。
static HOOK: OnceLock<(AppHandle, OnChange)> = OnceLock::new();

/// **挂在这个应用元素上**的通知：跟"哪扇窗最前"有关的那几条。
///
/// ★★ 这里必须分清"通知挂在哪个元素上"（2026-10-15 调试最久的一个坑）：
///
/// - `AXWindowMoved` / `AXWindowResized` / `AXWindowMiniaturized` /
///   `AXWindowDeminiaturized` 是 **window 元素**的通知 —— 挂在 **app 元素**上
///   `AXObserverAddNotification` 会**失败**（非 0 返回），而且**悄无声息**；
/// - `AXFocusedWindowChanged` 是 **app 元素**的通知。
///
/// 第一版四条全挂在 app 上 ⇒ 只有最后那条挂上了 ⇒ **拖动、最小化一条都收不到**
/// （作者报的"拖动走了也没跟随 / 最小化了没变化"就是这个）。
const APP_NOTIFICATIONS: [&str; 1] = ["AXFocusedWindowChanged"];

/// 挂在**那一扇窗**上的通知。
const WINDOW_NOTIFICATIONS: [&str; 4] = [
    "AXWindowMoved",
    "AXWindowResized",
    "AXWindowMiniaturized",
    "AXWindowDeminiaturized",
];

/// AX 回调：**在 AX 自己的线程上**。
///
/// ★★★ 这个身子**只做一件事**：把"变了，去看看"排进主线程队列。**别的什么都不碰** ——
///
/// - 不读 AX（那是调用对方进程，可能阻塞）；
/// - 不 `pgrep`（[`crate::ax::slicer_visibility`] 里会起子进程，一路拖动就是几十次）；
/// - 不碰任何窗口。
///
/// 之前是在这儿读一次可见性再传下去，代价是"在 AX 的线程上跑了一整套 AX 读 + 起进程"。
/// 现在可见性由 [`crate::ipc::guide::sync`]**在主线程上读一次**（单一读点）。
///
/// ★★ 整个身子**包在 `catch_unwind` 里**：这是 `extern "C"` 的回调，panic 一旦往外
/// 展开就是**跨 FFI 边界** —— 那是 UB，实际表现就是**整个进程被抬走**。
/// 宁可丢掉这条通知（下一个动作还会再通知），也不要让 App 闪退。
extern "C" fn on_event(_: AXObserverRef, _: AXUIElementRef, _: CFStringRef, _: *mut c_void) {
    let Some((app, hook)) = HOOK.get() else {
        return;
    };
    let inner = app.clone();
    let hook = *hook;
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = app.run_on_main_thread(move || hook(&inner));
    }));
    if outcome.is_err() {
        tracing::warn!("处理 AX 通知时 panic 了（已吞掉，不让它跨 FFI 边界掀掉进程）");
    }
}

/// 装 watcher（已经装着就先卸掉重装 —— 切片器可能重启了，pid 换了）。
///
/// `hook` 是"发生了就叫我"的落点，由 `ipc::guide` 给。
///
/// ★★★ **整件事都派回主线程做**。`CFRunLoop` **不是线程安全的**，而
/// `ipc::guide::guide_show` 是 `#[tauri::command] async fn` —— 它跑在 tokio
/// **工作线程**上。在工作线程里 `CFRunLoopAddSource(CFRunLoopGetMain(), …)`
/// 是 UB，实际表现就是**点了就闪退**（2026-10-15）。
///
/// 因此这里**不再同步返回成败**：真结果走日志与 [`is_complete`]，
/// 调用方要"挂满了没有"就查那个（`ipc::guide::sync` 就是这么补挂的）。
pub fn start(app: &AppHandle, hook: OnChange) {
    let inner = app.clone();
    /* 已经主线程了也走排队（Tauri 的 `run_on_main_thread` 是投递，不会就地执行） */
    let _ = app.run_on_main_thread(move || start_now(&inner, hook));
}

/// [`start`] 的正身 —— **只允许在主线程上调**。
fn start_now(app: &AppHandle, hook: OnChange) {
    stop_now();
    let Some(pid) = crate::ax::slicer_pid() else {
        return;
    };

    unsafe {
        let mut observer: AXObserverRef = std::ptr::null_mut();
        if AXObserverCreate(pid, on_event, &mut observer) != 0 || observer.is_null() {
            return;
        }
        let el = AXUIElementCreateApplication(pid);

        /* ① app 元素那一条（哪扇窗最前） */
        let mut added = add_all(observer, el, &APP_NOTIFICATIONS);

        /*
         * ② 那扇窗自己的四条（移动 / 缩放 / 最小化 / 还原）。
         *
         * ★ 窗口是**后来才有的**（切片器可能正开着工程）：取不到就只挂 app 那条，
         * 并**记住没取到** —— 下一次 `FocusedWindowChanged` 会再来一遍
         * （见 [`crate::ipc::guide::sync`] 里那条"没挂满就补挂"的路）。
         *
         * ★★ 用 `with_top_window`（回调）而不是"拿一个句柄回去"：句柄底下那个 CF
         * 对象归 `Owned` 管，一离开那个作用域就 `CFRelease` 了 —— 拿着它去挂通知
         * 就是读已释放的内存（同一档闪退）。
         */
        added +=
            crate::ax::with_top_window(pid, |win| add_all(observer, win, &WINDOW_NOTIFICATIONS))
                .unwrap_or(0);

        /* 一条都没挂上 = 不吃通知（多半是没授权），当作没装成 */
        if added == 0 {
            crate::ax::cf_release(observer as CFTypeRef);
            crate::ax::cf_release(el as CFTypeRef);
            tracing::warn!(pid, "一条 AX 通知都没挂上（没辅助功能权限？）");
            return;
        }

        let source = get_run_loop_source(observer);
        if source.is_null() {
            crate::ax::cf_release(observer as CFTypeRef);
            crate::ax::cf_release(el as CFTypeRef);
            return;
        }
        let mode = CFString::new("kCFRunLoopCommonModes");
        CFRunLoopAddSource(
            CFRunLoopGetMain(),
            source,
            mode.as_CFTypeRef() as CFStringRef,
        );
        drop(mode);

        /* 上下文只该有一份：已经装过（且刚 stop 掉了）就沿用第一次的那对 */
        let _ = HOOK.set((app.clone(), hook));
        if let Ok(mut g) = slot().lock() {
            *g = Some(Watcher {
                observer,
                source,
                app: el,
                pid,
            });
        }
        let complete = added == FULL_SET;
        COMPLETE.store(complete, std::sync::atomic::Ordering::Relaxed);
        tracing::info!(pid, added, complete, "已挂上切片器窗口的 AX 通知");
    }

    /* ★ 挂上了才开始回读（挂着都没挂上就没必要跑）—— 见 [`TICK`] 的注释 */
    start_tick(app, hook);
}

/// 一组通知全挂上了是多少条（用来报"挂满了没有"）。
const FULL_SET: usize = APP_NOTIFICATIONS.len() + WINDOW_NOTIFICATIONS.len();

/// **细定时回读**的间隔（2026-10-15 作者："拖动了它没有跟随"）。
///
/// ★★★ 为什么非要有它，而不是纯靠通知（作者原本的要求是"不要频繁的轮询"）：
///
/// 切片器是 **Qt 应用**，实测它**不发** `AXWindowMoved` —— 拖动窗口时一条通知都没有
/// （而最小化 / 还原 / 换焦点那几条它是发的，那几条仍然走通知）。位置这一条因此
/// 没有 push 可指望，只能自己按一个**很低**的频率回读。
///
/// ★ 这次回读**很便宜**：[`crate::ipc::guide::sync`] 先花一两次属性读算一个"状态键"
/// （窗口矩形 + 最前是谁 + 最小化了没有），**没变就直接返回** —— 不重扫控件树。
/// 真正重扫只在"确实变了"时发生，也就是用户真的动了窗口。
///
/// ★ 只在卡片可能露脸的那段时间跑（watcher 活着的期间），点完「知道了」就停。
const TICK: Duration = Duration::from_millis(350);

/// 回读线程该活着吗（`stop_now` 关掉它）。
static TICKING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// 起那条细定时回读（幂等：已经跑着就不重复起）。
///
/// 线程里只做两件事：睡 [`TICK`]、把 `hook` 排到主线程去跑。判断"要不要干活"
/// 全交给 `hook` 自己（它第一步就是比那个很便宜的状态键，见 [`TICK`] 的注释）。
fn start_tick(app: &AppHandle, hook: OnChange) {
    if TICKING.swap(true, std::sync::atomic::Ordering::Relaxed) {
        /* 已经跑着一条了（`stop_now` 只置标志，线程可能还没睡醒退出） */
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        while TICKING.load(std::sync::atomic::Ordering::Relaxed) {
            std::thread::sleep(TICK);
            if !TICKING.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            let inner = app.clone();
            /* 判断在主线程做（AX 读 + 窗口动作都在那边，见 `ipc::guide::sync`） */
            let _ = app.run_on_main_thread(move || hook(&inner));
        }
    });
}

/// 上一次装的时候**挂满了没有**。
///
/// ★ 为什么要有它：切片器那扇窗可能**还没出现**（没打开工程）—— 那时只有 app 那条
/// 挂得上，而"拖动跟随 / 最小化消失"全靠窗口那四条。调用方（`ipc::guide::sync`）
/// 靠这个值决定"下次它看得见时再补挂一次"。
static COMPLETE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// 上一次 `start` 是不是把所有通知都挂上了。
pub fn is_complete() -> bool {
    COMPLETE.load(std::sync::atomic::Ordering::Relaxed)
}

/// 正在盯着的那个切片器进程号（没装着就是 `None`）。
///
/// ★ 给 [`crate::ipc::guide::sync`] 用：那边每条通知都要判可见性，而可见性要用 pid
/// —— 从这里拿，别再起 `pgrep`（拖动一次几十条通知，那是几十个子进程）。
pub fn watched_pid() -> Option<i32> {
    let m = WATCHER.get()?;
    let g = m.lock().ok()?;
    g.as_ref().map(|w| w.pid)
}

/// 把一组通知挂到某个元素上，返回**成功挂上**的条数。
///
/// ★ 逐条计数而不是"有一条第成功就算成"：挂错元素时会**部分成功**，
/// 而那正是最难查的一档（第一版四条全挂 app 上 ⇒ 只成功一条 ⇒ 静默失去跟随）。
unsafe fn add_all(observer: AXObserverRef, el: AXUIElementRef, names: &[&str]) -> usize {
    let mut added = 0;
    for name in names {
        let key = CFString::new(name);
        let ok = AXObserverAddNotification(
            observer,
            el,
            key.as_CFTypeRef() as CFStringRef,
            std::ptr::null_mut(),
        ) == 0;
        drop(key);
        if ok {
            added += 1;
        } else {
            tracing::warn!(notification = name, "这条 AX 通知没挂上");
        }
    }
    added
}

/* `AXObserverGetRunLoopSource` 在 ApplicationServices 里，与上面那个 extern 块
分开放只是为了读着顺；`unsafe` 包在调用点上。 */
extern "C" {
    fn AXObserverGetRunLoopSource(observer: AXObserverRef) -> CFRunLoopSourceRef;
}

fn get_run_loop_source(observer: AXObserverRef) -> CFRunLoopSourceRef {
    unsafe { AXObserverGetRunLoopSource(observer) }
}

/// 卸掉 watcher（没装着就是 no-op）。
///
/// ★★ 与 [`start`] 同理，**必须先派回主线程**：`CFRunLoop` 不是线程安全的。
/// `guide_close` 也是 `async` 命令（工作线程），从那儿直接卸是 UB。
pub fn stop(app: &AppHandle) {
    COMPLETE.store(false, std::sync::atomic::Ordering::Relaxed);
    let _ = app.run_on_main_thread(stop_now);
}

/// [`stop`] 的正身 —— **只允许在主线程上调**。
///
/// 顺序有讲究：**先摘 source、再释放 observer**（[`CFRunLoopSourceInvalidate`] 之类
/// 都省了 —— 那个 source 归 observer 所有，**不是我们的**，多碰一下就是多一份
/// 风险；摘掉 + 释放 observer，source 跟着没）。
fn stop_now() {
    TICKING.store(false, std::sync::atomic::Ordering::Relaxed);
    let Some(m) = WATCHER.get() else {
        return;
    };
    let Ok(mut g) = m.lock() else {
        return;
    };
    let Some(w) = g.take() else {
        return;
    };
    unsafe {
        if !w.source.is_null() {
            let mode = CFString::new("kCFRunLoopCommonModes");
            CFRunLoopRemoveSource(
                CFRunLoopGetMain(),
                w.source,
                mode.as_CFTypeRef() as CFStringRef,
            );
            drop(mode);
        }
        if !w.observer.is_null() {
            crate::ax::cf_release(w.observer as CFTypeRef);
        }
        if !w.app.is_null() {
            crate::ax::cf_release(w.app as CFTypeRef);
        }
    }
    tracing::info!(pid = w.pid, "已摘下切片器窗口的 AX 通知");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ★★ **通知归属哪一边，是这套跟随的成败点**（2026-10-15 那个静默 bug）。
    ///
    /// 这四条是 **window 元素**的通知，挂到 app 元素上会**失败且不报错**；
    /// 挂错了 = 拖动不跟随、最小化不消失，而且日志里只有一行 warn。
    /// 判据钉在这里，改名单前先想清楚。
    #[test]
    fn window_notifications_belong_to_the_window_element() {
        assert_eq!(
            WINDOW_NOTIFICATIONS,
            [
                "AXWindowMoved",
                "AXWindowResized",
                "AXWindowMiniaturized",
                "AXWindowDeminiaturized",
            ],
            "这四条只有挂在 window 元素上才挂得住 —— 挪到 app 那组会让跟随整条哑掉"
        );
    }

    /// 这一条反过来：`AXFocusedWindowChanged` 是 **app 元素**的通知。
    #[test]
    fn focus_notification_belongs_to_the_app_element() {
        assert_eq!(APP_NOTIFICATIONS, ["AXFocusedWindowChanged"]);
    }

    /// 两组**不许有重叠**：同一条挂错边就是静默失败。
    #[test]
    fn the_two_groups_do_not_overlap() {
        for name in WINDOW_NOTIFICATIONS {
            assert!(
                !APP_NOTIFICATIONS.contains(&name),
                "{name} 同时在两组里 —— 它只能属于一边"
            );
        }
    }

    /// "挂满了"是按两组总数算的 —— 只挂上 app 那条**不算**挂满（要触发补挂）。
    #[test]
    fn full_set_counts_both_groups() {
        assert_eq!(
            FULL_SET,
            APP_NOTIFICATIONS.len() + WINDOW_NOTIFICATIONS.len(),
            "挂满 = 两组都挂上；只挂上 app 那条必须判为不完整，否则不会补挂"
        );
        assert!(
            APP_NOTIFICATIONS.len() < FULL_SET,
            "只有 app 那组时**必须**小于 FULL_SET"
        );
    }
}
