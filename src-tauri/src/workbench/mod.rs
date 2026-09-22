//! 后厨工作台的 Rust 侧。
//!
//! **整个模块只在 `workbench` feature 下存在。** `lib.rs` 里的 `mod workbench;` 挂着
//! `#[cfg(feature = "workbench")]`，所以默认构建连编译都不会碰这些文件 —— 给用户的
//! 二进制里搜不到下面任何一个命令名。这是 doc §4 的第二道闸（第一道在 vite 那边，
//! 管的是 `workbench.html` 在不在前端产物里）。
//!
//! 分工：
//! - [`paths`] —— 数据根指向仓库里的开发数据，不是 appDataDir
//! - [`model`] —— 开发源数据的形状（字段定义 / 机型基底 / 版本覆盖）
//! - [`store`] —— 读写。写盘一律复用 `fsx::atomic`
//! - [`resolve`] —— 继承解析与有效配方指纹
//! - [`clock`] —— 时间戳的唯一来源
//!
//! 命令一律走 [`crate::ipc::traced`] 包一层，与客户端命令共用同一套 trace id 与错误模型。
//! 这里不另造一套 —— 工作台出错时同样要能按 traceId 去日志里捞。

pub mod bbs;
pub mod capability;
pub mod catalog;
pub mod clock;
pub mod generate;
pub mod matrix;
pub mod model;
pub mod ops;
pub mod paths;
pub mod preflight;
pub mod publish;
pub mod resolve;
pub mod state;
pub mod store;

/// 端到端链路（doc §16.4）。只在测试时编进来
#[cfg(test)]
mod e2e;

use serde::Serialize;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::error::AppError;
use crate::ipc::traced;
use crate::workbench::model::{Machine, Registry, Version};
use crate::workbench::resolve::Effective;
use crate::workbench::store::{BootstrapReport, Store};

/// 工作台窗口的标签。与客户端的 `main` 分开，`get_webview_window` 拿的是各自那一个
const WINDOW_LABEL: &str = "workbench";

/// 开工作台窗口。已经开着就聚焦，不重复开第二个。
///
/// 刻意**不带** transparent / titleBarStyle=Overlay 这些客户端的窗口花活：
/// 工作台是密集表格界面，原生标题栏反而省事，也不需要为它再写一套拖拽与缩放边。
pub fn open_window(app: &AppHandle) -> Result<(), AppError> {
    if let Some(win) = app.get_webview_window(WINDOW_LABEL) {
        let _ = win.set_focus();
        return Ok(());
    }

    WebviewWindowBuilder::new(app, WINDOW_LABEL, WebviewUrl::App("workbench.html".into()))
        .title("SupportEase 后厨工作台")
        .inner_size(1360.0, 900.0)
        .min_inner_size(900.0, 560.0)
        .resizable(true)
        .build()
        .map_err(|e| AppError::internal("建不出工作台窗口").with_detail(e.to_string()))?;

    tracing::info!("工作台窗口已打开");
    Ok(())
}

/* ---------- 自检与首次启动 ---------- */

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Roots {
    /// 开发源数据根（仓库里的 `workbench/`）
    pub workbench: String,
    /// 发布目录（仓库里的 `dist-presets/`）
    pub dist: String,
}

/// 把两个数据根的实际位置交出来。
///
/// 存在的意义不是"有个命令能调通"，而是**出问题时第一句要问的问题**：
/// 它到底在读哪个目录。界面顶栏常驻显示这两行。
#[tauri::command]
pub fn wb_roots() -> Result<Roots, AppError> {
    traced("wb_roots", |_| {
        Ok(Roots {
            workbench: paths::workbench_root()?.display().to_string(),
            dist: paths::dist_root()?.display().to_string(),
        })
    })
}

/// 建目录 + 补全局配置。**一个配方都不写** —— 见 doc §7
#[tauri::command]
pub fn wb_bootstrap() -> Result<BootstrapReport, AppError> {
    traced("wb_bootstrap", |_| Store::open()?.bootstrap())
}

/* ---------- 字段定义（只读） ---------- */

