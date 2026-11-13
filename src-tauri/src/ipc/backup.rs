//! **备份的两条命令**（2026-10-17）：打一份、恢复一份。
//!
//! 界面在**设置页**那一扇「备份与恢复」里（2026-11-09 起两页合一）：备份那一页挑一个
//! 文件夹（默认是桌面）、可选**不带预设** → 打一份 zip；恢复那一页挑一份 zip → 全量恢复。
//! 路径由系统文件选择框给（前端那侧），后端只认"用户点名的那个位置"。
//!
//! # 为什么是 `async` + `spawn_blocking`（2026-11-09）
//!
//! 这两条命令原来写成同步 `fn`：Tauri 把同步命令体放在**主线程**上跑，几百 MB 的执行归档
//! 一进 zip，整个窗口就不动了 —— 作者看到的是"一直在转圈 / 卡在这"。命令体是纯阻塞 IO，
//! 所以整块丢进线程池，主线程只等结果；过程由 `on_tick` 那条 Channel 一跳一跳送回界面。
//!
//! # 恢复仍然不可撤销
//!
//! 恢复会把现在的数据换掉，所以：
//!
//! - 二次确认在**界面**那侧做（与「清空历史」同一条规矩：危险动作都要问一句）；
//! - 后端这一步只做一件事：**照做**，做完把"恢复到哪一份、多少个文件"回给界面。

use postprocess::diag::messages;

use std::path::PathBuf;

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};

use crate::error::AppError;
use crate::runtime::backup::{BackupSummary, Progress, Stage};

/// 一次备份 / 恢复的水位（走 Channel 送回**这一次调用**，不是全局广播）。
///
/// `done` / `total` 的单位按动作定：打包是字节，恢复是文件条数（见 [`Progress`]）。
/// 界面只认 `total > 0` 才画百分比 —— 清点阶段 `total = 0` 是"还不知道"，不是"零"。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupTick {
    /// `scan`（清点 / 准备）或 `pack`（真在搬）
    pub stage: String,
    /// 正在处理的那一份（清点阶段是空串）
    pub file: String,
    pub done: u64,
    pub total: u64,
}

/// 发货。**Channel 关了（调用方已经不再听）不是错误** —— 记一行 debug 就够，
/// 不能因为没人听而把整次备份判失败（与下载那条 `send_tick` 同一条口径）。
fn send_tick(on_tick: &Channel<BackupTick>, at: &Progress) {
    if let Err(e) = on_tick.send(BackupTick {
        stage: match at.stage {
            Stage::Scan => "scan",
            Stage::Pack => "pack",
        }
        .to_owned(),
        file: at.file.clone(),
        done: at.done,
        total: at.total,
    }) {
        tracing::debug!("备份进度送不出去（没人听了？）：{e}");
    }
}

/// **打一份备份**：把这台机器的家当收进 zip，落在 `dest_dir` 里。
///
/// `include_presets = false`：跟预设有关的那几棵（用户 toml / 修改历史 / 云端原文镜像 /
/// 原文备份 / 回收站）不进包 —— 界面那一扇里的「连同预设一起备份」就是它。
///
/// `include_history = false`：执行归档（`user/gcode_history`）不进包 —— 那是包里最大的一块，
/// 界面那一扇里的「连同执行记录一起备份」就是它。
///
/// 返回**备份文件的绝对路径** —— 界面照原样显示（用户要知道东西落在哪了）。
#[tauri::command]
pub async fn create_backup(
    app: AppHandle,
    dest_dir: String,
    include_presets: bool,
    include_history: bool,
    on_tick: Channel<BackupTick>,
) -> Result<BackupSummary, AppError> {
    let task = tauri::async_runtime::spawn_blocking(move || {
        crate::ipc::traced("createBackup", |_| {
            let internal = crate::fsx::paths::internal_root(&app)?;
            let dest = std::path::Path::new(dest_dir.trim());
            if dest_dir.trim().is_empty() {
                return Err(AppError::invalid_argument(messages::text(
                    "ui.ipc.backup.038",
                    &[],
                )));
            }
            if !dest.is_dir() {
                return Err(AppError::not_found(messages::text(
                    "ui.ipc.backup.039",
                    &[("v1", &dest.display().to_string())],
                )));
            }
            let mut forward = |at: &Progress| send_tick(&on_tick, at);
            let sum = crate::runtime::backup::create(
                &internal,
                dest,
                include_presets,
                include_history,
                &mut forward,
            )?;
            tracing::info!(
                path = %sum.path,
                files = sum.files,
                bytes = sum.bytes,
                presets = include_presets,
                history = include_history,
                "备份打好了"
            );
            Ok(sum)
        })
    });
    task.await.map_err(|e| {
        AppError::internal(messages::text("ui.ipc.backup.041", &[])).with_detail(e.to_string())
    })?
}

