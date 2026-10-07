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

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// 平台 API 一次请求的**总时间上限**（含连接与读完响应体）。
///
/// ★ **必须有值**：发布事务是"点一次、等结果"的一条链 —— 网络不给答案时它必须落成
/// 一个错误，而不是转圈到天荒地老（与 `runtime/net.rs` 同一条纪律；2026-10-04 真机上
/// 吃过一次"没有超时就没有尽头"的亏）。
pub const API_TIMEOUT: Duration = Duration::from_secs(30);

/// 连接建立的上限。比总时短 —— 连不上是最常见的一类故障，不该让人等满 30 秒
pub const API_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// 平台 API 专用的 `ureq` Agent（带超时）。GitHub / Gitee **共用这一处** ——
/// 超时口径只写一遍，将来调也只调这里。
pub(super) fn agent() -> ureq::Agent {
    agent_with(API_TIMEOUT)
}

/// 上传专用的 `ureq` Agent（总时长 [`UPLOAD_TIMEOUT`]，连接仍是 [`API_CONNECT_TIMEOUT`]）。
pub(super) fn upload_agent() -> ureq::Agent {
    agent_with(UPLOAD_TIMEOUT)
}

/// 带超时 **+ 系统代理** 的 Agent。**两个出口共用这一处**（与客户端 `runtime/net.rs`
/// 读的是同一个 [`crate::runtime::net::system_proxy_url`]）——
/// 本机配了代理却只有一半出口认它，是最难查的一类"有时通有时不通"。
fn agent_with(global: Duration) -> ureq::Agent {
    let mut builder = ureq::Agent::config_builder()
        .timeout_global(Some(global))
        .timeout_connect(Some(API_CONNECT_TIMEOUT));
    if let Some(url) = crate::runtime::net::system_proxy_url() {
        match ureq::Proxy::new(url.as_str()) {
            Ok(proxy) => builder = builder.proxy(Some(proxy)),
            Err(e) => tracing::warn!("系统代理地址认不出（按直连走）：{url}（{e}）"),
        }
    }
    builder.build().into()
}

/// 评审（GitHub 叫 PR、Gitee 叫 MR）在**工作台这一侧**的统一状态。
///
/// 平台方言（`opened` / `active` / `merged` / `declined`…）由 [`collapse_state`] 收敛到这一档。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
///
/// ★ 也进**发布历史**（`publish-history.json`）⇒ 需要能反序列化回来（`Deserialize`）、
/// 也要能比较（回执往返判据用 `PartialEq`）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

/// **上传安装包**的总时间上限（第四刀）。
///
/// ★ 它与 [`API_TIMEOUT`] **刻意不是一个数**：一个 dmg 几十 MB，30 秒装不下；
/// 而没有上限的上传在断网时会挂到天荒地老（与"没有超时就没有尽头"同一条纪律）。
/// 15 分钟是"家用宽带传一个 dmg"的宽松上界 —— 真到不了会如实报错，不假装还在传。
pub const UPLOAD_TIMEOUT: Duration = Duration::from_secs(15 * 60);

/// 建一个 Release 要的东西（第四刀）。
#[derive(Debug, Clone)]
pub struct ReleaseSpec {
    pub owner: String,
    pub repo: String,
    /// tag 名（**带 `v` 前缀**：`v0.0.2`）—— 与 git tag 逐字一致，Release 因此挂在那一笔上
    pub tag_name: String,
    /// Release 标题（给人在网页上看到的那一句）
    pub name: String,
    /// 正文（发布说明）
    pub body: String,
}

/// 建好的 Release（平台无关）。`id` 是上传 asset 要用的那个号。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteRelease {
    /// 平台 id：`"github"` | `"gitee"`
    pub platform: String,
    pub id: u64,
    pub tag_name: String,
    /// 网页地址（**进 `release.json` 的 `url`** —— 客户端「查看更新」点开的就是它）
    pub url: String,
}

