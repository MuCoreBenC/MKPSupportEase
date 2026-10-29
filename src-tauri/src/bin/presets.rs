//! **预设工作流 CLI** —— 给人和 AI 用的正式入口，与工作台**同一个内核**。
//!
//! ```text
//! presets check                 检查 TOML / 引用 / 交付约束（只读）
//! presets generate [--all]      跑生成器，刷新交付产物与目录（**不推进发布序号**）
//! presets diff                  报"盘上 vs 真生成会写成什么"（只读）
//! presets verify                核对 catalog / manifest / 账本 / SHA 一致性（只读）
//! presets publish --dry-run     预演正式发布：说出会写成第几号，**一个字节都不写**
//! presets publish               执行正式发布事务（**唯一能让序号 +1 的动作**）
//! ```
//!
//! # 为什么要有它（2026-10-28 作者裁定）
//!
//! 改几个参数不该先开工作台跑一遍完整发布。所以把三件事分开：
//!
//! | 动作 | 谁做 | 推进 `publishSerial` 吗 |
//! |---|---|---|
//! | 改源 TOML（人 / AI 直接改、CLI 不需要参与） | 编辑器 | 否 |
//! | 生成目录（`presets generate`） | 本 CLI | **否** |
//! | 正式发布（`presets publish`） | 本 CLI 的发布事务 | **是**（且只有这里能） |
//!
//! 于是"AI 忘了生成"或"生成完就以为发了"都不会让目录宣称一个没发出去的新版本。
//!
//! # 它**不是**第二套实现
//!
//! 每一行业务都在 `workbench::app`: `audit_with` / `generate_with` / `plan_publish` /
//! `publish_into` / `publish_tx::run` / `publish_book`。这里只做：解析参数、开会话、
//! 调那一处、把结果按**一套固定的 JSON** 打出来。
//!
//! # 输出约定（与 `bin/release.rs` 同一条）
//!
//! - **stdout**：一行 JSON（给脚本 / AI 解析）
//! - **stderr**：人话摘要
//! - **退出码**：`0` 成功 / `1` 执行失败或校验未通过 / `2` 发布闸拦下（要人处理）
//!
//! ★ "有差异"**不是**失败：`diff` 发现改了东西照样退 0，差异由 JSON 的 `changed` 字段说。
//!   退出码只表达"这次操作本身成不成"。

use std::path::PathBuf;

use serde_json::json;

use mkp_support_ease_lib::error::AppError;
use mkp_support_ease_lib::workbench::app::{self as app_pkg, publish_book, Ctx};

fn main() {
    let code = match run_cli() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("✗ {}", e.message);
            if let Some(d) = &e.detail {
                eprintln!("  {d}");
            }
            /*
             * 退出码分档：闸门拦下（要人处理）与执行失败不是一回事 ——
             * 脚本据此决定"改参数重试"还是"先去修数据"。
             */
            match e.code {
                mkp_support_ease_lib::error::ErrorCode::InvalidArgument => 2,
                _ => 1,
            }
        }
    };
    std::process::exit(code);
}

fn run_cli() -> Result<i32, AppError> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first().cloned() else {
        usage();
        return Ok(2);
    };
    let rest = &args[1..];
    let all = rest.iter().any(|a| a == "--all");
    let dry_run = rest.iter().any(|a| a == "--dry-run");

    match cmd.as_str() {
        "check" => cmd_check(),
        "generate" => cmd_generate(all),
        "diff" => cmd_diff(),
        "verify" => cmd_verify(),
        "publish" => cmd_publish(dry_run),
        "-h" | "--help" | "help" => {
            usage();
            Ok(0)
        }
        other => {
            eprintln!("✗ 不认识的命令：{other}");
            usage();
            Ok(2)
        }
    }
}

