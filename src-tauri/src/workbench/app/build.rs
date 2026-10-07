//! 生成 / 校验 / 恢复 / 发布（doc §10）。
//!
//! # 生成是原子的
//!
//! **先把全部产物算完，任一项算不出来则整批不动**；全部成功才逐个原子替换。
//! 半批生成的后果是最难查的一种：delivery 里有一半新一半旧，而两边都是合法的 TOML，
//! 客户端下载下来也不会报错，只是行为对不上。
//!
//! # 字节没变不重写文件
//!
//! `mtime` 变了会让同步工具以为有更新。所以比较的是**内容**。
//!
//! 但这里有个坑：产物头部有 `release_time`，它每次都不一样，于是"字节比较"永远不相等，
//! 这条规则就永远不生效。所以比较时**跳过那一行**（[`same_payload`]）。
//! 生成路径的「跳过」与预演的「无变化」（[`preview_one`]）都认这一道判据 ——
//! 两边各说各话，「将写入 N 份」就是假的。
//!
//! # 产物头没有 uuid（2026-10-05 删，别加回来）
//!
//! 头注释曾写过一行 `# uuid:`（旧系统防"用户复制与官方冲突"的遗产）。当日审计查证：
//! 全仓没有任何代码读它，公开渠道也找不到读它的消费方 —— 官方/用户的分界由
//! **角色目录与血统三行**承担，完整性与版本由 **SHA 与 release_time** 承担。
//! 一行没有读者的字段就是假字段。有过期判定需求的是**工作台**，用的是
//! **两层指纹**（定义 / 值分开算，见 `presetdata::resolve::Layers::fingerprint`）——
//! 它只进 `built.json` 当「待更新」判据，**不进产物**。
//! 历史基线那 9 份（`crates/preset` 一侧）头里还带着 uuid，那是历史事实，不是兼容壳。
//!
//! # TOML 的形状照上游的真产物
//!
//! 实测 `presets/mkp/A1.toml`（上游头里有 uuid，我们**不写**那一行 —— 见上一节）：
//!
//! ```toml
//! # release_time: 2026-08-19 01:38:13
//! # machine: A1
//! # variant: standard
//! #胶笔配置
//!
//! [toolhead]
//! offset = { x = -1, y = 18.6, z = 4 } # 笔尖偏移
//! speed_limit = 70 # 涂胶速度限制 (mm/s)
//! # 自定义工具头获取 G-code
//! custom_mount_gcode = """
//! G92 E0
//! """
//! ```
//!
//! 两条规则是从数据里读出来的，不是写死的：
//!
//! - **段名** = `param.section`（实测只有 `toolhead` 与 `wiping` 两个）
//! - **几个参数共享同一个 `tomlKey` 时合成内联表**，成员名取 `jsonKey`
//!   （`toolhead.offset.x/y/z` 的 `tomlKey` 都是 `offset`，`jsonKey` 分别是 `x/y/z`）
//!
//! **不追求与上游产物逐字节相同**：那是另一个 builder 生成的，键序也不一致。
//! 要的是「合法 TOML + 值对 + 同样的输入产出同样的字节」。
//!
//! # 恢复配方走唯一写入口
//!
//! `wb_revert_preview` 只**算**出要提交哪些 patch，前端拿着它走 `wb_apply_draft`。
//! 这样恢复也是一条撤销、也进同一份差异清单 —— 而不是第二条写路径。
//!
//! 快照里只有有效值，看不出当时是哪一层给的，所以恢复一律写在**版本**这一层。
//! 原本继承来的那几项会从此脱钩，**名单当场列出来，不许闷着改**。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::AppError;
use crate::ipc::traced;
use crate::workbench::clock;
use crate::workbench::domain::derive::Book;
use crate::workbench::domain::issues::{self, Report};
use crate::workbench::domain::patch::{BuiltRecord, Patch};
use crate::workbench::domain::wording as w;
use crate::workbench::domain::Level;
use crate::workbench::paths;
use crate::workbench::presets::registry::{ParamDef, UiComponent, ValueType};

use super::delivery::DeliveryStage;
use super::{state, with_ctx, with_ctx_mut};

/// 发布渠道。原先是上游 manifest 的 `compat.channel`；上游整层删掉后，
/// 工作台自己发的是正式渠道，定成常量
const PUBLISH_CHANNEL: &str = "stable";

/* ---------- 校验 ---------- */

/// 预检（b05 Task 11.8）：[`issues::inspect`] 的全部 + 清单 ↔ 配方对齐。
///
/// 配方正文直接取 `preset::PRESET_RECIPES_TOML`（编进二进制的真源，与 `gen-presets`
/// 咬同一份）。读不回来不报错 —— 它变成报告里的**一条**，其余检查照跑；
/// 让预检整个失败等于把「数据坏了」变成「工具坏了」。
///
/// 生成闸门（[`issues::inspect`]，`wb_generate` 里那道）**刻意不含**配方对齐：
/// 生成读参数注册表，不读配方，对不上不影响工作台的产物（见 `issues.rs` 那边的说明）。
#[tauri::command(async)]
pub fn wb_preflight() -> Result<Report, AppError> {
    traced("wb_preflight", |_| {
        with_ctx(|ctx| {
            let (c, d, _) = state(ctx)?;
            let book = Book::new(&ctx.presets, &c, &d);
            let recipe = preset::recipe::Recipe::parse(preset::PRESET_RECIPES_TOML)
                .map_err(|e| e.to_string());
            let recipe_ref = recipe.as_ref().map_err(String::as_str);
            // 交付目录的「清单 ↔ 文件」自查在 app 层做（要摸盘），结果作为一条交给预检
            let delivery = super::delivery::audit_catalog(
                &crate::workbench::paths::delivery_root_path(),
                &book,
            );
            // 内嵌目录（编译进安装包的那份）跟仓库数据对不对得上也是盘上的事实 ——
            // 同一条路交给预检：坏了自己成为报告里的一条，不让整个预检失败
            let embedded = audit_embedded_catalog();
            Ok(issues::preflight(&book, recipe_ref, delivery, embedded))
        })
    })
}

/// 内嵌目录自查：`catalog.generated.json`（编译进二进制、客户端首启释放的那份）
/// 与当前仓库重新构建出来的那份是否**逐字节一致**。
///
/// 之前只有一条 Rust 单测（`runtime::tests::embedded_matches_rebuild`）盯着 ——
/// 红在 `cargo test` 里，检查页上没人提。现在预检把这件事摆上桌面：
/// 改了 presets/（或交付产物）没跑 `cargo run --bin gen-catalog`，装出来的
/// 客户端首屏拿的还是旧数据 —— 这是发布前该知道的事。
fn audit_embedded_catalog() -> Result<(), String> {
    let repo = crate::workbench::paths::repo_root();
    let catalog = crate::runtime::catalog::Catalog::build_from_repo(&repo)
        .map_err(|e| format!("重新构建内嵌目录失败：{}", e.message))?;
    let json = catalog
        .to_pretty_json()
        .map_err(|e| format!("内嵌目录序列化失败：{}", e.message))?;
    if json.as_bytes() == crate::runtime::EMBEDDED_CATALOG {
        return Ok(());
    }
    Err("安装包里编译的那份，与按当前 presets/ 重新构建出来的不一致".to_owned())
}

/* ---------- 渲染 ---------- */

/// 一份产物的文本 + 它的指纹。指纹用来判「要不要重写」与「产物过期没有」
pub struct Rendered {
    pub uid: String,
    pub file_name: String,
    pub text: String,
    pub fingerprint: String,
    /// 进快照的那一份有效值
    pub snapshot: BTreeMap<String, Value>,
}

/// 把一个版本的有效配方渲染成 MKP TOML
fn render(book: &Book<'_>, uid: &str) -> Result<Rendered, AppError> {
    let v = book
        .version(uid)
        .ok_or_else(|| AppError::not_found(format!("版本 {uid} 不存在")))?;
    let layers = book
        .version_layers(uid)
        .ok_or_else(|| AppError::corrupted(format!("{uid} 的取值层取不出来")))?;
    // 机型从**我们自己那份清单**里查（`presets/machines/*.toml`，借自 `Committed.catalog`）。
    //
    // 以前查的是上游那份（`book.up.catalog`）—— 于是渲染任一产物都要先有上游仓库，
    // 而机型 id 与版本 id 一样是**我们的身份**（`docs/ARCHITECTURE.md` §10.1），
    // 上游那层正在退出（b04 Task 12）。
    //
    // 直接理由（b05 Task 3.3）：没有这一改，那条「渲染出来的产物 vs 真机基线」的判据
    // 在 CI 里永远跑不起来 —— 上游不在仓库里（`local-reference/` 刻意不入库），
    // 而它恰恰是唯一盯着这条渲染链的判据。
    let machine = book
        .machines()
        .iter()
        .find(|m| m.id == v.machine_id)
        .ok_or_else(|| AppError::not_found(format!("机型 {} 不存在", v.machine_id)))?;

    let fingerprint = layers.fingerprint();
    let recipe = layers.effective_recipe();
    let snapshot: BTreeMap<String, Value> = recipe
        .iter()
        .map(|(k, val)| ((*k).to_owned(), (*val).clone()))
        .collect();

    let mut out = String::new();
    // 头里只有 release_time。曾经的 `# uuid:`（指纹截断）已于 2026-10-05 删——
    // 没有读者的死字段，见模块文档「产物头没有 uuid」一节；过期判定走两层指纹
    out.push_str(&format!("# release_time: {}\n", clock::now_release_time()));
    out.push_str(&format!("# machine: {}\n", machine.id));
    out.push_str(&format!("# variant: {}\n", v.version_id.to_lowercase()));
    out.push_str("#胶笔配置\n");

    // 段名从数据里来。段序按每段内最小的 layout.order —— 与界面上的顺序一致
    let mut sections: Vec<(&str, f64)> = Vec::new();
    for key in layers.keys() {
        let Some(p) = book.presets.registry.param(key) else {
            continue;
        };
        match sections.iter_mut().find(|(s, _)| *s == p.section) {
            Some((_, o)) => *o = o.min(p.layout.order),
            None => sections.push((p.section.as_str(), p.layout.order)),
        }
    }
    sections.sort_by(|a, b| a.1.total_cmp(&b.1));

    for (section, _) in sections {
        out.push_str(&format!("\n[{section}]\n"));

        // 同一个 tomlKey 下可能有多个参数（内联表）。按 tomlKey 首次出现的顺序排
        let mut groups: Vec<(&str, Vec<&ParamDef>)> = Vec::new();
        for key in layers.keys() {
            let Some(p) = book.presets.registry.param(key) else {
                continue;
            };
            if p.section != section {
                continue;
            }
            match groups.iter_mut().find(|(t, _)| *t == p.toml_key) {
                Some((_, v)) => v.push(p),
                None => groups.push((p.toml_key.as_str(), vec![p])),
            }
        }

        // 段内键序 = **tomlKey 字母序**（大小写不敏感）。作者 2026-10-03：
        // 「明明都是 O 开头的 offset 都是一起的，生成的时候却改变了它的顺序」——
        // 以前按界面顺序（layout.order）排，注册表条目一挪、产物键序就漂，
        // diff 里满屏错位。字母序谁都能预期：offset_x/y/z 永远连在一起，
        // 生成不再改变没改过的那些行的位置。
        groups.sort_by_key(|g| g.0.to_lowercase());
        for params in groups.iter_mut() {
            params.1.sort_by_key(|p| p.json_key.to_lowercase());
        }

        for (toml_key, params) in groups {
            if params.len() > 1 {
                // 内联表：`offset = { x = -1, y = 18.6, z = 4 } # 笔尖偏移`
                let members: Vec<String> = params
                    .iter()
                    .filter_map(|p| {
                        layers
                            .effective(&p.key)
                            .map(|hit| format!("{} = {}", p.json_key, scalar(p, hit.value)))
                    })
                    .collect();
                out.push_str(&format!("{toml_key} = {{ {} }}", members.join(", ")));
                // 内联表的注释取第一个成员的 —— 三个成员各写一句会把行撑得没法读
                push_comment(&mut out, &params[0].toml_comment, true);
                continue;
            }

            let p = params[0];
            let Some(hit) = layers.effective(&p.key) else {
                continue;
            };
            if p.ui_component == UiComponent::Gcode {
                // 多行字符串：注释在上一行，值用 `"""`
                push_comment(&mut out, &p.toml_comment, false);
                out.push_str(&format!(
                    "{toml_key} = \"\"\"\n{}\n\"\"\"\n",
                    gcode_body(hit.value)
                ));
            } else {
                out.push_str(&format!("{toml_key} = {}", scalar(p, hit.value)));
                push_comment(&mut out, &p.toml_comment, true);
            }
        }
    }

    Ok(Rendered {
        uid: uid.to_owned(),
        file_name: preset_file_name(&machine.id, &v.version_id),
        text: out,
        fingerprint,
        snapshot,
    })
}

