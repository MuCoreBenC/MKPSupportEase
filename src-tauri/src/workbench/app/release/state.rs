//! **发布状态机** —— 只做一件事：把四份远端事实折算成「这一代发布到哪一步、每个平台
//! 每个源到哪一步、有没有冲突」。
//!
//! # 为什么状态是**派生**的
//!
//! 本地账本会漂：换一台机器、上一趟被打断、工作区被清掉，账本就对不上了。而远端那几样
//! 事实不会漂 —— Gitee 上挂着哪些附件、GitHub 上挂着哪些附件、正式 tag 有没有、main 上的
//! 清单写了什么。所以判据一律现读，本地 journal 只记过程，**不参与判断**。
//!
//! # 状态粒度是「版本 × 平台 × 下载源」
//!
//! 旧内核只有一个 `Taken` 布尔（「这一版发过没有」），于是「补另一个平台」判不出来。
//! 这里每份产物记四个格：Gitee 暂存 / GitHub 暂存 / Gitee 正式 / GitHub 正式。
//!
//! # 派生是纯函数
//!
//! 网络读进 [`Snapshot`] 之后，[`derive`] 不发一个请求、不读一个文件 —— 判据因此可以
//! 用假远端逐条钉住（见文件末尾的测试）。

use serde::Serialize;

use crate::runtime::release_info::{ReleaseInfo, UpdateChannel};
use crate::workbench::app::platform::RemoteAsset;

use super::platforms::{planned_artifacts, Platform, PLATFORMS};

/// 一次发布走到了哪一步。**顺序就是推进方向**，跳步不允许。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    /// 还没冻结：没有暂存 tag，也没有任何暂存资产
    Preparing,
    /// 源码已冻结（暂存 tag 指着那一笔），还没有任何资产
    Frozen,
    /// 平台构建 / 暂存进行中（有的平台齐了、有的没齐）
    Building,
    /// 所有平台的资产在两个源上都齐了（还在暂存面上）
    Staged,
    /// main 上的清单已经包含这一代的全部平台资产
    ManifestMerged,
    /// 封版前的六条判据全过（只差打 tag）
    Verified,
    /// 正式 tag 已创建、正式 Release 与清单逐份一致
    Sealed,
}

impl Phase {
    pub fn wire(self) -> &'static str {
        match self {
            Phase::Preparing => "preparing",
            Phase::Frozen => "frozen",
            Phase::Building => "building",
            Phase::Staged => "staged",
            Phase::ManifestMerged => "manifestMerged",
            Phase::Verified => "verified",
            Phase::Sealed => "sealed",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Phase::Preparing => "准备中",
            Phase::Frozen => "源码已冻结",
            Phase::Building => "平台构建中",
            Phase::Staged => "平台资产已暂存",
            Phase::ManifestMerged => "清单已合并",
            Phase::Verified => "核验通过",
            Phase::Sealed => "已封版",
        }
    }
}

/// 单个平台走到哪：
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PlatformStage {
    /// 一份都没有
    Missing,
    /// 有一部分（构建了一半 / 只传上去一份）
    Partial,
    /// 两个源都齐了，但还在暂存面上
    Staged,
    /// 两个源的正式 Release 上都有了
    Published,
}

impl PlatformStage {
    pub fn wire(self) -> &'static str {
        match self {
            PlatformStage::Missing => "missing",
            PlatformStage::Partial => "partial",
            PlatformStage::Staged => "staged",
            PlatformStage::Published => "published",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            PlatformStage::Missing => "待构建",
            PlatformStage::Partial => "部分完成",
            PlatformStage::Staged => "已暂存",
            PlatformStage::Published => "已发布",
        }
    }
}

/// 一份产物在四个格上的样子。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactState {
    pub name: String,
    pub kind: &'static str,
    pub gitee_staged: Option<RemoteAsset>,
    pub github_staged: Option<RemoteAsset>,
    pub gitee_published: Option<RemoteAsset>,
    pub github_published: Option<RemoteAsset>,
}

impl ArtifactState {
    /// 两个源都暂存齐了（封版前的要求）。
    pub fn staged(&self) -> bool {
        self.gitee_staged.is_some() && self.github_staged.is_some()
    }
    /// 两个源都正式发布了。
    pub fn published(&self) -> bool {
        self.gitee_published.is_some() && self.github_published.is_some()
    }
    /// 有没有任何一格有东西（判断"这一份开始动手了没有"）。
    pub fn touched(&self) -> bool {
        self.gitee_staged.is_some()
            || self.github_staged.is_some()
            || self.gitee_published.is_some()
            || self.github_published.is_some()
    }
}

