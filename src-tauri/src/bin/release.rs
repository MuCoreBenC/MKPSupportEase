//! **「发布软件版本」的命令行入口**（第四刀）—— 与工作台那颗按钮**同一个内核**。
//!
//! ```text
//!             发布软件版本事务核心（workbench::app::release_tx::run）
//!                          │
//!         ┌────────────────┴────────────────┐
//!   工作台 UI（wb_release_software）   本文件 + scripts/release.mjs
//! ```
//!
//! ★ 它**不是**第二套实现：这里只做三件事 —— 解析命令行、凑出发布目标、把内核的
//! 报告打出来。步骤本身（预检 / 版本 / tag / 构建 / Release / 上传 / release.json）
//! 一行都不在这里。`scripts/release.mjs` 保留它的前置校验、交互提问与三条纪律，
//! 把"改版本号与后面那一串"交给这里 —— 两个入口，一条路。
//!
//! # 用法
//!
//! ```text
//! cargo run --bin release --features workbench -- 0.0.2 "修了 X"        # 第一阶段：预检 → 版本 → 提交 → 推送 → PR
//! cargo run --bin release --features workbench -- 0.0.2 "修了 X" --merge # 第二阶段：合并 → 打 tag → 构建 → Release → 上传 → release.json
//! cargo run --bin release --features workbench -- --dry-run              # 只看预检结论，一个字节都不动
//! ```
//!
//! `--merge` 单独一趟是**刻意的**：合并之前要先看 CI（`scripts/release.mjs` 用
//! `gh pr checks` 等），而"等 CI"是脚本那边的活 —— 内核不轮询（作者定死的纪律）。
//!
//! # 凭据从哪来
//!
//! | 来源 | 怎么用 |
//! |---|---|
//! | `MKPSE_APP_DIR` 指向工作台的应用数据目录 | 复用发布账户配置 + 凭据文件里的 Token（**与工作台完全一致**） |
//! | `MKPSE_RELEASE_{PLATFORM,OWNER,REPO,TOKEN}` | 无窗口环境下显式给（CI / 纯终端） |
//! | 都没有 | 只做本地那一半（提交 / 推送 / 打 tag / 构建），如实说"没建 Release" |

use std::path::PathBuf;

use mkp_support_ease_lib::error::AppError;
use mkp_support_ease_lib::workbench::app::platform::{self, Hosting};
use mkp_support_ease_lib::workbench::app::publish_tx::{resolve_target, PublishTarget};
use mkp_support_ease_lib::workbench::app::release_history::{self, ReleaseRecord};
use mkp_support_ease_lib::workbench::app::release_tx::{
    run, ReleaseChannel, ReleaseOptions, ReleaseTxReport,
};
use mkp_support_ease_lib::workbench::clock;

