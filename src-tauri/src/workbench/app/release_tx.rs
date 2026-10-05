//! **发布软件版本事务**（第四刀）—— 把「确认主线 → 版本号 → tag → 构建 → Release → 上传 →
//! 写 `release.json`」串成**一次手势**，给两个入口（工作台 / CLI）同一个内核。
//!
//! # 为什么是"内核 + 两个壳"
//!
//! 作者 2026-10-04 定死：`scripts/release.mjs` **不废弃**，也不让工作台再复制一遍 ——
//! 它与工作台是**两个入口、同一套 Rust 核心**。所以这里只收参数、不收 `AppHandle`：
//! 平台客户端与仓库根都从外面注入，好让判据塞一个假的 `Hosting` 进来跑全流程。
//!
//! # 为什么不收 `&Ctx`（与 `publish_tx` 的一处**刻意**不同）
//!
//! 本事务**不碰 `Presets`**（它改的是版本号与发布信息，不是预设数据），所以它根本
//! 不需要进 `with_ctx` —— 从根上躲开那把不可重入的锁。这是设计，不是偷懒：
//! 发布预设那条链必须读预设，那是它的活；这条链的活不在这儿。
//!
//! # 阶段快照，不是 `success: bool`
//!
//! 与 [`PublishTxReport`][super::publish_tx::PublishTxReport] 同一个形状取向：
//! 事务可能停在**任何一步**（预检红 / 建 PR 失败 / 上传没成），前端据 `stage` 显示
//! "停在哪、为什么"。没有把过程压扁的布尔。
//!
//! # 两笔提交
//!
//! ```text
//! ① 版本号那一批（Cargo.toml + 三个派生）  → 分支 → PR → 合并 → 打 tag
//! ② presets/delivery/release.json（上传成功之后）   → 新分支 → PR → **停在这里等人合并**
//! ```
//!
//! ② 之所以是另一笔、且**不再自动合并**：`release.json` 只有在**合并进 main**之后才
//! 会被 `raw.githubusercontent.com` 吐出去 —— 那一刻才是"客户端看见新版本"的那一刻。
//! 合并留给人（作者定死的纪律）。

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::AppError;
use crate::fsx;
use crate::runtime::release_info::{self, RELEASE_SCHEMA};

use super::git::{Git, RELEASE_STAGE_ALLOWLIST};
use super::platform::{Hosting, RemoteRelease, RemoteReview, ReviewSpec};
use super::publish_tx::PublishTarget;
use super::version;

/// tag 的前缀。`v0.0.2` —— 与 `scripts/release.mjs` 一直在用的形状逐字一致。
pub const TAG_PREFIX: &str = "v";
/// 安装包的产品名（asset 命名与 Release 标题用它）。
pub const PRODUCT: &str = "SupportEase";
/// 安装包产物目录（相对仓库根）：`tauri build` 把 dmg 放在这里。
///
/// ★ 两个候选都在：本仓库的 `CARGO_TARGET_DIR` 指到**仓库根**的 `target/`（真机实测
/// 2026-10-04 构建落在 `target/release/bundle/dmg/`），而老机器 / 别的配置下会在
/// `src-tauri/target/…`。**按顺序找，找到哪个里有就用哪个** —— 两个都没有才报错。
/// `.app` 所在目录（与 [`DMG_DIRS`] 同一个"仓库根 target 优先"的前提）
pub const MACOS_BUNDLE_DIR: &str = "target/release/bundle/macos";

pub const DMG_DIRS: [&str; 2] = [
    "target/release/bundle/dmg",
    "src-tauri/target/release/bundle/dmg",
];
/// `release.json` 相对仓库根的落点。**它是客户端"有没有新版本"的唯一正式信息源**。
/// `release.json` 相对仓库根的落点。**它是客户端"有没有新版本"的唯一正式信息源**。
///
/// ★ 2026-10-05 寻址改造（总纲 §1②）：从 `presets/release.json` 入住交付根
/// `presets/delivery/` —— 它本来就是客户端消费的交付元数据，与 catalog / manifest /
/// source 同边界；客户端按 Source Manifest 的 `release` 声明取它（旧的"上跳一级"
/// 推导已废除）。测试钉位见 `runtime::release_info`。
pub const RELEASE_INFO_REL: &str = "presets/delivery/release.json";

/// **断代线**（M6，总纲 §5-M6）：从这个版本起，`release.json` 里的 URL 一律来自
/// **Gitee Release**（发布仓库），`github.com/.../releases` 只允许出现在断代前的
/// 历史记录里（v≤0.0.5 是 GitHub 发的，照旧合法 —— 那是事实，不改写）。
pub const FIRST_GITEE_RELEASE: &str = "0.0.6";

/// 断代校验（写 `release.json` **之前**调，违规即事务短路 —— 坏 URL 不许落盘）。
///
/// ★ 与 `scripts/check-release-source.mjs`（CI）同一条规则的两个实现：
/// 那边查**已入库**的历史档，这边拦**正在生成**的这一份。常量两边同值。
fn validate_release_source(
    version: &str,
    release_url: &str,
    asset: Option<&crate::runtime::release_info::ReleaseAsset>,
) -> Result<(), AppError> {
    // `compare(latest, current)` = latest 更新；"断代线比这一版还新" = 还在断代前
    if crate::runtime::release_info::compare(FIRST_GITEE_RELEASE, version) {
        return Ok(());
    }
    let mut bad: Vec<String> = Vec::new();
    if release_url.contains("github.com") {
        bad.push(format!("Release 页 {}", release_url));
    }
    if let Some(a) = asset {
        if a.url.contains("github.com") {
            bad.push(format!("安装包 {}", a.url));
        }
    }
    if bad.is_empty() {
        Ok(())
    } else {
        Err(AppError::invalid_argument(format!(
            "release.json 的下载地址必须是 Gitee（总纲 §5-M6：{FIRST_GITEE_RELEASE} 起断代）——              这些还指着 GitHub：{}",
            bad.join("；")
        )))
    }
}

/// **软件版本的发布通道**（M6，2026-10-05 定成架构规则）：
/// Release 与安装包附件住在 **Gitee**（发布仓库 = 设置里的 Gitee 账户），
/// 与 `target`（PR / tag / 推送，跟仓库 remote 走）**刻意分开** ——
/// 代码主线在 GitHub，发布面（Release + 附件 + 数据源镜像）在 Gitee。
/// 客户端只读 `release.json → asset.url`，永远不知道发布平台细节。
pub struct ReleaseChannel<'a> {
    pub hosting: &'a dyn Hosting,
    /// Gitee 的发布目标（owner/repo/凭据）
    pub target: &'a PublishTarget,
}

/// 发布软件版本的**阶段**。顺序即流程，前端只认这一档枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ReleaseStage {
    /// 预检没过 —— **零写入**
    BlockedPreflight,
    /// 预检过了（演练到此为止）
    Ready,
    /// 版本号已推进（含三个派生）
    VersionBumped,
    /// ① 已提交
    Committed,
    /// ① 已推送
    Pushed,
    /// ① PR 已建 / 已回读
    ReviewOpened,
    /// ① PR 已合并
    Merged,
    /// tag 已打在 main 的 tip 上并推送
    Tagged,
    /// 安装包已构建出来
    Built,
    /// Release 已建
    ReleaseCreated,
    /// 安装包已上传
    AssetUploaded,
    /// ② `release.json` 已提交
    InfoCommitted,
    /// ② 已推送
    InfoPushed,
    /// ② PR 已建 —— **停在这里等人合并**
    InfoReviewOpened,
}

impl ReleaseStage {
    /// 线上名字（与 serde 的 camelCase **逐字一致**；判据钉它 —— 前端按这个字符串画图）。
    pub fn wire_name(self) -> &'static str {
        match self {
            ReleaseStage::BlockedPreflight => "blockedPreflight",
            ReleaseStage::Ready => "ready",
            ReleaseStage::VersionBumped => "versionBumped",
            ReleaseStage::Committed => "committed",
            ReleaseStage::Pushed => "pushed",
            ReleaseStage::ReviewOpened => "reviewOpened",
            ReleaseStage::Merged => "merged",
            ReleaseStage::Tagged => "tagged",
            ReleaseStage::Built => "built",
            ReleaseStage::ReleaseCreated => "releaseCreated",
            ReleaseStage::AssetUploaded => "assetUploaded",
            ReleaseStage::InfoCommitted => "infoCommitted",
            ReleaseStage::InfoPushed => "infoPushed",
            ReleaseStage::InfoReviewOpened => "infoReviewOpened",
        }
    }
}

