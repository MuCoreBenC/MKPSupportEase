//! **代码托管平台出口**（GitHub / Gitee）—— 发布事务的「远程那一半」。
//!
//! # 与本地 git 的分工（产品决定，不是实现细节）
//!
//! ```text
//! 本地 git（[`super::git`]）   提交 / 推送   —— 本地版本控制工具，子进程复用系统 git
//! 平台出口（本模块）           PR/MR / CI   —— SupportEase 自持凭据，自己发 HTTP
//! ```
//!
//! ★ **不借用户的 `gh` / `git` 登录态**：用户打开 SupportEase、配一次 Token，
//! 就能发布 —— 而不是"先装好 GitHub CLI 并登录"。这是作者 2026-10-04 定死的边界。
//!
//! # 网络只住这一处（第二处）
//!
//! 仓库的纪律是"网络只住 `runtime/net.rs`"。本模块是**唯一被批准的第二个网络出口**
//! （工作台发布平台 API），`scripts/check-zero-network.mjs` 第①道闸的措辞随之放宽成
//! "网络只住 `runtime/net.rs` 与工作台的发布平台出口"。**新增网络调用只能加在这里**，
//! 不许散到别的模块。
//!
//! # 统一状态模型
//!
//! GitHub 的 "PR + check run / statuses" 与 Gitee 的 "MR + status" 是两套方言。
//! 前端**只认** [`ReviewState`] 与 [`ChecksSummary`] 这两档 —— 平台差异在各自 impl 内消化。

pub mod gitee;
pub mod github;

use serde::Serialize;

use crate::error::AppError;

/// 评审（GitHub 叫 PR、Gitee 叫 MR）在**工作台这一侧**的统一状态。
///
/// 平台方言（`opened` / `active` / `merged` / `declined`…）由 [`collapse_state`] 收敛到这一档。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ReviewState {
    /// 开着（GitHub `open` / `opened`，Gitee `active`）
    Open,
    /// 已合并
    Merged,
    /// 已关闭 / 已拒绝
    Closed,
    /// 认不出 —— **不猜**（平台加了新状态时如实说"不认识"）
    Unknown,
}

/// CI 检查汇总，工作台这一侧的档。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ChecksSummary {
    /// 还没出结果 / 正在跑
    Pending,
    /// 全过
    Passed,
    /// 有红的
    Failed,
    /// 这个平台 / 这个 PR 没有可读的检查（不是"通过"）
    None,
    /// 认不出
    Unknown,
}

/// 一份评审的**平台无关**快照。前端只拿它画状态条。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteReview {
    /// 平台 id：`"github"` | `"gitee"`
    pub platform: String,
    /// PR / MR 编号
    pub number: u64,
    /// 网页地址
    pub url: String,
    /// 统一评审状态
    pub state: ReviewState,
    /// CI 汇总
    pub checks: ChecksSummary,
    /// 标题（给前端显示"发的是哪一版"）
    pub title: String,
    /// 源分支 → 目标分支
    pub head: String,
    pub base: String,
}

/// 建一份评审要的东西。各平台 impl 自己映射到它那套字段。
#[derive(Debug, Clone)]
pub struct ReviewSpec {
    pub owner: String,
    pub repo: String,
    pub head: String,
    pub base: String,
    pub title: String,
    pub body: String,
}

/// 评审的定位（回读状态用）。
#[derive(Debug, Clone)]
pub struct ReviewId {
    pub owner: String,
    pub repo: String,
    pub number: u64,
}

/// 一个代码托管平台的**同一张脸**。GitHub / Gitee 各实现一份。
///
/// 只收 `&self`（凭据在构造时注入）：网络调用是同步的（`ureq`），
/// 所以整套接口是阻塞的 —— 与工作台既有命令的"在 `spawn_blocking` 里跑"一致。
pub trait Hosting {
    /// 平台 id（`"github"` / `"gitee"`）—— 写进 [`RemoteReview::platform`]
    fn kind(&self) -> &'static str;

    /// 建一份 PR / MR
    fn create_review(&self, spec: &ReviewSpec) -> Result<RemoteReview, AppError>;

    /// 回读一份 PR / MR 的状态（PR state + 检查汇总一起取）
    fn get_review(&self, id: &ReviewId) -> Result<RemoteReview, AppError>;
}

/// 从 git remote URL 推断平台。**纯函数**，判据钉它。
///
/// 认得出 github.com → `"github"`；gitee.com → `"gitee"`；
/// 认不出（自建源 / 别的平台 / 格式不对）→ `None`（前端据此要求用户在设置里显式选）。
///
/// 同时收 `https://` 与 `git@host:owner/repo.git`（SSH）两种形状。
pub fn detect_platform(remote_url: &str) -> Option<&'static str> {
    let u = remote_url.trim().to_ascii_lowercase();
    if u.is_empty() {
        return None;
    }
    // 取 host：https://host/... 或 git@host:...
    let host = if let Some(rest) = u
        .strip_prefix("https://")
        .or_else(|| u.strip_prefix("http://"))
    {
        rest.split('/').next().unwrap_or("")
    } else if let Some(rest) = u.strip_prefix("git@") {
        rest.split([':', '/']).next().unwrap_or("")
    } else {
        // 别的形状（ssh://git@host/... 等）—— 退一步按 host 片段找
        u.split("://")
            .nth(1)
            .and_then(|r| {
                let r = r.split('@').next_back().unwrap_or(r);
                r.split('/').next()
            })
            .unwrap_or("")
    };
    if host.ends_with("github.com") {
        Some("github")
    } else if host.ends_with("gitee.com") {
        Some("gitee")
    } else {
        None
    }
}

