//! **发布闸**（第二刀）：发布前的**唯一入口判定器**。
//!
//! # 它是什么
//!
//! 一次发布 = 「检查 → 生成 → commit → push → 建 PR」这条链上**最前面那一格**，也是
//! 唯一有权说「可以往下走」的那一格。作者定的两条硬规矩：
//!
//! 1. **任何一项红（Blocker 非 Pass），绝对不进 commit / push / PR**；
//! 2. **它是 Rust 核心** —— 工作台界面、将来的 CLI、`cargo test` 判据调的都是
//!    [`publish_audit`] 这一个函数。**界面不许自己再实现一套检查**（那正是
//!    「发布闸」之前散落各处的老病根：两边各算一遍，各自看着都对）。
//!
//! # 一条检查 = 一个 [`AuditItem`]
//!
//! 加一项检查 = 加一个 `AuditItem`，**不改流程**。这是 `docs/PUBLISH-ARCHITECTURE.md`
//! §5.1 定下的扩展性要求：将来发布闸长到 30 项，这一份文件的结构不变。
//!
//! 十五项的清单与编号照 §5.2（改编号要连文档一起改 —— id 是契约）。
//!
//! # 分档（§5.1）
//!
//! ```text
//! Blocker  Fail → 不许发布（can_publish = false）
//! Warning  Warn → 可以发，但要人看一眼
//! Skipped  ——   这一项今天还没有实现/不适用，**显式摆出来**，不假装 Pass
//! ```
//!
//! ★ `Skipped` 是刻意的一档：把"没实现"伪装成"通过"，比红色更危险。
//!
//! # 两层检查（2026-10-05 起）
//!
//! 预检管**源状态**（git/clean、源数据、结构、可达性），交付账本的最终一致由
//! [`finalize_consistency`] 在**发布事务定稿之后、commit 之前**断言。关键裁定：
//! ⑭ `manifest/correct` 在预检里是**提示档**（Warning）—— manifest 是上一版发布
//! 写下的账本，落后于 delivery 是「生成过、还没发」的常规状态，拦了就是
//! 「manifest 不一致 → 不许发布 → 无法通过发布修 manifest」的死循环。

use serde::Serialize;

use super::build::Scope;
use super::delivery;
use super::{state, with_ctx, Book};
use crate::error::AppError;
use crate::workbench::paths;

/* ---------- 结果模型 ---------- */

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AuditStatus {
    Pass,
    Fail,
    Warn,
    /// 这一项**没有跑**（今天还没有实现 / 不适用）。**不许当 Pass 用**
    Skipped,
}

/// 这一项的**分量**。`Fail` 时它决定拦不拦：只有 `Blocker` 能把 `can_publish` 压成 false
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AuditSeverity {
    Blocker,
    Warning,
}

/// 发布闸里的一行
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditItem {
    /// 稳定 id（§5.2 那张表的 id）。前端按它 key 住，列表不会每次重排
    pub id: String,
    /// 界面上那一行的名字
    pub name: String,
    pub status: AuditStatus,
    pub severity: AuditSeverity,
    /// 一句话说清「红了是什么、在哪」。通过时也写（那时的结论）
    pub details: String,
    #[serde(default)]
    pub affected_files: Vec<String>,
    /// 界面上那颗「去修」的入口
    pub fix_hint: String,
}

impl AuditItem {
    fn base(id: &str, name: &str, severity: AuditSeverity, fix_hint: &str) -> Self {
        Self {
            id: id.to_owned(),
            name: name.to_owned(),
            status: AuditStatus::Skipped,
            severity,
            details: String::new(),
            affected_files: Vec::new(),
            fix_hint: fix_hint.to_owned(),
        }
    }

    fn pass(mut self, details: impl Into<String>) -> Self {
        self.status = AuditStatus::Pass;
        self.details = details.into();
        self
    }

    fn fail(mut self, details: impl Into<String>, affected: Vec<String>) -> Self {
        self.status = AuditStatus::Fail;
        self.details = details.into();
        self.affected_files = affected;
        self
    }

    fn warn(mut self, details: impl Into<String>, affected: Vec<String>) -> Self {
        self.status = AuditStatus::Warn;
        self.details = details.into();
        self.affected_files = affected;
        self
    }

    /// 显式说「这一项没跑」。**不许省略** —— 省略等于伪装成通过。
    ///
    /// ★ 第三刀之后**十五项都用不上它**了（⑫ 是最后一项 Skipped，已经真跑起来）。
    /// 留着这个构造器是因为那条规矩还要用：**将来加一项还没实现的检查时，它必须
    /// 显式摆成 `Skipped`，而不是悄悄 Pass 或干脆不摆**。
    #[allow(dead_code)]
    fn skip(mut self, why: impl Into<String>) -> Self {
        self.status = AuditStatus::Skipped;
        self.details = why.into();
        self
    }
}

/// 一次发布闸的全部结果
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishAudit {
    pub items: Vec<AuditItem>,
    /// 这次发布会让 `presets/` 新增多少文件（Git 那一格给的）
    pub files_added: usize,
    pub files_changed: usize,
    pub files_removed: usize,
    /// 这一代结构要求的**最低正式客户端版本**（规则表里登记的）。
    ///
    /// `None` = ⑫ 没给出答案（签名没登记 / 规则表读不出来）—— 那种情况 ⑫ 是 Blocker，
    /// `can_publish` 已经是 false，这一格只是把"发出去该写什么"摆给人看。
    pub min_version: Option<String>,
    /// 全部 Blocker 都 Pass 才是 true —— **这是能不能往下走的唯一答案**
    pub can_publish: bool,
}

impl PublishAudit {
    /// 有几项 Blocker 没过
    pub fn blockers(&self) -> usize {
        self.items
            .iter()
            .filter(|i| i.severity == AuditSeverity::Blocker && i.status == AuditStatus::Fail)
            .count()
    }
}

/* ---------- 入口 ---------- */

/// **跑一遍发布闸**。纯读：不写盘、不动 git、不发网络请求。
///
/// 与 `wb_publish` 共用同一个会话上下文（`with_ctx`），所以「闸里看到的」与
/// 「发布时会写出去的」是同一份数据 —— 不存在"检查的是一个状态、发布的是另一个"。
pub fn publish_audit() -> Result<PublishAudit, AppError> {
    with_ctx(audit_with)
}

/// 十五项的**锁无关内核**：给一份会话就算，自己不取锁。
///
/// ★ **发布事务必须调这一个**（[`super::publish_tx::run`]），不能调 [`publish_audit`]：
/// 事务跑在 `with_ctx` 里，而 `with_ctx` 的锁**不可重入** —— 回头调那个"会自己取锁"的
/// 入口就是自锁，表现是**主线程挂死**（不是报错；2026-10-04 真机上就是这么卡死的）。
/// 与 `build::preview_with` / `generate_with` 同一条纪律，`publish_tx` 的源码扫描判据钉住它。
pub(super) fn audit_with(ctx: &super::Ctx) -> Result<PublishAudit, AppError> {
    let (c, d, _) = state(ctx)?;
    let book = Book::new(&ctx.presets, &c, &d);
    // ★ 根从**会话那份 presets 的根**派生（`*_at`，与生成 / 发布同一条派生）——
    // 审计查的就是这个会话要发的那一棵交付树
    let delivery_root = paths::delivery_root_at(ctx.presets.root());
    // ★ 只读定位，不 `create_dir_all` —— 一个自称「只读：不写盘」的闸
    // 不该顺手造出一个目录
    let asset_root = paths::assets_root_at(ctx.presets.root());

    let mut items: Vec<AuditItem> = Vec::with_capacity(16);

    /* ① 源数据完整 —— 能走到这里就说明 `presets/` 整套读得出来（加载期校验过） */
    items.push(sources_complete(&book));

    /* ② 引用都能落地 */
    items.push(refs_resolve(&ctx.presets));

    /* ③ A 类资产的文件真在载荷根里 */
    items.push(assets_exist(&ctx.presets, &asset_root));

    /* ④ catalog.path 唯一且合法 */
    items.push(paths_unique(&ctx.presets));

    /* ⑤ B 类能渲染（走 preview 的锁无关内核 —— 只算不写） */
    items.push(presets_renderable(ctx));

    /* ⑥ 渲染结果真在 delivery/mkp/presets 下 */
    items.push(presets_rendered(&book, &delivery_root));

    /* ⑦ catalog 登记的每一条都取得到 */
    items.push(registered_paths_reachable(&ctx.presets, &delivery_root));

    /* ⑧ SHA / size 对真字节算且一致 */
    items.push(catalog_sha_size(&delivery_root));

    /* ⑨ 没有幽灵条目（空 path / 非法 path） */
    items.push(no_phantoms(&ctx.presets));

    /* ⑩ delivery 里没有残留 */
    items.push(no_strays(&book, &delivery_root));

    /* ⑪ 套餐引用闭包完整 */
    items.push(bundles_closure(&ctx.presets));

    /* ⑫ 结构代次与最低客户端版本（第三刀：不再是 Skipped） */
    let (structure, min_version) = structure_gate(&delivery_root);
    items.push(structure);

    /* ⑬ source.json 正确 */
    items.push(source_correct(&delivery_root));

    /* ⑭ manifest 与交付集合一致（预检 = 提示档；严格核对在事务 ③½ `finalize_consistency`） */
    items.push(manifest_correct(&delivery_root));

    /* ⑮ Git 工作区状态（"新增/改动/删除"计数随行 —— 从原始 porcelain 状态解析） */
    let (git, added, changed, removed) = git_clean();
    /* ★ ⑮ **必须先入列再判定** —— 否则 `can_publish` 看不到它：
    脏工作区（⑮ 红）时闸仍会亮「确认发布」，绕过「除交付产物外工作区必须干净」那条保护。
    入列顺序与 `blockers()`（它也遍历全部 items）一致，两者才严格等价。 */
    items.push(git);

    /* ⑯ 客户端视角 URL 对账（`dist/dist` 事故的直接产物：跨端契约从此有主人） */
    items.push(client_url_reconciliation(
        &paths::repo_root(),
        &delivery_root,
    ));

    let can_publish = items
        .iter()
        .all(|i| !(i.severity == AuditSeverity::Blocker && i.status == AuditStatus::Fail));

    Ok(PublishAudit {
        items,
        files_added: added,
        files_changed: changed,
        files_removed: removed,
        min_version,
        can_publish,
    })
}