/// **从一份备份恢复**：按 `scope` 只动选中的那一块。
///
/// `scope` 三档（与界面那颗下拉一一对应）：`all` 全放回、`presets` 只预设、
/// `settings` 只设置与执行记录。认不出的值**报错**（不拿默认值圆过去）。
///
/// 恢复完界面要**整窗重载**（数据在盘上换过了，内存里的东西都不算数）——
/// 这一步由前端做，这里只回"恢复到哪一份、多少个文件"。
#[tauri::command]
pub async fn restore_backup(
    app: AppHandle,
    zip_path: String,
    scope: String,
    on_tick: Channel<BackupTick>,
) -> Result<BackupSummary, AppError> {
    let task = tauri::async_runtime::spawn_blocking(move || {
        crate::ipc::traced("restoreBackup", |_| {
            let internal = crate::fsx::paths::internal_root(&app)?;
            let at = std::path::Path::new(zip_path.trim());
            if zip_path.trim().is_empty() || !at.is_file() {
                return Err(AppError::not_found(messages::text(
                    "ui.ipc.backup.040",
                    &[],
                )));
            }
            let scope = match scope.as_str() {
                "all" => crate::runtime::backup::Scope::All,
                "presets" => crate::runtime::backup::Scope::Presets,
                "settings" => crate::runtime::backup::Scope::Settings,
                other => {
                    return Err(AppError::invalid_argument(messages::text(
                        "ui.ipc.backup.046",
                        &[("v1", other)],
                    )))
                }
            };
            let mut forward = |tick: &Progress| send_tick(&on_tick, tick);
            let sum = crate::runtime::backup::restore(&internal, at, scope, &mut forward)?;
            tracing::info!(
                path = %sum.path,
                files = sum.files,
                scope = ?scope,
                "从备份恢复完了"
            );
            Ok(sum)
        })
    });
    task.await.map_err(|e| {
        AppError::internal(messages::text("ui.ipc.backup.041", &[])).with_detail(e.to_string())
    })?
}

/// 备份的**默认落点**：桌面（2026-11-09 作者定的）。
///
/// 与导出的默认目录不同：报告要落在 `<桌面>/MKP-导出报告/` 里（一批文件得有个自己的
/// 文件夹），而备份是**一个 zip**，作者要的就是"桌面上直接看得见那个包"。
///
/// 三道兜底，退到哪儿就在界面上照实显示哪儿（界面那串路径就是它算出来的）：
/// 1. 系统认得的桌面；
/// 2. 桌面在盘上还不存在 ⇒ 建出来；
/// 3. 连桌面都问不到（无桌面环境）⇒ 退回数据根下的 `exports/`。
#[tauri::command]
pub fn default_backup_dir(app: AppHandle) -> Result<String, AppError> {
    crate::ipc::traced("defaultBackupDir", |_| {
        let dir = default_export_dir(&app)?;
        std::fs::create_dir_all(&dir).map_err(|e| {
            AppError::io(messages::text(
                "ui.ipc.backup.044",
                &[("v1", &dir.display().to_string())],
            ))
            .with_detail(e.to_string())
        })?;
        Ok(dir.display().to_string())
    })
}

/// 导出类动作的**默认落点**：桌面 → 桌面问不到就退数据根 `exports/`。
///
/// 备份与「导出日志」共用这一处：两处各算一遍"桌面在哪"，迟早给出两个不同的落点。
/// **只算不建** —— 建目录失败时该说哪句话由各自的命令决定（备份与日志的说法不一样）。
pub(crate) fn default_export_dir(app: &AppHandle) -> Result<PathBuf, AppError> {
    let desktop = app.path().desktop_dir().ok().filter(|d| d.is_absolute());
    match desktop {
        Some(d) => Ok(d),
        None => Ok(crate::fsx::paths::user_root(app)?.join(crate::fsx::paths::EXPORTS_DIR)),
    }
}

/// **在系统文件管理器里选中那份备份**（备份框成功之后那颗「打开位置」）。
///
/// 闸门比 [`reveal_report_export`](super::report::reveal_report_export) 窄一档：
/// 那份 zip 的路径是 [`create_backup`] 自己算出来的、原样回给界面的，这里只是把它收回来
/// —— 所以只认"**绝对路径 + `.zip` + 盘上真有这一份**"，别的一律拒。
/// `reveal_item_in_dir` 只选中、不启动它（`open_path` 拿到可执行物是会启动的）。
#[tauri::command]
pub async fn reveal_backup(app: AppHandle, path: String) -> Result<(), AppError> {
    crate::ipc::traced("revealBackup", |_| {
        let at = PathBuf::from(path.trim());
        let zipped = at
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("zip"));
        if !at.is_absolute() || !zipped {
            return Err(AppError::invalid_argument(messages::text(
                "ui.ipc.backup.042",
                &[],
            )));
        }
        if !at.is_file() {
            return Err(AppError::not_found(messages::text(
                "ui.ipc.backup.043",
                &[("v1", &at.display().to_string())],
            )));
        }
        let opener = tauri_plugin_opener::OpenerExt::opener(&app);
        opener.reveal_item_in_dir(&at).map_err(|e| {
            AppError::io(messages::text("ui.ipc.backup.045", &[])).with_detail(e.to_string())
        })
    })
}
