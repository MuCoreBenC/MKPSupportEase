//! **发布的唯一入口**（命令行那一半）。
//!
//! ```bash
//! npm run ship -- --check                      # 只读：这一代到哪一步、下一步做什么
//! npm run ship -- --what stable --dry-run      # 演练：一个字节都不动
//! npm run ship -- --what stable --confirm      # 真发（要人当次许可）
//! npm run ship -- --what stable --confirm --stop-after staging --platform windows
//! ```
//!
//! 它自己**不问「这是首发还是补平台」**：状态从远端现读，谁没齐就补谁。
//! 于是换一台机器、上一趟被打断，都只需要再跑一次同样的命令。
//!
//! `--args-only` 只解析参数就退出（零写入、不联网）：给 `scripts/check-release-cli.mjs`
//! 这种自检用。

use mkp_support_ease_lib::error::AppError;
use mkp_support_ease_lib::runtime::release_info::UpdateChannel;
use mkp_support_ease_lib::workbench::app::release::{
    self, ReleaseOptions, ReleaseReport, StopAfter,
};

fn main() {
    match run_cli() {
        Ok(Some(report)) => {
            eprintln!("{}", human(&report));
            match serde_json::to_string(&report) {
                Ok(json) => println!("{json}"),
                Err(e) => {
                    eprintln!("报告序列化失败：{e}");
                    std::process::exit(1);
                }
            }
        }
        Ok(None) => {}
        Err(e) => {
            eprintln!("✗ {}", e.message);
            if let Some(d) = e.detail {
                eprintln!("  {d}");
            }
            std::process::exit(1);
        }
    }
}

fn run_cli() -> Result<Option<ReleaseReport>, AppError> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let parsed = parse(&args)?;

    /* 自检出口：只回答「这段参数会被解析成什么」——零写入、不联网、不碰 git */
    if parsed.args_only {
        println!(
            "{}",
            serde_json::json!({
                "mode": parsed.mode,
                "version": parsed.options.version,
                "channel": parsed.options.channel.map(|c| c.wire()),
                "stopAfter": parsed.options.stop_after.map(|s| format!("{s:?}").to_lowercase()),
                "platform": parsed.options.only_platform,
                "dryRun": parsed.options.dry_run,
                "confirmed": parsed.options.confirmed,
            })
        );
        return Ok(None);
    }

    let repo_root = mkp_support_ease_lib::workbench::paths::repo_root();
    let app_dir = release::app_dir();
    eprintln!("仓库根：{}", repo_root.display());

    let opts = ReleaseOptions {
        /* `--check` 只读：走 dry_run，但不要求确认 */
        dry_run: parsed.options.dry_run || parsed.mode == Mode::Check,
        ..parsed.options
    };
    let report = release::run(&repo_root, &app_dir, &opts)?;
    Ok(Some(report))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
enum Mode {
    Check,
    Publish,
}

#[derive(Debug)]
struct Parsed {
    mode: Mode,
    options: ReleaseOptions,
    args_only: bool,
}

fn parse(args: &[String]) -> Result<Parsed, AppError> {
    let mut mode = Mode::Publish;
    let mut version: Option<String> = None;
    let mut notes = String::new();
    let mut channel: Option<UpdateChannel> = None;
    let mut dry_run = false;
    let mut confirmed = false;
    let mut stop_after: Option<StopAfter> = None;
    let mut only_platform: Option<String> = None;
    let mut args_only = false;

    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        i += 1;
        match a.as_str() {
            "--check" | "--inspect" => mode = Mode::Check,
            "--dry-run" => dry_run = true,
            "--confirm" => confirmed = true,
            "--args-only" => args_only = true,
            "--notes" => {
                notes = args
                    .get(i)
                    .cloned()
                    .ok_or_else(|| AppError::invalid_argument("--notes 后面要跟一句话"))?;
                i += 1;
            }
            "--channel" => {
                let v = args.get(i).cloned().ok_or_else(|| {
                    AppError::invalid_argument("--channel 后面要跟 stable / preview")
                })?;
                i += 1;
                channel = Some(parse_channel(&v)?);
            }
            "--preview" => channel = Some(UpdateChannel::Preview),
            "--stable" => channel = Some(UpdateChannel::Stable),
            "--stop-after" => {
                let v = args.get(i).cloned().ok_or_else(|| {
                    AppError::invalid_argument(
                        "--stop-after 后面要跟 freeze / staging / manifest / seal",
                    )
                })?;
                i += 1;
                stop_after = Some(parse_stop(&v)?);
            }
            "--platform" => {
                let v = args.get(i).cloned().ok_or_else(|| {
                    AppError::invalid_argument("--platform 后面要跟 macos / windows")
                })?;
                i += 1;
                only_platform = Some(v);
            }
            _ => {
                if let Some(v) = a.strip_prefix("--version=") {
                    version = Some(v.to_owned());
                } else if let Some(v) = a.strip_prefix("--channel=") {
                    channel = Some(parse_channel(v)?);
                } else if let Some(v) = a.strip_prefix("--stop-after=") {
                    stop_after = Some(parse_stop(v)?);
                } else if let Some(v) = a.strip_prefix("--platform=") {
                    only_platform = Some(v.to_owned());
                } else if let Some(v) = a.strip_prefix("--notes=") {
                    notes = v.to_owned();
                } else if !a.starts_with('-') {
                    /* 位置参数：`<版本> [说明]`（与旧命令同一手感） */
                    if version.is_none() {
                        version = Some(a.clone());
                    } else if notes.is_empty() {
                        notes = a.clone();
                    } else {
                        return Err(AppError::invalid_argument(format!("多出来的参数：{a}")));
                    }
                } else {
                    return Err(AppError::invalid_argument(format!("认不出的参数：{a}")));
                }
            }
        }
    }

    Ok(Parsed {
        mode,
        args_only,
        options: ReleaseOptions {
            version,
            notes,
            channel,
            dry_run,
            confirmed,
            stop_after,
            only_platform,
        },
    })
}

