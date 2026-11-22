//! **发布内核 2.0** —— 一次冻结源码，多平台分批构建，资产集中暂存，统一上传两个源，
//! 清单合并之后才封版打 tag。
//!
//! ```text
//! 冻结源码 ──→ 平台构建（macOS 本机 / Windows CI）──→ 暂存（Gitee staging tag + GitHub draft）
//!    │                                                        │
//!    └── 清单 PR（读 origin/main 合并全部平台资产）←───────────┘
//!                          │
//!                    六条判据全过 → 封版（正式 tag + 正式 Release + 清暂存）
//! ```
//!
//! # 与旧内核的根本区别
//!
//! - 旧：「先打 tag 再补资产」——tag 那一刻的清单是上一代；「补平台」是一条特殊分支。
//! - 新：tag 是**最后**一步；每个平台是同一张表里的一行，谁没齐就补谁，没有特殊分支。
//!
//! # 状态从哪来
//!
//! 全部由 [`inspect`] 现读四个远端事实派生（见 [`state`]），本地 journal 只记过程。
//! 于是「换一台机器接着跑」不需要任何本地上下文 —— 内核自己知道下一步做什么。

pub mod build;
pub mod ci;
pub mod journal;
pub mod manifest;
pub mod platforms;
pub mod seal;
pub mod staging;
pub mod state;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::runtime::release_info::UpdateChannel;
use crate::workbench::app::git::Git;
use crate::workbench::app::platform::actions::Actions;
use crate::workbench::app::platform::{Hosting, RemoteRelease, RemoteReview, ReviewId, ReviewSpec};
use crate::workbench::app::publish_tx::{resolve_target, PublishTarget};
use crate::workbench::app::version;

use platforms::{host_platform_id, tag_of, ArtifactFact, Platform, PLATFORMS};
use state::{Phase, PlatformStage, ReleaseState, Snapshot};

/// 一轮发布能停在哪（验收时模拟中断用；正常跑就一直做完）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StopAfter {
    /// 冻结源码、建好暂存面就停
    Freeze,
    /// 构建 + 暂存做完就停（不碰清单）
    Staging,
    /// 清单 PR 合并完就停（不封版）
    Manifest,
    /// 全部做完（缺省）
    Seal,
}

/// 一次发布的入参。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseOptions {
    /// 空 = 用 `version.toml` 的真值
    pub version: Option<String>,
    pub notes: String,
    #[serde(default)]
    pub channel: Option<UpdateChannel>,
    #[serde(default)]
    pub dry_run: bool,
    /// **人类当次确认**：真发布缺它会被零副作用拒绝
    #[serde(default)]
    pub confirmed: bool,
    #[serde(default)]
    pub stop_after: Option<StopAfter>,
    /// 只处理这一个平台（验收时"先做 Windows 再换机器做 Mac"用它）
    #[serde(default)]
    pub only_platform: Option<String>,
}

/// 一轮发布的结果（**状态快照**，不是"成功/失败"两档）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseReport {
    pub version: String,
    pub channel: UpdateChannel,
    pub phase: Phase,
    pub tag: String,
    pub staging_tag: String,
    pub source_commit: Option<String>,
    pub platforms: Vec<state::PlatformState>,
    pub manifest: state::ManifestState,
    pub conflicts: Vec<String>,
    pub seal_blockers: Vec<String>,
    pub next: String,
    /// 这一趟做了什么（逐条）
    pub summary: Vec<String>,
    /// 还在等人做的（例如「清单 PR 已合，下一趟封版」）
    pub pending: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gitee_release: Option<RemoteRelease>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub github_release: Option<RemoteRelease>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review: Option<RemoteReview>,
    pub dry_run: bool,
}

/* ---------- 远端通道 ---------- */

/// 发布要用到的两条远端通道（代码仓 = GitHub，发布仓 = Gitee）。
pub struct Ports {
    pub code: PublishTarget,
    pub code_hosting: Box<dyn Hosting>,
    pub release: PublishTarget,
    pub release_hosting: Box<dyn Hosting>,
}

