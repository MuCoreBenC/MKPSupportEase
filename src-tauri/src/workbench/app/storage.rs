//! 落盘：把 `workbench/` 里的文件读成 [`Committed`]，把草稿与保存写回去。
//!
//! # 文件布局
//!
//! ```text
//! workbench/
//!   machines/{机型}.json                    机型基底（我们写的那一半）
//!   machines/{机型}/versions/{版本}.json    版本覆盖 + 版本身份（改名/归档/BBS）
//!   delivery.json                           菜单可见性 + 套餐改动
//!   built.json                              生成记录（uid → 指纹 + 时间）
//!   .draft/book.json                        整本草稿
//!   .draft/ui.json                          界面状态（本机，不入库）
//!   .snapshots/ .trash/                     快照与回收站
//! ```
//!
//! 每份文件都带 `v`（结构版本）。**不带的话，将来改形状只能靠"试着反序列化，
//! 失败就当空"**，而那会把用户的一整本配方静默当成空的。
//!
//! # 三条规矩
//!
//! 1. **文件不存在 = 干净状态，不是错误。** 干净仓库里这些文件一个都没有（doc §7：
//!    首次启动只建目录）。解析不了才是错。
//! 2. **写只走 `fsx::atomic`。** `store` 已经把这条封住了，这里不再另开口子。
//! 3. **保存是「先写新的、再删旧的」。** 中途失败最坏是多一份文件，不会两头都没有。
//!
//! # uid 在保存时会变，所以保存要返回改名表
//!
//! uid 是 `"{加载时的机型}/{版本 id}"`（doc §3.5）。于是两种情况下它会变：
//!
//! - 草稿里新建的 `new-3`，保存后成了 `A1/NEW3`
//! - 从 A1 搬到 P1S 的 `A1/STANDARD`，保存后成了 `P1S/STANDARD`
//!
//! 前端的选中、勾选列、撤销栈都按 uid 记，所以 [`SaveOutcome::remap`] 必须交出去 ——
//! 不交的话，保存之后用户的选中会指向一个不存在的 uid，而界面上只表现为"选中莫名没了"。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::workbench::clock;
use crate::workbench::domain::layer::Overrides;
use crate::workbench::domain::patch::{
    BuiltRecord, BundleEdit, Committed, CommittedVersion, Draft, Visibility,
};
use crate::workbench::store::{validate_id, Store};
use crate::workbench::upstream::Upstream;

/// 结构版本。读到更大的数字就拒绝，而不是硬着头皮解析
const V: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MachineFile {
    v: u32,
    #[serde(default)]
    base: Overrides,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VersionFile {
    v: u32,
    /// **`None` = 没改过名，用上游那个**。存空串会把上游的中文名盖成空白
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    archived: bool,
    /// `None` = 跟机型默认
    #[serde(default)]
    bbs: Option<Vec<String>>,
    #[serde(default)]
    overrides: Overrides,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeliveryFile {
    #[serde(default)]
    v: u32,
    #[serde(default)]
    visibility: BTreeMap<String, Visibility>,
    #[serde(default)]
    bundles: BTreeMap<String, BundleEdit>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BuiltFile {
    #[serde(default)]
    v: u32,
    #[serde(default)]
    records: BTreeMap<String, BuiltRecord>,
}

const DELIVERY_REL: &str = "delivery.json";
const BUILT_REL: &str = "built.json";

/// 读出来的那一份，外加**读的时候发现的问题**。
///
/// 问题不能只写日志：仓库里有一个上游已经不存在的版本文件时，
/// 它在界面上会整个消失，而"我的改动去哪了"是一个查不出来的问题
pub struct Loaded {
    pub committed: Committed,
    /// 给界面的提示句
    pub notices: Vec<String>,
}

/// 手写而不是派生：派生版会把整本配方打印出来，而 `unwrap_err()` 失败时打的就是它
impl std::fmt::Debug for Loaded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Loaded({} 机型 / {} 版本 / {} 条提示)",
            self.committed.machines.len(),
            self.committed.versions.len(),
            self.notices.len()
        )
    }
}

