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

// 后厨工作台（B03）。**默认构建里下面这一行不成立**，所以 `src/workbench/` 整个子树连编译
// 都不会被碰，给用户的二进制里搜不到任何 `wb_` 命令。
// 见 .comate/specs/b03-backstage-workbench/doc.md §4
//
// （这里刻意用行注释：块注释在 Rust 里是可嵌套的，正文里出现 `/` 加 `*` 会开一个新注释，
// 而路径通配写法很容易写出那两个字符。）
#[cfg(feature = "workbench")]
pub mod workbench;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    with_commands(tauri::Builder::default())
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

            /* 后厨工作台的第二个窗口。**刻意在运行时建，而不是写进 tauri.conf.json**：
            配置里的 windows 数组是整体覆盖的，把工作台窗口写进一份"给工作台用的配置"
            就得把 main 窗口也抄一遍 —— 两份声明迟早漂移。写在这里，窗口的存在与
            feature 严格同生共死，不需要任何一份配置去声明它。
            开不出来只警告不中止：工作台开不出来是开发者的事，不该让客户端窗口也起不来 */
            #[cfg(feature = "workbench")]
            if let Err(e) = workbench::open_window(&handle) {
                tracing::warn!("工作台窗口开不出来：{e}");
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Tauri 启动失败");
}

/* ---------- 命令清单 ----------

Tauri 只允许调一次 `invoke_handler`，所以"客户端命令 + 可选的工作台命令"没法拼接，
只能按 feature 给出两份完整清单。

**两份里客户端那几个必须一字不差地同时出现。** 新增客户端命令时改两处 —— 这是
Tauri 的 API 形状决定的，不是这里想省事；把它放在相邻的两个函数里，是为了漏改时
一眼能看出来。 */

#[cfg(not(feature = "workbench"))]
fn with_commands(b: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    b.invoke_handler(tauri::generate_handler![
        ipc::get_preset,
        ipc::save_offsets,
        ipc::get_calib_models,
        ipc::open_model,
    ])
}

#[cfg(feature = "workbench")]
fn with_commands(b: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    b.invoke_handler(tauri::generate_handler![
        ipc::get_preset,
        ipc::save_offsets,
        ipc::get_calib_models,
        ipc::open_model,
        workbench::wb_roots,
        workbench::wb_bootstrap,
        workbench::wb_registry,
        workbench::wb_tree,
        workbench::wb_effective,
        workbench::wb_save_version,
        workbench::wb_save_machine_base,
        workbench::wb_draft,
        workbench::wb_save_draft,
        workbench::wb_clear_draft,
        workbench::wb_base_draft,
        workbench::wb_save_base_draft,
        workbench::wb_clear_base_draft,
        workbench::wb_create_machine,
        workbench::wb_create_version,
        workbench::wb_clone_version,
        workbench::wb_rename_version,
        workbench::wb_preview_move,
        workbench::wb_move_version,
        workbench::wb_trash_version,
        workbench::wb_trash_list,
        workbench::wb_restore_from_trash,
        workbench::wb_purge_from_trash,
        workbench::wb_matrix,
        workbench::wb_bulk_preview,
        workbench::wb_bulk_apply,
        workbench::wb_view_state,
        workbench::wb_save_view_state,
        workbench::wb_status,
        workbench::wb_generate_stale,
        workbench::wb_generate_all,
        workbench::wb_generate_one,
        workbench::wb_restore_recipe,
        workbench::wb_bbs_list,
        workbench::wb_bbs_import,
        workbench::wb_set_machine_bbs,
        workbench::wb_set_version_bbs,
        workbench::wb_catalog,
        workbench::wb_save_catalog,
        workbench::wb_bundles,
        workbench::wb_save_bundles,
        workbench::wb_preflight,
        workbench::wb_publish,
    ])
}