/// 从发布账户目录凑出两条通道。**缺一条就不往下走** —— 这一代发布两个源都要有。
pub fn ports(app_dir: &Path) -> Result<Ports, AppError> {
    let code = resolve_target(app_dir, Some("github")).map_err(|e| {
        AppError::invalid_argument("发布账户里没有配好 GitHub（PR / 草稿 / CI 都靠它）")
            .with_detail(e.message)
    })?;
    let release = resolve_target(app_dir, Some("gitee")).map_err(|e| {
        AppError::invalid_argument("发布账户里没有配好 Gitee（正式 Release 的主地址）")
            .with_detail(e.message)
    })?;
    let code_hosting = crate::workbench::app::platform::hosting(&code.platform, code.token.clone())
        .ok_or_else(|| AppError::invalid_argument("认不出代码仓平台（只支持 github / gitee）"))?;
    let release_hosting =
        crate::workbench::app::platform::hosting(&release.platform, release.token.clone())
            .ok_or_else(|| {
                AppError::invalid_argument("认不出发布仓平台（只支持 github / gitee）")
            })?;
    Ok(Ports {
        code,
        code_hosting,
        release,
        release_hosting,
    })
}

fn targets<'a>(p: &'a Ports) -> staging::Targets<'a> {
    staging::Targets {
        gitee: p.release_hosting.as_ref(),
        gitee_owner: p.release.owner.clone(),
        gitee_repo: p.release.repo.clone(),
        github: Some(p.code_hosting.as_ref()),
        github_owner: Some(p.code.owner.clone()),
        github_repo: Some(p.code.repo.clone()),
    }
}

/* ---------- 状态现读 ---------- */

/// 现读四个远端事实并派生状态（**只读、可重复调**）。
pub fn inspect(
    git: &Git,
    p: &Ports,
    version: &str,
    channel: UpdateChannel,
) -> Result<ReleaseState, AppError> {
    let t = targets(p);
    let mut snap = Snapshot::default();
    staging::fill_snapshot(&t, version, channel, git, &mut snap)?;
    Ok(state::derive(version, channel, &snap))
}

/* ---------- 主流程 ---------- */

