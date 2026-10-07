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
use sysinfo::{Pid, ProcessesToUpdate, System};

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
/// `tauri build` 的**产物根**（相对仓库根）的两个候选。
///
/// ★ 两个候选都在：本仓库的 `CARGO_TARGET_DIR` 指到**仓库根**的 `target/`（真机实测
/// 2026-10-04 构建落在 `target/release/bundle/dmg/`），而老机器 / 别的配置下会在
/// `src-tauri/target/…`。**按顺序找，找到哪个里有就用哪个** —— 两个都没有才报错。
pub const BUNDLE_ROOT_CANDIDATES: [&str; 2] =
    ["target/release/bundle", "src-tauri/target/release/bundle"];

/// `.app` 所在的那一层（macOS 打应用内更新的 zip 用；与 [`BUNDLE_ROOT_CANDIDATES`] 同一个
/// "仓库根 target 优先"的前提）
pub const MACOS_BUNDLE_DIR: &str = "target/release/bundle/macos";

/// **宿主平台打出来的那一份安装包**（第四刀只做 macOS；2026-10-07 起两端各在自己的机器上打）。
///
/// 为什么不在一台机器上打两个平台的包：macOS 的 dmg 要 `hdiutil`、Windows 的 NSIS 要
/// `makensis` —— 交叉打包不是"多传一个参数"的事。**各自平台打各自的包**是这一阶段的决定，
/// 也是这个类型存在的理由：谁也别假装自己能打别人的。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstallerPlan {
    /// `bundle/` 下的子目录（`dmg` / `nsis`）
    pub dir: &'static str,
    /// 认的产物后缀，同时也是 asset 命名的后缀（`dmg` / `exe`）
    pub ext: &'static str,
    /// 给人看的一句话
    pub label: &'static str,
}

/// 本机能打什么。**别的一律 `None`** —— 不假装能构建，也不悄悄换一种产物顶上。
pub fn installer_plan() -> Option<InstallerPlan> {
    if cfg!(target_os = "macos") {
        Some(InstallerPlan {
            dir: "dmg",
            ext: "dmg",
            label: "dmg",
        })
    } else if cfg!(target_os = "windows") {
        Some(InstallerPlan {
            dir: "nsis",
            ext: "exe",
            label: "NSIS 安装包",
        })
    } else {
        None
    }
}

