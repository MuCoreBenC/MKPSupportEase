//! **封版** —— 这一代发布的最后一个动作，也是唯一不可逆的动作。
//!
//! 顺序是死的一个方向：
//!
//! ```text
//! 六条判据全过 → 打正式 tag（指向冻结源码）→ Gitee 正式 Release 收全部资产
//!            → GitHub 草稿改名为正式 tag 并发布 → 逐份核对 → 清暂存
//! ```
//!
//! # 六条判据（任一条不过就不许封版）
//!
//! 1. 所有计划支持的平台都齐了（两个源都在）
//! 2. Gitee 资产完整
//! 3. GitHub 资产完整
//! 4. 两边同一份字节的大小 / 摘要一致
//! 5. `release.json` 含全部平台资产（**main 上那一份**）
//! 6. 版本号 / 源码 SHA / 资产身份一致（冻结源码就是 tag 指的那一笔）
//!
//! 判据一至四与第六条的一半由 [`super::state`] 派生（纯函数、有测试）；这里只做
//! 「不满足就停」和「满足就按顺序做完」。

use std::path::Path;

use crate::error::AppError;
use crate::runtime::release_info::UpdateChannel;
use crate::workbench::app::git::Git;
use crate::workbench::app::platform::{ReleaseSpec, ReleaseUpdate, RemoteRelease};

use super::platforms::{tag_of, ArtifactFact, PRODUCT};
use super::staging::{self, Targets};
use super::state::{ReleaseState, Snapshot};

/// 封版做完之后的回执。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SealOutcome {
    pub tag: String,
    pub gitee: RemoteRelease,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub github: Option<RemoteRelease>,
}

/// 封版前的最后一道闸：判据不过就带着原因停下（零副作用）。
pub fn ensure_sealable(state: &ReleaseState) -> Result<(), AppError> {
    if !state.conflicts.is_empty() {
        return Err(AppError::invalid_argument(format!(
            "这一代有 {} 处冲突，不许封版",
            state.conflicts.len()
        ))
        .with_detail(state.conflicts.join("；")));
    }
    if !state.seal_blockers.is_empty() {
        return Err(AppError::invalid_argument(format!(
            "封版判据有 {} 条没过",
            state.seal_blockers.len()
        ))
        .with_detail(state.seal_blockers.join("；")));
    }
    Ok(())
}

/// 打正式 tag。**指向冻结源码，不是 main 的 tip** —— 清单 PR 合并之后 main 已经前进了，
/// 但这一代发的是被构建的那一笔。
pub fn create_formal_tag(
    git: &Git,
    version: &str,
    source_commit: &str,
    notes: &str,
    summary: &mut Vec<String>,
) -> Result<String, AppError> {
    let tag = tag_of(version);
    if git.tag_exists(&tag)? {
        let at = git.rev_parse(&format!("refs/tags/{tag}^{{commit}}"))?;
        if at != source_commit {
            return Err(AppError::invalid_argument(format!(
                "正式 tag `{tag}` 已经指向 {}，冻结源码是 {} —— 已封版的号不许重打",
                short(&at),
                short(source_commit)
            )));
        }
        summary.push(format!("正式 tag {tag} 已存在（同一笔），沿用"));
        return Ok(tag);
    }
    git.tag_at(&tag, source_commit, &format!("{tag} {}", notes.trim()))?;
    git.push_tag(&tag)?;
    summary.push(format!("已打正式 tag {tag} → {}", short(source_commit)));
    Ok(tag)
}

/// 在 Gitee 建正式 Release 并把**这一代全部资产**收进去（幂等：已有的同字节跳过）。
pub fn publish_gitee(
    t: &Targets<'_>,
    version: &str,
    source_commit: &str,
    notes: &str,
    channel: UpdateChannel,
    facts: &[ArtifactFact],
    summary: &mut Vec<String>,
) -> Result<RemoteRelease, AppError> {
    let tag = tag_of(version);
    let release = t.gitee.create_release(&ReleaseSpec {
        owner: t.gitee_owner.clone(),
        repo: t.gitee_repo.clone(),
        tag_name: tag.clone(),
        target_commitish: Some(source_commit.to_owned()),
        name: format!("{PRODUCT} {tag}"),
        body: release_body(notes, channel),
        prerelease: channel == UpdateChannel::Preview,
        draft: false,
    })?;
    staging::upload_missing(
        t.gitee,
        &t.gitee_owner,
        &t.gitee_repo,
        &tag,
        release.id,
        facts,
        summary,
    )?;
    Ok(release)
}

