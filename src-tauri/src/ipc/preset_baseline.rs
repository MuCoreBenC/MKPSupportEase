//! baseline 的命令面：**官方版本账**（`get_official_versions`）。
//!
//! ★ 2026-10-08 资源库改判：原先的第二条命令（`get_preset_defaults`，「恢复默认值」
//! 的基准值）**退役** —— 那套"按血统找当初那版官方默认值"的能力随「恢复默认值」
//! 功能一起退场（用户要的就是"有问题删掉重来一份"）。baseline 本身仍由下载管道写入，
//! 留给下一轮和切片器档一起收口。
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
//! # 二、「恢复默认」的基准值 —— **已退役**（2026-10-08 资源库改判）
//!
//! 参数页的「恢复默认值」不再以"当初那版官方的默认值"为基准 —— 这个功能整个退场
//! （用户要的是"有问题删掉重来一份"）。命令、类型与它的读取都随之下线。

use serde::Serialize;
use tauri::AppHandle;

use crate::error::AppError;
use crate::fsx::paths::internal_root;
use crate::ipc::traced;
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