/// `bundle/<子目录>` 在盘上的实际位置：两个候选按顺序找，**找到哪个里有就用哪个**。
fn bundle_dir(repo_root: &Path, sub: &str) -> Option<PathBuf> {
    BUNDLE_ROOT_CANDIDATES
        .iter()
        .map(|root| repo_root.join(root).join(sub))
        .find(|dir| dir.is_dir())
}
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
    // ★ 版本撞车**按平台判**（2026-10-07）：tag 打了不等于这一版发不了 ——
    //   同一版本可以补别的平台的包（见 [`ReleaseSlot`]）。
    //   不构建（`--no-build`）的那几趟没有"平台"可言，照老规矩：tag 不是 HEAD 就当它被占。
    let mut slot = ReleaseSlot::Fresh;
    if opts.build {
        match release_slot(&git, &tag, release) {
            Ok(ReleaseSlot::Taken) => blocked.push(format!(
                "tag `{tag}` 上已经有{}了 —— 这一版这一平台发过了，换一个版本号\
                 （同一版本可以补别的平台，同一个平台不重发）",
                installer_plan().map(|p| p.label).unwrap_or("安装包")
            )),
            Ok(s) => slot = s,
            // 问不到（没发布通道 / 网络）就不往下走：不猜"还没发过"
            Err(e) => blocked.push(e.message),
        }
    } else if git.tag_exists(&tag)? && !tag_points_at_head(&git, &tag) {
        blocked.push(format!(
            "tag `{tag}` 已经存在。换一个版本号，或先确认那一版发到哪了"
        ));
    }
    let append = slot == ReleaseSlot::Append;
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

    /* ---------- ③④ 版本号 / 提交 / 推送（**补平台那趟全都跳过**） ---------- */

    // ★ **补平台**（同一版本发第二个平台，2026-10-07 作者要的）：版本号、提交、PR、tag
    //   上一趟都做完了 —— 这一趟只做 ⑦ 构建 → ⑧ 上传 → ⑨ 发布信息。
    //   重做一遍的后果不是"多跑几步"：bump 会拿人填的目标（= 当前真值）把版本号改回去、
    //   ④ 会把分支再推一遍、⑤ 会再开一份一模一样的 PR。
    if append {
        report.summary.push_str(&format!(
            "{tag} 已经发过 —— 这一趟只往它上面补本平台（{}）的安装包：\
             不动版本号、不提交、不开 PR、不重打 tag。",
            installer_plan().map(|p| p.label).unwrap_or("安装包")
        ));
    } else {
        /* ③ 推进版本号（给了新版本才动） */

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
    }

    /* ---------- ⑤ ① 建 PR →（可选）合并 ---------- */

    // ★ 平台没配 = CLI 只做本地那一半（脚本自己用 gh 开 PR 的历史路径不受影响）
    let (Some(hosting), Some(t)) = (hosting, target) else {
        report.summary.push_str("没有配置发布平台，到推送为止。");
        return Ok(report);
    };
    // ★ **补平台那趟不开 PR**：这一版的分支早合进 main 了（tag 就在 main 的 tip 上），
    //   再开一份一模一样的 PR 只会让人以为又要发一次。
    if !append && opts.open_review {
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

    // ★ **主线的源头一律取 `origin/{base}`，不是本地 `{base}`**（2026-10-07 真机踩的）：
    //   合并发生在**平台上**，本地那个 {base} 常常停在旧提交（这台机器上就落后一个 PR）。
    //   补平台那趟（Append）根本不走下面 ⑥ 的切分支 + pull —— 拿本地 {base} 往发布仓库推
    //   就是**非快进**，被 pre-push 闸③ 拦下，界面上只剩一句"git push 失败了"，
    //   原因得翻日志才看得见（见 `git.rs::push_ref_to_authenticated` 的错误形状）。
    //   与镜像同步（`publish_tx::mirror_sync`）同一条规矩：先 fetch，拿远端跟踪引用当源；
    //   读不到就不推 —— **宁可不推，也不推一份旧的**。
    git.fetch("origin")?;
    let remote_base = format!("refs/remotes/origin/{base}");
    if git.rev_parse_short(&remote_base).is_err() {
        return Err(AppError::invalid_argument(format!(
            "本机读不到 {remote_base} —— 读不到主线就不敢往发布仓库推（也许远端还没有 {base}）"
        )));
    }

    if !git.tag_exists(&tag)? {
        // ★ 这一动会**切换开发者本机的当前分支** —— 界面与 CLI 都要在闸里明说（作者定死的
        // "破坏性命令要讲明白"）。squash 重写过提交，分支上打的 tag 指向不在 main 历史里的提交。
        git.switch(base)?;
        git.pull_ff("origin", base)?;
        git.tag(&tag, &format!("{tag} {}", opts.notes.trim()))?;
        match target {
            Some(t) => {
                git.push_ref_to_authenticated(
                    &t.repository_url,
                    &format!("refs/tags/{tag}"),
                    &t.username,
                    &t.token,
                )?;
            }
            None => {
                git.push_tag(&tag)?;
            }
        }
        report
            .summary
            .push_str(&format!("已切到 {base} 并在 tip 上打 {tag}、推送。"));

        // ★★ **主线也推一份到目标仓库**（2026-10-05，作者定"后续走 Gitee"）：
        //   客户端的**数据源**读的就是仓库里的 `presets/delivery/`（release.json 也在里面，按 Manifest 声明取）
        //   （`raw/<branch>/…`）。只推 tag 的话，tag 在、main 上的数据源没过去 ——
        //   国内客户端连上 Gitee 之后看到的仍是上一版目录。
        //   这一步是**幂等**的：同一笔 main 推两次，第二次是 no-op。
        //
        // ★ 但**目标仓库就是本机 origin 时跳过**（2026-10-07）：合并本来就发生在 origin 上，
        //   `origin/{base}` 已经是刚合出来的那一笔 —— 对着自己推一次 no-op，会被本地
        //   pre-push 闸②（PR-only，见 main 就拦）挡下，白报一次"git push 失败了"。
        //   只有目标**不是** origin（真镜像）时这一推才有内容。
        if let Some(t) = target {
            if git.remote_matches(&t.repository_url)? {
                report.summary.push_str(&format!(
                    "{base} 由平台上的 PR 合并推进，目标仓库就是本机 origin —— 主线不必再推一次。"
                ));
            } else {
                git.push_ref_to_authenticated(
                    &t.repository_url,
                    &format!("{remote_base}:refs/heads/{base}"),
                    &t.username,
                    &t.token,
                )?;
                report.summary.push_str(&format!(
                    "已把 {base} 推到发布仓库（数据源与 release.json 随之过去）。"
                ));
            }
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
     * 主线的**源是 `origin/{base}`**（上面刚 fetch 过）：本地 {base} 旧不旧都不影响这一推 ——
     * 拿本地分支当源正是 2026-10-07 那次"git push 失败了"的成因（非快进，闸③ 拦）。
     * 远端有本地没有的提交时如实报错（那是镜像分叉，得先解决，不能悄悄覆盖）。
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
            &format!("{remote_base}:refs/heads/{base}"),
            &g.username,
            &g.token,
        )?;
        report.summary.push_str(&format!(
            "已把 {tag} 与 {base} 推到发布仓库 {}（Release 的 tag 就位，数据源主线随之同步）。",
            g.repository_url
        ));
    }

    /* ---------- ⑦ 构建本平台的安装包（macOS → dmg / Windows → NSIS；都不签名） ---------- */

    if !opts.build {
        report.summary.push_str("（本次不构建安装包）");
        return Ok(report);
    }
    let built = build_installer(repo_root)?;
    report.artifact = Some(ArtifactInfo {
        name: built.name.clone(),
        size: built.size,
        path: built.path.display().to_string(),
    });
    report.stage = ReleaseStage::Built;
    report.summary.push_str(&format!("已构建 {}。", built.name));

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
        name: built.name.clone(),
        content_type: super::platform::content_type_for(&built.name).to_owned(),
        path: built.path.clone(),
    })?;
    report.stage = ReleaseStage::AssetUploaded;
    report.summary.push_str(&format!(
        "已上传 {}（{} 字节）。",
        uploaded.name, uploaded.size
    ));

    /* ---------- ⑧之二  应用内更新用的 `.app.zip`（第五刀；**只有 macOS 有这条路**） ---------- */
    //
    // dmg 是"给人手动装的"，zip 是"给程序自己下载并替换的" —— 两种用途，两个文件。
    // ★ 这一步**失败不挡发版**：打不出 zip 就如实说"这一版只能打开下载页"，
    //   release.json 里就没有 `asset`，客户端照旧退回那条路（比整个发版失败好）。
    //
    // ★ 2026-10-07：客户端那条应用内更新（`runtime::updater`）替换的是
    //   `/Applications/SupportEase.app`，靠系统 `ditto` / `unzip` —— **它只认 macOS**。
    //   Windows 版没有对应的替换路径（NSIS 静默安装那套客户端还没接），所以这里**不做**、
    //   `release.json` 里也就没有 `asset`：客户端的 Windows 版走"打开下载页"，
    //   而 Release 页上那份 `.exe` 就在那儿。**不假装打一个 zip 出来顶上。**
    if cfg!(target_os = "macos") {
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
    } else {
        report
            .summary
            .push_str("应用内更新（.app.zip）只对 macOS 包做 —— 这一版客户端走「打开下载页」。");
    }

    /* ---------- ⑨ 写 `release.json` → 它自己的分支与 PR（**合并留给人**） ---------- */

    // ★ 先算一遍"它该长什么样"（**纯函数**，不落盘）：与盘上那份一字不差就**不动分支、
    //   不开 PR**（2026-10-07）。补平台那一趟正是这种情况 —— asset 保留的是上一份，
    //   内容一个字节都不变；照老写法会切到一条不相干的分支上、再开一份空的 PR。
    let previous = previous_release_info(repo_root);
    // ★ 补平台那一趟**不改软件发布信息**：`notes` / `url` 沿用盘上那一份 —— 人这一趟填的
    //   说明只进 Release 正文那一步，而那一步对**已经存在**的 Release 本来就不改
    //   （`create_release` 是回读，不是改）。于是 release.json 一个字节都不动：
    //   既不会多开一份 PR，也不会把上一趟发布的那句话悄悄换掉。
    let (info_notes, info_url) = match previous
        .as_ref()
        .filter(|p| append && p.version.trim() == version.trim())
    {
        Some(p) => (p.notes.clone(), p.url.clone()),
        None => (opts.notes.clone(), release.url.clone()),
    };
    let want_info = release_info_text(
        &version,
        &info_notes,
        &info_url,
        report.zip.as_ref(),
        previous.as_ref(),
    )?;
    if std::fs::read_to_string(repo_root.join(RELEASE_INFO_REL))
        .ok()
        .as_deref()
        == Some(want_info.as_str())
    {
        report
            .summary
            .push_str("release.json 与盘上那份一字不差（这一趟不改软件发布信息），没有要提交的。");
        return Ok(report);
    }

    let info_rel = write_release_info(
        repo_root,
        &version,
        &info_notes,
        &info_url,
        report.zip.as_ref(),
    )?;
    // ★ 分支名必须带合规前缀（`chore/`）—— 本机闸门⑥ 会拒没有前缀的分支名
    //   （`release/0.0.1` 这种在提交那一步会被钩子挡下来，白跑一趟）。
    let info_branch = format!("chore/release-{tag}");
    // ★ 那条分支**已经存在就切过去**，不再新建（2026-10-07）：重跑 / 补平台时它还在
    //   （上一趟建的），`switch -c` 会以"已存在"失败 —— 那不是发版失败。
    if git.branch_exists(&info_branch)? {
        git.switch(&info_branch)?;
    } else {
        // 新分支从**当前**提交起（此刻人在 main 上、main 已含 ① 的合并结果）
        git.switch_new(&info_branch)?;
    }
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
        Some(t) => {
            git.push_authenticated(&info_branch, &t.username, &t.token)?;
        }
        None => {
            git.push(&info_branch)?;
        }
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

