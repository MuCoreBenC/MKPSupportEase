//! BBS 收录、归属与三态。
//!
//! BBS 是**外部资源**：不是配方生成的，工作台也不改它的内容 —— 收货、贴标、决定配给谁，
//! 就这三件事（doc §10）。所以这里只有"原样存放"与"记谁用它"，没有任何编辑函数。
//!
//! 三态不是一个存在文件里的字段，而是**算出来的**：
//! - **已分配** —— 有版本会交付它，且它在菜单上
//! - **可选** —— 它在菜单上，标了 optional，用户可以额外下载
//! - **仅归档** —— 在仓库里，但**不在菜单上**。客户端完全看不到也下不了
//!
//! "仅归档"刻意没有对应的枚举值存进菜单（见 `catalog::BbsOffering` 的注释）：
//! 不在菜单上就是仅归档。给它一个菜单里的值，等于承认"在菜单上但不交付"这种
//! 自相矛盾的状态可以被表达出来。

use serde::Serialize;

use crate::error::AppError;
use crate::fsx::atomic::atomic_write;
use crate::workbench::catalog::{self, BbsOffering};
use crate::workbench::model::BbsBinding;
use crate::workbench::store::{validate_id, Store};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BbsState {
    Assigned,
    Optional,
    ArchivedOnly,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BbsFile {
    pub id: String,
    pub size: u64,
    pub sha256: String,
    /// JSON 解析得通吗。**解析不通也要列出来** —— 它在仓库里是个事实，
    /// 藏起来只会让人以为文件丢了
    pub parses: bool,
    /// 哪些版本会交付它（`机型/版本`）
    pub used_by: Vec<String>,
    pub state: BbsState,
}

/// 清单。**未分配的也常显**（doc §10）：出货检查那一条轻提示才有对照
pub fn list(store: &Store) -> Result<Vec<BbsFile>, AppError> {
    let dir = store.root.join("bbs");
    let catalog = catalog::load_catalog(store)?;

    // 先把"谁用它"算出来：机型默认清单 + 版本自己的清单
    let mut usage: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    for mid in store.machine_ids()? {
        let Ok(machine) = store.machine(&mid) else {
            continue;
        };
        for vid in store.version_ids(&mid)? {
            let Ok(version) = store.version(&mid, &vid) else {
                continue;
            };
            let ids = match &version.bbs {
                BbsBinding::Inherit => machine.default_bbs.clone(),
                BbsBinding::Own { ids } => ids.clone(),
            };
            for id in ids {
                usage.entry(id).or_default().push(format!("{mid}/{vid}"));
            }
        }
    }

    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Ok(out);
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let Some(id) = p.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let bytes = std::fs::read(&p).unwrap_or_default();
        let listed = catalog.bbs.iter().find(|b| b.bbs_id == id);
        let used_by = usage.get(id).cloned().unwrap_or_default();

        let state = match listed.map(|b| b.offering) {
            // 在菜单上且标了已分配：还要真有版本用它，否则它只是"挂在菜单上没人配"
            Some(BbsOffering::Assigned) if !used_by.is_empty() => BbsState::Assigned,
            Some(BbsOffering::Assigned) | Some(BbsOffering::Optional) => BbsState::Optional,
            None => BbsState::ArchivedOnly,
        };

        out.push(BbsFile {
            id: id.to_owned(),
            size: bytes.len() as u64,
            sha256: sha256_bytes(&bytes),
            parses: serde_json::from_slice::<serde_json::Value>(&bytes).is_ok(),
            used_by,
            state,
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

/// 收录一个外部文件。**原样存放，不解析、不重排、不改一个字节**。
///
/// 只做一件校验：JSON 得解析得通。不通就拒收 —— 收进来一个坏文件，
/// 它会一路走到出货检查才被发现，而那时你已经忘了它是哪来的。
pub fn import(store: &Store, id: &str, bytes: &[u8]) -> Result<BbsFile, AppError> {
    validate_id(id, "BBS id")?;
    serde_json::from_slice::<serde_json::Value>(bytes)
        .map_err(|e| AppError::corrupted("这个文件不是合法 JSON，没有收录").with_detail(e.to_string()))?;

    let path = store.root.join("bbs").join(format!("{id}.json"));
    if path.exists() {
        return Err(AppError::invalid_argument(format!(
            "已经有一个叫 {id} 的 BBS 了，换个 id 或先删掉它"
        )));
    }
    atomic_write(&path, bytes)?;

    list(store)?
        .into_iter()
        .find(|b| b.id == id)
        .ok_or_else(|| AppError::internal("刚收录的文件又读不到了"))
}

/// 机型的默认 BBS 清单
pub fn set_machine_default(
    store: &Store,
    machine_id: &str,
    ids: Vec<String>,
) -> Result<(), AppError> {
    ensure_all_exist(store, &ids)?;
    let mut m = store.machine(machine_id)?;
    m.default_bbs = ids;
    store.save_machine(&m)
}

/// 版本的 BBS 绑定：继承机型，或**脱钩成完全独立的一份清单**。
///
/// 脱钩后不做增删式继承（doc §10）：BBS 是外部文件不是配方字段，
/// "在机型清单上加两瓶减一瓶"这种规则一旦引入，"这个版本到底带哪几瓶"就要靠心算
pub fn set_version_binding(
    store: &Store,
    machine_id: &str,
    version_id: &str,
    binding: BbsBinding,
) -> Result<(), AppError> {
    if let BbsBinding::Own { ids } = &binding {
        ensure_all_exist(store, ids)?;
    }
    let mut v = store.version(machine_id, version_id)?;
    v.bbs = binding;
    store.save_version(&v)
}

fn ensure_all_exist(store: &Store, ids: &[String]) -> Result<(), AppError> {
    for id in ids {
        validate_id(id, "BBS id")?;
        if !store.root.join("bbs").join(format!("{id}.json")).exists() {
            return Err(AppError::not_found(format!("仓库里没有 BBS {id}")));
        }
    }
    Ok(())
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
    use crate::workbench::catalog::{BbsEntry, Catalog};
    use crate::workbench::model::{Machine, Version};
    use crate::workbench::store::empty_params;

    fn setup() -> (tempfile::TempDir, Store) {
        let d = tempfile::tempdir().unwrap();
        let s = Store::at(d.path());
        s.bootstrap().unwrap();
        s.save_machine(&Machine {
            id: "A1".into(),
            display_name: "A1".into(),
            base: empty_params(),
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
        (d, s)
    }

    fn listed(s: &Store, id: &str, offering: BbsOffering) {
        catalog::save_catalog(
            s,
            &Catalog {
                bbs: vec![BbsEntry {
                    bbs_id: id.into(),
                    display_name: id.into(),
                    resource: format!("bbs/{id}.json"),
                    sha256: None,
                    size: None,
                    min_client_version: None,
                    offering,
                }],
                ..Default::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn imports_bytes_verbatim() {
        let (_d, s) = setup();
        let raw = b"{\n  \"a\": 1\n}\n";
        let f = import(&s, "BBS-01", raw).unwrap();
        assert_eq!(f.id, "BBS-01");
        let on_disk = std::fs::read(s.root.join("bbs/BBS-01.json")).unwrap();
        assert_eq!(on_disk, raw, "收录时改了字节");
    }

    #[test]
    fn refuses_broken_json() {
        let (_d, s) = setup();
        let e = import(&s, "BBS-01", b"{ not json").unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::Corrupted);
        assert!(!s.root.join("bbs/BBS-01.json").exists(), "拒收了却写了文件");
    }

    #[test]
    fn refuses_duplicate_id() {
        let (_d, s) = setup();
        import(&s, "BBS-01", b"{}").unwrap();
        assert!(import(&s, "BBS-01", b"{}").is_err());
    }

    /// 不在菜单上 = 仅归档。客户端看不到，但工作台清单里**必须看得到**
    #[test]
    fn unlisted_file_is_archived_only_but_still_listed() {
        let (_d, s) = setup();
        import(&s, "BBS-09", b"{}").unwrap();
        let all = list(&s).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].state, BbsState::ArchivedOnly);
        assert!(all[0].used_by.is_empty());
    }

    /// 在菜单上、标了已分配、且真有版本用它 → 已分配
    #[test]
    fn assigned_requires_both_menu_and_a_user() {
        let (_d, s) = setup();
        import(&s, "BBS-01", b"{}").unwrap();
        listed(&s, "BBS-01", BbsOffering::Assigned);

        // 还没有版本用它 → 只能算可选，不能算已分配
        assert_eq!(list(&s).unwrap()[0].state, BbsState::Optional);

        set_machine_default(&s, "A1", vec!["BBS-01".into()]).unwrap();
        let f = &list(&s).unwrap()[0];
        assert_eq!(f.state, BbsState::Assigned);
        assert_eq!(f.used_by, vec!["A1/std"]);
    }

    /// 版本脱钩后是**完全独立**的一份清单，不继承机型默认
    #[test]
    fn own_binding_replaces_inherited_list() {
        let (_d, s) = setup();
        import(&s, "BBS-01", b"{}").unwrap();
        import(&s, "BBS-02", b"{}").unwrap();
        set_machine_default(&s, "A1", vec!["BBS-01".into()]).unwrap();

        set_version_binding(
            &s,
            "A1",
            "std",
            BbsBinding::Own {
                ids: vec!["BBS-02".into()],
            },
        )
        .unwrap();

        let all = list(&s).unwrap();
        let by = |id: &str| all.iter().find(|b| b.id == id).unwrap();
        assert!(by("BBS-01").used_by.is_empty(), "脱钩后还在继承机型默认");
        assert_eq!(by("BBS-02").used_by, vec!["A1/std"]);
    }

    /// 绑定一个仓库里没有的 id：当场拒绝，不留一个指向空气的引用
    #[test]
    fn binding_to_missing_file_is_refused() {
        let (_d, s) = setup();
        let e = set_machine_default(&s, "A1", vec!["BBS-404".into()]).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::NotFound);
    }

    /// 坏 JSON 若是被手动塞进目录的：列出来并标 parses=false，不跳过
    #[test]
    fn broken_file_on_disk_is_listed_with_a_flag() {
        let (_d, s) = setup();
        atomic_write(&s.root.join("bbs/BBS-BAD.json"), b"{ oops").unwrap();
        let all = list(&s).unwrap();
        assert_eq!(all.len(), 1);
        assert!(!all[0].parses);
    }
}
