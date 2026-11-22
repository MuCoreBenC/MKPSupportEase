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
//! 交付预设那条链必须读预设，那是它的活；这条链的活不在这儿。
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
/// `tauri build` 的**产物根**（相对仓库根）的两个标准候选。
///
/// ★ 两个候选都在：本仓库的 `CARGO_TARGET_DIR` 指到**仓库根**的 `target/`（真机实测
/// 2026-10-04 构建落在 `target/release/bundle/dmg/`），而老机器 / 别的配置下会在
/// `src-tauri/target/…`。**按顺序找，找到哪个里有就用哪个** —— 两个都没有才报错。
///
/// ★ 但"这一次构建用哪个目录"以 `CARGO_TARGET_DIR`（若设了）为第一优先 ——
/// 找产物与找它的是同一套候选，见 [`bundle_roots`]。
pub const BUNDLE_ROOT_CANDIDATES: [&str; 2] =
    ["target/release/bundle", "src-tauri/target/release/bundle"];

/// `bundle/` 的根在盘上可能在哪 —— 按"**这一次构建**落在哪个目录"排序。
///
/// 1. `CARGO_TARGET_DIR`（设了才算；相对路径按仓库根解析，与 cargo 的规矩一致）——
///    **它是本次构建真正生效的那个**：工作台 dev 带着 `target-workbench` 在跑
///    （`scripts/tauri-workbench.mjs` 给 dev 会话设的），[`npm_build`] 的子进程原样
///    继承同一个值，产物就落在 `<CARGO_TARGET_DIR>/release/bundle/`。2026-10-07 真机
///    踩的"构建成功却说没产出"：包在 `target-workbench/release/bundle/nsis/` 里躺着，
///    而这里当时只找下面两个候选；
/// 2. 两个标准候选（没设 env、或从打包的工作台里发版时，落在这些）。
fn bundle_roots(repo_root: &Path) -> Vec<PathBuf> {
    bundle_roots_from(repo_root, std::env::var_os("CARGO_TARGET_DIR"))
}

/// [`bundle_roots`] 的纯函数形状（env 值当参数传）：并行测试共享进程环境，
/// 摆弄 `CARGO_TARGET_DIR` 会互相干扰 —— 判据要能直接喂值。
fn bundle_roots_from(
    repo_root: &Path,
    cargo_target_dir: Option<std::ffi::OsString>,
) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Some(dir) = cargo_target_dir {
        if !dir.is_empty() {
            let target = PathBuf::from(dir);
            let abs = if target.is_absolute() {
                target
            } else {
                repo_root.join(target)
            };
            roots.push(abs.join("release").join("bundle"));
        }
    }
    roots.extend(BUNDLE_ROOT_CANDIDATES.iter().map(|rel| repo_root.join(rel)));
    roots
}

/// 候选目录（含构建时真正生效的那个）拼成一行 —— 错误详情里给"去哪些地方看"。
fn bundle_candidates_text(repo_root: &Path, sub: &str) -> String {
    bundle_roots(repo_root)
        .iter()
        .map(|root| root.join(sub).display().to_string())
        .collect::<Vec<_>>()
        .join("、")
}

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
    /// 写进清单 `assets[].platform` 的词（与客户端 `host_platform()` 同一词表）
    pub platform: &'static str,
    /// 写进清单 `assets[].kind`
    pub kind: &'static str,
    /// 给人看的一句话
    pub label: &'static str,
}

/// 本机能打什么。**别的一律 `None`** —— 不假装能构建，也不悄悄换一种产物顶上。
///
/// ★ 平台与类型在这里**一次定死**，清单、上传、核验都读它，不在别处再判一遍。
pub fn installer_plan() -> Option<InstallerPlan> {
    if cfg!(target_os = "macos") {
        Some(InstallerPlan {
            dir: "dmg",
            ext: "dmg",
            platform: "macos",
            kind: "dmg",
            label: "dmg",
        })
    } else if cfg!(target_os = "windows") {
        Some(InstallerPlan {
            dir: "nsis",
            ext: "exe",
            platform: "windows",
            kind: "nsis",
            label: "NSIS 安装包",
        })
    } else {
        None
    }
}

/// `bundle/<子目录>` 在盘上的实际位置：[`bundle_roots`] 按顺序找，**找到哪个里有就用哪个**。
fn bundle_dir(repo_root: &Path, sub: &str) -> Option<PathBuf> {
    bundle_roots(repo_root)
        .into_iter()
        .map(|root| root.join(sub))
        .find(|dir| dir.is_dir())
}
/// `release.json` 相对仓库根的落点。**它是客户端"有没有新版本"的唯一正式信息源**。
///
/// ★ 2026-10-05 寻址改造（总纲 §1②）：从 `presets/release.json` 入住交付根
/// `presets/delivery/` —— 它本来就是客户端消费的交付元数据，与 catalog / manifest /
/// source 同边界；客户端按 Source Manifest 的 `release` 声明取它（旧的"上跳一级"
/// 推导已废除）。测试钉位见 `runtime::release_info`。
pub const RELEASE_INFO_REL: &str = "presets/delivery/release.json";

/// **预览版**发布信息的落点（`release-preview.json`，2026-11-05 预览发布事务）。
///
/// 与 [`RELEASE_INFO_REL`] **同一个交付根、同一条代次、同一套断代校验**，只是**两条通道**：
/// 正式版用户只读 `release.json`（会主动通知），预览版候选只在用户主动点「重新检查」时
/// 从这一份里取。两份清单互不覆盖 —— 发预览**不许**动正式版那一份。
pub const RELEASE_PREVIEW_REL: &str = "presets/delivery/release-preview.json";