/// 跑一轮发布：**只做状态机要求的下一步**，不做完所有事。
pub fn run(
    repo_root: &Path,
    app_dir: &Path,
    opts: &ReleaseOptions,
) -> Result<ReleaseReport, AppError> {
    if !opts.dry_run && !opts.confirmed {
        return Err(AppError::invalid_argument(
            "真发布需要显式确认：命令行加 --confirm（或工作台现点一次），只想看会做什么用 --dry-run",
        ));
    }
    let git = Git::open(repo_root);
    let truth = version::app_version(repo_root)?;
    let version = opts
        .version
        .clone()
        .unwrap_or_else(|| truth.clone())
        .trim()
        .to_owned();
    let channel = opts.channel.unwrap_or(UpdateChannel::Stable);
    check_channel_shape(&version, channel)?;

    let mut summary: Vec<String> = Vec::new();
    let mut pending: Vec<String> = Vec::new();

    /* 预检：两条通道都要在 —— 这次事故是跑到镜像阶段才 401 */
    let p = ports(app_dir)?;
    summary.push(format!(
        "发布通道：代码仓 {} / 发布仓 {}",
        p.code.platform, p.release.platform
    ));

    /* 冻结源：main 的 tip。版本号必须已经在那一笔上（版本号 PR 先合） */
    git.fetch("origin")?;
    let tip = git.rev_parse("origin/main")?;
    let tip_version = version_at(repo_root, &tip)?;
    if tip_version != version {
        return Err(AppError::invalid_argument(format!(
            "origin/main 上是 {tip_version}，要发的是 {version} —— 版本号那一笔还没合进主线"
        )));
    }

    let t = targets(&p);
    let mut snap = Snapshot::default();
    staging::fill_snapshot(&t, &version, channel, &git, &mut snap)?;
    let mut st = state::derive(&version, channel, &snap);

    if opts.dry_run {
        summary.push("演练：一个字节都没动".to_owned());
        return Ok(report(&st, summary, pending, None, None, None, true));
    }

    /* ---------- 逐阶段推进 ---------- */
    let stop = opts.stop_after.unwrap_or(StopAfter::Seal);

    /* ① 冻结：建暂存 tag + 两边暂存 Release */
    if st.phase == Phase::Preparing || snap.staging_tag_commit.is_none() {
        let source = if st.phase == Phase::Preparing {
            tip.clone()
        } else {
            snap.source_commit().cloned().unwrap_or_else(|| tip.clone())
        };
        let (tag, created) = staging::ensure_staging_tag(&git, &version, &source)?;
        summary.push(if created {
            format!("已冻结源码 {}（{tag}）", short(&source))
        } else {
            format!("冻结源码未变（{tag} → {}）", short(&source))
        });
        staging::ensure_gitee_staging(&t, &version, &source, &opts.notes)?;
        staging::ensure_github_draft(&t, &version, &source, &opts.notes)?;
        summary.push("暂存面就绪（Gitee staging tag + GitHub 草稿）".to_owned());
        refresh(&t, &git, &version, channel, &mut snap, &mut st)?;
    }
    if stop == StopAfter::Freeze {
        pending.push("下一趟：构建各平台".to_owned());
        return Ok(report(&st, summary, pending, None, None, None, false));
    }

    /* ② 构建 + 暂存：只补没齐的平台 */
    let source = st
        .source_commit
        .clone()
        .ok_or_else(|| AppError::internal("状态里没有冻结源码（暂存 tag 丢了）"))?;
    let mut built: Vec<ArtifactFact> = Vec::new();
    let worktree = build::worktree_dir(repo_root, &source);
    for pf in PLATFORMS {
        if let Some(only) = opts.only_platform.as_deref() {
            if pf.id != only {
                continue;
            }
        }
        let ps = st.platforms.iter().find(|x| x.id == pf.id);
        if matches!(
            ps.map(|x| x.stage),
            Some(PlatformStage::Staged) | Some(PlatformStage::Published)
        ) {
            summary.push(format!("{} 已在暂存面，跳过", pf.label));
            continue;
        }
        let facts = build_platform(
            &p,
            repo_root,
            &worktree,
            &version,
            pf,
            &source,
            &mut summary,
        )?;
        let st_tag = staging_tag(&version);
        let gitee_release = staging::ensure_gitee_staging(&t, &version, &source, &opts.notes)?;
        staging::upload_missing(
            p.release_hosting.as_ref(),
            &p.release.owner,
            &p.release.repo,
            &st_tag,
            gitee_release.id,
            &facts,
            &mut summary,
        )?;
        let draft = staging::ensure_github_draft(&t, &version, &source, &opts.notes)?
            .ok_or_else(|| AppError::internal("GitHub 草稿没建起来"))?;
        staging::upload_missing(
            p.code_hosting.as_ref(),
            &p.code.owner,
            &p.code.repo,
            &st_tag,
            draft.id,
            &facts,
            &mut summary,
        )?;
        built.extend(facts);
        refresh(&t, &git, &version, channel, &mut snap, &mut st)?;
    }
    if stop == StopAfter::Staging {
        pending.push("下一趟：生成并合并清单 PR".to_owned());
        return Ok(report(&st, summary, pending, None, None, None, false));
    }

    /* ③ 清单：基底取 origin/main，按平台整组写全 */
    if st.phase == Phase::Staged {
        let facts = facts_for_manifest(&snap, &built, &version)?;
        let url = format!(
            "https://gitee.com/{}/{}/releases/tag/{}",
            p.release.owner,
            p.release.repo,
            tag_of(&version)
        );
        let (branch, staged_paths) = seal::commit_manifest(
            &git,
            repo_root,
            &version,
            channel,
            &opts.notes,
            &url,
            &facts,
            &t,
            &mut summary,
        )?;
        if staged_paths.is_empty() {
            summary.push("清单没有变化（已在 main 上）".to_owned());
        } else {
            let review = open_or_reuse_review(&p, &branch, &version, &mut summary)?;
            let review = wait_checks(&p, &review, &mut summary)?;
            let merged = p.code_hosting.merge_review(
                &ReviewId {
                    owner: p.code.owner.clone(),
                    repo: p.code.repo.clone(),
                    number: review.number,
                },
                crate::workbench::app::platform::MergeMethod::Squash,
            )?;
            let state_text = format!("{:?}", merged.state);
            summary.push(format!("清单 PR #{} 已合并（{state_text}）", merged.number));
            git.switch("main").ok();
            git.pull_ff("origin", "main")?;
        }
        refresh(&t, &git, &version, channel, &mut snap, &mut st)?;
    }
    if stop == StopAfter::Manifest {
        pending.push("下一趟：封版（打正式 tag + 发布正式 Release）".to_owned());
        return Ok(report(&st, summary, pending, None, None, None, false));
    }

    /* ④ 封版 */
    if st.phase == Phase::Verified {
        seal::ensure_sealable(&st)?;
        let source = st
            .source_commit
            .clone()
            .ok_or_else(|| AppError::internal("状态里没有冻结源码"))?;
        let facts = facts_for_manifest(&snap, &built, &version)?;
        let tag = seal::create_formal_tag(&git, &version, &source, &opts.notes, &mut summary)?;
        let gitee = seal::publish_gitee(
            &t,
            &version,
            &source,
            &opts.notes,
            channel,
            &facts,
            &mut summary,
        )?;
        let github =
            seal::publish_github(&t, &version, &opts.notes, channel, &facts, &mut summary)?;
        let problems = seal::verify_after_seal(&t, &version, &facts, &mut summary)?;
        if !problems.is_empty() {
            return Err(AppError::io(format!(
                "封版后核验有 {} 处不对（tag {tag} 已创建，不回退）",
                problems.len()
            ))
            .with_detail(problems.join("；")));
        }
        seal::cleanup(&t, &git, &version, &mut summary)?;
        refresh(&t, &git, &version, channel, &mut snap, &mut st)?;
        return Ok(report(
            &st,
            summary,
            pending,
            Some(gitee),
            github,
            None,
            false,
        ));
    }

    if st.phase == Phase::Sealed {
        summary.push("这一代已封版：再跑只核对，不重复上传、不重复建 tag".to_owned());
    } else if st.phase == Phase::ManifestMerged {
        let joined = st.seal_blockers.join("；");
        pending.push(format!("封版判据没过：{joined}"));
    } else {
        pending.push(st.next.clone());
    }
    Ok(report(&st, summary, pending, None, None, None, false))
}

