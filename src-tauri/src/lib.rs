//! SupportEase 的 Rust 侧。
//!
//! 结构就四块：
//! - [`error`] —— 跨 IPC 边界的错误模型（与 `src/api/contract.ts` 对齐）
//! - [`obs`] —— 日志与 trace id
//! - [`fsx`] —— 两层数据根、防穿越、唯一的写盘出口
//! - [`ipc`] —— 前端能调的命令
//!
//! 启动顺序有讲究：**先建数据根，再装日志**（日志要写进 `internal_root/logs`），
//! 而这两步失败都不阻断启动 —— 用户要的是软件能开，不是日志齐全。

pub mod chrome;
pub mod error;
pub mod fsx;
pub mod ipc;
pub mod obs;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();

            /* 数据根：建不出来也继续，日志目录随之退到 stderr。
            把路径打出来是刻意的 —— 出问题时第一句要问的就是"它在找哪个目录" */
            match fsx::paths::internal_root(&handle) {
                Ok(root) => {
                    obs::tracing::init_tracing(&root.join("logs"));
                    tracing::info!(root = %root.display(), "内部数据根就位");
                    match fsx::paths::user_root(&handle) {
                        Ok(user) => tracing::info!(root = %user.display(), "用户数据根就位"),
                        Err(e) => tracing::warn!("用户数据根建不出来：{e}"),
                    }
                }
                Err(e) => {
                    eprintln!("[setup] 内部数据根建不出来，日志只写 stderr：{e}");
                    obs::tracing::init_tracing(std::path::Path::new("/tmp/supportease-logs"));
                    tracing::warn!("内部数据根不可用：{e}");
                }
            }

            /* 窗口外观：原生圆角 + 让 AppKit 按统一工具栏那一档摆红绿灯。
            两件事都只在 macOS 上有意义，失败都只打日志 —— 外观问题不该挡启动 */
            if let Some(win) = app.get_webview_window("main") {
                chrome::install_unified_toolbar(&win);
                chrome::apply_native_corners(&win);
            } else {
                eprintln!("[setup] 找不到 main 窗口，跳过窗口外观");
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ipc::get_preset,
            ipc::save_offsets,
            ipc::get_calib_models,
            ipc::open_model,
            ipc::get_test_models,
        ])
        .run(tauri::generate_context!())
        .expect("Tauri 启动失败");
}