/// 从 remote URL 里拆出 `(owner, repo)`。`https://github.com/owner/repo.git` → `("owner","repo")`。
pub fn parse_owner_repo(remote_url: &str) -> Option<(String, String)> {
    let u = remote_url.trim().trim_end_matches(".git");
    let tail = if let Some(rest) = u
        .strip_prefix("https://")
        .or_else(|| u.strip_prefix("http://"))
    {
        rest.split_once('/')?.1
    } else if let Some(rest) = u.strip_prefix("git@") {
        rest.split_once(':')?.1
    } else {
        None?
    };
    let mut parts = tail.split('/').filter(|s| !s.is_empty());
    let owner = parts.next()?.to_owned();
    let repo = parts.next()?.to_owned();
    Some((owner, repo))
}

/// 平台方言 → 统一评审状态。**纯函数**（判据调它）。
pub fn collapse_state(raw: &str) -> ReviewState {
    match raw.trim().to_ascii_lowercase().as_str() {
        "open" | "opened" | "active" => ReviewState::Open,
        "merged" => ReviewState::Merged,
        "closed" | "declined" | "rejected" => ReviewState::Closed,
        _ => ReviewState::Unknown,
    }
}

/// 检查状态串 → 统一汇总。**纯函数**。
///
/// 输入是各平台 impl 归一化后的**少量**词：`pending` / `success` / `failure` /
/// `error` / `none`。平台原始的多档（GitHub 的 `queued/in_progress/completed` ×
/// `success/failure/neutral...`）在 impl 里先压成这几个词。
pub fn collapse_checks(raw: &str) -> ChecksSummary {
    match raw.trim().to_ascii_lowercase().as_str() {
        "pending" | "queued" | "in_progress" | "running" => ChecksSummary::Pending,
        "success" | "passed" | "pass" => ChecksSummary::Passed,
        "failure" | "failed" | "fail" | "error" | "errored" => ChecksSummary::Failed,
        "none" | "empty" => ChecksSummary::None,
        _ => ChecksSummary::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **remote URL → 平台**（纯函数）。含 https / ssh / 自建 / 空 的边界。
    #[test]
    fn detect_platform_maps_remote_urls() {
        assert_eq!(
            detect_platform("https://github.com/MuCoreBenC/MKPSupportEase.git"),
            Some("github")
        );
        assert_eq!(
            detect_platform("git@github.com:MuCoreBenC/MKPSupportEase.git"),
            Some("github")
        );
        assert_eq!(
            detect_platform("https://gitee.com/someone/repo.git"),
            Some("gitee")
        );
        assert_eq!(
            detect_platform("git@gitee.com:someone/repo.git"),
            Some("gitee")
        );
        // 自建源 / 认不出 = None（不猜）
        assert_eq!(detect_platform("https://git.example.com/a/b.git"), None);
        assert_eq!(detect_platform(""), None);
    }

    /// owner/repo 解析：https 与 ssh 两种形状都要出来。
    #[test]
    fn parse_owner_repo_handles_https_and_ssh() {
        assert_eq!(
            parse_owner_repo("https://github.com/MuCoreBenC/MKPSupportEase.git"),
            Some(("MuCoreBenC".to_owned(), "MKPSupportEase".to_owned()))
        );
        assert_eq!(
            parse_owner_repo("git@gitee.com:someone/repo.git"),
            Some(("someone".to_owned(), "repo".to_owned()))
        );
        assert_eq!(parse_owner_repo("https://bad"), None);
    }

    /// **方言收敛**：GitHub 与 Gitee 的原始状态词落到同一档 —— 前端不认识任何一个平台。
    #[test]
    fn remote_state_collapses_platform_dialects() {
        for raw in ["open", "opened", "active"] {
            assert_eq!(collapse_state(raw), ReviewState::Open, "{raw}");
        }
        assert_eq!(collapse_state("merged"), ReviewState::Merged);
        for raw in ["closed", "declined"] {
            assert_eq!(collapse_state(raw), ReviewState::Closed, "{raw}");
        }
        assert_eq!(collapse_state("???"), ReviewState::Unknown, "认不出不猜");
    }

    /// 检查汇总的收敛：多档原始词 → 四档（+ None / Unknown）。
    #[test]
    fn checks_summary_collapses_to_four_buckets() {
        assert_eq!(collapse_checks("queued"), ChecksSummary::Pending);
        assert_eq!(collapse_checks("in_progress"), ChecksSummary::Pending);
        assert_eq!(collapse_checks("success"), ChecksSummary::Passed);
        assert_eq!(collapse_checks("failure"), ChecksSummary::Failed);
        assert_eq!(collapse_checks("none"), ChecksSummary::None);
        assert_eq!(collapse_checks("huh"), ChecksSummary::Unknown);
    }
}
