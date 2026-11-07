//! **发布对账**（只读）：`git tag` × 发布回执 × 当前开发号。
//!
//! # 为什么是"对账"而不是"从 git 重建历史"
//!
//! git 里有：版本号、冻结时间、提交。git 里**没有**：安装包大小 / SHA、PR 编号、
//! Release 地址、那一条回执的结论 —— 那些只存在于发布事务写下的时候。
//! 所以这里只回答"哪一版发过、哪一版有回执、缺的是哪一边"，**不伪造回执字段**。
//!
//! # 三个来源，一个口径
//!
//! - **tag** = 冻结的唯一证据（AGENTS §4.5）；
//! - **回执** = `release-history.json`（工作台那本 + 命令行那本，见
//!   [`super::release_history::load_merged`]）；
//! - **当前开发号** = `src-tauri/Cargo.toml` 的 `[package].version`（与
//!   `npm run version:check` 的「当前开发」同一个真值）。
//!
//! 对不上不是错误，是**要看见的事实**：`tagOnly` 说明那一版不是从这条入口发的；
//! `recordOnly` 说明记了账但仓库里没有对应的冻结点。

use std::collections::BTreeMap;

use serde::Serialize;

use crate::error::AppError;

/// 一行对账结论。
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LedgerStatus {
    /// tag 与回执都在
    Both,
    /// 只有 tag：发过，但没留下回执
    TagOnly,
    /// 只有回执：记了账，但仓库里没有这个 tag
    RecordOnly,
}

/// 对账表的一行。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerRow {
    /// 版本号（`0.0.10-preview.7`）
    pub version: String,
    /// 对应的 tag（对不上就是 `null`）
    pub tag: Option<String>,
    /// 排序与展示用的时间：有 tag 用 tag 日期，否则用回执时间
    pub at: String,
    /// tag 指向的提交短 sha
    pub commit: Option<String>,
    /// 最新一条回执的写入时刻
    pub receipt_at: Option<String>,
    /// 最新一条回执停在哪一阶段
    pub receipt_stage: Option<String>,
    pub status: LedgerStatus,
}

/// 整张对账表。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseLedger {
    /// 当前开发号
    pub developing: String,
    /// 当前开发号已经有同名 tag（= 已冻结）
    pub developing_frozen: bool,
    /// 最新 tag 的名字
    pub latest_tag: Option<String>,
    /// 回执总条数（同一版可能有好几条：补平台、补清单）
    pub receipts: usize,
    pub rows: Vec<LedgerRow>,
}

/// 把三个来源对齐。**纯计算**（git 与回执由调用方给），判据不依赖机器状态。
pub fn build(
    developing: String,
    tags: &[super::git::TagRow],
    receipts: &[super::release_history::ReleaseRecordView],
) -> ReleaseLedger {
    let mut rows: BTreeMap<String, LedgerRow> = BTreeMap::new();

    for t in tags {
        let version = t.tag.trim_start_matches('v').to_owned();
        rows.insert(
            version.clone(),
            LedgerRow {
                version,
                tag: Some(t.tag.clone()),
                at: t.at.clone(),
                commit: Some(t.commit.clone()),
                receipt_at: None,
                receipt_stage: None,
                status: LedgerStatus::TagOnly,
            },
        );
    }

    for r in receipts {
        let version = r.record.version.clone();
        let entry = rows.entry(version.clone()).or_insert_with(|| LedgerRow {
            version: version.clone(),
            tag: None,
            at: r.record.at.clone(),
            commit: None,
            receipt_at: None,
            receipt_stage: None,
            status: LedgerStatus::RecordOnly,
        });
        /* 同一版有多条回执：留**最新**那条的时间与阶段（回执本来是最新在前，
           但合并读之后不保证顺序，所以自己比一次） */
        #[allow(clippy::unnecessary_map_or)]
        if entry
            .receipt_at
            .as_deref()
            .map_or(true, |cur| r.record.at.as_str() > cur)
        {
            entry.receipt_at = Some(r.record.at.clone());
            entry.receipt_stage = Some(r.record.stage.clone());
        }
        entry.status = if entry.tag.is_some() {
            LedgerStatus::Both
        } else {
            LedgerStatus::RecordOnly
        };
    }

    let mut rows: Vec<LedgerRow> = rows.into_values().collect();
    rows.sort_by(|a, b| b.at.cmp(&a.at));

    let tag_of_dev = format!("v{developing}");
    ReleaseLedger {
        developing_frozen: tags.iter().any(|t| t.tag == tag_of_dev),
        latest_tag: tags.first().map(|t| t.tag.clone()),
        receipts: receipts.len(),
        developing,
        rows,
    }
}

