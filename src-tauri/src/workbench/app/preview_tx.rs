//! **本地预览交付事务** —— 版本管理核心。
//!
//! # 唯一规则
//!
//! **Tag 决定版本是否冻结。** 没有 tag 就一直是当前开发版本（改代码、编译、commit、push
//! 都不改变它）。只有点「生成并交付」（或 CLI 带 `--confirm`）才冻结这一版，成功后立刻
//! 把唯一版本源 `version.toml` 滚到下一版。
//!
//! # 阶段（每一步都幂等，重跑 = 从断点继续）
//!
//! ```text
//! 准备    读 version.toml；列已跟踪改动与未跟踪文件；未跟踪即阻断
//!   │
//! 提交    只 `git add -- <已跟踪改动>`（绝不 add -A）→ 得到确定的 commit
//!   │
//! 构建    从**该 commit 的隔离 worktree** 构建（工作区再变也不影响产物）
//!   │
//! 校验    产物自报的号必须等于 version.toml，且四处派生文件全等
//!   │
//! 冻结    `git tag -a`（指向那个 commit；已存在则核对后跳过）
//!   │
//! 记账    账本追加 ok（只在 tag 成功之后；这是"已交付"的唯一书面依据）
//!   │
//! 滚动    `version.mjs ensure-preview` → 下一版
//! ```
//!
//! 全程**不推送、不建 Release、不碰 CI**；与 `release_tx`（云端发版）物理隔离。
//!
//! # 为什么先提交再构建
//!
//! tag 必须指向"构建实际用的那堆字节"。Rust 构建读的是工作区，所以只能先把要冻的那一版
//! 提交出来，再从这个 commit 的 worktree 构建 —— 否则构建期间工作区被改（AI 或另一个
//! 终端），产物就和 tag 对不上了。

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// 唯一版本源。
const SOURCE: &str = "version.toml";

/// 交付账本（入库；「哪一版已冻结」的书面依据）。
const LEDGER: &str = "delivery/local-preview-history.json";

/// 事务日志（`target/` 下，git 看不见）。
const JOURNAL: &str = "target/local-preview/journal.json";

/// 交付日志。
const LOG: &str = "target/local-preview/deliver.log";

/// **交付前看一眼**（只读，不做任何写入）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewPlan {
    /// 要冻结的那一版（`version.toml`）
    pub version: String,
    /// 冻结之后会滚到的号（只做展示，真正写盘由滚动那一步做）
    pub next_version: String,
    pub tag: String,
    /// 能不能交付
    pub ok: bool,
    /// 阻断原因（空 = 可以交付）
    pub blocked_by: Vec<String>,
    /// **会被本次提交**的文件（已跟踪的改动；绝不 `add -A`）
    pub deliverable: Vec<String>,
    /// **未跟踪**的文件 —— 构建可能读到它们，却不在任何 commit 里，所以阻断
    pub strangers: Vec<String>,
    pub branch: String,
    /// 上一趟没跑完时的一句话（`None` = 上一趟跑完了）
    pub resume: Option<String>,
    /// 部署配置一致性（`version.toml` 与四处派生文件）
    pub consistent: bool,
}

/// 一趟交付的结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewReport {
    pub version: String,
    pub next_version: String,
    pub tag: String,
    pub commit: Option<String>,
    /// `blocked` / `failed` / `committed` / `built` / `tagged` / `recorded` / `rolled`
    pub stage: String,
    pub message: String,
    pub log: String,
}

fn repo() -> PathBuf {
    crate::workbench::paths::repo_root()
}

