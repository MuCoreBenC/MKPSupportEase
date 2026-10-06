//! **发布历史** —— 每一次「发布预设」事务留下的回执，住 `<appDataDir>/publish-history.json`。
//!
//! # 为什么要有它（作者 2026-10-04）
//!
//! "我点了发布之后，关掉模态框再打开又是新的" —— 工作台得记住自己刚才干了什么、
//! 走到哪、结果如何。历史 = **回执日志**：一次事务一条，最新在前，最多 [`HISTORY_MAX`] 条。
//!
//! # 存储规矩（沿用 `account.rs` 那一套，不另发明）
//!
//! 一种状态一个文件、住 `<appDataDir>` 根、带 `*Schema` 代次字段、写走 `atomic_write`、
//! **坏档报 `CORRUPTED` 不静默**。缺文件 = 空历史（合法）。
//!
//! ★ 它**不是**发布配置（那是 `publish-account.json`）：这里只有"发生过什么"，
//! 没有秘密、没有可编辑项。删掉它不影响发布能力，只丢展示。
//!
//! # 状态是快照，刷新是手动（不做轮询）
//!
//! 记录里的 PR/MR 状态是**写入那一刻**的；要看现在走到哪，由界面拿编号去
//! `wb_publish_status` **手动刷新**（作者定死：状态是"看一看"，不是常驻任务）。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::fsx::atomic::atomic_write;

/// 历史文件的固定名字（`<appDataDir>/publish-history.json`）。
pub const HISTORY_FILE: &str = "publish-history.json";

/// 格式代次。加字段不升号、改语义才升（与 source / catalog / account 同一条）。
pub const HISTORY_SCHEMA: u32 = 1;

/// 最多留多少条 —— 回执是"最近发生过什么"，不是审计账本。
pub const HISTORY_MAX: usize = 100;

/// 一条发布回执。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublishRecord {
    /// 写入时刻（ISO8601）
    pub at: String,
    /// 停在 / 走到了哪一阶段（[`super::publish_tx::PublishStage::wire_name`]）
    pub stage: String,
    /// 发布时所在分支（没走到 git 那一步 = `None`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// 这次发布提交的短 sha（没有提交 = `None`，如实说）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    /// PR / MR 快照（建了才有；**状态是写入那一刻的**，刷新是另一件事）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review: Option<super::platform::RemoteReview>,
    /// 交付根产物份数
    pub files: usize,
    /// 本轮新生成份数
    pub generated: usize,
    /// 审计十五项里 Pass / Fail 的计数（Warn 不算 Fail）
    pub audit_passed: usize,
    pub audit_failed: usize,
    /// 一句话结论（与事务报告同一句）
    pub summary: String,
    /// **合并之后把主线同步到第二个官方源**的结论（2026-10-06 加）。
    ///
    /// `None` = 这一条记下来的时候还没走到合并那一步（或还没做这一步）。
    /// ★ 它不由"刷新"改写：刷新拿到的是**平台上的 PR 状态**，与"镜像跟没跟上"
    /// 是两件事 —— 刷一次把这条抹掉就等于把"客户端为什么还看不到"的证据丢了。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mirror: Option<super::publish_tx::MirrorSync>,
}

impl PublishRecord {
    /// 从一次事务报告造一条回执 —— **字段映射只写这一处**。
    pub fn from_report(report: &super::publish_tx::PublishTxReport, at: String) -> Self {
        Self {
            at,
            stage: report.stage.wire_name().to_owned(),
            branch: report.branch.clone(),
            commit: report.commit.clone(),
            review: report.review.clone(),
            files: report.files,
            generated: report.generated,
            audit_passed: report.audit_passed,
            audit_failed: report.audit_failed,
            summary: report.summary.clone(),
            /* 发布事务本身**不推镜像** —— 那一步发生在 PR/MR **合并之后**
            （`wb_merge_review`），由 `update_review` 把结论补进这一格。 */
            mirror: None,
        }
    }
}

/// 历史文件（`publish-history.json` 的根）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublishHistory {
    #[serde(default = "default_schema")]
    pub history_schema: u32,
    /// **最新在前**（写入时插到最前；读取顺序 = 界面显示顺序）
    #[serde(default)]
    pub records: Vec<PublishRecord>,
}

fn default_schema() -> u32 {
    HISTORY_SCHEMA
}

impl Default for PublishHistory {
    fn default() -> Self {
        Self {
            history_schema: HISTORY_SCHEMA,
            records: Vec::new(),
        }
    }
}

