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
//! **不在这里写规则**（那是 `domain`），**不在这里读磁盘**（那是 `presets` / `storage`）。
//!
//! # 预设缓存一次，其余每次现读
//!
//! | 数据 | 什么时候读 | 为什么 |
//! |---|---|---|
//! | 预设真相源（`presets/`：74 参数 / 机型 / 套餐 / 资产） | **开工作台时一次**，`wb_reload` 显式重读 | 一次操作里各处看到的必须是同一份，否则派生出来的状态会自相矛盾 |
//! | 已落盘（`workbench/`） | 每次命令现读 | 它是可变的那一面。缓存它就等于又造了一个"当前状态"的副本（doc §1 第四条铁律） |
//! | 草稿 | 每次命令现读 | 同上 |
//!
//! **唯一的数据根是 `presets/`**（旧的上游仓库连同它的三级定位、环境变量、设置项一起
//! 退休了）。一个进程只有一个工作台窗口，所以那一份放在模块级的 [`SESSION`] 里。
//! **命令本身只是薄壳**：真正的逻辑都在拿 `&Ctx` 的自由函数上，
//! 所以单测可以直接造 `Ctx`，不碰那个全局。
//!
//! # `wb_effective` 就是一列的 `wb_matrix`
//!
//! 单个版本的字段详情要的东西（值、来源、自有、被谁关着、能不能改）
//! 与矩阵单元格**一模一样**。再建一套 DTO 等于给同一件事写两个形状，
//! 迟早只改其中一个。所以字段详情走 `wb_matrix(cols=[那一列])`。

/// **发布账户配置**（第三刀下半）：`repositoryUrl` + `username`，住
/// `<appDataDir>/publish-account.json`（**不含 token**；token 住凭据文件
/// `credentials.json`，见 [`credentials`]）。发布目标由它决定，`git remote` 只作校验
pub mod account;
/// **标注源稿**（2026-10-16）：工作台「标注台」读写的那一份"能再编辑的源文件"，
/// 住在 `presets/assets/<同类目录>/<对>.anno.json`（与它描述的图**同层**、随仓库走、
/// **不进交付目录**）。路径由后端从台账派生，前端只给资产 id
pub mod annotation;
/// 「资产库」。**它也走 `Presets` 现读**（与 `machines` 同一条纪律：清单类的页面
/// 不套参数值那套状态机）。现在只有读 —— 写入口在数据层，接上要有界面（Task 14）
pub mod assets;
/// **发布闸**（第二刀）：发布前的唯一入口判定器。**它是 Rust 核心** ——
/// 工作台界面 / 将来的 CLI / `cargo test` 判据调的都是 `publish_audit` 这一个函数，
/// 界面不许自己再实现一套检查
pub mod audit;
pub mod build;
/// 「套餐管理」（b05 Task 10）。同一套纪律：只读，写入口在数据层（Task 14 接界面）
pub mod bundles;
/// 发布账户凭据（第三刀下半）：每平台一份 Token，住**本机凭据文件**（`credentials.json`，
/// 0600），前端拿不到原值
pub mod credentials;
/// 交付层（b05 Task 12）：目录类 JSON 与资产复制，`wb_publish` 落盘
pub mod delivery;
/// 「本地测试源（开发）」：一颗按钮＝把 `npm run dev:test-update` 替你敲了
/// （起本地假云端 + 以 `MKPSE_PRESET_SOURCE_URL` 起客户端 dev）。
/// **它不碰任何配置** —— 源地址是环境变量注入的，只在 debug 构建里认
pub mod dev_source;
/// **开发台 · 故障注入**（2026-10-19）：给某一个 API 方法**装一条错**，
/// 客户端里所有走这个方法的界面从此真的报错（作者：「所有模态框都能触发，
/// 而且要真实的 mock 触发」）。与 [`devsim`] 是两件事：那是"打开一格界面"，
/// 这是"让后端失一次真"。
pub mod devfault;
/// **开发台 · 真实后处理测试矩阵**（2026-10-19）：六档（成功×2 / 硬失败 / 机型不匹配 /
/// 挪塔碰撞 / 取消）各跑在**独立临时根**里，串行、只按真凭据判定（`_meta.json` 的
/// `error` / `cancelled` / `relocation`）。问句档与取消档**需人在客户端窗口真操作**
/// —— 验的不只是"管线能问"，还有"界面能显示、能收答复、能继续/停下"。
/// 数据落 `target/dev-run/matrix/`，正式数据目录一个字节不动（见该文件头）。
pub mod devmatrix;
/// **开发台 · 运行真实后处理**（2026-10-19）：选一份预设 + 一份测试 G-code，
/// 起一个**真的钩子子进程**（`--Toml` + `--Gcode`，与切片器注册的那一行逐字一致），
/// 让客户端窗口跑一趟真管线 —— 真进度、真问句、真落盘、真归档，界面上没有"预览"标记。
/// 与 [`devsim`]（演出）/ [`devfault`]（让后端失一次真）是开发台三件互不重叠的事。
/// ★ 它**不加协议变体**：复用真机那条入口即可（见该文件头）
pub mod devreal;
/// **开发台**：把"平时看不见的那几屏"做成工作台里一排按钮（先接后处理那一组）。
/// 它是假钩子 `hook-sim` 的**界面版** —— 进程内直接喂 [`crate::hook_ui::HookView`]，
/// 与真钩子来的那条线同一个落点（见模块头：工作台与客户端是同一个进程）。
pub mod devsim;
/// 本地 git 子进程封装（第三刀下半）：发布事务的「本地那一半」——白名单 stage / commit / push
pub mod git;
pub mod history;
/// 写盘日志的字段拼装：一个键一行事实，`why` 由产生写入的那段代码填
pub mod logging;
/// 「机型与版本」那一页。**它不走 `Ctx` / `Committed` / `Draft`** ——
/// 那一套是参数值的，这一页管清单，两件事不共用状态机（见该文件头）
pub mod machines;
/// 代码托管平台出口（第三刀下半）：GitHub / Gitee 的 PR/MR + CI，**自持凭据**
/// （被批准的第二个网络出口，见模块文档）
pub mod platform;
/// **发布账本**：`presets/delivery/publish-serial.json` —— 发布线身份 + 单调序号。
/// 只有正式发布事务能推进它；生成 / 预演只沿用（见该文件头）
pub mod publish_book;
/// **发布事务**（第三刀下半）：audit → generate → 定稿 → 本地 git → 平台 PR/MR，
/// 串成一次手势 + 平台无关的状态模型。**锁无关内核**，只收 `&Ctx`
pub mod publish_tx;
/// **生成事务回执**（2026-11-01）：一次生成留下的一份结构化结果 ——
/// 工作台模态框 / CLI JSON / AI 报告读的是同一份，不是三套说法。
pub mod receipt;
/// **软件版本发布历史**（第四刀）：`<appDataDir>/release-history.json`。
/// ★ **与交付预设那本分开** —— 两条链不混
pub mod release_history;
/// **发布软件版本事务**（第四刀）：确认主线 → 版本号 → tag → 构建安装包 →
/// Release → 上传 → 写 `release.json`。**锁无关内核**（不碰 `Presets`，因此不进 `with_ctx`），
/// 工作台与 CLI 两个入口共用它
pub mod release_tx;
/// **成品图渲染**（2026-10-16，方案 A）：把标注**烧进图片**，产出交付给客户端的
/// `<图干>.rendered.webp`（无损 webp，纯 Rust 编码）—— 客户端 `<img>` 直接显示，
/// 不再"读原图 + 前端现画框"。原图与源稿照旧留存（像 PSD）可再编辑
pub mod render;
/// 参数台 v2 的只读数据面（2026-11-02）。**与旧 `wb_matrix` 并行存在**，
/// 旧命令一个字节不改；新页只走这一条。见 `docs/WORKBENCH-SHEET-V2-PROTOCOL.md`。
pub mod sheet;

/// **发布身份的故障注入判据**（2026-10-28）：中断 / 重发 / 回滚 / 换线 / 坏账本。
/// 见该文件头那张表 —— 咬的就是"出错时也必须说得清"。
#[cfg(test)]
mod fault_tests;

/* ---------- CLI 门面（`bin/presets`）---------- */

/// **预设工作流 CLI 的唯一入口**：把内核里那几处自由函数转出来给 `bin/presets`。
///
/// 为什么要有这几行（2026-10-28）：`audit_with` / `generate_with` / `preview_with` /
/// `publish_tx::run` 都是 `pub(super)`（锁纪律：库里只能从 `app` 里调）。CLI 是
/// `bin/`，在 crate 外面，够不着 —— 于是这里开一扇**只转发、不加逻辑**的门。
///
/// ★ 纪律：这一节**不许有业务判断**。谁在这里加一句"应该怎样"，工作台与 CLI 就
///   开始各说各话了 —— 那正是这一轮要根治的毛病（两套入口必须同一条路）。
pub use self::audit::{AuditStatus, PublishAudit};
pub use self::build::{DiffState, GenerateReport, PreviewReport, Scope};
pub use self::publish_tx::{PublishStage, PublishTarget, TxOptions};

/// 跑发布闸（只读，不写盘）。工作台按钮与 CLI `check` 走的是同一个函数。
pub fn audit_check(ctx: &Ctx) -> Result<PublishAudit, AppError> {
    self::audit::audit_with(ctx)
}

/// 跑生成器（`Scope::Stale` / `All`）。**不推进发布序号**（序号在 `publish_into`）。
pub fn generate(ctx: &mut Ctx, scope: &Scope) -> Result<GenerateReport, AppError> {
    /* 与工作台那颗按钮同一道闸（2026-10-31）：CLI 也不许拿旧影子覆盖真仓库 */
    self::build::require_shadow_in_sync(ctx)?;
    self::build::generate_with(ctx, scope, false)
}

/// 生成**预演**（只算不写）。`diff` 用它 —— 于是"会变什么"与真生成算的是同一批。
pub fn generate_preview(ctx: &Ctx) -> Result<PreviewReport, AppError> {
    self::build::preview_with(ctx, &Scope::Stale)
}

/// 发布事务（工作台那颗按钮的内核）。
pub fn publish_tx(
    ctx: &mut Ctx,
    opts: &TxOptions,
    target: Option<&PublishTarget>,
    hosting: Option<&dyn self::platform::Hosting>,
    repo_root: Option<std::path::PathBuf>,
) -> Result<self::publish_tx::PublishTxReport, AppError> {
    self::publish_tx::run(ctx, opts, target, hosting, repo_root)
}

/// 解析发布账户配置（没配 ⇒ `None`，退化成纯本地推送）。与工作台同一条。
///
/// 配置住 `<appDataDir>/publish-account.json`（+ 凭据文件）；读不出来 / 不完整一律
/// 当"没配"（不是错误）—— 那时发布只走本地那一半，如实说"没建 PR"。
pub fn resolve_target_opt(_ctx: &Ctx) -> Result<Option<PublishTarget>, AppError> {
    /*
     * CLI 里没有 `AppHandle`，配置根从**环境**取（与工作台同一条规则：
     * 应用数据目录）。取不到 = 这台机器上没配发布账户 ⇒ 只走本地那一半。
     */
    let Some(root) = crate::fsx::paths::internal_root_headless() else {
        return Ok(None);
    };
    Ok(self::publish_tx::resolve_target(&root, None).ok())
}

/// 按发布目标建平台出口（没配 / 平台不认识 ⇒ `None`）。
pub fn hosting_for(
    target: &Option<PublishTarget>,
) -> Result<Option<Box<dyn self::platform::Hosting>>, AppError> {
    let Some(t) = target else { return Ok(None) };
    Ok(match t.platform.as_str() {
        "github" => Some(Box::new(self::platform::github::GitHub::new(
            t.token.clone(),
        ))),
        "gitee" => Some(Box::new(self::platform::gitee::Gitee::new(t.token.clone()))),
        _ => None,
    })
}

/// 交付根的**一致性体检**结论（CLI `verify` 用）。
#[derive(Debug, Clone, Default)]
pub struct VerifyReport {
    /// 核对了多少份文件（catalog 的 `files` 条数）
    pub files_checked: usize,
    /// 一处问题一句话（空 = 全一致）
    pub problems: Vec<String>,
    pub serial: Option<u64>,
    pub catalog_id: Option<String>,
}

/// **核对交付根**：catalog 能不能解析、登记的文件在不在、字节对不对、manifest 与账本
/// 跟目录是不是同一代。**只读**（一个字节都不写）。
///
/// 与发布事务里的 `finalize_consistency` 不同：那个是"刚定稿的这一刻"的严格核对；
/// 这个是**随时可以问一次**的体检（人和 AI 在改完 TOML 之后想知道"盘上还自洽吗"）。
pub fn verify_delivery(
    delivery_root: &std::path::Path,
    publish_root: &std::path::Path,
) -> Result<VerifyReport, AppError> {
    let mut report = VerifyReport::default();
    let catalog_path = delivery_root.join(self::delivery::NEW_CATALOG_FILE);
    let bytes = std::fs::read(&catalog_path).map_err(|e| {
        AppError::not_found(format!("{} 读不出来", catalog_path.display()))
            .with_detail(e.to_string())
    })?;
    let catalog = crate::runtime::Catalog::parse(&bytes)?;
    report.files_checked = catalog.files.len();
    report.serial = catalog.publish_serial;
    report.catalog_id = catalog.catalog_id.clone();

    /* ① 目录登记的每一份，在发布根下真有、字节对得上 */
    for f in &catalog.files {
        let on_disk = publish_root.join(&f.path);
        let Ok(raw) = std::fs::read(&on_disk) else {
            report.problems.push(format!(
                "目录登记了 {}，发布根里没有这份文件（{}）",
                f.path,
                on_disk.display()
            ));
            continue;
        };
        if let Some(want) = f.expected_sha() {
            use sha2::Digest as _;
            let got = crate::runtime::catalog::hex(&sha2::Sha256::digest(&raw));
            if !want.eq_ignore_ascii_case(&got) {
                report
                    .problems
                    .push(format!("{} 的字节与目录登记的 SHA 不一致", f.path));
            }
        }
    }

    /* ② 账本与目录是不是同一代（两个都说得出身份时才比） */
    match publish_book::load(delivery_root)? {
        None => {
            if catalog.publish_serial.is_some() {
                report
                    .problems
                    .push("目录有发布序号，但交付根里没有发布账本 —— 生成 / 发布不同源".to_owned());
            }
        }
        Some(book) => {
            if !book.matches_catalog(&bytes) {
                report.problems.push(format!(
                    "账本说的是第 {} 号（{}），与盘上这份目录不是同一代",
                    book.serial, book.revision
                ));
            }
            if book.catalog_id != catalog.catalog_id.clone().unwrap_or_default() {
                report
                    .problems
                    .push("账本的发布线与目录的发布线不一致".to_owned());
            }
        }
    }
    Ok(report)
}
pub mod storage;
/// **版本号：唯一真值 + 单向派生**（第四刀）：`src-tauri/Cargo.toml` 真身，
/// `package.json` / `tauri.conf.json` / `Cargo.lock` 是派生结果
pub mod version;
pub mod words;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::AppError;
use crate::ipc::traced;
use crate::workbench::domain::derive::{Book, BookView, ColRef, Desk, Matrix};
/* 编辑历史：与**本模块自己的 `history`（发布回执）同名** —— 它必须起个别的名字 */
use crate::workbench::domain::history as edit_history;

use crate::workbench::domain::layer::Layers;
use crate::workbench::domain::patch::{apply as apply_patches, Draft, Patch};
use crate::workbench::domain::preview::BulkPreview;
use crate::workbench::domain::shadow_diff;
use crate::workbench::domain::txn::{self, Op, Txn};
use crate::workbench::domain::variants;
use crate::workbench::domain::wording as w;
use crate::workbench::domain::{Committed, Level};
use crate::workbench::presets::registry::{
    ParamDef, ParamMetaEdit, SectionMetaEdit, ShowWhen, TabMeta, UiComponent, ValueType,
};
use crate::workbench::presets::Presets;
use crate::workbench::store::{Store, TrashEntry};
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
    /// 我们自己那份预设数据。**机型与版本的清单从这里来**（b04 Task 8），
    /// 而且它是可写的 —— 新建版本要落进 `presets/machines/*.toml`
    pub presets: Presets,
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
    /// 这个会话读进来时，`presets/` 源文件的盘上戳（见 [`presets_source_stamp`]）。
    ///
    /// 拿来回答一个问题：**盘上现在跟界面手里的是不是同一份**。外部改过要能在
    /// 窗口切回来时被发现（作者 2026-10-20：「我这个软件完全没有看到改的呀」），
    /// 但**不自己重载** —— 会话里可能正开着一个草稿，换掉定义会把旧值按新定义存回去
    source_stamp: String,
    /// **真仓库的 `presets/`**。保存时只往这里写 —— 编辑一律写影子树
    real_root: PathBuf,
    /// **影子工作树**（`workbench/.draft/presets/`）。所有编辑写这里
    shadow_root: PathBuf,
    /// **盘上已保存的那一份**（2026-11-02 刀 1）—— 「改之前的值」唯一来源。
    ///
    /// 从 `txn::base_root()`（`.draft/base/`）加载，与 `presets` **必须是两份数据**：
    /// `presets` 是编辑树（值写进去就变），`base` 只在「打开会话」与「保存成功」时推进。
    /// 有了它，`Book::with_rest` 才答得出「这一格保存前是多少」。
    ///
    /// 之前没有这一格，`rest_at` 与当前值同源 ⇒ `dirty` 恒假 ⇒ 徽章永不亮（见 `derive::Book::rest`）。
    base: Presets,
    /// 统一编辑事务（2026-10-31）：结构编辑的撤销/重做/保存/放弃。
    ///
    /// ★ 2026-11-03 起**值编辑不再走它** —— 那条走 [`Self::history`]（内存命令历史）。
    /// 迁移期两套栈并存：值编辑 = `history`，结构编辑 = `txn`（见
    /// `docs/WORKBENCH-EDITOR-MODEL.md` §9）。
    txn: Txn,
    /// **值编辑的命令历史**（2026-11-03）：撤销记的是"哪个参数改前多少改后多少"，
    /// 不是文件字节。它也是「未保存 N（处）」「改了什么」在值这一侧的原料
    history: edit_history::History,
    /// **这一条命令改的是哪一格**（2026-11-02）。`wb_apply_draft` 在动手前填上，
    /// [`run_ctx`] 记事务时取走交给 [`Txn::record`] —— 同一格 5 秒内的连点并成一条。
    /// 结构编辑不填（`None`），那类永远不合并
    edit_key: Option<txn::EditKey>,
    /// **这一条命令真的改了哪些字段**（2026-11-03）：命令在写盘**之后**填，
    /// [`run_ctx`] 记事务时取走交给 [`Txn::record_with_fields`]。
    ///
    /// 这是「未保存 N」与「改了什么」的原料 —— 必须由**写盘那一刻的事实**产出
    /// （值类来自 `apply_draft_now_traced` 的 `KeyChange`，结构类来自
    /// `ParamDef::summary` / `Dimensions::summary`），不许事后从影子树反推。
    /// 不填的命令（只改文件结构、没有"字段"可言的）用空 vec ⇒ 那一笔不计处数。
    pending_fields: Vec<txn::FieldEntry>,
    /// **"几份文件有改动"的缓存**。影子树与真源的全量比对要读两百来 KB，
    /// 而界面每 800 ms 轮询一次「未保存 N」—— 每次现算等于每帧一次全树读。
    /// 由 [`Ctx::refresh_unsaved_files`] 在**每条命令跑完之后**刷新（那时才可能变），
    /// 其余时刻读这一格
    unsaved_files: usize,
}

/// 打开会话时**先把影子树这件事定下来**：影子要不要对齐、有没有草稿。
///
/// 返回 `recovered`：`true` = 影子里有用户的未保存草稿（含冲突），界面按「一条未保存改动」呈现。
///
/// ★ **会话永远读影子树**（`<workbench>/.draft/presets`），真源只用来"铺"。
/// 2026-11-01 22:17 那桩案子就是这里改错的代价：`Ctx::open` 一度改成"干净时读真源"，
/// 编辑于是直接写进真仓库、而事务快照还在拍影子 ⇒ 差集恒为空 ⇒ 保存永不亮、放弃退不回。
/// **别再"优化"回去。**
///
/// # 影子树的一生（2026-11-01 收口）
///
/// **`presets/` 是唯一权威源；影子树只代表"还没保存的草稿"。**
///
/// | 影子 | 基准 | 真源 | 判定（`recovered`）与动作 |
/// |---|---|---|---|
/// | 空 | — | — | 铺影子 + 铺基准（内容 = 真源），`false` |
/// | =基准 | =基准 | =基准 | 干净；影子若落后就对齐，`false` |
/// | 改了 | 未变 | 未变 | **用户的草稿** → 影子原样保留，`true` |
/// | 未变 | 未变 | 改了 | **外部修改** → 对齐影子+基准，`false` |
/// | 改了 | 未变 | 改了 | **冲突** → 影子原样保留，等人定，`true` |
///
/// 不论哪一行，**开会话读的都是影子**（上表第一列）—— 干净时它刚被对齐成真源，
/// 有草稿时它就是编辑态。
///
/// # 前两轮踩过的坑（记在这里免得走回头路）
///
/// · 一开始"影子非空就读影子" ⇒ 外部改源被当成草稿，`diff` 报"0 份会变"；
/// · 改成"只看影子独有文件" ⇒ 认不出"用户改了既有文件"，草稿会被静默覆盖；
/// · 现在靠**基准快照**（`txn::base_root`）三方对比，才分得出谁动了什么。
///
/// # 基准读不出来时（2026-11-01 R1）
///
/// **绝不拿真源顶替**。以前 `unwrap_or_else(|_| real_snap.clone())` 会让
/// "基准 = 真源"，于是"真源改过、影子没改"变成"影子改了、真源没改" ——
/// 凭空冒出一堆草稿，`reload` 每次都问、生成每次都被拦，且**不会自愈**。
///
/// 现在的做法是：基准缺失 ⇒ 只有**能证明影子没有未保存内容**才重建基准，
/// 否则一律按"有草稿"处理（读影子、让人看见），宁可多问一句也不丢东西。
fn settle_shadow(workbench_root: &Path, real_root: &Path) -> Result<bool, AppError> {
    use crate::workbench::domain::txn::{self, ShadowVerdict};

    let shadow_root = txn::shadow_root(workbench_root);
    let base_root = txn::base_root(workbench_root);
    /* 工作区现在住在应用数据目录（`paths::set_workspace_root`），父目录可能还没建过 ——
    `mirror` 的 rename 需要一个存在的落点。这里先建齐，空目录不影响下面那条
    "影子是不是空"的判断。 */
    std::fs::create_dir_all(&shadow_root)?;
    std::fs::create_dir_all(&base_root)?;

    let shadow_snap = txn::snapshot(&shadow_root)?;
    if shadow_snap.is_empty() {
        // 没有影子：整份铺一份，并记下"从哪一份真源铺的"
        txn::mirror(real_root, &shadow_root)?;
        txn::mirror(real_root, &base_root)?;
        return Ok(false);
    }

    let real_snap = txn::snapshot(real_root)?;
    /* **空快照也算缺失**（2026-11-01）：`txn::walk` 对不存在的目录返回 `Ok(())`，
     * 于是"基准目录不在"会得到一个**空的** `Ok`。若把它当成有效基准，
     * 每份文件都会被算成"影子/真源新增"，结论全错。
     * 空基准跟没有基准是一回事 —— 都证明不了"影子从哪里变过来的"。 */
    let base_snap = txn::snapshot(&base_root).ok().filter(|b| !b.is_empty());

    let verdict = match &base_snap {
        Some(base) => shadow_snap.reconcile(&real_snap, base),
        /* 基准读不出来（目录不在 / 文件缺 / 内容坏了）—— **不许拿真源冒充基准**。
         *
         * 唯一能安全重建基准的情形：**影子与真源逐字节相同**。
         * 那说明影子没有任何独有内容（连"用户改了既有文件"都不可能 ——
         * 改了就会不同），重建基准不会抹掉任何人的改动。
         *
         * 其余情况一律按"有草稿"处理：读影子、让人看见。
         * 宁可多问一句，也不能静默覆盖用户的编辑。 */
        None => {
            let identical = shadow_snap.identical_sources(&real_snap);
            if identical {
                txn::mirror(real_root, &base_root)?;
                return Ok(false);
            }
            tracing::warn!(
                "基准快照缺失或损坏，且影子与真源不同 —— 按「有未保存草稿」处理，不重建基准"
            );
            ShadowVerdict {
                drafts: shadow_snap
                    .source_paths_union(&real_snap)
                    .into_iter()
                    .collect(),
                ..ShadowVerdict::default()
            }
        }
    };

    if verdict.safe_to_reset() {
        // 没有草稿也没有冲突：影子最多是旧拷贝，对齐它（"外部改源"的常态）
        txn::mirror(real_root, &shadow_root)?;
        txn::mirror(real_root, &base_root)?;
    } else {
        /* ★ **草稿不跨重启**（2026-11-02 作者裁定）。
        以前这里把工作区原样留着、界面按「一条未保存改动」呈现，靠 `Txn::recovered()`
        撑住那条合成 op。新口径是：**关掉工作台 = 草稿没了**，下次打开以磁盘为准。
        判定照旧跑（认得清"上次有草稿 / 有冲突"），只是动作从"留着"改成"以真源为准"。
        */
        tracing::warn!(
            drafts = ?verdict.drafts,
            conflicts = ?verdict.conflicts,
            "上次会话有未保存的改动 —— 按新口径丢弃，以磁盘那一份为准"
        );
        txn::mirror(real_root, &shadow_root)?;
        txn::mirror(real_root, &base_root)?;
    }
    /* 永远没有"恢复出来的草稿"：工作区要么刚铺、要么刚对齐真源 */
    Ok(false)
}

impl Ctx {
    /// **真仓库的 `presets/`**（编辑落在影子树，只有保存 / 发布才碰这里）。
    ///
    /// ★★★ 2026-10-31：**发布与核对必须用它，不能用 `presets.root()`**。
    ///
    /// `presets.root()` 是**影子树**（`workbench/.draft/presets/`）。发布若落在影子，
    /// 而 `git.stage_allowed` stage 的是真仓库里的 `presets/delivery/`，结果就是
    /// 「生成阶段写进真仓库的那几份」（catalog 的 revision / 参数改动）进了 commit，
    /// **账本与发布戳留在影子里** —— `presets verify` 读影子反而绿、`check:artifacts`
    /// 读真实树红（R1「交付目录变了但账本没变」），push 被闸拦下。
    pub fn real_root(&self) -> &Path {
        &self.real_root
    }

    /// 真仓库。
    ///
    /// **唯一的数据根是 `presets/`**（旧的上游仓库连同它的定位、设置项一起退休了）：
    /// 机型 / 参数 / 套餐 / 资产 / 交付产物都在 `presets/` 下面。
    ///
    /// 会失败的是 `presets/` 定位不到 —— 那是清单与字段定义的唯一来源，
    /// 缺了它每一页都是空的，装成能跑只会让人以为"我们没有机型"。
    pub fn open() -> Result<Self, AppError> {
        let store = Store::open()?;
        let real_root = crate::workbench::paths::presets_root().ok_or_else(|| {
            AppError::not_found("找不到工作台的预设数据根 presets/").with_detail(
                "期望 <repo>/presets/registry/param_registry.toml 存在 —— \
                 工作台的数据根指向**仓库里的开发源数据**，不是 appDataDir"
                    .to_owned(),
            )
        })?;
        Self::open_at(store, real_root)
    }

