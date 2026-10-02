//! 交付层（b05 Task 12）：`presets/dist/` 的**目录类 JSON** 与资产复制。
//!
//! # 目录结构定稿（12.1；**2026-10-02 与客户端落点对齐**）
//!
//! 交付根就是「客户端拿 URL 直取的那个根」（数据源地址指向它）。所以这里的相对路径
//! **不是发布侧的自由**，而是契约的一半 —— 客户端下载用的地址是
//! `数据源地址 + catalog 里登记的那个相对位置`：
//!
//! ```text
//! presets/dist/                       ← 数据源地址指向的根
//! ├── catalog.json                    **两端共用契约**（与客户端 runtime::Catalog 同
//! │                                   schema；客户端按 join_url(base,"catalog.json") 取）
//! ├── manifest.json                   发布账：本次发了哪些文件、SHA、大小（最后写）
//! ├── content/                        目录类 JSON（doc §7 的那一面，客户端暂不读）
//! │   ├── machine_catalog.json        机型 + 版本 + 关系（12.2）
//! │   ├── bundles.json                套餐 + 包含什么（12.3）
//! │   └── assets_index.json           资产清单 + 位置 + 归属（12.4）
//! └── mkp/                            产品资源区 —— **与客户端下载区同名同形**
//!     ├── presets/<preset_file_name>  MKP 产物（`wb_generate` 落的，9 份起）
//!     ├── bbs/Process/…               切片器配置
//!     ├── models/…                    3mf 模型
//!     └── icons/…                     机型图标
//! ```
//!
//! **落点由 catalog 说了算**：交付集合里每一个文件的相对路径，都等于它在
//! `catalog.json` 的 `files[].path`（两边共用 [`crate::runtime::catalog::dest_of_asset`]
//! 一处算法）。发布侧**不再自己拼一遍** —— 曾经这里是 `assets/<载荷 path>` +
//! `presets/mkp/<名字>`，而客户端认的落点是 `mkp/<kind 目录>/…`：URL 拼得上、
//! 落点对不上，上传成功、客户端点下载却 404。这一类错在两边各自"看着对"的时候最难查，
//! 所以 [`publish_into`] 收尾时**逐条核对**盘上真字节（见那个函数的最后一段）。
//!
//! **`mkp/presets/` 子层保留**（不收成 `mkp/` 一层）：MKP 预设与 BBS 预设是两类预设
//! （G-3 的切片器开放维度），子层给「按预设类型」留位置，与客户端 `kind_dir` 的分层一致。
//!
//! # 12.5 「文件名字段全部由命名函数算出」的边界
//!
//! - **MKP 产物名**：由 [`crate::workbench::app::build::preset_file_name`]（权威实现
//!   `preset::preset_file_name`）算出，交付侧不出现一个手写文件名；
//! - **资产 path**：是**登记值**（assets.toml 的唯一一份路径），目录 JSON 原样引用、
//!   不重新拼接 —— 「不存在手写字面量」指的是生成器里不再出现第二份名字，不是要求
//!   把登记值改成计算值。BBS 文件名为什么保持登记值见 9.3a（旧云端代号，未裁决）。
//!
//! # sha256 / size 为什么不在这里
//!
//! 那是**交付物**的属性，发布时按真实字节算（Task 13.6，与 manifest 扩容一起做）。
//! assets_index 只管「清单、位置、归属」三样（doc §7）。
//!
//! # 13.x 发布（Task 13）：可达集合、残留拦截、manifest v3
//!
//! - **交付集合**（[`deliverable_set`]）是从引用关系**算出来**的文件相对路径全集 ——
//!   manifest 不维护第二份资产列表，它是这个集合的哈希清单；
//! - **残留拦截**（13.4 / doc §9.1）：发布前扫描交付目录，不在集合内的文件一律列出，
//!   **有残留就中止发布、不写 manifest** —— 残留会被消费端真的下载到，而且它多半是
//!   构建器自己留下的历史产物；[`clean_strays`] 是显式的清理动作，走
//!   `workbench/.trash/dist/<stamp>/` 回收（保留相对路径，可还原），不直接删；
//! - **manifest v3**：assets 扩到**全部交付文件**（mkp_preset 9 条 + 引用集资产 13 条），
//!   **删掉了 `bundles` 字段** —— 那是从上游透传的第二份套餐列表，它的 `assetRefs`
//!   还是旧资产 id 空间（`a1_bbs_mkpprocess…`），跟新的 assets_index 根本 join 不上；
//!   套餐的唯一真相是 `content/bundles.json`。结构变了，`manifestVersion` 升 3。
//! - **两类 id 的边界**（审查点名核实过）：manifest 条目有两种命名空间 ——
//!   `mkp_preset` 条目的 id 就是**产物名**（`A1-standard.toml`，命名规则算出，
//!   资产域 enum 刻意没有 mkpPreset 档，doc §12.5），资产条目的 id 用**资产域**
//!   Asset.id（`a1-image`）。`machine_catalog.json` 版本条目的 `mkpPresetAssetId`
//!   是**连接键**（指向 manifest 里 mkp_preset 条目），不是资产域 Asset ID ——
//!   词汇撞名，域不同。删掉上游之后，这个连接键不再来自上游 manifest，
//!   而是与产物名同一份计算值。
//! - **resourceType 词汇**：新条目用资产域 `kind.key()`（`image` / `icon` / `model` /
//!   `slicerProfile`），不用上游的 `bbs_profile` —— manifest 与 assets_index 是同一批
//!   资产的两种视图，join 键（id + type）必须一致；旧词汇是迁移输入（doc §6.2）。

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::AppError;
use crate::workbench::domain::derive::Book;
use crate::workbench::domain::BuildState;
use crate::workbench::presets::{Asset, AssetKind};

/// 交付根下的**产品资源区**：与客户端下载区 `mkp/` 同名同形（见模块头）。
/// 交付集合里每一个文件都落在它下面，相对路径 = catalog 的 `files[].path`
pub const MKP_DIR: &str = "mkp";
/// MKP 产物在交付根里的子目录（对应客户端 `kind_dir(mkp_preset)` 那一格）
pub const MKP_PRESETS_DIR: &str = "mkp/presets";
/// 目录类 JSON 的子目录
pub const CONTENT_DIR: &str = "content";
/// manifest 的固定文件名
pub const MANIFEST_FILE: &str = "manifest.json";
/// 新世界目录的固定文件名（两端共用契约：与客户端 `runtime::Catalog` 同 schema）
/// —— **与 [`crate::runtime::source::CATALOG_FILE`] 同一个值**（直接引用它：
/// 同一份名字写在两处，迟早有一处改了另一处没改）
pub const NEW_CATALOG_FILE: &str = crate::runtime::source::CATALOG_FILE;
/// 官方源入口文件（Bootstrap）的固定名。客户端拿它解析"catalog 在哪、文件根在哪"
/// （见 `runtime::source::parse_bootstrap`）——第十七刀起它是发布产物的一部分
pub const SOURCE_FILE: &str = "source.json";

/// Bootstrap 的正文：**就两件事** —— schema 代次 + catalog 在哪。
///
/// `baseUrl` **不写**（客户端缺省理解成"与 source.json 同目录"）：同一份 dist 推到
/// 哪里都对；将来要把文件根指向别的 CDN 时才由人加它 —— 那正是 Bootstrap 存在的意义
/// （换部署只改它，客户端不重发）。
pub fn bootstrap_json() -> serde_json::Value {
    serde_json::json!({
        "sourceSchema": crate::runtime::source::BOOTSTRAP_SCHEMA,
        "catalog": NEW_CATALOG_FILE,
    })
}

