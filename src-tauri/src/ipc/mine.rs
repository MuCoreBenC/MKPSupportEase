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
use crate::fsx::paths::{internal_root, resolve, Root};
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

/// 编辑中的那一份（临时文件）给界面的形状。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetDraftDto {
    /// 从哪一份改出来的（下载区里的文件名）
    pub source_file_name: String,
    /// 正文：用户改到哪算哪
    pub text: String,
    /// 最后改动时刻（UTC epoch 秒）
    pub updated_unix: u64,
    /// 这次打开是**接着上次改**（草稿本来就是这一份的），不是新建的
    pub reused: bool,
}

/// 另存完成的结果（用户文件落在哪、多大、是不是盖掉了上一次那份）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommittedDraftDto {
    /// 相对**用户根**的路径（`presets-mine/A1-fast（已修改）.toml`）
    pub path: String,
    pub file_name: String,
    pub size: u64,
    /// 盖掉了一份同名的用户文件（第二次保存就是这种）
    pub replaced: bool,
}

/// **开始改一份官方交付预设**：把正文复制进临时文件（`run/draft-preset.json`），
/// 官方原件**一动不动**。
///
/// 这是"临时编辑"那条链的第一步（总纲 §1③「预设 TOML 的一生」）：
/// 用户改的永远是临时文件，点保存才另存进用户根；官方原件只有"云端换版本"能替换它。
///
/// 两条前置条件都是**前置**，不靠报错提示：
///   - 只改 MKP 预设（TOML）—— 其它资源不是这一层的对象；
///   - 盘上得真有那一份（没下载就没正文可改，先说"先去下载"）。
///
/// 已经有一份**同一来源**的草稿时：**接着改**（`reused: true`），不覆盖用户的改动。
#[tauri::command]
pub async fn begin_preset_edit(
    app: AppHandle,
    file_name: String,
) -> Result<PresetDraftDto, AppError> {
    traced("beginPresetEdit", |_| {
        let root = internal_root(&app)?;
        let catalog = runtime::load_released_catalog(&root)?;
        let file = catalog
            .files
            .iter()
            .find(|f| f.file_name == file_name)
            .ok_or_else(|| AppError::not_found(format!("目录里没有 {file_name} 这一份")))?;
        if file.kind != runtime::catalog::kind::PRESET {
            return Err(AppError::invalid_argument(format!(
                "{file_name} 不是 MKP 预设 —— 这一层只改 TOML 预设"
            )));
        }

        if let Some(draft) = runtime::state::load_draft(&root)? {
            if draft.source_file_name == file_name {
                return Ok(PresetDraftDto {
                    source_file_name: draft.source_file_name,
                    text: draft.text,
                    updated_unix: draft.updated_unix,
                    reused: true,
                });
            }
        }

        let bytes = std::fs::read(root.join(&file.path)).map_err(|_| {
            AppError::not_found(format!("{file_name} 还没下载到本机 —— 先下载，再改"))
        })?;
        let text = String::from_utf8(bytes)
            .map_err(|_| AppError::corrupted(format!("{file_name} 不是 UTF-8 文本，改不了")))?;
        let draft = runtime::state::save_draft(&root, &file.file_name, &file.sha256, &text)?;
        Ok(PresetDraftDto {
            source_file_name: draft.source_file_name,
            text: draft.text,
            updated_unix: draft.updated_unix,
            reused: false,
        })
    })
}

/// 把改动写进临时文件（界面边改边存）。**只动正文** —— 来源与那一刻的指纹保持不动。
#[tauri::command]
pub async fn put_preset_draft(app: AppHandle, text: String) -> Result<(), AppError> {
    traced("putPresetDraft", |_| {
        let root = internal_root(&app)?;
        let draft = runtime::state::load_draft(&root)?
            .ok_or_else(|| AppError::invalid_argument("现在没有正在改的那一份"))?;
        runtime::state::save_draft(&root, &draft.source_file_name, &draft.source_sha256, &text)?;
        Ok(())
    })
}

/// 放弃这次编辑：丢掉临时文件。
///
/// 这一步**天生安全**：官方原件与下载区全程没被碰过，所以"放弃"只是扔掉一份草稿
/// （而且是幂等的 —— 没有草稿时调它也不算错）。
#[tauri::command]
pub async fn discard_preset_draft(app: AppHandle) -> Result<(), AppError> {
    traced("discardPresetDraft", |_| {
        let root = internal_root(&app)?;
        runtime::state::clear_draft(&root)
    })
}

/// **把这一份另存成用户自己的文件**：`presets-mine/<原名>（已修改）<后缀>`，然后丢掉草稿。
///
/// 三件事都不做（这一层的边界）：不碰官方原件、不碰下载区、**不碰使用中指针**
/// （"生效"是另一条线）。再存一次就是**覆盖它自己** —— 用户改的是"我那份"，
/// 不该越存越多（`replaced` 说出来这次是不是盖掉了上一次那份）。
#[tauri::command]
pub async fn commit_preset_draft(app: AppHandle) -> Result<CommittedDraftDto, AppError> {
    traced("commitPresetDraft", |_| {
        let root = internal_root(&app)?;
        let user_root = crate::fsx::paths::user_root(&app)?;
        let draft = runtime::state::load_draft(&root)?
            .ok_or_else(|| AppError::invalid_argument("现在没有正在改的那一份，没得存"))?;

        /* 另存本体在 `runtime::mine`（纯函数、有判据盯着"官方原件一动不动"） */
        let done = runtime::mine::commit_draft(&user_root, &draft.source_file_name, &draft.text)?;
        /* 存完就该丢掉草稿：它会盖住下一次「改这份」的"接着上次改" */
        runtime::state::clear_draft(&root)?;

        Ok(CommittedDraftDto {
            path: done.path,
            file_name: done.file_name,
            size: done.size,
            replaced: done.replaced,
        })
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