fn staging_tag(version: &str) -> String {
    platforms::staging_tag_of(version)
}

fn refresh(
    t: &staging::Targets<'_>,
    git: &Git,
    version: &str,
    channel: UpdateChannel,
    snap: &mut Snapshot,
    st: &mut ReleaseState,
) -> Result<(), AppError> {
    *snap = Snapshot::default();
    staging::fill_snapshot(t, version, channel, git, snap)?;
    *st = state::derive(version, channel, snap);
    Ok(())
}

/// 构建一个平台（本机 or CI）。
fn build_platform(
    p: &Ports,
    repo_root: &Path,
    worktree: &Path,
    version: &str,
    pf: &'static Platform,
    source_commit: &str,
    summary: &mut Vec<String>,
) -> Result<Vec<ArtifactFact>, AppError> {
    if platforms::built_locally(pf) {
        if host_platform_id() != Some(pf.id) {
            return Err(AppError::invalid_argument(format!(
                "{} 要在它自己的系统上构建，本机是 {}",
                pf.label,
                host_platform_id().unwrap_or(std::env::consts::OS)
            )));
        }
        let dir = build::ensure_worktree(repo_root, version, source_commit, summary)?;
        let _ = worktree;
        build::build_local(&dir, version, pf, summary)
    } else {
        let actions = Actions::new(p.code.token.clone());
        let mut started = iso_now();
        /* 同一冻结 SHA 上已经有运行 ⇒ 不重新触发（正在跑就等它、已成功就复用产物） */
        let prefer = match ci::reusable_run(&actions, &p.code.owner, &p.code.repo, source_commit)? {
            Some(run) => {
                summary.push(format!(
                    "复用同一冻结源码上的 Windows 构建（run {}，{}）",
                    run.id, run.status
                ));
                /* 复用时不再按时间筛，按 id 认它 */
                started.clear();
                Some(run.id)
            }
            None => {
                ci::dispatch(
                    &actions,
                    &p.code.owner,
                    &p.code.repo,
                    source_commit,
                    version,
                    summary,
                )?;
                None
            }
        };
        let deadline = Instant::now() + Duration::from_secs(60 * 60);
        let run = ci::wait(
            &actions,
            &p.code.owner,
            &p.code.repo,
            source_commit,
            &started,
            prefer,
            deadline,
            summary,
        )?;
        let dest = repo_root.join("target").join("release-artifacts");
        ci::collect(
            &actions,
            &p.code.owner,
            &p.code.repo,
            run.id,
            version,
            pf,
            &dest,
            summary,
        )
    }
}

