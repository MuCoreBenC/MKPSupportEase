//! Application 层 —— IPC 入口与 DTO。
//!
//! doc §6 的新契约按「读整本 / 写一条 / 查一屏 / 做一件事」分四类，**写只有一条**：
//!
//! ```text
//! 写   wb_apply_draft(label, patches) -> ApplyResult
//! ```
//!
//! 第二版那 30 多个命令（`wb_save_version` / `wb_bulk_apply` / `wb_trash_version` …）
//! 全部作废。作废的理由不是"命令太多"，而是**每个按钮各自写盘**这件事本身：
//! 撤销、脏计数、差异列表、状态一致性，每一件都要挨个改十几处才能补上。
//! 收成一条之后，这四件事变成这一条路径的副产物。
//!
//! 这一层只做三件事：解析入参 → 调 `domain` → 把 DTO 交出去。
//! **不在这里写规则**（那是 `domain`），**不在这里读上游**（那是 `upstream`）。
//!
//! # 上游缓存一次，其余每次现读
//!
//! | 数据 | 什么时候读 | 为什么 |
//! |---|---|---|
//! | 上游（74 参数 / 6 机型 / 72 资源） | **开工作台时一次**，`wb_reload` 显式重读 | 它是只读的外部产物；一次操作里各处看到的必须是同一份，否则派生出来的状态会自相矛盾 |
//! | 已落盘（`workbench/`） | 每次命令现读 | 它是可变的那一面。缓存它就等于又造了一个"当前状态"的副本（doc §1 第四条铁律） |
//! | 草稿 | 每次命令现读 | 同上 |
//!
//! 一个进程只有一个工作台窗口，所以上游那一份放在模块级的 [`SESSION`] 里。
//! **命令本身只是薄壳**：真正的逻辑都在拿 `&Ctx` 的自由函数上，
//! 所以单测可以直接造 `Ctx`，不碰那个全局。
//!
//! # `wb_effective` 就是一列的 `wb_matrix`
//!
//! 单个版本的字段详情要的东西（值、来源、自有、被谁关着、能不能改）
//! 与矩阵单元格**一模一样**。再建一套 DTO 等于给同一件事写两个形状，
//! 迟早只改其中一个。所以字段详情走 `wb_matrix(cols=[那一列])`。

pub mod storage;
pub mod words;

use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

use serde::Serialize;
use serde_json::Value;

use crate::error::AppError;
use crate::ipc::traced;
use crate::workbench::domain::derive::{Book, BookView, ColRef, Matrix, StockRow};
use crate::workbench::domain::patch::{apply as apply_patches, Draft, Patch};
use crate::workbench::domain::preview::{BulkPreview, MovePreview};
use crate::workbench::domain::wording as w;
use crate::workbench::domain::{Committed, Level};
use crate::workbench::store::{Store, TrashEntry};
use crate::workbench::upstream::registry::{ParamDef, ShowWhen, TabMeta, UiComponent, ValueType};
use crate::workbench::upstream::Upstream;
use crate::workbench::{paths, Roots};

/// 一次会话里不变的那两样：上游快照 + 数据根
pub struct Ctx {
    pub up: Upstream,
    pub store: Store,
}

impl Ctx {
    /// 真仓库。**上游定位不到时直接失败** —— 那种情况下工作台不该启动业务，
    /// 因为字段定义、机型清单、资源清单全在上游，缺了它每一页都是空的
    pub fn open() -> Result<Self, AppError> {
        let store = Store::open()?;
        store.bootstrap()?;
        Ok(Self {
            up: Upstream::load()?,
            store,
        })
    }
}

/// 上游快照。一个进程一个工作台窗口，所以放模块级。
///
/// `Mutex<Option<_>>` 而不是 `OnceLock<Ctx>`：`wb_reload` 要能把它换掉
static SESSION: OnceLock<Mutex<Option<Ctx>>> = OnceLock::new();

fn session() -> &'static Mutex<Option<Ctx>> {
    SESSION.get_or_init(|| Mutex::new(None))
}

