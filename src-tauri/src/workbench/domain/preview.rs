//! 只读推演：移动预览四组、批量影响范围（doc §8.4、tasks 7.7 / 7.8）。
//!
//! 这一层**不写任何东西**。它回答的是"我按下去会发生什么"，
//! 而这个问题必须在按下去之前有答案 —— 否则用户只能改完再看，
//! 而"改完再看"对批量来说等于没有预览（一次盖掉十几列）。
//!
//! # 移动为什么需要四组而不是一句话
//!
//! 把一个版本从 A1 搬到 P1S，有四件**同时**发生的事，混成一句话说不清：
//!
//! | 组 | 发生了什么 | 为什么要单独说 |
//! |---|---|---|
//! | 保留的覆盖 | 我们写在这一版上的项跟着走 | uid 稳定，所以它们不会丢（doc §3.5） |
//! | 继承值会变 | 没写过的项，继承的是新机型那一层 | **值真的变了**，而界面上那一格没有任何改动标记 |
//! | 目标多出来的 | 新机型有、老机型没有的字段 | 这一版会突然多出几项能改的 |
//! | 目标没有的 | 老机型有、新机型没有的字段 | 我们写在上面的值会变成**再也进不了产物**的孤儿 |
//!
//! 第二组和第四组是这个操作真正的风险，而它们都不会报错。
//!
//! # 批量：静默跳过两类目标
//!
//! 「不适用」与「被上级条件关着」的列**不出现在预览里、也不计入「N 列」**（doc §8.4）。
//! 理由是这两类目标上落值没有意义：前者根本没有这一项，后者的值现在不生效。
//! 但**跳过要能查** —— 所以 [`BulkPreview::skipped`] 把它们列出来并写明原因，
//! 而不是让"我勾了 6 列怎么只改了 4 列"变成一个谜。
//!
//! G-code **拒绝批量**：一段多行脚本被整体盖掉是不可逆的误操作。

use std::collections::BTreeSet;

use serde::Serialize;
use serde_json::Value;

use super::derive::{Book, ColRef};
use super::layer::{no_overrides, Layers, Level, Origin};
use super::visibility::{BlockedBy, Gate};
use super::wording as w;

