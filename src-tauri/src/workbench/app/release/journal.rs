//! **发布日志**（只追加、只作审计）。
//!
//! 它**不参与任何判断** —— 状态一律从远端现读（见 [`super::state`]）。
//! 这里只回答「上一趟停在哪、哪一步花了多久」，给人和事后排查用。
//!
//! 落点：`target/release-journal.jsonl`（`target/` 是构建产物目录，git 看不见）。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// 一条事件。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub at: String,
    pub version: String,
    pub phase: String,
    pub step: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// 日志文件（相对仓库根）。
pub fn path_of(repo_root: &Path) -> PathBuf {
    repo_root.join("target").join("release-journal.jsonl")
}

/// 追加一条。**尽力而为**：写不进去只记警告，不把发布判失败（账本不是发布的一部分）。
pub fn append(repo_root: &Path, entry: &Entry) {
    let path = path_of(repo_root);
    let line = match serde_json::to_string(entry) {
        Ok(l) => l,
        Err(e) => {
            tracing::warn!("发布日志序列化失败：{e}");
            return;
        }
    };
    /* 只追加这件事，在小文件上用「读出来 + 拼一行 + 原子写」表达：
    走仓库唯一的写盘出口（fsx::atomic），不用靠 append 句柄赌断电时的完整性。 */
    let mut text = std::fs::read_to_string(&path).unwrap_or_default();
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&line);
    text.push('\n');
    if let Err(e) = crate::fsx::atomic::atomic_write(&path, text.as_bytes()) {
        tracing::warn!("发布日志写不进去：{}", e.message);
    }
}

/// 读到最近 N 条（只给人看，不做判断）。
pub fn tail(repo_root: &Path, n: usize) -> Vec<Entry> {
    let Ok(text) = std::fs::read_to_string(path_of(repo_root)) else {
        return Vec::new();
    };
    let mut out: Vec<Entry> = text
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    let start = out.len().saturating_sub(n);
    out.split_off(start)
}
