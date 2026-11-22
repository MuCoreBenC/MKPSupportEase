//! **CI 构建通道** —— Windows 包只能由 Windows 编；但**发布权仍在内核**。
//!
//! ```text
//! 内核 workflow_dispatch(ref=main, inputs.source_sha=<冻结 SHA>)
//!   → 轮询这次运行（按"触发之后新建的那一次"认，不按 head_sha 认）
//!   → 拉 Actions artifact（zip）
//!   → 解包、按计划产物名挑文件、**自己算 SHA-256**
//!   → 交回调用方（上传、清单、封版都在内核里）
//! ```
//!
//! # 为什么按"触发之后"认运行，而不是按 `head_sha`
//!
//! `workflow_dispatch` 的 `ref` 必须是分支/标签（不能是提交），我们给的是 `main`；
//! 于是 `workflow_run.head_sha` 是**main 那一刻的 tip**，不是 `inputs.source_sha`。
//! 一旦 main 在这期间前进，按 `head_sha == source_commit` 就会永远匹配不到 ——
//! 构建其实成功了，内核却空等到超时（2026-11-22 审查发现，真机上恰好 main 没动才没炸）。
//!
//! 现在的口径：触发后只看"`created_at >= 触发时刻` 且工作流对得上"的运行；
//! 被构建的到底是哪一笔，由**产物回执**（`build-receipt.json` 的 `source_sha`）与
//! 内核自己算的 SHA 一起证明。
//!
//! # 回执只作交叉核对
//!
//! 工作流会写 `build-receipt.json`（版本 / 源码 SHA / 每个文件的 size+sha256）。
//! 但内核**以自己算的为准**：回执是给"构建真的成功了吗、是不是这一笔"背书。

use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::error::AppError;

use super::platforms::{ArtifactFact, Platform};
use crate::workbench::app::platform::actions::{Actions, WorkflowRun};

/// 这条工作流文件在仓库里的路径（`workflow_dispatch` 用它点名）。
pub const WINDOWS_WORKFLOW: &str = "windows-build.yml";

/// Windows 产物的 Artifact 名前缀（工作流里写死的那个）。
pub const WINDOWS_ARTIFACT_PREFIX: &str = "SupportEase-windows";

/// **构建通道**：真实现是 [`ActionsCi`]；演练时换成假实现。
///
/// 抽它只为一件事：Windows 那条路在没有网络、没有 runner 的情况下也要能跑完整编排
/// （中断、换机器、重复执行）。
pub trait Ci {
    /// 同一冻结 SHA 上已有的运行（正在跑 / 已成功），能复用就不重新触发。
    fn reusable(
        &self,
        owner: &str,
        repo: &str,
        source: &str,
    ) -> Result<Option<WorkflowRun>, AppError>;

    /// 触发一次构建。
    fn start(
        &self,
        owner: &str,
        repo: &str,
        source: &str,
        version: &str,
        summary: &mut Vec<String>,
    ) -> Result<(), AppError>;

    /// 等这次运行跑完。
    fn wait(
        &self,
        owner: &str,
        repo: &str,
        created_after: &str,
        prefer: Option<u64>,
        deadline: Instant,
        summary: &mut Vec<String>,
    ) -> Result<WorkflowRun, AppError>;

    /// 取回产物并算出身份。
    #[allow(clippy::too_many_arguments)] // 通道/仓/运行/源码/版本/平台/落点/摘要 都是不可省的
    fn collect(
        &self,
        owner: &str,
        repo: &str,
        run_id: u64,
        source: &str,
        version: &str,
        platform: &'static Platform,
        dest_root: &Path,
        summary: &mut Vec<String>,
    ) -> Result<Vec<ArtifactFact>, AppError>;
}

/// 真实现：GitHub Actions。
pub struct ActionsCi {
    actions: Actions,
}

impl ActionsCi {
    pub fn new(token: String) -> Self {
        Self {
            actions: Actions::new(token),
        }
    }
}

impl Ci for ActionsCi {
    fn reusable(
        &self,
        owner: &str,
        repo: &str,
        source: &str,
    ) -> Result<Option<WorkflowRun>, AppError> {
        reusable_run(&self.actions, owner, repo, source)
    }