/// 官方源（Bootstrap）地址的**规范化** —— 工作台输入侧的唯一一处。
///
/// - **GitHub 的 blob 页地址**（人从浏览器地址栏复制的那个）→ raw 直取地址：
///   `https://github.com/<owner>/<repo>/blob/<ref>/<path>`
///   → `https://raw.githubusercontent.com/<owner>/<repo>/<ref>/<path>`
///   （ref 里有斜杠的罕见分支名不猜——那种情况请直接填 raw 地址）
/// - 已经是 `raw.githubusercontent.com` / 别的 http(s)：原样通过（自建源合法）
/// - `github.com` 但**不是 blob 页**（仓库首页 / tree 目录页）→ 拒，说清"那是个目录"
/// - 其余（非 http(s)）→ 拒
pub fn normalize_bootstrap_url(raw: &str) -> Result<String, AppError> {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(AppError::invalid_argument("Bootstrap 地址是空的"));
    }
    if let Some(rest) = trimmed
        .strip_prefix("https://github.com/")
        .or_else(|| trimmed.strip_prefix("http://github.com/"))
    {
        let parts: Vec<&str> = rest.split('/').collect();
        /* github.com/<owner>/<repo>/blob/<ref>/<path…> */
        if parts.len() >= 5 && parts[2] == "blob" && !parts[4].is_empty() {
            return Ok(format!(
                "https://raw.githubusercontent.com/{}/{}/{}/{}",
                parts[0],
                parts[1],
                parts[3],
                parts[4..].join("/")
            ));
        }
        return Err(AppError::invalid_argument(
            "这是 GitHub 的仓库 / 目录页，不是一个文件 —— 请填指向 source.json 的 blob 链接（我们会自动转成 raw）",
        )
        .with_detail(format!("收到：{trimmed}")));
    }
    let ok = trimmed.starts_with("http://") || trimmed.starts_with("https://");
    if !ok {
        return Err(AppError::invalid_argument(format!(
            "Bootstrap 地址只认 http:// 或 https://，填进来的是 {trimmed}"
        )));
    }
    Ok(trimmed.to_owned())
}

/// content 子树的三份文件（相对交付根）
pub const CONTENT_FILES: [&str; 3] = [
    "content/machine_catalog.json",
    "content/bundles.json",
    "content/assets_index.json",
];

/* ---------- 三份目录 JSON 的形状 ---------- */

/// 12.3：套餐。字段就是 `bundles.toml` 那五个，一个不多 ——
/// 消费端按 `assetRefs` join [`assets_index_json`] 的 id
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BundleEntry<'a> {
    id: &'a str,
    display: &'a str,
    machine_id: &'a str,
    asset_refs: &'a [String],
    updated_at: &'a Option<String>,
}

/// 12.4：资产索引的一条。**只有引用可达的资产进交付**（doc §7 原则 1：
/// 没被任何有效内容引用的文件不进交付 —— 正式的可达性分析在 Task 13，这里先按引用集收）
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AssetIndexEntry<'a> {
    id: &'a str,
    #[serde(rename = "type")]
    kind: AssetKind,
    machine_id: &'a Option<String>,
    name: &'a str,
    /// **交付根相对的落点**（`mkp/…`）—— 与 catalog 的 `CatalogFile.path` 同值。
    /// 曾经这里是"相对 `assets/` 的一段、与资产根同形"，对齐后统一成 catalog 那套
    /// （算出来的，所以是 owned）
    path: String,
    slicer: &'a Option<String>,
    profile: &'a Option<String>,
}

/// 12.2：版本条目。`mkpPresetAssetId` 是**连接键**（指向 manifest 的 assets[].id），
/// 产物本体不在 JSON 里、也不在这里重新算文件名 —— manifest 与 presets/mkp/ 已有
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VersionEntry<'a> {
    id: &'a str,
    name: &'a str,
    tag: &'a Option<String>,
    recommended_bundle: &'a Option<String>,
    mkp_preset_asset_id: Option<String>,
}

/// 12.2：机型条目。占位机型（A2L）也在：`hasDimensions: false` 就是它的标注 ——
/// 与上游契约同形（四处皆空的占位机型同样进上游清单），消费端按这个跳过
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MachineEntry<'a> {
    id: &'a str,
    display: &'a str,
    brand: &'a str,
    external_aliases: &'a [String],
    /// 引用资产 id（join assets_index），不是文件路径
    image: &'a Option<String>,
    icon: &'a Option<String>,
    default_bundle: &'a Option<String>,
    has_dimensions: bool,
    versions: Vec<VersionEntry<'a>>,
}

/* ---------- 引用可达集（12.4 的输入） ---------- */

/// **被有效内容引用的资产**：机型 `image` / `icon` + 套餐 `assetRefs`。
///
/// 这就是 doc §7「可达性」的引用面（Task 13.1 会把它正式化成独立分析，集合不变）。
/// 去重靠资产 id（大小写不敏感），顺序照 assets.toml 的登记顺序 —— 稳定的输出
/// 才有稳定的 diff。**不在集合里的不进交付**（夹具的 a1-extra-image、
/// 真数据的三份模型与四份 0.2mm BBS 都是刻意的反例）。
pub fn referenced_assets(book: &Book<'_>) -> Vec<Asset> {
    let mut used: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    // 引用面取**①层真源**（presets.catalog）：image/icon 的字段全在那里；
    // book.machines() 的 CatalogMachine 是它的投影（没有 image）
    for m in book.presets.catalog.machines() {
        // 占位机型不参与交付，它引用的图也不进（A2L 本来就没有图；
        // 将来给它登记了图，可达性也要等它先有 [dimensions]）
        if !m.has_dimensions {
            continue;
        }
        if let Some(id) = m.image.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            used.insert(id.to_lowercase());
        }
        if let Some(id) = m.icon.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            used.insert(id.to_lowercase());
        }
    }
    for b in book.presets.bundles.items() {
        for r in &b.asset_refs {
            used.insert(r.trim().to_lowercase());
        }
    }
    book.presets
        .assets
        .items()
        .iter()
        .filter(|a| used.contains(&a.id.to_lowercase()))
        .cloned()
        .collect()
}

/* ---------- JSON 组装（纯函数，写盘在 write_content） ---------- */

/// 12.2：机型目录。**机型与版本的清单、关系全部来自 presets 域**，
/// mkpPresetAssetId 是唯一一处从产物侧（manifest）带过来的连接键
pub fn machine_catalog_json(book: &Book<'_>) -> serde_json::Value {
    let machines: Vec<MachineEntry<'_>> = book
        .presets
        .catalog
        .machines()
        .iter()
        .map(|m| MachineEntry {
            id: &m.id,
            display: &m.display,
            brand: &m.brand,
            external_aliases: &m.external_aliases,
            image: &m.image,
            icon: &m.icon,
            default_bundle: &m.default_bundle,
            has_dimensions: m.has_dimensions,
            versions: m
                .versions
                .iter()
                .map(|v| {
                    let uid = format!("{}/{}", m.id, v.id);
                    VersionEntry {
                        id: &v.id,
                        name: &v.name,
                        tag: &v.tag,
                        recommended_bundle: &v.recommended_bundle,
                        // 连接键指向 manifest 里这一版的 mkp_preset 条目，
                        // 值就是产物名（命名规则算出）。占位版本（没有可产出的东西）
                        // 不进交付，连接键照实留空
                        mkp_preset_asset_id: book
                            .version(&uid)
                            .filter(|_| book.build_state(&uid) != BuildState::NoResources)
                            .map(|x| x.mkp_file.clone()),
                    }
                })
                .collect(),
        })
        .collect();
    let brands: Vec<serde_json::Value> = book
        .presets
        .catalog
        .brands()
        .iter()
        .map(|b| serde_json::json!({ "id": b.id, "name": b.name, "logo": b.logo }))
        .collect();
    serde_json::json!({ "brands": brands, "machines": machines })
}