/// 上传一个安装包要的东西。**文件按路径给**（不把字节塞进这个结构）——
/// dmg 有几十 MB，传引用而不是值，也让"上传前先确认文件在不在"有地方落。
#[derive(Debug, Clone)]
pub struct AssetUpload {
    pub owner: String,
    pub repo: String,
    pub release_id: u64,
    /// 上传后在 Release 页面上显示的文件名（`SupportEase_0.0.2_aarch64.dmg`）
    pub name: String,
    /// `Content-Type`（按扩展名定，见 [`content_type_for`]）
    pub content_type: String,
    /// 本地绝对路径
    pub path: std::path::PathBuf,
}

/// 上传完成的结果。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UploadedAsset {
    pub name: String,
    /// 字节数（回执里显示"传了多大"）
    pub size: u64,
    /// 下载直链
    pub url: String,
}

/// 安装包文件名 → `Content-Type`。**纯函数**（判据钉它）。
///
/// 只认会产出的那几种：macOS 的 `.dmg` 与 `.app` 打成的 `.zip`、Windows 的 NSIS `.exe`
/// 与 `.msi`（2026-10-07 两端各自打包起）；认不出 → 通用二进制类型，
/// **不猜**（猜错的代价是用户下下来打不开，而报错至少还能看见）。
pub fn content_type_for(file_name: &str) -> &'static str {
    let lower = file_name.to_ascii_lowercase();
    if lower.ends_with(".dmg") {
        "application/x-apple-diskimage"
    } else if lower.ends_with(".zip") {
        "application/zip"
    } else if lower.ends_with(".exe") {
        "application/vnd.microsoft.portable-executable"
    } else if lower.ends_with(".msi") {
        "application/x-msi"
    } else {
        "application/octet-stream"
    }
}

/// **安装包命名**（第四刀 · 作者裁决 ⑤：沿用项目既有风格，不另造体系）。
///
/// `SupportEase_0.0.2_aarch64.dmg` —— 产品名 + 版本 + 架构，与 Tauri 自己产出的
/// 文件名（含 `universal` / 机型后缀）对齐。
pub fn asset_name(product: &str, version: &str, arch: &str, ext: &str) -> String {
    format!("{product}_{version}_{arch}.{ext}")
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

    /// 合并一份 PR / MR（见 [`MergeMethod`]）。
    ///
    /// ★ **只在用户显式点过「合并」之后调**：没有自动合并、没有定时合并。
    /// ★ **CI 不是前置条件**（作者 2026-10-04 拍：不强制等 CI —— CI 状态由界面在
    /// 二次确认里说清，决定权在人）。平台自己有保护规则时会如实报错，不掩盖。
    fn merge_review(&self, id: &ReviewId, method: MergeMethod) -> Result<RemoteReview, AppError>;

    /// 这一支（`head` → `base`）上**已经开着的**那份 PR / MR，没有 = `None`。
    ///
    /// ★ 用途：**发布可重跑**。平台不许同一个 head→base 开两份 PR —— 在合并之前再点一次
    /// 「发布」，`create_review` 会落 422。那不是失败（产物已经推上去了），是"这一支已经
    /// 有一份在等人合"：**回读那一份**继续，别把人堵在一条死路上（作者 2026-10-04）。
    fn find_open_review(
        &self,
        owner: &str,
        repo: &str,
        head: &str,
        base: &str,
    ) -> Result<Option<RemoteReview>, AppError>;

    /// 基于一个 **已经推上去的 tag** 建 Release（第四刀）。
    ///
    /// ★ 顺序是"先有 tag 再有 Release"：Release 挂在 tag 上，tag 挂在 main 的 tip 上。
    /// 倒过来的后果是 Release 指着一个还不存在的提交。
    fn create_release(&self, spec: &ReleaseSpec) -> Result<RemoteRelease, AppError> {
        let _ = spec;
        Err(platform_not_supported(self.kind()))
    }

    /// 往一个 Release 上**传安装包**（二进制 body，第四刀）。
    ///
    /// ★ 与 JSON 出口不是一回事：body 是文件字节、超时走 [`UPLOAD_TIMEOUT`]。
    fn upload_asset(&self, up: &AssetUpload) -> Result<UploadedAsset, AppError> {
        let _ = up;
        Err(platform_not_supported(self.kind()))
    }

    /// 这个 tag 的 Release 上**已经挂了哪些附件**（文件名）。**没有那个 Release = 空表**
    /// （不是错误 —— 还没发过就是这个答案）。
    ///
    /// ★ 用途（2026-10-07）：**同一版本可以发多个平台** —— macOS 发过之后，Windows 还能
    /// 往同一个 Release 补一个包。闸与事务据此回答"**本平台**是不是已经发过了"，
    /// 而不是笼统地"tag 被占就不许发"。
    fn release_assets(&self, owner: &str, repo: &str, tag: &str) -> Result<Vec<String>, AppError> {
        let _ = (owner, repo, tag);
        Err(platform_not_supported(self.kind()))
    }
}