/// 把 `workbench/` 读成 [`Committed`]。
///
/// 需要 `upstream` 是因为版本的**显示名**默认来自上游 —— 我们的文件里只在改过名时才有它
pub fn load(store: &Store, up: &Upstream) -> Result<Loaded, AppError> {
    let mut notices = Vec::new();
    let mut machines: BTreeMap<String, Overrides> = BTreeMap::new();
    let mut versions: BTreeMap<String, CommittedVersion> = BTreeMap::new();

    let known: BTreeMap<&str, Vec<&str>> = up
        .catalog
        .machines()
        .iter()
        .map(|m| (m.id.as_str(), m.versions.iter().map(|v| v.id.as_str()).collect()))
        .collect();

    for m in up.catalog.machines() {
        let rel = store.machine_rel(&m.id)?;
        let base = match store.read_doc::<MachineFile>(&rel, "机型基底")? {
            Some(f) => {
                check_version(f.v, &rel)?;
                f.base
            }
            None => Overrides::new(),
        };
        machines.insert(m.id.clone(), base);

        // **先给上游声明的每一版建一条空记录。**
        //
        // `Committed` 是"这些版本当前落盘的样子"，而不是"我们写过文件的那些版本"。
        // 只按文件建的话，干净仓库里 `Committed.versions` 是空的，于是
        // `patch::validate` 会把「改 A1/STANDARD 的某一项」判成"版本不存在" ——
        // 而那一版明明在上游清单里。第一次接 IPC 层时就是这么红的。
        for v in &m.versions {
            versions.insert(
                format!("{}/{}", m.id, v.id),
                CommittedVersion {
                    machine_id: m.id.clone(),
                    version_id: v.id.clone(),
                    name: v.name.clone(),
                    overrides: Overrides::new(),
                    archived: false,
                    bbs: None,
                    declared_upstream: true,
                },
            );
        }

        for vid in store.version_ids(&m.id)? {
            let rel = store.version_rel(&m.id, &vid)?;
            let Some(f) = store.read_doc::<VersionFile>(&rel, "版本覆盖")? else {
                continue;
            };
            check_version(f.v, &rel)?;

            let upstream_name = up
                .catalog
                .machine(&m.id)
                .and_then(|x| x.version(&vid))
                .map(|x| x.name.clone());
            if upstream_name.is_none() {
                // 上游已经没有这一版了。**说出来** —— 它在树上会整个消失。
                // 但记录还是要留着：留着才有机会把它的值搬走或删掉
                notices.push(format!(
                    "{}/{} 在上游已经不存在，它的改动不会出现在树上（文件还在，没有删）",
                    m.id, vid
                ));
            }
            versions.insert(
                format!("{}/{}", m.id, vid),
                CommittedVersion {
                    machine_id: m.id.clone(),
                    version_id: vid.clone(),
                    name: f.name.or(upstream_name.clone()).unwrap_or_else(|| vid.clone()),
                    overrides: f.overrides,
                    archived: f.archived,
                    bbs: f.bbs,
                    declared_upstream: upstream_name.is_some(),
                },
            );
        }
    }

    // 上游不认识的机型目录：同样要说出来
    for id in store.machine_ids()? {
        if !known.contains_key(id.as_str()) {
            notices.push(format!(
                "仓库里有机型 {id} 的配方，但上游机型清单里没有它 —— 这一份不会出现在树上"
            ));
        }
    }

    let delivery = store
        .read_doc::<DeliveryFile>(DELIVERY_REL, "菜单与套餐")?
        .unwrap_or_default();
    let built = store
        .read_doc::<BuiltFile>(BUILT_REL, "生成记录")?
        .unwrap_or_default();

    Ok(Loaded {
        committed: Committed {
            machines,
            versions,
            visibility: delivery.visibility,
            bundles: delivery.bundles,
            built: built.records,
            machine_ids: up
                .catalog
                .machines()
                .iter()
                .map(|m| m.id.clone())
                .collect(),
        },
        notices,
    })
}

fn check_version(v: u32, rel: &str) -> Result<(), AppError> {
    if v > V {
        return Err(AppError::corrupted(format!("{rel} 是更新版本的工作台写的"))
            .with_detail(format!("文件结构版本 {v}，本程序只认到 {V}。请升级工作台，不要在这个版本上改动")));
    }
    Ok(())
}

