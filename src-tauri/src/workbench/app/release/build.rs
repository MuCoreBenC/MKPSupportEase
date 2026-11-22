//! **构建** —— 只在**冻结源码**那一笔上构建，产物带身份交出去。
//!
//! ```text
//! target/release-worktree/<sha8>/       ← 冻结源码的 detached worktree（人不用碰）
//!   └─ npm run tauri -- build           ← 本机那一档（macOS 出 dmg + .app）
//! ```
//!
//! # 为什么必须用 worktree
//!
//! 构建期间工作区可能被改、分支可能被切。发布出去的包必须对应**某一笔确定的提交**，
//! 不能是"当时工作区长什么样"。这个 worktree 由内核自己建、自己复用（按 sha 命名），
//! 人不参与 —— 于是也不需要再有人记得「从哪个 tag 开 worktree」。
//!
//! # 产物自报版本
//!
//! `APP_VERSION` 是编译期常量：改了号不重编，包里还是旧号。所以构建完**先读包里那个号**
//! （macOS 读 `Info.plist`），对不上就停 —— 宁可这一趟失败，也不发一个名不副实的包。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::workbench::app::release::state::ArtifactState;

use super::platforms::{artifact_name, ArtifactFact, Platform, PRODUCT};

/// 回执里那一格「安装包」：文件名 + 字节数 + 本地路径。
///
/// `path` 在「CI 拿回来的那一份」上可能是空串（临时目录会清），所以它不是判据。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactInfo {
    pub name: String,
    pub size: u64,
    #[serde(default)]
    pub path: String,
}

/// 从状态里的产物摊出一格回执（安装包优先：dmg / nsis）。
pub fn artifact_info(a: &ArtifactState) -> ArtifactInfo {
    let remote = a
        .gitee_published
        .as_ref()
        .or(a.github_published.as_ref())
        .or(a.gitee_staged.as_ref())
        .or(a.github_staged.as_ref());
    ArtifactInfo {
        name: a.name.clone(),
        size: remote.and_then(|r| r.size).unwrap_or_default(),
        path: String::new(),
    }
}

/// 冻结源码的 worktree 落在哪（`target/` 已在 `.gitignore` 里）。
pub fn worktree_dir(repo_root: &Path, source_commit: &str) -> PathBuf {
    let short: String = source_commit.chars().take(8).collect();
    repo_root
        .join("target")
        .join("release-worktree")
        .join(short)
}

/// 确保有这么一棵 **detached worktree** 指着冻结源码，返回它的路径。
///
/// 复用规则：目录已经在、`HEAD` 就是那个 sha ⇒ 直接复用（构建缓存也一起留着）。
pub fn ensure_worktree(
    repo_root: &Path,
    version: &str,
    source_commit: &str,
    summary: &mut Vec<String>,
) -> Result<PathBuf, AppError> {
    let dir = worktree_dir(repo_root, source_commit);
    if dir.join(".git").exists() {
        let head = git_in(&dir, &["rev-parse", "HEAD"])?;
        if head.trim() == source_commit {
            summary.push(format!("复用冻结 worktree {}", dir.display()));
            link_node_modules(repo_root, &dir)?;
            assert_worktree_version(&dir, version)?;
            return Ok(dir);
        }
        /* 同一 sha8 目录指着别的提交（极少见）：删了重建，别拿错源码编 */
        let _ = std::process::Command::new("git")
            .args(["worktree", "remove", "--force"])
            .arg(&dir)
            .current_dir(repo_root)
            .output();
        let _ = std::fs::remove_dir_all(&dir);
    }
    if let Some(parent) = dir.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::io("建不了发布 worktree 目录").with_detail(e.to_string()))?;
    }
    let out = std::process::Command::new("git")
        .args(["worktree", "add", "--detach"])
        .arg(&dir)
        .arg(source_commit)
        .current_dir(repo_root)
        .output()
        .map_err(|e| AppError::io("起不了 git worktree").with_detail(e.to_string()))?;
    if !out.status.success() {
        return Err(AppError::io("建不了冻结源码的 worktree")
            .with_detail(String::from_utf8_lossy(&out.stderr).trim().to_owned()));
    }
    summary.push(format!(
        "已建冻结 worktree {} → {}",
        dir.display(),
        short(source_commit)
    ));
    link_node_modules(repo_root, &dir)?;
    assert_worktree_version(&dir, version)?;
    Ok(dir)
}

