//! 出货检查。
//!
//! 分两档，界限很明确：
//! - **阻断** = 发布出去一定会出问题（引用指向不存在的东西、兼容性无法判定）
//! - **非阻断** = 值得知道，但不一定是错（有未生成的修改、有 BBS 没人配）
//!
//! 一条硬规矩（doc §12）：**"无法校验"归为阻断，不归为通过。**
//! 能力定义没放好的那次发布，如果什么都不拦，兼容性就是靠运气。
//!
//! 一处对 doc §12 举例的**收窄**，如实登记：doc 里把"未配置"一律写成阻断。
//! 这里改成**只有上架了的未配置版本才阻断**，没上架的只给一条提示 ——
//! 仓库里留一个刚建好还没写的版本是完全正常的工作状态，让它拦住整次发布
//! 会逼人为了发别的东西先把它删掉。

use std::path::Path;

use serde::Serialize;

use crate::error::AppError;
use crate::fsx::paths::resolve_in;
use crate::workbench::bbs::{self, BbsState};
use crate::workbench::capability;
use crate::workbench::catalog;
use crate::workbench::state::{self, GenState};
use crate::workbench::store::Store;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Blocking,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Category {
    /// 配方齐不齐（能不能烤）
    Recipe,
    /// 引用对不对得上
    Reference,
    /// 客户端吃不吃得下
    Compatibility,
    /// 资源盘点（饮料在不在库）
    Inventory,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub severity: Severity,
    pub category: Category,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreflightReport {
    pub findings: Vec<Finding>,
    pub blocking: usize,
    pub warnings: usize,
    /// 阻断项清零才为 true。界面据此决定发布按钮能不能点
    pub can_publish: bool,
}

fn blocking(category: Category, message: impl Into<String>) -> Finding {
    Finding {
        severity: Severity::Blocking,
        category,
        message: message.into(),
        detail: None,
    }
}

fn warning(category: Category, message: impl Into<String>) -> Finding {
    Finding {
        severity: Severity::Warning,
        category,
        message: message.into(),
        detail: None,
    }
}

pub fn run(store: &Store, dist_root: &Path) -> Result<PreflightReport, AppError> {
    let mut f: Vec<Finding> = Vec::new();

    let catalog = catalog::load_catalog(store)?;
    let bundles = catalog::load_bundles(store)?;
    let statuses = state::status_all(store, dist_root)?;
    let listed = |mid: &str, vid: &str| {
        catalog
            .presets
            .iter()
            .any(|p| p.machine == mid && p.version == vid)
    };

    /* ---------- 配方与产物 ---------- */
    for s in &statuses {
        let on_menu = listed(&s.machine_id, &s.version_id);
        let who = format!("{} / {}", s.machine_name, s.version_name);

        match s.state {
            GenState::Unconfigured if on_menu => f.push(blocking(
                Category::Recipe,
                format!("{who}：已上架，但还没写配方"),
            )),
            GenState::Unconfigured => f.push(warning(
                Category::Recipe,
                format!("{who}：还没写配方（没上架，不影响本次发布）"),
            )),
            GenState::Stale if on_menu => f.push(blocking(
                Category::Recipe,
                format!("{who}：已上架，但产物不对应当前配方 —— 先生成"),
            )),
            GenState::Stale => f.push(warning(
                Category::Recipe,
                format!("{who}：有未生成的修改（没上架）"),
            )),
            GenState::Generated => {}
        }

        if on_menu && s.state != GenState::Unconfigured {
            if !s.output_present {
                f.push(blocking(
                    Category::Recipe,
                    format!("{who}：产物文件不在发布目录里"),
                ));
            } else if !s.output_matches {
                f.push(blocking(
                    Category::Recipe,
                    format!("{who}：产物字节与生成时记录的不一致 —— 有人手改过它？"),
                ));
            }
        }

        if s.orphan_count > 0 {
            f.push(warning(
                Category::Recipe,
                format!(
                    "{who}：有 {} 个覆盖值在当前机型上不适用（保留着，但不会进 TOML）",
                    s.orphan_count
                ),
            ));
        }

        if let Some(fail) = &s.last_failure {
            f.push(warning(
                Category::Recipe,
                format!("{who}：上次生成失败（{}）—— 旧产物没被动过", fail.reason),
            ));
        }
    }

    /* ---------- 引用完整性 ---------- */
    let mut seen_ids = std::collections::BTreeSet::new();
    for p in &catalog.presets {
        if !seen_ids.insert(p.preset_id.clone()) {
            f.push(blocking(
                Category::Reference,
                format!("presetId 重复：{}", p.preset_id),
            ));
        }
        if store.version(&p.machine, &p.version).is_err() {
            f.push(blocking(
                Category::Reference,
                format!(
                    "菜单里的 {} 指向 {}/{}，但那个版本不存在（被删了或改名了？）",
                    p.preset_id, p.machine, p.version
                ),
            ));
        }
        match resolve_in(dist_root, &p.resource) {
            Ok(path) if path.exists() => {}
            _ => f.push(blocking(
                Category::Reference,
                format!("菜单里的 {} 指向 {}，文件不存在", p.preset_id, p.resource),
            )),
        }
    }

    let bbs_files = bbs::list(store)?;
    for b in &catalog.bbs {
        if !bbs_files.iter().any(|x| x.id == b.bbs_id) {
            f.push(blocking(
                Category::Reference,
                format!("菜单里的 BBS {} 在仓库中不存在", b.bbs_id),
            ));
        }
    }

    for bd in &bundles.bundles {
        for id in &bd.presets {
            if !catalog.presets.iter().any(|p| &p.preset_id == id) {
                f.push(blocking(
                    Category::Reference,
                    format!("套餐「{}」引用了没上架的预设 {id}", bd.display_name),
                ));
            }
        }
        for id in &bd.bbs {
            if !bbs_files.iter().any(|x| &x.id == id) {
                f.push(blocking(
                    Category::Reference,
                    format!("套餐「{}」引用了仓库里没有的 BBS {id}", bd.display_name),
                ));
            }
        }
    }

    /* ---------- 兼容性 ---------- */
    match capability::load_supported(store) {
        Err(e) => f.push(Finding {
            severity: Severity::Blocking,
            category: Category::Compatibility,
            message: "无法校验兼容性".into(),
            detail: Some(e.message.clone()),
        }),
        Ok(caps) => {
            if catalog.catalog_schema_version > 0
                && !caps
                    .iter()
                    .any(|c| c.catalog_schema_version >= catalog.catalog_schema_version)
            {
                f.push(blocking(
                    Category::Compatibility,
                    format!(
                        "菜单格式版本是 {}，支持期内没有客户端能解析它",
                        catalog.catalog_schema_version
                    ),
                ));
            }
            for p in &catalog.presets {
                let s = statuses
                    .iter()
                    .find(|s| s.machine_id == p.machine && s.version_id == p.version);
                if s.and_then(|s| s.min_client_version.clone()).is_none() {
                    f.push(blocking(
                        Category::Compatibility,
                        format!(
                            "{} 没有最低客户端版本 —— 它由生成时算出，说明这份产物还没生成过",
                            p.preset_id
                        ),
                    ));
                }
            }
        }
    }

    /* ---------- 资源盘点 ---------- */
    let unassigned = bbs_files
        .iter()
        .filter(|b| b.state == BbsState::ArchivedOnly)
        .count();
    if unassigned > 0 {
        f.push(warning(
            Category::Inventory,
            format!("有 {unassigned} 个 BBS 文件没上架（仅归档）—— 不一定是错，备用件就是这样"),
        ));
    }
    for b in bbs_files.iter().filter(|b| !b.parses) {
        f.push(blocking(
            Category::Inventory,
            format!("BBS {} 不是合法 JSON", b.id),
        ));
    }

    let blocking_n = f.iter().filter(|x| x.severity == Severity::Blocking).count();
    Ok(PreflightReport {
        warnings: f.len() - blocking_n,
        blocking: blocking_n,
        can_publish: blocking_n == 0,
        findings: f,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::capability::{CapField, ClientCapability, Support};
    use crate::workbench::catalog::{Bundle, Bundles, Catalog, PresetEntry};
    use crate::workbench::generate;
    use crate::workbench::model::{builtin_registry, BbsBinding, Machine, Version};
    use crate::workbench::store::empty_params;

    fn caps() -> Vec<ClientCapability> {
        vec![ClientCapability {
            client_version: "0.1.0".into(),
            catalog_schema_version: 1,
            machines: vec!["A1".into()],
            fields: builtin_registry()
                .fields
                .iter()
                .map(|f| CapField {
                    key: f.key.clone(),
                    value_type: f.value_type,
                    min: f.min,
                    max: f.max,
                    step: f.step,
                })
                .collect(),
        }]
    }

    fn setup() -> (tempfile::TempDir, tempfile::TempDir, Store) {
        let dist = tempfile::tempdir().unwrap();
        let d = tempfile::tempdir().unwrap();
        let s = Store::at(d.path());
        s.bootstrap().unwrap();

        let mut base = empty_params();
        base.insert("toolhead.z_offset".into(), serde_json::json!(0.10));
        s.save_machine(&Machine {
            id: "A1".into(),
            display_name: "A1".into(),
            base,
            default_bbs: vec![],
        })
        .unwrap();
        s.save_version(&Version {
            id: "std".into(),
            display_name: "标准版".into(),
            machine_id: "A1".into(),
            overrides: empty_params(),
            bbs: BbsBinding::Inherit,
        })
        .unwrap();
        s.write_doc(
            "capability/support.json",
            &Support {
                supported: vec!["0.1.0".into()],
            },
        )
        .unwrap();
        s.write_doc("capability/client-0.1.0.json", &caps()[0]).unwrap();

        (dist, d, s)
    }

    fn list_it(s: &Store) {
        catalog::save_catalog(
            s,
            &Catalog {
                presets: vec![PresetEntry {
                    preset_id: "a1-std".into(),
                    machine: "A1".into(),
                    version: "std".into(),
                    display_name: "A1 标准版".into(),
                    resource: "presets/a1-std.toml".into(),
                    sha256: None,
                    size: None,
                    min_client_version: None,
                    standalone: true,
                }],
                ..Default::default()
            },
        )
        .unwrap();
    }

    fn has_blocking(r: &PreflightReport, needle: &str) -> bool {
        r.findings
            .iter()
            .any(|f| f.severity == Severity::Blocking && f.message.contains(needle))
    }

    /// 生成过、上架了、能力定义齐 → 可以发布
    #[test]
    fn clean_state_can_publish() {
        let (dist, _d, s) = setup();
        list_it(&s);
        generate::generate_one(&s, dist.path(), "A1", "std", &caps()).unwrap();

        let r = run(&s, dist.path()).unwrap();
        assert!(r.can_publish, "{:?}", r.findings);
        assert_eq!(r.blocking, 0);
    }

    /// 上架了但没生成 → 阻断
    #[test]
    fn listed_but_never_generated_blocks() {
        let (dist, _d, s) = setup();
        list_it(&s);
        let r = run(&s, dist.path()).unwrap();
        assert!(has_blocking(&r, "产物不对应当前配方"), "{:?}", r.findings);
        assert!(!r.can_publish);
    }

    /// 没上架的未配置版本只给提示，**不拦住整次发布**（对 doc 举例的收窄）
    #[test]
    fn unlisted_unconfigured_version_only_warns() {
        let (dist, _d, s) = setup();
        list_it(&s);
        generate::generate_one(&s, dist.path(), "A1", "std", &caps()).unwrap();

        s.save_machine(&Machine {
            id: "A2L".into(),
            display_name: "A2L".into(),
            base: empty_params(),
            default_bbs: vec![],
        })
        .unwrap();
        s.save_version(&Version {
            id: "std".into(),
            display_name: "标准版".into(),
            machine_id: "A2L".into(),
            overrides: empty_params(),
            bbs: BbsBinding::Inherit,
        })
        .unwrap();

        let r = run(&s, dist.path()).unwrap();
        assert!(r.can_publish, "{:?}", r.findings);
        assert!(r
            .findings
            .iter()
            .any(|f| f.severity == Severity::Warning && f.message.contains("还没写配方")));
    }

    /// 能力定义缺失 = **无法校验**，归阻断而不是通过
    #[test]
    fn missing_capability_blocks_instead_of_passing() {
        let (dist, _d, s) = setup();
        std::fs::remove_file(s.root.join("capability/support.json")).unwrap();
        let r = run(&s, dist.path()).unwrap();
        assert!(has_blocking(&r, "无法校验兼容性"), "{:?}", r.findings);
    }

    /// 菜单指向一个已经不存在的版本（手改过菜单 / 版本被改名）→ 阻断
    #[test]
    fn dangling_menu_reference_blocks() {
        let (dist, _d, s) = setup();
        catalog::save_catalog(
            &s,
            &Catalog {
                presets: vec![PresetEntry {
                    preset_id: "ghost".into(),
                    machine: "A1".into(),
                    version: "nope".into(),
                    display_name: "不存在".into(),
                    resource: "presets/ghost.toml".into(),
                    sha256: None,
                    size: None,
                    min_client_version: None,
                    standalone: true,
                }],
                ..Default::default()
            },
        )
        .unwrap();
        let r = run(&s, dist.path()).unwrap();
        assert!(has_blocking(&r, "那个版本不存在"), "{:?}", r.findings);
    }

    #[test]
    fn duplicate_preset_id_blocks() {
        let (dist, _d, s) = setup();
        let e = PresetEntry {
            preset_id: "a1-std".into(),
            machine: "A1".into(),
            version: "std".into(),
            display_name: "A1 标准版".into(),
            resource: "presets/a1-std.toml".into(),
            sha256: None,
            size: None,
            min_client_version: None,
            standalone: true,
        };
        catalog::save_catalog(
            &s,
            &Catalog {
                presets: vec![e.clone(), e],
                ..Default::default()
            },
        )
        .unwrap();
        let r = run(&s, dist.path()).unwrap();
        assert!(has_blocking(&r, "presetId 重复"), "{:?}", r.findings);
    }

    /// 套餐引用了仓库里没有的 BBS → 阻断
    #[test]
    fn bundle_referencing_missing_bbs_blocks() {
        let (dist, _d, s) = setup();
        list_it(&s);
        generate::generate_one(&s, dist.path(), "A1", "std", &caps()).unwrap();
        catalog::save_bundles(
            &s,
            &Bundles {
                bundles: vec![Bundle {
                    bundle_id: "combo".into(),
                    display_name: "组合".into(),
                    presets: vec!["a1-std".into()],
                    bbs: vec!["BBS-09".into()],
                    min_client_version: None,
                }],
            },
        )
        .unwrap();
        let r = run(&s, dist.path()).unwrap();
        assert!(has_blocking(&r, "BBS-09"), "{:?}", r.findings);
    }

    /// 产物被人手改过（字节与生成时记录的不一致）→ 阻断
    #[test]
    fn hand_edited_output_blocks() {
        let (dist, _d, s) = setup();
        list_it(&s);
        let out = generate::generate_one(&s, dist.path(), "A1", "std", &caps()).unwrap();
        let p = resolve_in(dist.path(), &out.output_rel).unwrap();
        crate::fsx::atomic::atomic_write(&p, "# 我手改了\n".as_bytes()).unwrap();

        let r = run(&s, dist.path()).unwrap();
        assert!(has_blocking(&r, "有人手改过它"), "{:?}", r.findings);
    }

    /// 未分配的 BBS 只给一条**轻**提示
    #[test]
    fn unassigned_bbs_is_only_a_warning() {
        let (dist, _d, s) = setup();
        list_it(&s);
        generate::generate_one(&s, dist.path(), "A1", "std", &caps()).unwrap();
        bbs::import(&s, "BBS-09", b"{}").unwrap();

        let r = run(&s, dist.path()).unwrap();
        assert!(r.can_publish, "未分配不该拦住发布");
        assert!(r.findings.iter().any(|f| f.message.contains("仅归档")));
    }
}