/* ---------- 事务的最后一道安全检查（③½） ---------- */

/// **发布事务的最终一致性核对**：`publish_into` 定稿之后、stage / commit / push 之前，
/// 由 [`super::publish_tx::run`] 调用。核对"发出去的字节"与账本：manifest、catalog、
/// 真实交付文件、残留。
///
/// ★ 它**不是第二套发布闸**，三个固定：
///   · **时机固定** —— 只挂在定稿之后这一处，不是又一个可以随时跑的入口判定器；
///   · **清单固定** —— 就这三项，**刻意不含 ⑮ git/clean**：定稿后的工作区理应带着
///     `presets/delivery/` 的改动，那正是这次要提交的东西，拿它拦自己又是死锁；
///   · **语义固定** —— 红 = 内部错误或并发改动（定稿刚写下的账不该对不上），
///     调用方 Err 短路，绝不产生半截发布提交。
pub(super) fn finalize_consistency(
    book: &Book<'_>,
    delivery_root: &std::path::Path,
) -> Vec<AuditItem> {
    vec![
        manifest_correct_strict(delivery_root),
        catalog_sha_size(delivery_root),
        no_strays(book, delivery_root),
    ]
}

/* ---------- 十五项 ---------- */

fn sources_complete(book: &Book<'_>) -> AuditItem {
    let item = AuditItem::base(
        "sources/complete",
        "源数据完整",
        AuditSeverity::Blocker,
        "源读不出来时工作台本身就开不起来 —— 去 `presets/` 看那一份 TOML",
    );
    let machines = book.presets.catalog.machines().len();
    let versions = book.versions().len();
    let assets = book.presets.assets.items().len();
    let bundles = book.presets.bundles.items().len();
    if machines == 0 || versions == 0 {
        return item.fail(
            format!("源里一台机型或一个版本都没有（机型 {machines} / 版本 {versions}）"),
            vec!["presets/".to_owned()],
        );
    }
    item.pass(format!(
        "机型 {machines} 台 · 版本 {versions} 个 · 资产 {assets} 条 · 套餐 {bundles} 套，\
         加载期校验全过"
    ))
}

/// 引用能不能落地。**大小写不敏感**（与 `runtime::catalog::Catalog::asset` 同一口径）
fn refs_resolve(presets: &crate::presetdata::Presets) -> AuditItem {
    let item = AuditItem::base(
        "refs/resolve",
        "引用都能落地",
        AuditSeverity::Blocker,
        "去资产页把那条引用补上，或改掉引用它的那一处",
    );
    let mut dangling: Vec<String> = Vec::new();
    for b in presets.bundles.items() {
        for r in &b.asset_refs {
            if presets.assets.get(r).is_none() {
                dangling.push(format!("套餐 {} → {}", b.id, r));
            }
        }
    }
    // 机型自己的三个图位（外观图 / 硬件变体图 / 图标）+ 各版本的专属图
    for m in presets.catalog.machines() {
        for id in [
            m.image.as_deref(),
            m.image_variant.as_deref(),
            m.icon.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            if !id.trim().is_empty() && presets.assets.get(id).is_none() {
                dangling.push(format!("机型 {} → {}", m.id, id));
            }
        }
        for v in &m.versions {
            let Some(id) = v.image.as_deref() else {
                continue;
            };
            if !id.trim().is_empty() && presets.assets.get(id).is_none() {
                dangling.push(format!("机型 {} 版本 {} → {}", m.id, v.id, id));
            }
        }
    }
    // 品牌图也是资产 id（`brands.toml` 的 `logo`）—— 客户端品牌卡会直接取它
    for b in presets.catalog.brands() {
        let Some(id) = b.logo.as_deref() else {
            continue;
        };
        if !id.trim().is_empty() && presets.assets.get(id).is_none() {
            dangling.push(format!("品牌 {} → {}", b.id, id));
        }
    }
    if dangling.is_empty() {
        item.pass("套餐 assetRefs、机型三图位、版本图、品牌 logo 的引用全部能查到")
    } else {
        dangling.sort();
        item.fail(
            format!("{} 条引用查不到对应的资产", dangling.len()),
            dangling,
        )
    }
}

fn assets_exist(presets: &crate::presetdata::Presets, asset_root: &std::path::Path) -> AuditItem {
    let item = AuditItem::base(
        "assets/exist",
        "A 类资产的文件真在",
        AuditSeverity::Blocker,
        "把文件放回 `presets/assets/` 下台账登记的那个位置（A 类原地交付，不复制）",
    );
    let mut missing: Vec<String> = Vec::new();
    for a in presets.assets.items() {
        // 不进交付的那一类（`bundled` 图片）不查 —— 它随包、不下载
        if crate::runtime::catalog::dest_of_asset(a).is_none() {
            continue;
        }
        if !asset_root.join(&a.path).is_file() {
            missing.push(a.path.clone());
        }
    }
    if missing.is_empty() {
        item.pass("每一条要交付的资产，文件都在载荷根里")
    } else {
        missing.sort();
        item.fail(
            format!("{} 条登记了却取不到文件（客户端点了会 404）", missing.len()),
            missing,
        )
    }
}

fn paths_unique(presets: &crate::presetdata::Presets) -> AuditItem {
    let item = AuditItem::base(
        "assets/path-unique",
        "catalog.path 唯一且不越界",
        AuditSeverity::Blocker,
        "改台账 `presets/assets.toml` 里那条 path（不许 `..`、不许两条撞同一处）",
    );
    let mut seen: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut bad: Vec<String> = Vec::new();
    for a in presets.assets.items() {
        let Some(dest) = crate::runtime::catalog::dest_of_asset(a) else {
            continue;
        };
        *seen.entry(dest.clone()).or_default() += 1;
        if dest.split('/').any(|seg| seg == "..") {
            bad.push(format!("{dest}（越出发布根）"));
        }
    }
    for (path, n) in &seen {
        if *n > 1 {
            bad.push(format!("{path}（{n} 条登记撞同一处）"));
        }
    }
    if bad.is_empty() {
        item.pass(format!("{} 条落点互不相同、无一越界", seen.len()))
    } else {
        bad.sort();
        item.fail(format!("{} 处落点有问题", bad.len()), bad)
    }
}

/// ⑤ 能不能渲染 —— 走**同一台渲染器**（[`super::build::preview_with`]，只算不写）。
///
/// ★ 这里**必须**调那个内核，不能调 `wb_generate_preview` 命令壳：发布闸已经在
/// `with_ctx` 里，命令壳会再取一次那把**不可重入**的锁 —— 那是自己把自己挂死。
/// 「一个入口」这条规矩就落在这两处共用同一个 `preview_with` 上。
fn presets_renderable(ctx: &super::Ctx) -> AuditItem {
    let item = AuditItem::base(
        "presets/renderable",
        "B 类预设能成功渲染",
        AuditSeverity::Blocker,
        "先在生成视角生成一次，看它报的是哪一条",
    );
    match super::build::preview_with(ctx, &Scope::All) {
        Ok(rep) => {
            if let Some(b) = rep.blocked.as_deref() {
                item.fail(format!("预演被拦下：{b}"), Vec::new())
            } else {
                item.pass(format!(
                    "{} 份渲染成功（其中 {} 份字节没变）",
                    rep.files.len(),
                    rep.unchanged
                ))
            }
        }
        Err(e) => item.fail(format!("预演跑不起来：{}", e.message), Vec::new()),
    }
}

/// ⑥ 渲染结果真在 `delivery/mkp/presets/` 下（`delivery_expected_set` 就是"该有的那批"）
fn presets_rendered(book: &Book<'_>, delivery_root: &std::path::Path) -> AuditItem {
    let item = AuditItem::base(
        "presets/rendered",
        "渲染产物真在交付目录里",
        AuditSeverity::Blocker,
        "在生成视角里生成一次（收尾会自动补资产并重算 catalog）",
    );
    let mut missing: Vec<String> = Vec::new();
    for rel in delivery::delivery_expected_set(book) {
        if rel.starts_with(delivery::MKP_PRESETS_DIR) && !delivery_root.join(&rel).is_file() {
            missing.push(rel);
        }
    }
    if missing.is_empty() {
        item.pass("该生成的那几份都在 delivery/mkp/presets/ 下")
    } else {
        missing.sort();
        item.fail(format!("{} 份产物还没生成", missing.len()), missing)
    }
}