/// 一个平台的汇总（UI 的「平台 × 源」那一行）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformState {
    pub id: &'static str,
    pub label: &'static str,
    pub built_by: &'static str,
    pub stage: PlatformStage,
    /// Gitee 上这一平台的包齐了没有
    pub gitee_done: bool,
    /// GitHub 上这一平台的包齐了没有
    pub github_done: bool,
    pub artifacts: Vec<ArtifactState>,
}

/// main 上那份清单的状态。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestState {
    /// main 上有清单文件吗
    pub present: bool,
    /// main 上那份是哪一版
    pub version: Option<String>,
    /// 它是不是就是这一代
    pub same_version: bool,
    /// 这一代的产物里，哪些名字它没有写
    pub missing: Vec<String>,
    /// 它是不是已经包含这一代的全部产物
    pub complete: bool,
}

/// 派生状态所需的**全部事实**（由 [`super::inspect`] 现读组装）。
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub staging_tag_commit: Option<String>,
    pub formal_tag_commit: Option<String>,
    pub gitee_staging: Vec<RemoteAsset>,
    pub gitee_formal: Vec<RemoteAsset>,
    pub github_staging: Vec<RemoteAsset>,
    pub github_formal: Vec<RemoteAsset>,
    /// GitHub 暂存那份还是草稿（正常应为 `true`）
    pub github_staging_draft: bool,
    /// `origin/main` 上的清单
    pub main: Option<ReleaseInfo>,
}

/// 派生结论。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseState {
    pub version: String,
    pub channel: UpdateChannel,
    pub tag: String,
    pub staging_tag: String,
    /// 冻结源码是哪一笔（暂存 tag 指向它）
    pub source_commit: Option<String>,
    pub phase: Phase,
    pub platforms: Vec<PlatformState>,
    pub manifest: ManifestState,
    /// 冲突（有就停下，不自作主张）
    pub conflicts: Vec<String>,
    /// 封版六条判据没过的地方（空 = 可以封版）
    pub seal_blockers: Vec<String>,
    /// 下一步该做什么（一句话，给人看）
    pub next: String,
}

/// 找一份同名资产。
fn find<'a>(list: &'a [RemoteAsset], name: &str) -> Option<&'a RemoteAsset> {
    list.iter().find(|a| a.name == name)
}

/// 同一个名字在暂存与正式之间**大小 / 摘要**对不上 = 两份不同的字节。
fn mismatched(staged: &RemoteAsset, published: &RemoteAsset) -> Option<String> {
    match (staged.size, published.size) {
        (Some(a), Some(b)) if a != b => {
            return Some(format!("{}：暂存 {} 字节、正式 {} 字节", staged.name, a, b))
        }
        _ => {}
    }
    match (&staged.sha256, &published.sha256) {
        (Some(a), Some(b)) if a != b => {
            return Some(format!("{}：暂存与正式的 SHA-256 不同", staged.name))
        }
        _ => {}
    }
    None
}

