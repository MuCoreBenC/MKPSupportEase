//! 生成 / 校验 / 恢复 / 发布（doc §10）。
//!
//! # 生成是原子的
//!
//! **先把全部产物算完，任一项算不出来则整批不动**；全部成功才逐个原子替换。
//! 半批生成的后果是最难查的一种：dist 里有一半新一半旧，而两边都是合法的 TOML，
//! 客户端下载下来也不会报错，只是行为对不上。
//!
//! # 字节没变不重写文件
//!
//! `mtime` 变了会让同步工具以为有更新。所以比较的是**内容**。
//!
//! 但这里有个坑：产物头部有 `release_time`，它每次都不一样，于是"字节比较"永远不相等，
//! 这条规则就永远不生效。所以比较时**跳过那一行**（[`same_payload`]）。
//! 另一半是 `uuid`：它按有效配方的指纹算，不是随机数 —— 随机 uuid 会让同一份配方
//! 每次产出不同的字节，同样把这条规则废掉。
//!
//! # TOML 的形状照上游的真产物
//!
//! 实测 `presets/mkp/A1.toml`：
//!
//! ```toml
//! # uuid: f444aeaf-…
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
use crate::workbench::domain::patch::Patch;
use crate::workbench::domain::wording as w;
use crate::workbench::domain::Level;
use crate::workbench::paths;
use crate::workbench::presets::registry::{ParamDef, UiComponent, ValueType};

use super::{state, with_ctx};

/* ---------- 校验 ---------- */

#[tauri::command]
pub fn wb_preflight() -> Result<Report, AppError> {
    traced("wb_preflight", |_| {
        with_ctx(|ctx| {
            let (c, d, _) = state(ctx)?;
            Ok(issues::inspect(&Book::new(&ctx.up, &ctx.presets, &c, &d)))
        })
    })
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
    let machine = book
        .up
        .catalog
        .machine(&v.machine_id)
        .ok_or_else(|| AppError::not_found(format!("机型 {} 不存在", v.machine_id)))?;

    let fingerprint = layers.fingerprint();
    let recipe = layers.effective_recipe();
    let snapshot: BTreeMap<String, Value> = recipe
        .iter()
        .map(|(k, val)| ((*k).to_owned(), (*val).clone()))
        .collect();

    let mut out = String::new();
    // uuid 按指纹算，**不是随机数** —— 随机的话同一份配方每次产出不同字节，
    // 「字节没变不重写」这条规则就永远不生效
    out.push_str(&format!("# uuid: {}\n", uuid_from(&fingerprint)));
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

/// 按指纹算一个稳定 uuid。格式照 uuid v4 的分段，但**内容是确定的**
fn uuid_from(fingerprint: &str) -> String {
    let h: Vec<char> = fingerprint.chars().take(32).collect();
    let part = |a: usize, b: usize| h[a..b].iter().collect::<String>();
    format!(
        "{}-{}-{}-{}-{}",
        part(0, 8),
        part(8, 12),
        part(12, 16),
        part(16, 20),
        part(20, 32)
    )
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
    /// 生成记录要走 `wb_apply_draft` 落进草稿，所以把 patch 交给前端
    pub mark: Patch,
}

/// 生成。**有阻断时直接拒绝** —— 那是全程唯一的硬闸门
#[tauri::command]
pub fn wb_generate(scope: Scope) -> Result<GenerateReport, AppError> {
    traced("wb_generate", |_| {
        with_ctx(|ctx| {
            let (c, d, _) = state(ctx)?;
            let book = Book::new(&ctx.up, &ctx.presets, &c, &d);

            let report = issues::inspect(&book);
            if let Some(b) = report.first_block() {
                return Err(AppError::invalid_argument(w::disabled::BUILD_BLOCKED)
                    .with_detail(format!("{}：{}", b.title, b.detail)));
            }

            let rows = book.build_rows();
            let wanted: Vec<&str> = match &scope {
                Scope::Stale => rows
                    .iter()
                    .filter(|r| r.buildable)
                    .map(|r| r.uid.as_str())
                    .collect(),
                Scope::All => rows.iter().map(|r| r.uid.as_str()).collect(),
                Scope::Picked(uids) => uids.iter().map(String::as_str).collect(),
            };

            let mut skipped: Vec<(String, String)> = Vec::new();
            let mut todo: Vec<&str> = Vec::new();
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
                    Some(_) => todo.push(uid),
                    None => skipped.push((uid.to_owned(), "这一版不在树上".to_owned())),
                }
            }

            // ① 全部算完。**任一项算不出来则整批不动**
            let mut rendered: Vec<Rendered> = Vec::with_capacity(todo.len());
            for uid in todo {
                rendered.push(render(&book, uid)?);
            }

            // ② 全部成功才逐个原子替换
            let dist = paths::dist_root()?.join("presets").join("mkp");
            let mut written = Vec::new();
            let mut unchanged = Vec::new();
            let mut fingerprints: BTreeMap<String, String> = BTreeMap::new();

            for r in &rendered {
                let target = dist.join(&r.file_name);
                let existing = std::fs::read_to_string(&target).ok();
                if existing
                    .as_deref()
                    .is_some_and(|old| same_payload(old, &r.text))
                {
                    unchanged.push(r.uid.clone());
                } else {
                    crate::fsx::atomic::atomic_write(&target, r.text.as_bytes())?;
                    written.push(r.uid.clone());
                }
                // 快照照样写：它是恢复配方的依据，和有没有重写产物无关
                let v = book.version(&r.uid).expect("刚才渲染过");
                ctx.store.write_doc(
                    &ctx.store.snapshot_rel(&v.machine_id, &v.version_id)?,
                    &r.snapshot,
                )?;
                fingerprints.insert(r.uid.clone(), r.fingerprint.clone());
            }

            let stamp = clock::now_iso8601();
            tracing::info!(
                written = written.len(),
                unchanged = unchanged.len(),
                skipped = skipped.len(),
                "生成完成"
            );
            Ok(GenerateReport {
                mark: Patch::MarkBuilt {
                    uids: fingerprints.keys().cloned().collect(),
                    stamp: stamp.clone(),
                    fingerprints,
                },
                stamp,
                written,
                unchanged,
                skipped,
            })
        })
    })
}