/// 读 `version.toml` 里的版本（唯一源）。
fn source_version(repo: &Path) -> Result<String, AppError> {
    let text = std::fs::read_to_string(repo.join(SOURCE))
        .map_err(|e| AppError::io(format!("读不到 {SOURCE}")).with_detail(e.to_string()))?;
    text.lines()
        .find_map(|l| l.strip_prefix("version = \""))
        .and_then(|rest| rest.split('"').next())
        .map(str::to_owned)
        .ok_or_else(|| AppError::invalid_argument(format!("{SOURCE} 里没有 version")))
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
        if code == "??" {
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

fn rev(repo: &Path, spec: &str) -> Option<String> {
    Command::new("git")
        .args(["rev-parse", spec])
        .current_dir(repo)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
}

fn tag_exists(repo: &Path, tag: &str) -> bool {
    rev(repo, &format!("refs/tags/{tag}^{{commit}}")).is_some()
}

/// 下一版（只做展示；真正写盘由 `ensure-preview` 做）。
fn next_version(version: &str) -> String {
    match version.rsplit_once("-preview.") {
        Some((head, seq)) => match seq.parse::<u64>() {
            Ok(n) => format!("{head}-preview.{}", n + 1),
            Err(_) => version.to_owned(),
        },
        None => version.to_owned(),
    }
}

/// 上一趟没跑完吗？返回一句话（版本 + 停在哪一步）。
fn unfinished_last(repo: &Path) -> Option<String> {
    let text = std::fs::read_to_string(repo.join(JOURNAL)).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let stage = v.get("stage")?.as_str()?;
    if stage == "rolled" {
        return None;
    }
    let version = v.get("version").and_then(|x| x.as_str()).unwrap_or("?");
    Some(format!("上一趟（{version}）停在 {stage}"))
}

/// 四处派生文件是否与 `version.toml` 一致（问 `version.mjs check --json`，不在这儿重算）。
fn versions_consistent(repo: &Path) -> bool {
    Command::new("node")
        .args(["scripts/version.mjs", "check", "--json"])
        .current_dir(repo)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| serde_json::from_slice::<serde_json::Value>(&o.stdout).ok())
        .and_then(|v| v.get("consistent").and_then(|x| x.as_bool()))
        .unwrap_or(false)
}

fn plan_of(repo: &Path) -> Result<PreviewPlan, AppError> {
    let version = source_version(repo)?;
    let (deliverable, strangers) = status_paths(repo)?;
    let tag = tag_of(&version);
    let mut blocked = Vec::new();

    /* 未跟踪文件可能被构建读到（`include_str!` / `mod` 都可能），却不在任何 commit 里
    —— 让它们进包等于把没提交的字节冻进不可变的 tag。 */
    if !strangers.is_empty() {
        blocked.push(format!(
            "{} 个文件还没有被 git 跟踪，无法确定是否属于这一版 —— 先提交或加进 .gitignore",
            strangers.len()
        ));
    }
    if tag_exists(repo, &tag) {
        blocked.push(format!(
            "{tag} 已经存在 —— 这一版冻结过了。要再来一轮先 npm run version:ensure"
        ));
    }
    let consistent = versions_consistent(repo);
    if !consistent {
        blocked.push(format!(
            "版本号不一致 —— 以 {SOURCE} 为准，跑 npm run version:sync 把四处对齐"
        ));
    }

    Ok(PreviewPlan {
        next_version: next_version(&version),
        version,
        tag,
        ok: blocked.is_empty(),
        blocked_by: blocked,
        deliverable,
        strangers,
        branch: current_branch(repo),
        resume: unfinished_last(repo),
        consistent,
    })
}

/// **交付前看一眼**（只读）：会不会阻断、会提交哪些文件。
#[tauri::command]
pub fn wb_preview_preflight() -> Result<PreviewPlan, AppError> {
    crate::ipc::traced("wb_preview_preflight", |_| plan_of(&repo()))
}

/// 交付日志（追加语义，但走 `fsx::atomic` —— 本仓禁 `OpenOptions`）。
///
/// 攒在内存里、每次全量落盘：日志在 `target/` 下，量级几 KB，重写一遍比开一个
/// 不原子的句柄便宜。
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
        for line in String::from_utf8_lossy(bytes).lines() {
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

/// 账本里那一版的记录（`None` = 没有这一版）。给"这一版到底交没交付"用。
#[cfg_attr(not(test), allow(dead_code))]
fn ledger_entry(repo: &Path, version: &str) -> Option<serde_json::Value> {
    let text = std::fs::read_to_string(repo.join(LEDGER)).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    v.get("items")?
        .as_array()?
        .iter()
        .find(|e| e.get("version").and_then(|x| x.as_str()) == Some(version))
        .cloned()
}

/// 记一笔交付（只在 tag 成功之后调用）。同版本+同 target 覆盖旧条目。
fn append_ledger(repo: &Path, report: &PreviewReport) -> Result<(), AppError> {
    let path = repo.join(LEDGER);
    let mut all: serde_json::Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_else(|| serde_json::json!({ "items": [] }));
    let overlay = repo
        .join("target/local-release")
        .join("release-preview.json");
    let manifest: Option<serde_json::Value> = std::fs::read_to_string(&overlay)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok());
    let entry = serde_json::json!({
        "version": report.version,
        "commit": report.commit,
        "tag": report.tag,
        "at": crate::workbench::clock::now_iso8601(),
        "channel": "preview",
        "target": "local",
        "result": "ok",
        "asset": manifest.as_ref().and_then(|m| m.get("asset").cloned()),
    });
    let items = all
        .get_mut("items")
        .and_then(|x| x.as_array_mut())
        .ok_or_else(|| AppError::invalid_argument("账本形状不对（缺 items）"))?;
    items.retain(|e| {
        !(e.get("version").and_then(|x| x.as_str()) == Some(report.version.as_str())
            && e.get("target").and_then(|x| x.as_str()) == Some("local"))
    });
    items.push(entry);
    let text = serde_json::to_string_pretty(&all)
        .map_err(|e| AppError::io("账本序列化失败").with_detail(e.to_string()))?;
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    crate::fsx::atomic::atomic_write(&path, format!("{text}\n").as_bytes())
        .map_err(|e| AppError::io("写不进账本").with_detail(e.message))
}

/// 从**确定的 commit** 构建：在 `target/` 下开一个 detach 的 worktree，用完删掉。
///
/// `CARGO_TARGET_DIR` 仍指向主仓库 `target/`（复用编译缓存，避免整套重编）；
/// 缓存只存**产物**，源码一律从 worktree 读，所以主工作区的改动不会混进来。
fn build_from_snapshot(repo: &Path, commit: &str, log: &mut LogBuf) -> Result<(), AppError> {
    let wt = repo.join("target").join("local-preview").join("wt");
    let _ = std::fs::remove_dir_all(&wt);
    let wt_arg = wt.to_string_lossy().into_owned();
    run_logged(
        repo,
        log,
        &["git", "worktree", "add", "--detach", &wt_arg, commit],
    )?;

    let target = repo.join("target");
    let target_arg = target.to_string_lossy().into_owned();
    let built = run_logged_env(
        &wt,
        log,
        &["npm", "run", "tauri", "--", "build"],
        &[("CARGO_TARGET_DIR", &target_arg)],
    );

    /* 产物从 worktree 的 bundle 搬回主仓 target —— 后两步（清单/校验）都在主仓看它 */
    if built.is_ok() {
        let from = wt.join("target/release/bundle");
        let to = repo.join("target/release/bundle");
        if from.is_dir() {
            let _ = run_logged(
                repo,
                log,
                &[
                    "rsync",
                    "-a",
                    "--delete",
                    &format!("{}/", from.display()),
                    &format!("{}/", to.display()),
                ],
            );
        }
    }
    let _ = Command::new("git")
        .args(["worktree", "remove", "--force", &wt_arg])
        .current_dir(repo)
        .output();
    let _ = std::fs::remove_dir_all(&wt);
    built
}

/// 这条事务的全部步骤。`confirm=false` 只做只读预检（CLI 默认档）。
pub fn deliver(confirm: bool) -> Result<PreviewReport, AppError> {
    let repo = repo();
    let plan = plan_of(&repo)?;
    let log_path = repo.join(LOG);
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
        return Err(AppError::invalid_argument(report.message.clone())
            .with_detail(format!("详细日志：{}", report.log)));
    }

    /* 只读预演：说清会做什么，一个字节都不写 */
    if !confirm {
        report.message = format!(
            "预演：会提交 {} 个已跟踪改动 → 从该提交的隔离快照构建 → 校验 → 打 tag {} → 记账 → 滚到 {}。\
             加 --confirm 才真的执行。",
            plan.deliverable.len(),
            plan.tag,
            plan.next_version
        );
        return Ok(report);
    }

    report.stage = "started".to_owned();
    write_journal(&repo, &report);

    /* ① 提交（只有已跟踪改动；绝不 add -A） */
    if !plan.deliverable.is_empty() {
        let mut args: Vec<String> = vec!["add".into(), "--".into()];
        args.extend(plan.deliverable.iter().cloned());
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        if let Err(e) = run_logged(&repo, &mut log, &refs) {
            report.stage = "failed".to_owned();
            report.message = format!("暂存失败：{}", e.message);
            write_journal(&repo, &report);
            return Err(e);
        }
        let staged = Command::new("git")
            .args(["diff", "--cached", "--quiet"])
            .current_dir(&repo)
            .status()
            .map(|s| !s.success())
            .unwrap_or(false);
        if staged {
            let msg = format!(
                "preview: {}\n\n本地预览交付（未推送、未建 Release）。",
                plan.version
            );
            /* 闸① 拦 main 上的直接提交；这条事务是"人点按钮/明确 --confirm"的本地冻结动作，
            带逃生口提交并让它留痕，比让整个功能用不了更诚实。 */
            if let Err(e) = run_logged_env(
                &repo,
                &mut log,
                &["git", "commit", "-m", &msg],
                &[("ALLOW_COMMIT_ON_MAIN", "1")],
            ) {
                report.stage = "failed".to_owned();
                report.message = format!("提交失败：{}", e.message);
                write_journal(&repo, &report);
                return Err(e);
            }
        }
    }
    report.commit = rev(&repo, "HEAD");
    report.stage = "committed".to_owned();
    write_journal(&repo, &report);
    let commit = report
        .commit
        .clone()
        .ok_or_else(|| AppError::invalid_argument("取不到 HEAD 提交"))?;

    /* ② 从**这个 commit** 构建 */
    if let Err(e) = build_from_snapshot(&repo, &commit, &mut log) {
        report.stage = "failed".to_owned();
        report.message = format!("构建失败，这一版没有冻结：{}", e.message);
        write_journal(&repo, &report);
        return Err(e.with_detail(format!("详细日志：{}", report.log)));
    }
    report.stage = "built".to_owned();
    write_journal(&repo, &report);

    /* ③ 摆好清单（它自己校验"清单号 = 包内号"） */
    if let Err(e) = run_logged(&repo, &mut log, &["node", "scripts/local-release.mjs"]) {
        report.stage = "failed".to_owned();
        report.message = format!("写清单失败，这一版没有冻结：{}", e.message);
        write_journal(&repo, &report);
        return Err(e.with_detail(format!("详细日志：{}", report.log)));
    }

    /* ④ 校验：版本源与四处派生文件必须全等（清单那边已核对过包内号） */
    if !versions_consistent(&repo) {
        report.stage = "failed".to_owned();
        report.message = format!("{SOURCE} 与四处派生文件不一致，拒绝冻结");
        write_journal(&repo, &report);
        return Err(AppError::invalid_argument(report.message.clone()));
    }

    /* ⑤ 冻结：tag 指向刚提交的那个 commit（已存在则核对后跳过 —— 幂等） */
    match rev(&repo, &format!("refs/tags/{}^{{commit}}", plan.tag)) {
        Some(existing) if existing != commit => {
            report.stage = "failed".to_owned();
            report.message = format!("{} 指向另一笔提交 —— 绝不把新代码挂到旧 tag 上", plan.tag);
            write_journal(&repo, &report);
            return Err(AppError::invalid_argument(report.message.clone()));
        }
        Some(_) => log.push(&format!("[out] {} 已存在且指向同一笔提交，跳过", plan.tag)),
        None => {
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
    }
    report.stage = "tagged".to_owned();
    write_journal(&repo, &report);

    /* ⑥ 记账 —— 只在 tag 成功之后 */
    append_ledger(&repo, &report)?;
    report.stage = "recorded".to_owned();
    write_journal(&repo, &report);

    /* ⑦ 滚动到下一版（幂等：重跑只会把它推到同一个号） */
    run_logged(
        &repo,
        &mut log,
        &["node", "scripts/version.mjs", "ensure-preview"],
    )?;
    report.next_version = source_version(&repo).unwrap_or(plan.next_version);
    report.stage = "rolled".to_owned();
    report.message = format!(
        "{} 已冻结（tag {}，commit {}），当前开发版本 {}",
        plan.version,
        plan.tag,
        &commit[..7.min(commit.len())],
        report.next_version
    );
    write_journal(&repo, &report);
    Ok(report)
}

/// 上一趟留下的现场：**已经打到 tag、但账本或滚动没走完**。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recovery {
    /// 哪一版（= 已存在的那个 tag）
    pub version: String,
    pub tag: String,
    pub commit: String,
    /// `ledger`（缺账本）/ `roll`（缺滚动）/ `both`
    pub missing: String,
}