/// **纯函数**：事实 → 状态。
pub fn derive(version: &str, channel: UpdateChannel, snap: &Snapshot) -> ReleaseState {
    let planned = planned_artifacts(version);
    let mut platforms: Vec<PlatformState> = Vec::new();

    for p in PLATFORMS {
        let mut states: Vec<ArtifactState> = Vec::new();
        for (owner, _kind, name) in planned.iter().filter(|(owner, _, _)| owner.id == p.id) {
            let _ = owner;
            states.push(ArtifactState {
                name: name.clone(),
                kind: _kind.kind,
                gitee_staged: find(&snap.gitee_staging, name).cloned(),
                github_staged: find(&snap.github_staging, name).cloned(),
                gitee_published: find(&snap.gitee_formal, name).cloned(),
                github_published: find(&snap.github_formal, name).cloned(),
            });
        }
        let all = !states.is_empty();
        let published = all && states.iter().all(|a| a.published());
        let staged = all && states.iter().all(|a| a.staged());
        let touched = states.iter().any(|a| a.touched());
        let stage = if published {
            PlatformStage::Published
        } else if staged {
            PlatformStage::Staged
        } else if touched {
            PlatformStage::Partial
        } else {
            PlatformStage::Missing
        };
        let gitee_done = states.iter().all(|a| {
            if published {
                a.gitee_published.is_some()
            } else {
                a.gitee_staged.is_some()
            }
        });
        let github_done = states.iter().all(|a| {
            if published {
                a.github_published.is_some()
            } else {
                a.github_staged.is_some()
            }
        });
        platforms.push(PlatformState {
            id: p.id,
            label: p.label,
            built_by: p.built_by,
            stage,
            gitee_done,
            github_done,
            artifacts: states,
        });
    }

    /* ---------- 清单状态 ---------- */
    let names: Vec<&str> = planned.iter().map(|(_, _, n)| n.as_str()).collect();
    let manifest = match &snap.main {
        None => ManifestState {
            present: false,
            version: None,
            same_version: false,
            missing: names.iter().map(|n| (*n).to_owned()).collect(),
            complete: false,
        },
        Some(info) => {
            let listed: Vec<&str> = info
                .assets
                .as_deref()
                .unwrap_or(&[])
                .iter()
                .map(|a| a.name.as_str())
                .collect();
            let missing: Vec<String> = names
                .iter()
                .filter(|n| !listed.contains(*n))
                .map(|n| (*n).to_owned())
                .collect();
            let same = info.version.trim() == version.trim();
            ManifestState {
                present: true,
                version: Some(info.version.clone()),
                same_version: same,
                complete: same && missing.is_empty(),
                missing,
            }
        }
    };

    /* ---------- 冲突 ---------- */
    let mut conflicts: Vec<String> = Vec::new();
    if let (Some(staging), Some(formal)) = (&snap.staging_tag_commit, &snap.formal_tag_commit) {
        if staging != formal {
            conflicts.push(format!(
                "暂存 tag 指向 {}，正式 tag 指向 {} —— 冻结源码对不上",
                short(staging),
                short(formal)
            ));
        }
    }
    for p in &platforms {
        for a in &p.artifacts {
            if let (Some(s), Some(f)) = (&a.gitee_staged, &a.gitee_published) {
                if let Some(m) = mismatched(s, f) {
                    conflicts.push(m);
                }
            }
            if let (Some(s), Some(f)) = (&a.github_staged, &a.github_published) {
                if let Some(m) = mismatched(s, f) {
                    conflicts.push(m);
                }
            }
        }
    }
    if manifest.same_version && !manifest.complete {
        /* 清单已经写成这一代，却缺平台资产：不许悄悄补，说明有人手工写过 / 上一趟只发了一半 */
        conflicts.push(format!(
            "main 上的清单写着 {}，但缺 {} 份资产",
            version,
            manifest.missing.len()
        ));
    }
    if snap.gitee_staging.is_empty() && !snap.gitee_formal.is_empty() && !manifest.same_version {
        conflicts.push("Gitee 正式 Release 已有资产，main 上的清单却不是这一代".to_owned());
    }

    /* ---------- 封版六条 ---------- */
    let mut blockers: Vec<String> = Vec::new();
    for p in &platforms {
        let done = p.stage == PlatformStage::Staged || p.stage == PlatformStage::Published;
        if !done {
            blockers.push(format!("{} 还没齐（{}）", p.label, p.stage.label()));
        }
        if !p.gitee_done {
            blockers.push(format!("{} 在 Gitee 上不全", p.label));
        }
        if !p.github_done {
            blockers.push(format!("{} 在 GitHub 上不全", p.label));
        }
    }
    if !manifest.complete {
        blockers.push("main 上的清单没有包含这一代的全部平台资产".to_owned());
    }
    if snap.source_commit().is_none() {
        blockers.push("没有冻结源码（暂存 tag 不存在）".to_owned());
    }
    if !snap.github_staging.is_empty()
        && !snap.github_staging_draft
        && snap.formal_tag_commit.is_none()
    {
        blockers.push("GitHub 暂存 Release 不是草稿 —— 正式 tag 会提前出现".to_owned());
    }

    /* ---------- 阶段 ---------- */
    let all_staged = platforms
        .iter()
        .all(|p| p.stage == PlatformStage::Staged || p.stage == PlatformStage::Published);
    let all_published = platforms
        .iter()
        .all(|p| p.stage == PlatformStage::Published);
    let any_touched = platforms.iter().any(|p| p.stage != PlatformStage::Missing);
    let phase = if !conflicts.is_empty() {
        /* 冲突时如实停在 Building/Staged 那一档，由 conflicts 说明为什么不能往前走 */
        if all_staged {
            Phase::Staged
        } else if any_touched {
            Phase::Building
        } else {
            Phase::Preparing
        }
    } else if snap.formal_tag_commit.is_some() && all_published && manifest.complete {
        Phase::Sealed
    } else if manifest.complete && all_staged {
        if blockers.is_empty() {
            Phase::Verified
        } else {
            Phase::ManifestMerged
        }
    } else if all_staged {
        Phase::Staged
    } else if snap.staging_tag_commit.is_some() && !any_touched {
        Phase::Frozen
    } else if any_touched {
        Phase::Building
    } else {
        Phase::Preparing
    };

    let next = next_action(phase, &platforms, &conflicts);

    ReleaseState {
        version: version.to_owned(),
        channel,
        tag: super::platforms::tag_of(version),
        staging_tag: super::platforms::staging_tag_of(version),
        /* 封版后暂存 tag 会被清理，来源要退回正式 tag 指的那一笔 */
        source_commit: snap.source_commit().cloned(),
        phase,
        platforms,
        manifest,
        conflicts,
        seal_blockers: blockers,
        next,
    }
}