/// 事务开关。**不是给前端摆的开关** —— 它给"两个入口 + 判据"控制"走到哪一步停"。
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseOptions {
    /// 目标版本号（`x.y.z`，不带 v）。`None` = 沿用当前真值（第一版 v0.0.1 就是这么发的）
    #[serde(default)]
    pub version: Option<String>,
    /// 一句话发布说明（进 Release 正文与 `release.json` 的 `notes`）
    #[serde(default)]
    pub notes: String,
    /// PR 的 base 分支。空 = `main`
    #[serde(default)]
    pub base: String,
    /// 只预检、报"会做什么"，不动一个字节
    #[serde(default)]
    pub dry_run: bool,
    /// 建了 PR 之后**接着合并**它（人在界面上点过「确认发布」才给 true）
    #[serde(default)]
    pub merge: bool,
    /// 构建 macOS 安装包（**默认开**：发软件版本就是要有安装包）
    #[serde(default = "default_build")]
    pub build: bool,
    /// 提交推送之后要不要建 PR
    #[serde(default = "default_open_review")]
    pub open_review: bool,
}

fn default_build() -> bool {
    true
}
fn default_open_review() -> bool {
    true
}

impl Default for ReleaseOptions {
    fn default() -> Self {
        Self {
            version: None,
            notes: String::new(),
            base: "main".to_owned(),
            dry_run: false,
            merge: false,
            build: true,
            open_review: true,
        }
    }
}

impl ReleaseOptions {
    /// PR 的 base：空串回落 `main`。
    pub fn base_or_main(&self) -> &str {
        if self.base.trim().is_empty() {
            "main"
        } else {
            self.base.trim()
        }
    }
}

/// 构建出来的安装包。
#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactInfo {
    /// 上传用的文件名（`SupportEase_0.0.2_aarch64.dmg`）
    pub name: String,
    /// 字节数
    pub size: u64,
    /// 本地绝对路径（回执里"在访达中显示"用得上）
    pub path: String,
}

/// 一轮发布软件版本的结果（**阶段快照**）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseTxReport {
    pub stage: ReleaseStage,
    /// 这一轮的版本号（`x.y.z`，不带 v）
    pub version: String,
    /// tag（`v0.0.2`）
    pub tag: String,
    /// ① 所在分支
    pub branch: Option<String>,
    /// ① 提交的短 sha
    pub commit: Option<String>,
    /// ① 的 PR / MR
    pub review: Option<RemoteReview>,
    /// 建好的 Release（网页地址进 `release.json` 的 `url`）
    pub release: Option<RemoteRelease>,
    /// 上传上去的安装包（dmg，给愿意手动装的人）
    pub artifact: Option<ArtifactInfo>,
    /// ★ **应用内更新用的 `.app.zip`**（第五刀）：客户端在程序里下载它、自动替换并重启。
    /// **没有它客户端就退回"打开下载页"** —— 那条路仍然成立
    pub zip: Option<crate::runtime::release_info::ReleaseAsset>,
    /// ② 所在分支（`release/vX.Y.Z`）
    pub info_branch: Option<String>,
    /// ② 的 PR / MR —— **它由人合并**，合并完客户端才看得到新版本
    pub info_review: Option<RemoteReview>,
    /// ① 提交的路径
    pub committed_paths: Vec<String>,
    /// ② 提交的路径
    pub info_committed_paths: Vec<String>,
    /// 预检没过的原因（逐条给人看见）
    pub blocked_reasons: Vec<String>,
    /// 这一步的一句话说明（给状态条）
    pub summary: String,
}

impl ReleaseTxReport {
    fn blocked(version: String, tag: String, reasons: Vec<String>) -> Self {
        Self {
            stage: ReleaseStage::BlockedPreflight,
            version,
            tag,
            branch: None,
            commit: None,
            review: None,
            release: None,
            artifact: None,
            zip: None,
            info_branch: None,
            info_review: None,
            committed_paths: Vec::new(),
            info_committed_paths: Vec::new(),
            blocked_reasons: reasons.clone(),
            summary: format!("预检没过（{} 条），一个字节都没动。", reasons.len()),
        }
    }
}

fn tag_of(version: &str) -> String {
    format!("{TAG_PREFIX}{version}")
}