/// 这一趟该写哪一份清单（**只此一处映射**）。
///
/// 让"写正式版"与"写预览版"共用同一条写盘链（同一个形状、同一条断代校验、同一套
/// 原子写），只在**落点**上分叉 —— 两份清单各写各的代码，迟早一份跟着另一份漂。
fn info_rel_for(channel: release_info::UpdateChannel) -> &'static str {
    match channel {
        release_info::UpdateChannel::Stable => RELEASE_INFO_REL,
        release_info::UpdateChannel::Preview => RELEASE_PREVIEW_REL,
    }
}

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
/// Release 的主地址在 **Gitee**（发布仓库 = 设置里的 Gitee 账户），
/// 与 `target`（PR / tag / 推送，跟仓库 remote 走）**刻意分开** ——
/// 代码主线在 GitHub，发布主面在 Gitee；GitHub 做同字节镜像（`sources.github`）。
/// 客户端按用户当前源读 `release.json → assets[].sources`，不写死任何一家。
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
    /// **人类确认位**（2026-11-05 硬约束，见 AGENTS.md）：真发版必须显式给 true。
    ///
    /// 这一条会打 tag、构建并上传安装包、写 `release.json` —— 用户能收到"有新版"、
    /// 老版本会被要求升级，是**最难撤回的那一种动作**。工作台「确认发布」按钮、
    /// CLI 的 `--confirm` 才是这个 `true` 的来源；`dry_run` 只是预演，不需要确认。
    #[serde(default)]
    pub confirmed: bool,
    /// 构建 macOS 安装包（**默认开**：发软件版本就是要有安装包）
    #[serde(default = "default_build")]
    pub build: bool,
    /// 提交推送之后要不要建 PR
    #[serde(default = "default_open_review")]
    pub open_review: bool,
    /// **发哪一条通道**（2026-11-05 预览发布事务）。
    ///
    /// - `stable`（缺省）：写 `release.json`，Release 不标 prerelease —— 会主动通知所有用户；
    /// - `preview`：写 `release-preview.json`，Release 标 prerelease —— **正式版那一份一个
    ///   字节都不动**，只有主动点「重新检查」的人才看得到。
    ///
    /// 缺省是 `stable`：老调用方（旧前端 / 不带 `--channel` 的脚本）语义不变。
    #[serde(default)]
    pub channel: release_info::UpdateChannel,
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
            confirmed: false,
            channel: release_info::UpdateChannel::Stable,
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
    /// 这一轮走的是哪条通道（`stable` / `preview`）—— 回执里必须能看出来，
    /// 否则"我这一趟到底动没动正式版那一份"事后只能靠翻 git log
    pub channel: release_info::UpdateChannel,
    /// 这一轮的版本号（`x.y.z` / `x.y.z-preview.N`，不带 v）
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
    /// Windows 绿色版 zip（与安装版一起进 Release；非 Windows 为 `None`）
    pub portable: Option<ArtifactInfo>,
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
    fn blocked(
        version: String,
        tag: String,
        channel: release_info::UpdateChannel,
        reasons: Vec<String>,
    ) -> Self {
        Self {
            stage: ReleaseStage::BlockedPreflight,
            channel,
            version,
            tag,
            branch: None,
            commit: None,
            review: None,
            release: None,
            artifact: None,
            portable: None,
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

/// 一份 Release 资产在两边的下载地址。`url` 是 Gitee 主地址，`sources` 带 GitHub 镜像。
fn mirrored_asset(
    name: &str,
    gitee_url: String,
    size: u64,
    sha256: String,
    platform: &str,
    kind: &str,
    github_url: Option<String>,
) -> crate::runtime::release_info::ReleaseAsset {
    let mut sources = std::collections::BTreeMap::new();
    sources.insert("gitee".to_owned(), gitee_url.clone());
    if let Some(u) = github_url {
        sources.insert("github".to_owned(), u);
    }
    crate::runtime::release_info::ReleaseAsset {
        name: name.to_owned(),
        url: gitee_url,
        size,
        sha256,
        platform: Some(platform.to_owned()),
        kind: Some(kind.to_owned()),
        sources: Some(sources),
    }
}

/// 把这一趟的产物**同一份字节**镜像到 GitHub（代码仓库的 Release）。
///
/// 只在调用方给了 GitHub 通道（`hosting.kind() == "github"`）时调用：没有通道就不镜像，
/// 清单也就不写 `sources.github` —— 写了就是一个下不到的地址。
///
/// 返回**真的在 GitHub 上**的那些：`.app.zip` 传不上去只剔掉它（这一版退回「打开下载页」），
/// 其余产物传不上去是硬错（清单不许宣告一个下不到的地址）。
///
/// ★ 与 `gh release upload --clobber` 同一个语义：同名资产先清掉再传。不清的话重编同一版
///   时 GitHub 会拒（422），而"跳过"会留下旧字节 —— 清单里的 SHA 与远端就对不上。
fn mirror_artifacts<'a>(
    hosting: &dyn Hosting,
    target: &PublishTarget,
    tag: &str,
    notes: &str,
    channel: release_info::UpdateChannel,
    artifacts: &[&'a BuiltArtifact],
    summary: &mut String,
) -> Result<Vec<&'a BuiltArtifact>, AppError> {
    let release = hosting.create_release(&super::platform::ReleaseSpec {
        owner: target.owner.clone(),
        repo: target.repo.clone(),
        tag_name: tag.to_owned(),
        name: format!("{PRODUCT} {tag}"),
        body: release_body(notes, channel),
        prerelease: channel == release_info::UpdateChannel::Preview,
    })?;
    summary.push_str(&format!("GitHub Release {tag} 已就绪。"));
    let mut done: Vec<&BuiltArtifact> = Vec::new();
    for a in artifacts {
        let put = hosting.upload_asset(&super::platform::AssetUpload {
            owner: target.owner.clone(),
            repo: target.repo.clone(),
            release_id: release.id,
            name: a.name.clone(),
            content_type: super::platform::content_type_for(&a.name).to_owned(),
            path: a.path.clone(),
        });
        match put {
            Ok(p) => {
                if p.size != a.size {
                    return Err(AppError::invalid_argument(format!(
                        "GitHub 回执 {} 字节，本地 {} 字节（{}）—— 传上去的不是这一份",
                        p.size, a.size, a.name
                    )));
                }
                summary.push_str(&format!("已镜像到 GitHub：{}。", p.name));
                done.push(a);
            }
            Err(e) if a.kind == "appZip" => summary.push_str(&format!(
                "应用内更新的 zip 传不上 GitHub（{e}）—— 这一版退回「打开下载页」。"
            )),
            Err(e) => return Err(e),
        }
    }
    Ok(done)
}

/// 补平台那一趟：本平台的资产整组替换，**别的平台的原样留住**。
///
/// 不合并的话，Windows 补包会把 macOS 的 dmg / `.app.zip` 从 `assets[]` 里抹掉 ——
/// 客户端那边就是"新版本忽然不能应用内更新了"，而清单里看不出少过什么。
fn merge_assets_by_platform(
    fresh: Vec<crate::runtime::release_info::ReleaseAsset>,
    previous: Option<&release_info::ReleaseInfo>,
    version: &str,
) -> Vec<crate::runtime::release_info::ReleaseAsset> {
    let Some(prev) = previous.filter(|p| p.version.trim() == version.trim()) else {
        return fresh;
    };
    /* 判据是**平台整组**：本平台的资产全部由 fresh 决定（同平台可能有两份：
    安装包 + 应用内更新包），别的平台的整组原样搬过来。 */
    let fresh_platforms: Vec<Option<String>> = {
        let mut seen: Vec<Option<String>> = Vec::new();
        for a in &fresh {
            if !seen.contains(&a.platform) {
                seen.push(a.platform.clone());
            }
        }
        seen
    };
    let mut out = fresh;
    for a in prev.assets.as_deref().unwrap_or(&[]) {
        if !fresh_platforms.contains(&a.platform) {
            out.push(a.clone());
        }
    }
    out
}

/// 把**这一趟真的在 Release 上**的产物摊成清单资产（纯函数）。
///
/// 平台、类型、文件名、字节数、SHA-256 全部来自产物自己；这一层只补两个下载地址。
/// 于是"macOS 的 dmg 被标成 windows/nsis"那种错没有地方可以发生 —— 这里没有字面量可写。
fn manifest_assets(
    on_release: &[&BuiltArtifact],
    owner: &str,
    repo: &str,
    tag: &str,
    github: Option<&(String, String)>,
) -> Vec<crate::runtime::release_info::ReleaseAsset> {
    on_release
        .iter()
        .map(|a| {
            let gitee = format!(
                "https://gitee.com/{owner}/{repo}/releases/download/{tag}/{}",
                a.name
            );
            let mirror = github.map(|(o, r)| {
                format!(
                    "https://github.com/{o}/{r}/releases/download/{tag}/{}",
                    a.name
                )
            });
            mirrored_asset(
                &a.name,
                gitee,
                a.size,
                a.sha256.clone(),
                a.platform,
                a.kind,
                mirror,
            )
        })
        .collect()
}

/// **打了 tag 就得让清单同代，且通道身份一致**（2026-11-05；预览通道同日补）。
///
/// 纯函数、只读一串文本：判据不用真仓库、不碰网络。三条一起钉：
/// 1. 清单版本号 == 这一趟的版本号；
/// 2. 由它推出来的 tag（`v` + 版本号）== 这一趟的 tag；
/// 3. **通道身份**：预览通道写出来的必须是预发布版本号，正式通道写出来的必须不是。
///    第 3 条防的是"预览版清单里写着正式号"（客户端会把它当成正式版候选）与反过来
///    "正式版清单里写着预览号"（全量用户收到预览通知）这两种串台。
fn assert_release_info_matches_tag(
    text: &str,
    version: &str,
    tag: &str,
    channel: release_info::UpdateChannel,
) -> Result<(), AppError> {
    let info = release_info::parse(text.as_bytes())?;
    if info.version.trim() != version.trim() || tag_of(info.version.trim()) != tag {
        return Err(AppError::invalid_argument(format!(
            "发布清单的版本号（{}）与本次 tag（{tag}）不是同一代 —— 不许打了 tag 却让客户端看不到新版",
            info.version
        )));
    }
    let is_pre = release_info::is_prerelease(&info.version);
    let want_pre = channel == release_info::UpdateChannel::Preview;
    if is_pre != want_pre {
        return Err(AppError::invalid_argument(format!(
            "通道与版本身份不一致：{:?} 通道写出来的版本号是 {}（{}预发布号）",
            channel.wire(),
            info.version,
            if is_pre { "是" } else { "不是" }
        )));
    }
    Ok(())
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
    let res = run_inner(repo_root, opts, target, hosting, release);

    /* **成功与失败都记一笔**：失败时只有 `Err` 一句话，事后想不起"上次停在哪一版"。
    记录落在 `target/` 下（git 看不见）—— 它是运行痕迹，不是发布产物。 */
    let version = match &res {
        Ok(r) => r.version.clone(),
        Err(_) => version::app_version(repo_root).unwrap_or_default(),
    };
    let entry = TxJournalEntry {
        at: crate::workbench::clock::now_iso8601(),
        tag: tag_of(&version),
        version,
        commit: Git::open(repo_root).head_short().unwrap_or_default(),
        stage: match &res {
            Ok(r) => format!("{:?}", r.stage),
            Err(_) => "failed".to_owned(),
        },
        ok: res.is_ok(),
    };
    journal_append(repo_root, &entry);
    res
}

fn run_inner(
    repo_root: &Path,
    opts: &ReleaseOptions,
    target: Option<&PublishTarget>,
    hosting: Option<&dyn Hosting>,
    release: Option<&ReleaseChannel<'_>>,
) -> Result<ReleaseTxReport, AppError> {
    /* ⓪ **没有人类当次确认 = 不许真发版**（2026-11-05 硬约束）。
     * 放在所有写盘 / 打 tag / 建 Release / 联网之前，零副作用。预演不要确认。 */
    if !opts.dry_run && !opts.confirmed {
        return Err(AppError::invalid_argument(
            "真发版需要显式确认：请在工作台点「确认发布」，或在命令行加 --confirm；\
             只想看会做什么请用 --dry-run。",
        ));
    }

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
    /* **版本形状按通道判**（2026-11-05 预览发布事务）：
     *   · 正式通道：只认 `x.y.z`（`-dev` 只是开发期占位，去掉才是「定稿」）；
     *   · 预览通道：只认 `x.y.z-preview.N`（没有序号的预览号说不清谁更新）。
     * ★ 交叉约束两条，都是"一发出去就难撤"的那一类，所以放在**零写入**的预检里：
     *   · 预览号走正式通道 —— 全量用户会收到一个预览版的"有新版"通知；
     *   · 正式号走预览通道 —— 它不会通知任何人，白占一个正式号。 */
    match opts.channel {
        release_info::UpdateChannel::Stable => {
            if !version::is_semver(&version) {
                blocked.push(format!(
                    "正式通道的发布版本必须是 x.y.z（不能带 -dev / -preview）：{version}\
                     —— 先 npm run version:release"
                ));
            }
        }
        release_info::UpdateChannel::Preview => {
            if !version::is_preview_version(&version) {
                blocked.push(format!(
                    "预览通道的发布版本必须是 x.y.z-preview.N（序号从 1 起）：{version}"
                ));
            }
        }
    }
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
    /* **会按当前工作区重新构建的趟：tag 存在 ⇒ 必须指向当前 HEAD。**
    续跑会沿用那个 tag、却从当前工作区构建；两者不是同一笔提交时，
    新代码的包就会被挂到旧 tag 的 Release 上 —— 版本号还一样，事后查不出来。
    `InfoOnly` 不构建、不重传（只按本地这一版的产物写清单），不受这条约束；
    `Taken` 已经报过更准的那句，这里不重复。 */
    if rebuilds_from_worktree(slot) && slot != ReleaseSlot::Taken {
        if let Some((at_tag, at_head)) = frozen_elsewhere(&git, &tag) {
            blocked.push(format!(
                "tag `{tag}` 指向 {}，当前 HEAD 是 {} —— 这一版冻结在另一笔提交上：\
                 切回那笔提交再续跑，或换一个版本号",
                short_sha(&at_tag),
                short_sha(&at_head),
            ));
        }
    }
    let append = matches!(slot, ReleaseSlot::Append | ReleaseSlot::InfoOnly);
    let info_only = slot == ReleaseSlot::InfoOnly;
    /* 旧版本产物必须在**构建开始前**拦住：只补清单不构建，跳过。 */
    if opts.build && !info_only {
        if let Err(e) = assert_no_stale_bundles(repo_root, &version) {
            blocked.push(e.message);
        }
    }
    // 版本派生不一致 = 有人手改了别处。给了新版本号的话 `bump` 会把它推平，
    // 所以只在"沿用当前真值"时它才是问题。
    if opts.version.is_none() {
        for rel in version::derived_mismatch(repo_root)? {
            blocked.push(format!("{rel} 与版本真值不一致（版本号只有一处真值）"));
        }
    }
    if !blocked.is_empty() {
        return Ok(ReleaseTxReport::blocked(
            version,
            tag,
            opts.channel,
            blocked,
        ));
    }

    let mut report = ReleaseTxReport {
        stage: ReleaseStage::Ready,
        channel: opts.channel,
        version: version.clone(),
        tag: tag.clone(),
        branch: Some(branch.clone()),
        commit: None,
        review: None,
        release: None,
        artifact: None,
        portable: None,
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

    /* 阶段 B：先把主线同步到构建基线；**tag 还没打**。
    tag 是阶段 C 的动作，必须等构建与产物校验全部通过。 */
    let build_head = if !git.tag_exists(&tag)? {
        // ★ 这一动会**切换开发者本机的当前分支** —— 界面与 CLI 都要在闸里明说（作者定死的
        // "破坏性命令要讲明白"）。squash 重写过提交，分支上打的 tag 指向不在 main 历史里的提交。
        git.switch(base)?;
        git.pull_ff("origin", base)?;
        git.rev_parse("HEAD")?
    } else {
        /* 补包趟（tag 已存在）：会构建的那些只能用 tag 指向的那一版做基线；
        只补清单（InfoOnly）不构建，产物是 tag 那一刻的，与 HEAD 无关。 */
        if rebuilds_from_worktree(slot) {
            if let Some((at_tag, at_head)) = frozen_elsewhere(&git, &tag) {
                return Err(AppError::invalid_argument(format!(
                    "{tag} 指向 {}，当前 HEAD 是 {} —— 补包不能拿别的提交构建",
                    short_sha(&at_tag),
                    short_sha(&at_head),
                )));
            }
        }
        git.rev_parse("HEAD")?
    };
    report
        .summary
        .push_str(&format!("构建基线 {}。", short_sha(&build_head)));

    /* ---------- ⑦ 构建 + 产物自报版本核对（**tag 仍未创建**） ---------- */

    /* 本次发布**唯一**的产物列表：安装包 + 绿色版（Windows）+ 应用内更新包（macOS）。
    名字 / 大小 / SHA-256 / 平台 / 类型都在造它的那一刻定下；上传与写清单都只遍历这一份。 */
    let mut artifacts: Vec<BuiltArtifact> = Vec::new();
    if opts.build && !info_only {
        artifacts.push(build_installer(repo_root)?);
        /* ★ **构建完先问一遍产物自己是谁**（2026-11-06 作者：「你这都是 0.0.4 了，
        里面还显示 0.0.3，那不是很不合理吗？」）。核对不过就停在 Built 之前 ——
        不建 tag、不建 Release、不上传、不写清单：宁可这一趟失败，也不发一个"名不副实"的包。 */
        assert_artifact_speaks_version(repo_root, &version)?;
        if cfg!(target_os = "windows") {
            artifacts.push(build_portable(repo_root, &version)?);
        }
        if cfg!(target_os = "macos") {
            /* 打不出 .app.zip **不挡发版**：这一版退回「打开下载页」，清单里就没有它。 */
            match build_app_zip(repo_root, &version) {
                Ok(z) => artifacts.push(z),
                Err(e) => report.summary.push_str(&format!(
                    "打不出应用内更新的 zip（{e}）—— 这一版退回「打开下载页」。"
                )),
            }
        }
        /* 构建期间 HEAD 被切走 ⇒ 这个包不是构建基线那一笔的产物，不许往上挂 */
        let now = git.rev_parse("HEAD")?;
        if now != build_head {
            return Err(AppError::invalid_argument(format!(
                "构建期间 HEAD 从 {} 变成了 {} —— 这个安装包不是构建基线的产物，不往上挂",
                short_sha(&build_head),
                short_sha(&now),
            )));
        }
    } else {
        report.summary.push_str("（本次不构建安装包）");
    }
    /* **只补清单**那一趟：不构建、不重传，但清单要写的是**这一版真实的资产**。
    从磁盘上找这一版的两份（还没有就如实报），不拿上一版的名单顶上 ——
    上一版的名字带上一版的号，URL 却指向这一趟的 tag，客户端会下到 404。 */
    if info_only {
        artifacts.push(locate_installer(repo_root)?);
        if let Some(p) = locate_portable(repo_root, &version)? {
            artifacts.push(p);
        }
    }
    if let Some(b) = artifacts.iter().find(|a| a.kind != "portable") {
        report.artifact = Some(ArtifactInfo {
            name: b.name.clone(),
            size: b.size,
            path: b.path.display().to_string(),
        });
        report.stage = ReleaseStage::Built;
        if !info_only {
            report.summary.push_str(&format!("已构建 {}。", b.name));
        }
    }
    if let Some(p) = artifacts.iter().find(|a| a.kind == "portable") {
        report.portable = Some(ArtifactInfo {
            name: p.name.clone(),
            size: p.size,
            path: p.path.display().to_string(),
        });
    }

    if !artifacts.is_empty() {
        validate_built_artifacts(&artifacts, &version)?;
    }

    /* ---------- ⑧ 构建与产物校验都过了，才创建并推送 tag ---------- */

    if !git.tag_exists(&tag)? {
        git.tag(&tag, &format!("{tag} {}", opts.notes.trim()))?;
        /* origin（代码主线）与发布仓库都要有这个 tag。 */
        git.push_tag(&tag)?;
        if let Some(t) = target {
            if !git.remote_matches(&t.repository_url)? {
                git.push_ref_to_authenticated(
                    &t.repository_url,
                    &format!("refs/tags/{tag}"),
                    &t.username,
                    &t.token,
                )?;
            }
        }
        report.summary.push_str(&format!(
            "构建与产物校验通过，最后在 {} 的 tip 上打 {tag}、推送。",
            short_sha(&build_head)
        ));
    } else {
        report.summary.push_str("tag 已存在（补包趟），沿用。");
    }
    report.stage = ReleaseStage::Tagged;

    /* ---------- ⑨ 发布面同步（M6）：tag 与 main 推到 Gitee（幂等） ----------
     *
     * Release 挂在 tag 上，所以 **tag 必须在 Gitee 真实存在**；main 同步过去是
     * "数据源主线跟随发布仓库"（作者 2026-10-05 定）—— 客户端的数据源读的就是
     * 仓库里的 `presets/delivery/`。两步都幂等：tag 先 `ls-remote` 问一句（已有且
     * 同一笔提交就跳过，指到别的提交就报分叉）；main 同一笔重跑是 no-op / fast-forward。
     * 主线的**源是 `origin/{base}`**（上面刚 fetch 过）：本地 {base} 旧不旧都不影响这一推 ——
     * 拿本地分支当源正是 2026-10-07 那次"git push 失败了"的成因（非快进，闸③ 拦）。
     * 远端有本地没有的提交时如实报错（那是镜像分叉，得先解决，不能悄悄覆盖）。
     */
    if let Some(ch) = release {
        let g = ch.target;
        // ★ 发布仓库上**已有这个 tag** 时不要硬推：同名不同对象的 tag，git 自己会拒
        //   （already exists），pre-push 闸③ 也会把"远端 tag 对象不是本地的祖先"报成
        //   非快进（2026-10-07 真机：本地与发布仓库的 v0.0.6 是两回打的 annotated tag，
        //   对象不同、**指向同一笔提交** —— tag 不是提交，闸的祖先判定在它身上只会误报，
        //   界面上就剩一句"git push 失败了"）。剥了皮指向同一笔提交 = 早就同步过去了，
        //   跳过（幂等）；指向别的提交 = tag 真分叉 —— 已发布的 tag 不许重打/移动，
        //   停下来让人裁决，不悄悄覆盖。
        let local_commit = git.rev_parse(&format!("refs/tags/{tag}^{{commit}}"))?;
        match git.remote_tag_commit(&g.repository_url, &tag, &g.username, &g.token)? {
            Some(remote_commit) if remote_commit == local_commit => {
                report
                    .summary
                    .push_str(&format!("{tag} 发布仓库里已有（同一笔提交），不重推。"));
            }
            Some(remote_commit) => {
                return Err(AppError::invalid_argument(format!(
                    "{tag} 在发布仓库上指向 {remote_commit}，本地这个指向 {local_commit} —— \
                     tag 分叉了，不许自动覆盖（已发布的 tag 不许重打/移动）；\
                     先弄清哪一笔才是真发布，再重跑这一趟"
                )));
            }
            None => {
                git.push_ref_to_authenticated(
                    &g.repository_url,
                    &format!("refs/tags/{tag}"),
                    &g.username,
                    &g.token,
                )?;
            }
        }
        git.push_ref_to_authenticated(
            &g.repository_url,
            &format!("{remote_base}:refs/heads/{base}"),
            &g.username,
            &g.token,
        )?;
        report.summary.push_str(&format!(
            "已把 {base} 推到发布仓库 {}（数据源与 release.json 随之过去）。",
            g.repository_url
        ));
    }

    /* 不构建的趟（只验编排 / 只打 tag）到此为止：没有产物可建 Release。 */
    if !opts.build {
        report.summary.push_str("（本次不构建安装包，到 tag 为止）");
        return Ok(report);
    }

    /* ---------- ⑩ 建 Release + 上传**产物列表里的每一份**（M6：主地址 Gitee） ---------- */

    // ★ **架构规则**（总纲 §5-M6）：软件版本 Release 的主地址在 Gitee。
    //   macOS 的 `.app.zip` 与 Windows 的绿色版走**同一条上传路径** —— 不再有"上传完
    //   再单独打一个 zip"的第二段：那样它永远进不了同一个列表，镜像也就跟着漏。
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
        body: release_body(&opts.notes, opts.channel),
        // ★ 预览版**必须**标 prerelease：不标的话它会以"最新正式版"的身份挂在仓库首页，
        //   而正式版用户根本没被通知过 —— 那是这一整套双通道要避免的事故。
        prerelease: opts.channel == release_info::UpdateChannel::Preview,
    })?;
    report.release = Some(release.clone());
    report.stage = ReleaseStage::ReleaseCreated;
    report.summary.push_str("已建 Release。");

    /* 这一趟**真的在 Release 上**的那些 —— 清单只写它们，不写没传上去的名字。 */
    let mut on_release: Vec<&BuiltArtifact> = Vec::new();
    if info_only {
        report
            .summary
            .push_str("产物已在 Release 上，这一趟只补 release.json，不重传。");
    } else {
        if artifacts.is_empty() {
            return Err(AppError::invalid_argument(
                "这一趟没有构建产物，不能建 Release / 上传；先构建再发",
            ));
        }
        for a in &artifacts {
            let put = ch.hosting.upload_asset(&super::platform::AssetUpload {
                owner: g.owner.clone(),
                repo: g.repo.clone(),
                release_id: release.id,
                name: a.name.clone(),
                content_type: super::platform::content_type_for(&a.name).to_owned(),
                path: a.path.clone(),
            });
            match put {
                Ok(p) => {
                    /* **回执必须与本地产物一致**：大小对不上说明传上去的不是这一份
                    （截断 / 换了文件），清单写下去客户端永远校验不过。 */
                    if p.size != a.size {
                        return Err(AppError::invalid_argument(format!(
                            "{} 上传回执 {} 字节，本地 {} 字节 —— 传上去的不是这一份，不写清单",
                            a.name, p.size, a.size
                        )));
                    }
                    report
                        .summary
                        .push_str(&format!("已上传 {}（{} 字节）。", p.name, p.size));
                    on_release.push(a);
                }
                /* `.app.zip` 传不上去**不挡发版**：这一版退回「打开下载页」。
                其余产物传不上去是硬错 —— 清单不能宣告一个下不到的地址。 */
                Err(e) if a.kind == "appZip" => report.summary.push_str(&format!(
                    "应用内更新的 zip 传不上去（{e}）—— 这一版退回「打开下载页」。"
                )),
                Err(e) => return Err(e),
            }
        }
        if on_release.iter().any(|a| a.kind != "appZip") {
            report.stage = ReleaseStage::AssetUploaded;
        }
        if !on_release.iter().any(|a| a.kind == "appZip") && cfg!(target_os = "macos") {
            report
                .summary
                .push_str("这一版客户端走「打开下载页」（没有应用内更新的 zip）。");
        }
    }

    /* ---------- ⑩之二 同一份产物镜像到 GitHub（代码仓库） ----------
     *
     * 镜像**由这一趟发布事务自己做**，不再依赖 `release.mjs` 在外层补一次：
     * 那条路只有 CLI 走得到，工作台发的版 GitHub 上永远没有资产，而清单照样写着
     * `sources.github` —— 承诺了一个下不到的地址。
     *
     * 只在调用方给了 GitHub 通道时做（`hosting.kind() == "github"`）：没有通道 = 这张
     * 清单就**不写** GitHub 源，而不是写一个没人传上去的地址。 */
    let mirror = (!info_only && hosting.kind() == "github").then_some((t, hosting));
    if let Some((t, h)) = mirror {
        on_release = mirror_artifacts(
            h,
            t,
            &tag,
            &opts.notes,
            opts.channel,
            &on_release,
            &mut report.summary,
        )?;
    }

    /* ---------- ⑨ 写清单（`release.json` / `release-preview.json`）→ 它自己的分支与 PR
     *          （**合并留给人**） ---------- */

    // ★ **落点按通道选**（2026-11-05）：正式版写 `release.json`，预览版写
    //   `release-preview.json`。两条通道**各写各的**，发预览时正式版那一份一个字节都不动。
    let info_rel = info_rel_for(opts.channel);

    // ★ 先算一遍"它该长什么样"（**纯函数**，不落盘）：与盘上那份一字不差就**不动分支、
    //   不开 PR**（2026-10-07）。补平台那一趟正是这种情况 —— asset 保留的是上一份，
    //   内容一个字节都不变；照老写法会切到一条不相干的分支上、再开一份空的 PR。
    let previous = previous_release_info(repo_root, info_rel);
    // ★ 补平台那一趟**不改软件发布信息**：`notes` / `url` 沿用盘上那一份 —— 人这一趟填的
    //   说明只进 Release 正文那一步，而那一步对**已经存在**的 Release 本来就不改
    //   （`create_release` 是回读，不是改）。于是清单一个字节都不动：
    //   既不会多开一份 PR，也不会把上一趟发布的那句话悄悄换掉。
    let (info_notes, info_url) = match previous
        .as_ref()
        .filter(|p| append && p.version.trim() == version.trim())
    {
        Some(p) => (p.notes.clone(), p.url.clone()),
        None => (opts.notes.clone(), release.url.clone()),
    };
    /* 双平台 / 双源资产：**从同一份产物列表摊开** —— 平台与类型读产物自己带的那两个词，
    不在这一层再判一次（从前这里写死 `windows/nsis`，macOS 的 dmg 就被标成 Windows）。
    主地址 Gitee，`sources.github` 给选了 GitHub 的客户端。 */
    /* `sources.github` 只在**真的镜像过**时写：镜像通道与上传目标是同一对 owner/repo。 */
    let gh = mirror.map(|(t, _)| (t.owner.clone(), t.repo.clone()));
    let platform_assets = manifest_assets(&on_release, &g.owner, &g.repo, &tag, gh.as_ref());
    /* 只补清单（tag/产物都在）时没有新构建：保留上一份的 assets，一个字节都不改。
    补平台时本平台整组替换、**别的平台原样留住**（不合并就会把另一平台的包抹掉）。 */
    let platform_assets = if platform_assets.is_empty() {
        previous
            .as_ref()
            .and_then(|p| p.assets.clone())
            .unwrap_or_default()
    } else {
        merge_assets_by_platform(platform_assets, previous.as_ref(), &version)
    };
    /* 老客户端的单资产入口（`asset` 格）就是 macOS 的应用内更新包 —— 从同一份列表里取。 */
    report.zip = platform_assets
        .iter()
        .find(|a| a.kind.as_deref() == Some("appZip"))
        .cloned();
    let assets_param = (!platform_assets.is_empty()).then_some(platform_assets.as_slice());

    let want_info = release_info_text(
        &version,
        &info_notes,
        &info_url,
        report.zip.as_ref(),
        assets_param,
        previous.as_ref(),
    )?;
    /* ⓪ 「动了 tag 的版本号，就必须让清单跟着动」（2026-11-05 作者原话）。
     * 这条本来靠"两个地方都传 version"维持；加一道纯读判据，将来谁绕开写文件、
     * 或改了版本号却没同步信息源，事务会在这一步红，而不是"打了 tag 客户端永远
     * 不知道有新版"。`want_info` 就是这一趟该写/该保留的那份，早退分支也过这道。 */
    assert_release_info_matches_tag(&want_info, &version, &tag, opts.channel)?;
    if std::fs::read_to_string(repo_root.join(info_rel))
        .ok()
        .as_deref()
        == Some(want_info.as_str())
    {
        report.summary.push_str(&format!(
            "{info_rel} 与盘上那份一字不差（这一趟不改软件发布信息），没有要提交的。"
        ));
        return Ok(report);
    }

    let written_rel = write_release_info(
        repo_root,
        info_rel,
        &version,
        &info_notes,
        &info_url,
        report.zip.as_ref(),
        assets_param,
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
    let staged =
        git.stage_allowed_in(std::slice::from_ref(&written_rel), &RELEASE_STAGE_ALLOWLIST)?;
    if !git.has_staged()? {
        report.info_branch = Some(info_branch);
        report
            .summary
            .push_str(&format!("{written_rel} 没有变化，没有可提交的。"));
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
            "由 SupportEase 发布软件版本后写。\n\n{}\n\n★ 合并之后客户端才看得到{}。",
            notes_line(&opts.notes),
            match opts.channel {
                release_info::UpdateChannel::Stable => "新版本",
                release_info::UpdateChannel::Preview => "这个预览版",
            }
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
    report.summary.push_str(&format!(
        "已写 {written_rel} 并开了 PR —— **合并它之后{}才看得到{}**。",
        match opts.channel {
            release_info::UpdateChannel::Stable => "客户端",
            release_info::UpdateChannel::Preview => "主动检查更新的人",
        },
        match opts.channel {
            release_info::UpdateChannel::Stable => "新版本",
            release_info::UpdateChannel::Preview => "这个预览版",
        }
    ));

    Ok(report)
}

/// ① 会被 stage 的候选（版本号真值 + 三个派生）。**具体进不进得去由白名单再判一次**。
const RELEASE_STAGE_CANDIDATES: [&str; 4] = [
    version::MANIFEST_REL,
    version::PACKAGE_REL,
    version::CONF_REL,
    version::LOCK_REL,
];

/// 一份**已构建、可上传**的发布产物 —— 本次发布的**唯一产物事实**。
///
/// 平台、类型、文件名、字节数、SHA-256 在构建完成时一次填好：清单、上传器、
/// 核验器都读这一份，谁也不再自己推导（从前清单处把平台写死成 Windows，macOS
/// 的 dmg 就被标成了 `windows/nsis`）。
#[derive(Debug, Clone)]
struct BuiltArtifact {
    name: String,
    size: u64,
    sha256: String,
    path: PathBuf,
    platform: &'static str,
    kind: &'static str,
}

impl BuiltArtifact {
    /// 从刚构建出来的文件造一条产物事实。大小与 SHA **只在这里读一次**，
    /// 之后写清单不再碰磁盘 —— 中间文件被换掉的话，清单写的就是别的东西了。
    fn new(
        path: PathBuf,
        name: String,
        platform: &'static str,
        kind: &'static str,
    ) -> Result<Self, AppError> {
        let size = std::fs::metadata(&path).map(|m| m.len()).map_err(|e| {
            AppError::io(format!("读不了刚构建的产物：{}", path.display()))
                .with_detail(e.to_string())
        })?;
        let sha256 = sha256_file(&path)?;
        Ok(Self {
            name,
            size,
            sha256,
            path,
            platform,
            kind,
        })
    }
}

/// 资产名取**磁盘上的真名**；取不到才退回模板。Windows 安装器是 `…-setup.exe`，
/// 按模板拼会变成 `…_x64.exe`，清单与附件就对不上。
fn artifact_name(path: &Path, fallback: impl FnOnce() -> String) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(str::to_owned)
        .unwrap_or_else(fallback)
}

/// 某个目录里带目标扩展名、但文件名不含当前版本号的残留产物。
/// **纯函数**：判据直接喂它造文件，不依赖真仓库。
fn stale_bundle_files(dir: &Path, version: &str, ext: &str) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.is_file()
                        && p.extension().and_then(|s| s.to_str()) == Some(ext)
                        && p.file_name()
                            .and_then(|n| n.to_str())
                            .is_some_and(|n| !n.contains(version))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// **构建前**拦住旧版本产物：`bundle/nsis` / `bundle/macos` 与 `dist-portable`
/// 里只要还有别的版本，就不让构建开始。否则会在编译十几分钟后才报“找到多份”，
/// 或者在清单里挑错一份。
fn assert_no_stale_bundles(repo_root: &Path, version: &str) -> Result<(), AppError> {
    let Some(plan) = installer_plan() else {
        return Ok(());
    };
    let mut stale = Vec::new();
    if let Some(dir) = bundle_dir(repo_root, plan.dir) {
        stale.extend(stale_bundle_files(&dir, version, plan.ext));
    }
    stale.extend(stale_bundle_files(
        &repo_root.join("dist-portable"),
        version,
        "zip",
    ));
    if stale.is_empty() {
        return Ok(());
    }
    stale.sort();
    Err(AppError::invalid_argument(format!(
        "构建目录里有旧版本产物 —— 先移出 {}，再开始构建；\
         否则会在构建完才发现多份包，或者挑错要发的那一份",
        stale
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join("、")
    )))
}

/// 构建后、打 tag 前核对**每一份**产物：文件名必须是磁盘真名、大小必须与磁盘一致；
/// 压缩包类还要真的解得开、里面有该有的东西。**SHA 不在这里重算** ——
/// 它是 [`BuiltArtifact::new`] 在构建完成时读的那一份（同一时刻的事实，只读一次）。
fn validate_built_artifacts(list: &[BuiltArtifact], version: &str) -> Result<(), AppError> {
    fn one(a: &BuiltArtifact, version: &str, label: &str) -> Result<(), AppError> {
        let meta = std::fs::metadata(&a.path)
            .map_err(|e| AppError::io(format!("{label}路径读不到")).with_detail(e.to_string()))?;
        if !meta.is_file() {
            return Err(AppError::io(format!(
                "{label}不是文件：{}",
                a.path.display()
            )));
        }
        if meta.len() != a.size {
            return Err(AppError::invalid_argument(format!(
                "{label}大小对不上：记录 {}，磁盘 {} —— 产物在记录之后被改过",
                a.size,
                meta.len()
            )));
        }
        let real = artifact_name(&a.path, || a.name.clone());
        if real != a.name {
            return Err(AppError::invalid_argument(format!(
                "{label}文件名对不上：清单/上传记的是 {}，磁盘是 {}",
                a.name, real
            )));
        }
        if !a.name.contains(version) {
            return Err(AppError::invalid_argument(format!(
                "{label}文件名不含目标版本 {version}：{}",
                a.name
            )));
        }
        Ok(())
    }

    /// zip 里的条目名（解不开 = 这一份不是能发的 zip）。
    fn zip_names(a: &BuiltArtifact, label: &str) -> Result<Vec<String>, AppError> {
        let f = std::fs::File::open(&a.path)
            .map_err(|e| AppError::io(format!("{label} zip 打不开")).with_detail(e.to_string()))?;
        let mut archive = zip::ZipArchive::new(f)
            .map_err(|e| AppError::io(format!("{label}不是合法 zip")).with_detail(e.to_string()))?;
        Ok((0..archive.len())
            .filter_map(|i| archive.by_index(i).ok().map(|e| e.name().to_owned()))
            .collect())
    }

    for a in list {
        match a.kind {
            "portable" => {
                one(a, version, "绿色版")?;
                let names = zip_names(a, "绿色版")?;
                if !names.iter().any(|n| n.ends_with("SupportEase.exe")) {
                    return Err(AppError::invalid_argument(
                        "绿色版 zip 里没有 SupportEase.exe —— 解压出来不是能用的绿色版",
                    ));
                }
                if !names.iter().any(|n| n.contains("卸载")) {
                    return Err(AppError::invalid_argument(
                        "绿色版 zip 里没有卸载器 —— 缺了它用户没法干净卸载",
                    ));
                }
            }
            "appZip" => {
                one(a, version, "应用内更新包")?;
                let names = zip_names(a, "应用内更新包")?;
                if !names.iter().any(|n| n.contains(".app/")) {
                    return Err(AppError::invalid_argument(
                        "应用内更新包 zip 里没有 .app —— 客户端替换不了",
                    ));
                }
            }
            _ => one(a, version, "安装包")?,
        }
    }
    Ok(())
}

/// **构建本平台的安装包**。
///
/// macOS 走 `npm run tauri -- build`；Windows 走 `npm run build:installer` —— 自研安装器
/// 的 payload 里有**两档内核**（安装版 / 绿色版），绿色版自更新写 `--portable` 那一支。
/// 真发版若沿用 Tauri 的 NSIS 单内核包，绿色版更新会被装成安装版（数据根漂到
/// `%APPDATA%`），所以两条链必须都走各自真正的构建命令。
///
/// 走 npm script 而不是直接 `npx tauri`：那确保用的是本项目 `node_modules` 里那一版 CLI，
/// 而不是 PATH 上碰巧存在的另一个。
///
/// ★ 产物**只认 `bundle/<本平台目录>` 下的唯一一份**：找到 0 份如实报（不猜别的目录），
/// 找到多份也如实报（让人自己决定哪一份）—— "挑一个"这种事不该由程序偷偷做。
/// **包里自报的版本必须就是这一次要发的号。**
///
/// 为什么非核不可：`APP_VERSION` 是 `env!("CARGO_PKG_VERSION")` —— **编译期**常量。
/// 改了 `Cargo.toml` 却没重编，包里那个号还是旧的，而 tag / Release / 清单报的是新的。
/// 发出去之后，"用户看到的版本"与"发布记录里的版本"永远对不上，而且**没有任何一处会报错**
/// —— 作者 2026-11-06 说的"你这都是 0.0.4 了，里面还显示 0.0.3"正是这个。
///
/// 读的是 `.app/Contents/Info.plist` 的 `CFBundleShortVersionString`：Tauri 打包时从
/// `tauri.conf.json` 取，那一处由内核从 `Cargo.toml` 派生 —— 三者本该相等。
#[cfg(target_os = "macos")]
fn assert_artifact_speaks_version(repo_root: &Path, want: &str) -> Result<(), AppError> {
    let dir = bundle_dir(repo_root, "macos").ok_or_else(|| {
        AppError::io("找不到 .app（构建说成功，但 bundle/macos 下什么都没有）")
            .with_detail(bundle_candidates_text(repo_root, "macos"))
    })?;
    let app = std::fs::read_dir(&dir)
        .ok()
        .and_then(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .find(|p| p.extension().and_then(|s| s.to_str()) == Some("app"))
        })
        .ok_or_else(|| AppError::io(format!("{} 下没有 .app", dir.display())))?;
    let plist = app.join("Contents").join("Info.plist");
    let out = std::process::Command::new("plutil")
        .args(["-extract", "CFBundleShortVersionString", "raw", "-o", "-"])
        .arg(&plist)
        .output()
        .map_err(|e| AppError::io("读不了包里的 Info.plist").with_detail(e.to_string()))?;
    if !out.status.success() {
        return Err(AppError::io("包里没有 CFBundleShortVersionString")
            .with_detail(String::from_utf8_lossy(&out.stderr).trim().to_owned()));
    }
    let got = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    if got != want {
        return Err(AppError::invalid_argument(format!(
            "包内自报 {got}，这次要发的是 {want} —— 两个号不是同一个，发出去就对不上账"
        ))
        .with_detail(
            "多半是改了版本号却没重新编译（`APP_VERSION` 是编译期常量）。\
             重编一份再发：npm run tauri -- build"
                .to_owned(),
        ));
    }
    Ok(())
}