/* ---------- 草稿 ---------- */

pub fn read_draft(store: &Store) -> Result<Draft, AppError> {
    Ok(store
        .read_doc::<Draft>(Store::DRAFT_REL, "草稿")?
        .unwrap_or_default())
}

pub fn write_draft(store: &Store, draft: &Draft) -> Result<(), AppError> {
    if draft.is_clean() {
        // 干净就把文件删掉，而不是留一个全空的。留着的话
        //「有没有未保存改动」在文件系统上看不出来，排查时只能猜
        return store.remove(Store::DRAFT_REL);
    }
    store.write_doc(Store::DRAFT_REL, draft)
}

/* ---------- 界面状态 ---------- */

/// 折叠 / 页签 / 勾选。**本机状态，形状归前端** —— 后端不解析它，
/// 所以前端加一个界面开关不需要动 Rust
pub fn read_ui(store: &Store) -> Result<serde_json::Value, AppError> {
    Ok(store
        .read_doc::<serde_json::Value>(Store::UI_REL, "界面状态")?
        .unwrap_or_else(|| serde_json::json!({})))
}

pub fn write_ui(store: &Store, ui: &serde_json::Value) -> Result<(), AppError> {
    store.write_doc(Store::UI_REL, ui)
}

/* ---------- 保存 ---------- */

pub struct SaveOutcome {
    /// 旧 uid → 新 uid。**新建与移动都会改 uid**（见模块文档）
    pub remap: BTreeMap<String, String>,
    pub notices: Vec<String>,
}