/// **构建本平台的安装包**（`npm run tauri -- build`）。
///
/// 走 npm script 而不是直接 `npx tauri`：那确保用的是本项目 `node_modules` 里那一版 CLI，
/// 而不是 PATH 上碰巧存在的另一个。
///
/// ★ 产物**只认 `bundle/<本平台目录>` 下的唯一一份**：找到 0 份如实报（不猜别的目录），
/// 找到多份也如实报（让人自己决定哪一份）—— "挑一个"这种事不该由程序偷偷做。
fn build_installer(repo_root: &Path) -> Result<BuiltArtifact, AppError> {
    let plan = installer_plan().ok_or_else(|| {
        AppError::invalid_argument(format!(
            "本机是 {} —— 安装包只在 macOS（dmg）与 Windows（NSIS）上构建",
            std::env::consts::OS
        ))
    })?;

    let out = npm_build(repo_root)?;
    if !out.status.success() {
        return Err(AppError::io("构建安装包失败")
            .with_detail(String::from_utf8_lossy(&out.stderr).trim().to_owned()));
    }

    let dir = bundle_dir(repo_root, plan.dir).ok_or_else(|| {
        AppError::io(format!("找不到安装包目录（构建没产出 {}？）", plan.ext)).with_detail(
            BUNDLE_ROOT_CANDIDATES
                .iter()
                .map(|root| repo_root.join(root).join(plan.dir).display().to_string())
                .collect::<Vec<_>>()
                .join("、"),
        )
    })?;
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|e| {
            AppError::io("读不了安装包目录").with_detail(format!("{}：{e}", dir.display()))
        })?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some(plan.ext))
        .collect();
    found.sort();
    match found.as_slice() {
        [] => Err(AppError::io(format!(
            "构建完了但 bundle/{} 下没有 {}",
            plan.dir, plan.ext
        ))
        .with_detail(dir.display().to_string())),
        [one] => {
            // ★ 只有 macOS 的 dmg 要追加「安装说明 + 终端快捷方式」：应用未签名，macOS 会拦
            //   "浏览器下载"的第一次打开（"已损坏"）—— 让 dmg 自己带着解法（2026-10-06 真机踩的）。
            //   说明文档能直接打开；终端快捷方式指向 Apple 签名的系统应用，也不会被拦。
            //   ★ 刻意不放可执行脚本 —— 脚本和应用一样被隔离拦下，形同虚设。
            //   ★ Windows 那边是另一回事（SmartScreen 警告，解法在 Release 说明里），
            //     这一步帮不上忙，也不假装帮 —— 不往 .exe 里塞东西。
            if cfg!(target_os = "macos") {
                let dmg_path = one.display().to_string();
                let patched = std::process::Command::new("bash")
                    .args(["scripts/patch-dmg-extras.sh", &dmg_path])
                    .current_dir(repo_root)
                    .output()
                    .map_err(|e| {
                        AppError::io("起不了 patch-dmg-extras").with_detail(e.to_string())
                    })?;
                if !patched.status.success() {
                    return Err(AppError::io("安装包追加说明失败")
                        .with_detail(String::from_utf8_lossy(&patched.stderr).trim().to_owned()));
                }
            }
            let size = std::fs::metadata(one).map(|m| m.len()).unwrap_or(0);
            Ok(BuiltArtifact {
                name: super::platform::asset_name(
                    PRODUCT,
                    &version_of(repo_root),
                    arch_name(),
                    plan.ext,
                ),
                size,
                path: one.clone(),
            })
        }
        many => Err(AppError::io(format!(
            "bundle/{} 下不止一份 {} —— 自己确认要发哪一份",
            plan.dir, plan.ext
        ))
        .with_detail(
            many.iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join("、"),
        )),
    }
}

