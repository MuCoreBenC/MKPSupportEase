//! **切片器窗口的跟随信号 —— Windows 那一半**（2026-10-19）。
//!
//! 与 [`crate::ax_watch`]（macOS）同一个角色：卡片露脸时装上、收起来时卸掉，
//! 每次变化把 [`crate::ipc::guide::sync`] 叫起来重算一次。
//!
//! # 为什么这里只有定时回读、没有推送
//!
//! macOS 那边的主体是 AX 通知（push），定时回读只是兜底；Windows 上对应的推送
//! 是 `SetWinEventHook` —— 但**没有它也行**：
//!
//! - `sync → decide` 的第一段本来就是一个**便宜的闸门**：读一次"窗口矩形 + 层序"
//!   （`EnumWindows`，全在本地，不打给切片器进程），**没变就一丁点多余的活都不干**
//!   （重扫控件树才是贵的，而它只在"真变了"时发生）；
//! - 作者 2026-10-15 定的省电口径："那个卡片多等一会再跟上也没关系，怎么样节省就怎么样做吧"
//!   ⇒ macOS 的兜底回读是 700ms 一次、每次只比一个本地键 —— Windows 直接**就用这个频率**
//!   当唯一信号，观感与省电都落在作者认可的那一档；
//! - 换 `SetWinEventHook` 要多养一条消息泵线程、多一组 `EVENT_OBJECT_LOCATIONCHANGE`
//!   的过滤与去抖，换来的只是"跟得更快几百毫秒" —— 不值。
//!
//! ★ 所以这里**不是高频轮询**：每 700ms 一次本地窗口列表查询 + 一次键比较；
//! 键没变时连切片器的控件树都不碰。与 macOS 的"通知 + 兜底回读"在 `decide`
//! 那一层汇合成同一套逻辑（见 `ipc::guide::sync`）。
//!
//! # 边界（与 `ax.rs` / `ax_watch.rs` 同一条）
//!
//! **只读**：只查窗口列表与矩形，不模拟输入、不碰别的进程的内存、不改切片器参数。

#![cfg(target_os = "windows")]

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tauri::AppHandle;

/// 收到"该重算一次"时跳的钩子 —— 由 `ipc::guide` 给（与 macOS 那边同一个 `sync`）。
pub type OnChange = fn(&AppHandle);

/// 回读的间隔。**与 macOS 的兜底回读同一个数**（见 `ax_watch` 的 `TICK`）：
/// 每跳只比一个本地键，键没变就返回 —— 跟随慢半拍但几乎不花电。
const TICK: Duration = Duration::from_millis(700);

/// 回读线程该活着吗（`stop` 关掉它）。
static TICKING: AtomicBool = AtomicBool::new(false);

/// 装上 watcher（已经装着就不动 —— 与 macOS 侧不同：Windows 没有 pid 可换装的问题，
/// `watched_pid` 每跳都从 [`crate::ax::slicer_pid`] 现读，切片器重启也自然跟上）。
pub fn start(app: &AppHandle, hook: OnChange) {
    if TICKING.swap(true, Ordering::Relaxed) {
        return;
    }
    let app = app.clone();
    let spawned = std::thread::Builder::new()
        .name("guide-win-watch".into())
        .spawn(move || {
            while TICKING.load(Ordering::Relaxed) {
                std::thread::sleep(TICK);
                if !TICKING.load(Ordering::Relaxed) {
                    break;
                }
                hook(&app);
            }
        });
    if spawned.is_err() {
        /* 线程没起成：照实归零，下一跳补装还会再试 */
        TICKING.store(false, Ordering::Relaxed);
    }
}

/// 卸掉 watcher（没装着就是 no-op）。
pub fn stop(_app: &AppHandle) {
    TICKING.store(false, Ordering::Relaxed);
}

/// watcher 活着吗（`ipc::guide::ensure_watcher` 用它决定要不要补装）。
pub fn is_complete() -> bool {
    TICKING.load(Ordering::Relaxed)
}

/// 正在盯着的切片器进程号。
///
/// ★ **现读**而不是记在装 watcher 那一刻：切片器可能被重启（pid 换了），
/// 而 Windows 这边"认进程"就是一次 Toolhelp 快照，每跳现读没有额外负担；
/// 与 `decide` 的闸门配合：pid 变了 = 键变了 = 那一跳本来就会走真读。
pub fn watched_pid() -> Option<i32> {
    crate::ax::slicer_pid()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 回读间隔与 macOS 的兜底回读必须同档 —— 这是有作者裁决背书的数（省电口径）。
    #[test]
    fn tick_matches_the_macos_fallback() {
        assert_eq!(TICK, Duration::from_millis(700));
    }
}
