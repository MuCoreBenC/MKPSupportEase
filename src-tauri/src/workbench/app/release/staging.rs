//! **暂存面** —— 各平台产物在封版之前的落脚处。
//!
//! ```text
//! Gitee   Release @ staging/<版本>   （没有草稿概念，用专属 staging tag 承载）
//! GitHub  Release  draft:true        （草稿不创建 tag，直到封版那一刻才发布）
//! ```
//!
//! # 为什么必须有个暂存面
//!
//! 两边 Release 都挂在 tag 上。要让「正式 tag 只在封版时出现」，资产就必须先落在
//! **不叫 `v*`** 的地方 —— 于是一条 staging tag 承载 Gitee 的暂存 Release，
//! 一份 GitHub 草稿承载镜像。封版时把同一批字节放进正式 Release（Gitee 不能搬附件，
//! 只能重传；GitHub 把草稿改个 tag 发布出去）。
//!
//! # 幂等
//!
//! 每一份上传前都先读远端：同名同大小 ⇒ 直接跳过（被打断后重跑不会重传几十 MB）。
//! 同名但字节不同 ⇒ 先删再传（**不跳过**，否则清单里的 SHA 与远端对不上）。

use std::path::Path;

use crate::error::AppError;
use crate::runtime::release_info::UpdateChannel;
use crate::workbench::app::git::Git;
use crate::workbench::app::platform::{
    content_type_for, AssetUpload, Hosting, ReleaseSpec, RemoteAsset, RemoteRelease,
};

use super::platforms::{staging_tag_of, ArtifactFact, PRODUCT};
use super::state::Snapshot;

/// 一次暂存操作要用的远端目标。
pub struct Targets<'a> {
    /// Gitee（主地址）
    pub gitee: &'a dyn Hosting,
    pub gitee_owner: String,
    pub gitee_repo: String,
    /// GitHub（镜像 / 草稿）
    pub github: Option<&'a dyn Hosting>,
    pub github_owner: Option<String>,
    pub github_repo: Option<String>,
}

impl Targets<'_> {
    pub(super) fn github_parts(&self) -> Option<(&dyn Hosting, &str, &str)> {
        match (self.github, &self.github_owner, &self.github_repo) {
            (Some(h), Some(o), Some(r)) => Some((h, o.as_str(), r.as_str())),
            _ => None,
        }
    }
}

/// **冻结源码**：建 staging tag（已存在就核对它指向的那一笔）。
///
/// 返回 `(tag, 是否新建)`。已存在但指向别的提交 ⇒ 停下（这是冲突，不是"重跑"）。
pub fn ensure_staging_tag(
    git: &Git,
    version: &str,
    source_commit: &str,
) -> Result<(String, bool), AppError> {
    let tag = staging_tag_of(version);
    if git.tag_exists(&tag)? {
        let at = git.rev_parse(&format!("refs/tags/{tag}^{{commit}}"))?;
        if at != source_commit {
            return Err(AppError::invalid_argument(format!(
                "暂存 tag `{tag}` 已经指着 {}，这一趟要冻结的是 {} —— \
                 冻结过的源码不许换（换号或先清理那一代）",
                short(&at),
                short(source_commit)
            )));
        }
        return Ok((tag, false));
    }
    git.tag(&tag, &format!("{tag} 发布暂存（正式 tag 在封版时才创建）"))?;
    /* 推到 origin：换一台机器要能从这里读到「冻结的是哪一笔」。
    正式仓库（Gitee）上的同名 tag 由建暂存 Release 时按 target_commitish 建出来。 */
    git.push_tag(&tag)?;
    Ok((tag, true))
}

/// 清掉 staging tag（本地 + 两端）。封版收尾用。
pub fn drop_staging_tag(git: &Git, version: &str) -> Result<(), AppError> {
    let tag = staging_tag_of(version);
    if !git.tag_exists(&tag)? {
        return Ok(());
    }
    git.delete_tag_local(&tag)?;
    /* 远端那一条**能删就删**：它不在 `v*` 命名空间里，不会被 ruleset 保护。
    删不掉（比如别处没有这个 tag）不算失败 —— 本地那一条已经没了。 */
    let _ = git.delete_tag_remote("origin", &tag);
    Ok(())
}

