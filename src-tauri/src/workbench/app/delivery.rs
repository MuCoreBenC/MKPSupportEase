//! 交付层（b05 Task 12）：`presets/delivery/` 的**目录类 JSON** 与资产复制。
//!
//! # 目录结构定稿（12.1；**2026-10-02 与客户端落点对齐**）
//!
//! 交付根就是「客户端拿 URL 直取的那个根」（数据源地址指向它）。所以这里的相对路径
//! **不是发布侧的自由**，而是契约的一半 —— 客户端下载用的地址是
//! `数据源地址 + catalog 里登记的那个相对位置`：
//!
//! ```text
//! presets/delivery/                       ← 数据源地址指向的根
//! ├── catalog.json                    **两端共用契约**（与客户端 runtime::Catalog 同
//! │                                   schema；客户端按 join_url(base,"catalog.json") 取）
//! ├── manifest.json                   发布账：本次发了哪些文件、SHA、大小（最后写）
//! ├── content/                        目录类 JSON（那一面的清单，客户端暂不读）
//! │   ├── machine_catalog.json        机型 + 版本 + 关系（12.2）
//! │   ├── bundles.json                套餐 + 包含什么（12.3）
//! │   └── assets_index.json           资产清单 + 位置 + 归属（12.4）
//! └── mkp/presets/<preset_file_name>  **B 类渲染产物**（`wb_generate` 落的，9 份起）
//! ```
//!
//! ★ **2026-10-04「唯一路径语义」（裁决 C/甲）之后，`delivery/` 只装两类东西**：
//! 上面这些**自产元数据**与 **B 类渲染产物**（源里没有实体、必须落盘的那几份 TOML）。
//! **A 类资产（BBS / 模型 / 图标）不在这里** —— 它们原地住在 `presets/assets/…`，
//! `catalog.path` 直接指过去（`assets/…`，相对**发布根 `presets/`**）。
//! 所以发布根下的形状是：
//!
//! ```text
//! presets/                 ← 发布根（`catalog.path` 的基准）
//! ├── assets/…             ← A 类：唯一实体，不复制（不再有 delivery/mkp/{bbs,icons,models}）
//! └── delivery/
//!     ├── catalog.json / manifest.json / source.json / content/**
//!     └── mkp/presets/…    ← B 类：渲染产物（`catalog.path` = `delivery/mkp/presets/…`）
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
//! （G-3 的切片器开放维度），子层给「按预设类型」留位置。它同时也是客户端那一侧的
//! 落点形状（`catalog.path` 的 `delivery/` 之后那一段），改名就会两边不同形。
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
//! assets_index 只管「清单、位置、归属」三样。
//!
//! # 13.x 发布（Task 13）：可达集合、残留拦截、manifest v3
//!
//! - **交付集合**（[`deliverable_set`]）是从引用关系**算出来**的文件相对路径全集 ——
//!   manifest 不维护第二份资产列表，它是这个集合的哈希清单；
//! - **残留拦截**（13.4 / doc §9.1）：发布前扫描交付目录，不在集合内的文件一律列出，
//!   **有残留就中止发布、不写 manifest** —— 残留会被消费端真的下载到，而且它多半是
//!   构建器自己留下的历史产物；[`clean_strays`] 是显式的清理动作，走
//!   `workbench/.trash/delivery/<stamp>/` 回收（保留相对路径，可还原），不直接删；
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
/// 交付集合里每一个文件都落在它下面，相对路径 = catalog 的 `files[].path`。
/// 常量本体住在 [`crate::workbench::paths`] —— 读侧（生成状态兜底）也要用，
/// 这里 re-export 保持原引用不断。
pub use crate::workbench::paths::{MKP_DIR, MKP_PRESETS_DIR};
/// 目录类 JSON 的子目录
pub const CONTENT_DIR: &str = "content";
/// manifest 的固定文件名
pub const MANIFEST_FILE: &str = "manifest.json";
/// 新世界目录的固定文件名（两端共用契约：与客户端 `runtime::Catalog` 同 schema）
/// —— **与 [`crate::runtime::source::CATALOG_FILE`] 同一个值**（直接引用它：
/// 同一份名字写在两处，迟早有一处改了另一处没改）
pub const NEW_CATALOG_FILE: &str = crate::runtime::source::CATALOG_FILE;
/// 官方源入口文件（Source Manifest）的固定名。客户端拿它解析寻址规则
/// （见 `runtime::source` / `runtime::resolver`）——第十七刀起它是发布产物的一部分
pub const SOURCE_FILE: &str = "source.json";

/// **Source Manifest v2** 的正文：寻址规则声明（`runtime::resolver`）——
/// catalog / manifest / release / content / filesRoot，**全部相对引用**。
///
/// 相对引用不是风格偏好，是**双镜像共存的生死线**：同一份文件随同一笔提交推
/// GitHub 与 Gitee 两个远端，写死任何一个绝对地址等于把另一个镜像的用户指回去。
/// `filesRoot: ".."` 是**全局锚点规则**（"catalog.files 从交付目录的上一层算"），
/// 不是 Entry 路径的一部分；它只在这一处出现，进过评审（总纲铁律 ⑤⑥）。
pub fn bootstrap_json() -> serde_json::Value {
    serde_json::json!({
        "sourceSchema": crate::runtime::resolver::MANIFEST_SCHEMA,
        "catalog": NEW_CATALOG_FILE,
        "manifest": MANIFEST_FILE,
        "release": crate::runtime::source::RELEASE_FILE,
        "content": format!("{CONTENT_DIR}/"),
        "filesRoot": "..",
    })
}

/// 官方源（Bootstrap）默认分支与**默认交付路径**：仓库地址补成 `main/<这里>`
/// （`source.json` 在交付根里，见模块头）。
///
/// 这是**产品契约的一半**："我有一个仓库" → 系统自己去 `main` 的交付目录找 Bootstrap。
/// 用户不必知道 `raw.githubusercontent.com` / `blob` / `presets/delivery` / `source.json`
/// 中的任何一个 —— 那正是 Bootstrap 作为内部机制的意义。
const DEFAULT_REF: &str = "main";
/// 交付根相对仓库根的路径（`presets/delivery`），与 `paths::delivery_root()` 同一处布局
const DELIVERY_REL_PATH: &str = "presets/delivery";

