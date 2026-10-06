//! **出处账**：用户文件是「复制」还是「导入」来的、从哪一份复制来的。
//!
//! # 为什么要有这本账（而不是写在文件里）
//!
//! 血统（`# based_on*` 三行）记的是**官方来源**，写在文件头里、跟着文件走 —— 那条线
//! 已经有了。但「我这份是从**我自己另一份**复制出来的」这件事**没有地方可写**：
//! 第十一层「另存为一份新的」是**按字节**复制（血统原样带过去），往文件头里再加一行
//! 就得重写字节 —— 那违背它"原文件一个字节不动、副本按字节复制"的约定。
//! 导入同理：外部文件进 `presets-mine/` 前后的字节是用户自己的，客户端不往里写东西。
//!
//! 所以出处记在**用户根的账本**里（`provenance.json`）：`to`（副本路径）→
//! `from`（来源路径，导入没有）+ `kind`（copy / import）。它只回答界面上那一格
//! 「来源：复制自 X / 导入」—— **不是权限、不是身份**：账本丢了或没记上，
//! 那一份照常能用能改能删，界面上退回「我的」，什么都不坏。
//!
//! # 盘当底账的例外
//!
//! 用户线一贯"盘就是底账"（扫盘、不记账），这一本是**唯一的例外**——因为"从哪复制来的"
//! 这个事实盘上不存在，不记就永远丢了。它记的是**历史事件**，不是文件的现在：
//! 文件被改名、被删除之后账上的条目要跟着改（[`repath`]）、可以悬空（界面照实退回）。

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::fsx::atomic::atomic_write;

/// 出处的两种来路。落账本/契约的就是这两个词（跨 IPC 的稳定词，与 `MineState` 同一规矩）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvenanceKind {
    /// 从用户自己的另一份复制出来的（第十一层「另存为一份新的」）
    Copy,
    /// 从外部导入进来的（第十二层）
    Import,
}

impl ProvenanceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ProvenanceKind::Copy => "copy",
            ProvenanceKind::Import => "import",
        }
    }
}

/// 一条出处。`to` 是副本（现在存在的那份）相对用户根的路径；`from` 是来源，
/// 导入的那一档没有来源路径（外部文件不在客户端的地盘里）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    pub kind: String,
    /// 记账时刻（UTC epoch 秒）。只给排查用，界面不显示
    pub at: u64,
    #[serde(default)]
    schema: u8,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Ledger {
    #[serde(default)]
    entries: Vec<Entry>,
}

/// 账本落点：用户根下的 `provenance.json`（与 `presets-mine/` 同一层 ——
/// 它说的是用户那份的事，跟用户的数据住一起）
fn ledger_path(user_root: &Path) -> std::path::PathBuf {
    user_root.join("provenance.json")
}

/// 读账本。读不到（还没记过）/ 坏档 ⇒ 空表 —— 出处是附加信息，缺了不许让任何读失败
pub fn load(user_root: &Path) -> Vec<Entry> {
    let Ok(bytes) = std::fs::read(ledger_path(user_root)) else {
        return Vec::new();
    };
    serde_json::from_slice::<Ledger>(&bytes)
        .map(|l| l.entries)
        .unwrap_or_default()
}

/// 记一条（或盖掉同一份的旧条目 —— 同一路径再复制/导入一次以最后一次为准）。
/// 只追加这一条；写不进去不影响主流程（出处是附加信息，调用方不因它失败）
pub fn record(
    user_root: &Path,
    to: &str,
    from: Option<&str>,
    kind: ProvenanceKind,
) -> Result<(), AppError> {
    let mut ledger = Ledger {
        entries: load(user_root),
    };
    ledger.entries.retain(|e| e.to != to);
    ledger.entries.push(Entry {
        to: to.to_owned(),
        from: from.map(str::to_owned),
        kind: kind.as_str().to_owned(),
        at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        schema: 1,
    });
    save(user_root, &ledger)
}