/// **事务内核**：走完"预检 → 版本号 → 提交推送 → PR → 合并 → tag → 构建 → Release
/// → 上传 → 写 `release.json` → 它的 PR"，停在它能停的地方并如实报。
///
/// - `repo_root`：仓库根（工作台传 [`crate::workbench::paths::repo_root()`]，判据传临时仓库）。
/// - `target` / `hosting`：发布目标与平台客户端（`None` = 只做本地那一半，不碰平台）。
///   **两者都是参数**而不是内部构造 —— 判据因此能注入假的，不真发网络请求。
///
/// ★ **失败即停**：任何一步出错就带着已完成的事实返回/报出，**不回滚远端**
/// （远端已经发生的事回退不了，那是人的决定 —— 回执会给下一步）。
pub fn run(
    repo_root: &Path,
    opts: &ReleaseOptions,
    target: Option<&PublishTarget>,
    hosting: Option<&dyn Hosting>,
    release: Option<&ReleaseChannel<'_>>,
) -> Result<ReleaseTxReport, AppError> {
    let git = Git::open(repo_root);
    let base = opts.base_or_main();

    /* ---------- ① 确定版本号 ---------- */

    // ★ **真值**与"这一轮要发的版本号"是两件事：真值读自 manifest，目标可能由人指定。
    //   下面"要不要推进"比的是**真值**，不是目标 —— 拿目标跟目标比永远相等，
    //   版本号就再也推不动了（这个 bug 真实发生过一次，判据把它钉在这里）。
    let truth = version::app_version(repo_root)?;
    let version = match opts.version.as_deref().map(str::trim) {
        Some(v) if !v.is_empty() => v.to_owned(),
        _ => truth.clone(),
    };
    let tag = tag_of(&version);

    /* ---------- ② 预检（**没有任何写操作**） ---------- */

    let mut blocked = Vec::new();
    let branch = git.branch().unwrap_or_default();
    if branch == base {
        blocked.push(format!(
            "不能在主线 `{base}` 上发版：发版是把分支上的改动发出去，本身要从分支开始"
        ));
    }
    if !git.is_clean()? {
        blocked.push("工作区不干净 —— 发版会改版本号并提交，混在一起说不清".to_owned());
    }
    // 版本号撞车：tag 已存在且**指的又是另一笔提交** = 这一版发出去过，换版本号。
    // 指向当前 HEAD（续跑上一次的事务）不算撞车，下面的步骤会跳过重复动作。
    if git.tag_exists(&tag)? && !tag_points_at_head(&git, &tag) {
        blocked.push(format!(
            "tag `{tag}` 已经存在。换一个版本号，或先确认那一版发到哪了"
        ));
    }
    // 版本派生不一致 = 有人手改了别处。给了新版本号的话 `bump` 会把它推平，
    // 所以只在"沿用当前真值"时它才是问题。
    if opts.version.is_none() {
        for rel in version::derived_mismatch(repo_root)? {
            blocked.push(format!("{rel} 与版本真值不一致（版本号只有一处真值）"));
        }
    }
    if !blocked.is_empty() {
        return Ok(ReleaseTxReport::blocked(version, tag, blocked));
    }

    let mut report = ReleaseTxReport {
        stage: ReleaseStage::Ready,
        version: version.clone(),
        tag: tag.clone(),
        branch: Some(branch.clone()),
        commit: None,
        review: None,
        release: None,
        artifact: None,
        zip: None,
        info_branch: None,
        info_review: None,
        committed_paths: Vec::new(),
        info_committed_paths: Vec::new(),
        blocked_reasons: Vec::new(),
        summary: format!("预检通过：{base} 上发 {tag}。"),
    };

    if opts.dry_run {
        report.summary.push_str("（演练：一个字节都没动）");
        return Ok(report);
    }

    /* ---------- ③ 推进版本号（给了新版本才动） ---------- */

    // ★ **续跑**：真值已经是目标版本（上一趟推进过了 / 这一趟就没给新版本号）就不重复推 ——
    //   发版可能被切成两趟（先建 PR 看 CI、再合并），第二趟进来时版本号已经到位。
    //   ★ 比的是**解析之后的目标**（空串早被折成真值），不是原始入参 ——
    //   拿 `Some("")` 去 bump 会在校验那一步报"形状不对"，那是参数解析的锅，不是发版的锅。
    if version == truth {
        report.summary.push_str(&format!(
            "版本号已经是 {version}（没给新版本号 / 上一趟推进过），跳过。"
        ));
    } else {
        let changed = version::bump(repo_root, &version)?;
        report.committed_paths = changed;
        report.stage = ReleaseStage::VersionBumped;
        report.summary.push_str(&format!(
            "版本号已推进到 {version}（改了 {} 处）。",
            report.committed_paths.len()
        ));
    }

    /* ---------- ④ ① 提交 → 推送 ---------- */

    let candidates: Vec<String> = RELEASE_STAGE_CANDIDATES
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    let staged = git.stage_allowed_in(&candidates, &RELEASE_STAGE_ALLOWLIST)?;
    if git.has_staged()? {
        git.commit(&format!("chore: {tag}"))?;
        report.commit = git.head_short().ok();
        report.committed_paths = staged;
        report.stage = ReleaseStage::Committed;
        report.summary.push_str(&format!(
            "已提交{}。",
            commit_suffix(report.commit.as_deref())
        ));
    } else {
        report
            .summary
            .push_str("版本号那一批没有新变化（可能上次已经提交过），跳过提交。");
    }

    // 推送：**从发布账户配置取凭据**（SupportEase 自持），不借用户 gh/git 登录态。
    match target {
        Some(t) => {
            if !git.remote_matches(&t.repository_url)? {
                return Err(AppError::invalid_argument(
                    "当前仓库的远端地址和「发布账户」里配置的不一致，先确认一下发布目标",
                )
                .with_detail(format!("配置：{}", t.repository_url)));
            }
            git.push_authenticated(&branch, &t.username, &t.token)?;
        }
        None => git.push(&branch)?,
    }
    report.stage = ReleaseStage::Pushed;
    report.summary.push_str(&format!("已推送到 `{branch}`。"));

    /* ---------- ⑤ ① 建 PR →（可选）合并 ---------- */

    let (Some(hosting), Some(t)) = (hosting, target) else {
        // 没有平台 = CLI 只做本地那一半（脚本自己用 gh 开 PR 的历史路径不受影响）
        report.summary.push_str("没有配置发布平台，到推送为止。");
        return Ok(report);
    };
    if opts.open_review {
        let spec = ReviewSpec {
            owner: t.owner.clone(),
            repo: t.repo.clone(),
            head: branch.clone(),
            base: base.to_owned(),
            title: format!("发版 {tag}"),
            body: format!(
                "由 SupportEase 发布软件版本。\n\n版本 {tag}。{}\n\n合并之后由同一事务在 main 的 tip 上打 tag。",
                notes_line(&opts.notes)
            ),
        };
        let review = match hosting.find_open_review(&t.owner, &t.repo, &branch, base)? {
            // ★ **先找后建**（2026-10-06 真机踩的）：squash 合并之后同一个 head→base 还能
            //   再开 PR —— 先建后找的写法在"人手动合并过上一趟的 PR / 事务重跑"时，
            //   会开出第二份同样的 PR（v0.0.6 的 #46 就是这么来的，无害但难看）。
            //   已开着的直接复用；真没有才建（建挂了如实报错）。
            Some(r) => r,
            None => hosting.create_review(&spec)?,
        };
        report.review = Some(review);
        report.stage = ReleaseStage::ReviewOpened;
        report.summary.push_str("已建 PR。");

        // 合并：**只有人点过确认才做**（`opts.merge`）。没有自动合并、没有定时合并。
        if opts.merge {
            let merged = hosting.merge_review(
                &super::platform::ReviewId {
                    owner: t.owner.clone(),
                    repo: t.repo.clone(),
                    number: report.review.as_ref().map(|r| r.number).unwrap_or(0),
                },
                super::platform::MergeMethod::Squash,
            )?;
            report.review = Some(merged);
            report.stage = ReleaseStage::Merged;
            report.summary.push_str("已 squash 合并。");
        } else {
            report
                .summary
                .push_str("合并留给人 —— 合并之后重跑本事务接着打 tag。");
            return Ok(report);
        }
    }

    /* ---------- ⑥ 回 main、打 tag（★ tag 必须打在 main 的 tip 上） ---------- */

    if !git.tag_exists(&tag)? {
        // ★ 这一动会**切换开发者本机的当前分支** —— 界面与 CLI 都要在闸里明说（作者定死的
        // "破坏性命令要讲明白"）。squash 重写过提交，分支上打的 tag 指向不在 main 历史里的提交。
        git.fetch("origin")?;
        git.switch(base)?;
        git.pull_ff("origin", base)?;
        git.tag(&tag, &format!("{tag} {}", opts.notes.trim()))?;
        match target {
            Some(t) => git.push_ref_to_authenticated(
                &t.repository_url,
                &format!("refs/tags/{tag}"),
                &t.username,
                &t.token,
            )?,
            None => git.push_tag(&tag)?,
        }
        report
            .summary
            .push_str(&format!("已切到 {base} 并在 tip 上打 {tag}、推送。"));

        // ★★ **主线也推一份到发布仓库**（2026-10-05，作者定"后续走 Gitee"）：
        //   客户端的**数据源**读的就是仓库里的 `presets/delivery/`（release.json 也在里面，按 Manifest 声明取）
        //   （`raw/<branch>/…`）。只推 tag 的话，tag 在、main 上的数据源没过去 ——
        //   国内客户端连上 Gitee 之后看到的仍是上一版目录。
        //   这一步是**幂等**的：同一笔 main 推两次，第二次是 no-op。
        if let Some(t) = target {
            git.push_ref_to_authenticated(
                &t.repository_url,
                &format!("refs/heads/{base}"),
                &t.username,
                &t.token,
            )?;
            report.summary.push_str(&format!(
                "已把 {base} 推到发布仓库（数据源与 release.json 随之过去）。"
            ));
        }
    } else {
        report.summary.push_str("tag 已存在，沿用。");
    }
    report.stage = ReleaseStage::Tagged;

    /* ---------- ⑥½ 发布面同步（M6）：tag 与 main 推到 Gitee（幂等） ----------
     *
     * Release 挂在 tag 上，所以 **tag 必须在 Gitee 真实存在**；main 同步过去是
     * "数据源主线跟随发布仓库"（作者 2026-10-05 定）—— 客户端的数据源读的就是
     * 仓库里的 `presets/delivery/`。两步都幂等：同一笔重跑是 no-op / fast-forward。
     * Gitee 主线与本地分叉时如实报错（那是镜像没同步，得先解决，不能悄悄覆盖）。
     */
    if let Some(ch) = release {
        let g = ch.target;
        git.push_ref_to_authenticated(
            &g.repository_url,
            &format!("refs/tags/{tag}"),
            &g.username,
            &g.token,
        )?;
        git.push_ref_to_authenticated(
            &g.repository_url,
            &format!("refs/heads/{base}"),
            &g.username,
            &g.token,
        )?;
        report.summary.push_str(&format!(
            "已把 {tag} 与 {base} 推到发布仓库 {}（Release 的 tag 就位，数据源主线随之同步）。",
            g.repository_url
        ));
    }

    /* ---------- ⑦ 构建 macOS 安装包（第一阶段只做 macOS、不签名） ---------- */

    if !opts.build {
        report.summary.push_str("（本次不构建安装包）");
        return Ok(report);
    }
    let dmg = build_installer(repo_root)?;
    report.artifact = Some(ArtifactInfo {
        name: dmg.name.clone(),
        size: dmg.size,
        path: dmg.path.display().to_string(),
    });
    report.stage = ReleaseStage::Built;
    report.summary.push_str(&format!("已构建 {}。", dmg.name));

    /* ---------- ⑧ 建 Release + 上传安装包（M6：走 Gitee 发布通道） ---------- */

    // ★ **架构规则**（总纲 §5-M6）：软件版本的 Release 与附件**只发 Gitee**。
    //   没有发布通道 = preflight 就该拦下（这里兜底，话说清楚）。
    let Some(ch) = release else {
        return Err(AppError::invalid_argument(
            "软件版本的 Release 与安装包发布到 Gitee（总纲 §5-M6）——              先在设置里配好 Gitee 发布账户（仓库地址 / 用户名 / Token）",
        ));
    };
    let g = ch.target;
    let release = ch.hosting.create_release(&super::platform::ReleaseSpec {
        owner: g.owner.clone(),
        repo: g.repo.clone(),
        tag_name: tag.clone(),
        name: format!("{PRODUCT} {tag}"),
        body: release_body(&opts.notes),
    })?;
    report.release = Some(release.clone());
    report.stage = ReleaseStage::ReleaseCreated;
    report.summary.push_str("已建 Release。");

    let uploaded = ch.hosting.upload_asset(&super::platform::AssetUpload {
        owner: g.owner.clone(),
        repo: g.repo.clone(),
        release_id: release.id,
        name: dmg.name.clone(),
        content_type: super::platform::content_type_for(&dmg.name).to_owned(),
        path: dmg.path.clone(),
    })?;
    report.stage = ReleaseStage::AssetUploaded;
    report.summary.push_str(&format!(
        "已上传 {}（{} 字节）。",
        uploaded.name, uploaded.size
    ));

    /* ---------- ⑧之二  应用内更新用的 `.app.zip`（第五刀） ---------- */
    //
    // dmg 是"给人手动装的"，zip 是"给程序自己下载并替换的" —— 两种用途，两个文件。
    // ★ 这一步**失败不挡发版**：打不出 zip 就如实说"这一版只能打开下载页"，
    //   release.json 里就没有 `asset`，客户端照旧退回那条路（比整个发版失败好）。
    match build_app_zip(repo_root, &version) {
        Ok(zip) => {
            let zip_name = zip.name.clone();
            match ch.hosting.upload_asset(&super::platform::AssetUpload {
                owner: g.owner.clone(),
                repo: g.repo.clone(),
                release_id: release.id,
                name: zip.name.clone(),
                content_type: super::platform::content_type_for(&zip.name).to_owned(),
                path: zip.path.clone(),
            }) {
                Ok(put) => {
                    let sha = sha256_file(&zip.path).unwrap_or_default();
                    // ★ 下载地址**用平台回给我们的那个**（`browser_download_url`），
                    //   不自己拼 —— 2026-10-05 真机踩过：拼出来的
                    //   `…/releases/tag/v0.0.4/download/v0.0.4/…` 多了一层 `/tag/{tag}`，
                    //   GitHub 宽容地重定向了（能下），但**换到 Gitee 就未必**，
                    //   而"平台告诉我们它的下载页在哪"是唯一跨平台可靠的说法。
                    //   平台没给（某些 Gitee 版本）才回退到 Release 页。
                    let url = if put.url.trim().is_empty() {
                        release.url.clone()
                    } else {
                        put.url.clone()
                    };
                    report.zip = Some(crate::runtime::release_info::ReleaseAsset {
                        name: zip_name.clone(),
                        url,
                        size: put.size,
                        sha256: sha,
                    });
                    report.summary.push_str(&format!(
                        "已上传 {zip_name}（{} 字节，应用内更新用）。",
                        put.size
                    ));
                }
                Err(e) => report.summary.push_str(&format!(
                    "应用内更新的 zip 传不上去（{e}）—— 这一版退回「打开下载页」。"
                )),
            }
        }
        Err(e) => report.summary.push_str(&format!(
            "打不出应用内更新的 zip（{e}）—— 这一版退回「打开下载页」。"
        )),
    }

    /* ---------- ⑨ 写 `release.json` → 它自己的分支与 PR（**合并留给人**） ---------- */

    let info_rel = write_release_info(
        repo_root,
        &version,
        &opts.notes,
        &release.url,
        report.zip.as_ref(),
    )?;
    // ★ 分支名必须带合规前缀（`chore/`）—— 本机闸门⑥ 会拒没有前缀的分支名
    //   （`release/0.0.1` 这种在提交那一步会被钩子挡下来，白跑一趟）。
    let info_branch = format!("chore/release-{tag}");
    // 新分支从**当前**提交起（此刻人在 main 上、main 已含 ① 的合并结果）
    git.switch_new(&info_branch)?;
    let staged = git.stage_allowed_in(std::slice::from_ref(&info_rel), &RELEASE_STAGE_ALLOWLIST)?;
    if !git.has_staged()? {
        report.info_branch = Some(info_branch);
        report
            .summary
            .push_str("release.json 没有变化，没有可提交的。");
        return Ok(report);
    }
    git.commit(&format!("发布信息：{tag}"))?;
    report.info_committed_paths = staged;
    report.info_branch = Some(info_branch.clone());
    report.stage = ReleaseStage::InfoCommitted;

    match target {
        Some(t) => git.push_authenticated(&info_branch, &t.username, &t.token)?,
        None => git.push(&info_branch)?,
    }
    report.stage = ReleaseStage::InfoPushed;

    let spec = ReviewSpec {
        owner: t.owner.clone(),
        repo: t.repo.clone(),
        head: info_branch.clone(),
        base: base.to_owned(),
        title: format!("发布信息：{tag}"),
        body: format!(
            "由 SupportEase 发布软件版本后写。\n\n{}\n\n★ 合并之后客户端才看得到新版本。",
            notes_line(&opts.notes)
        ),
    };
    let info_review = match hosting.create_review(&spec) {
        Ok(r) => r,
        Err(_) => hosting
            .find_open_review(&t.owner, &t.repo, &info_branch, base)?
            .ok_or_else(|| {
                AppError::io("建 PR 失败，也找不到这一支上开着的 PR")
                    .with_detail("release.json 已经提交并推送 —— 重跑本事务会回读那一份")
            })?,
    };
    report.info_review = Some(info_review);
    report.stage = ReleaseStage::InfoReviewOpened;
    report.summary.push_str(
        "已写 presets/delivery/release.json 并开了 PR —— **合并它之后客户端才看得到新版本**。",
    );

    Ok(report)
}

