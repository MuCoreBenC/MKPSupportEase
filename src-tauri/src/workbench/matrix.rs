//! 参数矩阵与批量编辑。
//!
//! 矩阵是"横着看"：一行一个字段，一列一个版本。配方编辑页是"竖着看"单个版本。
//! 两种视角都需要 —— 调一个版本用竖的，比较/统一多个版本用横的。
//!
//! 批量编辑只有一条规矩，但它是这一整块的全部意义：
//!
//! > **改基底只改基底；改版本值则创建/更新那个版本的覆盖。**
//! > 绝不因为你勾了某些版本，就把它们原有的继承关系顺手打破。
//!
//! 这条在类型上兑现：[`BulkTarget`] 是枚举，一次请求里的每个目标各自说明自己是
//! "某个版本的覆盖"还是"某个机型的基底"。没有"选了版本却写到基底"这条路可走。
//!
//! 还有一条：**没有"一键应用到所有机型"**。跨机型批量走同一条显式勾列的路 ——
//! 隐式的全局应用是那种"当时很方便、三天后查不出谁改的"操作。

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::AppError;
use crate::workbench::model::{FieldDef, Registry};
use crate::workbench::resolve::{self, Origin};
use crate::workbench::store::Store;

/// 一列 = 一个版本
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixColumn {
    pub machine_id: String,
    pub version_id: String,
    pub machine_name: String,
    pub version_name: String,
    /// 有效配方为空 = 未配置。列头要能看出来，否则一整列空白会被当成"读失败"
    pub unconfigured: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixCell {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    /// `None` = 这个字段在该机型上不适用
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<Origin>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixRow {
    pub key: String,
    pub label: String,
    pub section: String,
    pub unit: Option<String>,
    /// 与 [`Matrix::columns`] 一一对应，顺序相同
    pub cells: Vec<MatrixCell>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Matrix {
    pub columns: Vec<MatrixColumn>,
    pub rows: Vec<MatrixRow>,
}

/// 读矩阵。`machine_filter` 为空表示全部机型
pub fn build(store: &Store, machine_filter: &[String]) -> Result<Matrix, AppError> {
    let reg = store.registry()?;
    let fb = store.fallback()?;

    let mut columns = Vec::new();
    // 每列的有效配方先算好，后面按字段取 —— 否则会对每个字段重复解析一遍
    let mut per_column = Vec::new();

    for mid in store.machine_ids()? {
        if !machine_filter.is_empty() && !machine_filter.iter().any(|m| m == &mid) {
            continue;
        }
        let Ok(machine) = store.machine(&mid) else {
            continue;
        };
        for vid in store.version_ids(&mid)? {
            let Ok(version) = store.version(&mid, &vid) else {
                continue;
            };
            let eff = resolve::resolve(&reg, &fb, &machine, &version);
            columns.push(MatrixColumn {
                machine_id: mid.clone(),
                version_id: vid.clone(),
                machine_name: machine.display_name.clone(),
                version_name: version.display_name.clone(),
                unconfigured: eff.is_unconfigured(),
            });
            per_column.push(eff);
        }
    }

    let mut rows = Vec::new();
    for f in &reg.fields {
        let cells = per_column
            .iter()
            .map(|eff| match eff.values.get(&f.key) {
                Some(v) => MatrixCell {
                    value: Some(v.value.clone()),
                    origin: Some(v.origin),
                },
                None => MatrixCell {
                    value: None,
                    origin: None,
                },
            })
            .collect();
        rows.push(MatrixRow {
            key: f.key.clone(),
            label: f.label.clone(),
            section: f.section.clone(),
            unit: f.unit.clone(),
            cells,
        });
    }
    // 按 order 排，与配方编辑页一致
    rows.sort_by(|a, b| order_of(&reg, &a.key).total_cmp(&order_of(&reg, &b.key)));

    Ok(Matrix { columns, rows })
}

fn order_of(reg: &Registry, key: &str) -> f64 {
    reg.field(key).map(|f| f.order).unwrap_or(f64::MAX)
}

/* ---------- 批量编辑 ---------- */

/// 批量的目标。两个变体是这一整块的安全边界：
/// 选的是版本就只写覆盖，选的是基底就只写基底
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum BulkTarget {
    VersionOverride {
        machine_id: String,
        version_id: String,
    },
    MachineBase {
        machine_id: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BulkKind {
    /// 这个版本以前继承基底，改完会**多出一条覆盖**
    CreatesOverride,
    /// 以前就有覆盖，只是换个值
    UpdatesOverride,
    UpdatesBase,
    /// 新值和现值一样，写了也白写
    NoChange,
    /// 该机型基底里没有这个字段 → 写进去只会变成一条不适用的覆盖
    NotApplicable,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkEffect {
    pub target: BulkTarget,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<Value>,
    pub after: Value,
    pub kind: BulkKind,
}

/// 预览影响范围。**界面必须先显示它再让人确认** —— doc §9.7
pub fn preview_bulk(
    store: &Store,
    field_key: &str,
    value: &Value,
    targets: &[BulkTarget],
) -> Result<Vec<BulkEffect>, AppError> {
    let reg = store.registry()?;
    let field: &FieldDef = reg
        .field(field_key)
        .ok_or_else(|| AppError::invalid_argument(format!("字段定义里没有 {field_key}")))?;

    let mut out = Vec::new();
    for t in targets {
        match t {
            BulkTarget::MachineBase { machine_id } => {
                let m = store.machine(machine_id)?;
                let before = m.base.get(field_key).cloned();
                out.push(BulkEffect {
                    target: t.clone(),
                    label: format!("{} 的基底", m.display_name),
                    kind: if before.as_ref() == Some(value) {
                        BulkKind::NoChange
                    } else {
                        BulkKind::UpdatesBase
                    },
                    before,
                    after: value.clone(),
                });
            }
            BulkTarget::VersionOverride {
                machine_id,
                version_id,
            } => {
                let m = store.machine(machine_id)?;
                let v = store.version(machine_id, version_id)?;
                let existing = v.overrides.get(field_key).cloned();
                let base = m.base.get(field_key).cloned();
                let effective = existing.clone().or_else(|| base.clone());

                let kind = if base.is_none() {
                    BulkKind::NotApplicable
                } else if effective.as_ref() == Some(value) && existing.is_some() {
                    BulkKind::NoChange
                } else if existing.is_some() {
                    BulkKind::UpdatesOverride
                } else {
                    BulkKind::CreatesOverride
                };

                out.push(BulkEffect {
                    target: t.clone(),
                    label: format!("{} / {}", m.display_name, v.display_name),
                    kind,
                    before: effective,
                    after: value.clone(),
                });
            }
        }
    }

    // 字段本身的区间校验放在这里做一次，省得每个目标各报一遍
    if let Some(min) = field.min {
        if let Some(n) = value.as_f64() {
            if n < min {
                return Err(AppError::invalid_argument(format!(
                    "{} 的下限是 {min}，给的是 {n}",
                    field.label
                )));
            }
        }
    }
    if let Some(max) = field.max {
        if let Some(n) = value.as_f64() {
            if n > max {
                return Err(AppError::invalid_argument(format!(
                    "{} 的上限是 {max}，给的是 {n}",
                    field.label
                )));
            }
        }
    }

    Ok(out)
}

/// 应用。跳过 `NoChange` 与 `NotApplicable` —— 前者是白写，后者会造出一条
/// 立刻变成 orphan 的覆盖
pub fn apply_bulk(
    store: &Store,
    field_key: &str,
    value: &Value,
    targets: &[BulkTarget],
) -> Result<Vec<BulkEffect>, AppError> {
    let effects = preview_bulk(store, field_key, value, targets)?;

    for e in &effects {
        if matches!(e.kind, BulkKind::NoChange | BulkKind::NotApplicable) {
            continue;
        }
        match &e.target {
            BulkTarget::MachineBase { machine_id } => {
                let mut m = store.machine(machine_id)?;
                m.base.insert(field_key.to_owned(), value.clone());
                store.save_machine(&m)?;
            }
            BulkTarget::VersionOverride {
                machine_id,
                version_id,
            } => {
                let mut v = store.version(machine_id, version_id)?;
                v.overrides.insert(field_key.to_owned(), value.clone());
                store.save_version(&v)?;
            }
        }
    }

    Ok(effects)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::model::{BbsBinding, Machine, Version};
    use crate::workbench::store::empty_params;

    fn setup() -> (tempfile::TempDir, Store) {
        let d = tempfile::tempdir().unwrap();
        let s = Store::at(d.path());
        s.bootstrap().unwrap();

        let mut base = empty_params();
        base.insert("toolhead.z_offset".into(), serde_json::json!(0.10));
        base.insert("motion.travel_speed".into(), serde_json::json!(300.0));
        s.save_machine(&Machine {
            id: "A1".into(),
            display_name: "A1".into(),
            base,
            default_bbs: vec![],
        })
        .unwrap();

        // std：全继承
        s.save_version(&Version {
            id: "std".into(),
            display_name: "标准版".into(),
            machine_id: "A1".into(),
            overrides: empty_params(),
            bbs: BbsBinding::Inherit,
        })
        .unwrap();

        // fast：覆盖 z_offset
        let mut ov = empty_params();
        ov.insert("toolhead.z_offset".into(), serde_json::json!(0.15));
        s.save_version(&Version {
            id: "fast".into(),
            display_name: "快拆版".into(),
            machine_id: "A1".into(),
            overrides: ov,
            bbs: BbsBinding::Inherit,
        })
        .unwrap();

        (d, s)
    }

    #[test]
    fn matrix_has_one_column_per_version_and_marks_origins() {
        let (_d, s) = setup();
        let m = build(&s, &[]).unwrap();
        assert_eq!(m.columns.len(), 2);

        let z = m.rows.iter().find(|r| r.key == "toolhead.z_offset").unwrap();
        let col = |vid: &str| m.columns.iter().position(|c| c.version_id == vid).unwrap();

        assert_eq!(z.cells[col("std")].origin, Some(Origin::Base));
        assert_eq!(z.cells[col("fast")].origin, Some(Origin::Override));
        assert_eq!(z.cells[col("fast")].value, Some(serde_json::json!(0.15)));
    }

    /// 机型基底里没有的字段 → 单元格是空的，且明确标成"不适用"（origin 为 None）
    #[test]
    fn inapplicable_cells_are_empty() {
        let (_d, s) = setup();
        let m = build(&s, &[]).unwrap();
        let row = m.rows.iter().find(|r| r.key == "support.line_width").unwrap();
        assert!(row.cells.iter().all(|c| c.value.is_none() && c.origin.is_none()));
    }

    /// 改版本值 = 创建/更新覆盖，**基底一个字节都不动**
    #[test]
    fn bulk_on_versions_never_touches_base() {
        let (_d, s) = setup();
        let before_base = s.machine("A1").unwrap().base.clone();

        let targets = vec![
            BulkTarget::VersionOverride {
                machine_id: "A1".into(),
                version_id: "std".into(),
            },
            BulkTarget::VersionOverride {
                machine_id: "A1".into(),
                version_id: "fast".into(),
            },
        ];
        let effects = apply_bulk(&s, "toolhead.z_offset", &serde_json::json!(0.2), &targets).unwrap();

        // std 以前是继承 → 现在多了一条覆盖；fast 以前就有覆盖 → 只是换值
        let kind_of = |vid: &str| {
            effects
                .iter()
                .find(|e| {
                    matches!(&e.target, BulkTarget::VersionOverride { version_id, .. } if version_id == vid)
                })
                .unwrap()
                .kind
        };
        assert_eq!(kind_of("std"), BulkKind::CreatesOverride);
        assert_eq!(kind_of("fast"), BulkKind::UpdatesOverride);

        assert_eq!(s.machine("A1").unwrap().base, before_base, "基底被动了");
        assert_eq!(
            s.version("A1", "std").unwrap().overrides["toolhead.z_offset"],
            serde_json::json!(0.2)
        );
    }

    /// 改基底 = 只动基底，**不给任何版本造覆盖**（继承关系不被打破）
    #[test]
    fn bulk_on_base_creates_no_overrides() {
        let (_d, s) = setup();
        let targets = vec![BulkTarget::MachineBase {
            machine_id: "A1".into(),
        }];
        apply_bulk(&s, "toolhead.z_offset", &serde_json::json!(0.3), &targets).unwrap();

        assert_eq!(
            s.machine("A1").unwrap().base["toolhead.z_offset"],
            serde_json::json!(0.3)
        );
        assert!(
            s.version("A1", "std").unwrap().overrides.is_empty(),
            "改基底却给版本造了覆盖"
        );
        // fast 原来的覆盖不该被基底变化擦掉
        assert_eq!(
            s.version("A1", "fast").unwrap().overrides["toolhead.z_offset"],
            serde_json::json!(0.15)
        );
    }

    /// 预览只报，不写
    #[test]
    fn preview_does_not_write() {
        let (_d, s) = setup();
        let before = s.version("A1", "std").unwrap().overrides.clone();
        preview_bulk(
            &s,
            "toolhead.z_offset",
            &serde_json::json!(0.9),
            &[BulkTarget::VersionOverride {
                machine_id: "A1".into(),
                version_id: "std".into(),
            }],
        )
        .unwrap();
        assert_eq!(s.version("A1", "std").unwrap().overrides, before);
    }

    /// 值越界当场拒绝，不写一半
    #[test]
    fn out_of_range_value_is_refused() {
        let (_d, s) = setup();
        let e = apply_bulk(
            &s,
            "toolhead.z_offset",
            &serde_json::json!(99.0), // 上限 2
            &[BulkTarget::MachineBase {
                machine_id: "A1".into(),
            }],
        )
        .unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::InvalidArgument);
        assert_eq!(
            s.machine("A1").unwrap().base["toolhead.z_offset"],
            serde_json::json!(0.10),
            "越界值被写进去了"
        );
    }

    /// 不适用的目标被跳过，不会造出一条立刻变 orphan 的覆盖
    #[test]
    fn inapplicable_target_is_skipped() {
        let (_d, s) = setup();
        let effects = apply_bulk(
            &s,
            "support.line_width",
            &serde_json::json!(0.5),
            &[BulkTarget::VersionOverride {
                machine_id: "A1".into(),
                version_id: "std".into(),
            }],
        )
        .unwrap();
        assert_eq!(effects[0].kind, BulkKind::NotApplicable);
        assert!(s.version("A1", "std").unwrap().overrides.is_empty());
    }

    #[test]
    fn unknown_field_is_refused() {
        let (_d, s) = setup();
        assert!(preview_bulk(&s, "toolhead.mustard", &serde_json::json!(1), &[]).is_err());
    }
}
