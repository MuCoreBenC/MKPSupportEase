//! 回收站的 IPC：列表 / 恢复 / 永久删 / 清空。
//!
//! 恢复**不自动设为使用中** —— 用户要的是"找回来"，不是"替我切过去"。

use serde::Serialize;
use tauri::AppHandle;

use crate::error::AppError;
use crate::fsx::paths::{internal_root, user_root};
use crate::ipc::traced;
use crate::runtime::recovery::{self, RecoveryEntry};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryEntryDto {
    pub id: String,
    pub path: String,
    pub file_name: String,
    pub deleted_at: u64,
    pub has_origin: bool,
    /// 删除时对应的云端文件名（没有记录就是 `null`）
    pub cloud_file: Option<String>,
}

fn dto(entry: RecoveryEntry) -> RecoveryEntryDto {
    RecoveryEntryDto {
        id: entry.id,
        path: entry.path,
        file_name: entry.file_name,
        deleted_at: entry.deleted_at,
        has_origin: entry.has_origin,
        cloud_file: entry.provenance.and_then(|p| p.cloud_file),
    }
}

#[tauri::command]
pub async fn list_recovery(app: AppHandle) -> Result<Vec<RecoveryEntryDto>, AppError> {
    traced("listRecovery", |_| {
        let root = internal_root(&app)?;
        Ok(recovery::list(&root).into_iter().map(dto).collect())
    })
}

/// 恢复一条，返回恢复后的相对路径（界面据此定位/提示）。
#[tauri::command]
pub async fn restore_local_preset(app: AppHandle, id: String) -> Result<String, AppError> {
    traced("restoreLocalPreset", |_| {
        let root = internal_root(&app)?;
        let user = user_root(&app)?;
        let entry = recovery::restore(&root, &user, &id)?;
        Ok(entry.path)
    })
}

#[tauri::command]
pub async fn purge_recovery(app: AppHandle, id: String) -> Result<(), AppError> {
    traced("purgeRecovery", |_| {
        let root = internal_root(&app)?;
        recovery::purge(&root, &id)
    })
}

#[tauri::command]
pub async fn empty_recovery(app: AppHandle) -> Result<usize, AppError> {
    traced("emptyRecovery", |_| {
        let root = internal_root(&app)?;
        recovery::empty(&root)
    })
}
