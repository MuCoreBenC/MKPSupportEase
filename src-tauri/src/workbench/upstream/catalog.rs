//! `content/machine_catalog.json` + `manifest.json → machines` —— 机型与版本的身份。
//!
//! # 为什么必须读两个文件
//!
//! 这不是双轨，是**两份各缺一半**。实测：
//!
//! | 只有 `machine_catalog` 有 | 只有 `manifest.machines` 有 |
//! |---|---|
//! | `forbiddenZones`（P1S/P2S/X1C 各 2 块禁区） | `machineKey`（`"A1:FASTV3.3"`） |
//! | `externalAliases`（X1C 有 5 个别名） | `mkpPresetAssetId`（产物的稳定 id） |
//! | `versions[].presetFile`（`"A1F_260628.toml"`，文件名） | `brand` |
//!
//! 少了左边，禁区图画不出来、客户端传来的 `X1 Carbon` 认不出是哪台机型；
//! 少了右边，`machineVariants` 的版本键对不上（doc §3.3），产物也只能靠**文件名**
//! 去认 —— 而文件名会改，`mkpPresetAssetId` 不会。
//!
//! # 尺寸只认 `machine_catalog` 这一份
//!
//! manifest 把尺寸内联了一份同样的。两处都建类型 = 床身尺寸有两个声明，
//! 所以这里只从 `machine_catalog` 读形状，manifest 那份只用来对**在不在** ——
//! 客户端读的是 manifest 那一份，如果两边一个有一个没有，客户端会显示
//! 「暂无尺寸」而工作台显示有值，两边都不会报错。这就是 [`Catalog::check_consistency`]
//! 第四条存在的理由。
//!
//! # A2L 是两件事，别混（doc §11）
//!
//! A2L 在这里同时命中四处缺失：`dimensions` 没有它、`forbiddenZones` 没有它、
//! `presetFile` 是空串、`mkpPresetAssetId` 是空串。但**参数不缺** ——
//! 参数落到出厂默认（灰色「出厂」，有值可看）。缺的只有资源，那一面写「暂无资源」。
//! 这两件事在类型上就是分开的：[`MachineVersion::mkp_preset`] 为 `None`
//! 不影响任何参数的解析。

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::workbench::paths;
use crate::workbench::store::read_json;

use super::manifest::Manifest;

const CATALOG_REL: &str = "content/machine_catalog.json";

/* ---------- 尺寸 ---------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BedSize {
    pub width: f64,
    pub depth: f64,
}

/// 运动范围。**`minX` 可以是负数**（A1 是 -40，A1_MINI 是 -10）——
/// 拿它当"床身左边界"会把擦拭位算到床上去
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MovementRange {
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
    pub max_z: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlueArea {
    pub glue_min_x: f64,
    pub glue_max_x: f64,
    pub glue_min_y: f64,
    pub glue_max_y: f64,
    pub wipe_x: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineFlags {
    /// 生成的 G-code 里标记机型的注释行，如 `";===== machine: A1 mini"`
    pub gcode_marker: String,
    /// P1S/P2S/X1C 为 true，A1 系列为 false
    pub has_second_fan: bool,
}

/// 校准点坐标。实测 10 个键，全机型都齐，**每台机型的值都不同** ——
/// 所以它不能提到出厂默认层去
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Calibration {
    pub l_shape_base_x: f64,
    pub l_shape_base_y: f64,
    pub x_line_x: f64,
    pub x_line_y: f64,
    pub x_line_y_end: f64,
    pub y_line_x: f64,
    pub y_line_x_end: f64,
    pub y_line_y: f64,
    pub z_start_x: f64,
    pub z_start_y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dimensions {
    pub bed_size: BedSize,
    pub movement_range: MovementRange,
    pub glue_area: GlueArea,
    pub calibration: Calibration,
    pub flags: MachineFlags,
    pub edge_zone: f64,
}

/// 禁区多边形。实测每台有两块（6 点的和 4 点的），**没有 name 字段** ——
/// 界面上要显示就只能按序号叫「禁区 1 / 禁区 2」，编一个名字出来是假的
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForbiddenZone {
    pub points: Vec<Point>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/* ---------- 品牌 ---------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Brand {
    pub id: String,
    pub name: String,
    pub logo: String,
}

/* ---------- 合起来的机型 ---------- */

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineVersion {
    /// 稳定 id，如 `STANDARD` / `FAST` / `FASTV3.3` / `LITE`。**带点，见 `store::validate_id`**
    pub id: String,
    /// 显示名，如「标准版」
    pub name: String,
    /// `"A1:FASTV3.3"`。`machineVariants` 的版本键用的就是这个形状
    pub machine_key: String,
    pub tag: Option<String>,
    pub description: Option<String>,
    /// 这一版烤出来的 MKP TOML。**`None` = 还没有产物**（A2L 就是这样）
    pub mkp_preset: Option<MkpPreset>,
    /// 推荐套餐 id。`None` = 没有推荐
    pub recommended_bundle: Option<String>,
}