/// 历史文件路径（`<appDataDir>/publish-history.json`）。
pub fn history_path(root: &Path) -> PathBuf {
    root.join(HISTORY_FILE)
}

/// 读历史。缺文件 = 空历史（合法）；坏 JSON / 代次认不出 = `CORRUPTED`（**不静默当"没发过"**）。
pub fn load(root: &Path) -> Result<PublishHistory, AppError> {
    let path = history_path(root);
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(PublishHistory::default()),
        Err(e) => {
            return Err(AppError::io("读不了发布历史").with_detail(e.to_string()));
        }
    };
    let history: PublishHistory = serde_json::from_slice(&bytes).map_err(|e| {
        AppError::corrupted("发布历史（publish-history.json）解析不了").with_detail(e.to_string())
    })?;
    if history.history_schema != HISTORY_SCHEMA {
        return Err(AppError::corrupted(format!(
            "发布历史的格式代次认不了：文件是 {}，程序认 {}",
            history.history_schema, HISTORY_SCHEMA
        )));
    }
    Ok(history)
}

/// 写历史（`atomic_write`，与全仓写盘同一条纪律）。
pub fn save(root: &Path, history: &PublishHistory) -> Result<(), AppError> {
    let path = history_path(root);
    let mut text = serde_json::to_string_pretty(history)
        .map_err(|e| AppError::internal("发布历史序列化不了").with_detail(e.to_string()))?;
    text.push('\n');
    atomic_write(&path, text.as_bytes())
}

/// 追加一条回执（**最新在前**；超 [`HISTORY_MAX`] 条从尾部截断）。
///
/// 返回写入后的整份历史 —— 界面可以顺手拿来显示，不必再读一遍。
pub fn append(root: &Path, record: PublishRecord) -> Result<PublishHistory, AppError> {
    let mut history = load(root)?;
    history.records.insert(0, record);
    history.records.truncate(HISTORY_MAX);
    save(root, &history)?;
    Ok(history)
}