fn main() {
    match run_cli() {
        Ok(report) => {
            // 摘要给人看（stderr），**结果给脚本看**（stdout 一行 JSON）——
            // 混在一起的话 `scripts/release.mjs` 就得解析人类语言。
            eprintln!("{}", report.summary);
            match serde_json::to_string(&report) {
                Ok(json) => println!("{json}"),
                Err(e) => {
                    eprintln!("报告序列化失败：{e}");
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("✗ {}", e.message);
            if let Some(d) = e.detail {
                eprintln!("  {d}");
            }
            std::process::exit(1);
        }
    }
}

fn run_cli() -> Result<mkp_support_ease_lib::workbench::app::release_tx::ReleaseTxReport, AppError>
{
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut positional: Vec<String> = Vec::new();
    let mut merge = false;
    let mut build = true;
    let mut dry_run = false;
    let mut open_review = true;
    let mut base = String::from("main");

    for a in &args {
        match a.as_str() {
            "--merge" => merge = true,
            "--no-build" => build = false,
            "--dry-run" => dry_run = true,
            "--no-review" => open_review = false,
            _ => {
                if let Some(v) = a.strip_prefix("--base=") {
                    base = v.to_owned();
                } else if !a.starts_with('-') {
                    positional.push(a.clone());
                } else {
                    return Err(AppError::invalid_argument(format!("认不出的参数：{a}")));
                }
            }
        }
    }
    // 位置参数：`[版本号] [说明]`。版本号可以省（沿用真值），说明也可以省。
    let version = positional.first().cloned();
    let notes = positional.get(1).cloned().unwrap_or_default();

    let opts = ReleaseOptions {
        version,
        notes,
        base,
        dry_run,
        merge,
        build,
        open_review,
    };

    let (target, hosting) = resolve_from_env();
    // M6（总纲 §5-M6）：Release 与附件发布到 Gitee —— 通道从 Gitee 发布账户来
    //（MKPSE_APP_DIR 复用工作台配置；MKPSE_RELEASE_* 显式给时 platform=gitee 即通道）。
    let release_parts = release_channel_parts();
    let release_channel = release_parts.as_ref().map(|(t, h)| ReleaseChannel {
        hosting: h.as_ref(),
        target: t,
    });
    let repo_root = mkp_support_ease_lib::workbench::paths::repo_root();
    eprintln!("仓库根：{}", repo_root.display());
    let report = run(
        &repo_root,
        &opts,
        target.as_ref(),
        hosting.as_ref().map(|h| h.as_ref()),
        release_channel.as_ref(),
    )?;
    record_history(&opts, &report);
    Ok(report)
}

/// 把这一趟记进**发布历史**（`<appDataDir>/release-history.json`，与工作台同一个文件）。
///
/// ★ 2026-10-07 补的一笔：以前只有工作台那颗「确认发布」会记账，CLI 这条路（也就是
///   "把提示词贴给 AI 让它代跑"那条路）发完在工作台历史里**查无此次** —— 人看不见
///   AI 到底发了什么。落点靠 `MKPSE_APP_DIR` 认门（发布账户也从它来，本来就是同一个目录）；
///   没给（CI 那种无窗口环境）就跳过。
/// ★ **尽力而为**：账本不是发布的一部分，记不上只提示一句，不把发版判成失败。
/// ★ 演练（`--dry-run`）不记账：那趟一个字节都没动。
fn record_history(opts: &ReleaseOptions, report: &ReleaseTxReport) {
    if opts.dry_run {
        return;
    }
    let Some(dir) = std::env::var_os("MKPSE_APP_DIR") else {
        return;
    };
    let root = PathBuf::from(dir);
    let file = root.join("release-history.json");
    match release_history::append(
        &root,
        ReleaseRecord::from_report(report, clock::now_iso8601()),
    ) {
        Ok(_) => eprintln!("已记入发布历史：{}", file.display()),
        Err(e) => eprintln!(
            "（发布历史没记上：{} —— 不影响这一趟发布）",
            e.message
        ),
    }
}

/// M6 的发布通道零件：Gitee 账户（工作台配置或 MKPSE_RELEASE_* 显式给）。
/// 没配 = `None` —— build=true 时内核如实报错。
fn release_channel_parts() -> Option<(PublishTarget, Box<dyn Hosting>)> {
    if let Some(dir) = std::env::var_os("MKPSE_APP_DIR") {
        let root = PathBuf::from(dir);
        if let Ok(t) = resolve_target(&root, Some("gitee")) {
            let hosting = platform::hosting(&t.platform, t.token.clone())?;
            return Some((t, hosting));
        }
    }
    if let (Some(owner), Some(repo), Some(token)) = (
        std::env::var("MKPSE_RELEASE_OWNER").ok(),
        std::env::var("MKPSE_RELEASE_REPO").ok(),
        std::env::var("MKPSE_RELEASE_TOKEN").ok(),
    ) {
        let platform =
            std::env::var("MKPSE_RELEASE_PLATFORM").unwrap_or_else(|_| "gitee".to_owned());
        let t = PublishTarget {
            platform: platform.clone(),
            repository_url: format!("https://gitee.com/{owner}/{repo}"),
            username: std::env::var("MKPSE_RELEASE_USER").unwrap_or_else(|_| owner.clone()),
            token,
            owner,
            repo,
        };
        let hosting = platform::hosting(&t.platform, t.token.clone())?;
        return Some((t, hosting));
    }
    None
}

/// 凑出发布目标与平台客户端。**没有就返回 `None`** —— 内核会退化成"只做本地那一半"。
fn resolve_from_env() -> (Option<PublishTarget>, Option<Box<dyn Hosting>>) {
    // ① 给了工作台的应用数据目录 → 复用它的发布账户配置（含凭据文件里的 Token）
    if let Some(dir) = std::env::var_os("MKPSE_APP_DIR") {
        let root = PathBuf::from(dir);
        if let Ok(t) = resolve_target(&root, None) {
            return (
                Some(t.clone()),
                platform::hosting(&t.platform, t.token.clone()),
            );
        }
    }
    // ② 环境变量显式给（无窗口环境）
    if let (Some(owner), Some(repo), Some(token)) = (
        std::env::var("MKPSE_RELEASE_OWNER").ok(),
        std::env::var("MKPSE_RELEASE_REPO").ok(),
        std::env::var("MKPSE_RELEASE_TOKEN").ok(),
    ) {
        let platform =
            std::env::var("MKPSE_RELEASE_PLATFORM").unwrap_or_else(|_| "github".to_owned());
        let t = PublishTarget {
            platform: platform.clone(),
            repository_url: format!("https://github.com/{owner}/{repo}"),
            username: std::env::var("MKPSE_RELEASE_USER").unwrap_or_else(|_| owner.clone()),
            token,
            owner,
            repo,
        };
        return (
            Some(t.clone()),
            platform::hosting(&t.platform, t.token.clone()),
        );
    }
    (None, None)
}
