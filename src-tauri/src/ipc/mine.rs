//! 用户线的两条读：**用户自己的预设文件**（`~/Documents/SupportEase/presets-mine/`）。
//!
//! 官方线的读在 [`crate::ipc::catalog`] 里（`mkp/` 下载区、`archive/` 归档区）；
//! 这一条是**另一条线**（总纲 §1③「预设 TOML 的一生」），两条不许混：
//! 官方原件不可变、用户修改另存、用户那份**永远不回写官方原件**。
//!
//! **只有读**：「临时编辑 → 保存 → 用户文件」是下一层的事。所以今天这两条在真机上
//! 多半返回空 —— **空是合法状态，不是错误**（用户一份都没另存过）。
//! 它在界面上就是本地表里那一半「我的文件」：看得见、认得出、看得了。

use serde::Serialize;
use tauri::AppHandle;

use crate::error::AppError;
use crate::fsx::paths::{resolve, Root};
use crate::ipc::traced;
use crate::runtime;

/// 用户自己的一份文件（给界面看的形状）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPresetFileDto {
    /// 相对**用户根**的路径（`presets-mine/A1-fast.toml`）—— 读正文时把它交回来
    pub path: String,
    pub file_name: String,
    pub size: u64,
    /// 最后改动时刻（UTC epoch 秒）。界面自己转人话：默认构建不引时间库
    pub modified_unix: Option<u64>,
    /// 认得出是哪一类就给；**认不出是 `null`**（见 [`runtime::mine::kind_of`]）。
    /// 界面上认不出的那一档**在任何类型档下都列** —— 不藏，也不替用户猜
    pub kind: Option<String>,
}

/// 用户自己有哪些文件（`presets-mine/` 里躺着什么）。
///
/// **盘就是底账**：扫盘，不记账本 —— 这一份的主人就是用户，他随时可能在 Finder 里改它。
/// 没有"官方身份"可说：云端没有它，所以没有 SHA、不属于任何版本、也不参与套餐。
#[tauri::command]
pub async fn get_user_preset_files(app: AppHandle) -> Result<Vec<UserPresetFileDto>, AppError> {
    traced("getUserPresetFiles", |_| {
        let root = crate::fsx::paths::user_root(&app)?;
        Ok(runtime::mine::mine_files(&root)
            .into_iter()
            .map(|f| UserPresetFileDto {
                path: f.path,
                file_name: f.file_name,
                size: f.size,
                modified_unix: f.modified_unix,
                kind: f.kind.map(str::to_owned),
            })
            .collect())
    })
}

/// 读用户自己那份的正文。
///
/// **只认 `presets-mine/`**：入参是 [`get_user_preset_files`] 给的那条路径，
/// 这里再核一次前缀（别越到 `exports/` `reports/` 去）并过防穿越。
/// 不是 UTF-8 就如实报错 —— 用户自己的文件也一样，读不出来就说读不出来。
#[tauri::command]
pub async fn read_user_preset_text(app: AppHandle, path: String) -> Result<String, AppError> {
    let task = tauri::async_runtime::spawn_blocking(move || {
        traced("readUserPresetText", |_| {
            let rel = path.trim_start_matches('/').to_owned();
            runtime::mine::check_mine_prefix(&rel)?;
            let target = resolve(&app, Root::User, &rel)?;
            let bytes = std::fs::read(&target).map_err(|_| {
                AppError::not_found(format!("找不到 {rel} —— 它可能已经被移走或删掉了"))
            })?;
            String::from_utf8(bytes)
                .map_err(|_| AppError::corrupted(format!("{rel} 不是 UTF-8 文本，这一条读不出来")))
        })
    });
    task.await
        .map_err(|e| AppError::internal("读用户文件没跑到终局").with_detail(e.to_string()))?
}
