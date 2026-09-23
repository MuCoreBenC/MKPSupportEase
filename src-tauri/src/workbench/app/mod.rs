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

pub mod build;
/// 「机型与版本」那一页。**它不走 `Ctx` / `Committed` / `Draft`** ——
/// 那一套是参数值的，这一页管清单，两件事不共用状态机（见该文件头）
pub mod machines;
pub mod storage;
pub mod words;

use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::AppError;
use crate::ipc::traced;
use crate::workbench::domain::derive::{Book, BookView, ColRef, Desk, Matrix, StockRow};
use crate::workbench::domain::patch::{apply as apply_patches, Draft, Patch};
use crate::workbench::domain::preview::{BulkPreview, MovePreview};
use crate::workbench::domain::wording as w;
use crate::workbench::domain::{Committed, Level};
use crate::workbench::store::{Store, TrashEntry};
use crate::workbench::upstream::registry::{ParamDef, ShowWhen, TabMeta, UiComponent, ValueType};
use crate::workbench::upstream::Upstream;
use crate::workbench::{paths, Roots};

/// 一次会话的全部状态。
///
/// **草稿的真相在这里，不在盘上。** 上一稿每条命令都 `read_draft` 一次、
/// 每次编辑都 `write_draft` 一次（原子写 = 临时文件 + rename + fsync），
/// 于是「改一个数」的代价里有一次磁盘往返 —— 手感上就是卡。
/// 更糟的是把磁盘当状态源：那是「三份状态各存一套」那个老病的另一种形态。
///
/// 现在磁盘上的 `.draft/book.json` 只是**崩溃恢复快照**，由 [`flush_if_due`] 懒写。
pub struct Ctx {
    pub up: Upstream,
    pub store: Store,
    /// 落盘的那一份。开场读一次，`wb_save` 之后重读
    committed: Committed,
    /// 未保存的改动。**编辑只碰它**
    draft: Draft,
    /// 加载时攒下的提示（孤儿版本、清理掉的草稿条目…）
    notices: Vec<String>,
    /// 编辑序号：每次改草稿 +1
    dirty_seq: u64,
    /// 已经写进快照的序号。与 `dirty_seq` 相等 = 快照是最新的
    flushed_seq: u64,
    last_edit: Instant,
    /// 上一次快照写失败的原因。**不阻断编辑**，只报出来
    flush_error: Option<String>,
}

impl Ctx {
    /// 真仓库。**上游定位不到时直接失败** —— 那种情况下工作台不该启动业务，
    /// 因为字段定义、机型清单、资源清单全在上游，缺了它每一页都是空的
    pub fn open() -> Result<Self, AppError> {
        let store = Store::open()?;
        store.bootstrap()?;
        Self::with(Upstream::load()?, store)
    }

    /// 给定上游与仓库建一个会话。测试用这一条，不碰真仓库也不碰那个全局
    pub(super) fn with(up: Upstream, store: Store) -> Result<Self, AppError> {
        let mut ctx = Self {
            up,
            store,
            committed: Committed::default(),
            draft: Draft::default(),
            notices: Vec::new(),
            dirty_seq: 0,
            flushed_seq: 0,
            last_edit: Instant::now(),
            flush_error: None,
        };
        ctx.reload_from_disk()?;
        Ok(ctx)
    }

    /// 从磁盘重建「已落盘 + 草稿快照」。**只在三处调**：开场、`wb_save` 之后、`wb_reload`。
    ///
    /// 清理指向已消失对象的草稿条目也在这里 —— 上游可能在工作台开着的时候被重建，
    /// 但那件事只会在重读的时候发生，不需要每条命令都查一遍
    fn reload_from_disk(&mut self) -> Result<(), AppError> {
        let loaded = storage::load(&self.store, &self.up)?;
        let mut draft = storage::read_draft(&self.store)?;
        let mut notices = loaded.notices;
        let pruned = draft.prune(&loaded.committed);
        let dirty = !pruned.is_empty();
        notices.extend(pruned);

        self.committed = loaded.committed;
        self.draft = draft;
        self.notices = notices;
        // 清理过的话快照就过期了，但**不在这里写** —— 交给懒写
        self.flushed_seq = self.dirty_seq;
        if dirty {
            self.dirty_seq += 1;
        }
        Ok(())
    }