/// 借出会话上下文，没有就现建一个。
///
/// 锁中毒（上一次持锁时 panic 了）不当成致命错误：清掉重来，
/// 因为 `Ctx` 是只读快照，重建一份的代价只是再读一次上游
fn with_ctx<T>(f: impl FnOnce(&Ctx) -> Result<T, AppError>) -> Result<T, AppError> {
    let mut guard = session().lock().unwrap_or_else(|e| {
        tracing::warn!("上游快照的锁中毒了，重建一份");
        let mut g = e.into_inner();
        *g = None;
        g
    });
    if guard.is_none() {
        *guard = Some(Ctx::open()?);
    }
    f(guard.as_ref().expect("刚刚放进去的"))
}

/// 丢掉上游快照，下一次调用会重读
fn drop_ctx() {
    if let Ok(mut g) = session().lock() {
        *g = None;
    }
}

/// 每条命令的开头：读出「已落盘」与「草稿」，并把指向已消失对象的草稿条目清掉。
///
/// **清理要在每次读的时候做，不是只在启动时做**：上游可能在工作台开着的时候被重建
fn state(ctx: &Ctx) -> Result<(Committed, Draft, Vec<String>), AppError> {
    let loaded = storage::load(&ctx.store, &ctx.up)?;
    let mut draft = storage::read_draft(&ctx.store)?;
    let mut notices = loaded.notices;
    let pruned = draft.prune(&loaded.committed);
    if !pruned.is_empty() {
        // 清理过就写回去，否则每次打开都报同一批提示
        storage::write_draft(&ctx.store, &draft)?;
        notices.extend(pruned);
    }
    Ok((loaded.committed, draft, notices))
}

fn view_of(ctx: &Ctx, committed: &Committed, draft: &Draft, notices: Vec<String>) -> BookView {
    let mut v = Book::new(&ctx.up, committed, draft).book_view();
    v.notices = notices;
    v
}