/// Windows：自研安装器在构建时把 `MKP_APP_VERSION` 编进 payload（`--version` 自报）。
/// 这里真跑一遍刚编出来的 setup，把包里那个号与要发的号对账 —— 不再留空实现。
#[cfg(target_os = "windows")]
fn assert_artifact_speaks_version(repo_root: &Path, want: &str) -> Result<(), AppError> {
    let dir = bundle_dir(repo_root, "nsis").ok_or_else(|| {
        AppError::io("找不到 nsis 目录（构建说成功，但 bundle/nsis 下什么都没有）")
            .with_detail(bundle_candidates_text(repo_root, "nsis"))
    })?;
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|e| AppError::io("读不了 nsis 目录").with_detail(e.to_string()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.is_file()
                && p.extension().and_then(|s| s.to_str()) == Some("exe")
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with(PRODUCT))
        })
        .collect();
    found.sort();
    let setup = match found.as_slice() {
        [one] => one,
        [] => return Err(AppError::io("bundle/nsis 下没有安装包")),
        many => {
            return Err(AppError::io(format!(
                "bundle/nsis 下有 {} 份安装包 —— 自己确认要发哪一份",
                many.len()
            ))
            .with_detail(
                many.iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join("、"),
            ))
        }
    };
    let out = std::process::Command::new(setup)
        .arg("--version")
        .output()
        .map_err(|e| AppError::io("读不了安装包自报的版本").with_detail(e.to_string()))?;
    if !out.status.success() {
        return Err(AppError::io("安装包 --version 没成功返回")
            .with_detail(String::from_utf8_lossy(&out.stderr).trim().to_owned()));
    }
    let got = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    if got != want {
        return Err(AppError::invalid_argument(format!(
            "包内自报 {got}，这次要发的是 {want} —— 两个号不是同一个，发出去就对不上账"
        ))
        .with_detail(
            "多半是改了版本号却没重新编译（`MKP_APP_VERSION` 是编译期常量）。\
             重编一份再发：npm run build:installer"
                .to_owned(),
        ));
    }
    Ok(())
}