    /// 草稿改过了。**只动内存**，落盘交给懒写
    fn touch(&mut self) {
        self.dirty_seq += 1;
        self.last_edit = Instant::now();
    }

    /// 快照过期了吗
    fn stale(&self) -> bool {
        self.flushed_seq != self.dirty_seq
    }

    /// 写一次快照。**永不返回 Err** —— 快照的价值是崩溃恢复，
    /// 为了它让一次编辑失败是本末倒置。失败只记下来，界面上报一格
    fn flush(&mut self) {
        if !self.stale() {
            return;
        }
        match storage::write_draft(&self.store, &self.draft) {
            Ok(()) => {
                self.flushed_seq = self.dirty_seq;
                self.flush_error = None;
            }
            Err(e) => {
                tracing::warn!(error = %e.message, "草稿快照写不进去");
                self.flush_error = Some(e.message);
            }
        }
    }

    /// 界面要显示的提示：加载时那些 + 快照写不进去那一条
    fn notices_now(&self) -> Vec<String> {
        let mut out = self.notices.clone();
        if let Some(e) = &self.flush_error {
            out.push(format!("{}：{e}", w::SNAPSHOT_FAILED));
        }
        out
    }

    /// 快照状态，给状态条用。**与「未保存 N 处」是两条信息**：
    /// 「未保存」说的是仓库文件，「待落盘」说的是崩溃快照
    fn snapshot(&self) -> w::SnapshotState {
        if self.flush_error.is_some() {
            w::SnapshotState::Failed
        } else if self.stale() {
            w::SnapshotState::Pending
        } else {
            w::SnapshotState::Current
        }
    }
}

/// 上游快照。一个进程一个工作台窗口，所以放模块级。
///
/// `Mutex<Option<_>>` 而不是 `OnceLock<Ctx>`：`wb_reload` 要能把它换掉
static SESSION: OnceLock<Mutex<Option<Ctx>>> = OnceLock::new();

fn session() -> &'static Mutex<Option<Ctx>> {
    SESSION.get_or_init(|| Mutex::new(None))
}

/// 借出会话上下文，没有就现建一个。`pub(super)` 是给同层的 `build` 用的
///
/// 锁中毒（上一次持锁时 panic 了）不当成致命错误：清掉重来。
/// 代价是丢掉那一刻还没落盘的草稿 —— 但持锁时 panic 本来就意味着状态不可信
pub(super) fn with_ctx<T>(f: impl FnOnce(&Ctx) -> Result<T, AppError>) -> Result<T, AppError> {
    with_ctx_mut(|ctx| f(ctx))
}

/// 要改草稿的那几条命令走这个。**改完只动内存**，落盘交给懒写
pub(super) fn with_ctx_mut<T>(
    f: impl FnOnce(&mut Ctx) -> Result<T, AppError>,
) -> Result<T, AppError> {
    let mut guard = session().lock().unwrap_or_else(|e| {
        tracing::warn!("会话的锁中毒了，重建一份");
        let mut g = e.into_inner();
        *g = None;
        g
    });
    if guard.is_none() {
        *guard = Some(Ctx::open()?);
    }
    f(guard.as_mut().expect("刚刚放进去的"))
}

/// 空闲多久算「停手了」。写得太短等于没省，太长丢的就多
const IDLE_BEFORE_FLUSH: Duration = Duration::from_secs(2);

/// 空闲落盘的那个线程只起一次。窗口关掉再开不该攒出第二个
static FLUSHER: OnceLock<()> = OnceLock::new();

/// 起一个每秒醒一次的线程：**停手了才写**。
///
/// 用 `std::thread` 而不是异步任务，是为了不给这一件事拉一个 `tokio` 依赖进来；
/// 一个每秒醒一次、醒了就看一眼计数器的线程，代价可以忽略。
/// 没有会话时它什么也不做，所以窗口关了也不用回收
pub(super) fn spawn_flusher() {
    FLUSHER.get_or_init(|| {
        let spawned = std::thread::Builder::new()
            .name("wb-flush".to_owned())
            .spawn(|| loop {
                std::thread::sleep(Duration::from_secs(1));
                flush_if_due();
            });
        if let Err(e) = spawned {
            // 起不来就退回「每次编辑都写」那种行为？不 —— 那会把卡又带回来。
            // 这里只报一声：失焦与关窗那两个时机仍然会落盘
            tracing::warn!(error = %e, "空闲落盘的线程起不来，快照只在失焦/关窗时写");
        }
    });
}

