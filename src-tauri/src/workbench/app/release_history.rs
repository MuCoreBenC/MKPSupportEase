//! **软件版本发布历史** —— 每一次「发布软件版本」事务留下的回执，
//! 住 `<appDataDir>/release-history.json`。
//!
//! # 为什么**另起一份**，不并进 `publish-history.json`
//!
//! 作者定死的两条链（见 `docs/RELEASE-TRANSACTIONS.md` §1）：
//! 「发布预设」与「发布软件版本」是**两套事务**，产物、阶段、语义全不一样。
//! 历史是"发生过什么"的账 —— 混在一本账里，迟早要按来源再拆一遍，那时界面上已经
//! 有用户按错的结果了。所以这里一份、那边一份，**各自只记自己那条链**。
//!
//! # 存储规矩（沿用 `history.rs` 那一套，不另发明）
//!
//! 一种状态一个文件、住 `<appDataDir>` 根、带 `*Schema` 代次字段、写走 `atomic_write`、
//! **坏档报 `CORRUPTED` 不静默**。缺文件 = 空历史（合法）。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::fsx::atomic::atomic_write;

/// 历史文件的固定名字（`<appDataDir>/release-history.json`）。
pub const HISTORY_FILE: &str = "release-history.json";

/// 格式代次。加字段不升号、改语义才升（与 source / catalog / account 同一条）。
pub const HISTORY_SCHEMA: u32 = 1;

/// 最多留多少条 —— 回执是"最近发生过什么"，不是审计账本。
pub const HISTORY_MAX: usize = 100;

/// 一条软件版本发布回执。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseRecord {
    /// 写入时刻（ISO8601）
    pub at: String,
    /// 停在 / 走到了哪一阶段（[`super::release_tx::ReleaseStage::wire_name`]）
    pub stage: String,
    /// 这一轮的版本号（`x.y.z`）
    pub version: String,
    /// tag（`v0.0.2`）
    pub tag: String,
    /// ① 版本号那一笔所在分支
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// ① 提交的短 sha
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    /// ① 的 PR / MR（**状态是写入那一刻的**）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review: Option<super::platform::RemoteReview>,
    /// 建好的 Release（网页地址 = 用户「查看更新」点开的那一页）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release: Option<super::platform::RemoteRelease>,
    /// 安装包（文件名 + 字节数）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<super::release_tx::ArtifactInfo>,
    /// ② `release.json` 那一笔的分支
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub info_branch: Option<String>,
    /// ② 的 PR / MR —— **它由人合并**，合并完客户端才看得到新版本
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub info_review: Option<super::platform::RemoteReview>,
    /// 一句话结论（与事务报告同一句）
    pub summary: String,
}

impl ReleaseRecord {
    /// 从一次事务报告造一条回执 —— **字段映射只写这一处**。
    pub fn from_report(report: &super::release_tx::ReleaseTxReport, at: String) -> Self {
        Self {
            at,
            stage: report.stage.wire_name().to_owned(),
            version: report.version.clone(),
            tag: report.tag.clone(),
            branch: report.branch.clone(),
            commit: report.commit.clone(),
            review: report.review.clone(),
            release: report.release.clone(),
            artifact: report.artifact.clone(),
            info_branch: report.info_branch.clone(),
            info_review: report.info_review.clone(),
            summary: report.summary.clone(),
        }
    }
}

/// 历史文件（`release-history.json` 的根）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseHistory {
    #[serde(default = "default_schema")]
    pub history_schema: u32,
    /// **最新在前**（写入时插到最前；读取顺序 = 界面显示顺序）
    #[serde(default)]
    pub records: Vec<ReleaseRecord>,
}

fn default_schema() -> u32 {
    HISTORY_SCHEMA
}

impl Default for ReleaseHistory {
    fn default() -> Self {
        Self {
            history_schema: HISTORY_SCHEMA,
            records: Vec::new(),
        }
    }
}

/// 历史文件路径（`<appDataDir>/release-history.json`）。
pub fn history_path(root: &Path) -> PathBuf {
    root.join(HISTORY_FILE)
}