/// 一版的产物。`asset_id` 与 `file_name` 都留着：
/// 前者是稳定引用（用来 join 资源清单拿哈希），后者是给人看的
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MkpPreset {
    pub asset_id: String,
    pub file_name: String,
    pub relative_path: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Machine {
    pub id: String,
    /// 显示名。**不是 `name`** —— 实测上游 6 台机型的 `name` 全是空串，
    /// 显示名在 `display` 里（`"A1 mini"`）
    pub display: String,
    pub brand: String,
    pub icon: Option<String>,
    pub image: Option<String>,
    /// 客户端传来的别名，如 X1C 的 `X1` / `X1S` / `X1E` / `X1 Carbon` / `X1CARBON`。
    /// 认不出别名的后果是"这台机器没有配方"，而用户那边看不出为什么
    pub external_aliases: Vec<String>,
    pub default_bundle: Option<String>,
    /// **`None` = 上游没登记这台机型的尺寸**（A2L）。不是 0，不是默认床身
    pub dimensions: Option<Dimensions>,
    /// 空 = 这台机型没有禁区（A1 系列与 A2L）。**空和"没登记"在这里不用分** ——
    /// 没有禁区就是可以全床走
    pub forbidden_zones: Vec<ForbiddenZone>,
    pub versions: Vec<MachineVersion>,
}

impl Machine {
    pub fn version(&self, version_id: &str) -> Option<&MachineVersion> {
        self.versions.iter().find(|v| v.id == version_id)
    }

    /// 这台机型有没有可交付的产物。A2L 是 false —— 资源那一面写「暂无资源」
    pub fn has_any_product(&self) -> bool {
        self.versions.iter().any(|v| v.mkp_preset.is_some())
    }
}

/* ---------- 原始文件 ---------- */

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCatalog {
    #[serde(default)]
    brands: Vec<Brand>,
    /// 品牌名 → 机型数组。实测只有一个品牌 `"Bambu Lab"`
    #[serde(default)]
    models: BTreeMap<String, Vec<RawModel>>,
    #[serde(default)]
    dimensions: BTreeMap<String, Dimensions>,
    #[serde(default)]
    forbidden_zones: BTreeMap<String, Vec<ForbiddenZone>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawModel {
    id: String,
    #[serde(default)]
    external_aliases: Vec<String>,
    #[serde(default)]
    versions: Vec<RawModelVersion>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawModelVersion {
    id: String,
}

pub struct Catalog {
    machines: Vec<Machine>,
    brands: Vec<Brand>,
}

impl std::fmt::Debug for Catalog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ids: Vec<&str> = self.machines.iter().map(|m| m.id.as_str()).collect();
        write!(f, "Catalog([{}])", ids.join(", "))
    }
}

impl Catalog {
    pub fn load() -> Result<Self, AppError> {
        let root = paths::upstream_root().ok_or_else(|| {
            AppError::not_found("找不到上游预设仓库 mkpse-presets")
                .with_detail(format!("试过：{}", paths::upstream_candidates().join("；")))
        })?;
        let manifest = Manifest::load_from(&root)?;
        Self::load_from(&root, &manifest)
    }

    /// **清单按引用传进来**，不在这里再读一遍 `manifest.json` ——
    /// 42KB 读两遍是小事，同一个文件有两套形状不是
    pub fn load_from(root: &Path, manifest: &Manifest) -> Result<Self, AppError> {
        let raw: RawCatalog = read_json(&root.join(CATALOG_REL), "机型清单")?;

        let mut machines = Vec::new();
        for models in raw.models.values() {
            for m in models {
                let Some(mm) = manifest.machines().get(&m.id) else {
                    return Err(AppError::corrupted(format!(
                        "机型 {} 在机型清单里有，资源清单里没有",
                        m.id
                    ))
                    .with_detail("客户端读的是资源清单，所以这台机型对用户等于不存在"));
                };

                let mut versions = Vec::new();
                for v in &m.versions {
                    let Some(mv) = mm.versions.iter().find(|x| x.id == v.id) else {
                        return Err(AppError::corrupted(format!(
                            "版本 {}/{} 在机型清单里有，资源清单里没有",
                            m.id, v.id
                        )));
                    };
                    // 悬空的 assetId 已经被 `Manifest::check_consistency` 拦过，
                    // 所以这里拿不到只可能是"没有产物"
                    let mkp_preset = mv
                        .mkp_preset_asset_id
                        .as_deref()
                        .and_then(|id| manifest.asset(id))
                        .map(|a| MkpPreset {
                            asset_id: a.id.clone(),
                            file_name: a.file_name.clone(),
                            relative_path: a.relative_path.clone(),
                            sha256: a.sha256.clone(),
                            size: a.size,
                        });
                    versions.push(MachineVersion {
                        id: mv.id.clone(),
                        name: mv.name.clone(),
                        machine_key: mv.machine_key.clone(),
                        tag: mv.tag.clone(),
                        description: mv.description.clone(),
                        mkp_preset,
                        recommended_bundle: mv.recommended_bundle.clone(),
                    });
                }

                machines.push(Machine {
                    id: m.id.clone(),
                    display: mm.display.clone(),
                    brand: mm.brand.clone(),
                    icon: mm.icon.clone(),
                    image: mm.image.clone(),
                    external_aliases: m.external_aliases.clone(),
                    default_bundle: mm.default_bundle.clone(),
                    dimensions: raw.dimensions.get(&m.id).cloned(),
                    forbidden_zones: raw.forbidden_zones.get(&m.id).cloned().unwrap_or_default(),
                    versions,
                });
            }
        }
        machines.sort_by(|a, b| a.id.cmp(&b.id));

        let out = Self {
            machines,
            brands: raw.brands,
        };
        out.check_consistency(manifest)?;
        Ok(out)
    }

    /// 五条断言。每一条对应一种**不会自己报错**的分歧
    pub fn check_consistency(&self, manifest: &Manifest) -> Result<(), AppError> {
        // ① 机型 id 不重复
        let ids: BTreeSet<&str> = self.machines.iter().map(|m| m.id.as_str()).collect();
        if ids.len() != self.machines.len() {
            return Err(AppError::corrupted("机型清单里有重复的机型 id"));
        }

        // ② 两份的机型集合要一致。上面的循环只查了"清单里有、资源里没有"这一向，
        //    反向（资源里有、清单里没有）会让那台机型在工作台里整个消失
        let extra: Vec<&str> = manifest
            .machines()
            .keys()
            .map(String::as_str)
            .filter(|id| !ids.contains(*id))
            .collect();
        if !extra.is_empty() {
            return Err(
                AppError::corrupted("资源清单里的机型在机型清单里没有").with_detail(format!(
                    "{} —— 它们在工作台里会整个消失，而客户端仍然看得到",
                    extra.join("、")
                )),
            );
        }

        // ③ 别名不能撞车。两台机型认领同一个别名时，客户端传这个别名过来，
        //    匹配到哪一台取决于遍历顺序
        let mut owner: BTreeMap<&str, &str> = BTreeMap::new();
        for m in &self.machines {
            for a in &m.external_aliases {
                if let Some(prev) = owner.insert(a.as_str(), m.id.as_str()) {
                    return Err(AppError::corrupted(format!("别名 {a} 被两台机型同时认领"))
                        .with_detail(format!(
                            "{prev} 与 {} —— 客户端传这个别名过来会匹配到运气",
                            m.id
                        )));
                }
            }
            // 别名也不能和别人的正名撞
            if let Some(dup) = m.external_aliases.iter().find(|a| ids.contains(a.as_str())) {
                return Err(AppError::corrupted(format!(
                    "{} 的别名 {dup} 与另一台机型的正名相同",
                    m.id
                )));
            }
        }

        // ④ 尺寸在两份里要么都有、要么都没有。客户端读 manifest 那一份，
        //    工作台读这一份，分歧的表现是"客户端说没有、工作台说有"
        for m in &self.machines {
            let in_manifest = manifest
                .machines()
                .get(&m.id)
                .is_some_and(|x| x.has_dimensions());
            if in_manifest != m.dimensions.is_some() {
                return Err(
                    AppError::corrupted(format!("{} 的尺寸两份对不上", m.id)).with_detail(format!(
                        "machine_catalog {}，manifest {} —— 客户端读 manifest 那一份",
                        if m.dimensions.is_some() {
                            "有"
                        } else {
                            "没有"
                        },
                        if in_manifest { "有" } else { "没有" }
                    )),
                );
            }
        }

        // ⑤ 有禁区就必须有尺寸 —— 禁区是床面坐标，没有床面画不出来
        for m in &self.machines {
            if !m.forbidden_zones.is_empty() && m.dimensions.is_none() {
                return Err(
                    AppError::corrupted(format!("{} 登记了禁区却没有床身尺寸", m.id))
                        .with_detail("禁区是床面坐标，没有床面就画不出来，也判不了越界"),
                );
            }
        }

        Ok(())
    }

    pub fn machines(&self) -> &[Machine] {
        &self.machines
    }

    pub fn machine(&self, id: &str) -> Option<&Machine> {
        self.machines.iter().find(|m| m.id == id)
    }

    pub fn brands(&self) -> &[Brand] {
        &self.brands
    }

    /// 正名或别名都能认出来。客户端传的是 `"X1 Carbon"` 这种，不是 `"X1C"`
    pub fn resolve(&self, name: &str) -> Option<&Machine> {
        self.machines
            .iter()
            .find(|m| m.id == name || m.external_aliases.iter().any(|a| a == name))
    }

    /// 所有 `机型:版本` 键。`machineVariants` 的版本键就是这个集合的子集（doc §3.3）
    pub fn machine_keys(&self) -> Vec<&str> {
        self.machines
            .iter()
            .flat_map(|m| m.versions.iter())
            .map(|v| v.machine_key.as_str())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dims() -> serde_json::Value {
        serde_json::json!({
            "bedSize": { "width": 260, "depth": 255 },
            "movementRange": { "minX": -40, "maxX": 260, "minY": 0, "maxY": 255, "maxZ": 999 },
            "glueArea": { "glueMinX": -40, "glueMaxX": 260, "glueMinY": 0, "glueMaxY": 255, "wipeX": 252 },
            "calibration": {
                "lShapeBaseX": 68.21, "lShapeBaseY": 126.373,
                "xLineX": 114.523, "xLineY": 104.83, "xLineYEnd": 114.83,
                "yLineX": 104.53, "yLineXEnd": 114.53, "yLineY": 114.83,
                "zStartX": 68.21, "zStartY": 126.373
            },
            "flags": { "gcodeMarker": ";===== machine: A1", "hasSecondFan": false },
            "edgeZone": 10
        })
    }

    /// 机型清单：A1（有尺寸有产物）+ A2L（四处皆空）
    fn catalog() -> serde_json::Value {
        serde_json::json!({
            "brands": [{ "id": "Bambu Lab", "name": "拓竹 (Bambu Lab)", "logo": "bambu-logo.png" }],
            "models": { "Bambu Lab": [
                { "id": "A1", "externalAliases": ["A1C"], "versions": [{ "id": "STANDARD" }] },
                { "id": "A2L", "externalAliases": [], "versions": [{ "id": "STANDARD" }] }
            ] },
            "dimensions": { "A1": dims() },
            "forbiddenZones": {}
        })
    }

    fn manifest_json() -> serde_json::Value {
        serde_json::json!({
            "manifestVersion": 2,
            "channel": "stable",
            "updated": "2026-01-01T00:00:00Z",
            "assets": [{
                "id": "a1_mkp_standard", "resourceType": "mkp_preset", "category": "",
                "machineId": "A1", "fileName": "A1.toml",
                "relativePath": "presets/mkp/A1.toml",
                "sha256": "abc", "size": 4299, "updatedAt": "2026-01-01T00:00:00Z"
            }],
            "bundles": [],
            "machines": {
                "A1": {
                    "id": "A1", "display": "A1", "brand": "Bambu Lab",
                    "icon": "a1", "image": "a1.webp", "defaultBundle": "A1_default",
                    "dimensions": dims(),
                    "versions": [{
                        "id": "STANDARD", "name": "标准版", "machineKey": "A1:STANDARD",
                        "tag": "推荐", "description": "官方标准配置",
                        "mkpPresetAssetId": "a1_mkp_standard", "recommendedBundle": ""
                    }]
                },
                "A2L": {
                    "id": "A2L", "display": "A2L", "brand": "Bambu Lab",
                    "icon": "a1", "image": "", "defaultBundle": "",
                    "dimensions": null,
                    "versions": [{
                        "id": "STANDARD", "name": "标准版", "machineKey": "A2L:STANDARD",
                        "tag": "", "description": "",
                        "mkpPresetAssetId": "", "recommendedBundle": ""
                    }]
                }
            },
            "tracks": [],
            "contentFiles": []
        })
    }

    fn load(
        cat: serde_json::Value,
        man: serde_json::Value,
    ) -> (tempfile::TempDir, Result<Catalog, AppError>) {
        let d = tempfile::tempdir().unwrap();
        crate::fsx::atomic::atomic_write_json(&d.path().join(CATALOG_REL), &cat).unwrap();
        crate::fsx::atomic::atomic_write_json(&d.path().join("manifest.json"), &man).unwrap();
        let m = Manifest::load_from(d.path()).expect("夹具里的清单应该是自洽的");
        let r = Catalog::load_from(d.path(), &m);
        (d, r)
    }

    #[test]
    fn joins_both_halves() {
        let (_d, r) = load(catalog(), manifest_json());
        let c = r.unwrap();
        let a1 = c.machine("A1").unwrap();

        // 左边这一份独有的
        assert_eq!(a1.external_aliases, vec!["A1C"]);
        assert_eq!(a1.dimensions.as_ref().unwrap().bed_size.width, 260.0);
        assert_eq!(a1.dimensions.as_ref().unwrap().movement_range.min_x, -40.0);
        // 右边这一份独有的
        let v = a1.version("STANDARD").unwrap();
        assert_eq!(v.machine_key, "A1:STANDARD");
        assert_eq!(v.mkp_preset.as_ref().unwrap().asset_id, "a1_mkp_standard");
        assert_eq!(v.mkp_preset.as_ref().unwrap().sha256, "abc");
        assert_eq!(a1.brand, "Bambu Lab");
        assert_eq!(c.brands().len(), 1);
    }

    /// **A2L 的两件事**：资源全空，但这不是错误，也不影响它是一台正常机型
    #[test]
    fn a2l_has_no_resources_but_is_a_valid_machine() {
        let (_d, r) = load(catalog(), manifest_json());
        let c = r.unwrap();
        let a2l = c.machine("A2L").unwrap();

        assert!(a2l.dimensions.is_none(), "上游没登记它的尺寸");
        assert!(a2l.forbidden_zones.is_empty());
        assert_eq!(a2l.default_bundle, None);
        assert!(
            !a2l.has_any_product(),
            "还没有产物 —— 资源那面写「暂无资源」"
        );
        assert_eq!(a2l.version("STANDARD").unwrap().machine_key, "A2L:STANDARD");
        assert!(c.machine("A1").unwrap().has_any_product(), "A1 有产物");
    }

    /// 别名要能认出来 —— 认不出的后果是用户那边"这台机器没有配方"
    #[test]
    fn resolves_by_alias_and_by_real_id() {
        let (_d, r) = load(catalog(), manifest_json());
        let c = r.unwrap();
        assert_eq!(c.resolve("A1").unwrap().id, "A1");
        assert_eq!(c.resolve("A1C").unwrap().id, "A1");
        assert!(c.resolve("KOBRA").is_none());
    }

    /// 两台机型认领同一个别名 —— 匹配到哪一台会变成遍历顺序的运气
    #[test]
    fn duplicate_alias_is_rejected() {
        let mut cat = catalog();
        cat["models"]["Bambu Lab"][1]["externalAliases"] = serde_json::json!(["A1C"]);
        let (_d, r) = load(cat, manifest_json());
        let e = r.unwrap_err();
        assert!(e.message.contains("A1C"), "{}", e.message);
    }

    /// 别名撞上别人的正名，同样是运气
    #[test]
    fn alias_colliding_with_a_real_id_is_rejected() {
        let mut cat = catalog();
        cat["models"]["Bambu Lab"][1]["externalAliases"] = serde_json::json!(["A1"]);
        let (_d, r) = load(cat, manifest_json());
        assert!(r.unwrap_err().message.contains("正名"));
    }

    /// 资源清单里有、机型清单里没有 —— 那台机型在工作台里会整个消失
    #[test]
    fn machine_only_in_manifest_is_rejected() {
        let mut cat = catalog();
        cat["models"]["Bambu Lab"] = serde_json::json!([
            { "id": "A1", "externalAliases": ["A1C"], "versions": [{ "id": "STANDARD" }] }
        ]);
        let (_d, r) = load(cat, manifest_json());
        let e = r.unwrap_err();
        assert!(
            e.detail.unwrap_or_default().contains("A2L"),
            "{}",
            e.message
        );
    }

    /// 机型清单里有、资源清单里没有 —— 那台机型对客户等于不存在
    #[test]
    fn machine_only_in_catalog_is_rejected() {
        let mut cat = catalog();
        cat["models"]["Bambu Lab"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "id": "P1S", "externalAliases": [], "versions": [{ "id": "LITE" }]
            }));
        let (_d, r) = load(cat, manifest_json());
        assert!(r.unwrap_err().message.contains("P1S"));
    }

    /// 版本只在一边有，同样要拦
    #[test]
    fn version_only_in_catalog_is_rejected() {
        let mut cat = catalog();
        cat["models"]["Bambu Lab"][0]["versions"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({ "id": "FAST" }));
        let (_d, r) = load(cat, manifest_json());
        assert!(r.unwrap_err().message.contains("FAST"));
    }

    /// 尺寸两份对不上 —— 客户端说没有、工作台说有，两边都不报错
    #[test]
    fn dimensions_present_in_only_one_file_is_rejected() {
        let mut man = manifest_json();
        man["machines"]["A1"]["dimensions"] = serde_json::Value::Null;
        let (_d, r) = load(catalog(), man);
        let e = r.unwrap_err();
        assert!(e.message.contains("尺寸"), "{}", e.message);
        assert!(e.detail.unwrap_or_default().contains("客户端读 manifest"));
    }

    /// 有禁区没床面 —— 禁区是床面坐标
    #[test]
    fn forbidden_zone_without_dimensions_is_rejected() {
        let mut cat = catalog();
        cat["forbiddenZones"]["A2L"] = serde_json::json!([
            { "points": [{ "x": 0, "y": 0 }, { "x": 10, "y": 0 }, { "x": 10, "y": 10 }] }
        ]);
        let (_d, r) = load(cat, manifest_json());
        assert!(r.unwrap_err().message.contains("禁区"));
    }

    #[test]
    fn missing_catalog_is_not_found() {
        let d = tempfile::tempdir().unwrap();
        crate::fsx::atomic::atomic_write_json(&d.path().join("manifest.json"), &manifest_json())
            .unwrap();
        let m = Manifest::load_from(d.path()).unwrap();
        let e = Catalog::load_from(d.path(), &m).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::NotFound);
    }

    /* ---------- 真上游对齐 ---------- */

    /// 同前两条的边界：**没上游时不执行**。验不变式与漏读字段，不钉条数
    #[test]
    fn real_upstream_loads_and_satisfies_the_invariants() {
        let Some(root) = paths::upstream_root() else {
            eprintln!("没定位到上游 mkpse-presets，这条对齐检查未执行（不是通过）");
            return;
        };
        let manifest = Manifest::load_from(&root).unwrap();
        let c = Catalog::load_from(&root, &manifest).expect("真上游的机型清单读不出来或断言不过");

        assert!(!c.machines().is_empty(), "读出来 0 台机型，判据已空转");
        assert!(
            c.machines().iter().any(|m| m.has_any_product()),
            "一台有产物的机型都没有"
        );
        // 每台机型至少一个版本 —— 没有版本的机型在配方本里点不开
        for m in c.machines() {
            assert!(!m.versions.is_empty(), "{} 一个版本都没有", m.id);
        }
        // machineKey 形如 `机型:版本`，且前半必须是这台机型的 id
        for m in c.machines() {
            for v in &m.versions {
                assert_eq!(
                    v.machine_key,
                    format!("{}:{}", m.id, v.id),
                    "machineVariants 的版本键要靠这个形状去对"
                );
            }
        }

        let raw: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(root.join(CATALOG_REL)).unwrap())
                .unwrap();
        let top: BTreeSet<&str> = ["brands", "models", "dimensions", "forbiddenZones"]
            .into_iter()
            .collect();
        let unknown: Vec<&String> = raw
            .as_object()
            .unwrap()
            .keys()
            .filter(|k| !top.contains(k.as_str()))
            .collect();
        assert!(unknown.is_empty(), "机型清单顶层有我没读的键：{unknown:?}");

        // models[] 与 versions[] 的字段：重叠的那些刻意从 manifest 取（见模块文档），
        // 所以这里把"知道但不从这里读"也列出来，而不是假装它们不存在
        let model_known: BTreeSet<&str> = [
            "id",
            "externalAliases",
            "versions",
            // 下面这些从 manifest.machines 取：那边才有 machineKey 与 mkpPresetAssetId，
            // 同一台机型的显示名不该有两个来源
            "name",
            "display",
            "defaultBundle",
            "image",
            "icon",
        ]
        .into_iter()
        .collect();
        let version_known: BTreeSet<&str> = [
            "id",
            // 同上；presetFile 是文件名，manifest 那边给的是稳定 asset id
            "name",
            "presetFile",
            "recommendedBundle",
            "tag",
            "description",
        ]
        .into_iter()
        .collect();
        let mut unknown_model: BTreeSet<String> = BTreeSet::new();
        for models in raw["models"].as_object().unwrap().values() {
            for m in models.as_array().unwrap() {
                for k in m.as_object().unwrap().keys() {
                    if !model_known.contains(k.as_str()) {
                        unknown_model.insert(format!("models[].{k}"));
                    }
                }
                for v in m["versions"].as_array().unwrap() {
                    for k in v.as_object().unwrap().keys() {
                        if !version_known.contains(k.as_str()) {
                            unknown_model.insert(format!("versions[].{k}"));
                        }
                    }
                }
            }
        }
        assert!(
            unknown_model.is_empty(),
            "机型/版本有我没读的字段：{unknown_model:?} —— 要么读进来，要么写明为什么不读"
        );

        // 禁区没有 name 字段，所以界面只能按序号叫。这条钉住"别编一个名字出来"
        for zones in raw["forbiddenZones"].as_object().unwrap().values() {
            for z in zones.as_array().unwrap() {
                let keys: Vec<&String> = z.as_object().unwrap().keys().collect();
                assert_eq!(keys, vec!["points"], "禁区多了字段：{keys:?}");
            }
        }
    }
}
