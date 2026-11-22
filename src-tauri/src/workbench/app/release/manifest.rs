//! **发布清单**（`presets/delivery/release.json` / `release-preview.json`）。
//!
//! # 三条纪律
//!
//! 1. **基底永远取 `origin/main`**。补一个平台/重跑一趟时，合并的是「main 上已发布的
//!    那一份」，不是某一笔 tag 里的旧清单 —— 2026-11-22 的事故就是读错了这一处。
//! 2. **清单只由发布命令产生**。人不手改；写入走 [`write`]（原子写）。
//! 3. **每一份资产都在远端真的存在**才许写进去。上传回执与字节数在 [`super::staging`]
//!    那一步核过，这里只按身份摊开，不再自己推导平台。

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;

use crate::error::AppError;
use crate::fsx::atomic::atomic_write;
use crate::runtime::release_info::{ReleaseAsset, ReleaseInfo, UpdateChannel};

use super::platforms::{planned_artifacts, ArtifactFact, PRODUCT, TAG_PREFIX};

/// 正式版清单落点。
pub const RELEASE_INFO_REL: &str = "presets/delivery/release.json";
/// 预览版清单落点（与正式版**互不覆盖**）。
pub const RELEASE_PREVIEW_REL: &str = "presets/delivery/release-preview.json";

/// **断代线**（M6）：从这一版起 `release.json` 的 URL 一律来自 Gitee Release。
/// 与 `scripts/check-release-source.mjs` 里的那份副本必须同值。
pub const FIRST_GITEE_RELEASE: &str = "0.0.6";

/// 这一趟该写哪一份清单（**只此一处映射**）。
pub fn info_rel(channel: UpdateChannel) -> &'static str {
    match channel {
        UpdateChannel::Stable => RELEASE_INFO_REL,
        UpdateChannel::Preview => RELEASE_PREVIEW_REL,
    }
}

/// 一份资产的下载地址（主地址 Gitee + 可选的 GitHub 镜像）。
fn urls(
    owner: &str,
    repo: &str,
    tag: &str,
    name: &str,
    gh: Option<&(String, String)>,
) -> (String, BTreeMap<String, String>) {
    let gitee = format!("https://gitee.com/{owner}/{repo}/releases/download/{tag}/{name}");
    let mut sources = BTreeMap::new();
    sources.insert("gitee".to_owned(), gitee.clone());
    if let Some((o, r)) = gh {
        sources.insert(
            "github".to_owned(),
            format!("https://github.com/{o}/{r}/releases/download/{tag}/{name}"),
        );
    }
    (gitee, sources)
}

/// **纯函数**：这一代该写什么清单。
///
/// `facts` 是这一代**全部计划产物**的身份（顺序按平台表）。少一份就是错 ——
/// 客户端会以为某个平台"这一版不提供"，而实际是发布没做完。
#[allow(clippy::too_many_arguments)] // 清单的输入就是这些事实（版本/通道/说明/URL/产物/基底/仓库/镜像）
pub fn render(
    version: &str,
    channel: UpdateChannel,
    notes: &str,
    url: &str,
    facts: &[ArtifactFact],
    previous: Option<&ReleaseInfo>,
    owner: &str,
    repo: &str,
    gh: Option<&(String, String)>,
) -> Result<String, AppError> {
    /* 通道与版本身份必须一致：预览号走正式通道会把预览推给全量用户，反过来则谁也看不到 */
    let is_preview = version.contains("-preview.");
    if (channel == UpdateChannel::Preview) != is_preview {
        return Err(AppError::invalid_argument(format!(
            "通道 {channel:?} 与版本号 {version} 不匹配",
        )));
    }

    let planned = planned_artifacts(version);
    let missing: Vec<String> = planned
        .iter()
        .filter(|(_, _, name)| !facts.iter().any(|f| &f.name == name))
        .map(|(_, _, name)| name.clone())
        .collect();
    if !missing.is_empty() {
        return Err(AppError::invalid_argument(format!(
            "清单要写全这一代的每一份产物，但缺 {} 份：{}",
            missing.len(),
            missing.join("、")
        )));
    }

    let tag = format!("{TAG_PREFIX}{version}");
    let mut assets: Vec<ReleaseAsset> = Vec::new();
    for (_, kind, name) in &planned {
        let fact = facts
            .iter()
            .find(|f| &f.name == name)
            .expect("上面已经核过缺项");
        let (gitee, sources) = urls(owner, repo, &tag, name, gh);
        assets.push(ReleaseAsset {
            name: name.clone(),
            url: gitee,
            size: fact.size,
            sha256: fact.sha256.clone(),
            platform: Some(fact.platform.to_owned()),
            kind: Some(kind.kind.to_owned()),
            sources: Some(sources),
        });
    }

    /* 老客户端的单资产入口（`asset` 格）—— 只认 macOS 的应用内更新包 */
    let asset = assets
        .iter()
        .find(|a| a.kind.as_deref() == Some("appZip"))
        .cloned();

    /* 说明与下载页：**同一版本**重跑时沿用 main 上那一份（补平台不该把发布说明换掉） */
    let (notes, url) = match previous.filter(|p| p.version.trim() == version.trim()) {
        Some(p) if !p.notes.trim().is_empty() => (p.notes.clone(), p.url.clone()),
        _ => (notes.to_owned(), url.to_owned()),
    };

    /* 字段顺序是契约的一部分（与盘上那一份逐字可比）：`releaseSchema` 在最前，
    `assets` 在最后。`serde_json` 开了 `preserve_order`，按插入序落盘。 */
    let mut value = serde_json::json!({
        "releaseSchema": crate::runtime::release_info::RELEASE_SCHEMA,
        "version": version,
        "notes": notes.trim(),
        "url": url,
    });
    if let Some(a) = &asset {
        value["asset"] = serde_json::json!({
            "name": a.name,
            "url": a.url,
            "size": a.size,
            "sha256": a.sha256,
            "sources": a.sources,
        });
    }
    value["assets"] = serde_json::json!(assets
        .iter()
        .map(|a| serde_json::json!({
            "name": a.name,
            "url": a.url,
            "size": a.size,
            "sha256": a.sha256,
            "platform": a.platform,
            "kind": a.kind,
            "sources": a.sources,
        }))
        .collect::<Vec<_>>());
    let text = serde_json::to_string_pretty(&value)
        .map_err(|e| AppError::internal("清单序列化失败").with_detail(e.to_string()))?;
    assert_epoch_legal(&text)?;
    Ok(format!("{text}\n"))
}

