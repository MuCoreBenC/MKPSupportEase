//! **本地预览交付事务**（2026-11-06 作者定的最终规则）。
//!
//! # 一句话
//!
//! 「生成并交付当前预览版本」= 编译 → 写本地清单 → **提交并打不可变的 tag** →
//! 把版本源滚到下一轮。全程**不推送、不建 Release、不碰 CI**。
//!
//! # 为什么顺序是这样
//!
//! ```text
//! preflight（只读）：
//!   · 读要冻结的那一版（src-tauri/Cargo.toml）
//!   · git status：**未跟踪文件一律阻断**（作者：不能顺手提交别的未完成工作）
//!         │
//! build      →  编译失败 ⇒ 什么都没发生（不提交、不打 tag、不涨号）
//!         │
//! deliver    →  local-release：写清单 + 历史副本 + 账本（三处同号，它自己校验）
//!         │
//! commit     →  只 stage **已跟踪的改动** + 账本；消息由版本号生成
//!         │
//! tag        →  `git tag -a v<版本>`（已存在就跳过 —— 恢复时是幂等的）
//!         │
//! rollover   →  `version.mjs ensure-preview`：版本源滚到下一轮
//!         │
//! journal    →  target/local-preview/journal.json 记下这一趟走到哪
//! ```
//!
//! ★ **tag 只在"构建成功 + 交付文件完整"之后才建**（作者原话）。所以这里没有
//!   "先打 tag 再构建"那种顺序 —— 那是 `release_tx`（云端发版）的旧形状，别照抄。
//!
//! ★ **未跟踪文件阻断**的理由不是洁癖：`git add -A` 会把别人正在写的半成品
//!   一起冻进 tag，而 tag 是**不可变**的 —— 那种错误撤销不掉。

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// 交付账本（辅助，不决定版本号；版本冻结由 git tag 决定）。
const LEDGER: &str = "delivery/local-preview-history.json";

/// 事务日志（`target/` 下，git 看不见）。
const JOURNAL: &str = "target/local-preview/journal.json";

/// 一次 preflight 的结论：能不能交付、会提交些什么。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewPlan {
    /// 要冻结的那一版（`src-tauri/Cargo.toml` 里的号）
    pub version: String,
    /// 冻结之后会滚到的号（`ensure-preview` 算出来的，这里只做展示）
    pub next_version: String,
    pub tag: String,
    /// 会不会真的写东西（有阻断原因时 = false）
    pub ok: bool,
    /// 阻断原因（空 = 可以交付）
    pub blocked_by: Vec<String>,
    /// 这一趟会提交的文件（已跟踪改动 + 账本）
    pub files: Vec<String>,
    /// 有改动但**不属于本次交付**的文件（未跟踪）—— 有它就阻断
    pub strangers: Vec<String>,
    /// 当前分支（在 main 上交付会**只在本地**提交一次，绝不推送）
    pub branch: String,
}

/// 一趟交付的结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewReport {
    pub version: String,
    pub next_version: String,
    pub tag: String,
    pub commit: Option<String>,
    /// 走到哪一步（`built` / `delivered` / `committed` / `tagged` / `rolled` / `failed`）
    pub stage: String,
    /// 人话结论
    pub message: String,
    /// 日志（`target/local-preview/deliver.log`）
    pub log: String,
}

fn repo() -> PathBuf {
    crate::workbench::paths::repo_root()
}

fn cargo_version(repo: &Path) -> Result<String, AppError> {
    let text = std::fs::read_to_string(repo.join("src-tauri").join("Cargo.toml"))
        .map_err(|e| AppError::io("读不到 src-tauri/Cargo.toml").with_detail(e.to_string()))?;
    text.lines()
        .find_map(|l| l.strip_prefix("version = \""))
        .and_then(|rest| rest.split('"').next())
        .map(str::to_owned)
        .ok_or_else(|| AppError::invalid_argument("src-tauri/Cargo.toml 里没有 version"))
}

fn tag_of(version: &str) -> String {
    format!("v{version}")
}

/// `git status --porcelain` → （已跟踪改动, 未跟踪文件）。
fn status_paths(repo: &Path) -> Result<(Vec<String>, Vec<String>), AppError> {
    let out = Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=all"])
        .current_dir(repo)
        .output()
        .map_err(|e| AppError::io("起不来 git").with_detail(e.to_string()))?;
    if !out.status.success() {
        return Err(AppError::invalid_argument("git status 失败")
            .with_detail(String::from_utf8_lossy(&out.stderr).into_owned()));
    }
    let mut tracked = Vec::new();
    let mut untracked = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        if line.len() < 4 {
            continue;
        }
        let (code, path) = line.split_at(2);
        let path = path.trim_start();
        if path == LEDGER {
            /* 账本是**这一趟要写的**，它自己不是"别人的半成品" */
            tracked.push(path.to_owned());
        } else if code == "??" {
            untracked.push(path.to_owned());
        } else {
            tracked.push(path.to_owned());
        }
    }
    Ok((tracked, untracked))
}