    fn start(
        &self,
        owner: &str,
        repo: &str,
        source: &str,
        version: &str,
        summary: &mut Vec<String>,
    ) -> Result<(), AppError> {
        dispatch(&self.actions, owner, repo, source, version, summary)
    }

    fn wait(
        &self,
        owner: &str,
        repo: &str,
        created_after: &str,
        prefer: Option<u64>,
        deadline: Instant,
        summary: &mut Vec<String>,
    ) -> Result<WorkflowRun, AppError> {
        wait(
            &self.actions,
            owner,
            repo,
            created_after,
            prefer,
            deadline,
            summary,
        )
    }

    fn collect(
        &self,
        owner: &str,
        repo: &str,
        run_id: u64,
        source: &str,
        version: &str,
        platform: &'static Platform,
        dest_root: &Path,
        summary: &mut Vec<String>,
    ) -> Result<Vec<ArtifactFact>, AppError> {
        collect(
            &self.actions,
            owner,
            repo,
            run_id,
            source,
            version,
            platform,
            dest_root,
            summary,
        )
    }
}

/// 触发一次 Windows 构建（触发接口不回 run id，等下一轮轮询按时间认它）。
pub fn dispatch(
    actions: &Actions,
    owner: &str,
    repo: &str,
    source_commit: &str,
    version: &str,
    summary: &mut Vec<String>,
) -> Result<(), AppError> {
    actions.dispatch(
        owner,
        repo,
        WINDOWS_WORKFLOW,
        /* 工作流文件必须取自一条存在的分支；**被构建的代码**由 inputs 决定 */
        "main",
        &[
            ("source_sha", source_commit.to_owned()),
            ("version", version.to_owned()),
        ],
    )?;
    summary.push(format!(
        "已触发 Windows 构建（{WINDOWS_WORKFLOW} @ {}）",
        short(source_commit)
    ));
    Ok(())
}

/// 同一冻结 SHA 上**已经存在**的运行 —— 正在跑就等它、已经成功就复用它。
///
/// 这一条就是「允许从已有 CI 产物恢复、不重复构建」：换机器、上一趟被打断之后
/// 再跑同一条命令，不该再占一台 Windows runner。
///
/// ★ 认的是 `head_sha`（= 触发时的 main tip）：**同一笔冻结源码**上只会有一串运行，
/// 而"被构建的代码是不是这一笔"由产物回执与内核自算的 SHA 兜底（见 [`collect`]）。
pub fn reusable_run(
    actions: &Actions,
    owner: &str,
    repo: &str,
    source_commit: &str,
) -> Result<Option<WorkflowRun>, AppError> {
    let runs = actions.runs(owner, repo, WINDOWS_WORKFLOW)?;
    let mine: Vec<_> = runs
        .into_iter()
        .filter(|r| r.head_sha == source_commit)
        .collect();
    if let Some(r) = mine.iter().find(|r| !r.finished()) {
        return Ok(Some(r.clone()));
    }
    Ok(mine.into_iter().find(|r| r.ok()))
}

/// 等这次运行跑完。
///
/// `prefer` = 点名某一次（复用那趟）；否则取**触发之后新建的**那一次。
#[allow(clippy::too_many_arguments)] // 通道/仓/起点/点名 run/时限/摘要 都是不可省的
pub fn wait(
    actions: &Actions,
    owner: &str,
    repo: &str,
    created_after: &str,
    prefer_run: Option<u64>,
    deadline: Instant,
    summary: &mut Vec<String>,
) -> Result<WorkflowRun, AppError> {
    let mut last_note = String::new();
    loop {
        let runs = actions.runs(owner, repo, WINDOWS_WORKFLOW)?;
        let mine = runs.iter().find(|r| match prefer_run {
            Some(id) => r.id == id,
            None => created_after.is_empty() || r.created_at.as_str() >= created_after,
        });
        if let Some(run) = mine {
            if run.finished() {
                if run.ok() {
                    summary.push(format!("Windows 构建完成（run {}）", run.id));
                    return Ok(run.clone());
                }
                return Err(AppError::io(format!(
                    "Windows 构建失败：{}（{}）",
                    run.conclusion.clone().unwrap_or_else(|| "未知".to_owned()),
                    run.url
                )));
            }
            let note = format!("Windows 构建中：{}（run {}）", run.status, run.id);
            if note != last_note {
                summary.push(note.clone());
                last_note = note;
            }
        } else if last_note != "等 Windows 构建排队" {
            summary.push("等 Windows 构建排队".to_owned());
            last_note = "等 Windows 构建排队".to_owned();
        }
        if Instant::now() > deadline {
            return Err(AppError::io("等 Windows 构建超时").with_detail(
                "工作流没在时限内跑完 —— 去 Actions 页面看一眼；产物在就能接着跑".to_owned(),
            ));
        }
        std::thread::sleep(std::time::Duration::from_secs(15));
    }
}

