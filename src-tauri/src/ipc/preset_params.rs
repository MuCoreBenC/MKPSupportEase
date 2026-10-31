//! 对比台的两条命令：**读**一份用户预设的参数 / **写回**改过的那几项。
//!
//! # 为什么单独两条
//!
//! 参数页那条链是「草稿」（`run/draft-preset.json` 里的整份正文 + 三层取值 + 撤销栈），
//! 一次只编一份。对比台要的是另一件事：**同时读 2~3 份**做对照，并把用户显式改过的
//! 那几项写回**各自那份**。混进草稿链会把"编辑目标是哪一份"这个问题重新引进来 ——
//! 而这一版的产品模型恰恰要把它去掉（每份都是独立的本地预设，谁都不覆盖谁）。
//!
//! # 边界
//!
//! - **只认 `presets-local/`**（[`runtime::local::read_text`] / [`runtime::local::write_text`]
//!   的前缀 + 防穿越两道闸）：baseline 与云端当前版永远不被写；
//! - **结构保真**：一切改动逐项走 [`crate::presetdata::patch::patch_preset_toml`] ——
//!   注释 / 键序 / inline table / 多行字面量 / 行尾一个字节不动；任何一项失败整批不落；
//! - **不碰任何状态**：不改使用中指针、不建草稿、不进 archive。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::error::AppError;
use crate::fsx::paths::{internal_root, user_root};
use crate::ipc::traced;
use crate::presetdata::params as param_alg;
use crate::runtime;
use postprocess::diag::messages;

/// 一份用户预设的参数值（对比台里的一列）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetParamValuesDto {
    /// 相对用户根的路径（`presets-local/A1-fast-2026-10-15.toml`）
    pub path: String,
    pub file_name: String,
    /// 参数 key → **控件看得懂的值**（`-1.5` / `true` / `standard` / 多行 G-code）。
    /// 读不出来时是空表，`problem` 会说话
    pub values: BTreeMap<String, String>,
    /// 这一份读不出来时的一句人话 —— 对比台那一列如实说，**不给会报错的控件**
    pub problem: Option<String>,
}

/// 一次参数改动（参数 key + 目标值的字符串形式；形态由注册表决定，前端不拼 TOML）
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParamEditDto {
    pub param_key: String,
    pub value: String,
}

/// 读一份用户预设的参数。
///
/// **读不出来不报错**：返回空表 + `problem` 一句人话 —— 对比台的一列要能如实说
/// "这份读不出来"，而不是把整个对话框打掉。
#[tauri::command]
pub async fn read_preset_params(
    app: AppHandle,
    path: String,
) -> Result<PresetParamValuesDto, AppError> {
    let user = user_root(&app)?;
    let catalog = runtime::load_released_catalog(&internal_root(&app)?)?;
    let file_name = path.rsplit('/').next().unwrap_or(&path).to_owned();
    traced("readPresetParams", |_| {
        let text = match runtime::local::read_text(&user, &path) {
            Ok(t) => t,
            Err(e) => {
                return Ok(PresetParamValuesDto {
                    path,
                    file_name,
                    values: BTreeMap::new(),
                    problem: Some(e.message),
                })
            }
        };
        match param_alg::read_param_values(&text, &catalog.registry.params) {
            Ok(values) => Ok(PresetParamValuesDto {
                path,
                file_name,
                values,
                problem: None,
            }),
            Err(e) => Ok(PresetParamValuesDto {
                path,
                file_name,
                values: BTreeMap::new(),
                problem: Some(e.message),
            }),
        }
    })
}

/// 把一批改过的参数写回**它自己那一份**（同一路径，不产生第二份）。
///
/// # 2026-10-31：这里**不再直接写盘**
///
/// 对比台原来绕过草稿，`runtime::local::write_text` 一按保存就落盘 —— 同一份文件
/// 于是有两条写路径（参数页走草稿、对比台直写），谁先谁后都能把对方的改动冲掉。
///
/// 现在改成**走同一条草稿链**：`begin_preset_edit` → `patch_preset_draft_batch` →
/// `commit_preset_draft`。也就是：
///
///   · 保存前不碰正式文件（改的是 `run/draft-preset.json`）；
///   · 「保存」= 写盘边界，与参数页那颗按钮同一套语义；
///   · 编辑器里已有的草稿会**被复用**（`reused`），两处看到的是同一份未保存改动；
///   · 若另一份预设正开着草稿 ⇒ **拒绝**（不能悄悄把它的改动丢掉或一起提交）。
#[tauri::command]
pub async fn save_preset_params(
    app: AppHandle,
    path: String,
    edits: Vec<ParamEditDto>,
) -> Result<(), AppError> {
    if edits.is_empty() {
        return Ok(());
    }
    let root = internal_root(&app)?;
    let file_name = path
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::invalid_argument(messages::text("ui.ipc.local.150", &[])))?
        .to_owned();

    /*
     * 先看有没有**另一份**的草稿开着：有就拒绝。
     * 直接 begin 会把那份草稿换掉（`begin_preset_edit` 的实现就是"不是同一份就重开"），
     * 用户会在完全不知情的情况下丢掉另一份的未保存改动。
     */
    if let Some(existing) = runtime::app_state::draft(&root)? {
        let same = existing.origin == crate::runtime::state::ActiveOrigin::Local
            && existing.path.as_deref() == Some(path.as_str());
        if !same {
            return Err(AppError::invalid_argument(messages::text(
                "ui.ipc.local.151",
                &[],
            )));
        }
    }

    let _ = super::local::begin_preset_edit(
        app.clone(),
        file_name,
        Some(crate::runtime::state::ActiveOrigin::Local),
        Some(path.clone()),
    )
    .await?;
    super::local::patch_preset_draft_batch(app.clone(), edits).await?;
    let _ = super::local::commit_preset_draft(app.clone()).await?;
    Ok(())
}