/// 确保 Gitee 上的暂存 Release 存在。
pub fn ensure_gitee_staging(
    t: &Targets<'_>,
    version: &str,
    source_commit: &str,
    notes: &str,
) -> Result<RemoteRelease, AppError> {
    let tag = staging_tag_of(version);
    if let Some(v) = t.gitee.find_release(&t.gitee_owner, &t.gitee_repo, &tag)? {
        return Ok(RemoteRelease {
            platform: "gitee".to_owned(),
            id: v.id,
            tag_name: v.tag_name,
            url: v.url,
        });
    }
    t.gitee.create_release(&ReleaseSpec {
        owner: t.gitee_owner.clone(),
        repo: t.gitee_repo.clone(),
        tag_name: tag,
        target_commitish: Some(source_commit.to_owned()),
        name: format!("{PRODUCT} {version}（暂存）"),
        body: staging_body(notes),
        /* 暂存面标成 prerelease：它不该在仓库首页冒充"最新正式版" */
        prerelease: true,
        draft: false,
    })
}

/// 确保 GitHub 上的**草稿** Release 存在（草稿不创建 tag）。
pub fn ensure_github_draft(
    t: &Targets<'_>,
    version: &str,
    source_commit: &str,
    notes: &str,
) -> Result<Option<RemoteRelease>, AppError> {
    let Some((gh, owner, repo)) = t.github_parts() else {
        return Ok(None);
    };
    let tag = staging_tag_of(version);
    if let Some(v) = gh.find_release(owner, repo, &tag)? {
        if !v.draft {
            return Err(AppError::invalid_argument(format!(
                "GitHub 上的 `{tag}` 不是草稿 —— 那一份已经对外可见，不能当暂存面用"
            )));
        }
        return Ok(Some(RemoteRelease {
            platform: "github".to_owned(),
            id: v.id,
            tag_name: v.tag_name,
            url: v.url,
        }));
    }
    let r = gh.create_release(&ReleaseSpec {
        owner: owner.to_owned(),
        repo: repo.to_owned(),
        tag_name: tag,
        target_commitish: Some(source_commit.to_owned()),
        name: format!("{PRODUCT} {version}（暂存）"),
        body: staging_body(notes),
        prerelease: true,
        draft: true,
    })?;
    Ok(Some(r))
}

fn staging_body(notes: &str) -> String {
    let mut s = String::from("发布中的暂存 Release —— 资产齐了才会进正式 Release。\n");
    if !notes.trim().is_empty() {
        s.push('\n');
        s.push_str(notes.trim());
        s.push('\n');
    }
    s
}

/// 把缺的资产传上去（**同名同字节跳过**；同名不同字节先删再传）。
///
/// 返回真正上传了的那几份（给回执和摘要用）。
pub fn upload_missing(
    hosting: &dyn Hosting,
    owner: &str,
    repo: &str,
    tag: &str,
    release_id: u64,
    facts: &[ArtifactFact],
    summary: &mut Vec<String>,
) -> Result<Vec<String>, AppError> {
    let existing = hosting.assets(owner, repo, tag)?;
    let mut uploaded = Vec::new();
    for f in facts {
        let remote = existing.iter().find(|a| a.name == f.name);
        if let Some(a) = remote {
            let same_size = a.size.map(|s| s == f.size).unwrap_or(true);
            let same_sha = a
                .sha256
                .as_deref()
                .map(|s| s.eq_ignore_ascii_case(&f.sha256))
                .unwrap_or(true);
            if same_size && same_sha {
                summary.push(format!("{} 已在远端（同字节，跳过）", f.name));
                continue;
            }
            /* 同名不同字节：删掉再传。不删的话远端留着旧包，清单写的却是新 SHA。 */
            if let Some(id) = a.id {
                hosting.delete_asset(owner, repo, release_id, id)?;
                summary.push(format!("{} 远端那一份字节不同，已删除重传", f.name));
            } else {
                return Err(AppError::invalid_argument(format!(
                    "{} 在远端已存在且字节不同，但平台不给附件 id —— 手动清掉那一份再重跑",
                    f.name
                )));
            }
        }
        let path = f.path.as_ref().ok_or_else(|| {
            AppError::invalid_argument(format!(
                "{} 只有身份没有本地文件 —— 上传前必须先把包拿到本机",
                f.name
            ))
        })?;
        let up = hosting.upload_asset(&AssetUpload {
            owner: owner.to_owned(),
            repo: repo.to_owned(),
            release_id,
            name: f.name.clone(),
            content_type: content_type_for(&f.name).to_owned(),
            path: path.clone(),
        })?;
        /* 回执必须与本地产物一致：大小对不上说明传上去的不是这一份 */
        if up.size != f.size {
            return Err(AppError::invalid_argument(format!(
                "{} 上传回执 {} 字节，本地 {} 字节 —— 传上去的不是这一份",
                f.name, up.size, f.size
            )));
        }
        summary.push(format!("已上传 {}（{} 字节）", f.name, up.size));
        uploaded.push(f.name.clone());
    }
    Ok(uploaded)
}