fn parse_channel(raw: &str) -> Result<UpdateChannel, AppError> {
    UpdateChannel::parse(raw).ok_or_else(|| {
        AppError::invalid_argument(format!("认不出的通道：{raw}（只认 stable / preview）"))
    })
}

fn parse_stop(raw: &str) -> Result<StopAfter, AppError> {
    match raw.trim() {
        "freeze" => Ok(StopAfter::Freeze),
        "staging" => Ok(StopAfter::Staging),
        "manifest" => Ok(StopAfter::Manifest),
        "seal" => Ok(StopAfter::Seal),
        other => Err(AppError::invalid_argument(format!(
            "认不出的停点：{other}（freeze / staging / manifest / seal）"
        ))),
    }
}

/// 人看的那一份：阶段 + 平台 × 源矩阵 + 这一趟做了什么。
fn human(r: &ReleaseReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "\n{} {} · {}\n",
        r.version,
        channel_label(r.channel),
        r.phase.label()
    ));
    out.push_str(&format!(
        "  冻结源码：{}\n",
        r.source_commit
            .as_deref()
            .map(|s| s.chars().take(7).collect::<String>())
            .unwrap_or_else(|| "（还没冻结）".to_owned())
    ));
    out.push_str(&format!(
        "  正式 tag：{}\n  暂存 tag：{}\n",
        r.tag, r.staging_tag
    ));
    out.push_str("\n  平台            Gitee  GitHub  阶段\n");
    for p in &r.platforms {
        out.push_str(&format!(
            "  {:<14} {:<6} {:<8} {}\n",
            p.label,
            if p.gitee_done { "齐" } else { "缺" },
            if p.github_done { "齐" } else { "缺" },
            p.stage.label()
        ));
    }
    if !r.conflicts.is_empty() {
        out.push_str("\n  冲突：\n");
        for c in &r.conflicts {
            out.push_str(&format!("    ✗ {c}\n"));
        }
    }
    if !r.seal_blockers.is_empty() {
        out.push_str("\n  封版判据：\n");
        for c in &r.seal_blockers {
            out.push_str(&format!("    · {c}\n"));
        }
    }
    if !r.summary.is_empty() {
        out.push_str("\n  这一趟：\n");
        for s in &r.summary {
            out.push_str(&format!("    · {s}\n"));
        }
    }
    out.push_str(&format!("\n  下一步：{}\n", r.next));
    out
}

fn channel_label(c: UpdateChannel) -> &'static str {
    match c {
        UpdateChannel::Stable => "正式版",
        UpdateChannel::Preview => "预览版",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(v: &[&str]) -> Parsed {
        parse(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>()).expect("参数应能解析")
    }

    #[test]
    fn channel_and_stop_points_are_parsed() {
        let p = a(&["--channel", "preview", "--stop-after", "staging"]);
        assert_eq!(p.options.channel, Some(UpdateChannel::Preview));
        assert_eq!(p.options.stop_after, Some(StopAfter::Staging));
    }

    #[test]
    fn channel_must_be_known() {
        let err = parse(&["--channel".to_owned(), "beta".to_owned()]).unwrap_err();
        assert!(err.message.contains("stable"));
    }

    #[test]
    fn stop_point_must_be_known() {
        let err = parse(&["--stop-after".to_owned(), "half".to_owned()]).unwrap_err();
        assert!(err.message.contains("freeze"));
    }

    #[test]
    fn a_positional_version_and_note_are_accepted() {
        let p = a(&["0.0.16", "修了一处"]);
        assert_eq!(p.options.version.as_deref(), Some("0.0.16"));
        assert_eq!(p.options.notes, "修了一处");
    }

    #[test]
    fn args_only_does_not_require_confirmation() {
        let p = a(&["--args-only", "--channel", "stable"]);
        assert!(p.args_only);
        assert!(!p.options.confirmed);
    }

    #[test]
    fn check_mode_is_read_only() {
        let p = a(&["--check"]);
        assert_eq!(p.mode, Mode::Check);
        assert!(!p.options.dry_run, "dry_run 在 run_cli 里按 mode 补");
    }

    #[test]
    fn unknown_flags_are_refused() {
        let err = parse(&["--force".to_owned()]).unwrap_err();
        assert!(err.message.contains("认不出"));
    }
}