    /// [`Self::open`] 的本体（**抽出来是为了可测**）：给定工作台数据根与真仓库开一个会话。
    ///
    /// IPC 入口与回归判据走的是同一条路 ——
    /// `a_session_opened_from_disk_edits_the_shadow_until_save` 直接调它，
    /// 所以"真机上点箭头为什么会写进 `presets/`"这类问题能在测试里重放。
    pub(super) fn open_at(store: Store, real_root: PathBuf) -> Result<Self, AppError> {
        store.bootstrap()?;
        let shadow_root = crate::workbench::domain::txn::shadow_root(&store.root);
        /* 上次「整目录替换」没做完（崩溃/强杀）⇒ 先收拾现场，再让 `settle_shadow` 去看。
         *
         * 顺序是死的：`settle_shadow` 判「影子里有没有草稿」要读影子树，
         * 而影子的完整性正由这一步保证。两个根都要收拾 —— `save_in` 会
         * `mirror(real → base)`，`wb_generate` 会 `mirror(real → shadow)`，
         * 两处都可能停在「旧的让开了、新的没进来」。 */
        let base_root = txn::base_root(&store.root);
        for line in crate::workbench::domain::txn::recover_dir_replace(&shadow_root)? {
            tracing::warn!("{line}");
        }
        for line in crate::workbench::domain::txn::recover_dir_replace(&base_root)? {
            tracing::warn!("{line}");
        }
        /* ★ **保存阶段补完**（刀 5），必须在 `settle_shadow` 之前：
        上次保存可能停在「磁盘已换、基准未搬」，那一步不补，
        `settle_shadow` 会把"已保存"误判成"用户有草稿"，
        接着用户点「放弃」就把刚存的东西退回去。 */
        for line in txn::recover_save_stage(&store.root, &real_root, &base_root)? {
            tracing::warn!("{line}");
        }
        let recovered = settle_shadow(&store.root, &real_root)?;
        /* ★ **读影子树，不读真源**（2026-11-01 悬案的正解）。
         *
         * `settle_shadow` 已经保证影子里是"该看见的那一份"：没草稿时它刚被对齐成真源，
         * 有草稿时它就是用户的编辑态。**编辑只能写影子**（`wb_save` 才 commit 到真仓库），
         * 所以会话的 `Presets` 根必须指影子 —— 指真源就等于"边改边落盘"：
         * 事务快照还在拍影子，差集恒空，保存永不亮、放弃退不回。
         * 这个错误曾经真实发生过（22:17 那场），别再把 `source_root` 引回来。 */
        let mut presets = crate::presetdata::Presets::load_from(&shadow_root)?;
        if let Ok(asset_root) = crate::workbench::paths::assets_root() {
            // 载荷根仍指真仓库：资产是只读读出来的，编辑走「暂存 + 保存时搬」
            presets.set_asset_root(&asset_root);
        }
        /* `base_root` 在上面已经算过 —— `store` 马上要被 move 进会话 */
        let mut ctx = Self::with(presets, store)?;
        ctx.real_root = real_root;
        ctx.shadow_root = shadow_root;
        /* `Self::with` 里那次 `reload_from_disk` 记的是**影子**的盘上戳；
         * 真仓库换上之后必须重记一次 —— 否则第一条编辑命令前后比的是真仓库、
         * 参照的却是影子戳，会被误判成"盘上被别人改过"。 */
        ctx.source_stamp = presets_source_stamp(&ctx.real_root);
        /* **盘上已保存的那一份**（刀 1）。`settle_shadow` 刚刚把它铺好/对齐过，
        所以这里读到的就是「打开会话时 / 上次保存后」的内容 ——
        行上那枚 `↩ 改之前的值` 的唯一来源。 */
        ctx.base = Presets::load_from(&base_root)?;
        /* **草稿不跨重启**（2026-11-02）：`settle_shadow` 已经把工作区对齐成磁盘那一份，
        所以这里永远是"干净的一次新会话" —— 记下磁盘此刻的样子即可。
        `Txn::recovered()` 那条合成 op 随之退休。 */
        let _ = recovered;
        txn::write_disk_stamp(&ctx.store.root, &presets_source_stamp(&ctx.real_root))?;
        /* 两个根换过之后重算：`with` 那次比的是"预设根 vs 自己"（恒等，必为 0），
        而这里才是真正的"真仓库 vs 影子树" */
        ctx.refresh_unsaved_files();
        Ok(ctx)
    }

    /// 给定预设数据与仓库建一个会话。测试用这一条，不碰真仓库也不碰那个全局。
    /// （`Ctx::open_at` 随后会把 `real_root` / `shadow_root` 换掉。）
    pub(super) fn with(presets: Presets, store: Store) -> Result<Self, AppError> {
        /* 测试夹具没有影子树：两个根都指回它自己的那份预设根（镜像 = 恒等）。
        `Ctx::open` 会随后把真仓库 / 影子树填上。 */
        let root = presets.root().to_path_buf();
        /* 「改之前的值」要读**另一份**数据。测试夹具里两份同源（镜像 = 恒等），
        所以 `dirty` 恒假 —— 这是对的：夹具没有"保存过"这个动作。
        `open_at` 随后会把 `base` 换成 `.draft/base/` 那一份。 */
        let base = presets.reload()?;
        let mut ctx = Self {
            presets,
            base,
            store,
            committed: Committed::default(),
            draft: Draft::default(),
            pending_fields: Vec::new(),
            notices: Vec::new(),
            dirty_seq: 0,
            flushed_seq: 0,
            last_edit: Instant::now(),
            flush_error: None,
            source_stamp: String::new(),
            real_root: root.clone(),
            shadow_root: root,
            txn: Txn::new(),
            history: edit_history::History::new(),
            edit_key: None,
            unsaved_files: 0,
        };
        ctx.reload_from_disk()?;
        ctx.refresh_unsaved_files();
        Ok(ctx)
    }

    /// 重算「几份文件有改动」并写回缓存。**唯一实现** ——
    /// 页头那颗「未保存（N）」、状态栏、`TxnState`、以及「代码对比」里的「N 份文件」
    /// 都从这里来，不许各自再算一套（两套口径迟早不一致，作者 2026-11-02 报的
    /// 「改一次涨到 14」就是"按编辑次数"和"按文件数"两套口径打架）。
    ///
    /// 口径与 `wb_shadow_diff` 逐字相同：`snapshot` 两份、`without_products()` 滤掉
    /// `delivery/`、比差集。`assets/` 里的二进制只比头部指纹（够判"换了没有"）。
    fn refresh_unsaved_files(&mut self) {
        let count = (|| -> Result<usize, AppError> {
            let a = txn::snapshot(&self.real_root)?.without_products();
            let b = txn::snapshot(&self.shadow_root)?.without_products();
            Ok(txn::changed_files(&a, &b, &self.real_root, &self.shadow_root)?.len())
        })();
        match count {
            Ok(n) => self.unsaved_files = n,
            Err(e) => {
                /* 读不出来不许假装 0：那是"看着干净、其实有改动"。留上一轮的数并记一笔 */
                tracing::warn!(error = %e.message, "算不出未保存改动份数");
            }
        }
    }

    /// 几份文件有改动（读缓存，零开销）
    fn unsaved_files(&self) -> usize {
        self.unsaved_files
    }

    /// 事务视图（撤销/重做/保存按钮读它）。
    ///
    /// `dirty_count` = **改了几行**（2026-11-03 定案，见 `Txn::dirty_count` 的注）。
    ///
    /// 三个数各答一件事，别再互相顶替：
    ///
    /// | 数 | 答什么 | 读哪儿 |
    /// |---|---|---|
    /// | `dirty_count` | 文件里动了几行 | `Txn::dirty_count()` + 草稿层 |
    /// | `ops.len()` | 撤销栈有几格 | `txn.ops()` |
    /// | `unsaved_files` | 保存会写几份文件 | `unsaved_files()` |
    ///
    /// 作者 2026-11-03 报的「改了很多还是只有 1」就是第一格曾经写成第三个：
    /// 一次 `setValue` 连带改四行，按文件数永远只算 1。
    pub(super) fn txn_state(&self) -> TxnState {
        TxnState {
            /* 「脏」不再问影子树（值编辑根本不写它）：历史里有字段 / 草稿里有挂着的 /
            结构编辑有字节差 —— 三处任一非空就是脏。`unsaved_files` 只作兜底
            （它数的是影子与真源的差，值编辑不写影子之后它恒为 0） */
            dirty: !self.history.is_empty()
                || !self.draft.is_clean()
                || self.txn.is_dirty()
                || self.unsaved_files() > 0,
            /* 「改了几处」= 历史里还剩几个字段 + 草稿里挂着的格 + 结构编辑的字段。
            **必须是同一个量纲（一格字段）**：历史那边按 `(层, 归属, 键)` 去重，
            与 `txn::FieldEntry` 同一把尺子。 */
            dirty_count: edit_history::dirty_fields(
                &self.history.entries().cloned().collect::<Vec<_>>(),
            ) + self.draft.dirty_count()
                + self.txn.dirty_count(),
            can_undo: self.history.undo_depth() > 0 || self.txn.undo_depth() > 0,
            can_redo: self.history.redo_depth() > 0 || self.txn.redo_depth() > 0,
            ops: self.txn.ops(),
            /* 回执与事务状态同一趟取走：前端本来就在轮询这一个（800ms），
            不必为它另开一条通道 */
            receipt: take_receipt(),
        }
    }

    /// **重建内存文档，但把值编辑历史重放回去**（2026-11-03）。
    ///
    /// 值编辑**只落内存**，所以任何"从影子树重建 `ctx.presets`"的动作都会把它们
    /// 冲掉 —— 这条路统一走这里，重建完立刻 `replay_into`。
    ///
    /// 例外：`wb_save` 之后（历史已清空）、`wb_reload` / `discard`（语义就是丢弃），
    /// 那几处用各自的路径。
    fn reload_presets(&mut self) -> Result<(), AppError> {
        self.presets = self.presets.reload()?;
        self.history
            .replay_into(&mut self.presets, &self.committed)?;
        Ok(())
    }

    /// 从磁盘重建「清单 + 已落盘 + 草稿快照」。**只在三处调**：开场、`wb_save` 之后、`wb_reload`。
    ///
    /// **清单也一起重读**：它在 `presets/` 里，而「机型与版本」页会直接往那儿写
    /// （那一页每次操作自己读自己写）。不重读的话，这边的树会比盘上旧一步，
    /// 而"界面显示的必须是落盘结果"是这个项目的头一条纪律。
    ///
    /// 清理指向已消失对象的草稿条目也在这里
    fn reload_from_disk(&mut self) -> Result<(), AppError> {
        /* **重建内存文档 + 重放值编辑历史**（2026-11-03）：值编辑只落内存，
        直接 `reload()` 会把它们冲掉（`reload_presets` 里补了重放那一步）。 */
        self.reload_presets()?;
        let loaded = storage::load(&self.store, &self.presets)?;
        let mut draft = storage::read_draft(&self.store)?;
        let mut notices = loaded.notices;
        let pruned = draft.prune(&loaded.committed);
        let dirty = !pruned.is_empty();
        notices.extend(pruned);

        self.committed = loaded.committed;
        self.draft = draft;
        self.notices = notices;
        /* 刚读的那一份就是"界面手里这份"——盘上戳跟着记下来。
         *
         * ★ 盯的是**真仓库**（`real_root`），不是 `presets.root()`：后者是影子树，
         * 我们自己每改一笔它都在动，拿它当"外部改过"的参照等于永远误报。
         * 这条在 `Ctx::open` 读到影子之后尤其要紧（2026-11-01）。 */
        self.source_stamp = presets_source_stamp(&self.real_root);
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
/// **命令 → 人类标签**（撤销栈 / 日志 / 状态栏看的那句话）。键是 IPC 名。
///
/// ★ 新增写命令时这里要加一行，否则撤销栈里那一条只能显示 IPC 名。
const COMMAND_LABELS: &[(&str, &str)] = &[
    ("wb_set_machine_field", "改机型信息"),
    ("wb_set_version_field", "改版本信息"),
    ("wb_set_brand_field", "改品牌信息"),
    ("wb_add_machine", "新增机型"),
    ("wb_add_version", "新增版本"),
    ("wb_copy_version", "复制版本"),
    ("wb_copy_recipe", "复制版本参数"),
    ("wb_remove_version", "删除版本"),
    ("wb_add_brand", "新增品牌"),
    ("wb_move_machine_to_brand", "移动机型"),
    ("wb_set_machine_dimensions", "改机型尺寸"),
    ("wb_set_machine_zones", "改禁区"),
    ("wb_set_bundle_refs", "改套餐成员"),
    ("wb_add_bundle", "新建套餐"),
    ("wb_rename_bundle", "套餐改名"),
    ("wb_copy_bundle", "复制套餐"),
    ("wb_remove_bundle", "删除套餐"),
    ("wb_assign_bundle_versions", "指派版本到套餐"),
    ("wb_remove_asset", "删除资产"),
    ("wb_set_asset_delivery", "改资产交付档"),
    ("wb_set_calibration_identity", "改校准身份"),
    ("wb_set_group", "改资产分组"),
    ("wb_remove_group", "删除资产分组"),
    ("wb_set_param_meta", "改参数定义"),
    ("wb_set_section_meta", "改分组名与位置"),
    ("wb_remove_section", "删除分组"),
    ("wb_anno_write", "保存标注"),
];

/// **还走 `txn` 文件事务栈的编辑命令**（迁移期 2026-11-03 只剩这一条）。
///
/// 其余写命令都已迁走：
///
/// - **值编辑**走 `wb_apply_draft` + `ctx.history`（只落内存，`docs/…-EDITOR-MODEL.md` §1）；
/// - **结构编辑**走 [`with_struct_ctx`]（一条命令 = 若干文件的前后字节，
///   一对象一文件，粒度正好；不再拍整树快照）。
///
/// `wb_anno_write` 暂留：它一趟写**源稿 JSON + 一批成品图**（`render_group`），
/// 文件清单要从渲染结果里拿，等标注台一起搬。**留着不冲突** ——
/// `undo_in` / `redo_in` 按"谁后发生"选栈（`undo_last_at`）。
const TXN_EDIT_COMMANDS: &[&str] = &["wb_anno_write"];

thread_local! {
    /// 当前正在跑的命令名。`traced` 设置、跑完还原（支持嵌套）
    static CURRENT_COMMAND: std::cell::Cell<Option<&'static str>> = const { std::cell::Cell::new(None) };
    /// 当前这一笔的**请求号**（`r0000002a`）。与命令名同一层、同一生命周期
    static CURRENT_REQ: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// **成功返回的信封**：`{"req": "r0000002a", "value": <原来的返回值>}`（2026-11-02）。
///
/// 为什么不改每条命令的返回类型：工作台有二百多条命令，逐条包一层是二百处改动、
/// 二百处出错的机会。包在 [`crate::ipc::traced`] 这**一个出口**上，前端只需在一个
/// 地方拆（`api.ts` 的 `invoke`）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct WithReq<T> {
    pub req: String,
    pub value: T,
}

pub mod reqed;

/// 请求号的单调计数器。进程内唯一、只增不减 —— 一次手势 → 队列 → IPC → 后端 →
/// 派生 → 前端刷新，靠同一个 `req=` 串起来（见 [`crate::ipc::traced`]）
static REQ_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// 下一个请求号。十六进制、定宽，便于 `grep` 与按长度对齐
fn next_req() -> String {
    let n = REQ_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("r{n:08x}")
}

/// `traced` 进入时调：返回上一个命令名，供 [`leave_command`] 还原
pub(crate) fn enter_command(name: &'static str) -> Option<&'static str> {
    let req = next_req();
    CURRENT_REQ.with(|c| *c.borrow_mut() = Some(req));
    CURRENT_COMMAND.with(|c| c.replace(Some(name)))
}

pub(crate) fn leave_command(previous: Option<&'static str>) {
    CURRENT_REQ.with(|c| *c.borrow_mut() = None);
    CURRENT_COMMAND.with(|c| c.set(previous));
}

thread_local! {
    /// 最近一次命令发出的请求号（`traced` 退出前写下，`reqed` 取走）。
    ///
    /// 为什么要多这一格：`CURRENT_REQ` 在 `leave_command` 里就清了，而"把号交给前端"
    /// 发生在命令**之后**。两格各管一件事：一格回答"现在在跑哪一笔"（给日志），
    /// 一格回答"刚跑完的是哪一笔"（给返回值外面那层信封）。
    static LAST_REQ: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// `traced` 退出前调：把这一笔的号留给 `reqed`。
pub(crate) fn remember_req(req: &str) {
    LAST_REQ.with(|c| *c.borrow_mut() = Some(req.to_owned()));
}

/// 取走最近一次的号（取过就空，避免把上一笔的号认成本笔的）。
pub fn take_last_req() -> String {
    LAST_REQ.with(|c| c.borrow_mut().take().unwrap_or_default())
}

thread_local! {
    /// **最近一笔命令的回执**（`reqed::CommandReceipt`），等前端来取。
    ///
    /// 为什么是"等前端来取"而不是 emit：`ipc::traced` 手上没有 `AppHandle`，
    /// 而前端本来就在轮询 `wb_txn_state`（800ms）—— 顺手捎回去即可，不必为这一条
    /// 再加一条事件通道。
    static RECEIPT: std::cell::RefCell<Option<reqed::CommandReceipt>> =
        const { std::cell::RefCell::new(None) };
}

/// `traced` 每跑完一条命令就写一次（成功与失败都写）。
pub fn note_command(req: &str, command: &'static str, ok: bool) {
    RECEIPT.with(|c| {
        *c.borrow_mut() = Some(reqed::CommandReceipt {
            req: req.to_owned(),
            command,
            ok,
        })
    });
}

/// 前端在 `wb_txn_state` 里取走最近一笔回执（取走即清，前端不会重复处理同一笔）。
pub fn take_receipt() -> Option<reqed::CommandReceipt> {
    RECEIPT.with(|c| c.borrow_mut().take())
}

/// 当前这一笔的请求号（不在命令里时是 `None`）。
///
/// 写盘日志每一行都带它 —— 作者 2026-11-02 的原话是要把「前端变化 / 后端变化 /
/// 不是我点的非直接变化」串起来看，而一次操作会打出七八行，**没有共同的号就串不起来**。
pub fn current_req() -> Option<String> {
    CURRENT_REQ.with(|c| c.borrow().clone())
}

/// 当前命令如果是一条编辑，返回它的人类标签
fn current_edit_label() -> Option<&'static str> {
    CURRENT_COMMAND.with(|c| c.get()).and_then(|name| {
        if !TXN_EDIT_COMMANDS.contains(&name) {
            /* 不再进 `txn` 的命令：不拍整树快照、不记文件事务 */
            return None;
        }
        COMMAND_LABELS
            .iter()
            .find(|(k, _)| *k == name)
            .map(|(_, label)| *label)
    })
}

/// 当前 traced 命令的人类标签（`COMMAND_LABELS` 里有就用那条，没有就用命令名）。
fn current_command_label() -> Option<String> {
    let name = CURRENT_COMMAND.with(|c| c.get())?;
    Some(
        COMMAND_LABELS
            .iter()
            .find(|(k, _)| *k == name)
            .map(|(_, l)| (*l).to_owned())
            .unwrap_or_else(|| name.to_owned()),
    )
}

/// **结构编辑的统一入口**（2026-11-03 阶段 3）：一条命令 = 若干文件的前后字节。
///
/// # 为什么不再走 `run_ctx` 的整树快照
///
/// `run_ctx` 在命令前后各 `txn::snapshot(shadow_root)` 一次（两百来 KB），
/// 而结构编辑是"一对象一文件"（`machines/A1.toml` / `bundles.toml` …）——
/// 只读那几个文件就够算出这一条的正/反向。改机型显示名那种单击，
/// 成本从「整树 ×2 + 记文件字节」降到「一个小文件 ×2」。
///
/// # `field` 是什么
///
/// 合并键里的"字段"（`display` / `dimensions` / …）。**空串 = 一次性动作**
/// （新建版本、删套餐…），永不与前后合并。
///
/// # 为什么还写盘（不改成"只落内存"）
///
/// 值编辑的热路径是每敲一下（70+ 行 × 连打），那条已经改成只落内存；
/// 结构编辑是单击，一次一个小文件的写盘不是瓶颈，而**它们的逆命令就是文件字节**
/// （一对象一文件，粒度正好）。见 `docs/WORKBENCH-EDITOR-MODEL.md` §5。
pub(super) fn with_struct_ctx<T>(
    field: &str,
    files_of: impl FnOnce(&Ctx) -> Vec<String>,
    f: impl FnOnce(&mut Ctx) -> Result<T, AppError>,
) -> Result<T, AppError> {
    with_ctx_mut(|ctx| {
        let label = current_command_label().unwrap_or_else(|| "编辑".to_owned());
        run_struct(ctx, &label, field, files_of, f)
    })
}

/// [`with_struct_ctx`] 的本体（**抽出来是为了可测**）：测试里没有 `traced`，
/// 标签要显式给。
pub(super) fn run_struct<T>(
    ctx: &mut Ctx,
    label: &str,
    field: &str,
    files_of: impl FnOnce(&Ctx) -> Vec<String>,
    f: impl FnOnce(&mut Ctx) -> Result<T, AppError>,
) -> Result<T, AppError> {
    {
        let label = label.to_owned();
        let files = files_of(ctx);
        let root = ctx.shadow_root.clone();
        let before: Vec<(String, Option<Vec<u8>>)> = files
            .iter()
            .map(|p| (p.clone(), std::fs::read(root.join(p)).ok()))
            .collect();
        let out = f(ctx)?;
        let swaps = before
            .into_iter()
            .map(|(path, b)| edit_history::FileSwap {
                after: std::fs::read(root.join(&path)).ok(),
                path,
                before: b,
            })
            .collect();
        let fields = std::mem::take(&mut ctx.pending_fields);
        ctx.history.record_struct(
            label,
            edit_history::struct_merge_key(files.first().map(|s| s.as_str()).unwrap_or(""), field),
            edit_history::StructCmd { files: swaps },
            fields,
        );
        ctx.touch();
        Ok(out)
    }
}

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
    let ctx = guard.as_mut().expect("刚刚放进去的");
    run_ctx(ctx, current_edit_label(), f)
}

/// `with_ctx_mut` 的本体（**抽出来是为了可测**）：拿一份现成会话跑一次命令，
/// 该记事务的记事务。`edit` 为 `Some(label)` = 这条命令是编辑。
pub(super) fn run_ctx<T>(
    ctx: &mut Ctx,
    edit: Option<&'static str>,
    f: impl FnOnce(&mut Ctx) -> Result<T, AppError>,
) -> Result<T, AppError> {
    /*
     * 盘上戳的两处用途，一次 stat 走完：
     *
     *   · 这一条命令自己写了 `presets/`（before != after）→ 界面手里那份就是新的，
     *     把戳推到这一代（否则下一次 focus 会误报"外部改过"）；
     *   · 此前已经有外部改动待发现（before != 记着的戳）→ **不覆盖**，
     *     否则自己随手改一格就把那条提示吃掉了。
     */
    /* ★ 2026-10-31：编辑 = 一条事务。登记表里的命令才拍快照 —— 读命令零开销。
    拍的是**影子树**：写命令改的就是它（`ctx.presets` 的根）。
    ★ `edit` 是**参数**（调用方给），不许在这里再算一遍 —— 早先写过一版
      在这里 `let edit = current_edit_label();`，把参数遮住了：测试传进来的标签
      被丢掉，于是"编辑没进事务"（真机表现为状态栏永远是 0）。 */
    let shot = match edit {
        Some(_) => Some(txn::snapshot(&ctx.shadow_root)?),
        None => None,
    };

    /* 盘上戳只盯**真仓库**：影子树是我们自己改的，不该触发「外部改过」提示 */
    let before = presets_source_stamp(&ctx.real_root);
    let out = f(ctx);
    let after = presets_source_stamp(&ctx.real_root);
    if before != after && before == ctx.source_stamp {
        ctx.source_stamp = after;
    }

    if let (Some(label), Some(shot_before)) = (edit, shot) {
        // 命令失败也记：它可能已经改了影子树（写了一半、或校验在写之后），
        // 不记的话这部分改动就没有撤销点，用户只能放弃整份草稿。
        let shot_after = txn::snapshot(&ctx.shadow_root)?;
        /* 值编辑带"这一格"的身份证，同一格 5 秒内的连点会并成一条（见 `txn::MERGE_MS`）。
        结构编辑给 `None` —— 建一个版本和删一个版本从来不是"同一件事的延续"。 */
        if let Some(op) = ctx.txn.record_with_fields(
            label,
            ctx.edit_key.take(),
            &shot_before,
            &shot_after,
            &ctx.shadow_root,
            std::mem::take(&mut ctx.pending_fields),
        )? {
            tracing::info!(
                req = %current_req().unwrap_or_default(),
                label = %op.label,
                files = op.files.len(),
                ops = ctx.txn.undo_depth(),
                /* **改了几处**（字段数）—— 与界面上那个 N 同一个数，日志里可直接对账 */
                fields = ctx.txn.dirty_count(),
                "编辑已进事务"
            );
        }
        // 视图与影子树对齐：界面显示的是编辑态
        if ctx.txn.is_dirty() || out.is_ok() {
            ctx.reload_presets()?;
        }
    }
    // 命令跑了就可能动了影子树：缓存的"几份文件有改动"必须重算，不能留上一轮的数
    ctx.refresh_unsaved_files();
    out
}

/// `presets/` 源文件的盘上戳：`相对路径 | 字节数 | mtime(ns)` 的 sha256。
///
/// **只 stat，不读内容**：这条链路每个命令都要看一眼，几十个小文件的修改时间
/// 足够判「变没变」—— 定义文件被改一个字节，mtime 一定动（`atomic_write` 换 inode）。
///
/// 跳过两处：`delivery/`（生成写的产物，不是"源定义"）与 `assets/`（二进制载荷）。
/// 漏掉它们的话，点一次生成就会让界面误报"外部改过定义"。
fn presets_source_stamp(root: &Path) -> String {
    use sha2::{Digest, Sha256};

    fn walk(dir: &Path, base: &Path, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let path = e.path();
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_dir() {
                let name = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default();
                if name == paths::DELIVERY_SUBDIR || name == "assets" {
                    continue;
                }
                walk(&path, base, out);
                continue;
            }
            if path.extension().and_then(|s| s.to_str()) != Some("toml") {
                continue;
            }
            let rel = path
                .strip_prefix(base)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let meta = e.metadata().ok();
            let len = meta.as_ref().map_or(0, |m| m.len());
            /* **只认内容**，不认 mtime（2026-11-01 作者裁定）。
             *
             * 原来这里把 `mtime` 的纳秒也揉进戳里，于是：`git checkout`、
             * 编辑器"另存"、任何一次 `touch` —— 内容一个字节没变，也会被当成
             * "盘上的 presets/ 在外面改过了"，横幅弹出来、点重新加载能消掉、
             * 下次一碰又出现，永远关不掉。
             *
             * 作者的原话是「AI 在外面改了预设，工作台打开后却把它当成异常」——
             * 判断"改没改"该看内容，文件是怎么被写到盘上的不该参与。
             * 文件不大（几十份 TOML），读一遍哈希比一次 mtime 只贵一点。 */
            let content_hash = std::fs::read(&path)
                .map(|b| {
                    use sha2::{Digest, Sha256};
                    format!("{:x}", Sha256::digest(&b))
                })
                .unwrap_or_default();
            out.push(format!("{rel}|{len}|{content_hash}"));
        }
    }

    let mut rows: Vec<String> = Vec::new();
    walk(root, root, &mut rows);
    rows.sort();
    let mut h = Sha256::new();
    h.update(rows.join("\n").as_bytes());
    format!("{:x}", h.finalize())
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
    let Ok(mut guard) = session().lock() else {
        return;
    };
    let Some(ctx) = guard.as_mut() else { return };
    if ctx.stale() && ctx.last_edit.elapsed() >= IDLE_BEFORE_FLUSH {
        ctx.flush();
    }
}

/// 立刻落盘。窗口失焦 / 关闭前调，**不等那 2 秒**
pub(super) fn flush_now() {
    let Ok(mut guard) = session().lock() else {
        return;
    };
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
    Ok((ctx.committed.clone(), ctx.draft.clone(), ctx.notices_now()))
}

fn view_of(ctx: &Ctx, committed: &Committed, draft: &Draft, notices: Vec<String>) -> BookView {
    let probe = build::renderer_for(ctx);
    let mut v = build::book_with_probe(ctx, &ctx.presets, committed, draft, &probe).book_view();
    v.notices = notices;
    v.snapshot = ctx.snapshot();
    /* ★ 2026-11-03（第四次改口径，也是定稿）：未保存改动 = **文件里动了几行**。
    三次口径的来路，别再走回去：

    | 口径 | 作者看到的 | 错在哪 |
    |---|---|---|
    | `unsaved_files()`（文件数） | 永远 1 | 两处改动常在同一个 toml 里 |
    | 值草稿条数 + 事务条数 | 连点涨到 14 | 连点不是 14 处 |
    | 格子数（`层|归属|键`） | 改了很多还是 1 | 一次 `setValue` 连带写 4 行，只记 1 格 |

    现在数**行**（`Txn::dirty_count` → `changed_lines`）。它与 diff 框里
    红绿行的条数同源，也和"`ops.len()` = 撤销栈几格"分得开。
    唯一真值仍然是 `Txn::dirty_count()` —— 页头那个数由前端回填这一个。 */
    v.dirty_count = ctx.txn.dirty_count() + draft.dirty_count();
    v
}

/// 影子树与磁盘是否已经不是同一份（开场判断「上次会话有没有没保存的改动」）
fn shadow_differs(real: &Path, shadow: &Path) -> Result<bool, AppError> {
    /* 只比**源**：`delivery/` 是生成产物，每生成一次就变一次 —— 算进来的话，
    横幅会在每次生成之后复活（作者 2026-11-01 报的就是这个）。 */
    let a = txn::snapshot(real)?.without_products();
    let b = txn::snapshot(shadow)?.without_products();
    Ok(!txn::equal(&a, &b))
}

/// 统一事务的状态（页头三按钮 + 状态栏读它）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TxnState {
    pub dirty: bool,
    pub dirty_count: usize,
    pub can_undo: bool,
    pub can_redo: bool,
    pub ops: Vec<Op>,
    /// **刚跑完那一笔的请求号**（2026-11-02）。`null` = 这一轮没跑过命令。
    ///
    /// 状态栏把号显示出来，作者就能拿它去日志里 `grep` 出这一次点击的全部行。
    /// 取走即清（[`take_receipt`]）—— 同一个号不会在一屏里重复出现。
    pub receipt: Option<reqed::CommandReceipt>,
}