impl Snapshot {
    pub fn source_commit(&self) -> Option<&String> {
        self.staging_tag_commit
            .as_ref()
            .or(self.formal_tag_commit.as_ref())
    }
}

fn short(sha: &str) -> String {
    sha.chars().take(7).collect()
}

fn next_action(phase: Phase, platforms: &[PlatformState], conflicts: &[String]) -> String {
    if !conflicts.is_empty() {
        return "先解决冲突（见明细），不要继续".to_owned();
    }
    match phase {
        Phase::Preparing => "冻结源码（建暂存 tag），开始这一代发布".to_owned(),
        Phase::Frozen => {
            let todo: Vec<&str> = platforms
                .iter()
                .filter(|p| p.stage == PlatformStage::Missing)
                .map(|p| p.label)
                .collect();
            format!("构建并暂存：{}", todo.join("、"))
        }
        Phase::Building => {
            let todo: Vec<&str> = platforms
                .iter()
                .filter(|p| p.stage != PlatformStage::Staged && p.stage != PlatformStage::Published)
                .map(|p| p.label)
                .collect();
            format!("继续补：{}", todo.join("、"))
        }
        Phase::Staged => "生成清单 PR（合并后客户端才看得到新版）".to_owned(),
        Phase::ManifestMerged => "封版前核验未过 —— 看下面的拦项".to_owned(),
        Phase::Verified => "封版：打正式 tag、发布两个源的正式 Release".to_owned(),
        Phase::Sealed => "这一代已封版（再跑只会核对，不会重复上传）".to_owned(),
    }
}

