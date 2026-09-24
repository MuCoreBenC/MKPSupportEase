//! 只读推演：批量影响范围（doc §8.4、tasks 7.8）。
//!
//! 这一层**不写任何东西**。它回答的是"我按下去会发生什么"，
//! 而这个问题必须在按下去之前有答案 —— 否则用户只能改完再看，
//! 而"改完再看"对批量来说等于没有预览（一次盖掉十几列）。
//!
//! 以前这里还有一整个"移动预览"（把某个版本搬到另一台机型前后有什么不同）。
//! b04 Task 12 把「移动」连同那条 Conservative 清单类动作一起删掉了（REPORT §7），
//! 于是那四组推演失去了对象 —— 版本搬到哪儿去现在是「机型与版本」页的事。
//!
//! # 批量：静默跳过两类目标
//!
//! 「不适用」与「被上级条件关着」的列**不出现在预览里、也不计入「N 列」**（doc §8.4）。
//! 理由是这两类目标上落值没有意义：前者根本没有这一项，后者的值现在不生效。
//! 但**跳过要能查** —— 所以 [`BulkPreview::skipped`] 把它们列出来并写明原因，
//! 而不是让"我勾了 6 列怎么只改了 4 列"变成一个谜。
//!
//! G-code **拒绝批量**：一段多行脚本被整体盖掉是不可逆的误操作。

use serde::Serialize;
use serde_json::Value;

use super::derive::{Book, ColRef};
use super::layer::Level;
use super::visibility::{BlockedBy, Gate};
use super::wording as w;

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
    /// 把 `key` 改成 `value` 会落到哪几列。
    ///
    /// 目标列由调用方给（= 树上勾了什么），**能落到哪几列由这里判**
    pub fn preview_bulk(&self, key: &str, value: &Value, cols: &[ColRef]) -> BulkPreview {
        let Some(p) = self.presets.registry.param(key) else {
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
            let blocked = Gate::new(&self.presets.registry, &layers).blocked(key);
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
    use crate::workbench::domain::testkit::{fixture_catalog, Fixture};
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
                    ..Default::default()
                },
            );
        }
        Committed {
            versions,
            catalog: fixture_catalog(),
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

    // 移动预览那一整组测试删了（b04 Task 12）：`Patch::MoveVersion` 与
    // `Book::preview_move` 一起没了 —— 版本搬到哪台机型下是「机型与版本」页的事，
    // 清单就写在 `presets/machines/{机型}.toml` 里。

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
            &f.presets.registry,
            &[Patch::SetValue {
                level: Level::Machine,
                owner: "A1".to_owned(),
                key: "wiping.mode".to_owned(),
                value: Some(serde_json::json!("disk")),
            }],
        )
        .unwrap();

        let b = super::super::Book::new(Some(&f.up), &f.presets, &c, &d);
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
        assert_eq!(
            hit,
            vec!["P1S", "P1S/LITE"],
            "A1 那两列被关着，不该算进 N 列"
        );
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
        let b = super::super::Book::new(Some(&f.up), &f.presets, &c, &d);

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
        let b = super::super::Book::new(Some(&f.up), &f.presets, &c, &d);
        let target = cols(&[("A1", Some("A1/STANDARD")), ("A1", Some("A1/FAST"))]);

        // ① 这一层**没钉着**那个键 → 新增覆盖。`wiping.child` 在 `machineVariants`
        //    里一条都没有，所以两个版本列都算脱钩
        let p = b.preview_bulk("wiping.child", &serde_json::json!(8), &target);
        assert!(
            p.effects.iter().all(|e| e.kind == BulkKind::Detaching),
            "{:#?}",
            p.effects
        );

        // ② 这一层**钉着**那个键 → 改值。A1 的两个版本各有一条 `A1:xxx` 的版本键，
        //    而版本层的值现在就是从那张表里读出来的，所以两列都算「自己写过」
        let p = b.preview_bulk("toolhead.offset.x", &serde_json::json!(8), &target);
        assert!(
            p.effects.iter().all(|e| e.kind == BulkKind::Changing),
            "{:#?}",
            p.effects
        );
        assert_eq!(p.effects[0].before, "-1 mm");
        assert_eq!(p.effects[0].after, "8 mm");

        // ③ 落一个和 A1/STANDARD 现在一样的值 → 那一列没有变化
        let p = b.preview_bulk("toolhead.offset.x", &serde_json::json!(-1), &target);
        assert_eq!(p.effects[0].kind, BulkKind::NoChange);
        assert_eq!(p.effects[1].kind, BulkKind::Changing, "另一版现在是 -0.7");
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
            &f.presets.registry,
            &[Patch::SetValue {
                level: Level::Version,
                owner: "A1/STANDARD".to_owned(),
                key: "toolhead.offset.x".to_owned(),
                value: Some(serde_json::json!(2)),
            }],
        )
        .unwrap();
        let b = super::super::Book::new(Some(&f.up), &f.presets, &c, &d);
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
        let b = super::super::Book::new(Some(&f.up), &f.presets, &c, &d);

        let p = b.preview_bulk(
            "toolhead.script",
            &serde_json::json!("G28"),
            &cols(&[("A1", None)]),
        );
        assert!(!p.allowed);
        assert_eq!(
            p.blocked_reason.as_deref(),
            Some(w::disabled::BULK_REFUSES_GCODE)
        );
        assert!(p.effects.is_empty(), "拒绝了就不该给出影响列表");
    }

    /// 预览里的列序和矩阵一致 —— 两份不同顺序的清单要人自己对照
    #[test]
    fn bulk_columns_follow_the_same_order_as_the_matrix() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let b = super::super::Book::new(Some(&f.up), &f.presets, &c, &d);
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
        let p = super::super::Book::new(Some(&f.up), &f.presets, &c, &d).preview_bulk(
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
