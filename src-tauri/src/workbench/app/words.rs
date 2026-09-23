//! 把 `domain::wording` 的整张词表交给前端（doc §13「一个词一处来源」）。
//!
//! # 为什么要有这条命令
//!
//! 没有它的话，前端拿到的是 `build: "stale"` 这种枚举值，于是必须在 TSX 里写
//! `{ stale: '待生成', … }` —— 同一个词就有了两处来源。而两处来源的失效方式很难看：
//! Rust 那边改了措辞，界面上还是老词，没有任何东西会报错。
//!
//! 所以词表**只在 Rust 里写一遍**，开场取一次，前端按枚举值查。
//! 代价是多一条命令；换来的是 Task 8.5 那条「`src/workbench` 里不出现状态词字面量」
//! 的断言变得可能 —— 否则它注定是一条必然失败的判据。
//!
//! # 键用 serde 的 camelCase 名
//!
//! 与 `BookView` 里那些字段序列化出来的值一模一样，所以前端能直接
//! `words.build[node.build]`，不用再做一层映射。
//! [`tests::keys_match_the_serialized_enum_values`] 逐个对过。

use std::collections::BTreeMap;

use serde::Serialize;

use crate::error::AppError;
use crate::ipc::traced;
use crate::workbench::domain::layer::{Level, Origin};
use crate::workbench::domain::patch::Visibility;
use crate::workbench::domain::preview::BulkKind;
use crate::workbench::domain::wording as w;
use crate::workbench::domain::wording::{
    ArtifactState, BbsAssign, BbsSource, BuildState, SaveState,
};

/// 一个词 + 它的解释句。**解释句写「改了会怎样」，不是「这个状态怎么算的」**
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Word {
    pub label: &'static str,
    /// 有些词只有名字没有解释（层名、来源名的短标签），那时为 `None`
    pub explain: Option<&'static str>,
}

impl Word {
    fn new(label: &'static str, explain: &'static str) -> Self {
        Self {
            label,
            explain: Some(explain),
        }
    }
    fn bare(label: &'static str) -> Self {
        Self {
            label,
            explain: None,
        }
    }
}

type Table = BTreeMap<&'static str, Word>;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Words {
    pub build: Table,
    pub artifact: Table,
    pub save: Table,
    pub bbs_assign: Table,
    pub bbs_source: Table,
    pub origin: Table,
    pub level: Table,
    pub visibility: Table,
    pub bulk_kind: Table,
    /// 占位词。四个近义词各管一件事，不许混用（见 `wording` 的模块文档那张表）
    pub placeholder: BTreeMap<&'static str, &'static str>,
    /// 「为什么不能点」。**每个可能被禁用的动作各一句**
    pub disabled: BTreeMap<&'static str, &'static str>,
    /// 空状态那几句。**不留白** —— 空白会被读成「还没算」
    pub empty: BTreeMap<&'static str, &'static str>,
    /// 关联那一组里不带变量的那几句
    pub relate: BTreeMap<&'static str, &'static str>,
    /// 崩溃快照三态。**与 `save` 不是一回事**
    pub snapshot: Table,
}

/// 整张词表。开场取一次
#[tauri::command]
pub fn wb_words() -> Result<Words, AppError> {
    traced("wb_words", |_| Ok(words()))
}