/// `npm run tauri -- build`。
///
/// ★ **Windows 上必须过一层 `cmd /C`**：那边 `npm` 是 `npm.cmd`，而 `Command::new("npm")`
/// 走的是 `CreateProcess`，它**不会**替你补 `.cmd` 后缀 —— 直接起会报"程序找不到"。
/// 这不是优化，是"在 Windows 上这条命令根本跑不起来"。
fn npm_build(repo_root: &Path) -> Result<std::process::Output, AppError> {
    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = std::process::Command::new("cmd");
        c.arg("/C").arg("npm");
        c
    } else {
        std::process::Command::new("npm")
    };
    cmd.args(["run", "tauri", "--", "build"])
        .current_dir(repo_root)
        .output()
        .map_err(|e| AppError::io("起不了 npm").with_detail(e.to_string()))
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

/// 上一份 `release.json`（读不到 / 读不懂 = `None` —— 那不是错误：第一次发就是没有）。
fn previous_release_info(repo_root: &Path) -> Option<release_info::ReleaseInfo> {
    let bytes = std::fs::read(repo_root.join(RELEASE_INFO_REL)).ok()?;
    release_info::parse(&bytes).ok()
}

/// `release.json` 该长什么样（**纯函数**）：写盘与"要不要写"共用它。
///
/// ★ `asset` 为 `None` 而**盘上那份是同一个版本**时，把它那一格原样留住（`previous`）——
///   2026-10-07 补平台那一刀：Windows 没有应用内更新的包（那条链只认 macOS 的 `.app`），
///   要是不留住，补一个 Windows 包就会把 macOS 的 `.app.zip` 抹掉，客户端的应用内更新
///   退回"打开下载页"。**那是真丢东西**，不是风格问题。
fn release_info_text(
    version: &str,
    notes: &str,
    url: &str,
    asset: Option<&crate::runtime::release_info::ReleaseAsset>,
    previous: Option<&release_info::ReleaseInfo>,
) -> Result<String, AppError> {
    let kept = asset.or_else(|| {
        previous
            .filter(|p| p.version.trim() == version.trim())
            .and_then(|p| p.asset.as_ref())
    });
    let mut value = serde_json::json!({
        "releaseSchema": RELEASE_SCHEMA,
        "version": version,
        "notes": notes.trim(),
        "url": url,
    });
    // ★ 有安装包就带上 `asset`（**可选格**：没有它客户端退回"打开下载页"，那条路仍然成立）
    if let Some(a) = kept {
        value["asset"] = serde_json::json!({
            "name": a.name,
            "url": a.url,
            "size": a.size,
            "sha256": a.sha256,
        });
    }
    // ★ 断代校验在**写盘之前**：坏 URL 不许落盘（总纲 §5-M6）
    validate_release_source(version, url, kept)?;
    Ok(serde_json::to_string_pretty(&value)
        .map_err(|e| AppError::internal("发布信息序列化失败").with_detail(e.to_string()))?
        + "\n")
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
    let previous = previous_release_info(repo_root);
    let text = release_info_text(version, notes, url, asset, previous.as_ref())?;
    let path = repo_root.join(RELEASE_INFO_REL);
    fsx::atomic::atomic_write(&path, text.as_bytes())?;
    // 写完立刻自己解析一遍：**写进去的东西必须是客户端读得懂的东西**（代次 / 版本形状）
    let bytes = std::fs::read(&path)?;
    release_info::parse(&bytes)?;
    Ok(RELEASE_INFO_REL.to_owned())
}

fn release_body(notes: &str) -> String {
    let n = notes.trim();
    // 未签名那句**按平台说**（2026-10-07）：两边的拦截不是同一件事，给同一句等于给错解法
    let unsigned = if cfg!(target_os = "macos") {
        "（未签名：首次打开需要右键 → 打开绕过 Gatekeeper 警告。）"
    } else if cfg!(target_os = "windows") {
        "（未签名：首次运行会被 SmartScreen 拦一下 ——「更多信息」→「仍要运行」。）"
    } else {
        "（未签名的构建。）"
    };
    if n.is_empty() {
        format!("{PRODUCT} 的正式版本。\n\n{unsigned}")
    } else {
        format!("{n}\n\n{unsigned}")
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

/* ---------- 版本 × 平台：这一版本平台发过没有 ---------- */

/// **这一版（tag）× 本平台**的状态。闸与事务共用这一套判断（各写一份迟早不一致：
/// 闸说能发、事务说发过了）。
///
/// ★ 2026-10-07 改口径（作者）：「应该给我选择 Windows 还是 macOS 的吧。发布过的才不让发，
///   没发布过的就可以发呀」—— 软件版本**一个版本可以有多个平台的包**：macOS 发过之后，
///   Windows 还能往同一个 Release 补一份。于是"tag 被占"不再等于"不许发"。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReleaseSlot {
    /// tag 还没打过：走完整流程（版本号 → PR → tag → 构建 → 上传）
    Fresh,
    /// tag 打了、**本平台**还没有：只补本平台的包（不重打 tag、不开 PR、不动版本号）
    Append,
    /// **本平台**那一份已经在 Release 上了：换一个版本号（同一个平台不重发）
    Taken,
}

/// 去发布仓库问一句：这一版**本平台**发过没有。**只读**。
///
/// 读不到（没账户 / 网络）**不猜**：如实报错 —— "猜成还没发过"的代价是让人对着一个
/// 传不上去的附件，把整趟发版（含几分钟的构建）跑完才发现。
fn release_slot(
    git: &Git,
    tag: &str,
    release: Option<&ReleaseChannel<'_>>,
) -> Result<ReleaseSlot, AppError> {
    if !git.tag_exists(tag)? {
        return Ok(ReleaseSlot::Fresh);
    }
    let plan = installer_plan().ok_or_else(|| {
        AppError::invalid_argument(format!(
            "本机是 {} —— 安装包只在 macOS（dmg）与 Windows（NSIS）上构建",
            std::env::consts::OS
        ))
    })?;
    let Some(ch) = release else {
        return Err(AppError::invalid_argument(format!(
            "`{tag}` 已经打过 —— 要判断这一版本平台发过没有，得先配好发布通道（Gitee）"
        )));
    };
    let names = ch
        .hosting
        .release_assets(&ch.target.owner, &ch.target.repo, tag)?;
    if super::platform::platform_asset_uploaded(&names, plan.ext) {
        Ok(ReleaseSlot::Taken)
    } else {
        Ok(ReleaseSlot::Append)
    }
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
    /// run-env 拦下时给出的 dev 监视器 PID —— 前端据此画「杀掉 dev 监视进程」按钮；
    /// null = 没有要杀的（闸放行或错误另有说法）
    pub dev_watcher_pid: Option<u32>,
}

/// 本进程自己的 dev 监视器（祖先进程链上那个 `tauri dev` CLI）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevWatcher {
    /// CLI 进程的 PID（前端「杀掉 dev 监视进程」按钮拿它调 [`wb_release_kill_dev_watcher`]）
    pub pid: u32,
    /// 命令行（截短给人看，确认杀的是谁）
    pub cmdline: String,
    /// 命令行带 `--no-watch` = 监视器关死（工作台脚本起的）：活着但不碍事，放行
    pub no_watch: bool,
}