/// 单独看一份产物的文本（生成前确认、看差异都用它）
#[tauri::command]
pub fn wb_preview_toml(uid: String) -> Result<String, AppError> {
    traced("wb_preview_toml", |_| {
        with_ctx(|ctx| {
            let (c, d, _) = state(ctx)?;
            Ok(render(&Book::new(&ctx.up, &ctx.presets, &c, &d), &uid)?.text)
        })
    })
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
#[tauri::command]
pub fn wb_revert_preview(uid: String) -> Result<RevertPreview, AppError> {
    traced("wb_revert_preview", |_| {
        with_ctx(|ctx| {
            let (c, d, _) = state(ctx)?;
            let book = Book::new(&ctx.up, &ctx.presets, &c, &d);
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
    /// 上游没声明最低客户端版本时是 `None`，界面写「未声明」。**不编一个版本号出来**
    pub minimum_client: Option<String>,
    /// 待办与提示**不挡发布**，但要在报告里列出来
    pub todos: usize,
    pub hints: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DistAsset {
    id: String,
    resource_type: String,
    machine_id: String,
    file_name: String,
    relative_path: String,
    sha256: String,
    size: u64,
}

/// 发布：把 `dist-presets/` 里的产物连同清单一起定稿。
///
/// **清单最后写**：先写资源、最后写指向它们的清单。反过来的话，中途失败会留下一份
/// 指向不存在文件的清单，而客户端读到它只会 404 —— 那种失败在用户机器上才出现
#[tauri::command]
pub fn wb_publish() -> Result<PublishReport, AppError> {
    traced("wb_publish", |_| {
        with_ctx(|ctx| {
            let (c, d, _) = state(ctx)?;
            let book = Book::new(&ctx.up, &ctx.presets, &c, &d);
            let report = issues::inspect(&book);
            if let Some(b) = report.first_block() {
                return Err(AppError::invalid_argument("有阻断问题没解决，不能发布")
                    .with_detail(format!("{}：{}", b.title, b.detail)));
            }

            let root = paths::dist_root()?;
            let mkp = root.join("presets").join("mkp");
            let mut assets: Vec<DistAsset> = Vec::new();

            for v in book.versions() {
                let Some(p) = &v.mkp_preset else {
                    continue; // 暂无资源：跳过，不报错
                };
                let path = mkp.join(preset_file_name(&v.machine_id, &v.version_id));
                let Ok(bytes) = std::fs::read(&path) else {
                    return Err(AppError::not_found(format!("{} 的产物还没生成", v.name))
                        .with_detail(format!(
                            "{} 不存在。先在生成视角里生成，再发布",
                            path.display()
                        )));
                };
                assets.push(DistAsset {
                    // 哈希**按发布出去的那份字节算**，不抄上游的 —— 抄了就等于声明
                    // 一个我们没验证过的哈希
                    sha256: sha256_of(&bytes),
                    size: bytes.len() as u64,
                    id: p.asset_id.clone(),
                    resource_type: "mkp_preset".to_owned(),
                    machine_id: v.machine_id.clone(),
                    file_name: preset_file_name(&v.machine_id, &v.version_id),
                    relative_path: format!("presets/mkp/{}", p.file_name),
                });
            }

            let stamp = clock::now_iso8601();
            let manifest = serde_json::json!({
                "manifestVersion": 2,
                "channel": ctx.up.manifest.compat.channel,
                "updated": stamp,
                // **上游未声明就照实留空**，不编一个版本号出来（doc §12）
                "minimumClient": ctx.up.manifest.compat.minimum_client.clone().unwrap_or_default(),
                "version": ctx.up.manifest.compat.version.clone().unwrap_or_default(),
                "assets": assets,
                "bundles": ctx.up.manifest.bundles(),
            });
            // 清单最后写。`fsx::atomic` 是仓库唯一的写盘出口
            crate::fsx::atomic::atomic_write_json(&root.join("manifest.json"), &manifest)?;

            tracing::info!(files = assets.len(), at = %stamp, "发布完成");
            Ok(PublishReport {
                stamp,
                root: root.display().to_string(),
                files: assets.len(),
                minimum_client: ctx.up.manifest.compat.minimum_client.clone(),
                todos: report.todos,
                hints: report.hints,
            })
        })
    })
}

fn sha256_of(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::domain::patch::{apply, Committed, CommittedVersion, Draft};
    use crate::workbench::domain::testkit::{fixture_catalog, Fixture};
    use crate::workbench::store::Store;

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
        let book = Book::new(&f.up, &f.presets, &c, &draft);
        let r = render(&book, "A1/STANDARD").unwrap();

        assert!(r.text.starts_with("# uuid: "));
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
        // 三个共享 tomlKey 的参数合成内联表，成员名取 jsonKey。
        // **整数不写小数点** —— 上游真产物就是 `offset = { x = -1, y = 18.6, z = 4 }`
        assert_eq!(
            toolhead["offset"]["x"].as_integer(),
            Some(-1),
            "A1:STANDARD 的上游覆盖"
        );
        assert_eq!(toolhead["offset"]["y"].as_float(), Some(18.6));
        assert_eq!(toolhead["offset"]["z"].as_integer(), Some(4));
        // G-code 走多行字符串
        assert!(toolhead["script"].as_str().is_some());
        assert_eq!(parsed["wiping"]["mode"].as_str(), Some("tower"));
        // 行尾注释要在
        assert!(
            r.text.contains("} # 笔尖偏移"),
            "内联表的注释丢了：\n{}",
            r.text
        );
    }

    /// **M0：我们渲染出来的产物 vs 真机验证过的那份基线。**（b04 Task 11）
    ///
    /// 这是整个迁移的前置判据。基线是 `mkp-ssr` 里那 9 份内置预设之一 ——
    /// 它们由那边的 `gen-presets` 从 `preset_recipes.toml` 生成，**上过真机**。
    /// 我们这条链（`presets/*.toml` → `render()`）算出来的如果与它正文字节相同，
    /// 说明两套真源等值、迁移不需要修数据；不同就必须先定哪边对 ——
    /// 搬完 3 万行代码再发现值对不上，会留下一批"生成出来但和验证过的不一样"的产物，
    /// 而那种错在产物上看不出来。
    ///
    /// 比的是**正文**：`uuid` 与 `release_time` 两行按定义就该不同
    /// （前者按内容指纹算、后者是当下时间），它们不参与比较。
    ///
    /// 基线路径是兄弟仓库，找不到就跳过 —— 这条判据的寿命到 Task 22.2 为止
    /// （那时基线会被删掉，因为那时只有我们一份）。
    #[test]
    fn our_render_matches_the_machine_verified_baseline() {
        let (Some(_), Some(_)) = (paths::presets_root(), paths::upstream_root()) else {
            eprintln!("没同时定位到 presets 与上游，这条 M0 检查未执行（不是通过）");
            return;
        };
        let baseline_dir = paths::repo_root().join("../mkp-ssr/crates/preset/assets/presets");
        if !baseline_dir.is_dir() {
            eprintln!(
                "没找到基线目录 {}，这条 M0 检查未执行（不是通过）",
                baseline_dir.display()
            );
            return;
        }

        let up = crate::workbench::upstream::Upstream::load().expect("真上游");
        let presets = crate::workbench::presets::Presets::load().expect("真 presets");
        let tmp = tempfile::tempdir().unwrap();
        let store = crate::workbench::store::Store::at(tmp.path());
        store.bootstrap().unwrap();
        let c = super::super::storage::load(&store, &presets)
            .expect("干净仓库读得通")
            .committed;
        let draft = Draft::default();
        let book = Book::new(&up, &presets, &c, &draft);

        /// 去掉那两行按定义会变的头部
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
        for v in book.versions() {
            let name = preset_file_name(&v.machine_id, &v.version_id);
            let path = baseline_dir.join(&name);
            let Ok(want) = std::fs::read_to_string(&path) else {
                continue; // 基线里没有这一份（A2L 就是这样）—— 不是差异
            };
            let got = render(&book, &v.uid).expect("渲染得出来").text;
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

        assert!(compared > 0, "一份都没比到，这条判据在空转");
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

    /// 同样的输入**产出同样的字节**（除了时间戳那一行）——
    /// uuid 用随机数的话这条就不成立，而「字节没变不重写」也就废了
    #[test]
    fn rendering_is_deterministic_apart_from_the_timestamp() {
        let (_d, f, c) = setup();
        let draft = Draft::default();
        let book = Book::new(&f.up, &f.presets, &c, &draft);
        let a = render(&book, "A1/STANDARD").unwrap();
        let b = render(&book, "A1/STANDARD").unwrap();
        assert!(same_payload(&a.text, &b.text));
        assert_eq!(a.fingerprint, b.fingerprint);
        assert_eq!(uuid_from(&a.fingerprint).len(), 36);
    }

    /// 改一个值 → 产物真的变了（不然「待生成」是句空话）
    #[test]
    fn changing_a_value_changes_the_output() {
        let (_d, f, c) = setup();
        let mut draft = Draft::default();
        let before = render(&Book::new(&f.up, &f.presets, &c, &draft), "A1/STANDARD").unwrap();

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
        let after = render(&Book::new(&f.up, &f.presets, &c, &draft), "A1/STANDARD").unwrap();

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
        let book = Book::new(&f.up, &f.presets, &c, &draft);
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
        let book = Book::new(&f.up, &f.presets, &c, &draft);

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
        let book = Book::new(&f.up, &f.presets, &c, &draft);

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

    /// **生成出来的名字必须正好是消费端认得的那一批。**（b04 §02 契约的第一条）
    ///
    /// 消费端按 `{机型}-{版本小写}.toml` 找文件，它内置的 9 份就是这个形状
    /// （`AUDIT-EVIDENCE.md` §2 逐份列过）。名字对不上的后果不是报错，
    /// 是它**找不到** —— 我们生成了一堆它认不出的文件，而两边都不会说话。
    ///
    /// 这是一条**会随清单变化而红**的判据，而且红了就该看：
    /// 改版本 id 或加机型都会改变这批名字，那时要同步的是消费端的内置表。
    /// 期望值里多出的 `A2L-standard.toml` 是占位机型 —— 它不会被生成，
    /// 但名字口径照样成立
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

        // 消费端 crates/preset/assets/presets/ 下实测的 9 份 + 占位的 A2L
        let want: std::collections::BTreeSet<String> = [
            "A1-standard.toml",
            "A1-fast.toml",
            "A1-fastv3.3.toml",
            "A1_MINI-standard.toml",
            "A1_MINI-fast.toml",
            "A1_MINI-fastv3.3.toml",
            "A2L-standard.toml",
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
}
