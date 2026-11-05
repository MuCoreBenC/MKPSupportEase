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
    /* 内容一字未改时仍然重发（会推进序号）—— 默认不发，见 `TxOptions::republish` */
    let republish = rest.iter().any(|a| a == "--republish");
    /* **人类确认位**（2026-11-05 硬约束）：真发布缺 `--confirm` 会被内核零副作用拒绝。
     * 只有人当次明确许可，AI / CI 才拿得到这个 flag。dry-run 不需要。 */
    let confirmed = rest.iter().any(|a| a == "--confirm");
    /* `reload --force`：确认丢弃影子树里未保存的编辑（2026-11-01） */
    let force = rest.iter().any(|a| a == "--force");

    match cmd.as_str() {
        /* 这一条**不读任何文件、不建任何目录**：它只回答「我这个二进制是哪次编的」。
        用来分辨 `target/debug` 与 `target-workbench/debug` 里哪一份是最新的，
        以及「编了但没重启」时窗里跑的是不是旧字节。 */
        "build" => {
            println!("{}", mkp_support_ease_lib::build_fingerprint());
            Ok(0)
        }
        "check" => cmd_check(),
        "generate" => cmd_generate(all),
        "diff" => cmd_diff(),
        "verify" => cmd_verify(),
        "reload" => cmd_reload(force),
        "publish" => cmd_publish(dry_run, republish, confirmed),
        "normalize" => cmd_normalize(dry_run),
        "follows-migrate" => cmd_follows_migrate(dry_run),
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
  presets build                 说清这个二进制是哪次编译的（只读，不碰数据）
  presets check                 检查源 TOML / 引用 / 交付约束（只读）
  presets generate [--all]      跑生成器刷新交付产物（**不推进发布序号**）
  presets diff                  报盘上与真生成结果的差异（只读）
  presets verify                核对 catalog / manifest / 账本 / SHA（只读）
  presets reload                把工作台影子树对齐到 presets/（= 工作台「重新加载」）
  presets normalize --dry-run   预演源 TOML 重排：说差多少，一个字节都不写
  presets normalize             把 presets/registry/param_registry.toml 重排成机械序
  presets follows-migrate --dry-run  预演「跟随状态」迁移：说会有几条，不写任何文件
  presets follows-migrate       把存量版本键补成显式的「不跟随」+ 写迁移台账
  presets publish --dry-run     预演发布：说会写成第几号，一个字节都不写（不需要确认）
  presets publish --confirm     执行正式发布事务（唯一能让序号 +1 的动作）
                                ★ 真发布必须人当次明确许可；缺 --confirm 直接拒绝

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

/// **把工作台的影子树对齐到真仓库**（= 工作台那颗「重新加载」）。
///
/// 为什么需要它：影子与 `presets/` 分叉时生成会被拒（`ui.workbench.txn.diverged`）——
/// 那条闸故意**不自动对齐**，因为影子可能含着用户还没保存的编辑。CLI 这边要往下走，
/// 就得显式说一句"放弃影子里的编辑态，回到磁盘那一版"。
/// 与 `wb_reload` 是同一件事（`txn::mirror(real → shadow)`），不是第二套逻辑。
fn cmd_reload(force: bool) -> Result<i32, AppError> {
    let real = mkp_support_ease_lib::workbench::paths::presets_root()
        .ok_or_else(|| AppError::not_found("找不到 <repo>/presets"))?;
    let wb = mkp_support_ease_lib::workbench::paths::real_workbench_root();
    let shadow = mkp_support_ease_lib::workbench::domain::txn::shadow_root(&wb);

    /* **有未保存草稿时要先确认**（2026-11-01）。
    以前这条下去就丢，谁点一下用户的编辑就没了。`--force` 才是"我知道，丢吧"。
    注意只拦"影子里有、真源里没有"的那种（用户的草稿）；
    外部改了 presets/ 时影子是落后的，那不算草稿，直接对齐就是正确动作。 */
    let edited = mkp_support_ease_lib::workbench::domain::txn::snapshot(&shadow)
        .ok()
        .zip(mkp_support_ease_lib::workbench::domain::txn::snapshot(&real).ok())
        .map_or(0, |(sh, rl)| sh.edited_against(&rl));
    if edited > 0 && !force {
        let msg = postprocess::diag::messages::text(
            "ui.workbench.reload.drafts",
            &[("count", &edited.to_string())],
        );
        let _ = emit(
            json!({
                "command": "reload",
                "ok": false,
                "changed": false,
                "drafts": edited,
                "error": msg,
                "summary": format!("影子树里有 {edited} 处未保存的改动；确认要丢弃请加 --force"),
            }),
            1,
        );
        return Ok(1);
    }

    std::fs::create_dir_all(&shadow)
        .map_err(|e| AppError::io("建不出影子树").with_detail(e.to_string()))?;
    mkp_support_ease_lib::workbench::domain::txn::mirror(&real, &shadow)?;
    /* 基准也一起铺（与 `Ctx::open` 那条同一条语义）：铺影子 = 记下"从哪一份真源铺的"。
    不铺它，下次判"有没有草稿"就失去锚点，会把用户改过的既有文件当成没草稿。 */
    let base = mkp_support_ease_lib::workbench::domain::txn::base_root(&wb);
    std::fs::create_dir_all(&base)
        .map_err(|e| AppError::io("建不出基准快照目录").with_detail(e.to_string()))?;
    mkp_support_ease_lib::workbench::domain::txn::mirror(&real, &base)?;
    emit(
        json!({
            "command": "reload",
            "ok": true,
            "changed": true,
            "drafts": edited,
            "summary": if edited > 0 {
                format!("影子树已对齐到 presets/（已丢弃 {edited} 处未保存的改动）")
            } else {
                "影子树已对齐到 presets/".to_owned()
            },
        }),
        0,
    )
}

/// 交付根（`<presets>/delivery`）—— **真仓库那一棵**（不是影子树）。
///
/// ★ 2026-10-31：原来用 `ctx.presets.root()`（= `workbench/.draft/presets/`），
/// 于是 `verify` 核对的是影子里的交付目录 —— 发布把账本写进影子、真仓库还停在旧账本时，
/// 它照样报「✓ 一致」，而 `check:artifacts`（读真实 git 树）红。两边必须看同一棵树。
fn delivery_root(ctx: &Ctx) -> PathBuf {
    mkp_support_ease_lib::workbench::paths::delivery_root_at(ctx.real_root())
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

/// **源 TOML 的机械化重排**（2026-11-03 作者拍板）。
///
/// # 为什么单独一条命令，而不是放进 `generate`
///
/// `generate` 只碰**产物**（`presets/delivery/` 与 `crates/preset/assets/`），
/// **从不改源**。重排源文件是另一件事：它会把整份 `param_registry.toml` 重写一遍，
/// 那个 diff 必须单独成一次提交，否则 review 时分不清"格式变了"与"数据变了"。
///
/// # 闸：结构无损才写
///
/// 重排后按**同一个解析器**再读一遍，与原文逐字段比（[`normalize::check_roundtrip`]）。
/// 比不过就一个字节都不写 —— 注释、引号、顺序允许变，数据不许变。
fn cmd_normalize(dry_run: bool) -> Result<i32, AppError> {
    let ctx = open_ctx()?;
    let file = ctx.real_root().join("registry").join("param_registry.toml");
    let src = std::fs::read_to_string(&file).map_err(|e| {
        AppError::not_found(format!("读不到 {}", file.display())).with_detail(e.to_string())
    })?;

    let out = mkp_support_ease_lib::presetdata::normalize::normalize_registry(&src)?;
    let report = mkp_support_ease_lib::presetdata::normalize::check_roundtrip(&src, &out)?;

    /* 结构化数据的闸：不通过就停在这里，别写 */
    if !report.ok {
        eprintln!("✗ 重排会改变数据，拒绝写入：");
        for p in &report.problems {
            eprintln!("  · {p}");
        }
        return emit(
            json!({
                "command": "normalize",
                "ok": false,
                "changed": false,
                "problems": report.problems,
                "summary": "重排会改变结构化数据，已拒绝写盘",
            }),
            2,
        );
    }

    let changed = out != src;
    if dry_run {
        eprintln!(
            "{}（{} 个参数 / {} 个分组，结构无损）：",
            if changed {
                "会有改动"
            } else {
                "已经是最新形态"
            },
            report.params,
            report.tabs
        );
        if changed {
            /* 不打整份 diff（三千多行），只报行数与首位差异 —— 真要看得自己跑 diff */
            let a: Vec<&str> = src.lines().collect();
            let b: Vec<&str> = out.lines().collect();
            eprintln!("  原文 {} 行 → 重排后 {} 行", a.len(), b.len());
        }
        return emit(
            json!({
                "command": "normalize",
                "ok": true,
                "changed": changed,
                "params": report.params,
                "tabs": report.tabs,
                "summary": if changed { "会重排（未写盘）" } else { "已是最新形态" },
            }),
            0,
        );
    }

    if changed {
        mkp_support_ease_lib::fsx::atomic::atomic_write(&file, out.as_bytes())?;
        eprintln!("✓ 已重排 {}", file.display());
    } else {
        eprintln!("✓ 已经是最新形态，未写盘");
    }
    emit(
        json!({
            "command": "normalize",
            "ok": true,
            "changed": changed,
            "params": report.params,
            "tabs": report.tabs,
            "summary": if changed { "已重排" } else { "已是最新形态" },
        }),
        0,
    )
}

/// **一次性的「跟随状态」迁移**（2026-11-03）。
///
/// # 为什么要有它
///
/// 2026-11-03 之前，"这一版跟不跟基底"没有写在文件里 —— 靠"`machineVariants`
/// 里有没有 `机型:版本` 那一行"推断。现在它是 `[params.follows]` 里的一句
/// `= false`。存量文件里没有这句话，含义就变成了"全部跟随"，那会让今天显示
/// 「不跟随」的格子当场变脸。
///
/// 所以迁移做的就是**把推断变成写下来的状态**：有版本行 ⇒ `= false`（不跟随），
/// 没有 ⇒ 什么都不写（跟随）。一格的表现都不变。
///
/// # 台账
///
/// `presets/registry/follows-migration.json` 记「值恰好等于基底」的那些格：
/// 它们当初是自己定的还是跟随过来的，已经无法证明。记下来是为了日后能回答
/// 「为什么这一版不跟随」，不改判定。
///
/// # 幂等
///
/// 只补没有状态记录的键，第二遍是空操作。
fn cmd_follows_migrate(dry_run: bool) -> Result<i32, AppError> {
    let ctx = open_ctx()?;
    let root = ctx.real_root().to_path_buf();
    let mut p = mkp_support_ease_lib::presetdata::Presets::load_from(&root)?;
    let before = p.registry.to_toml();

    let at = mkp_support_ease_lib::workbench::clock::now_iso8601();
    let ledger = mkp_support_ease_lib::presetdata::follows::migrate(&mut p.registry, &at)?;

    let after = p.registry.to_toml();
    let changed = before != after;
    let mut total_versions = 0usize;
    for param in p.registry.params() {
        total_versions += param.follows.len();
    }

    if dry_run {
        eprintln!(
            "{}：会给 {} 个版本键写上「不跟随」（其中 {} 个「值恰好等于基底」，进台账）",
            if changed {
                "会有改动"
            } else {
                "已经是迁移后的形态"
            },
            ledger.marked,
            ledger.ambiguous.len()
        );
        return emit(
            json!({
                "command": "follows-migrate",
                "ok": true,
                "changed": changed,
                "marked": ledger.marked,
                "ambiguous": ledger.ambiguous.len(),
                "followsEntries": total_versions,
                "summary": if changed { "会写迁移结果（未写盘）" } else { "已经迁过，空操作" },
            }),
            0,
        );
    }

    if changed {
        p.registry.write_back()?;
    }
    let ledger_file = root.join("registry").join("follows-migration.json");
    if changed {
        let text = serde_json::to_string_pretty(&ledger).unwrap_or_else(|_| "{}".to_owned());
        mkp_support_ease_lib::fsx::atomic::atomic_write(
            &ledger_file,
            format!("{text}\n").as_bytes(),
        )?;
        eprintln!(
            "✓ 迁完：{} 个版本键标成「不跟随」，台账 {} 条 → {}",
            ledger.marked,
            ledger.ambiguous.len(),
            ledger_file.display()
        );
    } else {
        eprintln!("✓ 已经迁过，未写盘");
    }
    emit(
        json!({
            "command": "follows-migrate",
            "ok": true,
            "changed": changed,
            "marked": ledger.marked,
            "ambiguous": ledger.ambiguous.len(),
            "followsEntries": total_versions,
            "summary": if changed { "已迁移" } else { "已经迁过" },
        }),
        0,
    )
}

/// **发布**：`--dry-run` 只说不写；不带就走正式事务。
fn cmd_publish(dry_run: bool, republish: bool, confirmed: bool) -> Result<i32, AppError> {
    let mut ctx = open_ctx()?;
    /*
     * ★ 发布事务用 `TxOptions::default()` 的 dry_run / republish 开关 —— **与工作台同一个
     *   字段**，于是"预演""重发"在 CLI 与工作台两处是同一段代码判的（不是两套开关）。
     */
    let opts = app_pkg::TxOptions {
        dry_run,
        republish,
        confirmed,
        ..Default::default()
    };
    let target = app_pkg::resolve_target_opt(&ctx)?;
    let hosting = app_pkg::hosting_for(&target)?;
    /* 与工作台**同一个应用数据根**，好让 CLI 这一趟也进「发布历史」面板。
     * `internal_root_headless` 在没窗口时按 APP_IDENTIFIER 认门；取不到就跳过记账
     * （不把一次成功的发布说成失败）。 */
    let history_root = mkp_support_ease_lib::fsx::paths::internal_root_headless();
    let report = app_pkg::publish_tx(
        &mut ctx,
        &opts,
        target.as_ref(),
        hosting.as_deref(),
        None,
        history_root.as_deref(),
    )?;

    eprintln!("{}", report.summary);
    /*
     * `NoChange` 算**成功**：这一趟的结论就是"没有可发的变化"，不是失败 ——
     * 脚本拿退出码判断时会把它当异常，那就本末倒置了。
     */
    let ok = matches!(
        report.stage,
        app_pkg::PublishStage::Committed
            | app_pkg::PublishStage::Pushed
            | app_pkg::PublishStage::ReviewOpened
            | app_pkg::PublishStage::StatusRead
            | app_pkg::PublishStage::NoChange
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
            /* 只有真写出去了才算 changed —— `NoChange` 与预演都不算 */
            "changed": !dry_run
                && matches!(
                    report.stage,
                    app_pkg::PublishStage::Committed
                        | app_pkg::PublishStage::Pushed
                        | app_pkg::PublishStage::ReviewOpened
                        | app_pkg::PublishStage::StatusRead
                ),
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
