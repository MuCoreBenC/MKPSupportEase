//! 发布。
//!
//! 发布是**独立动作**：与编辑、保存、生成分开（doc §13）。它只做两件事 ——
//! 把 BBS 原样转运到发布目录，把菜单与套餐写出去，并给每个条目补上
//! sha256 / size / minClientVersion。
//!
//! 两条顺序上的硬要求：
//!
//! 1. **出货检查有阻断项就不发**。不是"发了再说"，是不发。
//! 2. **`catalog.json` 最后一个替换**。它是客户端的唯一入口 —— 先写它，
//!    客户端就可能读到"目录说有、文件还没到"的中间态。
//!
//! 还有一条不在顺序里但同样要紧：**开发配方 JSON 不在发布对象里**。
//! 这个函数从头到尾没有任何一处会去读 `machines/`，所以它不可能被带出去。
//! （仓库是开源的，别人能在仓库里看到那些文件 —— 那是"来后厨参观"，不是交付通道。）

use std::path::Path;

use serde::Serialize;

use crate::error::AppError;
use crate::fsx::atomic::{atomic_write, atomic_write_json};
use crate::fsx::paths::resolve_in;
use crate::workbench::catalog;
use crate::workbench::clock;
use crate::workbench::preflight;
use crate::workbench::state;
use crate::workbench::store::Store;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishReport {
    pub preset_count: usize,
    pub bbs_count: usize,
    pub bundle_count: usize,
    pub published_at: String,
    /// 发布了什么文件，按实际写盘顺序列出 —— catalog.json 必须在最后
    pub files: Vec<String>,
}

pub fn run(store: &Store, dist_root: &Path) -> Result<PublishReport, AppError> {
    let report = preflight::run(store, dist_root)?;
    if !report.can_publish {
        let first = report
            .findings
            .iter()
            .filter(|f| f.severity == preflight::Severity::Blocking)
            .map(|f| f.message.clone())
            .collect::<Vec<_>>()
            .join("；");
        return Err(
            AppError::invalid_argument(format!("出货检查有 {} 个阻断项，没有发布", report.blocking))
                .with_detail(first),
        );
    }

    let mut catalog = catalog::load_catalog(store)?;
    let bundles = catalog::load_bundles(store)?;
    let statuses = state::status_all(store, dist_root)?;
    let mut files = Vec::new();

    /* BBS 原样转运。**不解析、不重排、不改一个字节** ——
    客户端拿到的必须与外部给我们的那一份完全相同 */
    for entry in &mut catalog.bbs {
        let src = store.root.join("bbs").join(format!("{}.json", entry.bbs_id));
        let bytes = std::fs::read(&src).map_err(|e| {
            AppError::io(format!("读不出 BBS {}", entry.bbs_id)).with_detail(e.to_string())
        })?;
        let rel = format!("bbs/{}.json", entry.bbs_id);
        let dst = resolve_in(dist_root, &rel)?;
        // 字节相同就不重写：mtime 变化会让同步工具以为有更新
        let same = std::fs::read(&dst).map(|old| old == bytes).unwrap_or(false);
        if !same {
            atomic_write(&dst, &bytes)?;
        }
        entry.resource = rel.clone();
        entry.sha256 = Some(sha256_bytes(&bytes));
        entry.size = Some(bytes.len() as u64);
        files.push(rel);
    }

    /* 预设条目补元数据。sha256 与 minClientVersion **都取自生成时的记录**，
    不在这里重算 minClientVersion —— 那是生成时按能力定义算出来的结论，
    发布时再算一遍就成了第二个真相源 */
    for p in &mut catalog.presets {
        let path = resolve_in(dist_root, &p.resource)?;
        let bytes = std::fs::read(&path).map_err(|e| {
            AppError::io(format!("读不出产物 {}", p.resource)).with_detail(e.to_string())
        })?;
        p.sha256 = Some(sha256_bytes(&bytes));
        p.size = Some(bytes.len() as u64);
        p.min_client_version = statuses
            .iter()
            .find(|s| s.machine_id == p.machine && s.version_id == p.version)
            .and_then(|s| s.min_client_version.clone());
        files.push(p.resource.clone());
    }

    // 套餐先写
    let bundles_rel = "bundles.json";
    atomic_write_json(&resolve_in(dist_root, bundles_rel)?, &bundles)?;
    files.push(bundles_rel.to_owned());

    // 菜单最后写 —— 顺序就是这个函数存在的一半理由
    catalog.generated_at = Some(clock::now_iso8601());
    let catalog_rel = catalog::CATALOG_FILE;
    atomic_write_json(&resolve_in(dist_root, catalog_rel)?, &catalog)?;
    files.push(catalog_rel.to_owned());

    Ok(PublishReport {
        preset_count: catalog.presets.len(),
        bbs_count: catalog.bbs.len(),
        bundle_count: bundles.bundles.len(),
        published_at: catalog.generated_at.unwrap_or_default(),
        files,
    })
}