/// 产物文件名。`{机型}-{版本小写}.toml`，例如 `A1-standard.toml`、`A1_MINI-fastv3.3.toml`。
///
/// # 为什么是这一套，不是上游那一套
///
/// 上游 manifest 给的名字是 `A1.toml` / `A1F.toml` / `A1F_260628.toml` 这种历史命名，
/// 而**消费端按 `{机型}-{版本小写}.toml` 找文件**（它 9 份内置预设全是这个形状，
/// 见 b04 `AUDIT-EVIDENCE.md` §2）。名字对不上的后果不是报错，是消费端找不到 ——
/// 我们生成了一堆它认不出的文件。
///
/// 所以这条改动同时**断掉了对上游 `mkp_preset.file_name` 的依赖**：
/// 名字现在只由「机型 id + 版本 id」决定，而这两样都在我们自己的清单里。
/// 上游整层删掉时（b04 Task 12）这里一个字都不用改。
///
/// 版本 id 小写是跟着 `# variant:` 那一行走的 —— 同一份产物里两处指同一个东西，
/// 大小写不一致会让人以为是两个变体。
///
/// # 这是薄壳，规则不在这一层
///
/// 权威实现在 `crates/preset/src/generate.rs` 的 `file_name`（从 crate 根导出为
/// `preset::preset_file_name`），规范写在 `docs/ARCHITECTURE.md` §10。**小写化在那边
/// 发生，只发生一次。**
///
/// 这里以前是**第二份实现**：形状相同，但两份独立实现之间没有编译器 —— 漂移的表现
/// 不是报错，是消费端找不到文件。b05 Task 2 把它并掉了，代价是 `mkpse-preset` 成了
/// 依赖（`workbench` feature 下的可选依赖，见 `src-tauri/Cargo.toml`）：
/// **默认（发布）构建的依赖图里没有它**，那 56 KB 参数注册表与 9 份内置预设也就
/// 进不去给用户的二进制。换掉的是"一个拼字符串的函数要拖进整个内核 crate"这条顾虑，
/// 换来的是一条规则只有一处。
///
/// 两边仍然可能被各自改坏 —— 连着它们的是本文件的判据
/// `naming_matches_the_preset_crate`（逐例对两边），不是类型系统。
pub fn preset_file_name(machine_id: &str, version_id: &str) -> String {
    preset::preset_file_name(machine_id, version_id)
}

/// 行尾注释或独立注释行。**空注释不写 `# `** —— 一个孤零零的井号是噪音
fn push_comment(out: &mut String, comment: &str, trailing: bool) {
    let c = comment.trim();
    if c.is_empty() {
        out.push('\n');
        return;
    }
    if trailing {
        out.push_str(&format!(" # {c}\n"));
    } else {
        out.push_str(&format!("# {c}\n"));
    }
}

/// 标量的 TOML 字面量。**字符串要转义** —— 值里有个引号就能让整份配方解析失败
fn scalar(p: &ParamDef, v: &Value) -> String {
    match p.value_type {
        ValueType::Bool => match v.as_bool() {
            Some(b) => b.to_string(),
            // 类型对不上时不猜。这种数据在校验里已经是阻断，走不到这儿
            None => "false".to_owned(),
        },
        ValueType::Float | ValueType::Int => match v.as_f64() {
            Some(n) if n.fract() == 0.0 => format!("{}", n as i64),
            Some(n) => format!("{n}"),
            None => "0".to_owned(),
        },
        ValueType::Text => toml_string(v.as_str().unwrap_or_default()),
    }
}

fn toml_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// 多行字符串的内容。`"""` 出现在里面会提前结束字面量，所以要断开
fn gcode_body(v: &Value) -> String {
    v.as_str()
        .unwrap_or_default()
        .replace("\"\"\"", "\"\"\\\"")
        .trim_end()
        .to_owned()
}