/// 把草稿写回仓库文件。
///
/// 顺序是**先算出全部要写的内容，再逐个原子写，最后删旧文件**。
/// 中途失败最坏是多一份文件（下一次加载会报"上游没有这一版"），不会两头都没有
pub fn save(
    store: &Store,
    up: &Upstream,
    committed: &Committed,
    draft: &Draft,
) -> Result<SaveOutcome, AppError> {
    let mut remap: BTreeMap<String, String> = BTreeMap::new();
    let mut notices: Vec<String> = Vec::new();

    // ① 机型基底
    for (machine_id, base) in &committed.machines {
        let mut next = base.clone();
        let mut touched = false;
        for (key, value) in pending_of(draft, crate::workbench::domain::Level::Machine, machine_id) {
            touched = true;
            match value {
                Some(v) => next.insert(key, v),
                None => next.remove(&key),
            };
        }
        if touched {
            store.write_doc(&store.machine_rel(machine_id)?, &MachineFile { v: V, base: next })?;
        }
    }

    // ② 已有版本：改名 / 归档 / BBS / 覆盖 / 移动
    for (uid, cv) in &committed.versions {
        if draft.purged.contains(uid) {
            continue; // 留给第 ④ 步
        }
        let to_machine = draft.moved.get(uid).unwrap_or(&cv.machine_id).clone();
        let renamed = draft.renamed.get(uid);
        let archived = draft.archived.get(uid).copied();
        let bbs = draft.bbs.get(uid).cloned();
        let values: Vec<_> =
            pending_of(draft, crate::workbench::domain::Level::Version, uid).collect();

        let moved = to_machine != cv.machine_id;
        if !moved && renamed.is_none() && archived.is_none() && bbs.is_none() && values.is_empty() {
            continue;
        }

        let mut next = cv.overrides.clone();
        for (key, value) in values {
            match value {
                Some(v) => next.insert(key, v),
                None => next.remove(&key),
            };
        }
        let file = VersionFile {
            v: V,
            // 只在真的改过名时才写进文件；否则留 `None`，显示名跟着上游走
            name: renamed.cloned().or_else(|| {
                let upstream_name = up
                    .catalog
                    .machine(&to_machine)
                    .and_then(|m| m.version(&cv.version_id))
                    .map(|x| x.name.as_str());
                match upstream_name {
                    Some(n) if n == cv.name => None,
                    _ => Some(cv.name.clone()),
                }
            }),
            archived: archived.unwrap_or(cv.archived),
            bbs: bbs.unwrap_or_else(|| cv.bbs.clone()),
            overrides: next,
        };
        let new_rel = store.version_rel(&to_machine, &cv.version_id)?;
        store.write_doc(&new_rel, &file)?;

        if moved {
            // 先写新的再删旧的
            store.remove(&store.version_rel(&cv.machine_id, &cv.version_id)?)?;
            let new_uid = format!("{}/{}", to_machine, cv.version_id);
            remap.insert(uid.clone(), new_uid);
        }
    }

    // ③ 草稿里新建的版本
    for v in &draft.added {
        if draft.purged.contains(&v.uid) {
            continue;
        }
        validate_id(&v.version_id, "版本 id")?;
        let mut over = Overrides::new();
        for (key, value) in pending_of(draft, crate::workbench::domain::Level::Version, &v.uid) {
            if let Some(val) = value {
                over.insert(key, val);
            }
        }
        let rel = store.version_rel(&v.machine_id, &v.version_id)?;
        if store
            .read_doc::<serde_json::Value>(&rel, "版本覆盖")?
            .is_some()
        {
            // 撞上已有文件。**不覆盖** —— 那会悄悄吃掉别人的配方
            notices.push(format!(
                "{} 下已经有一个 id 为 {} 的版本文件，「{}」没有保存",
                v.machine_id, v.version_id, v.name
            ));
            continue;
        }
        store.write_doc(
            &rel,
            &VersionFile {
                v: V,
                name: Some(v.name.clone()),
                archived: draft.archived.get(&v.uid).copied().unwrap_or(false),
                bbs: draft.bbs.get(&v.uid).cloned().flatten(),
                overrides: over,
            },
        )?;
        remap.insert(v.uid.clone(), format!("{}/{}", v.machine_id, v.version_id));
    }

    // ④ 删除：**进回收站，不是直接删**
    for uid in &draft.purged {
        if let Some(cv) = committed.versions.get(uid) {
            match store.trash(&cv.machine_id, &cv.version_id) {
                Ok(stem) => notices.push(format!("{uid} 已移入回收站（{stem}）")),
                Err(e) if e.code == crate::error::ErrorCode::NotFound => {
                    // 文件本来就没有（从没保存过）—— 不算错
                }
                Err(e) => return Err(e),
            }
        }
    }

    // ⑤ 菜单 / 套餐 / 生成记录
    if !draft.visibility.is_empty() || !draft.bundles.is_empty() {
        let mut d = DeliveryFile {
            v: V,
            visibility: committed.visibility.clone(),
            bundles: committed.bundles.clone(),
        };
        d.visibility.extend(draft.visibility.clone());
        d.bundles.extend(draft.bundles.clone());
        store.write_doc(DELIVERY_REL, &d)?;
    }
    if !draft.built.is_empty() {
        let mut b = BuiltFile {
            v: V,
            records: committed.built.clone(),
        };
        b.records.extend(draft.built.clone());
        store.write_doc(BUILT_REL, &b)?;
    }

    // ⑥ 草稿清空
    store.remove(Store::DRAFT_REL)?;

    tracing::info!(
        changed = draft.dirty_count(),
        remapped = remap.len(),
        at = %clock::now_iso8601(),
        "配方已保存"
    );
    Ok(SaveOutcome { remap, notices })
}

