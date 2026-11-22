//! **CI 构建通道** —— Windows 包只能由 Windows 编；但**发布权仍在内核**。
//!
//! ```text
//! 内核 workflow_dispatch(ref=main, inputs.source_sha=<冻结 SHA>)
//!   → 轮询运行
//!   → 拉 Actions artifact（zip）
//!   → 解包、按计划产物名挑文件、**自己算 SHA-256**
//!   → 交回调用方（上传、清单、封版都在内核里）
//! ```
//!
//! # 回执只作交叉核对
//!
//! 工作流会写 `build-receipt.json`（版本 / 源码 SHA / 每个文件的 size+sha256）。
//! 但内核**以自己算的为准**：回执是给"构建真的成功了吗"背书，不是身份来源。

use std::path::{Path, PathBuf};

use crate::error::AppError;

use super::platforms::{ArtifactFact, Platform};

/// 这条工作流文件在仓库里的路径（`workflow_dispatch` 用它点名）。
pub const WINDOWS_WORKFLOW: &str = "windows-build.yml";

/// 触发一次 Windows 构建，返回这次运行。
pub fn dispatch(
    actions: &crate::workbench::app::platform::actions::Actions,
    owner: &str,
    repo: &str,
    source_commit: &str,
    version: &str,
    summary: &mut Vec<String>,
) -> Result<u64, AppError> {
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
    /* 触发接口不回 run id；等下一轮轮询按 head_sha 认它 */
    Ok(0)
}

/// 同一冻结 SHA 上**已经存在**的运行 —— 正在跑就等它、已经成功就复用它。
///
/// 这一条就是「允许从已有 CI 产物恢复、不重复构建」：换机器、上一趟被打断之后
/// 再跑同一条命令，不该再占一台 Windows runner。
pub fn reusable_run(
    actions: &crate::workbench::app::platform::actions::Actions,
    owner: &str,
    repo: &str,
    source_commit: &str,
) -> Result<Option<crate::workbench::app::platform::actions::WorkflowRun>, AppError> {
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

/// 等这一笔源码的那次运行跑完。`created_after` 是触发时刻的 ISO8601（防止认到上一趟）。
#[allow(clippy::too_many_arguments)] // 等一次运行要的上下文（通道/仓/源码/起点/点名 run/时限/摘要）都是不可省的
pub fn wait(
    actions: &crate::workbench::app::platform::actions::Actions,
    owner: &str,
    repo: &str,
    source_commit: &str,
    created_after: &str,
    prefer_run: Option<u64>,
    deadline: std::time::Instant,
    summary: &mut Vec<String>,
) -> Result<crate::workbench::app::platform::actions::WorkflowRun, AppError> {
    let mut last_note = String::new();
    loop {
        let runs = actions.runs(owner, repo, WINDOWS_WORKFLOW)?;
        let mine = runs.iter().find(|r| match prefer_run {
            Some(id) => r.id == id,
            None => {
                r.head_sha == source_commit
                    && (created_after.is_empty() || r.created_at.as_str() >= created_after)
            }
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
        } else {
            if last_note != "等 Windows 构建排队" {
                summary.push("等 Windows 构建排队".to_owned());
                last_note = "等 Windows 构建排队".to_owned();
            }
        }
        if std::time::Instant::now() > deadline {
            return Err(AppError::io("等 Windows 构建超时").with_detail(
                "工作流没在时限内跑完 —— 去 Actions 页面看一眼；产物在就能接着跑".to_owned(),
            ));
        }
        std::thread::sleep(std::time::Duration::from_secs(15));
    }
}

/// 取回产物：下载 artifact → 解包 → 按计划名挑文件 → 自算身份。
#[allow(clippy::too_many_arguments)] // 一次取产物要的上下文（通道/仓/run/版本/平台/落点/摘要）都是不可省的
pub fn collect(
    actions: &crate::workbench::app::platform::actions::Actions,
    owner: &str,
    repo: &str,
    run_id: u64,
    version: &str,
    platform: &'static Platform,
    dest_root: &Path,
    summary: &mut Vec<String>,
) -> Result<Vec<ArtifactFact>, AppError> {
    let artifacts = actions.artifacts(owner, repo, run_id)?;
    let pick = artifacts
        .iter()
        .find(|a| !a.expired && a.name.starts_with("SupportEase-windows"))
        .or_else(|| artifacts.iter().find(|a| !a.expired))
        .ok_or_else(|| AppError::not_found(format!("run {run_id} 上没有可用的 artifact")))?;
    let dir = dest_root.join(format!("run-{run_id}"));
    std::fs::create_dir_all(&dir)
        .map_err(|e| AppError::io("建不了 CI 产物目录").with_detail(e.to_string()))?;
    let zip = dir.join("artifact.zip");
    let bytes = actions.download_artifact(owner, repo, pick.id, &zip)?;
    summary.push(format!("已下载 artifact {}（{bytes} 字节）", pick.name));

    let unpacked = dir.join("unpacked");
    unzip(&zip, &unpacked)?;
    let receipt = read_receipt(&unpacked);

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
    fn cross_check(&self, fact: &ArtifactFact) -> Result<(), AppError> {
        if !self.version.is_empty()
            && self.version.trim() != fact.name.split('_').nth(1).unwrap_or("")
        {
            return Err(AppError::invalid_argument(format!(
                "CI 回执写的版本是 {}，产物 {} 不是这一版",
                self.version, fact.name
            )));
        }
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