fn current_branch(repo: &Path) -> String {
    Command::new("git")
        .args(["symbolic-ref", "--short", "HEAD"])
        .current_dir(repo)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .unwrap_or_else(|| "（detached）".to_owned())
}

fn tag_exists(repo: &Path, tag: &str) -> bool {
    Command::new("git")
        .args([
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("refs/tags/{tag}"),
        ])
        .current_dir(repo)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// 冻结之后会滚到哪一版（问 `version.mjs`，不在这儿重算一遍规则）。
fn next_version(repo: &Path) -> String {
    let base = cargo_version(repo).unwrap_or_default();
    /* `node scripts/version.mjs current` 只回显；算下一号要跑 ensure（会写盘）——
    这里只做展示，所以照"预览号 +1"的规则推一个，**真正写盘由 rollover 做**。 */
    match base.rsplit_once("-preview.") {
        Some((head, seq)) => match seq.parse::<u64>() {
            Ok(n) => format!("{head}-preview.{}", n + 1),
            Err(_) => base,
        },
        None => base,
    }
}

fn plan_of(repo: &Path) -> Result<PreviewPlan, AppError> {
    let version = cargo_version(repo)?;
    let (tracked, untracked) = status_paths(repo)?;
    let tag = tag_of(&version);
    let mut blocked = Vec::new();
    if !untracked.is_empty() {
        blocked.push(format!(
            "工作区里有 {} 个**不属于本次交付**的文件（未跟踪）—— 先把它们提交或加进 .gitignore，\
             这条事务不会替你决定它们的归属",
            untracked.len()
        ));
    }
    if tag_exists(repo, &tag) {
        blocked.push(format!(
            "{tag} 已经存在 —— 这一版已经冻结过。要再来一轮请先 npm run version:ensure"
        ));
    }
    Ok(PreviewPlan {
        next_version: next_version(repo),
        version,
        tag,
        ok: blocked.is_empty(),
        blocked_by: blocked,
        files: tracked,
        strangers: untracked,
        branch: current_branch(repo),
    })
}

/// **交付前看一眼**（只读）：会不会阻断、会提交哪些文件。
#[tauri::command]
pub fn wb_preview_preflight() -> Result<PreviewPlan, AppError> {
    crate::ipc::traced("wb_preview_preflight", |_| plan_of(&repo()))
}

/// 跑一条命令，把 stdout / stderr 追加进日志；失败就带着**最后几行**报错。
/// 交付日志（**追加语义**，但走 `fsx::atomic`）。
///
/// 本仓禁 `OpenOptions`（clippy.toml 里写着理由：那是绕开原子写的闸门）。
/// 这里要的只是"一行行攒下来"，所以攒在内存里、每次 `atomic_write` 全量落盘 ——
/// 日志在 `target/` 下，量级几 KB，重写一遍比开一个不原子的句柄便宜。
struct LogBuf {
    path: PathBuf,
    text: String,
}

impl LogBuf {
    fn open(path: PathBuf) -> Self {
        let mut text = std::fs::read_to_string(&path).unwrap_or_default();
        /* 只留最后 32KB：这是给人看"断在哪一步"的，不是归档 */
        if text.len() > 32 * 1024 {
            let cut = text.len() - 32 * 1024;
            text = text.split_off(cut);
        }
        let mut me = Self { path, text };
        me.push("\n--- 新的一趟 ---");
        me
    }

    fn push(&mut self, line: &str) {
        self.text.push_str(line);
        self.text.push('\n');
        let _ = crate::fsx::atomic::atomic_write(&self.path, self.text.as_bytes());
    }
}

fn run_logged(repo: &Path, log: &mut LogBuf, argv: &[&str]) -> Result<(), AppError> {
    run_logged_env(repo, log, argv, &[])
}

fn run_logged_env(
    repo: &Path,
    log: &mut LogBuf,
    argv: &[&str],
    env: &[(&str, &str)],
) -> Result<(), AppError> {
    log.push(&format!(
        "\n$ {} {}",
        env.iter()
            .map(|(k, _)| format!("{k}=…"))
            .collect::<Vec<_>>()
            .join(" "),
        argv.join(" ")
    ));
    let mut cmd = Command::new(argv[0]);
    cmd.args(&argv[1..]).current_dir(repo);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd
        .output()
        .map_err(|e| AppError::io(format!("起不来 {}", argv[0])).with_detail(e.to_string()))?;
    for (name, bytes) in [("out", &out.stdout), ("err", &out.stderr)] {
        let text = String::from_utf8_lossy(bytes);
        for line in text.lines() {
            log.push(&format!("[{name}] {line}"));
        }
    }
    if !out.status.success() {
        let tail: String = String::from_utf8_lossy(&out.stderr)
            .lines()
            .rev()
            .take(6)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        return Err(
            AppError::invalid_argument(format!("{} 失败", argv[..2].join(" "))).with_detail(tail),
        );
    }
    Ok(())
}

fn write_journal(repo: &Path, report: &PreviewReport) {
    let path = repo.join(JOURNAL);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_string_pretty(report) {
        let _ = crate::fsx::atomic::atomic_write(&path, text.as_bytes());
    }
}

/**
 * **生成并交付当前预览版本**（人点按钮触发，= 授权这一趟的 commit + tag）。
 *
 * ★ 不推送、不建 Release、不触发 CI —— 与 `release_tx`（云端发版）是两条物理隔离的路。
 */
#[tauri::command]
pub fn wb_preview_deliver() -> Result<PreviewReport, AppError> {
    crate::ipc::traced("wb_preview_deliver", |_| {
        let repo = repo();
        let plan = plan_of(&repo)?;
        let log_path = repo.join("target/local-preview/deliver.log");
        if let Some(dir) = log_path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let log_text = log_path.display().to_string();
        let mut log = LogBuf::open(log_path);

        let mut report = PreviewReport {
            version: plan.version.clone(),
            next_version: plan.next_version.clone(),
            tag: plan.tag.clone(),
            commit: None,
            stage: "preflight".to_owned(),
            message: String::new(),
            log: log_text,
        };

        if !plan.ok {
            report.stage = "blocked".to_owned();
            report.message = plan.blocked_by.join("；");
            write_journal(&repo, &report);
            return Err(AppError::invalid_argument(report.message.clone())
                .with_detail(format!("详细日志：{}", report.log)));
        }

        /* ① 编译（失败 ⇒ 什么都没发生） */
        if let Err(e) = run_logged(&repo, &mut log, &["npm", "run", "tauri", "--", "build"]) {
            report.stage = "failed".to_owned();
            report.message = format!("编译失败，这一版没有冻结：{}", e.message);
            write_journal(&repo, &report);
            return Err(e.with_detail(format!("详细日志：{}", report.log)));
        }
        report.stage = "built".to_owned();

        /* ② 写本地清单 + 历史副本 + 账本（它自己会校验"清单号 = 包内号"） */
        if let Err(e) = run_logged(&repo, &mut log, &["node", "scripts/local-release.mjs"]) {
            report.stage = "failed".to_owned();
            report.message = format!("交付失败（清单没写成），这一版没有冻结：{}", e.message);
            write_journal(&repo, &report);
            return Err(e.with_detail(format!("详细日志：{}", report.log)));
        }
        report.stage = "delivered".to_owned();

        /* ③ 提交**已跟踪的改动**（未跟踪的在 preflight 已经拦掉了） */
        if !plan.files.is_empty() {
            let mut args: Vec<String> = vec!["add".into(), "--".into()];
            args.extend(plan.files.iter().cloned());
            let refs: Vec<&str> = args.iter().map(String::as_str).collect();
            run_logged(&repo, &mut log, &refs)?;
        }
        let staged = Command::new("git")
            .args(["diff", "--cached", "--quiet"])
            .current_dir(&repo)
            .status()
            .map(|s| !s.success())
            .unwrap_or(false);
        if staged {
            let msg = format!(
                "preview: {}\n\n本地预览交付（未推送、未建 Release）。\n包与清单同号，账本见 {LEDGER}。",
                plan.version
            );
            /* ★ 闸① 不许在 main 上直接提交 —— 但这条事务是**人点按钮**、**只在本机**、
             **绝不推送**的冻结动作；带逃生口提交并让它记一笔，比让整个功能用不了更诚实。 */
            run_logged_env(
                &repo,
                &mut log,
                &["git", "commit", "-m", &msg],
                &[("ALLOW_COMMIT_ON_MAIN", "1")],
            )?;
            report.commit = head_of(&repo);
            report.stage = "committed".to_owned();
        }

        /* ④ 打**不可变**的 tag（已存在就跳过 —— 恢复时幂等） */
        if !tag_exists(&repo, &plan.tag) {
            run_logged(
                &repo,
                &mut log,
                &[
                    "git",
                    "tag",
                    "-a",
                    &plan.tag,
                    "-m",
                    &format!("本地预览交付 {}", plan.version),
                ],
            )?;
        }
        report.stage = "tagged".to_owned();

        /* ⑤ 滚到下一轮 */
        run_logged(
            &repo,
            &mut log,
            &["node", "scripts/version.mjs", "ensure-preview"],
        )?;
        report.next_version = cargo_version(&repo).unwrap_or(plan.next_version);
        report.stage = "rolled".to_owned();
        report.message = format!(
            "{} 已冻结（tag {}），工作区已进入 {}",
            plan.version, plan.tag, report.next_version
        );

        write_journal(&repo, &report);
        Ok(report)
    })
}

fn head_of(repo: &Path) -> Option<String> {
    Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 造一个最小仓库：`git init` + 把 `src-tauri/Cargo.toml` 收进索引。
    ///
    /// ★ 身份用 `-c user.email=…` 传，**不写 git config**（那是全局副作用）。
    fn scratch_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let git = |args: &[&str]| {
            let out = Command::new("git")
                /* ★ 关掉钩子：本仓的 `core.hooksPath` 指向 `.githooks`（闸① 不许在 main 上提交），
                测试仓库会继承那条全局配置 —— 而这里要验的是"分得清谁的文件"，不是闸。 */
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
        git(&["init", "-q"]);
        std::fs::create_dir_all(root.join("src-tauri")).unwrap();
        crate::fsx::atomic::atomic_write(
            &root.join("src-tauri").join("Cargo.toml"),
            b"version = \"0.0.12-preview.7\"\n",
        )
        .unwrap();
        git(&["add", "src-tauri/Cargo.toml"]);
        git(&["commit", "-q", "-m", "seed"]);
        dir
    }

    /// ★ 作者 2026-11-06：「不能顺手提交你其他未完成的工作」——
    /// 工作区里有**未跟踪**文件时，事务必须停下并把它们列出来，而不是一起冻进 tag。
    #[test]
    fn an_untracked_file_blocks_the_delivery() {
        let d = scratch_repo();
        crate::fsx::atomic::atomic_write(&d.path().join("scratch.txt"), b"wip").unwrap();

        let plan = plan_of(d.path()).unwrap();
        assert!(!plan.ok, "有未跟踪文件时不许放行");
        assert!(
            plan.strangers.iter().any(|f| f.contains("scratch.txt")),
            "要指名道姓：{:?}",
            plan.strangers
        );
        assert!(
            plan.blocked_by.iter().any(|m| m.contains("不属于本次交付")),
            "要说清为什么停：{:?}",
            plan.blocked_by
        );
    }

    /// 已跟踪的改动**属于本次交付**（构建就是照这个状态跑的），要进 `files`。
    #[test]
    fn tracked_changes_are_part_of_this_delivery() {
        let d = scratch_repo();
        crate::fsx::atomic::atomic_write(
            &d.path().join("src-tauri").join("Cargo.toml"),
            b"version = \"0.0.12-preview.7\"\n# touched\n",
        )
        .unwrap();

        let plan = plan_of(d.path()).unwrap();
        assert!(plan.ok, "只有已跟踪改动时不该阻断：{:?}", plan.blocked_by);
        assert!(
            plan.files.iter().any(|f| f.contains("Cargo.toml")),
            "这一份改动要进提交清单：{:?}",
            plan.files
        );
        assert!(plan.strangers.is_empty());
        assert_eq!(plan.next_version, "0.0.12-preview.8");
        assert_eq!(plan.tag, "v0.0.12-preview.7");
    }

    /// 同一个号**不许冻结第二次**（tag 是不可变的）：已经打过就停下。
    #[test]
    fn a_frozen_version_cannot_be_delivered_again() {
        let d = scratch_repo();
        let out = Command::new("git")
            .args(["tag", "-a", "v0.0.12-preview.7", "-m", "x"])
            .current_dir(d.path())
            .output()
            .unwrap();
        assert!(out.status.success());

        let plan = plan_of(d.path()).unwrap();
        assert!(!plan.ok);
        assert!(
            plan.blocked_by.iter().any(|m| m.contains("已经冻结过")),
            "要说清这一版冻结过：{:?}",
            plan.blocked_by
        );
    }
}