/// 12.3：套餐目录。`bundles.toml` 的直出 —— 那边五个字段就是契约
pub fn bundles_json(book: &Book<'_>) -> serde_json::Value {
    let bundles: Vec<BundleEntry<'_>> = book
        .presets
        .bundles
        .items()
        .iter()
        .map(|b| BundleEntry {
            id: &b.id,
            display: &b.display,
            machine_id: &b.machine_id,
            asset_refs: &b.asset_refs,
            updated_at: &b.updated_at,
        })
        .collect();
    serde_json::json!({ "bundles": bundles })
}

/// 12.4：资产索引。**只编进交付的那部分**（[`referenced_assets`]），
/// 位置是**交付根相对的落点**（`mkp/…`，与 catalog 的 `CatalogFile.path` 同值 —— 同一处算法）
pub fn assets_index_json(assets: &[Asset]) -> serde_json::Value {
    let entries: Vec<AssetIndexEntry<'_>> = assets
        .iter()
        .filter_map(|a| {
            // 台账里不登记进交付的那一类（今天的 `image`）不进索引 ——
            // **索引与 catalog 的登记面必须一致**：索引里有、catalog 里没有，
            // 就成了"发布了却没人能下载到"的静默坑
            let path = crate::runtime::catalog::dest_of_asset(a)?;
            Some(AssetIndexEntry {
                id: &a.id,
                kind: a.kind,
                machine_id: &a.machine_id,
                name: &a.name,
                path,
                slicer: &a.slicer,
                profile: &a.profile,
            })
        })
        .collect();
    serde_json::json!({ "assets": entries })
}

/* ---------- 落盘 ---------- */

/// 一次交付的内容写入结果
#[derive(Debug)]
pub struct DistContent {
    /// 三份目录 JSON（machine_catalog / bundles / assets_index）
    pub content_files: usize,
    /// 复制进交付根 `mkp/…` 的资产文件数
    pub assets_copied: usize,
}

/// 写三份目录 JSON，并把引用可达的资产从资产根复制进交付根（`mkp/<kind 目录>/…`）。
///
/// **资产根是参数**（依赖注入）：生产上是 `public/assets/`，测试给临时目录 ——
/// 「登记了但文件不在」在这里是错误（加载期只查 id 认不认得出，
/// 文件在不在正是发布要守的最后一道），但缺了它的测试就造不出夹具。
///
/// 落点**由 [`crate::runtime::catalog::dest_of_asset`] 给出**，不在这里拼第二次：
/// 客户端就是按那个相对位置取文件的。复制走「读源 → 原子写目标」：
/// 交付目录是只读输出（doc §7 原则 3），原子写保证半份文件不会出现在那里。
pub fn write_content(
    dist_root: &Path,
    asset_root: &Path,
    book: &Book<'_>,
) -> Result<DistContent, AppError> {
    let catalog = machine_catalog_json(book);
    let bundles = bundles_json(book);
    let referenced = referenced_assets(book);
    let index = assets_index_json(&referenced);

    let content = dist_root.join(CONTENT_DIR);
    crate::fsx::atomic::atomic_write_json(&content.join("machine_catalog.json"), &catalog)?;
    crate::fsx::atomic::atomic_write_json(&content.join("bundles.json"), &bundles)?;
    crate::fsx::atomic::atomic_write_json(&content.join("assets_index.json"), &index)?;

    let mut copied = 0usize;
    for a in &referenced {
        // 不登记的那一类不进交付（与 catalog / 资产索引同一口径）
        let Some(rel) = crate::runtime::catalog::dest_of_asset(a) else {
            continue;
        };
        let src = asset_root.join(&a.path);
        let bytes = std::fs::read(&src).map_err(|e| {
            AppError::not_found(format!("资产 {} 的文件不在：{}", a.id, src.display()))
                .with_detail(e.to_string())
        })?;
        crate::fsx::atomic::atomic_write(&dist_root.join(&rel), &bytes)?;
        copied += 1;
    }

    Ok(DistContent {
        content_files: 3,
        assets_copied: copied,
    })
}

/* ---------- 发布（b05 Task 13） ---------- */

/// **交付集合**（13.1）：本次发布应当存在于交付目录的全部文件（**相对交付根的路径**）。
///
/// 它是从引用关系**算出来**的：`content/` 三份自产、`manifest.json` 自产、
/// `catalog.json` 自产、`source.json` 自产（第十七刀：客户端拿它解析远端在哪）、
/// 资产来自引用可达集（落点 = catalog 的 `files[].path`）、
/// `mkp/presets/<file_name>` 来自「有产物的版本」（产物名由命名函数算出）。
/// 残留在语义上就是「目录里有、这个集合里没有」。
///
/// **集合里每个路径都是客户端会按 URL 去取的那个相对位置** —— 交付根就是数据源地址
/// 指向的根，所以这份集合同时是"发布出去的东西"与"客户端能取到的东西"两份清单。
pub fn deliverable_set(book: &Book<'_>) -> BTreeSet<String> {
    let mut set: BTreeSet<String> = BTreeSet::new();
    set.extend(CONTENT_FILES.map(str::to_owned));
    set.insert(MANIFEST_FILE.to_owned());
    set.insert(NEW_CATALOG_FILE.to_owned());
    set.insert(SOURCE_FILE.to_owned());
    for a in referenced_assets(book) {
        // 落点与客户端同源（[`crate::runtime::catalog::dest_of_asset`]）；
        // 台账里不登记的那一类不进交付集合
        if let Some(rel) = crate::runtime::catalog::dest_of_asset(&a) {
            set.insert(rel);
        }
    }
    for v in book.versions() {
        // 占位版本（没有可产出的东西）不进交付；其余产物名由命名规则算出
        if book.build_state(&v.uid) == BuildState::NoResources {
            continue;
        }
        set.insert(format!("{MKP_PRESETS_DIR}/{}", v.mkp_file));
    }
    set
}

/// 递归收集 `dir` 下的全部文件，返回**以 `/` 分隔**的相对路径。
/// 目录不存在（还没发布过）= 空集合
fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_files(&p, out);
        } else if p.is_file() {
            out.push(p);
        }
    }
}

/// **残留扫描**（13.4）：交付目录里存在、但不在本次交付集合内的文件。
///
/// 按字典序返回，同样的残留每次都得到同样的清单 —— 列表顺序就是处理顺序。
pub fn scan_strays(dist_root: &Path, expected: &BTreeSet<String>) -> Vec<String> {
    let mut found: Vec<PathBuf> = Vec::new();
    collect_files(dist_root, &mut found);
    let mut strays: Vec<String> = found
        .iter()
        .filter_map(|p| p.strip_prefix(dist_root).ok())
        .map(|rel| rel.to_string_lossy().replace('\\', "/"))
        .filter(|rel| !expected.contains(rel))
        .collect();
    strays.sort();
    strays
}

/// **清理残留**（13.5）：把残留文件移进 `trash_root/dist/<stamp>/`（保留相对路径）。
///
/// 走回收而不是直接删 ——「删错了」在交付场景没有自动恢复，回收站有。
/// rename 在同一卷上是原子的；返回清理的文件数。
pub fn clean_strays(
    dist_root: &Path,
    strays: &[String],
    trash_root: &Path,
    stamp: &str,
) -> Result<usize, AppError> {
    let mut moved = 0usize;
    for rel in strays {
        let src = dist_root.join(rel);
        let dst = trash_root.join("dist").join(stamp).join(rel);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AppError::io(format!("建不出回收目录 {}", parent.display()))
                    .with_detail(e.to_string())
            })?;
        }
        std::fs::rename(&src, &dst).map_err(|e| {
            AppError::io(format!(
                "移不动残留文件：{} → {}",
                src.display(),
                dst.display()
            ))
            .with_detail(e.to_string())
        })?;
        moved += 1;
    }
    Ok(moved)
}