/// 进程命令行是不是"tauri dev CLI"那副模样（小写比过：路径里的大写不管）。
fn looks_like_tauri_dev_cmd(cmd: &str) -> bool {
    cmd.contains("tauri") && (cmd.contains(" dev ") || cmd.ends_with(" dev"))
}

/// 命令行拼串（sysinfo 给的是 OsString 列表）。
fn process_cmdline(proc: &sysinfo::Process) -> String {
    proc.cmd()
        .iter()
        .map(|s| s.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ")
}

/// 沿**祖先进程链**找出本进程自己的 `tauri dev` 监视器。
///
/// dev 会话的进程树是 `应用 ← cargo run ← node(tauri.js dev …) ← 包装层（cmd/npm）`，
/// 从应用往外交祖，**第一个**长得像 `tauri dev` 的就是 CLI 本尊（更外面的 npm/cmd
/// 包装层杀不死监视器）。找不到 = 不是 dev 起的（或 CLI 已死）→ None。
fn own_dev_watcher() -> Option<DevWatcher> {
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);
    let mut pid = Pid::from_u32(std::process::id());
    for _ in 0..16 {
        let proc = sys.process(pid)?;
        let cmd = process_cmdline(proc);
        if looks_like_tauri_dev_cmd(&cmd.to_lowercase()) {
            return Some(DevWatcher {
                pid: pid.as_u32(),
                cmdline: {
                    let mut c = cmd.chars().take(120).collect::<String>();
                    if cmd.chars().count() > 120 {
                        c.push('…');
                    }
                    c
                },
                no_watch: cmd.to_lowercase().contains("--no-watch"),
            });
        }
        pid = proc.parent()?;
    }
    None
}

/// run-env 闸的判定：本进程自己的监视器**活着且没关**（版本号一落盘它就重建重启，
/// 发版事务必被杀在半路 —— 2026-10-06 真机踩出来的）。
fn watcher_on() -> Option<DevWatcher> {
    if !cfg!(debug_assertions) {
        return None; // 安装版没有 dev 监视器这回事
    }
    own_dev_watcher().filter(|w| !w.no_watch)
}

/// **杀掉 dev 监视进程**（run-env 拦下时给人的一键解法）。
///
/// ★ 先**重新读一遍该 PID 的命令行**验明正身才动手 —— 防 PID 复用误伤无辜进程。
/// 只杀 CLI 本尊（sysinfo 的 kill 只动这一个进程）：应用与 vite 都是它的子进程，
/// Windows 上父死子活，所以窗口照常用、HMR 照常跑，死的只有"文件监视 + 自动重建"。
#[tauri::command]
pub fn wb_release_kill_dev_watcher(pid: u32) -> Result<bool, AppError> {
    crate::ipc::traced("wb_release_kill_dev_watcher", |_| {
        let mut sys = System::new();
        sys.refresh_processes(ProcessesToUpdate::All, true);
        let proc = sys
            .process(Pid::from_u32(pid))
            .ok_or_else(|| AppError::invalid_argument(format!("进程 {pid} 已经不在了")))?;
        let cmd = process_cmdline(proc).to_lowercase();
        if !looks_like_tauri_dev_cmd(&cmd) {
            return Err(AppError::invalid_argument(format!(
                "PID {pid} 的命令行不像 tauri dev（可能是 PID 已被复用）—— 拒绝杀，请手动确认"
            )));
        }
        Ok(proc.kill())
    })
}

/// 提示词的**事实来源**（生成在 Rust —— 界面不重写一份，事实只有一处）。
struct PromptFacts<'a> {
    repo: &'a Path,
    app_dir: &'a str,
    history_file: &'a str,
    branch: &'a str,
    version: &'a str,
    tag: &'a str,
    notes: &'a str,
    base: &'a str,
    account: Option<&'a PublishTarget>,
    channel: Option<&'a PublishTarget>,
    plan: Option<InstallerPlan>,
}