/// 取回产物：下载 artifact → 解包 → 按计划名挑文件 → 自算身份。
#[allow(clippy::too_many_arguments)] // 一次取产物要的上下文（通道/仓/run/源码/版本/平台/落点/摘要）都是不可省的
pub fn collect(
    actions: &Actions,
    owner: &str,
    repo: &str,
    run_id: u64,
    source_commit: &str,
    version: &str,
    platform: &'static Platform,
    dest_root: &Path,
    summary: &mut Vec<String>,
) -> Result<Vec<ArtifactFact>, AppError> {
    let artifacts = actions.artifacts(owner, repo, run_id)?;
    let candidates: Vec<String> = artifacts
        .iter()
        .filter(|a| !a.expired)
        .map(|a| a.name.clone())
        .collect();
    /* ★ 只认这一趟工作流自己声明的前缀：挑"第一个未过期的"会拿到回执之类的附件，
    报出来的错却是"artifact 里没有 X"，把排查带到错的方向。 */
    let pick = artifacts
        .iter()
        .find(|a| !a.expired && a.name.starts_with(WINDOWS_ARTIFACT_PREFIX))
        .ok_or_else(|| {
            AppError::not_found(format!(
                "run {run_id} 上没有 {WINDOWS_ARTIFACT_PREFIX}* 的 artifact"
            ))
            .with_detail(format!("这一趟的附件：{}", candidates.join("、")))
        })?;
    let dir = dest_root.join(format!("run-{run_id}"));
    std::fs::create_dir_all(&dir)
        .map_err(|e| AppError::io("建不了 CI 产物目录").with_detail(e.to_string()))?;
    let zip = dir.join("artifact.zip");
    let bytes = actions.download_artifact(owner, repo, pick.id, &zip)?;
    summary.push(format!("已下载 artifact {}（{bytes} 字节）", pick.name));

    let unpacked = dir.join("unpacked");
    unzip(&zip, &unpacked)?;
    let receipt = read_receipt(&unpacked);
    if let Some(r) = &receipt {
        r.check_identity(source_commit, version)?;
    }

    let mut facts = Vec::new();
    for kind in platform.artifacts {
        let want = super::platforms::artifact_name(version, kind);
        let found = find_in(&unpacked, &want).ok_or_else(|| {
            AppError::not_found(format!(
                "artifact 里没有 {want} —— 工作流产出的文件名与计划不一致"
            ))
            .with_detail(format!("解包目录：{}", unpacked.display()))
        })?;
        let fact = ArtifactFact::from_path(version, platform, kind, &found)?;
        if let Some(r) = &receipt {
            r.cross_check(&fact)?;
        }
        summary.push(format!("CI 产物 {}（{} 字节）", fact.name, fact.size));
        facts.push(fact);
    }
    Ok(facts)
}

/// CI 写下的构建回执（缺了不影响判据 —— 身份以内核自算为准）。
#[derive(Debug, Clone, serde::Deserialize)]
struct Receipt {
    #[serde(default)]
    version: String,
    #[serde(default)]
    source_sha: String,
    #[serde(default)]
    files: Vec<ReceiptFile>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ReceiptFile {
    name: String,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    sha256: String,
}

impl Receipt {
    /// 回执说的"这一版、这一笔"必须与内核要发的一致。
    fn check_identity(&self, source_commit: &str, version: &str) -> Result<(), AppError> {
        if !self.version.is_empty() && self.version.trim() != version.trim() {
            return Err(AppError::invalid_argument(format!(
                "CI 回执写的版本是 {}，这一趟要发 {version}",
                self.version
            )));
        }
        if !self.source_sha.is_empty() && !source_commit.is_empty() {
            let want = self.source_sha.trim();
            if !source_commit.starts_with(want) && !want.starts_with(source_commit) {
                return Err(AppError::invalid_argument(format!(
                    "CI 回执构建的是 {}，冻结源码是 {} —— 复用的运行不是这一笔",
                    &want[..want.len().min(7)],
                    &source_commit[..source_commit.len().min(7)]
                )));
            }
        }
        Ok(())
    }