/// ⑦ 目录登记的每一条，`发布根 + catalog.path` 处都取得到。
/// ★ 这正是第一刀之后必须重写的那一项（A 类不在 delivery 里，核对方向要反过来）
fn registered_paths_reachable(
    presets: &crate::presetdata::Presets,
    delivery_root: &std::path::Path,
) -> AuditItem {
    let item = AuditItem::base(
        "catalog/matches-files",
        "登记的每一条都取得到",
        AuditSeverity::Blocker,
        "生成一次让清单重算；A 类资产要放回 `presets/assets/`",
    );
    let catalog = crate::runtime::catalog::Catalog::build_from_presets_lenient(
        presets,
        &delivery_root.join(delivery::MKP_PRESETS_DIR),
    );
    let publish_root = match delivery_root.parent() {
        Some(p) => p,
        None => {
            return item.fail(
                "交付根没有父目录（发布根）—— 布局不对".to_owned(),
                Vec::new(),
            )
        }
    };
    let mut unreachable: Vec<String> = Vec::new();
    for f in &catalog.files {
        if !publish_root.join(&f.path).is_file() {
            unreachable.push(f.path.clone());
        }
    }
    if unreachable.is_empty() {
        item.pass(format!(
            "登记的 {} 条，发布根下逐条都在",
            catalog.files.len()
        ))
    } else {
        unreachable.sort();
        item.fail(
            format!("{} 条登记了却取不到", unreachable.len()),
            unreachable,
        )
    }
}

/// ⑧ 发布侧的期望值：每条都得有 sha256 / size，且对真字节
fn catalog_sha_size(delivery_root: &std::path::Path) -> AuditItem {
    let item = AuditItem::base(
        "catalog/sha-size",
        "SHA / 大小对真字节算且一致",
        AuditSeverity::Blocker,
        "生成一次（清单会跟着重算），别手改 catalog.json",
    );
    let Ok(bytes) = std::fs::read(delivery_root.join(delivery::NEW_CATALOG_FILE)) else {
        return item.fail(
            "delivery/catalog.json 还不存在 —— 先生成一次".to_owned(),
            Vec::new(),
        );
    };
    let Ok(catalog) = serde_json::from_slice::<crate::runtime::catalog::Catalog>(&bytes) else {
        return item.fail("delivery/catalog.json 解析不出来".to_owned(), Vec::new());
    };
    let publish_root = match delivery_root.parent() {
        Some(p) => p,
        None => return item.fail("交付根没有父目录".to_owned(), Vec::new()),
    };
    let mut bad: Vec<String> = Vec::new();
    for f in &catalog.files {
        let (Some(want_sha), Some(want_size)) = (f.expected_sha(), f.expected_size()) else {
            bad.push(format!("{}（没登记 SHA / 大小）", f.path));
            continue;
        };
        let Ok(got) = std::fs::read(publish_root.join(&f.path)) else {
            bad.push(format!("{}（文件不在）", f.path));
            continue;
        };
        if got.len() as u64 != want_size || delivery::sha256_of(&got) != want_sha {
            bad.push(format!("{}（字节与登记的不同）", f.path));
        }
    }
    if bad.is_empty() {
        item.pass(format!("{} 条的期望值都对得上真字节", catalog.files.len()))
    } else {
        bad.sort();
        item.fail(format!("{} 条对不上", bad.len()), bad)
    }
}

/// ⑨ 幽灵：**台账层面**就没有实体可指的条目（空 path / 越界 path）。
///
/// 与 ⑦ 的分工：⑦ 管"path 有了、发布根上取不到"，这一项管"path 本身不成立" ——
/// 那种条目连"去哪取"都没说，客户端会拿它画出一个点不动的按钮。
fn no_phantoms(presets: &crate::presetdata::Presets) -> AuditItem {
    let item = AuditItem::base(
        "catalog/no-phantoms",
        "没有幽灵条目",
        AuditSeverity::Blocker,
        "改台账 `presets/assets.toml` 里那条 path；空 path 的 MKP 预设条目不进交付",
    );
    let mut phantoms: Vec<String> = Vec::new();
    let mut checked = 0usize;
    for a in presets.assets.items() {
        // ★ 只查**要交付**的那几条。`Image`（随包）/ `MkPreset`（文件条目由生成侧按命名规则算出）
        // 本来就不持 path —— 拿它们的空 path 报警是**把设计当故障**（9 条 MKP 预设会被全数误判）
        if crate::runtime::catalog::dest_of_asset(a).is_none() {
            continue;
        }
        checked += 1;
        let path = a.path.trim();
        if path.is_empty() {
            phantoms.push(format!("{}（没有 path）", a.id));
        } else if path.split('/').any(|seg| seg == "..") {
            phantoms.push(format!("{}（{} 越出发布根）", a.id, a.path));
        }
    }
    if phantoms.is_empty() {
        item.pass(format!("要交付的 {checked} 条 path 都指得到实处"))
    } else {
        phantoms.sort();
        item.fail(
            format!("{checked} 条里有 {} 条幽灵", phantoms.len()),
            phantoms,
        )
    }
}

fn no_strays(book: &Book<'_>, delivery_root: &std::path::Path) -> AuditItem {
    let item = AuditItem::base(
        "delivery/no-strays",
        "交付目录里没有残留",
        AuditSeverity::Blocker,
        "点「清理残留」（进 workbench/.trash/delivery/<时间戳>/，可还原）",
    );
    let strays = delivery::scan_strays(delivery_root, &delivery::delivery_expected_set(book));
    if strays.is_empty() {
        item.pass("delivery_root 里没有本次交付集合之外的文件")
    } else {
        item.fail(format!("{} 个残留文件", strays.len()), strays)
    }
}

/// ⑪ 套餐引用闭包：每个套餐都得有一条 MKP，且它引用的每一条都可达
fn bundles_closure(presets: &crate::presetdata::Presets) -> AuditItem {
    let item = AuditItem::base(
        "bundles/closure",
        "套餐引用闭包完整",
        AuditSeverity::Warning,
        "去套餐页把缺的那一条补上",
    );
    let mut bad: Vec<String> = Vec::new();
    for b in presets.bundles.items() {
        if b.asset_refs.is_empty() {
            bad.push(format!("套餐 {} 一条引用都没有", b.id));
            continue;
        }
        for r in &b.asset_refs {
            if presets.assets.get(r).is_none() {
                bad.push(format!("套餐 {} 的 {} 查不到", b.id, r));
            }
        }
    }
    if bad.is_empty() {
        item.pass(format!(
            "{} 套套餐的引用闭包都完整",
            presets.bundles.items().len()
        ))
    } else {
        bad.sort();
        item.warn(format!("{} 处闭包不完整", bad.len()), bad)
    }
}

/// ⑫ **结构代次与最低客户端版本**（第三刀）。
///
/// # 它判什么（两件事，缺一不可）
///
/// 1. **这一代在规则表里有主** —— `presets/structure-signatures.toml` 里查得到当前
///    结构签名。查不到就是 **Blocker**：**没有规则 = 不准发**。
///    "这次改动看起来兼容，所以 minVersion 不变"是**猜**，作者定的规矩是宁可发不出去。
/// 2. **本构建读得懂这一代**（签名 ∈ `SUPPORTED_SIGNATURES`）—— 那是"我这个构建有
///    处理它的代码"的声明。1 与 2 一般同时满足，分开判是为了让红的那一栏说得清是哪种。
///
/// # 查不到时怎么办
///
/// **不猜、不自己填**。详情里直接把该登记的那一行印出来（照抄进规则表、想清楚
/// `minClient` 填什么 —— 那是产品决定），`affectedFiles` 指到那份文件上。
///
/// # 采样为什么用随包那份目录
///
/// 签名只取决于**类型**（必填性 / JSON 形态），与拿哪一份数据去探无关 ——
/// `runtime::structure` 的 `the_signature_does_not_depend_on_which_sample_it_is_taken_from`
/// 钉着这一条。用随包那份的额外好处：闸**不依赖 delivery 当前状态**（结构该是什么，
/// 是代码的事，不是这批文件的事）。
fn structure_gate(delivery_root: &std::path::Path) -> (AuditItem, Option<String>) {
    use crate::runtime::structure;

    let item = AuditItem::base(
        "version/structure",
        "结构代次与最低客户端版本",
        AuditSeverity::Blocker,
        "在 presets/structure-signatures.toml 里登记这一代的签名与最低客户端版本",
    );
    let Some(publish_root) = delivery_root.parent() else {
        return (
            item.fail("交付根没有父目录 —— 定位不到发布根".to_owned(), Vec::new()),
            None,
        );
    };
    let sample = match serde_json::from_slice::<serde_json::Value>(crate::runtime::EMBEDDED_CATALOG)
    {
        Ok(v) => v,
        Err(e) => {
            return (
                item.fail(
                    format!("随包目录读不成 JSON，算不出结构签名：{e}"),
                    Vec::new(),
                ),
                None,
            )
        }
    };
    let signature = structure::signature_from(&sample);
    let epoch = structure::STRUCTURE_EPOCH;
    let rel = structure::RULES_REL_PATH.to_owned();

    let rules = match structure::RuleTable::load(publish_root) {
        Ok(t) => t,
        Err(e) => {
            return (
                item.fail(
                    format!("结构规则表读不出来：{}", e.message),
                    vec![rel.clone()],
                ),
                None,
            )
        }
    };

    // ① 有主吗
    let Some(rule) = rules.rule_of(&signature) else {
        let mut failed = item.fail(
            format!(
                "结构代次 {epoch}（签名 {signature}）在规则表里没有登记 —— \
                 **没有规则 = 不准发**，不猜兼容性"
            ),
            vec![rel.clone()],
        );
        failed.fix_hint = format!(
            "把这一条登记进 {rel}（`minClient` 填「哪个正式版起读得懂这一代」——\
             不要求那个版本已经发布）：\n{}",
            rules.suggestion(&signature)
        );
        return (failed, None);
    };

    // ② 本构建自己读不读得懂
    if !structure::SUPPORTED_SIGNATURES.contains(&signature.as_str()) {
        return (
            item.fail(
                format!(
                    "结构签名 {signature} 不在本构建的 `SUPPORTED_SIGNATURES` 里 —— \
                     规则表替它背了书，这一个构建却读不懂它"
                ),
                vec![rel],
            ),
            Some(rule.min_client.clone()),
        );
    }

    (
        item.pass(format!(
            "结构代次 {epoch} · 签名 {signature} · 最低正式客户端版本 {}（{}）",
            rule.min_client, rule.note
        )),
        Some(rule.min_client.clone()),
    )
}

