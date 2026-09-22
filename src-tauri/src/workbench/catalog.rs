//! 菜单 / 资源目录与 Bundle 套餐。
//!
//! **两份文件，不合并**（doc §11）：
//! - `bundles.json` —— Bundle 定义"套餐里装什么"，0..N 个 TOML + 0..N 个 BBS
//! - `catalog.json` —— 菜单决定"客户端能看到、能下载什么"
//!
//! 分开的理由不是洁癖：可见性变化（上下架、改显示名）是高频操作，而套餐内容变化
//! 是低频的产品决策。塞进一个 JSON 会让每次上下架都 diff 到套餐定义。
//!
//! 身份用 [`PresetEntry::preset_id`]，**不用文件路径**（doc §5）：改显示名、换文件名
//! 都不该让客户端把它当成另一个预设。

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::workbench::store::{validate_id, Store};

pub const CATALOG_FILE: &str = "catalog.json";
pub const BUNDLES_FILE: &str = "bundles.json";

/// 客户端读的目录格式版本。结构一变就要递增 —— 老客户端据此整份拒绝，
/// 而不是拿新结构按旧规则解析
pub const CATALOG_SCHEMA_VERSION: u32 = 1;

/// 一个上架的预设。`machine` / `version` 是它在后厨的来源，
/// `resource` 是交付文件的相对路径
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetEntry {
    /// 稳定身份。改它等于"删旧建新"，客户端会当成另一个预设
    pub preset_id: String,
    pub machine: String,
    pub version: String,
    /// 客户端显示名。**随时可改，不影响身份，也不需要升级客户端**
    pub display_name: String,
    /// 相对发布目录的路径。可调，但引用要跟着改
    pub resource: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    /// 这份产物要求的最低客户端版本。由生成时按能力定义算出来，不手填
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_client_version: Option<String>,
    /// 允许单独下载（不必通过套餐）
    #[serde(default)]
    pub standalone: bool,
}

/// BBS 在菜单里的三态之一。
///
/// 注意**没有"仅归档"这个值** —— 仅归档的意思就是"不在菜单里"，
/// 给它一个枚举值反而会让"在菜单里但不交付"变成可表达的状态，那是自相矛盾的。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BbsOffering {
    /// 已分配：随套餐交付
    Assigned,
    /// 可选：用户可以额外选择下载
    Optional,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BbsEntry {
    pub bbs_id: String,
    pub display_name: String,
    pub resource: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_client_version: Option<String>,
    pub offering: BbsOffering,
}

/// 套餐。**0..N + 0..N** —— 纯预设、纯 BBS、混合、一预设配两 BBS 都合法
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bundle {
    pub bundle_id: String,
    pub display_name: String,
    #[serde(default)]
    pub presets: Vec<String>,
    #[serde(default)]
    pub bbs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_client_version: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bundles {
    pub bundles: Vec<Bundle>,
}

/// 菜单。客户端的唯一入口
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub catalog_schema_version: u32,
    #[serde(default)]
    pub presets: Vec<PresetEntry>,
    #[serde(default)]
    pub bbs: Vec<BbsEntry>,
    /// 发布时才填。工作台里编辑的那份留空 —— 它不是发布产物
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated_at: Option<String>,
}

impl Default for Catalog {
    fn default() -> Self {
        Self {
            catalog_schema_version: CATALOG_SCHEMA_VERSION,
            presets: Vec::new(),
            bbs: Vec::new(),
            generated_at: None,
        }
    }
}

pub fn load_catalog(store: &Store) -> Result<Catalog, AppError> {
    Ok(store.read_doc(CATALOG_FILE, "菜单")?.unwrap_or_default())
}

pub fn save_catalog(store: &Store, c: &Catalog) -> Result<(), AppError> {
    for p in &c.presets {
        validate_id(&p.preset_id, "presetId")?;
    }
    store.write_doc(CATALOG_FILE, c)
}

pub fn load_bundles(store: &Store) -> Result<Bundles, AppError> {
    Ok(store.read_doc(BUNDLES_FILE, "套餐")?.unwrap_or_default())
}

pub fn save_bundles(store: &Store, b: &Bundles) -> Result<(), AppError> {
    for x in &b.bundles {
        validate_id(&x.bundle_id, "bundleId")?;
    }
    store.write_doc(BUNDLES_FILE, b)
}

