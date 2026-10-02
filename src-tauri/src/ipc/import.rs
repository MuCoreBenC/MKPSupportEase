//! **通用文件导入入口的两条命令**（第十二层）：看落点（`stage_import`）与提交（`commit_import`）。
//!
//! 入口本身是**通用**的 —— 拖拽（App 层）与文件选择器（前端 `pickImportFiles`）都走这里，
//! Preset 只是第一个消费者；核心与判据在 [`crate::runtime::import`]。
//!
//! **两条命令都不碰任何状态**：不改使用中指针、不迁移 / 不创建草稿、不进 archive；
//! 落点固定 `presets-mine/`，安全检查走现有用户文件边界（不新造第二套路径规则）。

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::error::AppError;
use crate::ipc::traced;
use crate::runtime;

/// 一份外部文件的落点检查结果。`state`：`ready` / `collision` / `rejected`
/// （`rejected` 带 `reason`，界面原话显示）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StagedImportDto {
    pub source: String,
    pub file_name: String,
    pub state: String,
    pub reason: Option<String>,
}

/// 提交导入的一份：`newName` 只在"重名、用户改了名"时给（复用改名那套名字门槛）
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportItemDto {
    pub source: String,
    #[serde(default)]
    pub new_name: Option<String>,
}

/// 逐份结局（与下载同一副规矩：一份出错不拖累别人）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOutcomeDto {
    pub source: String,
    pub ok: bool,
    pub path: String,
    pub file_name: String,
    pub message: String,
}

/// **第一段：看落点**。只检查，不动盘。重名只如实说，不自动改名。
#[tauri::command]
pub async fn stage_import(
    app: AppHandle,
    sources: Vec<String>,
) -> Result<Vec<StagedImportDto>, AppError> {
    traced("stageImport", |_| {
        let user_root = crate::fsx::paths::user_root(&app)?;
        Ok(runtime::import::stage(&user_root, &sources)
            .into_iter()
            .map(|staged| {
                let (state, reason) = match staged.state {
                    runtime::import::StageState::Ready => ("ready", None),
                    runtime::import::StageState::Collision => ("collision", None),
                    runtime::import::StageState::Rejected(reason) => ("rejected", Some(reason)),
                };
                StagedImportDto {
                    source: staged.source,
                    file_name: staged.file_name,
                    state: state.to_owned(),
                    reason,
                }
            })
            .collect())
    })
}

/// **第二段：真的复制**。逐份独立；源文件只读；不覆盖；内容按字节复制、不校验 TOML。
#[tauri::command]
pub async fn commit_import(
    app: AppHandle,
    items: Vec<ImportItemDto>,
) -> Result<Vec<ImportOutcomeDto>, AppError> {
    traced("commitImport", |_| {
        let user_root = crate::fsx::paths::user_root(&app)?;
        let items: Vec<runtime::import::ImportItem> = items
            .into_iter()
            .map(|item| runtime::import::ImportItem {
                source: item.source,
                new_name: item.new_name,
            })
            .collect();
        Ok(runtime::import::commit(&user_root, &items)
            .into_iter()
            .map(|outcome| ImportOutcomeDto {
                source: outcome.source,
                ok: outcome.ok,
                path: outcome.path,
                file_name: outcome.file_name,
                message: outcome.message,
            })
            .collect())
    })
}