/// 把某份 PR/MR 的**新状态**写回历史里对应那几条（同一个 platform + number）。
///
/// 什么时候用：**合并成功之后**。那是我们亲手造成的状态变化，记账不需要问网络
/// （问网络的是 `wb_publish_status` 那条手动刷新）。别的操作不动历史 —— 快照就让它快照。
///
/// `mirror` = 合并之后"把主线同步到第二个官方源"的结论（2026-10-06 加）。
/// ★ **只填不清**：传 `None` 时保留记录里已有的那一条 —— 刷新拿到的是平台上的 PR 状态，
/// 与"镜像跟没跟上"是两件事，刷一次把后者抹掉就等于把"客户端为什么还看不到"的证据丢了。
///
/// 返回改了几条（0 条 = 历史里没有这份 PR，正常）。
pub fn update_review(
    root: &Path,
    review: &super::platform::RemoteReview,
    mirror: Option<&super::publish_tx::MirrorSync>,
) -> Result<usize, AppError> {
    let mut history = load(root)?;
    let mut hit = 0usize;
    for r in &mut history.records {
        let same = r
            .review
            .as_ref()
            .is_some_and(|old| old.platform == review.platform && old.number == review.number);
        if same {
            r.review = Some(review.clone());
            if let Some(m) = mirror {
                r.mirror = Some(m.clone());
            }
            hit += 1;
        }
    }
    if hit > 0 {
        save(root, &history)?;
    }
    Ok(hit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::app::platform::{ChecksSummary, RemoteReview, ReviewState};

    fn rec(at: &str) -> PublishRecord {
        PublishRecord {
            at: at.to_owned(),
            stage: "statusRead".to_owned(),
            branch: Some("feat/x".to_owned()),
            commit: Some("08ec040".to_owned()),
            review: Some(RemoteReview {
                platform: "github".to_owned(),
                number: 27,
                url: "https://github.com/o/r/pull/27".to_owned(),
                state: ReviewState::Open,
                checks: ChecksSummary::Pending,
                title: "发布：17 份产物".to_owned(),
                head: "feat/x".to_owned(),
                base: "main".to_owned(),
            }),
            files: 17,
            generated: 0,
            audit_passed: 14,
            audit_failed: 0,
            summary: "已生成 0 份、定稿 17 份产物。已提交并推送到 `feat/x`。已建 PR !27。"
                .to_owned(),
            mirror: None,
        }
    }

    /// 存取往返；缺文件 = 空历史（合法）。
    #[test]
    fn history_roundtrips_and_a_missing_file_is_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(load(d.path()).unwrap().records.is_empty(), "缺文件 = 空");

        let written = append(d.path(), rec("2026-10-04T14:02:00+08:00")).unwrap();
        assert_eq!(written.records.len(), 1);
        let back = load(d.path()).unwrap();
        assert_eq!(back, written, "写进去再读回来必须一样");
        assert_eq!(back.history_schema, HISTORY_SCHEMA);
    }

    /// **最新在前**；超上限从尾部截断（回执是"最近"，不是账本）。
    #[test]
    fn the_newest_record_is_first_and_the_tail_is_dropped() {
        let d = tempfile::tempdir().unwrap();
        // 先铺满上限，再追加一条 —— 最老的应该被挤掉
        let mut full = PublishHistory::default();
        for i in 0..HISTORY_MAX {
            full.records.push(rec(&format!("旧 {i}")));
        }
        save(d.path(), &full).unwrap();

        let after = append(d.path(), rec("最新一条")).unwrap();
        assert_eq!(after.records.len(), HISTORY_MAX, "上限保持不变");
        assert_eq!(after.records[0].at, "最新一条", "新的在最前");
        assert_eq!(
            after.records.last().unwrap().at,
            format!("旧 {}", HISTORY_MAX - 2),
            "最老的一条被挤出"
        );
    }

    /// 坏档报 `CORRUPTED` —— **不静默当"没发过"**。
    #[test]
    fn a_corrupted_history_is_reported_not_silently_emptied() {
        let d = tempfile::tempdir().unwrap();
        atomic_write(&history_path(d.path()), b"{ not json").unwrap();
        assert_eq!(
            load(d.path()).unwrap_err().code,
            crate::error::ErrorCode::Corrupted
        );

        // 代次认不出同样报错（将来改了格式，旧程序别装作读懂了）
        atomic_write(
            &history_path(d.path()),
            br#"{"historySchema":999,"records":[]}"#,
        )
        .unwrap();
        assert_eq!(
            load(d.path()).unwrap_err().code,
            crate::error::ErrorCode::Corrupted
        );
    }

    /// 合并之后把新状态写回历史：**只动同一个 platform + number 的那几条**。
    #[test]
    fn a_merge_writes_the_new_state_back_to_the_history() {
        let d = tempfile::tempdir().unwrap();
        append(d.path(), rec("第一条 #27")).unwrap();
        let mut other = rec("另一份 #9");
        other.review.as_mut().unwrap().number = 9;
        append(d.path(), other).unwrap();

        let mut merged = rec("x").review.unwrap();
        merged.state = ReviewState::Merged;
        assert_eq!(
            update_review(d.path(), &merged, None).unwrap(),
            1,
            "只该命中 #27 那条"
        );

        let back = load(d.path()).unwrap();
        // 按编号找（顺序是"最新在前"，追加了两次 —— 别按位置写断言）
        let state_of = |n: u64| {
            back.records
                .iter()
                .find_map(|r| {
                    let rv = r.review.as_ref()?;
                    (rv.number == n).then_some(rv.state)
                })
                .expect("这条在历史里")
        };
        assert_eq!(state_of(27), ReviewState::Merged, "#27 那条被写成了已合并");
        assert_eq!(state_of(9), ReviewState::Open, "别的 PR 不动");
    }

    /// 事务报告 → 回执的映射只写一处：阶段走 `wire_name`，提交号原样带过来。
    #[test]
    fn a_record_maps_the_report_fields() {
        use crate::workbench::app::publish_tx::{PublishStage, PublishTxReport};
        let report = PublishTxReport {
            stage: PublishStage::ReviewOpened,
            audit_passed: 14,
            audit_failed: 0,
            generated: 9,
            unchanged: 0,
            committed_paths: vec!["presets/delivery/catalog.json".to_owned()],
            review: Some(rec("x").review.unwrap()),
            branch: Some("feat/x".to_owned()),
            commit: Some("08ec040".to_owned()),
            files: 17,
            summary: "一句话".to_owned(),
            published_at: Some("2026-10-06T03:39:21Z".to_owned()),
            revision: Some("ebc8e6d7a74e467c".to_owned()),
            changes: None,
            changed_files: Vec::new(),
        };
        let r = PublishRecord::from_report(&report, "2026-10-04T14:02:00+08:00".to_owned());
        assert_eq!(r.stage, "reviewOpened");
        assert_eq!(r.commit.as_deref(), Some("08ec040"));
        assert_eq!(r.review.as_ref().unwrap().number, 27);
        assert_eq!(r.files, 17);
        assert_eq!(r.generated, 9);
        assert_eq!(r.audit_passed, 14);
        assert_eq!(r.summary, "一句话");
    }
}