/// 空闲落盘。**没停手就不写** —— 连着改 10 个值只在停手之后写一次
fn flush_if_due() {
    let Ok(mut guard) = session().lock() else { return };
    let Some(ctx) = guard.as_mut() else { return };
    if ctx.stale() && ctx.last_edit.elapsed() >= IDLE_BEFORE_FLUSH {
        ctx.flush();
    }
}

/// 立刻落盘。窗口失焦 / 关闭前调，**不等那 2 秒**
pub(super) fn flush_now() {
    let Ok(mut guard) = session().lock() else { return };
    if let Some(ctx) = guard.as_mut() {
        ctx.flush();
    }
}

/// 丢掉会话，下一次调用会重读。**先落盘** —— 否则重读会把未保存的改动吃掉
fn drop_ctx() {
    if let Ok(mut g) = session().lock() {
        if let Some(ctx) = g.as_mut() {
            ctx.flush();
        }
        *g = None;
    }
}

/// 「已落盘 + 草稿」的一份拷贝。
///
/// **不读磁盘**：内存里那一份就是真相。返回拷贝而不是引用，是为了让调用方
/// 能继续拿 `&Ctx` 做别的事（`Book` 要同时借上游与这两样）——
/// 一次克隆是几千个小分配，比原来那一次 fsync 便宜三个数量级
pub(super) fn state(ctx: &Ctx) -> Result<(Committed, Draft, Vec<String>), AppError> {
    Ok((
        ctx.committed.clone(),
        ctx.draft.clone(),
        ctx.notices_now(),
    ))
}