/// 断代校验：这一版的下载地址必须是 Gitee（GitHub 只能作 `sources` 镜像）。
pub fn assert_epoch_legal(text: &str) -> Result<(), AppError> {
    let info: ReleaseInfo = serde_json::from_str(text)
        .map_err(|e| AppError::corrupted("清单不是合法 JSON").with_detail(e.to_string()))?;
    if !crate::workbench::app::version::is_newer(&info.version, FIRST_GITEE_RELEASE)
        && info.version.trim() != FIRST_GITEE_RELEASE
    {
        /* 断代线之前的历史档照旧合法 */
        return Ok(());
    }
    let mut bad: Vec<String> = Vec::new();
    if !info.url.is_empty() && !info.url.contains("gitee.com") {
        bad.push(format!("url = {}", info.url));
    }
    for a in info.assets.as_deref().unwrap_or(&[]) {
        if !a.url.contains("gitee.com") {
            bad.push(format!("{} 的主地址不是 Gitee", a.name));
        }
        for (src, u) in a.sources.iter().flatten() {
            let host = match src.as_str() {
                "gitee" => "gitee.com",
                "github" => "github.com",
                _ => continue,
            };
            if !u.contains(host) {
                bad.push(format!("{} 的 {src} 地址不是 {host}", a.name));
            }
        }
    }
    if bad.is_empty() {
        Ok(())
    } else {
        Err(AppError::invalid_argument(format!(
            "清单里的下载地址不合断代线（{} 起只认 Gitee 主地址）：{}",
            FIRST_GITEE_RELEASE,
            bad.join("；")
        )))
    }
}

/// 读`origin/main` 上的清单（**唯一合法基底**）。
///
/// 读不到就是 `None`（第一版 / 仓库还没有清单）—— 不拿工作区那份顶上。
pub fn published_info(
    repo_root: &Path,
    channel: UpdateChannel,
) -> Result<Option<ReleaseInfo>, AppError> {
    let rel = info_rel(channel);
    let out = std::process::Command::new("git")
        .args(["show", &format!("origin/main:{rel}")])
        .current_dir(repo_root)
        .output()
        .map_err(|e| AppError::io("起不了 git").with_detail(e.to_string()))?;
    if !out.status.success() {
        return Ok(None);
    }
    match crate::runtime::release_info::parse(&out.stdout) {
        Ok(info) => Ok(Some(info)),
        Err(e) => Err(AppError::corrupted(format!(
            "origin/main 上的 {rel} 读不懂（不拿它当基底）"
        ))
        .with_detail(e.message)),
    }
}

/// 落盘（原子写）。返回相对仓库根的路径。
pub fn write(repo_root: &Path, channel: UpdateChannel, text: &str) -> Result<String, AppError> {
    let rel = info_rel(channel);
    atomic_write(&repo_root.join(rel), text.as_bytes())?;
    Ok(rel.to_owned())
}