/// 版本进回收站时，把它的预设从菜单与套餐里摘掉。
///
/// doc §9.7：**进回收站的版本，它的 TOML 立即从菜单里移除**，不能还算有效交付物。
/// 如果只是"出货检查时报一条"，那在报之前它一直是个会被发布出去的条目。
///
/// 返回被摘掉的 presetId，供界面据实说明改了什么
pub fn detach_version(store: &Store, machine: &str, version: &str) -> Result<Vec<String>, AppError> {
    let mut catalog = load_catalog(store)?;
    let removed: Vec<String> = catalog
        .presets
        .iter()
        .filter(|p| p.machine == machine && p.version == version)
        .map(|p| p.preset_id.clone())
        .collect();

    if removed.is_empty() {
        return Ok(removed);
    }

    catalog
        .presets
        .retain(|p| !(p.machine == machine && p.version == version));
    save_catalog(store, &catalog)?;

    // 套餐里也要摘 —— 留着就成了"引用不存在的预设"，出货检查会阻断，但那太晚了
    let mut bundles = load_bundles(store)?;
    let mut touched = false;
    for b in &mut bundles.bundles {
        let before = b.presets.len();
        b.presets.retain(|id| !removed.contains(id));
        touched |= b.presets.len() != before;
    }
    if touched {
        save_bundles(store, &bundles)?;
    }

    Ok(removed)
}

/// 版本改了 id 或换了机型时，把菜单里指向它的条目跟着改。
///
/// **presetId 不动** —— 那是客户端认的身份，改名换机型都不该让顾客以为这是个新预设
/// （doc §5）。改的只是"它在后厨的来源"这两个字段。
pub fn relink_version(
    store: &Store,
    from_machine: &str,
    from_version: &str,
    to_machine: &str,
    to_version: &str,
) -> Result<usize, AppError> {
    let mut catalog = load_catalog(store)?;
    let mut n = 0;
    for p in &mut catalog.presets {
        if p.machine == from_machine && p.version == from_version {
            p.machine = to_machine.to_owned();
            p.version = to_version.to_owned();
            n += 1;
        }
    }
    if n > 0 {
        save_catalog(store, &catalog)?;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, Store) {
        let d = tempfile::tempdir().unwrap();
        let s = Store::at(d.path());
        s.bootstrap().unwrap();
        (d, s)
    }

    #[test]
    fn missing_files_read_as_empty_not_error() {
        let (_d, s) = store();
        assert_eq!(load_catalog(&s).unwrap().presets.len(), 0);
        assert_eq!(load_bundles(&s).unwrap().bundles.len(), 0);
    }

    #[test]
    fn catalog_roundtrips() {
        let (_d, s) = store();
        let c = Catalog {
            presets: vec![PresetEntry {
                preset_id: "a1-std".into(),
                machine: "A1".into(),
                version: "standard".into(),
                display_name: "A1 标准版".into(),
                resource: "presets/a1-std.toml".into(),
                sha256: None,
                size: None,
                min_client_version: None,
                standalone: true,
            }],
            ..Default::default()
        };
        save_catalog(&s, &c).unwrap();
        let back = load_catalog(&s).unwrap();
        assert_eq!(back.presets[0].preset_id, "a1-std");
        assert_eq!(back.catalog_schema_version, CATALOG_SCHEMA_VERSION);
    }

    /// 删版本 → 菜单与套餐里的引用**立即**消失，不是"出货检查时报一条"
    #[test]
    fn detach_removes_from_catalog_and_bundles() {
        let (_d, s) = store();
        save_catalog(
            &s,
            &Catalog {
                presets: vec![
                    PresetEntry {
                        preset_id: "a1-std".into(),
                        machine: "A1".into(),
                        version: "standard".into(),
                        display_name: "A1 标准版".into(),
                        resource: "presets/a1-std.toml".into(),
                        sha256: None,
                        size: None,
                        min_client_version: None,
                        standalone: true,
                    },
                    PresetEntry {
                        preset_id: "a1-fast".into(),
                        machine: "A1".into(),
                        version: "quickswap".into(),
                        display_name: "A1 快拆版".into(),
                        resource: "presets/a1-fast.toml".into(),
                        sha256: None,
                        size: None,
                        min_client_version: None,
                        standalone: true,
                    },
                ],
                ..Default::default()
            },
        )
        .unwrap();
        save_bundles(
            &s,
            &Bundles {
                bundles: vec![Bundle {
                    bundle_id: "combo".into(),
                    display_name: "组合".into(),
                    presets: vec!["a1-std".into(), "a1-fast".into()],
                    bbs: vec![],
                    min_client_version: None,
                }],
            },
        )
        .unwrap();

        let removed = detach_version(&s, "A1", "standard").unwrap();
        assert_eq!(removed, vec!["a1-std"]);

        let c = load_catalog(&s).unwrap();
        assert_eq!(c.presets.len(), 1);
        assert_eq!(c.presets[0].preset_id, "a1-fast");

        let b = load_bundles(&s).unwrap();
        assert_eq!(b.bundles[0].presets, vec!["a1-fast"]);
    }

    /// 没上架的版本被删：不改任何文件，也不报错
    #[test]
    fn detach_of_unlisted_version_is_a_noop() {
        let (_d, s) = store();
        assert!(detach_version(&s, "A1", "standard").unwrap().is_empty());
    }
}