/// 有没有"冻结了但没走完"的版本（只读）。
///
/// 判据只看两件事：**tag 在不在**（冻结事实）、**账本有没有 ok**（交付记录）。
/// tag 在而账本没有 ⇒ 补记；账本在而版本源还停在这一版 ⇒ 补滚。
fn recovery_of(repo: &Path) -> Option<Recovery> {
    let text = std::fs::read_to_string(repo.join(JOURNAL)).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let stage = v.get("stage")?.as_str()?;
    if stage == "rolled" {
        return None;
    }
    let version = v.get("version")?.as_str()?.to_owned();
    let tag = v
        .get("tag")
        .and_then(|x| x.as_str())
        .map(str::to_owned)
        .unwrap_or_else(|| tag_of(&version));
    let commit = rev(repo, &format!("refs/tags/{tag}^{{commit}}"))?;
    let has_ledger = ledger_entry(repo, &version).is_some();
    let rolled = source_version(repo).ok().as_deref() != Some(version.as_str());
    let missing = match (has_ledger, rolled) {
        (false, false) => "both",
        (false, true) => "ledger",
        (true, false) => "roll",
        (true, true) => return None,
    }
    .to_owned();
    Some(Recovery {
        version,
        tag,
        commit,
        missing,
    })
}

/// **接着把上一趟走完**（幂等：只补缺的那几步）。
#[tauri::command]
pub fn wb_preview_recover() -> Result<PreviewReport, AppError> {
    crate::ipc::traced("wb_preview_recover", |_| recover(true))
}