/// ① 会被 stage 的候选（版本号真值 + 三个派生）。**具体进不进得去由白名单再判一次**。
const RELEASE_STAGE_CANDIDATES: [&str; 4] = [
    version::MANIFEST_REL,
    version::PACKAGE_REL,
    version::CONF_REL,
    version::LOCK_REL,
];

/// 构建出来的安装包（名字 + 大小 + 绝对路径）。
struct BuiltArtifact {
    name: String,
    size: u64,
    path: PathBuf,
}

/// **构建 macOS 安装包**（`npm run tauri -- build`）。
///
/// 走 npm script 而不是直接 `npx tauri`：那确保用的是本项目 `node_modules` 里那一版 CLI，
/// 而不是 PATH 上碰巧存在的另一个。
///
/// ★ 产物**只认 `bundle/dmg` 下的唯一一份 dmg**：找到 0 份如实报（不猜别的目录），
/// 找到多份也如实报（让人自己决定哪一份）—— "挑一个"这种事不该由程序偷偷做。
fn build_installer(repo_root: &Path) -> Result<BuiltArtifact, AppError> {
    let out = std::process::Command::new("npm")
        .args(["run", "tauri", "--", "build"])
        .current_dir(repo_root)
        .output()
        .map_err(|e| AppError::io("起不了 npm").with_detail(e.to_string()))?;
    if !out.status.success() {
        return Err(AppError::io("构建安装包失败")
            .with_detail(String::from_utf8_lossy(&out.stderr).trim().to_owned()));
    }

    // 两个候选目录：**找到哪个里有就用哪个**（顺序见 `DMG_DIRS` 的说明）
    let dir = DMG_DIRS
        .iter()
        .map(|d| repo_root.join(d))
        .find(|d| d.is_dir())
        .ok_or_else(|| {
            AppError::io("找不到安装包目录（构建没产出 dmg？）").with_detail(
                DMG_DIRS
                    .iter()
                    .map(|d| repo_root.join(d).display().to_string())
                    .collect::<Vec<_>>()
                    .join("、"),
            )
        })?;
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|e| {
            AppError::io("读不了安装包目录").with_detail(format!("{}：{e}", dir.display()))
        })?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("dmg"))
        .collect();
    found.sort();
    match found.as_slice() {
        [] => {
            Err(AppError::io("构建完了但 bundle/dmg 下没有 dmg")
                .with_detail(dir.display().to_string()))
        }
        [one] => {
            // ★ 追加「安装说明 + 终端快捷方式」：应用未签名，macOS 会拦"浏览器下载"的
            //   第一次打开（"已损坏"）—— 让 dmg 自己带着解法（2026-10-06 真机踩的）。
            //   说明文档能直接打开；终端快捷方式指向 Apple 签名的系统应用，也不会被拦。
            //   ★ 刻意不放可执行脚本 —— 脚本和应用一样被隔离拦下，形同虚设。
            let dmg_path = one.display().to_string();
            let patched = std::process::Command::new("bash")
                .args(["scripts/patch-dmg-extras.sh", &dmg_path])
                .current_dir(repo_root)
                .output()
                .map_err(|e| AppError::io("起不了 patch-dmg-extras").with_detail(e.to_string()))?;
            if !patched.status.success() {
                return Err(AppError::io("安装包追加说明失败")
                    .with_detail(String::from_utf8_lossy(&patched.stderr).trim().to_owned()));
            }
            let size = std::fs::metadata(one).map(|m| m.len()).unwrap_or(0);
            Ok(BuiltArtifact {
                name: super::platform::asset_name(
                    PRODUCT,
                    &version_of(repo_root),
                    arch_name(),
                    "dmg",
                ),
                size,
                path: one.clone(),
            })
        }
        many => Err(
            AppError::io("bundle/dmg 下不止一份 dmg —— 自己确认要发哪一份").with_detail(
                many.iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join("、"),
            ),
        ),
    }
}