/// 读历史。缺文件 = 空历史（合法）；坏 JSON / 代次认不出 = `CORRUPTED`（**不静默当"没发过"**）。
pub fn load(root: &Path) -> Result<ReleaseHistory, AppError> {
    let path = history_path(root);
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(ReleaseHistory::default()),
        Err(e) => return Err(AppError::io("读不了软件版本发布历史").with_detail(e.to_string())),
    };
    let history: ReleaseHistory = serde_json::from_slice(&bytes).map_err(|e| {
        AppError::corrupted("软件版本发布历史（release-history.json）解析不了")
            .with_detail(e.to_string())
    })?;
    if history.history_schema != HISTORY_SCHEMA {
        return Err(AppError::corrupted(format!(
            "软件版本发布历史的格式代次认不了：文件是 {}，程序认 {}",
            history.history_schema, HISTORY_SCHEMA
        )));
    }
    Ok(history)
}

/// 追加一条（**插到最前**），超出 [`HISTORY_MAX`] 的从尾部丢掉。
pub fn append(root: &Path, record: ReleaseRecord) -> Result<ReleaseHistory, AppError> {
    let mut history = load(root)?;
    history.records.insert(0, record);
    history.records.truncate(HISTORY_MAX);
    let bytes = serde_json::to_vec_pretty(&history)
        .map_err(|e| AppError::internal("软件版本发布历史序列化失败").with_detail(e.to_string()))?;
    atomic_write(&history_path(root), &bytes)?;
    Ok(history)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(tag: &str) -> ReleaseRecord {
        ReleaseRecord {
            at: "2026-10-04T00:00:00Z".to_owned(),
            stage: "tagged".to_owned(),
            version: tag.trim_start_matches('v').to_owned(),
            tag: tag.to_owned(),
            branch: Some("feat/x".to_owned()),
            commit: Some("abc1234".to_owned()),
            review: None,
            release: None,
            artifact: None,
            info_branch: None,
            info_review: None,
            summary: "测试".to_owned(),
        }
    }

    /// 缺文件 = 空历史（合法），不是错误。
    #[test]
    fn a_missing_file_is_an_empty_history() {
        let dir = tempfile::tempdir().unwrap();
        let h = load(dir.path()).expect("缺文件该是空历史");
        assert!(h.records.is_empty());
        assert_eq!(h.history_schema, HISTORY_SCHEMA);
    }

    /// 坏 JSON / 未来代次 → `CORRUPTED`，**不静默当"没发过"**。
    #[test]
    fn broken_history_is_corrupted_not_silently_empty() {
        let dir = tempfile::tempdir().unwrap();
        crate::fsx::atomic::atomic_write(&history_path(dir.path()), b"not json").unwrap();
        assert_eq!(
            load(dir.path()).unwrap_err().code,
            crate::error::ErrorCode::Corrupted
        );
        crate::fsx::atomic::atomic_write(
            &history_path(dir.path()),
            b"{\"historySchema\":99,\"records\":[]}",
        )
        .unwrap();
        assert_eq!(
            load(dir.path()).unwrap_err().code,
            crate::error::ErrorCode::Corrupted,
            "认不出的代次不许当成空账"
        );
    }

    /// 最新在前 + 超量截断（回执是"最近发生过什么"）。
    #[test]
    fn recent_records_come_first_and_the_oldest_are_dropped() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..HISTORY_MAX + 5 {
            append(dir.path(), record(&format!("v0.0.{i}"))).unwrap();
        }
        let h = load(dir.path()).unwrap();
        assert_eq!(h.records.len(), HISTORY_MAX);
        assert_eq!(h.records[0].tag, format!("v0.0.{}", HISTORY_MAX + 4));
    }

    /// ★ **两条链的账是两本**：`release-history.json` 与 `publish-history.json`
    /// 是不同的文件 —— 发预设不会在这里留痕，反之亦然。
    #[test]
    fn the_two_chains_keep_separate_books() {
        assert_ne!(HISTORY_FILE, super::super::history::HISTORY_FILE);
        let dir = tempfile::tempdir().unwrap();
        append(dir.path(), record("v0.0.2")).unwrap();
        assert!(
            !history_path(dir.path()).ends_with(super::super::history::HISTORY_FILE),
            "不该写到发布预设那本账上"
        );
    }
}