/// manifest 里一条资产。**两种命名空间共存**（见模块头「两类 id 的边界」）：
/// mkp_preset 条目的 id 来自上游，资产条目的 id 是资产域 Asset.id
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DistAsset {
    pub id: String,
    pub resource_type: String,
    pub machine_id: String,
    pub file_name: String,
    pub relative_path: String,
    pub sha256: String,
    pub size: u64,
}

/// 发布的元信息（从调用方带进来：clock 与渠道常量都不属于交付层）。
/// 最低客户端与版本原先是上游 manifest 的 `compat`，上游删掉后没有来源、照实留空
#[derive(Debug, Clone)]
pub struct PublishMeta {
    pub stamp: String,
    pub channel: String,
    pub minimum_client: String,
    pub version: String,
}

/// 一次发布的产出计数
#[derive(Debug)]
pub struct PublishOutcome {
    /// manifest.assets 条目数（mkp 产物 + 资产）
    pub files: usize,
    /// 其中资产条目数
    pub assets_copied: usize,
    /// 其中 mkp 产物条目数
    pub presets: usize,
}

/// **发布核心**（13.1–13.8）：闸门在命令层（[`super::build::wb_publish`] 先跑校验），
/// 这里从「算交付集合」到「manifest 落盘」一条路走完：
///
/// 1. **残留扫描**：不在交付集合内的文件一律列出，**有残留就中止** ——
///    一个字节都不写，manifest 更不会写（13.4）；
/// 2. 写三份目录 JSON + 复制引用集资产（Task 12 的 [`write_content`]）；
/// 3. 读 mkp 产物与已复制的资产字节，**按发布出去的那份算 sha256**（13.6）；
/// 4. 产出 `catalog.json`（两端共用契约），并**逐条核对它的 `path` 在交付根里真有
///    那份文件、字节也对得上** —— 发布布局 = 客户端落点，这条错了在这里就红；
/// 5. manifest 最后写（13.7，原子写）。
pub fn publish_into(
    dist_root: &Path,
    asset_root: &Path,
    book: &Book<'_>,
    meta: &PublishMeta,
) -> Result<PublishOutcome, AppError> {
    let expected = deliverable_set(book);

    // 13.4：**先扫残留，再动任何一个字节**。带着残留写新内容，
    // 等于默认「这批历史产物是好的」—— 而没人验证过它
    let strays = scan_strays(dist_root, &expected);
    if !strays.is_empty() {
        return Err(AppError::invalid_argument(format!(
            "交付目录里有 {} 个不在本次交付集合内的残留文件，先清理再发布",
            strays.len()
        ))
        .with_detail(strays.join("、")));
    }

    let content = write_content(dist_root, asset_root, book)?;
    let referenced = referenced_assets(book);

    let mut assets: Vec<DistAsset> = Vec::new();
    let mut presets_count = 0usize;

    for v in book.versions() {
        if book.build_state(&v.uid) == BuildState::NoResources {
            continue; // 暂无资源：跳过，不报错
        }
        // 产物名由命名规则算出（机型 id + 版本 id），不再查上游 manifest
        let name = v.mkp_file.clone();
        let rel = format!("{MKP_PRESETS_DIR}/{name}");
        let bytes = std::fs::read(dist_root.join(&rel)).map_err(|e| {
            AppError::not_found(format!("{} 的产物还没生成", v.name)).with_detail(format!(
                "{} 不存在（{}）。先在生成视角里生成，再发布",
                dist_root.join(&rel).display(),
                e
            ))
        })?;
        assets.push(DistAsset {
            // 哈希**按发布出去的那份字节算**，不抄上游的 —— 抄了就等于声明
            // 一个我们没验证过的哈希
            sha256: sha256_of(&bytes),
            size: bytes.len() as u64,
            // mkp 条目的 id 用产物名（唯一一处连接键，machine_catalog 的
            // mkpPresetAssetId 指向它）
            id: name.clone(),
            resource_type: "mkp_preset".to_owned(),
            machine_id: v.machine_id.clone(),
            file_name: name,
            relative_path: rel,
        });
        presets_count += 1;
    }

    for a in &referenced {
        // 落点与客户端同源；不登记的类跳过（不进交付集合，也不该进 manifest）
        let Some(rel) = crate::runtime::catalog::dest_of_asset(a) else {
            continue;
        };
        // 刚在上面 write_content 复制过，读不到属于内部错误 —— 但还是带上 id 报
        let bytes = std::fs::read(dist_root.join(&rel)).map_err(|e| {
            AppError::not_found(format!("资产 {} 的文件不在交付目录", a.id))
                .with_detail(e.to_string())
        })?;
        let file_name = a.path.rsplit('/').next().unwrap_or(&a.path).to_owned();
        assets.push(DistAsset {
            sha256: sha256_of(&bytes),
            size: bytes.len() as u64,
            id: a.id.clone(),
            resource_type: a.kind.key().to_owned(),
            machine_id: a.machine_id.clone().unwrap_or_default(),
            file_name,
            relative_path: rel,
        });
    }

    let manifest = serde_json::json!({
        "manifestVersion": 3,
        "channel": meta.channel,
        "updated": meta.stamp,
        // **上游未声明就照实留空**，不编一个版本号出来（doc §12）
        "minimumClient": meta.minimum_client,
        "version": meta.version,
        "assets": assets,
    });

    // **新世界的目录**（两端共用契约）：与客户端 `runtime::Catalog` 同一个类型、
    // 同一个 schema、同一套指纹。工作台发布的是"远端那份"，客户端检查/应用更新
    // 就是对两个 revision 做比较——不再有第二种清单格式。
    // 宽松构建：没有产物的版本合法（交付集合本来就不含它）。产物目录给的就是
    // 交付根里那一格（`mkp/presets/`）——**与客户端落点同形**，不再有一层自己的坐标系。
    let new_catalog = crate::runtime::catalog::Catalog::build_from_presets_lenient(
        book.presets,
        &dist_root.join(MKP_PRESETS_DIR),
    );

    // **收尾核对**（2026-10-02）：本次发出去的每一份文件，说明书里都登记了、字节也对得上。
    //
    // 这一条把"发布布局 = 客户端落点"从约定变成运行时判据。客户端取文件的地址是
    // `数据源地址 + catalog 的 path`，所以任何一处错位都是"上传成功、用户点了下载却 404"：
    // 复制到别的目录去了、少复制一份、复制错了。跑在这里，而不是等用户在界面里撞上。
    // 顺带把 catalog 的 SHA / 大小也验了：那是下载后校验的期望值，发布时先对一遍真字节。
    //
    // 核对的是**本次发出的集合**（[`deliverable_set`]），不是 catalog 的全部条目：
    // 说明书里登记 15 条资产，但只有**被引用可达的 8 条**进交付（doc §7 原则 1 的可达性
    // 收窄）—— 客户端也只下载它够得着的那 8 条，所以"登记得比发得多"是设计，不是漏洞。
    let registered: std::collections::HashMap<&str, &crate::runtime::catalog::CatalogFile> =
        new_catalog
            .files
            .iter()
            .map(|f| (f.path.as_str(), f))
            .collect();
    for rel in expected.iter() {
        if rel == MANIFEST_FILE
            || rel == NEW_CATALOG_FILE
            || rel == SOURCE_FILE
            || CONTENT_FILES.contains(&rel.as_str())
        {
            continue; // 自产的几类不在说明书里（说明书自己、Bootstrap、目录类都是）
        }
        let Some(f) = registered.get(rel.as_str()) else {
            return Err(AppError::internal(format!(
                "交付了 {rel}，但说明书里没有登记它"
            ))
            .with_detail(
                "客户端只认 catalog 的 `files[].path`：登记漏了就等于这份文件发出去也没人能取到"
                    .to_owned(),
            ));
        };
        let bytes = std::fs::read(dist_root.join(rel)).map_err(|e| {
            AppError::internal(format!("目录登记了 {}，交付根里却没有这份文件", f.path))
                .with_detail(format!(
                    "客户端会按「数据源地址 + {}」去取 —— 发布布局必须与 catalog 的 path 对齐：{e}",
                    f.path
                ))
        })?;
        if bytes.len() as u64 != f.size || sha256_of(&bytes) != f.sha256 {
            return Err(AppError::internal(format!(
                "{} 的字节与目录登记的不一致（大小或 SHA）",
                f.path
            ))
            .with_detail(
                "目录里的 SHA / 大小是下载后校验的期望值：对不上就是发布出去的字节与说明书说的不是同一份"
                    .to_owned(),
            ));
        }
    }

    // 清单、目录与 Bootstrap 收尾写（都过了核对才落）；`fsx::atomic` 是仓库唯一的写盘出口
    let new_catalog_text = new_catalog.to_pretty_json()?;
    crate::fsx::atomic::atomic_write_json(&dist_root.join(MANIFEST_FILE), &manifest)?;
    crate::fsx::atomic::atomic_write(
        &dist_root.join(NEW_CATALOG_FILE),
        new_catalog_text.as_bytes(),
    )?;
    /* Bootstrap：客户端"官方内置地址"指向的就是它（`resolve_source` 解析它拿两个地址） */
    crate::fsx::atomic::atomic_write_json(&dist_root.join(SOURCE_FILE), &bootstrap_json())?;

    Ok(PublishOutcome {
        files: assets.len(),
        assets_copied: content.assets_copied,
        presets: presets_count,
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
    use crate::workbench::app::build::preset_file_name;
    use crate::workbench::domain::patch::{Committed, CommittedVersion};
    use crate::workbench::domain::testkit::{fixture_catalog, Fixture};
    use std::collections::BTreeMap;

    /// 夹具 Committed（与 issues.rs tests 的 committed() 同一份清单）
    fn committed() -> Committed {
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
        Committed {
            versions,
            catalog: fixture_catalog(),
            ..Default::default()
        }
    }

    /// **引用可达集**：夹具 6 条资产里只有 4 条进交付 ——
    /// a1-extra-image（没人引用）与 p1s-bbs-02-010（没进任何套餐）是刻意的反例。
    /// 少了这条，referenced_assets 实现「永远全量」也能蒙混过去
    #[test]
    fn referenced_assets_is_the_reachable_subset() {
        let f = Fixture::load();
        let c = committed();
        let d = crate::workbench::domain::patch::Draft::default();
        let book = Book::new(&f.presets, &c, &d);
        let got = referenced_assets(&book);
        let ids: Vec<&str> = got.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["a1-image", "a1-icon", "p1s-icon", "a1-bbs-04-020"],
            "可达集 = 机型 image/icon + 套餐 assetRefs；登记顺序输出"
        );
    }

    /// 三份 JSON 的形状：字段、关系、连接键（夹具级）
    #[test]
    fn the_three_catalog_jsons_carry_the_relations() {
        let f = Fixture::load();
        let c = committed();
        let d = crate::workbench::domain::patch::Draft::default();
        let book = Book::new(&f.presets, &c, &d);

        let cat = machine_catalog_json(&book);
        let machines = cat["machines"].as_array().expect("machines 数组");
        assert_eq!(machines.len(), 3, "夹具 3 台机型（含占位 A2L）");
        let a1 = machines
            .iter()
            .find(|m| m["id"] == "A1")
            .expect("A1 必须在");
        assert_eq!(a1["image"], "a1-image", "机型→资产是 id 引用");
        assert_eq!(a1["defaultBundle"], "A1_default");
        assert_eq!(a1["hasDimensions"], true);
        let versions = a1["versions"].as_array().unwrap();
        assert_eq!(versions.len(), 2);
        assert_eq!(
            versions[0]["mkpPresetAssetId"], "A1-standard.toml",
            "版本→manifest 资产的连接键 = 产物名（命名规则算出）"
        );
        assert!(
            versions[0].get("presetFile").is_none(),
            "G-2 要删的字段不许写进交付契约"
        );
        let a2l = machines.iter().find(|m| m["id"] == "A2L").unwrap();
        assert_eq!(a2l["hasDimensions"], false, "占位机型有标注");

        let bundles = bundles_json(&book)["bundles"].as_array().unwrap().len();
        assert_eq!(bundles, 1, "夹具一条套餐");

        let idx = assets_index_json(&referenced_assets(&book));
        let assets = idx["assets"].as_array().unwrap();
        assert!(assets.iter().any(|a| a["id"] == "a1-bbs-04-020"));
        assert!(
            assets.iter().all(|a| a.get("sha256").is_none()),
            "sha256/size 是 Task 13.6 的事，这里不给"
        );
    }

    /// **12.6 判据（夹具级）**：write_content 落盘后，引用的每个文件
    /// 都在交付目录里真实存在；缺文件的资产要在发布侧报错而不是静默跳过
    #[test]
    fn write_content_copies_every_referenced_file_and_the_index_tells_the_truth() {
        let f = Fixture::load();
        let c = committed();
        let d = crate::workbench::domain::patch::Draft::default();
        let book = Book::new(&f.presets, &c, &d);

        // 夹具资产根：给可达集里的每条资产造一个假文件（登记了但文件不在
        // 是发布侧要拦的形状，所以可达集之外的 a1-extra-image 故意不造）
        let asset_root = tempfile::tempdir().unwrap();
        for rel in [
            "printers/a1.webp",
            "icons/a1.svg",
            "bbs/A1/process.json",
            "icons/p1s.svg",
        ] {
            let p = asset_root.path().join(rel);
            crate::fsx::atomic::atomic_write(&p, b"payload").unwrap();
        }

        let dist = tempfile::tempdir().unwrap();
        let out = write_content(dist.path(), asset_root.path(), &book).expect("落盘");
        assert_eq!(out.content_files, 3);
        // 可达集 4 条（a1-image + a1-icon + p1s-icon + a1-bbs-04-020），但 **image 类
        // 不登记进交付**（就是整机图那一档）—— 所以真正落盘的是 3 条：
        // 与真台账「整机图剥离」同一条规则，夹具这一条是它的反空转锚点
        assert_eq!(out.assets_copied, 3);

        // 三份 JSON 真的在盘上
        for name in ["machine_catalog.json", "bundles.json", "assets_index.json"] {
            assert!(dist.path().join("content").join(name).is_file(), "{name}");
        }
        // 12.6：资产索引引用的每一条都在交付目录里真实存在 ——
        // 索引里的 `path` 就是**交付根相对的落点**（`mkp/…`），直接拿它查盘
        let idx = assets_index_json(&referenced_assets(&book));
        for a in idx["assets"].as_array().unwrap() {
            let rel = a["path"].as_str().unwrap();
            assert!(
                rel.starts_with(&format!("{MKP_DIR}/")),
                "索引里的落点要落进 mkp/：{rel}"
            );
            let p = dist.path().join(rel);
            assert!(p.is_file(), "索引里的 {} 没有落在交付目录", a["id"]);
        }

        // 缺文件的资产：发布必须拦下，且要说清是哪一条
        let asset_root2 = tempfile::tempdir().unwrap();
        crate::fsx::atomic::atomic_write(&asset_root2.path().join("printers/a1.webp"), b"payload")
            .unwrap();
        let err =
            write_content(dist.path(), asset_root2.path(), &book).expect_err("文件不在必须报错");
        assert!(err.message.contains("文件不在"), "实测：{}", err.message);
    }

    /// **真数据上的交付集**（12.6 的真数据版 + 防空转锚点）。
    ///
    /// 锚点来自真数据的两个数：图标 3（P2S/X1C 借 p1s-icon）、
    /// BBS 5（五条套餐各一条 0.4mm）→ 可达集 **8**；三份模型与
    /// 四份 0.2mm BBS 没被引用，**刻意不进交付**（Task 13 的可达性分析收窄它们，
    /// 集合本身不变）。12.6 逐条：索引引用的每个文件都在交付目录真实存在。
    ///
    /// 机型图那 3 条 2026-10-01 也随之离场 —— 整机图从资产台账剥离（它是界面素材，
    /// 不进 Catalog / Delivery），机型的 `image` 字段照实为空，可达集里自然没有它。
    #[test]
    fn the_real_delivery_set_matches_the_real_references() {
        let Some(root) = crate::workbench::paths::presets_root() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let Ok(asset_root) = crate::workbench::paths::assets_root() else {
            eprintln!("没定位到资产根，这条检查未执行（不是通过）");
            return;
        };
        let real = crate::workbench::presets::Presets::load_from(&root).expect("真 presets");
        let c = Committed::default();
        let d = crate::workbench::domain::patch::Draft::default();
        let book = Book::new(&real, &c, &d);

        // JSON 条数锚点：5 台机型、9 个版本、5 条套餐
        let cat = machine_catalog_json(&book);
        let machines = cat["machines"].as_array().unwrap();
        assert_eq!(machines.len(), 5, "真数据 5 台机型");
        let versions: usize = machines
            .iter()
            .map(|m| m["versions"].as_array().unwrap().len())
            .sum();
        assert_eq!(versions, 9, "版本总数变了 —— 说清为什么再改判据");
        assert_eq!(bundles_json(&book)["bundles"].as_array().unwrap().len(), 5);

        // 可达集 8 = 图标 3 + BBS 5；模型与 0.2mm BBS 刻意不进
        // （机型图那 3 条随"整机图剥离台账"离场：机型的 image 字段照实为空）
        let referenced = referenced_assets(&book);
        assert_eq!(
            referenced.len(),
            8,
            "可达集条数变了 —— 机型引用或套餐 assetRefs 动了，说清为什么"
        );
        assert!(
            referenced.iter().all(|a| a.kind != AssetKind::Model),
            "模型没被任何内容引用，不进交付（doc §7 原则 1）"
        );
        let bbs = referenced
            .iter()
            .filter(|a| a.kind == AssetKind::SlicerProfile)
            .count();
        assert_eq!(bbs, 5, "BBS 引用 = 套餐 assetRefs 合计，5 条套餐各 1 条");

        // 落盘（真资产根 → 临时交付根），12.6 逐条核对 + assetRefs join 闭合
        let dist = tempfile::tempdir().unwrap();
        let out = write_content(dist.path(), &asset_root, &book).expect("真数据落盘");
        assert_eq!(out.assets_copied, 8, "可达集 8 条，一条不少一条不多");
        let idx = assets_index_json(&referenced);
        for a in idx["assets"].as_array().unwrap() {
            // 落点是 `mkp/<kind 目录>/…`（交付根相对），直接查盘
            let p = dist.path().join(a["path"].as_str().unwrap());
            assert!(p.is_file(), "索引里的 {} 没有落在交付目录", a["id"]);
        }
        let index_ids: Vec<&str> = idx["assets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["id"].as_str().unwrap())
            .collect();
        for b in book.presets.bundles.items() {
            for r in &b.asset_refs {
                assert!(
                    index_ids.iter().any(|i| i.eq_ignore_ascii_case(r)),
                    "套餐 {} 引用的 {} 不在资产索引里 —— 引用集漏了它",
                    b.id,
                    r
                );
            }
        }
    }

    /* ---------- 发布（b05 Task 13） ---------- */

    /// **发布全链**（13.4 → 13.8）：残留拦下 → 清理 → 重发 → manifest v3 条目齐、
    /// 哈希与真实文件一致、交付目录里没有清单之外的文件。
    ///
    /// 这是 13.8 的主判据：**manifest 条目数等于交付目录实际文件数**（口径见下），
    /// 每条 sha256 与文件真实哈希一致。少一条都不行 —— manifest 少描述一个文件，
    /// 消费端就少下载一个；多描述一个，客户端就 404 一个
    #[test]
    fn publish_blocks_on_strays_then_manifests_every_delivered_file() {
        let mut f = Fixture::load();
        let c = committed();
        let d = crate::workbench::domain::patch::Draft::default();
        // 载荷根：夹具默认不带（`Fixture::load` 只写 TOML），这里补上 ——
        // 生产上 `load_presets` 会挂真实载荷根，挂上之后说明书里才会有资产条目
        // （没挂 = 资产全部算"载荷缺失"，目录里只剩预设，发布侧的收尾核对会当场发现）
        let asset_root = tempfile::tempdir().unwrap();
        for rel in [
            "printers/a1.webp",
            "icons/a1.svg",
            "bbs/A1/process.json",
            "icons/p1s.svg",
        ] {
            crate::fsx::atomic::atomic_write(&asset_root.path().join(rel), b"payload").unwrap();
        }
        f.presets.set_asset_root(asset_root.path());
        let book = Book::new(&f.presets, &c, &d);

        // 交付集合（13.1）：content 3 + manifest + catalog.json + source.json +
        // 夹具资产 3 + mkp 产物 3。夹具可达集有 4 条，`a1-image` 是 image 类 ——
        // **不登记进交付**（整机图那条规则）
        let expected = deliverable_set(&book);
        assert_eq!(
            expected.len(),
            12,
            "content 3 + manifest 1 + catalog.json 1 + source.json 1 + assets 3（image 类不进）+ mkp 3"
        );
        assert!(expected.contains("mkp/presets/A1-standard.toml"));
        assert!(expected.contains("mkp/bbs/A1/process.json"));
        assert!(
            !expected.iter().any(|p| p.contains("printers/")),
            "整机图（image 类）不进交付集合：{expected:?}"
        );
        assert!(
            expected.contains(NEW_CATALOG_FILE),
            "新世界目录在交付集合里"
        );
        assert!(
            expected.contains(SOURCE_FILE),
            "Bootstrap（source.json）在交付集合里"
        );

        let dist = tempfile::tempdir().unwrap();
        // 夹具三版的 mkp 产物（wb_generate 的等价物：文件在，内容任意）；
        // 落点就是 `wb_generate` 落的那一格
        for name in ["A1-standard.toml", "A1-fast.toml", "P1S-lite.toml"] {
            crate::fsx::atomic::atomic_write(
                &dist.path().join(MKP_PRESETS_DIR).join(name),
                format!("# preset {name}").as_bytes(),
            )
            .unwrap();
        }

        // **残留拦截**（13.4）：放一个不在集合内的文件 → 中止，且不写任何东西
        crate::fsx::atomic::atomic_write(&dist.path().join("stale.json"), b"old").unwrap();
        let meta = PublishMeta {
            stamp: "2026-09-24T00:00:00Z".to_owned(),
            channel: "stable".to_owned(),
            minimum_client: String::new(),
            version: String::new(),
        };
        let err = publish_into(dist.path(), asset_root.path(), &book, &meta)
            .expect_err("有残留必须中止发布");
        assert!(err.message.contains("残留"), "实测：{}", err.message);
        assert!(
            err.detail.unwrap_or_default().contains("stale.json"),
            "要列出残留是哪个文件"
        );
        assert!(!dist.path().join("manifest.json").exists(), "不写 manifest");
        assert_eq!(
            std::fs::read(dist.path().join("stale.json")).unwrap(),
            b"old",
            "中止发布时一个字节都不该动"
        );

        // **清理残留**（13.5）：进回收站（保留相对路径），不直接删
        let trash = tempfile::tempdir().unwrap();
        let strays = scan_strays(dist.path(), &expected);
        assert_eq!(strays, vec!["stale.json"]);
        let moved = clean_strays(dist.path(), &strays, trash.path(), "20260924").unwrap();
        assert_eq!(moved, 1);
        assert!(!dist.path().join("stale.json").exists());
        assert_eq!(
            std::fs::read(trash.path().join("dist/20260924/stale.json")).unwrap(),
            b"old",
            "回收站里要能找回原文件"
        );

        // **重发成功**：manifest v3、无 bundles 字段（第二份套餐列表删掉了）、
        // 条目 = mkp 3 + 资产 3（可达 4 条里 image 类不进）
        let out = publish_into(dist.path(), asset_root.path(), &book, &meta).expect("发布");
        assert_eq!(out.presets, 3);
        assert_eq!(out.assets_copied, 3);
        assert_eq!(out.files, 6);

        let manifest_text = std::fs::read_to_string(dist.path().join("manifest.json")).unwrap();
        let manifest: serde_json::Value = serde_json::from_str(&manifest_text).unwrap();
        assert_eq!(manifest["manifestVersion"], 3, "结构变了就升版本");
        assert!(
            manifest.get("bundles").is_none(),
            "套餐的唯一真相在 bundles.json"
        );

        // **新世界目录**：与客户端同 schema、指纹非空、条目 = 有产物的版本；
        // 内容字节的 SHA 与 dist 里真实字节一致（消费端将来拿它当校验期望值）
        let new_catalog = crate::runtime::Catalog::parse(
            &std::fs::read(dist.path().join(NEW_CATALOG_FILE)).expect("catalog.json 该被写出"),
        )
        .expect("发布产出的 catalog.json 必须是合法目录");
        assert_eq!(
            new_catalog.catalog_schema,
            crate::runtime::catalog::CATALOG_SCHEMA
        );
        assert_eq!(new_catalog.revision.len(), 16, "指纹 16 位");
        assert_eq!(
            new_catalog.files.len(),
            6,
            "夹具：3 个有产物的版本 + 3 条有载荷的资产 —— p1s-bbs-02-010 没被引用、\
             夹具载荷根里也没造它的文件，两条 image 类不登记"
        );
        for f in &new_catalog.files {
            // 按**目录登记的 path** 去盘上取（不是按 file_name 拼）——
            // 客户端就是这么取的：数据源地址 + path。拼错方向在这条上就红
            let bytes = std::fs::read(dist.path().join(&f.path))
                .unwrap_or_else(|e| panic!("目录登记了 {}，盘上没有：{e}", f.path));
            use sha2::{Digest, Sha256};
            assert_eq!(
                crate::runtime::catalog::hex(&Sha256::digest(&bytes)),
                f.sha256
            );
        }

        // **13.8**：manifest 每条 relativePath 的文件存在、sha256 与真实字节一致；
        // 且交付目录里除 manifest/content 外，没有清单之外的文件
        use sha2::{Digest, Sha256};
        let assets = manifest["assets"].as_array().unwrap();
        assert_eq!(assets.len(), 6);
        for a in assets {
            let rel = a["relativePath"].as_str().unwrap();
            // 发布布局的锚点：每一份交付文件都落在 `mkp/…` 下（客户端下载区的形状）
            assert!(
                rel.starts_with(&format!("{MKP_DIR}/")),
                "交付文件必须在 mkp/ 下：{rel}"
            );
            let p = dist.path().join(rel);
            let bytes = std::fs::read(&p).unwrap_or_else(|_| panic!("{} 不在交付目录", a["id"]));
            let h = format!("{:x}", Sha256::digest(&bytes));
            assert_eq!(a["sha256"], h, "{} 的哈希与真实字节不符", a["id"]);
            assert_eq!(a["size"], bytes.len() as u64);
        }
        // **发布面 ⊆ 登记面**（第 2 步的核心不变式）：发出去的每一份都能在
        // catalog 里按同一个相对路径找到 —— 找不到就是"发了也没人能取到"
        let catalog_paths: std::collections::BTreeSet<&str> =
            new_catalog.files.iter().map(|f| f.path.as_str()).collect();
        for a in assets {
            let rel = a["relativePath"].as_str().unwrap();
            assert!(
                catalog_paths.contains(rel),
                "交付了 {rel}，但说明书里没有登记它（客户端只认 catalog 的 path）"
            );
        }

        let mut actual: Vec<PathBuf> = Vec::new();
        collect_files(dist.path(), &mut actual);
        let mut actual_rel: Vec<String> = actual
            .iter()
            .map(|p| {
                p.strip_prefix(dist.path())
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .filter(|rel| {
                rel != MANIFEST_FILE
                    && rel != NEW_CATALOG_FILE
                    && rel != SOURCE_FILE
                    && !CONTENT_FILES.contains(&rel.as_str())
            })
            .collect();
        actual_rel.sort();
        let mut listed: Vec<String> = assets
            .iter()
            .map(|a| a["relativePath"].as_str().unwrap().to_owned())
            .collect();
        listed.sort();
        assert_eq!(
            actual_rel, listed,
            "交付目录里的文件与 manifest 条目必须一一对应"
        );
    }

    /// **真数据上的交付集合**（13.1 的真数据版 + 防空转）：
    /// mkp 产物名 9 个（命名函数逐版算出）、资产 8 条（图标 3 + BBS 5；
    /// 整机图 2026-10-01 剥离台账后不再进交付）—— 集合计数锚点变了就说明清单或套餐变了
    #[test]
    fn the_real_deliverable_set_has_the_expected_shape() {
        let Some(root) = crate::workbench::paths::presets_root() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let real = crate::workbench::presets::Presets::load_from(&root).expect("真 presets");
        let c = Committed {
            catalog: real
                .catalog
                .machines()
                .iter()
                .map(|m| crate::workbench::domain::patch::CatalogMachine {
                    id: m.id.clone(),
                    display: m.display.clone(),
                    icon: m.icon.clone(),
                    default_bundle: m.default_bundle.clone(),
                    has_dimensions: m.has_dimensions,
                    version_ids: m.versions.iter().map(|v| v.id.clone()).collect(),
                })
                .collect(),
            ..Default::default()
        };
        let d = crate::workbench::domain::patch::Draft::default();
        let book = Book::new(&real, &c, &d);

        let expected = deliverable_set(&book);
        // 反空转锚点：content 3 + manifest 1 + catalog.json 1 + source.json 1 + 资产 8
        // + mkp 9（五台机型全部有套餐）= 23
        assert_eq!(
            expected.len(),
            23,
            "交付集合条数变了 —— 说清为什么（整机图剥离台账后资产从 11 条降到 8 条；\
             第十七刀起多一份 Bootstrap source.json）"
        );
        // **9 份 MKP 产物名单独立锚定**：命名函数逐版算出（wb_generate 将写的名单），
        // 与交付集合必须一致 —— 这是发布集合在真数据下的目标形状
        let mkp_names: std::collections::BTreeSet<String> = real
            .catalog
            .machines()
            .iter()
            .filter(|m| m.has_dimensions)
            .flat_map(|m| {
                m.versions
                    .iter()
                    .map(move |v| preset_file_name(&m.id, &v.id))
            })
            .collect();
        assert_eq!(mkp_names.len(), 9, "五台交付机型的产物名单必须是 9 份");
        // 集合里的 mkp 条目来自**每一版有可产出内容的版本**（五台机型全部有套餐，
        // build_state != NoResources）：9 版都在，逐份锚定
        for name in ["A1-standard.toml", "A1-fast.toml", "P1S-lite.toml"] {
            assert!(
                expected.contains(&format!("{MKP_PRESETS_DIR}/{name}")),
                "版本 {name} 必须进交付集合"
            );
        }
        assert_eq!(
            expected
                .iter()
                .filter(|p| p.starts_with(&format!("{MKP_PRESETS_DIR}/")))
                .count(),
            9,
            "9 版都有套餐 → 9 份 mkp 产物，少一条说明集合在空转"
        );
        assert_eq!(
            expected
                .iter()
                .filter(|p| p.starts_with("mkp/bbs/"))
                .count(),
            5,
            "BBS 进交付的只有套餐引用的 5 条"
        );
        assert!(
            !expected.iter().any(|p| p.contains("models/")),
            "模型没被引用，不进交付（13.2）"
        );

        // 空目录零残留（还没发布过是正常状态，不是错误）
        let dist = tempfile::tempdir().unwrap();
        assert!(scan_strays(dist.path(), &expected).is_empty());
    }

    /// **发布布局 = 客户端落点**（第 2 步的核心判据，真数据版）。
    ///
    /// 这条是这一刀要防的那种错：发布侧曾经自己拼一套（`assets/<载荷 path>` +
    /// `presets/mkp/<名字>`），而客户端认的落点是 `mkp/<kind 目录>/…` ——
    /// **URL 拼得上、落点对不上**：上传成功、用户点了下载却 404，
    /// 而且两边各自"看着都对"。所以判据不做字符串锚点，直接对真字节：
    ///
    /// - 目录登记的每个相对位置，盘上真有那份文件，且大小 / SHA 一致；
    /// - 反过来，写盘面与登记面**逐一对应**（没有登记了没写的，也没有写了没登记的）。
    #[test]
    fn the_published_layout_lands_where_the_catalog_says() {
        let Some(root) = crate::workbench::paths::presets_root() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let asset_root = crate::workbench::paths::assets_root().expect("资产载荷根");
        let mut real = crate::workbench::presets::Presets::load_from(&root).expect("真 presets");
        // 两端同一份载荷源：客户端从入库产物的 catalog 里拿的是同一批字节
        real.set_asset_root(&asset_root);

        let c = Committed {
            catalog: real
                .catalog
                .machines()
                .iter()
                .map(|m| crate::workbench::domain::patch::CatalogMachine {
                    id: m.id.clone(),
                    display: m.display.clone(),
                    icon: m.icon.clone(),
                    default_bundle: m.default_bundle.clone(),
                    has_dimensions: m.has_dimensions,
                    version_ids: m.versions.iter().map(|v| v.id.clone()).collect(),
                })
                .collect(),
            ..Default::default()
        };
        let d = crate::workbench::domain::patch::Draft::default();
        let book = Book::new(&real, &c, &d);

        let dist = tempfile::tempdir().unwrap();
        write_content(dist.path(), &asset_root, &book).expect("资产落盘");

        // 与发布侧同一次构建（产物目录给的是交付根里那一格；这里没跑 wb_generate，
        // 宽松构建会跳过没有产物的版本 —— 资产的登记面不受影响）
        let catalog = crate::runtime::catalog::Catalog::build_from_presets_lenient(
            &real,
            &dist.path().join(MKP_PRESETS_DIR),
        );

        // 交付根里到底写了些什么（三份目录 JSON 不算交付文件）
        let mut on_disk: Vec<PathBuf> = Vec::new();
        collect_files(dist.path(), &mut on_disk);
        let mut written: Vec<String> = on_disk
            .iter()
            .map(|p| {
                p.strip_prefix(dist.path())
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .filter(|rel| !CONTENT_FILES.contains(&rel.as_str()))
            .collect();
        written.sort();

        // 反空转：真数据里进交付的是 8 条资产（图标 3 + BBS 5）—— 整机图不在其中
        assert_eq!(
            written.len(),
            8,
            "写盘面该是 8 条资产（图标 3 + BBS 5）：{written:?}"
        );
        assert!(
            written
                .iter()
                .all(|p| p.starts_with(&format!("{MKP_DIR}/"))),
            "交付文件都该落在 mkp/ 下：{written:?}"
        );

        // **本条判据的核心**：盘上每一份文件，说明书里都按同一个相对路径登记着，
        // 而且 SHA / 大小与真字节一致 —— 客户端就是按那个路径拼 URL 去取的
        let registered: std::collections::HashMap<&str, &crate::runtime::catalog::CatalogFile> =
            catalog.files.iter().map(|f| (f.path.as_str(), f)).collect();
        assert_eq!(
            catalog.files.len(),
            15,
            "反空转：说明书登记台账全部 15 条（9 BBS + 3 图标 + 3 模型），交付只发可达的 8 条"
        );
        for rel in &written {
            let f = registered
                .get(rel.as_str())
                .unwrap_or_else(|| panic!("交付了 {rel}，说明书里却没有登记"));
            let bytes = std::fs::read(dist.path().join(rel))
                .unwrap_or_else(|e| panic!("读不到 {rel}: {e}"));
            assert_eq!(bytes.len() as u64, f.size, "{rel} 的大小");
            assert_eq!(sha256_of(&bytes), f.sha256, "{rel} 的 SHA");
        }
    }

    /* ---------- 第十七刀：Bootstrap（规范化 / 生成 / 两端形状） ---------- */

    /// GitHub blob 页 → raw 直链（人会从浏览器地址栏复制的那一种）；raw / 自建源原样
    #[test]
    fn blob_urls_turn_into_raw_urls() {
        assert_eq!(
            normalize_bootstrap_url(
                "https://github.com/MuCoreBenC/MKPSupportEase/blob/main/release/presets/source.json"
            )
            .expect("blob 页该转成 raw"),
            "https://raw.githubusercontent.com/MuCoreBenC/MKPSupportEase/main/release/presets/source.json"
        );
        assert_eq!(
            normalize_bootstrap_url(
                "  https://raw.githubusercontent.com/o/r/main/x/source.json/  "
            )
            .expect("raw 原样（只收拾空白与尾斜杠）"),
            "https://raw.githubusercontent.com/o/r/main/x/source.json"
        );
        assert_eq!(
            normalize_bootstrap_url("http://127.0.0.1:8000/source.json").expect("自建源合法"),
            "http://127.0.0.1:8000/source.json"
        );
    }

    /// 目录页 / 仓库首页 / 非 http(s) —— 拒（"那不是文件"这件事要说得出）
    #[test]
    fn directory_pages_and_non_http_are_refused() {
        for bad in [
            "https://github.com/MuCoreBenC/MKPSupportEase",
            "https://github.com/MuCoreBenC/MKPSupportEase/tree/main/presets",
            "file:///Users/me/source.json",
            "/Users/me/source.json",
            "   ",
        ] {
            let e = normalize_bootstrap_url(bad).unwrap_err();
            assert_eq!(
                e.code,
                crate::error::ErrorCode::InvalidArgument,
                "拒绝：{bad}"
            );
        }
    }

    /// 生成的 Bootstrap 就两件事；`baseUrl` **不写**（缺省 = 同目录）；
    /// 而且**客户端解析器认得发布侧写的这一份**（两端同一形状，一边写一边读，钉住）
    #[test]
    fn bootstrap_json_is_minimal_and_readable_by_the_client_parser() {
        let v = bootstrap_json();
        assert_eq!(v["sourceSchema"], crate::runtime::source::BOOTSTRAP_SCHEMA);
        assert_eq!(v["catalog"], NEW_CATALOG_FILE);
        assert_eq!(v["catalog"], crate::runtime::source::CATALOG_FILE);
        assert!(
            v.get("baseUrl").is_none(),
            "缺省 = 与 source.json 同目录 —— 发布侧不写它"
        );

        let parsed = crate::runtime::source::parse_bootstrap(
            "https://host/x/source.json",
            v.to_string().as_bytes(),
        )
        .expect("客户端解析器认得发布侧写的这一份");
        assert_eq!(parsed.base_url, "https://host/x");
        assert_eq!(parsed.catalog_url, "https://host/x/catalog.json");
    }
}
