//! **「哪份预设用得最多」的一条读命令**（2026-10-17）。
//!
//! 账本本身住 [`crate::runtime::usage`]（`run/usage.json`，**不进备份**）；
//! 写入不在这里 —— 每次"应用预设"成功之后由 [`crate::ipc::catalog::apply_active_preset`]
//! 顺手 +1，所以这里只有一条读。
//!
//! 界面拿它做什么：选择器抽屉里**同一版本的行**按"用得多"排前面
//! （只影响顺序，不改"用哪一份"的结果 —— 那件事仍由 `local_copy_for` 的三段判据定）。

use serde::{Deserialize, Serialize};

use tauri::AppHandle;

use crate::error::AppError;
use crate::runtime::usage;

/// 一份预设的用量（与前端 `PresetUsageEntry` 同一套字面量）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetUsageEntryDto {
    /// `local:<用户根相对路径>` / `cloud:<内部根相对路径>`
    pub key: String,
    pub count: u64,
    pub last_unix: i64,
}

/// 整本账。**没学过 = 空数组**（界面按默认顺序排，不是错误）
#[tauri::command]
pub fn get_preset_usage(app: AppHandle) -> Result<Vec<PresetUsageEntryDto>, AppError> {
    crate::ipc::traced("getPresetUsage", |_| {
        let root = crate::fsx::paths::internal_root(&app)?;
        Ok(usage::list(&root)
            .into_iter()
            .map(|(key, e)| PresetUsageEntryDto {
                key,
                count: e.count,
                last_unix: e.last_unix,
            })
            .collect())
    })
}