pub fn recover(confirm: bool) -> Result<PreviewReport, AppError> {
    let repo = repo();
    let Some(rec) = recovery_of(&repo) else {
        return Err(AppError::invalid_argument("没有需要接着走的交付"));
    };
    let log_path = repo.join(LOG);
    if let Some(dir) = log_path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let log_text = log_path.display().to_string();
    let mut log = LogBuf::open(log_path);
    let mut report = PreviewReport {
        version: rec.version.clone(),
        next_version: next_version(&rec.version),
        tag: rec.tag.clone(),
        commit: Some(rec.commit.clone()),
        stage: "recovering".to_owned(),
        message: String::new(),
        log: log_text,
    };
    if !confirm {
        report.message = format!(
            "预演：{} 已经打了 tag，但{}。加 --confirm 才真的补上。",
            rec.tag,
            match rec.missing.as_str() {
                "ledger" => "账本里还没有交付记录",
                "roll" => "版本源还没滚到下一版",
                _ => "账本和滚动都缺",
            }
        );
        return Ok(report);
    }

    if rec.missing != "roll" {
        append_ledger(&repo, &report)?;
        report.stage = "recorded".to_owned();
        write_journal(&repo, &report);
    }
    if rec.missing != "ledger" {
        run_logged(
            &repo,
            &mut log,
            &["node", "scripts/version.mjs", "ensure-preview"],
        )?;
    }
    report.next_version = source_version(&repo).unwrap_or(report.next_version);
    report.stage = "rolled".to_owned();
    report.message = format!(
        "{} 的现场已补齐（tag {}），当前开发版本 {}",
        rec.version, rec.tag, report.next_version
    );
    write_journal(&repo, &report);
    Ok(report)
}