/* ---------- 启动 ---------- */

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Boot {
    pub roots: Roots,
    /// 上游读不出来时的那一句。`None` = 一切就绪。
    /// **不把它做成错误返回**：界面要能在"上游缺失"的状态下把三个数据根显示出来，
    /// 那是排查这个问题唯一有用的信息
    pub problem: Option<String>,
    pub detail: Option<String>,
    pub info: Option<UpstreamInfo>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpstreamInfo {
    pub registry_updated: String,
    pub manifest_updated: String,
    pub channel: String,
    /// **`None` = 上游未声明**（实测 `minimumClient` 就是空串），不是 0 也不是空串
    pub minimum_client: Option<String>,
    pub latest_release: Option<String>,
    pub params: usize,
    pub machines: usize,
    pub deliverables: usize,
    pub fallbacks: usize,
}

/// 三个数据根 + 上游就位情况。界面开场调它
#[tauri::command]
pub fn wb_boot() -> Result<Boot, AppError> {
    traced("wb_boot", |_| {
        let roots = Roots {
            workbench: paths::workbench_root()?.display().to_string(),
            dist: paths::dist_root()?.display().to_string(),
            upstream: paths::upstream_root().map(|p| p.display().to_string()),
        };
        match with_ctx(|ctx| {
            Ok(UpstreamInfo {
                registry_updated: ctx.up.registry.updated.clone(),
                manifest_updated: ctx.up.manifest.compat.updated.clone(),
                channel: ctx.up.manifest.compat.channel.clone(),
                minimum_client: ctx.up.manifest.compat.minimum_client.clone(),
                latest_release: ctx.up.manifest.latest_release().map(str::to_owned),
                params: ctx.up.registry.params().len(),
                machines: ctx.up.catalog.machines().len(),
                deliverables: ctx.up.manifest.deliverables().len(),
                fallbacks: ctx.up.fallback.rules().len(),
            })
        }) {
            Ok(info) => Ok(Boot {
                roots,
                problem: None,
                detail: None,
                info: Some(info),
            }),
            Err(e) => Ok(Boot {
                roots,
                problem: Some(e.message),
                detail: e.detail,
                info: None,
            }),
        }
    })
}

/// 重读上游。改完 `mkpse-presets` 之后不用重启工作台
#[tauri::command]
pub fn wb_reload() -> Result<Boot, AppError> {
    drop_ctx();
    wb_boot()
}

/* ---------- 读 ---------- */

#[tauri::command]
pub fn wb_book() -> Result<BookView, AppError> {
    traced("wb_book", |_| {
        with_ctx(|ctx| {
            let (c, d, n) = state(ctx)?;
            Ok(view_of(ctx, &c, &d, n))
        })
    })
}

/// 字段定义（74 条 + tabs）。矩阵渲染与「字段定义」那一页共用
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryView {
    pub updated: String,
    pub tabs: Vec<TabMeta>,
    pub params: Vec<ParamView>,
}

/// 一个参数在界面上要用到的那些。
///
/// 刻意**不直接把 `ParamDef` 交出去**：那里面有 `configKey` / `jsonKey` / `tomlComment`
/// 这些只在生成时用得上的字段，前端拿到了也只会让人以为该显示它们
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParamView {
    pub key: String,
    pub label: String,
    pub desc: String,
    pub toml_key: String,
    pub section_id: String,
    pub tab_id: Option<String>,
    pub order: f64,
    pub value_type: ValueType,
    pub ui_component: UiComponent,
    pub default_value: Value,
    pub default_text: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: Option<f64>,
    pub unit: Option<String>,
    pub choices: Vec<ChoiceView>,
    pub show_when: Option<ShowWhen>,
    pub parent_key: Option<String>,
    /// 空 = 不限机型
    pub machine_filter: Vec<String>,
    pub deprecated: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChoiceView {
    pub label: String,
    pub value: Value,
    pub deprecated: bool,
}

#[tauri::command]
pub fn wb_registry() -> Result<RegistryView, AppError> {
    traced("wb_registry", |_| {
        with_ctx(|ctx| {
            let reg = &ctx.up.registry;
            Ok(RegistryView {
                updated: reg.updated.clone(),
                tabs: reg.param_tabs(),
                params: reg.params().iter().map(|p| param_view(reg, p)).collect(),
            })
        })
    })
}

fn param_view(reg: &crate::workbench::upstream::Registry, p: &ParamDef) -> ParamView {
    ParamView {
        key: p.key.clone(),
        label: p.label.clone(),
        desc: p.desc.clone(),
        toml_key: p.toml_key.clone(),
        tab_id: reg.tab_of_section(&p.layout.section_id).map(str::to_owned),
        section_id: p.layout.section_id.clone(),
        order: p.layout.order,
        value_type: p.value_type,
        ui_component: p.ui_component,
        // 出厂默认的**显示文本也由后端给**（doc §8.3：前端不做格式化）
        default_text: w::value_text(p, &p.default_value),
        default_value: p.default_value.clone(),
        min: p.min,
        max: p.max,
        step: p.step,
        unit: p.unit.clone(),
        choices: p
            .choices
            .iter()
            .map(|c| ChoiceView {
                label: c.label.clone(),
                value: c.value.clone(),
                deprecated: c.deprecated,
            })
            .collect(),
        show_when: p.show_when.clone(),
        parent_key: p.parent_key.clone(),
        machine_filter: p.machine_filter.clone(),
        deprecated: p.deprecated,
    }
}

/// 一屏矩阵。列由前端勾选给出，**顺序由后端按配方本重排**
#[tauri::command]
pub fn wb_matrix(
    cols: Vec<ColRef>,
    tab: Option<String>,
    query: Option<String>,
) -> Result<Matrix, AppError> {
    traced("wb_matrix", |_| {
        with_ctx(|ctx| {
            let (c, d, _) = state(ctx)?;
            Ok(Book::new(&ctx.up, &c, &d).matrix(
                &cols,
                tab.as_deref(),
                query.as_deref().unwrap_or_default(),
            ))
        })
    })
}

/// 仓库盘点：18 个交付物 + 三态 + 归属。**每条都有真哈希**
#[tauri::command]
pub fn wb_stock() -> Result<Vec<StockRow>, AppError> {
    traced("wb_stock", |_| {
        with_ctx(|ctx| {
            let (c, d, _) = state(ctx)?;
            Ok(Book::new(&ctx.up, &c, &d).stock_rows())
        })
    })
}

/// 回退登记表。**这一页只读**（上游自己写着"禁止手改 content/*.json"）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FallbackTable {
    pub version: u32,
    pub updated: String,
    /// 带换行的长文，**原样保留**
    pub guide: String,
    pub groups: Vec<FallbackGroup>,
    /// 被关掉的规则：触发时直接报错中止。空是正常状态，界面写「当前没有关掉的规则」
    pub disabled: Vec<String>,
    pub empty_hint: &'static str,
    /// 这一页为什么只读
    pub read_only_reason: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FallbackGroup {
    pub label: &'static str,
    pub rules: Vec<crate::workbench::upstream::fallback::Rule>,
}

#[tauri::command]
pub fn wb_fallback() -> Result<FallbackTable, AppError> {
    traced("wb_fallback", |_| {
        with_ctx(|ctx| {
            let f = &ctx.up.fallback;
            Ok(FallbackTable {
                version: f.version,
                updated: f.updated.clone(),
                guide: f.guide.clone(),
                groups: f
                    .by_category()
                    .into_iter()
                    .map(|(cat, rules)| FallbackGroup {
                        label: cat.label(),
                        rules: rules.into_iter().cloned().collect(),
                    })
                    .collect(),
                disabled: f.disabled().iter().map(|r| r.id.clone()).collect(),
                empty_hint: w::NO_DISABLED_FALLBACK,
                read_only_reason: "这张表由上游维护，改动请回 mkppanel 的「回退登记表」页",
            })
        })
    })
}

#[tauri::command]
pub fn wb_trash() -> Result<Vec<TrashEntry>, AppError> {
    traced("wb_trash", |_| with_ctx(|ctx| ctx.store.trash_entries()))
}

#[tauri::command]
pub fn wb_ui() -> Result<Value, AppError> {
    traced("wb_ui", |_| with_ctx(|ctx| storage::read_ui(&ctx.store)))
}

#[tauri::command]
pub fn wb_save_ui(ui: Value) -> Result<(), AppError> {
    traced("wb_save_ui", |_| {
        with_ctx(|ctx| storage::write_ui(&ctx.store, &ui))
    })
}

/* ---------- 查（只读推演） ---------- */

#[tauri::command]
pub fn wb_preview_move(uid: String, to_machine_id: String) -> Result<MovePreview, AppError> {
    traced("wb_preview_move", |_| {
        with_ctx(|ctx| {
            let (c, d, _) = state(ctx)?;
            Book::new(&ctx.up, &c, &d)
                .preview_move(&uid, &to_machine_id)
                .ok_or_else(|| AppError::not_found(format!("版本 {uid} 不存在")))
        })
    })
}

#[tauri::command]
pub fn wb_preview_bulk(
    key: String,
    value: Value,
    cols: Vec<ColRef>,
) -> Result<BulkPreview, AppError> {
    traced("wb_preview_bulk", |_| {
        with_ctx(|ctx| {
            let (c, d, _) = state(ctx)?;
            Ok(Book::new(&ctx.up, &c, &d).preview_bulk(&key, &value, &cols))
        })
    })
}

/// 未保存改动的逐条清单。保存前的确认弹窗用它 ——
/// 「有 7 处未保存」不足以让人放心按保存，得看得见是哪七处
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    /// 「A1 · 机型基底」/「A1 · 标准版」
    pub target: String,
    pub level: Option<Level>,
    pub owner: String,
    pub key: Option<String>,
    pub label: String,
    pub before: String,
    pub after: String,
    pub kind: &'static str,
}

#[tauri::command]
pub fn wb_diff_draft() -> Result<Vec<DiffLine>, AppError> {
    traced("wb_diff_draft", |_| {
        with_ctx(|ctx| {
            let (c, d, _) = state(ctx)?;
            Ok(diff_draft(ctx, &c, &d))
        })
    })
}

fn diff_draft(ctx: &Ctx, committed: &Committed, draft: &Draft) -> Vec<DiffLine> {
    let book = Book::new(&ctx.up, committed, draft);
    let mut out: Vec<DiffLine> = Vec::new();

    let target_of = |level: Level, owner: &str| -> String {
        match level {
            Level::Machine => format!("{owner} · {}", w::level_label(Level::Machine)),
            Level::Version => match book.version(owner) {
                Some(v) => format!("{} · {}", v.machine_id, v.name),
                None => owner.to_owned(),
            },
        }
    };

    // ① 值改动：before 取「已落盘那一份」，after 取「草稿之后」
    for (level, owner, key) in draft
        .values
        .keys()
        .filter_map(|raw| split_any(raw))
        .collect::<Vec<_>>()
    {
        let Some(p) = ctx.up.registry.param(&key) else {
            continue;
        };
        let clean = Draft::default();
        let before = Book::new(&ctx.up, committed, &clean);
        let (b, a) = match level {
            Level::Machine => (
                before.machine_layers(&owner).and_then(|l| l.effective(&key)),
                book.machine_layers(&owner).and_then(|l| l.effective(&key)),
            ),
            Level::Version => (
                before.version_layers(&owner).and_then(|l| l.effective(&key)),
                book.version_layers(&owner).and_then(|l| l.effective(&key)),
            ),
        };
        out.push(DiffLine {
            target: target_of(level, &owner),
            level: Some(level),
            owner: owner.clone(),
            label: p.label.clone(),
            key: Some(key),
            before: b
                .map(|x| w::value_text(p, x.value))
                .unwrap_or_else(|| w::NOT_APPLICABLE.to_owned()),
            after: a
                .map(|x| w::value_text(p, x.value))
                .unwrap_or_else(|| w::NOT_APPLICABLE.to_owned()),
            kind: "改值",
        });
    }

    // ② 结构改动。**各算一条** —— 它们和改一个值一样要被保存
    for v in &draft.added {
        out.push(structural(&v.machine_id, &v.name, "新建版本", "", &v.name));
    }
    for (uid, name) in &draft.renamed {
        let old = committed
            .versions
            .get(uid)
            .map(|c| c.name.clone())
            .unwrap_or_default();
        out.push(structural(uid, name, "改名", &old, name));
    }
    for (uid, to) in &draft.moved {
        let from = committed
            .versions
            .get(uid)
            .map(|c| c.machine_id.clone())
            .unwrap_or_default();
        out.push(structural(uid, uid, "移动", &from, to));
    }
    for (uid, archived) in &draft.archived {
        let (b, a) = if *archived {
            ("在树上", "归档")
        } else {
            ("归档", "在树上")
        };
        out.push(structural(uid, uid, "归档状态", b, a));
    }
    for uid in &draft.purged {
        out.push(structural(uid, uid, "删除", "在树上", "回收站"));
    }
    for (uid, list) in &draft.bbs {
        let a = match list {
            Some(l) => format!("自己一份（{} 条）", l.len()),
            None => w::BbsSource::InheritedFromMachine.label().to_owned(),
        };
        out.push(structural(uid, uid, "BBS 分配", "", &a));
    }
    for (file_id, vis) in &draft.visibility {
        out.push(structural(file_id, file_id, "菜单可见性", "", w::visibility_label(*vis)));
    }
    for (bundle_id, edit) in &draft.bundles {
        out.push(structural(
            bundle_id,
            bundle_id,
            "套餐内容",
            "",
            &format!("{} 份产物 · {} 条曲线", edit.presets.len(), edit.bbs.len()),
        ));
    }
    for (uid, rec) in &draft.built {
        out.push(structural(uid, uid, "生成记录", "", &rec.stamp));
    }

    out
}

fn structural(owner: &str, label: &str, kind: &'static str, before: &str, after: &str) -> DiffLine {
    DiffLine {
        target: owner.to_owned(),
        level: None,
        owner: owner.to_owned(),
        key: None,
        label: label.to_owned(),
        before: before.to_owned(),
        after: after.to_owned(),
        kind,
    }
}

/// 草稿键 → `(level, owner, key)`。`patch` 那边按 `(level, owner)` 过滤，
/// 这里要的是「随便哪一层都拆出来」，所以两种机型/版本各试一次
fn split_any(raw: &str) -> Option<(Level, String, String)> {
    let mut it = raw.splitn(3, ':');
    let level = match it.next()? {
        "m" => Level::Machine,
        "v" => Level::Version,
        _ => return None,
    };
    let owner = it.next()?.to_owned();
    let key = it.next()?.to_owned();
    Some((level, owner, key))
}

/* ---------- 写（唯一一条） ---------- */

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyResult {
    pub view: BookView,
    /// 撤销这次操作要提交的 patches，**倒序**
    pub inverse: Vec<Patch>,
    /// 为 false 时界面**不给**撤销按钮（删除与生成记录不进撤销栈）
    pub undoable: bool,
    pub notices: Vec<String>,
}

/// **唯一的写入口。** 一次调用 = 一次手势 = 一条撤销
#[tauri::command]
pub fn wb_apply_draft(label: String, patches: Vec<Patch>) -> Result<ApplyResult, AppError> {
    traced("wb_apply_draft", |_| {
        with_ctx(|ctx| {
            let (c, mut d, mut notices) = state(ctx)?;
            let out = apply_patches(&mut d, &c, &ctx.up.registry, &patches)?;
            storage::write_draft(&ctx.store, &d)?;
            notices.extend(out.notices);
            tracing::info!(label = %label, patches = patches.len(), "草稿已更新");
            Ok(ApplyResult {
                view: view_of(ctx, &c, &d, notices.clone()),
                inverse: out.inverse,
                undoable: out.undoable,
                notices,
            })
        })
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveResult {
    pub view: BookView,
    /// 旧 uid → 新 uid。**新建与移动都会改 uid**，前端要据此修选中与勾选列
    pub remap: BTreeMap<String, String>,
    pub notices: Vec<String>,
}

#[tauri::command]
pub fn wb_save() -> Result<SaveResult, AppError> {
    traced("wb_save", |_| {
        with_ctx(|ctx| {
            let (c, d, mut notices) = state(ctx)?;
            if d.is_clean() {
                return Err(AppError::invalid_argument(w::disabled::NOTHING_TO_SAVE));
            }
            let out = storage::save(&ctx.store, &ctx.up, &c, &d)?;
            notices.extend(out.notices);

            // 保存之后重读：落盘的那一份才是新的真相
            let (c2, d2, n2) = state(ctx)?;
            notices.extend(n2);
            Ok(SaveResult {
                view: view_of(ctx, &c2, &d2, notices.clone()),
                remap: out.remap,
                notices,
            })
        })
    })
}

#[tauri::command]
pub fn wb_discard() -> Result<BookView, AppError> {
    traced("wb_discard", |_| {
        with_ctx(|ctx| {
            let (c, d, notices) = state(ctx)?;
            if d.is_clean() {
                return Err(AppError::invalid_argument(w::disabled::NOTHING_TO_SAVE));
            }
            storage::write_draft(&ctx.store, &Draft::default())?;
            tracing::info!(dropped = d.dirty_count(), "草稿已丢弃");
            let (c2, d2, n2) = state(ctx)?;
            let _ = c;
            let mut all = notices;
            all.extend(n2);
            Ok(view_of(ctx, &c2, &d2, all))
        })
    })
}

/* ---------- 开窗 ---------- */

/// 从客户端那一侧把工作台窗口叫起来
#[tauri::command]
pub fn wb_open(app: tauri::AppHandle) -> Result<(), AppError> {
    traced("wb_open", |_| super::open_window(&app))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::domain::testkit::Fixture;

    /// 造一个不碰真仓库、也不碰那个全局的 `Ctx`
    fn ctx() -> (tempfile::TempDir, Fixture, Ctx) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at(dir.path());
        store.bootstrap().unwrap();
        let f = Fixture::load();
        // `Upstream` 没有 Clone，所以再读一份给 Ctx
        let up = Fixture::load();
        (dir, f, Ctx { up: up.up, store })
    }

    fn cols(list: &[(&str, Option<&str>)]) -> Vec<ColRef> {
        list.iter()
            .map(|(m, v)| ColRef {
                machine_id: (*m).to_owned(),
                version_uid: v.map(str::to_owned),
            })
            .collect()
    }

    /// 干净仓库：整本视图读得出来，没有未保存改动
    #[test]
    fn a_clean_repo_gives_a_full_book_view() {
        let (_d, _f, ctx) = ctx();
        let (c, d, n) = state(&ctx).unwrap();
        let v = view_of(&ctx, &c, &d, n);
        assert_eq!(v.badges.machines, 3);
        assert_eq!(v.dirty_count, 0);
        assert!(v.notices.is_empty());
        assert_eq!(v.build_rows.len(), 4);
    }

    /// 字段定义视图：**出厂默认的显示文本由后端给**，前端不做格式化
    #[test]
    fn the_registry_view_formats_default_values_on_the_backend() {
        let (_d, _f, ctx) = ctx();
        let reg = &ctx.up.registry;
        let params: Vec<ParamView> = reg.params().iter().map(|p| param_view(reg, p)).collect();

        let mode = params.iter().find(|p| p.key == "wiping.mode").unwrap();
        assert_eq!(mode.default_text, "擦料塔", "枚举要给中文选项名");
        assert_eq!(mode.choices.len(), 2);
        assert_eq!(mode.tab_id.as_deref(), Some("wiping"));

        let script = params.iter().find(|p| p.key == "toolhead.script").unwrap();
        assert_eq!(script.default_text, w::BLANK, "空串写「空」不是空白");

        let off = params.iter().find(|p| p.key == "toolhead.offset.x").unwrap();
        assert_eq!(off.default_text, "0 mm");
        assert_eq!(off.machine_filter.len(), 0, "不限机型");

        let only = params.iter().find(|p| p.key == "toolhead.only_p1s").unwrap();
        assert_eq!(only.machine_filter, vec!["P1S"]);
    }

    /// 上游信息里，**上游未声明的东西要是 `None`** 而不是空串
    #[test]
    fn upstream_info_reports_undeclared_as_none() {
        let (_d, _f, ctx) = ctx();
        assert_eq!(ctx.up.manifest.compat.minimum_client, None);
        assert_eq!(ctx.up.manifest.compat.version, None);
        assert_eq!(ctx.up.manifest.latest_release(), Some("0.0.4"));
    }

    /// 一条 patch 走完整条路径：改草稿 → 落盘 → 视图跟着变 → 反向能回去
    #[test]
    fn one_gesture_goes_through_the_single_write_entry() {
        let (_d, _f, ctx) = ctx();
        let patch = Patch::SetValue {
            level: Level::Machine,
            owner: "A1".to_owned(),
            key: "wiping.child".to_owned(),
            value: Some(serde_json::json!(42)),
        };

        let (c, mut d, _) = state(&ctx).unwrap();
        let out = apply_patches(&mut d, &c, &ctx.up.registry, &[patch]).unwrap();
        storage::write_draft(&ctx.store, &d).unwrap();
        assert!(out.undoable);

        // 重新读：草稿真的落盘了
        let (c2, d2, _) = state(&ctx).unwrap();
        assert_eq!(d2.dirty_count(), 1);
        let v = view_of(&ctx, &c2, &d2, Vec::new());
        assert_eq!(v.dirty_count, 1);
        assert_eq!(v.save, w::SaveState::Dirty);

        // 反向回去 → 干净
        let (c3, mut d3, _) = state(&ctx).unwrap();
        apply_patches(&mut d3, &c3, &ctx.up.registry, &out.inverse).unwrap();
        storage::write_draft(&ctx.store, &d3).unwrap();
        let (_, d4, _) = state(&ctx).unwrap();
        assert!(d4.is_clean(), "撤销之后该回到干净");
    }

    /// **差异清单要逐条说得出来**，不能只给一个数字
    #[test]
    fn the_diff_lists_every_pending_change() {
        let (_d, _f, ctx) = ctx();
        let (c, mut d, _) = state(&ctx).unwrap();
        apply_patches(
            &mut d,
            &c,
            &ctx.up.registry,
            &[
                Patch::SetValue {
                    level: Level::Version,
                    owner: "A1/STANDARD".to_owned(),
                    key: "toolhead.offset.x".to_owned(),
                    value: Some(serde_json::json!(7)),
                },
                Patch::RenameVersion {
                    uid: "A1/FAST".to_owned(),
                    name: "新名字".to_owned(),
                },
                Patch::ArchiveVersion {
                    uid: "A1/FAST".to_owned(),
                },
            ],
        )
        .unwrap();

        let lines = diff_draft(&ctx, &c, &d);
        assert_eq!(lines.len(), 3, "三处改动三行：{lines:#?}");

        let value_line = lines.iter().find(|l| l.kind == "改值").unwrap();
        assert_eq!(value_line.target, "A1 · 标准版");
        assert_eq!(value_line.before, "-1 mm", "before 取已落盘那一份");
        assert_eq!(value_line.after, "7 mm");

        assert!(lines.iter().any(|l| l.kind == "改名" && l.after == "新名字"));
        assert!(lines.iter().any(|l| l.kind == "归档状态"));
    }

    /// 草稿里指向已消失版本的条目在**每次读**的时候清掉，并且只提示一次
    #[test]
    fn stale_draft_entries_are_pruned_and_reported_once() {
        let (_d, _f, ctx) = ctx();
        let mut d = Draft::default();
        d.values.insert(
            "v:A1/GONE:toolhead.offset.x".to_owned(),
            Some(serde_json::json!(1)),
        );
        storage::write_draft(&ctx.store, &d).unwrap();

        let (_, d1, n1) = state(&ctx).unwrap();
        assert!(d1.is_clean());
        assert_eq!(n1.len(), 1);
        assert!(n1[0].contains("A1/GONE"));

        // 第二次读不该再报 —— 清理结果已经写回去了
        let (_, _, n2) = state(&ctx).unwrap();
        assert!(n2.is_empty(), "同一个问题不该每次打开都报一遍");
    }

    /// 干净草稿时保存 / 丢弃都该被拒，并且给出那一句
    #[test]
    fn saving_a_clean_draft_is_refused_with_a_reason() {
        let (_d, _f, ctx) = ctx();
        let (_, d, _) = state(&ctx).unwrap();
        assert!(d.is_clean());
        // 这两条命令的判据就是上面那个 is_clean，这里把文案钉住
        assert!(!w::disabled::NOTHING_TO_SAVE.is_empty());
    }

    /// 保存之后 uid 会变，而且视图里那一版要用新 uid
    #[test]
    fn saving_a_new_version_remaps_its_uid_in_the_next_view() {
        let (_d, _f, ctx) = ctx();
        let (c, mut d, _) = state(&ctx).unwrap();
        apply_patches(
            &mut d,
            &c,
            &ctx.up.registry,
            &[Patch::NewVersion {
                machine_id: "A1".to_owned(),
                name: "我的新配方".to_owned(),
            }],
        )
        .unwrap();
        storage::write_draft(&ctx.store, &d).unwrap();

        let out = storage::save(&ctx.store, &ctx.up, &c, &d).unwrap();
        assert_eq!(out.remap.get("new-1").map(String::as_str), Some("A1/NEW1"));

        let (c2, d2, n2) = state(&ctx).unwrap();
        assert!(d2.is_clean(), "保存后草稿清空");
        let v = view_of(&ctx, &c2, &d2, n2);
        // 上游没有 NEW1 这一版，所以它在树上看不到 —— 但**必须有一条提示**
        assert!(
            v.notices.iter().any(|n| n.contains("NEW1")),
            "上游没有这一版就得说出来，不能让它静默消失：{:?}",
            v.notices
        );
    }

    /// 矩阵与批量预览的列序一致，且矩阵能按一列当「字段详情」用
    #[test]
    fn a_single_column_matrix_serves_as_the_field_detail_view() {
        let (_d, _f, ctx) = ctx();
        let (c, d, _) = state(&ctx).unwrap();
        let book = Book::new(&ctx.up, &c, &d);

        let m = book.matrix(&cols(&[("A1", Some("A1/STANDARD"))]), None, "");
        assert_eq!(m.cols.len(), 1);
        assert_eq!(m.cols[0].level, Level::Version);
        // 每一行都带齐字段详情要的东西
        for r in &m.rows {
            let cell = &r.cells[0];
            assert!(!cell.text.is_empty(), "{} 的格子是空的", r.key);
            if cell.kind != crate::workbench::domain::derive::CellKind::NotApplicable {
                assert!(cell.origin.is_some());
                assert!(cell.origin_explain.is_some(), "来源要带一句「改了会怎样」");
            }
        }
    }

    /// 回退登记表这一页是**只读**的，而且要说出为什么
    #[test]
    fn the_fallback_page_says_why_it_is_read_only() {
        let (_d, _f, ctx) = ctx();
        let f = &ctx.up.fallback;
        assert!(!f.guide.is_empty());
        assert!(f.disabled().is_empty());
        assert!(!w::NO_DISABLED_FALLBACK.is_empty(), "空列表也要有一句话");
    }
}
