//! **源身份账**：`run/` 下的两本小账，专门回答「这份东西当初是从哪个源来的」。
//!
//! ```text
//! run/source-ledger.json   每次换源、成功拿到一份目录 ⇒ 记一笔（源身份 + 发布时刻 + 目录指纹）
//! run/lineage-source.json  用户线每建一份副本 ⇒ 记一笔（副本相对路径 + 那时生效的源身份）
//! ```
//!
//! # 为什么必须记在 `run/` 而不是 preset 文件里
//!
//! 血统三行（`# based_on*`）是**文件内容**，跟着文件走、用户能看见、也能自己删。
//! 作者 2026-10-27 定：**不许为了记账去改用户的 preset 文件** —— 所以源身份另存。
//! 这两本都是**附加信息**：丢了、坏了、从来没记过，都只让判定退回「说不清」
//! （[`super::local::BasedOn::Unknown`] / `Elsewhere`），**绝不影响文件本身能不能用**。
//!
//! # 两本各管什么
//!
//! - **目录账**（[`SourceNote`]）：这个源我见过几次、每次的发布时刻与目录指纹。
//!   用途是"当前这一份目录，跟血统里那一版比，谁更早"（`publishedAt` 只做辅助，
//!   见 [`super::local::BasedOn`] 的判定顺序）。
//! - **副本账**（[`CopySource`]）：某份用户文件是**在哪个源生效时**建出来的。
//!   用途是回答"我这份与当前云端是不是同一个源" —— 没有它，换源之后所有文件
//!   都会被当成"同一个源、只是内容不同"，也就是那个不可信的「有新版」。
//!
//! # 坏档
//!
//! 一律**按空账**：读不出来就当没记过（判定退回"说不清"），不猜测、不覆盖任何文件。
//! 写盘走 [`atomic_write`]，与 `app-state` / `provenance` 同一条。

use std::path::{Path, PathBuf};

use postprocess::diag::messages;
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::fsx::atomic::atomic_write;

use super::source_identity::SourceIdentity;

/// 目录账落点（内部根的 `run/` 下，与 `app-state.json` 同一层）
const NOTES_FILE: &str = "run/source-ledger.json";
/// 副本账落点
const COPIES_FILE: &str = "run/lineage-source.json";
/// 目录账最多留几条。换源不频繁，几十条足够回看；再多也没人看，白占盘
const MAX_NOTES: usize = 40;

/// 见过一次某个源（一次换源 / 一次成功拉到目录）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceNote {
    /// 规范化后的源身份（[`SourceIdentity::key`] 是它的比较口径）
    pub identity: SourceIdentity,
    /// 那一份目录的发布时刻（RFC3339）。随包 bootstrap 目录刻意没有 ⇒ `None`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published_at: Option<String>,
    /// 目录指纹（`Catalog.revision`）：能确认"是不是同一份目录"
    pub revision: String,
    /// 见到它的时刻（UTC epoch 秒，只给排查与详情显示）
    pub seen_at: u64,
}

