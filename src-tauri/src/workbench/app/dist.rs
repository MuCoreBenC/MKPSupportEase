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

use std::path::Path;

use serde::Serialize;

use crate::error::AppError;
use crate::workbench::domain::derive::Book;
use crate::workbench::presets::{Asset, AssetKind};

/// 交付目录里资产子树的根名。**与资产根 public/assets/ 的形状一致**（见模块头）
pub const ASSETS_DIR: &str = "assets";
/// 目录类 JSON 的子目录
pub const CONTENT_DIR: &str = "content";

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
        let book = Book::new(&f.up, &f.presets, &c, &d);
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
        let book = Book::new(&f.up, &f.presets, &c, &d);

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
        let book = Book::new(&f.up, &f.presets, &c, &d);

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
        let book = Book::new(&f.up, &real, &c, &d);

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
}