/// 依赖目录软链回主仓：worktree 里没有 `node_modules`，而 `npm run tauri` 需要它。
fn link_node_modules(repo_root: &Path, dir: &Path) -> Result<(), AppError> {
    let target = dir.join("node_modules");
    if target.exists() {
        return Ok(());
    }
    let src = repo_root.join("node_modules");
    if !src.exists() {
        return Err(AppError::not_found(
            "主仓没有 node_modules —— 先在主仓跑一次 npm ci",
        ));
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(&src, &target)
        .map_err(|e| AppError::io("链不上 node_modules").with_detail(e.to_string()))?;
    Ok(())
}

/// worktree 里那一笔的 `version.toml` 必须就是这一版（冻结源码没改版本号 = 编出来的包是别人的号）。
fn assert_worktree_version(dir: &Path, version: &str) -> Result<(), AppError> {
    let text = std::fs::read_to_string(dir.join("version.toml")).map_err(|e| {
        AppError::io("读不了冻结 worktree 的 version.toml").with_detail(e.to_string())
    })?;
    let got = text
        .lines()
        .find_map(|l| l.trim().strip_prefix("version = "))
        .map(|v| v.trim().trim_matches('"').to_owned())
        .unwrap_or_default();
    if got != version {
        return Err(AppError::invalid_argument(format!(
            "冻结源码里的版本号是 {got}，这一趟要发 {version} —— 版本号那一笔还没合进冻结源码",
        )));
    }
    Ok(())
}

/// 本机那一档构建：跑 `npm run tauri -- build`，出 dmg 与 `.app`，再打 `.app.zip`。
///
/// 返回**本机平台**的产物身份（macOS 两份 / Windows 两份）。
pub fn build_local(
    worktree: &Path,
    version: &str,
    platform: &'static Platform,
    summary: &mut Vec<String>,
) -> Result<Vec<ArtifactFact>, AppError> {
    summary.push(format!("开始构建 {}（{version}）", platform.label));
    let started = std::time::Instant::now();
    let out = std::process::Command::new("npm")
        .args(["run", "tauri", "--", "build"])
        .current_dir(worktree)
        /* 工作台 dev 会注入 TAURI_CONFIG（空窗口清单）；客户端构建必须清掉 */
        .env_remove("TAURI_CONFIG")
        .output()
        .map_err(|e| AppError::io("起不了 npm").with_detail(e.to_string()))?;
    if !out.status.success() {
        return Err(AppError::io("构建安装包失败").with_detail(
            String::from_utf8_lossy(&out.stderr)
                .lines()
                .rev()
                .take(12)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("\n"),
        ));
    }
    summary.push(format!("构建完成（{} 秒）", started.elapsed().as_secs()));

    /* 包里自报的号 —— 与要发的号对不上就停（APP_VERSION 是编译期常量） */
    let app = locate_app(worktree)?;
    assert_app_speaks_version(&app, version)?;

    let mut facts = Vec::new();
    for kind in platform.artifacts {
        match kind.kind {
            "dmg" => {
                let dmg = locate_dmg(worktree, version, kind.arch)?;
                patch_dmg_extras(worktree, &dmg)?;
                facts.push(ArtifactFact::from_path(version, platform, kind, &dmg)?);
            }
            "appZip" => {
                let zip = pack_app_zip(&app, version, platform)?;
                facts.push(ArtifactFact::from_path(version, platform, kind, &zip)?);
            }
            other => {
                return Err(AppError::internal(format!(
                    "本机构建不认识这种产物：{other}"
                )))
            }
        }
    }
    for f in &facts {
        summary.push(format!("产物 {}（{} 字节）", f.name, f.size));
    }
    Ok(facts)
}

fn locate_app(worktree: &Path) -> Result<PathBuf, AppError> {
    let dir = worktree.join("target/release/bundle/macos");
    let mut apps: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|e| {
            AppError::io("找不到 macOS 应用包目录（构建没产出 .app？）")
                .with_detail(format!("{}：{e}", dir.display()))
        })?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("app"))
        .collect();
    apps.sort();
    match apps.as_slice() {
        [one] => Ok(one.clone()),
        [] => Err(AppError::io(format!(
            "bundle/macos 下没有 .app：{}",
            dir.display()
        ))),
        many => Err(AppError::io(format!(
            "bundle/macos 下有 {} 份 .app —— 自己确认要发哪一份",
            many.len()
        ))),
    }
}