/// 打应用内更新用的 `.app.zip`（**系统 `ditto`，不引压缩依赖**）。
///
/// 源是 `bundle/macos/<PRODUCT>.app`，产物落在 dmg 旁边：`SupportEase_<版本>_<arch>.app.zip`。
/// **找不到 `.app` 就如实报错**（不猜路径、不拿别的东西顶替）。
fn build_app_zip(repo_root: &Path, version: &str) -> Result<BuiltArtifact, AppError> {
    let macos_dir = repo_root.join(MACOS_BUNDLE_DIR);
    let mut apps: Vec<PathBuf> = std::fs::read_dir(&macos_dir)
        .map_err(|e| {
            AppError::io("找不到 macOS 应用包目录")
                .with_detail(format!("{}：{e}", macos_dir.display()))
        })?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("app"))
        .collect();
    apps.sort();
    let [app] = apps.as_slice() else {
        return Err(AppError::io("macOS 应用包目录里没有 .app").with_detail(
            apps.iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join("、"),
        ));
    };
    let name = super::platform::asset_name(PRODUCT, version, arch_name(), "app.zip");
    let zip_path = app.with_file_name(&name);
    let out = std::process::Command::new("ditto")
        // `--sequesterRsrc` 把扩展属性塞进 `__MACOSX`（下载解包后仍保留 Finder 扩展）
        .args(["-c", "-k", "--sequesterRsrc", "--keepParent"])
        .arg(app)
        .arg(&zip_path)
        .output()
        .map_err(|e| AppError::io("起不了系统 ditto").with_detail(e.to_string()))?;
    if !out.status.success() {
        return Err(AppError::io("打包 .app.zip 失败")
            .with_detail(String::from_utf8_lossy(&out.stderr).trim().to_owned()));
    }
    let size = std::fs::metadata(&zip_path).map(|m| m.len()).unwrap_or(0);
    Ok(BuiltArtifact {
        name,
        size,
        path: zip_path,
    })
}

/// 文件的 SHA-256（小写十六进制；读不到就空字符串 —— 那就是"不校验"）
fn sha256_file(path: &Path) -> Result<String, AppError> {
    crate::runtime::updater::sha256_file(path)
}

fn version_of(repo_root: &Path) -> String {
    version::app_version(repo_root).unwrap_or_else(|_| "unknown".to_owned())
}

/// 架构名（asset 命名用）。与 Rust 的 `std::env::consts::ARCH` 对齐。
fn arch_name() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "aarch64",
        "x86_64" => "x64",
        other => other,
    }
}

/// 写 `presets/delivery/release.json`（**上传成功之后才写**）。
///
/// ★ 顺序的意义：先说"有新版"再上传，用户点进去会撞上一个空的下载页。
/// 这里是"安装包已经在 Release 上了"之后才宣告。
fn write_release_info(
    repo_root: &Path,
    version: &str,
    notes: &str,
    url: &str,
    asset: Option<&crate::runtime::release_info::ReleaseAsset>,
) -> Result<String, AppError> {
    let mut value = serde_json::json!({
        "releaseSchema": RELEASE_SCHEMA,
        "version": version,
        "notes": notes.trim(),
        "url": url,
    });
    // ★ 有安装包就带上 `asset`（**可选格**：没有它客户端退回"打开下载页"，那条路仍然成立）
    if let Some(a) = asset {
        value["asset"] = serde_json::json!({
            "name": a.name,
            "url": a.url,
            "size": a.size,
            "sha256": a.sha256,
        });
    }
    // ★ 断代校验在**写盘之前**：坏 URL 不许落盘（总纲 §5-M6）
    validate_release_source(version, url, asset)?;
    let text = serde_json::to_string_pretty(&value)
        .map_err(|e| AppError::internal("发布信息序列化失败").with_detail(e.to_string()))?
        + "\n";
    let path = repo_root.join(RELEASE_INFO_REL);
    fsx::atomic::atomic_write(&path, text.as_bytes())?;
    // 写完立刻自己解析一遍：**写进去的东西必须是客户端读得懂的东西**（代次 / 版本形状）
    let bytes = std::fs::read(&path)?;
    release_info::parse(&bytes)?;
    Ok(RELEASE_INFO_REL.to_owned())
}

fn release_body(notes: &str) -> String {
    let n = notes.trim();
    if n.is_empty() {
        format!(
            "{PRODUCT} 的正式版本。\n\n（未签名：首次打开需要右键 → 打开绕过 Gatekeeper 警告。）"
        )
    } else {
        format!("{n}\n\n（未签名：首次打开需要右键 → 打开绕过 Gatekeeper 警告。）")
    }
}

fn notes_line(notes: &str) -> String {
    let n = notes.trim();
    if n.is_empty() {
        String::new()
    } else {
        format!("说明：{n}")
    }
}

fn commit_suffix(commit: Option<&str>) -> String {
    match commit {
        Some(c) => format!(" {c}"),
        None => String::new(),
    }
}

fn tag_points_at_head(git: &Git, tag: &str) -> bool {
    let head = git.head_short().unwrap_or_default();
    let tagged = git
        .rev_parse_short(&format!("{tag}^{{commit}}"))
        .unwrap_or_default();
    !head.is_empty() && head == tagged
}

/* ---------- 工作台入口（命令壳） ----------
 *
 * ★ 与 CLI（`src/bin/release.rs`）**同一个内核**：这里只做"取账户 → 调内核 → 记账"。
 * 步骤一行都不在这里 —— 两条入口共用一份实现，是这个设计的全部意义。
 */

/// 预检一格的结论。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreflightItem {
    /// 检查项的**稳定 id**（界面按 id 排序与定位，与文案无关）
    pub id: String,
    pub label: String,
    /// `pass` / `fail`
    pub status: String,
    /// 这一项的一句话结论（红了就写"为什么"）
    pub detail: String,
}