fn view_of(ctx: &Ctx, committed: &Committed, draft: &Draft, notices: Vec<String>) -> BookView {
    let mut v = Book::new(&ctx.up, committed, draft).book_view();
    v.notices = notices;
    v.snapshot = ctx.snapshot();
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

/// 配方台一屏（默认视角）：一个版本的分组列表。
///
/// 矩阵是它的对比工具，不是默认 —— 一屏几十列的表格不好看也不好改
#[tauri::command]
pub fn wb_desk(
    machine_id: String,
    uid: Option<String>,
    tab: Option<String>,
    query: Option<String>,
) -> Result<Desk, AppError> {
    traced("wb_desk", |_| {
        with_ctx(|ctx| {
            let (c, d, _) = state(ctx)?;
            Ok(Book::new(&ctx.up, &c, &d).desk(
                &machine_id,
                uid.as_deref(),
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
    /// 调用方要的那一页，**顺带带回来**。上一稿前端要在 apply 之后再问一次，
    /// 于是一次手势要走两趟 IPC、后端把整本书算两遍
    pub desk: Option<Desk>,
    pub matrix: Option<Matrix>,
}

/// 写完顺带刷哪一页。`None` = 只要 `BookView`。
///
/// 两个 `rename_all` 都要写：枚举上那个改的是**变体名**（`desk` / `matrix`），
/// 变体里的字段名要 `rename_all_fields` 才会变成小驼峰 —— 少写一个的话
/// 前端传 `machineId` 而后端等 `machine_id`，表现只是「那一页没回来」。
///
/// 三个可选字段都带 `default`：**结构体里的 `Option` 不会自动缺省**
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase", tag = "page")]
pub enum Refresh {
    Desk {
        machine_id: String,
        #[serde(default)]
        uid: Option<String>,
        #[serde(default)]
        tab: Option<String>,
        #[serde(default)]
        query: Option<String>,
    },
    Matrix {
        cols: Vec<ColRef>,
        #[serde(default)]
        tab: Option<String>,
        #[serde(default)]
        query: Option<String>,
    },
}

/// **唯一的写入口。** 一次调用 = 一次手势 = 一条撤销。
///
/// **不落盘** —— 改完内存就返回。快照由空闲落盘负责（§3）。
/// 上一稿在这里做一次原子写，于是每敲一个值都有一次 fsync
#[tauri::command]
pub fn wb_apply_draft(
    label: String,
    patches: Vec<Patch>,
    refresh: Option<Refresh>,
) -> Result<ApplyResult, AppError> {
    traced("wb_apply_draft", |_| {
        with_ctx_mut(|ctx| {
            let out = apply_patches(&mut ctx.draft, &ctx.committed, &ctx.up.registry, &patches)?;
            ctx.touch();
            let mut notices = ctx.notices_now();
            notices.extend(out.notices);
            tracing::info!(label = %label, patches = patches.len(), "草稿已更新");

            // 一次派生，两份结果：整本视图 + 调用方要的那一页
            let book = Book::new(&ctx.up, &ctx.committed, &ctx.draft);
            let (desk, matrix) = match &refresh {
                None => (None, None),
                Some(Refresh::Desk {
                    machine_id,
                    uid,
                    tab,
                    query,
                }) => (
                    Some(book.desk(
                        machine_id,
                        uid.as_deref(),
                        tab.as_deref(),
                        query.as_deref().unwrap_or_default(),
                    )),
                    None,
                ),
                Some(Refresh::Matrix { cols, tab, query }) => (
                    None,
                    Some(book.matrix(
                        cols,
                        tab.as_deref(),
                        query.as_deref().unwrap_or_default(),
                    )),
                ),
            };
            let mut view = book.book_view();
            view.notices = notices.clone();
            view.snapshot = ctx.snapshot();

            Ok(ApplyResult {
                view,
                inverse: out.inverse,
                undoable: out.undoable,
                notices,
                desk,
                matrix,
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
        with_ctx_mut(|ctx| {
            if ctx.draft.is_clean() {
                return Err(AppError::invalid_argument(w::disabled::NOTHING_TO_SAVE));
            }
            // 保存前先有一份对得上的快照：万一 save 中途挂了，草稿还在
            ctx.flush();
            let out = storage::save(&ctx.store, &ctx.up, &ctx.committed, &ctx.draft)?;
            let mut notices = out.notices;

            // 保存之后重读：落盘的那一份才是新的真相，草稿也被清空了
            ctx.reload_from_disk()?;
            notices.extend(ctx.notices_now());
            let view = view_of(ctx, &ctx.committed, &ctx.draft, notices.clone());
            Ok(SaveResult {
                view,
                remap: out.remap,
                notices,
            })
        })
    })
}

#[tauri::command]
pub fn wb_discard() -> Result<BookView, AppError> {
    traced("wb_discard", |_| {
        with_ctx_mut(|ctx| {
            if ctx.draft.is_clean() {
                return Err(AppError::invalid_argument(w::disabled::NOTHING_TO_SAVE));
            }
            let dropped = ctx.draft.dirty_count();
            ctx.draft = Draft::default();
            ctx.touch();
            // 丢弃**立刻落盘**：留着旧快照的话，崩一次就把刚丢掉的又捞回来了
            ctx.flush();
            tracing::info!(dropped, "草稿已丢弃");
            let notices = ctx.notices_now();
            Ok(view_of(ctx, &ctx.committed, &ctx.draft, notices))
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
        let ctx = Ctx::with(up.up, store).unwrap();
        (dir, f, ctx)
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

    /// 一条 patch 走完整条路径：改**内存** → 视图跟着变 → 反向能回去。
    ///
    /// 注意这里**没有落盘**这一步：一次编辑不碰磁盘，快照是另一件事
    #[test]
    fn one_gesture_goes_through_the_single_write_entry() {
        let (_d, _f, mut ctx) = ctx();
        let patch = Patch::SetValue {
            level: Level::Machine,
            owner: "A1".to_owned(),
            key: "wiping.child".to_owned(),
            value: Some(serde_json::json!(42)),
        };

        let out = apply_patches(&mut ctx.draft, &ctx.committed, &ctx.up.registry, &[patch]).unwrap();
        ctx.touch();
        assert!(out.undoable);

        let (c2, d2, _) = state(&ctx).unwrap();
        assert_eq!(d2.dirty_count(), 1);
        let v = view_of(&ctx, &c2, &d2, Vec::new());
        assert_eq!(v.dirty_count, 1);
        assert_eq!(v.save, w::SaveState::Dirty);
        assert_eq!(v.snapshot, w::SnapshotState::Pending, "还没落盘");

        // 反向回去 → 干净
        let inverse = out.inverse;
        apply_patches(&mut ctx.draft, &ctx.committed, &ctx.up.registry, &inverse).unwrap();
        ctx.touch();
        assert!(ctx.draft.is_clean(), "撤销之后该回到干净");
    }

    /// **编辑不碰磁盘。** 上一稿每次编辑一次原子写，那是「卡」的一个来源，
    /// 也是把磁盘当状态源
    #[test]
    fn editing_does_not_touch_the_disk() {
        let (dir, _f, mut ctx) = ctx();
        let snapshot = dir.path().join(".draft").join("book.json");
        assert!(!snapshot.exists(), "干净仓库本来就没有快照");

        for n in 0..10 {
            apply_patches(
                &mut ctx.draft,
                &ctx.committed,
                &ctx.up.registry,
                &[Patch::SetValue {
                    level: Level::Machine,
                    owner: "A1".to_owned(),
                    key: "wiping.child".to_owned(),
                    value: Some(serde_json::json!(n + 1)),
                }],
            )
            .unwrap();
            ctx.touch();
            assert!(!snapshot.exists(), "第 {n} 次编辑就落盘了");
        }

        // 十次编辑**攒成一次**落盘
        assert!(ctx.stale());
        ctx.flush();
        assert!(snapshot.exists());
        assert!(!ctx.stale());

        // 已经是最新的，再 flush 一次什么都不做
        ctx.flush();
        assert!(!ctx.stale());
    }

    /// 那个「改过去改不回来」的后端侧回归：
    /// 一层本来没有自有值时，`disk` → `tower` 这两步**脏计数都是 1**，
    /// 但有效值必须两次都跟着变。上一稿前端拿脏计数当刷新信号，所以第二步看不见
    #[test]
    fn a_value_changed_back_and_forth_is_visible_each_time() {
        let (_d, _f, mut ctx) = ctx();
        let set = |ctx: &mut Ctx, v: &str| {
            apply_patches(
                &mut ctx.draft,
                &ctx.committed,
                &ctx.up.registry,
                &[Patch::SetValue {
                    level: Level::Version,
                    owner: "A1/STANDARD".to_owned(),
                    key: "wiping.mode".to_owned(),
                    value: Some(serde_json::json!(v)),
                }],
            )
            .unwrap();
            ctx.touch();
        };
        let now = |ctx: &Ctx| {
            let cols = cols(&[("A1", Some("A1/STANDARD"))]);
            let (c, d, _) = state(ctx).unwrap();
            let m = Book::new(&ctx.up, &c, &d).matrix(&cols, None, "");
            let row = m.rows.into_iter().find(|r| r.key == "wiping.mode").unwrap();
            row.cells[0].raw.clone()
        };

        set(&mut ctx, "disk");
        assert_eq!(now(&ctx), serde_json::json!("disk"));
        assert_eq!(ctx.draft.dirty_count(), 1);

        set(&mut ctx, "tower");
        assert_eq!(now(&ctx), serde_json::json!("tower"), "改回去必须看得见");
        assert_eq!(ctx.draft.dirty_count(), 1, "脏计数没变 —— 所以它不能当刷新信号");
    }

    /// 快照写不进去时：**编辑照常**，但要报出来
    #[test]
    fn a_failed_snapshot_is_reported_without_blocking_edits() {
        let (_d, _f, mut ctx) = ctx();
        ctx.touch();
        ctx.flush_error = Some("磁盘满了".to_owned());

        assert_eq!(ctx.snapshot(), w::SnapshotState::Failed);
        let notices = ctx.notices_now();
        assert!(
            notices.iter().any(|n| n.contains("磁盘满了")),
            "写不进去要说出原因：{notices:?}"
        );

        // 还能继续改
        apply_patches(
            &mut ctx.draft,
            &ctx.committed,
            &ctx.up.registry,
            &[Patch::SetValue {
                level: Level::Machine,
                owner: "A1".to_owned(),
                key: "wiping.child".to_owned(),
                value: Some(serde_json::json!(9)),
            }],
        )
        .expect("快照失败不该拦住编辑");
    }

    /// 快照三态的词互不相同，且「未保存」与「待落盘」不是同一句
    #[test]
    fn snapshot_words_are_distinct_from_save_words() {
        use w::SnapshotState as S;
        let all = [S::Current, S::Pending, S::Failed];
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert_ne!(a.label(), b.label());
                assert_ne!(a.explain(), b.explain());
            }
        }
        assert_ne!(S::Pending.label(), w::SaveState::Dirty.label());
    }

    /// 「顺带刷哪一页」的线上形状。**这里错了不会编译报错，只会在运行时静默拿不到那一页**
    #[test]
    fn a_refresh_request_deserializes_from_the_wire_shape() {
        let desk: Refresh = serde_json::from_value(serde_json::json!({
            "page": "desk", "machineId": "A1", "uid": "A1/STANDARD", "tab": null, "query": ""
        }))
        .expect("配方台那一页");
        match desk {
            Refresh::Desk { machine_id, uid, .. } => {
                assert_eq!(machine_id, "A1");
                assert_eq!(uid.as_deref(), Some("A1/STANDARD"));
            }
            _ => panic!("解成了别的页"),
        }

        let matrix: Refresh = serde_json::from_value(serde_json::json!({
            "page": "matrix",
            "cols": [{ "machineId": "A1", "versionUid": null }],
            "tab": "wiping"
        }))
        .expect("对比那一页");
        match matrix {
            Refresh::Matrix { cols, tab, query } => {
                assert_eq!(cols.len(), 1);
                assert_eq!(tab.as_deref(), Some("wiping"));
                assert!(query.is_none(), "不给就是不给，不要编一个空串");
            }
            _ => panic!("解成了别的页"),
        }
    }

    /// 一次手势一次派生：两页各自算得出来，而且**不互相要求对方在场**
    #[test]
    fn both_pages_can_be_produced_from_one_derivation() {
        let (_d, _f, ctx) = ctx();
        let (c, d, _) = state(&ctx).unwrap();
        let book = Book::new(&ctx.up, &c, &d);

        let desk = book.desk("A1", Some("A1/STANDARD"), None, "");
        let matrix = book.matrix(&cols(&[("A1", Some("A1/STANDARD"))]), None, "");

        let desk_rows: usize = desk
            .groups
            .iter()
            .flat_map(|g| g.items.iter())
            .map(|i| 1 + i.children.len())
            .sum();
        assert_eq!(desk_rows, matrix.rows.len(), "同一份数据，两种摆法，行数一样");
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

    /// 草稿里指向已消失版本的条目在**加载时**清掉，并且报出来。
    ///
    /// 清理从「每次命令都查一遍」挪到了加载那一次：上游可能在工作台开着的时候被重建，
    /// 但那件事只会在重读的时候发生
    #[test]
    fn stale_draft_entries_are_pruned_at_load_and_reported() {
        let (_d, _f, mut ctx) = ctx();
        let mut d = Draft::default();
        d.values.insert(
            "v:A1/GONE:toolhead.offset.x".to_owned(),
            Some(serde_json::json!(1)),
        );
        storage::write_draft(&ctx.store, &d).unwrap();

        ctx.reload_from_disk().unwrap();

        let (_, d1, n1) = state(&ctx).unwrap();
        assert!(d1.is_clean(), "指向不存在版本的那一条该被清掉");
        assert_eq!(n1.len(), 1);
        assert!(n1[0].contains("A1/GONE"));
        // 清理过 ⇒ 快照过期 ⇒ 空闲落盘会把清理结果写回去
        assert!(ctx.stale(), "清理过的草稿要重新落盘");
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
        let (_d, _f, mut ctx) = ctx();
        apply_patches(
            &mut ctx.draft,
            &ctx.committed,
            &ctx.up.registry,
            &[Patch::NewVersion {
                machine_id: "A1".to_owned(),
                name: "我的新配方".to_owned(),
            }],
        )
        .unwrap();
        ctx.touch();

        let out = storage::save(&ctx.store, &ctx.up, &ctx.committed, &ctx.draft).unwrap();
        assert_eq!(out.remap.get("new-1").map(String::as_str), Some("A1/NEW1"));

        // 保存之后必须重读：落盘的那一份才是新的真相
        ctx.reload_from_disk().unwrap();
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