/// 交付进度推送的事件名（`stage` + 日志尾行，界面据此显示"它还在动"）。
pub const PREVIEW_PROGRESS: &str = "preview-deliver-progress";

/// 一次交付跑了多久的**跨进程可见**状态（界面每 1.5 秒轮询它）。
///
/// 为什么不只用事件：`invoke` 回来的那一刻窗口才解锁，而构建要一两分钟 ——
/// 期间界面既没有返回值也没有事件，鼠标就一直转圈。轮询日志尾行是"它在动"的
/// 唯一同步可见的东西。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewProgress {
    /// 有没有一趟正在跑（含跑完但界面还没醒的那一小段）
    pub running: bool,
    /// 当前阶段（`preparing` / `committing` / `building` / `tagging` / `recording` / `rolling`）
    pub stage: String,
    /// 日志最后一行（编译时是 `Building [=====> ] 515/520`）
    pub tail: String,
    /// 跑完时的阶段（`rolled` / `failed` / `blocked`），跑着时为 null
    pub finished: Option<String>,
    /// 失败原因（只有 `failed` / `blocked` 时有）
    pub error: Option<String>,
}

impl PreviewProgress {
    fn idle() -> Self {
        Self {
            running: false,
            stage: String::new(),
            tail: String::new(),
            finished: None,
            error: None,
        }
    }
}