/// 这批附件名里**有没有本平台那一份**（按后缀认：macOS 认 `.dmg`、Windows 认 `.exe`）。
///
/// 判据收在这里而不是各写一遍：闸与事务都拿它回答"这一版这一平台发过没有"，
/// 两处要是各写各的，"闸说能发、事务说发过了"就会同时出现。**纯函数**（判据钉它）。
pub fn platform_asset_uploaded(names: &[String], ext: &str) -> bool {
    let suffix = format!(".{}", ext.to_ascii_lowercase());
    names
        .iter()
        .any(|n| n.to_ascii_lowercase().ends_with(&suffix))
}

/// **平台 id → 平台客户端**。**两张入口（工作台 / CLI）共用这一张映射表** ——
/// 抽屉只开一次，加第三个平台时改一处就够。认不出 → `None`（由调用方如实报，不猜）。
pub fn hosting(kind: &str, token: String) -> Option<Box<dyn Hosting>> {
    match kind {
        "github" => Some(Box::new(github::GitHub::new(token))),
        "gitee" => Some(Box::new(gitee::Gitee::new(token))),
        _ => None,
    }
}

/// 「这个平台暂不支持软件 Release」的**统一说法**（第四刀）。
///
/// ★ **如实报，不假装支持**：Gitee 没有把 Release API 接进来之前，调用它要得到一个
/// 明确的"没做"，而不是一次假装成功的空返回 —— 假装成功的后果是用户以为发布了。
pub fn platform_not_supported(platform: &str) -> AppError {
    AppError::not_implemented(format!(
        "{platform} 这一支还没接「发布软件版本」（Release / 上传安装包）"
    ))
}

/// 「这一支上开着的 PR / MR」的列表查询串。**纯函数**（两个平台参数名同形：`state` /
/// `head` / `base`）—— 查询写错比报错更难查，判据钉它。
///
/// `head` 要**带 owner 前缀**（GitHub 的约定：`?head=owner:branch`）；分支名里的 `/`
/// 在 query 里是合法字符，不用转义。
pub fn open_reviews_query(owner: &str, head: &str, base: &str) -> String {
    format!("/pulls?state=open&head={owner}:{head}&base={base}")
}

/// 合并方式。**只留 Squash 一档**（作者 2026-10-04 拍）。
///
/// 不摆 merge commit / rebase：摆出来就得解释三种历史的差别，而工作台这条链的目的
/// 是"发布 → 看结果 → 合并"，不是教人挑合并策略。将来真要加，加在这里。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeMethod {
    Squash,
}

impl MergeMethod {
    /// 平台方言里的名字（GitHub 与 Gitee 都收 `squash`）。
    pub fn wire_name(self) -> &'static str {
        match self {
            MergeMethod::Squash => "squash",
        }
    }
}