/* ---------- 启动 ---------- */

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Boot {
    pub roots: Roots,
    /// **真正的开场失败**（今天只剩"`presets/` 定位不到"这一种）。`None` = 业务能跑
    pub problem: Option<String>,
    pub detail: Option<String>,
    /// 工作台子目录的职责（14.7）
    pub store_dirs: Vec<StoreDirRole>,
    /// 云端源（Bootstrap）地址：**发布产物发到哪**（`workbench/bootstrap.json`，入库）。
    /// `None` = 还没配 —— 客户端构建时就不会注入默认源（`build.rs` 读同一份文件）。
    /// **它只对应"工作台配置"这一份**：改了要重新构建客户端才生效（编译期注入）
    pub bootstrap_url: Option<String>,
    /// 第二云端源（Gitee 镜像，国内直连）。`None` = 没配 —— 客户端构建不注入那一档，
    /// 客户端界面也不出现它。与 `bootstrap_url` **各读各的**
    pub gitee_bootstrap_url: Option<String>,
}

/// `workbench/bootstrap.json` 的形状（`wb_set_bootstrap` 写、`boot_inner` 读、
/// `src-tauri/build.rs` 读同一份文件的两个键）。**一处定义**，三处引用。
///
/// ★ 双源（2026-10-05，作者拍"国内走 Gitee"）：`bootstrapUrl` = GitHub（主源），
///   `giteeBootstrapUrl` = Gitee 镜像。**各是各的配置**：保存时两个格子一起显式写，
///   不存在"只写主源、另一格靠保留"——单字段时代`wb_set_bootstrap`重写整份文件、
///   把手填的 Gitee 键无声冲掉，正是这条要堵的坑。
#[derive(Debug, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapConfig {
    pub bootstrap_url: String,
    /// `default`：读单键老档（双源之前写的）不炸；`skip_serializing_if`：
    /// 清除后文件里不留 `null` 键 —— `build.rs` 的 pick 对缺键/空档都当"没配"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gitee_bootstrap_url: Option<String>,
}

/// 读工作台配置里的**双云端源**。**读不出 / 坏档 = 全 `None` + 日志一条**：
/// 它是可选配置，挡不住开场（修法：在工作台里再保存一次覆盖它，或手动删掉整个文件）。
/// 与"坏档不静默"不冲突 —— 不静默的那一位是日志
fn read_bootstrap_config() -> (Option<String>, Option<String>) {
    let clean = |url: Option<String>| url.map(|u| u.trim().to_owned()).filter(|u| !u.is_empty());
    let Some(store) = Store::open().ok() else {
        return (None, None);
    };
    match store.read_doc::<BootstrapConfig>(Store::BOOTSTRAP_REL, "云端源配置") {
        Ok(Some(cfg)) => (
            clean(Some(cfg.bootstrap_url)),
            clean(cfg.gitee_bootstrap_url),
        ),
        Ok(None) => (None, None),
        Err(e) => {
            tracing::warn!(error = %e.message, "云端源配置读不出来，按「还没配」处理");
            (None, None)
        }
    }
}

/// 工作台子目录的职责（14.7）。**谁写它、谁读它、能不能当编辑对象**，
/// 一句话说清 —— 界面上要显式摆出来，不留给前端猜
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreDirRole {
    pub name: String,
    pub role: &'static str,
}

/// 子目录职责清单。名字来自 [`paths::WORKBENCH_DIRS`]（同一份清单，不抄第二遍），
/// 职责的措辞跟着 store.rs 的文档走：bbs 零读写、.snapshots 只写不读、
/// .draft 只存「改了什么」、.trash 是回收站、machines 是机型配方的真源读写。
fn store_dir_roles() -> Vec<StoreDirRole> {
    let role_of = |name: &str| match name {
        "machines" => "机型配方的真源（machineVariants 按机型一份）。机型页读写它",
        "bbs" => "零读写 —— BBS 预设今天不经工作台，占位",
        ".draft" => "草稿与界面状态（book.json / ui.json）。只存「改了什么」，懒写",
        ".trash" => "回收站：删除版本的生成快照、交付残留回收，都在这里",
        ".snapshots" => "生成快照：生成时写入，恢复配方时只读 —— 不当编辑对象",
        _ => "",
    };
    paths::WORKBENCH_DIRS
        .iter()
        .map(|name| StoreDirRole {
            name: (*name).to_owned(),
            role: role_of(name),
        })
        .collect()
}

/// 三个数据根。界面开场调它。**唯一的数据根是 `presets/`**
#[tauri::command(async)]
pub fn wb_boot() -> Result<Boot, AppError> {
    traced("wb_boot", |_| boot_inner())
}

/// 重读我们自己那份预设数据。改完 `presets/` 之后不用重启工作台
#[tauri::command]
pub fn wb_reload(discard_drafts: Option<bool>) -> Result<Boot, AppError> {
    /* **有未保存草稿时必须确认**（2026-11-01 作者定）。
     *
     * 重新加载 = 放弃编辑态、回到磁盘那一版。这句语义没错，但它会**丢用户的东西**：
     * 以前一条命令就执行完，谁点一下草稿就没了。现在先看影子与真源是否真的分叉：
     *
     *   · 没分叉（没有草稿）→ 直接重新加载，不必打扰人；
     *   · 分叉了（真有未保存编辑）→ 没拿到 `discardDrafts=true` 就**什么都不做**，
     *     返回一个明确的拒绝，界面据此弹"重新加载会丢弃当前未保存草稿"。
     *
     * 与"AI 在外面改了 presets/"是两件事：那种情况影子与真源也"不同"，
     * 但那是**磁盘更新**，不是用户的草稿 —— 见下面 `drafts_at_risk` 的判定。 */
    let mut drafts = drafts_at_risk();
    /* ★ **内存里的值编辑也算"草稿"**（2026-11-03）：值编辑根本不写影子树，
    `drafts_at_risk` 的三方对账看不出分叉 —— 不补这一条，「重新加载」会
    **静默丢掉**所有未保存的参数改动。 */
    if let Some(extra) = crate::workbench::app::history_at_risk() {
        drafts.edited += extra;
        drafts.doomed = true;
    }
    if drafts.doomed && discard_drafts != Some(true) {
        /* 冲突与普通草稿分开说：冲突是"同一个文件两边都改过"，
        更需要人看清楚再决定（2026-11-01）。 */
        let key = if drafts.conflicts > 0 {
            "ui.workbench.reload.conflict"
        } else {
            "ui.workbench.reload.drafts"
        };
        return Err(AppError::invalid_argument(
            postprocess::diag::messages::text(
                key,
                &[
                    ("count", &drafts.edited.to_string()),
                    ("conflicts", &drafts.conflicts.to_string()),
                ],
            ),
        ));
    }

    drop_ctx();
    if let (Ok(store), Some(real)) = (Store::open(), crate::workbench::paths::presets_root()) {
        let shadow = txn::shadow_root(&store.root);
        std::fs::create_dir_all(&shadow)
            .map_err(|e| AppError::io("建不出影子树").with_detail(e.to_string()))?;
        txn::mirror(&real, &shadow)?;
    }
    boot_inner()
}

/// 内存里的值编辑有几处（`None` = 没有会话）—— 给 `wb_reload` 判"要不要确认"。
pub(super) fn history_at_risk() -> Option<usize> {
    with_ctx(|ctx| {
        let cmds = ctx.history.undo_depth();
        Ok(if cmds == 0 && ctx.draft.is_clean() {
            0
        } else {
            cmds.max(1)
        })
    })
    .ok()
}

/// 重新加载会不会丢掉用户的东西。
///
/// 这里要分清两种"影子与真源不同"（2026-11-01）：
///
/// | 情形 | 影子相对真源 | 是用户的草稿吗 |
/// |---|---|---|
/// | AI / 编辑器 / git 改了 `presets/` | 影子**落后**（内容是真源的旧版） | **不是** —— 那是磁盘更新 |
/// | 用户在工作台里改了、没保存 | 影子**多出**真源没有的内容 | **是** |
///
/// 只有后者才需要确认。把前者也拦下来，就变成"外部改一次源，重新加载就问一遍"，
/// 正是作者讨厌的那种打扰。
#[derive(Debug, Default)]
struct DraftsAtRisk {
    /// 影子里有、真源里没有（或内容不同）的文件数 —— 用户的未保存编辑
    edited: usize,
    /// 其中"同一个文件两边都改过"的处数（冲突，不能自动选边）
    conflicts: usize,
    /// 丢不丢东西
    doomed: bool,
}

fn drafts_at_risk() -> DraftsAtRisk {
    let (Ok(store), Some(real)) = (Store::open(), crate::workbench::paths::presets_root()) else {
        return DraftsAtRisk::default();
    };
    let shadow = txn::shadow_root(&store.root);
    let Ok(shadow_snap) = txn::snapshot(&shadow) else {
        return DraftsAtRisk::default();
    };
    let Ok(real_snap) = txn::snapshot(&real) else {
        return DraftsAtRisk::default();
    };
    /* **三方对账**：只有"影子相对基准改过、真源没改"才是用户的草稿。
    用 `edited_against`（两两比内容）会把"外部改了真源、影子是旧拷贝"也数进来，
    于是 reload 每次都问一遍 —— 正是作者讨厌的那种打扰。 */
    let base = txn::base_root(&store.root);
    let base_snap = txn::snapshot(&base).unwrap_or_else(|_| real_snap.clone());
    let verdict = shadow_snap.reconcile(&real_snap, &base_snap);
    let edited = verdict.drafts.len() + verdict.conflicts.len();
    DraftsAtRisk {
        edited,
        conflicts: verdict.conflicts.len(),
        doomed: verdict.needs_attention(),
    }
}

/// 盘上的 `presets/` 与这个会话手里的那一份**不一样**吗（外部改过 / 别处写过）。
///
/// 只回答"要不要重新加载"，**不自己重载** —— 会话里可能正开着一个草稿，
/// 悄悄换掉定义会把那份草稿按旧定义存回去（`sectionId` / 弃用这些格尤其危险）。
/// 界面拿它画一条提示 + 「重新加载」那颗按钮，动作仍然是人的。
#[tauri::command]
pub fn wb_presets_changed() -> Result<bool, AppError> {
    traced("wb_presets_changed", |_| {
        // 盯**真仓库**：影子树是我们自己的编辑态，不算"外部改过"
        with_ctx(|ctx| Ok(presets_source_stamp(&ctx.real_root) != ctx.source_stamp))
    })
}

/// **当前安装的 SupportEase 版本号**（构建期 `CARGO_PKG_VERSION`，全仓唯一真值）。
///
/// 「生成与发布」页那块**只读的「软件版本」**展示位用它 —— 与客户端 `get_app_version`
/// **同一个常量**（[`crate::runtime::structure::APP_VERSION`]），不是第二处真值。
///
/// ★ 它只报"当前装的是哪一版"，**不发版本、不碰发布事务** —— 「发布软件版本」是另一套事务
/// （见 `docs/RELEASE-TRANSACTIONS.md`），本轮不实现。
#[tauri::command(async)]
pub fn wb_app_version() -> Result<String, AppError> {
    traced("wb_app_version", |_| {
        Ok(crate::runtime::structure::APP_VERSION.to_owned())
    })
}

/// 记下云端源（Bootstrap）**双源**地址 —— **工作台里唯一一处"发布到哪"**（写入库的
/// `workbench/bootstrap.json`）。GitHub 是主源；`gitee_url` 传 `None` / 空白 = 清除
/// Gitee 镜像档（界面那格空着就是清除，如实写盘，不留 null 键）。
///
/// **不进制 draft 体系**：它不是配方内容（没有撤销 / 差异 / 快照可言），是一次单值
/// 配置写；界面上是「设置」页的两格，改完当场回显。
/// 校验与规范化在 [`delivery::normalize_bootstrap_url`]（**仓库地址 → 默认发布入口的 raw**；
/// blob 页按人指的转；raw / 自建源原样；Gitee 是同一座桥 —— 见那个函数的输入契约表）。
/// **两个格子一笔写全**：调用方（设置页）把两格的现值都传进来 —— 这一笔就是配置的
/// 全部真值，没有"只动一格、另一格靠后端帮忙保留"的暗规则。
/// **对客户端生效要重新构建**（`build.rs` 构建期读同一份文件注入）——返回值只是回显。
#[tauri::command]
pub fn wb_set_bootstrap(
    url: String,
    gitee_url: Option<String>,
) -> Result<BootstrapConfig, AppError> {
    traced("wb_set_bootstrap", |_| {
        let store = Store::open()?;
        write_bootstrap_config(&store, &url, gitee_url.as_deref())
    })
}

/// 写盘的那一半（从命令里拆出来给测试用：命令走 `Store::open()`，测试用 `Store::at`）。
/// 规范化与"空 = 清除"的语义见 [`wb_set_bootstrap`]；返回写下去的形状（回显用）。
fn write_bootstrap_config(
    store: &Store,
    url: &str,
    gitee_url: Option<&str>,
) -> Result<BootstrapConfig, AppError> {
    let bootstrap_url = delivery::normalize_bootstrap_url(url)?;
    let gitee_bootstrap_url = gitee_url
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(delivery::normalize_bootstrap_url)
        .transpose()?;
    let cfg = BootstrapConfig {
        bootstrap_url,
        gitee_bootstrap_url,
    };
    store.write_doc(Store::BOOTSTRAP_REL, &cfg)?;
    Ok(cfg)
}

/// 开场那一段。先算出三个数据根，再开一次会话（`presets/` 定位不到才会是 problem）
fn boot_inner() -> Result<Boot, AppError> {
    match (|| -> Result<Boot, AppError> {
        let roots = Roots {
            workbench: paths::workbench_root()?.display().to_string(),
            presets: paths::presets_root()
                .ok_or_else(|| AppError::not_found("定位不到 <repo>/presets"))?
                .display()
                .to_string(),
            delivery: paths::delivery_root()?.display().to_string(),
        };
        // 建一次会话：能开就说明 presets/ 读得通（会话自己也读一次）
        with_ctx(|_| Ok(()))?;
        let (bootstrap_url, gitee_bootstrap_url) = read_bootstrap_config();
        Ok(Boot {
            roots,
            problem: None,
            detail: None,
            store_dirs: store_dir_roles(),
            bootstrap_url,
            gitee_bootstrap_url,
        })
    })() {
        Ok(boot) => Ok(boot),
        Err(e) => {
            /* 云端源那格与 presets/ 无关：就算预设源定位不到，配置也照读出来显示 */
            let (bootstrap_url, gitee_bootstrap_url) = read_bootstrap_config();
            Ok(Boot {
                roots: Roots {
                    workbench: paths::workbench_root()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default(),
                    presets: paths::presets_root()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default(),
                    delivery: paths::delivery_root()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default(),
                },
                problem: Some(e.message),
                detail: e.detail,
                store_dirs: store_dir_roles(),
                bootstrap_url,
                gitee_bootstrap_url,
            })
        }
    }
}

/* ---------- 读 ---------- */

#[tauri::command(async)]
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
    /// 布局里的**全部分组**（含暂时空掉的、含 `component` 整块组件卡），按页签顺序。
    ///
    /// 两处读者口味不同，所以由前端自己筛，后端别替它决定：
    ///   · 「所属分组」下拉 —— 只留 `component == null` 的（后端 `move_param` 拒收组件卡）；
    ///   · 分组抽屉 —— 全都要（点得开组件卡的卡头，也看得见它们在哪儿）。
    pub sections: Vec<SectionTarget>,
}

/// 一个分组（「所属分组」下拉与分组抽屉共用的一行）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionTarget {
    pub id: String,
    pub label: String,
    pub tab_id: String,
    pub tab_label: String,
    /// 整块组件卡（`layout_schema` 的 `component`）。`None` = 普通参数分组。
    /// 组件卡里有参数（`tower-map` 那种），所以它并不是"空组"，只是不许当搬家的目标
    pub component: Option<String>,
    /// **整块卡**（`layout = 'component'`）：`settings` 下那 11 张自己画自己的卡。
    /// 客户端那边是固定长好的，工作台既不列它、也不让人把参数搬进去
    pub standalone: bool,
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
    /// 四册的另外三册（2026-10-31）：工作台定义框要能看见/编辑它们，
    /// 也是「预览语言」切换的数据来源。空 = 这一册没译，界面回落中文
    pub label_en: String,
    pub label_ja: String,
    pub label_tw: String,
    pub desc: String,
    pub desc_en: String,
    pub desc_ja: String,
    pub desc_tw: String,
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
    /// **全局弃用**（所有机型）
    pub deprecated: bool,
    /// **只在这些机型上弃用**（与 `deprecated` 叠加）
    pub deprecated_machines: Vec<String>,
    /// **哪几册译文跟着旧中文一起过期了**（2026-11-02，`labelEn` 这种键名）。
    /// 空 = 没问题。客户端不读；工作台在定义框里标出来、`stale-translations` 脚本按它出清单
    pub stale_translations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChoiceView {
    pub label: String,
    /// 选项名的另外三册（2026-10-31）
    pub label_en: String,
    pub label_ja: String,
    pub label_tw: String,
    pub value: Value,
    /// **显式全局弃用** —— 定义框那颗勾的状态，回写只认这一格。
    /// 之前这里是「显式 ∪ 推定」的并集，打开框再保存会把推定值固化成 `deprecated = true`
    pub deprecated: bool,
    /// **点名弃用**的机型（回写用这一格）
    pub deprecated_machines: Vec<String>,
    /// 推出来的弃用（选这一档放开的子参数全弃用），**只显示、不回写**
    pub derived: bool,
}

#[tauri::command(async)]
pub fn wb_registry() -> Result<RegistryView, AppError> {
    traced("wb_registry", |_| {
        with_ctx(|ctx| Ok(registry_view(&ctx.presets.registry)))
    })
}

/// 注册表视图。`wb_registry` 与 `wb_set_param_meta` 共用一份拼法
fn registry_view(reg: &crate::workbench::presets::ParamRegistry) -> RegistryView {
    RegistryView {
        updated: reg.updated().to_owned(),
        tabs: reg.param_tabs(),
        params: reg.params().iter().map(|p| param_view(reg, p)).collect(),
        sections: {
            /* `layout = 'component'` 的那些卡（设置页 11 张）不归这里管 —— 见 `standalone` */
            let mut whole: std::collections::HashSet<String> = std::collections::HashSet::new();
            for t in reg.layout() {
                for s in &t.sections {
                    if s.layout.as_deref() == Some("component") {
                        whole.insert(s.id.clone());
                    }
                }
            }
            reg.layout_sections_all()
                .into_iter()
                .map(|(tab_id, tab_label, id, label, component)| SectionTarget {
                    standalone: whole.contains(&id),
                    id,
                    label,
                    tab_id,
                    tab_label,
                    component,
                })
                .collect()
        },
    }
}

/// **改一条参数的定义**（名称 / 说明 / 单位 / 值类型 / 控件 / 范围 / 步进 /
/// 出厂默认 / 属于 / 前置条件 / 弃用）。
///
/// # 即时落盘；撤销走外壳那条栈的「定义」条目
///
/// 定义与值不是一种改动，但撤销只有状态栏那一个入口：值的条目带反向 patch，
/// 定义的条目**自带改前/改后两份整包载荷** —— 撤销就是把改前那份再交回
/// [`crate::presetdata::registry::ParamRegistry::set_param_meta`]，重做就是改后
/// 那份。条目的 label 写明是「定义 · 某参数」，与值那条「机型 · 参数」在栈里
/// 各说各的，不存在「撤销一步说不清」的问题。
///
/// # 写完整份重读
///
/// 定义一变，注册表指纹就变了（它进「待生成」的判定），而指纹缓存是
/// **构造完不再变**的 —— 换掉整份 [`Presets`] 是唯一让指纹跟上改动的方式，
/// 顺带保证内存与盘重新对表。返回重读后的注册表视图。
#[tauri::command(async)]
pub fn wb_set_param_meta(key: String, edit: ParamMetaEdit) -> Result<RegistryView, AppError> {
    traced("wb_set_param_meta", |_| {
        with_struct_ctx(
            "meta",
            |_| {
                vec![
                    "registry/param_registry.toml".to_owned(),
                    "layout_schema.toml".to_owned(),
                ]
            },
            |ctx| {
                /* ★ **改之前那份要先抄下来**（2026-11-03）：`set_param_meta` 是就地改，
                改完就再也拿不到 `before` 了。而「改了什么」框里要显示
                `label: 涂胶速度 → 涂胶速度限制` 这种**真实前后值**，
                不许事后从日志摘要里拼。 */
                let before = ctx.presets.registry.param(&key).cloned();
                ctx.presets.registry.set_param_meta(&key, edit)?;
                ctx.presets.registry.write_back()?;
                ctx.reload_presets()?;
                /* 结构编辑同样要进字段账 —— 否则改说明 / 改默认值时「改了什么」还是空的 */
                if let Some(old) = before {
                    if let Some(new) = ctx.presets.registry.param(&key) {
                        ctx.pending_fields =
                            structural_fields(&key, new.label.clone(), new.summary(&old));
                    }
                }
                Ok(registry_view(&ctx.presets.registry))
            },
        )
    })
}

/// **清掉译文台账里的几格**（2026-11-02，逃生通道）。
///
/// # 为什么要开这条口子
///
/// 台账（`staleTranslations`）在 [`wb_set_param_meta`] 里**只在值真的变了**时摘 ——
/// 那是撤销 / 重做幂等的前提，不能动。可译文的另一种改法不经过那条路：直接改文件、
/// 或按 `scripts/stale-translations.mjs` 的清单一次改一批（AI 干的正是这件事）。
/// 那种"改"在后端看来等于没改，标记永远摘不掉。
///
/// 所以这里要调用方**点名**：清 `labelEn` 还是 `descJa`。不在这里替人判断
/// "哪几格已经译好了" —— 前端没有"旧中文"，判不出译文是不是跟着那一版写的。
/// 判据放宽会导致撤销重放同一笔也清账，那才是真错。
///
/// # 只动台账，不动译文
///
/// 译文由别的路写好，这条命令**不校验、不改写**它们；它只把那一串键名从台账里摘掉。
/// 名字写错一个整笔拒收（`ui.presetdata.registry.356`），不做"能清几个算几个"。
#[tauri::command(async)]
pub fn wb_clear_stale_translations(
    key: String,
    keys: Vec<String>,
) -> Result<RegistryView, AppError> {
    traced("wb_clear_stale_translations", |_| {
        with_ctx_mut(|ctx| {
            ctx.presets.registry.clear_stale_translations(&key, &keys)?;
            ctx.presets.registry.write_back()?;
            ctx.reload_presets()?;
            Ok(registry_view(&ctx.presets.registry))
        })
    })
}

/// **改一个分组的名字 / 位置**（参数台卡片头那颗可点的标题与上下箭头，2026-11-01）。
///
/// 和 [`wb_set_param_meta`] 同一套：就地写 `param_registry.toml`（位置那一笔还会
/// 同步 `layout_schema.toml` 的数组次序），写完整份重读 —— 指纹缓存是构造完就
/// 不再变的，换掉整份 [`Presets`] 是唯一让"待生成"跟上改动的方式。
#[tauri::command(async)]
pub fn wb_set_section_meta(
    section_id: String,
    edit: SectionMetaEdit,
) -> Result<RegistryView, AppError> {
    traced("wb_set_section_meta", |_| {
        with_struct_ctx(
            "section",
            |_| {
                vec![
                    "registry/param_registry.toml".to_owned(),
                    "layout_schema.toml".to_owned(),
                ]
            },
            |ctx| {
                ctx.presets.registry.set_section_meta(&section_id, edit)?;
                ctx.presets.registry.write_back()?;
                ctx.reload_presets()?;
                Ok(registry_view(&ctx.presets.registry))
            },
        )
    })
}

/// **删掉一个空分组**（抽屉里那颗「删除分组」，2026-11-01）。
///
/// 与 [`wb_set_section_meta`] 同一条路：就地写两份 TOML（注册表 + 布局），写完整份
/// 重读。删除也是编辑事务里的一笔 —— **删完只是进草稿**，右上角「保存」才真落盘，
/// 「放弃」就退回来（作者 2026-11-01：「删除的操作也是一样的，需要点保存或者放弃」）。
///
/// 只允许删「没有参数归它、不是组件卡、不带 showWhen」的组，判据在
/// [`ParamRegistry::remove_section`]，这里不再另立一套。
#[tauri::command(async)]
pub fn wb_remove_section(section_id: String) -> Result<RegistryView, AppError> {
    traced("wb_remove_section", |_| {
        with_struct_ctx(
            "",
            |_| {
                vec![
                    "registry/param_registry.toml".to_owned(),
                    "layout_schema.toml".to_owned(),
                ]
            },
            |ctx| {
                ctx.presets.registry.remove_section(&section_id)?;
                ctx.presets.registry.write_back()?;
                ctx.reload_presets()?;
                Ok(registry_view(&ctx.presets.registry))
            },
        )
    })
}

fn param_view(reg: &crate::workbench::presets::ParamRegistry, p: &ParamDef) -> ParamView {
    // 选项级弃用（C14 §五 / §4-2）：上游标的 + 推出来的（这一档放开的参数全弃用）取并集。
    // **并集只用来标"推定"**：显式那两格（全局 / 点名机型）单独下发，回写只认它们，
    // 否则「打开定义框 → 保存」会把推定值固化成一条手写的 `deprecated = true`
    let dead = reg.choice_deprecated(&p.key);
    ParamView {
        key: p.key.clone(),
        label: p.label.clone(),
        label_en: p.label_en.clone(),
        label_ja: p.label_ja.clone(),
        label_tw: p.label_tw.clone(),
        desc: p.desc.clone(),
        desc_en: p.desc_en.clone(),
        desc_ja: p.desc_ja.clone(),
        desc_tw: p.desc_tw.clone(),
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
            .zip(dead)
            .map(|(c, dead)| ChoiceView {
                label: c.label.clone(),
                label_en: c.label_en.clone(),
                label_ja: c.label_ja.clone(),
                label_tw: c.label_tw.clone(),
                value: c.value.clone(),
                derived: dead && !c.deprecated && c.deprecated_machines.is_empty(),
                deprecated: c.deprecated,
                deprecated_machines: c.deprecated_machines.clone(),
            })
            .collect(),
        show_when: p.show_when.clone(),
        parent_key: p.parent_key.clone(),
        machine_filter: p.machine_filter.clone(),
        deprecated: p.deprecated,
        deprecated_machines: p.deprecated_machines.clone(),
        stale_translations: p.stale_translations.clone(),
    }
}

/// 一屏矩阵。列由前端勾选给出，**顺序由后端按配方本重排**；
/// `baseMachineId`（对照模式的基准机型）驱动差异判据与行序 —— 见 [`derive::Matrix`]
#[tauri::command(async)]
pub fn wb_matrix(
    cols: Vec<ColRef>,
    tab: Option<String>,
    query: Option<String>,
    base_machine_id: Option<String>,
) -> Result<Matrix, AppError> {
    traced("wb_matrix", |_| {
        with_ctx(|ctx| {
            /* ★ `book_of` 带 `with_rest` —— 参数台那一屏的格子就是这条命令吐出去的。
            漏掉它 `rest_at` 与 `hit.value` 同源，`dirty` 恒假、徽章永不亮。 */
            Ok(book_of(ctx).matrix(
                &cols,
                tab.as_deref(),
                query.as_deref().unwrap_or_default(),
                base_machine_id.as_deref(),
            ))
        })
    })
}

/// **给界面用的 `Book`：永远带「盘上已保存那一份」**（2026-11-02 定死）。
///
/// # 为什么要有这个函数
///
/// 「徽章一闪就灭」连着犯了三次，每次都是同一件事：某个命令自己 `Book::new(...)`，
/// **漏了 `.with_rest(&ctx.base)`** ⇒ `rest` 退回编辑树 ⇒ 刚写进去的值自己跟自己比
/// ⇒ `dirty` 恒假 ⇒ 徽章亮一帧就灭。
///
/// 已经漏过两处：`wb_matrix`（刀 1 时逮到）、`wb_apply_draft` 的 refresh（2026-11-02
/// 作者报「这次时间更短」）。**靠 grep 逮漏是不可靠的** —— 所以把这件事收成一处：
/// 生产路径要 `Book` 就调它，别再手写 `Book::new`。
///
/// 不需要「改之前的值」的命令（`diff_draft` / 生成预演）才配自己 new。
fn book_of(ctx: &Ctx) -> Book<'_> {
    Book::new(&ctx.presets, &ctx.committed, &ctx.draft).with_rest(&ctx.base)
}

