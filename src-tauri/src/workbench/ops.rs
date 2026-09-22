//! 版本的增、克隆、改名、换机型、删。
//!
//! 每个操作都有一条共同的规矩：**不静默覆盖、不静默丢数据**。
//! - 重名 → 当场拒绝并要求改名，**不自动加后缀**（自动加后缀会让你以为改成功了）
//! - 换机型 → 覆盖值一个不丢，先给预览再动手
//! - 删 → 进回收站，同时把它从菜单与套餐里摘掉（doc §9.7）
//!
//! 换机型的预览是本文件最该看的部分：它区分"值变了"和"出错了"。
//! 继承值因为基底不同而变化是**正常的**，界面用橙/黄标，不用红色 —— 用红色会让人
//! 以为操作有问题，从而不敢做一个完全合理的操作。

use serde::Serialize;
use serde_json::Value;

use crate::error::AppError;
use crate::workbench::catalog;
use crate::workbench::model::{BbsBinding, Machine, Version};
use crate::workbench::store::{empty_params, validate_id, Store};

/// 换机型后某个字段会怎样
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MoveKind {
    /// 继承值变了。**这不是错误**
    InheritedChanged,
    /// 本版覆盖，原样保留
    OverrideKept,
    /// 新机型基底里没有这个字段 → 值留着但不进 TOML
    BecomesInapplicable,
    /// 新机型基底里才有这个字段 → 这个版本从此多一个可用字段
    BecomesApplicable,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveChange {
    pub key: String,
    pub label: String,
    pub kind: MoveKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MovePreview {
    pub from_machine: String,
    pub to_machine: String,
    /// 只列**会变的**字段。没变的不列 —— 一屏全是"没变"会把真正的变化埋掉
    pub changes: Vec<MoveChange>,
    /// 目标机型下是否已有同名版本。为 true 时不能移
    pub name_taken: bool,
}

/// 新建空版本。**不预填任何参数** —— 新建出来就是"未配置"，那是真实状态
pub fn create_version(
    store: &Store,
    machine_id: &str,
    id: &str,
    display_name: &str,
) -> Result<Version, AppError> {
    validate_id(id, "版本 id")?;
    // 机型必须先存在：允许往不存在的机型下建版本，等于默许造出读不出来的树
    let _ = store.machine(machine_id)?;
    ensure_name_free(store, machine_id, id)?;

    let v = Version {
        id: id.to_owned(),
        display_name: display_name.trim().to_owned(),
        machine_id: machine_id.to_owned(),
        overrides: empty_params(),
        bbs: BbsBinding::Inherit,
    };
    store.save_version(&v)?;
    Ok(v)
}

/// 新建机型。基底为空 —— 同上，不替你猜参数
pub fn create_machine(store: &Store, id: &str, display_name: &str) -> Result<Machine, AppError> {
    validate_id(id, "机型 id")?;
    if store.machine(id).is_ok() {
        return Err(AppError::invalid_argument(format!("机型 {id} 已经存在")));
    }
    let m = Machine {
        id: id.to_owned(),
        display_name: display_name.trim().to_owned(),
        base: empty_params(),
        default_bbs: Vec::new(),
    };
    store.save_machine(&m)?;
    Ok(m)
}

/// 克隆。覆盖值与 BBS 绑定一起复制 —— 克隆出来的东西应该和原件一模一样，
/// 只有 id 与显示名不同
pub fn clone_version(
    store: &Store,
    machine_id: &str,
    version_id: &str,
    new_id: &str,
    new_display_name: &str,
) -> Result<Version, AppError> {
    validate_id(new_id, "版本 id")?;
    let src = store.version(machine_id, version_id)?;
    ensure_name_free(store, machine_id, new_id)?;

    let v = Version {
        id: new_id.to_owned(),
        display_name: new_display_name.trim().to_owned(),
        machine_id: machine_id.to_owned(),
        overrides: src.overrides,
        bbs: src.bbs,
    };
    store.save_version(&v)?;
    Ok(v)
}

/// 克隆时的默认显示名：`XX 副本`。
///
/// 弹框预填它，可直接确认也可改 —— doc §9.7。**重名不自动加后缀**，
/// 所以这个默认值撞了就得你自己改，界面会当场说
pub fn suggest_copy_name(display_name: &str) -> String {
    format!("{display_name} 副本")
}

/// 改名。显示名随便改；id 变了要搬文件，并把菜单里的来源引用跟着改
pub fn rename_version(
    store: &Store,
    machine_id: &str,
    version_id: &str,
    new_id: &str,
    new_display_name: &str,
) -> Result<Version, AppError> {
    validate_id(new_id, "版本 id")?;
    let mut v = store.version(machine_id, version_id)?;
    v.display_name = new_display_name.trim().to_owned();

    if new_id == version_id {
        store.save_version(&v)?;
        return Ok(v);
    }

    ensure_name_free(store, machine_id, new_id)?;
    v.id = new_id.to_owned();
    // 先写新的再删旧的：中途失败最坏是多一份，不会两头都没有
    store.save_version(&v)?;
    store.remove_version_file(machine_id, version_id)?;
    store.rename_snapshot(machine_id, version_id, machine_id, new_id)?;
    store.clear_draft(machine_id, version_id)?;
    catalog::relink_version(store, machine_id, version_id, machine_id, new_id)?;
    Ok(v)
}

/// 换机型的预览。**先看再动手** —— doc §9.6
pub fn preview_move(
    store: &Store,
    machine_id: &str,
    version_id: &str,
    to_machine_id: &str,
) -> Result<MovePreview, AppError> {
    let reg = store.registry()?;
    let v = store.version(machine_id, version_id)?;
    let from = store.machine(machine_id)?;
    let to = store.machine(to_machine_id)?;

    let mut changes = Vec::new();
    for f in &reg.fields {
        let overridden = v.overrides.contains_key(&f.key);
        let in_from = from.base.get(&f.key);
        let in_to = to.base.get(&f.key);

        match (overridden, in_from, in_to) {
            // 本版覆盖过：值不动。只有在新机型仍适用时才算"保留"
            (true, _, Some(_)) => changes.push(MoveChange {
                key: f.key.clone(),
                label: f.label.clone(),
                kind: MoveKind::OverrideKept,
                from: v.overrides.get(&f.key).cloned(),
                to: v.overrides.get(&f.key).cloned(),
            }),
            (true, _, None) => changes.push(MoveChange {
                key: f.key.clone(),
                label: f.label.clone(),
                kind: MoveKind::BecomesInapplicable,
                from: v.overrides.get(&f.key).cloned(),
                to: None,
            }),
            (false, Some(a), Some(b)) if a != b => changes.push(MoveChange {
                key: f.key.clone(),
                label: f.label.clone(),
                kind: MoveKind::InheritedChanged,
                from: Some(a.clone()),
                to: Some(b.clone()),
            }),
            (false, Some(a), None) => changes.push(MoveChange {
                key: f.key.clone(),
                label: f.label.clone(),
                kind: MoveKind::BecomesInapplicable,
                from: Some(a.clone()),
                to: None,
            }),
            (false, None, Some(b)) => changes.push(MoveChange {
                key: f.key.clone(),
                label: f.label.clone(),
                kind: MoveKind::BecomesApplicable,
                from: None,
                to: Some(b.clone()),
            }),
            // 继承值一样、或者两边都没有 → 不是变化，不列
            _ => {}
        }
    }

    Ok(MovePreview {
        from_machine: from.display_name,
        to_machine: to.display_name,
        changes,
        name_taken: store.version(to_machine_id, version_id).is_ok(),
    })
}

/// 真的移。覆盖值原样带走 —— 一个都不丢，包括在新机型上暂时不适用的那些
pub fn move_version(
    store: &Store,
    machine_id: &str,
    version_id: &str,
    to_machine_id: &str,
) -> Result<Version, AppError> {
    if machine_id == to_machine_id {
        return Err(AppError::invalid_argument("目标机型和当前机型是同一个"));
    }
    let _ = store.machine(to_machine_id)?;
    let mut v = store.version(machine_id, version_id)?;
    ensure_name_free(store, to_machine_id, version_id)?;

    v.machine_id = to_machine_id.to_owned();
    store.save_version(&v)?;
    store.remove_version_file(machine_id, version_id)?;
    store.rename_snapshot(machine_id, version_id, to_machine_id, version_id)?;
    store.clear_draft(machine_id, version_id)?;
    catalog::relink_version(store, machine_id, version_id, to_machine_id, version_id)?;
    Ok(v)
}

/// 删 = 进回收站 + 从菜单与套餐里摘掉
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashResult {
    pub trashed_file: String,
    /// 被一并摘掉的 presetId。界面要据实说出改了什么，不能只说"已删除"
    pub detached_presets: Vec<String>,
}

pub fn trash_version(
    store: &Store,
    machine_id: &str,
    version_id: &str,
) -> Result<TrashResult, AppError> {
    let detached = catalog::detach_version(store, machine_id, version_id)?;
    let path = store.trash_version(machine_id, version_id)?;
    Ok(TrashResult {
        trashed_file: path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_owned(),
        detached_presets: detached,
    })
}

fn ensure_name_free(store: &Store, machine_id: &str, version_id: &str) -> Result<(), AppError> {
    if store.version(machine_id, version_id).is_ok() {
        return Err(AppError::invalid_argument(format!(
            "{machine_id} 下已经有一个叫 {version_id} 的版本，换个名字"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::model::builtin_registry;

    fn setup() -> (tempfile::TempDir, Store) {
        let d = tempfile::tempdir().unwrap();
        let s = Store::at(d.path());
        s.bootstrap().unwrap();

        let mut a1 = empty_params();
        a1.insert("toolhead.offset_x".into(), serde_json::json!(168.0));
        a1.insert("toolhead.offset_y".into(), serde_json::json!(-2.0));
        a1.insert("toolhead.z_offset".into(), serde_json::json!(0.10));
        s.save_machine(&Machine {
            id: "A1".into(),
            display_name: "A1".into(),
            base: a1,
            default_bbs: vec![],
        })
        .unwrap();

        let mut p1 = empty_params();
        p1.insert("toolhead.offset_x".into(), serde_json::json!(175.0));
        p1.insert("toolhead.offset_y".into(), serde_json::json!(-1.0));
        p1.insert("toolhead.z_offset".into(), serde_json::json!(0.10));
        p1.insert("motion.accel".into(), serde_json::json!(8000.0));
        s.save_machine(&Machine {
            id: "P1".into(),
            display_name: "P1".into(),
            base: p1,
            default_bbs: vec![],
        })
        .unwrap();

        let mut ov = empty_params();
        ov.insert("toolhead.z_offset".into(), serde_json::json!(0.15));
        s.save_version(&Version {
            id: "quickswap".into(),
            display_name: "快拆版".into(),
            machine_id: "A1".into(),
            overrides: ov,
            bbs: BbsBinding::Inherit,
        })
        .unwrap();

        (d, s)
    }

    #[test]
    fn clone_copies_overrides_and_rejects_duplicate_name() {
        let (_d, s) = setup();
        let c = clone_version(&s, "A1", "quickswap", "quickswap2", "快拆版 副本").unwrap();
        assert_eq!(c.overrides["toolhead.z_offset"], serde_json::json!(0.15));

        // 重名当场拒绝，不自动加后缀
        let e = clone_version(&s, "A1", "quickswap", "quickswap2", "又一份").unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::InvalidArgument);
    }

    #[test]
    fn suggested_copy_name_is_prefilled_not_forced() {
        assert_eq!(suggest_copy_name("快拆版"), "快拆版 副本");
    }

    /// 预览只列会变的，而且分得清"值变了"和"不适用"
    #[test]
    fn move_preview_separates_change_kinds() {
        let (_d, s) = setup();
        let p = preview_move(&s, "A1", "quickswap", "P1").unwrap();

        let by = |k: &str| p.changes.iter().find(|c| c.key == k).cloned();

        // 继承值变了：168 → 175
        let x = by("toolhead.offset_x").expect("offset_x 应该被列出来");
        assert_eq!(x.kind, MoveKind::InheritedChanged);
        assert_eq!(x.from, Some(serde_json::json!(168.0)));
        assert_eq!(x.to, Some(serde_json::json!(175.0)));

        // 本版覆盖：保留
        let z = by("toolhead.z_offset").expect("z_offset 应该被列出来");
        assert_eq!(z.kind, MoveKind::OverrideKept);
        assert_eq!(z.to, Some(serde_json::json!(0.15)));

        // 新机型才有的字段
        let a = by("motion.accel").expect("accel 应该被列出来");
        assert_eq!(a.kind, MoveKind::BecomesApplicable);

        // 两边相同的字段不列（本例里没有这种，用"未出现"反证：
        // registry 有 10 个字段，变化项应当远少于它）
        assert!(p.changes.len() < builtin_registry().fields.len());
        assert!(!p.name_taken);
    }

    #[test]
    fn move_keeps_overrides_and_updates_machine_id() {
        let (_d, s) = setup();
        let moved = move_version(&s, "A1", "quickswap", "P1").unwrap();
        assert_eq!(moved.machine_id, "P1");
        assert_eq!(moved.overrides["toolhead.z_offset"], serde_json::json!(0.15));
        assert!(s.version("A1", "quickswap").is_err(), "原处还在");
        assert!(s.version("P1", "quickswap").is_ok());
    }

    #[test]
    fn move_to_same_machine_is_rejected() {
        let (_d, s) = setup();
        let e = move_version(&s, "A1", "quickswap", "A1").unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::InvalidArgument);
    }

    /// 目标机型下已有同名版本：预览要说，执行要拒
    #[test]
    fn move_into_taken_name_is_refused() {
        let (_d, s) = setup();
        create_version(&s, "P1", "quickswap", "同名").unwrap();

        let p = preview_move(&s, "A1", "quickswap", "P1").unwrap();
        assert!(p.name_taken);

        let e = move_version(&s, "A1", "quickswap", "P1").unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::InvalidArgument);
    }

    #[test]
    fn rename_moves_file_and_keeps_values() {
        let (_d, s) = setup();
        let v = rename_version(&s, "A1", "quickswap", "fast", "快拆").unwrap();
        assert_eq!(v.id, "fast");
        assert_eq!(v.display_name, "快拆");
        assert!(s.version("A1", "quickswap").is_err());
        assert_eq!(
            s.version("A1", "fast").unwrap().overrides["toolhead.z_offset"],
            serde_json::json!(0.15)
        );
    }

    /// 只改显示名不搬文件
    #[test]
    fn rename_display_only_keeps_id() {
        let (_d, s) = setup();
        let v = rename_version(&s, "A1", "quickswap", "quickswap", "快拆版 v2").unwrap();
        assert_eq!(v.id, "quickswap");
        assert_eq!(v.display_name, "快拆版 v2");
        assert!(s.version("A1", "quickswap").is_ok());
    }

    #[test]
    fn create_version_on_missing_machine_fails() {
        let (_d, s) = setup();
        assert!(create_version(&s, "NOPE", "std", "标准版").is_err());
    }

    #[test]
    fn new_version_is_unconfigured_not_prefilled() {
        let (_d, s) = setup();
        let v = create_version(&s, "A1", "std", "标准版").unwrap();
        assert!(v.overrides.is_empty(), "新建版本不该预填参数");
    }
}