/// ⑬ Bootstrap（source.json）：认得出来 + 它指的那个 catalog 真在交付根里
fn source_correct(delivery_root: &std::path::Path) -> AuditItem {
    let item = AuditItem::base(
        "source/correct",
        "Source Manifest（source.json）正确",
        AuditSeverity::Blocker,
        "重跑一次发布（`publish_into` 收尾会重写它）",
    );
    let source_path = delivery_root.join(delivery::SOURCE_FILE);
    let Ok(bytes) = std::fs::read(&source_path) else {
        return item.fail("delivery/source.json 还不存在".to_owned(), Vec::new());
    };
    /*
     * ★ **本地校验 = 纯解析**：`SourceManifest::parse` 是纯函数、不需要 URL ——
     *   它验协议代次（v2）、全部相对引用合法性、`filesRoot` 的两个合法值。
     *   解析过了，再用**客户端同一套 Resolver 语义**核对声明的东西真在交付根里：
     *   catalog 必在；声明了的 release / manifest 也必在（少一样 = 客户端那一链断）。
     */
    let manifest = match crate::runtime::resolver::SourceManifest::parse(&bytes) {
        Ok(m) => m,
        Err(e) => return item.fail(e.message.clone(), Vec::new()),
    };
    let mut missing: Vec<String> = Vec::new();
    let mut must_exist = |rel: &str, what: &str| {
        if !delivery_root.join(rel.trim_end_matches('/')).is_file() {
            missing.push(format!("{what} → {rel}"));
        }
    };
    must_exist(&manifest.catalog, "catalog");
    if let Some(rel) = &manifest.release {
        must_exist(rel, "release");
    }
    if let Some(rel) = &manifest.manifest {
        must_exist(rel, "manifest");
    }
    if missing.is_empty() {
        item.pass(format!(
            "v2 寻址规则声明认得出来（catalog={}，filesRoot={}），声明的东西都在交付根里",
            manifest.catalog, manifest.files_root
        ))
    } else {
        item.fail(
            format!("Manifest 声明了 {} 样东西，交付根里却没有", missing.len()),
            missing,
        )
    }
}