/// 字段定义全表。界面只展示，不提供任何编辑入口（doc §9.8）
#[tauri::command]
pub fn wb_registry() -> Result<Registry, AppError> {
    traced("wb_registry", |_| Store::open()?.registry())
}

/* ---------- 机型树 ---------- */

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionBrief {
    pub id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineNode {
    pub id: String,
    pub display_name: String,
    /// 机型基底里写了几个字段。为 0 时这个机型下的版本全是"未配置"
    pub base_field_count: usize,
    pub versions: Vec<VersionBrief>,
}

/// 左侧机型树。读不出来的单个机型/版本**跳过并记日志**，不让一个坏文件把整棵树打没
#[tauri::command]
pub fn wb_tree() -> Result<Vec<MachineNode>, AppError> {
    traced("wb_tree", |_| {
        let store = Store::open()?;
        let mut out = Vec::new();

        for mid in store.machine_ids()? {
            let m = match store.machine(&mid) {
                Ok(m) => m,
                Err(e) => {
                    tracing::warn!(machine = %mid, "机型读不出来，跳过：{e}");
                    continue;
                }
            };

            let mut versions = Vec::new();
            for vid in store.version_ids(&mid)? {
                match store.version(&mid, &vid) {
                    Ok(v) => versions.push(VersionBrief {
                        id: v.id,
                        display_name: v.display_name,
                    }),
                    Err(e) => tracing::warn!(machine = %mid, version = %vid, "版本读不出来，跳过：{e}"),
                }
            }

            out.push(MachineNode {
                id: m.id,
                display_name: m.display_name,
                base_field_count: m.base.len(),
                versions,
            });
        }

        Ok(out)
    })
}

/* ---------- 有效配方 ---------- */

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveView {
    pub machine: Machine,
    pub version: Version,
    pub effective: Effective,
    /// 有效配方为空 = 还没写配方（doc §8 的"未配置"）
    pub unconfigured: bool,
    /// 机型基底自己的指纹。编辑基底时当作并发令牌
    pub machine_hash: String,
}

/// 某个版本的有效配方 —— 界面上每个字段的值与来源都从这里来
#[tauri::command]
pub fn wb_effective(machine_id: String, version_id: String) -> Result<EffectiveView, AppError> {
    traced("wb_effective", |_| {
        view_of(&Store::open()?, &machine_id, &version_id)
    })
}

fn view_of(store: &Store, machine_id: &str, version_id: &str) -> Result<EffectiveView, AppError> {
    let reg = store.registry()?;
    let fb = store.fallback()?;
    let machine = store.machine(machine_id)?;
    let version = store.version(machine_id, version_id)?;
    let effective = resolve::resolve(&reg, &fb, &machine, &version);
    Ok(EffectiveView {
        unconfigured: effective.is_unconfigured(),
        machine_hash: model::sha256_of(&machine),
        machine,
        version,
        effective,
    })
}

/* ---------- 保存 ---------- */

/// 写版本覆盖。
///
/// `expected_hash` 是打开这份配方时拿到的有效配方指纹。**对不上就拒绝** ——
/// 它可能在另一个窗口被改过、也可能是从回收站还原过。不做锁，只做"不静默覆盖"：
/// 锁要处理持有者崩溃、超时、跨进程，而这里的实际场景是"我自己开了两个窗口"。
#[tauri::command]
pub fn wb_save_version(
    machine_id: String,
    version_id: String,
    display_name: String,
    overrides: model::Params,
    expected_hash: String,
) -> Result<EffectiveView, AppError> {
    traced("wb_save_version", |_| {
        let store = Store::open()?;
        let current = view_of(&store, &machine_id, &version_id)?;
        ensure_fresh(&current.effective.hash, &expected_hash)?;

        let mut v = current.version.clone();
        v.display_name = display_name;
        v.overrides = overrides;
        store.save_version(&v)?;
        // 保存成功 = 草稿的使命结束
        store.clear_draft(&machine_id, &version_id)?;

        view_of(&store, &machine_id, &version_id)
    })
}

/// 写机型基底。**只改基底** —— 它拿不到任何版本，改不到别人的覆盖
#[tauri::command]
pub fn wb_save_machine_base(
    machine_id: String,
    base: model::Params,
    expected_machine_hash: String,
) -> Result<Machine, AppError> {
    traced("wb_save_machine_base", |_| {
        let store = Store::open()?;
        let mut m = store.machine(&machine_id)?;
        ensure_fresh(&model::sha256_of(&m), &expected_machine_hash)?;
        m.base = base;
        store.save_machine(&m)?;
        // 保存成功 = 基底草稿的使命结束
        store.clear_base_draft(&machine_id)?;
        Ok(m)
    })
}

fn ensure_fresh(current: &str, expected: &str) -> Result<(), AppError> {
    if current == expected {
        return Ok(());
    }
    Err(
        AppError::invalid_argument("这份配方在别处被改过，请先重新载入再保存").with_detail(
            format!("当前指纹 {current}，你手上那份是 {expected}"),
        ),
    )
}

/* ---------- 草稿 ---------- */

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftView {
    pub draft: Option<store::Draft>,
    /// 草稿记的 baseHash 与当前配方是否一致。
    ///
    /// 不一致 = 这份配方在写草稿之后被改过（另一个窗口、git pull、从回收站还原…）。
    /// 这时**不静默套用也不静默丢弃**，交给人选 —— 两种都可能是对的。
    pub matches_current: bool,
    pub current_hash: String,
}

/// 取某个版本的草稿
#[tauri::command]
pub fn wb_draft(machine_id: String, version_id: String) -> Result<DraftView, AppError> {
    traced("wb_draft", |_| {
        let store = Store::open()?;
        let current = view_of(&store, &machine_id, &version_id)?.effective.hash;
        let draft = store.draft(&machine_id, &version_id)?;
        Ok(DraftView {
            matches_current: draft.as_ref().is_some_and(|d| d.base_hash == current),
            current_hash: current,
            draft,
        })
    })
}

/// 写草稿。前端防抖后调用 —— 落盘走原子写，所以中途关窗口最坏是少最后一次改动，
/// 不会留下半个 JSON
#[tauri::command]
pub fn wb_save_draft(
    machine_id: String,
    version_id: String,
    base_hash: String,
    overrides: model::Params,
) -> Result<(), AppError> {
    traced("wb_save_draft", |_| {
        Store::open()?.save_draft(
            &machine_id,
            &version_id,
            &store::Draft {
                base_hash,
                overrides,
                saved_at: clock::now_iso8601(),
            },
        )
    })
}

#[tauri::command]
pub fn wb_clear_draft(machine_id: String, version_id: String) -> Result<(), AppError> {
    traced("wb_clear_draft", |_| {
        Store::open()?.clear_draft(&machine_id, &version_id)
    })
}

/// 机型基底的草稿。与版本草稿是两个槽位 —— 基底编辑同样是"未保存的编辑"
#[tauri::command]
pub fn wb_base_draft(machine_id: String) -> Result<DraftView, AppError> {
    traced("wb_base_draft", |_| {
        let store = Store::open()?;
        let m = store.machine(&machine_id)?;
        let current = model::sha256_of(&m);
        let draft = store.base_draft(&machine_id)?;
        Ok(DraftView {
            matches_current: draft.as_ref().is_some_and(|d| d.base_hash == current),
            current_hash: current,
            draft,
        })
    })
}

#[tauri::command]
pub fn wb_save_base_draft(
    machine_id: String,
    base_hash: String,
    overrides: model::Params,
) -> Result<(), AppError> {
    traced("wb_save_base_draft", |_| {
        Store::open()?.save_base_draft(
            &machine_id,
            &store::Draft {
                base_hash,
                overrides,
                saved_at: clock::now_iso8601(),
            },
        )
    })
}

#[tauri::command]
pub fn wb_clear_base_draft(machine_id: String) -> Result<(), AppError> {
    traced("wb_clear_base_draft", |_| {
        Store::open()?.clear_base_draft(&machine_id)
    })
}

/* ---------- 增 / 克隆 / 改名 / 换机型 / 删 ---------- */

#[tauri::command]
pub fn wb_create_machine(id: String, display_name: String) -> Result<Machine, AppError> {
    traced("wb_create_machine", |_| {
        ops::create_machine(&Store::open()?, &id, &display_name)
    })
}

#[tauri::command]
pub fn wb_create_version(
    machine_id: String,
    id: String,
    display_name: String,
) -> Result<Version, AppError> {
    traced("wb_create_version", |_| {
        ops::create_version(&Store::open()?, &machine_id, &id, &display_name)
    })
}

/// 克隆。`new_display_name` 由前端预填成「XX 副本」，可改；**重名当场拒绝，不自动加后缀**
#[tauri::command]
pub fn wb_clone_version(
    machine_id: String,
    version_id: String,
    new_id: String,
    new_display_name: String,
) -> Result<Version, AppError> {
    traced("wb_clone_version", |_| {
        ops::clone_version(
            &Store::open()?,
            &machine_id,
            &version_id,
            &new_id,
            &new_display_name,
        )
    })
}

#[tauri::command]
pub fn wb_rename_version(
    machine_id: String,
    version_id: String,
    new_id: String,
    new_display_name: String,
) -> Result<Version, AppError> {
    traced("wb_rename_version", |_| {
        ops::rename_version(
            &Store::open()?,
            &machine_id,
            &version_id,
            &new_id,
            &new_display_name,
        )
    })
}

/// 换机型前的预览。界面必须先显示它再让人确认 —— doc §9.6
#[tauri::command]
pub fn wb_preview_move(
    machine_id: String,
    version_id: String,
    to_machine_id: String,
) -> Result<ops::MovePreview, AppError> {
    traced("wb_preview_move", |_| {
        ops::preview_move(&Store::open()?, &machine_id, &version_id, &to_machine_id)
    })
}

#[tauri::command]
pub fn wb_move_version(
    machine_id: String,
    version_id: String,
    to_machine_id: String,
) -> Result<Version, AppError> {
    traced("wb_move_version", |_| {
        ops::move_version(&Store::open()?, &machine_id, &version_id, &to_machine_id)
    })
}

/// 删除 = 进回收站 + 从菜单与套餐里摘掉。返回值说明"顺带摘掉了哪些上架条目"
#[tauri::command]
pub fn wb_trash_version(
    machine_id: String,
    version_id: String,
) -> Result<ops::TrashResult, AppError> {
    traced("wb_trash_version", |_| {
        ops::trash_version(&Store::open()?, &machine_id, &version_id)
    })
}

#[tauri::command]
pub fn wb_trash_list() -> Result<Vec<store::TrashEntry>, AppError> {
    traced("wb_trash_list", |_| Store::open()?.trash_entries())
}

#[tauri::command]
pub fn wb_restore_from_trash(file: String) -> Result<Version, AppError> {
    traced("wb_restore_from_trash", |_| {
        Store::open()?.restore_from_trash(&file)
    })
}

/// 彻底删。**二次确认由界面负责** —— 这里不再问一遍，否则两处都以为对方在问
#[tauri::command]
pub fn wb_purge_from_trash(file: String) -> Result<(), AppError> {
    traced("wb_purge_from_trash", |_| {
        Store::open()?.purge_from_trash(&file)
    })
}

/* ---------- 参数矩阵与批量 ---------- */

/// 矩阵。`machine_filter` 为空 = 全部机型
#[tauri::command]
pub fn wb_matrix(machine_filter: Vec<String>) -> Result<matrix::Matrix, AppError> {
    traced("wb_matrix", |_| {
        matrix::build(&Store::open()?, &machine_filter)
    })
}

/// 批量预览。**界面必须先显示它再让人确认**
#[tauri::command]
pub fn wb_bulk_preview(
    field_key: String,
    value: serde_json::Value,
    targets: Vec<matrix::BulkTarget>,
) -> Result<Vec<matrix::BulkEffect>, AppError> {
    traced("wb_bulk_preview", |_| {
        matrix::preview_bulk(&Store::open()?, &field_key, &value, &targets)
    })
}

#[tauri::command]
pub fn wb_bulk_apply(
    field_key: String,
    value: serde_json::Value,
    targets: Vec<matrix::BulkTarget>,
) -> Result<Vec<matrix::BulkEffect>, AppError> {
    traced("wb_bulk_apply", |_| {
        matrix::apply_bulk(&Store::open()?, &field_key, &value, &targets)
    })
}

/* ---------- 界面状态 ---------- */

/// 矩阵的筛选与展开状态。和草稿同一个目录 —— 都是本机状态，都不入库
#[tauri::command]
pub fn wb_view_state(name: String) -> Result<Option<serde_json::Value>, AppError> {
    traced("wb_view_state", |_| Store::open()?.view_state(&name))
}

#[tauri::command]
pub fn wb_save_view_state(name: String, value: serde_json::Value) -> Result<(), AppError> {
    traced("wb_save_view_state", |_| {
        Store::open()?.save_view_state(&name, &value)
    })
}

/* ---------- 状态与生成 ---------- */

/// 每个版本各自的状态。**没有"全店一个状态"这种东西** —— 你不是每次都全量生成
#[tauri::command]
pub fn wb_status() -> Result<Vec<state::VersionStatus>, AppError> {
    traced("wb_status", |_| {
        state::status_all(&Store::open()?, &paths::dist_root()?)
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenFailureItem {
    pub machine_id: String,
    pub version_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenReport {
    pub generated: Vec<generate::GenOutcome>,
    pub failed: Vec<GenFailureItem>,
    /// 这一批里有几个产物字节没变。**"没变"是好事**：说明没有制造出
    /// 会让用户端莫名要更新的新哈希
    pub unchanged_count: usize,
}

/// 生成待更新项。这是**默认按钮** —— 没改的不重烤（doc §9.4）
#[tauri::command]
pub fn wb_generate_stale() -> Result<GenReport, AppError> {
    traced("wb_generate_stale", |_| {
        let store = Store::open()?;
        let dist = paths::dist_root()?;
        let targets = state::plan_stale(&store, &dist)?;
        run_generate(&store, &dist, &targets)
    })
}

/// 全部重新生成。次要入口 —— 平时不该点它
#[tauri::command]
pub fn wb_generate_all() -> Result<GenReport, AppError> {
    traced("wb_generate_all", |_| {
        let store = Store::open()?;
        let dist = paths::dist_root()?;
        let targets: Vec<(String, String)> = state::status_all(&store, &dist)?
            .into_iter()
            // 未配置的照样跳过：它不是"要重烤"，是还没写配方
            .filter(|s| s.state != state::GenState::Unconfigured)
            .map(|s| (s.machine_id, s.version_id))
            .collect();
        run_generate(&store, &dist, &targets)
    })
}

#[tauri::command]
pub fn wb_generate_one(machine_id: String, version_id: String) -> Result<GenReport, AppError> {
    traced("wb_generate_one", |_| {
        let store = Store::open()?;
        let dist = paths::dist_root()?;
        run_generate(&store, &dist, &[(machine_id, version_id)])
    })
}

/// 批量生成的共同路径。
///
/// 能力定义**一次读好**：一批里读 N 遍不只是慢，还可能读到中途被改的两份不同定义。
/// 单个失败不中断整批 —— 但每个失败都记一笔，且**旧产物不动**（doc §9.5）。
fn run_generate(
    store: &Store,
    dist: &std::path::Path,
    targets: &[(String, String)],
) -> Result<GenReport, AppError> {
    let caps = capability::load_supported(store)?;
    let mut generated = Vec::new();
    let mut failed = Vec::new();

    for (mid, vid) in targets {
        match generate::generate_one(store, dist, mid, vid, &caps) {
            Ok(out) => generated.push(out),
            Err(e) => {
                // 失败原因落进快照，界面上才能在那一行看到"为什么没成"
                let _ = generate::record_failure(store, mid, vid, &e.to_string());
                failed.push(GenFailureItem {
                    machine_id: mid.clone(),
                    version_id: vid.clone(),
                    reason: e.message.clone(),
                });
            }
        }
    }

    Ok(GenReport {
        unchanged_count: generated.iter().filter(|o| o.unchanged).count(),
        generated,
        failed,
    })
}

/// 恢复到上次成功生成时的配方。**恢复完还得显式点生成**
#[tauri::command]
pub fn wb_restore_recipe(
    machine_id: String,
    version_id: String,
) -> Result<state::RestoreReport, AppError> {
    traced("wb_restore_recipe", |_| {
        state::restore_snapshot_recipe(&Store::open()?, &machine_id, &version_id)
    })
}

/* ---------- BBS ---------- */

#[tauri::command]
pub fn wb_bbs_list() -> Result<Vec<bbs::BbsFile>, AppError> {
    traced("wb_bbs_list", |_| bbs::list(&Store::open()?))
}

/// 从一个路径收录 BBS。
///
/// 为什么让前端传路径而不是弹系统文件对话框：本仓库没装 tauri 的 dialog 插件，
/// 为工作台单独引一个插件会连带改客户端的权限清单。这是开发者工具，
/// 粘一个路径可以接受 —— 如实登记，不假装它是完整的导入体验。
///
/// 读的是任意路径，所以这条命令**只在 workbench feature 下存在**；
/// 写入侧仍然被夹在工作台数据根内（`store.root/bbs/<id>.json`）。
#[tauri::command]
pub fn wb_bbs_import(id: String, source_path: String) -> Result<bbs::BbsFile, AppError> {
    traced("wb_bbs_import", |_| {
        let bytes = std::fs::read(&source_path).map_err(|e| {
            AppError::io(format!("读不出 {source_path}")).with_detail(e.to_string())
        })?;
        bbs::import(&Store::open()?, &id, &bytes)
    })
}

#[tauri::command]
pub fn wb_set_machine_bbs(machine_id: String, ids: Vec<String>) -> Result<(), AppError> {
    traced("wb_set_machine_bbs", |_| {
        bbs::set_machine_default(&Store::open()?, &machine_id, ids)
    })
}

#[tauri::command]
pub fn wb_set_version_bbs(
    machine_id: String,
    version_id: String,
    binding: model::BbsBinding,
) -> Result<(), AppError> {
    traced("wb_set_version_bbs", |_| {
        bbs::set_version_binding(&Store::open()?, &machine_id, &version_id, binding)
    })
}

/* ---------- 菜单与套餐 ---------- */

#[tauri::command]
pub fn wb_catalog() -> Result<catalog::Catalog, AppError> {
    traced("wb_catalog", |_| catalog::load_catalog(&Store::open()?))
}

#[tauri::command]
pub fn wb_save_catalog(value: catalog::Catalog) -> Result<(), AppError> {
    traced("wb_save_catalog", |_| {
        catalog::save_catalog(&Store::open()?, &value)
    })
}

#[tauri::command]
pub fn wb_bundles() -> Result<catalog::Bundles, AppError> {
    traced("wb_bundles", |_| catalog::load_bundles(&Store::open()?))
}

#[tauri::command]
pub fn wb_save_bundles(value: catalog::Bundles) -> Result<(), AppError> {
    traced("wb_save_bundles", |_| {
        catalog::save_bundles(&Store::open()?, &value)
    })
}

/* ---------- 出货检查与发布 ---------- */

#[tauri::command]
pub fn wb_preflight() -> Result<preflight::PreflightReport, AppError> {
    traced("wb_preflight", |_| {
        preflight::run(&Store::open()?, &paths::dist_root()?)
    })
}

/// 发布。**有阻断项就不发** —— 这条判定在 Rust 侧，不靠界面把按钮禁掉
#[tauri::command]
pub fn wb_publish() -> Result<publish::PublishReport, AppError> {
    traced("wb_publish", |_| {
        publish::run(&Store::open()?, &paths::dist_root()?)
    })
}
