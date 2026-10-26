//! **应用本体这一层**（2026-10-17）：打开预设目录、卸载。
//!
//! 两条命令都不碰业务数据：
//!   · [`open_preset_folder`] 只是把 Finder / 资源管理器喊出来（一个字节都不读）
//!   · [`uninstall`] 删的是**程序本体**，用户数据（`AppData` 那几棵树）一个不碰
//!
//! 为什么单开一个模块：`ipc/prefs.rs` 管的是"用户点出来的一档偏好"，
//! `ipc/catalog.rs` 管"数据从哪儿来"；这两条既不是偏好也不是数据源 ——
//! 它们是**进程与安装包自己**的事。混进去会让"这条命令要不要联网/碰文件"这个问题
//! 又得靠通读代码来回答。

use postprocess::diag::messages;

use std::path::{Path, PathBuf};

use tauri::AppHandle;

use crate::error::AppError;

use super::traced;

/// 打开**用户预设目录**（`<appDataDir>/presets-local`）并把窗口喊出来。
///
/// 「预设位置」那一行的行为就是这个 —— 作者的原话：「预设位置其实也就是打开位置而已，
/// 打开我们那个程序那些文件的位置」。
///
/// ★ 目录不在就先建出来：新建的机器上这个目录可能还没被创建过，
///   报"找不到"只会让用户以为坏了；建一个空目录是**无害**的（它本来就该在）。
/// ★ 与 `ipc::local::reveal_in_folder` 分工不同：那一条认**某一份文件**
///   （还要过用户根的两道闸），这一条只认**那个目录本身** —— 不接受任何由前端拼的路径。
#[tauri::command]
pub async fn open_preset_folder(app: AppHandle) -> Result<String, AppError> {
    traced("openPresetFolder", |_| {
        let dir = preset_folder(&app)?;
        reveal_dir(&app, &dir)?;
        Ok(dir.display().to_string())
    })
}

/// 用户预设目录 `<appDataDir>/presets-local`（`presets-local/` 那一格）。
fn preset_folder(app: &AppHandle) -> Result<PathBuf, AppError> {
    let user_root = crate::fsx::paths::user_root(app)?;
    let dir = user_root.join(crate::fsx::paths::LOCAL_DIR);
    if !dir.is_dir() {
        /* 建一个空目录：它是"你那些预设该放这儿"的落点，不存在时建成空的无害 */
        std::fs::create_dir_all(&dir).map_err(|e| {
            AppError::io(messages::text("ui.ipc.app.030", &[])).with_detail(e.to_string())
        })?;
    }
    Ok(dir)
}

/// 在文件管理器里**打开这个目录**（不是"在里面选中某个文件"）。
fn reveal_dir(app: &AppHandle, dir: &Path) -> Result<(), AppError> {
    use tauri_plugin_opener::OpenerExt as _;
    app.opener()
        .open_path(dir.display().to_string(), None::<&str>)
        .map_err(|e| AppError::io(messages::text("ui.ipc.app.031", &[])).with_detail(e.to_string()))
}

/// **卸载应用**：删程序本体，然后退出。
///
/// # 它到底做什么
///
/// - macOS：把 `.app` 包整个删掉（`current_exe` 往上找到 `.app` 那一层），然后退出。
///   正在运行的包被删掉之后系统会留着那些进程，直到它们自己退出 —— 所以我们
///   **先删再退**：删完这一下进程就没了，Dock 上那个图标也没了。
/// - Windows：起一个 detach 的 `cmd`，等我们退出之后再删（正在运行的 exe 删不掉）。
/// - 其他平台：如实报"没做"，**不假装卸掉了**。
///
/// # 用户数据怎么办
///
/// **一个不碰**。`AppData` 那几棵树（预设、执行记录、备份）原样留着 ——
/// 这与界面上那句"用户数据保留"必须一致，否则用户重装回来发现东西全没了。
/// 真想连数据一起清，那是另一件事（本轮明确不做）。
///
/// # 失败
///
/// 原样报错，带错误码（`E_SYS_UNINSTALL_001`）—— 这是不可逆动作，
/// 静默失败会让用户以为已经卸了、结果下次切片器还在调它。
#[tauri::command]
pub async fn uninstall(app: AppHandle) -> Result<(), AppError> {
    traced("uninstall", |_| {
        let exe = std::env::current_exe().map_err(|e| {
            AppError::io(messages::text("ui.ipc.app.032", &[])).with_detail(e.to_string())
        })?;

        #[cfg(target_os = "macos")]
        {
            let bundle = macos_app_bundle(&exe)?;
            std::fs::remove_dir_all(&bundle).map_err(|e| {
                /* detail 里带上错误码：卸载是不可逆动作，用户报问题时一个码就能定位 */
                AppError::io(messages::text("ui.ipc.app.033", &[]))
                    .with_detail(format!("E_SYS_UNINSTALL_001 {}：{e}", bundle.display()))
            })?;
            /* 删完就走：进程留着会让 Dock 上留一个空图标 */
            app.exit(0);
            Ok(())
        }

        #[cfg(target_os = "windows")]
        {
            /* 正在跑的 exe 删不掉 ⇒ 交给一个 detach 的 cmd：等两秒、删目录、删自己 */
            let dir = exe
                .parent()
                .ok_or_else(|| AppError::io(messages::text("ui.ipc.app.034", &[])))?
                .to_path_buf();
            std::process::Command::new("cmd")
                .args([
                    "/c",
                    "timeout",
                    "/t",
                    "2",
                    "/nobreak",
                    ">nul",
                    "&",
                    "rmdir",
                    "/s",
                    "/q",
                    &dir.display().to_string(),
                ])
                .spawn()
                .map_err(|e| {
                    AppError::io(messages::text("ui.ipc.app.035", &[]))
                        .with_detail(format!("E_SYS_UNINSTALL_001 {e}"))
                })?;
            app.exit(0);
            Ok(())
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let _ = exe;
            Err(
                AppError::io(messages::text("ui.ipc.app.036", &[])).with_detail(
                    ::postprocess::diag::messages::text("ui.ipc.app.why_uninstall_platform", &[]),
                ),
            )
        }
    })
}

/// 从 `…/Foo.app/Contents/MacOS/foo` 往上找到 `…/Foo.app`。
///
/// 找不到就报错 —— **绝不**拿 `current_exe` 自己当目标删（那是"把可执行文件删了
/// 但包装还留着"的半吊子状态，比不删更糟）。
#[cfg(target_os = "macos")]
fn macos_app_bundle(exe: &Path) -> Result<PathBuf, AppError> {
    exe.ancestors()
        .find(|p| p.extension().is_some_and(|e| e == "app"))
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            AppError::io(messages::text("ui.ipc.app.037", &[]))
                .with_detail(format!("E_SYS_UNINSTALL_001 {}", exe.display()))
        })
}