/// 把"这一版要怎么发"写成一段**能直接交给 AI 的任务书**。
///
/// ★ 为什么放在 Rust：分支 / 版本 / tag / 账户 / 通道 / 产物名 / 平台都是这里的事实，
///   前端不许凭自己的状态再拼一份（拼错了就是"AI 照着一份错的任务书去发版"）。
fn release_prompt_text(f: &PromptFacts) -> String {
    let asset = match f.plan {
        Some(p) => super::platform::asset_name(PRODUCT, f.version, arch_name(), p.ext),
        None => format!("（本机是 {}，打不出安装包）", std::env::consts::OS),
    };
    let label = f.plan.map(|p| p.label).unwrap_or("打不出安装包");
    let account = f
        .account
        .map(|t| format!("{} · {}/{}", t.platform, t.owner, t.repo))
        .unwrap_or_else(|| {
            "**还没配** —— 先在「设置 → 发布账户」里配好，否则这趟只能在本地走到推送".to_owned()
        });
    let channel = f
        .channel
        .map(|t| format!("{} · {}/{}", t.platform, t.owner, t.repo))
        .unwrap_or_else(|| {
            "**还没配** —— 软件版本的 Release 与安装包只发 Gitee，先在「设置」里配 Gitee 发布账户"
                .to_owned()
        });
    let notes = if f.notes.trim().is_empty() {
        "（**还没填** —— 先用一句话说清这一版改了什么，再发）"
    } else {
        f.notes.trim()
    };
    // 推荐命令：把「发布账户与凭据从哪来」一并写进同一行（CLI 靠 MKPSE_APP_DIR 复用工作台的配置）
    let cmd = if cfg!(target_os = "windows") {
        format!(
            "$env:MKPSE_APP_DIR=\"{}\"; npm run release -- {} \"{}\"",
            f.app_dir,
            f.version,
            notes
        )
    } else {
        format!(
            "MKPSE_APP_DIR=\"{}\" npm run release -- {} \"{}\"",
            f.app_dir, f.version, notes
        )
    };

    format!(
        r#"# 任务：发布 SupportEase {tag}

在这个仓库里把 **{tag}** 发出去（本机平台：{os} —— 只会构建 {label}）。

## 事实（照这个核对，别自己猜）
- 仓库根：{repo}
- 当前分支：{branch}（发版 PR 的目标分支是 {base}）
- 目标版本：{version}　tag：{tag}
- 更新说明（会成为 Release 正文与 release.json 的 notes）：{notes}
- 发布账户：{account}
- 发布通道（Release 与安装包**只发这里**）：{channel}
- 安装包产物名：{asset}
- release.json 落点：{release_info}（客户端"有没有新版本"的唯一正式信息源，按 Source Manifest 声明取）

## 怎么发（跑仓库自己的发版命令，别手写流程）
```text
{cmd}
```
等价的底层命令：`cargo run --bin release --features workbench -- {version} "{notes}" --merge`

★ 前置条件：`gh` 已登录、工作区干净（`git status` 没有别的改动）、当前不在 {base} 上、
  **没有 `tauri dev` / 工作台 dev 在跑**（版本号一落盘 dev 会重建重启，把这一趟搅乱）。
★ 不要自己手写 git 命令去凑发版流程：版本号是"**一处真值 + 三处派生**"，必须一起改 ——
  src-tauri/Cargo.toml（真值）/ package.json / src-tauri/tauri.conf.json / Cargo.lock。

## 这条命令会按顺序做（照它核对进度）
1. 预检：分支 / 工作区干净 / 四处版本号一致 / 这一版这一平台还没发过 / 本机能打这个包
2. 版本号四处一起推进到 {version} → 提交 `chore: {tag}` → 推当前分支
3. 开 PR「发版 {tag}」（base={base}）→ 等 CI → squash 合并
4. 切回 {base}，在 {base} 的 **tip** 上打 annotated tag `{tag}` 并推送
5. 把 tag 与 {base} 推到发布仓库（客户端的数据源读的就是那边的 presets/delivery/）
6. 构建本平台安装包：`{asset}`
7. 在发布仓库建 Release「{product} {tag}」，上传安装包
8. 写 {release_info}，开 PR「发布信息：{tag}」（新分支 `chore/release-{tag}`，base={base}）

## 纪律（不许破）
- {base} 只能由 PR 推进：不要在 {base} 上直接提交 / 推送
- tag 必须打在 {base} 的 tip 上、必须是 annotated（`git tag -a`）；已发布的 tag 不许重打 / 移动
- 同一个版本同一个平台不重发（要补另一个平台的包，就换一台对应平台的机器跑）
- **第 8 步那个 PR 必须由人合并** —— 你不要自己合并，做完把它留给用户（合并之后客户端才看得到新版本）
- 不许用 pre-push 钩子的逃生开关：ALLOW_PUSH_MAIN / ALLOW_FORCE_PUSH / ALLOW_TAG_MISMATCH / ALLOW_DELETE_REMOTE

## 做完回报（缺哪项就说缺哪项）
版本 / 分支 / 提交 sha / PR 链接 / tag / 是否已合并 / 安装包名与大小 / Release 链接 /
release.json 的 PR 链接 / **还差什么**（例如"等你合并 release.json 的 PR"）。
★ 任何一步失败：把命令的**原始 stdout / stderr 整段**贴回来，不要只说一句"失败了"。

## 发完之后
这一趟会记进工作台的发布历史（「生成与发布 → 历史」，文件是 {history_file}）——
前提是命令带上了上面那个 MKPSE_APP_DIR。
"#,
        tag = f.tag,
        version = f.version,
        os = std::env::consts::OS,
        label = label,
        repo = f.repo.display(),
        branch = f.branch,
        base = f.base,
        notes = notes,
        asset = asset,
        product = PRODUCT,
        release_info = RELEASE_INFO_REL,
        history_file = f.history_file,
    )
}