/// 这一代全部产物的身份：本轮构建过的优先，其余从远端重建。
fn facts_for_manifest(
    snap: &Snapshot,
    built: &[ArtifactFact],
    version: &str,
) -> Result<Vec<ArtifactFact>, AppError> {
    let mut out: Vec<ArtifactFact> = Vec::new();
    for (pf, kind, name) in platforms::planned_artifacts(version) {
        if let Some(f) = built.iter().find(|f| f.name == name) {
            out.push(f.clone());
            continue;
        }
        /* 远端重建：大小取 Gitee 或 GitHub，摘要取 GitHub（Gitee 不返回摘要） */
        let staged = snap
            .gitee_staging
            .iter()
            .find(|a| a.name == name)
            .or_else(|| snap.github_staging.iter().find(|a| a.name == name));
        let published = snap
            .gitee_formal
            .iter()
            .find(|a| a.name == name)
            .or_else(|| snap.github_formal.iter().find(|a| a.name == name));
        let size = staged
            .and_then(|a| a.size)
            .or_else(|| published.and_then(|a| a.size))
            .ok_or_else(|| {
                AppError::not_found(format!("{name} 远端拿不到字节数 —— 这一份还没传上去"))
            })?;
        let sha = snap
            .github_staging
            .iter()
            .find(|a| a.name == name)
            .or_else(|| snap.github_formal.iter().find(|a| a.name == name))
            .and_then(|a| a.sha256.clone())
            .ok_or_else(|| {
                AppError::invalid_argument(format!(
                    "{name} 在 GitHub 上拿不到摘要 —— 清单里的 SHA 无从核对，先补齐镜像"
                ))
            })?;
        out.push(ArtifactFact {
            name: name.clone(),
            platform: pf.id,
            kind: kind.kind,
            size,
            sha256: sha,
            path: None,
        });
    }
    Ok(out)
}

/// 开清单 PR（已经开着就复用）。
fn open_or_reuse_review(
    p: &Ports,
    branch: &str,
    version: &str,
    summary: &mut Vec<String>,
) -> Result<RemoteReview, AppError> {
    if let Some(r) = p
        .code_hosting
        .find_open_review(&p.code.owner, &p.code.repo, branch, "main")?
    {
        summary.push(format!("复用已开着的清单 PR #{}", r.number));
        return Ok(r);
    }
    let r = p.code_hosting.create_review(&ReviewSpec {
        owner: p.code.owner.clone(),
        repo: p.code.repo.clone(),
        head: branch.to_owned(),
        base: "main".to_owned(),
        title: manifest::pr_title(version),
        body: format!(
            "由发布内核写的这一代清单（{}）。\n\n合并之后客户端才看得到新版本；\
             合并后同一趟继续封版：打正式 tag、发布两个源的正式 Release。",
            manifest::pr_title(version)
        ),
    })?;
    summary.push(format!("已开清单 PR #{}", r.number));
    Ok(r)
}

/// 等清单 PR 的 CI 全绿。
fn wait_checks(
    p: &Ports,
    review: &RemoteReview,
    summary: &mut Vec<String>,
) -> Result<RemoteReview, AppError> {
    let deadline = Instant::now() + Duration::from_secs(35 * 60);
    let id = ReviewId {
        owner: p.code.owner.clone(),
        repo: p.code.repo.clone(),
        number: review.number,
    };
    let mut last = String::new();
    loop {
        let got = p.code_hosting.get_review(&id)?;
        match got.checks {
            crate::workbench::app::platform::ChecksSummary::Passed => {
                summary.push(format!("清单 PR #{} CI 全绿", got.number));
                return Ok(got);
            }
            crate::workbench::app::platform::ChecksSummary::Failed => {
                return Err(AppError::io(format!(
                    "清单 PR #{} 的 CI 红了 —— 修完重跑（清单已在分支上）",
                    got.number
                )));
            }
            other => {
                let note = format!("等 CI：{other:?}");
                if note != last {
                    summary.push(note.clone());
                    last = note;
                }
            }
        }
        if Instant::now() > deadline {
            return Err(AppError::io("等清单 PR 的 CI 超时"));
        }
        std::thread::sleep(Duration::from_secs(15));
    }
}