/// 某份用户文件是在哪个源生效时建出来的
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopySource {
    /// 用户线那份的相对路径（`presets-local/xxx.toml`）
    pub path: String,
    pub identity: SourceIdentity,
    /// 记账时刻（UTC epoch 秒）
    pub at: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct NotesLedger {
    #[serde(default)]
    notes: Vec<SourceNote>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct CopiesLedger {
    #[serde(default)]
    entries: Vec<CopySource>,
}

fn notes_path(root: &Path) -> PathBuf {
    root.join(NOTES_FILE)
}

fn copies_path(root: &Path) -> PathBuf {
    root.join(COPIES_FILE)
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// 读目录账。读不到 / 坏档 ⇒ 空表（判定退回"说不清"，不影响任何读）
pub fn load_notes(root: &Path) -> Vec<SourceNote> {
    let Ok(bytes) = std::fs::read(notes_path(root)) else {
        return Vec::new();
    };
    serde_json::from_slice::<NotesLedger>(&bytes)
        .map(|l| l.notes)
        .unwrap_or_default()
}

/// 记一笔「现在这个源长这样」。
///
/// 同一个身份**覆盖**成最新一条（账是"这个源最近一次见到的样子"，不是流水账：
/// 流水账会让"血统那一版对应哪次发布"变得要靠猜）。写不进去返回错误，由调用方
/// 决定是否咽掉 —— 记账失败不该让换源失败。
pub fn note_seen(
    root: &Path,
    identity: &SourceIdentity,
    published_at: Option<&str>,
    revision: &str,
) -> Result<(), AppError> {
    let mut ledger = NotesLedger {
        notes: load_notes(root),
    };
    ledger.notes.retain(|n| n.identity != *identity);
    ledger.notes.push(SourceNote {
        identity: identity.clone(),
        published_at: published_at.map(str::to_owned),
        revision: revision.to_owned(),
        seen_at: now_secs(),
    });
    if ledger.notes.len() > MAX_NOTES {
        let cut = ledger.notes.len() - MAX_NOTES;
        ledger.notes.drain(0..cut);
    }
    save(&notes_path(root), &ledger)
}

/// 账上这个源最近一次见到的样子（没有 ⇒ `None`）
pub fn find_note<'a>(notes: &'a [SourceNote], identity: &SourceIdentity) -> Option<&'a SourceNote> {
    notes.iter().find(|n| n.identity == *identity)
}

/// 读副本账。读不到 / 坏档 ⇒ 空表
pub fn load_copies(root: &Path) -> Vec<CopySource> {
    let Ok(bytes) = std::fs::read(copies_path(root)) else {
        return Vec::new();
    };
    serde_json::from_slice::<CopiesLedger>(&bytes)
        .map(|l| l.entries)
        .unwrap_or_default()
}

/// 这份副本当初是在哪个源生效时建的（没有记账 ⇒ `None`，判定退回"说不清"）
pub fn copy_source(root: &Path, path: &str) -> Option<SourceIdentity> {
    load_copies(root)
        .into_iter()
        .find(|e| e.path == path)
        .map(|e| e.identity)
}

/// 记下「这份副本是在这个源下建的」。同一路径覆盖（改过名/重建过以最后一次为准）
pub fn remember_copy(root: &Path, path: &str, identity: &SourceIdentity) -> Result<(), AppError> {
    let mut ledger = CopiesLedger {
        entries: load_copies(root),
    };
    ledger.entries.retain(|e| e.path != path);
    ledger.entries.push(CopySource {
        path: path.to_owned(),
        identity: identity.clone(),
        at: now_secs(),
    });
    save(&copies_path(root), &ledger)
}

/// 文件改名时把条目一起挪（与 `provenance::repath` 同一件事：账记的是历史，路径是现在）
pub fn repath(root: &Path, from: &str, to: &str) -> Result<(), AppError> {
    let mut entries = load_copies(root);
    if !entries.iter().any(|e| e.path == from) {
        return Ok(());
    }
    for e in &mut entries {
        if e.path == from {
            e.path = to.to_owned();
        }
    }
    save(&copies_path(root), &CopiesLedger { entries })
}

fn save<T: Serialize>(path: &Path, value: &T) -> Result<(), AppError> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| {
        AppError::io(messages::text("ui.runtime.source_ledger.493", &[])).with_detail(e.to_string())
    })?;
    atomic_write(path, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::source_identity::parse_identity;

    fn gh() -> SourceIdentity {
        parse_identity("https://github.com/o/r/tree/main").unwrap()
    }

    fn gt() -> SourceIdentity {
        parse_identity("https://gitee.com/o/r/tree/main").unwrap()
    }

    /// 同一个源记两次 ⇒ 只有一条（"最近一次见到的样子"），且带着新的发布时间
    #[test]
    fn the_same_source_is_one_note() {
        let root = tempfile::tempdir().unwrap();
        note_seen(root.path(), &gh(), Some("2026-10-25T00:00:00Z"), "rev-a").unwrap();
        note_seen(root.path(), &gh(), Some("2026-10-26T00:00:00Z"), "rev-b").unwrap();
        let notes = load_notes(root.path());
        assert_eq!(notes.len(), 1, "同一个源只留一条");
        assert_eq!(notes[0].revision, "rev-b");
        assert_eq!(
            notes[0].published_at.as_deref(),
            Some("2026-10-26T00:00:00Z")
        );
    }

    /// 两个源各占一条，互不覆盖（这正是"谁也没比谁新"的底账）
    #[test]
    fn two_sources_keep_two_notes() {
        let root = tempfile::tempdir().unwrap();
        note_seen(root.path(), &gh(), Some("2026-10-25T00:00:00Z"), "rev-a").unwrap();
        note_seen(root.path(), &gt(), Some("2026-10-19T00:00:00Z"), "rev-b").unwrap();
        let notes = load_notes(root.path());
        assert_eq!(notes.len(), 2);
        assert_eq!(
            find_note(&notes, &gh()).and_then(|n| n.published_at.clone()),
            Some("2026-10-25T00:00:00Z".to_owned())
        );
    }

    /// 副本账：按路径记，同一个源写两次不留两条；改名跟着走
    #[test]
    fn the_copy_ledger_follows_the_path() {
        let root = tempfile::tempdir().unwrap();
        let path = "presets-local/A1.toml";
        assert!(copy_source(root.path(), path).is_none(), "没记过就是不知道");
        remember_copy(root.path(), path, &gh()).unwrap();
        remember_copy(root.path(), path, &gh()).unwrap();
        assert_eq!(load_copies(root.path()).len(), 1, "同一路径不留两条");
        assert_eq!(copy_source(root.path(), path), Some(gh()));

        repath(root.path(), path, "presets-local/B1.toml").unwrap();
        assert!(copy_source(root.path(), path).is_none(), "旧名字不再有人");
        assert_eq!(
            copy_source(root.path(), "presets-local/B1.toml"),
            Some(gh())
        );
    }

    /// 坏档按空账：读不出来不许让任何判定失败，更不许覆盖别的东西
    #[test]
    fn a_corrupt_ledger_reads_as_empty() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("run")).unwrap();
        atomic_write(&root.path().join(NOTES_FILE), b"{ not json").unwrap();
        atomic_write(&root.path().join(COPIES_FILE), b"]").unwrap();
        assert!(load_notes(root.path()).is_empty());
        assert!(load_copies(root.path()).is_empty());
        assert!(copy_source(root.path(), "presets-local/A1.toml").is_none());
        /* 空账之上还能正常记账（坏档不挡路） */
        note_seen(root.path(), &gh(), None, "rev-a").unwrap();
        assert_eq!(load_notes(root.path()).len(), 1);
    }
}