/// **发射提示词**（"傻瓜化"出口）：把"这一版要怎么发"整成一段能直接贴给 AI 的任务书。
///
/// ★ 只读：不动仓库、不联网（除了读本机的发布账户配置）—— 点几次都没副作用。
#[tauri::command]
pub fn wb_release_prompt(
    app: tauri::AppHandle,
    version: Option<String>,
    notes: Option<String>,
) -> Result<String, AppError> {
    crate::ipc::traced("wb_release_prompt", |_| {
        let root = crate::fsx::paths::internal_root(&app)?;
        let repo = crate::workbench::paths::repo_root();
        let git = Git::open(&repo);
        let current = version::app_version(&repo)?;
        let want = version
            .map(|v| v.trim().to_owned())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| current.clone());
        let tag = tag_of(&want);
        let account = super::publish_tx::resolve_target(&root, None).ok();
        let channel = super::publish_tx::resolve_target(&root, Some("gitee")).ok();
        let app_dir = root.display().to_string();
        let history_file = root.join("release-history.json").display().to_string();
        let opts = ReleaseOptions::default();
        Ok(release_prompt_text(&PromptFacts {
            repo: &repo,
            app_dir: &app_dir,
            history_file: &history_file,
            branch: &git.branch().unwrap_or_default(),
            version: &want,
            tag: &tag,
            notes: notes.as_deref().unwrap_or_default(),
            base: opts.base_or_main(),
            account: account.as_ref(),
            channel: channel.as_ref(),
            plan: installer_plan(),
        }))
    })
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

        // ★ **版本 × 平台**（2026-10-07 改口径）：**同一版本可以发多个平台** ——
        //   macOS 发过之后，Windows 还能往同一个 Release 补一份。所以判的是
        //   "**本平台**这一份发过没有"，不是笼统的"tag 被占"。
        //   ★ 判据与事务共用 [`release_slot`] —— 两处各写一份就成了"闸说能发、
        //     事务说发过了"或者反过来。
        let gitee = super::publish_tx::resolve_target(&root, Some("gitee")).ok();
        let gitee_hosting = gitee
            .as_ref()
            .and_then(|t| super::platform::hosting(&t.platform, t.token.clone()));
        let channel = gitee
            .as_ref()
            .zip(gitee_hosting.as_deref())
            .map(|(target, hosting)| ReleaseChannel { hosting, target });
        let label = installer_plan().map(|p| p.label).unwrap_or("安装包");
        match release_slot(&git, &tag, channel.as_ref()) {
            Ok(ReleaseSlot::Fresh) => push(
                &mut items,
                "tag",
                "版本 · 平台",
                true,
                format!("`{tag}` 还没打过 —— 走完整流程（版本号 → PR → tag → 构建 → 上传）"),
            ),
            Ok(ReleaseSlot::Append) => push(
                &mut items,
                "tag",
                "版本 · 平台",
                true,
                format!(
                    "`{tag}` 已经发过 —— 这一趟只往它上面补一个{label}\
                     （不重打 tag、不开 PR、不动版本号）"
                ),
            ),
            Ok(ReleaseSlot::Taken) => push(
                &mut items,
                "tag",
                "版本 · 平台",
                false,
                format!(
                    "`{tag}` 的{label}已经在发布仓库里了 —— 这一版这一平台发过了，换一个版本号"
                ),
            ),
            // 读不到（没配发布通道 / 网络）**不猜**：不假设"还没发过"
            Err(e) => push(&mut items, "tag", "版本 · 平台", false, e.message),
        }

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
        //   （`gitee` 在上一格「版本 · 平台」里已经取过 —— 那一格要问它本平台发过没有。）
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

        // ★ dev 构建发版要慎重（2026-10-06 真机踩出来的）：发版要推进版本号，改的
        //   4 个文件（Cargo.toml / tauri.conf.json / package.json / Cargo.lock）
        //   原本全在 `tauri dev` 文件监视器的清单里 —— 版本号一落盘，dev 就重建并
        //   重启应用，发版事务被杀在半路。**死因是那个监视器，不是"dev"这个身份**：
        //   监视器活着 → 拦（并给 PID，前端出「杀掉 dev 监视进程」按钮）；
        //   监视器不在或带 --no-watch → 放行。判定**实时查进程**，不认出身。
        let watcher = watcher_on();
        push(
            &mut items,
            "run-env",
            "发布环境",
            watcher.is_none(),
            match &watcher {
                Some(w) => format!(
                    "发现本进程自己的 dev 监视器（PID {}：{}）—— 版本号一落盘它就重建重启，\
                     发版事务被杀在半路。点「杀掉 dev 监视进程」再「重新检查」（窗口照常用），\
                     或重启一次 dev 会话",
                    w.pid, w.cmdline
                ),
                None if cfg!(debug_assertions) => {
                    "dev 构建，但文件监视器不在 / 已关（--no-watch）—— 版本号落盘不会重启应用，可以真发"
                        .to_owned()
                }
                None => "安装版（发布面）—— 可以真发".to_owned(),
            },
        );

        // ★ 构建那一步**按宿主平台分流**（macOS → dmg / Windows → NSIS，2026-10-07）：
        //   两个平台各自的打包链都在本机，别的一概做不出来 —— 进闸里明说。
        let plan = installer_plan();
        push(
            &mut items,
            "platform-build",
            "构建环境",
            plan.is_some(),
            match plan {
                Some(p) => format!(
                    "本机是 {} —— 会构建 {}（{}）",
                    std::env::consts::OS,
                    p.label,
                    arch_name()
                ),
                None => format!(
                    "本机是 {} —— 安装包只在 macOS（dmg）与 Windows（NSIS）上构建",
                    std::env::consts::OS
                ),
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
            // 只在 run-env 真拦下时给 PID —— 前端那颗「杀掉 dev 监视进程」只认它
            dev_watcher_pid: watcher.map(|w| w.pid),
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
    // ★ 与闸里 run-env 那一格同一条规矩的**硬闸**：真发版前**再探一次**监视器
    //   （人可能在预检之后又起/又杀了 dev —— 以落键那一刻的进程表为准）。
    //   演练（dry_run）一个字节都不写，放行。
    if !opts.dry_run {
        if let Some(w) = watcher_on() {
            return Err(AppError::invalid_argument(
                "本进程的 dev 监视器还活着 —— 版本号一落盘，它就重建重启应用，发版事务被杀在半路",
            )
            .with_detail(format!(
                "先在闸里点「杀掉 dev 监视进程」（PID {}）或重启一次 dev 会话（npm run tauri:workbench:dev，脚本已带 --no-watch），再发",
                w.pid
            )));
        }
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

    /// **发布提示词把事实写全**（2026-10-07）：这份文本是要贴给 AI 当任务书的 ——
    /// 少了 tag / 版本 / 产物名 / release.json 落点 / "那个 PR 要人合并"里的任何一条，
    /// 接过话的那个执行者就只能猜，而猜错的代价是一次真发版。
    #[test]
    fn the_release_prompt_carries_the_facts_and_the_discipline() {
        let repo = PathBuf::from("/tmp/repo");
        let mk = |platform: &str| PublishTarget {
            platform: platform.to_owned(),
            repository_url: format!("https://{platform}.com/o/r"),
            username: "u".to_owned(),
            token: "t".to_owned(),
            owner: "o".to_owned(),
            repo: "r".to_owned(),
        };
        let account = mk("github");
        let channel = mk("gitee");
        let text = release_prompt_text(&PromptFacts {
            repo: &repo,
            app_dir: "/tmp/app",
            history_file: "/tmp/app/release-history.json",
            branch: "feat/demo",
            version: "1.2.3",
            tag: "v1.2.3",
            notes: "修了 X",
            base: "main",
            account: Some(&account),
            channel: Some(&channel),
            plan: installer_plan(),
        });

        for must in [
            "v1.2.3",                 // tag
            "1.2.3",                  // 版本号
            "feat/demo",              // 当前分支
            "修了 X",                  // 更新说明
            "npm run release -- 1.2.3", // 推荐命令
            "github · o/r",           // 发布账户
            "gitee · o/r",            // 发布通道
            RELEASE_INFO_REL,         // release.json 落点
            "chore/release-v1.2.3",   // 第二个 PR 的分支名
            "必须由人合并",             // 那条不能代劳的纪律
            "ALLOW_PUSH_MAIN",        // 逃生开关不许用
            "MKPSE_APP_DIR",          // 让 CLI 复用工作台的发布账户与账本
        ] {
            assert!(text.contains(must), "提示词里少了「{must}」：\n{text}");
        }
        // 找得到本平台的产物名（本机是 macOS / Windows 时）
        if let Some(p) = installer_plan() {
            assert!(
                text.contains(&super::super::platform::asset_name(
                    PRODUCT,
                    "1.2.3",
                    arch_name(),
                    p.ext
                )),
                "提示词里少了安装包名：\n{text}"
            );
        }
    }

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
        /// 发布仓库上这一版的附件名（判据按它造「补平台 / 发过了」两种局面）
        assets: std::cell::RefCell<Vec<String>>,
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
        fn release_assets(
            &self,
            _owner: &str,
            _repo: &str,
            _tag: &str,
        ) -> Result<Vec<String>, AppError> {
            Ok(self.assets.borrow().clone())
        }
    }

    /* ---------- 版本 × 平台：同一版本可以发多个平台（2026-10-07） ---------- */

    /// 三档判定：没打过 = `Fresh`；打了而**本平台**的包不在 = `Append`；
    /// 本平台那一份在 = `Taken`。**闸与事务共用这一处** —— 它错了就是
    /// "闸说能发、事务说发过了"（或反过来）。
    #[test]
    fn the_release_slot_is_decided_per_platform() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let git = init_repo(root, "feat/demo");
        let fake = FakeHosting::default();
        let target = PublishTarget {
            platform: "fake".to_owned(),
            repository_url: "https://fake/o/r".to_owned(),
            username: "u".to_owned(),
            token: "t".to_owned(),
            owner: "o".to_owned(),
            repo: "r".to_owned(),
        };
        let channel = ReleaseChannel {
            hosting: &fake,
            target: &target,
        };
        let ext = installer_plan().expect("判据跑在能打包的平台上").ext;

        // 还没打过 ⇒ 首发（走完整流程）
        assert_eq!(
            release_slot(&git, "v0.0.2", Some(&channel)).unwrap(),
            ReleaseSlot::Fresh
        );

        // 打过了、发布仓库上还没有本平台的包 ⇒ **补平台**
        git.tag("v0.0.2", "v0.0.2").unwrap();
        assert_eq!(
            release_slot(&git, "v0.0.2", Some(&channel)).unwrap(),
            ReleaseSlot::Append
        );

        // 别的平台的包**不算**本平台发过 —— 这正是"补第二个平台"的判据
        let other = if ext == "dmg" { "exe" } else { "dmg" };
        *fake.assets.borrow_mut() = vec![format!("SupportEase_0.0.2_x86_64.{other}")];
        assert_eq!(
            release_slot(&git, "v0.0.2", Some(&channel)).unwrap(),
            ReleaseSlot::Append,
            "只有别的平台的包时，本平台还没发过"
        );

        // 本平台那一份在 ⇒ 这一版这一平台发过了，换版本号
        *fake.assets.borrow_mut() = vec![format!("SupportEase_0.0.2_x86_64.{ext}")];
        assert_eq!(
            release_slot(&git, "v0.0.2", Some(&channel)).unwrap(),
            ReleaseSlot::Taken
        );

        // 没有发布通道 = **问不到**：如实报错，不猜"还没发过"
        let err = release_slot(&git, "v0.0.2", None).unwrap_err();
        assert!(err.message.contains("发布通道"), "{}", err.message);
    }

    /// 补平台**不许把别的平台的安装包抹掉**（2026-10-07 的真机场景：Windows 那一趟
    /// 没有 `.app.zip`，而 `release.json` 里那一格正是 macOS 的应用内更新的入口）。
    #[test]
    fn appending_a_platform_keeps_the_existing_asset() {
        let mac = release_info::ReleaseAsset {
            name: "SupportEase_0.0.6_aarch64.app.zip".to_owned(),
            url: "https://gitee.com/o/r/releases/download/v0.0.6/x.app.zip".to_owned(),
            size: 7,
            sha256: "ab".to_owned(),
        };
        let prev = release_info::ReleaseInfo {
            version: "0.0.6".to_owned(),
            notes: "第一版".to_owned(),
            url: "https://gitee.com/o/r/releases/tag/v0.0.6".to_owned(),
            release_schema: RELEASE_SCHEMA,
            asset: Some(mac.clone()),
        };
        let url = "https://gitee.com/o/r/releases/tag/v0.0.6";

        // 这一趟没有新的 zip（Windows）⇒ 留住上一份那一格
        let text = release_info_text("0.0.6", "补一个 Windows 包", url, None, Some(&prev)).unwrap();
        let parsed = release_info::parse(text.as_bytes()).unwrap();
        assert_eq!(parsed.asset, Some(mac), "补平台把 macOS 那一格抹掉了");
        assert_eq!(parsed.version, "0.0.6");

        // **别的版本**那份不搬过来（0.0.7 是另一版，它的 asset 该由它自己那一趟写）
        let next = "https://gitee.com/o/r/releases/tag/v0.0.7";
        let text = release_info_text("0.0.7", "", next, None, Some(&prev)).unwrap();
        assert_eq!(release_info::parse(text.as_bytes()).unwrap().asset, None);

        // 这一趟有新的 zip（macOS 补包）⇒ 用新的
        let new = release_info::ReleaseAsset {
            name: "SupportEase_0.0.6_x64.app.zip".to_owned(),
            url: "https://gitee.com/o/r/releases/download/v0.0.6/y.app.zip".to_owned(),
            size: 8,
            sha256: "cd".to_owned(),
        };
        let text = release_info_text("0.0.6", "", url, Some(&new), Some(&prev)).unwrap();
        assert_eq!(
            release_info::parse(text.as_bytes()).unwrap().asset,
            Some(new)
        );
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