/// 平台表里某个 id 的那一行（给执行器找「这份包归谁」）。
pub fn platform_of(id: &str) -> Option<&'static Platform> {
    PLATFORMS.iter().find(|p| p.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::app::platform::RemoteAsset;

    fn asset(name: &str, size: u64) -> RemoteAsset {
        RemoteAsset {
            name: name.to_owned(),
            size: Some(size),
            sha256: None,
            id: Some(1),
            url: format!("https://example.invalid/{name}"),
        }
    }

    fn all_staged(version: &str) -> Snapshot {
        let mut snap = Snapshot {
            staging_tag_commit: Some("abc1234".to_owned()),
            github_staging_draft: true,
            ..Default::default()
        };
        for (_, _, name) in planned_artifacts(version) {
            snap.gitee_staging.push(asset(&name, 10));
            snap.github_staging.push(asset(&name, 10));
        }
        snap
    }

    fn manifest(version: &str) -> ReleaseInfo {
        let assets = planned_artifacts(version)
            .into_iter()
            .map(
                |(_, kind, name)| crate::runtime::release_info::ReleaseAsset {
                    name,
                    url: format!("https://gitee.com/o/r/releases/download/v{version}/x"),
                    size: 10,
                    sha256: "0".repeat(64),
                    platform: Some(if kind.kind == "dmg" || kind.kind == "appZip" {
                        "macos".to_owned()
                    } else {
                        "windows".to_owned()
                    }),
                    kind: Some(kind.kind.to_owned()),
                    sources: None,
                },
            )
            .collect();
        ReleaseInfo {
            release_schema: 1,
            version: version.to_owned(),
            notes: String::new(),
            url: String::new(),
            asset: None,
            assets: Some(assets),
        }
    }

    #[test]
    fn nothing_on_the_remote_is_preparing() {
        let s = derive("0.0.16", UpdateChannel::Stable, &Snapshot::default());
        assert_eq!(s.phase, Phase::Preparing);
        assert!(s
            .platforms
            .iter()
            .all(|p| p.stage == PlatformStage::Missing));
    }

    #[test]
    fn a_staging_tag_without_assets_is_frozen() {
        let snap = Snapshot {
            staging_tag_commit: Some("abc1234".to_owned()),
            ..Default::default()
        };
        let s = derive("0.0.16", UpdateChannel::Stable, &snap);
        assert_eq!(s.phase, Phase::Frozen);
    }

    #[test]
    fn both_platforms_on_both_sources_is_staged() {
        let s = derive("0.0.16", UpdateChannel::Stable, &all_staged("0.0.16"));
        assert_eq!(s.phase, Phase::Staged);
        assert!(s.platforms.iter().all(|p| p.stage == PlatformStage::Staged));
        assert!(s.platforms.iter().all(|p| p.gitee_done && p.github_done));
    }

    #[test]
    fn only_one_platform_staged_is_building() {
        let mut snap = all_staged("0.0.16");
        snap.gitee_staging.retain(|a| !a.name.contains("x64"));
        snap.github_staging.retain(|a| !a.name.contains("x64"));
        let s = derive("0.0.16", UpdateChannel::Stable, &snap);
        assert_eq!(s.phase, Phase::Building);
        let win = s.platforms.iter().find(|p| p.id == "windows").unwrap();
        assert_eq!(win.stage, PlatformStage::Missing);
    }

    #[test]
    fn manifest_plus_staged_assets_is_verified() {
        let mut snap = all_staged("0.0.16");
        snap.main = Some(manifest("0.0.16"));
        let s = derive("0.0.16", UpdateChannel::Stable, &snap);
        assert_eq!(s.phase, Phase::Verified);
        assert!(s.seal_blockers.is_empty());
    }

    #[test]
    fn a_manifest_missing_a_platform_is_a_conflict() {
        let mut snap = all_staged("0.0.16");
        let mut info = manifest("0.0.16");
        info.assets
            .as_mut()
            .unwrap()
            .retain(|a| a.name.contains("aarch64"));
        snap.main = Some(info);
        let s = derive("0.0.16", UpdateChannel::Stable, &snap);
        assert!(!s.conflicts.is_empty(), "缺平台资产的清单要判冲突");
        assert_eq!(s.manifest.missing.len(), 2);
    }

    #[test]
    fn a_sealed_release_is_sealed() {
        let mut snap = all_staged("0.0.16");
        snap.main = Some(manifest("0.0.16"));
        snap.formal_tag_commit = Some("abc1234".to_owned());
        for (_, _, name) in planned_artifacts("0.0.16") {
            snap.gitee_formal.push(asset(&name, 10));
            snap.github_formal.push(asset(&name, 10));
        }
        let s = derive("0.0.16", UpdateChannel::Stable, &snap);
        assert_eq!(s.phase, Phase::Sealed);
        assert!(s.seal_blockers.is_empty());
    }

    #[test]
    fn different_bytes_between_staging_and_formal_is_a_conflict() {
        let mut snap = all_staged("0.0.16");
        for (_, _, name) in planned_artifacts("0.0.16") {
            snap.gitee_formal.push(asset(&name, 11));
            snap.github_formal.push(asset(&name, 11));
        }
        let s = derive("0.0.16", UpdateChannel::Stable, &snap);
        assert!(s.conflicts.iter().any(|c| c.contains("字节")));
    }

    #[test]
    fn a_non_draft_github_staging_blocks_the_seal() {
        let mut snap = all_staged("0.0.16");
        snap.github_staging_draft = false;
        snap.main = Some(manifest("0.0.16"));
        let s = derive("0.0.16", UpdateChannel::Stable, &snap);
        assert!(s.seal_blockers.iter().any(|b| b.contains("不是草稿")));
    }

    #[test]
    fn platform_stage_covers_every_planned_artifact() {
        let s = derive("0.0.16", UpdateChannel::Stable, &Snapshot::default());
        let mac = s.platforms.iter().find(|p| p.id == "macos").unwrap();
        assert_eq!(mac.artifacts.len(), 2);
        let win = s.platforms.iter().find(|p| p.id == "windows").unwrap();
        assert_eq!(win.artifacts.len(), 2);
    }
}
