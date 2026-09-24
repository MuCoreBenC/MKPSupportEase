//! 交付层（b05 Task 12）：`dist-presets/` 的**目录类 JSON** 与资产复制。
//!
//! # 目录结构定稿（12.1；doc §7 的示意在此落定）
//!
//! ```text
//! dist-presets/
//! ├── content/
//! │   ├── machine_catalog.json     机型 + 版本 + 关系（12.2）
//! │   ├── bundles.json             套餐 + 包含什么（12.3）
//! │   └── assets_index.json        资产清单 + 位置 + 归属（12.4）
//! ├── presets/
//! │   └── mkp/<preset_file_name>   参数本体（wb_generate 落的，9 份起）
//! ├── assets/                      资产根（public/assets/）的**引用可达子集**
//! │   └── printers/ icons/ models/ bbs/…   path 在两个根下同形
//! └── manifest.json                最后写（wb_publish 收尾）
//! ```
//!
//! 两条定稿决定，各有一条理由：
//!
//! - **`presets/mkp/` 子层保留**（不收成示意里的一层）：MKP 预设与 BBS 预设是两类
//!   预设（G-3 的切片器开放维度），子层给「按预设类型」留位置，而且 `wb_generate`
//!   已经按这个形状在写 —— 定稿是**承认现状为契约**，不是另起一套；
//! - **`assets/` 沿用资产根的目录形状**（`printers/` 而不是示意里的 `machines/`）：
//!   资产 path 在两个根下**逐字节同形**，资产索引 JSON 的位置字段在交付根下直接可用
//!   —— 不存在第二份路径映射。改用示意的 `machines/` 等于把 21 条 path 全部重写一遍，
//!   而那正是 assets.toml 刻意避免的「路径只有一处」被破坏。
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
//!   `mkp_preset` 条目的 id 沿用**上游**的（`a1_mkp_standard`；资产域 enum 刻意没有
//!   mkpPreset 档，doc §12.5），资产条目的 id 用**资产域** Asset.id（`a1-image`）。
//!   `machine_catalog.json` 版本条目的 `mkpPresetAssetId` 是**连接键**（指向 manifest
//!   里 mkp_preset 条目），不是资产域 Asset ID —— 词汇撞名，域不同。
//! - **resourceType 词汇**：新条目用资产域 `kind.key()`（`image` / `icon` / `model` /
//!   `slicerProfile`），不用上游的 `bbs_profile` —— manifest 与 assets_index 是同一批
//!   资产的两种视图，join 键（id + type）必须一致；旧词汇是迁移输入（doc §6.2）。

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::AppError;
use crate::workbench::domain::derive::Book;
use crate::workbench::presets::{Asset, AssetKind};

use super::build::preset_file_name;