/* ---------- 移动预览 ---------- */

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MovePreview {
    pub uid: String,
    pub from_machine: String,
    pub to_machine: String,
    /// 能不能移。目标机型不存在时为 false，并给出理由（doc §15：当没移）
    pub allowed: bool,
    pub blocked_reason: Option<String>,
    /// 我们写在这一版上的项，跟着走
    pub kept: Vec<String>,
    /// 没写过、但继承来的值会变的项
    pub inherited_changes: Vec<InheritedChange>,
    /// 目标机型多出来的字段
    pub gained: Vec<String>,
    /// 目标机型没有的字段。**我们写在上面的值会变成孤儿**
    pub lost: Vec<LostKey>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InheritedChange {
    pub key: String,
    pub label: String,
    pub before: String,
    pub after: String,
    pub before_origin: Origin,
    pub after_origin: Origin,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LostKey {
    pub key: String,
    pub label: String,
    /// 我们在这一版写过它吗。为真时这个值会变成再也进不了产物的孤儿
    pub had_own_value: bool,
}

/* ---------- 批量预览 ---------- */

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkPreview {
    pub key: String,
    pub label: String,
    /// 能不能批量。G-code 不行
    pub allowed: bool,
    pub blocked_reason: Option<String>,
    /// 会落到的列。**「N 列」数的是这个**
    pub effects: Vec<BulkEffect>,
    /// 静默跳过的列，附原因 —— 跳过要能查
    pub skipped: Vec<BulkSkip>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkEffect {
    /// 列的 key（机型 id 或版本 uid）
    pub col: String,
    pub machine: String,
    pub label: String,
    pub level: Level,
    pub before: String,
    pub after: String,
    pub kind: BulkKind,
}

/// 这一列落下去属于哪一类改动
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BulkKind {
    /// 这一层本来没写过 → 新脱钩一项
    Detaching,
    /// 这一层写过，值会被改
    Changing,
    /// 值和现在一样，落下去没有变化
    NoChange,
}

impl BulkKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Detaching => "新增覆盖",
            Self::Changing => "改值",
            Self::NoChange => "没有变化",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkSkip {
    pub col: String,
    pub machine: String,
    pub label: String,
    pub reason: String,
    /// 被条件关着时，卡在哪一项上（根在前）
    pub blocked: Vec<BlockedBy>,
}

impl Book<'_> {
    /// 把 `uid` 搬到 `to_machine_id` 会发生什么。**不改任何东西**
    pub fn preview_move(&self, uid: &str, to_machine_id: &str) -> Option<MovePreview> {
        let v = self.version(uid)?;
        let from = v.machine_id.clone();

        let to_exists = self.up.catalog.machine(to_machine_id).is_some();
        let mut out = MovePreview {
            uid: uid.to_owned(),
            from_machine: from.clone(),
            to_machine: to_machine_id.to_owned(),
            allowed: to_exists && from != to_machine_id,
            blocked_reason: None,
            kept: Vec::new(),
            inherited_changes: Vec::new(),
            gained: Vec::new(),
            lost: Vec::new(),
        };
        if !to_exists {
            // doc §15：当没移。让这个版本从树上整个消失比不动更糟
            out.blocked_reason = Some(format!("机型 {to_machine_id} 不存在，不会移动"));
            return Some(out);
        }
        if from == to_machine_id {
            out.blocked_reason = Some("已经在这台机型下了".to_owned());
            return Some(out);
        }

        let before = self.version_layers(uid)?;
        let after = self.layers_as_if_moved(uid, to_machine_id)?;

        let before_keys: BTreeSet<&str> = before.keys().into_iter().collect();
        let after_keys: BTreeSet<&str> = after.keys().into_iter().collect();

        for key in before_keys.intersection(&after_keys) {
            if before.has_own(Level::Version, key) {
                out.kept.push((*key).to_owned());
                // 自有的项跟着走，值不变，所以不进"继承会变"那一组
                continue;
            }
            let (Some(b), Some(a)) = (before.effective(key), after.effective(key)) else {
                continue;
            };
            if b.value == a.value {
                continue;
            }
            let Some(p) = self.up.registry.param(key) else {
                continue;
            };
            out.inherited_changes.push(InheritedChange {
                key: (*key).to_owned(),
                label: p.label.clone(),
                before: w::value_text(p, b.value),
                after: w::value_text(p, a.value),
                before_origin: b.origin,
                after_origin: a.origin,
            });
        }

        for key in after_keys.difference(&before_keys) {
            out.gained.push((*key).to_owned());
        }
        for key in before_keys.difference(&after_keys) {
            let Some(p) = self.up.registry.param(key) else {
                continue;
            };
            out.lost.push(LostKey {
                key: (*key).to_owned(),
                label: p.label.clone(),
                had_own_value: before.has_own(Level::Version, key),
            });
        }

        out.kept.sort();
        out.gained.sort();
        Some(out)
    }

    /// 「假如搬到那台机型」的三层视图。
    ///
    /// 上游那张 `machineVariants` 的版本键是 `"A1:STANDARD"` 这种形状，
    /// 搬到 P1S 之后 `"P1S:STANDARD"` 多半不存在 —— 所以版本那一半的上游差异会没了，
    /// 改成继承 P1S 的机型差异。**这就是"继承值会变"的来处**
    fn layers_as_if_moved(&self, uid: &str, to_machine_id: &str) -> Option<Layers<'_>> {
        let v = self.version(uid)?;
        let d = self.digests.get(to_machine_id)?;
        let base = self.bases.get(to_machine_id)?;
        let over = self.overs.get(uid)?;
        let id = self.up.catalog.machine(to_machine_id)?.id.as_str();
        Some(Layers::new(
            &self.up.registry,
            id,
            &d.base,
            base,
            d.versions.get(&v.version_id).unwrap_or(no_overrides()),
            over,
        ))
    }

    /// 把 `key` 改成 `value` 会落到哪几列。
    ///
    /// 目标列由调用方给（= 树上勾了什么），**能落到哪几列由这里判**
    pub fn preview_bulk(&self, key: &str, value: &Value, cols: &[ColRef]) -> BulkPreview {
        let Some(p) = self.up.registry.param(key) else {
            return BulkPreview {
                key: key.to_owned(),
                label: key.to_owned(),
                allowed: false,
                blocked_reason: Some("字段定义里没有这一项".to_owned()),
                effects: Vec::new(),
                skipped: Vec::new(),
            };
        };

        let mut out = BulkPreview {
            key: key.to_owned(),
            label: p.label.clone(),
            allowed: !w::is_gcode(p),
            blocked_reason: if w::is_gcode(p) {
                Some(w::disabled::BULK_REFUSES_GCODE.to_owned())
            } else {
                None
            },
            effects: Vec::new(),
            skipped: Vec::new(),
        };
        if !out.allowed {
            return out;
        }

        // 列序照配方本，与矩阵同一条规则：预览里的顺序要和表里的一致，
        // 否则用户得在两份不同顺序的清单之间对照
        for col in self.matrix(cols, None, "").cols {
            let machine = col.machine.clone();
            let layers = match col.level {
                Level::Machine => self.machine_layers(&col.machine_id),
                Level::Version => col
                    .version_uid
                    .as_deref()
                    .and_then(|u| self.version_layers(u)),
            };
            let Some(layers) = layers else {
                continue;
            };

            if !layers.applies(key) {
                out.skipped.push(BulkSkip {
                    col: col.key.clone(),
                    machine,
                    label: col.label.clone(),
                    reason: w::disabled::NOT_APPLICABLE.to_owned(),
                    blocked: Vec::new(),
                });
                continue;
            }
            let blocked = Gate::new(&self.up.registry, &layers).blocked(key);
            if !blocked.is_empty() {
                out.skipped.push(BulkSkip {
                    col: col.key.clone(),
                    machine,
                    label: col.label.clone(),
                    reason: w::disabled::BLOCKED_BY_CONDITION.to_owned(),
                    blocked,
                });
                continue;
            }

            let before = layers.effective(key);
            let had_own = layers.has_own(col.level, key);
            let kind = match before {
                Some(b) if b.value == value => BulkKind::NoChange,
                _ if had_own => BulkKind::Changing,
                _ => BulkKind::Detaching,
            };
            out.effects.push(BulkEffect {
                col: col.key.clone(),
                machine,
                label: col.label.clone(),
                level: col.level,
                before: before
                    .map(|b| w::value_text(p, b.value))
                    .unwrap_or_else(|| w::NOT_APPLICABLE.to_owned()),
                after: w::value_text(p, value),
                kind,
            });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::domain::patch::{apply, Committed, CommittedVersion, Draft, Patch};
    use crate::workbench::domain::testkit::Fixture;
    use std::collections::BTreeMap;

    fn committed() -> Committed {
        let mut versions = BTreeMap::new();
        for (uid, machine, vid, name) in [
            ("A1/STANDARD", "A1", "STANDARD", "标准版"),
            ("A1/FAST", "A1", "FAST", "高速版"),
            ("A2L/STANDARD", "A2L", "STANDARD", "标准版"),
            ("P1S/LITE", "P1S", "LITE", "精简版"),
        ] {
            versions.insert(
                uid.to_owned(),
                CommittedVersion {
                    machine_id: machine.to_owned(),
                    version_id: vid.to_owned(),
                    name: name.to_owned(),
                    declared_upstream: true,
                    ..Default::default()
                },
            );
        }
        Committed {
            machines: ["A1", "A2L", "P1S"]
                .into_iter()
                .map(|m| (m.to_owned(), super::super::layer::Overrides::new()))
                .collect(),
            versions,
            machine_ids: ["A1", "A2L", "P1S"].into_iter().map(str::to_owned).collect(),
            ..Default::default()
        }
    }

    fn cols(list: &[(&str, Option<&str>)]) -> Vec<ColRef> {
        list.iter()
            .map(|(m, v)| ColRef {
                machine_id: (*m).to_owned(),
                version_uid: v.map(str::to_owned),
            })
            .collect()
    }

    /// **四组都要说出来**。把 A1/STANDARD 搬到 P1S：
    ///
    /// - 继承值会变：`toolhead.offset.x` 从上游的 A1:STANDARD 覆盖（-1）
    ///   变成 P1S 的机型基底（-25.9，由归并上提而来）
    /// - 目标多出来的：`toolhead.only_p1s`（只给 P1S）
    /// - 目标没有的：空（P1S 是 A1 的超集）
    #[test]
    fn moving_reports_all_four_groups() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let b = super::super::Book::new(&f.up, &c, &d);

        let p = b.preview_move("A1/STANDARD", "P1S").unwrap();
        assert!(p.allowed);
        assert_eq!(p.from_machine, "A1");

        let changed: Vec<&str> = p.inherited_changes.iter().map(|x| x.key.as_str()).collect();
        assert!(
            changed.contains(&"toolhead.offset.x"),
            "继承值会变的那一组漏了：{changed:?}"
        );
        let ch = p
            .inherited_changes
            .iter()
            .find(|x| x.key == "toolhead.offset.x")
            .unwrap();
        assert_eq!(ch.before, "-1 mm", "搬走前继承的是上游 A1:STANDARD 那一份");
        assert_eq!(ch.after, "-25.9 mm", "搬过去继承 P1S 的机型基底");
        assert_eq!(ch.before_origin, Origin::Version);
        assert_eq!(ch.after_origin, Origin::Machine, "来源层也变了");

        assert_eq!(p.gained, vec!["toolhead.only_p1s"]);
        assert!(p.lost.is_empty());
        assert!(p.kept.is_empty(), "这一版我们一项都没写过");
    }

    /// 我们写过的项**跟着走、值不变**，所以不进"继承会变"那一组
    #[test]
    fn our_own_overrides_travel_with_the_version() {
        let f = Fixture::load();
        let c = committed();
        let mut d = Draft::default();
        apply(
            &mut d,
            &c,
            &f.up.registry,
            &[Patch::SetValue {
                level: Level::Version,
                owner: "A1/STANDARD".to_owned(),
                key: "wiping.child".to_owned(),
                value: Some(serde_json::json!(77)),
            }],
        )
        .unwrap();

        let b = super::super::Book::new(&f.up, &c, &d);
        let p = b.preview_move("A1/STANDARD", "P1S").unwrap();
        assert_eq!(p.kept, vec!["wiping.child"]);
        assert!(
            !p.inherited_changes.iter().any(|x| x.key == "wiping.child"),
            "自有的项值不变，不该出现在「继承会变」那一组里"
        );
    }

    /// 搬到少字段的机型：我们写在那些字段上的值会变成**孤儿**，必须当场点名
    #[test]
    fn moving_to_a_machine_without_a_field_flags_the_value_that_becomes_an_orphan() {
        let f = Fixture::load();
        let c = committed();
        let mut d = Draft::default();
        apply(
            &mut d,
            &c,
            &f.up.registry,
            &[Patch::SetValue {
                level: Level::Version,
                owner: "P1S/LITE".to_owned(),
                key: "toolhead.only_p1s".to_owned(),
                value: Some(serde_json::json!(5)),
            }],
        )
        .unwrap();

        let b = super::super::Book::new(&f.up, &c, &d);
        let p = b.preview_move("P1S/LITE", "A1").unwrap();
        let lost = p
            .lost
            .iter()
            .find(|x| x.key == "toolhead.only_p1s")
            .expect("A1 没有这一项，该出现在「目标没有的」那一组里");
        assert!(
            lost.had_own_value,
            "我们写过它 —— 搬过去之后这个值再也进不了产物，得说出来"
        );
    }

    /// 目标机型不存在 → **当没移**，并给出理由（doc §15）
    #[test]
    fn moving_to_a_missing_machine_is_reported_as_a_no_op() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let b = super::super::Book::new(&f.up, &c, &d);

        let p = b.preview_move("A1/STANDARD", "KOBRA").unwrap();
        assert!(!p.allowed);
        assert!(p.blocked_reason.unwrap().contains("KOBRA"));
        assert!(p.inherited_changes.is_empty());

        let same = b.preview_move("A1/STANDARD", "A1").unwrap();
        assert!(!same.allowed);
        assert!(same.blocked_reason.is_some());
    }

    /* ---------- 批量 ---------- */

    /// 批量**静默跳过不适用与被关着的列**，但跳过要能查（tasks 7.8）
    #[test]
    fn bulk_skips_not_applicable_and_blocked_columns_but_lists_them() {
        let f = Fixture::load();
        let c = committed();
        let mut d = Draft::default();
        // 把 A1 基底改成圆盘 → A1 那三列的 wiping.child 全被关着
        apply(
            &mut d,
            &c,
            &f.up.registry,
            &[Patch::SetValue {
                level: Level::Machine,
                owner: "A1".to_owned(),
                key: "wiping.mode".to_owned(),
                value: Some(serde_json::json!("disk")),
            }],
        )
        .unwrap();

        let b = super::super::Book::new(&f.up, &c, &d);
        let p = b.preview_bulk(
            "wiping.child",
            &serde_json::json!(66),
            &cols(&[
                ("A1", None),
                ("A1", Some("A1/STANDARD")),
                ("P1S", None),
                ("P1S", Some("P1S/LITE")),
            ]),
        );

        assert!(p.allowed);
        let hit: Vec<&str> = p.effects.iter().map(|e| e.col.as_str()).collect();
        assert_eq!(hit, vec!["P1S", "P1S/LITE"], "A1 那两列被关着，不该算进 N 列");
        assert_eq!(p.skipped.len(), 2);
        assert!(p.skipped.iter().all(|s| !s.blocked.is_empty()));
        assert_eq!(p.skipped[0].blocked[0].key, "wiping.mode");
    }

    /// 不适用的列也跳过，理由与"被关着"不同
    #[test]
    fn a_not_applicable_column_is_skipped_with_its_own_reason() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let b = super::super::Book::new(&f.up, &c, &d);

        let p = b.preview_bulk(
            "toolhead.only_p1s",
            &serde_json::json!(3),
            &cols(&[("A1", None), ("P1S", None)]),
        );
        assert_eq!(p.effects.len(), 1);
        assert_eq!(p.effects[0].col, "P1S");
        assert_eq!(p.skipped.len(), 1);
        assert_eq!(p.skipped[0].col, "A1");
        assert_eq!(p.skipped[0].reason, w::disabled::NOT_APPLICABLE);
        assert!(p.skipped[0].blocked.is_empty(), "它不是被关着，是根本没有");
    }

    /// 三种 kind 分得清：新增覆盖 / 改值 / 没有变化
    #[test]
    fn bulk_tells_detaching_from_changing_from_no_change() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let b = super::super::Book::new(&f.up, &c, &d);
        let target = cols(&[("A1", Some("A1/STANDARD")), ("A1", Some("A1/FAST"))]);

        // A1 两个版本在 offset.x 上有**上游**覆盖，但我们没写过 → 新增覆盖
        let p = b.preview_bulk("toolhead.offset.x", &serde_json::json!(8), &target);
        assert!(p.effects.iter().all(|e| e.kind == BulkKind::Detaching));
        assert_eq!(p.effects[0].before, "-1 mm");
        assert_eq!(p.effects[0].after, "8 mm");

        // 落一个和现在一样的值 → 没有变化
        let p = b.preview_bulk("toolhead.offset.x", &serde_json::json!(-1), &target);
        assert_eq!(p.effects[0].kind, BulkKind::NoChange);
        assert_eq!(p.effects[1].kind, BulkKind::Detaching, "另一版现在是 -0.7");
    }

    /// 我们写过之后再批量 → 改值
    #[test]
    fn bulk_on_a_key_we_already_wrote_is_a_change() {
        let f = Fixture::load();
        let c = committed();
        let mut d = Draft::default();
        apply(
            &mut d,
            &c,
            &f.up.registry,
            &[Patch::SetValue {
                level: Level::Version,
                owner: "A1/STANDARD".to_owned(),
                key: "toolhead.offset.x".to_owned(),
                value: Some(serde_json::json!(2)),
            }],
        )
        .unwrap();
        let b = super::super::Book::new(&f.up, &c, &d);
        let p = b.preview_bulk(
            "toolhead.offset.x",
            &serde_json::json!(9),
            &cols(&[("A1", Some("A1/STANDARD"))]),
        );
        assert_eq!(p.effects[0].kind, BulkKind::Changing);
        assert_eq!(p.effects[0].before, "2 mm");
    }

    /// **G-code 拒绝批量**，并且说清为什么
    #[test]
    fn bulk_refuses_gcode() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let b = super::super::Book::new(&f.up, &c, &d);

        let p = b.preview_bulk(
            "toolhead.script",
            &serde_json::json!("G28"),
            &cols(&[("A1", None)]),
        );
        assert!(!p.allowed);
        assert_eq!(p.blocked_reason.as_deref(), Some(w::disabled::BULK_REFUSES_GCODE));
        assert!(p.effects.is_empty(), "拒绝了就不该给出影响列表");
    }

    /// 预览里的列序和矩阵一致 —— 两份不同顺序的清单要人自己对照
    #[test]
    fn bulk_columns_follow_the_same_order_as_the_matrix() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let b = super::super::Book::new(&f.up, &c, &d);
        let shuffled = cols(&[
            ("P1S", Some("P1S/LITE")),
            ("A1", Some("A1/FAST")),
            ("A1", Some("A1/STANDARD")),
        ]);

        let matrix_order: Vec<String> = b
            .matrix(&shuffled, None, "")
            .cols
            .into_iter()
            .map(|c| c.key)
            .collect();
        let bulk_order: Vec<String> = b
            .preview_bulk("wiping.child", &serde_json::json!(1), &shuffled)
            .effects
            .into_iter()
            .map(|e| e.col)
            .collect();
        assert_eq!(bulk_order, matrix_order);
    }

    /// 字段不存在时不 panic，给一句话
    #[test]
    fn bulk_on_an_unknown_key_is_refused_politely() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let p = super::super::Book::new(&f.up, &c, &d).preview_bulk(
            "toolhead.made_up",
            &serde_json::json!(1),
            &cols(&[("A1", None)]),
        );
        assert!(!p.allowed);
        assert!(p.blocked_reason.is_some());
    }

    /// 三种 kind 的词不重复
    #[test]
    fn bulk_kind_words_are_distinct() {
        let all = [BulkKind::Detaching, BulkKind::Changing, BulkKind::NoChange];
        let mut seen: Vec<&str> = Vec::new();
        for k in all {
            assert!(!k.label().is_empty());
            assert!(!seen.contains(&k.label()), "两种 kind 用了同一个词");
            seen.push(k.label());
        }
    }
}
