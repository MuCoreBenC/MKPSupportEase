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

use serde::Serialize;

use super::build::Scope;
use super::dist;
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
    with_ctx(|ctx| {
        let (c, d, _) = state(ctx)?;
        let book = Book::new(&ctx.presets, &c, &d);
        let dist = paths::dist_root_path();
        // ★ 只读定位（`assets_root_path`），不是 `assets_root()` —— 后者会 `create_dir_all`，
        // 一个自称「只读：不写盘」的闸不该顺手造出一个目录
        let asset_root = paths::assets_root_path();

        let mut items: Vec<AuditItem> = Vec::with_capacity(15);

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

        /* ⑥ 渲染结果真在 dist/mkp/presets 下 */
        items.push(presets_rendered(&book, &dist));

        /* ⑦ catalog 登记的每一条都取得到 */
        items.push(registered_paths_reachable(&ctx.presets, &dist));

        /* ⑧ SHA / size 对真字节算且一致 */
        items.push(catalog_sha_size(&dist));

        /* ⑨ 没有幽灵条目（空 path / 非法 path） */
        items.push(no_phantoms(&ctx.presets));

        /* ⑩ dist 里没有残留 */
        items.push(no_strays(&book, &dist));

        /* ⑪ 套餐引用闭包完整 */
        items.push(bundles_closure(&ctx.presets));

        /* ⑫ 结构代次与最低客户端版本（第三刀：不再是 Skipped） */
        let (structure, min_version) = structure_gate(&dist);
        items.push(structure);

        /* ⑬ source.json 正确 */
        items.push(source_correct(&dist));

        /* ⑭ manifest 与交付集合一致 */
        items.push(manifest_correct(&dist));

        /* ⑮ Git 工作区状态 */
        let git = git_clean();
        let (added, changed, removed) = git_counts(&git);
        /* ★ ⑮ **必须先入列再判定** —— 否则 `can_publish` 看不到它：
        脏工作区（⑮ 红）时闸仍会亮「确认发布」，绕过「除交付产物外工作区必须干净」那条保护。
        入列顺序与 `blockers()`（它也遍历全部 items）一致，两者才严格等价。 */
        items.push(git);

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
    })
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

/// ⑥ 渲染结果真在 `dist/mkp/presets/` 下（`dist_expected_set` 就是"该有的那批"）
fn presets_rendered(book: &Book<'_>, dist: &std::path::Path) -> AuditItem {
    let item = AuditItem::base(
        "presets/rendered",
        "渲染产物真在交付目录里",
        AuditSeverity::Blocker,
        "在生成视角里生成一次（收尾会自动补资产并重算 catalog）",
    );
    let mut missing: Vec<String> = Vec::new();
    for rel in dist::dist_expected_set(book) {
        if rel.starts_with(dist::MKP_PRESETS_DIR) && !dist.join(&rel).is_file() {
            missing.push(rel);
        }
    }
    if missing.is_empty() {
        item.pass("该生成的那几份都在 dist/mkp/presets/ 下")
    } else {
        missing.sort();
        item.fail(format!("{} 份产物还没生成", missing.len()), missing)
    }
}