/// 把 GitHub 草稿改成正式 tag 并发布出去（草稿到这一步才真正创建 `v*` tag）。
pub fn publish_github(
    t: &Targets<'_>,
    version: &str,
    notes: &str,
    channel: UpdateChannel,
    facts: &[ArtifactFact],
    summary: &mut Vec<String>,
) -> Result<Option<RemoteRelease>, AppError> {
    let Some((gh, owner, repo)) = t.github_parts() else {
        return Ok(None);
    };
    let tag = tag_of(version);
    /* 先看正式 tag 上有没有（重跑时已经发布过） */
    if let Some(v) = gh.find_release(owner, repo, &tag)? {
        if v.draft {
            return Err(AppError::invalid_argument(format!(
                "GitHub 上 `{tag}` 还是一份草稿 —— 上次发布没走完，手工确认后再重跑"
            )));
        }
        summary.push(format!("GitHub Release {tag} 已存在（沿用）"));
        return Ok(Some(RemoteRelease {
            platform: "github".to_owned(),
            id: v.id,
            tag_name: v.tag_name,
            url: v.url,
        }));
    }
    let staging_tag = super::platforms::staging_tag_of(version);
    let Some(draft) = gh.find_release(owner, repo, &staging_tag)? else {
        return Err(AppError::invalid_argument(format!(
            "GitHub 上没有 `{staging_tag}` 的暂存草稿 —— 资产还没镜像过，不能发布"
        )));
    };
    if !draft.draft {
        return Err(AppError::invalid_argument(format!(
            "GitHub 上的 `{staging_tag}` 不是草稿"
        )));
    }
    let r = gh.update_release(&ReleaseUpdate {
        owner: owner.to_owned(),
        repo: repo.to_owned(),
        release_id: draft.id,
        tag_name: Some(tag.clone()),
        name: Some(format!("{PRODUCT} {tag}")),
        body: Some(release_body(notes, channel)),
        prerelease: Some(channel == UpdateChannel::Preview),
        draft: Some(false),
    })?;
    summary.push(format!("GitHub 草稿已发布为 {tag}"));
    /* 草稿里那几份资产跟着 Release 一起过来了；万一有缺，这一步补 */
    staging::upload_missing(gh, owner, repo, &tag, r.id, facts, summary)?;
    Ok(Some(r))
}

/// 封版之后**再核一遍**：两个源的正式 Release 上，每一份都同字节。
pub fn verify_after_seal(
    t: &Targets<'_>,
    version: &str,
    facts: &[ArtifactFact],
    summary: &mut Vec<String>,
) -> Result<Vec<String>, AppError> {
    let tag = tag_of(version);
    let mut problems: Vec<String> = Vec::new();
    let gitee = t.gitee.assets(&t.gitee_owner, &t.gitee_repo, &tag)?;
    check_sources("Gitee", &gitee, facts, &mut problems, summary);
    if let Some((gh, owner, repo)) = t.github_parts() {
        let github = gh.assets(owner, repo, &tag)?;
        check_sources("GitHub", &github, facts, &mut problems, summary);
    } else {
        problems.push("没有 GitHub 通道 —— 镜像那一半没做".to_owned());
    }
    Ok(problems)
}

fn check_sources(
    label: &str,
    remote: &[crate::workbench::app::platform::RemoteAsset],
    facts: &[ArtifactFact],
    problems: &mut Vec<String>,
    summary: &mut Vec<String>,
) {
    for f in facts {
        let Some(got) = remote.iter().find(|a| a.name == f.name) else {
            problems.push(format!("{label} 上没有 {}", f.name));
            continue;
        };
        if let Some(size) = got.size {
            if size != f.size {
                problems.push(format!(
                    "{label} 上的 {} 是 {size} 字节，本地 {}",
                    f.name, f.size
                ));
                continue;
            }
        }
        if let Some(sha) = &got.sha256 {
            if !sha.eq_ignore_ascii_case(&f.sha256) {
                problems.push(format!("{label} 上的 {} SHA-256 与本地不同", f.name));
                continue;
            }
        }
        summary.push(format!("{label} 核对通过：{}", f.name));
    }
}

