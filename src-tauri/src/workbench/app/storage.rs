//! 落盘：把仓库里的文件读成 [`Committed`]，把草稿写回去。
//!
//! # 文件布局
//!
//! ```text
//! workbench/
//!   delivery.json            菜单可见性 + 套餐改动
//!   built.json               生成记录（uid → 指纹 + 时间）
//!   .draft/book.json         整本草稿
//!   .draft/ui.json           界面状态（本机，不入库）
//!   .snapshots/ .trash/      快照与回收站
//! ```
//!
//! # **`workbench/` 里没有参数值**（b04 Task 12）
//!
//! 值的唯一真源是 `presets/registry/param_registry.toml` 的 `machineVariants`，
//! 由 [`Presets::apply_values`] 写。**这里不存第二个副本** ——
//! 以前这里有 `machines/{机型}.json` 与 `machines/{机型}/versions/{版本}.json`
//! 两棵树，"分层"因此要排五档才能查出一个值（见 `domain::layer` 的模块文档）。
//! 那份数据盘上本来就一个文件都没有，删掉不需要迁移。
//!
//! 每份文件都带 `v`（结构版本）。**不带的话，将来改形状只能靠"试着反序列化，
//! 失败就当空"**，而那会把用户的一整本配方静默当成空的。
//!
//! # 清单与身份也不在这里（b04 Task 8 / 12）
//!
//! 有哪些机型、每台有哪些版本、版本叫什么，来自 `presets/machines/*.toml` ——
//! 那是我们自己的数据，**可写**。改名 / 加版本 / 删版本都在「机型与版本」页即时落盘，
//! 不走草稿也不进撤销栈。所以 [`load`] 收 `&Presets`。
//!
//! # 三条规矩
//!
//! 1. **文件不存在 = 干净状态，不是错误。** 干净仓库里这些文件一个都没有（doc §7：
//!    首次启动只建目录）。解析不了才是错。
//! 2. **写只走 `fsx::atomic`。** `store` 已经把这条封住了，这里不再另开口子。
//! 3. **保存是「先写新的、再删旧的」。** 中途失败最坏是多一份文件，不会两头都没有。
//!
//! # `remap` 为什么还在
//!
//! 历史上保存会改 uid（新建的 `new-3` 落成 `A1/NEW3`，搬家的 `A1/STANDARD`
//! 变成 `P1S/STANDARD`），所以要交一张改名表给前端。b04 Task 12 把新建与搬家
//! 都挪去了「机型与版本」页，现在保存不再改 uid —— 这个字段留着是因为
//! `wb_save` 的返回值形状是前端契约的一部分，删它要连 `api.ts` 一起动，
//! 那是 Task 20 那一轮的事。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::workbench::clock;
use crate::workbench::domain::layer::Level;
use crate::workbench::domain::patch::{
    BuiltRecord, BundleEdit, CatalogMachine, Committed, CommittedVersion, Draft, Visibility,
};
use crate::workbench::domain::variants;
use crate::workbench::presets::Presets;
use crate::workbench::store::Store;

/// 结构版本。读到更大的数字就拒绝，而不是硬着头皮解析
const V: u32 = 1;

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
            self.committed.catalog.len(),
            self.committed.versions.len(),
            self.notices.len()
        )
    }
}