/// ⑦ 目录登记的每一条，`发布根 + catalog.path` 处都取得到。
/// ★ 这正是第一刀之后必须重写的那一项（A 类不在 dist 里，核对方向要反过来）
fn registered_paths_reachable(
    presets: &crate::presetdata::Presets,
    dist: &std::path::Path,
) -> AuditItem {
    let item = AuditItem::base(
        "catalog/matches-files",
        "登记的每一条都取得到",
        AuditSeverity::Blocker,
        "生成一次让清单重算；A 类资产要放回 `presets/assets/`",
    );
    let catalog = crate::runtime::catalog::Catalog::build_from_presets_lenient(
        presets,
        &dist.join(dist::MKP_PRESETS_DIR),
    );
    let publish_root = match dist.parent() {
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
fn catalog_sha_size(dist: &std::path::Path) -> AuditItem {
    let item = AuditItem::base(
        "catalog/sha-size",
        "SHA / 大小对真字节算且一致",
        AuditSeverity::Blocker,
        "生成一次（清单会跟着重算），别手改 catalog.json",
    );
    let Ok(bytes) = std::fs::read(dist.join(dist::NEW_CATALOG_FILE)) else {
        return item.fail(
            "dist/catalog.json 还不存在 —— 先生成一次".to_owned(),
            Vec::new(),
        );
    };
    let Ok(catalog) = serde_json::from_slice::<crate::runtime::catalog::Catalog>(&bytes) else {
        return item.fail("dist/catalog.json 解析不出来".to_owned(), Vec::new());
    };
    let publish_root = match dist.parent() {
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
        if got.len() as u64 != want_size || dist::sha256_of(&got) != want_sha {
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

fn no_strays(book: &Book<'_>, dist: &std::path::Path) -> AuditItem {
    let item = AuditItem::base(
        "dist/no-strays",
        "交付目录里没有残留",
        AuditSeverity::Blocker,
        "点「清理残留」（进 workbench/.trash/dist/<时间戳>/，可还原）",
    );
    let strays = dist::scan_strays(dist, &dist::dist_expected_set(book));
    if strays.is_empty() {
        item.pass("dist 里没有本次交付集合之外的文件")
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
/// 钉着这一条。用随包那份的额外好处：闸**不依赖 dist 当前状态**（结构该是什么，
/// 是代码的事，不是这批文件的事）。
fn structure_gate(dist: &std::path::Path) -> (AuditItem, Option<String>) {
    use crate::runtime::structure;

    let item = AuditItem::base(
        "version/structure",
        "结构代次与最低客户端版本",
        AuditSeverity::Blocker,
        "在 presets/structure-signatures.toml 里登记这一代的签名与最低客户端版本",
    );
    let Some(publish_root) = dist.parent() else {
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
fn source_correct(dist: &std::path::Path) -> AuditItem {
    let item = AuditItem::base(
        "source/correct",
        "Bootstrap（source.json）正确",
        AuditSeverity::Blocker,
        "重跑一次发布（`publish_into` 收尾会重写它）",
    );
    let source_path = dist.join(dist::SOURCE_FILE);
    let Ok(bytes) = std::fs::read(&source_path) else {
        return item.fail("dist/source.json 还不存在".to_owned(), Vec::new());
    };
    /*
     * ★ **本地校验，不走远端解析器**：这一格验的是**交付根里那一份文件**，
     * 不该用要求 http URL 的 `parse_bootstrap`（那是给"从远端 URL 取回来的字节"用的）——
     * 它会在 baseUrl 缺省时去本地路径上回退目录，本地绝对路径不是 URL ⇒ 报错 ⇒
     * "文件明明合法、闸却判解析不出来"。本地校验只验内容（JSON / 代次 / catalog 相对路径）。
     */
    let catalog_rel = match crate::runtime::source::validate_bootstrap_local(&bytes) {
        Ok(rel) => rel,
        Err(e) => return item.fail(e.message.clone(), Vec::new()),
    };
    /* 核它指的那份目录文件真在交付根里（本地 = `dist/<catalog_rel>`） */
    let local = dist.join(&catalog_rel);
    if local.is_file() {
        item.pass(format!("认得出来，指向 {catalog_rel}"))
    } else {
        item.fail(
            format!("它指向的 {catalog_rel} 不在交付根里"),
            vec![catalog_rel],
        )
    }
}

/// ⑭ manifest 与 catalog 交叉：manifest 里每一条都在 catalog 里、且 SHA 一致
fn manifest_correct(dist: &std::path::Path) -> AuditItem {
    let item = AuditItem::base(
        "manifest/correct",
        "manifest 与交付集合一致",
        AuditSeverity::Blocker,
        "重跑一次发布（manifest 在最后一步写）",
    );
    let Ok(bytes) = std::fs::read(dist.join(dist::MANIFEST_FILE)) else {
        return item.fail("dist/manifest.json 还不存在".to_owned(), Vec::new());
    };
    let Ok(manifest) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return item.fail("dist/manifest.json 解析不出来".to_owned(), Vec::new());
    };
    let Ok(catalog_bytes) = std::fs::read(dist.join(dist::NEW_CATALOG_FILE)) else {
        return item.fail("dist/catalog.json 还不存在".to_owned(), Vec::new());
    };
    let Ok(catalog) = serde_json::from_slice::<crate::runtime::catalog::Catalog>(&catalog_bytes)
    else {
        return item.fail("dist/catalog.json 解析不出来".to_owned(), Vec::new());
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
    if bad.is_empty() {
        item.pass(format!("{n} 条两账一致"))
    } else {
        bad.sort();
        item.fail(format!("{} 处两账脱节", bad.len()), bad)
    }
}

/// ⑮ Git 工作区：**除 `presets/dist/` 之外**没有别的改动，而且不在游离 HEAD 上。
///
/// `presets/dist/` 必须排除 —— 它就是这次要提交的产物本身，拿它的未跟踪状态
/// 去拦自己的发布是个死锁（与 `scripts/publish-presets.mjs` 同一条口径）。
fn git_clean() -> AuditItem {
    let item = AuditItem::base(
        "git/clean",
        "Git 工作区干净",
        AuditSeverity::Blocker,
        "把无关改动提交或收起来；从 main 起发（一次性分支由发布流程自己建）",
    );
    let repo = paths::repo_root();
    let branch = git(&repo, &["symbolic-ref", "--short", "HEAD"]);
    let dirty = git(
        &repo,
        &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
    );
    let (Some(branch), Some(dirty)) = (branch, dirty) else {
        return item.skip("这台机器上取不到 git（`git` 不在 PATH 或不是 Git 仓库）—— 不假装通过");
    };
    let outside: Vec<String> = dirty
        .split('\0')
        .filter(|e| !e.is_empty())
        .filter(|e| {
            let path = e.get(3..).unwrap_or("").trim_matches('"');
            !path.starts_with("presets/dist/")
        })
        .map(|e| e.get(3..).unwrap_or("").to_owned())
        .collect();
    if !outside.is_empty() {
        return item.fail(
            format!("`presets/dist/` 之外还有 {} 处改动", outside.len()),
            outside,
        );
    }
    item.pass(format!(
        "在 `{}` 上，除交付产物外没有别的改动",
        branch.trim()
    ))
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

/// 从 ⑮ 那一格的结论里数出「这次发布会让 presets/ 怎么变」。
/// 只数 `presets/` 下面的（其余是无关改动，上面那条已经拦下了）
fn git_counts(item: &AuditItem) -> (usize, usize, usize) {
    let mut added = 0usize;
    let mut changed = 0usize;
    let mut removed = 0usize;
    for f in &item.affected_files {
        let path = f.trim_matches('"');
        if !path.starts_with("presets/") {
            continue;
        }
        let code = f.get(..2).unwrap_or("  ");
        if code.contains('?') {
            added += 1;
        } else if code.contains('D') {
            removed += 1;
        } else if code.contains('M') {
            changed += 1;
        }
    }
    (added, changed, removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 发布闸的十五项编号 —— **与 `docs/PUBLISH-ARCHITECTURE.md` §5.2 那张表同一个契约**。
    /// 改编号要连文档一起改，所以这里把它钉死。
    const IDS: [&str; 15] = [
        "sources/complete",
        "refs/resolve",
        "assets/exist",
        "assets/path-unique",
        "presets/renderable",
        "presets/rendered",
        "catalog/matches-files",
        "catalog/sha-size",
        "catalog/no-phantoms",
        "dist/no-strays",
        "bundles/closure",
        "version/structure",
        "source/correct",
        "manifest/correct",
        "git/clean",
    ];

    /// **发布闸的形状与闸门**（第二刀的主判据）。
    ///
    /// 它咬三件事：① 十五项一项不少、顺序照契约；② 每一行都有名有结论有「去修」；
    /// ③ **`can_publish` 与"有没有 Blocker 红"严格等价** —— 这是"任何一项红
    /// 就不许进 commit / push / PR"那条规矩在代码里的唯一落点。
    ///
    /// ★ 它**不断言"今天全绿"**：真仓库此刻可能就是有待修的东西（那正是发布闸
    /// 该报出来的）。断言的是**闸门本身的正确性**。
    #[test]
    fn the_gate_lists_every_item_and_only_opens_when_no_blocker_fails() {
        let audit = publish_audit().expect("发布闸跑得起来");

        let got: Vec<&str> = audit.items.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(got, IDS, "十五项一项不少、顺序照 §5.2");

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
        let dist = tmp.path().join("dist");
        std::fs::create_dir_all(&dist).expect("建目录");

        let (item, min_version) = structure_gate(&dist);
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
        let (item, min_version) = structure_gate(&paths::dist_root_path());
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
}
