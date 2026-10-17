//! **备份的两条命令**（2026-10-17）：打一份、恢复一份。
//!
//! 界面在**设置页**：用户挑一个文件夹 → 打一份 zip；挑一份 zip → 全量恢复。
//! 路径由系统文件选择框给（前端那侧），后端只认"用户点名的那个位置" ——
//! 不发明默认目录，也不替用户决定放哪。
//!
//! 恢复是**不可撤销**的动作（它会把现在的数据换掉），所以：
//!
//! - 二次确认在**界面**那侧做（与「清空历史」同一条规矩：危险动作都要问一句）；
//! - 后端这一步只做一件事：**照做**，做完把"恢复到哪一份、多少个文件"回给界面。

use tauri::AppHandle;

use crate::error::AppError;
use crate::runtime::backup::BackupSummary;

/// **打一份备份**：把这台机器的家当收进 zip，落在用户点名的那个文件夹里。
///
/// 返回**备份文件的绝对路径** —— 界面照原样显示（用户要知道东西落在哪了）。
#[tauri::command]
pub fn create_backup(app: AppHandle, dest_dir: String) -> Result<BackupSummary, AppError> {
    crate::ipc::traced("createBackup", |_| {
        let internal = crate::fsx::paths::internal_root(&app)?;
        let dest = std::path::Path::new(dest_dir.trim());
        if dest_dir.trim().is_empty() {
            return Err(AppError::invalid_argument("没说备份放哪 —— 先挑一个文件夹"));
        }
        if !dest.is_dir() {
            return Err(AppError::not_found(format!(
                "那个文件夹不在了（{}）—— 重新挑一个",
                dest.display()
            )));
        }
        let sum = crate::runtime::backup::create(&internal, dest)?;
        tracing::info!(
            path = %sum.path,
            files = sum.files,
            bytes = sum.bytes,
            "备份打好了"
        );
        Ok(sum)
    })
}

/// **从一份备份恢复**（全量覆盖：备份里有的一律覆盖，被管的几棵树里备份没有的删掉）。
///
/// 恢复完界面要**整窗重载**（数据在盘上换过了，内存里的东西都不算数）——
/// 这一步由前端做，这里只回"恢复到哪一份、多少个文件"。
#[tauri::command]
pub fn restore_backup(app: AppHandle, zip_path: String) -> Result<BackupSummary, AppError> {
    crate::ipc::traced("restoreBackup", |_| {
        let internal = crate::fsx::paths::internal_root(&app)?;
        let at = std::path::Path::new(zip_path.trim());
        if zip_path.trim().is_empty() || !at.is_file() {
            return Err(AppError::not_found("那份备份不在了 —— 重新挑一份"));
        }
        let sum = crate::runtime::backup::restore(&internal, at)?;
        tracing::info!(
            path = %sum.path,
            files = sum.files,
            "从备份恢复完了（全量覆盖）"
        );
        Ok(sum)
    })
}