/// 草稿里属于 `(level, owner)` 的那些值改动
fn pending_of<'a>(
    draft: &'a Draft,
    level: crate::workbench::domain::Level,
    owner: &'a str,
) -> impl Iterator<Item = (String, Option<serde_json::Value>)> + 'a {
    draft.values.iter().filter_map(move |(raw, v)| {
        crate::workbench::domain::patch::split_value_key(raw, level, owner)
            .map(|key| (key, v.clone()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::domain::patch::{apply, Patch};
    use crate::workbench::domain::testkit::Fixture;
    use crate::workbench::domain::Level;

    fn store() -> (tempfile::TempDir, Store) {
        let d = tempfile::tempdir().unwrap();
        let s = Store::at(d.path());
        s.bootstrap().unwrap();
        (d, s)
    }

    /// 干净仓库：一个文件都没有。**`Committed.versions` 仍然要有上游那四版** ——
    /// 它是"这些版本当前落盘的样子"，不是"我们写过文件的那些版本"
    #[test]
    fn a_clean_repo_loads_as_empty_not_as_an_error() {
        let f = Fixture::load();
        let (_d, s) = store();
        let loaded = load(&s, &f.up).unwrap();
        assert_eq!(loaded.committed.machines.len(), 3, "三台机型都有一张空表");
        assert_eq!(loaded.committed.versions.len(), 4, "四个版本都在，覆盖都是空的");
        assert!(loaded
            .committed
            .versions
            .values()
            .all(|v| v.overrides.is_empty() && !v.archived));
        assert_eq!(loaded.committed.versions["A1/STANDARD"].name, "标准版");
        assert!(loaded.notices.is_empty());
        assert!(read_draft(&s).unwrap().is_clean());
    }

    /// 保存 → 重新读 → 值与身份都回来
    #[test]
    fn saving_then_loading_round_trips() {
        let f = Fixture::load();
        let (_d, s) = store();
        let c = load(&s, &f.up).unwrap().committed;
        let mut draft = Draft::default();

        apply(
            &mut draft,
            &c,
            &f.up.registry,
            &[
                Patch::SetValue {
                    level: Level::Machine,
                    owner: "A1".to_owned(),
                    key: "wiping.child".to_owned(),
                    value: Some(serde_json::json!(42)),
                },
                Patch::SetValue {
                    level: Level::Version,
                    owner: "A1/STANDARD".to_owned(),
                    key: "toolhead.offset.x".to_owned(),
                    value: Some(serde_json::json!(7)),
                },
                Patch::RenameVersion {
                    uid: "A1/STANDARD".to_owned(),
                    name: "我改的名".to_owned(),
                },
            ],
        )
        .unwrap();
        write_draft(&s, &draft).unwrap();

        let out = save(&s, &f.up, &c, &draft).unwrap();
        assert!(out.remap.is_empty(), "没有新建也没有移动");

        let again = load(&s, &f.up).unwrap().committed;
        assert_eq!(again.machines["A1"]["wiping.child"], serde_json::json!(42));
        let v = &again.versions["A1/STANDARD"];
        assert_eq!(v.overrides["toolhead.offset.x"], serde_json::json!(7));
        assert_eq!(v.name, "我改的名");
        // 草稿要被清掉
        assert!(read_draft(&s).unwrap().is_clean());
    }

    /// 没改过名的版本**不把名字写进文件** —— 写了的话上游以后改中文名，我们这边收不到
    #[test]
    fn an_unrenamed_version_does_not_pin_its_name() {
        let f = Fixture::load();
        let (_d, s) = store();
        let c = load(&s, &f.up).unwrap().committed;
        let mut draft = Draft::default();
        apply(
            &mut draft,
            &c,
            &f.up.registry,
            &[Patch::ArchiveVersion {
                uid: "A1/FAST".to_owned(),
            }],
        )
        .unwrap();
        save(&s, &f.up, &c, &draft).unwrap();

        let raw: serde_json::Value = s
            .read_doc(&s.version_rel("A1", "FAST").unwrap(), "版本")
            .unwrap()
            .unwrap();
        assert_eq!(raw["name"], serde_json::Value::Null, "名字不该被钉住");
        assert_eq!(raw["archived"], true);

        let again = load(&s, &f.up).unwrap().committed;
        assert_eq!(again.versions["A1/FAST"].name, "高速版", "显示名跟着上游");
    }

    /// **新建版本保存后 uid 会变**，所以要交出改名表
    #[test]
    fn a_new_version_gets_remapped_on_save() {
        let f = Fixture::load();
        let (_d, s) = store();
        let c = load(&s, &f.up).unwrap().committed;
        let mut draft = Draft::default();
        apply(
            &mut draft,
            &c,
            &f.up.registry,
            &[Patch::NewVersion {
                machine_id: "A1".to_owned(),
                name: "我的新配方".to_owned(),
            }],
        )
        .unwrap();
        apply(
            &mut draft,
            &c,
            &f.up.registry,
            &[Patch::SetValue {
                level: Level::Version,
                owner: "new-1".to_owned(),
                key: "wiping.child".to_owned(),
                value: Some(serde_json::json!(5)),
            }],
        )
        .unwrap();

        let out = save(&s, &f.up, &c, &draft).unwrap();
        assert_eq!(out.remap.get("new-1").map(String::as_str), Some("A1/NEW1"));

        let again = load(&s, &f.up).unwrap();
        // 上游没有 NEW1 这一版 → 要有一条提示，而不是静默消失
        assert_eq!(again.notices.len(), 1);
        assert!(again.notices[0].contains("NEW1"));
        let v = &again.committed.versions["A1/NEW1"];
        assert_eq!(v.name, "我的新配方");
        assert_eq!(v.overrides["wiping.child"], serde_json::json!(5));
    }

    /// **移动之后 uid 也会变**，而且旧文件要删掉
    #[test]
    fn moving_rewrites_the_file_and_remaps_the_uid() {
        let f = Fixture::load();
        let (_d, s) = store();

        // 先落一份 A1/FAST
        let c = load(&s, &f.up).unwrap().committed;
        let mut draft = Draft::default();
        apply(
            &mut draft,
            &c,
            &f.up.registry,
            &[Patch::SetValue {
                level: Level::Version,
                owner: "A1/FAST".to_owned(),
                key: "wiping.child".to_owned(),
                value: Some(serde_json::json!(11)),
            }],
        )
        .unwrap();
        save(&s, &f.up, &c, &draft).unwrap();

        // 再把它搬到 P1S
        let c = load(&s, &f.up).unwrap().committed;
        let mut draft = Draft::default();
        apply(
            &mut draft,
            &c,
            &f.up.registry,
            &[Patch::MoveVersion {
                uid: "A1/FAST".to_owned(),
                to_machine_id: "P1S".to_owned(),
            }],
        )
        .unwrap();
        let out = save(&s, &f.up, &c, &draft).unwrap();
        assert_eq!(out.remap.get("A1/FAST").map(String::as_str), Some("P1S/FAST"));

        assert!(
            s.read_doc::<serde_json::Value>(&s.version_rel("A1", "FAST").unwrap(), "版本")
                .unwrap()
                .is_none(),
            "旧文件要删掉，不然下次加载会出现两份"
        );
        let again = load(&s, &f.up).unwrap();
        assert_eq!(
            again.committed.versions["P1S/FAST"].overrides["wiping.child"],
            serde_json::json!(11)
        );
    }

    /// 删除**进回收站**。
    ///
    /// 能删的只有「上游已经不再有的版本」—— 上游还声明着的版本删不掉（`patch` 那边拦着），
    /// 因为版本清单是上游的，删掉我们的文件它照样在树上
    #[test]
    fn purging_an_orphan_version_moves_the_file_to_the_trash() {
        let f = Fixture::load();
        let (_d, s) = store();
        // 手工放一份上游没有的版本 —— 相当于上游把 OLD 这一版删掉了
        s.write_doc(
            &s.version_rel("A1", "OLD").unwrap(),
            &VersionFile {
                v: V,
                name: Some("老版本".to_owned()),
                archived: false,
                bbs: None,
                overrides: [("wiping.child".to_owned(), serde_json::json!(1))]
                    .into_iter()
                    .collect(),
            },
        )
        .unwrap();

        let loaded = load(&s, &f.up).unwrap();
        assert!(!loaded.committed.versions["A1/OLD"].declared_upstream);
        assert_eq!(loaded.notices.len(), 1, "上游没有这一版要有一条提示");

        let c = loaded.committed;
        let mut draft = Draft::default();
        apply(
            &mut draft,
            &c,
            &f.up.registry,
            &[Patch::PurgeVersion {
                uid: "A1/OLD".to_owned(),
            }],
        )
        .unwrap();
        let out = save(&s, &f.up, &c, &draft).unwrap();

        assert_eq!(s.trash_entries().unwrap().len(), 1);
        assert!(out.notices.iter().any(|n| n.contains("回收站")));
        let again = load(&s, &f.up).unwrap();
        assert!(!again.committed.versions.contains_key("A1/OLD"));
        assert!(again.notices.is_empty(), "孤儿清掉了，提示也该没了");
    }

    /// 上游还声明着的版本**删不掉**，并且要指出该用归档
    #[test]
    fn purging_an_upstream_version_is_refused_and_points_at_archiving() {
        let f = Fixture::load();
        let (_d, s) = store();
        let c = load(&s, &f.up).unwrap().committed;
        let mut draft = Draft::default();
        let e = apply(
            &mut draft,
            &c,
            &f.up.registry,
            &[Patch::PurgeVersion {
                uid: "A1/FAST".to_owned(),
            }],
        )
        .unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::InvalidArgument);
        assert!(e.detail.unwrap_or_default().contains("归档"));
        assert!(draft.is_clean());
    }

    /// 新建的 id 撞上已有文件时**不覆盖**，记一条提示
    #[test]
    fn a_colliding_new_version_is_refused_not_overwritten() {
        let f = Fixture::load();
        let (_d, s) = store();
        // 手工放一份 A1/NEW1
        s.write_doc(
            &s.version_rel("A1", "NEW1").unwrap(),
            &VersionFile {
                v: V,
                name: Some("别人的配方".to_owned()),
                archived: false,
                bbs: None,
                overrides: Overrides::new(),
            },
        )
        .unwrap();

        let c = load(&s, &f.up).unwrap().committed;
        let mut draft = Draft::default();
        apply(
            &mut draft,
            &c,
            &f.up.registry,
            &[Patch::NewVersion {
                machine_id: "A1".to_owned(),
                name: "我的新配方".to_owned(),
            }],
        )
        .unwrap();
        let out = save(&s, &f.up, &c, &draft).unwrap();

        assert!(out.remap.is_empty());
        assert_eq!(out.notices.len(), 1);
        let again = load(&s, &f.up).unwrap().committed;
        assert_eq!(
            again.versions["A1/NEW1"].name, "别人的配方",
            "别人的配方一个字节都不该被动"
        );
    }

    /// 干净草稿**不留空文件** —— 留着的话「有没有未保存改动」在文件系统上看不出来
    #[test]
    fn a_clean_draft_leaves_no_file() {
        let f = Fixture::load();
        let (_d, s) = store();
        let c = load(&s, &f.up).unwrap().committed;
        let mut draft = Draft::default();
        apply(
            &mut draft,
            &c,
            &f.up.registry,
            &[Patch::SetValue {
                level: Level::Machine,
                owner: "A1".to_owned(),
                key: "wiping.child".to_owned(),
                value: Some(serde_json::json!(1)),
            }],
        )
        .unwrap();
        write_draft(&s, &draft).unwrap();
        assert!(s
            .read_doc::<serde_json::Value>(Store::DRAFT_REL, "草稿")
            .unwrap()
            .is_some());

        write_draft(&s, &Draft::default()).unwrap();
        assert!(s
            .read_doc::<serde_json::Value>(Store::DRAFT_REL, "草稿")
            .unwrap()
            .is_none());
    }

    /// 更新版本的工作台写的文件**要拒绝**，不能硬着头皮解析成半份
    #[test]
    fn a_file_from_a_newer_workbench_is_refused() {
        let f = Fixture::load();
        let (_d, s) = store();
        s.write_doc(
            &s.machine_rel("A1").unwrap(),
            &serde_json::json!({ "v": 99, "base": { "wiping.child": 1 } }),
        )
        .unwrap();
        let e = load(&s, &f.up).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::Corrupted);
        assert!(e.detail.unwrap_or_default().contains("99"));
    }

    /// 界面状态**后端不解析**：前端加一个开关不该需要动 Rust
    #[test]
    fn ui_state_is_stored_opaquely() {
        let (_d, s) = store();
        assert_eq!(read_ui(&s).unwrap(), serde_json::json!({}));
        let ui = serde_json::json!({ "collapsed": ["A1"], "随便加的键": 1 });
        write_ui(&s, &ui).unwrap();
        assert_eq!(read_ui(&s).unwrap(), ui);
    }
}