/// **发布对账**（只读）：不写盘、不联网、不打 tag。
///
/// 回执走**多根合并**（工作台那本 + 命令行那本）—— 命令行发的版本同样算数。
#[tauri::command(async)]
pub fn wb_release_ledger(app: tauri::AppHandle) -> Result<ReleaseLedger, AppError> {
    crate::ipc::traced("wb_release_ledger", |_| {
        let repo = crate::workbench::paths::repo_root();
        let tags = super::git::Git::open(&repo).tags()?;
        let receipts = super::release_history::load_merged(&super::history_roots(&app)?)?;
        let developing = super::version::app_version(&repo)?;
        Ok(build(developing, &tags, &receipts.records))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::app::git::TagRow;
    use crate::workbench::app::release_history::{ReleaseRecord, ReleaseRecordView};

    fn receipt(tag: &str, at: &str) -> ReleaseRecordView {
        ReleaseRecordView {
            record: ReleaseRecord {
                at: at.to_owned(),
                stage: "tagged".to_owned(),
                version: tag.trim_start_matches('v').to_owned(),
                tag: tag.to_owned(),
                branch: None,
                commit: None,
                review: None,
                release: None,
                artifact: None,
                info_branch: None,
                info_review: None,
                summary: String::new(),
            },
            source: "workbench".to_owned(),
        }
    }

    fn tag(name: &str, at: &str) -> TagRow {
        TagRow {
            tag: name.to_owned(),
            at: at.to_owned(),
            commit: "abc1234".to_owned(),
        }
    }

    /// 三种状态各归各位：有 tag 无回执 / 有回执无 tag / 都有。
    #[test]
    fn the_ledger_names_the_missing_side() {
        let tags = vec![tag("v0.0.10-preview.7", "2026-11-06"), tag("v0.0.6", "2026-10-30")];
        let receipts = vec![
            receipt("v0.0.10-preview.2", "2026-11-06T06:12:58Z"),
            receipt("v0.0.6", "2026-10-30T10:00:00Z"),
        ];
        let out = build("0.0.10-preview.8".to_owned(), &tags, &receipts);

        assert_eq!(out.latest_tag.as_deref(), Some("v0.0.10-preview.7"));
        assert!(!out.developing_frozen, "8 还没有 tag");
        assert_eq!(out.receipts, 2, "回执条数照实说");

        let by = |v: &str| out.rows.iter().find(|r| r.version == v).cloned().unwrap();
        assert_eq!(by("0.0.10-preview.7").status, LedgerStatus::TagOnly);
        assert_eq!(by("0.0.10-preview.2").status, LedgerStatus::RecordOnly);
        assert_eq!(by("0.0.6").status, LedgerStatus::Both);
        assert_eq!(
            by("0.0.6").commit.as_deref(),
            Some("abc1234"),
            "有 tag 的行要带提交"
        );
    }

    /// 同一版多条回执（补平台 / 补清单）：留**最新**那条的时间与阶段；
    /// 没有 tag 时"开发号已冻结"必须是 false —— 冻结只认 tag。
    #[test]
    fn several_receipts_for_one_version_keep_the_latest() {
        let receipts = vec![
            receipt("v0.0.10-preview.1", "2026-11-05T17:53:46Z"),
            receipt("v0.0.10-preview.1", "2026-11-05T18:29:28Z"),
        ];
        let out = build("0.0.10-preview.1".to_owned(), &[], &receipts);
        assert_eq!(out.rows.len(), 1, "同一版只占一行");
        assert_eq!(
            out.rows[0].receipt_at.as_deref(),
            Some("2026-11-05T18:29:28Z"),
            "留最新那条回执"
        );
        assert!(!out.developing_frozen, "冻结只认 tag：没有 tag 就还没冻结");
    }
}