/// 清单落盘 + 提交 + 推 + 开 PR（**基底取 origin/main**）。
///
/// 落点由通道决定；内容由 [`super::manifest::render`] 生成。
#[allow(clippy::too_many_arguments)] // 发布清单的入参都是身份（版本/通道/说明/URL/产物/目标），拆开反而要再造一个结构
pub fn commit_manifest(
    git: &Git,
    repo_root: &Path,
    version: &str,
    channel: UpdateChannel,
    notes: &str,
    url: &str,
    facts: &[ArtifactFact],
    targets: &Targets<'_>,
    summary: &mut Vec<String>,
) -> Result<(String, Vec<String>), AppError> {
    let previous = super::manifest::published_info(repo_root, channel)?;
    let gh = match (&targets.github_owner, &targets.github_repo) {
        (Some(o), Some(r)) => Some((o.clone(), r.clone())),
        _ => None,
    };
    let text = super::manifest::render(
        version,
        channel,
        notes,
        url,
        facts,
        previous.as_ref(),
        &targets.gitee_owner,
        &targets.gitee_repo,
        gh.as_ref(),
    )?;
    let branch = super::manifest::branch_name(version);
    let current = git.branch().unwrap_or_default();
    if current != branch {
        if git.branch_exists(&branch)? {
            git.switch(&branch)?;
        } else {
            /* ★ 从 origin/main 起，不从当前 HEAD 起：否则 PR 会把本地别的提交带进来 */
            git.switch_new_at(&branch, "origin/main")?;
        }
    }
    /* ★ **切完分支再写盘**：先写会让工作区带一份未提交的清单，checkout 直接被拒
       （断点续跑最常见的现场：上一趟写完清单就被打断）。 */
    let rel = super::manifest::write(repo_root, channel, &text)?;
    summary.push(format!("已写 {rel}"));
    let staged = git.stage_allowed_in(
        std::slice::from_ref(&rel),
        &crate::workbench::app::git::RELEASE_STAGE_ALLOWLIST,
    )?;
    /* 提交的是**整个索引**：别人事先 stage 的文件会跟着进这一笔，这里拦下来 */
    let outside: Vec<String> = git
        .staged_names()?
        .into_iter()
        .filter(|p| {
            !crate::workbench::app::git::RELEASE_STAGE_ALLOWLIST
                .iter()
                .any(|a| a == p)
        })
        .collect();
    if !outside.is_empty() {
        return Err(AppError::invalid_argument(
            "索引里还有不在发布白名单里的文件 —— 先清掉再发，否则它们会跟着清单一起提交",
        )
        .with_detail(outside.join("、")));
    }
    if git.has_staged()? {
        git.commit(&super::manifest::pr_title(version))?;
        summary.push(format!("已提交清单（分支 {branch}）"));
    } else {
        summary.push("清单与 main 上那份一致，没有要提交的".to_owned());
        return Ok((branch, staged));
    }
    git.push(&branch)?;
    summary.push(format!("已推送分支 {branch}"));
    Ok((branch, staged))
}

/// 发布说明的正文（正式 / 预览同一套口径）。
pub fn release_body(notes: &str, channel: UpdateChannel) -> String {
    let tail = match channel {
        UpdateChannel::Stable => {
            "正式版本。\n\n（未签名：首次运行会被系统拦一下，dmg 里带安装说明。）"
        }
        UpdateChannel::Preview => "预览版本：只在主动点「检查更新」时出现，不影响正式版用户。",
    };
    let n = notes.trim();
    if n.is_empty() {
        tail.to_owned()
    } else {
        format!("{n}\n\n{tail}")
    }
}

/// 封版收尾：清掉暂存面（Gitee 暂存 Release + staging tag）。
pub fn cleanup(
    t: &Targets<'_>,
    git: &Git,
    version: &str,
    summary: &mut Vec<String>,
) -> Result<(), AppError> {
    if let Some(line) = staging::cleanup_gitee_staging(t, version)? {
        summary.push(line);
    }
    staging::drop_staging_tag(git, version)?;
    summary.push(format!(
        "已清理暂存 tag {}",
        super::platforms::staging_tag_of(version)
    ));
    Ok(())
}

/// 封版那一刻的完整状态（给回执用）。
pub fn snapshot_after(
    t: &Targets<'_>,
    git: &Git,
    version: &str,
    channel: UpdateChannel,
) -> Result<Snapshot, AppError> {
    let mut snap = Snapshot::default();
    staging::fill_snapshot(t, version, channel, git, &mut snap)?;
    Ok(snap)
}

fn short(sha: &str) -> String {
    sha.chars().take(7).collect()
}