/// **发布软件版本的闸**（第四刀）：给人看"现在能不能发这一版"。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleasePreflight {
    pub items: Vec<PreflightItem>,
    /// 全过 = 可以发（界面据此点亮「确认发布」）
    pub can_release: bool,
    /// 当前版本真值（`x.y.z`）
    pub current_version: String,
    /// 这一轮要发的 tag（`v0.0.2`）
    pub tag: String,
    /// 当前分支
    pub branch: String,
    /// 有没有配发布账户（没配就只做本地那一半）
    pub has_account: bool,
}

/// **跑一遍闸**（只读）：工作区 / 分支 / 版本派生 / 发布账户 / tag 有没有被占 / 平台支不支持。
///
/// ★ `(async)` 且**一个字节都不写** —— 点几次都不会有副作用（界面上「重新检查」就靠这条）。
#[tauri::command(async)]
pub fn wb_release_preflight(
    app: tauri::AppHandle,
    version: Option<String>,
) -> Result<ReleasePreflight, AppError> {
    crate::ipc::traced("wb_release_preflight", |_| {
        let root = crate::fsx::paths::internal_root(&app)?;
        let repo = crate::workbench::paths::repo_root();
        let git = Git::open(&repo);
        let opts = ReleaseOptions {
            version,
            ..ReleaseOptions::default()
        };

        let current = version::app_version(&repo)?;
        let want = opts.version.clone().unwrap_or_else(|| current.clone());
        let tag = tag_of(&want);
        let branch = git.branch().unwrap_or_default();
        let base = opts.base_or_main();

        let mut items = Vec::new();
        let push =
            |items: &mut Vec<PreflightItem>, id: &str, label: &str, ok_: bool, detail: String| {
                items.push(PreflightItem {
                    id: id.to_owned(),
                    label: label.to_owned(),
                    status: if ok_ { "pass" } else { "fail" }.to_owned(),
                    detail,
                });
            };

        push(
            &mut items,
            "branch",
            "分支",
            !branch.is_empty() && branch != base,
            if branch == base {
                format!("不能在主线 `{base}` 上发版 —— 先切到一条分支")
            } else {
                format!("在 `{branch}` 上发，PR 进 `{base}`")
            },
        );

        let clean = git.is_clean()?;
        push(
            &mut items,
            "clean",
            "工作区",
            clean,
            if clean {
                "干净（发版会改版本号并提交，不能有别的东西混着）".to_owned()
            } else {
                "工作区不干净 —— 先提交或 stash".to_owned()
            },
        );

        let mismatch = version::derived_mismatch(&repo)?;
        push(
            &mut items,
            "version",
            "版本号一致",
            mismatch.is_empty(),
            if mismatch.is_empty() {
                format!("真值 {current} · 三处派生都跟着（版本号只有一处真值）")
            } else {
                format!("{} 与真值不一致", mismatch.join("、"))
            },
        );

        // tag 撞车：tag 已存在且指的是另一笔提交 = 这一版发出去过
        let tagged = git.tag_exists(&tag)?;
        let same = tagged && tag_points_at_head(&git, &tag);
        push(
            &mut items,
            "tag",
            "tag 没被占",
            !tagged || same,
            if !tagged {
                format!("`{tag}` 还没打过")
            } else if same {
                format!("`{tag}` 已经打在这一笔上（续跑上一趟）")
            } else {
                format!("`{tag}` 已经存在 —— 换一个版本号")
            },
        );

        // 发布账户（没配 → 只做本地那一半，如实说）
        let account = super::publish_tx::resolve_target(&root, None).ok();
        let has_account = account.is_some();
        push(
            &mut items,
            "account",
            "发布账户",
            has_account,
            match &account {
                Some(t) => format!("{} · {}/{}", t.platform, t.owner, t.repo),
                None => "还没配发布账户 —— 只能做本地那一半（不会建 Release）".to_owned(),
            },
        );
        // ★ M6（总纲 §5-M6）：软件版本的 Release 与附件**发布到 Gitee**（发布仓库），
        //   与 PR/tag 的 target 刻意分开。Gitee 账户没配 = 发不了版，进闸里明说。
        let gitee = super::publish_tx::resolve_target(&root, Some("gitee")).ok();
        push(
            &mut items,
            "release-channel",
            "软件发布通道（Gitee）",
            gitee.is_some(),
            match &gitee {
                Some(t) => format!(
                    "Gitee 发布仓库 · {}/{}（Release 与安装包住这里）",
                    t.owner, t.repo
                ),
                None => {
                    "Gitee 发布账户还没配 —— 0.0.6 起软件版本发布到 Gitee（总纲 §5-M6）".to_owned()
                }
            },
        );

        // ★ 开发构建发不得（2026-10-06 真机踩出来的）：发版要推进版本号，改的
        //   4 个文件（Cargo.toml / tauri.conf.json / package.json / Cargo.lock）
        //   **全在 `tauri dev` 文件监视器的清单里** —— 版本号一落盘，dev 就重建并
        //   重启应用，发版事务被杀在半路（分支推出去了、PR 没建、历史没记，
        //   界面上就是一次"闪退"）。真发版用**安装版工作台**或 release CLI。
        let dev_build = cfg!(debug_assertions);
        push(
            &mut items,
            "run-env",
            "发布环境",
            !dev_build,
            if dev_build {
                "这是开发构建（npm run tauri dev）—— 真发版会在版本号落盘时被 dev 重启杀掉。\
                 用安装版工作台或 release CLI 发版；这里只能演练"
                    .to_owned()
            } else {
                "安装版（发布面）—— 可以真发".to_owned()
            },
        );

        // ★ 第一阶段只做 macOS：构建那一步在别的系统上做不出来，进闸里明说
        let mac = std::env::consts::OS == "macos";
        push(
            &mut items,
            "platform-build",
            "构建环境",
            mac,
            if mac {
                format!("本机是 macOS —— 会构建 dmg（{}）", arch_name())
            } else {
                format!(
                    "本机是 {} —— 第一阶段只做 macOS 安装包",
                    std::env::consts::OS
                )
            },
        );

        let can_release = items.iter().all(|i| i.status == "pass");
        Ok(ReleasePreflight {
            items,
            can_release,
            current_version: current,
            tag,
            branch,
            has_account,
        })
    })
}

/// **发布软件版本**（一次手势）：预检过了、人点了「确认发布」才调。
///
/// ★ `(async)` + `spawn_blocking`：这一趟要**构建安装包 + 传几十 MB**，以分钟计。
/// 同步命令跑在主线程 = 界面冻住（作者铁律）。
#[tauri::command]
pub async fn wb_release_software(
    app: tauri::AppHandle,
    opts: ReleaseOptions,
) -> Result<ReleaseTxReport, AppError> {
    // ★ 与闸里 run-env 那一格同一条规矩的**硬闸**：dev 构建里真发版必死在半路
    //   （bump 的 4 个文件一落盘，`tauri dev` 就重建重启，事务被杀 —— 2026-10-06）。
    //   演练（dry_run）一个字节都不写，放行。
    if cfg!(debug_assertions) && !opts.dry_run {
        return Err(AppError::invalid_argument(
            "开发构建里不能真发版 —— 版本号一落盘，dev 的文件监视器就重启应用，发版事务被杀在半路",
        )
        .with_detail(
            "真发版用安装版工作台（npm run tauri build 出的安装包）或 release CLI；这里只能演练",
        ));
    }
    let root = crate::fsx::paths::internal_root(&app)?;
    let repo = crate::workbench::paths::repo_root();
    let target = super::publish_tx::resolve_target(&root, None).ok();
    // M6（总纲 §5-M6）：发布通道 = Gitee 账户（与 target 刻意分开）。没配 → None，
    // build=true 时 run() 会如实报错（preflight 已经先拦过一道）。
    let release_target = super::publish_tx::resolve_target(&root, Some("gitee")).ok();
    let task = tauri::async_runtime::spawn_blocking(move || {
        crate::ipc::traced("wb_release_software", |_| {
            let (hosting, target) = match &target {
                Some(t) => (
                    Some(
                        super::platform::hosting(&t.platform, t.token.clone()).ok_or_else(
                            || AppError::invalid_argument(format!("不认识的平台：{}", t.platform)),
                        )?,
                    ),
                    Some(t.clone()),
                ),
                None => (None, None),
            };
            let release_hosting = release_target
                .as_ref()
                .map(|t| {
                    super::platform::hosting(&t.platform, t.token.clone()).ok_or_else(|| {
                        AppError::invalid_argument(format!("不认识的平台：{}", t.platform))
                    })
                })
                .transpose()?;
            let release_channel =
                release_target
                    .as_ref()
                    .zip(release_hosting.as_ref())
                    .map(|(t, h)| ReleaseChannel {
                        hosting: h.as_ref(),
                        target: t,
                    });
            let report = run(
                &repo,
                &opts,
                target.as_ref(),
                hosting.as_ref().map(|h| h.as_ref()),
                release_channel.as_ref(),
            )?;
            // 记账：这一趟留在 `release-history.json`（**与发布预设那本账分开**）
            let _ = super::release_history::append(
                &root,
                super::release_history::ReleaseRecord::from_report(
                    &report,
                    crate::workbench::clock::now_iso8601(),
                ),
            );
            Ok(report)
        })
    });
    task.await
        .map_err(|e| AppError::internal("发布软件版本没跑到终局").with_detail(e.to_string()))?
}

