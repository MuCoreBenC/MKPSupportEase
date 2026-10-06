//! **预设事件时间模型**：凡是界面上要"永久解释"的时间，先定义它属于哪个事件，
//! 事件发生的那一刻把时刻定格进账 —— 之后永远只追加，不回写。
//!
//! # 为什么要有这本账（mtime 退役的理由）
//!
//! 以前这些业务时间全靠文件 mtime 顶：下载时间 = mtime、归档时间 = mtime。
//! mtime 只回答"这个文件系统对象最后什么时候被改"，它会被复制、同步、触碰改写，
//! 也说不出"这份字节属于哪一代目录"—— 拿它解释业务，就是 2026-10-06 那批
//! 「下载时间比云端更新还早」「旧版本的时间是现在」的根子。业务语义全部来自事件，
//! mtime 从此只归文件系统。
//!
//! # 三条硬规则（作者定，2026-10-06）
//!
//! ① **一个事件只有一个语义** —— 不许一个字段今天叫下载时间、明天叫归档时间；
//! ② **事件发生时定格** —— `at` 写进去之后，云端换代、文件挪动、再次扫描，都不能改变它；
//! ③ **没发生过 / 不知道，就没有** —— 读侧给 `null`，界面写「未知」，
//!    **禁止** `事件时刻 ?? mtime` 这种猜。
//!
//! # 五个事件与各自的记法（第二阶段裁决：哪些落账、哪些反查）
//!
//! | 事件 | 记法 |
//! |---|---|
//! | `ReleasePublished`（发布） | **不落账**：发布侧盖进目录的 `publishedAt` 随目录字节与版本链（`archive/catalogs/`）走到每台客户端，按字节指纹反查即得 |
//! | `DeliveryDownloaded`（下载） | **落这本账**：交付文件第一次落进本机的那一刻 |
//! | `DeliveryReplaced`（替换） | **落这本账**：旧份进归档、新份落盘的同一刻 |
//! | `MineModified`（我的文件修改） | **不落账**：按作者裁决就是文件系统 mtime —— 声明在这里，免得它再被派别的用场 |
//! | `ProvenanceRecorded`（出处） | 已有自己的账（`provenance.json`），不动 |
//!
//! 账建立之前的存量（盘上已有的下载文件、归档）由 [`super::delivery`] 一次性
//! bootstrap 进来（见 [`super::delivery::load_events`]），之后永远追加。
//! 账是**附加信息**：读不到 / 坏档一律当空表，不许让任何读失败 —— 界面退回「未知」，
//! 什么都不坏（与 `provenance.json` 同一条口径）。

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::fsx::atomic::atomic_write;

/// 预设系统里需要被永久解释的事件。**每个变体一个语义**（硬规则①），
/// 字段只带自己需要的 —— 不做通用事件信封（actor / context / metadata 那套是过度设计）。
///
/// 时间一律 UTC epoch 秒（与全库同一口径，界面自己转本地时区）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PresetEvent {
    /// **下载**：这份字节第一次落进本机（此前盘上没有它）。
    DeliveryDownloaded {
        /// 相对内部根的路径 = catalog `files[].path`（唯一路径语义，2026-10-04）
        file: String,
        /// 落下去的字节指纹。事件跟着**字节**走：同一份文件换过几版，就有几条各自的事件
        sha256: String,
        /// 下载时生效的那一代目录指纹。读不到（随包目录损坏等）= `null`，不编
        revision: Option<String>,
        /// 事件时刻，写定之后永不再改（硬规则②）
        at: u64,
    },
    /// **替换**：这份字节换掉了盘上的旧份（旧份进归档、新份落盘的同一刻）。
    ///
    /// 一条事件同时解释两件事，各答各的格子，互不顶替：
    /// - 归档里那份旧字节（`old_sha256`）的「替换时间」= 本事件的 `at`；
    /// - 盘上这份新字节（`new_sha256`）的「到位时间」也是它。
    DeliveryReplaced {
        file: String,
        /// 被换下的字节指纹（进归档的那份）
        old_sha256: String,
        /// 被换下那一版属于哪代目录。**查不出 = `null`** —— 认不出的字节不猜（硬规则③）
        old_revision: Option<String>,
        /// 换上去的字节指纹。存量 bootstrap 记不到时 = `null`
        new_sha256: Option<String>,
        new_revision: Option<String>,
        at: u64,
    },
}