    fn cross_check(&self, fact: &ArtifactFact) -> Result<(), AppError> {
        let Some(f) = self.files.iter().find(|f| f.name == fact.name) else {
            return Ok(());
        };
        if f.size != 0 && f.size != fact.size {
            return Err(AppError::invalid_argument(format!(
                "{} 的回执写 {} 字节，本地 {} 字节",
                fact.name, f.size, fact.size
            )));
        }
        if !f.sha256.is_empty() && !f.sha256.eq_ignore_ascii_case(&fact.sha256) {
            return Err(AppError::invalid_argument(format!(
                "{} 的回执 SHA 与本地算出来的不同 —— 传输途中被改过",
                fact.name
            )));
        }
        Ok(())
    }
}

fn read_receipt(dir: &Path) -> Option<Receipt> {
    let path = find_in(dir, "build-receipt.json")?;
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// 在解包目录里按文件名递归找（`upload-artifact` 会套一层目录）。
pub fn find_in(dir: &Path, name: &str) -> Option<PathBuf> {
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let entries = std::fs::read_dir(&d).ok()?;
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.file_name().and_then(|n| n.to_str()) == Some(name) {
                return Some(p);
            }
        }
    }
    None
}

fn unzip(zip_path: &Path, dest: &Path) -> Result<(), AppError> {
    std::fs::create_dir_all(dest)
        .map_err(|e| AppError::io("建不了解包目录").with_detail(e.to_string()))?;
    let file = std::fs::File::open(zip_path)
        .map_err(|e| AppError::io("打不开 artifact zip").with_detail(e.to_string()))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| AppError::corrupted("artifact 不是合法 zip").with_detail(e.to_string()))?;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| AppError::corrupted("读不了 zip 条目").with_detail(e.to_string()))?;
        /* 只取普通文件，且不许 `..` 跳出目录（zip slip） */
        let Some(rel) = entry.enclosed_name().map(|p| p.to_path_buf()) else {
            continue;
        };
        let out = dest.join(rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&out)
                .map_err(|e| AppError::io("建解包子目录失败").with_detail(e.to_string()))?;
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AppError::io("建解包子目录失败").with_detail(e.to_string()))?;
        }
        crate::fsx::atomic::atomic_write_stream(&out, |f| {
            std::io::copy(&mut entry, f)
                .map_err(|e| AppError::io("解包失败").with_detail(e.to_string()))?;
            Ok(())
        })?;
    }
    Ok(())
}

fn short(sha: &str) -> String {
    sha.chars().take(7).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt(json: &str) -> Receipt {
        serde_json::from_str(json).expect("回执应能解析")
    }

    #[test]
    fn a_receipt_from_another_commit_is_refused() {
        let r = receipt(
            r#"{"version":"0.0.16","source_sha":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","files":[]}"#,
        );
        assert!(r
            .check_identity("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", "0.0.16")
            .is_err());
        assert!(r
            .check_identity("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "0.0.16")
            .is_ok());
    }

    #[test]
    fn a_receipt_from_another_version_is_refused() {
        let r = receipt(r#"{"version":"0.0.15","files":[]}"#);
        assert!(r.check_identity("", "0.0.16").is_err());
    }

    #[test]
    fn the_artifact_prefix_is_the_one_the_workflow_writes() {
        let yml = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../.github/workflows/windows-build.yml"),
        )
        .expect("读不到工作流");
        assert!(
            yml.contains(&format!(
                "name: {WINDOWS_ARTIFACT_PREFIX}-${{{{ env.source_sha }}}}"
            )),
            "工作流的 artifact 名与内核认的前缀不一致（{WINDOWS_ARTIFACT_PREFIX}）"
        );
    }
}