/// 把仓库读成 [`Committed`]。
///
/// **清单与身份全部来自 `presets/machines/*.toml`**（b04 Task 8），值一个都不在这里读 ——
/// 它们住在 `param_registry.toml` 的 `machineVariants` 里，由 `Presets` 持有。
/// 所以 [`Committed`] 在这里只被填出"有哪些机型、每台有哪些版本、版本叫什么"。
pub fn load(store: &Store, presets: &Presets) -> Result<Loaded, AppError> {
    let notices = Vec::new();
    let mut versions: BTreeMap<String, CommittedVersion> = BTreeMap::new();
    let mut catalog: Vec<CatalogMachine> = Vec::new();

    for m in presets.catalog.machines() {
        catalog.push(CatalogMachine {
            id: m.id.clone(),
            display: m.display.clone(),
            icon: m.icon.clone(),
            default_bundle: m.default_bundle.clone(),
            has_dimensions: m.has_dimensions,
            version_ids: m.versions.iter().map(|v| v.id.clone()).collect(),
        });

        // 清单里的每一版都建一条记录。**不可以只给"有文件的版本"建** ——
        // 那样干净仓库里 `Committed.versions` 是空的，`patch::validate`
        // 会把「改 A1/STANDARD 的某一项」判成"版本不存在"，而那一版明明在清单里
        for v in &m.versions {
            versions.insert(
                format!("{}/{}", m.id, v.id),
                CommittedVersion {
                    machine_id: m.id.clone(),
                    version_id: v.id.clone(),
                    name: v.name.clone(),
                    tag: v.tag.clone(),
                },
            );
        }
    }

    let delivery = store
        .read_doc::<DeliveryFile>(DELIVERY_REL, "菜单与套餐")?
        .unwrap_or_default();
    let built = store
        .read_doc::<BuiltFile>(BUILT_REL, "生成记录")?
        .unwrap_or_default();
    check_version(delivery.v, DELIVERY_REL)?;
    check_version(built.v, BUILT_REL)?;

    Ok(Loaded {
        committed: Committed {
            versions,
            visibility: delivery.visibility,
            bundles: delivery.bundles,
            built: built.records,
            catalog,
        },
        notices,
    })
}

