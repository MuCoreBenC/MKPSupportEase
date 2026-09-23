//! `manifest.json` —— 资源清单，也是客户端的入口文件。
//!
//! # 为什么它是资源这一面的唯一权威
//!
//! 上游有四个文件看起来都在讲资源，实测只有这一个能用：
//!
//! | 文件 | 条数 | 有真 sha256 吗 | 结论 |
//! |---|---|---|---|
//! | `manifest.json → assets[]` | 72 | **有**（72/72 非空） | 权威 |
//! | `content/assets_index.json` | 18 | 没有（sha256 全空、size 全 0） | **不读**，见下 |
//! | `content/bundles.json` | 5 | —— | **不读**，与 `manifest.bundles` 逐字节相同 |
//! | `content/preset_registry.json` | 9 | 没有 | 读，但只取它独有的 `nozzle` / `layerHeight` |
//!
//! ## 为什么不读 `assets_index.json`
//!
//! 它的 18 条里 `sha256` 全是空串、`size` 全是 0、`isRegistered` 全是 `false` ——
//! 而这 18 个文件的 `relativePath` **全部**出现在 `manifest.assets` 里，也就是说
//! 它们全都已登记。这个字段不是"还没登记"，是**没人维护**。
//!
//! 读它的代价是具体的：文件清单页会给 18 个已登记文件打上「未登记」，给 72 个有真
//! 哈希的文件显示「大小未知」。参考实现的库存页满屏 `UNKNOWN` 就是这么来的 ——
//! 它只有这一个来源。我们有 manifest，所以文件清单页不该有一个 UNKNOWN。
//!
//! ## 为什么不读 `content/bundles.json`
//!
//! 实测与 `manifest.bundles` 逐字节相同，而且**它不在 `contentFiles` 里** ——
//! 客户端从头到尾不会下载它。读它只会让"套餐有几个"这件事有两处声明，
//! 而其中一处没有任何消费者。
//!
//! # 刻意不在这里读 `machines[].dimensions`
//!
//! manifest 把机型尺寸内联了一份，`content/machine_catalog.json` 里也有一份同样的。
//! 两处都建类型 = 床身尺寸有两个声明。这里只看 `dimensions` **在不在**
//! （[`ManifestMachine::has_dimensions`]），形状归 [`super::catalog`]。
//!
//! 只看在不在也不是多余的：客户端读的是 manifest 这一份。如果 `machine_catalog`
//! 有尺寸而 manifest 这份是 `null`，客户端会显示「暂无尺寸」而工作台显示有值 ——
//! 这种分歧没有任何一方会报错。`catalog` 里有一条断言盯着它。

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::workbench::paths;
use crate::workbench::store::read_json;

const MANIFEST_REL: &str = "manifest.json";
const PRESET_REGISTRY_REL: &str = "content/preset_registry.json";

/* ---------- 资源 ---------- */

/// 实测只有这三种。
///
/// 前两种是**交付物**（客户端要下载的配方与曲线），`Image` 是界面素材 ——
/// 它们 54/72 占了大头，但与配方无关，所以 [`Manifest::deliverables`] 把它们排除
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    /// BBS 的 Process 曲线（进口饮料）。9 条
    BbsProfile,
    /// 我们烤出来的 MKP TOML（面包）。9 条
    MkpPreset,
    /// 界面素材：机型图、FAQ 截图、头像、图标。54 条
    Image,
}

impl ResourceType {
    /// 是不是要交付给客户的东西
    pub fn is_deliverable(self) -> bool {
        matches!(self, Self::BbsProfile | Self::MkpPreset)
    }
}