/// 暂存面上的资产清单（给状态派生用）。
pub fn staging_assets(
    t: &Targets<'_>,
    version: &str,
) -> Result<(Vec<RemoteAsset>, Vec<RemoteAsset>), AppError> {
    let tag = staging_tag_of(version);
    let gitee = t.gitee.assets(&t.gitee_owner, &t.gitee_repo, &tag)?;
    let github = match t.github_parts() {
        Some((gh, owner, repo)) => gh.assets(owner, repo, &tag)?,
        None => Vec::new(),
    };
    Ok((gitee, github))
}

/// 往 [`Snapshot`] 里灌**暂存面**与**正式面**的事实（[`super::inspect`] 的唯一实现处）。
pub fn fill_snapshot(
    t: &Targets<'_>,
    version: &str,
    channel: UpdateChannel,
    git: &Git,
    snap: &mut Snapshot,
) -> Result<(), AppError> {
    let staging = staging_tag_of(version);
    let formal = super::platforms::tag_of(version);
    snap.staging_tag_commit = tag_commit(git, &staging)?;
    snap.formal_tag_commit = tag_commit(git, &formal)?;

    snap.gitee_staging = if snap.staging_tag_commit.is_some() {
        t.gitee.assets(&t.gitee_owner, &t.gitee_repo, &staging)?
    } else {
        Vec::new()
    };
    snap.gitee_formal = t.gitee.assets(&t.gitee_owner, &t.gitee_repo, &formal)?;

    if let Some((gh, owner, repo)) = t.github_parts() {
        snap.github_staging = if snap.staging_tag_commit.is_some() {
            gh.assets(owner, repo, &staging)?
        } else {
            Vec::new()
        };
        snap.github_staging_draft = gh
            .find_release(owner, repo, &staging)?
            .map(|v| v.draft)
            .unwrap_or(true);
        snap.github_formal = gh.assets(owner, repo, &formal)?;
    } else {
        /* 没有 GitHub 通道 = 这一趟只能做 Gitee 那一半，状态里如实体现（不假装齐了） */
        snap.github_staging_draft = true;
    }

    snap.main = super::manifest::published_info(git.repo(), channel)?;
    Ok(())
}

fn tag_commit(git: &Git, tag: &str) -> Result<Option<String>, AppError> {
    if !git.tag_exists(tag)? {
        return Ok(None);
    }
    Ok(Some(git.rev_parse(&format!("refs/tags/{tag}^{{commit}}"))?))
}

/// 封版收尾：删暂存 Release（Gitee 那一份就是全删；GitHub 那一份此时已经改成正式，
/// 不再删）。
pub fn cleanup_gitee_staging(t: &Targets<'_>, version: &str) -> Result<Option<String>, AppError> {
    let tag = staging_tag_of(version);
    let Some(v) = t.gitee.find_release(&t.gitee_owner, &t.gitee_repo, &tag)? else {
        return Ok(None);
    };
    t.gitee
        .delete_release(&t.gitee_owner, &t.gitee_repo, v.id)?;
    Ok(Some(format!("已删除暂存 Release（Gitee {tag}）")))
}

fn short(sha: &str) -> String {
    sha.chars().take(7).collect()
}

/// 产物在盘上？给执行器判断「这一份还要不要构建」。
pub fn local_path_present(f: &ArtifactFact) -> bool {
    f.path.as_deref().map(Path::exists).unwrap_or(false)
}