/// 除了 `release_time` 那一行，两份文本一样吗。
///
/// 这条判据存在的唯一理由：产物头里有时间戳，不跳过它「字节没变不重写」永远不生效，
/// 而那条规则本身是为了不让同步工具误判有更新
fn same_payload(a: &str, b: &str) -> bool {
    let strip = |s: &str| {
        s.lines()
            .filter(|l| !l.starts_with("# release_time:"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    strip(a) == strip(b)
}

/// 这一行要不要进生成记录（`MarkBuilt`）。
///
/// 口径（2026-10-05 作者裁定：**没写文件就不许动台账**）：
/// · 写出去了（`same_payload_on_disk = false`）→ 必记，stamp 与指纹都归本次；
/// · 没写（字节没变）→ **不记** —— 老 stamp 说的「这份文件是何时写出来的」依然成立；
///   只有台账缺失或指纹对不上（记录还是旧源状态的）才补记，
///   否则「待更新」永远消不掉，生成按钮永远亮。
/// 补记走的也是本函数 → no-op 的生成（9 份全无变化且台账对得上）一份都不记，
/// `mark` 为 `None`，`built.json` 一个字节都不变。
fn needs_built_record(
    same_payload_on_disk: bool,
    recorded: Option<&str>,
    fingerprint: &str,
) -> bool {
    !same_payload_on_disk || recorded != Some(fingerprint)
}

/* ---------- 生成 ---------- */

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Scope {
    /// 主按钮「生成待更新项」
    Stale,
    /// 「全部生成」
    All,
    /// 「生成勾选的」
    Picked(Vec<String>),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateReport {
    pub stamp: String,
    /// 真的写了盘的
    pub written: Vec<String>,
    /// 算出来和现在的文件**一模一样**，所以没重写。这一条要显式说出来 ——
    /// 不说的话「点了生成但文件时间没变」看起来像失败了
    pub unchanged: Vec<String>,
    /// 跳过的（暂无资源）+ 原因
    pub skipped: Vec<(String, String)>,
    // 没有生成记录字段：台账（built.json）在生成事务里直接落盘，报告不再携带
    // 它、前端也不再回填草稿（2026-10-06 状态机修正）。
}

/// 这次要生成哪些（`todo`）与跳过了哪些（带原因）。
///
/// **`wb_generate` 与 `wb_generate_preview` 共用这一处** —— 预演必须与真生成算的是
/// 同一批、同一套跳过理由，否则"确认过的"和"真写的"就会是两回事。
fn planned_todos(book: &Book<'_>, scope: &Scope) -> (Vec<String>, Vec<(String, String)>) {
    let rows = book.build_rows();
    let wanted: Vec<&str> = match scope {
        Scope::Stale => rows
            .iter()
            .filter(|r| r.buildable)
            .map(|r| r.uid.as_str())
            .collect(),
        Scope::All => rows.iter().map(|r| r.uid.as_str()).collect(),
        Scope::Picked(uids) => uids.iter().map(String::as_str).collect(),
    };

    let mut skipped: Vec<(String, String)> = Vec::new();
    let mut todo: Vec<String> = Vec::new();
    for uid in wanted {
        // **没有床身尺寸的机型：跳过时说真因。**
        //
        // 这一台生成出来也用不了 —— 消费端读预设时会在内置尺寸表里查不到它，
        // 然后拒掉整份配方（不是少一项检查）。b04 的 P0 审计查明了这件事。
        // 原来它走的是「暂无资源」那条通用话术，而那句话让人去找资源，
        // 方向是错的：要补的是尺寸。
        let no_dims = book
            .version(uid)
            .and_then(|v| book.machines().iter().find(|m| m.id == v.machine_id))
            .is_some_and(|m| !m.has_dimensions);
        if no_dims {
            skipped.push((uid.to_owned(), w::disabled::BUILD_NO_DIMENSIONS.to_owned()));
            continue;
        }
        match rows.iter().find(|r| r.uid == uid) {
            Some(r) if r.state == w::BuildState::NoResources => {
                skipped.push((uid.to_owned(), w::disabled::BUILD_NO_RESOURCES.to_owned()))
            }
            Some(_) => todo.push(uid.to_owned()),
            None => skipped.push((uid.to_owned(), "这一版不在树上".to_owned())),
        }
    }
    (todo, skipped)
}

/// 生成。**有阻断时直接拒绝** —— 那是全程唯一的硬闸门。
///
/// ★ 它是 [`generate_with`] 的薄壳（`with_ctx_mut` + trace）。真正的写在那个收
/// `&mut Ctx` 的自由函数里 —— **发布事务要复用同一台生成器**，而它已经在
/// `with_ctx_mut` 里了（锁不可重入），只能调自由函数。要可变借出，是因为生成
/// 事务要把台账记进 `built.json` 并同步会话内的 committed（见 [`generate_with`]）。
#[tauri::command]
pub fn wb_generate(scope: Scope) -> Result<GenerateReport, AppError> {
    traced("wb_generate", |_| {
        with_ctx_mut(|ctx| generate_with(ctx, &scope))
    })
}

/// 生成的**锁无关内核**：给定一份会话，按 scope 把产物写进 `delivery/mkp/presets/` 并重算目录。
///
/// 与 [`preview_with`] 是同一条理由（见那里）：「生成前预演」与「真生成」、
/// 「发布事务里的生成」必须是**同一批 todo、同一处渲染、同一处落点**。
/// 命令壳只负责 `with_ctx_mut` + trace。
///
/// ⚠ **它会写盘**（产物 + 快照 + **台账** + catalog）。调用方负责先过闸
/// （[`issues::inspect`]）。
pub(super) fn generate_with(
    ctx: &mut super::Ctx,
    scope: &Scope,
) -> Result<GenerateReport, AppError> {
    let (c, d, _) = state(ctx)?;
    let book = Book::new(&ctx.presets, &c, &d);

    let report = issues::inspect(&book);
    if let Some(b) = report.first_block() {
        return Err(AppError::invalid_argument(w::disabled::BUILD_BLOCKED)
            .with_detail(format!("{}：{}", b.title, b.detail)));
    }

    let (todo, skipped) = planned_todos(&book, scope);

    // ① 全部算完。**任一项算不出来则整批不动**
    let mut rendered: Vec<Rendered> = Vec::with_capacity(todo.len());
    for uid in &todo {
        rendered.push(render(&book, uid)?);
    }

    // ② 全部成功才逐个原子替换。落点是**交付根里的 `mkp/presets/`** ——
    // 与客户端下载区同名同形（消费者拿 catalog 的 path 拼 URL，两个根必须同形）。
    // 根从**会话那份 presets 的根**派生（与 `product_on_disk` 同一条同源规矩），
    // 不绕全局 repo_root —— 会话的数据树在哪，交付就落在哪。
    let presets_root = ctx.presets.root();
    let delivery_root = paths::delivery_root_at(presets_root).join(super::delivery::MKP_DIR);
    let delivery_mkp = delivery_root.join("presets");
    let mut written = Vec::new();
    let mut unchanged = Vec::new();
    let mut fingerprints: BTreeMap<String, String> = BTreeMap::new();

    for r in &rendered {
        let target = delivery_mkp.join(&r.file_name);
        let existing = std::fs::read_to_string(&target).ok();
        let same = existing
            .as_deref()
            .is_some_and(|old| same_payload(old, &r.text));
        if same {
            unchanged.push(r.uid.clone());
        } else {
            crate::fsx::atomic::atomic_write(&target, r.text.as_bytes())?;
            written.push(r.uid.clone());
        }
        // 快照照样写：它是恢复配方的依据，和有没有重写产物无关
        //（内容没变时 `atomic_write` 自己会跳过 —— 连修改时间都不动）
        let v = book.version(&r.uid).expect("刚才渲染过");
        ctx.store.write_doc(
            &ctx.store.snapshot_rel(&v.machine_id, &v.version_id)?,
            &r.snapshot,
        )?;
        /* ★ 生成记录只记「真的变了」的（2026-10-05 作者裁定：没写文件就不许动台账）：
        · 写出去的行 —— stamp 与指纹都归本次；
        · 没写的行 —— 字节没变，老 stamp 说的「这份文件是何时写出来的」依然成立，
          一个字节都不动；只有记录缺失 / 指纹对不上（记录是旧的）才补记。
        不这么改的后果：no-op 的生成也把整本台账顶新 → built.json 必脏 →
        git/clean 永远红（真机踩过：只是打开预演看了一眼，工作区就脏了）。 */
        if needs_built_record(same, book.built_fingerprint(&r.uid), &r.fingerprint) {
            fingerprints.insert(r.uid.clone(), r.fingerprint.clone());
        }
    }

    let stamp = clock::now_iso8601();

    /* ★★ **台账与产物同一批事务落盘**（2026-10-06 状态机修正）。
    生成记录曾经装进报告交前端回填草稿、等下一次「保存」才进 built.json ——
    那允许「交付文件已是新字节、台账还是上一代」的中间态存在：本机（草稿叠加态）
    显示已生成，干净检出（CI）判待生成，提交/发布把分裂状态写进 git。现在生成
    记录在这里**直接落盘**（[`super::storage::merge_built_records`]）：生成完成
    = 台账已是这一代，不再有「生成完还得记得保存」这个 UX，也不再需要它。
    落盘之后同步会话内的 committed —— 界面的「已生成」读的就是这份内存态。 */
    let updates: BTreeMap<String, BuiltRecord> = fingerprints
        .iter()
        .map(|(uid, fp)| {
            (
                uid.clone(),
                BuiltRecord {
                    stamp: stamp.clone(),
                    fingerprint: fp.clone(),
                },
            )
        })
        .collect();
    super::storage::merge_built_records(&ctx.store, &c.built, updates.clone())?;
    ctx.committed.built.extend(updates);

    // ③ **清单跟着重算**（作者 2026-10-03）：产物直接写进交付根 —— 目录要是不
    // 跟上，delivery 就处于「文件是新的、目录记的还是旧的」，客户端字节校验必挂
    //（「下载失败：响应比目录登记的大」真机踩了两回）。收尾把 catalog.json
    // 重算一遍，**记录永远与文件同一代**。manifest（版本 / 时间戳 / 渠道）仍归
    // 发布写 —— 生成不替发布定稿。
    //
    // 引用资产（图标 / BBS / 模型）也补进交付根：目录里登记了它们（按源字节
    // 算的 SHA），文件不在 = 客户端 404（作者真机看到的「目录登记了，文件不在」
    // ×7）。先补文件、再重算目录，两头对上。
    let dist_root = paths::delivery_root_at(presets_root);
    let asset_root = paths::assets_root_at(presets_root);
    std::fs::create_dir_all(&asset_root)
        .map_err(|e| AppError::io("建不出资产目录").with_detail(e.to_string()))?;
    super::delivery::write_content(&dist_root, &asset_root, &book)?;
    super::delivery::write_catalog_json(&dist_root, &book, &stamp)?;

    tracing::info!(
        written = written.len(),
        unchanged = unchanged.len(),
        skipped = skipped.len(),
        "生成完成"
    );
    Ok(GenerateReport {
        // 台账已经在上面落盘 —— 报告里不再带生成记录（草稿与生成记录彻底无关）。
        stamp,
        written,
        unchanged,
        skipped,
    })
}

/* ---------- 生成前预演（生成前确认那一步） ---------- */

/// 一份文件的预演结论。
///
/// 三个状态就是「点生成会怎样」的全部可能：
///   · `added`    —— 磁盘上还没有这一份（首次生成 / 被清理过）：正文全绿
///   · `modified` —— 有这一份，但这次算出来的和它不一样：会**原子替换**
///   · `unchanged`—— **正文相同**（只有头部时间戳那行会不一样），不会重写。
///     判据与 [`same_payload`] / [`wb_generate`] 的「跳过」**同一道** ——
///     预演说「要写」而真生成跳过，是把「将写入 N 份」报成假的。
///     作者 2026-10-05 被它吓过一回：只改一个值，确认框说 9 份全要写。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffState {
    Added,
    Modified,
    Unchanged,
}

/// 行级 diff 的一种行。
///
/// 状态判据跟写盘走（见 [`DiffState`]）；**行级差异仍比真文本** ——
/// 真要重写的文件，头里时间戳那行确实会变，如实显示出来。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    /// `context` 没变 / `added` 这次新有 / `removed` 这次没有
    pub kind: DiffLineKind,
    pub text: String,
    /// 第几行（1 起；`removed` 记它在**磁盘旧版**里的行号，其余记新版的）
    pub no: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffLineKind {
    Context,
    Added,
    Removed,
}

/// 一份产物的预演：状态 + 行级差异 + 计数。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewFile {
    pub uid: String,
    pub file_name: String,
    pub state: DiffState,
    /// 行级差异（含未变的上下文行）。`unchanged` 时是**整份正文**（全 `context`）——
    /// 确认框的「完整」视图要能看原文（作者 2026-10-07：「就算它一模一样不会重写，
    /// 我也希望看一个完整的」）
    pub lines: Vec<DiffLine>,
    pub added: usize,
    pub removed: usize,
    /// **谁写的**（生成时重算 / 发布时定稿 / 软件发布链）。
    ///
    /// 附属文件那几份不都由生成写：`manifest.json` / `source.json` 归发布、
    /// `release.json` 归软件发布链 —— 它们也列在确认框里（作者 2026-10-07 问
    /// 「怎么没有这」），界面照这个字段说"本次生成不动"、也不把它们算进「将写入 N 份」
    pub stage: DeliveryStage,
}

/// 预演报告。跳过的项照实列出（与 `wb_generate` 同一套原因）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewReport {
    /// **产物**（`mkp/presets/*.toml`）—— 一份一个版本
    pub files: Vec<PreviewFile>,
    /// **附属文件**（`content/*.json` + `catalog.json`，`uid` / `fileName` 都是交付根相对的路径）。
    ///
    /// ★ 2026-10-07 加：生成不只写 toml —— 它同时重算目录与清单。作者原话
    /// 「我希望到时候它还能显示一个会变化的 Json」。单独一格而不是并进 `files`：
    /// 发布闸的 ⑤「能不能渲染」数的是 `files`（渲染产物），口径不能混。
    pub aux: Vec<PreviewFile>,
    pub skipped: Vec<(String, String)>,
    /// 会被写盘的**产物**份数（`added` + `modified`）
    pub to_write: usize,
    /// 不变的**产物**份数
    pub unchanged: usize,
    /// 有阻断时的唯一原因（与 `wb_generate` 同一道闸，前端照它压按钮）
    pub blocked: Option<String>,
}

/// **生成前预演**：把这次要写的产物都算出来，与磁盘上现存的逐份比，给出行级 diff。
///
/// **一个字节都不写** —— 它就是 [`wb_generate`] 的彩排：同一批 `todo`（同一套跳过理由）、
/// 同一处渲染、同一处落点（`<delivery>/mkp/presets/`），只是把"写"换成"读出来比"。
/// 界面上「点生成 → 先看这个 → 再点确认」的第二步靠它。
///
/// 阻断也照实报（`blocked` 非空 = 生成会被拒），不假装能生成。
#[tauri::command(async)]
pub fn wb_generate_preview(scope: Scope) -> Result<PreviewReport, AppError> {
    traced("wb_generate_preview", |_| {
        with_ctx(|ctx| preview_with(ctx, &scope))
    })
}

/// 预演的**锁无关内核**：给定一份会话，把这次要写的产物都算出来。
///
/// ★ 这一层存在的唯一理由是**「一个入口」**：界面上的「生成前预演」与**发布闸**
/// （[`super::audit`]）必须算同一件事。而发布闸本身已经在 `with_ctx` 里，`with_ctx`
/// 的锁**不可重入** —— 闸里再调一次 `wb_generate_preview` 命令就是自己把自己锁死。
/// 所以「同一台渲染器」落在这一层，**不落在命令壳上**；命令壳只负责 `with_ctx` + trace。
///
/// 只读：`dist_root_path` 不建目录，缺文件按「新增」算，不顺手造出一个 `mkp/presets/`。
pub(super) fn preview_with(ctx: &super::Ctx, scope: &Scope) -> Result<PreviewReport, AppError> {
    let (c, d, _) = state(ctx)?;
    let book = Book::new(&ctx.presets, &c, &d);

    // 生成闸门与 `wb_generate` 是同一道：有阻断就如实说，不往下算
    let report = issues::inspect(&book);
    if let Some(b) = report.first_block() {
        return Ok(PreviewReport {
            files: Vec::new(),
            aux: Vec::new(),
            skipped: Vec::new(),
            to_write: 0,
            unchanged: 0,
            blocked: Some(format!("{}：{}", b.title, b.detail)),
        });
    }

    let (todo, skipped) = planned_todos(&book, scope);
    let delivery = paths::delivery_root_path();
    let presets_dir = delivery.join(super::delivery::MKP_DIR).join("presets");

    let mut files: Vec<PreviewFile> = Vec::with_capacity(todo.len());
    let mut to_write = 0usize;
    let mut unchanged = 0usize;
    for uid in &todo {
        let r = render(&book, uid)?;
        let existing = std::fs::read_to_string(presets_dir.join(&r.file_name)).ok();
        let pf = preview_one(&r, existing.as_deref());
        match pf.state {
            DiffState::Unchanged => unchanged += 1,
            _ => to_write += 1,
        }
        files.push(pf);
    }

    // ★ **附属文件也预演**（作者 2026-10-07：「我希望到时候它还能显示一个会变化的 Json」）。
    //
    // 生成不只写 toml —— 它同时重算目录与清单（content 那三份 + `catalog.json`）。
    // 那几份也在确认框里列出来，比法与产物同一处（`preview_text`）。
    //
    // 正文由 `delivery` 那两台**纯函数**给出（与真生成同一处构造）；
    // 时间戳就用当下这一个（真生成也在这一刻取，预演不另编一个）。
    // ⚠ `catalog.json` 头里有 `publishedAt`：它每次生成都会换，所以它**每次都会显示"修改"**
    // —— 那是实话（`atomic_write` 比的是整份字节，时间戳变了就重写），不是误报。
    let stamp = clock::now_iso8601();
    let mut aux: Vec<PreviewFile> = Vec::new();
    for (rel, text) in super::delivery::content_json_texts(&book)? {
        let old = std::fs::read_to_string(delivery.join(rel)).ok();
        aux.push(preview_text(rel, rel, &text, old.as_deref()));
    }
    let catalog_text = super::delivery::built_catalog(&delivery, &book, &stamp)?.to_pretty_json()?;
    let catalog_rel = super::delivery::NEW_CATALOG_FILE;
    let old = std::fs::read_to_string(delivery.join(catalog_rel)).ok();
    aux.push(preview_text(catalog_rel, catalog_rel, &catalog_text, old.as_deref()));

    // 生成**不动**的那几份也照实列出来（作者 2026-10-07：「那我那个生成的里面怎么没有」）——
    // manifest / source 由发布定稿、release.json 属软件发布链，界面把它们标成"本次生成不动"。
    for (rel, stage) in [
        (super::delivery::MANIFEST_FILE, DeliveryStage::Publish),
        (super::delivery::SOURCE_FILE, DeliveryStage::Publish),
        (crate::runtime::source::RELEASE_FILE, DeliveryStage::Software),
    ] {
        let old = std::fs::read_to_string(delivery.join(rel)).ok();
        aux.push(preview_frozen(rel, stage, old.as_deref()));
    }

    Ok(PreviewReport {
        files,
        aux,
        skipped,
        to_write,
        unchanged,
        blocked: None,
    })
}

/// 比一份产物：见 [`PreviewFile`] 的三档状态。
fn preview_one(r: &Rendered, existing: Option<&str>) -> PreviewFile {
    preview_text(&r.uid, &r.file_name, &r.text, existing)
}

/// 比一份文本：磁盘读得到且正文相同（[`same_payload`]，与 [`wb_generate`] 的「跳过」同一道）
/// → `unchanged`；读得到但不同 → `modified`；读不到 → `added`。
///
/// **无变化也把正文带回去**（作者 2026-10-07）：确认框的「完整」视图要看整份原文 ——
/// 「就算它一模一样不会重写，我也希望看一个完整的」。所以这里给 [`full_lines`]
/// （全 `context`、`added` / `removed` 都是 0），「不会重写」那句只在**对比**视图说。
///
/// 产物（`preview_one`）与**附属文件**（目录 JSON）都走这一处 —— 两边各写一套比法，
/// 迟早有一边把"会写"报成"不会写"。
fn preview_text(uid: &str, file_name: &str, text: &str, existing: Option<&str>) -> PreviewFile {
    let (state, lines) = match existing {
        None => (DiffState::Added, diff_added(text)),
        Some(old) if same_payload(old, text) => (DiffState::Unchanged, full_lines(text)),
        Some(old) => (DiffState::Modified, diff_lines(old, text)),
    };
    let added = lines
        .iter()
        .filter(|l| l.kind == DiffLineKind::Added)
        .count();
    let removed = lines
        .iter()
        .filter(|l| l.kind == DiffLineKind::Removed)
        .count();
    PreviewFile {
        uid: uid.to_owned(),
        file_name: file_name.to_owned(),
        state,
        lines,
        added,
        removed,
        stage: DeliveryStage::Generate,
    }
}

/// 附属文件里**本次生成不动**的那几份（`manifest.json` / `source.json` 归发布定稿、
/// `release.json` 归软件发布链）。
///
/// 它们也列在确认框里（作者 2026-10-07：「那我那个生成的里面怎么没有」）——
/// 但状态**永远是「无变化」**：生成一个字节都不碰它们；详情给盘上原文，看得见就行。
fn preview_frozen(rel: &str, stage: DeliveryStage, existing: Option<&str>) -> PreviewFile {
    PreviewFile {
        uid: rel.to_owned(),
        file_name: rel.to_owned(),
        state: DiffState::Unchanged,
        lines: existing.map(full_lines).unwrap_or_default(),
        added: 0,
        removed: 0,
        stage,
    }
}

/// 新增一份：每一行都是 `added`（界面全绿，不折叠）
fn diff_added(text: &str) -> Vec<DiffLine> {
    text.lines()
        .enumerate()
        .map(|(i, t)| DiffLine {
            kind: DiffLineKind::Added,
            text: t.to_owned(),
            no: i + 1,
        })
        .collect()
}

/// 整份正文，逐行都是 `context`（无变化的文件在「完整」视图里要能看全文）。
///
/// 与 [`diff_added`] 的差别只在 kind：那个是"这次新有的"（界面全绿），
/// 这个每一行都是"磁盘上本来就这样"（素底）。
fn full_lines(text: &str) -> Vec<DiffLine> {
    text.lines()
        .enumerate()
        .map(|(i, t)| DiffLine {
            kind: DiffLineKind::Context,
            text: t.to_owned(),
            no: i + 1,
        })
        .collect()
}

/// 行级 diff：先剥掉两端的公共行，中间那段做最简 LCS，再拼回来。
///
/// **不引第三方 diff 库**（守"不引入新依赖"）。TOML 一行一条、行数在几十到几百，
/// 这个 O(n·m) 的 LCS 在这里毫无压力。剥前缀/后缀是为了让"只改了一行"这种常见情形
/// 退化成"一大段 context + 一两行变化"，避免整份文件都进 LCS。
fn diff_lines(old: &str, new: &str) -> Vec<DiffLine> {
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();

    // 公共前缀
    let mut head = 0;
    while head < a.len() && head < b.len() && a[head] == b[head] {
        head += 1;
    }
    // 公共后缀（不越过头）
    let mut tail = 0;
    while tail < a.len() - head
        && tail < b.len() - head
        && a[a.len() - 1 - tail] == b[b.len() - 1 - tail]
    {
        tail += 1;
    }

    let mid_a = &a[head..a.len() - tail];
    let mid_b = &b[head..b.len() - tail];

    let mut out: Vec<DiffLine> = Vec::with_capacity(a.len().max(b.len()) + mid_a.len());
    for (i, t) in a[..head].iter().enumerate() {
        out.push(DiffLine {
            kind: DiffLineKind::Context,
            text: (*t).to_owned(),
            no: i + 1,
        });
    }
    lcs_diff(mid_a, mid_b, head + 1, head + 1, &mut out);
    for (i, t) in a[a.len() - tail..].iter().enumerate() {
        out.push(DiffLine {
            kind: DiffLineKind::Context,
            text: (*t).to_owned(),
            no: a.len() - tail + i + 1,
        });
    }
    out
}

/// 中间那段的最简 LCS（标准 DP + 回溯），产出 `context` / `added` / `removed` 三种行。
fn lcs_diff(a: &[&str], b: &[&str], a_base: usize, b_base: usize, out: &mut Vec<DiffLine>) {
    let n = a.len();
    let m = b.len();
    // dp[i][j] = a[i..] 与 b[j..] 的最长公共子序列长度
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if a[i] == b[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }
    let (mut i, mut j) = (0usize, 0usize);
    while i < n && j < m {
        if a[i] == b[j] {
            out.push(DiffLine {
                kind: DiffLineKind::Context,
                text: a[i].to_owned(),
                no: a_base + i,
            });
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            out.push(DiffLine {
                kind: DiffLineKind::Removed,
                text: a[i].to_owned(),
                no: a_base + i,
            });
            i += 1;
        } else {
            out.push(DiffLine {
                kind: DiffLineKind::Added,
                text: b[j].to_owned(),
                no: b_base + j,
            });
            j += 1;
        }
    }
    while i < n {
        out.push(DiffLine {
            kind: DiffLineKind::Removed,
            text: a[i].to_owned(),
            no: a_base + i,
        });
        i += 1;
    }
    while j < m {
        out.push(DiffLine {
            kind: DiffLineKind::Added,
            text: b[j].to_owned(),
            no: b_base + j,
        });
        j += 1;
    }
}

/* ---------- 恢复配方 ---------- */

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevertPreview {
    pub uid: String,
    /// 能不能恢复。没有快照时为 false
    pub allowed: bool,
    pub blocked_reason: Option<String>,
    /// 要提交的 patch。前端走 `wb_apply_draft` —— 恢复也是一条撤销
    pub patches: Vec<Patch>,
    /// 会变的项：字段名 + 现在 + 恢复成
    pub changes: Vec<RevertChange>,
    /// **原本继承来的、会从此脱钩的那几项。不许闷着改**
    pub detaching: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevertChange {
    pub key: String,
    pub label: String,
    pub before: String,
    pub after: String,
}

/// 「恢复到上次成功生成时的配方」。**只算不写** —— 写走唯一那条入口
#[tauri::command(async)]
pub fn wb_revert_preview(uid: String) -> Result<RevertPreview, AppError> {
    traced("wb_revert_preview", |_| {
        with_ctx(|ctx| {
            let (c, d, _) = state(ctx)?;
            let book = Book::new(&ctx.presets, &c, &d);
            let v = book
                .version(&uid)
                .ok_or_else(|| AppError::not_found(format!("版本 {uid} 不存在")))?;
            let mut out = RevertPreview {
                uid: uid.clone(),
                allowed: false,
                blocked_reason: None,
                patches: Vec::new(),
                changes: Vec::new(),
                detaching: Vec::new(),
            };

            let rel = ctx.store.snapshot_rel(&v.machine_id, &v.version_id)?;
            let Some(snap) = ctx
                .store
                .read_doc::<BTreeMap<String, Value>>(&rel, "生成快照")?
            else {
                out.blocked_reason =
                    Some("这一版还没有成功生成过，没有可以恢复到的配方".to_owned());
                return Ok(out);
            };
            out.allowed = true;

            let layers = book
                .version_layers(&uid)
                .ok_or_else(|| AppError::corrupted("取值层取不出来"))?;
            for (key, want) in &snap {
                let Some(p) = ctx.presets.registry.param(key) else {
                    continue; // 上游已经删了这个参数，恢复它没有意义
                };
                let Some(hit) = layers.effective(key) else {
                    continue;
                };
                if hit.value == want {
                    continue;
                }
                // **一律写在版本这一层**：快照里只有有效值，看不出当时是哪一层给的
                if !layers.has_own(Level::Version, key) {
                    out.detaching.push(p.label.clone());
                }
                out.changes.push(RevertChange {
                    key: key.clone(),
                    label: p.label.clone(),
                    before: w::value_text(p, hit.value),
                    after: w::value_text(p, want),
                });
                out.patches.push(Patch::SetValue {
                    level: Level::Version,
                    owner: uid.clone(),
                    key: key.clone(),
                    value: Some(want.clone()),
                });
            }
            Ok(out)
        })
    })
}

/* ---------- 发布 ---------- */

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishReport {
    pub stamp: String,
    pub root: String,
    pub files: usize,
    /// 这次发出去的目录登记的**最低正式客户端版本**（结构规则表里对当前结构签名那条）。
    ///
    /// 以前这一格是"上游 manifest 声明了吗"，上游删掉之后恒空；现在它有真来源，
    /// 而且**不要求那个版本已经发布**（Dev 场景，见 `runtime::structure`）。
    /// `None` 只该出现在"这一代还没登记"时 —— 那种情况发布闸会先拦住。
    pub minimum_client: Option<String>,
    /// 待办与提示**不挡发布**，但要在报告里列出来
    pub todos: usize,
    pub hints: usize,
}

/// **发布闸**：发布前那十五项的逐项结果（第二刀）。
///
/// 它**只读**：不写盘、不动 git、不发网络请求。界面拿它画那个逐项打勾的框，
/// 往下走的那颗按钮只在 `can_publish` 为真时亮 —— **任何一项 Blocker 红了
/// 就不许往下走**（作者定的硬规矩）。
///
/// 与 [`wb_publish`] 共用同一个会话上下文，所以"闸里看到的"就是"发布会写出去的"。
///
/// ★ **命令壳是薄的**：算的是 [`super::audit::publish_audit`]，界面与 `cargo test`
/// 判据调的是同一个函数（「界面不许自己再实现一套检查」）。
#[tauri::command(async)]
pub fn wb_publish_audit() -> Result<super::audit::PublishAudit, AppError> {
    traced("wb_publish_audit", |_| super::audit::publish_audit())
}

/// **发布事务**（第三刀下半）：把「审计 → 生成 → 定稿 → 本地 git → 平台 PR/MR」跑成一次手势。
///
/// ★ 这是**唯一对外的发布动作**：`wb_generate` / `wb_publish_audit` 都是它的**内部步骤**
/// （前端不摆「生成」「创建 PR」按钮）。事务内核见 [`super::publish_tx::run`] ——
/// **锁无关**（只收 `&Ctx`），因为 `with_ctx` 锁不可重入（回头调命令壳会自锁）。
///
/// 发布目标（平台 / 仓库 / 用户名 / Token）在**锁外**从发布账户配置解析（[`resolve_publish`]）；
/// **没有配发布账户**时退化成"只生成 + 定稿 + 本地推送"，如实说"没建 PR" ——
/// 那是"还没配账户"，不是失败。
/// ★ `(async)` 不是性能优化，是正确性：这条命令要起 git 子进程、发平台 HTTP ——
/// 跑在主线程上就是"整个窗口一动不动"
/// （2026-10-04 真机事故；与上面「读命令必须异步」同一条病，只是它更重）。
#[tauri::command(async)]
pub fn wb_publish(
    app: tauri::AppHandle,
    opts: Option<super::publish_tx::TxOptions>,
) -> Result<super::publish_tx::PublishTxReport, AppError> {
    traced("wb_publish", |_| {
        let opts = opts.unwrap_or_default();
        // 发布目标 + 平台客户端在锁外构造（读配置 / 凭据文件 / remote，都不碰会话）
        let (target, hosting) = resolve_publish(&app, opts.platform.as_deref());
        let report = with_ctx_mut(|ctx| {
            super::publish_tx::run(
                ctx,
                &opts,
                target.as_ref(),
                hosting.as_ref().map(|h| h.as_ref()),
                None,
            )
        })?;

        // 收尾：把这次回执落进**发布历史**（作者 2026-10-04）。
        // - 壳层做：写历史要 `AppHandle` 拿 appDataDir，而事务内核不碰 AppHandle（锁纪律）；
        // - **只记真发生过的**：被闸拦下（零写入）与 `dry_run`（演练）都不记 —— 回执不是"我点过"，
        //   是"发生过什么"；
        // - 历史写失败**不许**把一次成功的发布说成失败：如实在 summary 上补一句，照样返回。
        let mut report = report;
        if !opts.dry_run && report.stage != super::publish_tx::PublishStage::BlockedAudit {
            match crate::fsx::paths::internal_root(&app) {
                Ok(root) => {
                    let record = super::history::PublishRecord::from_report(
                        &report,
                        crate::workbench::clock::now_iso8601(),
                    );
                    if let Err(e) = super::history::append(&root, record) {
                        report
                            .summary
                            .push_str(&format!("（发布历史没记上：{}）", e.message));
                    }
                }
                Err(e) => report
                    .summary
                    .push_str(&format!("（发布历史没记上：{}）", e.message)),
            }
        }
        Ok(report)
    })
}

/// **发布历史**（只读、`async`）：最近若干次「发布预设」事务的回执，**最新在前**。
///
/// 界面开场读一次；每条"现在走到哪"由 [`super::publish_tx::wb_publish_status`] **手动刷新**
/// （作者定死：状态是"看一看"，不是常驻任务 —— **不做轮询**）。
#[tauri::command(async)]
pub fn wb_publish_history(
    app: tauri::AppHandle,
) -> Result<super::history::PublishHistory, AppError> {
    traced("wb_publish_history", |_| {
        let root = crate::fsx::paths::internal_root(&app)?;
        super::history::load(&root)
    })
}

/// **在系统浏览器里打开一个 URL**（回执屏的「查看 PR」）。
///
/// ★ 只放行 `http(s)://`：这是个"把字符串变成系统动作"的口子，白名单要窄。
/// 与 `wb_reveal_asset` 同一条取向：不写任何应用状态，只开系统程序。
/// ★ `(async)`：命令一律不占主线程（读命令那条规矩的同一个理由）。
#[tauri::command(async)]
pub fn wb_open_external(app: tauri::AppHandle, url: String) -> Result<(), AppError> {
    traced("wb_open_external", |_| {
        let u = url.trim();
        if !(u.starts_with("https://") || u.starts_with("http://")) {
            return Err(AppError::invalid_argument(
                "只允许打开 http(s) 链接 —— 别的形状不交给系统",
            ));
        }
        tauri_plugin_opener::OpenerExt::opener(&app)
            .open_url(u, None::<&str>)
            .map_err(|e| AppError::io("打不开浏览器").with_detail(e.to_string()))
    })
}

/// 解析发布目标 + 造平台客户端（都在锁外）。
///
/// - 配了发布账户 → 解析出 [`PublishTarget`] 并按平台造 `Hosting`（用配置里的 Token）；
/// - **没配**（或配得不完整）→ 两者都是 `None` —— 事务退化成"生成 + 定稿 + 本地推送"，
///   **不报错**（"还没配账户"是正常状态，不是失败）。
fn resolve_publish(
    app: &tauri::AppHandle,
    platform: Option<&str>,
) -> (
    Option<super::publish_tx::PublishTarget>,
    Option<Box<dyn super::platform::Hosting>>,
) {
    let Ok(root) = crate::fsx::paths::internal_root(app) else {
        return (None, None);
    };
    let Ok(target) = super::publish_tx::resolve_target(&root, platform) else {
        // 没配 / 配不完整：不报错，退化成纯本地推送
        return (None, None);
    };
    let hosting: Box<dyn super::platform::Hosting> = match target.platform.as_str() {
        "github" => Box::new(super::platform::github::GitHub::new(target.token.clone())),
        "gitee" => Box::new(super::platform::gitee::Gitee::new(target.token.clone())),
        _ => return (Some(target), None),
    };
    (Some(target), Some(hosting))
}

/// **旧发布壳**（第三刀上半及以前）：只把 `presets/delivery/` 定稿，不生成、不动 git。
///
/// 已被 [`wb_publish`] 事务取代，**不再是前端入口**。留着它是因为它仍是"定稿"这一步的
/// 可单测入口（发布事务内核走的是同一段 `publish_into`）。前端只用 `wb_publish`。
#[allow(dead_code)]
fn publish_deliverable_only() -> Result<PublishReport, AppError> {
    with_ctx(|ctx| {
        let (c, d, _) = state(ctx)?;
        let book = Book::new(&ctx.presets, &c, &d);
        let report = issues::inspect(&book);
        if let Some(b) = report.first_block() {
            return Err(AppError::invalid_argument("有阻断问题没解决，不能发布")
                .with_detail(format!("{}：{}", b.title, b.detail)));
        }

        let root = paths::delivery_root()?;
        let asset_root = paths::assets_root()?;
        let meta = super::delivery::PublishMeta {
            stamp: clock::now_iso8601(),
            channel: PUBLISH_CHANNEL.to_owned(),
            version: String::new(),
        };
        let out = super::delivery::publish_into(&root, &asset_root, &book, &meta)?;
        let stamp = meta.stamp;

        tracing::info!(files = out.files, at = %stamp, "发布完成");
        Ok(PublishReport {
            stamp,
            root: root.display().to_string(),
            files: out.files,
            minimum_client: out.minimum_client.clone(),
            todos: report.todos,
            hints: report.hints,
        })
    })
}

/// **交付目录的残留清单**（b05 Task 13.4 的查询面）。
///
/// 「不在本次交付集合内」的文件，按字典序。发布被残留拦下时，界面先给这一条
/// 让人看清是什么，再决定要不要清理。
#[tauri::command(async)]
pub fn wb_dist_strays() -> Result<Vec<String>, AppError> {
    traced("wb_dist_strays", |_| {
        with_ctx(|ctx| {
            let (c, d, _) = state(ctx)?;
            let book = Book::new(&ctx.presets, &c, &d);
            let expected = super::delivery::delivery_expected_set(&book);
            Ok(super::delivery::scan_strays(
                &paths::delivery_root()?,
                &expected,
            ))
        })
    })
}

/// **清理残留**（b05 Task 13.5）：显式动作，走 `workbench/.trash/delivery/<stamp>/`
/// 回收（保留相对路径，可还原），不直接删。清理完重新发布即可。
#[tauri::command]
pub fn wb_clean_dist_strays() -> Result<usize, AppError> {
    traced("wb_clean_dist_strays", |_| {
        with_ctx(|ctx| {
            let (c, d, _) = state(ctx)?;
            let book = Book::new(&ctx.presets, &c, &d);
            let expected = super::delivery::delivery_expected_set(&book);
            let root = paths::delivery_root()?;
            let strays = super::delivery::scan_strays(&root, &expected);
            if strays.is_empty() {
                return Ok(0);
            }
            let stamp = clock::now_iso8601();
            // 收集符号里的 `:` 会让 Windows 路径出问题，压成安全形状
            let stamp = stamp.replace([':', ' '], "-");
            let trash_root = paths::workbench_root()?.join(".trash");
            let moved = super::delivery::clean_strays(&root, &strays, &trash_root, &stamp)?;
            tracing::info!(moved, at = %stamp, "交付残留已移入回收站");
            Ok(moved)
        })
    })
}

/* ---------- 交付文件清单（「发布预设」卡看这次都会写出什么） ---------- */

/// **交付文件清单**（只读）：本次交付集合里都有哪些文件、盘上有没有、谁写的。
///
/// 作者 2026-10-07：「我现在只能知道这个 TOML 的生成，我不知道这些其他的……
/// 还有什么文件需要生成的，我也想看到」。名单就是交付集合（与残留审计同一份），
/// 分类见 [`super::delivery::DeliveryStage`]。
#[tauri::command(async)]
pub fn wb_delivery_files() -> Result<Vec<super::delivery::DeliveryFile>, AppError> {
    traced("wb_delivery_files", |_| {
        with_ctx(|ctx| {
            let (c, d, _) = state(ctx)?;
            let book = Book::new(&ctx.presets, &c, &d);
            // 只读定位，不顺手建交付目录（和 `wb_dist_strays` 的读侧一样）
            Ok(super::delivery::delivery_files(
                &paths::delivery_root_path(),
                &book,
            ))
        })
    })
}

/// 看一份交付文件的**盘上原文**（只读）。
///
/// 只认交付集合里的路径 —— 界面传什么都读不了集合外的文件。
/// 盘上还没有那一份（还没生成 / 还没发布）时如实报错，不假装有内容。
#[tauri::command(async)]
pub fn wb_delivery_file(rel: String) -> Result<String, AppError> {
    traced("wb_delivery_file", |_| {
        with_ctx(|ctx| {
            let (c, d, _) = state(ctx)?;
            let book = Book::new(&ctx.presets, &c, &d);
            if !super::delivery::delivery_expected_set(&book).contains(&rel) {
                return Err(AppError::invalid_argument(format!("不熟这一份：{rel}"))
                    .with_detail("只给看本次交付集合里的文件".to_owned()));
            }
            let path = paths::delivery_root_path().join(&rel);
            std::fs::read_to_string(&path).map_err(|e| {
                AppError::not_found(format!("盘上还没有这一份：{rel}"))
                    .with_detail(format!("{e} —— 生成 / 发布过之后才有"))
            })
        })
    })
}

/* ---------- 对照基线（b05 Task 14.9） ---------- */

/// 基线 diff 的一条：产物（`BUILTIN_PRESETS`，判据保证与入库目录一份不差）vs
/// `crates/postprocess/tests/fixtures/presets/` 的同名文件
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BaselineDiffEntry {
    pub file_name: String,
    /// `same` / `changed` / `missingBaseline`。没有"产物侧缺失"：
    /// 产物名单来自编译进二进制的表，它有判据盯着
    pub status: String,
    /// 两侧内容哈希前 16 位。给界面确认用 —— 哈希不同就是变了
    pub product_sha: String,
    pub baseline_sha: Option<String>,
}

/// **基线 diff**（14.9 的第①步，**只读**）：列出九份产物的同步状态，
/// 人看过这份清单、点确认，才轮到 [`wb_sync_baseline`] 写。
///
/// 为什么产物侧用 `BUILTIN_PRESETS` 而不是读盘：那张表有判据
/// （`builtin_presets_match_dir`）保证与入库目录**一份不差**，编译进二进制
/// 意味着"发布者看到的"与"用户二进制里带的"是同一份。
#[tauri::command(async)]
pub fn wb_baseline_diff() -> Result<Vec<BaselineDiffEntry>, AppError> {
    traced("wb_baseline_diff", |_| {
        Ok(baseline_diff_against(&preset::generate::fixtures_dir()))
    })
}

/// diff 的领域体：对哪份基线目录比对由调用方给 ——
/// 判据要在系统临时目录的基线上做反向走查（落点闸允许的那个豁免）
fn baseline_diff_against(fixtures: &std::path::Path) -> Vec<BaselineDiffEntry> {
    let mut out = Vec::new();
    for (name, content) in preset::BUILTIN_PRESETS {
        let product_sha = short_sha(content.as_bytes());
        let baseline_bytes = std::fs::read(fixtures.join(name));
        let (status, baseline_sha) = match &baseline_bytes {
            Ok(b) if b.as_slice() == content.as_bytes() => ("same", short_sha(b)),
            Ok(b) => ("changed", short_sha(b)),
            Err(_) => ("missingBaseline", String::new()),
        };
        out.push(BaselineDiffEntry {
            file_name: (*name).to_owned(),
            status: status.to_owned(),
            product_sha,
            baseline_sha: (!baseline_sha.is_empty()).then_some(baseline_sha),
        });
    }
    out
}

/// **同步对照基线**（14.9 的第②步，**显式写入动作**）。
///
/// 前提：人已经看过 [`wb_baseline_diff`] 的清单并确认。这里直接转调
/// `preset::generate::sync_baseline` —— **落点闸在它内部**
/// （`check_baseline_target` 只认真 fixtures 目录或系统临时目录），src-tauri
/// 不经手路径，也就没有绕过闸的口子。内容相同的自动跳过，返回真正写入的份数。
#[tauri::command]
pub fn wb_sync_baseline() -> Result<usize, AppError> {
    traced("wb_sync_baseline", |_| {
        let n = preset::generate::sync_baseline(
            &preset::generate::assets_dir(),
            &preset::generate::fixtures_dir(),
        )
        .map_err(|e| AppError::invalid_argument("基线同步被拒绝").with_detail(e))?;
        tracing::info!(synced = n, "对照基线已同步（人工确认后）");
        Ok(n)
    })
}

fn short_sha(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let h = Sha256::digest(bytes);
    format!("{:x}", h).chars().take(16).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::domain::patch::{apply, Committed, CommittedVersion, Draft};
    use crate::workbench::domain::testkit::{fixture_catalog, Fixture};
    use crate::workbench::store::Store;
    use std::collections::BTreeSet;

    /* ---------- 生成 × 台账（2026-10-06 状态机修正的判据） ---------- */

    use super::super::Ctx;
    use crate::runtime::catalog::Catalog as RuntimeCatalog;
    use crate::workbench::domain::BuildState as TestBuildState;
    use crate::workbench::presets::AssetKind as TestAssetKind;

    /// 一份挂在临时目录上的会话 + 一份被引用的 BBS 资产文件（write_content 的健检要它）。
    /// 生成 / 台账 / 快照 / catalog 的落盘全部从 `ctx.presets.root()` 派生 —— 与真仓库零接触。
    fn fixture_ctx() -> (tempfile::TempDir, Ctx) {
        let f = crate::workbench::domain::testkit::Fixture::load();
        let (dir, presets) = f.into_parts();
        // 夹具引用的资产文件落齐（write_content 的健检要求真实存在）：
        // 两台机型的图标 + A1 的机型图 + A1 的 BBS 曲线。
        // 写盘走 atomic_write —— 写盘纪律对测试同样生效。
        for rel in ["icons/a1.svg", "icons/p1s.svg", "printers/a1.webp"] {
            let p = dir.path().join("presets/assets").join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            crate::fsx::atomic::atomic_write(&p, b"asset").unwrap();
        }
        let bbs = dir.path().join("presets/assets/bbs/A1");
        std::fs::create_dir_all(&bbs).unwrap();
        crate::fsx::atomic::atomic_write(&bbs.join("process.json"), br#"{"x":1}"#).unwrap();
        let store = Store::at(dir.path().join("workbench"));
        store.bootstrap().unwrap();
        (dir, Ctx::with(presets, store).unwrap())
    }

    /// ★ **生成成功 = 台账已经落盘**：不调用 `wb_save`，`built.json` 里就有本批的
    /// 新指纹，且与「现在会渲染出来的」同一代 —— 这就是「产物新、台账旧」那个
    /// 分裂状态的墓碑（真机踩过：本机 Built、CI Stale）。
    #[test]
    fn generation_writes_the_ledger_in_the_same_transaction() {
        let (dir, mut ctx) = fixture_ctx();

        let report = generate_with(&mut ctx, &Scope::All).expect("生成成功");
        assert!(!report.written.is_empty(), "首生成必须真写了东西");

        // **绕开会话**直接读盘（干净检出的视角）：台账里有本批每一笔
        let loaded = super::super::storage::load(&ctx.store, &ctx.presets).unwrap();
        let empty = Draft::default();
        let book = Book::new(&ctx.presets, &loaded.committed, &empty);
        for uid in &report.written {
            let rec = loaded
                .committed
                .built
                .get(uid)
                .unwrap_or_else(|| panic!("台账里必须有 {uid} —— 生成与落账是同一批"));
            let fp = book.version_layers(uid).unwrap().fingerprint();
            assert_eq!(
                rec.fingerprint, fp,
                "{uid} 的台账指纹必须是这一代的渲染指纹"
            );
            // 干净 Book（无草稿）上就是已生成
            assert_eq!(book.build_state(uid), TestBuildState::Built);
        }
        // 草稿从头到尾没被生成碰过
        assert!(ctx.draft.is_clean(), "生成不写草稿 —— 它不是编辑");

        // catalog 登记的 sha 与交付真字节同一代
        let catalog = RuntimeCatalog::parse(
            &std::fs::read(
                dir.path()
                    .join("presets/delivery")
                    .join(crate::workbench::app::delivery::NEW_CATALOG_FILE),
            )
            .unwrap(),
        )
        .unwrap();
        for f in &catalog.files {
            if let Some(sha) = &f.sha256 {
                let bytes = std::fs::read(dir.path().join("presets").join(&f.path)).unwrap();
                assert_eq!(
                    preset::lineage::sha256_hex(&String::from_utf8_lossy(&bytes)),
                    *sha,
                    "{} 的 catalog 登记与真字节必须一致",
                    f.path
                );
            }
        }
    }

    /// ★ **销毁草稿不能把已生成翻成待生成** —— 之前 CI 红的那条链
    /// （生成 → 不保存 → 本机 Built → 草稿一没就 Stale）从这里开始不再成立：
    /// 台账在磁盘上，草稿是死是活与生成状态无关。连「旧版快照里还带着
    /// built 字段」那种历史文件也只被当草稿编辑态解析，翻不了账。
    #[test]
    fn destroying_the_draft_cannot_flip_built_back_to_stale() {
        let (dir, mut ctx) = fixture_ctx();
        generate_with(&mut ctx, &Scope::All).expect("生成成功");

        // 造一份**带着旧 built 字段的草稿快照**（老版本工作台写出来的形状）
        let legacy = serde_json::json!({
            "values": {},
            "visibility": {},
            "bundles": {},
            "built": { "A1/STANDARD": { "stamp": "old", "fingerprint": "deadbeef" } }
        });
        let draft_file = dir.path().join("workbench").join(Store::DRAFT_REL);
        std::fs::create_dir_all(draft_file.parent().unwrap()).unwrap();
        crate::fsx::atomic::atomic_write(&draft_file, legacy.to_string().as_bytes()).unwrap();

        // 干净检出视角：重新从盘上载入（草稿读回来 + 台账读回来）→ 仍然是已生成
        let loaded = super::super::storage::load(&ctx.store, &ctx.presets).unwrap();
        let draft = super::super::storage::read_draft(&ctx.store).unwrap();
        let book = Book::new(&ctx.presets, &loaded.committed, &draft);
        assert_eq!(
            book.build_state("A1/STANDARD"),
            TestBuildState::Built,
            "草稿快照里的旧 built 记录不得覆盖台账"
        );

        // 把草稿整个删掉（等于从没保存过任何草稿）→ 还是已生成
        std::fs::remove_file(&draft_file).unwrap();
        let loaded = super::super::storage::load(&ctx.store, &ctx.presets).unwrap();
        let empty = Draft::default();
        let book = Book::new(&ctx.presets, &loaded.committed, &empty);
        assert_eq!(book.build_state("A1/STANDARD"), TestBuildState::Built);
    }

    /// no-op 的生成**连台账都不碰**：字节没变、指纹全对上时，built.json 原样
    ///（2026-10-05 裁定在生成事务层依然成立 —— git/clean 不因为"看了一眼"变红）。
    #[test]
    fn a_noop_generation_leaves_the_ledger_byte_identical() {
        let (dir, mut ctx) = fixture_ctx();
        generate_with(&mut ctx, &Scope::All).expect("首次生成");

        let ledger = dir.path().join("workbench").join("built.json");
        let before = std::fs::read(&ledger).unwrap();

        let report = generate_with(&mut ctx, &Scope::Stale).expect("二次生成");
        assert!(report.written.is_empty(), "没有新东西要写");
        // 一切都对得上 → 计划清单是空的（既没有要写的，也没有要跳过的）

        let after = std::fs::read(&ledger).unwrap();
        assert_eq!(before, after, "no-op 生成不许动台账（逐字节）");
    }

    /// 台账里的指纹就是渲染指纹 → 每一份有产物的版本都该是「已生成」；
    /// A2L 那种「暂无资源」不参与（它是没有东西可生成，不是生成过没生成过）。
    #[test]
    fn every_renderable_version_reads_built_after_a_full_generation() {
        let (_dir, mut ctx) = fixture_ctx();
        generate_with(&mut ctx, &Scope::All).expect("全量生成");

        let loaded = super::super::storage::load(&ctx.store, &ctx.presets).unwrap();
        let empty = Draft::default();
        let book = Book::new(&ctx.presets, &loaded.committed, &empty);
        for uid in ["A1/STANDARD", "A1/FAST", "P1S/LITE"] {
            assert_eq!(
                book.build_state(uid),
                TestBuildState::Built,
                "{uid} 在全量生成后必须是已生成"
            );
        }
        // 资产台账上的 mkp 条目也全部「在」—— 另一条 CI 判据（资产清单完整）的形状
        let list =
            crate::workbench::app::assets::wb_assets(None, None, None, None, None, None).unwrap();
        let missing: Vec<&str> = list
            .assets
            .iter()
            .filter(|a| a.kind == TestAssetKind::MkPreset && !a.present)
            .map(|a| a.id.as_str())
            .collect();
        assert!(missing.is_empty(), "mkp 资产不该有「不在」的：{missing:?}");
    }

    /* ---------- 对照基线（b05 Task 14.9） ---------- */

    /// **真数据只读锚点**：当前产物与基线应当逐字节相同（K-G0' 绿的现状），
    /// 所以 diff 必须是 9 条全 `same`。这条同时验证 diff 命令**只读**：
    /// 它跑前后基线目录的文件集合不能变。
    #[test]
    fn the_real_baseline_diff_reports_all_same() {
        let fixtures = preset::generate::fixtures_dir();
        let before: BTreeSet<String> = walk_shas(&fixtures);
        let out = wb_baseline_diff().expect("diff");
        assert_eq!(
            out.len(),
            9,
            "BUILTIN_PRESETS 是 9 份 —— 名单变了就说清为什么"
        );
        let not_same: Vec<_> = out.iter().filter(|e| e.status != "same").collect();
        assert!(
            not_same.is_empty(),
            "真产物与基线应当全绿，却有：{not_same:?} —— 谁改了没同步？"
        );
        let after: BTreeSet<String> = walk_shas(&fixtures);
        assert_eq!(before, after, "diff 是只读的，不许动基线目录");
    }

    /// **反向走查（在系统临时目录做，落点闸明确允许；真基线一个字节不碰）**：
    /// 改一份基线 → diff 报 `changed` → sync 写入 → diff 回到全 `same`，
    /// 且写入后的字节与产物**逐字节相同**。未经确认直接写在这里不存在 ——
    /// sync 是显式命令，判据同时证明它**只动 diff 说过的那一份**
    #[test]
    fn sync_baseline_writes_exactly_what_the_diff_named() {
        let tmp = tempfile::tempdir().unwrap();
        let fixtures = tmp.path().to_path_buf();
        // 铺 9 份与产物相同的基线
        for (name, content) in preset::BUILTIN_PRESETS {
            crate::fsx::atomic::atomic_write(&fixtures.join(name), content.as_bytes()).unwrap();
        }
        // 改其中一份（反向：diff 必须抓到）
        let victim = preset::BUILTIN_PRESETS[0].0;
        crate::fsx::atomic::atomic_write(
            &fixtures.join(victim),
            format!("{}\n# 改过了\n", preset::BUILTIN_PRESETS[0].1).as_bytes(),
        )
        .unwrap();

        let out = baseline_diff_against(&fixtures);
        let changed: Vec<_> = out.iter().filter(|e| e.status == "changed").collect();
        assert_eq!(changed.len(), 1, "恰好一份变了：{changed:?}");
        assert_eq!(changed[0].file_name, victim);
        assert!(changed[0].baseline_sha.is_some());

        // sync（tempdir 在落点闸的允许清单里）：只写那一份，其余跳过
        let n = preset::generate::sync_baseline(&preset::generate::assets_dir(), &fixtures)
            .expect("sync");
        assert_eq!(n, 1, "只同步 diff 点名的那一份");

        // 重回全绿，且那份的字节与产物逐字节相同
        let out2 = baseline_diff_against(&fixtures);
        assert!(out2.iter().all(|e| e.status == "same"), "sync 后必须全绿");
        assert_eq!(
            std::fs::read(fixtures.join(victim)).unwrap(),
            preset::BUILTIN_PRESETS[0].1.as_bytes(),
            "写入的字节就是产物本体"
        );
    }

    /// 目录里全部文件的 sha 指纹（文件名 → sha 前 16），给"只读"断言用
    fn walk_shas(dir: &std::path::Path) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        fn walk(dir: &std::path::Path, out: &mut BTreeSet<String>) {
            for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, out);
                } else {
                    let bytes = std::fs::read(&p).unwrap_or_default();
                    out.insert(format!(
                        "{}:{}",
                        p.file_name()
                            .map(|s| s.to_string_lossy())
                            .unwrap_or_default(),
                        short_sha(&bytes)
                    ));
                }
            }
        }
        if dir.is_dir() {
            walk(dir, &mut out);
        }
        out
    }

    fn setup() -> (tempfile::TempDir, Fixture, Committed) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at(dir.path());
        store.bootstrap().unwrap();
        let f = Fixture::load();
        let mut versions = BTreeMap::new();
        for (uid, machine, vid, name) in [
            ("A1/STANDARD", "A1", "STANDARD", "标准版"),
            ("A1/FAST", "A1", "FAST", "高速版"),
            ("A2L/STANDARD", "A2L", "STANDARD", "标准版"),
            ("P1S/LITE", "P1S", "LITE", "精简版"),
        ] {
            versions.insert(
                uid.to_owned(),
                CommittedVersion {
                    machine_id: machine.to_owned(),
                    version_id: vid.to_owned(),
                    name: name.to_owned(),
                    ..Default::default()
                },
            );
        }
        let c = Committed {
            versions,
            catalog: fixture_catalog(),
            ..Default::default()
        };
        (dir, f, c)
    }

    /// 渲染出来的东西要**能被 TOML 解析器读回来**，而且值对得上
    #[test]
    fn rendered_toml_parses_and_keeps_the_values() {
        let (_d, f, c) = setup();
        let draft = Draft::default();
        let book = Book::new(&f.presets, &c, &draft);
        let r = render(&book, "A1/STANDARD").unwrap();

        assert!(r.text.starts_with("# release_time: "));
        assert!(r.text.contains("# machine: A1"));
        assert!(r.text.contains("# variant: standard"));
        assert!(
            r.text.contains("[toolhead]"),
            "段名要从数据里来：\n{}",
            r.text
        );

        // 解析成 `Table` 而不是 `Value`：toml 1.x 的 `Value` 从字符串解析要求整份文档
        // 就是一个值，而我们产出的是一份带表头的文档
        let parsed: toml::Table = r.text.parse().expect("产出来的必须是合法 TOML");
        let toolhead = &parsed["toolhead"];
        /*
         * 三轴偏移：**三个独立字段**（2026-10-02 从共享 tomlKey 的内联表 `offset = { x, y, z }`
         * 拆出来）。所以这里读的是 `offset_x/y/z` 三个裸键，不再是 `toolhead["offset"]["x"]`。
         * **整数不写小数点** —— 上游真产物就是 `offset_x = -1`。
         */
        assert_eq!(
            toolhead["offset_x"].as_integer(),
            Some(-1),
            "A1:STANDARD 的上游覆盖"
        );
        assert_eq!(toolhead["offset_y"].as_float(), Some(18.6));
        assert_eq!(toolhead["offset_z"].as_integer(), Some(4));
        // G-code 走多行字符串
        assert!(toolhead["script"].as_str().is_some());
        assert_eq!(parsed["wiping"]["mode"].as_str(), Some("tower"));
        // 行尾注释要在（挂在 offset_x 那一行上）
        assert!(
            r.text.contains("# 笔尖偏移"),
            "偏移那一行的行尾注释丢了：\n{}",
            r.text
        );
    }

    /// **M0：我们渲染出来的产物 vs 真机验证过的那份基线。**（b04 Task 11）
    ///
    /// 这是整个迁移的前置判据。基线是那 9 份内置预设 —— 它们由 `gen-presets` 从
    /// `preset_recipes.toml` 生成，**上过真机**。我们这条链（`presets/*.toml` → `render()`）
    /// 算出来的如果与它正文字节相同，说明两套真源等值、迁移不需要修数据；
    /// 不同就必须先定哪边对 —— 搬完 3 万行代码再发现值对不上，会留下一批
    /// "生成出来但和验证过的不一样"的产物，而那种错在产物上看不出来。
    ///
    /// 比的是**正文**：基线（历史产物）头里带着 `# uuid:` 与 `# release_time:`，
    /// 现役产物头里只有后者 —— 剥离这两行才能把两边对齐到正文
    /// （uuid 是 2026-10-05 删掉的死字段，见模块文档；基线不改，它是历史事实）。
    ///
    /// # 基线在仓内，配对认身份不认文件名（b05 Task 3.3）
    ///
    /// 基线以前指向兄弟仓库 `../mkp-ssr/crates/preset/assets/presets`：开发机上有它、
    /// CI 上没有 —— 于是在 CI 里这条判据一次都没跑过（那句"没找到基线目录"连颜色都不变）。
    /// 现在改指 `preset::generate::fixtures_dir()`：**仓内那 9 份，与旧仓那份逐字节相同**
    /// （sha256 9/9 一致，只差文件名），判据的强度没有变，但它从此在 CI 里也真跑。
    ///
    /// 配对**按文件头的 `# machine:` / `# variant:`**，不按文件名：夹具那边现在还是
    /// 云端那套名字（`A1MF_260628.toml`），b05 Task 5 会把它们改成产物命名
    /// （`A1_MINI-fastv3.3.toml`）。两套名字都配得上，这条判据不用跟着改。
    ///
    /// # 反空转：9 份一份都不能少
    ///
    /// 基线少了、同一身份重了、或某一份没被任何版本配上，都直接失败 ——
    /// 一条"比了 0 份"的判据比没有判据更坏，因为它是绿的。
    #[test]
    fn our_render_matches_the_machine_verified_baseline() {
        // 基线是消费端内置的 9 份，必须与真 presets 逐份对上。
        // 机型与版本全部来自我们自己那份清单（`presets/machines/*.toml`），
        // `render()` 不查任何外部来源。
        let presets = crate::workbench::load_presets().expect("真 presets");
        let tmp = tempfile::tempdir().unwrap();
        let store = crate::workbench::store::Store::at(tmp.path());
        store.bootstrap().unwrap();
        let c = super::super::storage::load(&store, &presets)
            .expect("干净仓库读得通")
            .committed;
        let draft = Draft::default();
        let book = Book::new(&presets, &c, &draft);

        /// 基线目录按**身份**索引：`(机型 id, 版本 id 小写)` → 文件。
        ///
        /// 读不出来的、同一身份出现两次的，都在这里直接报错：静默跳过会让这条判据
        /// 悄悄少比几份，而它看起来还是绿的。
        fn baseline_by_identity(
            dir: &std::path::Path,
        ) -> BTreeMap<(String, String), std::path::PathBuf> {
            let mut out: BTreeMap<(String, String), std::path::PathBuf> = BTreeMap::new();
            let entries = std::fs::read_dir(dir)
                .unwrap_or_else(|e| panic!("读不到基线目录 {}：{e}", dir.display()));
            for entry in entries {
                let path = entry.expect("目录项").path();
                if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                    continue;
                }
                let text = std::fs::read_to_string(&path)
                    .unwrap_or_else(|e| panic!("读不到 {}：{e}", path.display()));
                let file = preset::read_preset_from_bytes(text)
                    .unwrap_or_else(|e| panic!("{} 读不出来：{e}", path.display()));
                let variant = file.variant.as_deref().unwrap_or_else(|| {
                    panic!("{} 的文件头没有 `# variant:`，身份认不出来", path.display())
                });
                let key = (file.machine.clone(), variant.to_lowercase());
                if let Some(prev) = out.insert(key.clone(), path.clone()) {
                    panic!(
                        "基线 {:?} 有两份：{} 与 {}",
                        key,
                        prev.display(),
                        path.display()
                    );
                }
            }
            out
        }

        let baseline_dir = preset::generate::fixtures_dir();
        let baselines = baseline_by_identity(&baseline_dir);
        assert_eq!(
            baselines.len(),
            9,
            "基线应当正好 9 份（实测 {}）：{:?} —— 是谁动的？",
            baselines.len(),
            baselines.keys().collect::<Vec<_>>()
        );

        /// 去掉按定义不对齐的头部行：release_time 是当下时间；
        /// uuid 只存在于历史基线那侧（现役产物已不写它，见模块文档）
        fn body(s: &str) -> String {
            s.lines()
                .filter(|l| !l.starts_with("# uuid:") && !l.starts_with("# release_time:"))
                .collect::<Vec<_>>()
                .join("\n")
        }

        /// 同一份正文按行排序 —— 用来把「值不一样」与「只是顺序不一样」分开。
        ///
        /// 这两件事的处置完全不同：值不一样要定哪边对（可能要改数据），
        /// 顺序不一样只要定一个口径（TOML 键序不影响语义，`load_ir` 读进去是一样的）。
        /// 合在一起报「不一致」等于把一个能一句话解决的问题说成了一个要查数据的问题
        fn sorted_body(s: &str) -> Vec<String> {
            let mut v: Vec<String> = body(s).lines().map(str::to_owned).collect();
            v.sort();
            v
        }

        let mut compared = 0usize;
        let mut value_diff: Vec<String> = Vec::new();
        let mut order_only: Vec<String> = Vec::new();
        let mut matched: Vec<(String, String)> = Vec::new();
        for v in book.versions() {
            // 产物名只进报告（它比 uid 更接近用户看到的东西）；配对靠的是身份
            let name = preset_file_name(&v.machine_id, &v.version_id);
            let key = (v.machine_id.clone(), v.version_id.to_lowercase());
            let Some(path) = baselines.get(&key) else {
                continue; // 没有基线的版本（A2L 那台占位就是这样）—— 不是差异
            };
            let want = std::fs::read_to_string(path).expect("读基线");
            let got = render(&book, &v.uid).expect("渲染得出来").text;
            matched.push(key);
            compared += 1;
            let (a, b) = (body(&got), body(&want));
            if a == b {
                continue;
            }
            if sorted_body(&got) == sorted_body(&want) {
                // 每一行两边都有，只是排的位置不同
                order_only.push(name);
                continue;
            }
            // 真差异：逐行列出只在一边出现的
            let (sa, sb) = (sorted_body(&got), sorted_body(&want));
            let only_ours: Vec<&String> = sa.iter().filter(|l| !sb.contains(l)).take(4).collect();
            let only_theirs: Vec<&String> = sb.iter().filter(|l| !sa.contains(l)).take(4).collect();
            value_diff.push(format!(
                "  {name}\n    只在我们：{only_ours:?}\n    只在基线：{only_theirs:?}"
            ));
        }

        // 反空转①：9 份基线一份不少地配上号。
        // 配不上只有一种原因：`presets/machines/*.toml` 里那条身份没了（改名/删版本），
        // 而基线还留着 —— 那种情况必须有人处置，不能靠 `continue` 悄悄少比一份
        let unmatched: Vec<&(String, String)> =
            baselines.keys().filter(|k| !matched.contains(*k)).collect();
        assert!(
            unmatched.is_empty(),
            "基线里有 {} 份没配上任何版本：{unmatched:?} —— \
             `presets/machines/*.toml` 与夹具的身份对不上了",
            unmatched.len()
        );
        // 反空转②：比到的份数 == 基线份数（上面那条为空 + 一对一配对 ⇒ 这条冗余，
        // 它写在这是为了让「基线 9 份、比了 9 份」成为一句能直接读的结论）
        assert_eq!(
            compared,
            baselines.len(),
            "比了 {compared} 份，基线有 {} 份",
            baselines.len()
        );
        assert!(
            value_diff.is_empty(),
            "**值不一致**（比了 {compared} 份）—— 这要先定哪边对，不能直接搬代码：\n{}",
            value_diff.join("\n")
        );

        // **M0 的结论（2026-09-23 实测）：9 份逐行同集合，值全对上了。**
        //
        // 剩下的唯一差异是**段内键序**：基线是 offset → speed_limit → custom_mount_gcode…，
        // 我们按 `layout.order`（界面顺序）。TOML 键序不影响语义，`load_ir` 读进去一样，
        // 所以这不是数据问题，是口径问题。
        //
        // 为什么这里只记录不断言：**定案倾向跟基线**（那 9 份已经发布、上过真机，
        // 字节相同意味着"换成我们生成"在交付面上是零变化），但改排序会牵动
        // 好几条既有判据，属于一次独立的改动。它是 Task 11 的收尾项。
        //
        // 不断言不等于放过：`value_diff` 那一条是真判据（值一变就红），
        // 而键序这一条一旦修好，把下面这个 `if` 换成 `assert!` 即可 —— 留着这行是为了
        // 让"还没修"这件事在每次跑测试时都出现在眼前，而不是躺在某个清单里
        if !order_only.is_empty() {
            eprintln!(
                "【M0 已知差异，待 Task 11 收尾】值全对上了（{compared} 份逐行同集合），\
                 但段内键序不同：{} 份。定案倾向跟基线（它已发布、上过真机）。\
                 推断基线用的是注册表里 [[params]] 的出现顺序，而我们用 layout.order —— \
                 落地前要先验证这个推断",
                order_only.len()
            );
        }
    }

    /// 段内键序 = **tomlKey 字母序**（大小写不敏感）—— 生成不许改变没改过的行的位置。
    ///
    /// 作者 2026-10-03：「明明都是 O 开头的 offset 都是一起的，生成的时候却改变了
    /// 它的顺序」—— 以前按界面顺序排，注册表一挪条目产物键序就漂。
    #[test]
    fn sections_are_sorted_by_key_name_so_the_order_never_drifts() {
        let (_d, f, c) = setup();
        let draft = Draft::default();
        let book = Book::new(&f.presets, &c, &draft);
        let r = render(&book, "A1/STANDARD").unwrap();

        // 抠出 [toolhead] 段的键（小写化后必须已经有序）
        let seg = r
            .text
            .split("[toolhead]\n")
            .nth(1)
            .unwrap()
            .lines()
            .take_while(|l| !l.starts_with('[') && !l.is_empty())
            .filter_map(|l| l.split('=').next())
            .map(|k| k.trim().to_lowercase())
            .collect::<Vec<_>>();
        assert!(
            seg.len() >= 2,
            "fixture 的 toolhead 段该有几个键：{:?}",
            seg
        );
        let mut sorted = seg.clone();
        sorted.sort();
        assert_eq!(seg, sorted, "段内键要按字母序：{:?}", seg);
    }

    /// 同样的输入**产出同样的字节**（除了时间戳那一行）——
    /// 头里若再有"每次都变"的东西（随机 uuid 之类），这条就不成立，
    /// 而「字节没变不重写」也就废了
    #[test]
    fn rendering_is_deterministic_apart_from_the_timestamp() {
        let (_d, f, c) = setup();
        let draft = Draft::default();
        let book = Book::new(&f.presets, &c, &draft);
        let a = render(&book, "A1/STANDARD").unwrap();
        let b = render(&book, "A1/STANDARD").unwrap();
        assert!(same_payload(&a.text, &b.text));
        assert_eq!(a.fingerprint, b.fingerprint);
    }

    /// ★ 生成记录的口径（2026-10-05）：**没写文件就不许动台账**。
    /// no-op 的生成（产物字节没变、记录也对得上）一份都不记 → `mark` 为 `None`
    /// → `built.json` 一个字节不变 → git/clean 不会因为"看了一眼"就红。
    /// （真机踩过：只是打开预演，整本台账 stamp 被顶新，发布闸拦发布。）
    #[test]
    fn a_noop_generate_records_nothing() {
        let fp = "abc";
        // 产物字节没变 + 记录的指纹对得上 → 不记
        assert!(!needs_built_record(true, Some(fp), fp));
        // 写出去了 → 必记（stamp 归本次）
        assert!(needs_built_record(false, Some(fp), fp));
        // 没写，但记录是旧源状态的（指纹对不上）→ 补记，否则「待更新」消不掉
        assert!(needs_built_record(true, Some("old"), fp));
        // 没写，且台账里根本没有这条（记录丢了）→ 补记
        assert!(needs_built_record(true, None, fp));
    }

    /// 产物头**不许**出现 `# uuid:` —— 2026-10-05 删掉的死字段（全仓无读者、
    /// 公开渠道无消费方，见模块文档），谁把它加回来这条就红。
    /// 历史基线那 9 份头里还带着它，那是历史事实，与本判据无关。
    #[test]
    fn the_product_header_carries_no_uuid() {
        let (_d, f, c) = setup();
        let draft = Draft::default();
        let book = Book::new(&f.presets, &c, &draft);
        for uid in ["A1/STANDARD", "A1/FAST", "A2L/STANDARD", "P1S/LITE"] {
            let r = render(&book, uid).unwrap();
            assert!(
                !r.text.contains("# uuid:"),
                "{uid} 的产物头里出现了 uuid —— 一行没有读者的假字段又回来了"
            );
        }
    }

    /// 改一个值 → 产物真的变了（不然「待生成」是句空话）
    #[test]
    fn changing_a_value_changes_the_output() {
        let (_d, f, c) = setup();
        let mut draft = Draft::default();
        let before = render(&Book::new(&f.presets, &c, &draft), "A1/STANDARD").unwrap();

        apply(
            &mut draft,
            &c,
            &f.presets.registry,
            &[Patch::SetValue {
                level: Level::Version,
                owner: "A1/STANDARD".to_owned(),
                key: "wiping.child".to_owned(),
                value: Some(serde_json::json!(33)),
            }],
        )
        .unwrap();
        let after = render(&Book::new(&f.presets, &c, &draft), "A1/STANDARD").unwrap();

        assert!(!same_payload(&before.text, &after.text));
        assert_ne!(before.fingerprint, after.fingerprint);
        assert!(after.text.contains("child = 33"));
    }

    /// 字符串里的引号要转义 —— 不转义能让整份配方解析失败
    #[test]
    fn strings_are_escaped() {
        assert_eq!(toml_string("a\"b"), "\"a\\\"b\"");
        assert_eq!(toml_string("a\\b"), "\"a\\\\b\"");
        assert_eq!(toml_string("a\nb"), "\"a\\nb\"");
    }

    /// 多行字符串里出现 `"""` 要断开，否则字面量提前结束
    #[test]
    fn a_triple_quote_inside_gcode_is_broken_up() {
        let body = gcode_body(&serde_json::json!("G1\n\"\"\"\nG2"));
        assert!(!body.contains("\"\"\""), "还留着三引号：{body}");
        let text = format!("k = \"\"\"\n{body}\n\"\"\"\n");
        let parsed: toml::Table = text.parse().expect("断开之后要还能解析");
        assert!(parsed["k"].as_str().unwrap().contains("G1"));
    }

    /// `same_payload` 只忽略时间戳，别的一个字都不忽略
    #[test]
    fn same_payload_only_ignores_the_timestamp() {
        let a = "# uuid: x\n# release_time: 1\nk = 1\n";
        let b = "# uuid: x\n# release_time: 2\nk = 1\n";
        let c2 = "# uuid: x\n# release_time: 1\nk = 2\n";
        assert!(same_payload(a, b));
        assert!(!same_payload(a, c2));
    }

    /// 有阻断时**渲染照做、但生成该被拒**。
    /// 这里只验校验那一半（`wb_generate` 要真仓库，走不了单测）
    #[test]
    fn a_block_is_detected_before_generating() {
        let (_d, f, c) = setup();
        let mut draft = Draft::default();
        apply(
            &mut draft,
            &c,
            &f.presets.registry,
            &[Patch::SetValue {
                level: Level::Version,
                owner: "A1/STANDARD".to_owned(),
                key: "wiping.mode".to_owned(),
                value: Some(serde_json::json!("teleport")),
            }],
        )
        .unwrap();
        let book = Book::new(&f.presets, &c, &draft);
        let r = issues::inspect(&book);
        assert!(r.blocked());
        assert!(!w::disabled::BUILD_BLOCKED.is_empty());
    }

    /// A2L 没有产物 → 渲染它照样能出东西（参数全是出厂默认），
    /// 但生成会把它按「暂无资源」跳过 —— 这两件事分开
    #[test]
    fn a2l_renders_but_is_skipped_by_generate() {
        let (_d, f, c) = setup();
        let draft = Draft::default();
        let book = Book::new(&f.presets, &c, &draft);

        let r = render(&book, "A2L/STANDARD").unwrap();
        assert!(r.text.contains("# machine: A2L"));
        // 文件名只由清单决定（机型 id + 版本 id 小写），与上游给不给名字无关
        assert_eq!(r.file_name, "A2L-standard.toml");

        let row = book
            .build_rows()
            .into_iter()
            .find(|x| x.uid == "A2L/STANDARD")
            .unwrap();
        assert_eq!(row.state, w::BuildState::NoResources);
        assert!(!row.buildable, "生成时该跳过它");
    }

    /// **跳过占位机型时说的必须是真因，而且不许写成催办。**（b04 P0 审计的后续）
    ///
    /// A2L 现在只是占了个名字，还没开始做。空着是**闭合的**：这台不生成、不进清单，
    /// 消费端也拿不到（它自己的内置尺寸表里同样没有这台）。所以这里要的是
    /// 「收起来并说一句」，不是「催人去补」。
    ///
    /// 「暂无资源」那句话不能复用：缺资源是漏（该有的没有），占位是刻意 ——
    /// 说成缺资源会让人去找一件根本不存在的东西。
    ///
    /// 这一条同时是「判据的判据」：夹具里 A2L **刻意没有** `[dimensions]`
    /// （`testkit::has_dimensions`），所以它一定走得到这一支
    #[test]
    fn skipping_a_placeholder_machine_names_the_real_reason() {
        let (_d, f, c) = setup();
        let draft = Draft::default();
        let book = Book::new(&f.presets, &c, &draft);

        let a2l = book
            .machines()
            .iter()
            .find(|m| m.id == "A2L")
            .expect("夹具里有 A2L");
        assert!(
            !a2l.has_dimensions,
            "夹具的 A2L 本该没有尺寸，这条判据在空转"
        );

        // 别的机型有尺寸 —— 这一支不该把它们也拦下来
        assert!(
            book.machines()
                .iter()
                .filter(|m| m.id != "A2L")
                .all(|m| m.has_dimensions),
            "只有 A2L 该缺尺寸"
        );

        // 两句话必须是两句话，而且都不许是催办口气
        assert_ne!(
            w::disabled::BUILD_NO_DIMENSIONS,
            w::disabled::BUILD_NO_RESOURCES,
            "占位与缺资源说了同一句话，用户会照着错的方向去找"
        );
        assert!(
            w::disabled::BUILD_NO_DIMENSIONS.contains("占位"),
            "没说清这是占位：{}",
            w::disabled::BUILD_NO_DIMENSIONS
        );
        assert!(
            !w::disabled::BUILD_NO_DIMENSIONS.contains("补上"),
            "写成了催办：{}",
            w::disabled::BUILD_NO_DIMENSIONS
        );

        // 占位不挡生成 —— 别的机型照样交付
        let report = crate::workbench::domain::issues::inspect(&book);
        assert!(
            report.first_block().is_none(),
            "占位机型把整批生成挡住了：{:?}",
            report.first_block().map(|b| b.title.clone())
        );
    }

    /// 薄壳与权威实现逐例相等 —— 这是连着两处的**那根线**。
    ///
    /// 命名规则现在只有一处（`preset::preset_file_name`），这里只转调；但"转调"本身
    /// 没有类型系统兜底：哪天有人在薄壳里再补一次 `to_lowercase()`、或把分隔符从 `-`
    /// 改成 `_`，编译照样过，红的是消费端的查找。所以拿一组能区分行为的输入两边各算一遍。
    ///
    /// 输入刻意选在规则的每条边界上：版本 id 大小写混写（小写化在哪一侧发生）、
    /// 机型 id 含 `_`（它必须原样保留）、版本 id 含 `.`（`fastv3.3` 那种）。
    #[test]
    fn naming_matches_the_preset_crate() {
        for (machine, version) in [
            ("A1", "standard"),
            ("A1", "FASTV3.3"),
            ("A1", "Fast"),
            ("A1_MINI", "STANDARD"),
            ("A1_MINI", "FastV3.3"),
            ("P1S", "lite"),
            ("X1C", "LITE"),
        ] {
            assert_eq!(
                preset_file_name(machine, version),
                preset::preset_file_name(machine, version),
                "{machine}:{version} —— 两处算出的文件名不同，规则已经分岔"
            );
        }
    }

    /// **渲染的机型取自我们自己的清单。**
    ///
    /// `render()` 曾经用上游目录查机型，所以「上游清单里没有这台」会让渲染失败。
    /// 上游整层删掉之后，机型只来自 `presets/machines/*.toml`（借自 `Committed.catalog`）
    /// —— 这条判据钉住「机型 id 与版本 id 一样是我们的身份」（`docs/ARCHITECTURE.md` §10.1）。
    ///
    /// 反空转靠**挑一台夹具清单里没有、真 `presets/` 里有的机型**（A1_MINI）：
    /// 它不在 `testkit::FIXTURE_MACHINES`（A1 / A2L / P1S），渲染必须照样成功。
    #[test]
    fn render_takes_the_machine_from_our_own_catalog() {
        // 前提反空转：真 presets/ 必须有 A1_MINI —— 清单在我们这边
        let presets = crate::workbench::load_presets().expect("真 presets");
        assert!(
            presets.catalog.machine("A1_MINI").is_some(),
            "真 presets 里没有 A1_MINI，前提没了"
        );

        let tmp = tempfile::tempdir().unwrap();
        let store = crate::workbench::store::Store::at(tmp.path());
        store.bootstrap().unwrap();
        let c = super::super::storage::load(&store, &presets)
            .expect("干净仓库读得通")
            .committed;
        let draft = Draft::default();
        let book = Book::new(&presets, &c, &draft);

        let v = book
            .versions()
            .iter()
            .find(|v| v.machine_id == "A1_MINI")
            .expect("真 presets 里应当有 A1_MINI 的版本");
        let r = render(&book, &v.uid).expect("上游认不认这台机型，都不该影响渲染");

        assert!(
            r.text.contains(&format!("# machine: {}\n", v.machine_id)),
            "`# machine:` 那一行不是我们清单里的 id：{:?}",
            r.text.lines().take(4).collect::<Vec<_>>()
        );
        assert!(
            r.file_name.starts_with("A1_MINI-"),
            "产物名没跟着我们清单里的机型 id 走：{}",
            r.file_name
        );
    }

    /// **生成出来的名字必须正好是消费端认得的那一批。**（b04 §02 契约的第一条）
    ///
    /// 消费端按 `{机型}-{版本小写}.toml` 找文件，它内置的 9 份就是这个形状
    /// （`AUDIT-EVIDENCE.md` §2 逐份列过）。名字对不上的后果不是报错，
    /// 是它**找不到** —— 我们生成了一堆它认不出的文件，而两边都不会说话。
    ///
    /// 这是一条**会随清单变化而红**的判据，而且红了就该看：
    /// 改版本 id 或加机型都会改变这批名字，那时要同步的是消费端的内置表
    #[test]
    fn generated_names_match_what_the_consumer_looks_for() {
        let Some(root) = crate::workbench::paths::presets_root() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let p = crate::workbench::presets::Presets::load_from(&root).unwrap();
        let got: std::collections::BTreeSet<String> = p
            .catalog
            .machines()
            .iter()
            .flat_map(|m| m.versions.iter().map(|v| preset_file_name(&m.id, &v.id)))
            .collect();

        // 消费端 crates/preset/assets/presets/ 下实测的 9 份
        let want: std::collections::BTreeSet<String> = [
            "A1-standard.toml",
            "A1-fast.toml",
            "A1-fastv3.3.toml",
            "A1_MINI-standard.toml",
            "A1_MINI-fast.toml",
            "A1_MINI-fastv3.3.toml",
            "P1S-lite.toml",
            "P2S-standard.toml",
            "X1C-lite.toml",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();

        assert_eq!(
            got, want,
            "产物名与消费端认的那一批不一致 —— 它会找不到文件，而两边都不报错"
        );
    }

    /* ---------- 生成前预演：行级 diff ---------- */

    /// 逐行的 `(kind, 文本)`，方便断言时只写关心的部分。
    fn kinds(lines: &[DiffLine]) -> Vec<(&'static str, &str)> {
        lines
            .iter()
            .map(|l| {
                let k = match l.kind {
                    DiffLineKind::Context => "=",
                    DiffLineKind::Added => "+",
                    DiffLineKind::Removed => "-",
                };
                (k, l.text.as_str())
            })
            .collect()
    }

    #[test]
    fn a_new_file_shows_every_line_as_added() {
        let out = diff_added("a\nb\nc");
        assert_eq!(kinds(&out), vec![("+", "a"), ("+", "b"), ("+", "c")]);
        // 行号从 1 起、连续
        assert_eq!(out.iter().map(|l| l.no).collect::<Vec<_>>(), vec![1, 2, 3]);
    }

    #[test]
    fn an_identical_file_has_no_diff_lines() {
        // diff_lines 只服务「确实要重写」的文件；正文相同（含时间戳差异）的
        // 在 preview_one 就被 same_payload 拦成「无变化」，根本走不到这里
        assert!(diff_lines("x\ny", "x\ny")
            .iter()
            .all(|l| l.kind == DiffLineKind::Context));
    }

    /// 预演的「无变化」与 [`wb_generate`] 的「跳过」是**同一道判据**
    /// （[`same_payload`]）：只有头部时间戳那行不同的，不许报成「要写」——
    /// 否则「将写入 N 份」虚报，作者又被自己吓一跳（2026-10-05 真实吐槽）。
    /// 行级差异仍比真文本：真要重写的，时间戳那行如实显示。
    #[test]
    fn a_timestamp_only_difference_is_unchanged_not_modified() {
        let r = Rendered {
            uid: "A1/STANDARD".to_owned(),
            file_name: "A1-standard.toml".to_owned(),
            text: "# release_time: 1\nk = 1\n".to_owned(),
            fingerprint: "fp".to_owned(),
            snapshot: Default::default(),
        };
        // 正文一样、只有时间戳不同：不写盘，也不许报「要写」
        let same_but_time = "# release_time: 2\nk = 1\n";
        assert_eq!(
            preview_one(&r, Some(same_but_time)).state,
            DiffState::Unchanged
        );
        // 正文真变了：要写
        let changed_body = "# release_time: 1\nk = 2\n";
        assert_eq!(
            preview_one(&r, Some(changed_body)).state,
            DiffState::Modified
        );
        // 磁盘上没有：新增
        assert_eq!(preview_one(&r, None).state, DiffState::Added);
    }

    #[test]
    fn a_changed_line_shows_one_removed_and_one_added() {
        let out = diff_lines("a\nold\nc", "a\nnew\nc");
        // 与 git diff 同一套顺序：删在前、增在后
        assert_eq!(
            kinds(&out),
            vec![("=", "a"), ("-", "old"), ("+", "new"), ("=", "c")],
            "变的那一行要一删一增，前后未变的行保持 context"
        );
    }

    #[test]
    fn an_inserted_line_shows_only_an_added() {
        let out = diff_lines("a\nc", "a\nb\nc");
        assert_eq!(kinds(&out), vec![("=", "a"), ("+", "b"), ("=", "c")]);
    }

    #[test]
    fn a_deleted_line_shows_only_a_removed() {
        let out = diff_lines("a\nb\nc", "a\nc");
        assert_eq!(kinds(&out), vec![("=", "a"), ("-", "b"), ("=", "c")]);
    }

    #[test]
    fn the_counts_match_the_line_kinds() {
        // preview_one 的 added/removed 计数直接数行 —— 与界面上的「+N −N」是同一个数
        let r = Rendered {
            uid: "A1/standard".to_owned(),
            file_name: "A1-standard.toml".to_owned(),
            text: "a\nnew\nc\n".to_owned(),
            fingerprint: String::new(),
            snapshot: BTreeMap::new(),
        };

        let added = preview_one(&r, None);
        assert_eq!(added.state, DiffState::Added);
        assert_eq!((added.added, added.removed), (3, 0));

        let same = preview_one(&r, Some("a\nnew\nc\n"));
        assert_eq!(same.state, DiffState::Unchanged);
        // 无变化也带回**整份正文**（全 context）—— 确认框的「完整」视图要看原文；
        // 「不会重写」那句由界面在**对比**视图说（作者 2026-10-07）
        assert!(
            same.lines.iter().all(|l| l.kind == DiffLineKind::Context),
            "无变化带回的是整份正文，不是差异"
        );
        assert_eq!(same.lines.len(), 3, "正文几行就几行");
        assert_eq!((same.added, same.removed), (0, 0), "没变化就不算增减");

        let changed = preview_one(&r, Some("a\nold\nc\n"));
        assert_eq!(changed.state, DiffState::Modified);
        assert_eq!((changed.added, changed.removed), (1, 1));
    }

    /// **预演一个字节都不写** —— 它就是 `wb_generate` 的彩排。
    ///
    /// 这里用一个真实渲染器（`render`）产出的文本，落到临时目录：跑完预演后，
    /// 磁盘上的字节与预演前**逐字节相同**（新增的那份不会被预演创建出来）。
    #[test]
    fn preview_never_touches_the_disk() {
        let (_d, f, c) = setup();
        let draft = Draft::default();
        let book = Book::new(&f.presets, &c, &draft);
        let r = render(&book, "A1/STANDARD").unwrap();

        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join(&r.file_name);

        // 磁盘上还没有 → 预演判「新增」，但**不会把它写出来**
        let added = preview_one(&r, std::fs::read_to_string(&target).ok().as_deref());
        assert_eq!(added.state, DiffState::Added);
        assert!(!target.exists(), "预演不许创建任何文件");

        // 放一份**不同**的内容进去：预演判「修改」，且原字节一个不动
        crate::fsx::atomic::atomic_write(&target, b"# old\n").unwrap();
        let before = std::fs::read(&target).unwrap();
        let modified = preview_one(&r, std::fs::read_to_string(&target).ok().as_deref());
        assert_eq!(modified.state, DiffState::Modified);
        assert_eq!(
            std::fs::read(&target).unwrap(),
            before,
            "预演不许改任何文件"
        );

        // 放成与渲染结果逐字节相同：预演判「不变」
        crate::fsx::atomic::atomic_write(&target, r.text.as_bytes()).unwrap();
        let unchanged = preview_one(&r, std::fs::read_to_string(&target).ok().as_deref());
        assert_eq!(unchanged.state, DiffState::Unchanged);
    }
}