fn usage() {
    eprintln!(
        "用法：
  presets check                 检查源 TOML / 引用 / 交付约束（只读）
  presets generate [--all]      跑生成器刷新交付产物（**不推进发布序号**）
  presets diff                  报盘上与真生成结果的差异（只读）
  presets verify                核对 catalog / manifest / 账本 / SHA（只读）
  presets publish --dry-run     预演发布：说会写成第几号，一个字节都不写
  presets publish               执行正式发布事务（唯一能让序号 +1 的动作）

退出码：0 成功 · 1 执行失败 / 校验未通过 · 2 闸门拦下（要人处理）"
    );
}

/// 输出一份 JSON 结果并给出退出码。
///
/// 所有命令共用这一个出口 —— **不许每个命令自己发明一套 JSON**（作者明确要求）。
/// 字段固定为：`command` / `ok` / `changed` / `summary` + 各命令自己的 `detail`。
fn emit(value: serde_json::Value, code: i32) -> Result<i32, AppError> {
    match serde_json::to_string(&value) {
        Ok(json) => println!("{json}"),
        Err(e) => {
            eprintln!("结果序列化失败：{e}");
            return Ok(1);
        }
    }
    Ok(code)
}

fn open_ctx() -> Result<Ctx, AppError> {
    Ctx::open()
}

/// 交付根（`<presets>/delivery`）—— 与会话同一棵数据树。
fn delivery_root(ctx: &Ctx) -> PathBuf {
    mkp_support_ease_lib::workbench::paths::delivery_root_at(ctx.presets.root())
}

/// **检查**：跑发布闸（只读那个入口），把红项列出来。**不写盘**。
fn cmd_check() -> Result<i32, AppError> {
    let ctx = open_ctx()?;
    let audit = app_pkg::audit_check(&ctx)?;
    let failed: Vec<serde_json::Value> = audit
        .items
        .iter()
        .filter(|i| i.status == app_pkg::AuditStatus::Fail)
        .map(|i| {
            json!({
                "id": i.id,
                "name": i.name,
                "detail": i.details,
                "files": i.affected_files,
            })
        })
        .collect();
    let passed = audit
        .items
        .iter()
        .filter(|i| i.status == app_pkg::AuditStatus::Pass)
        .count();
    let ok = audit.can_publish;
    eprintln!(
        "{}：{} 项通过 / {} 项红",
        if ok {
            "✓ 检查通过"
        } else {
            "✗ 有阻断项"
        },
        passed,
        failed.len()
    );
    emit(
        json!({
            "command": "check",
            "ok": ok,
            "changed": false,
            "checksPassed": passed,
            "blockers": failed,
            "summary": if ok { "检查通过" } else { "有阻断项，不能发布" },
        }),
        if ok { 0 } else { 2 },
    )
}

/// **生成**：跑生成器刷新交付产物与目录。**不推进发布序号**。
fn cmd_generate(all: bool) -> Result<i32, AppError> {
    let mut ctx = open_ctx()?;
    let scope = if all {
        app_pkg::Scope::All
    } else {
        app_pkg::Scope::Stale
    };
    let report = app_pkg::generate(&mut ctx, &scope)?;
    // 生成之后重算目录（与工作台同一条：内容与目录不许各说一代）
    let root = delivery_root(&ctx);
    let written = report.written.len();
    let unchanged = report.unchanged.len();
    let serial = publish_book::load(&root)?.map(|b| b.serial);
    eprintln!(
        "✓ 生成完成：新写 {written} 份、未变 {unchanged} 份。发布序号仍是 {}（生成不推进序号）",
        serial
            .map(|s| s.to_string())
            .unwrap_or_else(|| "未发布".to_owned())
    );
    emit(
        json!({
            "command": "generate",
            "ok": true,
            // 写了产物 = 盘上内容变了；调用方据此决定要不要接着 verify / publish
            "changed": written > 0,
            "written": report.written,
            "unchanged": report.unchanged,
            "skipped": report.skipped,
            "publishSerial": serial,
            "summary": format!("新写 {written} 份、未变 {unchanged} 份；序号未推进"),
        }),
        0,
    )
}