/// 一份资源。`sha256` 与 `size` 是**真值**，所以状态派生（doc §5）可以直接用它对比 ——
/// 不用退回到比文件时间
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    /// 稳定 id，套餐用它引用（`assetRefs`）。实测 72 个全不重复
    pub id: String,
    pub resource_type: ResourceType,
    /// 分类。实测有 9 条是空串（那 9 个 MKP TOML），所以是 `Option`
    pub category: Option<String>,
    /// 54 个 image 没有机型，所以是 `Option` 而不是空串
    pub machine_id: Option<String>,
    pub file_name: String,
    pub relative_path: String,
    pub sha256: String,
    pub size: u64,
    pub updated_at: String,
    /// 喷嘴口径。**只有 BBS 有**，来自 `content/preset_registry.json`
    pub nozzle: Option<String>,
    /// 层高。同上
    pub layer_height: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawAsset {
    id: String,
    resource_type: ResourceType,
    #[serde(default)]
    category: String,
    #[serde(default)]
    machine_id: String,
    file_name: String,
    relative_path: String,
    sha256: String,
    size: u64,
    updated_at: String,
}

/* ---------- 套餐 ---------- */

/// 一个交付组合。`asset_refs` 指向 [`Asset::id`]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bundle {
    pub id: String,
    pub display: String,
    pub machine_id: String,
    #[serde(default)]
    pub asset_refs: Vec<String>,
}