/// 清单 PR 的分支名。
///
/// ★ 不带 `v` 号也带序号：历史上有过 `chore/release-v0.0.15`，而那一支的 PR 已经合并 ——
/// `pr.mjs` 按分支找 PR 会判「已合并」而拒绝新建。
pub fn branch_name(version: &str) -> String {
    format!("chore/release-manifest-{version}")
}

/// 清单 PR 的标题（= `pr.mjs` 要求的「分支最早那条提交的主题」）。
pub fn pr_title(version: &str) -> String {
    format!("发布信息：{TAG_PREFIX}{version}")
}

/// 给人看的一句话：清单里现在有几份、都是哪些平台。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestSummary {
    pub version: String,
    pub assets: usize,
    pub platforms: Vec<String>,
}

pub fn summarize(info: &ReleaseInfo) -> ManifestSummary {
    let mut platforms: Vec<String> = Vec::new();
    for a in info.assets.as_deref().unwrap_or(&[]) {
        if let Some(p) = &a.platform {
            if !platforms.contains(p) {
                platforms.push(p.clone());
            }
        }
    }
    ManifestSummary {
        version: info.version.clone(),
        assets: info.assets.as_deref().unwrap_or(&[]).len(),
        platforms,
    }
}

/// 产品名（清单里不写，但错误文案要用）。
pub const PRODUCT_NAME: &str = PRODUCT;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::release_info::UpdateChannel;

    fn facts(version: &str) -> Vec<ArtifactFact> {
        super::super::platforms::planned_artifacts(version)
            .into_iter()
            .enumerate()
            .map(|(i, (p, k, name))| ArtifactFact {
                name,
                platform: p.id,
                kind: k.kind,
                size: 100 + i as u64,
                sha256: format!("{i:064x}"),
                path: None,
            })
            .collect()
    }

    #[test]
    fn the_manifest_lists_every_platform_and_both_sources() {
        let text = render(
            "0.0.16",
            UpdateChannel::Stable,
            "一句话",
            "https://gitee.com/o/r/releases/tag/v0.0.16",
            &facts("0.0.16"),
            None,
            "o",
            "r",
            Some(&("go".to_owned(), "gr".to_owned())),
        )
        .unwrap();
        let info = crate::runtime::release_info::parse(text.as_bytes()).unwrap();
        let assets = info.assets.unwrap();
        assert_eq!(assets.len(), 4);
        assert!(assets
            .iter()
            .any(|a| a.platform.as_deref() == Some("macos")));
        assert!(assets
            .iter()
            .any(|a| a.platform.as_deref() == Some("windows")));
        assert!(assets
            .iter()
            .all(|a| a.sources.as_ref().unwrap().contains_key("github")));
        assert_eq!(
            info.asset.as_ref().map(|a| a.name.clone()),
            Some("SupportEase_0.0.16_aarch64.app.zip".to_owned()),
            "老客户端的单资产入口应是 macOS 的应用内更新包"
        );
    }

    #[test]
    fn a_missing_platform_refuses_to_write_a_manifest() {
        let mut f = facts("0.0.16");
        f.retain(|a| a.platform == "macos");
        let err = render(
            "0.0.16",
            UpdateChannel::Stable,
            "",
            "",
            &f,
            None,
            "o",
            "r",
            None,
        )
        .unwrap_err();
        assert!(err.message.contains("缺"));
    }

    #[test]
    fn a_rerun_keeps_the_published_notes() {
        let base = render(
            "0.0.16",
            UpdateChannel::Stable,
            "原来的说明",
            "https://gitee.com/o/r/releases/tag/v0.0.16",
            &facts("0.0.16"),
            None,
            "o",
            "r",
            None,
        )
        .unwrap();
        let prev = crate::runtime::release_info::parse(base.as_bytes()).unwrap();
        let again = render(
            "0.0.16",
            UpdateChannel::Stable,
            "这一趟填的别的说明",
            "https://gitee.com/other/other/releases/tag/v0.0.16",
            &facts("0.0.16"),
            Some(&prev),
            "o",
            "r",
            None,
        )
        .unwrap();
        let info = crate::runtime::release_info::parse(again.as_bytes()).unwrap();
        assert_eq!(info.notes, "原来的说明");
        assert_eq!(info.url, "https://gitee.com/o/r/releases/tag/v0.0.16");
    }

    #[test]
    fn github_main_urls_are_refused() {
        let text = serde_json::json!({
            "releaseSchema": 1,
            "version": "0.0.16",
            "url": "https://github.com/o/r/releases/tag/v0.0.16",
            "assets": []
        })
        .to_string();
        let err = assert_epoch_legal(&text).unwrap_err();
        assert!(err.message.contains("断代线"));
    }

    #[test]
    fn the_manifest_branch_is_not_the_historical_one() {
        assert_ne!(branch_name("0.0.16"), "chore/release-v0.0.16");
        assert_eq!(pr_title("0.0.16"), "发布信息：v0.0.16");
    }
}
