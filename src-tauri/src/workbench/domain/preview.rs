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

use std::collections::BTreeSet;

use serde::Serialize;
use serde_json::Value;

use super::derive::{Book, ColRef};
use super::layer::Level;
use super::patch::normalize_value;
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
    /// 这一列所属机型**基线的当前值**（人话）。版本列写它 = 与基线同值
    pub base: String,
    /**
     * 要写的值是否**恰好等于基线**。
     *
     * 是的话这一列不必各自钉住：写 `null` 让它**保持跟随**，值一样、还少一行
     * `machineVariants` + 一行 `follows`（作者 2026-11-09 要的就是这个默认行为）。
     * 只有版本列才有这个选择 —— 机型基线是这条链的根。
     */
    pub same_as_base: bool,
    /**
     * 这一列的机型基线**也在这一批里**（且真的会被写）。
     *
     * 那时版本列整列跟着基线走：写 `null`，值由基线带过去 —— 一批里若把
     * 基线设成 1.2，跟随它的版本结果也是 1.2，文件里不会再给每个版本各留一行
     * 1.2（作者 2026-11-09：「都是同样的值的时候，它完全可以跟随基线」）。
     *
     * 机型基线自己恒 `false`：它是这条链的根。
     */
    pub follows_base: bool,
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

/* ---------- 挂回跟随预览（批量） ---------- */