/// 非 macOS / 非 Windows 的兜底：没有本平台安装包链，保持空实现让调用点不必分支。
#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
fn assert_artifact_speaks_version(repo_root: &Path, want: &str) -> Result<(), AppError> {
    let _ = (repo_root, want);
    Ok(())
}

fn build_installer(repo_root: &Path) -> Result<BuiltArtifact, AppError> {
    assert_no_stale_bundles(repo_root, &version_of(repo_root))?;
    let out = npm_build(repo_root)?;
    if !out.status.success() {
        return Err(AppError::io("构建安装包失败")
            .with_detail(String::from_utf8_lossy(&out.stderr).trim().to_owned()));
    }
    /* 追加说明必须在**造 BuiltArtifact 之前**：它改的是文件字节，大小与 SHA 都跟着变 */
    let one = locate_installer_file(repo_root)?;
    patch_dmg_extras(repo_root, &one)?;
    installer_artifact(repo_root, one)
}

/// 从 `bundle/<本平台>` 找**唯一一份**安装包文件。**不构建、不追加说明**。
fn locate_installer_file(repo_root: &Path) -> Result<PathBuf, AppError> {
    let plan = installer_plan().ok_or_else(|| {
        AppError::invalid_argument(format!(
            "本机是 {} —— 安装包只在 macOS（dmg）与 Windows（NSIS）上构建",
            std::env::consts::OS
        ))
    })?;

    let dir = bundle_dir(repo_root, plan.dir).ok_or_else(|| {
        AppError::io(format!("找不到安装包目录（构建没产出 {}？）", plan.ext))
            .with_detail(bundle_candidates_text(repo_root, plan.dir))
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
        [one] => Ok(one.clone()),
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

/// macOS 的 dmg 追加「安装说明 + 终端快捷方式」：应用未签名，macOS 会拦"浏览器下载"的
/// 第一次打开（"已损坏"）—— 让 dmg 自己带着解法。说明文档能直接打开；终端快捷方式指向
/// Apple 签名的系统应用，也不会被拦。**刻意不放可执行脚本** —— 脚本和应用一样被隔离拦下。
///
/// Windows 不走这一步（SmartScreen 的解法在 Release 说明里），也不往 .exe 里塞东西。
fn patch_dmg_extras(repo_root: &Path, dmg: &Path) -> Result<(), AppError> {
    if !cfg!(target_os = "macos") {
        return Ok(());
    }
    let dmg_path = dmg.display().to_string();
    let patched = std::process::Command::new("bash")
        .args(["scripts/patch-dmg-extras.sh", &dmg_path])
        .current_dir(repo_root)
        .output()
        .map_err(|e| AppError::io("起不了 patch-dmg-extras").with_detail(e.to_string()))?;
    if !patched.status.success() {
        return Err(AppError::io("安装包追加说明失败")
            .with_detail(String::from_utf8_lossy(&patched.stderr).trim().to_owned()));
    }
    Ok(())
}

/// 只补清单那趟：找到这一版的那一份，**不构建、不追加说明**（dmg 在构建时已经打过了）。
fn locate_installer(repo_root: &Path) -> Result<BuiltArtifact, AppError> {
    installer_artifact(repo_root, locate_installer_file(repo_root)?)
}

/// 磁盘上的那一份 → 产物事实（名字用磁盘真名，大小与 SHA 现算）。
fn installer_artifact(repo_root: &Path, one: PathBuf) -> Result<BuiltArtifact, AppError> {
    let plan = installer_plan().ok_or_else(|| {
        AppError::invalid_argument(format!(
            "本机是 {} —— 安装包只在 macOS（dmg）与 Windows（NSIS）上构建",
            std::env::consts::OS
        ))
    })?;
    /* 资产名必须用**磁盘上的真名**：Windows 自研安装器是 `…-setup.exe`，
    按 `asset_name()` 模板拼会得到 `…_x64.exe`，清单与实际附件就对不上。 */
    let name = artifact_name(&one, || {
        super::platform::asset_name(PRODUCT, &version_of(repo_root), arch_name(), plan.ext)
    });
    BuiltArtifact::new(one, name, plan.platform, plan.kind)
}

/// 构建本平台安装包：macOS `npm run tauri -- build`，Windows `npm run build:installer`。
///
/// ★ **Windows 上必须过一层 `cmd /C`**：那边 `npm` 是 `npm.cmd`，而 `Command::new("npm")`
/// Windows 绿色版：`npm run build:portable`，产物是 `dist-portable/` 下带版本号的 zip。
///
/// 它必须和安装版一起进 Release：客户端的"绿色版自更新"下载的是安装包载体，
/// 但 Release 上仍要给愿意手动解压的人一份完整的绿色版。
fn build_portable(repo_root: &Path, version: &str) -> Result<BuiltArtifact, AppError> {
    let out = npm_script(repo_root, "build:portable")?;
    if !out.status.success() {
        return Err(AppError::io("构建绿色版失败")
            .with_detail(String::from_utf8_lossy(&out.stderr).trim().to_owned()));
    }
    locate_portable(repo_root, version)?
        .ok_or_else(|| AppError::io(format!("dist-portable 下没有 {version} 的绿色版 zip")))
}

/// 找 `dist-portable` 下这一版的 zip。**不构建**（InfoOnly 只补清单时用）；没有 = `None`。
fn locate_portable(repo_root: &Path, version: &str) -> Result<Option<BuiltArtifact>, AppError> {
    let dir = repo_root.join("dist-portable");
    if !dir.is_dir() {
        return Ok(None);
    }
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|e| AppError::io("读不了绿色版产物目录").with_detail(e.to_string()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.is_file()
                && p.extension().and_then(|s| s.to_str()) == Some("zip")
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.contains(version))
        })
        .collect();
    found.sort();
    match found.as_slice() {
        [] => Ok(None),
        [one] => {
            let name = one
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_owned();
            BuiltArtifact::new(one.clone(), name, "windows", "portable").map(Some)
        }
        many => Err(AppError::io(format!(
            "dist-portable 下有 {} 份 {version} 的绿色版 —— 自己确认要发哪一份",
            many.len()
        ))
        .with_detail(
            many.iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join("、"),
        )),
    }
}

/// 走的是 `CreateProcess`，它**不会**替你补 `.cmd` 后缀 —— 直接起会报"程序找不到"。
/// 这不是优化，是"在 Windows 上这条命令根本跑不起来"。
fn npm_build(repo_root: &Path) -> Result<std::process::Output, AppError> {
    if cfg!(target_os = "windows") {
        npm_script(repo_root, "build:installer")
    } else {
        npm_script(repo_root, "tauri")
    }
}

/// 跑一个 npm script；Windows 上 `npm` 必须过一层 `cmd /C`。
fn npm_script(repo_root: &Path, script: &str) -> Result<std::process::Output, AppError> {
    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = std::process::Command::new("cmd");
        c.arg("/C").arg("npm");
        c
    } else {
        std::process::Command::new("npm")
    };
    if cfg!(target_os = "windows") {
        cmd.args(["run", script]);
    } else if script == "tauri" {
        cmd.args(["run", "tauri", "--", "build"]);
    } else {
        cmd.args(["run", script]);
    }
    cmd.current_dir(repo_root)
        /* 工作台带着 `TAURI_CONFIG`（`tauri dev --config …` 注入的合并配置）；
        不清掉的话客户端构建会读到工作台那套（空窗口清单），编出一颗装上没窗的包。 */
        .env_remove("TAURI_CONFIG")
        .output()
        .map_err(|e| AppError::io("起不了 npm").with_detail(e.to_string()))
}