fn locate_dmg(worktree: &Path, version: &str, arch: &str) -> Result<PathBuf, AppError> {
    let name = format!("{PRODUCT}_{version}_{arch}.dmg");
    let path = worktree.join("target/release/bundle/dmg").join(&name);
    if path.is_file() {
        return Ok(path);
    }
    Err(AppError::io(format!(
        "构建完了但 {} 不存在 —— 打 dmg 那一步可能被跳过了",
        path.display()
    )))
}

/// dmg 里追加「安装说明 + 终端快捷方式」（未签名，macOS 会拦第一次打开）。
fn patch_dmg_extras(worktree: &Path, dmg: &Path) -> Result<(), AppError> {
    let out = std::process::Command::new("bash")
        .args(["scripts/patch-dmg-extras.sh"])
        .arg(dmg)
        .current_dir(worktree)
        .output()
        .map_err(|e| AppError::io("起不了 patch-dmg-extras").with_detail(e.to_string()))?;
    if !out.status.success() {
        return Err(AppError::io("安装包追加说明失败")
            .with_detail(String::from_utf8_lossy(&out.stderr).trim().to_owned()));
    }
    Ok(())
}

/// 应用内更新用的 `.app.zip`（系统 `ditto`，不引压缩依赖）。
fn pack_app_zip(
    app: &Path,
    version: &str,
    platform: &'static Platform,
) -> Result<PathBuf, AppError> {
    let kind = platform
        .artifacts
        .iter()
        .find(|k| k.kind == "appZip")
        .ok_or_else(|| AppError::internal("平台表里没有 appZip"))?;
    let zip = app.with_file_name(artifact_name(version, kind));
    let _ = std::fs::remove_file(&zip);
    let out = std::process::Command::new("ditto")
        .args(["-c", "-k", "--sequesterRsrc", "--keepParent"])
        .arg(app)
        .arg(&zip)
        .output()
        .map_err(|e| AppError::io("起不了系统 ditto").with_detail(e.to_string()))?;
    if !out.status.success() {
        return Err(AppError::io("打包 .app.zip 失败")
            .with_detail(String::from_utf8_lossy(&out.stderr).trim().to_owned()));
    }
    Ok(zip)
}

/// 读包里自报的 `CFBundleShortVersionString`，与要发的号对账。
fn assert_app_speaks_version(app: &Path, want: &str) -> Result<(), AppError> {
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
            "包内自报 {got}，这次要发的是 {want} —— 两个号不是同一个",
        ))
        .with_detail(
            "多半是改了版本号却没重新编译（同一棵 worktree 会复用缓存）。\
             删掉 target/release-worktree/<sha8> 再跑一次"
                .to_owned(),
        ));
    }
    Ok(())
}

fn git_in(dir: &Path, args: &[&str]) -> Result<String, AppError> {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| AppError::io("起不了 git").with_detail(e.to_string()))?;
    if !out.status.success() {
        return Err(AppError::io("git 命令失败")
            .with_detail(String::from_utf8_lossy(&out.stderr).trim().to_owned()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn short(sha: &str) -> String {
    sha.chars().take(7).collect()
}