/// 「把这些列挂回跟随」会落到哪几列。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachPreview {
    pub key: String,
    pub label: String,
    /// 能不能批量（G-code 不行，与 [`BulkPreview`] 同一条闸）
    pub allowed: bool,
    pub blocked_reason: Option<String>,
    /// 勾选里的**每一列版本**（适用的、没被条件关着的），含本来就在跟随的。
    ///
    /// 把"已在跟随"也列出来是因为**基线值要看得见**：作者 2026-11-09
    /// 「跟随基线，它这个默认就要把基线显示出来」——只列"要改的"那几列时，
    /// 全部已在跟随就只剩 9 行「本来就是继承来的」，一个数字都没有。
    pub effects: Vec<AttachEffect>,
    /// 跳过（不适用 / 被条件关着 / 机型基线是根），附原因
    pub skipped: Vec<BulkSkip>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachEffect {
    /// 列的 key（机型 id 或版本 uid）
    pub col: String,
    pub machine: String,
    pub label: String,
    pub level: Level,
    /// 这一层现在的值（人话）。本来就在跟随的，这里就是它跟随到的基线值
    pub before: String,
    /// 挂回之后这一层**显示的值** = 这一层不钉时继承到的那个（机型基线 → 出厂默认）。
    ///
    /// 与 [`BulkEffect::base`] 同一处取值（`Layers::inherited_value`）：两种预览
    /// 对"基线是多少"必须说同一个数。
    pub after: String,
    /// 这一层**本来就在跟随**（写下去是空操作，只是把基线值摊出来）
    pub following: bool,
    /// 挂回去**值会不会变**（归一化后的 `Value` 比较，不比格式化文本）。
    ///
    /// 界面上「值会变 / 值不变」只认它 —— 值不变的那些只是把状态改成跟随，
    /// 不该被说成"从 1.2 改成了 1.2"那样的假变化。
    pub changes: bool,
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
        // 要与盘上那份比，就得和写入口看同一个类型：界面发来的 `-1`（整数）与
        // 盘上的 `-1.0`（浮点）用 `==` 比不中，同一列会被误报成「改值」
        let value = &normalize_value(&self.presets.registry, key, value);

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

        let matrix = self.matrix(cols, None, "", None);

        /*
         * 这一批里**机型基线也会被写**的那些机型。
         *
         * 基线写进去之后，跟着它的版本自动变成同一个值 —— 那些版本列就不必
         * 各写一行相同的数（作者 2026-11-09：「都是同样的值的时候，它完全可以
         * 跟随基线」）。判据与下面正文同一套：适用 + 没被条件关着。
         */
        let mut base_written: BTreeSet<String> = BTreeSet::new();
        for col in &matrix.cols {
            if col.level != Level::Machine {
                continue;
            }
            let Some(layers) = self.machine_layers(&col.machine_id) else {
                continue;
            };
            if !layers.applies(key) {
                continue;
            }
            if !Gate::new(&self.presets.registry, &layers)
                .blocked(key)
                .is_empty()
            {
                continue;
            }
            base_written.insert(col.machine_id.clone());
        }

        // 列序照配方本，与矩阵同一条规则：预览里的顺序要和表里的一致，
        // 否则用户得在两份不同顺序的清单之间对照
        for col in matrix.cols {
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

            // 盘上那一份也归一到声明类型，理由见 [`normalize_value`]：两侧归一规则
            // 不一样的话，界面发来的 `-1` 与盘上的 `-1.0` 会把「没有变化」报成「改值」
            let before = layers
                .effective(key)
                .map(|b| normalize_value(&self.presets.registry, key, b.value));
            let had_own = layers.has_own(col.level, key);
            let kind = match &before {
                Some(b) if b == value => BulkKind::NoChange,
                _ if had_own => BulkKind::Changing,
                _ => BulkKind::Detaching,
            };
            /* 基线 = 这一层不写时会拿到的值（版本 → 机型基底 → 出厂默认）。
            与要写的值相同 ⇒ 这一列可以「保持跟随」而不是自己钉一行。 */
            let inherited = layers
                .inherited_value(col.level, key)
                .map(|v| normalize_value(&self.presets.registry, key, v));
            let same_as_base =
                col.level == Level::Version && inherited.as_ref().is_some_and(|b| b == value);
            let follows_base =
                col.level == Level::Version && base_written.contains(&col.machine_id);
            out.effects.push(BulkEffect {
                col: col.key.clone(),
                machine,
                label: col.label.clone(),
                level: col.level,
                before: before
                    .map(|b| w::value_text(p, &b))
                    .unwrap_or_else(|| w::NOT_APPLICABLE.to_owned()),
                after: w::value_text(p, value),
                kind,
                base: inherited
                    .map(|b| w::value_text(p, &b))
                    .unwrap_or_else(|| w::NOT_APPLICABLE.to_owned()),
                same_as_base,
                follows_base,
            });
        }
        out
    }

    /// **把这些列挂回跟随**会落到哪几列（写 `null` = 让这一层重新跟着基线）。
    ///
    /// 与 [`Self::preview_bulk`] 是同一个问题的另一半：那个答"设成这个值会发生什么"，
    /// 这个答"让它重新跟着基线会发生什么"。跳过判据完全一致（不适用 / 被条件关着 /
    /// 机型基线是根），但**本来就在跟随的版本也列进 `effects`**：预览要能把基线值
    /// 摊给人看，而不是"9 行本来就是继承来的、一个数字都没有"。
    ///
    /// 为什么要预览而不能直接写：批量一次动十几列，改的是"这一层自己钉住的状态"，
    /// 与写入一样是不可逆的误操作面 —— 先看清哪几列真的会变，再点。
    pub fn preview_attach(&self, key: &str, cols: &[ColRef]) -> AttachPreview {
        let Some(p) = self.presets.registry.param(key) else {
            return AttachPreview {
                key: key.to_owned(),
                label: key.to_owned(),
                allowed: false,
                blocked_reason: Some("字段定义里没有这一项".to_owned()),
                effects: Vec::new(),
                skipped: Vec::new(),
            };
        };
        let mut out = AttachPreview {
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

        // 列序照配方本，与矩阵 / 批量预览同一条规则
        for col in self.matrix(cols, None, "", None).cols {
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
            /*
             * 「这一层跟不跟随」只认 `[params.follows]`（与写值、徽章同一处），
             * **不能用 `Layers::has_own`** —— 同一机型下每个版本都写了同一个值时，
             * `digest` 会把它上提成基底，`over` 里就没有这一项了，于是明明 9 行版本键
             * 还在、界面却报"本来就是继承来的"（作者 2026-11-08 报的就是这个）。
             *
             * 机型基底不在这条链的"跟随"语义里（它是根），目标是它时直说。
             */
            let following = match col.level {
                Level::Machine => {
                    out.skipped.push(BulkSkip {
                        col: col.key.clone(),
                        machine,
                        label: col.label.clone(),
                        reason: w::disabled::DETACH_ON_ROOT_LAYER.to_owned(),
                        blocked: Vec::new(),
                    });
                    continue;
                }
                Level::Version => {
                    let owner = col.version_uid.as_deref().unwrap_or_default();
                    self.version_following(owner, key)
                }
            };

            let before = layers
                .effective(key)
                .map(|b| normalize_value(&self.presets.registry, key, b.value));
            /* 挂回之后拿到的就是继承值 —— 与 `preview_bulk` 的 `base` 同一处，
            两边各取一次的话「设成基线」与「跟随基线」会报出两个不同的数。 */
            let inherited = layers
                .inherited_value(col.level, key)
                .map(|v| normalize_value(&self.presets.registry, key, v));
            /* 已在跟随的列**也列出来**：要让人看见基线值是多少（作者 2026-11-09
            「它这个默认就要把基线显示出来」）。它写下去是空操作，`changes` 恒 false。 */
            out.effects.push(AttachEffect {
                col: col.key.clone(),
                machine,
                label: col.label.clone(),
                level: col.level,
                before: before
                    .clone()
                    .map(|b| w::value_text(p, &b))
                    .unwrap_or_else(|| w::NOT_APPLICABLE.to_owned()),
                after: inherited
                    .clone()
                    .map(|b| w::value_text(p, &b))
                    .unwrap_or_else(|| w::NOT_APPLICABLE.to_owned()),
                following,
                /* 归一化后的 `Value` 相比：`1` 与 `1.0` 是同一个数，不许被
                格式化文本的差别说成"值会变" */
                changes: !following && before != inherited,
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

        let b = super::super::Book::new(&f.presets, &c, &d);
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

    /// **新值恰好等于基线的列要标出来**：那一列可以「保持跟随」而不是各自钉住。
    ///
    /// 作者 2026-11-09：批量设成基线那个数时，不该顺手多出 N 行版本键 + N 行 follows。
    #[test]
    fn a_value_that_equals_the_base_is_flagged_as_same_as_base() {
        let f = Fixture::load();
        let c = committed();
        let mut d = Draft::default();
        apply(
            &mut d,
            &c,
            &f.presets.registry,
            &[Patch::SetValue {
                level: Level::Machine,
                owner: "P1S".to_owned(),
                key: "wiping.child".to_owned(),
                value: Some(serde_json::json!(66)),
            }],
        )
        .unwrap();
        let b = super::super::Book::new(&f.presets, &c, &d);
        let list = cols(&[("P1S", Some("P1S/LITE"))]);

        let hit = b.preview_bulk("wiping.child", &serde_json::json!(66), &list);
        let e = hit.effects.first().expect("该有一列");
        assert!(e.same_as_base, "新值等于基线 66 —— 可以保持跟随");
        assert!(!e.base.is_empty(), "基线值要摊给界面：{e:?}");

        let miss = b.preview_bulk("wiping.child", &serde_json::json!(67), &list);
        assert!(!miss.effects[0].same_as_base, "67 不等于基线，必须自己钉住");
    }

    /// **基线也在这一批里 ⇒ 版本列写「跟随基线」**（作者 2026-11-09）。
    ///
    /// 「基线设成 1.2，跟随它的版本结果也是 1.2」—— 不必给每个版本各写一行 1.2。
    /// 只勾版本、没勾基线时不能这么判：那时版本没有可跟的新值。
    #[test]
    fn a_batch_that_writes_the_base_lets_versions_follow_it() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let b = super::super::Book::new(&f.presets, &c, &d);

        let with_base = b.preview_bulk(
            "wiping.child",
            &serde_json::json!(42),
            &cols(&[("A1", None), ("A1", Some("A1/STANDARD"))]),
        );
        let base = with_base
            .effects
            .iter()
            .find(|e| e.level == Level::Machine)
            .expect("机型基线在列表里");
        assert!(!base.follows_base, "基线自己是根，谈不上跟随");
        let ver = with_base
            .effects
            .iter()
            .find(|e| e.level == Level::Version)
            .expect("版本列在列表里");
        assert!(
            ver.follows_base,
            "基线也在这一批里 ⇒ 版本跟着基线走：{ver:?}"
        );
        /* 预览里两列写的是同一个新值 —— 结果就是"基线 42、版本跟着 42" */
        assert_eq!(ver.after, base.after, "版本跟随后的值 = 基线的新值");

        /* 只勾版本：没有新的基线可跟，必须自己钉住 */
        let only_version = b.preview_bulk(
            "wiping.child",
            &serde_json::json!(42),
            &cols(&[("A1", Some("A1/STANDARD"))]),
        );
        assert!(!only_version.effects[0].follows_base);
    }

    /// **「跟随基线」只列「不跟随」的版本列**：判据是 `[params.follows]`，不是 `has_own`。
    ///
    /// 关键反例（作者 2026-11-08 报的）：同一机型每个版本都写了同一个值时，`digest`
    /// 会把它上提成基底 —— `has_own` 因此变 false、界面却仍标着「不跟」，于是
    /// 「跟随基线」把九列全报成"本来就是继承来的"，一条也挂不回去。
    #[test]
    fn attach_lists_versions_that_are_not_following() {
        let f = Fixture::load();
        let c = committed();
        let mut d = Draft::default();
        // 只给 P1S:LITE 写一个自己的值 ⇒ 它不跟随；其余的版本列没动过 ⇒ 仍跟随
        apply(
            &mut d,
            &c,
            &f.presets.registry,
            &[Patch::SetValue {
                level: Level::Version,
                owner: "P1S/LITE".to_owned(),
                key: "wiping.child".to_owned(),
                value: Some(serde_json::json!(66)),
            }],
        )
        .unwrap();

        let list = cols(&[
            ("A1", None),
            ("A1", Some("A1/STANDARD")),
            ("P1S", Some("P1S/LITE")),
        ]);
        let b = super::super::Book::new(&f.presets, &c, &d);
        let out = b.preview_attach("wiping.child", &list);

        assert!(out.allowed, "普通数值参数能挂回跟随");
        let hit: Vec<&str> = out.effects.iter().map(|e| e.col.as_str()).collect();
        assert_eq!(
            hit,
            vec!["A1/STANDARD", "P1S/LITE"],
            "版本列都列出来（含已在跟随的），基线值才看得见：{:?}",
            out.skipped
        );
        assert!(
            out.skipped.iter().any(|s| s.reason.contains("根")),
            "机型基线要单独说清它没有跟随：{:?}",
            out.skipped
        );
        /* 已在跟随的列也摊值：不然"全部已跟随"时预览里一个数字都没有 */
        let follow = out
            .effects
            .iter()
            .find(|e| e.col == "A1/STANDARD")
            .expect("A1/STANDARD 在列表里");
        assert!(follow.following, "它本来就在跟随");
        assert!(!follow.changes, "已在跟随 ⇒ 值不变");
        assert_eq!(follow.before, follow.after, "前后都写基线值");

        /* 挂回之后的值必须摊给界面：只有 `before` 的话，人看到的是
        「1.2 → 跟随基线」，不知道值会变成多少（作者 2026-11-09 报的）。 */
        let p = f
            .presets
            .registry
            .param("wiping.child")
            .expect("wiping.child");
        let e = out
            .effects
            .iter()
            .find(|e| e.col == "P1S/LITE")
            .expect("P1S/LITE 在列表里");
        assert_eq!(
            e.after,
            w::value_text(p, &serde_json::json!(20.0)),
            "跟随之后拿到的是基线（出厂默认 20）"
        );
        assert!(!e.following, "它自己钉着 66");
        assert!(e.changes, "66 → 20 是值会变");
    }

    /// 挂回时**值不变**的列要如实说「值不变」（只翻状态）。
    ///
    /// 「自己钉着一个与基底相同的数」也是不跟随：挂回会把这一行去掉，
    /// 但值从头到尾没变 —— 界面不能把它说成"值从 20 变成 20"。
    #[test]
    fn attach_marks_columns_whose_value_does_not_change() {
        let mut f = Fixture::load();
        let c = committed();
        f.presets
            .set_variant("wiping.child", "P1S:LITE", &serde_json::json!(20.0))
            .unwrap();
        let d = Draft::default();
        let b = super::super::Book::new(&f.presets, &c, &d);

        let out = b.preview_attach("wiping.child", &cols(&[("P1S", Some("P1S/LITE"))]));
        let e = out.effects.first().expect("钉住的那一列会挂回");
        assert!(!e.changes, "20 → 20 是值不变：{e:?}");
        assert_eq!(e.before, e.after, "前后值同一个数");
    }

    /// 不适用的列也跳过，理由与"被关着"不同
    #[test]
    fn a_not_applicable_column_is_skipped_with_its_own_reason() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let b = super::super::Book::new(&f.presets, &c, &d);

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
        let b = super::super::Book::new(&f.presets, &c, &d);
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
        assert_eq!(p.effects[0].before, "-1.0 mm");
        assert_eq!(p.effects[0].after, "8.0 mm");

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
        let b = super::super::Book::new(&f.presets, &c, &d);
        let p = b.preview_bulk(
            "toolhead.offset.x",
            &serde_json::json!(9),
            &cols(&[("A1", Some("A1/STANDARD"))]),
        );
        assert_eq!(p.effects[0].kind, BulkKind::Changing);
        assert_eq!(p.effects[0].before, "2.0 mm");
    }

    /// **G-code 拒绝批量**，并且说清为什么
    #[test]
    fn bulk_refuses_gcode() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let b = super::super::Book::new(&f.presets, &c, &d);

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
        let b = super::super::Book::new(&f.presets, &c, &d);
        let shuffled = cols(&[
            ("P1S", Some("P1S/LITE")),
            ("A1", Some("A1/FAST")),
            ("A1", Some("A1/STANDARD")),
        ]);

        let matrix_order: Vec<String> = b
            .matrix(&shuffled, None, "", None)
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
        let p = super::super::Book::new(&f.presets, &c, &d).preview_bulk(
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