/// 合并请求的 API 路径。**纯函数**（两个平台同形：都是 `pulls`）—— 端点写错比报错更难查，
/// 判据钉它。
pub fn merge_path(owner: &str, repo: &str, number: u64) -> String {
    format!("/repos/{owner}/{repo}/pulls/{number}/merge")
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

    /// 「本平台发过没有」只看**本平台那个后缀**（2026-10-07）：同一版本可以发多个平台，
    /// 判据错了就是"闸说能补、事务说发过了"（或反过来，白跑一趟构建）。
    #[test]
    fn the_platform_asset_check_only_looks_at_its_own_extension() {
        let names = vec![
            "SupportEase_0.0.6_aarch64.dmg".to_owned(),
            "SupportEase_0.0.6_aarch64.app.zip".to_owned(),
        ];
        assert!(platform_asset_uploaded(&names, "dmg"), "macOS 那一份在");
        assert!(
            !platform_asset_uploaded(&names, "exe"),
            "Windows 那一份不在"
        );
        // 大小写不敏感（平台回给我们的名字不一定是什么形状）
        assert!(platform_asset_uploaded(&names, "DMG"));
        // `.app.zip` 不该被当成 `.dmg`（一个真会踩的坑：`ends_with("dmg")` 少了那个点）
        assert!(!platform_asset_uploaded(
            &["SupportEase_0.0.6_aarch64.app.zip".to_owned()],
            "dmg"
        ));
        assert!(!platform_asset_uploaded(&[], "dmg"), "空表 = 还没发过");
    }

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

    /// **安装包命名**沿用项目既有风格（产品名 + 版本 + 架构），不另造体系。
    #[test]
    fn asset_name_is_product_version_arch() {
        assert_eq!(
            asset_name("SupportEase", "0.0.2", "aarch64", "dmg"),
            "SupportEase_0.0.2_aarch64.dmg"
        );
    }

    /// `.dmg` 必须是 Apple 磁盘镜像那个类型 —— 猜错的代价是用户下下来打不开。
    /// 认不出时用通用二进制类型，**不猜**。
    #[test]
    fn content_type_maps_installer_extensions_without_guessing() {
        assert_eq!(
            content_type_for("SupportEase_0.0.2_aarch64.dmg"),
            "application/x-apple-diskimage"
        );
        assert_eq!(content_type_for("x.zip"), "application/zip");
        assert_eq!(content_type_for("x.msi"), "application/x-msi");
        assert_eq!(
            content_type_for("mystery.bin"),
            "application/octet-stream",
            "认不出就用通用二进制类型"
        );
    }

    /// **没接的平台如实报"不支持"**，不返回假成功 —— 假装成功的后果是用户以为发布了。
    #[test]
    fn an_unsupported_platform_says_so() {
        let e = platform_not_supported("gitee");
        assert_eq!(e.code, crate::error::ErrorCode::NotImplemented);
        assert!(e.message.contains("gitee"));
    }

    /// 合并：**只有 squash 一档**；端点路径两个平台同形（写错端点比报错更难查）。
    #[test]
    fn merge_is_squash_only_and_the_path_is_shared() {
        assert_eq!(MergeMethod::Squash.wire_name(), "squash");
        assert_eq!(
            merge_path("MuCoreBenC", "MKPSupportEase", 27),
            "/repos/MuCoreBenC/MKPSupportEase/pulls/27/merge"
        );
    }

    /// 「这一支上开着的 PR」的查询串：head 带 owner 前缀、分支名里的 `/` 不转义。
    #[test]
    fn the_open_reviews_query_carries_the_owner_prefix() {
        assert_eq!(
            open_reviews_query("MuCoreBenC", "feat/param-def-controls-undo", "main"),
            "/pulls?state=open&head=MuCoreBenC:feat/param-def-controls-undo&base=main"
        );
    }
}