fn words() -> Words {
    Words {
        build: [
            ("built", BuildState::Built),
            ("stale", BuildState::Stale),
            ("neverBuilt", BuildState::NeverBuilt),
            ("noResources", BuildState::NoResources),
        ]
        .into_iter()
        .map(|(k, v)| (k, Word::new(v.label(), v.explain())))
        .collect(),

        artifact: [
            ("fresh", ArtifactState::Fresh),
            ("stale", ArtifactState::Stale),
            ("missing", ArtifactState::Missing),
        ]
        .into_iter()
        .map(|(k, v)| (k, Word::new(v.label(), v.explain())))
        .collect(),

        save: [("saved", SaveState::Saved), ("dirty", SaveState::Dirty)]
            .into_iter()
            .map(|(k, v)| (k, Word::bare(v.label())))
            .collect(),

        bbs_assign: [
            ("assigned", BbsAssign::Assigned),
            ("optional", BbsAssign::Optional),
            ("archiveOnly", BbsAssign::ArchiveOnly),
        ]
        .into_iter()
        .map(|(k, v)| (k, Word::new(v.label(), v.explain())))
        .collect(),

        bbs_source: [
            ("own", BbsSource::Own),
            ("inheritedFromMachine", BbsSource::InheritedFromMachine),
        ]
        .into_iter()
        .map(|(k, v)| (k, Word::new(v.label(), v.explain())))
        .collect(),

        origin: [
            ("factory", Origin::Factory),
            ("machine", Origin::Machine),
            ("version", Origin::Version),
        ]
        .into_iter()
        .map(|(k, v)| (k, Word::new(w::origin_label(v), w::origin_explain(v))))
        .collect(),

        level: [("machine", Level::Machine), ("version", Level::Version)]
            .into_iter()
            .map(|(k, v)| (k, Word::bare(w::level_label(v))))
            .collect(),

        visibility: [
            ("menu", Visibility::Menu),
            ("archiveOnly", Visibility::ArchiveOnly),
        ]
        .into_iter()
        .map(|(k, v)| {
            (
                k,
                Word::new(w::visibility_label(v), w::visibility_explain(v)),
            )
        })
        .collect(),

        bulk_kind: [
            ("detaching", BulkKind::Detaching),
            ("changing", BulkKind::Changing),
            ("noChange", BulkKind::NoChange),
        ]
        .into_iter()
        .map(|(k, v)| (k, Word::bare(v.label())))
        .collect(),

        placeholder: [
            ("blank", w::BLANK),
            ("notApplicable", w::NOT_APPLICABLE),
            ("undeclared", w::UNDECLARED),
            ("unconfigured", w::UNCONFIGURED),
            ("unsupported", w::UNSUPPORTED),
        ]
        .into_iter()
        .collect(),

        disabled: [
            ("detachNothing", w::disabled::DETACH_NOTHING_TO_DETACH),
            ("detachReady", w::disabled::DETACH_READY),
            ("blockedByCondition", w::disabled::BLOCKED_BY_CONDITION),
            ("notApplicable", w::disabled::NOT_APPLICABLE),
            ("bulkRefusesGcode", w::disabled::BULK_REFUSES_GCODE),
            ("buildBlocked", w::disabled::BUILD_BLOCKED),
            ("buildNothingToDo", w::disabled::BUILD_NOTHING_TO_DO),
            ("buildNoResources", w::disabled::BUILD_NO_RESOURCES),
            ("nothingToSave", w::disabled::NOTHING_TO_SAVE),
            ("nothingToUndo", w::disabled::NOTHING_TO_UNDO),
            ("notUndoable", w::disabled::NOT_UNDOABLE),
        ]
        .into_iter()
        .collect(),

        empty: [
            ("noIssues", w::NO_ISSUES),
            ("trashEmpty", w::TRASH_EMPTY),
            ("noDisabledFallback", w::NO_DISABLED_FALLBACK),
            ("matrixNoMatch", w::MATRIX_NO_MATCH),
            ("matrixNoCols", w::MATRIX_NO_COLS),
            ("matrixSearchSpansAllTabs", w::MATRIX_SEARCH_SPANS_ALL_TABS),
        ]
        .into_iter()
        .collect(),

        relate: [
            ("goFixIt", w::relate::GO_FIX_IT),
            ("showAnyway", w::relate::SHOW_ANYWAY),
        ]
        .into_iter()
        .collect(),

        snapshot: [
            ("current", w::SnapshotState::Current),
            ("pending", w::SnapshotState::Pending),
            ("failed", w::SnapshotState::Failed),
        ]
        .into_iter()
        .map(|(k, v)| (k, Word::new(v.label(), v.explain())))
        .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **键必须与枚举序列化出来的值一字不差**，否则前端
    /// `words.build[node.build]` 会查出 `undefined`，界面上就是一片空白 ——
    /// 而空白不会报错
    #[test]
    fn keys_match_the_serialized_enum_values() {
        let t = words();

        let check = |table: &Table, value: serde_json::Value, what: &str| {
            let key = value.as_str().expect("枚举该序列化成字符串").to_owned();
            assert!(
                table.contains_key(key.as_str()),
                "{what} 的 {key} 不在词表里 —— 前端会查出 undefined"
            );
        };

        for v in [
            BuildState::Built,
            BuildState::Stale,
            BuildState::NeverBuilt,
            BuildState::NoResources,
        ] {
            check(&t.build, serde_json::to_value(v).unwrap(), "BuildState");
        }
        for v in [ArtifactState::Fresh, ArtifactState::Stale, ArtifactState::Missing] {
            check(&t.artifact, serde_json::to_value(v).unwrap(), "ArtifactState");
        }
        for v in [SaveState::Saved, SaveState::Dirty] {
            check(&t.save, serde_json::to_value(v).unwrap(), "SaveState");
        }
        for v in [BbsAssign::Assigned, BbsAssign::Optional, BbsAssign::ArchiveOnly] {
            check(&t.bbs_assign, serde_json::to_value(v).unwrap(), "BbsAssign");
        }
        for v in [BbsSource::Own, BbsSource::InheritedFromMachine] {
            check(&t.bbs_source, serde_json::to_value(v).unwrap(), "BbsSource");
        }
        for v in [Origin::Factory, Origin::Machine, Origin::Version] {
            check(&t.origin, serde_json::to_value(v).unwrap(), "Origin");
        }
        for v in [Level::Machine, Level::Version] {
            check(&t.level, serde_json::to_value(v).unwrap(), "Level");
        }
        for v in [Visibility::Menu, Visibility::ArchiveOnly] {
            check(&t.visibility, serde_json::to_value(v).unwrap(), "Visibility");
        }
        for v in [BulkKind::Detaching, BulkKind::Changing, BulkKind::NoChange] {
            check(&t.bulk_kind, serde_json::to_value(v).unwrap(), "BulkKind");
        }
    }

    /// 每个词都要有内容；有解释句的那些，解释句不能只是把词重复一遍
    #[test]
    fn no_word_is_empty_or_a_restatement() {
        let t = words();
        for table in [
            &t.build,
            &t.artifact,
            &t.save,
            &t.bbs_assign,
            &t.bbs_source,
            &t.origin,
            &t.level,
            &t.visibility,
            &t.bulk_kind,
        ] {
            for (k, word) in table {
                assert!(!word.label.trim().is_empty(), "{k} 没有词");
                if let Some(e) = word.explain {
                    assert!(!e.trim().is_empty(), "{k} 的解释句是空的");
                    assert_ne!(e, word.label, "{k} 的解释句只是把词重复了一遍");
                }
            }
        }
        for table in [&t.placeholder, &t.disabled, &t.empty] {
            for (k, s) in table {
                assert!(!s.trim().is_empty(), "{k} 是空句子");
            }
        }
    }

    /// 词表整体能序列化成前端要的形状
    #[test]
    fn serializes_into_a_lookup_table() {
        let v = serde_json::to_value(words()).unwrap();
        assert_eq!(v["build"]["stale"]["label"], "待生成");
        assert!(v["build"]["stale"]["explain"].is_string());
        assert_eq!(v["origin"]["machine"]["label"], "机型");
        assert_eq!(v["level"]["machine"]["label"], "机型基底");
        assert_eq!(v["placeholder"]["blank"], "空");
        // `save` 这一档没有解释句 —— 为 null 而不是空串，前端才好判
        assert!(v["save"]["saved"]["explain"].is_null());
    }
}