/// 打应用内更新用的 `.app.zip`（**系统 `ditto`，不引压缩依赖**）。
///
/// 源是 `bundle/macos/<PRODUCT>.app`，产物落在 dmg 旁边：`SupportEase_<版本>_<arch>.app.zip`。
/// **找不到 `.app` 就如实报错**（不猜路径、不拿别的东西顶替）。
fn build_app_zip(repo_root: &Path, version: &str) -> Result<BuiltArtifact, AppError> {
    let macos_dir = bundle_dir(repo_root, "macos").ok_or_else(|| {
        AppError::io("找不到 macOS 应用包目录")
            .with_detail(bundle_candidates_text(repo_root, "macos"))
    })?;
    let mut apps: Vec<PathBuf> = std::fs::read_dir(&macos_dir)
        .map_err(|e| {
            AppError::io("读不了 macOS 应用包目录")
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
    BuiltArtifact::new(zip_path, name, "macos", "appZip")
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

/// 上一份清单（`rel` 由通道决定；读不到 / 读不懂 = `None` —— 那不是错误：第一次发就是没有）。
fn previous_release_info(repo_root: &Path, rel: &str) -> Option<release_info::ReleaseInfo> {
    let bytes = std::fs::read(repo_root.join(rel)).ok()?;
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
    // **双平台资产**（2026-11-05，可选）。本轮发布侧一律传 `None` —— 能力先具备，
    // 真正写 Windows 包要等下一轮「预览版发布事务」一起做，现有 `release.json` 一个字节不动。
    assets: Option<&[crate::runtime::release_info::ReleaseAsset]>,
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
            "sources": a.sources,
        });
    }
    if let Some(list) = assets {
        value["assets"] = serde_json::json!(list
            .iter()
            .map(|a| {
                serde_json::json!({
                    "name": a.name, "url": a.url, "size": a.size, "sha256": a.sha256,
                    "platform": a.platform, "kind": a.kind, "sources": a.sources,
                })
            })
            .collect::<Vec<_>>());
    }
    // ★ 断代校验在**写盘之前**：坏 URL 不许落盘（总纲 §5-M6）。
    //   两份清单（正式 / 预览）与双平台资产走同一条判定，不另立一套。
    validate_release_source(version, url, kept)?;
    for a in assets.unwrap_or(&[]) {
        validate_release_source(version, "", Some(a))?;
    }
    Ok(serde_json::to_string_pretty(&value)
        .map_err(|e| AppError::internal("发布信息序列化失败").with_detail(e.to_string()))?
        + "\n")
}

/// 写清单（`rel` 由通道决定：`release.json` 或 `release-preview.json`，**上传成功之后才写**）。
///
/// ★ 顺序的意义：先说"有新版"再上传，用户点进去会撞上一个空的下载页。
/// 这里是"安装包已经在 Release 上了"之后才宣告。
fn write_release_info(
    repo_root: &Path,
    rel: &str,
    version: &str,
    notes: &str,
    url: &str,
    asset: Option<&crate::runtime::release_info::ReleaseAsset>,
    assets: Option<&[crate::runtime::release_info::ReleaseAsset]>,
) -> Result<String, AppError> {
    let previous = previous_release_info(repo_root, rel);
    let text = release_info_text(version, notes, url, asset, assets, previous.as_ref())?;
    let path = repo_root.join(rel);
    fsx::atomic::atomic_write(&path, text.as_bytes())?;
    // 写完立刻自己解析一遍：**写进去的东西必须是客户端读得懂的东西**（代次 / 版本形状）
    let bytes = std::fs::read(&path)?;
    release_info::parse(&bytes)?;
    Ok(rel.to_owned())
}