/// 文件改名之后账跟着走（第十层）：副本改了名 ⇒ `to` 跟着改；**来源**改了名 ⇒ `from` 跟着改
/// （不然"复制自 X"第二天就指向一个不存在的名字）。没有匹配条目就是无事发生
pub fn repath(user_root: &Path, old_rel: &str, new_rel: &str) -> Result<(), AppError> {
    let mut ledger = Ledger {
        entries: load(user_root),
    };
    let mut touched = false;
    for e in &mut ledger.entries {
        if e.to == old_rel {
            e.to = new_rel.to_owned();
            touched = true;
        }
        if e.from.as_deref() == Some(old_rel) {
            e.from = Some(new_rel.to_owned());
            touched = true;
        }
    }
    if touched {
        save(user_root, &ledger)?;
    }
    Ok(())
}

fn save(user_root: &Path, ledger: &Ledger) -> Result<(), AppError> {
    let body = serde_json::to_vec_pretty(ledger)
        .map_err(|e| AppError::internal("出处账本序列化不了").with_detail(e.to_string()))?;
    atomic_write(&ledger_path(user_root), &body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_then_lookup_roundtrip() {
        let d = tempfile::tempdir().unwrap();
        record(
            d.path(),
            "presets-mine/B.toml",
            Some("presets-mine/A.toml"),
            ProvenanceKind::Copy,
        )
        .unwrap();
        let all = load(d.path());
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].to, "presets-mine/B.toml");
        assert_eq!(all[0].from.as_deref(), Some("presets-mine/A.toml"));
        assert_eq!(all[0].kind, "copy");
    }

    /// 同一路径再复制一次：以最后一次为准（盖旧条目），账不越长越多
    #[test]
    fn re_recording_replaces_the_entry_for_the_same_copy() {
        let d = tempfile::tempdir().unwrap();
        record(
            d.path(),
            "presets-mine/B.toml",
            Some("presets-mine/A.toml"),
            ProvenanceKind::Copy,
        )
        .unwrap();
        record(
            d.path(),
            "presets-mine/B.toml",
            Some("presets-mine/C.toml"),
            ProvenanceKind::Copy,
        )
        .unwrap();
        let all = load(d.path());
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].from.as_deref(), Some("presets-mine/C.toml"));
    }

    /// 改名：副本改名 `to` 跟走；来源改名 `from` 跟走 —— 两个方向都要
    #[test]
    fn repath_follows_both_sides_of_a_copy() {
        let d = tempfile::tempdir().unwrap();
        record(
            d.path(),
            "presets-mine/B.toml",
            Some("presets-mine/A.toml"),
            ProvenanceKind::Copy,
        )
        .unwrap();

        repath(d.path(), "presets-mine/B.toml", "presets-mine/B2.toml").unwrap();
        assert_eq!(load(d.path())[0].to, "presets-mine/B2.toml");

        repath(d.path(), "presets-mine/A.toml", "presets-mine/A2.toml").unwrap();
        let e = &load(d.path())[0];
        assert_eq!(e.to, "presets-mine/B2.toml");
        assert_eq!(e.from.as_deref(), Some("presets-mine/A2.toml"));
    }

    /// 坏档 ⇒ 空表：出处缺了不许让用户线的任何读失败
    #[test]
    fn a_broken_ledger_reads_as_empty() {
        let d = tempfile::tempdir().unwrap();
        atomic_write(&d.path().join("provenance.json"), b"{not json").unwrap();
        assert!(load(d.path()).is_empty());
    }

    /// 导入那一条：没有来源路径
    #[test]
    fn imports_have_no_from() {
        let d = tempfile::tempdir().unwrap();
        record(
            d.path(),
            "presets-mine/外来的.toml",
            None,
            ProvenanceKind::Import,
        )
        .unwrap();
        let all = load(d.path());
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].kind, "import");
        assert!(all[0].from.is_none());
    }
}