/// 交付目录里资产子树的根名。**与资产根 public/assets/ 的形状一致**（见模块头）
pub const ASSETS_DIR: &str = "assets";
/// 目录类 JSON 的子目录
pub const CONTENT_DIR: &str = "content";
/// manifest 的固定文件名
pub const MANIFEST_FILE: &str = "manifest.json";

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
    /// 相对交付根 `assets/` 的一段 —— 与资产根下的 path 同形
    path: &'a str,
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
                        mkp_preset_asset_id: book
                            .version(&uid)
                            .and_then(|x| x.mkp_preset.as_ref())
                            .map(|p| p.asset_id.clone()),
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
/// 位置是交付根 `assets/` 下的一段（与资产根同形）
pub fn assets_index_json(assets: &[Asset]) -> serde_json::Value {
    let entries: Vec<AssetIndexEntry<'_>> = assets
        .iter()
        .map(|a| AssetIndexEntry {
            id: &a.id,
            kind: a.kind,
            machine_id: &a.machine_id,
            name: &a.name,
            path: &a.path,
            slicer: &a.slicer,
            profile: &a.profile,
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
    /// 复制进 `assets/` 的资产文件数
    pub assets_copied: usize,
}

/// 写三份目录 JSON，并把引用可达的资产从资产根复制进交付根。
///
/// **资产根是参数**（依赖注入）：生产上是 `public/assets/`，测试给临时目录 ——
/// 「登记了但文件不在」在这里是错误（加载期只查 id 认不认得出，
/// 文件在不在正是发布要守的最后一道），但缺了它的测试就造不出夹具。
///
/// 复制走「读源 → 原子写目标」：交付目录是只读输出（doc §7 原则 3），
/// 原子写保证半份文件不会出现在那里。
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
    let out_root = dist_root.join(ASSETS_DIR);
    for a in &referenced {
        let src = asset_root.join(&a.path);
        let bytes = std::fs::read(&src).map_err(|e| {
            AppError::not_found(format!("资产 {} 的文件不在：{}", a.id, src.display()))
                .with_detail(e.to_string())
        })?;
        crate::fsx::atomic::atomic_write(&out_root.join(&a.path), &bytes)?;
        copied += 1;
    }

    Ok(DistContent {
        content_files: 3,
        assets_copied: copied,
    })
}

/* ---------- 发布（b05 Task 13） ---------- */

/// **交付集合**（13.1）：本次发布应当存在于交付目录的全部文件（相对路径）。
///
/// 它是从引用关系**算出来**的：`content/` 三份自产、`manifest.json` 自产、
/// `assets/<path>` 来自引用可达集、`presets/mkp/<file_name>` 来自「有产物的版本」
/// （产物名由命名函数算出）。残留在语义上就是「目录里有、这个集合里没有」。
pub fn deliverable_set(book: &Book<'_>) -> BTreeSet<String> {
    let mut set: BTreeSet<String> = BTreeSet::new();
    set.extend(CONTENT_FILES.map(str::to_owned));
    set.insert(MANIFEST_FILE.to_owned());
    for a in referenced_assets(book) {
        set.insert(format!("{ASSETS_DIR}/{}", a.path));
    }
    for v in book.versions() {
        if v.mkp_preset.is_some() {
            set.insert(format!(
                "presets/mkp/{}",
                preset_file_name(&v.machine_id, &v.version_id)
            ));
        }
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

/// 发布的元信息（从调用方带进来：clock 与上游 compat 都不属于交付层）
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
/// 4. manifest 最后写（13.7，原子写）。
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
        let Some(p) = &v.mkp_preset else {
            continue; // 暂无资源：跳过，不报错
        };
        let name = preset_file_name(&v.machine_id, &v.version_id);
        let rel = format!("presets/mkp/{name}");
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
            id: p.asset_id.clone(),
            resource_type: "mkp_preset".to_owned(),
            machine_id: v.machine_id.clone(),
            file_name: name,
            relative_path: rel,
        });
        presets_count += 1;
    }

    for a in &referenced {
        let rel = format!("{ASSETS_DIR}/{}", a.path);
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
    // 清单最后写。`fsx::atomic` 是仓库唯一的写盘出口
    crate::fsx::atomic::atomic_write_json(&dist_root.join(MANIFEST_FILE), &manifest)?;

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
        let book = Book::new(Some(&f.up), &f.presets, &c, &d);
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
        let book = Book::new(Some(&f.up), &f.presets, &c, &d);

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
            versions[0]["mkpPresetAssetId"], "a1_mkp_standard",
            "版本→manifest 资产的连接键"
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
        let book = Book::new(Some(&f.up), &f.presets, &c, &d);

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
        assert_eq!(out.assets_copied, 4);

        // 三份 JSON 真的在盘上
        for name in ["machine_catalog.json", "bundles.json", "assets_index.json"] {
            assert!(dist.path().join("content").join(name).is_file(), "{name}");
        }
        // 12.6：资产索引引用的每一条都在交付目录里真实存在
        let idx = assets_index_json(&referenced_assets(&book));
        for a in idx["assets"].as_array().unwrap() {
            let p = dist
                .path()
                .join(ASSETS_DIR)
                .join(a["path"].as_str().unwrap());
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
    /// 锚点来自真数据的三个数：机型图 5（A2L 占位无图）、图标 3（P2S/X1C 借
    /// p1s-icon）、BBS 5（五条套餐各一条 0.4mm）→ 可达集 **13**；三份模型与
    /// 四份 0.2mm BBS 没被引用，**刻意不进交付**（Task 13 的可达性分析收窄它们，
    /// 集合本身不变）。12.6 逐条：索引引用的每个文件都在交付目录真实存在。
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
        let f = Fixture::load();
        let c = Committed::default();
        let d = crate::workbench::domain::patch::Draft::default();
        let book = Book::new(Some(&f.up), &real, &c, &d);

        // JSON 条数锚点：6 台机型（含 A2L 占位）、10 个版本、5 条套餐
        let cat = machine_catalog_json(&book);
        let machines = cat["machines"].as_array().unwrap();
        assert_eq!(machines.len(), 6, "真数据 6 台机型（A2L 占位也在清单里）");
        let versions: usize = machines
            .iter()
            .map(|m| m["versions"].as_array().unwrap().len())
            .sum();
        assert_eq!(versions, 10, "版本总数变了 —— 说清为什么再改判据");
        assert_eq!(bundles_json(&book)["bundles"].as_array().unwrap().len(), 5);

        // 可达集 13 = 图 5 + 图标 3 + BBS 5；模型与 0.2mm BBS 刻意不进
        let referenced = referenced_assets(&book);
        assert_eq!(
            referenced.len(),
            13,
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
        assert_eq!(out.assets_copied, 13);
        let idx = assets_index_json(&referenced);
        for a in idx["assets"].as_array().unwrap() {
            let p = dist
                .path()
                .join(ASSETS_DIR)
                .join(a["path"].as_str().unwrap());
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
        let f = Fixture::load();
        let c = committed();
        let d = crate::workbench::domain::patch::Draft::default();
        let book = Book::new(Some(&f.up), &f.presets, &c, &d);

        // 交付集合（13.1）：content 3 + manifest + 夹具可达资产 4 + mkp 产物 3
        let expected = deliverable_set(&book);
        assert_eq!(
            expected.len(),
            11,
            "content 3 + manifest 1 + assets 4 + mkp 3"
        );
        assert!(expected.contains("presets/mkp/A1-standard.toml"));
        assert!(expected.contains("assets/bbs/A1/process.json"));

        let dist = tempfile::tempdir().unwrap();
        let asset_root = tempfile::tempdir().unwrap();
        for rel in [
            "printers/a1.webp",
            "icons/a1.svg",
            "bbs/A1/process.json",
            "icons/p1s.svg",
        ] {
            crate::fsx::atomic::atomic_write(&asset_root.path().join(rel), b"payload").unwrap();
        }
        // 夹具三版的 mkp 产物（wb_generate 的等价物：文件在，内容任意）
        for name in ["A1-standard.toml", "A1-fast.toml", "P1S-lite.toml"] {
            crate::fsx::atomic::atomic_write(
                &dist.path().join("presets").join("mkp").join(name),
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
        // 条目 = mkp 3 + 资产 4
        let out = publish_into(dist.path(), asset_root.path(), &book, &meta).expect("发布");
        assert_eq!(out.presets, 3);
        assert_eq!(out.assets_copied, 4);
        assert_eq!(out.files, 7);

        let manifest_text = std::fs::read_to_string(dist.path().join("manifest.json")).unwrap();
        let manifest: serde_json::Value = serde_json::from_str(&manifest_text).unwrap();
        assert_eq!(manifest["manifestVersion"], 3, "结构变了就升版本");
        assert!(
            manifest.get("bundles").is_none(),
            "套餐的唯一真相在 bundles.json"
        );

        // **13.8**：manifest 每条 relativePath 的文件存在、sha256 与真实字节一致；
        // 且交付目录里除 manifest/content 外，没有清单之外的文件
        use sha2::{Digest, Sha256};
        let assets = manifest["assets"].as_array().unwrap();
        assert_eq!(assets.len(), 7);
        for a in assets {
            let p = dist.path().join(a["relativePath"].as_str().unwrap());
            let bytes = std::fs::read(&p).unwrap_or_else(|_| panic!("{} 不在交付目录", a["id"]));
            let h = format!("{:x}", Sha256::digest(&bytes));
            assert_eq!(a["sha256"], h, "{} 的哈希与真实字节不符", a["id"]);
            assert_eq!(a["size"], bytes.len() as u64);
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
            .filter(|rel| rel != MANIFEST_FILE && !CONTENT_FILES.contains(&rel.as_str()))
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
    /// mkp 产物名 9 个（命名函数逐版算出）、资产 13 条 ——
    /// 集合计数锚点变了就说明清单或套餐变了
    #[test]
    fn the_real_deliverable_set_has_the_expected_shape() {
        let Some(root) = crate::workbench::paths::presets_root() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let real = crate::workbench::presets::Presets::load_from(&root).expect("真 presets");
        let f = Fixture::load();
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
        let book = Book::new(Some(&f.up), &real, &c, &d);

        let expected = deliverable_set(&book);
        // 反空转锚点：content 3 + manifest 1 + 资产 13 + mkp（夹具上游认 3 版）= 20。
        // mkp 条目跟着 mkp_preset 连接键走（夹具上游只认 A1×2 + P1S×1）；
        // 真上游在用户机器上时是全部 9 版
        assert_eq!(expected.len(), 20, "交付集合条数变了 —— 说清为什么");
        // **9 份 MKP 产物名单独立锚定**：命名函数逐版算出（wb_generate 将写的名单），
        // 与夹具上游认不认无关 —— 这是发布集合在真上游下的目标形状
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
        // 集合里的 mkp 条目只来自**夹具上游认的 3 版**（mkp_preset 连接键）：
        // A1×2 + P1S×1 必须在；其余 6 版夹具上游不认，留给真上游 —— 不在集合是对的
        for name in ["A1-standard.toml", "A1-fast.toml", "P1S-lite.toml"] {
            assert!(
                expected.contains(&format!("presets/mkp/{name}")),
                "夹具上游认的版本 {name} 必须进交付集合"
            );
        }
        assert_eq!(
            expected
                .iter()
                .filter(|p| p.starts_with("presets/mkp/"))
                .count(),
            3,
            "夹具上游只认 3 版 —— 多出来的 mkp 条目说明集合在空转"
        );
        assert_eq!(
            expected
                .iter()
                .filter(|p| p.starts_with("assets/bbs/"))
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
}
