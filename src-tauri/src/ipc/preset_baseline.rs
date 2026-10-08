//! baseline 的两条命令：**云端版本列表**与**「恢复默认」的基准值**。
//!
//! # 一、云端版本列表（`get_official_versions`）
//!
//! 云端表要回答的只有一句话：**「这个官方预设，我这一版下了没有」**。
//!
//! - 官方登记过哪几版 → [`runtime::versions`]（当前目录 + 版本链，按摘要去重）；
//! - 这一版下过没有 → [`runtime::baseline::baseline_exists`]（按摘要寻址，下过就有一份）。
//!
//! ★ **它不是"过时判定"**：这里不读用户预设的字节、不看血统，也**不会**在任何一行上
//! 说"你的那份该更新了"。用户自己那份是完整的、可继续使用的一份，系统不替他更新。
//!
//! 日期：已下载的那一版有**真值**（baseline 文件头的 `# release_time`）；
//! 没下载的只有目录代时间（`catalog.publishedAt`），界面按"目录发布时间"如实标注。
//!
//! # 二、「恢复默认」的基准值（`get_preset_defaults`）
//!
//! 参数页的「恢复默认值」原本退回**出厂值**（目录里的 `baseValue`）。这一版改成：
//! **以这份我的预设当初那版官方的默认值为准** —— 拿血统里的 `based_on_sha256` 去
//! [`runtime::baseline`] 取那一份，按注册表抽出参数值交给前端。
//!
//! 没有基准（导入的 / 从用户预设复制出来的 / 血统缺摘要）⇒ `values: null`，
//! 前端**回退到出厂值** —— 基准缺失是正常状态，不是错误。

use std::collections::BTreeMap;

use serde::Serialize;
use tauri::AppHandle;

use crate::error::AppError;
use crate::fsx::paths::{internal_root, user_root};
use crate::ipc::traced;
use crate::presetdata::params as param_alg;
use crate::runtime;

/// 官方预设的一个版本（云端表按预设归组之后，组内的每一行）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficialVersionDto {
    /// 归属的官方预设文件名（`A1-fast.toml`）—— 界面按它归组
    pub file_name: String,
    /// 内容摘要（版本的唯一身份，**界面不显示**）
    pub sha256: String,
    /// 官方**发布日期真值**（该版正文文件头的 `# release_time`）。
    /// `null` = 还没下载（拿不到真值，界面不编）
    pub release_time: Option<String>,
    /// 目录代时间（`catalog.publishedAt`）—— 兜底，界面上标明是"目录发布时间"
    pub published_at: Option<String>,
    /// 这一版本机下过没有（= baseline 里有它）
    pub downloaded: bool,
    /// 这一版是**当前目录登记的那一版** —— 只有它「下载」真能拿到（历史版本没法按摘要重下）
    pub current: bool,
}

/// 官方版本列表。`file_name` 给了就只返回那一个预设的版本。
#[tauri::command]
pub async fn get_official_versions(
    app: AppHandle,
    file_name: Option<String>,
) -> Result<Vec<OfficialVersionDto>, AppError> {
    let root = internal_root(&app)?;
    traced("getOfficialVersions", |_| {
        let wanted = file_name.as_deref().map(str::trim).filter(|s| !s.is_empty());
        let mut out = Vec::new();
        for v in runtime::versions::official_versions(&root) {
            if wanted.is_some_and(|w| w != v.file_name) {
                continue;
            }
            let downloaded = runtime::baseline::baseline_exists(&root, &v.sha256);
            /* 下载过的那一版才有发布日期真值（来自它自己的文件头） */
            let release_time = if downloaded {
                runtime::baseline::read_baseline(&root, &v.sha256)?
                    .and_then(|t| runtime::lineage::parse_release_time(&t))
            } else {
                None
            };
            out.push(OfficialVersionDto {
                file_name: v.file_name,
                sha256: v.sha256,
                release_time,
                published_at: v.published_at,
                downloaded,
                current: v.current,
            });
        }
        Ok(out)
    })
}

/// 「恢复默认」的基准值。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetDefaultsDto {
    /// 这份预设血统里的来源摘要（没血统 / 没摘要 ⇒ `null`）
    pub based_on_sha256: Option<String>,
    /// 基准参数值；`null` = 没有可用的基准，前端**回退出厂值**
    pub values: Option<BTreeMap<String, String>>,
}

/// 取一份用户预设的**官方基准默认值**（参数页「恢复默认值」的取值来源）。
///
/// 只读：读用户那份的**文件头**拿血统（不读整份 —— 用户可能扔进一个大文件），
/// 再按摘要取 baseline。**一个字节都不写**。
#[tauri::command]
pub async fn get_preset_defaults(
    app: AppHandle,
    path: String,
) -> Result<PresetDefaultsDto, AppError> {
    let root = internal_root(&app)?;
    let user = user_root(&app)?;
    let catalog = runtime::load_released_catalog(&root)?;
    traced("getPresetDefaults", |_| {
        let target = crate::fsx::paths::resolve_in(&user, path.trim_start_matches('/'))?;
        let sha = runtime::mine::lineage_of_file(&target).and_then(|l| l.based_on_sha256);
        let Some(sha) = sha else {
            /* 没有血统（导入的 / 手工拷的）—— 没有基准，前端回退出厂值 */
            return Ok(PresetDefaultsDto {
                based_on_sha256: None,
                values: None,
            });
        };
        let values = match runtime::baseline::read_baseline(&root, &sha)? {
            Some(text) => Some(param_alg::read_param_values(
                &text,
                &catalog.registry.params,
            )?),
            None => None,
        };
        Ok(PresetDefaultsDto {
            based_on_sha256: Some(sha),
            values,
        })
    })
}