fn report(
    st: &ReleaseState,
    summary: Vec<String>,
    pending: Vec<String>,
    gitee: Option<RemoteRelease>,
    github: Option<RemoteRelease>,
    review: Option<RemoteReview>,
    dry_run: bool,
) -> ReleaseReport {
    ReleaseReport {
        version: st.version.clone(),
        channel: st.channel,
        phase: st.phase,
        tag: st.tag.clone(),
        staging_tag: st.staging_tag.clone(),
        source_commit: st.source_commit.clone(),
        platforms: st.platforms.clone(),
        manifest: st.manifest.clone(),
        conflicts: st.conflicts.clone(),
        seal_blockers: st.seal_blockers.clone(),
        next: st.next.clone(),
        summary,
        pending,
        gitee_release: gitee,
        github_release: github,
        review,
        dry_run,
    }
}

fn check_channel_shape(version: &str, channel: UpdateChannel) -> Result<(), AppError> {
    let is_preview = version::is_preview_version(version);
    match channel {
        UpdateChannel::Stable if !version::is_semver(version) => Err(AppError::invalid_argument(
            format!("正式通道要发 x.y.z，现在是 {version}"),
        )),
        UpdateChannel::Preview if !is_preview => Err(AppError::invalid_argument(format!(
            "预览通道要发 x.y.z-preview.N，现在是 {version}"
        ))),
        _ => Ok(()),
    }
}

fn version_at(repo_root: &Path, sha: &str) -> Result<String, AppError> {
    let out = std::process::Command::new("git")
        .args(["show", &format!("{sha}:version.toml")])
        .current_dir(repo_root)
        .output()
        .map_err(|e| AppError::io("起不了 git").with_detail(e.to_string()))?;
    if !out.status.success() {
        return Err(AppError::io(format!("读不到 {sha} 上的 version.toml")));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .find_map(|l| l.trim().strip_prefix("version = "))
        .map(|v| v.trim().trim_matches('"').to_owned())
        .ok_or_else(|| AppError::corrupted("version.toml 里没有 version"))
}

fn iso_now() -> String {
    crate::workbench::clock::now_iso8601()
}

fn short(sha: &str) -> String {
    sha.chars().take(7).collect()
}

/// 发布账户目录（`MKPSE_APP_DIR` 优先，否则按平台规则算 —— 与工作台同一个 identifier）。
pub fn app_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("MKPSE_APP_DIR") {
        return PathBuf::from(dir);
    }
    let id = "SupportEase-Workbench";
    let home = std::env::var_os("HOME").map(PathBuf::from);
    if cfg!(target_os = "macos") {
        return home
            .map(|h| h.join("Library/Application Support").join(id))
            .unwrap_or_else(|| PathBuf::from(id));
    }
    if cfg!(target_os = "windows") {
        return std::env::var_os("APPDATA")
            .map(|p| PathBuf::from(p).join(id))
            .unwrap_or_else(|| PathBuf::from(id));
    }
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| home.map(|h| h.join(".local/share")))
        .map(|p| p.join(id))
        .unwrap_or_else(|| PathBuf::from(id))
}

/* ---------- 工作台入口（发布按钮只读；危险动作留给 CLI） ---------- */

/// 预检的一格。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreflightItem {
    pub id: String,
    pub label: String,
    pub status: String,
    pub detail: String,
}

/// 工作台「发布」那一栏读的东西。**只读**：真正的发布在 `npm run ship`。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleasePreflight {
    pub items: Vec<PreflightItem>,
    /// 能不能在**工作台**发 —— 2.0 起恒为 `false`（发布要走命令行那一条）
    pub can_release: bool,
    pub current_version: String,
    pub tag: String,
    pub branch: String,
    pub has_account: bool,
    pub dev_watcher_pid: Option<u32>,
}

/// 现读状态（只读命令；网络请求在 `spawn_blocking` 里跑）。
#[tauri::command]
pub async fn wb_release_state(
    app: tauri::AppHandle,
    version: Option<String>,
) -> Result<ReleaseState, AppError> {
    let root = crate::fsx::paths::internal_root(&app)?;
    let repo = crate::workbench::paths::repo_root();
    let want = match version {
        Some(v) if !v.trim().is_empty() => v,
        _ => version::app_version(&repo)?,
    };
    tauri::async_runtime::spawn_blocking(move || {
        crate::ipc::traced("wb_release_state", |_| {
            let git = Git::open(&repo);
            let p = ports(&root)?;
            inspect(&git, &p, &want, UpdateChannel::Stable)
        })
    })
    .await
    .map_err(|e| AppError::internal("状态读取没跑到终局").with_detail(e.to_string()))?
}