/// 进程内的当前进度。交付是在后台线程里跑的，界面只读这一份。
static PROGRESS: std::sync::Mutex<Option<PreviewProgress>> = std::sync::Mutex::new(None);

fn set_progress(p: PreviewProgress) {
    if let Ok(mut g) = PROGRESS.lock() {
        *g = Some(p);
    }
}

/// 读当前进度（不阻塞：锁只用来换一个克隆）。
fn read_progress() -> PreviewProgress {
    PROGRESS
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .unwrap_or_else(PreviewProgress::idle)
}

/// 日志尾行（压成一行、截断）—— 事件与轮询都复用它。
fn tail_line(repo: &Path) -> String {
    let text = std::fs::read_to_string(repo.join(LOG)).unwrap_or_default();
    let line = text
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("");
    line.replace('\r', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(160)
        .collect()
}

/// **派一趟交付到后台线程**，立刻返回。
///
/// 结束（成功或失败）时把结论推一条 `PREVIEW_PROGRESS` 事件，界面据此解锁按钮；
/// 期间界面靠 `wb_preview_progress` 轮询日志尾行 —— 构建要一两分钟，
/// 同步 `invoke` 会让人对着转圈的鼠标等。
#[tauri::command]
pub fn wb_preview_deliver(app: tauri::AppHandle) -> Result<(), AppError> {
    {
        let mut g = PROGRESS.lock().map_err(|_| AppError::io("进度锁坏了"))?;
        if g.as_ref().is_some_and(|p| p.running) {
            return Err(AppError::invalid_argument("上一趟交付还在跑，等它结束"));
        }
        *g = Some(PreviewProgress {
            running: true,
            stage: "preparing".to_owned(),
            tail: String::new(),
            finished: None,
            error: None,
        });
    }

    std::thread::spawn(move || {
        let root = crate::workbench::paths::repo_root();
        let mut last_tail = String::new();
        /* 后台盯日志尾行：每 400ms 把变化推给界面，这样不用等 invoke 返回 */
        let watcher_root = root.clone();
        let watcher_app = app.clone();
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stop_flag = stop.clone();
        let watcher = std::thread::spawn(move || {
            use tauri::Emitter as _;
            while !stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                let tail = tail_line(&watcher_root);
                if tail != last_tail {
                    last_tail = tail.clone();
                    let mut p = read_progress();
                    p.tail = tail.clone();
                    set_progress(p.clone());
                    let _ = watcher_app.emit(PREVIEW_PROGRESS, &p);
                }
                std::thread::sleep(std::time::Duration::from_millis(400));
            }
        });

        let result = deliver(true);
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        let _ = watcher.join();

        let mut p = read_progress();
        p.running = false;
        p.tail = tail_line(&root);
        match &result {
            Ok(r) => {
                p.stage = r.stage.clone();
                p.finished = Some(r.stage.clone());
                p.error = None;
            }
            Err(e) => {
                p.stage = "failed".to_owned();
                p.finished = Some("failed".to_owned());
                p.error = Some(e.message.clone());
            }
        }
        set_progress(p.clone());

        use tauri::Emitter as _;
        let _ = app.emit(PREVIEW_PROGRESS, &p);
    });

    Ok(())
}