fn release_body(notes: &str, channel: release_info::UpdateChannel) -> String {
    let n = notes.trim();
    // 未签名那句**按平台说**（2026-10-07）：两边的拦截不是同一件事，给同一句等于给错解法
    let unsigned = if cfg!(target_os = "macos") {
        "（未签名：首次打开需要右键 → 打开绕过 Gatekeeper 警告。）"
    } else if cfg!(target_os = "windows") {
        "（未签名：首次运行会被 SmartScreen 拦一下 ——「更多信息」→「仍要运行」。）"
    } else {
        "（未签名的构建。）"
    };
    // ★ 通道要说在正文第一行（2026-11-05）：下载页上"预览版"与"正式版"长得一样，
    //   只有这句能让人在装之前知道自己拿的是什么。
    let kind = match channel {
        release_info::UpdateChannel::Stable => "正式版本",
        release_info::UpdateChannel::Preview => {
            "**预览版**（只在软件里主动点「检查更新」才看得到；正式版用户不会收到通知）"
        }
    };
    if n.is_empty() {
        format!("{PRODUCT} 的{kind}。\n\n{unsigned}")
    } else {
        format!("{n}\n\n{kind}。\n\n{unsigned}")
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

/// 软件发版的事务记录（成功与失败都留），落在 `target/release/journal.json`。
///
/// 它回答"上一趟停在哪一版、有没有跑完"，**不参与**判断"这一版发过没有" ——
/// 那件事由 tag 与 Release 回答。
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TxJournalEntry {
    pub at: String,
    pub version: String,
    pub tag: String,
    pub commit: String,
    /// `started` / 具体 stage / `failed`
    pub stage: String,
    pub ok: bool,
}

/// 追加一条事务记录（保留最近 30 条；只追加，不改写历史条目）。
fn journal_append(repo: &Path, entry: &TxJournalEntry) {
    let path = repo.join("target").join("release").join("journal.json");
    let mut all: Vec<TxJournalEntry> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    all.push(entry.clone());
    if all.len() > 30 {
        let cut = all.len() - 30;
        all.drain(..cut);
    }
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_string_pretty(&all) {
        let _ = crate::fsx::atomic::atomic_write(&path, text.as_bytes());
    }
}

/// **这一版冻结在别的提交上**吗？`Some((tag 的提交, 当前 HEAD))` 表示是。
///
/// 续跑会沿用已存在的 tag、却从**当前工作区**构建 —— 两者不是同一笔提交时，
/// 就会把新代码的包装到旧 tag 的 Release 上（版本号一样，事后查不出来）。
/// 预检与事务共用这一份判断。
fn frozen_elsewhere(git: &Git, tag: &str) -> Option<(String, String)> {
    if !git.tag_exists(tag).ok()? {
        return None;
    }
    let at_tag = git.rev_parse(&format!("refs/tags/{tag}^{{commit}}")).ok()?;
    let at_head = git.rev_parse("HEAD").ok()?;
    (at_tag != at_head).then_some((at_tag, at_head))
}

/// 短 sha（7 位）—— 只用于**给人看**的报错文案。
fn short_sha(sha: &str) -> String {
    sha.chars().take(7).collect()
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
    /// tag 与产物都在，只差 `release.json`：只补清单（不重构建、不重上传、不动 tag）
    InfoOnly,
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
    let has_portable = !cfg!(target_os = "windows")
        || names
            .iter()
            .any(|n| n.to_ascii_lowercase().ends_with(".zip"));
    if super::platform::platform_asset_uploaded(&names, plan.ext) && has_portable {
        /* 产物已上传，但清单还没写成这一版 ⇒ 只补清单，不重跑构建与上传。 */
        let channel = if tag.contains("-preview.") {
            release_info::UpdateChannel::Preview
        } else {
            release_info::UpdateChannel::Stable
        };
        let want = tag.strip_prefix('v').unwrap_or(tag);
        let manifest_ok = previous_release_info(git.repo(), info_rel_for(channel))
            .is_some_and(|info| info.version == want);
        if manifest_ok {
            Ok(ReleaseSlot::Taken)
        } else {
            Ok(ReleaseSlot::InfoOnly)
        }
    } else {
        Ok(ReleaseSlot::Append)
    }
}

/// 这一趟会不会**按当前工作区重新构建**？
///
/// 会 ⇒ tag 必须指向 HEAD（否则新代码的包会挂到旧 tag 上）；
/// 只补清单那一趟（`InfoOnly`）不构建、不重传，产物是 tag 那一刻的，与 HEAD 无关。
fn rebuilds_from_worktree(slot: ReleaseSlot) -> bool {
    !matches!(slot, ReleaseSlot::InfoOnly)
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
    /// 这一趟是**首发**还是**补包**（问过发布仓库的结论，与闸共用 [`release_slot`]）——
    /// 任务书按错的趟别写，AI 就会照着八步去重开 PR / 动版本号。
    mode: PromptMode,
}

/// 提示词要写的**趟别**。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PromptMode {
    /// `{tag}` 还没发过：完整八步
    Fresh,
    /// `{tag}` 已发过、本平台缺包：只补安装包（不重打 tag / 不开新 PR / 不动版本号）
    Append,
    /// tag 与产物都在、只差清单：只补 `release.json`（不重构建 / 不重传 / 不动 tag）
    InfoOnly,
    /// 本平台那一份已经在 Release 上了：没什么可发
    Taken,
    /// 问不出来（发布通道没配 / 网络不通）：照实说，让 AI 以预检结论为准
    Unsure,
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
    // 趟别决定任务书的形状：补包趟把八步换成"构建 + 上传"那几步 —— 照首发写，
    // AI 就会去重开 PR、动版本号（2026-10-07 补 Windows 包那次真踩了）
    let mode_line = match f.mode {
        PromptMode::Fresh => String::new(),
        PromptMode::Append => format!(
            "- 这一趟是**补包**：`{tag}` 已经发过（tag 与 Release 都在发布仓库），\
             缺的只是本平台的安装包 —— 版本号、tag、PR 都不动\n",
            tag = f.tag
        ),
        PromptMode::InfoOnly => format!(
            "- 这一趟**只补清单**：`{tag}` 的 tag 与安装包都在，缺的只是 `release.json`；\
             不重构建、不重传、不动 tag、不开版本 PR\n",
            tag = f.tag
        ),
        PromptMode::Taken => format!(
            "- ⚠ `{tag}` 本平台的安装包**已经在发布仓库上了** —— 这一趟没什么可发；\
             要发新版就换一个版本号\n",
            tag = f.tag
        ),
        PromptMode::Unsure => {
            "- ⚠ 发布仓库这一版本平台发没发过**问不到**（发布通道没配 / 网络不通）\
             —— 跑命令时预检会给结论，照它走\n"
                .to_owned()
        }
    };
    let steps = match f.mode {
        PromptMode::Append => format!(
            r#"## 这条命令会按顺序做（**补包趟** —— 照它核对进度）
1. 预检：会认出这是补包（`{tag}` 已在发布仓库，本平台的安装包还没传）
2. 把 `{tag}` 与 `{base}` 对齐到发布仓库（幂等 —— 早同步过的就是 no-op）
3. 构建本平台安装包：`{asset}`
4. 把安装包传到发布仓库**已有的** Release「{product} {tag}」上
5. {release_info}：上一趟已经写过 / 开过 PR 的话不重开 —— 开了的话**仍要人合并**

★ 补包不重打 tag、不开新 PR、不动版本号 —— 版本号四处保持 {version} 原样。
"#,
            tag = f.tag,
            base = f.base,
            asset = asset,
            product = PRODUCT,
            release_info = RELEASE_INFO_REL,
            version = f.version,
        ),
        PromptMode::InfoOnly => format!(
            r#"## 这条命令会按顺序做（**只补清单** —— 照它核对进度）
1. 预检：认出版本 `{tag}` 的 tag 与安装包都在，只差 `release.json`
2. 不重构建、不重传、不动 tag、不动版本号
3. 写 `{release_info}`，开 PR「发布信息：{tag}」（新分支 `chore/release-{tag}`，base={base}）
4. **仍要人合并那个 PR**，合并后 `npm run ship -- --check` 才算生效
"#,
            tag = f.tag,
            base = f.base,
            release_info = RELEASE_INFO_REL,
        ),
        _ => format!(
            r#"## 这条命令会按顺序做（照它核对进度）
1. 预检：分支 / 工作区干净 / 四处版本号一致 / 这一版这一平台还没发过 / 本机能打这个包
2. 版本号四处一起推进到 {version} → 提交 `chore: {tag}` → 推当前分支
3. 开 PR「发版 {tag}」（base={base}）→ 等 CI → squash 合并
4. 切回 {base} 并同步，记录构建基线
5. 构建本平台安装包：`{asset}`，并核对包里自报的版本
6. **构建与产物校验都通过之后**，才在 {base} 的 tip 上打 annotated tag `{tag}` 并推送
7. 把 tag 与 {base} 推到发布仓库（客户端的数据源读的就是那边的 presets/delivery/）
8. 在发布仓库建 Release「{product} {tag}」，上传安装包
9. 写 {release_info}，开 PR「发布信息：{tag}」（新分支 `chore/release-{tag}`，base={base}）
"#,
            version = f.version,
            tag = f.tag,
            base = f.base,
            asset = asset,
            product = PRODUCT,
            release_info = RELEASE_INFO_REL,
        ),
    };
    // 推荐命令：把「发布账户与凭据从哪来」一并写进同一行（CLI 靠 MKPSE_APP_DIR 复用工作台的配置）
    let cmd = if cfg!(target_os = "windows") {
        format!(
            "$env:MKPSE_APP_DIR=\"{}\"; npm run release -- {} \"{}\"",
            f.app_dir, f.version, notes
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
{mode_line}- 更新说明（会成为 Release 正文与 release.json 的 notes）：{notes}
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

{steps}
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
        mode_line = mode_line,
        steps = steps,
        notes = notes,
        asset = asset,
        release_info = RELEASE_INFO_REL,
        history_file = f.history_file,
    )
}

/// **发射提示词**（"傻瓜化"出口）：把"这一版要怎么发"整成一段能直接贴给 AI 的任务书。
///
/// ★ 只读：不动仓库、不写任何东西 —— 但要**问一句发布仓库**（这一版本平台发过没有，
///   与闸/事务共用 [`release_slot`]）：任务书按首发写、实际是补包的话，AI 照着八步走
///   就会去重开 PR / 动版本号。所以这条命令是 `(async)` —— 网络慢时不挂界面。
#[tauri::command(async)]
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
        // 趟别：问一句发布仓库（与闸同一判据）。问不到就照实写"问不到"，不猜。
        let hosting = channel
            .as_ref()
            .and_then(|t| super::platform::hosting(&t.platform, t.token.clone()));
        let rel_channel = channel
            .as_ref()
            .zip(hosting.as_deref())
            .map(|(target, hosting)| ReleaseChannel { hosting, target });
        let mode = match release_slot(&git, &tag, rel_channel.as_ref()) {
            Ok(ReleaseSlot::Fresh) => PromptMode::Fresh,
            Ok(ReleaseSlot::Append) => PromptMode::Append,
            Ok(ReleaseSlot::InfoOnly) => PromptMode::InfoOnly,
            Ok(ReleaseSlot::Taken) => PromptMode::Taken,
            Err(_) => PromptMode::Unsure,
        };
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
            mode,
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
            Ok(ReleaseSlot::InfoOnly) => push(
                &mut items,
                "tag",
                "版本 · 平台",
                true,
                format!("`{tag}` 的{label}已在发布仓库，只差 `release.json` —— 这一趟只补清单"),
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
            // 记账：这一趟留在 `release-history.json`（**与交付预设那本账分开**）
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
///
/// 读的是**全部数据根**（工作台那棵 + 命令行 / 钩子那棵，见
/// [`super::history_roots`]）：命令行发的那几版同样算数，每条回执带 `source`。
#[tauri::command(async)]
pub fn wb_release_history(
    app: tauri::AppHandle,
) -> Result<super::release_history::ReleaseHistoryView, AppError> {
    crate::ipc::traced("wb_release_history", |_| {
        let roots = super::history_roots(&app)?;
        super::release_history::load_merged(&roots)
    })
}

/// **打开安装包目录**（发布对话框的「打开安装包目录」按钮，2026-10-07 作者要的）：
/// 不管这一趟发没发出去，都让人能去文件管理器里看一眼产物在哪个目录
/// （"我先去打开看一下它放在哪些位置"）。
///
/// 纪律与 `wb_reveal_asset` / `wb_open_external` 同一条：**只读、只开系统文件管理器、
/// 一个应用状态都不碰**。找不到目录就如实拒绝（附候选路径 —— 判据与事务共用
/// [`bundle_dir`]，两处不会漂移），不去猜、也不打开一个空的层。
#[tauri::command(async)]
pub fn wb_release_open_bundle(app: tauri::AppHandle) -> Result<(), AppError> {
    crate::ipc::traced("wb_release_open_bundle", |_| {
        let repo = crate::workbench::paths::repo_root();
        let plan = installer_plan().ok_or_else(|| {
            AppError::invalid_argument(format!(
                "本机是 {} —— 安装包只在 macOS（dmg）与 Windows（NSIS）上构建",
                std::env::consts::OS
            ))
        })?;
        let dir = bundle_dir(&repo, plan.dir).ok_or_else(|| {
            AppError::not_found(format!(
                "{}目录还没建出来（这次构建还没产出 {}）",
                plan.label, plan.ext
            ))
            .with_detail(bundle_candidates_text(&repo, plan.dir))
        })?;
        tauri_plugin_opener::OpenerExt::opener(&app)
            .open_path(dir.to_string_lossy().into_owned(), None::<&str>)
            .map_err(|e| AppError::io("打不开系统文件管理器").with_detail(e.to_string()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **包里自报的号必须等于要发的号**（2026-11-06 作者：「你这都是 0.0.4 了，
    /// 里面还显示 0.0.3，那不是很不合理吗？」）。
    ///
    /// 造一份真的 `.app/Contents/Info.plist`（`plutil` 认这个形状），验两件事：
    /// 一致时放行；不一致时**报错并且把两个号都写进那句话** —— 只说"不匹配"
    /// 等于让人回去猜是哪一头不对。
    #[cfg(target_os = "macos")]
    #[test]
    fn the_artifact_must_report_the_version_we_are_releasing() {
        const PLIST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleShortVersionString</key><string>0.0.11</string>
</dict></plist>
"#;
        let d = tempfile::tempdir().unwrap();
        let contents = d
            .path()
            .join("target/release/bundle/macos/SupportEase.app/Contents");
        std::fs::create_dir_all(&contents).unwrap();
        crate::fsx::atomic::atomic_write(&contents.join("Info.plist"), PLIST.as_bytes()).unwrap();

        assert!(
            assert_artifact_speaks_version(d.path(), "0.0.11").is_ok(),
            "一致就该放行"
        );

        let e = assert_artifact_speaks_version(d.path(), "0.0.12").unwrap_err();
        assert!(
            e.message.contains("0.0.11") && e.message.contains("0.0.12"),
            "两个号都要写进那句话：{}",
            e.message
        );
        assert!(
            e.detail.as_deref().unwrap_or("").contains("重"),
            "要指出多半是没重编：{:?}",
            e.detail
        );
    }

    /// ★ **候选的第一优先是 `CARGO_TARGET_DIR`**（2026-10-07 真机踩出来的）：dev 下
    /// 工作台进程带着 `target-workbench` 在跑（`scripts/tauri-workbench.mjs` 设的），
    /// `npm run tauri -- build` 的子进程原样继承同一个值 —— 包落在
    /// `target-workbench/release/bundle/nsis/`，而旧候选只有两个标准根，
    /// "构建成功"被报成"找不到安装包目录（构建没产出 exe？）"。
    #[test]
    fn bundle_roots_put_cargo_target_dir_first() {
        let repo = PathBuf::from("/tmp/repo");

        // 绝对路径：原样排第一 —— 后头跟着两个标准候选
        let abs_target = std::env::temp_dir().join("mkpse-custom-target");
        let roots = bundle_roots_from(&repo, Some(abs_target.clone().into_os_string()));
        assert_eq!(roots[0], abs_target.join("release").join("bundle"));
        assert_eq!(roots.len(), BUNDLE_ROOT_CANDIDATES.len() + 1);

        // 相对路径：按仓库根解析（工作台进程的 cwd 就是仓库根，与 cargo 的规矩一致）
        let roots = bundle_roots_from(&repo, Some("target-workbench".into()));
        assert_eq!(roots[0], repo.join("target-workbench/release/bundle"));

        // 没设 / 空串：两个标准候选原样，顺序不变
        let plain = bundle_roots_from(&repo, None);
        assert_eq!(plain, bundle_roots_from(&repo, Some("".into())));
        assert_eq!(plain[0], repo.join(BUNDLE_ROOT_CANDIDATES[0]));
        assert_eq!(plain[1], repo.join(BUNDLE_ROOT_CANDIDATES[1]));
    }

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
            mode: PromptMode::Fresh,
        });

        for must in [
            "v1.2.3",                   // tag
            "1.2.3",                    // 版本号
            "feat/demo",                // 当前分支
            "修了 X",                   // 更新说明
            "npm run release -- 1.2.3", // 推荐命令
            "github · o/r",             // 发布账户
            "gitee · o/r",              // 发布通道
            RELEASE_INFO_REL,           // release.json 落点
            "chore/release-v1.2.3",     // 第二个 PR 的分支名
            "必须由人合并",             // 那条不能代劳的纪律
            "ALLOW_PUSH_MAIN",          // 逃生开关不许用
            "MKPSE_APP_DIR",            // 让 CLI 复用工作台的发布账户与账本
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

    /// **补包趟的任务书按补包写**（2026-10-07）：这一版已经发过、只缺本平台的包时，
    /// 任务书还按首发八步写，接话的 AI 就会去重开 PR / 动版本号 —— 都是被闸拦下来
    /// 才知道走错了路。补包任务书要说清"只构建 + 上传"，并把完整八步从里面拿掉。
    #[test]
    fn the_append_prompt_talks_like_an_append_run() {
        let repo = PathBuf::from("/tmp/repo");
        let text = release_prompt_text(&PromptFacts {
            repo: &repo,
            app_dir: "/tmp/app",
            history_file: "/tmp/app/release-history.json",
            branch: "feat/demo",
            version: "0.0.6",
            tag: "v0.0.6",
            notes: "",
            base: "main",
            account: None,
            channel: None,
            plan: installer_plan(),
            mode: PromptMode::Append,
        });

        for must in ["补包", "已有的** Release", "不重打 tag", "仍要人合并"] {
            assert!(text.contains(must), "补包任务书里少了「{must}」：\n{text}");
        }
        for must_not in [
            "版本号四处一起推进", // 首发②的那一步 —— 补包不动版本号
            "开 PR「发版",        // 首发③ —— 补包不开新 PR
        ] {
            assert!(
                !text.contains(must_not),
                "补包任务书里不该有「{must_not}」：\n{text}"
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

        let confirmed = ReleaseOptions {
            confirmed: true,
            ..Default::default()
        };
        let report = run(root, &confirmed, None, None, None).expect("该返回报告");
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
        let confirmed = ReleaseOptions {
            confirmed: true,
            ..Default::default()
        };
        let report = run(root, &confirmed, None, None, None).expect("该返回报告");
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
            RELEASE_INFO_REL,
            "0.0.2",
            "修了几个问题",
            "https://host/r/tag/v0.0.2",
            None,
            None,
        )
        .expect("该写得出来");
        assert_eq!(rel, RELEASE_INFO_REL);
        let bytes = std::fs::read(root.join(&rel)).unwrap();
        let info = release_info::parse(&bytes).expect("客户端该读得懂");
        assert_eq!(info.version, "0.0.2");
        assert_eq!(info.notes, "修了几个问题");
        assert_eq!(info.url, "https://host/r/tag/v0.0.2");
        // 它**不进** catalog / 交付预设那一批 —— 两条链不混
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
            platform: None,
            kind: None,
            sources: None,
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
            platform: None,
            kind: None,
            sources: None,
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
            confirmed: true,
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

    /// **构建失败不许留下 tag**：tag 现在排在构建与产物校验之后。
    #[test]
    fn a_failed_build_leaves_no_tag() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let bare = tempfile::tempdir().unwrap();
        std::process::Command::new("git")
            .args(["init", "-q", "--bare"])
            .current_dir(bare.path())
            .status()
            .unwrap();
        let git = init_repo(root, "main");
        std::process::Command::new("git")
            .args(["remote", "add", "origin", &bare.path().to_string_lossy()])
            .current_dir(root)
            .status()
            .unwrap();
        git.push("main").expect("推 main");
        std::process::Command::new("git")
            .args(["checkout", "-q", "-b", "feat/demo"])
            .current_dir(root)
            .status()
            .unwrap();

        let target = PublishTarget {
            platform: "fake".to_owned(),
            repository_url: bare.path().to_string_lossy().to_string(),
            username: "t".to_owned(),
            token: "fake-token".to_owned(),
            owner: "o".to_owned(),
            repo: "r".to_owned(),
        };
        let fake = FakeHosting::default();
        let opts = ReleaseOptions {
            version: Some("0.0.2".to_owned()),
            notes: "构建失败判据".to_owned(),
            merge: true,
            /* 临时仓库没有 package.json，构建会立刻失败；这一条验的是"失败后没有 tag"。 */
            build: true,
            confirmed: true,
            ..ReleaseOptions::default()
        };
        let err = run(root, &opts, Some(&target), Some(&fake), None).expect_err("构建必须失败");
        assert!(!err.message.is_empty(), "失败要说清原因：{:?}", err);
        assert!(
            !Git::open(root).tag_exists("v0.0.2").unwrap(),
            "构建失败不许留下 tag"
        );
        assert!(fake.releases.borrow().is_empty(), "构建失败不许建 Release");
    }

    /// 假平台：记录调用、返回平台无关的形状。**判据用它跑全流程，不真发一个字节**。
    struct FakeHosting {
        /// 平台名：`"github"` 让事务走镜像那一支，其余值不走
        kind: &'static str,
        releases: std::cell::RefCell<Vec<super::super::platform::ReleaseSpec>>,
        /// 发布仓库上这一版的附件名（判据按它造「补平台 / 发过了」两种局面）
        assets: std::cell::RefCell<Vec<String>>,
        /// 这一趟真的被上传的文件名（按调用顺序）
        uploads: std::cell::RefCell<Vec<String>>,
        /// 这些名字一上传就失败（造"某一类产物传不上去"的局面）
        fail: std::cell::RefCell<Vec<String>>,
    }

    impl Default for FakeHosting {
        fn default() -> Self {
            Self {
                kind: "fake",
                releases: Default::default(),
                assets: Default::default(),
                uploads: Default::default(),
                fail: Default::default(),
            }
        }
    }

    impl super::super::platform::Hosting for FakeHosting {
        fn kind(&self) -> &'static str {
            self.kind
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
            if self.fail.borrow().iter().any(|n| n == &up.name) {
                return Err(AppError::io(format!("假上传失败：{}", up.name)));
            }
            self.uploads.borrow_mut().push(up.name.clone());
            /* 回执大小照磁盘给：内核会拿它跟本地产物核对，写死一个数会让判据失真 */
            let size = std::fs::metadata(&up.path).map(|m| m.len()).unwrap_or(0);
            Ok(super::super::platform::UploadedAsset {
                name: up.name.clone(),
                size,
                url: format!("https://fake/download/{}", up.name),
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

        // 本平台包与绿色版都在、但清单还没写成这一版 ⇒ 只补清单，不重传
        *fake.assets.borrow_mut() = vec![
            format!("SupportEase_0.0.2_x86_64.{ext}"),
            "SupportEase_0.0.2_x64-portable.zip".to_owned(),
        ];
        assert_eq!(
            release_slot(&git, "v0.0.2", Some(&channel)).unwrap(),
            ReleaseSlot::InfoOnly
        );

        // 清单也写成这一版 ⇒ 这一版这一平台发完了，换版本号
        std::fs::create_dir_all(root.join("presets/delivery")).unwrap();
        crate::fsx::atomic::atomic_write(
            &root.join("presets/delivery/release.json"),
            br#"{"releaseSchema":1,"version":"0.0.2","notes":"","url":"https://fake/tag/v0.0.2"}"#,
        )
        .unwrap();
        assert_eq!(
            release_slot(&git, "v0.0.2", Some(&channel)).unwrap(),
            ReleaseSlot::Taken
        );

        // 没有发布通道 = **问不到**：如实报错，不猜"还没发过"
        let err = release_slot(&git, "v0.0.2", None).unwrap_err();
        assert!(err.message.contains("发布通道"), "{}", err.message);
    }

    /// 资产名必须取磁盘真名：Windows 安装器是 `…-setup.exe`，模板拼出来的是 `…_x64.exe`。
    #[test]
    fn artifact_name_uses_the_real_file_name() {
        let p = Path::new("bundle/nsis/SupportEase_0.0.13_x64-setup.exe");
        assert_eq!(
            artifact_name(p, || "SupportEase_0.0.13_x64.exe".to_owned()),
            "SupportEase_0.0.13_x64-setup.exe"
        );
    }

    /// 旧版本残留必须在构建前就能被识别；当前版本自己的包不算残留。
    #[test]
    fn stale_bundle_files_are_only_old_versions() {
        let d = tempfile::tempdir().unwrap();
        let current = d.path().join("SupportEase_0.0.13_x64-setup.exe");
        let old = d.path().join("SupportEase_0.0.12_x64-setup.exe");
        crate::fsx::atomic::atomic_write(&current, b"new").unwrap();
        crate::fsx::atomic::atomic_write(&old, b"old").unwrap();
        let got = stale_bundle_files(d.path(), "0.0.13", "exe");
        assert_eq!(got, vec![old]);
    }

    /// 预检入口确实会在构建前拒绝旧 bundle（不是只在纯函数里成立）。
    #[test]
    fn stale_bundle_blocks_before_build() {
        let Some(plan) = installer_plan() else {
            return;
        };
        let d = tempfile::tempdir().unwrap();
        let dir = d.path().join("target/release/bundle").join(plan.dir);
        std::fs::create_dir_all(&dir).unwrap();
        let old = dir.join(format!("SupportEase_0.0.12_x64.{}", plan.ext));
        crate::fsx::atomic::atomic_write(&old, b"old").unwrap();
        let err = assert_no_stale_bundles(d.path(), "0.0.13").unwrap_err();
        assert!(err.message.contains("旧版本"), "{}", err.message);
    }

    /// 构建后、tag 前：文件名/大小/绿色版 zip 内容任何一项不对，都不许再往下打 tag。
    #[test]
    fn built_artifact_validation_refuses_wrong_name_and_broken_zip() {
        use std::io::Write as _;

        let d = tempfile::tempdir().unwrap();
        let setup = d.path().join("SupportEase_0.0.13_x64-setup.exe");
        crate::fsx::atomic::atomic_write(&setup, b"setup").unwrap();
        let built = BuiltArtifact::new(
            setup,
            "SupportEase_0.0.13_x64-setup.exe".to_owned(),
            "windows",
            "nsis",
        )
        .unwrap();
        let good_zip = d.path().join("SupportEase_0.0.13_x64-portable.zip");
        {
            /* 测试造 zip：zip writer 要 File，不走产品数据落盘。 */
            #[allow(clippy::disallowed_methods)]
            let f = std::fs::File::create(&good_zip).unwrap();
            let mut zw = zip::ZipWriter::new(f);
            for name in ["SupportEase.exe", "卸载 SupportEase.exe"] {
                zw.start_file(name, zip::write::SimpleFileOptions::default())
                    .unwrap();
                zw.write_all(b"x").unwrap();
            }
            zw.finish().unwrap();
        }
        let portable = BuiltArtifact::new(
            good_zip,
            "SupportEase_0.0.13_x64-portable.zip".to_owned(),
            "windows",
            "portable",
        )
        .unwrap();
        assert!(validate_built_artifacts(&[built.clone(), portable.clone()], "0.0.13").is_ok());

        let wrong = BuiltArtifact {
            name: "SupportEase_0.0.13_x64.exe".to_owned(),
            ..built.clone()
        };
        assert!(
            validate_built_artifacts(&[wrong], "0.0.13").is_err(),
            "模板拼的名字必须被拒"
        );

        let empty_zip = d.path().join("SupportEase_0.0.13_x64-portable.zip");
        crate::fsx::atomic::atomic_write(&empty_zip, b"not a zip").unwrap();
        let broken = BuiltArtifact::new(
            empty_zip,
            "SupportEase_0.0.13_x64-portable.zip".to_owned(),
            "windows",
            "portable",
        )
        .unwrap();
        assert!(
            validate_built_artifacts(&[built, broken], "0.0.13").is_err(),
            "坏 zip 必须被拒"
        );
    }

    /// 只有会重新构建的趟才要求"tag 必须坐在 HEAD 上"；补清单那趟不构建，不该被挡。
    #[test]
    fn only_building_slots_require_the_tag_to_sit_on_head() {
        assert!(rebuilds_from_worktree(ReleaseSlot::Fresh));
        assert!(rebuilds_from_worktree(ReleaseSlot::Append));
        assert!(!rebuilds_from_worktree(ReleaseSlot::InfoOnly));
    }

    /// 只补清单那一趟（InfoOnly）：从磁盘上找到**这一版**的那一份，不构建。
    ///
    /// 找不到就必须报错 —— 拿上一版的名单顶上会写成"0.0.15 的版本号 + 0.0.14 的文件名"，
    /// 客户端按这个 URL 去下只会 404。
    #[test]
    fn info_only_locates_the_artifacts_already_on_disk() {
        let Some(plan) = installer_plan() else {
            return;
        };
        let d = tempfile::tempdir().unwrap();
        let dir = d.path().join("target/release/bundle").join(plan.dir);
        std::fs::create_dir_all(&dir).unwrap();
        let setup = dir.join(format!("SupportEase_0.0.15_x64.{}", plan.ext));
        crate::fsx::atomic::atomic_write(&setup, b"setup").unwrap();
        let got = locate_installer(d.path()).unwrap();
        assert_eq!(got.size, 5);
        assert!(got.name.contains("0.0.15"), "{}", got.name);
    }

    /// 绿色版同理：这一版的 zip 在就找到，没有就是 `None`（不报错，非 Windows 本来就没有）。
    #[test]
    fn locate_portable_finds_this_versions_zip_or_says_none() {
        let d = tempfile::tempdir().unwrap();
        let dir = d.path().join("dist-portable");
        std::fs::create_dir_all(&dir).unwrap();
        assert!(locate_portable(d.path(), "0.0.15").unwrap().is_none());
        let zip = dir.join("SupportEase_0.0.15_x64-portable.zip");
        crate::fsx::atomic::atomic_write(&zip, b"zip").unwrap();
        let got = locate_portable(d.path(), "0.0.15").unwrap().unwrap();
        assert_eq!(got.name, "SupportEase_0.0.15_x64-portable.zip");
        assert!(locate_portable(d.path(), "0.0.16").unwrap().is_none());
    }

    /// 清单里的平台 / 类型 / SHA 必须**来自产物自己**：这一层没有字面量可写，
    /// SHA 也不重算（重算就会把假的那一份盖掉 —— 判据正是要它盖不掉）。
    #[test]
    fn manifest_assets_come_from_the_artifact_not_from_literals() {
        let d = tempfile::tempdir().unwrap();
        let dmg = d.path().join("SupportEase_0.0.13_aarch64.dmg");
        crate::fsx::atomic::atomic_write(&dmg, b"payload").unwrap();
        let a = BuiltArtifact {
            sha256: "deadbeef".to_owned(),
            ..BuiltArtifact::new(
                dmg,
                "SupportEase_0.0.13_aarch64.dmg".to_owned(),
                "macos",
                "dmg",
            )
            .unwrap()
        };
        let gh = ("gh-owner".to_owned(), "gh-repo".to_owned());
        let list = manifest_assets(&[&a], "o", "r", "v0.0.13", Some(&gh));
        assert_eq!(
            list[0].platform.as_deref(),
            Some("macos"),
            "不许被写成 windows"
        );
        assert_eq!(list[0].kind.as_deref(), Some("dmg"), "不许被写成 nsis");
        assert_eq!(list[0].sha256, "deadbeef", "清单必须用产物里那一份 SHA");
        assert!(list[0]
            .url
            .starts_with("https://gitee.com/o/r/releases/download/v0.0.13/"));
        assert!(list[0].sources.as_ref().unwrap()["github"]
            .starts_with("https://github.com/gh-owner/gh-repo/releases/download/v0.0.13/"));
    }

    /// 安装包平台词必须与客户端 `host_platform()` 同一套 —— 两边各写一个词就是归类错误的源头。
    #[test]
    fn installer_plan_speaks_the_client_platform_word() {
        let Some(plan) = installer_plan() else {
            return;
        };
        assert_eq!(plan.platform, crate::runtime::release_info::host_platform());
        assert_eq!(plan.kind, if plan.ext == "dmg" { "dmg" } else { "nsis" });
    }

    fn github_target() -> PublishTarget {
        PublishTarget {
            platform: "github".to_owned(),
            repository_url: "https://github.com/o/r".to_owned(),
            username: "u".to_owned(),
            token: "t".to_owned(),
            owner: "o".to_owned(),
            repo: "r".to_owned(),
        }
    }

    /// 镜像把**整份产物列表**传上去（含 macOS 的 `.app.zip`）—— 不再由脚本自己挑文件。
    #[test]
    fn mirror_uploads_every_artifact_in_the_list() {
        let d = tempfile::tempdir().unwrap();
        let dmg = d.path().join("SupportEase_0.0.13_aarch64.dmg");
        let app_zip = d.path().join("SupportEase_0.0.13_aarch64.app.zip");
        crate::fsx::atomic::atomic_write(&dmg, b"dmg").unwrap();
        crate::fsx::atomic::atomic_write(&app_zip, b"zip").unwrap();
        let a = BuiltArtifact::new(
            dmg,
            "SupportEase_0.0.13_aarch64.dmg".to_owned(),
            "macos",
            "dmg",
        )
        .unwrap();
        let b = BuiltArtifact::new(
            app_zip,
            "SupportEase_0.0.13_aarch64.app.zip".to_owned(),
            "macos",
            "appZip",
        )
        .unwrap();
        let fake = FakeHosting {
            kind: "github",
            ..Default::default()
        };
        let mut summary = String::new();
        let done = mirror_artifacts(
            &fake,
            &github_target(),
            "v0.0.13",
            "说明",
            release_info::UpdateChannel::Stable,
            &[&a, &b],
            &mut summary,
        )
        .unwrap();
        assert_eq!(done.len(), 2, "两份都要在 GitHub 上");
        assert_eq!(
            *fake.uploads.borrow(),
            vec![a.name.clone(), b.name.clone()],
            "上传顺序就是产物列表的顺序"
        );
        assert_eq!(fake.releases.borrow().len(), 1, "Release 只建一次");
    }

    /// `.app.zip` 传不上去只剔掉它：这一版退回「打开下载页」，不发版失败。
    #[test]
    fn a_failed_app_zip_is_dropped_but_the_rest_still_mirror() {
        let d = tempfile::tempdir().unwrap();
        let dmg = d.path().join("SupportEase_0.0.13_aarch64.dmg");
        let app_zip = d.path().join("SupportEase_0.0.13_aarch64.app.zip");
        crate::fsx::atomic::atomic_write(&dmg, b"dmg").unwrap();
        crate::fsx::atomic::atomic_write(&app_zip, b"zip").unwrap();
        let a = BuiltArtifact::new(
            dmg,
            "SupportEase_0.0.13_aarch64.dmg".to_owned(),
            "macos",
            "dmg",
        )
        .unwrap();
        let b = BuiltArtifact::new(
            app_zip,
            "SupportEase_0.0.13_aarch64.app.zip".to_owned(),
            "macos",
            "appZip",
        )
        .unwrap();
        let fake = FakeHosting {
            kind: "github",
            fail: std::cell::RefCell::new(vec![b.name.clone()]),
            ..Default::default()
        };
        let mut summary = String::new();
        let done = mirror_artifacts(
            &fake,
            &github_target(),
            "v0.0.13",
            "说明",
            release_info::UpdateChannel::Stable,
            &[&a, &b],
            &mut summary,
        )
        .unwrap();
        assert_eq!(done.len(), 1, "只有 dmg 上去了");
        assert_eq!(done[0].kind, "dmg");
        assert!(summary.contains("打开下载页"), "{summary}");
    }

    /// 补平台时另一平台的资产整组留住：不然 Windows 补包会把 macOS 那两份从清单里抹掉。
    #[test]
    fn appending_a_platform_keeps_the_other_platforms_assets() {
        let previous = release_info::parse(
            br#"{"releaseSchema":1,"version":"0.0.13",
                 "url":"https://gitee.com/o/r/releases/tag/v0.0.13",
                 "assets":[
                   {"name":"old.dmg","url":"https://gitee.com/old.dmg","size":1,
                    "sha256":"aa","platform":"macos","kind":"dmg"},
                   {"name":"old.app.zip","url":"https://gitee.com/old.zip","size":2,
                    "sha256":"bb","platform":"macos","kind":"appZip"}
                 ]}"#,
        )
        .unwrap();
        let fresh = vec![release_info::ReleaseAsset {
            name: "new.exe".to_owned(),
            url: "https://gitee.com/new.exe".to_owned(),
            size: 3,
            sha256: "cc".to_owned(),
            platform: Some("windows".to_owned()),
            kind: Some("nsis".to_owned()),
            sources: None,
        }];
        let merged = merge_assets_by_platform(fresh, Some(&previous), "0.0.13");
        assert_eq!(merged.len(), 3, "本平台 1 份 + 另一平台 2 份");
        assert!(merged.iter().any(|a| a.name == "new.exe"));
        assert!(
            merged.iter().any(|a| a.name == "old.app.zip"),
            "macOS 的应用内更新入口不许被补包抹掉"
        );
    }

    /// 清单写入必须把 Windows 双源资产完整带上：客户端选 GitHub 时读的那一条不能丢。
    #[cfg(target_os = "windows")]
    #[test]
    fn the_written_manifest_carries_both_source_urls() {
        let a = mirrored_asset(
            "SupportEase_0.0.12_x64-setup.exe",
            "https://gitee.com/o/r/releases/download/v0.0.12/setup.exe".to_owned(),
            10,
            "aa".to_owned(),
            "windows",
            "nsis",
            Some("https://github.com/o/r/releases/download/v0.0.12/setup.exe".to_owned()),
        );
        let text = release_info_text(
            "0.0.12",
            "说明",
            "https://gitee.com/o/r/releases/tag/v0.0.12",
            None,
            Some(&[a]),
            None,
        )
        .unwrap();
        let info = release_info::parse(text.as_bytes()).unwrap();
        let gitee = info.asset_for_host(Some("gitee")).expect("Gitee 该有");
        let github = info.asset_for_host(Some("github")).expect("GitHub 该有");
        assert!(gitee.url.starts_with("https://gitee.com/"));
        assert!(github.url.starts_with("https://github.com/"));
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
            platform: None,
            kind: None,
            sources: None,
        };
        let prev = release_info::ReleaseInfo {
            version: "0.0.6".to_owned(),
            notes: "第一版".to_owned(),
            url: "https://gitee.com/o/r/releases/tag/v0.0.6".to_owned(),
            release_schema: RELEASE_SCHEMA,
            asset: Some(mac.clone()),
            assets: None,
        };
        let url = "https://gitee.com/o/r/releases/tag/v0.0.6";

        // 这一趟没有新的 zip（Windows）⇒ 留住上一份那一格
        let text =
            release_info_text("0.0.6", "补一个 Windows 包", url, None, None, Some(&prev)).unwrap();
        let parsed = release_info::parse(text.as_bytes()).unwrap();
        assert_eq!(parsed.asset, Some(mac), "补平台把 macOS 那一格抹掉了");
        assert_eq!(parsed.version, "0.0.6");

        // **别的版本**那份不搬过来（0.0.7 是另一版，它的 asset 该由它自己那一趟写）
        let next = "https://gitee.com/o/r/releases/tag/v0.0.7";
        let text = release_info_text("0.0.7", "", next, None, None, Some(&prev)).unwrap();
        assert_eq!(release_info::parse(text.as_bytes()).unwrap().asset, None);

        // 这一趟有新的 zip（macOS 补包）⇒ 用新的
        let new = release_info::ReleaseAsset {
            name: "SupportEase_0.0.6_x64.app.zip".to_owned(),
            url: "https://gitee.com/o/r/releases/download/v0.0.6/y.app.zip".to_owned(),
            size: 8,
            sha256: "cd".to_owned(),
            platform: None,
            kind: None,
            sources: None,
        };
        let text = release_info_text("0.0.6", "", url, Some(&new), None, Some(&prev)).unwrap();
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

    /// **真发版不带确认 = 零副作用地拒绝**（2026-11-05 硬约束）。
    /// 预演不需要确认；带确认才往下走预检（临时仓库预检本来就会停在 blockedPreflight）。
    #[test]
    fn a_real_software_release_needs_an_explicit_confirmation() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        crate::fsx::atomic::atomic_write(&root.join("Cargo.toml"), b"version = \"0.0.1\"\n")
            .unwrap();
        let err = run(root, &ReleaseOptions::default(), None, None, None)
            .expect_err("没确认的真发版必须被拦下");
        assert!(
            err.message.contains("确认") || err.message.contains("confirm"),
            "拒绝理由得说清缺了什么：{}",
            err.message
        );

        let confirmed = ReleaseOptions {
            confirmed: true,
            ..Default::default()
        };
        match run(root, &confirmed, None, None, None) {
            Ok(_) => {}
            Err(e) => assert!(
                !e.message.contains("显式确认"),
                "带确认就不该再被确认闸拦下（临时仓库预检红是另一回事）：{}",
                e.message
            ),
        }
    }

    /// **动了 tag 就得让 release.json 同代**（2026-11-05 作者原话）。
    #[test]
    fn the_release_info_must_travel_with_the_tag() {
        let stable = release_info::UpdateChannel::Stable;
        let good = r#"{"version":"0.0.10","notes":"","url":"","releaseSchema":1}"#;
        assert!(assert_release_info_matches_tag(good, "0.0.10", "v0.0.10", stable).is_ok());

        let stale = r#"{"version":"0.0.6","notes":"","url":"","releaseSchema":1}"#;
        let err = assert_release_info_matches_tag(stale, "0.0.10", "v0.0.10", stable)
            .expect_err("打了 0.0.10 的 tag 却还留着 0.0.6 的信息源，必须红");
        assert!(
            err.message.contains("0.0.6"),
            "要说清是哪一代：{}",
            err.message
        );
    }

    /// ★ **通道身份不许串台**（2026-11-05 预览通道）：
    /// 预览通道写出正式号 = 客户端会把它当正式版候选；正式通道写出预览号 = 全量用户
    /// 收到预览通知。两种都是"发出去就难撤"的事故，所以在这一步纯函数里红。
    #[test]
    fn a_channel_never_writes_the_other_channels_version() {
        let stable = release_info::UpdateChannel::Stable;
        let preview = release_info::UpdateChannel::Preview;

        // 正常两条：各写各的
        assert!(assert_release_info_matches_tag(
            r#"{"version":"0.0.10","releaseSchema":1}"#,
            "0.0.10",
            "v0.0.10",
            stable
        )
        .is_ok());
        assert!(assert_release_info_matches_tag(
            r#"{"version":"0.0.10-preview.1","releaseSchema":1}"#,
            "0.0.10-preview.1",
            "v0.0.10-preview.1",
            preview
        )
        .is_ok());

        // 串台两种：都该红
        let err = assert_release_info_matches_tag(
            r#"{"version":"0.0.10-preview.1","releaseSchema":1}"#,
            "0.0.10-preview.1",
            "v0.0.10-preview.1",
            stable,
        )
        .expect_err("正式通道不许写预览号");
        assert!(err.message.contains("通道与版本身份"), "{}", err.message);

        let err = assert_release_info_matches_tag(
            r#"{"version":"0.0.10","releaseSchema":1}"#,
            "0.0.10",
            "v0.0.10",
            preview,
        )
        .expect_err("预览通道不许写正式号");
        assert!(err.message.contains("通道与版本身份"), "{}", err.message);
    }

    /// ★ **预览版写预览清单，正式版那份一个字节都不动**（本轮的核心约束）。
    ///
    /// 用临时目录直接调写盘链：两条通道各写一份，落点必须分开、内容各自读得懂，
    /// 而且**正式版那份保持原样**（连注释外的一个空格都没变）。
    #[test]
    fn a_preview_publish_never_touches_the_stable_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("presets/delivery")).unwrap();

        // 盘上先有一份正式清单（模拟已经发过的 0.0.6）
        let stable_before = r#"{
  "releaseSchema": 1,
  "version": "0.0.6",
  "notes": "旧",
  "url": "https://gitee.com/o/r/releases/tag/v0.0.6"
}
"#;
        crate::fsx::atomic::atomic_write(&root.join(RELEASE_INFO_REL), stable_before.as_bytes())
            .unwrap();

        // 预览通道：写预览清单
        let rel = write_release_info(
            root,
            info_rel_for(release_info::UpdateChannel::Preview),
            "0.0.10-preview.1",
            "试试新东西",
            "https://gitee.com/o/r/releases/tag/v0.0.10-preview.1",
            None,
            None,
        )
        .expect("预览清单该写得出来");
        assert_eq!(rel, RELEASE_PREVIEW_REL, "预览通道的落点");
        assert_ne!(rel, RELEASE_INFO_REL, "两条通道不许写同一份");

        // 预览那份读得懂，版本是预览号
        let bytes = std::fs::read(root.join(&rel)).unwrap();
        let info = release_info::parse(&bytes).expect("客户端该读得懂预览清单");
        assert_eq!(info.version, "0.0.10-preview.1");
        assert!(release_info::is_prerelease(&info.version));

        // ★ 正式版那一份**逐字未变**
        assert_eq!(
            std::fs::read_to_string(root.join(RELEASE_INFO_REL)).unwrap(),
            stable_before,
            "发预览不许动正式版的 release.json"
        );
    }

    /// **同一条通道重复发布**：盘上那份已经一模一样时，写盘链返回的内容不变
    /// （事务据此"不切分支、不开 PR"）—— 这是重跑 / 补平台那一趟的依据。
    #[test]
    fn republishing_the_same_preview_version_is_a_no_op() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("presets/delivery")).unwrap();
        let rel = info_rel_for(release_info::UpdateChannel::Preview);
        let url = "https://gitee.com/o/r/releases/tag/v0.0.10-preview.1";

        let first = write_release_info(root, rel, "0.0.10-preview.1", "说明", url, None, None)
            .expect("第一次写");
        let text_first = std::fs::read_to_string(root.join(&first)).unwrap();

        let second = write_release_info(root, rel, "0.0.10-preview.1", "说明", url, None, None)
            .expect("第二次写");
        let text_second = std::fs::read_to_string(root.join(&second)).unwrap();
        assert_eq!(
            text_first, text_second,
            "同一版同一说明重写一次，内容必须一模一样（否则会多开一份空 PR）"
        );
    }

    /// ★ **每个通道的清单落点都必须在暂存白名单里**（2026-11-06 真机事故）。
    ///
    /// 不在表里时 `stage_allowed_in` 会**静默过滤掉**（返回空列表、不报错），于是
    /// "文件写好了但没 stage" → 事务接着报"没有变化，没有可提交的" → **清单 PR 开不出来**。
    /// 发版当天才发现的代价是：tag 已经打了、包已经传了，而用户永远看不到这个版本。
    /// 这条判据把"加通道忘了加白名单"提前成一次编译后的红。
    #[test]
    fn every_channel_manifest_is_stageable() {
        for ch in [
            release_info::UpdateChannel::Stable,
            release_info::UpdateChannel::Preview,
        ] {
            let rel = info_rel_for(ch);
            assert!(
                crate::workbench::app::git::is_allowed_in(rel, &RELEASE_STAGE_ALLOWLIST),
                "{} 通道的清单 {rel} 不在 RELEASE_STAGE_ALLOWLIST 里 —— 写好了也 stage 不上",
                ch.wire()
            );
        }
    }
}

#[cfg(test)]
mod frozen_tag_tests {
    use std::process::Command;

    use super::*;

    /// 造一个真实仓库。钩子关掉：本仓的 `core.hooksPath` 会拦 main 上的提交，
    /// 而这里要验的是"tag 与 HEAD 是不是同一笔"。
    fn repo_with_commit() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let run = |args: &[&str]| {
            let out = Command::new("git")
                .args([
                    "-c",
                    "core.hooksPath=/nonexistent-mkpse-test-hooks",
                    "-c",
                    "user.email=t@example.invalid",
                    "-c",
                    "user.name=t",
                ])
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
        run(&["init", "-q", "-b", "main"]);
        crate::fsx::atomic::atomic_write(&root.join("a.txt"), b"1").unwrap();
        run(&["add", "a.txt"]);
        run(&["commit", "-q", "-m", "one"]);
        dir
    }

    /// **专项验收**：`Tag 已创建 → 构建失败 → 修改源码 → 重跑` 必须阻断。
    ///
    /// 重跑会沿用那个 tag、却从**当前工作区**构建 —— 必须在这里阻断，
    /// 不能把新代码的安装包挂到旧 tag 的 Release 上。
    #[test]
    fn changing_code_after_the_tag_blocks_the_resume() {
        let d = repo_with_commit();
        let root = d.path();
        let git = Git::open(root);
        let tag = "v0.0.12-preview.7";
        let run = |args: &[&str]| {
            let out = Command::new("git")
                .args([
                    "-c",
                    "core.hooksPath=/nonexistent-mkpse-test-hooks",
                    "-c",
                    "user.email=t@example.invalid",
                    "-c",
                    "user.name=t",
                ])
                .args(args)
                .current_dir(root)
                .output()
                .unwrap();
            assert!(out.status.success());
        };

        /* ① 冻结这一版（= 上次已经打了 tag，构建才失败） */
        run(&["tag", "-a", tag, "-m", "本地预览交付"]);
        assert!(
            frozen_elsewhere(&git, tag).is_none(),
            "tag 就打在 HEAD 上 —— 此时续跑是安全的"
        );

        /* ② 改了代码并提交（= 作者发现 bug 改完再重跑） */
        crate::fsx::atomic::atomic_write(&root.join("a.txt"), b"2").unwrap();
        run(&["add", "a.txt"]);
        run(&["commit", "-q", "-m", "two"]);

        /* ③ 必须阻断，且两个提交都要写出来 */
        let hit = frozen_elsewhere(&git, tag).expect("改了代码还放行 = 会把新包挂到旧 tag 上");
        assert!(
            hit.0 != hit.1,
            "两笔提交要不一样：tag={} head={}",
            hit.0,
            hit.1
        );
        assert_eq!(short_sha(&hit.0).len(), 7);
    }

    /// 没有 tag（全新一版）不该被这条判据误伤。
    #[test]
    fn a_fresh_version_is_not_blocked() {
        let d = repo_with_commit();
        let git = Git::open(d.path());
        assert!(frozen_elsewhere(&git, "v0.0.12-preview.8").is_none());
    }

    /// 事务记录：只追加、不改写，超过 30 条丢最旧的。
    #[test]
    fn the_journal_only_appends() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        for i in 0..33 {
            journal_append(
                root,
                &TxJournalEntry {
                    at: format!("t{i}"),
                    version: format!("0.0.12-preview.{i}"),
                    tag: format!("v0.0.12-preview.{i}"),
                    commit: "abc".to_owned(),
                    stage: if i % 2 == 0 { "failed" } else { "Built" }.to_owned(),
                    ok: i % 2 == 1,
                },
            );
        }
        let all: Vec<TxJournalEntry> = serde_json::from_str(
            &std::fs::read_to_string(root.join("target/release/journal.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(all.len(), 30, "只保留最近 30 条");
        assert_eq!(all.last().unwrap().at, "t32", "最后一条是刚写的");
        assert!(
            all.iter().any(|e| !e.ok),
            "失败那几笔也要留着 —— 那是恢复依据"
        );
    }
}