/// ⑯ **客户端视角 URL 对账**（`docs/RESOURCE-ADDRESSING-ROADMAP.md` §6.2）。
///
/// 模拟真实客户端：`workbench/bootstrap.json` 的内置地址 → 交付根 → 交付根里的
/// Manifest → 用**生产 Resolver**（与客户端同一处实现）逐条拼 URL → 折回仓库
/// 相对路径 → 与磁盘真实文件对账。catalog 全部条目 + 固定物（catalog / release /
/// manifest / content/*）都过一遍。
///
/// ★ 为什么必须有它：2026-10-05 的 `dist/dist` 事故里，发布闸十五项全绿 ——
/// 它们都在工作台自己的坐标系里对账，而"客户端怎么从 bootstrap 推锚点"这条
/// 跨端契约没有主人。这一格就是那个主人。**纯读、不发网络**。
fn client_url_reconciliation(
    repo_root: &std::path::Path,
    delivery_root: &std::path::Path,
) -> AuditItem {
    let item = AuditItem::base(
        "client/url-reconciliation",
        "客户端视角 URL 对账",
        AuditSeverity::Blocker,
        "锚点错位 = 发布出去的每个文件客户端都 404（见总纲 §7；修锚点，别修这条闸）",
    );
    // ① 内置 bootstrap 地址（工作台配置；Gitee 与 GitHub 同内容，对账 GitHub 一份即可）
    let bootstrap_path = repo_root.join("workbench").join("bootstrap.json");
    let Ok(text) = std::fs::read_to_string(&bootstrap_path) else {
        return item.fail(
            format!("读不到 {} —— 内置源没配", bootstrap_path.display()),
            Vec::new(),
        );
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
        return item.fail(
            "workbench/bootstrap.json 不是合法 JSON".to_owned(),
            Vec::new(),
        );
    };
    let Some(bootstrap_url) = v.get("bootstrapUrl").and_then(|x| x.as_str()) else {
        return item.fail(
            "workbench/bootstrap.json 没有 bootstrapUrl".to_owned(),
            Vec::new(),
        );
    };
    let delivery_url = match crate::runtime::source::bootstrap_delivery_url(bootstrap_url) {
        Ok(u) => u,
        Err(e) => return item.fail(e.message.clone(), Vec::new()),
    };
    // ② 交付根里的 Manifest → 装配生产 Resolver
    let Ok(manifest_bytes) = std::fs::read(delivery_root.join(delivery::SOURCE_FILE)) else {
        return item.fail(
            "delivery/source.json 还不存在（先发布一次）".to_owned(),
            Vec::new(),
        );
    };
    let resolver = match crate::runtime::resolver::SourceResolver::from_bootstrap(
        delivery_url.clone(),
        &manifest_bytes,
    ) {
        Ok(r) => r,
        Err(e) => return item.fail(e.message.clone(), Vec::new()),
    };
    // ③ catalog 全量条目 + 固定物，逐条 resolve → 折回仓库相对路径 → 核磁盘
    let Ok(catalog_bytes) = std::fs::read(delivery_root.join(delivery::NEW_CATALOG_FILE)) else {
        return item.fail(
            "delivery/catalog.json 还不存在（先生成一次）".to_owned(),
            Vec::new(),
        );
    };
    let catalog: crate::runtime::catalog::Catalog = match serde_json::from_slice(&catalog_bytes) {
        Ok(c) => c,
        Err(e) => return item.fail(format!("delivery/catalog.json 解析不出来：{e}"), Vec::new()),
    };
    let (anchor_delivery, anchor_files) = match resolver.anchor_urls() {
        Ok(a) => a,
        Err(e) => return item.fail(e.message.clone(), Vec::new()),
    };
    // URL 是百分号编码过的；折回仓库路径要解回来（文件名实测有空格）
    let decode = |s: &str| -> String {
        let bytes = s.as_bytes();
        let hex = |b: u8| -> Option<u8> {
            match b {
                b'0'..=b'9' => Some(b - b'0'),
                b'a'..=b'f' => Some(b - b'a' + 10),
                b'A'..=b'F' => Some(b - b'A' + 10),
                _ => None,
            }
        };
        let mut out = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' && i + 2 < bytes.len() {
                if let (Some(hi), Some(lo)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                    out.push(hi * 16 + lo);
                    i += 3;
                    continue;
                }
            }
            out.push(bytes[i]);
            i += 1;
        }
        String::from_utf8_lossy(&out).into_owned()
    };
    let mut bad: Vec<String> = Vec::new();
    {
        let check = |url: String, what: String, bad: &mut Vec<String>| {
            let repo_rel = if let Some(rest) = url.strip_prefix(&format!("{anchor_delivery}/")) {
                format!("presets/delivery/{rest}")
            } else if let Some(rest) = url.strip_prefix(&format!("{anchor_files}/")) {
                format!("presets/{rest}")
            } else {
                bad.push(format!(
                    "{what}：客户端会取 {url} —— 不落在仓库布局（交付根 / 发布根）内"
                ));
                return;
            };
            let repo_rel = decode(&repo_rel);
            if !repo_root.join(&repo_rel).is_file() {
                bad.push(format!("{what}：客户端会取 {url} —— 仓库里没有 {repo_rel}"));
            }
        };
        for f in &catalog.files {
            match resolver.resolve(crate::runtime::resolver::ResourceRef::Entry(f)) {
                Ok(addr) => match addr.remote() {
                    Some(url) => check(
                        url.to_owned(),
                        format!("{} · {}", f.kind, f.file_name),
                        &mut bad,
                    ),
                    None => bad.push(format!("{} · {}：Resolver 没给出地址", f.kind, f.file_name)),
                },
                Err(e) => bad.push(format!("{} · {}：{}", f.kind, f.file_name, e.message)),
            }
        }
        // 固定物：catalog / manifest / release / content/*
        use crate::runtime::resolver::ResourceRef;
        let fixed = [
            (ResourceRef::Catalog, "catalog"),
            (ResourceRef::Manifest, "manifest"),
            (ResourceRef::Release, "release"),
        ];
        for (r, what) in fixed {
            match resolver.resolve(r) {
                Ok(addr) => match addr.remote() {
                    Some(url) => check(url.to_owned(), what.to_owned(), &mut bad),
                    None => { /* 未声明 = 该面暂不提供，不算错 */ }
                },
                Err(_) => { /* 未声明同上；⑬ 已核"声明了就必须在" */ }
            }
        }
        if let Some(dir) = &resolver.manifest().content {
            let content_root = delivery_root.join(dir.trim_end_matches('/'));
            if let Ok(entries) = std::fs::read_dir(&content_root) {
                for e in entries.flatten() {
                    if e.path().is_file() {
                        if let Some(rel) = e.file_name().to_str() {
                            if let Ok(addr) = resolver
                                .resolve(crate::runtime::resolver::ResourceRef::Content(rel))
                            {
                                if let Some(url) = addr.remote() {
                                    check(url.to_owned(), format!("content/{rel}"), &mut bad);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    let total = catalog.files.len() + 3;
    if bad.is_empty() {
        item.pass(format!(
            "客户端视角逐条对上：catalog {} 条 + 固定物（catalog/manifest/release/content）按 {} 锚点全部落在仓库里",
            catalog.files.len(), anchor_delivery
        ))
    } else {
        item.fail(
            format!(
                "客户端视角 {} 条对不上（共核 ~{total} 条）—— 这些地址发出去就是 404",
                bad.len()
            ),
            bad,
        )
    }
}

/// manifest 与 catalog **两本账的比较内核**：预检 ⑭ 与事务定稿后的严格核对共用它 ——
/// 「两账怎么算一致」只有这一处定义，两层不许各算各的。
struct LedgerComparison {
    /// manifest 里的条目总数（一致时"n 条两账一致"用它）
    total: usize,
    /// 不一致的条目（空 = 两账一致）
    bad: Vec<String>,
    /// 哪本账读不到 / 解析不出来。**这不是"不一致"** —— 是根本没比成
    unreadable: Option<String>,
}

fn compare_ledgers(delivery_root: &std::path::Path) -> LedgerComparison {
    let mut out = LedgerComparison {
        total: 0,
        bad: Vec::new(),
        unreadable: None,
    };
    let Ok(bytes) = std::fs::read(delivery_root.join(delivery::MANIFEST_FILE)) else {
        out.unreadable = Some("delivery/manifest.json 还不存在".to_owned());
        return out;
    };
    let Ok(manifest) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        out.unreadable = Some("delivery/manifest.json 解析不出来".to_owned());
        return out;
    };
    let Ok(catalog_bytes) = std::fs::read(delivery_root.join(delivery::NEW_CATALOG_FILE)) else {
        out.unreadable = Some("delivery/catalog.json 还不存在".to_owned());
        return out;
    };
    let Ok(catalog) = serde_json::from_slice::<crate::runtime::catalog::Catalog>(&catalog_bytes)
    else {
        out.unreadable = Some("delivery/catalog.json 解析不出来".to_owned());
        return out;
    };
    let by_path: std::collections::HashMap<&str, &crate::runtime::catalog::CatalogFile> =
        catalog.files.iter().map(|f| (f.path.as_str(), f)).collect();

    let mut bad: Vec<String> = Vec::new();
    let mut n = 0usize;
    for a in manifest
        .get("assets")
        .and_then(|v| v.as_array())
        .unwrap_or(&vec![])
    {
        n += 1;
        let rel = a.get("relativePath").and_then(|v| v.as_str()).unwrap_or("");
        let sha = a.get("sha256").and_then(|v| v.as_str()).unwrap_or("");
        match by_path.get(rel) {
            None => bad.push(format!("{rel}（manifest 有、catalog 里没有）")),
            Some(f) if f.expected_sha() != Some(sha) => {
                bad.push(format!("{rel}（两账的 SHA 不一致）"))
            }
            Some(_) => {}
        }
    }
    bad.sort();
    out.total = n;
    out.bad = bad;
    out
}

/// ⑭ manifest 与交付集合一致 —— **预检是提示档，不是闸**。
///
/// manifest 是**上一版发布**写下的账本，只有发布事务定稿（`publish_into` 收尾）会
/// 重写它。所以「生成过、还没发」的落后是**常规状态**：预检若因它把发布拦死，就是
/// 「manifest 不一致 → 不许发布 → 无法通过发布修 manifest」的死循环（2026-10-05
/// 真机踩过：生成之后发布永远进不去，只能手工 restore 交付根绕过）。
/// 严格的逐条核对由 [`finalize_consistency`] 在**事务定稿之后、commit 之前**跑 ——
/// 那里才是这道检查管事的地方。
fn manifest_correct(delivery_root: &std::path::Path) -> AuditItem {
    let item = AuditItem::base(
        "manifest/correct",
        "manifest 与交付集合一致",
        AuditSeverity::Warning,
        "无需手工处理 —— 点「确认发布」，发布事务定稿时会重写 manifest",
    );
    let cmp = compare_ledgers(delivery_root);
    if let Some(why) = cmp.unreadable {
        // 缺失 = 首次发布；坏 = 定稿会整体重写。如实说，不猜、不拦。
        return item.warn(format!("{why} —— 发布事务定稿时会写入 / 重写"), Vec::new());
    }
    if cmp.bad.is_empty() {
        item.pass(format!("{} 条两账一致", cmp.total))
    } else {
        item.warn(
            format!(
                "落后 {} 处（上一版发布的账本）—— 定稿时重写对齐",
                cmp.bad.len()
            ),
            cmp.bad,
        )
    }
}

/// manifest 两账的**严格版**（[`finalize_consistency`] 专用）：Fail + Blocker。
/// 它只在发布事务定稿之后跑 —— 那时 manifest 是刚刚按真实字节写下的，再对不上
/// 就是内部错误或并发改动，必须把发布拦在 commit 之前。
fn manifest_correct_strict(delivery_root: &std::path::Path) -> AuditItem {
    let item = AuditItem::base(
        "manifest/correct",
        "manifest 与交付集合一致",
        AuditSeverity::Blocker,
        "定稿刚写下的账不该对不上 —— 重跑一次发布；反复红是 bug，去查并发改动",
    );
    let cmp = compare_ledgers(delivery_root);
    if let Some(why) = cmp.unreadable {
        return item.fail(why, Vec::new());
    }
    if cmp.bad.is_empty() {
        item.pass(format!("{} 条两账一致", cmp.total))
    } else {
        item.fail(format!("{} 处两账脱节", cmp.bad.len()), cmp.bad)
    }
}

/// ⑮ Git 工作区：**除 `presets/delivery/` 之外**没有别的改动，而且不在游离 HEAD 上。
///
/// `presets/delivery/` 必须排除 —— 它就是这次要提交的产物本身，拿它的未跟踪状态
/// 去拦自己的发布是个死锁（与 `scripts/publish-presets.mjs` 同一条口径）。
///
/// 返回值第二部分 = 「这次发布会让 `presets/` 怎么变」的计数（新增/改动/删除，
/// 弹窗顶上那三个数），**从原始 porcelain 状态码解析** —— 不许再从结论行反推。
/// 口径：`presets/` 前缀**含 `presets/delivery/`**（交付物本身就是"发布会动它几份"
/// 要数的对象）；新增 = `?`/`A`，删除 = `D`，其余非空格 = 改动（含 `R`/`C`）。
fn git_clean() -> (AuditItem, usize, usize, usize) {
    git_clean_at(&paths::repo_root())
}

/// ⑮ 的本体：**在给定的工作目录上**判（发布时 = 真仓库；判据里 = 临时造的目录）。
fn git_clean_at(repo: &std::path::Path) -> (AuditItem, usize, usize, usize) {
    let item = AuditItem::base(
        "git/clean",
        "Git 工作区干净",
        AuditSeverity::Blocker,
        "把无关改动提交或收起来；从 main 起发（一次性分支由发布流程自己建）",
    );
    let branch = git(repo, &["symbolic-ref", "--short", "HEAD"]);
    let dirty = git(
        repo,
        &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
    );
    let (Some(branch), Some(dirty)) = (branch, dirty) else {
        // ★ **这一格不适用**：取不到 Git、或取不到分支名（CI 的游离 HEAD 检出是常见形状）。
        //   不是"工作区干净"（不能伪造），也不是 Skipped（Blocker 不许靠 Skipped 混过去）——
        //   如实说"本环境执行不了这条检查"，**发布阻断责任归有 Git 工作区的真实发布环境**
        //   （作者 2026-10-04 裁决；见 `docs/RELEASE-TRANSACTIONS.md` §6 那一句）。
        //   老写法这里是 skip ⇒ PR 上 rust job 恒红（2026-10-04 PR #27 实测）。
        return (
            item.pass(
                "当前执行环境不是可用的 Git 工作区（`git` 不可用 / 不是仓库 / 游离 HEAD）\
                 —— 无法检查工作区变更，该检查不适用",
            ),
            0,
            0,
            0,
        );
    };
    /* porcelain v1 `-z`：条目 = `XY 路径`，NUL 分隔；**改名 / 复制**是
    `XY new` 后跟一个独立的 `old` token —— 不吃掉它，`old` 会被当成一条
    没有状态码的路径混进来（老实现正是在这里把路径当条目解析错位的）。 */
    let mut added = 0usize;
    let mut changed = 0usize;
    let mut removed = 0usize;
    let mut outside: Vec<String> = Vec::new();
    let mut tokens = dirty.split('\0').filter(|t| !t.is_empty()).peekable();
    while let Some(entry) = tokens.next() {
        if entry.len() < 3 {
            continue; // 连 `XY ` 都不够的是残片（改名的 old 段已被下面的 next() 吃掉）
        }
        let code = &entry[..2];
        let path = entry.get(3..).unwrap_or("").trim_matches('"').to_owned();
        if code.starts_with('R') || code.starts_with('C') {
            tokens.next(); // 消费 origPath，不让它冒充独立条目
        }
        if path.starts_with("presets/") {
            if code.contains('?') || code.contains('A') {
                added += 1;
            } else if code.contains('D') {
                removed += 1;
            } else {
                changed += 1;
            }
        }
        if !path.starts_with("presets/delivery/") {
            outside.push(path);
        }
    }
    let counts = (added, changed, removed);
    if !outside.is_empty() {
        return (
            item.fail(
                format!("`presets/delivery/` 之外还有 {} 处改动", outside.len()),
                outside,
            ),
            counts.0,
            counts.1,
            counts.2,
        );
    }
    (
        item.pass(format!(
            "在 `{}` 上，除交付产物外没有别的改动",
            branch.trim()
        )),
        counts.0,
        counts.1,
        counts.2,
    )
}

/* ---------- 小工具 ---------- */

/// 跑一条 git 命令。**取不到就 `None`**（不是错误）：发布闸不因为"没有 git"就整个停摆
fn git(repo: &std::path::Path, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 发布闸的编号清单 —— **与 `docs/PUBLISH-ARCHITECTURE.md` §5.2 那张表同一个契约**。
    /// 改编号要连文档一起改，所以这里把它钉死。
    const IDS: [&str; 16] = [
        "sources/complete",
        "refs/resolve",
        "assets/exist",
        "assets/path-unique",
        "presets/renderable",
        "presets/rendered",
        "catalog/matches-files",
        "catalog/sha-size",
        "catalog/no-phantoms",
        "delivery/no-strays",
        "bundles/closure",
        "version/structure",
        "source/correct",
        "manifest/correct",
        "git/clean",
        "client/url-reconciliation",
    ];

    /// **发布闸的形状与闸门**（第二刀的主判据）。
    ///
    /// 它咬三件事：① 十六项一项不少、顺序照契约；② 每一行都有名有结论有「去修」；
    /// ③ **`can_publish` 与"有没有 Blocker 红"严格等价** —— 这是"任何一项红
    /// 就不许进 commit / push / PR"那条规矩在代码里的唯一落点。
    ///
    /// ★ 它**不断言"今天全绿"**：真仓库此刻可能就是有待修的东西（那正是发布闸
    /// 该报出来的）。断言的是**闸门本身的正确性**。
    #[test]
    fn the_gate_lists_every_item_and_only_opens_when_no_blocker_fails() {
        let audit = publish_audit().expect("发布闸跑得起来");

        let got: Vec<&str> = audit.items.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(
            got, IDS,
            "十六项一项不少、顺序照 §5.2（⑯ 为 2026-10-05 寻址改造新增）"
        );

        for i in &audit.items {
            assert!(!i.name.is_empty(), "{} 该有名字", i.id);
            assert!(
                !i.details.is_empty(),
                "{} 该有一句话结论（红的是什么）",
                i.id
            );
            assert!(!i.fix_hint.is_empty(), "{} 该说去哪儿修", i.id);
            // ★ Blocker 不许 Skipped：把"没实现"塞进"能拦发布"的那一档，等于假装跑过了
            assert!(
                !(i.severity == AuditSeverity::Blocker && i.status == AuditStatus::Skipped),
                "{} 是 Blocker 却 Skipped —— 要么真跑，要么把分量降成 Warning",
                i.id
            );
        }
        // `Skipped` **第三刀之后一项都不该有**：全链闭环了。
        // （这条断言是刻意留着的：将来加一项还没实现的检查，它必须显式 Skipped —— 那会红，
        //  提醒你把它要么做完、要么明确摆出来，而不是悄悄 Pass。）
        let skipped: Vec<&str> = audit
            .items
            .iter()
            .filter(|i| i.status == AuditStatus::Skipped)
            .map(|i| i.id.as_str())
            .collect();
        assert!(
            skipped.is_empty(),
            "十五项今天都真跑了，不该有 Skipped：{skipped:?}"
        );

        // ⑫ 第三刀之后**不再是 Skipped**，而且它是个**阻断项**（没有规则 = 不准发）
        let structure = audit
            .items
            .iter()
            .find(|i| i.id == "version/structure")
            .expect("⑫ 在");
        assert_ne!(
            structure.status,
            AuditStatus::Skipped,
            "minVersion 已经做了，不许再 Skip"
        );
        assert_eq!(
            structure.severity,
            AuditSeverity::Blocker,
            "「没有规则 = 不准发」是阻断项"
        );

        // ★ 闸门：`can_publish` 与"有没有 Blocker 红"严格等价
        assert_eq!(
            audit.can_publish,
            audit.blockers() == 0,
            "「能不能发布」必须等于「没有任何 Blocker 红」"
        );
        // 规则表登记了这一代 ⇒ ⑫ 就该把最低客户端版本报出来（不许只判不报）
        assert!(
            audit.min_version.is_some(),
            "规则表里登记了当前签名，⑫ 就该给出 minClient。实测：{}",
            structure.details
        );
        assert!(
            structure.details.contains("结构代次"),
            "⑫ 的结论要把代次与签名摆出来：{}",
            structure.details
        );
    }

    /// ★ **没有规则 = 不准发**（第三刀的核心规矩）。
    ///
    /// 拿一个**没有规则表**的发布根驱动 ⑫：必须是 Blocker Fail —— 不许"看起来兼容就
    /// 放过去"。而且详情/`fixHint` 里要把**该登记的那一行**印出来：人只需要想清楚
    /// `minClient` 填什么（那是产品决定），不该去猜格式。
    #[test]
    fn an_unregistered_structure_generation_blocks_the_publish() {
        let tmp = tempfile::tempdir().expect("临时目录");
        let delivery_root = tmp.path().join("delivery");
        std::fs::create_dir_all(&delivery_root).expect("建目录");

        let (item, min_version) = structure_gate(&delivery_root);
        assert_eq!(item.id, "version/structure");
        assert_eq!(item.status, AuditStatus::Fail, "没登记就该红");
        assert_eq!(
            item.severity,
            AuditSeverity::Blocker,
            "「没有规则 = 不准发」—— 这一档必须是 Blocker"
        );
        assert!(min_version.is_none(), "没登记就不该编一个版本号出来");
        assert!(
            item.details.contains("没有登记"),
            "红的是什么要说清：{}",
            item.details
        );
        assert!(
            item.fix_hint.contains("[[signature]]") && item.fix_hint.contains("minClient"),
            "要把该登记的那一行印出来，实测：{}",
            item.fix_hint
        );
        assert_eq!(
            item.affected_files,
            ["presets/structure-signatures.toml"],
            "「去修」要指到那份文件上"
        );
    }

    /// 反过来说：真仓库这一代登记过了，⑫ 就是 Pass 且报得出最低客户端版本。
    #[test]
    fn the_registered_generation_passes_and_reports_the_minimum_client() {
        let (item, min_version) = structure_gate(&paths::delivery_root_path());
        assert_eq!(
            item.status,
            AuditStatus::Pass,
            "真仓库这一代登记过，该绿。实测：{} / {}",
            item.details,
            item.fix_hint
        );
        let min = min_version.expect("Pass 就该给出最低客户端版本");
        assert!(!min.trim().is_empty(), "最低客户端版本不许是空串");
        assert!(
            item.details.contains(&min),
            "结论里要摆出这个版本：{}",
            item.details
        );
    }

    /// **判据 4：闸门 + 「一个入口」**（`docs/PUBLISH-ARCHITECTURE.md` §6）。
    ///
    /// 咬两件事：
    ///
    /// 1. **`can_publish` 与"有没有 Blocker 红"严格等价** —— "任何一项红就不进
    ///    commit / push / PR"那条硬规矩在代码里唯一的落点；
    /// 2. **界面按钮与测试走的是同一个判定**：命令壳 `wb_publish_audit` 只做
    ///    `with_ctx` + trace，里面算的就是 [`publish_audit`]。两边一旦分岔（界面自己
    ///    再实现一套检查 —— 那正是「发布闸」之前的散落老病根），这条立刻红。
    #[test]
    fn publish_audit_all_blockers_pass() {
        let core = publish_audit().expect("核心跑得起来");
        let shell = crate::workbench::app::build::wb_publish_audit().expect("命令壳跑得起来");

        let ids =
            |a: &PublishAudit| -> Vec<String> { a.items.iter().map(|i| i.id.clone()).collect() };
        assert_eq!(ids(&shell), ids(&core), "壳与核心算的不是同一张表");
        assert_eq!(
            shell.can_publish, core.can_publish,
            "壳与核心给的结论必须一致（界面看到的 = 判据断言的）"
        );
        assert_eq!(
            shell.blockers(),
            core.blockers(),
            "壳与核心数的 Blocker 必须一致"
        );

        assert_eq!(
            core.can_publish,
            core.blockers() == 0,
            "「能不能发布」必须等于「没有任何 Blocker 红」"
        );
    }

    /// ★ 取不到 Git 的工作区里，⑮ 是 **Pass（该检查不适用）** —— 不是 Skipped、也不是"干净"。
    ///
    /// 为什么必须有这一条：CI（`pull_request` 是游离 HEAD 检出）里 ⑮ 取不到分支名 —— 老写法
    /// 走 `skip`，撞上「Blocker 不许 Skipped」那条总闸 ⇒ **PR 上 rust job 恒红**
    /// （2026-10-04 PR #27 实测，本地却恒绿）。裁决（作者 2026-10-04）：
    /// 这一格在"本环境执行不了"时如实说明并放行，**发布阻断责任归有 Git 工作区的真实发布环境**。
    #[test]
    fn a_gitless_workspace_marks_git_clean_as_not_applicable() {
        let run_git = |dir: &std::path::Path, args: &[&str]| -> bool {
            std::process::Command::new("git")
                .args(args)
                .current_dir(dir)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        };

        // ① 不是 Git 仓库的目录（临时目录的常态）—— CI 的真实形状之一
        let dir = tempfile::tempdir().expect("临时目录");
        let (item, added, changed, removed) = git_clean_at(dir.path());
        assert_eq!(
            (added, changed, removed),
            (0, 0, 0),
            "检查不适用时计数如实归零"
        );
        assert_eq!(
            item.status,
            AuditStatus::Pass,
            "取不到 Git ⇒ 该检查不适用（Pass）：{}",
            item.details
        );
        if !run_git(dir.path(), &["rev-parse", "--git-dir"]) {
            // 只有在"确实不在任何仓库里"时才钉文案（临时目录万一落在仓库里就不冤枉它）
            assert!(
                item.details.contains("不适用") && item.details.contains("Git 工作区"),
                "结论要把话说清楚（不是 Git 工作区 / 该检查不适用），不许写成\"工作区干净\"：{}",
                item.details
            );
        }

        // ② 游离 HEAD（CI 检出的另一种形状）：仍是 Pass
        let dir = tempfile::tempdir().expect("临时目录");
        // 初始分支不叫 main：本机装了「禁止在 main 直接提交」的全局钩子（CI 上没有，
        // 但这条判据要在两边都绿）；`gpgsign=false` 同理 —— 别让本机全局配置左右判据。
        if !run_git(dir.path(), &["init", "-q", "-b", "topic"]) {
            return; // 这台机器没有 git ⇒ ① 已经覆盖了"取不到"那一支
        }
        assert!(run_git(
            dir.path(),
            &[
                "-c",
                "user.email=test@example.com",
                "-c",
                "user.name=test",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-q",
                "--allow-empty",
                "-m",
                "x",
            ]
        ));
        assert!(run_git(dir.path(), &["checkout", "-q", "--detach"]));
        let (item, ..) = git_clean_at(dir.path());
        assert_eq!(
            item.status,
            AuditStatus::Pass,
            "游离 HEAD ⇒ 该检查不适用（Pass）：{}",
            item.details
        );
    }

    /* ---------- ⑭ / ③½ / ⑮ 计数 —— 两层检查的判据（2026-10-05） ---------- */

    use crate::workbench::domain::patch::{Committed, CommittedVersion};
    use crate::workbench::domain::testkit::{fixture_catalog, Fixture};

    /// 夹具 Committed（与 `delivery_root.rs` tests 的 committed() 同一份清单）
    fn committed() -> Committed {
        let mut versions = std::collections::BTreeMap::new();
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
        Committed {
            versions,
            catalog: fixture_catalog(),
            ..Default::default()
        }
    }

    /// 完整交付夹具：与 `delivery_root.rs` 发布全链测试同一份形状（A 类原地件在发布根的
    /// `assets/` 下，mkp 产物在 `delivery/mkp/presets/` 下）。
    struct Delivery {
        publish_root: tempfile::TempDir,
        asset_root: tempfile::TempDir,
        presets: Fixture,
        committed: Committed,
        draft: crate::workbench::domain::patch::Draft,
    }

    impl Delivery {
        fn book(&self) -> Book<'_> {
            Book::new(&self.presets.presets, &self.committed, &self.draft)
        }

        /// 交付根（`<publish_root>/delivery`）
        fn delivery_root(&self) -> std::path::PathBuf {
            self.publish_root.path().join("delivery")
        }

        /// 未定稿：delivery_root 里只有 mkp 产物 —— 「生成过、还没发」的形状，三本账还没写
        fn without_publish() -> Self {
            let publish_root = tempfile::tempdir().unwrap();
            let asset_root = tempfile::tempdir().unwrap();
            let mut presets = Fixture::load();
            for rel in [
                "printers/a1.webp",
                "icons/a1.svg",
                "bbs/A1/process.json",
                "icons/p1s.svg",
            ] {
                crate::fsx::atomic::atomic_write(&asset_root.path().join(rel), b"payload").unwrap();
            }
            presets.presets.set_asset_root(asset_root.path());
            let committed = committed();
            let draft = crate::workbench::domain::patch::Draft::default();
            let book = Book::new(&presets.presets, &committed, &draft);
            let dist_dir = publish_root.path().join("delivery");
            std::fs::create_dir_all(dist_dir.join(delivery::MKP_PRESETS_DIR)).unwrap();
            // A 类原地件（与生产布局同形：发布根的 `assets/` 下，不在 `delivery_root/` 里）
            for a in delivery::referenced_assets(&book) {
                let Some(rel) = crate::runtime::catalog::dest_of_asset(&a) else {
                    continue;
                };
                let dst = publish_root.path().join(&rel);
                if let Some(parent) = dst.parent() {
                    std::fs::create_dir_all(parent).unwrap();
                }
                std::fs::copy(asset_root.path().join(&a.path), &dst).unwrap();
            }
            // mkp 产物（wb_generate 的等价物：文件在，内容任意）
            for v in book.versions() {
                if book.build_state(&v.uid) == crate::workbench::domain::BuildState::NoResources {
                    continue;
                }
                crate::fsx::atomic::atomic_write(
                    &dist_dir.join(delivery::MKP_PRESETS_DIR).join(&v.mkp_file),
                    format!("# preset {}", v.mkp_file).as_bytes(),
                )
                .unwrap();
            }
            Self {
                publish_root,
                asset_root,
                presets,
                committed,
                draft,
            }
        }

        /// 定稿态：跑过 `publish_into` —— manifest / catalog / source 三账一致
        fn published() -> Self {
            let fx = Self::without_publish();
            let book = fx.book();
            let meta = delivery::PublishMeta {
                stamp: "2026-10-05T00:00:00Z".to_owned(),
                channel: "stable".to_owned(),
                version: String::new(),
            };
            delivery::publish_into(&fx.delivery_root(), fx.asset_root.path(), &book, &meta)
                .expect("定稿");
            fx
        }
    }

    /// 把 manifest 里第一条的 sha 改坏 —— 「生成重写了产物、manifest 还是上一版」
    /// 的最小等价物
    fn corrupt_first_manifest_sha(fx: &Delivery) {
        let manifest_path = fx.delivery_root().join(delivery::MANIFEST_FILE);
        let mut m: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
        m["assets"][0]["sha256"] = serde_json::Value::String("0".repeat(64));
        crate::fsx::atomic::atomic_write_json(&manifest_path, &m).unwrap();
    }

    /// ③½ 里红掉的项目 id（Blocker + Fail，与 `can_publish` 同一个判据）
    fn finalize_red_items(book: &Book<'_>, dist_path: &std::path::Path) -> Vec<String> {
        finalize_consistency(book, dist_path)
            .iter()
            .filter(|i| i.severity == AuditSeverity::Blocker && i.status == AuditStatus::Fail)
            .map(|i| i.id.clone())
            .collect()
    }

    /// **发布时刻进目录**（2026-10-06）：发布写的 catalog.json 带 `publishedAt`，
    /// 而且与 manifest 的 `updated` 是**同一个戳** —— 一次发布事件只有一个时间，
    /// 客户端云端表的「时间」列（`catalog.publishedAt`）显示的就是它。
    /// 随包 bootstrap 目录刻意不带这一格（`gen-catalog` 产物逐字节可复现），那是另一条路的裁决。
    #[test]
    fn the_published_catalog_carries_the_publish_stamp() {
        let fx = Delivery::published();
        let catalog: serde_json::Value = serde_json::from_slice(
            &std::fs::read(fx.delivery_root().join(delivery::NEW_CATALOG_FILE)).unwrap(),
        )
        .unwrap();
        assert_eq!(
            catalog["publishedAt"], "2026-10-05T00:00:00Z",
            "publishedAt = 发布侧落的那个戳"
        );
        let manifest: serde_json::Value = serde_json::from_slice(
            &std::fs::read(fx.delivery_root().join(delivery::MANIFEST_FILE)).unwrap(),
        )
        .unwrap();
        assert_eq!(
            manifest["updated"], catalog["publishedAt"],
            "目录与清单不许各说一个时间"
        );
    }

    /// **场景 A（预检侧）**：manifest 落后于交付物 —— 预检 ⑭ 是提示档（Warn），
    /// **不许**把发布拦死（老实现这里是 Blocker，构成「只能靠发布修、发布又进不去」
    /// 的死循环）。同一状态下严格版（③½ 用的那把尺）必须红 —— 两层各管各的。
    #[test]
    fn a_stale_manifest_warns_but_does_not_block_the_publish() {
        let fx = Delivery::published();
        corrupt_first_manifest_sha(&fx);

        let pre = manifest_correct(&fx.delivery_root());
        assert_eq!(pre.severity, AuditSeverity::Warning, "预检 ⑭ 是提示档");
        assert_eq!(pre.status, AuditStatus::Warn, "落后 = Warn，不是 Fail");
        assert_eq!(pre.affected_files.len(), 1, "要列出来落后的是哪几条");
        assert!(
            !pre.fix_hint.contains("restore"),
            "「去修」不许再让人手工 restore 交付根绕过：{}",
            pre.fix_hint
        );

        let strict = manifest_correct_strict(&fx.delivery_root());
        assert_eq!(strict.severity, AuditSeverity::Blocker);
        assert_eq!(strict.status, AuditStatus::Fail, "严格版在同一状态下必须红");
    }

    /// **场景 A（事务侧）**：落后状态直接跑 `publish_into`（run() 第③步调的就是它）
    /// —— manifest 按真实字节重写，随后 ③½ 全绿。「重跑一次发布」从此真的是修法。
    #[test]
    fn publish_into_heals_a_stale_manifest() {
        let fx = Delivery::published();
        corrupt_first_manifest_sha(&fx);

        let book = fx.book();
        let meta = delivery::PublishMeta {
            stamp: "2026-10-05T01:00:00Z".to_owned(),
            channel: "stable".to_owned(),
            version: String::new(),
        };
        delivery::publish_into(&fx.delivery_root(), fx.asset_root.path(), &book, &meta)
            .expect("定稿");

        for i in finalize_consistency(&book, &fx.delivery_root()) {
            assert_eq!(i.status, AuditStatus::Pass, "③½ 必须全绿：{}", i.id);
        }
    }

    /// **首次发布**：manifest 还不存在 —— 预检提示、不拦。老实现这里是 Blocker，
    /// 和「落后」一样是死锁：第一发永远发不出去。
    #[test]
    fn a_missing_manifest_is_the_first_publish_not_a_deadlock() {
        let fx = Delivery::without_publish();
        let item = manifest_correct(&fx.delivery_root());
        assert_eq!(item.severity, AuditSeverity::Warning);
        assert_eq!(
            item.status,
            AuditStatus::Warn,
            "manifest 还不存在 = 首次发布，不拦"
        );
        assert!(
            item.details.contains("不存在"),
            "要说清是哪本账、什么问题：{}",
            item.details
        );
    }

    /// **场景 C（负向）**：定稿之后交付根被动过 —— ③½ 必须红，发布停在 commit 之前
    /// （`run()` 里 Err 短路，不会产生半截发布提交）。
    #[test]
    fn finalize_consistency_catches_a_tampered_delivery() {
        // ① 篡改一份交付文件的字节 → catalog 登记的 SHA 对不上真字节
        let fx = Delivery::published();
        let victim = fx
            .delivery_root()
            .join(delivery::MKP_PRESETS_DIR)
            .join("A1-fast.toml");
        crate::fsx::atomic::atomic_write(&victim, b"tampered").unwrap();
        let red = finalize_red_items(&fx.book(), &fx.delivery_root());
        assert!(
            red.iter().any(|id| id == "catalog/sha-size"),
            "字节被改要被抓住：{red:?}"
        );

        // ② 篡改 manifest 里一条 sha → manifest 严格核对红
        let fx = Delivery::published();
        corrupt_first_manifest_sha(&fx);
        let red = finalize_red_items(&fx.book(), &fx.delivery_root());
        assert!(
            red.iter().any(|id| id == "manifest/correct"),
            "账被改要被抓住：{red:?}"
        );
    }

    /// 临时 Git 仓库（`git.rs` tests 同款纪律：任务分支上提交，绕开本机
    /// 「禁止在 main 直接提交」的全局钩子）。
    fn git_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let ok = |args: &[&str]| {
            assert!(
                std::process::Command::new("git")
                    .args(args)
                    .current_dir(dir.path())
                    .output()
                    .expect("起 git")
                    .status
                    .success(),
                "git {args:?} 失败"
            );
        };
        ok(&["init", "-q"]);
        ok(&["config", "user.email", "t@example.com"]);
        ok(&["config", "user.name", "t"]);
        ok(&["config", "commit.gpgsign", "false"]);
        ok(&["checkout", "-q", "-b", "publish/demo"]);
        dir
    }

    /// **场景 B**：`presets/delivery/` 之外的源码未提交 —— ⑮ 依旧是 Blocker，
    /// 发布照样拦。这次修复只把交付账本挪到事务里，**不放宽源状态**。
    #[test]
    fn uncommitted_source_outside_dist_still_blocks_the_publish() {
        let dir = git_repo();
        let write = |rel: &str, bytes: &[u8]| {
            let p = dir.path().join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            crate::fsx::atomic::atomic_write(&p, bytes).unwrap();
        };
        let git = |args: &[&str]| {
            assert!(
                std::process::Command::new("git")
                    .args(args)
                    .current_dir(dir.path())
                    .output()
                    .unwrap()
                    .status
                    .success(),
                "git {args:?} 失败"
            );
        };
        write("presets/delivery/mkp/presets/A1-fast.toml", b"v1");
        write("src/lib.rs", b"code");
        git(&["add", "-A"]);
        git(&["commit", "-q", "-m", "baseline"]);

        write("src/lib.rs", b"code v2");
        let (item, added, changed, removed) = git_clean_at(dir.path());
        assert_eq!(item.status, AuditStatus::Fail);
        assert_eq!(
            item.severity,
            AuditSeverity::Blocker,
            "⑮ 保持 Blocker：源状态必须可追溯"
        );
        assert!(
            item.affected_files.iter().any(|p| p.contains("src/lib.rs")),
            "红清单要指名是哪处：{:?}",
            item.affected_files
        );
        assert_eq!(
            (added, changed, removed),
            (0, 0, 0),
            "presets/ 没动，计数就是 0"
        );
    }

    /// **场景 D**：弹窗那三个数 = **真实 Git 状态**。老实现恒 0/0/0：
    /// 从结论行反推，而结论行剥掉了状态码、又按定义不含 `presets/delivery/`，双重错位。
    #[test]
    fn git_counts_reflect_real_presets_changes() {
        let dir = git_repo();
        let write = |rel: &str, bytes: &[u8]| {
            let p = dir.path().join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            crate::fsx::atomic::atomic_write(&p, bytes).unwrap();
        };
        let git = |args: &[&str]| {
            assert!(
                std::process::Command::new("git")
                    .args(args)
                    .current_dir(dir.path())
                    .output()
                    .unwrap()
                    .status
                    .success(),
                "git {args:?} 失败"
            );
        };
        // 基线：两份交付物 + 一份源 + 一份将被改名的交付物
        write("presets/delivery/mkp/presets/A1-fast.toml", b"v1");
        write("presets/delivery/mkp/presets/gone.toml", b"bye");
        write("presets/delivery/mkp/presets/old-name.toml", b"old");
        write("presets/registry/param_registry.toml", b"v1");
        write("src/lib.rs", b"code");
        git(&["add", "-A"]);
        git(&["commit", "-q", "-m", "baseline"]);

        // 工作区：M 一份交付物、?? 新增一份源、D 删一份交付物、R 改名一份交付物、
        // M 一份无关源（只进 ⑮ 的红清单，不进计数）
        write("presets/delivery/mkp/presets/A1-fast.toml", b"v2");
        write("presets/registry/new.toml", b"new");
        write("src/lib.rs", b"code v2");
        std::fs::remove_file(dir.path().join("presets/delivery/mkp/presets/gone.toml")).unwrap();
        git(&[
            "mv",
            "presets/delivery/mkp/presets/old-name.toml",
            "presets/delivery/mkp/presets/new-name.toml",
        ]);

        let (item, added, changed, removed) = git_clean_at(dir.path());
        assert_eq!(
            (added, changed, removed),
            (1, 2, 1),
            "?? 新增 1、M+R 改动 2、D 删除 1 —— 真实状态，不是恒 0"
        );
        assert_eq!(item.status, AuditStatus::Fail, "有无关源改动，⑮ 照样红");
        assert!(
            item.affected_files
                .iter()
                .all(|p| !p.starts_with("presets/delivery/")),
            "⑮ 的红清单仍排除 presets/delivery/（交付产物不拦自己）：{:?}",
            item.affected_files
        );
        assert!(
            item.affected_files
                .iter()
                .all(|p| !p.contains("old-name.toml")),
            "改名的 origPath 段不许冒充独立条目：{:?}",
            item.affected_files
        );
    }
}