/// **参数台 v2 的整屏数据**（2026-11-02）。
///
/// 与 [`wb_matrix`] **并行存在**，不是它的替代：旧命令一个字节不改，
/// 旧页照旧用它；新页只走这一条。
///
/// 区别只有一条但很关键：**返回值里没有「当前值」**。
/// 前端只能算 `draft ?? saved`，所以「值 / 保存值 / 脏」不可能再折叠成一个字段
/// （旧页的病根）。规则见 `docs/WORKBENCH-SHEET-V2-PROTOCOL.md`。
#[tauri::command(async)]
pub fn wb_param_sheet(
    cols: Vec<ColRef>,
    tab: Option<String>,
    query: Option<String>,
    base_machine_id: Option<String>,
) -> Result<sheet::ParamSheet, AppError> {
    traced("wb_param_sheet", |_| {
        with_ctx(|ctx| sheet::sheet_in(ctx, cols, tab, query, base_machine_id))
    })
}

/// 配方台一屏（默认视角）：一个版本的分组列表。
///
/// 矩阵是它的对比工具，不是默认 —— 一屏几十列的表格不好看也不好改
#[tauri::command(async)]
pub fn wb_desk(
    machine_id: String,
    uid: Option<String>,
    tab: Option<String>,
    query: Option<String>,
) -> Result<Desk, AppError> {
    traced("wb_desk", |_| {
        with_ctx(|ctx| {
            /* 与 `wb_matrix` 同一条理由：这一屏的格子也要带「改之前的值」 */
            Ok(book_of(ctx).desk(
                &machine_id,
                uid.as_deref(),
                tab.as_deref(),
                query.as_deref().unwrap_or_default(),
            ))
        })
    })
}

#[tauri::command(async)]
pub fn wb_trash() -> Result<Vec<TrashEntry>, AppError> {
    traced("wb_trash", |_| with_ctx(|ctx| ctx.store.trash_entries()))
}

#[tauri::command(async)]
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

#[tauri::command(async)]
pub fn wb_preview_bulk(
    key: String,
    value: Value,
    cols: Vec<ColRef>,
) -> Result<BulkPreview, AppError> {
    traced("wb_preview_bulk", |_| {
        with_ctx(|ctx| {
            let (c, d, _) = state(ctx)?;
            Ok(Book::new(&ctx.presets, &c, &d).preview_bulk(&key, &value, &cols))
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
    /// **这一笔落在哪一份文件**（相对预设根）。「改了什么」按它过滤 ——
    /// 没有它就只能拿文件名去猜归属，而值改动全在 `param_registry.toml` 里猜不出来
    pub file: String,
    /// **来源标签**（2026-11-03）：`user` / `promoted` / `copy` / `empty` /
    /// `unknown` / `structural`。界面照着显示「用户修改 / 归并连带 / …」。
    ///
    /// 作者定稿要「平铺 + 标签」：改一次 Z 连带写三个版本键，四条都列出来，
    /// 只有一条是 `user`，其余是 `promoted` —— 一眼看出哪条是自己点的。
    pub why: String,
}

#[tauri::command(async)]
pub fn wb_diff_draft() -> Result<Vec<DiffLine>, AppError> {
    traced("wb_diff_draft", |_| with_ctx(|ctx| Ok(diff_draft(ctx))))
}

/// **值改动落在哪一份文件**（2026-11-02）。
///
/// 三层取值全在 `presets/registry/param_registry.toml`：机型基底在
/// `[params.machineVariants]` 的 `A1` 那几条、版本覆盖在同表的 `A1:STANDARD`。
/// 所以值改动一律记这一份 —— 它不是"猜"出来的，是这个数据结构的**固定事实**。
///
/// 「改了什么」那一栏要按文件过滤，没有这个字段时只能拿文件名去猜归属
/// （`machines/A1.toml` → `A1`），而 `param_registry` 猜不出任何 owner ⇒ 永远显示 0 格。
const VALUE_FILE: &str = "registry/param_registry.toml";

/// 「A1 · 标准版」/「A1 · 机型基底」—— 撤销栈上那句人话的**目标**部分。
///
/// 与 `diff_draft` 里的 `target_of` 同一条口径（同一份 catalog、同一处措辞函数），
/// 别一边写 `A1/STANDARD` 一边写「A1 · 标准版」。
/// 造一条字段账（结构编辑用：`level` 为空、`why = "structural"`）。
///
/// `before` / `after` 已经是**给人看的字符串**（`None` = 原本没有这一行）——
/// 调用方从数据层读出原值，这里不猜。
pub(super) fn one_field(
    owner: String,
    key: &str,
    label: String,
    before: Option<String>,
    after: Option<String>,
) -> txn::FieldEntry {
    txn::FieldEntry {
        level: None,
        owner,
        key: key.to_owned(),
        label,
        before,
        after,
        why: "structural",
    }
}

/// 把结构编辑的 [`logfmt::FieldChange`] 转成事务的字段账。
///
/// 结构编辑没有"格"：改的是**定义本身**（说明 / 单位 / 出厂默认 / 床身尺寸）。
/// 所以 `level` 填 `None`（不属于机型层也不属于版本层），`owner` 是对象 id
/// （机型 id / 版本 uid / 参数 key），`key` 是字段路径（`label` / `bedSize.depth`）。
///
/// `why` 固定 `"structural"` —— 前端把它显示成「改定义」，与值改动的
/// 「用户修改 / 归并连带」区分开。
fn structural_fields(
    owner: &str,
    label: String,
    changes: Vec<crate::logfmt::FieldChange>,
) -> Vec<txn::FieldEntry> {
    changes
        .into_iter()
        .map(|c| txn::FieldEntry {
            level: None,
            owner: owner.to_owned(),
            key: c.field.clone(),
            label: label.clone(),
            before: Some(crate::logfmt::show(&c.before)),
            after: Some(crate::logfmt::show(&c.after)),
            why: "structural",
        })
        .collect()
}

/// 把写盘那一刻的 [`logging::KeyChange`] 转成事务的字段账（[`txn::FieldEntry`]）。
///
/// # 为什么要转，而不是直接存 `KeyChange`
///
/// `KeyChange` 是**日志**的形状（`owner` 是 `machineVariants` 里的写法，
/// 没有 `Level`）；界面要的是"哪一层的哪一格、给人看的名字叫什么"。
/// 而且事务住在 `domain`、日志住在 `app`，让 `domain` 依赖 `app` 是反的。
///
/// # `owner` / `level` 怎么定
///
/// `KeyChange.owner` 长这样：
///
/// ```text
/// A1             ← 机型基底（Level::Machine）
/// A1:STANDARD    ← 版本（Level::Version），uid 形状是 `机型/版本id`
/// ```
///
/// 写成 `机型:版本id` 是 `machineVariants` 的叫法。这里按有没有 `:` 分开填
/// `level` —— 与 `Patch::SetValue` 那一侧的口径一致，界面才好按层分组。
///
/// `why` 直接取 `Why::tag()`（`user` / `promoted` / `copy` / `empty` / `unknown`），
/// 前端据此显示「用户修改 / 归并连带 / …」。
fn value_field_entries(ctx: &Ctx, changes: &[logging::KeyChange]) -> Vec<txn::FieldEntry> {
    let mut out: Vec<txn::FieldEntry> = changes
        .iter()
        .map(|c| {
            let is_version = c.owner.contains(':');
            txn::FieldEntry {
                level: Some(if is_version {
                    format!("{:?}", Level::Version)
                } else {
                    format!("{:?}", Level::Machine)
                }),
                owner: c.owner.clone(),
                key: c.key.clone(),
                label: ctx
                    .presets
                    .registry
                    .param(&c.key)
                    .map(|p| p.label.clone())
                    .unwrap_or_else(|| c.key.clone()),
                /* ★ 2026-11-03 修：`None` **不在这里格式化**。
                以前写 `Some(show(&c.before))`，而 `show(None)` 是
                「（跟随上一层）」—— 那是**日志写法**，被当成值搬进了界面，
                于是 `A1:FAST` 那一行显示"3.85 → （跟随上一层）"。

                现在 `None` 保持 `None`，交给 `fields_after_model` 按继承关系
                解析成**具体数字**（见 `effective_text`）。日志那一行仍然用
                `logfmt::show`，两个受众、两种写法，各按各的需要。 */
                before: c
                    .before
                    .as_ref()
                    .map(|v| crate::logfmt::show(&Some(v.clone()))),
                after: c
                    .after
                    .as_ref()
                    .map(|v| crate::logfmt::show(&Some(v.clone()))),
                why: c.why.tag(),
            }
        })
        .collect();
    out = fields_after_model(ctx, &out);
    out
}

/// **把"写进文件的动作"翻译成"模型上实际变了哪些格"**（2026-11-03 重做）。
///
/// # 这一层要解决的两个毛病
///
/// 作者两次报的其实是同一件事：**标签跟着存储走，而不是跟着模型走**。
///
/// 1. `A1:FAST` 那一行显示 `（跟随上一层）` —— 那是 `logfmt::show(None)` 的
///    **日志写法**，被原样搬进界面。可界面上问的是"这一格的值从多少变成多少"，
///    一个删键动作 `3.85 → 删掉` 在模型里的意思是 `3.85 → 3.95`（跟着基底）。
/// 2. 同一屏里 `随基底生效` 与 `归并连带` 混着出现，而且 A1 是一种、A1_MINI 是另一种
///    —— 明明是同一种行为（改基底，版本跟着变），只因为 A1_MINI 文件里
///    还留着版本键就走了另一条分支。
///
/// # 现在只有三种来源，按**模型行为**分
///
/// | `why` | 意思 | 界面写 |
/// |---|---|---|
/// | `user` | 用户手改的那一格 | 用户修改 |
/// | `follows` | 跟着基底变（不管文件里写没写版本键） | 随基底生效 |
/// | `structural` | 改定义（说明 / 默认值 / 尺寸…） | 改定义 |
///
/// **`promoted` 从此不在界面上出现**：它是"我们为了保住这个值，额外写了几个键"
/// 的存储解释，用户不关心，也不该关心。日志里仍然保留（`why` 照原样进 `KeyChange`），
/// 排查时能看出这一次到底写了几个键。
///
/// # 判据：这个版本有没有自己的值
///
/// 基底改了值 → 遍历这台机型的每个版本 → 它**自己有独立值**（`machine_variants`
/// 里有 `机型:版本` 且不等于旧基底）⇒ 不受影响，不算；
/// 否则 ⇒ 它的有效值跟着基底变了 ⇒ 算一条 `follows`。
///
/// 注意这与"文件里有没有那一行"无关 —— 那是存储；这里问的是**模型**。
/// 所以 A1 与 A1_MINI 走**完全同一条分支**，行为天然一致。
fn fields_after_model(ctx: &Ctx, written: &[txn::FieldEntry]) -> Vec<txn::FieldEntry> {
    let mut out: Vec<txn::FieldEntry> = Vec::new();
    for e in written {
        /* 结构编辑与版本自己写的值：不展开，但**值要过一遍 `effective_text`**
        —— 日志写法不许漏到界面（作者 2026-11-03 截图那个
        「3.85 →（跟随上一层）」就是从这里出去的）。 */
        if e.level.as_deref() != Some("Machine") {
            out.push(txn::FieldEntry {
                level: e.level.clone(),
                owner: e.owner.clone(),
                key: e.key.clone(),
                label: e.label.clone(),
                before: Some(effective_text(ctx, &e.owner, &e.key, &e.before)),
                after: Some(effective_text(ctx, &e.owner, &e.key, &e.after)),
                why: e.why,
            });
            continue;
        }
        let Some(param) = ctx.presets.registry.param(&e.key) else {
            out.push(e.clone());
            continue;
        };
        let Some(m) = ctx.presets.catalog.machine(&e.owner) else {
            out.push(e.clone());
            continue;
        };

        /* 基底这一条：值要对得上"有效值"，不写 `（跟随上一层）` 那种日志话 */
        out.push(txn::FieldEntry {
            level: e.level.clone(),
            owner: e.owner.clone(),
            key: e.key.clone(),
            label: e.label.clone(),
            before: Some(effective_text(ctx, &e.owner, &e.key, &e.before)),
            after: Some(effective_text(ctx, &e.owner, &e.key, &e.after)),
            why: e.why,
        });

        for v in &m.versions {
            /* 这一版**不跟随** ⇒ 基底怎么改都动不到它。
            判据与徽章、写值路径**同一份**（`presetdata::follows::is_following`）——
            以前这里读"`machineVariants` 里有没有这一格"，而跟随的版本现在也会
            留着自己那一行（值同步成基底的那个数），拿行判会把它当成独立值。 */
            let uid_key = crate::presetdata::follows::uid_of(&e.owner, &v.id);
            if !crate::presetdata::follows::is_following(param, &uid_key) {
                continue;
            }
            let owner = format!("{}:{}", e.owner, v.id);
            /* 同一笔里已经单独记过这一格（真写了版本键）就不要再叠一条 */
            if written.iter().any(|w| {
                w.level.as_deref() == Some("Version") && w.owner == owner && w.key == e.key
            }) {
                /* 但要把它的来源归一成 `follows`，并把值换成有效值 */
                out.push(txn::FieldEntry {
                    level: Some(format!("{:?}", Level::Version)),
                    owner: owner.clone(),
                    key: e.key.clone(),
                    label: e.label.clone(),
                    before: Some(effective_text(ctx, &owner, &e.key, &e.before)),
                    after: Some(effective_text(ctx, &owner, &e.key, &e.after)),
                    why: WHY_FOLLOWS,
                });
                continue;
            }
            out.push(txn::FieldEntry {
                level: Some(format!("{:?}", Level::Version)),
                owner: owner.clone(),
                key: e.key.clone(),
                label: e.label.clone(),
                /* 它跟着基底走 ⇒ 前后值就是基底的前后值 */
                before: Some(effective_text(ctx, &e.owner, &e.key, &e.before)),
                after: Some(effective_text(ctx, &e.owner, &e.key, &e.after)),
                why: WHY_FOLLOWS,
            });
        }
    }
    out
}

/// 界面要显示的**值**：`None` 那种"删键"的动作，换成读出来的有效值。
///
/// 为什么必须换：`KeyChange` 里的 `before/after` 是**文件动作**
/// （`None` = 这个键被删了 / 本来没有），那是给日志看的。界面上问的是
/// "这一格从多少变成多少"，所以要把 `None` 解析成"当时那一层算出来是多少"。
///
/// 解析不到（参数被弃用、机型不适用）才退回日志写法 —— 那种情况本来就该说出来。
fn effective_text(ctx: &Ctx, owner: &str, key: &str, v: &Option<String>) -> String {
    /* ★ 双重保险：**任何**带日志写法的值都在这里被换成真实值。
    上游已经改成"`None` 不格式化"，但这个函数是界面取值的唯一出口 ——
    将来谁多写一处 `show(&None)`，也会在这里被挡住，而不是漏到屏幕上。
    作者 2026-11-03 截图里那个「→ （跟随上一层）」就是这么漏出去的。 */
    if let Some(text) = v {
        if !text.contains(FOLLOW_MARK) {
            return text.clone();
        }
    }
    let machine_id = owner.split(':').next().unwrap_or(owner);
    let param = ctx.presets.registry.param(key);
    let inherited = param
        .and_then(|p| p.machine_variants.get(machine_id).cloned())
        .or_else(|| param.map(|p| p.default_value.clone()));
    match inherited {
        Some(v) => crate::logfmt::show(&Some(v)),
        None => crate::logfmt::show(&None),
    }
}

/// `logfmt::show(None)` 里那个标记。界面不许把它显示出来（见 [`effective_text`]）。
const FOLLOW_MARK: &str = "跟随上一层";

/// 跟着基底变的那些格。**界面只有这一种"非用户"来源**
/// —— 不管文件里有没有真写版本键，用户看到的都是"基底带动一圈"。
pub(super) const WHY_FOLLOWS: &str = "follows";

fn value_target(ctx: &Ctx, level: Level, owner: &str) -> String {
    match level {
        Level::Machine => format!("{owner} · {}", w::level_label(Level::Machine)),
        /* 版本 uid 的形状是 `"{机型}/{版本 id}"`（见 `CommittedVersion`） */
        Level::Version => {
            let mut parts = owner.splitn(2, '/');
            let machine_id = parts.next().unwrap_or(owner);
            let version_id = parts.next().unwrap_or_default();
            match ctx
                .presets
                .catalog
                .machine(machine_id)
                .and_then(|m| m.versions.iter().find(|v| v.id == version_id))
            {
                Some(v) => format!("{machine_id} · {}", v.name),
                None => owner.to_owned(),
            }
        }
    }
}

/// **「改了什么」的数据源**（2026-11-03 定稿：事务是唯一数据源）。
///
/// # 为什么不再读 `ctx.draft`
///
/// 以前它遍历 `draft.values` / `draft.visibility` / `draft.bundles`。可
/// `wb_apply_draft` 把值写进影子树之后**立刻** `ctx.draft = Draft::default()`
/// —— 于是打开框时草稿层永远是空的，「改了什么」永远显示「这一份里没有格子级的
/// 改动」。真正改动只活在 `ctx.txn` 里，而这个函数以前没读它。
///
/// # 为什么从事务读而不是比影子树
///
/// 影子树里可能有**不是本次事务**造成的差异（上次会话的残留、外部改过），
/// 拿"影子树 vs 磁盘"去数会把它们一起算进来。事务里那份是"这一笔到底改了
/// 哪些字段"的确切记录 —— 作者 2026-11-03 的第 8 条硬约束。
///
/// 现在「未保存 N」也是数它（[`txn::Txn::dirty_count`]），两个数物理同源。
fn diff_draft(ctx: &Ctx) -> Vec<DiffLine> {
    ctx.txn
        .field_changes()
        .into_iter()
        .map(|e| {
            /* `owner` 的写法在两条路上不同：
            值改动是 `A1` / `A1:STANDARD`（`machineVariants` 的叫法），
            结构编辑是机型 id / 参数 key。`target` 只对前者有意义 ——
            后者把 `owner` 原样带出去，界面上它就是"改的是哪台机 / 哪个参数"。 */
            let target = match e.level.as_deref() {
                Some("Version") => target_of_version(ctx, &e.owner),
                Some("Machine") => format!("{} · {}", e.owner, w::level_label(Level::Machine)),
                _ => e.owner.clone(),
            };
            let file = file_of(ctx, e.why, &e.owner, &e.key);
            DiffLine {
                target,
                level: level_of(&e.level),
                owner: e.owner,
                key: Some(e.key),
                label: e.label,
                before: e.before.unwrap_or_default(),
                after: e.after.unwrap_or_default(),
                kind: kind_of(e.why),
                file,
                why: e.why.to_owned(),
            }
        })
        .collect()
}

/// 事务里的 `level` 是字符串（`domain` 不认识 `Level` 这个 wire 枚举），转回来
fn level_of(raw: &Option<String>) -> Option<Level> {
    match raw.as_deref() {
        Some("Machine") => Some(Level::Machine),
        Some("Version") => Some(Level::Version),
        _ => None,
    }
}

/// 版本归属的人话：`A1:STANDARD` → 「A1 · 标准版」
fn target_of_version(ctx: &Ctx, owner: &str) -> String {
    let mut parts = owner.splitn(2, ':');
    let machine_id = parts.next().unwrap_or(owner);
    let version_id = parts.next().unwrap_or_default();
    match ctx
        .presets
        .catalog
        .machine(machine_id)
        .and_then(|m| m.versions.iter().find(|v| v.id == version_id))
    {
        Some(v) => format!("{machine_id} · {}", v.name),
        None => owner.to_owned(),
    }
}

/// 来源标签 → 「改了什么」那一列的分类词（界面照它着色，不自己判）
fn kind_of(why: &str) -> &'static str {
    match why {
        "structural" => "改定义",
        /* 2026-11-03：`promoted`（为了保住值而多写的版本键）不再单独成一类 ——
        界面只问"这一格变没变"，不问"我们写了几个键"。它现在与不加键的继承
        是同一种表现，统一叫「随基底生效」。 */
        "follows" | "promoted" | "inherited" => "改值（随基底）",
        "copy" => "改值（复制）",
        _ => "改值",
    }
}

/// 这一笔落在哪一份文件（「代码对比」按它过滤用）
///
/// 值改动全在 `param_registry.toml`（`[params.machineVariants]` 那一族）；
/// 结构编辑按字段路径判断：值类定义在注册表，其余在机型 / 版本文件。
fn file_of(ctx: &Ctx, why: &str, owner: &str, key: &str) -> String {
    if why != "structural" {
        return VALUE_FILE.to_owned();
    }
    /* 结构编辑的 `owner` 是"谁被改了"：注册表里有这个参数 key ⇒ 注册表那一份
    （改说明 / 改范围 / 改出厂默认）；否则是机型 ⇒ 那台机型的文件。 */
    if ctx.presets.registry.param(owner).is_some() {
        return VALUE_FILE.to_owned();
    }
    let _ = key;
    format!("machines/{owner}.toml")
}

/* ---------- 写（唯一一条） ---------- */

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyResult {
    pub view: BookView,
    /// 撤销这次操作要提交的 patches，**倒序**。空 = 没有可撤销的改动
    /// （无变化的编辑不产反向，界面上不给撤销按钮）
    pub inverse: Vec<Patch>,
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
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "page"
)]
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
            /* **先声明这一笔改的是哪一格**，`run_ctx` 记事务时取走它做合并判定。
            一次调用只带一格（参数台的写入口按格发），带多格的（批量）不合并 ——
            那种"一次改十行"本来就该是一条，不需要靠窗口去并。 */
            ctx.edit_key = match patches.as_slice() {
                [Patch::SetValue {
                    level, owner, key, ..
                }] => Some(txn::EditKey {
                    level: *level,
                    owner: owner.clone(),
                    key: key.clone(),
                }),
                _ => None,
            };
            /* **撤销栈上那句人话在这里定**（2026-11-02）：前端给的是
            「A1 · 标准版 · 涂胶速度限制」，那是"给谁看的一句话"；而栈里的标签
            更应该像事件日志那样说清**改了什么**（`A1 · 标准版 · 改值`）。
            作者翻日志时看的正是这一条标题 —— 它一秒重复十几次，就说明没合并。 */
            let what_of = |key: &str| {
                ctx.presets
                    .registry
                    .param(key)
                    .map(|p| p.label.clone())
                    .unwrap_or_else(|| key.to_owned())
            };
            let label = match patches.as_slice() {
                [Patch::SetValue {
                    level, owner, key, ..
                }] => format!("{} · {}", value_target(ctx, *level, owner), what_of(key)),
                _ => label,
            };
            let out = apply_patches(
                &mut ctx.draft,
                &ctx.committed,
                &ctx.presets.registry,
                &patches,
            )?;
            /*
             * ★★ **2026-11-03：值编辑只落内存，一个字节都不写文件**。
             *
             * 以前这里是 `storage::apply_draft_now_traced(&mut ctx.presets, …)` ——
             * 算完立刻 `apply_values` **并**写影子树，于是每敲一下都是一次
             * 「快照整树 ×2 → 序列化整份 param_registry.toml → atomic_write → 记文件字节」。
             * 作者报的「我只是改个数字，怎么这么重」就是这条。见
             * `docs/WORKBENCH-EDITOR-MODEL.md` §6。
             *
             * 现在的顺序（**算 → 记命令 → 写内存**）：
             *
             * ① `stage_value_edits` 只**算**：把这一笔要写的每一行（含归并连带、
             *    跟随状态、`changes` 日志）算出来，不碰文档；
             * ② `history.record` 把这一笔收成**一条命令**（带逆命令），合并判据是时间；
             * ③ `stage_into` 把这些行写进**内存里的 `Presets`**。
             *
             * ②必须夹在①③之间：撤销要知道"这一笔改了什么"，而③之后就问不到了
             * （值已经是新的）。合并时反向沿用最早那条（见 `history::History::record`）。
             */
            let staged = storage::stage_value_edits(&ctx.presets, &ctx.committed, &ctx.draft);
            let fields = value_field_entries(ctx, &staged.changes);
            /*
             * 一次调用 = **一条命令**：单 patch 用它自己的 `EditKey`（允许按时间合并）；
             * 批量（多 patch 一次发）用 `None`（永不合并 —— 它们不是"同一件事的延续"）。
             * 逆命令由 `apply_patches` 现算（`out.inverse`，已倒序），`record` 只记账。
             */
            let merge_key = match patches.as_slice() {
                [one] => edit_history::merge_key_of(one),
                _ => None,
            };
            let recorded = ctx.history.record(
                label.clone(),
                merge_key,
                patches[0].clone(),
                out.inverse.clone(),
                fields.clone(),
            );
            let (written, changes) = storage::stage_into(&mut ctx.presets, &staged)?;
            /* 值已经落进**内存文档**（`ctx.presets`），草稿层随之清空 ——
            真正的"待保存差异"现在住在 `ctx.history` 里（字段账也在那儿）。 */
            ctx.pending_fields = fields;
            ctx.draft = Draft::default();
            ctx.touch();
            let notices = ctx.notices_now();
            tracing::info!(
                req = %current_req().unwrap_or_default(),
                label = %label,
                patches = patches.len(),
                written,
                merged = recorded.merged,
                undo_depth = ctx.history.undo_depth(),
                changes = %logging::render_changes(&changes),
                "值已应用（内存，未写文件）"
            );

            /* 一次派生，两份结果：整本视图 + 调用方要的那一页。
            ★ **必须走 `book_of`（带 `with_rest`）**：这一句就是参数台每次改值
            都会经过的那条路。漏了它 = 返回去的 `rest` 是**刚写进去的新值**，
            前端一比"值 == 保存值"就判不脏 ⇒ **徽章亮一帧就灭**（2026-11-02 作者报的
            「这次的时间更短」，比 `wb_matrix` 那处更短是因为它连下一次重取都不用等）。 */
            let book = book_of(ctx);
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
                        None,
                    )),
                ),
            };
            /* 用 `view_of` 而不是直接 `book.book_view()`：**未保存改动数 = 值草稿 +
            事务条数**，那一步的加法只住在 `view_of` 里。之前这里直接取 `book_view()`，
            而值已经搬进影子树、草稿层清空 ⇒ 返回的 `dirtyCount` 恒为 0，
            界面上就是"改完立刻显示未保存（0）"，只有 `wb_reload`（重开软件 /
            手动刷新）走 `view_of` 才对得上（2026-11-02 作者报的正是这一条）。 */
            let mut view = view_of(ctx, &ctx.committed, &Draft::default(), notices.clone());
            /* 状态灯要的是**存草稿之后**的状态，不是存之前那份 */
            view.snapshot = ctx.snapshot();

            Ok(ApplyResult {
                view,
                inverse: out.inverse,
                notices,
                desk,
                matrix,
            })
        })
    })
}

/// **从模板复制参数正文**（b05 Task 14.5 / doc §4.3 第 7 步）。
///
/// # 语义（裁决 2026-09-24：方案 A，独立快照）
///
/// 取模板版本的**完整有效配方**（defaults ⊕ 机型基底 ⊕ 模板覆盖，归并后的最终值），
/// 经 [`Presets::apply_values`] 批量钉成新版本的显式覆盖。复制后两边**独立演化**：
/// 再改模板或基底，不影响已复制的新版本 —— 界面上要说清「复制为独立版本，
/// 后续修改互不影响」。
///
/// # 两条不许越的线
///
/// - **缺失参数不伪造**：只写有效配方里真实存在的键（registry 可见键的归并值），
///   不在的继续继承 defaults —— 不给缺的键造默认值；
/// - **不复制无效配方**：有效配方来自 registry 归并，解析不过的版本根本进不了
///   layers，不存在「半份配方」这个形状。
///
/// # 为什么现读而不是走 `Ctx`
///
/// `wb_copy_version`（14.3）是独立命令、独立落盘；会话缓存 `ctx.committed` 里
/// **没有它**。这里全部从盘上现读 —— 盘上真相是唯一可靠的判定依据。
/// 这也是 doc 第 5 步与第 7 步分成两个动作的另一个好处：每步都是单文件写入，
/// 不需要跨文件事务。
/// [`wb_copy_recipe`] 的领域体（收 `&mut Presets` 以便判据直接测）。
pub(crate) fn copy_recipe(
    p: &mut Presets,
    machine_id: &str,
    template_version_id: &str,
    new_version_id: &str,
) -> Result<usize, AppError> {
    let m = p
        .catalog
        .machine(machine_id)
        .ok_or_else(|| AppError::not_found(format!("没有机型 {machine_id}")))?;
    let new_uid = format!("{machine_id}:{new_version_id}");
    // 新版本必须已在清单里 —— 14.3 先建定义，这里才有的复制
    if !m.versions.iter().any(|v| v.id == new_version_id) {
        return Err(AppError::not_found(format!(
            "版本 {new_uid} 不在清单里 —— 先复制版本定义，再复制参数正文"
        )));
    }
    if !m.versions.iter().any(|v| v.id == template_version_id) {
        return Err(AppError::not_found(format!(
            "模板版本 {machine_id}/{template_version_id} 不存在"
        )));
    }

    // 模板的完整有效配方：digest 出三层，归并后的值就是快照内容
    let version_ids: Vec<String> = m.versions.iter().map(|v| v.id.clone()).collect();
    let digested = variants::digest(&p.registry, machine_id, &version_ids);
    let template_over = digested
        .versions
        .get(template_version_id)
        .cloned()
        .unwrap_or_default();
    let layers = Layers::new(&p.registry, machine_id, &digested.base, &template_over);
    let effective = layers.effective_recipe();
    if effective.is_empty() {
        return Err(AppError::invalid_argument(format!(
            "模板版本 {template_version_id} 的有效配方是空的，没有可复制的内容"
        )));
    }

    let edits: Vec<(String, String, Option<serde_json::Value>)> = effective
        .iter()
        .map(|(k, v)| ((*k).to_owned(), new_uid.clone(), Some((*v).clone())))
        .collect();
    let keys = edits.len();
    // 批量入口：先全查 owner 与键，再一次落盘 —— 74 个键写 74 次盘是那条
    // 文档警告过的反模式
    p.apply_values(&edits)?;
    Ok(keys)
}