/// 当前交付进度（界面每 1.5 秒问一次；**不阻塞**，只读进程内那一份）。
#[tauri::command]
pub fn wb_preview_progress() -> Result<PreviewProgress, AppError> {
    Ok(read_progress())
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

    /// 造一个真实仓库（钩子关掉：本仓 `core.hooksPath` 会拦 main 上的提交，
    /// 而这里要验的是"分得清谁的文件"，不是闸）。
    fn scratch_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let git = |args: &[&str]| {
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
        git(&["init", "-q", "-b", "main"]);
        crate::fsx::atomic::atomic_write(&root.join(SOURCE), b"version = \"0.0.12-preview.7\"\n")
            .unwrap();
        git(&["add", SOURCE]);
        git(&["commit", "-q", "-m", "seed"]);
        dir
    }

    /// 未跟踪文件一律阻断，并**指名道姓列出来**。
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
            plan.blocked_by
                .iter()
                .any(|m| m.contains("还没有被 git 跟踪")),
            "要说清为什么停：{:?}",
            plan.blocked_by
        );
    }

    /// 已跟踪的改动**属于本次交付**（构建就是照这个状态跑的），进 `deliverable`。
    #[test]
    fn tracked_changes_are_what_gets_committed() {
        let d = scratch_repo();
        crate::fsx::atomic::atomic_write(
            &d.path().join(SOURCE),
            b"version = \"0.0.12-preview.7\"\n# touched\n",
        )
        .unwrap();

        let plan = plan_of(d.path()).unwrap();
        assert!(
            plan.deliverable.iter().any(|f| f == SOURCE),
            "这一份改动要进提交清单：{:?}",
            plan.deliverable
        );
        assert!(plan.strangers.is_empty(), "{:?}", plan.strangers);
        assert_eq!(plan.next_version, "0.0.12-preview.8");
        assert_eq!(plan.tag, "v0.0.12-preview.7");
    }

    /// 同一个号**不许冻结第二次**。
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
            plan.blocked_by.iter().any(|m| m.contains("冻结过了")),
            "{:?}",
            plan.blocked_by
        );
    }

    /// **预演一个字节都不写**（CLI 默认档）：`deliver(false)` 只回一句话，
    /// 不提交、不建 tag、不记账、不滚版本。
    #[test]
    fn a_dry_run_writes_nothing() {
        let d = scratch_repo();
        crate::fsx::atomic::atomic_write(
            &d.path().join(SOURCE),
            b"version = \"0.0.12-preview.7\"\n# touched\n",
        )
        .unwrap();
        let plan = plan_of(d.path()).unwrap();
        assert!(!plan.deliverable.is_empty(), "要说清会提交什么");
        /* 这个夹具仓库里没有四处派生文件，所以"版本一致性"那一条必然红 ——
        预演要验的是**它不写盘**，不是它在这一格上放行。 */
        assert!(
            plan.blocked_by.iter().all(|m| m.contains("版本号不一致")),
            "只该因为夹具缺派生文件而红：{:?}",
            plan.blocked_by
        );

        /* 预演分支本身不写盘：账本不存在、tag 不存在、版本没被滚 */
        assert!(ledger_entry(d.path(), &plan.version).is_none());
        assert!(!tag_exists(d.path(), &plan.tag));
        assert_eq!(source_version(d.path()).unwrap(), plan.version);
    }

    /// 账本：同版本覆盖，不同版本追加。
    #[test]
    fn the_ledger_replaces_the_same_version() {
        let d = scratch_repo();
        let mk = |commit: &str| PreviewReport {
            version: "0.0.12-preview.7".to_owned(),
            next_version: "0.0.12-preview.8".to_owned(),
            tag: "v0.0.12-preview.7".to_owned(),
            commit: Some(commit.to_owned()),
            stage: "tagged".to_owned(),
            message: String::new(),
            log: String::new(),
        };
        append_ledger(d.path(), &mk("aaa")).unwrap();
        append_ledger(d.path(), &mk("bbb")).unwrap();

        let text = std::fs::read_to_string(d.path().join(LEDGER)).unwrap();
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        let items = v.get("items").unwrap().as_array().unwrap();
        assert_eq!(items.len(), 1, "同版本只留一条：{text}");
        assert_eq!(items[0].get("commit").unwrap().as_str(), Some("bbb"));
    }
}