fn check_version(v: u32, rel: &str) -> Result<(), AppError> {
    if v > V {
        return Err(
            AppError::corrupted(format!("{rel} 是更新版本的工作台写的")).with_detail(format!(
                "文件结构版本 {v}，本程序只认到 {V}。请升级工作台，不要在这个版本上改动"
            )),
        );
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
    /// 旧 uid → 新 uid（见模块文档那条 `remap 为什么还在`）
    pub remap: BTreeMap<String, String>,
}

/// 把草稿写回仓库。
///
/// 值走 [`Presets::apply_values`] 一次写进 `machineVariants`，
/// 其余（菜单、套餐、生成记录）还是本品 `workbench/*.json`。
///
/// `presets` 是 `&mut` 的 —— 写值要动内存中那份文档
pub fn save(
    store: &Store,
    presets: &mut Presets,
    committed: &Committed,
    draft: &Draft,
) -> Result<SaveOutcome, AppError> {
    let edits = plan_value_edits(presets, committed, draft);
    presets.apply_values(&edits)?;

    // ② 菜单 / 套餐 / 生成记录
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

    // ③ 草稿清空
    store.remove(Store::DRAFT_REL)?;

    tracing::info!(
        changed = draft.dirty_count(),
        written = edits.len(),
        at = %clock::now_iso8601(),
        "配方已保存"
    );
    Ok(SaveOutcome {
        remap: BTreeMap::new(),
    })
}

/// 草稿里的值改动 → [`Presets::apply_values`] 要写的那一批 `(字段 key, owner, 值)`。
///
/// # 为什么机型层的一条改动可能要落好几个键
///
/// 归并（[`crate::workbench::domain::variants::digest`]）会把「**每个版本都写了同一个值**」
/// 的那些版本键上提成机型基底。于是「机型基底」在界面上是一列，
/// 在 `machineVariants` 里却可能是 `A1` + `A1:STANDARD` + `A1:FAST` + … 一整组键。
///
/// 只写裸键 `A1` 会被那几条版本键盖住（它们更具体），于是**用户点了保存，
/// 值又变回去了，而且没有任何一步报错** —— 这正是 Task 12.3 要处理的第一件事。
/// 反过来，版本层的一条改动**只写一个键**：`A1:FAST` 之外一个都不动 ——
/// 顺手删掉裸键 `A1` 会让别的版本失去它们继承来的那个值。
fn plan_value_edits(
    presets: &Presets,
    committed: &Committed,
    draft: &Draft,
) -> Vec<(String, String, Option<serde_json::Value>)> {
    let mut out = Vec::new();

    for m in &committed.catalog {
        for (key, value) in pending_of(draft, Level::Machine, &m.id) {
            // 上提过的那几条版本键此刻还钉着旧值 —— 它们的新值得跟着一起写，
            // 否则下一次归并又把它们提上来，把这一刀抹掉
            let mut owners = vec![m.id.clone()];
            if let Some(param) = presets.registry.param(&key) {
                if variants::promoted_to_base(param, &m.id, &m.version_ids) {
                    owners.extend(m.version_ids.iter().map(|v| format!("{}:{}", m.id, v)));
                }
            }
            for owner in owners {
                out.push((key.clone(), owner, value.clone()));
            }
        }

        for vid in &m.version_ids {
            let uid = format!("{}/{}", m.id, vid);
            for (key, value) in pending_of(draft, Level::Version, &uid) {
                out.push((key, format!("{}:{}", m.id, vid), value));
            }
        }
    }

    out
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
    //! # 这里盯的是现在还存在的两件事
    //!
    //! `storage` 剩下的活只有两件：
    //!
    //! 1. **`presets.catalog` → [`Committed`]**：有哪些机型、每台有哪些版本、版本叫什么。
    //!    `workbench/` 下**一个文件都没有**也不算错。
    //! 2. **值**：交给 [`Presets::apply_values`] 一次写进 `machineVariants`，本机一份文件都不写。
    //!
    //! 所以每条判据最后都要落在其中之一上。值在 TOML 里长什么样属于
    //! `presets::registry` 那一边的测试，这里只问「写进去的值还是不是它」。
    //!
    //! # 曾经在这里、现在整体没有对象了的几条
    //!
    //! | 旧测试 | 为什么不在这了 |
    //! |---|---|
    //! | `an_unrenamed_version_does_not_pin_its_name` | `VersionFile` 已删。名字唯一来源是 `presets/machines/*.toml` 的 `[[versions]].name`，由「机型与版本」页即时写，这里一个字节都不存 |
    //! | `a_new_version_shows_up_on_the_tree_after_saving` | `Patch::NewVersion` 没了。加版本走那一页，`remap` 因此永远是空 map |
    //! | `moving_rewrites_the_file_and_remaps_the_uid` | `Patch::MoveVersion` 没了，没有会改写 uid 的写路径 |
    //! | `purging_an_orphan_version_moves_the_file_to_the_trash` | 「有文件但清单里没这一版」不再可能：`workbench/` 里根本没有那种文件 |
    //! | `purging_a_declared_version_is_refused_and_points_at_archiving` | `Patch::PurgeVersion` 与归档都没了，本模块没有能拒绝的东西 |
    //! | `a_colliding_new_version_is_refused_not_overwritten` | 新建不在了，「撞已有 id」也不在了 |
    //!
    //! # 数字一律用 `as_f64` 比
    //!
    //! 值越过一次盘就落成 `valueType` 指定的表示（`float` 字段里 `42` 会写回
    //! `42.0`，见 `presets::registry::to_toml_value`）。所以从盘上读回来的数
    //! 用 `as_f64` 对，不拿整数字面量去比 —— 那是在赌表示，不是在验值。
    use super::*;
    use crate::workbench::domain::patch::{apply, Patch, Visibility};
    use crate::workbench::domain::testkit::Fixture;
    use crate::workbench::domain::Level;

    fn store() -> (tempfile::TempDir, Store) {
        let d = tempfile::tempdir().unwrap();
        let s = Store::at(d.path());
        s.bootstrap().unwrap();
        (d, s)
    }

    /// 干净仓库：一个文件都没有。**`Committed.versions` 仍然要有清单里那四版** ——
    /// 它是"清单现在说什么"，不是"我们写过文件的那些版本"
    #[test]
    fn a_clean_repo_loads_as_empty_not_as_an_error() {
        let f = Fixture::load();
        let (_d, s) = store();
        let loaded = load(&s, &f.presets).unwrap();

        // 四版都在，而且身份来自机型文件
        assert_eq!(loaded.committed.versions.len(), 4, "四个版本都在");
        let std = &loaded.committed.versions["A1/STANDARD"];
        assert_eq!(std.machine_id, "A1");
        assert_eq!(std.version_id, "STANDARD");
        assert_eq!(std.name, "标准版", "显示名来自 [[versions]] 的 name");
        assert_eq!(std.tag.as_deref(), Some("推荐"));
        assert_eq!(
            loaded.committed.versions["A2L/STANDARD"].tag, None,
            "空 tag 是 None，不是空串 —— 界面上要能分出「没配角标」"
        );
        assert_eq!(loaded.committed.versions["P1S/LITE"].name, "精简版");

        // 清单本身也要出来，而且照机型文件的顺序
        assert_eq!(
            loaded
                .committed
                .catalog
                .iter()
                .map(|m| m.id.as_str())
                .collect::<Vec<_>>(),
            ["A1", "A2L", "P1S"]
        );
        assert_eq!(
            loaded.committed.catalog[0].version_ids,
            ["STANDARD", "FAST"]
        );

        // 值一个都不在这里：干净仓库连一个文件都不该被读出来
        assert!(s
            .read_doc::<serde_json::Value>(DELIVERY_REL, "菜单与套餐")
            .unwrap()
            .is_none());
        assert!(s
            .read_doc::<serde_json::Value>(BUILT_REL, "生成记录")
            .unwrap()
            .is_none());
        assert!(loaded.notices.is_empty());
        assert!(read_draft(&s).unwrap().is_clean());
    }

    /// 保存 → 重新读：**值落在 `machineVariants` 里，身份仍然来自机型文件**
    #[test]
    fn saving_then_loading_round_trips() {
        // 要写 presets，临时目录得活过那次写，所以整份交出来（见 `Fixture::into_parts`）
        let (_dir, _up, mut presets) = Fixture::load().into_parts();
        let (_d, s) = store();
        let c = load(&s, &presets).unwrap().committed;
        let mut draft = Draft::default();

        apply(
            &mut draft,
            &c,
            &presets.registry,
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
            ],
        )
        .unwrap();
        write_draft(&s, &draft).unwrap();

        let out = save(&s, &mut presets, &c, &draft).unwrap();
        assert!(out.remap.is_empty(), "保存不再改 uid");

        // 值的真源现在是 registry，所以要**重读盘**再看；看内存等于没看
        let fresh = Presets::load_from(presets.root()).expect("保存之后还得读得通");
        let table = &fresh
            .registry
            .param("wiping.child")
            .unwrap()
            .machine_variants;
        assert_eq!(table["A1"].as_f64(), Some(42.0), "机型基底没写进去");
        let table = &fresh
            .registry
            .param("toolhead.offset.x")
            .unwrap()
            .machine_variants;
        assert_eq!(
            table["A1:STANDARD"].as_f64(),
            Some(7.0),
            "版本覆盖没写进去（owner 用的是冒号，不是斜杠）"
        );
        assert_eq!(
            table["A1:FAST"].as_f64(),
            Some(-0.7),
            "不该顺手动隔壁那一版"
        );

        // 名字只有机型文件一个来源 —— 这里不存第二个副本
        let again = load(&s, &fresh).unwrap();
        assert_eq!(again.committed.versions["A1/STANDARD"].name, "标准版");
        assert!(again.notices.is_empty());
        assert!(read_draft(&s).unwrap().is_clean(), "草稿要被清掉");
    }

    /// **上提过的那几条版本键要跟着一起写**（Task 12.3 那条回归）。
    ///
    /// 归并会把「每个版本都写了同一个值」当成机型基底显示。P1S 只有 LITE 一版，
    /// 于是 `toolhead.offset.x` 在界面上是"机型基底"，在 `machineVariants` 里却是
    /// `P1S:LITE` 那一条。只写裸键 `P1S` 会被那条更具体的版本键盖回去 ——
    /// **用户点了保存，值又变回去了，而且全程没有任何一步报错**。
    #[test]
    fn a_machine_level_edit_rewrites_the_promoted_version_keys() {
        let (_dir, _up, mut presets) = Fixture::load().into_parts();
        let (_d, s) = store();
        let c = load(&s, &presets).unwrap().committed;
        let mut draft = Draft::default();
        apply(
            &mut draft,
            &c,
            &presets.registry,
            &[Patch::SetValue {
                level: Level::Machine,
                owner: "P1S".to_owned(),
                key: "toolhead.offset.x".to_owned(),
                value: Some(serde_json::json!(-30)),
            }],
        )
        .unwrap();
        save(&s, &mut presets, &c, &draft).unwrap();

        let fresh = Presets::load_from(presets.root()).expect("重读");
        let table = &fresh
            .registry
            .param("toolhead.offset.x")
            .unwrap()
            .machine_variants;
        assert_eq!(table["P1S"].as_f64(), Some(-30.0), "裸键本身也要写");
        assert_eq!(
            table["P1S:LITE"].as_f64(),
            Some(-30.0),
            "上提过的那条版本键还钉着旧值，这一次改就会被它盖回去"
        );
        // 别的机型一个字节都不许动
        assert_eq!(table["A1:STANDARD"].as_f64(), Some(-1.0));
        assert_eq!(table["A1:FAST"].as_f64(), Some(-0.7));
    }

    /// 版本层的一条改动**只写自己那一个键**（上一条的另一半）。
    ///
    /// 直觉上「版本覆盖了，机型基底那条可以删了」，而删掉裸键会让**别的版本**
    /// 失去它们继承来的那个值 —— A1/FAST 没写这一项，它读的就是 `A1` 那条。
    #[test]
    fn a_version_level_edit_touches_only_its_own_key() {
        let (_dir, _up, mut presets) = Fixture::load().into_parts();
        let (_d, s) = store();
        // 先手工写一个机型基底（**不经过草稿**），让下面那条版本改动成为唯一的改动来源
        presets
            .set_variant("wiping.child", "A1", &serde_json::json!(3))
            .unwrap();

        let c = load(&s, &presets).unwrap().committed;
        let mut draft = Draft::default();
        apply(
            &mut draft,
            &c,
            &presets.registry,
            &[Patch::SetValue {
                level: Level::Version,
                owner: "A1/STANDARD".to_owned(),
                key: "wiping.child".to_owned(),
                value: Some(serde_json::json!(9)),
            }],
        )
        .unwrap();
        save(&s, &mut presets, &c, &draft).unwrap();

        let fresh = Presets::load_from(presets.root()).expect("重读");
        let table = &fresh
            .registry
            .param("wiping.child")
            .unwrap()
            .machine_variants;
        assert_eq!(table["A1:STANDARD"].as_f64(), Some(9.0));
        assert_eq!(
            table["A1"].as_f64(),
            Some(3.0),
            "裸键要留着：A1/FAST 还靠它继承"
        );
        assert!(
            !table.contains_key("A1:FAST"),
            "不该替没被改动的版本写一个键：{table:?}"
        );
    }

    /// `value: None` = **删键 = 挂回继承**，不是写空值。
    ///
    /// 这个 `Option` 要原样从 `plan_value_edits` 传到 `apply_values` ——
    /// 那里按 `Option` 的两个分支分岔，少一个分支就会把「继承」写成某个具体值。
    #[test]
    fn clearing_a_value_drops_the_key_not_sets_it_empty() {
        let (_dir, _up, mut presets) = Fixture::load().into_parts();
        let (_d, s) = store();
        presets
            .set_variant("wiping.child", "A1", &serde_json::json!(3))
            .unwrap();
        presets
            .set_variant("wiping.child", "A1:STANDARD", &serde_json::json!(5))
            .unwrap();

        let c = load(&s, &presets).unwrap().committed;
        let mut draft = Draft::default();
        apply(
            &mut draft,
            &c,
            &presets.registry,
            &[Patch::SetValue {
                level: Level::Machine,
                owner: "A1".to_owned(),
                key: "wiping.child".to_owned(),
                value: None,
            }],
        )
        .unwrap();
        save(&s, &mut presets, &c, &draft).unwrap();

        let fresh = Presets::load_from(presets.root()).expect("重读");
        let table = &fresh
            .registry
            .param("wiping.child")
            .unwrap()
            .machine_variants;
        assert!(
            !table.contains_key("A1"),
            "清掉的是整行，不是写一个空值：{table:?}"
        );
        assert_eq!(
            table["A1:STANDARD"].as_f64(),
            Some(5.0),
            "同一张表里别的键一个都不许少"
        );
    }

    /// 值以外那两样（菜单可见性、生成记录）**还在 `workbench/*.json`**，
    /// 而且写出的是「已落盘 + 草稿」合并之后的结果
    #[test]
    fn visibility_and_built_records_round_trip_through_workbench_files() {
        let (_dir, _up, mut presets) = Fixture::load().into_parts();
        let (_d, s) = store();
        let c = load(&s, &presets).unwrap().committed;
        let mut draft = Draft::default();
        apply(
            &mut draft,
            &c,
            &presets.registry,
            &[
                Patch::SetVisibility {
                    file_id: "a1_mkp_standard".to_owned(),
                    visibility: Visibility::ArchiveOnly,
                },
                Patch::MarkBuilt {
                    uids: vec!["A1/STANDARD".to_owned()],
                    stamp: "2026-01-01T00:00:00Z".to_owned(),
                    fingerprints: BTreeMap::from([("A1/STANDARD".to_owned(), "fp-abc".to_owned())]),
                },
            ],
        )
        .unwrap();
        save(&s, &mut presets, &c, &draft).unwrap();

        let again = load(&s, &presets).unwrap().committed;
        assert_eq!(
            again.visibility["a1_mkp_standard"],
            Visibility::ArchiveOnly,
            "菜单可见性要走 delivery.json 回来"
        );
        assert_eq!(again.built["A1/STANDARD"].fingerprint, "fp-abc");
        assert_eq!(again.built["A1/STANDARD"].stamp, "2026-01-01T00:00:00Z");
    }

    /// 干净草稿**不留空文件** —— 留着的话「有没有未保存改动」在文件系统上看不出来
    #[test]
    fn a_clean_draft_leaves_no_file() {
        let f = Fixture::load();
        let (_d, s) = store();
        let c = load(&s, &f.presets).unwrap().committed;
        let mut draft = Draft::default();
        apply(
            &mut draft,
            &c,
            &f.presets.registry,
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

    /// 更新版本的工作台写的文件**要拒绝**，不能硬着头皮解析成半份。
    ///
    /// `workbench/` 里现在只剩 delivery / built 两份，**两份各自查一次**才算那条闸门真的在
    #[test]
    fn a_file_from_a_newer_workbench_is_refused() {
        let f = Fixture::load();
        let (_d, s) = store();

        s.write_doc(
            DELIVERY_REL,
            &serde_json::json!({ "v": 99, "visibility": {} }),
        )
        .unwrap();
        let e = load(&s, &f.presets).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::Corrupted);
        assert!(e.detail.unwrap_or_default().contains("99"));

        s.remove(DELIVERY_REL).unwrap();
        s.write_doc(BUILT_REL, &serde_json::json!({ "v": 99, "records": {} }))
            .unwrap();
        let e = load(&s, &f.presets).unwrap_err();
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