/// 读**软件版本**的发布历史。★ 与 `wb_publish_history`（预设那本）不是同一本账。
#[tauri::command(async)]
pub fn wb_release_history(
    app: tauri::AppHandle,
) -> Result<super::release_history::ReleaseHistory, AppError> {
    crate::ipc::traced("wb_release_history", |_| {
        let root = crate::fsx::paths::internal_root(&app)?;
        super::release_history::load(&root)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 阶段名与 serde 的 camelCase **逐字一致** —— 前端按这个字符串画图，
    /// 两边漂移的表现是界面上某个阶段永远不亮，而编译照样过。
    #[test]
    fn stage_wire_names_match_the_serialized_shape() {
        for stage in [
            ReleaseStage::BlockedPreflight,
            ReleaseStage::Ready,
            ReleaseStage::VersionBumped,
            ReleaseStage::Committed,
            ReleaseStage::Pushed,
            ReleaseStage::ReviewOpened,
            ReleaseStage::Merged,
            ReleaseStage::Tagged,
            ReleaseStage::Built,
            ReleaseStage::ReleaseCreated,
            ReleaseStage::AssetUploaded,
            ReleaseStage::InfoCommitted,
            ReleaseStage::InfoPushed,
            ReleaseStage::InfoReviewOpened,
        ] {
            let json = serde_json::to_value(stage).expect("序列化");
            assert_eq!(json, serde_json::json!(stage.wire_name()), "{stage:?}");
        }
    }

    /// ★ **预检零写入**：不干净的临时仓库 → `BlockedPreflight`，且**一个字节都没改**。
    #[test]
    fn a_blocked_preflight_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let git = init_repo(root, "feat/demo");
        // 弄脏它
        crate::fsx::atomic::atomic_write(&root.join("dirty.txt"), b"x").unwrap();
        assert!(!git.is_clean().unwrap());

        let report = run(root, &ReleaseOptions::default(), None, None, None).expect("该返回报告");
        assert_eq!(report.stage, ReleaseStage::BlockedPreflight);
        assert!(
            report.blocked_reasons.iter().any(|r| r.contains("不干净")),
            "该点名工作区不干净：{:?}",
            report.blocked_reasons
        );
        assert!(report.committed_paths.is_empty(), "一个字节都不该写");
    }

    /// 在主线上发版也要被拦（发版是"把分支上的改动发出去"）。
    #[test]
    fn releasing_from_the_mainline_is_blocked() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        init_repo(root, "main");
        let report = run(root, &ReleaseOptions::default(), None, None, None).expect("该返回报告");
        assert_eq!(report.stage, ReleaseStage::BlockedPreflight);
        assert!(
            report.blocked_reasons.iter().any(|r| r.contains("主线")),
            "{:?}",
            report.blocked_reasons
        );
    }

    /// 演练：预检过了就停 —— `Ready`，不动一个字节。
    #[test]
    fn a_dry_run_stops_at_ready() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        init_repo(root, "feat/demo");
        let opts = ReleaseOptions {
            dry_run: true,
            ..ReleaseOptions::default()
        };
        let report = run(root, &opts, None, None, None).expect("该返回报告");
        assert_eq!(report.stage, ReleaseStage::Ready);
        assert!(report.committed_paths.is_empty());
    }

    /// ★ **写出来的 `release.json` 必须是客户端读得懂的那一份**：写完立刻自己解析一遍
    /// （代次、版本形状、非空 version）—— 这是"发布侧写"与"客户端读"之间唯一的接缝。
    #[test]
    fn the_written_release_info_is_readable_by_the_client() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("presets")).unwrap();
        let rel = write_release_info(
            root,
            "0.0.2",
            "修了几个问题",
            "https://host/r/tag/v0.0.2",
            None,
        )
        .expect("该写得出来");
        assert_eq!(rel, RELEASE_INFO_REL);
        let bytes = std::fs::read(root.join(&rel)).unwrap();
        let info = release_info::parse(&bytes).expect("客户端该读得懂");
        assert_eq!(info.version, "0.0.2");
        assert_eq!(info.notes, "修了几个问题");
        assert_eq!(info.url, "https://host/r/tag/v0.0.2");
        // 它**不进** catalog / 发布预设那一批 —— 两条链不混
        assert!(
            rel.starts_with("presets/"),
            "release.json 在交付根 presets/delivery/ 里：{rel}"
        );
    }

    /* ---------- M6：断代校验（release.json 的 URL 来源） ---------- */

    /// 0.0.6 起：URL 一律不许指 GitHub —— Release 页与安装包各拦一道。
    #[test]
    fn a_gitee_epoch_release_refuses_github_urls() {
        let zip = crate::runtime::release_info::ReleaseAsset {
            name: "SupportEase_0.0.6_aarch64.app.zip".to_owned(),
            url: "https://github.com/o/r/releases/download/v0.0.6/x.zip".to_owned(),
            size: 1,
            sha256: "ab".to_owned(),
        };
        let e = validate_release_source(
            "0.0.6",
            "https://gitee.com/o/r/releases/tag/v0.0.6",
            Some(&zip),
        )
        .expect_err("安装包指 GitHub 该拒");
        assert!(e.message.contains("必须是 Gitee"), "{}", e.message);
        assert!(
            e.message.contains("安装包 https://github.com"),
            "{}",
            e.message
        );

        let e =
            validate_release_source("0.0.6", "https://github.com/o/r/releases/tag/v0.0.6", None)
                .expect_err("Release 页指 GitHub 也该拒");
        assert!(
            e.message.contains("Release 页 https://github.com"),
            "{}",
            e.message
        );
    }

    /// Gitee 的 URL 照常过；断代线之前（≤0.0.5）的 GitHub 历史档照旧合法 ——
    /// 那是事实记录，不改写（总纲 §1③：测试版断代，不是抹历史）。
    #[test]
    fn gitee_urls_pass_and_the_pre_epoch_history_stays_legal() {
        let zip = crate::runtime::release_info::ReleaseAsset {
            name: "SupportEase_0.0.6_aarch64.app.zip".to_owned(),
            url: "https://gitee.com/o/r/releases/download/v0.0.6/x.zip".to_owned(),
            size: 1,
            sha256: "ab".to_owned(),
        };
        validate_release_source(
            "0.0.6",
            "https://gitee.com/o/r/releases/tag/v0.0.6",
            Some(&zip),
        )
        .expect("全 Gitee 该过");

        // 断代前：v0.0.5 的 GitHub 档是历史事实
        validate_release_source("0.0.5", "https://github.com/o/r/releases/tag/v0.0.5", None)
            .expect("断代前的历史档照旧合法");
    }

    /// ★ **假平台跑一遍**：本地那一半（预检 → 版本号 → 提交 → 推送 → PR → 合并 →
    /// 切 main → 打 tag → 推 tag）在临时仓库里真跑通，阶段快照与 `publish_tx` 同形
    /// （一个 `stage` + 一句话 `summary`，没有把过程压扁的 bool）。
    ///
    /// ★ 这里的「合并」是**假平台返回的 merged**，不是真的把分支合进 main ——
    /// 它验的是**编排顺序**（合并之后才打 tag、tag 推的是 `refs/tags/…`），
    /// 不是合并语义（那是平台的活）。真机上这一步由平台完成。
    #[test]
    fn a_fake_hosting_runs_the_local_half_up_to_the_tag() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        // 一个裸仓库当 origin（file:// 也能推，这样"推送"这一步是真跑的）
        let bare = tempfile::tempdir().unwrap();
        let bgit = |args: &[&str]| {
            let out = std::process::Command::new("git")
                .args(args)
                .current_dir(bare.path())
                .output()
                .expect("起 git");
            assert!(out.status.success(), "git {args:?} 失败");
        };
        bgit(&["init", "-q", "--bare"]);

        let git = init_repo(root, "main");
        std::process::Command::new("git")
            .args(["remote", "add", "origin", &bare.path().to_string_lossy()])
            .current_dir(root)
            .output()
            .unwrap();
        git.push("main").expect("推 main");
        std::process::Command::new("git")
            .args(["checkout", "-q", "-b", "feat/demo"])
            .current_dir(root)
            .output()
            .unwrap();

        let fake = FakeHosting::default();
        // 发布目标（真机上来自发布账户配置 + 凭据文件；这里用裸仓库的路径当 remote，
        // `remote_matches` 才过得去）。凭据是假的 —— 裸仓库走的是本地传输，不认证。
        let target = PublishTarget {
            platform: "fake".to_owned(),
            repository_url: bare.path().to_string_lossy().to_string(),
            username: "t".to_owned(),
            token: "fake-token".to_owned(),
            owner: "o".to_owned(),
            repo: "r".to_owned(),
        };
        let opts = ReleaseOptions {
            version: Some("0.0.2".to_owned()),
            notes: "第一版之后的第一个小修".to_owned(),
            merge: true,
            build: false, // 构建要几分钟，这一条只验编排；构建与上传在真机上验
            ..ReleaseOptions::default()
        };
        let report = run(root, &opts, Some(&target), Some(&fake), None).expect("该跑通");

        assert_eq!(report.stage, ReleaseStage::Tagged, "{:?}", report.summary);
        assert_eq!(report.version, "0.0.2");
        assert_eq!(report.tag, "v0.0.2");
        assert!(report.commit.is_some(), "该有提交 sha：{}", report.summary);
        assert!(report.review.is_some(), "该建了 PR");
        assert_eq!(
            report.review.as_ref().map(|r| r.state),
            Some(super::super::platform::ReviewState::Merged)
        );
        // 版本号那一批进索引了（真值 + 三个派生）
        assert!(
            report
                .committed_paths
                .contains(&version::MANIFEST_REL.to_owned()),
            "{:?}",
            report.committed_paths
        );
        // 真机上这一步之后人切回 main 会看到 tag —— 内核确实切过去了
        assert_eq!(Git::open(root).branch().unwrap(), "main");
        assert!(Git::open(root).tag_exists("v0.0.2").unwrap());
        assert!(fake.releases.borrow().is_empty(), "没构建就不该建 Release");
    }

    /// 假平台：记录调用、返回平台无关的形状。**判据用它跑全流程，不真发一个字节**。
    #[derive(Default)]
    struct FakeHosting {
        releases: std::cell::RefCell<Vec<super::super::platform::ReleaseSpec>>,
    }

    impl super::super::platform::Hosting for FakeHosting {
        fn kind(&self) -> &'static str {
            "fake"
        }
        fn create_review(
            &self,
            spec: &super::super::platform::ReviewSpec,
        ) -> Result<RemoteReview, AppError> {
            Ok(RemoteReview {
                platform: "fake".to_owned(),
                number: 1,
                url: format!("https://fake/{}/{}", spec.owner, spec.repo),
                state: super::super::platform::ReviewState::Open,
                checks: super::super::platform::ChecksSummary::None,
                title: spec.title.clone(),
                head: spec.head.clone(),
                base: spec.base.clone(),
            })
        }
        fn get_review(
            &self,
            id: &super::super::platform::ReviewId,
        ) -> Result<RemoteReview, AppError> {
            self.create_review(&super::super::platform::ReviewSpec {
                owner: id.owner.clone(),
                repo: id.repo.clone(),
                head: "h".to_owned(),
                base: "main".to_owned(),
                title: "t".to_owned(),
                body: String::new(),
            })
        }
        fn merge_review(
            &self,
            id: &super::super::platform::ReviewId,
            _method: super::super::platform::MergeMethod,
        ) -> Result<RemoteReview, AppError> {
            let mut r = self.get_review(id)?;
            r.state = super::super::platform::ReviewState::Merged;
            Ok(r)
        }
        fn find_open_review(
            &self,
            owner: &str,
            repo: &str,
            head: &str,
            base: &str,
        ) -> Result<Option<RemoteReview>, AppError> {
            let spec = super::super::platform::ReviewSpec {
                owner: owner.to_owned(),
                repo: repo.to_owned(),
                head: head.to_owned(),
                base: base.to_owned(),
                title: "t".to_owned(),
                body: String::new(),
            };
            self.create_review(&spec).map(Some)
        }
        fn create_release(
            &self,
            spec: &super::super::platform::ReleaseSpec,
        ) -> Result<RemoteRelease, AppError> {
            self.releases.borrow_mut().push(spec.clone());
            Ok(RemoteRelease {
                platform: "fake".to_owned(),
                id: 7,
                tag_name: spec.tag_name.clone(),
                url: format!("https://fake/{}/releases/tag/{}", spec.repo, spec.tag_name),
            })
        }
        fn upload_asset(
            &self,
            up: &super::super::platform::AssetUpload,
        ) -> Result<super::super::platform::UploadedAsset, AppError> {
            Ok(super::super::platform::UploadedAsset {
                name: up.name.clone(),
                size: 42,
                url: "https://fake/download".to_owned(),
            })
        }
    }

    /// 造一个最小仓库：git init + 一条提交 + 版本号真值那四个文件。
    fn init_repo(root: &Path, branch: &str) -> Git {
        let ok = |args: &[&str]| {
            let out = std::process::Command::new("git")
                .args(args)
                .current_dir(root)
                .output()
                .expect("起 git");
            assert!(
                out.status.success(),
                "git {args:?} 失败：{}",
                String::from_utf8_lossy(&out.stderr)
            );
        };
        ok(&["init", "-q"]);
        ok(&["config", "user.email", "t@example.com"]);
        ok(&["config", "user.name", "t"]);
        // 开发机上装了"不许在 main 上直接提交"的全局钩子（`scripts/hooks`），临时仓库也会继承。
        // 判据要在**任意分支**上造初始提交（含 main 那一档），所以这里把钩子路径指向一个
        // 不存在的目录 —— 仓库自己的纪律与这条判据要验的事（分支判定）不是同一件事。
        ok(&["config", "core.hooksPath", "/dev/null"]);
        ok(&["checkout", "-q", "-b", branch]);

        std::fs::create_dir_all(root.join("src-tauri")).unwrap();
        std::fs::create_dir_all(root.join("presets")).unwrap();
        crate::fsx::atomic::atomic_write(
            &root.join(version::MANIFEST_REL),
            b"[package]\nname = \"mkp-support-ease\"\nversion = \"0.0.1\"\n",
        )
        .unwrap();
        crate::fsx::atomic::atomic_write(
            &root.join(version::PACKAGE_REL),
            b"{\n  \"name\": \"mkp-support-ease\",\n  \"version\": \"0.0.1\"\n}",
        )
        .unwrap();
        crate::fsx::atomic::atomic_write(
            &root.join(version::CONF_REL),
            b"{\n  \"productName\": \"SupportEase\",\n  \"version\": \"0.0.1\"\n}",
        )
        .unwrap();
        crate::fsx::atomic::atomic_write(
            &root.join(version::LOCK_REL),
            b"[[package]]\nname = \"mkp-support-ease\"\nversion = \"0.0.1\"\n",
        )
        .unwrap();
        let git = Git::open(root);
        let candidates: Vec<String> = RELEASE_STAGE_CANDIDATES
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        git.stage_allowed_in(&candidates, &RELEASE_STAGE_ALLOWLIST)
            .unwrap();
        git.commit("初始").unwrap();
        git
    }
}