#[tauri::command]
pub fn wb_copy_recipe(
    machine_id: String,
    template_version_id: String,
    new_version_id: String,
) -> Result<usize, AppError> {
    traced("wb_copy_recipe", |_| {
        /* ★ 改**会话里那一份**（影子树），不是 `load_presets()`（真仓库）。
         * 这两条路以前各走各的：命令登记在 `EDIT_COMMANDS` 里、事务拍的是影子，
         * 而写盘却落在真仓库 —— 和 22:17 那桩案子同一种病（2026-11-01）。 */
        with_struct_ctx(
            "",
            |_| {
                vec![
                    "registry/param_registry.toml".to_owned(),
                    "layout_schema.toml".to_owned(),
                ]
            },
            |ctx| {
                let keys = copy_recipe(
                    &mut ctx.presets,
                    &machine_id,
                    &template_version_id,
                    &new_version_id,
                )?;
                ctx.reload_presets()?;
                tracing::info!(
                    machine = %machine_id,
                    template = %template_version_id,
                    version = %new_version_id,
                    keys,
                    "参数正文复制完成（独立快照，此后互不影响）"
                );
                Ok(keys)
            },
        )
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveResult {
    pub view: BookView,
    /// 旧 uid → 新 uid（见 `storage` 模块文档那条 `remap 为什么还在`）
    pub remap: BTreeMap<String, String>,
}

#[tauri::command]
pub fn wb_save() -> Result<SaveResult, AppError> {
    traced("wb_save", |_| with_ctx_mut(save_in))
}

/// [`wb_save`] 的本体（**抽出来是为了可测**）。
///
/// 保存是**唯一写 `presets/` 的边界**：值草稿先落进影子树，再和结构类编辑一起提交。
/// 回归判据 `a_session_opened_from_disk_edits_the_shadow_until_save` 直接调它 ——
/// 它要走的就是真机上那颗「保存」的同一段代码。
pub(super) fn save_in(ctx: &mut Ctx) -> Result<SaveResult, AppError> {
    /* ★ 2026-10-31：保存是**唯一写盘边界**。
    值草稿先落进影子树，再和结构类编辑一起提交 —— 两套编辑走同一个出口 */
    let values_dirty = !ctx.draft.is_clean() || !ctx.history.is_empty();
    if !values_dirty && !ctx.txn.is_dirty() {
        /* **没东西可存 = 已经保存过了**，不是失败（2026-11-02）。
        以前这里报 `NOTHING_TO_SAVE`，界面上表现成一条**永远消不掉**的错误横幅
        （`fatal` 只在设、从不给关），而人只是多点了一下保存。
        幂等：什么都不做，把现在的状态交回去。 */
        let notices = ctx.notices_now();
        return Ok(SaveResult {
            view: view_of(ctx, &ctx.committed, &ctx.draft, notices),
            remap: BTreeMap::new(),
        });
    }
    /* ★ 刀 6：磁盘在会话期间被外部改过 ⇒ **不许静默覆盖**。
     *
     * 2026-10-31「A1 三份 0.7 被写回 0.8」就是这条没拦：工作台拿着一份旧内容
     * 写回了真仓库。`diskStamp` 是「打开会话时 / 上次保存后」那一刻的指纹，
     * 与现在不同就说明中间有人动过 —— 保留哪一份是人的决定，不是程序的。
     *
     * 空 `diskStamp` = 老账本 / 测试夹具 ⇒ 跳过（它们没有"打开会话"这个动作）。 */
    let sess = txn::read_session(&ctx.store.root);
    if !sess.disk_stamp.is_empty() && sess.disk_stamp != presets_source_stamp(&ctx.real_root) {
        return Err(AppError::invalid_argument(
            postprocess::diag::messages::text("ui.workbench.save.stale", &[]),
        ));
    }
    // 保存前先有一份对得上的快照：万一 save 中途挂了，草稿还在
    ctx.flush();
    /*
     * ★★ **2026-11-03：先把历史里那些命令一次写进影子树**。
     *
     * 编辑态改了（值只落内存），所以影子树可能一个字节都没变过 —— 而
     * `txn::commit` 比的是真源 ↔ 影子。不先写这一笔，保存就会写空。
     *
     * 这里的字节来源是 `ctx.presets`（已经含着全部命令），所以**一次 `write_back`**，
     * 不是 N 次。仍然只有「保存」这一个时刻会碰文件。
     */
    if !ctx.history.is_empty() {
        ctx.presets.write_back()?;
        tracing::info!(
            commands = ctx.history.undo_depth(),
            "保存：历史里的命令一次写进影子树"
        );
    }
    let mut remap = BTreeMap::new();
    if values_dirty {
        let out = storage::save(&ctx.store, &mut ctx.presets, &ctx.committed, &ctx.draft)?;
        remap = out.remap;
    }
    // 影子树 → 真仓库：逐份原子写、删除进回收站、失败整体回滚
    let stamp = crate::workbench::clock::now_iso8601().replace(':', "-");
    /* ★ 提交阶段机（刀 5）：**逐份原子 ≠ 整笔原子**。
    下面四步之间任何一处崩溃，都要能在下次启动时分辨「磁盘动没动」。
    见 `txn::SaveStage` / `recover_save_stage`。 */
    let wb = ctx.store.root.clone();
    txn::write_save_stage(&wb, txn::SaveStage::Prepared)?;
    let report = txn::commit(
        &ctx.real_root,
        &ctx.shadow_root,
        &ctx.store.root.join(".trash"),
        &stamp,
    )?;
    // 走到这里 = 磁盘已经是新的。**从这一刻起，基准必须跟上**，否则崩溃会让
    // 「已保存」与「基准」分裂，用户点放弃就把刚存的东西退回去。
    txn::write_save_stage(&wb, txn::SaveStage::RealCommitted)?;
    tracing::info!(
        written = report.written,
        deleted = report.deleted,
        ops = ctx.txn.undo_depth(),
        "保存：事务已提交"
    );

    ctx.draft = Draft::default();
    ctx.txn.clear();
    ctx.history.clear();
    /* **保存成功后同步基准**（2026-11-01 R3）。
     *
     * 基准记的是"影子从哪里变过来的"。`commit` 已经把影子的内容写进真源，
     * 此刻真源 == 影子 == 保存后的状态，所以基准也该挪到这一份。
     *
     * 位置很关键：**必须放在 `commit` 成功之后**。
     * 提前写会在事务失败时把"证据"抹掉 —— 那样下次打开就分不出
     * "用户改了没存"和"已经有了"，失败时反而更不能恢复。
     *
     * 保存是全量提交（影子整份进真源），所以直接镜像即可，
     * 不存在"只同步一部分、把别的冲突一起抹掉"的问题。 */
    let base_root = txn::base_root(&ctx.store.root);
    txn::mirror(&ctx.real_root, &base_root)?;
    txn::write_save_stage(&wb, txn::SaveStage::BaselineAdvanced)?;
    /* 基准是一份**独立的内存文档**（刀 1）：盘上搬完了还得把它读进来，
    否则「改之前的值」还停在上一次保存 —— 刚存完徽章就会一直亮着。 */
    ctx.base = Presets::load_from(&base_root)?;
    // 保存之后重读：落盘的那一份才是新的真相，草稿也被清空了
    ctx.reload_from_disk()?;
    /* 保存把磁盘换成了新的一代 ⇒ 「打开时那一刻」也跟着变成这一代。
    不更新的话，下一次保存会被刀 6 的校验误判成「外部改过」。 */
    txn::write_disk_stamp(&wb, &presets_source_stamp(&ctx.real_root))?;
    // 全部落定才清阶段：半路崩在任何一个 `?` 上，下次启动都还知道该补哪一步
    txn::write_save_stage(&wb, txn::SaveStage::Idle)?;
    let notices = ctx.notices_now();
    let view = view_of(ctx, &ctx.committed, &ctx.draft, notices);
    Ok(SaveResult { view, remap })
}

#[tauri::command]
pub fn wb_discard() -> Result<BookView, AppError> {
    traced("wb_discard", |_| with_ctx_mut(discard_in))
}

/// [`wb_discard`] 的本体（**抽出来是为了可测**）：影子整个拉回磁盘原状、内存重读。
pub(super) fn discard_in(ctx: &mut Ctx) -> Result<BookView, AppError> {
    if ctx.draft.is_clean() && !ctx.txn.is_dirty() && ctx.history.is_empty() {
        /* **没有可放弃的东西 = 已经是你要的样子**（2026-11-02）。
        作者点第二次「放弃改动」时，这条 `Err` 变成一条永远消不掉的横幅
        （`fatal` 只设不清、z 60、不能点穿），只能重开软件。
        幂等：什么都不做，把现在的状态交回去。 */
        let notices = ctx.notices_now();
        return Ok(view_of(ctx, &ctx.committed, &ctx.draft, notices));
    }
    let dropped = ctx.draft.dirty_count() + ctx.txn.dirty_count() + ctx.history.undo_depth();
    ctx.draft = Draft::default();
    /*
     * ★ **放弃 = 把历史里的命令全部忘掉、文档从 `Base` 重新展开**（2026-11-03）。
     *
     * 值编辑不再写影子树，所以"放弃"不等于"把影子拉回去" —— 内存文档才是那个
     * 被改过的东西。把 `ctx.presets` 换成 `base` 的副本即可（`reload()` 从
     * `base_root` 读一次，几十个文件，可接受：放弃不是每帧都会发生的动作）。
     *
     * 顺序：**先**换文档再清历史 —— 反过来的话，中途失败会留下"文档还是新的、
     * 历史已经没了"，那些改动就再也退不掉了。
     */
    let had_commands = !ctx.history.is_empty();
    if had_commands {
        ctx.presets = ctx.base.reload()?;
    }
    ctx.history.clear();
    // 先把"空草稿"落进快照（干净 = 直接删文件）：不先落的话，
    // 下面 reload_from_disk 会把盘上那份旧草稿读回来，放弃就白放了
    ctx.flush();
    // ★ 放弃 = 把影子树整个拉回磁盘原状（结构类改动也一起丢）
    /* ★ **放弃 = 回到「打开工作台那一刻」**（刀 3，2026-11-02 作者裁定）。
     *
     * 以前这里是 `mirror(real → shadow)` —— 让影子等于**磁盘现在**。
     * 两者只在一种情况下不同，而那种情况恰恰最危险：开会话到点放弃之间，
     * `presets/` 被外部改过（`git checkout`、AI 直接改文件、另一个终端 publish）。
     * 那时「放弃」会把外部改动一起变成工作台的状态，而用户以为只是作废自己的编辑。
     *
     * 基准（`base_root`）**不动**：它就是"我来的时候"那一份。
     * 增删改名排序全都不用逐种回退 —— 整份覆盖天然覆盖它们。 */
    let base_root = txn::base_root(&ctx.store.root);
    txn::mirror(&base_root, &ctx.shadow_root)?;
    ctx.base = Presets::load_from(&base_root)?;
    ctx.txn.clear();
    /* **放弃后同样要重读内存**（2026-11-01）。
     *
     * 影子文件虽然拉回去了，但 `ctx.presets.registry` / `ctx.committed`
     * 还停在"改完之后"的那份内存态 —— 分组换位是直接改内存再写影子
     * 的，不重读的话 `view_of` 吐出来的还是换位后的次序，前端看起来
     * 就是"点了放弃没反应"。保存路径（`wb_save`）在 commit 后一直
     * 有这一步，这里是把漏掉的那半补齐。 */
    ctx.reload_from_disk()?;
    tracing::info!(dropped, "草稿与事务已丢弃（已回到打开工作台那一刻）");
    let notices = ctx.notices_now();
    Ok(view_of(ctx, &ctx.committed, &ctx.draft, notices))
}

/* ---------- 统一事务：撤销 / 重做 / 状态 ---------- */

#[tauri::command(async)]
pub fn wb_txn_state() -> Result<TxnState, AppError> {
    traced("wb_txn_state", |_| with_ctx(|ctx| Ok(ctx.txn_state())))
}

/// **未保存改动的逐行 diff**：盘上的 `presets/` ↔ 影子树。
///
/// 与 [`wb_diff_draft`]（对象视角：「哪一格从什么值改成什么值」）并列，不是替换 ——
/// 作者 2026-11-02：「其实我觉得代码的 diff 就很直观了」。两个框看的是同一件事的两面：
/// 对象视角回答"我动了哪些格子"，这个回答"文件因此变成什么样"。
///
/// **只读**：快照 + 读字节，一个字节都不写（`read_commands_are_async` 的白名单里有它）。
#[tauri::command(async)]
pub fn wb_shadow_diff() -> Result<shadow_diff::ShadowDiff, AppError> {
    traced("wb_shadow_diff", |_| {
        with_ctx(|ctx| shadow_diff::shadow_diff(&ctx.real_root, &ctx.shadow_root))
    })
}

/// 把一条结构命令的文件字节**换回去**（`undo=true` 换 `before`，`false` 换 `after`）。
///
/// 换完 `reload_presets()` 重建内存文档并重放值编辑历史 —— 结构编辑写的是影子文件，
/// 内存文档是它的镜像；而值编辑只落内存，不重放会被冲掉。
fn swap_struct_files(
    ctx: &mut Ctx,
    cmd: &edit_history::StructCmd,
    undo: bool,
) -> Result<(), AppError> {
    let root = ctx.shadow_root.clone();
    for f in &cmd.files {
        let bytes = if undo { &f.before } else { &f.after };
        match bytes {
            Some(b) => crate::fsx::atomic::atomic_write(&root.join(&f.path), b)?,
            /* `before` 为 `None` = 这一笔新建了它 ⇒ 撤销要删掉 */
            None => {
                let _ = std::fs::remove_file(root.join(&f.path));
            }
        }
    }
    ctx.reload_presets()?;
    Ok(())
}

/// 撤销一条（服务器侧）：把影子树里那份 `before` 写回去，再重读
#[tauri::command]
pub fn wb_txn_undo() -> Result<TxnState, AppError> {
    traced("wb_txn_undo", |_| with_ctx_mut(undo_in))
}

/// [`wb_txn_undo`] 的本体（**抽出来是为了可测**）：先撤值编辑那条内存历史，
/// 撤空了才回落到结构编辑的文件事务栈。
pub(super) fn undo_in(ctx: &mut Ctx) -> Result<TxnState, AppError> {
    {
        /*
         * ★ **两套栈并存时，撤销必须退"最近的那一笔"**（2026-11-03）。
         *
         * 迁移期：值编辑在 `ctx.history`（内存命令），结构编辑还在 `ctx.txn`
         * （文件事务）。谁后发生退谁 —— 按栈名轮流退会让撤销顺序与真实顺序相反：
         * 改了个参数、又改了机型信息之后按 Cmd+Z，必须退机型信息那一笔。
         * 两个 `Instant` 来自同一个进程，可比。
         */
        let use_history = match (ctx.history.undo_last_at(), ctx.txn.undo_last_at()) {
            (Some(h), Some(t)) => h >= t,
            (Some(_), None) => true,
            _ => false,
        };
        if use_history {
            /* 反向是「一条命令」：先算它要写哪些行（**在撤销之前算**，
            因为 `changes` 要的是"改前"），再应用 */
            let staged = ctx.history.inverse_staged(&ctx.presets, &ctx.committed);
            let Some(entry) = ctx.history.undo() else {
                return Ok(ctx.txn_state());
            };
            match &entry.action {
                edit_history::Action::Value { .. } => {
                    if let Some(staged) = staged {
                        storage::stage_into(&mut ctx.presets, &staged)?;
                    }
                }
                /* 结构编辑：把这一条碰过的那几个文件写回 `before` 字节 */
                edit_history::Action::Struct(cmd) => swap_struct_files(ctx, cmd, true)?,
            }
            /* 撤销后草稿回落到"没挂东西"：撤销一次 = 撤掉一条命令，草稿层不该留着
            那一条的痕迹（它已经不在历史里了） */
            ctx.draft = Draft::default();
            ctx.touch();
            tracing::info!(label = %entry.label, "撤销（命令历史）");
            return Ok(ctx.txn_state());
        }
        let mut undo_after_commit = false;
        if let Some(label) = ctx.txn.undo(&ctx.shadow_root)? {
            /*
             * ★ **重建内存文档之后必须重放值编辑历史**（2026-11-03）。
             *
             * `reload()` 从影子树重建 `ctx.presets` —— 而值编辑**根本不写影子树**，
             * 不重放的话，撤一次结构编辑会把内存里所有值编辑一起冲掉
             * （实测：撤机型改名，参数那一笔也跟着没了）。
             */
            ctx.reload_presets()?;
            ctx.touch();
            tracing::info!(%label, "事务撤销（结构编辑）");
            undo_after_commit = true;
        }
        if undo_after_commit {
            ctx.reload_from_disk()?;
        }
        Ok(ctx.txn_state())
    }
}

/// 重做一条
#[tauri::command]
pub fn wb_txn_redo() -> Result<TxnState, AppError> {
    traced("wb_txn_redo", |_| with_ctx_mut(redo_in))
}

/// [`wb_txn_redo`] 的本体（同 [`undo_in`]）
pub(super) fn redo_in(ctx: &mut Ctx) -> Result<TxnState, AppError> {
    {
        /* 同 `undo`：重做也按"谁被撤得更晚"选栈 */
        let use_history = match (ctx.history.redo_last_at(), ctx.txn.redo_last_at()) {
            (Some(h), Some(t)) => h >= t,
            (Some(_), None) => true,
            _ => false,
        };
        if use_history {
            let staged = ctx.history.redo_staged(&ctx.presets, &ctx.committed);
            let Some(entry) = ctx.history.redo() else {
                return Ok(ctx.txn_state());
            };
            match &entry.action {
                edit_history::Action::Value { .. } => {
                    if let Some(staged) = staged {
                        storage::stage_into(&mut ctx.presets, &staged)?;
                    }
                }
                edit_history::Action::Struct(cmd) => swap_struct_files(ctx, cmd, false)?,
            }
            ctx.draft = Draft::default();
            ctx.touch();
            tracing::info!(label = %entry.label, "重做（命令历史）");
            return Ok(ctx.txn_state());
        }
        let mut redone = false;
        if let Some(label) = ctx.txn.redo(&ctx.shadow_root)? {
            /* 同 `undo_in`：重建后重放值编辑历史 */
            ctx.reload_presets()?;
            ctx.touch();
            tracing::info!(%label, "事务重做（结构编辑）");
            redone = true;
        }
        if redone {
            ctx.reload_from_disk()?;
        }
        Ok(ctx.txn_state())
    }
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
    use crate::workbench::domain::wording::BuildState;
    use crate::workbench::domain::Origin;

    /* ---------- 验收：影子树只代表"未保存的草稿"（2026-11-01） ---------- */

    /// **外部改了真源、影子落后 ⇒ 那不是"用户的草稿"**。
    ///
    /// 这是 `reload` "要不要问一句"的判定基础：
    /// · 外部改源：影子落后于真源 → `edited_against` 算出 0 → 直接对齐，不打扰人；
    /// · 用户在工作台改了没保存：影子里多出真源没有的内容 → >0 → 必须先确认。
    #[test]
    fn shadow_edits_are_counted_only_when_they_are_the_users_drafts() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real");
        let shadow = dir.path().join("shadow");
        for d in [&real, &shadow] {
            std::fs::create_dir_all(d.join("machines")).unwrap();
        }
        crate::fsx::atomic::atomic_write(&real.join("machines/A1.toml"), b"v2\n").unwrap();
        crate::fsx::atomic::atomic_write(&shadow.join("machines/A1.toml"), b"v1\n").unwrap();

        let sh = txn::snapshot(&shadow).unwrap();
        let rl = txn::snapshot(&real).unwrap();

        /* **影子落后（外部改了真源）⇒ 不是草稿**。这是本次修复的核心判据：
        只看"内容不同"会把两种情形都算成草稿，于是 `diff` 拿旧影子比、
        报"0 份会变"（假阴性，2026-11-01 实测）。 */
        assert!(
            sh.has_files_absent_from(&rl).is_empty(),
            "影子只是旧拷贝、不含真源没有的文件 ⇒ 没有用户草稿"
        );

        // 用户新建了一个真源里没有的文件 ⇒ 那才是草稿
        crate::fsx::atomic::atomic_write(&shadow.join("machines/NEW.toml"), b"x\n").unwrap();
        let sh = txn::snapshot(&shadow).unwrap();
        assert_eq!(
            sh.has_files_absent_from(&rl),
            vec!["machines/NEW.toml".to_owned()],
            "影子独有的文件 = 用户的未保存新建"
        );

        // 生成产物不参与（每次生成都会变，不是用户的编辑）
        std::fs::create_dir_all(shadow.join("delivery/mkp/presets")).unwrap();
        crate::fsx::atomic::atomic_write(&shadow.join("delivery/mkp/presets/A1.toml"), b"y\n")
            .unwrap();
        let sh = txn::snapshot(&shadow).unwrap();
        assert_eq!(
            sh.has_files_absent_from(&rl),
            vec!["machines/NEW.toml".to_owned()],
            "delivery/ 变化不算用户草稿"
        );
    }

    /// ★ **同文件冲突**：用户改了既有文件没保存，AI 又改了同一个文件 ——
    /// 必须认成"冲突"，不许当成"没有草稿"把用户的改动覆盖掉（2026-11-01 作者点名）。
    ///
    /// ```text
    /// 基准 0.8 / 影子（用户）0.9 / 真源（AI）0.85
    /// → drafts=[…] conflicts=[…]  ← 不能自动选边
    /// ```
    #[test]
    fn an_unsaved_edit_to_an_existing_file_is_a_conflict_not_a_no_op() {
        let dir = tempfile::tempdir().unwrap();
        let (real, shadow, base) = (
            dir.path().join("real"),
            dir.path().join("shadow"),
            dir.path().join("base"),
        );
        for d in [&real, &shadow, &base] {
            std::fs::create_dir_all(d.join("registry")).unwrap();
            // 三方一开始都是 0.8（基准就是"铺影子时的那一份"）
            crate::fsx::atomic::atomic_write(&d.join("registry/param.toml"), b"v = 0.8\n").unwrap();
        }

        // 用户在影子树里改成都没保存；AI 把真源改成 0.85
        crate::fsx::atomic::atomic_write(&shadow.join("registry/param.toml"), b"v = 0.9\n")
            .unwrap();
        crate::fsx::atomic::atomic_write(&real.join("registry/param.toml"), b"v = 0.85\n").unwrap();

        let v = txn::snapshot(&shadow).unwrap().reconcile(
            &txn::snapshot(&real).unwrap(),
            &txn::snapshot(&base).unwrap(),
        );

        assert_eq!(
            v.conflicts,
            vec!["registry/param.toml".to_owned()],
            "两边都改了同一个文件 = 冲突"
        );
        assert!(v.drafts.is_empty(), "冲突不是普通草稿，要分开报");
        assert!(
            !v.safe_to_reset(),
            "冲突时绝不能按真源重置（会丢掉用户那份）"
        );
        assert!(v.needs_attention());
    }

    /// 只有**影子**改了既有文件（真源没动）→ 是草稿，不是冲突。
    #[test]
    fn a_lone_shadow_edit_to_an_existing_file_is_a_draft() {
        let dir = tempfile::tempdir().unwrap();
        let (real, shadow, base) = (
            dir.path().join("real"),
            dir.path().join("shadow"),
            dir.path().join("base"),
        );
        for d in [&real, &shadow, &base] {
            std::fs::create_dir_all(d.join("registry")).unwrap();
            crate::fsx::atomic::atomic_write(&d.join("registry/param.toml"), b"v = 0.8\n").unwrap();
        }
        crate::fsx::atomic::atomic_write(&shadow.join("registry/param.toml"), b"v = 0.9\n")
            .unwrap();

        let v = txn::snapshot(&shadow).unwrap().reconcile(
            &txn::snapshot(&real).unwrap(),
            &txn::snapshot(&base).unwrap(),
        );
        assert_eq!(v.drafts, vec!["registry/param.toml".to_owned()]);
        assert!(v.conflicts.is_empty());
        assert!(!v.safe_to_reset());
    }

    /// 只有**真源**改了（用户没动影子）→ 不是草稿，可以安全对齐。
    #[test]
    fn an_external_change_with_a_clean_shadow_is_safe_to_reset() {
        let dir = tempfile::tempdir().unwrap();
        let (real, shadow, base) = (
            dir.path().join("real"),
            dir.path().join("shadow"),
            dir.path().join("base"),
        );
        for d in [&real, &shadow, &base] {
            std::fs::create_dir_all(d.join("registry")).unwrap();
            crate::fsx::atomic::atomic_write(&d.join("registry/param.toml"), b"v = 0.8\n").unwrap();
        }
        crate::fsx::atomic::atomic_write(&real.join("registry/param.toml"), b"v = 0.85\n").unwrap();

        let v = txn::snapshot(&shadow).unwrap().reconcile(
            &txn::snapshot(&real).unwrap(),
            &txn::snapshot(&base).unwrap(),
        );
        assert!(v.drafts.is_empty());
        assert!(v.conflicts.is_empty());
        assert_eq!(v.external, vec!["registry/param.toml".to_owned()]);
        assert!(v.safe_to_reset(), "外部改源不该拦人");
    }

    /* ---------- R2：经 `settle_shadow` 的端到端测试（2026-11-01） ----------
     *
     * 上一轮只测了 `reconcile` 本身，没测"打开会话时到底会发生什么"。
     * 于是 R3（保存后不更新基准）和 R1（基准缺失拿真源顶替）都溜过去了。
     * 下面这些走 `settle_shadow`，验**返回的源与磁盘上的实际字节**，不验文案。
     */

    /// 搭一个"工作台 + 真源"的最小现场：`<tmp>/workbench` + `<tmp>/presets`。
    fn shadow_scene(real_v: &str) -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let wb = dir.path().join("workbench");
        let real = dir.path().join("presets");
        std::fs::create_dir_all(real.join("registry")).unwrap();
        crate::fsx::atomic::atomic_write(
            &real.join("registry/param.toml"),
            format!("v = {real_v}\n").as_bytes(),
        )
        .unwrap();
        std::fs::create_dir_all(&wb).unwrap();
        (dir, wb, real)
    }

    /// 读某一棵树里的实际字节（判断"读的是哪一份"用这个，不看路径字符串）
    fn read_v(root: &Path) -> String {
        std::fs::read_to_string(root.join("registry/param.toml")).unwrap_or_default()
    }

    /// 首次打开：铺影子 + 铺基准；**会话要读的那一份就在影子里**（真源只用来铺）。
    #[test]
    fn e2e_first_open_lays_down_shadow_and_base() {
        let (_d, wb, real) = shadow_scene("0.8");
        let recovered = settle_shadow(&wb, &real).unwrap();
        assert!(!recovered, "首次打开没有草稿");
        assert_eq!(read_v(&wb.join(".draft/presets")), "v = 0.8\n");
        assert_eq!(read_v(&real), "v = 0.8\n");
        assert!(
            wb.join(".draft/presets/registry/param.toml").exists(),
            "影子要铺出来"
        );
        assert!(
            wb.join(".draft/base/registry/param.toml").exists(),
            "基准要铺出来"
        );
    }

    /// 场景 1：基准 0.8 / 影子草稿 0.9 / 真源被外部改成 0.85 -> **重新打开仍识别冲突**。
    #[test]
    fn e2e_conflict_survives_reopening() {
        let (_d, wb, real) = shadow_scene("0.8");
        settle_shadow(&wb, &real).unwrap();

        crate::fsx::atomic::atomic_write(
            &wb.join(".draft/presets/registry/param.toml"),
            b"v = 0.9\n",
        )
        .unwrap();
        crate::fsx::atomic::atomic_write(&real.join("registry/param.toml"), b"v = 0.85\n").unwrap();

        /* ★ 2026-11-02 口径变了：**草稿不跨重启**。
        以前这里是"两边都改过 = 冲突 ⇒ 保留草稿、按一条未保存改动呈现"。
        现在重开一律以磁盘为准（冲突也一样 —— 草稿本来就不留），
        真源仍然不许被草稿覆盖，只是草稿也不留。 */
        let recovered = settle_shadow(&wb, &real).unwrap();
        assert!(!recovered, "草稿不跨重启 ⇒ 永远没有'恢复出来的草稿'");
        assert_eq!(
            read_v(&wb.join(".draft/presets")),
            "v = 0.85\n",
            "重开之后工作区对齐成磁盘那一份"
        );
        assert_eq!(read_v(&real), "v = 0.85\n", "真源不许被草稿覆盖");
    }

    /// 场景 2：保存成功后重新打开 -> **不再误报草稿或冲突**（R3）。
    #[test]
    fn e2e_after_save_reopening_is_clean() {
        let (_d, wb, real) = shadow_scene("0.8");
        settle_shadow(&wb, &real).unwrap();

        crate::fsx::atomic::atomic_write(
            &wb.join(".draft/presets/registry/param.toml"),
            b"v = 0.9\n",
        )
        .unwrap();
        // 与 `wb_save` 同一顺序：commit 成功 -> 同步基准
        crate::workbench::domain::txn::commit(
            &real,
            &wb.join(".draft/presets"),
            &wb.join(".trash"),
            "test",
        )
        .unwrap();
        crate::workbench::domain::txn::mirror(
            &real,
            &crate::workbench::domain::txn::base_root(&wb),
        )
        .unwrap();

        let recovered = settle_shadow(&wb, &real).unwrap();
        assert!(!recovered, "刚保存过，不该再有草稿");
        assert_eq!(read_v(&wb.join(".draft/presets")), "v = 0.9\n");
        assert_eq!(read_v(&real), "v = 0.9\n", "保存确实落盘了");
    }

    /// 场景 3：放弃草稿后重新打开 -> 状态正确（不误报）。
    #[test]
    fn e2e_after_discard_reopening_is_clean() {
        let (_d, wb, real) = shadow_scene("0.8");
        settle_shadow(&wb, &real).unwrap();

        crate::fsx::atomic::atomic_write(
            &wb.join(".draft/presets/registry/param.toml"),
            b"v = 0.9\n",
        )
        .unwrap();
        // 与 `wb_discard` 同一顺序：影子拉回真源 -> 同步基准
        crate::workbench::domain::txn::mirror(&real, &wb.join(".draft/presets")).unwrap();
        crate::workbench::domain::txn::mirror(
            &real,
            &crate::workbench::domain::txn::base_root(&wb),
        )
        .unwrap();

        let recovered = settle_shadow(&wb, &real).unwrap();
        assert!(!recovered, "已放弃，不该再有草稿");
        assert_eq!(
            read_v(&wb.join(".draft/presets")),
            "v = 0.8\n",
            "放弃后回到磁盘那一版"
        );
    }

    /// 场景 4：基准缺失/损坏时，**草稿不会被静默覆盖**（R1）。
    #[test]
    fn e2e_missing_base_never_silently_drops_drafts() {
        // 1 基准目录整个不在，影子与真源不同 -> 必须按"有草稿"处理
        let (_d, wb, real) = shadow_scene("0.8");
        settle_shadow(&wb, &real).unwrap();
        crate::fsx::atomic::atomic_write(
            &wb.join(".draft/presets/registry/param.toml"),
            b"v = 0.9\n",
        )
        .unwrap();
        std::fs::remove_dir_all(wb.join(".draft/base")).unwrap();

        let recovered = settle_shadow(&wb, &real).unwrap();
        /* ★ 2026-11-02：草稿不跨重启 —— 基准缺失也不再"留着草稿让人看"，
        一律以磁盘为准。真源不动这一条不变。 */
        assert!(!recovered, "草稿不跨重启 ⇒ 没有'恢复出来的草稿'");
        assert_eq!(
            read_v(&wb.join(".draft/presets")),
            "v = 0.8\n",
            "重开之后工作区对齐成磁盘那一份"
        );
        assert_eq!(read_v(&real), "v = 0.8\n", "真源不许被动");

        // 2 基准损坏（内容不是合法 JSON）-> 同上
        let (_d2, wb2, real2) = shadow_scene("0.8");
        settle_shadow(&wb2, &real2).unwrap();
        crate::fsx::atomic::atomic_write(
            &wb2.join(".draft/presets/registry/param.toml"),
            b"v = 0.9\n",
        )
        .unwrap();
        crate::fsx::atomic::atomic_write(
            &wb2.join(".draft/base/manifest.json"),
            b"not json at all",
        )
        .unwrap();

        let recovered2 = settle_shadow(&wb2, &real2).unwrap();
        assert!(!recovered2, "基准损坏也不再兜草稿 —— 一律以磁盘为准");
        assert_eq!(read_v(&wb2.join(".draft/presets")), "v = 0.8\n");

        // 3 基准缺失、影子与真源本来就相同 -> 结果一样（铺成磁盘那份），基准重建
        let (_d3, wb3, real3) = shadow_scene("0.8");
        settle_shadow(&wb3, &real3).unwrap();
        std::fs::remove_dir_all(wb3.join(".draft/base")).unwrap();

        let recovered3 = settle_shadow(&wb3, &real3).unwrap();
        assert!(!recovered3);
        assert_eq!(read_v(&wb3.join(".draft/presets")), "v = 0.8\n");
        assert!(
            wb3.join(".draft/base/registry/param.toml").exists(),
            "基准要重建"
        );
    }

    /// `wb_reload` 在有未保存草稿时必须**拒绝**（除非明确 `discardDrafts=true`）。
    ///
    /// 以前一条命令就静默丢弃，谁点一下用户的编辑就没了（2026-11-01 数据保护）。
    #[test]
    fn reload_refuses_to_silently_drop_unsaved_drafts() {
        /* 这条判据只验"闸的逻辑"，不碰真仓库：直接看 `drafts_at_risk` 的语义
        —— 它读的是 store 与真 presets 根，测试环境里两者都可能不存在，
        所以这里验的是"没草稿时不会拦"这条最基本的（有草稿的那条由
        `shadow_edits_are_counted_only_when_they_are_the_users_drafts` 覆盖）。 */
        let r = drafts_at_risk();
        assert!(
            !r.doomed || r.edited > 0,
            "只有真的数出草稿才算会丢东西，不许凭空拦人"
        );
    }

    /// 「盘上的 `presets/` 在外面改过」只看**内容**，不看文件时间（2026-11-01 作者裁定）。
    ///
    /// 作者的原话：「AI 在外面改了预设，工作台打开后却把它当成异常；点重新加载
    /// 没反应，下次打开又出现」。根因是戳里揉进了 mtime —— `git checkout`、
    /// 编辑器另存、`touch` 都会让"内容没变"的文件被当成外部改过，横幅关不掉。
    #[test]
    fn the_outside_change_stamp_ignores_file_times() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("machines")).unwrap();
        let f = root.join("machines/A1.toml");
        crate::fsx::atomic::atomic_write(&f, b"[machine]\nid = \"A1\"\n").unwrap();

        let stamp = presets_source_stamp(root);

        // 只碰时间、内容一个字节不变 → 戳必须不变（不算"外部改过"）
        std::thread::sleep(std::time::Duration::from_millis(20));
        crate::fsx::atomic::atomic_write(&f, b"[machine]\nid = \"A1\"\n").unwrap();
        assert_eq!(
            presets_source_stamp(root),
            stamp,
            "内容没变，mtime 变了也不算外部改过"
        );

        // 内容真变了 → 戳必变
        crate::fsx::atomic::atomic_write(&f, b"[machine]\nid = \"A2\"\n").unwrap();
        assert_ne!(presets_source_stamp(root), stamp, "内容变了必须能认出来");
    }

    /// 交付产物**不参与**「外部改过」的判断（它是生成出来的，不是用户改的源）。
    #[test]
    fn the_outside_change_stamp_ignores_generated_products() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("machines")).unwrap();
        crate::fsx::atomic::atomic_write(&root.join("machines/A1.toml"), b"id = \"A1\"\n").unwrap();
        let stamp = presets_source_stamp(root);

        let delivery = root.join(crate::workbench::paths::DELIVERY_SUBDIR);
        std::fs::create_dir_all(delivery.join("mkp/presets")).unwrap();
        crate::fsx::atomic::atomic_write(&delivery.join("mkp/presets/A1-standard.toml"), b"x\n")
            .unwrap();
        assert_eq!(
            presets_source_stamp(root),
            stamp,
            "生成产物变化不许冒充「源被外面改过」"
        );
    }

    /// **统一编辑事务的端到端判据**（2026-10-31）：走一遍真正的编辑包装器
    /// （`run_ctx`，与 IPC 命令同一条路），证明——
    ///
    /// 1. 编辑只写**影子树**，真仓库一个字节不动；
    /// 2. 它进了事务（状态栏那个 N）；
    /// 3. 「放弃」把影子树拉回磁盘原状。
    #[test]
    fn an_edit_writes_only_the_shadow_tree_and_lands_in_the_transaction() {
        use crate::presetdata::MachineField;

        let dir = tempfile::tempdir().unwrap();
        let (fx_dir, presets) = Fixture::load().into_parts();
        let real = presets.root().to_path_buf();
        let shadow = crate::workbench::domain::txn::shadow_root(&dir.path().join("workbench"));
        std::fs::create_dir_all(&shadow).unwrap();
        crate::workbench::domain::txn::mirror(&real, &shadow).unwrap();

        let store = Store::at(dir.path().join("workbench"));
        store.bootstrap().unwrap();
        let shadow_presets = crate::presetdata::Presets::load_from(&shadow).unwrap();
        let mut ctx = Ctx::with(shadow_presets, store).unwrap();
        ctx.real_root = real.clone();
        ctx.shadow_root = shadow.clone();

        let rel = "machines/A1.toml";
        let real_before = std::fs::read(real.join(rel)).unwrap();

        run_ctx(&mut ctx, Some("改机型信息"), |ctx| {
            ctx.presets
                .catalog
                .machine_mut("A1")?
                .set_field(MachineField::Display, Some("A1 改名"))?;
            ctx.presets.catalog.write_machine("A1")?;
            Ok(())
        })
        .unwrap();

        /* 这一笔是**手写闭包**直接改数据（不经命令 ⇒ 没填 `pending_fields`），
        所以字段账是 0 —— 这正是设计：没有字段记录就不虚构处数。
        "编辑进没进事务"看的是撤销栈与字节差，看字段数会把它判错。 */
        assert_eq!(ctx.txn.undo_depth(), 1, "编辑必须进事务（一条撤销）");
        assert!(ctx.txn.is_dirty(), "影子树与磁盘不同 ⇒ 脏");
        assert_eq!(
            std::fs::read(real.join(rel)).unwrap(),
            real_before,
            "编辑只许写影子树，真仓库一个字节都不能动"
        );
        assert!(
            std::fs::read_to_string(shadow.join(rel))
                .unwrap()
                .contains("A1 改名"),
            "影子树里应该是改后的那一份"
        );

        // 放弃 = 影子树拉回磁盘原状
        crate::workbench::domain::txn::mirror(&ctx.real_root, &ctx.shadow_root).unwrap();
        ctx.txn.clear();
        assert!(
            !std::fs::read_to_string(shadow.join(rel))
                .unwrap()
                .contains("A1 改名"),
            "放弃之后影子树必须回到磁盘那一份"
        );
        drop(fx_dir);
    }

    /// **2026-11-01 悬案的正解判据**：从磁盘开一个会话之后，编辑只能落在影子树。
    ///
    /// 现场：22:17 换位之后保存不亮、22:22 真仓库两份文件被无日志写入。
    /// 根因是 `Ctx::open` 在"干净"时把 `Presets` 读到了**真仓库**上 ——
    /// 于是 `write_back` 直接改仓库，而 `run_ctx` 拍的是影子快照，差集恒空
    /// （dirty = 0 ⇒ 保存不亮、放弃退不回）。
    ///
    /// 这条走**真入口**（`Ctx::open_at` + `run_ctx` + `discard_in` / `save_in`），
    /// 验的是盘上的字节，不是文案：
    ///   ① 刚开（没有草稿）时会话的根必须是影子树；
    /// **判据 11**（刀 3）：开会话之后外部改了 `presets/`，点「放弃」必须回到
    /// **打开那一刻**，不是回到磁盘现在 —— 后者会把别人的改动当成自己的编辑一起留下。
    #[test]
    fn discard_returns_to_the_opening_snapshot_not_the_current_disk() {
        let (_fx_dir, presets) = Fixture::load().into_parts();
        let real = presets.root().to_path_buf();
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = Ctx::open_at(Store::at(dir.path().join("workbench")), real.clone()).unwrap();

        run_ctx(&mut ctx, Some("改值"), |c| {
            c.presets
                .set_variant("wiping.mode", "A1", &serde_json::json!("disk"))
        })
        .unwrap();

        /* 会话进行中，外部（git checkout / AI 改文件 / 另一个终端）动了真仓库 */
        let rel = "machines/A1.toml";
        let mut outside = std::fs::read_to_string(real.join(rel)).unwrap();
        outside.push_str("\n# 外部改动\n");
        crate::fsx::atomic::atomic_write(&real.join(rel), outside.as_bytes()).unwrap();

        super::discard_in(&mut ctx).unwrap();

        let shadow_text = std::fs::read_to_string(ctx.shadow_root.join(rel)).unwrap();
        assert!(
            !shadow_text.contains("外部改动"),
            "放弃回到的是「打开那一刻」，不许把中途的外部改动吸进来：{shadow_text}"
        );
    }

    /// **判据 18**（刀 6）：会话期间磁盘被外部改过 ⇒ 保存**不许静默覆盖**。
    ///
    /// 2026-10-31「A1 三份 0.7 被写回 0.8」就是这条没拦。
    #[test]
    fn saving_over_an_externally_changed_presets_is_refused() {
        let (_fx_dir, presets) = Fixture::load().into_parts();
        let real = presets.root().to_path_buf();
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = Ctx::open_at(Store::at(dir.path().join("workbench")), real.clone()).unwrap();

        run_ctx(&mut ctx, Some("改值"), |c| {
            c.presets
                .set_variant("wiping.mode", "A1", &serde_json::json!("disk"))
        })
        .unwrap();

        /* 外部改了真仓库 —— 与会话里那份已经不一样了 */
        let rel = "machines/A1.toml";
        let mut outside = std::fs::read_to_string(real.join(rel)).unwrap();
        outside.push_str("\n# 外部改动\n");
        crate::fsx::atomic::atomic_write(&real.join(rel), outside.as_bytes()).unwrap();

        assert!(
            super::save_in(&mut ctx).is_err(),
            "磁盘被外部改过时必须拒绝保存，不许静默覆盖"
        );
        assert!(
            std::fs::read_to_string(real.join(rel))
                .unwrap()
                .contains("外部改动"),
            "被拒绝的保存不许在真仓库留下任何痕迹"
        );
    }

    /// **判据 17**（刀 5）：保存停在「磁盘已换、基准未搬」⇒ 下次打开补完基准，
    /// 不许变成「保存了却提示有未保存改动」（那会让人点「放弃」把成果退回去）。
    #[test]
    fn an_interrupted_save_does_not_look_like_unsaved_work() {
        let (_fx_dir, presets) = Fixture::load().into_parts();
        let real = presets.root().to_path_buf();
        let dir = tempfile::tempdir().unwrap();
        let wb = dir.path().join("workbench");

        {
            let mut ctx = Ctx::open_at(Store::at(wb.clone()), real.clone()).unwrap();
            run_ctx(&mut ctx, Some("改值"), |c| {
                c.presets
                    .set_variant("wiping.mode", "A1", &serde_json::json!("disk"))
            })
            .unwrap();

            /* 只做到 commit 为止 —— 这正是「①成功、②之前崩」的形状 */
            let stamp = crate::workbench::clock::now_iso8601().replace(':', "-");
            txn::commit(&ctx.real_root, &ctx.shadow_root, &wb.join(".trash"), &stamp).unwrap();
            txn::write_save_stage(&wb, txn::SaveStage::RealCommitted).unwrap();
        }

        assert_eq!(
            txn::read_save_stage(&wb),
            txn::SaveStage::RealCommitted,
            "现场造对了：阶段停在 real_committed"
        );

        // 重开：基准必须被补完成磁盘那一份
        let ctx2 = Ctx::open_at(Store::at(wb.clone()), real.clone()).unwrap();
        assert_eq!(
            txn::read_save_stage(&wb),
            txn::SaveStage::Idle,
            "补完之后阶段要清掉"
        );
        assert!(
            txn::equal(
                &txn::snapshot(&real).unwrap(),
                &txn::snapshot(&txn::base_root(&wb)).unwrap()
            ),
            "基准必须跟上磁盘 —— 否则下次打开会判成「有未保存改动」"
        );
        assert_eq!(ctx2.txn.dirty_count(), 0, "不许凭空多出一条未完成事务");
    }

    /// **端到端判据 1/2/3**（刀 1）：打开会话改一格值，界面拿到的那格必须带
    /// 「保存前那一个」，且改第二次不跟着跑、保存后消失。
    ///
    /// 走的是真机同一条链：`Ctx::open_at` → 写影子 → `Book::with_rest(&ctx.base)`。
    /// **不走这条就测不出 `rest` 读的是哪一份** —— 单元测试里两份数据是手搓的，
    /// 而这里 `ctx.base` 由 `settle_shadow` 真正铺出来。
    #[test]
    fn touching_a_value_makes_the_saved_value_visible_to_the_ui() {
        let (_fx_dir, presets) = Fixture::load().into_parts();
        let real = presets.root().to_path_buf();
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at(dir.path().join("workbench"));
        let mut ctx = Ctx::open_at(store, real.clone()).unwrap();

        let cols = vec![crate::workbench::domain::derive::ColRef {
            machine_id: "A1".to_owned(),
            version_uid: None,
        }];
        let cell_of = |ctx: &Ctx| {
            crate::workbench::domain::derive::Book::new(&ctx.presets, &ctx.committed, &ctx.draft)
                .with_rest(&ctx.base)
                .matrix(&cols, Some("wiping"), "", None)
                .rows
                .iter()
                .find(|r| r.key == "wiping.mode")
                .map(|r| r.cells[0].clone())
                .expect("wiping.mode 该在 wiping 组里")
        };

        assert!(!cell_of(&ctx).dirty, "刚打开是干净的");

        run_ctx(&mut ctx, Some("改值"), |c| {
            c.presets
                .set_variant("wiping.mode", "A1", &serde_json::json!("disk"))
        })
        .unwrap();
        let first = cell_of(&ctx);
        assert!(
            first.dirty,
            "改过必须标脏 —— 不脏就是徽章不出现（2026-10-31 ~ 11-02 的老病）"
        );
        assert_eq!(first.raw, serde_json::json!("disk"), "格子显示改完的值");

        run_ctx(&mut ctx, Some("再改"), |c| {
            c.presets
                .set_variant("wiping.mode", "A1", &serde_json::json!("wipe_tower"))
        })
        .unwrap();
        let again = cell_of(&ctx);
        assert_eq!(
            again.rest, first.rest,
            "改第二次，「改之前的值」不许变成上一个值"
        );
        assert_eq!(again.rest_pinned, first.rest_pinned);

        // 保存：新值成为新基准 ⇒ 徽章消失
        super::save_in(&mut ctx).unwrap();
        let saved = cell_of(&ctx);
        assert!(
            !saved.dirty,
            "保存后徽章必须消失（`Ctx::base` 要跟着搬，否则会一直亮着）"
        );
    }

    ///   ② 编辑之后真仓库一个字节不动，事务 +1；
    ///   ③ 放弃 → 影子回到磁盘原状、事务清零；
    ///   ④ 再改一次并保存 → 这时真仓库才变。
    #[test]
    fn a_session_opened_from_disk_edits_the_shadow_until_save() {
        use crate::presetdata::MachineField;

        let (fx_dir, presets) = Fixture::load().into_parts();
        let real = presets.root().to_path_buf();
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at(dir.path().join("workbench"));
        let mut ctx = Ctx::open_at(store, real.clone()).unwrap();

        assert_eq!(
            ctx.presets.root(),
            ctx.shadow_root.as_path(),
            "会话必须读影子树 —— 读到真仓库就等于边改边落盘"
        );

        let rel = "machines/A1.toml";
        let real_before = std::fs::read(real.join(rel)).unwrap();
        let rename = |ctx: &mut Ctx| -> Result<(), AppError> {
            ctx.presets
                .catalog
                .machine_mut("A1")?
                .set_field(MachineField::Display, Some("A1 改名"))?;
            ctx.presets.catalog.write_machine("A1")
        };

        run_ctx(&mut ctx, Some("改机型信息"), rename).unwrap();
        /* 手写闭包不经命令 ⇒ 没填字段账。"进没进事务"看撤销栈（`dirty_count` 是
        显示用的字段数，这一笔没字段可数 —— 见 `Txn::is_dirty` 的注）。 */
        assert_eq!(
            ctx.txn.undo_depth(),
            1,
            "编辑必须进事务 —— 否则右上角那颗「保存」永远不亮"
        );
        assert_eq!(
            std::fs::read(real.join(rel)).unwrap(),
            real_before,
            "未保存前真仓库一个字节都不许动"
        );
        assert!(
            std::fs::read_to_string(ctx.shadow_root.join(rel))
                .unwrap()
                .contains("A1 改名"),
            "改后的字节要在影子树里"
        );

        discard_in(&mut ctx).unwrap();
        assert_eq!(ctx.txn.dirty_count(), 0, "放弃之后事务清零");
        assert!(
            !std::fs::read_to_string(ctx.shadow_root.join(rel))
                .unwrap()
                .contains("A1 改名"),
            "放弃必须真退回磁盘那一份"
        );
        assert_eq!(
            std::fs::read(real.join(rel)).unwrap(),
            real_before,
            "放弃之后真仓库还是原样"
        );

        run_ctx(&mut ctx, Some("改机型信息"), rename).unwrap();
        save_in(&mut ctx).unwrap();
        assert!(
            std::fs::read_to_string(real.join(rel))
                .unwrap()
                .contains("A1 改名"),
            "保存才落真仓库"
        );
        assert_eq!(ctx.txn.dirty_count(), 0, "保存之后事务清零");
        drop(fx_dir);
    }

    /* ---------- 2026-11-03：值编辑只落内存（新编辑器模型） ---------- */

    /// **改一个值，一个字节都不写文件**，但撤销 / 重做立刻可用、保存才落盘。
    ///
    /// 这是新模型最核心的判据（`docs/WORKBENCH-EDITOR-MODEL.md` §6）：
    ///
    /// ```text
    /// 改值   →  影子树 mtime 不变 + 真仓库不变 + 撤销栈 +1
    /// 撤销   →  值回原样，影子树仍然不变
    /// 重做   →  值又变成新的，影子树仍然不变
    /// 保存   →  这一次才写：影子树 ←→ 真仓库都出现新值
    /// ```
    #[test]
    fn a_value_edit_never_touches_a_file_until_saved() {
        let (fx_dir, presets) = Fixture::load().into_parts();
        let real = presets.root().to_path_buf();
        let rel = "registry/param_registry.toml";
        let real_before = std::fs::read(real.join(rel)).unwrap();

        let dir = tempfile::tempdir().unwrap();
        let store = Store::at(dir.path().join("workbench"));
        let mut ctx = Ctx::open_at(store, real.clone()).unwrap();
        let shadow_before = std::fs::read(ctx.shadow_root.join(rel)).unwrap();

        /* ① 改值 */
        /* 用 `offset.x`：夹具里 `A1:STANDARD` 在它上面本来就钉着 -1（不跟随），
        改它才有"真正的变化"。`offset.z` 在夹具里只有 `A1` 的裸键，
        给它写版本值会同时翻跟随状态 —— 那条路是另一个测试的事。 */
        let patch = Patch::SetValue {
            level: Level::Version,
            owner: "A1/STANDARD".to_owned(),
            key: "toolhead.offset.x".to_owned(),
            value: Some(serde_json::json!(9.5)),
        };
        let out = apply_patches(
            &mut ctx.draft,
            &ctx.committed,
            &ctx.presets.registry,
            std::slice::from_ref(&patch),
        )
        .unwrap();
        let staged = storage::stage_value_edits(&ctx.presets, &ctx.committed, &ctx.draft);
        assert!(
            !staged.is_empty(),
            "夹具里这一格本来钉着 -1，改成 9.5 必须算出真实改动；staged={:?}",
            staged
        );
        ctx.history.record(
            "A1 · 标准版 · X 轴偏移".to_owned(),
            edit_history::merge_key_of(&patch),
            patch.clone(),
            out.inverse,
            Vec::new(),
        );
        storage::stage_into(&mut ctx.presets, &staged).unwrap();
        ctx.draft = Draft::default();

        assert_eq!(
            ctx.presets
                .registry
                .param("toolhead.offset.x")
                .unwrap()
                .machine_variants
                .get("A1:STANDARD")
                .cloned(),
            Some(serde_json::json!(9.5)),
            "内存文档里已经是新值"
        );
        assert_eq!(
            std::fs::read(ctx.shadow_root.join(rel)).unwrap(),
            shadow_before,
            "★ 改值不许写影子树（这正是作者说的「改个数字怎么这么重」）"
        );
        assert_eq!(
            std::fs::read(real.join(rel)).unwrap(),
            real_before,
            "真仓库更不许动"
        );
        assert_eq!(ctx.history.undo_depth(), 1, "撤销栈 +1");

        /* ② 撤销：值回原样，仍然不写文件 */
        undo_in(&mut ctx).unwrap();
        assert_eq!(
            ctx.presets
                .registry
                .param("toolhead.offset.x")
                .unwrap()
                .machine_variants
                .get("A1:STANDARD")
                .cloned(),
            Some(serde_json::json!(-1.0)),
            "撤销把值退回去了"
        );
        assert_eq!(
            std::fs::read(ctx.shadow_root.join(rel)).unwrap(),
            shadow_before,
            "撤销不许写影子树"
        );

        /* ③ 重做：值又变成新的 */
        redo_in(&mut ctx).unwrap();
        assert_eq!(
            ctx.presets
                .registry
                .param("toolhead.offset.x")
                .unwrap()
                .machine_variants
                .get("A1:STANDARD")
                .cloned(),
            Some(serde_json::json!(9.5)),
            "重做把值又改回去了"
        );

        /* ④ 保存：这一次才写 */
        save_in(&mut ctx).unwrap();
        let committed = crate::presetdata::Presets::load_from(&real).expect("重读真仓库");
        assert_eq!(
            committed
                .registry
                .param("toolhead.offset.x")
                .unwrap()
                .machine_variants
                .get("A1:STANDARD")
                .cloned(),
            Some(serde_json::json!(9.5)),
            "保存之后真仓库里是那个新值"
        );
        assert_eq!(ctx.history.undo_depth(), 0, "保存之后历史清空");

        drop(fx_dir);
    }

    /// **两套栈并存时，撤销必须退"最近的那一笔"**（2026-11-03 迁移期回归）。
    ///
    /// 迁移期：值编辑在 `ctx.history`（内存命令），结构编辑还在 `ctx.txn`（文件事务）。
    /// 谁后发生退谁 —— 要是按栈名轮流退，撤销顺序会与真实顺序相反：
    /// 「改个参数 → 改机型信息」之后按 Cmd+Z，退掉的却是参数那一笔。
    #[test]
    fn undo_across_the_two_stacks_goes_by_recency() {
        let (fx_dir, presets) = Fixture::load().into_parts();
        let real = presets.root().to_path_buf();
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at(dir.path().join("workbench"));
        let mut ctx = Ctx::open_at(store, real.clone()).unwrap();

        /* ① 值编辑（进 history） */
        let patch = Patch::SetValue {
            level: Level::Version,
            owner: "A1/STANDARD".to_owned(),
            key: "toolhead.offset.x".to_owned(),
            value: Some(serde_json::json!(7.0)),
        };
        let out = apply_patches(
            &mut ctx.draft,
            &ctx.committed,
            &ctx.presets.registry,
            std::slice::from_ref(&patch),
        )
        .unwrap();
        let staged = storage::stage_value_edits(&ctx.presets, &ctx.committed, &ctx.draft);
        ctx.history.record(
            "改值".to_owned(),
            edit_history::merge_key_of(&patch),
            patch,
            out.inverse,
            Vec::new(),
        );
        storage::stage_into(&mut ctx.presets, &staged).unwrap();
        ctx.draft = Draft::default();
        assert_eq!(ctx.history.undo_depth(), 1, "值编辑进历史");

        /* ② 结构编辑（进 txn）：改机型显示名 */
        use crate::presetdata::MachineField;
        let rename = |ctx: &mut Ctx| -> Result<(), AppError> {
            ctx.presets
                .catalog
                .machine_mut("A1")?
                .set_field(MachineField::Display, Some("A1 改名"))?;
            ctx.presets.catalog.write_machine("A1")
        };
        run_ctx(&mut ctx, Some("改机型信息"), rename).unwrap();
        assert_eq!(ctx.txn.undo_depth(), 1, "结构编辑进事务栈");

        /* ③ 撤销一次 ⇒ **退的是机型信息**（最近那笔），参数的改动还在 */
        undo_in(&mut ctx).unwrap();
        assert_eq!(
            ctx.presets.catalog.machine("A1").unwrap().display,
            "A1",
            "撤销退掉的是最近那笔（机型信息）"
        );
        assert_eq!(
            ctx.presets
                .registry
                .param("toolhead.offset.x")
                .unwrap()
                .machine_variants
                .get("A1:STANDARD")
                .cloned(),
            Some(serde_json::json!(7.0)),
            "参数那一笔还没撤（它更早）"
        );

        /* ④ 再撤销一次 ⇒ 这才轮到参数 */
        undo_in(&mut ctx).unwrap();
        assert_eq!(
            ctx.presets
                .registry
                .param("toolhead.offset.x")
                .unwrap()
                .machine_variants
                .get("A1:STANDARD")
                .cloned(),
            Some(serde_json::json!(-1.0)),
            "第二次撤销退回参数那一笔"
        );

        drop(fx_dir);
    }

    /// **结构命令的撤销 = 把那一份文件的 `before` 字节写回去**（阶段 3 回归）。
    ///
    /// 一条命令 = 一个对象一个文件的前后字节；撤销退的是**这一条**，不碰别的文件。
    #[test]
    fn a_struct_command_undo_restores_just_that_file() {
        let (fx_dir, presets) = Fixture::load().into_parts();
        let real = presets.root().to_path_buf();
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at(dir.path().join("workbench"));
        let mut ctx = Ctx::open_at(store, real.clone()).unwrap();

        use crate::presetdata::MachineField;
        /* ① 改机型显示名 */
        run_struct(
            &mut ctx,
            "改机型信息",
            "display",
            |_| vec!["machines/A1.toml".to_owned()],
            |ctx| {
                ctx.presets
                    .catalog
                    .machine_mut("A1")?
                    .set_field(MachineField::Display, Some("A1 改名"))?;
                ctx.presets.catalog.write_machine("A1")
            },
        )
        .unwrap();
        assert_eq!(ctx.history.undo_depth(), 1, "结构命令进了历史");
        assert_eq!(
            ctx.presets.catalog.machine("A1").unwrap().display,
            "A1 改名"
        );

        /* ② 撤销 ⇒ 显示名回原样 */
        undo_in(&mut ctx).unwrap();
        assert_eq!(
            ctx.presets.catalog.machine("A1").unwrap().display,
            "A1",
            "撤销退掉的是这一条（那一份文件的 before 字节）"
        );
        assert_eq!(ctx.history.undo_depth(), 0);

        /* ③ 重做 ⇒ 又是改过的名字 */
        redo_in(&mut ctx).unwrap();
        assert_eq!(
            ctx.presets.catalog.machine("A1").unwrap().display,
            "A1 改名"
        );

        drop(fx_dir);
    }

    /// **新建机型撤销 ⇒ 那个文件被删掉**（`before` 为 `None` = 这一笔新建了它）
    #[test]
    fn undoing_a_new_machine_removes_its_file() {
        let (fx_dir, presets) = Fixture::load().into_parts();
        let real = presets.root().to_path_buf();
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at(dir.path().join("workbench"));
        let mut ctx = Ctx::open_at(store, real.clone()).unwrap();

        run_struct(
            &mut ctx,
            "新增机型",
            "",
            |_| vec!["machines/NEW.toml".to_owned()],
            |ctx| {
                ctx.presets
                    .catalog
                    .add_machine("NEW", "BAMBU_LAB", "新机器")?;
                ctx.presets.catalog.write_machine("NEW")
            },
        )
        .unwrap();
        assert!(ctx.shadow_root.join("machines/NEW.toml").exists());

        undo_in(&mut ctx).unwrap();
        assert!(
            !ctx.shadow_root.join("machines/NEW.toml").exists(),
            "新建的那一份撤销时要删掉"
        );

        drop(fx_dir);
    }

    /// **机器页的结构编辑不许冲掉内存里的值编辑**（2026-11-03 换代回归）。
    ///
    /// 值编辑只落内存；结构编辑会 `reload()` 重建内存文档。重建完不重放历史，
    /// 改一下机型显示名，参数那一笔就静默没了。
    #[test]
    fn a_machine_field_edit_does_not_wipe_value_edits() {
        let (fx_dir, presets) = Fixture::load().into_parts();
        let real = presets.root().to_path_buf();
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at(dir.path().join("workbench"));
        let mut ctx = Ctx::open_at(store, real.clone()).unwrap();

        /* 值编辑（进 history，只落内存） */
        let patch = Patch::SetValue {
            level: Level::Version,
            owner: "A1/STANDARD".to_owned(),
            key: "toolhead.offset.x".to_owned(),
            value: Some(serde_json::json!(6.5)),
        };
        let out = apply_patches(
            &mut ctx.draft,
            &ctx.committed,
            &ctx.presets.registry,
            std::slice::from_ref(&patch),
        )
        .unwrap();
        let staged = storage::stage_value_edits(&ctx.presets, &ctx.committed, &ctx.draft);
        ctx.history.record(
            "改值".to_owned(),
            edit_history::merge_key_of(&patch),
            patch,
            out.inverse,
            Vec::new(),
        );
        storage::stage_into(&mut ctx.presets, &staged).unwrap();
        ctx.draft = Draft::default();

        /* 结构编辑：改机型显示名（这条会 reload + 重放） */
        use crate::presetdata::MachineField;
        run_ctx(&mut ctx, Some("改机型信息"), |ctx| {
            ctx.presets
                .catalog
                .machine_mut("A1")?
                .set_field(MachineField::Display, Some("A1 改名"))?;
            ctx.presets.catalog.write_machine("A1")
        })
        .unwrap();

        assert_eq!(
            ctx.presets
                .registry
                .param("toolhead.offset.x")
                .unwrap()
                .machine_variants
                .get("A1:STANDARD")
                .cloned(),
            Some(serde_json::json!(6.5)),
            "改机型显示名之后，值编辑还必须在"
        );
        assert_eq!(
            ctx.presets.catalog.machine("A1").unwrap().display,
            "A1 改名",
            "结构编辑本身生效"
        );

        drop(fx_dir);
    }

    /// **悬案原样重放**（2026-11-01 22:17）：从磁盘开会话 → 抽屉里换位 →
    /// 事务 +1（保存才亮），而真仓库那两份文件一个字节都不动。
    ///
    /// 与上一条的分工：上一条验"换位本身会不会记账"，这一条验"**会话根对不对**"。
    /// 两个条件缺一不可 —— 根错了，换位就写进真仓库，而事务拍的是影子，永远记 0。
    #[test]
    fn a_group_move_from_a_fresh_session_only_touches_the_shadow() {
        use crate::presetdata::registry::{SectionMetaEdit, SectionMove};

        let (fx_dir, presets) = Fixture::load().into_parts();
        let real = presets.root().to_path_buf();
        let registry_rel = "registry/param_registry.toml";
        let layout_rel = "layout_schema.toml";
        let real_registry = std::fs::read(real.join(registry_rel)).unwrap();
        let real_layout = std::fs::read(real.join(layout_rel)).unwrap();

        let dir = tempfile::tempdir().unwrap();
        let store = Store::at(dir.path().join("workbench"));
        let mut ctx = Ctx::open_at(store, real.clone()).unwrap();

        // 夹具的 wiping 下只有一个组：先建出第二组，才有"邻居"可换
        run_ctx(&mut ctx, Some("改分组名与位置"), |ctx| {
            ctx.presets
                .registry
                .ensure_section("wiping", "custom_1", "支撑")?;
            ctx.presets.registry.write_back()?;
            Ok(())
        })
        .expect("建组这一步不能失败");

        let before = ctx.txn.dirty_count();
        run_ctx(&mut ctx, Some("改分组名与位置"), |ctx| {
            ctx.presets.registry.set_section_meta(
                "custom_1",
                SectionMetaEdit {
                    label: None,
                    move_dir: Some(SectionMove::Up),
                    new_section: None,
                },
            )?;
            ctx.presets.registry.write_back()?;
            Ok(())
        })
        .expect("挪这一步不能失败");

        assert!(
            ctx.txn.undo_depth() > before,
            "换位没进事务 —— 右上角那颗「保存」就不会亮"
        );
        let ids: Vec<&str> = ctx.presets.registry.tabs()[1]
            .sections
            .iter()
            .map(|s| s.id.as_str())
            .collect();
        assert_eq!(ids, ["custom_1", "wipe"], "内存里的次序没换");
        assert_eq!(
            std::fs::read(real.join(registry_rel)).unwrap(),
            real_registry,
            "换位不许写进真仓库的注册表"
        );
        assert_eq!(
            std::fs::read(real.join(layout_rel)).unwrap(),
            real_layout,
            "换位不许写进真仓库的布局"
        );

        // 放弃 → 影子退回原状、事务清零
        discard_in(&mut ctx).unwrap();
        assert_eq!(ctx.txn.dirty_count(), 0);
        assert!(
            !ctx.presets
                .registry
                .tabs()
                .iter()
                .any(|t| t.sections.iter().any(|s| s.id == "custom_1")),
            "放弃之后新建的那一组要消失"
        );
        drop(fx_dir);
    }

    /// **分组换位也要进事务，并且真的换掉影子树里的次序**（2026-11-01 现场：
    /// 抽屉里点上下箭头，保存按钮不亮、列表不动）。
    ///
    /// 这条走的是命令那条路（`run_ctx` + `EDIT_COMMANDS` 里的标签），所以它红了
    /// 就等于「真机上点箭头不会有任何反应」—— 反过来，它绿了还点不动，问题就在
    /// 前端或用户手里那个还没重编的二进制。
    #[test]
    fn moving_a_group_lands_in_the_transaction_and_reorders_the_shadow() {
        use crate::presetdata::registry::SectionMetaEdit;
        use crate::presetdata::registry::SectionMove;

        let (_dirs, _f, mut ctx) = ctx();
        /* 夹具的 wiping 下本来只有一个 `wipe` —— 先建出第二组，才有"邻居"可换 */
        run_ctx(&mut ctx, Some("改分组名与位置"), |ctx| {
            ctx.presets
                .registry
                .ensure_section("wiping", "custom_1", "支撑")?;
            ctx.presets.registry.write_back()?;
            Ok(())
        })
        .expect("建组这一步不能失败");

        let before = ctx.txn.dirty_count();
        run_ctx(&mut ctx, Some("改分组名与位置"), |ctx| {
            ctx.presets.registry.set_section_meta(
                "custom_1",
                SectionMetaEdit {
                    label: None,
                    move_dir: Some(SectionMove::Up),
                    new_section: None,
                },
            )?;
            ctx.presets.registry.write_back()?;
            Ok(())
        })
        .expect("挪这一步不能失败");

        assert!(
            ctx.txn.undo_depth() > before,
            "换位没进事务 —— 右上角那颗「保存」就不会亮"
        );
        let ids: Vec<&str> = ctx.presets.registry.tabs()[1]
            .sections
            .iter()
            .map(|s| s.id.as_str())
            .collect();
        assert_eq!(ids, ["custom_1", "wipe"], "内存里的次序没换");

        /* 影子树上那份也换了（`write_back` 写的是它） */
        let shadow_reg =
            std::fs::read_to_string(ctx.shadow_root.join("registry/param_registry.toml"))
                .expect("影子树里要有注册表");
        let at = |id: &str| {
            shadow_reg
                .find(&format!("id = '{id}'"))
                .or_else(|| shadow_reg.find(&format!("id = \"{id}\"")))
                .unwrap_or_else(|| panic!("{id} 不见了"))
        };
        assert!(at("custom_1") < at("wipe"), "影子树里的数组次序没换");

        /*
         * **参数必须跟着自己的分组走**（2026-11-01 的回归点）：
         * 换位只能挪"块"，绝不能出现「A 的名字挂着 B 的参数」。
         * 夹具的 `wipe` 带 3 条参数、`custom_1` 是空的 —— 换完之后还得这样。
         */
        let again = crate::presetdata::Presets::load_from(&ctx.shadow_root)
            .expect("换完位这份数据还得读得通（`check_consistency` 会当场发现错位）");
        let wiping = again
            .registry
            .layout()
            .iter()
            .find(|t| t.id == "wiping")
            .expect("wiping 页签在");
        let counts: Vec<(&str, usize)> = wiping
            .sections
            .iter()
            .map(|s| (s.id.as_str(), s.items.len()))
            .collect();
        assert_eq!(
            counts,
            [("custom_1", 0), ("wipe", 3)],
            "换位把参数挪到了别的分组名下 —— `items` 必须整块跟着走"
        );
    }

    /// 造一个不碰真仓库、也不碰那个全局的 `Ctx`。
    ///
    /// 第一个返回值是**两个临时目录**：仓库那个，加上夹具那个。
    /// 夹具那个必须活到测试结束 —— `Ctx` 会往它的 `presets/` 里写（新建版本进清单）
    fn ctx() -> ((tempfile::TempDir, tempfile::TempDir), Fixture, Ctx) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at(dir.path());
        store.bootstrap().unwrap();
        let f = Fixture::load();
        // `Presets` 没有 Clone，所以再读一份给 Ctx
        let (fx_dir, presets) = Fixture::load().into_parts();
        let ctx = Ctx::with(presets, store).unwrap();
        ((dir, fx_dir), f, ctx)
    }

    /// **界面上不许出现「（跟随上一层）」**（2026-11-03 作者截图报的）。
    ///
    /// 那句话是 `logfmt::show(None)` 的**日志写法**，意思是"这一层把这个键删了，
    /// 值改由上一层提供"。挪进界面就成了「3.85 → （跟随上一层）」，用户看不懂 ——
    /// 界面上问的是"这一格从多少变成多少"，删键动作在模型里的意思是"变成基底那个值"。
    ///
    /// 这条测试把三种情况都过一遍：删键（'跟随'）、设值、以及基底展开。
    #[test]
    fn the_field_table_never_prints_the_log_phrase() {
        let ((_d, _f), _fx, ctx) = ctx();
        let Some(param) = ctx.presets.registry.param("wiping.child") else {
            return;
        };
        let Some(m) = ctx.presets.catalog.machines().first() else {
            return;
        };
        let (mid, first_ver) = (m.id.clone(), m.versions[0].id.clone());

        /* ① 删键动作：`Some → None`，界面上必须展开成具体数字 */
        let del = txn::FieldEntry {
            level: Some("Version".to_owned()),
            owner: format!("{mid}:{first_ver}"),
            key: "wiping.child".to_owned(),
            label: param.label.clone(),
            before: Some("3".to_owned()),
            /* 故意塞日志写法：这是上游以前的真实输出，必须在这里被换成数字 */
            after: Some("（跟随上一层）".to_owned()),
            why: "user",
        };
        /* ② 基底改动：会展开出一圈 follows */
        let base = txn::FieldEntry {
            level: Some("Machine".to_owned()),
            owner: mid.clone(),
            key: "wiping.child".to_owned(),
            label: param.label.clone(),
            before: Some("1".to_owned()),
            after: Some("2".to_owned()),
            why: "user",
        };

        for e in fields_after_model(&ctx, &[del, base]) {
            for text in [&e.before, &e.after].into_iter().flatten() {
                assert!(
                    !text.contains("跟随上一层"),
                    "界面上不该出现日志写法：{} · {:?}",
                    e.owner,
                    text
                );
            }
        }
    }

    /// **同一个行为只有一个标签**（2026-11-03 作者：「mini 的和 a1 的文案怎么不一样」）。
    ///
    /// 以前改基底时，A1 走 `inherited`（文件里没有版本键）、A1_MINI 走 `promoted`
    /// （文件里留着版本键）—— 同一件事两种说法，只因为文件长得不一样。
    /// 现在统一成 `follows`：**标签说的是行为，不是存储**。
    #[test]
    fn every_version_that_follows_the_base_gets_the_same_label() {
        let ((_d, _f), _fx, ctx) = ctx();
        let mut seen: Vec<(String, String)> = Vec::new();
        for m in ctx.presets.catalog.machines() {
            let base = txn::FieldEntry {
                level: Some("Machine".to_owned()),
                owner: m.id.clone(),
                key: "wiping.child".to_owned(),
                label: "擦料次数".to_owned(),
                before: Some("1".to_owned()),
                after: Some("2".to_owned()),
                why: "user",
            };
            for e in fields_after_model(&ctx, &[base]) {
                if e.level.as_deref() == Some("Version") {
                    seen.push((m.id.clone(), e.why.to_owned()));
                }
            }
        }
        let labels: std::collections::BTreeSet<&str> =
            seen.iter().map(|(_, w)| w.as_str()).collect();
        assert!(
            labels.len() <= 1,
            "所有机型的连带项必须用同一个标签，实际 {labels:?}"
        );
        if let Some(l) = labels.iter().next() {
            assert_eq!(*l, "follows", "标签应当统一成 follows");
        }
    }

    /// **验收：同样操作在不同机型上必须产生同样的有效变化**
    /// （2026-11-03 作者拍板的口径）。
    ///
    /// 作者原话：「同样的继承关系、同样的修改操作，不应该因为机型名称不同而
    /// 产生不同的行为」。之前在真数据上就不是：
    /// `toolhead.offset.z` 的 `machineVariants` 里 A1 只有基底一行、A1_MINI 还留着
    /// 三条与基底同值的版本键 —— 写值路径据此写出不同的键数（1 个 vs 4 个）。
    ///
    /// 现在展开只看**模型**（版本有没有自己那一行），所以**只要继承关系相同，
    /// 展开出来的条数就相同** —— 与机型叫什么、文件里有没有冗余键无关。
    /// 这条测试拿夹具里的两台机型各算一遍，比这个数。
    #[test]
    fn the_same_edit_behaves_the_same_on_every_machine() {
        let ((_d, _f), _fx, ctx) = ctx();
        let key = "wiping.child";
        let Some(param) = ctx.presets.registry.param(key) else {
            return;
        };

        /* 每台机型：基底 1 条 + "没有自己那一行"的版本各 1 条 */
        let mut counts = Vec::new();
        for m in ctx.presets.catalog.machines() {
            let inherited: Vec<String> = m
                .versions
                .iter()
                .filter(|v| {
                    !param
                        .machine_variants
                        .contains_key(&format!("{}:{}", m.id, v.id))
                })
                .map(|v| v.id.clone())
                .collect();
            let base = txn::FieldEntry {
                level: Some("Machine".to_owned()),
                owner: m.id.clone(),
                key: key.to_owned(),
                label: param.label.clone(),
                before: Some("1".to_owned()),
                after: Some("2".to_owned()),
                why: "user",
            };
            let all = {
                let mut v = vec![base.clone()];
                v = fields_after_model(&ctx, &v);
                v
            };
            assert_eq!(
                all.len(),
                1 + inherited.len(),
                "{}：总数 = 基底 + 继承项（继承 {} 个版本）",
                m.id,
                inherited.len()
            );
            counts.push((m.id.clone(), inherited.len(), all.len()));
        }
        assert!(!counts.is_empty(), "夹具里至少要有一台机型");

        /* ★ 真正要钉的：**继承数相同的机型，展开条数必须相同**。
        这一句就是作者那句"不该因为机型名称不同而行为不同"的可执行版本 ——
        与文件里有没有冗余版本键无关（那正是之前两个机型表现不一致的原因）。 */
        for a in &counts {
            for b in &counts {
                if a.1 == b.1 {
                    assert_eq!(
                        a.2, b.2,
                        "{} 与 {} 的继承版本数相同（{}），展开条数却不同",
                        a.0, b.0, a.1
                    );
                }
            }
        }
    }

    /// **有独立值的版本不被基底展开带进来**（2026-11-03）。
    ///
    /// 作者：「如果 STANDARD 原本有独立值，修改基底后它仍然是 3.70，不应该被算作变化」。
    #[test]
    fn a_version_with_its_own_value_is_not_swept_into_the_base_expansion() {
        let ((_d, _f), _fx, ctx) = ctx();
        let Some(param) = ctx.presets.registry.param("toolhead.offset.z") else {
            return;
        };
        /* 找一个**有独立版本键**的机型当反例；一个都没有就跳过（数据变了不算失败） */
        let with_own = ctx.presets.catalog.machines().iter().find_map(|m| {
            m.versions
                .iter()
                .any(|v| {
                    param
                        .machine_variants
                        .contains_key(&format!("{}:{}", m.id, v.id))
                })
                .then(|| m.id.clone())
        });
        let Some(mid) = with_own else {
            return;
        };

        let base = txn::FieldEntry {
            level: Some("Machine".to_owned()),
            owner: mid.clone(),
            key: "toolhead.offset.z".to_owned(),
            label: param.label.clone(),
            before: Some("1.1".to_owned()),
            after: Some("1.2".to_owned()),
            why: "user",
        };
        let all = fields_after_model(&ctx, &[base]);
        let total = ctx.presets.catalog.machine(&mid).unwrap().versions.len();
        assert!(
            all.len() < total,
            "{mid} 有独立值的版本不该被展开进来：展开了 {} / 共 {total}",
            all.len()
        );
    }

    /* ---------- 外部改动的发现（2026-10-20） ---------- */

    /// 盘上戳：**源定义改了要变；生成写的产物与资产载荷不算**。
    ///
    /// 这条判据是"外部改过 presets/ 就提示重新加载"的唯一依据 —— 它要是把
    /// `delivery/` 算进去，点一次生成就会让界面误报"外部改过定义"。
    #[test]
    fn source_stamp_tracks_definitions_not_products() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        std::fs::create_dir_all(root.join("registry")).unwrap();
        std::fs::create_dir_all(root.join("delivery/mkp/presets")).unwrap();
        std::fs::create_dir_all(root.join("assets/models")).unwrap();
        let registry = root.join("registry/param_registry.toml");
        crate::fsx::atomic::atomic_write(&registry, b"a = 1\n").unwrap();
        let before = presets_source_stamp(root);

        // 生成写的产物 + 二进制载荷：都不该动这个戳
        crate::fsx::atomic::atomic_write(
            &root.join("delivery/mkp/presets/A1-standard.toml"),
            b"x = 1\n",
        )
        .unwrap();
        crate::fsx::atomic::atomic_write(&root.join("assets/models/a.3mf"), b"binary").unwrap();
        assert_eq!(
            before,
            presets_source_stamp(root),
            "产物 / 载荷不该算进「源定义」"
        );

        // 改一个源定义：必须变（顺手改长度，避开 mtime 粒度）
        crate::fsx::atomic::atomic_write(&registry, b"a = 1234\n").unwrap();
        assert_ne!(
            before,
            presets_source_stamp(root),
            "源定义改了这个戳必须跟着变，否则外部改动永远发现不了"
        );
    }

    /* ---------- 参数正文复制（b05 Task 14.5，裁决 A：独立快照） ---------- */

    /// 算一个版本的有效配方（与 `copy_recipe` 内部同一套 digest + Layers）
    fn effective(
        p: &Presets,
        machine_id: &str,
        version_id: &str,
    ) -> BTreeMap<String, serde_json::Value> {
        let version_ids: Vec<String> = p
            .catalog
            .machine(machine_id)
            .expect("机型在")
            .versions
            .iter()
            .map(|v| v.id.clone())
            .collect();
        let digested = variants::digest(&p.registry, machine_id, &version_ids);
        let over = digested
            .versions
            .get(version_id)
            .cloned()
            .unwrap_or_default();
        Layers::new(&p.registry, machine_id, &digested.base, &over)
            .effective_recipe()
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.clone()))
            .collect()
    }

    /// **14.5 的两条验收**（裁决 A，2026-09-24）：
    /// ① 复制后两边有效配方一致；
    /// ② **改模板（版本层或机型基底）后新版本不变** —— 这是「独立快照」的决定性
    /// 证据：只复制自有覆盖的话，基底改动会传播过来，这条就会红。
    ///
    /// 顺带锁定 14.4 的翻转：新版本 `hasRecipe=false`（待补）→ 复制后 true
    #[test]
    fn copied_recipe_is_an_independent_snapshot() {
        let f = Fixture::load();
        let mut p = f.presets;

        // 14.3 的等价动作：先建版本定义（此刻它纯继承，hasRecipe = false）
        p.catalog
            .machine_mut("A1")
            .unwrap()
            .add_version("SNAPSHOT", "快照版")
            .unwrap();
        assert!(
            !p.registry.version_has_variants("A1:SNAPSHOT"),
            "新建版本没有显式键 —— 就是「参数源待补」"
        );

        // 复制：从模板 FAST 取完整有效配方
        let keys = copy_recipe(&mut p, "A1", "FAST", "SNAPSHOT").expect("复制");
        assert!(keys > 0, "模板的有效配方不应为空");
        assert!(
            p.registry.version_has_variants("A1:SNAPSHOT"),
            "复制后新版本有自己的显式键 —— 「待补」翻正"
        );

        // ① 复制后两边一致（逐键，值与键数都对）
        let template_eff = effective(&p, "A1", "FAST");
        let snapshot_eff = effective(&p, "A1", "SNAPSHOT");
        assert_eq!(template_eff.len(), snapshot_eff.len(), "键数一致");
        for (k, v) in &template_eff {
            assert_eq!(snapshot_eff.get(k), Some(v), "键 {k} 复制后两边不一致");
        }

        // ② 反向验证（版本层）：改模板的一个值 → 新版本不动
        let probe = "wiping.mode";
        let before = snapshot_eff.get(probe).expect("夹具有这个键").clone();
        let flipped = if before == "disk" { "tower" } else { "disk" };
        p.apply_values(&[(
            probe.to_owned(),
            "A1:FAST".to_owned(),
            Some(serde_json::json!(flipped)),
        )])
        .expect("改模板");
        assert_eq!(
            effective(&p, "A1", "FAST").get(probe),
            Some(&serde_json::json!(flipped)),
            "模板真的改了"
        );
        assert_eq!(
            effective(&p, "A1", "SNAPSHOT").get(probe),
            Some(&before),
            "快照版不该跟着模板动"
        );

        // ② 反向验证（**机型基底**）：改基底 → 模板跟着变（继承），快照版不动 ——
        // 快照版显式化了全部可见键，基底改动被它自己的值挡住。这是方案 A 与
        // 「只复制自有覆盖」的实质差异，也是裁决 A 要的独立演化
        let base_probe = "toolhead.offset.y";
        let snap_before = effective(&p, "A1", "SNAPSHOT").get(base_probe).cloned();
        p.apply_values(&[(
            base_probe.to_owned(),
            "A1".to_owned(),
            Some(serde_json::json!(99.5)),
        )])
        .expect("改基底");
        assert_eq!(
            effective(&p, "A1", "FAST").get(base_probe),
            Some(&serde_json::json!(99.5)),
            "模板继承基底，应该跟着变"
        );
        assert_eq!(
            effective(&p, "A1", "SNAPSHOT").get(base_probe),
            snap_before.as_ref(),
            "快照版被自己的显式值挡住，基底改动不传播"
        );

        // 「缺失参数不伪造」：复制写的键数 = 模板有效配方键数，
        // 没有为 registry 里不存在的键造值（apply_values 会拦未知键，到不了盘）
        assert_eq!(
            keys,
            template_eff.len(),
            "复制的键数就是模板有效配方的键数，一个不多一个不少"
        );
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
        let reg = &ctx.presets.registry;
        let params: Vec<ParamView> = reg.params().iter().map(|p| param_view(reg, p)).collect();

        let mode = params.iter().find(|p| p.key == "wiping.mode").unwrap();
        assert_eq!(mode.default_text, "擦料塔", "枚举要给中文选项名");
        assert_eq!(mode.choices.len(), 2);
        assert_eq!(mode.tab_id.as_deref(), Some("wiping"));

        let script = params.iter().find(|p| p.key == "toolhead.script").unwrap();
        assert_eq!(script.default_text, w::BLANK, "空串写「空」不是空白");

        let off = params
            .iter()
            .find(|p| p.key == "toolhead.offset.x")
            .unwrap();
        assert_eq!(off.default_text, "0.0 mm");
        assert_eq!(off.machine_filter.len(), 0, "不限机型");

        let only = params
            .iter()
            .find(|p| p.key == "toolhead.only_p1s")
            .unwrap();
        assert_eq!(only.machine_filter, vec!["P1S"]);
    }

    /// **「未保存 N」与「代码对比」是两个数，各说各的**（2026-11-03 改判）。
    ///
    /// 2026-11-02 一度把两者钉成同一个数（"页头写 1、框里写 2 谁也说不清"）。
    /// 2026-11-03 作者指出「改了很多还是只有 1」—— 钉在**文件数**上，一次改四行
    /// 永远只算 1。现在分开：页头那个数是**字段数**，框里那句话说的是**份**。
    ///
    /// 这条测试钉住"页头那个数不是文件数"：这里没人录事务 ⇒ 0，
    /// 而 `unsaved_files` / 框里那句仍然说 1 份（它们各答各的，不许互相顶替）。
    #[test]
    fn the_unsaved_count_counts_fields_not_files() {
        use crate::workbench::domain::shadow_diff;
        let (d, _f, mut ctx) = ctx();
        /* 造两棵**真的不同的**树：测试夹具默认两个根是同一个（镜像 = 恒等） */
        let real = d.0.path().join("presets-real");
        let shadow = d.0.path().join("presets-shadow");
        for (root, text) in [(&real, "a = 1\n"), (&shadow, "a = 2\n")] {
            std::fs::create_dir_all(root.join("machines")).unwrap();
            crate::fsx::atomic::atomic_write(&root.join("machines/A1.toml"), text.as_bytes())
                .unwrap();
        }
        ctx.real_root = real.clone();
        ctx.shadow_root = shadow.clone();
        ctx.refresh_unsaved_files();

        let diff = shadow_diff::shadow_diff(&real, &shadow).unwrap();
        assert_eq!(ctx.unsaved_files(), 1);
        assert_eq!(
            ctx.unsaved_files(),
            diff.files.len(),
            "框里那句「N 份文件」仍然按份算"
        );
        /* 页头那个数按**字段**算：这里没人录事务（`txn_state` 只累加 `txn` 与草稿层），
        所以它不统计"盘上已有的差异" —— 那是事务之外的账，别混进来。 */
        assert_eq!(ctx.txn_state().dirty_count, 0, "没有事务 ⇒ 页头那一格是 0");

        /* 双侧一致 ⇒ 归零（不是"永远至少 1"） */
        crate::fsx::atomic::atomic_write(&shadow.join("machines/A1.toml"), b"a = 1\n").unwrap();
        ctx.refresh_unsaved_files();
        assert_eq!(ctx.unsaved_files(), 0);
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

        let out = apply_patches(
            &mut ctx.draft,
            &ctx.committed,
            &ctx.presets.registry,
            &[patch],
        )
        .unwrap();
        ctx.touch();
        assert!(
            !out.inverse.is_empty(),
            "值编辑必须产反向，否则撤销栈没东西可推"
        );

        let (c2, d2, _) = state(&ctx).unwrap();
        assert_eq!(d2.dirty_count(), 1);
        let v = view_of(&ctx, &c2, &d2, Vec::new());
        /* 这条测试只往**内存草稿**里 apply 了一条 patch、没写影子树（真落盘那一步由
        `wb_apply_draft` 里的 `apply_draft_now` 做）。`view_of` 现在把草稿层也算进来
        （否则改完的那一瞬页头会显示 0），所以这里就是 1 —— 一格 = 一行。 */
        assert_eq!(v.dirty_count, 1, "内存草稿里那一格也算一行");
        assert_eq!(v.save, w::SaveState::Dirty);
        assert_eq!(v.snapshot, w::SnapshotState::Pending, "还没落盘");

        // 反向回去 → 干净
        let inverse = out.inverse;
        apply_patches(
            &mut ctx.draft,
            &ctx.committed,
            &ctx.presets.registry,
            &inverse,
        )
        .unwrap();
        ctx.touch();
        assert!(ctx.draft.is_clean(), "撤销之后该回到干净");
    }

    /// **编辑不碰磁盘。** 上一稿每次编辑一次原子写，那是「卡」的一个来源，
    /// 也是把磁盘当状态源
    #[test]
    fn editing_does_not_touch_the_disk() {
        let (dir, _f, mut ctx) = ctx();
        let snapshot = dir.0.path().join(".draft").join("book.json");
        assert!(!snapshot.exists(), "干净仓库本来就没有快照");

        for n in 0..10 {
            apply_patches(
                &mut ctx.draft,
                &ctx.committed,
                &ctx.presets.registry,
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

    /// 那个「改过去改不回来」的后端侧回归：**有效值每一步都必须跟着变**。
    ///
    /// 上一稿前端拿脏计数当刷新信号，所以这里连脏计数一起钉住：`toolhead.offset.x`
    /// 在版本层本来就钉着（`A1:STANDARD` = -1），5 → 7 两步**都脏**、脏计数一直是 1，
    /// 而值确实变了 —— 刷新信号只能是 `tick`，不能是脏计数。
    ///
    /// 第三步回到 `-1`（本层盘上那个值）：**脏计数归 0**，这一处不再是改动
    /// （作者实测报的那条：「改了又改回去就变成了已修改是不对的」）。
    #[test]
    fn a_value_changed_back_and_forth_is_visible_each_time() {
        let (_d, _f, mut ctx) = ctx();
        let set = |ctx: &mut Ctx, v: i64| {
            apply_patches(
                &mut ctx.draft,
                &ctx.committed,
                &ctx.presets.registry,
                &[Patch::SetValue {
                    level: Level::Version,
                    owner: "A1/STANDARD".to_owned(),
                    key: "toolhead.offset.x".to_owned(),
                    value: Some(serde_json::json!(v)),
                }],
            )
            .unwrap();
            ctx.touch();
        };
        let now = |ctx: &Ctx| {
            let cols = cols(&[("A1", Some("A1/STANDARD"))]);
            let (c, d, _) = state(ctx).unwrap();
            let m = Book::new(&ctx.presets, &c, &d).matrix(&cols, None, "", None);
            let row = m
                .rows
                .into_iter()
                .find(|r| r.key == "toolhead.offset.x")
                .unwrap();
            row.cells[0].raw.clone()
        };

        set(&mut ctx, 5);
        assert_eq!(now(&ctx), serde_json::json!(5.0));
        assert_eq!(ctx.draft.dirty_count(), 1);

        set(&mut ctx, 7);
        assert_eq!(now(&ctx), serde_json::json!(7.0), "改了必须看得见");
        assert_eq!(
            ctx.draft.dirty_count(),
            1,
            "脏计数没变 —— 所以它不能当刷新信号"
        );

        set(&mut ctx, -1);
        assert_eq!(now(&ctx), serde_json::json!(-1.0), "改回去同样必须看得见");
        assert_eq!(
            ctx.draft.dirty_count(),
            0,
            "改回本层盘上那个值 = 没改过，不该再挂着「已修改」"
        );
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
            &ctx.presets.registry,
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
            Refresh::Desk {
                machine_id, uid, ..
            } => {
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
        let book = Book::new(&ctx.presets, &c, &d);

        let desk = book.desk("A1", Some("A1/STANDARD"), None, "");
        let matrix = book.matrix(&cols(&[("A1", Some("A1/STANDARD"))]), None, "", None);

        let desk_rows: usize = desk
            .groups
            .iter()
            .flat_map(|g| g.items.iter())
            .map(|i| 1 + i.children.len())
            .sum();
        assert_eq!(
            desk_rows,
            matrix.rows.len(),
            "同一份数据，两种摆法，行数一样"
        );
    }

    /// **差异清单要逐条说得出来**，不能只给一个数字。
    ///
    /// 以前这一条还要列出改名与归档那两行 —— 两种动作都删掉了（REPORT §7），
    /// 于是**现在能进草稿的只有值改动**，这三行全是改值
    #[test]
    fn the_diff_lists_every_pending_change() {
        let (_d, _f, mut ctx) = ctx();
        /* `diff_draft` 是纯函数，只读 `ctx.txn.field_changes()` */
        let lines = diff_draft(&ctx);
        assert!(lines.is_empty(), "没有改动时框里是空的");

        /* 造一次真的事务：写影子树 + 记字段账 */
        let before = crate::workbench::domain::txn::snapshot(&ctx.shadow_root).unwrap();
        crate::fsx::atomic::atomic_write(
            &ctx.shadow_root.join("registry/param_registry.toml"),
            b"changed = true\n",
        )
        .unwrap();
        let after = crate::workbench::domain::txn::snapshot(&ctx.shadow_root).unwrap();
        ctx.txn
            .record_with_fields(
                "改参数值",
                None,
                &before,
                &after,
                &ctx.shadow_root,
                vec![
                    txn::FieldEntry {
                        level: Some("Machine".to_owned()),
                        owner: "A1".to_owned(),
                        key: "wiping.child".to_owned(),
                        label: "擦料次数".to_owned(),
                        before: Some("1".to_owned()),
                        after: Some("33".to_owned()),
                        why: "user",
                    },
                    txn::FieldEntry {
                        level: Some("Version".to_owned()),
                        owner: "A1:STANDARD".to_owned(),
                        key: "toolhead.offset.x".to_owned(),
                        label: "X 轴偏移".to_owned(),
                        before: Some("-1".to_owned()),
                        after: Some("7".to_owned()),
                        why: "promoted",
                    },
                ],
            )
            .unwrap();

        let lines = diff_draft(&ctx);
        assert_eq!(lines.len(), 2, "两处改动两行：{lines:#?}");
        assert_eq!(lines[0].target, "A1 · 机型基底");
        assert_eq!(lines[1].target, "A1 · 标准版", "版本归属要翻成人话");
        assert_eq!(lines[1].why, "promoted", "连带项的来源要带到界面上");
        /* 2026-11-03：`promoted` 与继承统一成「随基底生效」—— 界面不再区分
        "我们写没写版本键"，那是存储细节 */
        assert_eq!(lines[1].kind, "改值（随基底）");
        assert_eq!(lines[1].before, "-1");
        assert_eq!(lines[1].after, "7");
        assert!(lines.iter().all(|l| l.file == VALUE_FILE));
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

    /// **「改一个值 → 保存 → 重开工作台，值还在」**（b04 Task 12.6）。
    ///
    /// 这是这一轮的主线，也是以前一直断着的那个主功能：写回能力早就做好了，
    /// 但 `storage::save` 还在写那些没人再读的 json，于是**保存等于什么都没做**。
    ///
    /// 这条以前挂在「新建版本 → 保存 → 在树上看见」上，而新建现在归「机型与版本」页
    /// （REPORT §7.4），所以换成整条主链：**改值 → 保存 → 重读盘 → 值还在**。
    /// 「重读盘」这三个字是这条判据的全部 —— 只看内存的话等于什么都没验
    #[test]
    fn a_value_edited_then_saved_survives_reopening_the_workbench() {
        let (_d, _f, mut ctx) = ctx();
        apply_patches(
            &mut ctx.draft,
            &ctx.committed,
            &ctx.presets.registry,
            &[Patch::SetValue {
                level: Level::Version,
                owner: "A1/STANDARD".to_owned(),
                key: "toolhead.offset.x".to_owned(),
                value: Some(serde_json::json!(7)),
            }],
        )
        .unwrap();
        ctx.touch();

        let out = storage::save(&ctx.store, &mut ctx.presets, &ctx.committed, &ctx.draft).unwrap();
        assert!(out.remap.is_empty(), "保存不再改 uid");

        // 重开 = 重读盘。落盘的那一份才是新的真相
        ctx.reload_from_disk().unwrap();
        let (c2, d2, n2) = state(&ctx).unwrap();
        assert!(d2.is_clean(), "保存后草稿清空");
        let v = view_of(&ctx, &c2, &d2, n2);
        assert!(v.notices.is_empty(), "不该有任何提示：{:?}", v.notices);
        assert_eq!(v.dirty_count, 0);

        // ① 盘上：值躺在 `machineVariants` 的 `A1:STANDARD` 里
        let table = &ctx
            .presets
            .registry
            .param("toolhead.offset.x")
            .unwrap()
            .machine_variants;
        assert_eq!(table["A1:STANDARD"].as_f64(), Some(7.0), "值没落到盘上");
        assert_eq!(table["A1:FAST"].as_f64(), Some(-0.7), "顺手动了隔壁那一版");

        // ② 界面上：来源层该是「版本」—— 它自己钉着这一项
        let book = Book::new(&ctx.presets, &c2, &d2);
        let l = book.version_layers("A1/STANDARD").unwrap();
        assert_eq!(
            l.effective("toolhead.offset.x").unwrap().value.as_f64(),
            Some(7.0),
            "float 字段越过一次盘就是 7.0，不是在赌它的表示"
        );
        assert_eq!(
            l.effective("toolhead.offset.x").unwrap().origin,
            Origin::Version
        );

        // ③ 顺带：改了值，这一版的产物指纹就变了 → 该显示「待生成」
        assert_eq!(book.build_state("A1/STANDARD"), BuildState::NeverBuilt);
        assert_eq!(v.badges.versions, 4, "这一轮不改清单，版本数不变");
    }

    /// 矩阵与批量预览的列序一致，且矩阵能按一列当「字段详情」用
    #[test]
    fn a_single_column_matrix_serves_as_the_field_detail_view() {
        let (_d, _f, ctx) = ctx();
        let (c, d, _) = state(&ctx).unwrap();
        let book = Book::new(&ctx.presets, &c, &d);

        let m = book.matrix(&cols(&[("A1", Some("A1/STANDARD"))]), None, "", None);
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

    /* ---------- 参数台 v2 的数据面（2026-11-02） ---------- */

    /// **`saved` 读的是会话 BASE，不是编辑树** —— 新页与旧页的唯一分界线。
    ///
    /// 这条反了的话，新页和旧页就没有任何区别：编辑一格之后 `saved` 跟着变成新值，
    /// 徽章"改了也不出现"（旧页连报三次的第一种表现）；或者一出现就写着新值
    /// （第二种）。所以判据必须是"两边不同源"：
    ///
    ///   · 编辑树（`Book::matrix` 的 `raw`）= 刚改的 7
    ///   · `wb_param_sheet` 的 `saved`        = 打开工作台那一刻的值（-0.3 或夹具里那一份）
    #[test]
    fn the_v2_sheet_reads_saved_from_the_base_not_from_the_edit_tree() {
        let (_d, _f, mut ctx) = ctx();
        let machine_col = cols(&[("A1", None)]);

        let before = sheet::build(&ctx, &machine_col, None, "", None);
        let cell_before = before
            .cells
            .iter()
            .find(|c| c.key == "toolhead.offset.x")
            .expect("夹具里这一项在机器 A1 上");
        let saved_before = cell_before.saved.clone();
        let pinned_before = cell_before.saved_pinned;
        let text_before = cell_before.saved_text.clone();
        assert!(saved_before.is_some(), "这一格在 BASE 里应当有有效值");

        // 改值：只进草稿 / 编辑树，BASE 一个字节不动
        apply_patches(
            &mut ctx.draft,
            &ctx.committed,
            &ctx.presets.registry,
            &[Patch::SetValue {
                level: Level::Machine,
                owner: "A1".to_owned(),
                key: "toolhead.offset.x".to_owned(),
                value: Some(serde_json::json!(7)),
            }],
        )
        .unwrap();
        ctx.touch();

        // ① 编辑树看见的是新值（旧路径不变）
        let (c, d, _) = state(&ctx).unwrap();
        let m = Book::new(&ctx.presets, &c, &d).with_rest(&ctx.base).matrix(
            &machine_col,
            None,
            "",
            None,
        );
        let raw = &m
            .rows
            .iter()
            .find(|r| r.key == "toolhead.offset.x")
            .unwrap()
            .cells[0]
            .raw;
        assert_eq!(raw.as_f64(), Some(7.0), "编辑树该看见刚改的值");

        // ② 新页的 saved 一步都不许动
        let after = sheet::build(&ctx, &machine_col, None, "", None);
        let cell_after = after
            .cells
            .iter()
            .find(|c| c.key == "toolhead.offset.x")
            .unwrap();
        assert_eq!(
            cell_after.saved, saved_before,
            "saved 跟着编辑树变了 —— 徽章从此不会亮（旧页的病）"
        );
        assert_eq!(cell_after.saved_pinned, pinned_before, "钉状态也只认 BASE");
        assert_eq!(
            cell_after.saved_text, text_before,
            "保存值的显示文本必须是**保存值**的，不是刚打进去那个数"
        );
        assert!(
            !cell_after.saved_text.contains('7') || text_before.contains('7'),
            "`saved_text` 接到了当前值上（视频里徽章写错值就是这个）"
        );
    }

    /// **改完值之后，界面拿到的那份派生必须还说「脏」**（2026-11-02 作者：「一闪就不见了」）。
    ///
    /// `wb_apply_draft` 返回的 `desk` / `matrix` 走的是 [`book_of`] —— 它一漏
    /// `with_rest`，`rest` 就变成**刚写进去的新值**，前端一比「值 == 保存值」
    /// 当场判不脏，徽章亮一帧就灭（这次比 `wb_matrix` 那处更短，因为它连下一次
    /// 重取都不用等，就在返回的那一份里）。
    #[test]
    fn an_edit_never_makes_the_derived_view_look_clean() {
        let (_d, _f, mut ctx) = ctx();
        let machine_col = cols(&[("A1", None)]);
        let key = "toolhead.offset.x";
        let rest_before = book_of(&ctx)
            .matrix(&machine_col, None, "", None)
            .rows
            .iter()
            .find(|r| r.key == key)
            .unwrap()
            .cells[0]
            .rest
            .clone();

        apply_patches(
            &mut ctx.draft,
            &ctx.committed,
            &ctx.presets.registry,
            &[Patch::SetValue {
                level: Level::Machine,
                owner: "A1".to_owned(),
                key: key.to_owned(),
                value: Some(serde_json::json!(7)),
            }],
        )
        .unwrap();
        ctx.touch();

        let cell = book_of(&ctx)
            .matrix(&machine_col, None, "", None)
            .rows
            .iter()
            .find(|r| r.key == key)
            .unwrap()
            .cells[0]
            .clone();
        assert_eq!(cell.raw.as_f64(), Some(7.0), "这一格该显示刚改的值");
        assert_eq!(
            cell.rest, rest_before,
            "`rest` 跟着新值跑了 —— 徽章会亮一帧就灭"
        );
        assert!(cell.dirty, "改完却说没改：`dirty` 与 `rest` 必须同一份数据");
    }

    /// **没有可放弃 / 可保存的东西，不是错误**（2026-11-02 作者：「它永远不消失，
    /// 除非我重新打开软件，不能点穿」）。
    ///
    /// 作者点第二次「放弃改动」时，后端返回 `NOTHING_TO_SAVE`，前端那条 `fatal`
    /// 横幅**只设不清**、z 60 盖住页头 ⇒ 一块点不穿的黑板挂在左上角。
    /// 语义上「放弃一个已经干净的状态」= 已经是你要的样子，幂等返回就行。
    #[test]
    fn discarding_or_saving_without_changes_is_idempotent_not_an_error() {
        let (_d, _f, mut ctx) = ctx();
        assert!(
            discard_in(&mut ctx).is_ok(),
            "没有可放弃的东西不该报错 —— 它会变成一条关不掉的横幅"
        );
        assert!(
            save_in(&mut ctx).is_ok(),
            "没有可保存的东西同理：多点一下保存不等于出错"
        );
    }

    /// 一屏至少要一列：没有列就没有格子，给前端空表等于让它画一屏空白。
    #[test]
    fn the_v2_sheet_refuses_an_empty_column_list() {
        let (_d, _f, ctx) = ctx();
        assert!(sheet::sheet_in(&ctx, Vec::new(), None, None, None).is_err());
    }

    /* ---------- 读命令必须异步（2026-10-02 的性能修复，防回退） ---------- */

    /// **只读命令一律 `#[tauri::command(async)]`。**
    ///
    /// # 为什么这条必须有机器拦着
    ///
    /// Tauri 云端文档的原文：**不带 `async` 的命令在主线程上执行，除非写成
    /// `#[tauri::command(async)]`**。而主线程就是 webview 渲染那个线程 ——
    /// 同步命令算多久，**整个界面就冻结多久**：骨架屏画不出来、导航点不动，
    /// 用户看到的就是"像被冻住了，过了好一会才有反应"（作者原话）。
    ///
    /// 这不是"再优化一点"的问题，是**别的线程算、主线程继续画**的问题：
    /// 只要命令还在主线程上，算 14ms 还是 320ms 都是同一种病，只是轻重不同。
    /// 所以新增一条读命令**忘了写 `(async)`** 就是把这个病又带回来 —— 用源码扫描拦住。
    ///
    /// 写命令**故意不在这张单子里**：它们要落盘、要和草稿的锁打配合，改异步是另一
    /// 件要单独评估的事（`wb_apply_draft` / `wb_save` / `wb_generate` …）。
    ///
    /// ★ **但"碰网络 / 凭据盘"的命令必须在这张单子里**（下面 [`IO`]）——2026-10-04
    /// 真机事故：`wb_publish` 当时是同步命令，读 Keychain（系统弹密码框，凭据后来
    /// 改住凭据文件）、推 git、建 PR 全发生在主线程上，窗口一动不动；作者原话
    /// 「卡住了，我什么都没办法点」。那条命令现在已经 `(async)`，这里把它钉住，别再退回去。
    #[test]
    fn read_commands_are_async_so_they_never_freeze_the_window() {
        /// 这一批是**只读**（只算不写盘）的 —— 全部必须是 `(async)`。
        const READ: &[&str] = &[
            "wb_boot",
            "wb_words",
            "wb_book",
            "wb_registry",
            "wb_matrix",
            // 参数台 v2（2026-11-02）：只做投影，不写盘、不动会话状态
            "wb_param_sheet",
            "wb_desk",
            "wb_trash",
            "wb_ui",
            "wb_preview_bulk",
            "wb_diff_draft",
            // 未保存改动的逐行 diff：只做快照 + 读字节，不写盘
            "wb_shadow_diff",
            "wb_preflight",
            "wb_generate_preview",
            "wb_revert_preview",
            "wb_dist_strays",
            // 交付文件清单（2026-10-07）：只读一份名单 + 读一份盘上原文，都不写盘
            "wb_delivery_files",
            "wb_delivery_file",
            // 发布闸（第二刀）：只算不写 —— 与 `publish_audit` 是同一件事的两个壳
            "wb_publish_audit",
            "wb_baseline_diff",
            "wb_assets",
            "wb_asset_usage",
            "wb_asset_inspect",
            // 「在访达中显示」不写任何应用状态（只开系统文件管理器）—— 也归读这一档
            "wb_reveal_asset",
            "wb_bundles",
            "wb_machines",
            "wb_version_orphans",
            // 发布收尾（回执屏）：只读一份回执日志；「查看 PR」只开系统浏览器，不写应用状态
            "wb_publish_history",
            "wb_open_external",
        ];

        /// 这一批**碰网络或凭据盘**（发布事务那几条）—— 同样必须 `(async)`。
        ///
        /// 为什么它们比读命令更要紧：读命令只是"算得久"；这几条要**等网络**
        /// （git 推送、平台 API —— 等多久完全不可控），凭据文件虽是本地盘 IO，
        /// 也归进同一批：**碰外界的命令不许占主线程**是一条规矩，不为每一处单独开例外。
        const IO: &[&str] = &[
            "wb_publish",
            "wb_publish_account",
            "wb_set_publish_account",
            "wb_set_publish_token",
            // 设置页「眼睛」取明文：读凭据文件，同样是碰盘的那一档
            "wb_get_publish_token",
            "wb_clear_publish_account",
            "wb_publish_status",
            "wb_merge_review",
        ];

        // 路径用 CARGO_MANIFEST_DIR 拼（不用 file!()：建了 workspace 之后它的基准会变）
        let app = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("workbench")
            .join("app");
        let mut sources = String::new();
        for name in [
            "mod.rs",
            "build.rs",
            "assets.rs",
            "bundles.rs",
            "machines.rs",
            "words.rs",
            // 发布事务那几条命令住这里（IO 单子要扫到它们）
            "publish_tx.rs",
        ] {
            sources.push_str(
                &std::fs::read_to_string(app.join(name))
                    .unwrap_or_else(|e| panic!("读不到 {name}：{e}")),
            );
            sources.push('\n');
        }

        let mut missing: Vec<&str> = Vec::new();
        for fn_name in READ {
            // 找 `#[tauri::command...]` 紧跟着 `pub fn <fn_name>` 的那一处，看它带不带 (async)
            let needle = format!("pub fn {fn_name}(");
            let at = sources.find(&needle).unwrap_or_else(|| {
                panic!("源码里找不到读命令 {fn_name} —— 单子过时了（改了名或删了？）")
            });
            // 往回退到最近的那个 #[tauri::command 属性
            let head = &sources[..at];
            let attr_at = head.rfind("#[tauri::command").expect("前面一定有属性");
            let attr = &sources[attr_at..at];
            if !attr.contains("(async)") {
                missing.push(fn_name);
            }
        }
        assert!(
            missing.is_empty(),
            "这些读命令还跑在主线程上（算多久界面就冻多久）—— 加 `#[tauri::command(async)]`：{missing:?}"
        );

        // 同样的扫描，对「碰网络 / 凭据盘」的命令再来一遍 —— 它们的主线程代价更重
        let mut missing_io: Vec<&str> = Vec::new();
        for fn_name in IO {
            let needle = format!("pub fn {fn_name}(");
            let at = sources.find(&needle).unwrap_or_else(|| {
                panic!("源码里找不到命令 {fn_name} —— 单子过时了（改了名或删了？）")
            });
            let head = &sources[..at];
            let attr_at = head.rfind("#[tauri::command").expect("前面一定有属性");
            if !sources[attr_at..at].contains("(async)") {
                missing_io.push(fn_name);
            }
        }
        assert!(
            missing_io.is_empty(),
            "这些命令碰网络 / 凭据盘，却还跑在主线程上（推送要等网络、发布链还有落盘与\
             子进程 —— 界面会整段冻住）：{missing_io:?}"
        );
    }

    /* ---------- 云端源双源（2026-10-05：设置页两格，写盘不再无声冲掉 Gitee） ---------- */

    /// 单键老档（双源之前的形状）照读 —— `gitee_bootstrap_url` 缺席 = `None`，不炸
    #[test]
    fn old_single_key_bootstrap_config_still_reads() {
        let cfg: BootstrapConfig =
            serde_json::from_str(r#"{"bootstrapUrl": "https://example.com/s.json"}"#)
                .expect("单键老档该读得动");
        assert_eq!(cfg.bootstrap_url, "https://example.com/s.json");
        assert_eq!(cfg.gitee_bootstrap_url, None);
    }

    /// 双源一起写、一起读回；两边都是仓库地址 → 都被补成各自默认发布入口的 raw
    #[test]
    fn bootstrap_write_roundtrips_both_sources() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at(dir.path());
        store.bootstrap().unwrap();

        let cfg = write_bootstrap_config(
            &store,
            "https://github.com/o/r",
            Some("https://gitee.com/o/r"),
        )
        .expect("双源都合法");
        assert_eq!(
            cfg.bootstrap_url,
            "https://raw.githubusercontent.com/o/r/main/presets/delivery/source.json"
        );
        assert_eq!(
            cfg.gitee_bootstrap_url.as_deref(),
            Some("https://gitee.com/o/r/raw/main/presets/delivery/source.json")
        );

        let back: BootstrapConfig = store
            .read_doc(Store::BOOTSTRAP_REL, "云端源配置")
            .expect("读回")
            .expect("刚写的该在");
        assert_eq!(back.bootstrap_url, cfg.bootstrap_url);
        assert_eq!(back.gitee_bootstrap_url, cfg.gitee_bootstrap_url);
    }

    /// **清除语义**：`None` / 空白 = Gitee 档删掉（文件里不留 `null` 键），主源照写。
    /// 界面上那格空着就是清除 —— 是显式的操作，不是"没传就帮你保住"的暗规则
    #[test]
    fn bootstrap_write_clears_gitee_when_blank() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at(dir.path());
        store.bootstrap().unwrap();
        write_bootstrap_config(
            &store,
            "https://github.com/o/r",
            Some("https://gitee.com/o/r"),
        )
        .unwrap();

        let cfg = write_bootstrap_config(&store, "https://github.com/o/r2", None).unwrap();
        assert_eq!(cfg.gitee_bootstrap_url, None, "None = 清除");
        let cfg = write_bootstrap_config(&store, "https://github.com/o/r2", Some("   ")).unwrap();
        assert_eq!(cfg.gitee_bootstrap_url, None, "空白 = 清除");

        let text = std::fs::read_to_string(dir.path().join("bootstrap.json")).unwrap();
        assert!(
            !text.contains("giteeBootstrapUrl"),
            "清除后文件里不该留 null 键：{text}"
        );
    }
}