/// 工作台那一栏的预检：把派生状态折成几行。
#[tauri::command]
pub async fn wb_release_preflight(
    app: tauri::AppHandle,
    version: Option<String>,
) -> Result<ReleasePreflight, AppError> {
    let root = crate::fsx::paths::internal_root(&app)?;
    let repo = crate::workbench::paths::repo_root();
    let branch = Git::open(&repo).branch().unwrap_or_default();
    let want = match version {
        Some(v) if !v.trim().is_empty() => v,
        _ => version::app_version(&repo)?,
    };
    tauri::async_runtime::spawn_blocking(move || {
        crate::ipc::traced("wb_release_preflight", |_| {
            let git = Git::open(&repo);
            let p = ports(&root);
            let mut items: Vec<PreflightItem> = Vec::new();
            let (state, has_account) = match &p {
                Ok(p) => (inspect(&git, p, &want, UpdateChannel::Stable).ok(), true),
                Err(e) => {
                    items.push(PreflightItem {
                        id: "account".to_owned(),
                        label: "发布账户".to_owned(),
                        status: "fail".to_owned(),
                        detail: e.message.clone(),
                    });
                    (None, false)
                }
            };
            if let Some(st) = &state {
                for pl in &st.platforms {
                    items.push(PreflightItem {
                        id: format!("platform-{}", pl.id),
                        label: pl.label.to_owned(),
                        status: if pl.stage == PlatformStage::Staged
                            || pl.stage == PlatformStage::Published
                        {
                            "pass".to_owned()
                        } else {
                            "fail".to_owned()
                        },
                        detail: format!(
                            "{}（Gitee {} / GitHub {}）",
                            pl.stage.label(),
                            if pl.gitee_done { "齐" } else { "缺" },
                            if pl.github_done { "齐" } else { "缺" }
                        ),
                    });
                }
                items.push(PreflightItem {
                    id: "manifest".to_owned(),
                    label: "main 上的清单".to_owned(),
                    status: if st.manifest.complete { "pass" } else { "fail" }.to_owned(),
                    detail: st
                        .manifest
                        .version
                        .clone()
                        .unwrap_or_else(|| "（读不到）".to_owned()),
                });
            }
            Ok(ReleasePreflight {
                can_release: false,
                current_version: want.clone(),
                tag: tag_of(&want),
                branch,
                has_account,
                dev_watcher_pid: None,
                items,
            })
        })
    })
    .await
    .map_err(|e| AppError::internal("预检没跑到终局").with_detail(e.to_string()))?
}

/// **工作台不再发版**（发布内核 2.0）：留一个明确的拒绝，不留第二条能绕过状态机的路。
#[tauri::command]
pub async fn wb_release_software(
    _app: tauri::AppHandle,
    _opts: serde_json::Value,
) -> Result<serde_json::Value, AppError> {
    Err(AppError::invalid_argument(
        "发布改走命令行唯一入口：npm run ship -- --what stable --confirm",
    )
    .with_detail("工作台只提供「这一代到哪一步」的只读状态，不再有自己的发布路径".to_owned()))
}

/// 读软件版本的发布历史（与预设那一本分开）。
#[tauri::command(async)]
pub fn wb_release_history(
    app: tauri::AppHandle,
) -> Result<crate::workbench::app::release_history::ReleaseHistoryView, AppError> {
    crate::ipc::traced("wb_release_history", |_| {
        let roots = crate::workbench::app::history_roots(&app)?;
        crate::workbench::app::release_history::load_merged(&roots)
    })
}

/// 发版提示词（给 AI / 人复制的唯一一句话）。
#[tauri::command]
pub fn wb_release_prompt() -> Result<String, AppError> {
    Ok(
        "发布走一条命令：`npm run ship -- --what stable --confirm`\n\
        它自己读远端状态：源码冻结 → 各平台构建 → 暂存 → 清单 PR → 封版，\
        谁没齐就补谁。看状态用 `npm run ship -- --check`。"
            .to_owned(),
    )
}

/// 2.0 起工作台不再结束开发进程（那条闸随旧内核一起去掉了）。
#[tauri::command]
pub fn wb_release_kill_dev_watcher() -> Result<String, AppError> {
    Ok("发布改走命令行：在终端里停掉 dev 会话再跑 npm run ship".to_owned())
}