/// 官方源（Bootstrap）地址的**规范化** —— 工作台输入侧的唯一一处。
///
/// 产品契约（作者 2026-10-02 定死）——**用户只表达"我有一个仓库"，其余是系统的事**：
///
/// | 输入 | 结果 |
/// | --- | --- |
/// | GitHub 仓库地址 `https://github.com/<o>/<r>` | ✅ 补成 `main/presets/delivery/source.json` 的 raw |
/// | GitHub `.git` 克隆地址 `https://github.com/<o>/<r>.git` | ✅ 同上（`.git` 只是写法，去掉即可） |
/// | GitHub blob 页 `…/blob/<ref>/<path>` | ✅ 按人指的那份转 raw（尊重他显式的选择） |
/// | 已是 raw / 别的 http(s)（自建源） | ✅ 原样（只收拾空白与尾斜杠） |
/// | GitHub `tree/…` 目录页 | ❌ 拒（无法表达"要哪个发布入口"） |
/// | 空 / 非 http(s) | ❌ 拒 |
///
/// **Gitee 是同一座桥**（2026-10-05 双官方源）：仓库地址 / `.git` / blob 页的契约与
/// GitHub 完全对称，只有 raw 的落点不同 —— Gitee 的 raw 与网页**同域**
/// （`gitee.com/<o>/<r>/raw/<ref>/<path>`），所以比 GitHub 多认一种"已经是 raw"的形状
/// （GitHub 的 raw 住在别的域名上，天然落进"自建源原样"那一行）。
///
/// **`.git` 不是产品语义**：它只是克隆地址的一种写法，规范化时去掉。
pub fn normalize_bootstrap_url(raw: &str) -> Result<String, AppError> {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(AppError::invalid_argument("Bootstrap 地址是空的"));
    }
    if let Some(rest) = trimmed
        .strip_prefix("https://github.com/")
        .or_else(|| trimmed.strip_prefix("http://github.com/"))
    {
        let parts: Vec<&str> = rest.split('/').filter(|s| !s.is_empty()).collect();
        if parts.len() < 2 {
            return Err(AppError::invalid_argument(
                "这不是一个 GitHub 仓库地址 —— 要 <owner>/<repo> 两段",
            )
            .with_detail(format!("收到：{trimmed}")));
        }
        let owner = parts[0];
        /* 仓库名可能带 `.git` 后缀（克隆地址的写法）—— 去掉，它不是产品语义 */
        let repo = parts[1].strip_suffix(".git").unwrap_or(parts[1]);
        if repo.is_empty() {
            return Err(
                AppError::invalid_argument("这不是一个 GitHub 仓库地址 —— 仓库名是空的")
                    .with_detail(format!("收到：{trimmed}")),
            );
        }

        /* ① 仓库地址（两段，且第二段就是那个仓库）：补默认 ref + 默认交付路径 */
        if parts.len() == 2 {
            return Ok(format!(
                "https://raw.githubusercontent.com/{owner}/{repo}/{DEFAULT_REF}/{DELIVERY_REL_PATH}/{SOURCE_FILE}"
            ));
        }

        /* ② blob 页：人显式指了哪一份（含 ref 与路径），按他指的转 —— 尊重显式选择 */
        if parts[2] == "blob" && parts.len() >= 5 {
            return Ok(format!(
                "https://raw.githubusercontent.com/{owner}/{repo}/{}/{}",
                parts[3],
                parts[4..].join("/")
            ));
        }

        /* ③ 其余（`tree/…` 目录页、仓库下的别的路径）→ 拒：说不清"要哪个发布入口" */
        return Err(AppError::invalid_argument(
            "这是 GitHub 的目录页，不是一个文件 —— 请填**仓库地址**（我们会自动定位发布入口），或指向 source.json 的 blob 链接",
        )
        .with_detail(format!("收到：{trimmed}")));
    }
    if let Some(rest) = trimmed
        .strip_prefix("https://gitee.com/")
        .or_else(|| trimmed.strip_prefix("http://gitee.com/"))
    {
        let parts: Vec<&str> = rest.split('/').filter(|s| !s.is_empty()).collect();
        if parts.len() < 2 {
            return Err(AppError::invalid_argument(
                "这不是一个 Gitee 仓库地址 —— 要 <owner>/<repo> 两段",
            )
            .with_detail(format!("收到：{trimmed}")));
        }
        let owner = parts[0];
        /* 仓库名可能带 `.git` 后缀（克隆地址的写法）—— 去掉，它不是产品语义 */
        let repo = parts[1].strip_suffix(".git").unwrap_or(parts[1]);
        if repo.is_empty() {
            return Err(
                AppError::invalid_argument("这不是一个 Gitee 仓库地址 —— 仓库名是空的")
                    .with_detail(format!("收到：{trimmed}")),
            );
        }

        /* ① 仓库地址（两段，且第二段就是那个仓库）：补默认 ref + 默认交付路径 */
        if parts.len() == 2 {
            return Ok(format!(
                "https://gitee.com/{owner}/{repo}/raw/{DEFAULT_REF}/{DELIVERY_REL_PATH}/{SOURCE_FILE}"
            ));
        }

        /* ② 已是 raw 直链：原样（Gitee 的 raw 与网页同域，得在分支里认出来，
        不能落进下面的"自建源"—— 但结果一致，就是原样吐回去） */
        if parts[2] == "raw" && parts.len() >= 5 {
            return Ok(trimmed.to_owned());
        }

        /* ③ blob 页：人显式指了哪一份（含 ref 与路径），按他指的转 raw —— 与 GitHub 对称 */
        if parts[2] == "blob" && parts.len() >= 5 {
            return Ok(format!(
                "https://gitee.com/{owner}/{repo}/raw/{}/{}",
                parts[3],
                parts[4..].join("/")
            ));
        }

        /* ④ 其余（`tree/…` 目录页、仓库下的别的路径）→ 拒：说不清"要哪个发布入口" */
        return Err(AppError::invalid_argument(
            "这是 Gitee 的目录页，不是一个文件 —— 请填**仓库地址**（我们会自动定位发布入口），或指向 source.json 的 raw 直链",
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

/// 12.3：套餐。字段就是 `bundles.toml` 那五个 ——
/// 消费端按 `assetRefs` join [`assets_index_json`] 的 id（MKP 预设也在 `assetRefs` 里：
/// 它 2026-10-03 进了资产库，`type = 'mkPreset'`）
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BundleEntry<'a> {
    id: &'a str,
    display: &'a str,
    machine_id: &'a str,
    asset_refs: &'a [String],
    updated_at: &'a Option<String>,
}

/// 12.4：资产索引的一条。**只有引用可达的资产进交付**（可达性收窄：
/// 没被任何有效内容引用的文件不进交付 —— 规则正文见 `PUBLISH-ARCHITECTURE.md` §4.5）
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
/// 这就是「可达性」的引用面（语义正文见 `PUBLISH-ARCHITECTURE.md` §4.5）。
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

/// 12.3：套餐目录。`bundles.toml` 的直出 —— 那边五个字段 + `presets` 就是契约
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
    /// 体检通过的 A 类资产数（**不复制**，只确认载荷根里真有那份文件；2026-10-04 起）
    pub assets_checked: usize,
}

/// 写三份目录 JSON，并把引用可达的资产从资产根复制进交付根（`mkp/<kind 目录>/…`）。
///
/// **资产根是参数**（依赖注入）：生产上是 `public/assets/`，测试给临时目录 ——
/// 「登记了但文件不在」在这里是错误（加载期只查 id 认不认得出，
/// 文件在不在正是发布要守的最后一道），但缺了它的测试就造不出夹具。
///
/// 落点**由 [`crate::runtime::catalog::dest_of_asset`] 给出**，不在这里拼第二次：
/// 客户端就是按那个相对位置取文件的。复制走「读源 → 原子写目标」：
/// 交付目录是只读输出（生成器的唯一出口，总纲铁律 ②），原子写保证半份文件不会出现在那里。
pub fn write_content(
    delivery_root: &Path,
    asset_root: &Path,
    book: &Book<'_>,
) -> Result<DistContent, AppError> {
    let catalog = machine_catalog_json(book);
    let bundles = bundles_json(book);
    let referenced = referenced_assets(book);
    let index = assets_index_json(&referenced);

    let content = delivery_root.join(CONTENT_DIR);
    crate::fsx::atomic::atomic_write_json(&content.join("machine_catalog.json"), &catalog)?;
    crate::fsx::atomic::atomic_write_json(&content.join("bundles.json"), &bundles)?;
    crate::fsx::atomic::atomic_write_json(&content.join("assets_index.json"), &index)?;

    /*
     * ★ A 类资产**不复制**（2026-10-04 作者裁决 C/甲）。
     *
     * A 类（BBS / 模型 / 图标）在 `presets/assets/` 就是唯一实体，`catalog.path`
     * 直接指它（`assets/…`，相对发布根 `presets/`）—— 云端取的就是这一份，
     * 再复制一份进 `delivery/` 等于让同一份字节有第二个真相（那正是 404 与
     * "登记面 ≠ 实体面"的根）。
     *
     * 这里保留的是**体检**：确认每一条能交付的资产，它的文件真的在载荷根里。
     * 缺了当场报错（发布不该发一个取不到的 promise），但**不写任何东西到 delivery**。
     * 发布闸的「登记面 == 实体面」那一项（`docs/PUBLISH-ARCHITECTURE.md` §5.2 ⑦）
     * 检查的就是这件事。
     */
    let mut checked = 0usize;
    for a in &referenced {
        // 不登记的那一类不进交付（与 catalog / 资产索引同一口径）
        if crate::runtime::catalog::dest_of_asset(a).is_none() {
            continue;
        }
        let src = asset_root.join(&a.path);
        if !src.is_file() {
            return Err(AppError::not_found(format!(
                "资产 {} 的文件不在：{}",
                a.id,
                src.display()
            ))
            .with_detail(
                "A 类资产在发布根下原地交付（不复制进 delivery）—— 文件必须在载荷根里真实存在"
                    .to_owned(),
            ));
        }
        checked += 1;
    }

    Ok(DistContent {
        content_files: 3,
        assets_checked: checked,
    })
}

/* ---------- 发布（b05 Task 13） ---------- */

/// **`delivery/` 里该有什么**（13.1）：本次发布应当存在于交付目录的全部文件，
/// 相对 `delivery/` 的路径。
///
/// ★ 2026-10-04（裁决 C/甲）起，这里**只含 delivery 自己的东西**：
///
/// - `content/` 三份自产 · `manifest.json` · `catalog.json` · `source.json`；
/// - **B 类渲染产物** `mkp/presets/<file_name>`（源里没有实体，必须落 delivery）。
///
/// **A 类资产不在这个集合里** —— 它们原地住在 `presets/assets/`，不复制进 delivery。
/// 所以这份集合的用途是 `scan_strays`（扫 delivery 残留）：*残留 = delivery 里有、这里没有*。
///
/// 想查"客户端能不能取到某条 catalog 条目"用的是另一个函数
/// （[`every_registered_path_is_reachable`] 那一路），判据是 **发布根 + `catalog.path`**
/// 而不是 delivery —— 两者的面不一样，别混。
pub fn delivery_expected_set(book: &Book<'_>) -> BTreeSet<String> {
    let mut set: BTreeSet<String> = BTreeSet::new();
    set.extend(CONTENT_FILES.map(str::to_owned));
    set.insert(MANIFEST_FILE.to_owned());
    set.insert(NEW_CATALOG_FILE.to_owned());
    set.insert(SOURCE_FILE.to_owned());
    // ★ `release.json` 也住交付根（M6，2026-10-05 起）：它是**软件发布链**的信息源
    //   （客户端按 Source Manifest 的 `release` 声明取它），与预设产物同目录、不同链。
    //   不把它算进应有集合，发布闸的残留审计会永远拦着预设发布 —— 而且点「清理残留」
    //   会把客户端检查软件更新要用的那份扔进回收站（真机踩过，2026-10-06）。
    set.insert(crate::runtime::source::RELEASE_FILE.to_owned());
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
pub fn scan_strays(delivery_root: &Path, expected: &BTreeSet<String>) -> Vec<String> {
    let mut found: Vec<PathBuf> = Vec::new();
    collect_files(delivery_root, &mut found);
    let mut strays: Vec<String> = found
        .iter()
        .filter_map(|p| p.strip_prefix(delivery_root).ok())
        .map(|rel| rel.to_string_lossy().replace('\\', "/"))
        .filter(|rel| !expected.contains(rel))
        .collect();
    strays.sort();
    strays
}

/// **清理残留**（13.5）：把残留文件移进 `trash_root/delivery/<stamp>/`（保留相对路径）。
///
/// 走回收而不是直接删 ——「删错了」在交付场景没有自动恢复，回收站有。
/// rename 在同一卷上是原子的；返回清理的文件数。
pub fn clean_strays(
    delivery_root: &Path,
    strays: &[String],
    trash_root: &Path,
    stamp: &str,
) -> Result<usize, AppError> {
    let mut moved = 0usize;
    for rel in strays {
        let src = delivery_root.join(rel);
        let dst = trash_root.join("delivery").join(stamp).join(rel);
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
///
/// `version` 原先是上游 manifest 的包版本，上游删掉后没有来源、照实留空。
///
/// **最低客户端版本不在这一格里**（2026-10-04）：它只有一个来源 —— 结构规则表里对
/// 当前结构签名的那条登记，由 [`publish_into`] 自己查（见那里）。放两份必然漂，
/// 而"漂"的表现是发出去的目录说一套、manifest 说另一套。
#[derive(Debug, Clone)]
pub struct PublishMeta {
    pub stamp: String,
    pub channel: String,
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
    /// 这次发出去的目录里的**最低正式客户端版本**（结构规则表登记的）。
    /// `None` = 这一代还没登记 —— 发布闸会拦住这件事，所以正常路径上不会是 `None`
    pub minimum_client: Option<String>,
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
    delivery_root: &Path,
    asset_root: &Path,
    book: &Book<'_>,
    meta: &PublishMeta,
) -> Result<PublishOutcome, AppError> {
    let expected = delivery_expected_set(book);

    // 13.4：**先扫残留，再动任何一个字节**。带着残留写新内容，
    // 等于默认「这批历史产物是好的」—— 而没人验证过它
    let strays = scan_strays(delivery_root, &expected);
    if !strays.is_empty() {
        return Err(AppError::invalid_argument(format!(
            "交付目录里有 {} 个不在本次交付集合内的残留文件，先清理再发布",
            strays.len()
        ))
        .with_detail(strays.join("、")));
    }

    let content = write_content(delivery_root, asset_root, book)?;
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
        let bytes = std::fs::read(delivery_root.join(&rel)).map_err(|e| {
            AppError::not_found(format!("{} 的产物还没生成", v.name)).with_detail(format!(
                "{} 不存在（{}）。先在生成视角里生成，再发布",
                delivery_root.join(&rel).display(),
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
            // 发布根相对的完整路径（B 类在 `delivery/` 下）—— 与 `catalog.path` 同一基准
            relative_path: format!("delivery/{rel}"),
        });
        presets_count += 1;
    }

    for a in &referenced {
        // 落点与客户端同源；不登记的类跳过（不进交付集合，也不该进 manifest）
        let Some(rel) = crate::runtime::catalog::dest_of_asset(a) else {
            continue;
        };
        /*
         * ★ A 类资产**从载荷根读**（2026-10-04 裁决 C/甲）：它是原地交付的，
         * `delivery/` 里没有它的副本。这里算的 SHA / size 就是"客户端按
         * `baseUrl + assets/…` 取到的那份字节" —— 与 catalog 登记的期望值同源。
         */
        let bytes = std::fs::read(asset_root.join(&a.path)).map_err(|e| {
            AppError::not_found(format!("资产 {} 的文件不在载荷根", a.id))
                .with_detail(format!("{}：{e}", asset_root.join(&a.path).display()))
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

    // 发布根（`presets/`）—— `catalog.path` 的基准，也是结构规则表的家
    let publish_root = delivery_root
        .parent()
        .expect("delivery 必有父目录（发布根 presets/）");

    // **新世界的目录**（两端共用契约）：与客户端 `runtime::Catalog` 同一个类型、
    // 同一个 schema、同一套指纹。工作台发布的是"远端那份"，客户端检查/应用更新
    // 就是对两个 revision 做比较——不再有第二种清单格式。
    // 宽松构建：没有产物的版本合法（交付集合本来就不含它）。产物目录给的就是
    // 交付根里那一格（`mkp/presets/`）——**与客户端落点同形**，不再有一层自己的坐标系。
    let mut new_catalog = crate::runtime::catalog::Catalog::build_from_presets_lenient(
        book.presets,
        &delivery_root.join(MKP_PRESETS_DIR),
    );

    /*
     * **最低客户端版本只有一个来源**：结构规则表（`presets/structure-signatures.toml`）
     * 里对**当前结构签名**的那一条登记。
     *
     * 以前这一格抄的是上游 manifest 的 `compat`；上游删掉之后它就成了一直为空的死字段
     * （没人填、也没人读）。现在它 = `catalog.minClientVersion`，而发布闸 ⑫ 会拦住
     * "签名还没登记"的发布 —— 所以**发出去的这一格不会缺**，也不会是编出来的。
     *
     * 查不到就留空串（不猜）。真出得来这种情况，说明有人绕过了闸 —— 那正是要看见的。
     */
    let rules = crate::runtime::structure::RuleTable::load(publish_root)?;
    new_catalog.apply_min_client(&rules);
    let minimum_client = new_catalog.min_client_version.clone().unwrap_or_default();

    let manifest = serde_json::json!({
        "manifestVersion": 3,
        "channel": meta.channel,
        "updated": meta.stamp,
        "minimumClient": minimum_client,
        "version": meta.version,
        "assets": assets,
    });

    // **收尾核对**（2026-10-02）：本次发出去的每一份文件，说明书里都登记了、字节也对得上。
    //
    // 这一条把"发布布局 = 客户端落点"从约定变成运行时判据。客户端取文件的地址是
    // `数据源地址 + catalog 的 path`，所以任何一处错位都是"上传成功、用户点了下载却 404"：
    // 复制到别的目录去了、少复制一份、复制错了。跑在这里，而不是等用户在界面里撞上。
    // 顺带把 catalog 的 SHA / 大小也验了：那是下载后校验的期望值，发布时先对一遍真字节。
    //
    // 核对的是**本次发出的集合**（[`deliverable_set`]），不是 catalog 的全部条目：
    // 说明书里登记 15 条资产，但只有**被引用可达的 8 条**进交付（可达性收窄，
    // `PUBLISH-ARCHITECTURE.md` §4.5）—— 客户端也只下载它够得着的那 8 条，
    // 所以"登记得比发得多"是设计，不是漏洞。
    let registered: std::collections::HashMap<&str, &crate::runtime::catalog::CatalogFile> =
        new_catalog
            .files
            .iter()
            .map(|f| (f.path.as_str(), f))
            .collect();
    /*
     * ★ 收尾核对改成**按 `catalog.path` 在发布根下逐条核对**（2026-10-04 裁决 C/甲）。
     *
     * 以前是"拿 delivery 里的文件去说明书里找登记"，那预设了"发布物全在 delivery 里"。
     * 现在 A 类原地住在 `presets/assets/`，所以核对方向必须反过来：
     *
     *   说明书登记的每一条 → `发布根 + catalog.path` 处有没有那份文件、字节对不对
     *
     * 这正是发布闸第 ⑦ 项「登记面 == 实体面」（`PUBLISH-ARCHITECTURE.md` §5.2）——
     * 它在这里就已经跑了一遍，而且是**阻断式**的（对不上就不写 manifest）。
     *
     * 还有一个**反向**的核对（delivery 里有没有多发的）：由 `scan_strays` 用
     * [`delivery_expected_set`] 做，见本函数开头。
     */
    for f in new_catalog.files.iter() {
        if f.kind == crate::runtime::catalog::kind::PRESET {
            continue; // B 类下面单独核（它的 SHA 由生成侧写进目录，见 write_catalog_json）
        }
        let on_disk = publish_root.join(&f.path);
        let bytes = std::fs::read(&on_disk).map_err(|e| {
            AppError::internal(format!("目录登记了 {}，发布根里却没有这份文件", f.path))
                .with_detail(format!(
                    "客户端会按「数据源地址 + {}」去取 —— 它必须真实存在于发布根（{}）：{e}",
                    f.path,
                    on_disk.display()
                ))
        })?;
        /*
         * 发布侧**必须**有期望值：远端目录是下载校验的权威（随包 bootstrap 目录才不登记）。
         * `None` 出现在这里说明这份 catalog 不是发布侧构建的 —— 那是接线错误，当场报出来，
         * 不能默认通过（默认通过 = 发一份没人能校验的目录出去）。
         */
        let (Some(want_size), Some(want_sha)) = (f.expected_size(), f.expected_sha()) else {
            return Err(AppError::internal(format!(
                "{} 在发布目录里没有登记 SHA / 大小 —— 远端目录必须为下载校验背书",
                f.path
            ))
            .with_detail(
                "只有随包 bootstrap 目录才允许不登记期望值；发布产物必须逐份登记".to_owned(),
            ));
        };
        if bytes.len() as u64 != want_size || sha256_of(&bytes) != want_sha {
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

    /*
     * B 类（MKP 预设产物）单独核：它们落在 `delivery/mkp/presets/`，SHA 由生成侧写进目录。
     * 同样按 `catalog.path` 定位 —— 与 A 类同一条规则，只是根不同（发布根 vs delivery）。
     * 这一支跑完，`manifest` 才会写；任何一条取不到就是"发了一个取不到的 promise"。
     */
    for v in book.versions() {
        if book.build_state(&v.uid) == BuildState::NoResources {
            continue;
        }
        let rel = format!("{MKP_PRESETS_DIR}/{}", v.mkp_file);
        let catalog_path = format!("delivery/{rel}");
        let on_disk = delivery_root.join(&rel);
        let bytes = std::fs::read(&on_disk).map_err(|e| {
            AppError::internal(format!("产物 {} 不在交付根里", v.mkp_file)).with_detail(format!(
                "{} 不存在：{e}。先在生成视角里生成，再发布",
                on_disk.display()
            ))
        })?;
        let Some(f) = registered.get(catalog_path.as_str()) else {
            return Err(AppError::internal(format!(
                "产物 {rel} 发出来了，但说明书里没有登记 {}",
                catalog_path
            ))
            .with_detail(
                "客户端只认 catalog 的 `files[].path`：漏登记 = 发出去也没人取得到".to_owned(),
            ));
        };
        if let Some(want) = f.expected_sha() {
            if sha256_of(&bytes) != want {
                return Err(AppError::internal(format!(
                    "{} 的字节与目录登记的不一致（SHA）",
                    f.path
                )));
            }
        }
    }

    // 清单、目录与 Bootstrap 收尾写（都过了核对才落）；`fsx::atomic` 是仓库唯一的写盘出口
    crate::fsx::atomic::atomic_write_json(&delivery_root.join(MANIFEST_FILE), &manifest)?;
    /*
     * 发布时刻在这里盖：与 manifest 的 `updated` 是**同一个戳**（`meta.stamp`）——
     * 一次发布事件只有一个时间，目录与清单不许各说一个。客户端把它显示在
     * 云端表的「时间」列（`catalog.publishedAt` → 这次发布的时刻）。
     */
    write_catalog_json(delivery_root, book, &meta.stamp)?;
    /* Bootstrap：客户端"官方内置地址"指向的就是它（`resolve_source` 解析它拿两个地址） */
    crate::fsx::atomic::atomic_write_json(&delivery_root.join(SOURCE_FILE), &bootstrap_json())?;

    Ok(PublishOutcome {
        files: assets.len(),
        assets_copied: content.assets_checked,
        presets: presets_count,
        minimum_client: new_catalog.min_client_version.clone(),
    })
}

/// **只重算 catalog.json**（不动 manifest / source）—— [`publish_into`] 的目录那一半。
///
/// 为什么生成也要它（作者 2026-10-03）：`wb_generate` 直接把新产物写进交付根，
/// 清单要是不跟上，delivery 就处于「文件是新的、目录记的还是旧的」—— 客户端按目录
/// 登记的字节做下载校验，必挂（真机踩了两回：「下载失败：响应比目录登记的大」）。
/// 生成收尾把目录重算一遍，**记录永远与文件同一代**。manifest（版本 / 时间戳 /
/// 渠道，发布台账）仍归发布写 —— 生成不替发布定稿。
///
/// `published_at` 由**调用方给**（clock 不属于交付层，与 `PublishMeta` 同一条规矩）：
/// 发布传 `meta.stamp`（与 manifest.updated 同一个戳）、生成传当下。写进
/// `catalog.publishedAt`，客户端云端表的「时间」列显示的就是它。
///
/// 目录里的资产条目**按源字节算 SHA**（[`Catalog::build_from_presets_lenient`] 的
/// 口径）—— 所以调用方要先把引用资产补进交付根（`write_content`），否则就是
/// 「目录登记了，文件不在」。
pub fn write_catalog_json(
    delivery_root: &Path,
    book: &Book<'_>,
    published_at: &str,
) -> Result<usize, AppError> {
    let mut catalog = crate::runtime::catalog::Catalog::build_from_presets_lenient(
        book.presets,
        &delivery_root.join(MKP_PRESETS_DIR),
    );
    // 最低客户端版本与发布侧同一个来源（规则表）—— 生成与发布写出来的目录**同一代**，
    // 不许因为"谁先跑"而一个带这一格一个不带
    if let Some(publish_root) = delivery_root.parent() {
        let rules = crate::runtime::structure::RuleTable::load(publish_root)?;
        catalog.apply_min_client(&rules);
    }
    catalog.published_at = Some(published_at.to_owned());
    let count = catalog.files.len();
    let text = catalog.to_pretty_json()?;
    crate::fsx::atomic::atomic_write(&delivery_root.join(NEW_CATALOG_FILE), text.as_bytes())?;
    Ok(count)
}

/// **交付面的自查**（预检的「清单 ↔ 文件」一档）：三件事一起对 ——
/// 自产文件在不在 delivery、**登记的每一条**在发布根下取不取得到且字节对不对、delivery 有没有残留。
///
/// ★ **2026-10-04 起口径改成「登记面 == 实体面」**（裁决 C/甲）：不再"只查交付集合"。
/// 现在**catalog 登记的每一条**都要在 `发布根 + catalog.path` 处真能取到 ——
/// A 类查 `presets/`（原地交付），B 类查 `presets/delivery/`（渲染产物）。
///
/// 旧的"登记面刻意比交付面宽"那句话作废了：那是"复制进 delivery"时代的说法，
/// 也正是 0.2mm 那 4 份登记了却取不到的根源（`PUBLISH-ARCHITECTURE.md` §3）。
///
/// **只读**；修复动作是「在工作台生成一次」或重跑发布。
pub fn audit_catalog(delivery_root: &Path, book: &Book<'_>) -> Result<(), String> {
    let Ok(bytes) = std::fs::read(delivery_root.join(NEW_CATALOG_FILE)) else {
        return Ok(()); // 目录还没立起来（没发布过也没生成过）—— 没什么可对的
    };
    let catalog: crate::runtime::catalog::Catalog = serde_json::from_slice(&bytes)
        .map_err(|e| format!("catalog.json 读不出来（{e}）—— 客户端按它做下载校验，坏了要重算"))?;

    let publish_root = match delivery_root.parent() {
        Some(p) => p,
        None => return Err("交付根没有父目录（发布根）—— 布局不对".to_owned()),
    };

    // 自产：content 三份 + catalog.json（生成与发布都会写）—— 只查在不在 delivery 里
    let self_made: std::collections::BTreeSet<&str> = CONTENT_FILES
        .iter()
        .copied()
        .chain([NEW_CATALOG_FILE])
        .collect();
    let publish_only: std::collections::BTreeSet<&str> =
        [MANIFEST_FILE, SOURCE_FILE].into_iter().collect();

    let mut bad: Vec<String> = Vec::new();

    // ① 自产文件（都在 delivery 里）
    for rel in self_made.iter() {
        if !delivery_root.join(rel).exists() {
            bad.push(format!("{rel}（该发的自产文件不在）"));
        }
    }
    // ② **登记的每一条**：按 `发布根 + path` 定位（A 类在 presets/，B 类在 presets/delivery/）
    for f in catalog.files.iter() {
        let on_disk = publish_root.join(&f.path);
        match std::fs::read(&on_disk) {
            Err(_) => bad.push(format!(
                "{}（登记了，发布根里取不到：{}）",
                f.path,
                on_disk.display()
            )),
            Ok(b) => {
                if f.expected_sha().is_none() || f.expected_size().is_none() {
                    bad.push(format!("{}（目录没登记 SHA / 大小）", f.path));
                } else if Some(b.len() as u64) != f.expected_size() {
                    bad.push(format!(
                        "{}（目录记 {:?} 字节，实际 {} 字节）",
                        f.path,
                        f.expected_size(),
                        b.len()
                    ));
                } else if Some(sha256_of(&b).as_str()) != f.expected_sha() {
                    bad.push(format!("{}（大小相同，字节不同）", f.path));
                }
            }
        }
    }
    // ③ delivery 残留：`delivery/` 里有、本次发布会发的东西里没有的（delivery 面，不含 A 类）
    for rel in scan_strays(delivery_root, &delivery_expected_set(book)) {
        if publish_only.contains(rel.as_str()) {
            continue; // manifest / source 是发布台账，生成视角下还没写，不算残留
        }
        bad.push(format!("{rel}（交付目录里的残留）"));
    }

    if bad.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{} 处与目录登记不一致：{}",
            bad.len(),
            bad.join("；")
        ))
    }
}

pub(crate) fn sha256_of(bytes: &[u8]) -> String {
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
    /// ★ A 类资产**不复制进 delivery**（2026-10-04 裁决 C/甲），只做"文件真在"的体检。
    #[test]
    fn write_content_checks_a_class_assets_without_copying_them() {
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

        let delivery = tempfile::tempdir().unwrap();
        let out = write_content(delivery.path(), asset_root.path(), &book).expect("落盘");
        assert_eq!(out.content_files, 3);
        // 可达集 4 条（a1-image + a1-icon + p1s-icon + a1-bbs-04-020），但 **image 类
        // 不登记进交付**（就是整机图那一档）—— 所以体检过的是 3 条
        assert_eq!(out.assets_checked, 3);
        // ★ 但它们**一个字节都不该出现在 delivery 里** —— 这就是"唯一源"
        for a in referenced_assets(&book) {
            let Some(rel) = crate::runtime::catalog::dest_of_asset(&a) else {
                continue;
            };
            assert!(
                !delivery.path().join(&rel).exists(),
                "A 类资产不许被复制进 delivery：{rel}"
            );
        }

        // 三份 JSON 真的在盘上
        for name in ["machine_catalog.json", "bundles.json", "assets_index.json"] {
            assert!(
                delivery.path().join("content").join(name).is_file(),
                "{name}"
            );
        }
        // 12.6（2026-10-04 改口径）：资产索引里的 `path` 是**发布根相对**的
        // （`assets/…`），文件在**载荷根**里原地 —— 不再有 delivery 副本可查。
        let idx = assets_index_json(&referenced_assets(&book));
        for a in idx["assets"].as_array().unwrap() {
            let rel = a["path"].as_str().unwrap();
            assert!(
                rel.starts_with("assets/"),
                "索引里的落点要以 assets/ 开头（发布根基准）：{rel}"
            );
            let src = asset_root.path().join(rel.trim_start_matches("assets/"));
            assert!(src.is_file(), "索引里的 {} 在载荷根里不存在", a["id"]);
        }

        // 缺文件的资产：发布必须拦下，且要说清是哪一条
        let asset_root2 = tempfile::tempdir().unwrap();
        crate::fsx::atomic::atomic_write(&asset_root2.path().join("printers/a1.webp"), b"payload")
            .unwrap();
        let err = write_content(delivery.path(), asset_root2.path(), &book)
            .expect_err("文件不在必须报错");
        assert!(err.message.contains("文件不在"), "实测：{}", err.message);
    }

    /// **真数据上的交付集**（12.6 的真数据版 + 防空转锚点）。
    ///
    /// 锚点来自真数据的几个数：图标 3（P2S/X1C 借 p1s-icon）、
    /// BBS 5（九条套餐引用的是同 5 条 0.4mm）、**mkPreset 9**（2026-10-03 一版一套，
    /// 套餐 assetRefs 装上了那一版的预设登记 —— 它们没有落点、不进索引与交付）
    /// → 可达集 **20**；三份模型与四份 0.2mm BBS 没被引用，**刻意不进交付**。
    /// 12.6 逐条：索引引用的每个文件都在交付目录真实存在。
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

        // JSON 条数锚点：5 台机型、9 个版本、9 条套餐（一版一套）
        let cat = machine_catalog_json(&book);
        let machines = cat["machines"].as_array().unwrap();
        assert_eq!(machines.len(), 5, "真数据 5 台机型");
        let versions: usize = machines
            .iter()
            .map(|m| m["versions"].as_array().unwrap().len())
            .sum();
        assert_eq!(versions, 9, "版本总数变了 —— 说清为什么再改判据");
        assert_eq!(
            bundles_json(&book)["bundles"].as_array().unwrap().len(),
            9,
            "一版一套：A1 三版 + A1_MINI 三版 + P1S / P2S / X1C 各一"
        );

        // 可达集 20 = 整机图 3（bundled 档：被机型 image 字段引用，但 dest 为 None ——
        // 不复制、不下载、构建期随包）+ 图标 3 + BBS 5 + mkPreset 9（无落点）；
        // 模型与 0.2mm BBS 刻意不进
        let referenced = referenced_assets(&book);
        assert_eq!(
            referenced.len(),
            20,
            "可达集条数变了 —— 机型引用或套餐 assetRefs 动了，说清为什么\
             （2026-10-03：套餐改一版一套并装上 mkPreset 登记，11 → 20）"
        );
        assert!(
            referenced.iter().all(|a| a.kind != AssetKind::Model),
            "模型没被任何内容引用，不进交付（PUBLISH-ARCHITECTURE.md §4.5）"
        );
        // 整机图被引用但**不进交付集合**（bundled 档：客户端不下载）；
        // mkPreset 同样无落点 —— 它的产物文件由生成侧按 `mkp/presets/…` 登记，
        // 台账这条只是「哪一版叫什么」的归属，不登记第二份
        for kind in [AssetKind::Image, AssetKind::MkPreset] {
            assert!(
                referenced
                    .iter()
                    .filter(|a| a.kind == kind)
                    .all(|a| crate::runtime::catalog::dest_of_asset(a).is_none()),
                "{kind:?} 类被引用但不进交付集合（无落点）"
            );
        }
        let bbs = referenced
            .iter()
            .filter(|a| a.kind == AssetKind::SlicerProfile)
            .count();
        assert_eq!(
            bbs, 5,
            "BBS 引用去重后 5 条（A1 / A1_MINI 各三份套餐共用一条）"
        );

        // 落盘（真资产根 → 临时交付根）：A 类**只体检不复制**（裁决 C/甲），
        // 12.6 逐条核对 + assetRefs join 闭合
        let delivery = tempfile::tempdir().unwrap();
        let out = write_content(delivery.path(), &asset_root, &book).expect("真数据落盘");
        assert_eq!(
            out.assets_checked, 8,
            "有落点的可达资产 8 条（图标 3 + BBS 5），一条不少一条不多"
        );
        let idx = assets_index_json(&referenced);
        for a in idx["assets"].as_array().unwrap() {
            // 索引里的 path 是**发布根相对**（`assets/…`），文件在载荷根；
            // 而 delivery 里**不该有**它的副本 —— 两条都钉住
            let rel = a["path"].as_str().unwrap();
            assert!(
                rel.starts_with("assets/"),
                "索引落点要以 assets/ 开头：{rel}"
            );
            assert!(
                asset_root.join(rel.trim_start_matches("assets/")).is_file(),
                "索引里的 {} 在载荷根里不存在",
                a["id"]
            );
            assert!(
                !delivery.path().join(rel).exists(),
                "A 类资产不许出现在 delivery 里：{rel}"
            );
        }
        let index_ids: Vec<&str> = idx["assets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["id"].as_str().unwrap())
            .collect();
        for b in book.presets.bundles.items() {
            for r in &b.asset_refs {
                let Some(a) = book.presets.assets.get(r) else {
                    panic!("套餐 {} 引用不存在的资产 {r}", b.id);
                };
                if crate::runtime::catalog::dest_of_asset(a).is_none() {
                    // mkPreset（无落点）不在索引里 —— 它的产物条目由生成侧登记，
                    // 索引再有它就是同一个文件两条真相
                    assert!(
                        !index_ids.iter().any(|i| i.eq_ignore_ascii_case(r)),
                        "套餐 {} 引用的 {} 没有落点，不该出现在资产索引里",
                        b.id,
                        r
                    );
                    continue;
                }
                assert!(
                    index_ids.iter().any(|i| i.eq_ignore_ascii_case(r)),
                    "套餐 {} 引用的 {} 不在资产索引里 —— 引用集漏了它",
                    b.id,
                    r
                );
            }
        }
    }

    /// ★ **登记面 == 实体面**（2026-10-04 裁决 C/甲的核心判据）。
    ///
    /// 旧口径是"只查交付集合"（登记面刻意比交付面宽）；现在**每一条登记都要取得到**，
    /// 判据面是 **发布根 + `catalog.path`**：A 类查 `presets/`、B 类查 `presets/delivery/`。
    #[test]
    fn audit_requires_every_registered_path_to_exist() {
        let mut f = Fixture::load();
        let c = committed();
        let d = crate::workbench::domain::patch::Draft::default();
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

        /*
         * 造出**发布根**形状：`<root>/assets/…`（A 类，载荷根原地）+ `<root>/delivery/…`。
         * 新语义下 `audit_catalog` 按「发布根 + catalog.path」核 —— 所以要这样搭。
         */
        let publish_root = tempfile::tempdir().unwrap();
        let delivery = publish_root.path().join("delivery");
        std::fs::create_dir_all(&delivery).unwrap();
        write_content(&delivery, asset_root.path(), &book).expect("三份 JSON");
        for rel in delivery_expected_set(&book)
            .iter()
            .filter(|p| p.starts_with("mkp/presets/"))
        {
            crate::fsx::atomic::atomic_write(
                &delivery.join(rel),
                format!("# preset {rel}").as_bytes(),
            )
            .unwrap();
        }
        write_catalog_json(&delivery, &book, "2026-10-06T00:00:00Z").expect("目录重算");

        // A 类「实体」在发布根里 —— 把载荷根的内容摆过去（它就是原地交付的那份）
        let assets_on_publish_root = publish_root.path().join("assets");
        for a in referenced_assets(&book) {
            let Some(rel) = crate::runtime::catalog::dest_of_asset(&a) else {
                continue;
            };
            let dst = publish_root.path().join(&rel);
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::copy(asset_root.path().join(&a.path), &dst).unwrap();
        }
        let _ = assets_on_publish_root;

        // 每一条登记都取得到 → 过
        assert!(
            audit_catalog(&delivery, &book).is_ok(),
            "全齐时该过：{:?}",
            audit_catalog(&delivery, &book)
        );

        // 把发布根里**某一条 A 类资产**删掉 → 必须抓到（这就是 0.2mm 那种形状）
        let victim = referenced_assets(&book)
            .into_iter()
            .find_map(|a| crate::runtime::catalog::dest_of_asset(&a))
            .expect("夹具有一条 A 类资产");
        std::fs::remove_file(publish_root.path().join(&victim)).unwrap();
        let err = audit_catalog(&delivery, &book).unwrap_err();
        assert!(
            err.contains(&victim) && err.contains("取不到"),
            "登记了却取不到必须报出来：{err}"
        );
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
        // delivery 面（2026-10-04 起**不含 A 类资产** —— 它们原地交付）
        let expected = delivery_expected_set(&book);
        assert_eq!(
            expected.len(),
            10,
            "content 3 + manifest 1 + catalog.json 1 + source.json 1 + **release.json 1（M6 起\
             住交付根）** + mkp 产物 3 = 10（A 类不在 delivery 面：它原地住在发布根的 assets/ 下）"
        );
        assert!(expected.contains("mkp/presets/A1-standard.toml"));
        assert!(
            !expected.iter().any(|p| p.starts_with("assets/")),
            "A 类资产不进 delivery 面：{expected:?}"
        );
        assert!(
            expected.contains(NEW_CATALOG_FILE),
            "新世界目录在交付集合里"
        );
        assert!(
            expected.contains(SOURCE_FILE),
            "Bootstrap（source.json）在交付集合里"
        );

        // ★ 发布根形状：`<publish_root>/delivery`（发布物）+ `<publish_root>/assets`（A 类原地）
        let publish_root = tempfile::tempdir().unwrap();
        let delivery = publish_root.path().join("delivery");
        std::fs::create_dir_all(&delivery).unwrap();
        for a in referenced_assets(&book) {
            let Some(rel) = crate::runtime::catalog::dest_of_asset(&a) else {
                continue;
            };
            let dst = publish_root.path().join(&rel);
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::copy(asset_root.path().join(&a.path), &dst).unwrap();
        }
        // 夹具三版的 mkp 产物（wb_generate 的等价物：文件在，内容任意）；
        // 落点就是 `wb_generate` 落的那一格
        for name in ["A1-standard.toml", "A1-fast.toml", "P1S-lite.toml"] {
            crate::fsx::atomic::atomic_write(
                &delivery.join(MKP_PRESETS_DIR).join(name),
                format!("# preset {name}").as_bytes(),
            )
            .unwrap();
        }

        // **残留拦截**（13.4）：放一个不在集合内的文件 → 中止，且不写任何东西
        crate::fsx::atomic::atomic_write(&delivery.join("stale.json"), b"old").unwrap();
        let meta = PublishMeta {
            stamp: "2026-09-24T00:00:00Z".to_owned(),
            channel: "stable".to_owned(),
            version: String::new(),
        };
        let err = publish_into(&delivery, asset_root.path(), &book, &meta)
            .expect_err("有残留必须中止发布");
        assert!(err.message.contains("残留"), "实测：{}", err.message);
        assert!(
            err.detail.unwrap_or_default().contains("stale.json"),
            "要列出残留是哪个文件"
        );
        assert!(!delivery.join("manifest.json").exists(), "不写 manifest");
        assert_eq!(
            std::fs::read(delivery.join("stale.json")).unwrap(),
            b"old",
            "中止发布时一个字节都不该动"
        );

        // **清理残留**（13.5）：进回收站（保留相对路径），不直接删
        let trash = tempfile::tempdir().unwrap();
        let strays = scan_strays(&delivery, &expected);
        assert_eq!(strays, vec!["stale.json"]);
        let moved = clean_strays(&delivery, &strays, trash.path(), "20260924").unwrap();
        assert_eq!(moved, 1);
        assert!(!delivery.join("stale.json").exists());
        assert_eq!(
            std::fs::read(trash.path().join("delivery/20260924/stale.json")).unwrap(),
            b"old",
            "回收站里要能找回原文件"
        );

        // **重发成功**：manifest v3、无 bundles 字段（第二份套餐列表删掉了）、
        // 条目 = mkp 3 + 资产 3（可达 4 条里 image 类不进）
        let out = publish_into(&delivery, asset_root.path(), &book, &meta).expect("发布");
        assert_eq!(out.presets, 3);
        assert_eq!(out.assets_copied, 3);
        assert_eq!(out.files, 6);

        let manifest_text = std::fs::read_to_string(delivery.join("manifest.json")).unwrap();
        let manifest: serde_json::Value = serde_json::from_str(&manifest_text).unwrap();
        assert_eq!(manifest["manifestVersion"], 3, "结构变了就升版本");
        assert!(
            manifest.get("bundles").is_none(),
            "套餐的唯一真相在 bundles.json"
        );

        // **新世界目录**：与客户端同 schema、指纹非空、条目 = 有产物的版本；
        // 内容字节的 SHA 与 delivery 里真实字节一致（消费端将来拿它当校验期望值）
        let new_catalog = crate::runtime::Catalog::parse(
            &std::fs::read(delivery.join(NEW_CATALOG_FILE)).expect("catalog.json 该被写出"),
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
            /*
             * 按目录登记的 path 去**发布根**取（不是按 file_name 拼，也不在 delivery 里找）——
             * 客户端就是这么取的：数据源地址 + path。A 类在 `<发布根>/assets/`，
             * B 类在 `<发布根>/delivery/mkp/presets/` —— 同一套 path 语义，两个落点。
             */
            let bytes = std::fs::read(publish_root.path().join(&f.path))
                .unwrap_or_else(|e| panic!("目录登记了 {}，发布根上没有：{e}", f.path));
            use sha2::{Digest, Sha256};
            assert_eq!(
                Some(crate::runtime::catalog::hex(&Sha256::digest(&bytes)).as_str()),
                f.expected_sha()
            );
        }

        // **13.8**：manifest 每条 relativePath 的文件存在、sha256 与真实字节一致；
        // 且交付目录里除 manifest/content 外，没有清单之外的文件
        use sha2::{Digest, Sha256};
        let assets = manifest["assets"].as_array().unwrap();
        assert_eq!(assets.len(), 6);
        for a in assets {
            let rel = a["relativePath"].as_str().unwrap();
            // 发布布局的锚点：相对**发布根**（A 类 `assets/…`、B 类 `delivery/mkp/presets/…`）
            assert!(
                rel.starts_with("assets/") || rel.starts_with("delivery/"),
                "交付文件的 relativePath 是发布根相对的：{rel}"
            );
            let p = publish_root.path().join(rel);
            let bytes = std::fs::read(&p).unwrap_or_else(|_| panic!("{} 不在发布根", a["id"]));
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
        collect_files(&delivery, &mut actual);
        let mut actual_rel: Vec<String> = actual
            .iter()
            .map(|p| {
                p.strip_prefix(&delivery)
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
        /*
         * 只比 **delivery 面**（B 类）：manifest 里的 A 类条目相对发布根是 `assets/…`，
         * 而 delivery 里本来就没有它们 —— 拿整个 manifest 去比会误判。
         */
        let mut listed: Vec<String> = assets
            .iter()
            .map(|a| a["relativePath"].as_str().unwrap().to_owned())
            // B 类的 relativePath 是 `delivery/mkp/presets/…`（发布根相对），
            // 比的是 delivery 面 ⇒ 去掉 `delivery/` 那一段再比
            .filter(|rel| rel.starts_with("delivery/"))
            .map(|rel| rel["delivery/".len()..].to_owned())
            .collect();
        listed.sort();
        assert_eq!(
            actual_rel, listed,
            "交付目录里的文件与 manifest 的 delivery 面条目必须一一对应"
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

        let expected = delivery_expected_set(&book);
        // 反空转锚点（2026-10-04 改口径后）：content 3 + manifest 1 + catalog.json 1
        // + source.json 1 + **release.json 1（M6 起软件发布信息也住交付根）** + mkp 9
        // （五台机型全部有套餐）= 16 —— **A 类资产不在 delivery 面**
        assert_eq!(
            expected.len(),
            16,
            "delivery 面条数变了 —— 说清为什么（裁决 C/甲后 A 类资产原地交付，不再进 delivery）"
        );
        assert!(
            expected.contains(crate::runtime::source::RELEASE_FILE),
            "release.json 是交付根的合法住客（M6 软件发布链），不算残留 —— \
             少了它发布闸永远拦着预设发布，「清理残留」还会把客户端更新要用的文件扔进回收站"
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
        assert!(
            !expected.iter().any(|p| p.starts_with("assets/")),
            "★ A 类资产（BBS / 模型 / 图标）不进 delivery 面 —— 原地交付（裁决 C/甲）"
        );

        // 空目录零残留（还没发布过是正常状态，不是错误）
        let delivery = tempfile::tempdir().unwrap();
        assert!(scan_strays(delivery.path(), &expected).is_empty());
    }

    /// **发布布局 = 客户端落点**（第 2 步的核心判据，真数据版）。
    ///
    /// 这条是这一刀要防的那种错：发布侧曾经自己拼一套（`assets/<载荷 path>` +
    /// `presets/mkp/<名字>`），而客户端认的落点是 `mkp/<kind 目录>/…` ——
    /// **URL 拼得上、落点对不上**：上传成功、用户点了下载却 404，
    /// 而且两边各自"看着都对"。所以判据不做字符串锚点，直接对真字节。
    ///
    /// ★ 2026-10-04 裁决 C/甲之后，**"写盘面"这一半改了意思**：
    /// A 类资产**原地交付**（`presets/assets/…` 就是它唯一的实体），`write_content`
    /// 只体检不复制 —— 于是 delivery 里除三份目录 JSON 外**零交付文件**，而"登记面"
    /// 的每一条都在 `发布根 + catalog.path` 处取得到真字节。两半都钉住：
    ///
    /// - delivery 里不该出现任何 A 类副本（复制 = 同一份字节两个真相）；
    /// - 目录登记的每一条，在发布根下真有那份文件、SHA / 大小与登记一致。
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

        let delivery = tempfile::tempdir().unwrap();
        write_content(delivery.path(), &asset_root, &book).expect("资产落盘");

        // 与发布侧同一次构建（产物目录给的是交付根里那一格；这里没跑 wb_generate，
        // 宽松构建会跳过没有产物的版本 —— 资产的登记面不受影响）
        let catalog = crate::runtime::catalog::Catalog::build_from_presets_lenient(
            &real,
            &delivery.path().join(MKP_PRESETS_DIR),
        );

        // 交付根里到底写了些什么（三份目录 JSON 不算交付文件）
        let mut on_disk: Vec<PathBuf> = Vec::new();
        collect_files(delivery.path(), &mut on_disk);
        let mut written: Vec<String> = on_disk
            .iter()
            .map(|p| {
                p.strip_prefix(delivery.path())
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .filter(|rel| !CONTENT_FILES.contains(&rel.as_str()))
            .collect();
        written.sort();

        // ★ **A 类一个字节都不进 delivery**：它原地住在发布根的 `assets/` 下
        // （这里是 `write_content` 之后 —— 它只体检、不复制）
        assert!(
            written.is_empty(),
            "A 类资产原地交付，delivery 里一条副本都不该有：{written:?}"
        );

        /*
         * **发布根** = 资产载荷根的上一级（`presets/assets` 往上一级就是 `presets/`）。
         * A 类原地交付，所以 `发布根 + catalog.path` 正好是载荷根里那份真文件 ——
         * 与客户端 `baseUrl + path` 取的是同一处。
         */
        let publish_root = asset_root
            .parent()
            .expect("资产载荷根必有父目录（发布根 presets/）");
        assert!(
            publish_root.join("assets").is_dir(),
            "发布根下就该有 assets/ 那一层：{}",
            publish_root.display()
        );

        // **本条判据的核心**：目录登记的每一条，在发布根下都取得到，
        // 而且 SHA / 大小与真字节一致 —— 客户端就是按那个路径拼 URL 去取的
        assert_eq!(
            catalog.files.len(),
            15,
            "反空转：说明书登记台账全部 15 条（9 BBS + 3 图标 + 3 模型）；\
             没跑 wb_generate 所以没有 mkp 产物条目（宽松构建跳过）"
        );
        for f in &catalog.files {
            let bytes = std::fs::read(publish_root.join(&f.path))
                .unwrap_or_else(|e| panic!("目录登记了 {}，发布根上取不到：{e}", f.path));
            assert_eq!(
                Some(bytes.len() as u64),
                f.expected_size(),
                "{} 的大小",
                f.path
            );
            assert_eq!(
                Some(sha256_of(&bytes).as_str()),
                f.expected_sha(),
                "{} 的 SHA",
                f.path
            );
        }
    }

    /* ---------- 第十七刀：Bootstrap（规范化 / 生成 / 两端形状） ---------- */

    /// **仓库地址** → 默认 `main/presets/delivery/source.json` 的 raw ——
    /// 用户只表达"我有一个仓库"，ref 与交付路径由系统补（Bootstrap 是内部机制）。
    /// `.git` 只是克隆地址的一种写法，不是产品语义，去掉即可。
    #[test]
    fn repo_urls_get_the_default_delivery_entry() {
        let want = "https://raw.githubusercontent.com/MuCoreBenC/MKPSupportEase/main/presets/delivery/source.json";
        for input in [
            "https://github.com/MuCoreBenC/MKPSupportEase",
            "https://github.com/MuCoreBenC/MKPSupportEase/",
            "https://github.com/MuCoreBenC/MKPSupportEase.git",
            "  https://github.com/MuCoreBenC/MKPSupportEase.git/  ",
        ] {
            assert_eq!(
                normalize_bootstrap_url(input).unwrap_or_else(|e| panic!("{input} 该被接受：{e}")),
                want,
                "输入：{input}"
            );
        }
    }

    /// **blob 页**按人显式指的转 raw（尊重他的选择，不强行拉回默认路径）；
    /// **raw / 自建源**原样（只收拾空白与尾斜杠）。
    #[test]
    fn blob_urls_turn_into_raw_urls() {
        assert_eq!(
            normalize_bootstrap_url(
                "https://github.com/MuCoreBenC/MKPSupportEase/blob/main/presets/delivery/source.json"
            )
            .expect("blob 页该转成 raw"),
            "https://raw.githubusercontent.com/MuCoreBenC/MKPSupportEase/main/presets/delivery/source.json"
        );
        /* 人指了别的路径也照办 —— 不因为"默认是 A"就把显式的 B 改掉 */
        assert_eq!(
            normalize_bootstrap_url("https://github.com/o/r/blob/dev/x/y/source.json")
                .expect("blob 页该转成 raw"),
            "https://raw.githubusercontent.com/o/r/dev/x/y/source.json"
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

    /// **Gitee 是同一座桥**：仓库地址 / `.git` 补默认入口、blob 转 raw、
    /// **raw 与网页同域所以"已经是 raw"要认出来原样吐回**、目录页拒。
    /// 最后一例是入库配置里的真值（`workbench/bootstrap.json` 的 giteeBootstrapUrl）——
    /// 保存时走一遍规范化必须原样通过，不能被改写。
    #[test]
    fn gitee_urls_follow_the_same_bridge() {
        let want =
            "https://gitee.com/MuCoreBenC/MKPSupportEase/raw/main/presets/delivery/source.json";
        for input in [
            "https://gitee.com/MuCoreBenC/MKPSupportEase",
            "https://gitee.com/MuCoreBenC/MKPSupportEase/",
            "https://gitee.com/MuCoreBenC/MKPSupportEase.git",
        ] {
            assert_eq!(
                normalize_bootstrap_url(input).unwrap_or_else(|e| panic!("{input} 该被接受：{e}")),
                want,
                "输入：{input}"
            );
        }
        assert_eq!(
            normalize_bootstrap_url(
                "https://gitee.com/MuCoreBenC/MKPSupportEase/blob/dev/x/y/source.json"
            )
            .expect("blob 页该转成 raw"),
            "https://gitee.com/MuCoreBenC/MKPSupportEase/raw/dev/x/y/source.json"
        );
        assert_eq!(
            normalize_bootstrap_url(want).expect("已是 raw 直链，原样"),
            want
        );
        let e = normalize_bootstrap_url("https://gitee.com/o/r/tree/main/presets").unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::InvalidArgument, "目录页拒");
        let e = normalize_bootstrap_url("https://gitee.com/o").unwrap_err();
        assert_eq!(
            e.code,
            crate::error::ErrorCode::InvalidArgument,
            "单段不是仓库地址"
        );
    }

    /// **目录页 / 非 http(s) / 空** —— 拒（"那不是文件"这件事要说得出）。
    /// `tree/…` 与仓库地址的区别是：前者说不清"要哪个发布入口"，后者有默认入口。
    #[test]
    fn directory_pages_and_non_http_are_refused() {
        for bad in [
            "https://github.com/MuCoreBenC/MKPSupportEase/tree/main/presets",
            "https://github.com/MuCoreBenC/MKPSupportEase/tree/main",
            "https://github.com/MuCoreBenC",
            "https://github.com/MuCoreBenC/MKPSupportEase/blob/main",
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

    /// 生成的 Manifest 是**全相对引用**的寻址规则声明；**交付面禁止绝对 URL**
    /// （双镜像中立，CI 也有负向断言守着）。而且**客户端解析器认得发布侧写的
    /// 这一份**（两端同一形状，一边写一边读，钉住）。
    #[test]
    fn bootstrap_json_is_minimal_and_readable_by_the_client_parser() {
        let v = bootstrap_json();
        assert_eq!(v["sourceSchema"], crate::runtime::resolver::MANIFEST_SCHEMA);
        assert_eq!(v["catalog"], NEW_CATALOG_FILE);
        assert_eq!(v["catalog"], crate::runtime::source::CATALOG_FILE);
        assert_eq!(v["release"], crate::runtime::source::RELEASE_FILE);
        assert_eq!(v["filesRoot"], "..");
        let text = v.to_string();
        assert!(
            !text.contains("http://") && !text.contains("https://"),
            "交付面禁止绝对 URL（双镜像中立）：{text}"
        );

        let parsed = crate::runtime::source::parse_manifest_at(
            "https://host/x/source.json",
            text.as_bytes(),
        )
        .expect("客户端解析器认得发布侧写的这一份");
        assert_eq!(
            parsed.catalog_url().expect("该推得出"),
            "https://host/x/catalog.json"
        );
        assert_eq!(
            parsed.release_url().expect("声明了就该给"),
            "https://host/x/release.json"
        );
    }
    /// ★ **两本账的差集语义**（`PUBLISH-ARCHITECTURE.md` §4.5；2026-10-05 作者要求把
    /// "为什么"从注释升格为判据）：
    ///
    /// - `catalog.files[]` = 「所有客户端可见资源」的登记面；
    /// - `manifest.assets` = 「本版本完整性 / 下载管理范围」= **引用可达交付子集**；
    /// - 不变式：`manifest ⊆ catalog`，且 **manifest 恰好等于** 交付集合
    ///   （B 类产物全部 + 有落点的可达资产）—— 多一条少一条都红；
    /// - 差集（catalog − manifest）只许由"登记了但当前无引用"的资产构成 ——
    ///   今天是 0.2mm BBS 4 份 + 模型 3 份，**不是漏生成**。
    #[test]
    fn the_manifest_covers_exactly_the_referenced_deliverable_set() {
        let repo = crate::workbench::paths::repo_root();
        let presets_root = repo.join("presets");
        let mut presets =
            crate::presetdata::Presets::load_from(&presets_root).expect("真源读得出来");
        presets.set_asset_root(&repo.join(crate::runtime::catalog::REPO_ASSET_ROOT));
        let c = Committed::default();
        let d = crate::workbench::domain::patch::Draft::default();
        let book = Book::new(&presets, &c, &d);

        let delivery_root = presets_root.join("delivery");
        let manifest: serde_json::Value = serde_json::from_slice(
            &std::fs::read(delivery_root.join(MANIFEST_FILE)).expect("入库的 manifest.json 在"),
        )
        .expect("manifest 该解析得动");
        let catalog: crate::runtime::catalog::Catalog = serde_json::from_slice(
            &std::fs::read(delivery_root.join(NEW_CATALOG_FILE)).expect("入库的 catalog.json 在"),
        )
        .expect("catalog 该解析得动");

        // 期望交付集的两半：资产半 = 有落点的可达资产；B 类半 = catalog 登记的全部产物
        //（真仓库里 9 份产物都已生成并入库 —— build_state 属于工作台会话态，
        //  这条真数据判据以盘上交付面与登记面为准）
        let expected_assets: std::collections::BTreeSet<String> = referenced_assets(&book)
            .iter()
            .filter_map(crate::runtime::catalog::dest_of_asset)
            .collect();

        let manifest_paths: std::collections::BTreeSet<String> = manifest["assets"]
            .as_array()
            .expect("manifest.assets 是数组")
            .iter()
            .map(|a| a["relativePath"].as_str().expect("relativePath").to_owned())
            .collect();
        let catalog_paths: std::collections::BTreeSet<String> =
            catalog.files.iter().map(|f| f.path.clone()).collect();

        // ① 资产半：manifest 的资产条目**恰好等于**有落点的可达资产
        let manifest_assets: std::collections::BTreeSet<String> = manifest_paths
            .iter()
            .filter(|p| !p.starts_with("delivery/"))
            .cloned()
            .collect();
        assert_eq!(
            manifest_assets, expected_assets,
            "manifest 的资产半必须恰好等于可达有落点集 —— 多了（登记了没发）或少了（发了没登记）都红"
        );
        // ② B 类半：catalog 登记的每一条产物都在 manifest 里（登记了产物却没发 = 漏发）
        let catalog_b: std::collections::BTreeSet<String> = catalog_paths
            .iter()
            .filter(|p| p.starts_with("delivery/"))
            .cloned()
            .collect();
        let manifest_b: std::collections::BTreeSet<String> = manifest_paths
            .iter()
            .filter(|p| p.starts_with("delivery/"))
            .cloned()
            .collect();
        assert_eq!(catalog_b, manifest_b, "B 类登记面 == 交付面");
        // ③ 不变式：manifest ⊆ catalog
        assert!(
            manifest_paths.is_subset(&catalog_paths),
            "manifest ⊆ catalog（说明书不许登记目录里没有的资源）"
        );
        // ④ 差集的每一条都必须"不在可达有落点集合"里 —— 即纯登记、当前无人引用
        for f in &catalog.files {
            if !manifest_paths.contains(&f.path) {
                assert!(
                    !expected_assets.contains(&f.path),
                    "差集条目 {} 在可达交付集里却没进 manifest —— 那才是漏发",
                    f.path
                );
            }
        }
    }

    /// **交付面 catalog 与重建一致**（`RESOURCE-ADDRESSING-ROADMAP.md` §6.2；
    /// CLI 出口 `gen-catalog` 的交付重算与工作台 `write_catalog_json` 同一条构建内核 ——
    /// 这条判据盯"入库的那份没有未经审阅的变化"，与 `embedded_matches_rebuild` 同思路）。
    ///
    /// ★ `publishedAt` **不参与这条比对**（2026-10-06）：它是**事件事实**（这一次发布的
    /// 时刻，发布侧盖的），**不来自源** —— 重建永远算不出它，那不是"不一致"。所以判据把
    /// 入库那份的戳**补回重建结果上**再逐字节比：除了这一格，一个字节都不许差。
    ///
    /// 与随包那份正好相反：`catalog.generated.json` **刻意不带戳**（它要逐字节可复现，
    /// 见 `embedded_matches_rebuild`）。两种目录的差别就是"有没有发生过发布事件"。
    #[test]
    fn delivery_catalog_matches_rebuild() {
        let repo = crate::workbench::paths::repo_root();
        let presets_root = repo.join("presets");
        let mut presets =
            crate::presetdata::Presets::load_from(&presets_root).expect("真源读得出来");
        presets.set_asset_root(&repo.join(crate::runtime::catalog::REPO_ASSET_ROOT));

        let mut rebuilt = crate::runtime::catalog::Catalog::build_from_presets_lenient(
            &presets,
            &repo.join("presets/delivery/mkp/presets"),
        );
        let rules = crate::runtime::structure::RuleTable::load(&presets_root).expect("规则表在");
        rebuilt.apply_min_client(&rules);

        let committed = std::fs::read_to_string(repo.join("presets/delivery/catalog.json"))
            .expect("入库的交付面 catalog 在（2026-10-05 起入库）");
        /* 戳从**入库那份**里取、补到重建结果上：它是发布事件的痕迹，不是源的一部分
        （发布侧 `write_catalog_json` 盖的，与 manifest 的 `updated` 同一个戳） */
        rebuilt.published_at = crate::runtime::catalog::Catalog::parse(committed.as_bytes())
            .expect("入库的交付面 catalog 读得出来")
            .published_at;
        assert_eq!(
            committed.trim(),
            rebuilt.to_pretty_json().expect("该序列化得出").trim(),
            "交付面 catalog 与重建不一致 —— 改了源/产物后重跑 `cargo run --bin gen-catalog`"
        );
    }
}