/* ---------- 机型（只取 manifest 独有的那几样） ---------- */

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestVersion {
    pub id: String,
    pub name: String,
    /// `"A1:FASTV3.3"`。`machineVariants` 的版本键用的就是这个形状（doc §3.3）
    pub machine_key: String,
    pub tag: Option<String>,
    pub description: Option<String>,
    /// 这个版本对应的 MKP TOML 的 [`Asset::id`]。
    /// **A2L 这里是空串 → `None`**，意思是"这一版还没有产物"（doc §11）
    pub mkp_preset_asset_id: Option<String>,
    pub recommended_bundle: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestMachine {
    pub id: String,
    pub display: String,
    pub brand: String,
    pub icon: Option<String>,
    pub image: Option<String>,
    pub default_bundle: Option<String>,
    pub versions: Vec<ManifestVersion>,
    /// 只记在不在，形状归 [`super::catalog`]
    has_dimensions: bool,
}

impl ManifestMachine {
    pub fn has_dimensions(&self) -> bool {
        self.has_dimensions
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawVersion {
    id: String,
    #[serde(default)]
    name: String,
    machine_key: String,
    #[serde(default)]
    tag: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    mkp_preset_asset_id: String,
    #[serde(default)]
    recommended_bundle: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMachine {
    id: String,
    #[serde(default)]
    display: String,
    #[serde(default)]
    brand: String,
    #[serde(default)]
    icon: String,
    #[serde(default)]
    image: String,
    #[serde(default)]
    default_bundle: String,
    #[serde(default)]
    versions: Vec<RawVersion>,
    /// 读成 `Value` 只为判断在不在；形状不在这里建（见模块文档）
    #[serde(default)]
    dimensions: Option<serde_json::Value>,
}

/* ---------- 兼容声明 ---------- */

/// 上游对"客户端要多新才能用这份数据"的声明。
///
/// **实测 `minimumClient` 是空串、`version` 也是空串** —— 上游现在没有声明。
/// 这里把空串变成 `None` 而不是留着空串：界面要显示「未声明」，而
/// `if s.is_empty()` 这种判断散在十个地方，迟早有一个忘了写（doc §12）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Compatibility {
    /// 实测 2。清单结构本身的版本
    pub manifest_version: u32,
    /// 这份数据要求的最低客户端版本。**`None` = 上游未声明**
    pub minimum_client: Option<String>,
    /// 这份数据自己的版本。**`None` = 上游未声明**
    pub version: Option<String>,
    pub channel: String,
    pub updated: String,
    pub force_update: bool,
}

/// 发布轨道。实测 4 条，全是 `stable`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub base_version: String,
    pub channel: String,
    pub latest: String,
}

/// 客户端会下载的内容文件。实测 15 条，**有真 sha256**。
///
/// 只读身份与校验信息；`ttl` / `strategy` / `prefetch` / `priority` / `retry`
/// 是客户端的抓取策略，工作台既不产出也不消费，读进来只会多五个没人看的列
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentFile {
    pub file_name: String,
    pub relative_path: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawContentFile {
    file_name: String,
    relative_path: String,
    sha256: String,
    size: u64,
}

/* ---------- 文件本体 ---------- */

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawManifest {
    manifest_version: u32,
    #[serde(default)]
    version: String,
    #[serde(default)]
    minimum_client: String,
    #[serde(default)]
    channel: String,
    #[serde(default)]
    updated: String,
    #[serde(default)]
    force_update: bool,
    #[serde(default)]
    assets: Vec<RawAsset>,
    #[serde(default)]
    bundles: Vec<Bundle>,
    #[serde(default)]
    machines: BTreeMap<String, RawMachine>,
    #[serde(default)]
    tracks: Vec<Track>,
    #[serde(default)]
    content_files: Vec<RawContentFile>,
}

/// `content/preset_registry.json` —— BBS 曲线的元数据。
///
/// 它与 `manifest.assets` 有 8 个字段重叠，**只有 `nozzle` / `layerHeight` 是它独有的**。
/// 所以这里不给它建一套并行的资源表，只把这两样贴到对应的 [`Asset`] 上
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPresetRegistry {
    #[serde(default)]
    entries: Vec<RawPresetEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPresetEntry {
    relative_path: String,
    #[serde(default)]
    machine_ids: Vec<String>,
    #[serde(default)]
    nozzle: String,
    #[serde(default)]
    layer_height: String,
}

pub struct Manifest {
    assets: Vec<Asset>,
    bundles: Vec<Bundle>,
    machines: BTreeMap<String, ManifestMachine>,
    content_files: Vec<ContentFile>,
    tracks: Vec<Track>,
    pub compat: Compatibility,
}

/// 派生版会把 72 条资源 + 15 个内容文件整本打印，`unwrap_err()` 失败时真因会被冲走
impl std::fmt::Debug for Manifest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Manifest({} 资源 / {} 套餐 / {} 机型 / v{})",
            self.assets.len(),
            self.bundles.len(),
            self.machines.len(),
            self.compat.manifest_version
        )
    }
}

impl Manifest {
    pub fn load() -> Result<Self, AppError> {
        let root = paths::upstream_root().ok_or_else(|| {
            AppError::not_found("找不到上游预设仓库 mkpse-presets")
                .with_detail(format!("试过：{}", paths::upstream_candidates().join("；")))
        })?;
        Self::load_from(&root)
    }

    pub fn load_from(root: &Path) -> Result<Self, AppError> {
        let raw: RawManifest = read_json(&root.join(MANIFEST_REL), "资源清单")?;
        // BBS 元数据是**可选补充**：没有这个文件，资源表照样完整，只是少两列。
        // 所以缺文件不算错 —— 但解析不了算，那说明它坏了而不是没有
        let nozzles = load_nozzles(root)?;

        let mut assets: Vec<Asset> = Vec::with_capacity(raw.assets.len());
        for a in raw.assets {
            let extra = nozzles.get(&a.relative_path);
            let machine_id = non_empty(a.machine_id);

            // BBS 曲线的归属机型有两处声明（清单的单数 `machineId` 与登记表的
            // 复数 `machineIds`）。挂错的表现是"这台机器下载到了别的机器的曲线" ——
            // 没有任何一方会报错，所以在这里对一次
            if let Some(e) = extra {
                if !e.machine_ids.is_empty()
                    && !machine_id
                        .as_deref()
                        .is_some_and(|m| e.machine_ids.iter().any(|x| x == m))
                {
                    return Err(AppError::corrupted(format!(
                        "{} 的归属机型两处对不上",
                        a.relative_path
                    ))
                    .with_detail(format!(
                        "资源清单说 {}，BBS 登记表说 {} —— 客户端会下载到别的机型的曲线",
                        machine_id.as_deref().unwrap_or("（空）"),
                        e.machine_ids.join("、")
                    )));
                }
            }

            assets.push(Asset {
                id: a.id,
                resource_type: a.resource_type,
                category: non_empty(a.category),
                machine_id,
                file_name: a.file_name,
                relative_path: a.relative_path,
                sha256: a.sha256,
                size: a.size,
                updated_at: a.updated_at,
                nozzle: extra.and_then(|e| non_empty(e.nozzle.clone())),
                layer_height: extra.and_then(|e| non_empty(e.layer_height.clone())),
            });
        }
        assets.sort_by(|a, b| a.id.cmp(&b.id));

        let machines = raw
            .machines
            .into_iter()
            .map(|(k, m)| {
                let out = ManifestMachine {
                    display: non_empty(m.display).unwrap_or_else(|| m.id.clone()),
                    brand: m.brand,
                    icon: non_empty(m.icon),
                    image: non_empty(m.image),
                    default_bundle: non_empty(m.default_bundle),
                    has_dimensions: m.dimensions.is_some_and(|d| !d.is_null()),
                    versions: m
                        .versions
                        .into_iter()
                        .map(|v| ManifestVersion {
                            id: v.id,
                            name: v.name,
                            machine_key: v.machine_key,
                            tag: non_empty(v.tag),
                            description: non_empty(v.description),
                            mkp_preset_asset_id: non_empty(v.mkp_preset_asset_id),
                            recommended_bundle: non_empty(v.recommended_bundle),
                        })
                        .collect(),
                    id: m.id,
                };
                (k, out)
            })
            .collect();

        let out = Self {
            assets,
            bundles: raw.bundles,
            machines,
            content_files: raw
                .content_files
                .into_iter()
                .map(|c| ContentFile {
                    file_name: c.file_name,
                    relative_path: c.relative_path,
                    sha256: c.sha256,
                    size: c.size,
                })
                .collect(),
            tracks: raw.tracks,
            compat: Compatibility {
                manifest_version: raw.manifest_version,
                minimum_client: non_empty(raw.minimum_client),
                version: non_empty(raw.version),
                channel: raw.channel,
                updated: raw.updated,
                force_update: raw.force_update,
            },
        };
        out.check_consistency()?;
        Ok(out)
    }

    /// 四条断言。它们不成立时的后果同样是**静默的**：套餐指向一个不存在的文件，
    /// 客户端下载时才 404；而那时候错误出现在用户机器上，不在这里。
    pub fn check_consistency(&self) -> Result<(), AppError> {
        // ① 资源 id 不重复 —— 套餐按 id 引用，重复 id 会让"引用哪一个"变成运气
        let ids: BTreeSet<&str> = self.assets.iter().map(|a| a.id.as_str()).collect();
        if ids.len() != self.assets.len() {
            let mut seen = BTreeSet::new();
            let dup: Vec<&str> = self
                .assets
                .iter()
                .map(|a| a.id.as_str())
                .filter(|id| !seen.insert(*id))
                .collect();
            return Err(AppError::corrupted("资源清单里有重复的 id")
                .with_detail(format!("重复的：{}", dup.join("、"))));
        }

        // ② 套餐引用的资源必须存在
        for b in &self.bundles {
            let miss: Vec<&str> = b
                .asset_refs
                .iter()
                .map(String::as_str)
                .filter(|r| !ids.contains(*r))
                .collect();
            if !miss.is_empty() {
                return Err(
                    AppError::corrupted(format!("套餐 {} 引用了不存在的资源", b.id)).with_detail(
                        format!(
                            "悬空的 assetRef：{} —— 客户端下载时才会 404",
                            miss.join("、")
                        ),
                    ),
                );
            }
        }

        // ③ 版本指向的 MKP 产物必须存在。空的（A2L）不算悬空，那是"还没有产物"
        for m in self.machines.values() {
            for v in &m.versions {
                if let Some(aid) = &v.mkp_preset_asset_id {
                    if !ids.contains(aid.as_str()) {
                        return Err(AppError::corrupted(format!(
                            "{} 指向的 MKP 产物不在资源清单里",
                            v.machine_key
                        ))
                        .with_detail(format!("mkpPresetAssetId = {aid}")));
                    }
                }
            }
        }

        // ④ 交付物必须有真哈希。空哈希会让状态派生永远判"待生成"
        let no_hash: Vec<&str> = self
            .assets
            .iter()
            .filter(|a| a.resource_type.is_deliverable() && a.sha256.is_empty())
            .map(|a| a.id.as_str())
            .collect();
        if !no_hash.is_empty() {
            return Err(
                AppError::corrupted("有交付物没有 sha256").with_detail(format!(
                    "{} —— 没有哈希就没法判产物是不是过期的",
                    no_hash.join("、")
                )),
            );
        }

        Ok(())
    }

    pub fn assets(&self) -> &[Asset] {
        &self.assets
    }

    pub fn asset(&self, id: &str) -> Option<&Asset> {
        self.assets.iter().find(|a| a.id == id)
    }

    /// 要交付给客户的那些（BBS 曲线 + MKP TOML），界面素材不算。实测 18 条
    pub fn deliverables(&self) -> Vec<&Asset> {
        self.assets
            .iter()
            .filter(|a| a.resource_type.is_deliverable())
            .collect()
    }

    pub fn bundles(&self) -> &[Bundle] {
        &self.bundles
    }

    pub fn bundle(&self, id: &str) -> Option<&Bundle> {
        self.bundles.iter().find(|b| b.id == id)
    }

    pub fn machines(&self) -> &BTreeMap<String, ManifestMachine> {
        &self.machines
    }

    pub fn content_files(&self) -> &[ContentFile] {
        &self.content_files
    }

    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    /// 最新的发布版本号。轨道为空时是 `None` 而不是 `"0.0.0"` ——
    /// 编一个版本号出来会让发布页显示一个从没发布过的版本
    pub fn latest_release(&self) -> Option<&str> {
        self.tracks.first().map(|t| t.latest.as_str())
    }
}

/// `""` → `None`。
///
/// 上游用空串表示"没有"（A2L 的 `mkpPresetAssetId`、`defaultBundle`、`minimumClient`
/// 都是空串）。在边界上一次转成 `None`，后面就不会有人漏写 `if s.is_empty()`
fn non_empty(s: String) -> Option<String> {
    if s.trim().is_empty() {
        None
    } else {
        Some(s)
    }
}

/// `relativePath` → BBS 的补充元数据。文件缺失返回空表
fn load_nozzles(root: &Path) -> Result<BTreeMap<String, BbsExtra>, AppError> {
    let p = root.join(PRESET_REGISTRY_REL);
    if !p.is_file() {
        return Ok(BTreeMap::new());
    }
    let pr: RawPresetRegistry = read_json(&p, "BBS 曲线登记表")?;
    Ok(pr
        .entries
        .into_iter()
        .map(|e| {
            (
                e.relative_path,
                BbsExtra {
                    nozzle: e.nozzle,
                    layer_height: e.layer_height,
                    machine_ids: e.machine_ids,
                },
            )
        })
        .collect())
}

/// 从 `preset_registry.json` 贴过来的那几样
struct BbsExtra {
    nozzle: String,
    layer_height: String,
    /// **只用来对账，不进 [`Asset`]**：资源清单那边是单数 `machineId`。
    /// 实测 9/9 都只有一台机型，两边一致；但上游没有门禁保证这一点，
    /// 而一条 BBS 曲线挂错机型的表现是"这台机器下载到了别的机器的曲线"
    machine_ids: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, v: &serde_json::Value) {
        crate::fsx::atomic::atomic_write_json(&root.join(rel), v).unwrap();
    }

    fn asset(id: &str, rtype: &str, rel: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id, "resourceType": rtype, "category": "process", "machineId": "A1",
            "fileName": rel.rsplit('/').next().unwrap(), "relativePath": rel,
            "sha256": "aa", "size": 10, "updatedAt": "2026-01-01T00:00:00Z"
        })
    }

    fn good() -> serde_json::Value {
        serde_json::json!({
            "manifestVersion": 2,
            "version": "",
            "minimumClient": "",
            "channel": "stable",
            "updated": "2026-01-01T00:00:00Z",
            "forceUpdate": false,
            "assets": [
                asset("a1_mkp_standard", "mkp_preset", "presets/mkp/A1.toml"),
                asset("a1_bbs_04", "bbs_profile", "presets/bbs/Process/0.4mm/x.json"),
                asset("img_a1", "image", "assets/machines/a1.webp"),
            ],
            "bundles": [{
                "id": "A1_default", "display": "官方推荐", "machineId": "A1",
                "assetRefs": ["a1_bbs_04"]
            }],
            "machines": {
                "A1": {
                    "id": "A1", "display": "A1", "brand": "Bambu Lab",
                    "icon": "a1", "image": "a1.webp", "defaultBundle": "A1_default",
                    "dimensions": { "edgeZone": 10 },
                    "versions": [{
                        "id": "STANDARD", "name": "标准版", "machineKey": "A1:STANDARD",
                        "tag": "推荐", "description": "",
                        "mkpPresetAssetId": "a1_mkp_standard",
                        "recommendedBundle": "A1_default"
                    }]
                }
            },
            "tracks": [{ "baseVersion": "0.0.4", "channel": "stable", "latest": "0.0.4" }],
            "contentFiles": [{
                "id": "", "fileName": "param_registry.json",
                "relativePath": "content/param_registry.json",
                "sha256": "ff", "size": 100, "ttl": "never", "strategy": "no-swr",
                "prefetch": false, "priority": "normal"
            }]
        })
    }

    fn load(m: serde_json::Value) -> (tempfile::TempDir, Result<Manifest, AppError>) {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), MANIFEST_REL, &m);
        let r = Manifest::load_from(d.path());
        (d, r)
    }

    #[test]
    fn loads_and_separates_deliverables_from_images() {
        let (_d, r) = load(good());
        let m = r.unwrap();
        assert_eq!(m.assets().len(), 3);
        assert_eq!(m.deliverables().len(), 2, "界面素材不算交付物");
        assert_eq!(m.compat.manifest_version, 2);
        assert_eq!(m.latest_release(), Some("0.0.4"));
        assert_eq!(m.content_files().len(), 1);
    }

    /// 空串在边界上就变成 `None`，界面才好显示「未声明」而不是空白
    #[test]
    fn empty_strings_become_none_at_the_boundary() {
        let (_d, r) = load(good());
        let m = r.unwrap();
        assert_eq!(m.compat.minimum_client, None, "实测上游就是空串");
        assert_eq!(m.compat.version, None);
        // 空 description 也一样
        let v = &m.machines()["A1"].versions[0];
        assert_eq!(v.description, None);
        assert_eq!(v.tag.as_deref(), Some("推荐"));
    }

    /// A2L 的那一套空值：没有产物、没有套餐、没有尺寸，但**不是错误**
    #[test]
    fn a2l_shaped_machine_is_valid_with_nothing_attached() {
        let mut m = good();
        m["machines"]["A2L"] = serde_json::json!({
            "id": "A2L", "display": "A2L", "brand": "Bambu Lab",
            "icon": "a1", "image": "", "dimensions": null,
            "versions": [{
                "id": "STANDARD", "name": "标准版", "machineKey": "A2L:STANDARD",
                "tag": "", "description": "", "mkpPresetAssetId": "", "recommendedBundle": ""
            }]
        });
        let (_d, r) = load(m);
        let m = r.unwrap();
        let a2l = &m.machines()["A2L"];
        assert!(!a2l.has_dimensions(), "dimensions 是 null");
        assert_eq!(a2l.default_bundle, None);
        assert_eq!(a2l.versions[0].mkp_preset_asset_id, None, "还没有产物");
        assert_eq!(a2l.image, None);
    }

    /// 套餐引用了不存在的资源 —— 不拦住的话客户端下载时才 404
    #[test]
    fn dangling_asset_ref_is_rejected() {
        let mut m = good();
        m["bundles"][0]["assetRefs"] = serde_json::json!(["a1_bbs_04", "ghost"]);
        let (_d, r) = load(m);
        let e = r.unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::Corrupted);
        assert!(e.detail.unwrap_or_default().contains("ghost"), "没点名真因");
    }

    #[test]
    fn duplicate_asset_id_is_rejected() {
        let mut m = good();
        m["assets"].as_array_mut().unwrap().push(asset(
            "a1_bbs_04",
            "bbs_profile",
            "presets/bbs/dup.json",
        ));
        let (_d, r) = load(m);
        assert!(r
            .unwrap_err()
            .detail
            .unwrap_or_default()
            .contains("a1_bbs_04"));
    }

    /// 版本指向的产物不存在要拦；**指向空串不拦** —— 那是 A2L 的正常状态
    #[test]
    fn dangling_mkp_preset_id_is_rejected_but_empty_is_fine() {
        let mut m = good();
        m["machines"]["A1"]["versions"][0]["mkpPresetAssetId"] = serde_json::json!("ghost");
        let (_d, r) = load(m);
        let e = r.unwrap_err();
        assert!(e.message.contains("A1:STANDARD"), "{}", e.message);

        let mut m2 = good();
        m2["machines"]["A1"]["versions"][0]["mkpPresetAssetId"] = serde_json::json!("");
        let (_d2, r2) = load(m2);
        assert!(r2.is_ok(), "空 = 还没有产物，不是悬空引用");
    }

    /// 交付物的空哈希要拦；**界面素材不查** —— 它不进状态派生
    #[test]
    fn deliverable_without_hash_is_rejected() {
        let mut m = good();
        m["assets"][0]["sha256"] = serde_json::json!("");
        let (_d, r) = load(m);
        assert!(r.unwrap_err().message.contains("sha256"));

        let mut m2 = good();
        m2["assets"][2]["sha256"] = serde_json::json!(""); // image
        let (_d2, r2) = load(m2);
        assert!(r2.is_ok(), "界面素材没哈希不影响产物状态");
    }

    /// BBS 的口径/层高贴到对应资源上；其他资源保持 `None`
    #[test]
    fn nozzle_and_layer_height_are_joined_onto_bbs_assets() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), MANIFEST_REL, &good());
        write(
            d.path(),
            PRESET_REGISTRY_REL,
            &serde_json::json!({ "entries": [{
                "relativePath": "presets/bbs/Process/0.4mm/x.json",
                "machineIds": ["A1"], "nozzle": "0.4", "layerHeight": "0.20"
            }] }),
        );
        let m = Manifest::load_from(d.path()).unwrap();
        let bbs = m.asset("a1_bbs_04").unwrap();
        assert_eq!(bbs.nozzle.as_deref(), Some("0.4"));
        assert_eq!(bbs.layer_height.as_deref(), Some("0.20"));
        assert_eq!(m.asset("a1_mkp_standard").unwrap().nozzle, None);
    }

    /// 归属机型两处对不上 —— 客户端会给这台机器下载别的机器的曲线
    #[test]
    fn bbs_machine_mismatch_is_rejected() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), MANIFEST_REL, &good());
        write(
            d.path(),
            PRESET_REGISTRY_REL,
            &serde_json::json!({ "entries": [{
                "relativePath": "presets/bbs/Process/0.4mm/x.json",
                "machineIds": ["P1S"], "nozzle": "0.4", "layerHeight": "0.20"
            }] }),
        );
        let e = Manifest::load_from(d.path()).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::Corrupted);
        let detail = e.detail.unwrap_or_default();
        assert!(detail.contains("A1") && detail.contains("P1S"), "{detail}");
    }

    /// 少了 `preset_registry.json` 只是少两列，不该整个读不出来
    #[test]
    fn missing_preset_registry_only_costs_two_columns() {
        let (_d, r) = load(good());
        let m = r.unwrap();
        assert_eq!(m.asset("a1_bbs_04").unwrap().nozzle, None);
        assert_eq!(m.deliverables().len(), 2, "资源表照样完整");
    }

    #[test]
    fn missing_manifest_is_not_found() {
        let d = tempfile::tempdir().unwrap();
        let e = Manifest::load_from(d.path()).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::NotFound);
    }

    /* ---------- 真上游对齐 ---------- */

    /// 同 `registry` 那条的边界：**没上游时不执行，所以它不是门禁**。
    /// 验的是不变式与"有没有我没读的字段"，不钉条数
    #[test]
    fn real_upstream_loads_and_satisfies_the_invariants() {
        let Some(root) = paths::upstream_root() else {
            eprintln!("没定位到上游 mkpse-presets，这条对齐检查未执行（不是通过）");
            return;
        };
        let m = Manifest::load_from(&root).expect("真上游的资源清单读不出来或断言不过");

        assert!(!m.assets().is_empty(), "读出来 0 条资源，判据已空转");
        assert!(!m.deliverables().is_empty(), "一个交付物都没有");
        assert!(
            m.deliverables().iter().all(|a| !a.sha256.is_empty()),
            "交付物的哈希是状态派生的唯一依据"
        );

        let raw: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(root.join(MANIFEST_REL)).unwrap())
                .unwrap();

        let top_known: BTreeSet<&str> = [
            "manifestVersion",
            "version",
            "minimumClient",
            "channel",
            "updated",
            "forceUpdate",
            "assets",
            "bundles",
            "machines",
            "tracks",
            "contentFiles",
            "brands",
            // brands 与 machine_catalog.brands 逐字节相同，归 catalog 读
            "ttl", // 客户端抓取策略，工作台不消费
        ]
        .into_iter()
        .collect();
        let unknown: Vec<&String> = raw
            .as_object()
            .unwrap()
            .keys()
            .filter(|k| !top_known.contains(k.as_str()))
            .collect();
        assert!(
            unknown.is_empty(),
            "清单顶层有我没读的键：{unknown:?} —— 要么读进来，要么在这条清单里写明为什么不读"
        );

        let asset_known: BTreeSet<&str> = [
            "id",
            "resourceType",
            "category",
            "machineId",
            "fileName",
            "relativePath",
            "sha256",
            "size",
            "updatedAt",
        ]
        .into_iter()
        .collect();
        let mut unknown_asset: BTreeSet<String> = BTreeSet::new();
        for a in raw["assets"].as_array().unwrap() {
            for k in a.as_object().unwrap().keys() {
                if !asset_known.contains(k.as_str()) {
                    unknown_asset.insert(k.clone());
                }
            }
        }
        assert!(
            unknown_asset.is_empty(),
            "资源条目有我没读的字段：{unknown_asset:?}"
        );

        // 上游现在**没有**兼容声明。这条不是在要求它保持为空，是钉住
        // "为空时必须是 None"：一旦上游开始声明，这里会红，提醒我去接 doc §12 那条
        assert_eq!(
            m.compat.minimum_client, None,
            "上游开始声明最低客户端版本了 —— 去把 doc §12 的「未声明」分支接上"
        );
    }
}
