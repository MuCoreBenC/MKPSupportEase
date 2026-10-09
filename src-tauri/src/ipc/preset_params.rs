//! 对比台的两条命令：**读**一份用户预设的参数 / **写回**改过的那几项。
//!
//! # 为什么单独两条
//!
//! 参数页那条链是「草稿」（`run/draft-preset.json` 里的整份正文 + 三层取值 + 撤销栈），
//! 一次只编一份。对比台要的是另一件事：**同时读 2~3 份**做对照，并把用户显式改过的
//! 那几项写回**各自那份**。混进草稿链会把"编辑目标是哪一份"这个问题重新引进来 ——
//! 而这一版的产品模型恰恰要把它去掉（每份都是独立的我的预设，谁都不覆盖谁）。
//!
//! # 边界
//!
//! - **只认 `presets-mine/`**（[`runtime::mine::read_text`] / [`runtime::mine::write_text`]
//!   的前缀 + 防穿越两道闸）：baseline 与官方当前版永远不被写；
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
use crate::presetdata::patch::FieldEdit;
use crate::runtime;

/// 一份用户预设的参数值（对比台里的一列）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetParamValuesDto {
    /// 相对用户根的路径（`presets-mine/A1-fast-2026-10-15.toml`）
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
        let text = match runtime::mine::read_text(&user, &path) {
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
/// 空改动直接返回（不写盘）；算出来与原文一样也**不写盘**（一次空操作不该刷新 mtime）。
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
    let user = user_root(&app)?;
    let catalog = runtime::load_released_catalog(&root)?;
    traced("savePresetParams", |_| {
        let text = runtime::mine::read_text(&user, &path)?;
        let field_edits: Vec<FieldEdit> = edits
            .iter()
            .map(|e| FieldEdit::new(e.param_key.clone(), e.value.clone()))
            .collect();
        let next = param_alg::apply_param_edits(&text, &catalog.registry.params, &field_edits)?;
        if next == text {
            return Ok(());
        }
        runtime::mine::write_text(&user, &path, &next)
    })
}