/// **差异**：盘上 vs 真生成会写出什么。**只读**。
fn cmd_diff() -> Result<i32, AppError> {
    let ctx = open_ctx()?;
    let preview = app_pkg::generate_preview(&ctx)?;
    let changed = !preview.files.is_empty();
    eprintln!(
        "{}：{} 份会变",
        if changed {
            "有差异"
        } else {
            "✓ 无差异"
        },
        preview.files.len()
    );
    /*
     * ★ **有差异不是失败**（作者要求）：diff 的职责是如实报出来，
     *   退出码仍然是 0。只有跑不动才是非零。
     */
    emit(
        json!({
            "command": "diff",
            "ok": true,
            "changed": changed,
            "files": preview.files.iter().map(|f| json!({
                "file": f.file_name,
                "state": f.state,
            })).collect::<Vec<_>>(),
            "summary": format!("{} 份会变", preview.files.len()),
        }),
        0,
    )
}

/// **验证**：catalog / manifest / 账本 / SHA 一致性。**只读**。
fn cmd_verify() -> Result<i32, AppError> {
    let ctx = open_ctx()?;
    let root = delivery_root(&ctx);
    let report = app_pkg::verify_delivery(&root, ctx.presets.root())?;
    let problems: Vec<String> = report.problems.clone();
    eprintln!(
        "{}：核对 {} 份文件",
        if problems.is_empty() {
            "✓ 一致"
        } else {
            "✗ 不一致"
        },
        report.files_checked
    );
    for p in &problems {
        eprintln!("  · {p}");
    }
    emit(
        json!({
            "command": "verify",
            "ok": problems.is_empty(),
            "changed": false,
            "filesChecked": report.files_checked,
            "publishSerial": report.serial,
            "catalogId": report.catalog_id,
            "problems": problems,
            "summary": if report.problems.is_empty() {
                "catalog / manifest / 账本 / SHA 全部一致".to_owned()
            } else {
                format!("{} 处不一致", report.problems.len())
            },
        }),
        if report.problems.is_empty() { 0 } else { 1 },
    )
}

/// **发布**：`--dry-run` 只说不写；不带就走正式事务。
fn cmd_publish(dry_run: bool) -> Result<i32, AppError> {
    let mut ctx = open_ctx()?;
    /*
     * ★ 发布事务用 `TxOptions::default()` 的 dry_run 开关 —— **与工作台同一个字段**，
     *   于是"预演"在工作台、CLI 两处是同一段代码判的（不是两套开关）。
     */
    let opts = app_pkg::TxOptions {
        dry_run,
        ..Default::default()
    };
    let target = app_pkg::resolve_target_opt(&ctx)?;
    let hosting = app_pkg::hosting_for(&target)?;
    let report = app_pkg::publish_tx(&mut ctx, &opts, target.as_ref(), hosting.as_deref(), None)?;

    eprintln!("{}", report.summary);
    let ok = matches!(
        report.stage,
        app_pkg::PublishStage::Committed
            | app_pkg::PublishStage::Pushed
            | app_pkg::PublishStage::ReviewOpened
            | app_pkg::PublishStage::StatusRead
            | app_pkg::PublishStage::Generated
    ) && !(report.stage == app_pkg::PublishStage::Generated && !dry_run);
    let code = if report.stage == app_pkg::PublishStage::BlockedAudit {
        2
    } else if ok {
        0
    } else {
        1
    };
    emit(
        json!({
            "command": "publish",
            "dryRun": dry_run,
            "ok": ok,
            "changed": !dry_run && report.stage != app_pkg::PublishStage::Generated,
            "stage": report.stage,
            "publishSerial": report.publish_serial,
            "catalogId": report.catalog_id,
            "revision": report.revision,
            "commit": report.commit,
            "files": report.files,
            "summary": report.summary,
        }),
        code,
    )
}