/// 这份字节是**怎么到的**。界面上「下载时间 / 替换时间」两个标签跟着它走 ——
/// 标签是事件的名字，不是 UI 自己挑的。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrival {
    /// 这份字节是下载落盘的（`DeliveryDownloaded.at`）
    Downloaded(u64),
    /// 这份字节是替换上去的（`DeliveryReplaced.at`）
    Replaced(u64),
}

impl Arrival {
    pub fn at(self) -> u64 {
        match self {
            Arrival::Downloaded(at) | Arrival::Replaced(at) => at,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Ledger {
    #[serde(default)]
    schema: u8,
    #[serde(default)]
    events: Vec<PresetEvent>,
}

/// 账本落点：内部根下的 `preset_events.json`。它说的是**程序自己管的下载区**的事，
/// 与 catalog / archive 住同一层（用户文件的出处才住用户根，那本是 `provenance.json`）
fn ledger_path(internal_root: &Path) -> std::path::PathBuf {
    internal_root.join("preset_events.json")
}

/// 账建过了吗（bootstrap 只在没建过时跑一次的判据）
pub fn is_initialized(internal_root: &Path) -> bool {
    ledger_path(internal_root).exists()
}

/// 读账。读不到（还没建账）/ 坏档 ⇒ 空表 —— 事件是附加信息，缺了不许让任何读失败
pub fn load(internal_root: &Path) -> Vec<PresetEvent> {
    let Ok(bytes) = std::fs::read(ledger_path(internal_root)) else {
        return Vec::new();
    };
    serde_json::from_slice::<Ledger>(&bytes)
        .map(|l| l.events)
        .unwrap_or_default()
}

/// 建账（bootstrap 专用）：只在账文件**还不存在**时写入 —— 已有账绝不回写（硬规则②）。
/// 两个读同时建账会各写一份内容相同的东西（同一块盘、同一批 mtime），后写的盖上先写的，无害
pub fn init(internal_root: &Path, events: Vec<PresetEvent>) {
    if is_initialized(internal_root) {
        return;
    }
    let _ = save(internal_root, &Ledger { schema: 1, events });
}

/// 追加一条。只追加，不盖旧的（同一份文件换几版就有几条，历史不合并 ——
/// 合并就等于改写历史）。写不进去不影响主流程：事件是附加信息，调用方不因它失败
pub fn append(internal_root: &Path, event: PresetEvent) {
    let mut ledger = Ledger {
        schema: 1,
        events: load(internal_root),
    };
    ledger.events.push(event);
    if let Err(e) = save(internal_root, &ledger) {
        tracing::warn!("事件账写不进去（界面时间退回「未知」，不影响主流程）：{e}");
    }
}

/// 现在（UTC epoch 秒）。事件时刻的唯一取值处 —— 取的是**事件发生那一刻**
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// 这份字节（file + sha）是**怎么到的、什么时候到的** —— 读侧唯一入口。
/// 事件按时间序追加，取**最后**一条匹配（同一份字节被删了重下，以最后一次为准）
pub fn arrival_of(events: &[PresetEvent], file: &str, sha256: &str) -> Option<Arrival> {
    let mut hit = None;
    for event in events {
        match event {
            PresetEvent::DeliveryDownloaded {
                file: f,
                sha256: s,
                at,
                ..
            } if f == file && s == sha256 => hit = Some(Arrival::Downloaded(*at)),
            PresetEvent::DeliveryReplaced {
                file: f,
                new_sha256: Some(s),
                at,
                ..
            } if f == file && s == sha256 => hit = Some(Arrival::Replaced(*at)),
            _ => {}
        }
    }
    hit
}

/// 归档里那份旧字节是**什么时候被换下来的**（`DeliveryReplaced.at`，按 `old_sha256` 对号）
pub fn replaced_at_of(events: &[PresetEvent], file: &str, old_sha256: &str) -> Option<u64> {
    let mut hit = None;
    for event in events {
        if let PresetEvent::DeliveryReplaced {
            file: f,
            old_sha256: s,
            at,
            ..
        } = event
        {
            if f == file && s == old_sha256 {
                hit = Some(*at);
            }
        }
    }
    hit
}

fn save(internal_root: &Path, ledger: &Ledger) -> Result<(), crate::error::AppError> {
    let body = serde_json::to_vec_pretty(ledger).map_err(|e| {
        crate::error::AppError::internal("事件账序列化不了").with_detail(e.to_string())
    })?;
    atomic_write(&ledger_path(internal_root), &body)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn downloaded(file: &str, sha: &str, at: u64) -> PresetEvent {
        PresetEvent::DeliveryDownloaded {
            file: file.to_owned(),
            sha256: sha.to_owned(),
            revision: Some("gen-1".to_owned()),
            at,
        }
    }

    #[test]
    fn append_then_load_roundtrip() {
        let d = tempfile::tempdir().unwrap();
        append(d.path(), downloaded("mkp/A.toml", "aa", 100));
        append(
            d.path(),
            PresetEvent::DeliveryReplaced {
                file: "mkp/A.toml".to_owned(),
                old_sha256: "aa".to_owned(),
                old_revision: Some("gen-1".to_owned()),
                new_sha256: Some("bb".to_owned()),
                new_revision: Some("gen-2".to_owned()),
                at: 200,
            },
        );
        let all = load(d.path());
        assert_eq!(all.len(), 2);
        assert!(matches!(
            &all[1],
            PresetEvent::DeliveryReplaced { at: 200, .. }
        ));
    }

    /// 硬规则③的读侧：没有事件就是没有（`None`），读侧不许拿别的东西顶
    #[test]
    fn unknown_bytes_have_no_arrival() {
        let d = tempfile::tempdir().unwrap();
        append(d.path(), downloaded("mkp/A.toml", "aa", 100));
        assert!(arrival_of(&load(d.path()), "mkp/A.toml", "bb").is_none());
        assert!(arrival_of(&load(d.path()), "mkp/B.toml", "aa").is_none());
    }

    /// 同一份字节删了重下：以最后一次为准（事件按时间序追加，取最后一条匹配）
    #[test]
    fn re_download_takes_the_latest_event() {
        let d = tempfile::tempdir().unwrap();
        append(d.path(), downloaded("mkp/A.toml", "aa", 100));
        append(d.path(), downloaded("mkp/A.toml", "aa", 900));
        assert_eq!(
            arrival_of(&load(d.path()), "mkp/A.toml", "aa"),
            Some(Arrival::Downloaded(900))
        );
    }

    /// 替换事件对两个 sha 都答得出「到位时间」；归档那份按 old_sha 对「替换时间」
    #[test]
    fn replaced_event_answers_both_sides() {
        let d = tempfile::tempdir().unwrap();
        append(
            d.path(),
            PresetEvent::DeliveryReplaced {
                file: "mkp/A.toml".to_owned(),
                old_sha256: "old".to_owned(),
                old_revision: None,
                new_sha256: Some("new".to_owned()),
                new_revision: Some("gen-2".to_owned()),
                at: 300,
            },
        );
        let events = load(d.path());
        assert_eq!(
            arrival_of(&events, "mkp/A.toml", "new"),
            Some(Arrival::Replaced(300))
        );
        assert_eq!(replaced_at_of(&events, "mkp/A.toml", "old"), Some(300));
    }

    /// 坏档 ⇒ 空表：事件缺了不许让用户线的任何读失败
    #[test]
    fn a_broken_ledger_reads_as_empty() {
        let d = tempfile::tempdir().unwrap();
        atomic_write(&d.path().join("preset_events.json"), b"{not json").unwrap();
        assert!(load(d.path()).is_empty());
    }
}