fn sha256_bytes(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::bbs;
    use crate::workbench::capability::{CapField, ClientCapability, Support};
    use crate::workbench::catalog::{BbsEntry, BbsOffering, Catalog, PresetEntry};
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

        generate::generate_one(&s, dist.path(), "A1", "std", &caps()).unwrap();

        catalog::save_catalog(
            &s,
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

        (dist, d, s)
    }

    #[test]
    fn publishes_catalog_last() {
        let (dist, _d, s) = setup();
        let r = run(&s, dist.path()).unwrap();
        assert_eq!(r.files.last().unwrap(), "catalog.json", "菜单不是最后写的");
        assert!(dist.path().join("catalog.json").is_file());
        assert!(dist.path().join("bundles.json").is_file());
    }

    /// 发布时才给条目补上 sha256 / size / minClientVersion，
    /// 而 minClientVersion **取自生成时的记录**，不在发布时重算
    #[test]
    fn fills_metadata_from_generation_record() {
        let (dist, _d, s) = setup();
        run(&s, dist.path()).unwrap();

        let text = std::fs::read_to_string(dist.path().join("catalog.json")).unwrap();
        let c: Catalog = serde_json::from_str(&text).unwrap();
        let p = &c.presets[0];
        assert!(p.sha256.is_some());
        assert!(p.size.unwrap() > 0);
        assert_eq!(p.min_client_version.as_deref(), Some("0.1.0"));
        assert!(c.generated_at.is_some());
    }

    /// 出货检查有阻断项 → **不发布**，而且发布目录里不会多出 catalog.json
    #[test]
    fn blocking_findings_abort_publish() {
        let (dist, _d, s) = setup();
        // 把产物删掉 → 上架条目指向不存在的文件 → 阻断
        std::fs::remove_file(dist.path().join("presets/a1-std.toml")).unwrap();

        let e = run(&s, dist.path()).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::InvalidArgument);
        assert!(!dist.path().join("catalog.json").exists(), "阻断了却还是写了菜单");
    }

    /// BBS 原样转运，字节必须完全相同
    #[test]
    fn bbs_is_copied_byte_for_byte() {
        let (dist, _d, s) = setup();
        let raw = b"{\n  \"drink\": \"\xe8\xbf\x9b\xe5\x8f\xa3\"\n}\n";
        bbs::import(&s, "BBS-01", raw).unwrap();

        let mut c = catalog::load_catalog(&s).unwrap();
        c.bbs.push(BbsEntry {
            bbs_id: "BBS-01".into(),
            display_name: "进口饮料".into(),
            resource: "bbs/BBS-01.json".into(),
            sha256: None,
            size: None,
            min_client_version: None,
            offering: BbsOffering::Optional,
        });
        catalog::save_catalog(&s, &c).unwrap();

        run(&s, dist.path()).unwrap();
        assert_eq!(
            std::fs::read(dist.path().join("bbs/BBS-01.json")).unwrap(),
            raw,
            "转运时改了字节"
        );
    }

    /// **开发配方不进发布目录。** 这条是这一整套边界里最该有断言的一条
    #[test]
    fn development_recipes_never_reach_the_dist_dir() {
        let (dist, _d, s) = setup();
        run(&s, dist.path()).unwrap();

        let mut found = Vec::new();
        walk(dist.path(), &mut found);
        for p in &found {
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            assert_ne!(name, "registry.json", "字段定义被发布出去了");
            assert_ne!(name, "fallback.json", "回退表被发布出去了");
        }
        assert!(
            !found.iter().any(|p| p.components().any(|c| c.as_os_str() == "machines")),
            "机型/版本的开发配方出现在发布目录里"
        );
    }

    fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else {
                out.push(p);
            }
        }
    }
}
