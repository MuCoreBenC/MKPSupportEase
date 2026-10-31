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

/// 这一份用户文件是**怎么来的**（跨 IPC 的稳定词）。
///
/// `Unknown` 专门给"账是旧格式 / 从来没记过"那一档：旧账只有路径 + 源身份，
/// 分不出下载还是拷贝；**不替它挑一个**（挑成 `copy` 或 `download` 都是编历史）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OriginKind {
    /// 从云端下载落成的本地文件（用户看到的"来自云端"就是这一档）
    Download,
    /// 用户用「另存为一份新的」拷出来的（母本在 `originPath`）
    Copy,
    /// 从外部导入的（拖拽 / 文件选择器）
    Import,
    /// 旧账 / 没记过：来路说不清，界面照实说「未知」
    Unknown,
}

impl OriginKind {
    pub fn as_str(self) -> &'static str {
        match self {
            OriginKind::Download => "download",
            OriginKind::Copy => "copy",
            OriginKind::Import => "import",
            OriginKind::Unknown => "unknown",
        }
    }

    fn unknown() -> Self {
        OriginKind::Unknown
    }
}

/// **一份本地文件的完整来路**（`run/lineage-source.json` 的一条）。
///
/// 它回答四件不同的事，一格都不许互相顶替：
///
/// ```text
/// kind / identity / originPath   这份从哪来（云端哪个源 / 本地哪一份母本 / 导入）
/// catalogId / serial / revision  对应云端目录的发布身份
/// publishedAt                    ★ 下载那一刻那一代目录的发布时刻 = 界面的「本地版本时间」
/// sourceSha256                   落盘的那一版云端原文摘要（不是本地文件此刻的摘要）
/// ```
///
/// `publishedAt` 必须在**下载那一刻**定格：云端后来换代，这一格一个字都不改。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalProvenance {
    /// 用户线那份的相对路径（`presets-local/xxx.toml`）
    pub path: String,
    /// 来路。旧账没有这一格 ⇒ `Unknown`（不猜）
    #[serde(default = "OriginKind::unknown")]
    pub kind: OriginKind,
    /// 这份是在哪个源下建出来的（`平台#仓库@ref`）。下载时取当时的源；拷贝继承母本
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<SourceIdentity>,
    /// 拷贝的母本（相对用户根）。下载 / 导入没有
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin_path: Option<String>,
    /// 云端目录里的文件名（认亲用：目录换路径也还认得出是同一份）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cloud_file: Option<String>,
    /// 下载那一刻目录的 `catalogId`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog_id: Option<String>,
    /// 下载那一刻目录的 `publishSerial`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serial: Option<u64>,
    /// 下载那一刻目录的 `revision`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    /// ★ 下载那一刻目录的 `publishedAt`（RFC3339）—— 「本地版本时间」的真值
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published_at: Option<String>,
    /// 落盘的那一版**云端原文**的 SHA-256（不是本地文件此刻的字节）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_sha256: Option<String>,
    /// 记账时刻（UTC epoch 秒）—— 下载 / 拷贝 / 导入实际发生的那一刻
    pub at: u64,
}

/// 一次下载要抄进账本的目录身份（由调用方从**下载那一刻**的目录取，读侧不再拼）。
#[derive(Debug, Clone, Copy)]
pub struct CatalogStamp<'a> {
    pub catalog_id: Option<&'a str>,
    pub serial: Option<u64>,
    pub revision: &'a str,
    pub published_at: Option<&'a str>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct NotesLedger {
    #[serde(default)]
    notes: Vec<SourceNote>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct CopiesLedger {
    #[serde(default)]
    entries: Vec<LocalProvenance>,
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

/// 写这两本账（来路 / 目录）时全进程排队（2026-10-29）。
///
/// 与 [`super::delivery_index::record_file`] 同一个理由：前端允许**并行下载**之后，
/// 两次落盘会同时读整本、各自插一条、再整本写回 —— 后写的把先写的那条吃掉，
/// 那一份于是"下过但账上没有"（下次还要再取一遍网络，且来源说不清）。
///
/// 只包"读-改-写"，不包网络与写字节。中毒（某次 panic）不废弃这本账：重读一遍就自愈。
fn write_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
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
    let _guard = write_lock();
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

/// 读本地来路账。读不到 / 坏档 ⇒ 空表
pub fn load_provenance(root: &Path) -> Vec<LocalProvenance> {
    let Ok(bytes) = std::fs::read(copies_path(root)) else {
        return Vec::new();
    };
    serde_json::from_slice::<CopiesLedger>(&bytes)
        .map(|l| l.entries)
        .unwrap_or_default()
}

/// 整本按路径建索引（调用方一次读、多次查；别在每份文件里各读一遍账）
pub fn index_by_path(
    entries: &[LocalProvenance],
) -> std::collections::HashMap<&str, &LocalProvenance> {
    entries.iter().map(|e| (e.path.as_str(), e)).collect()
}

/// 这一份的来路（没有记账 ⇒ `None`，判定退回"说不清"）
pub fn provenance_of(root: &Path, path: &str) -> Option<LocalProvenance> {
    load_provenance(root).into_iter().find(|e| e.path == path)
}

/// 这一份当初是在哪个源生效时建的（没有记账 ⇒ `None`）
pub fn source_of(root: &Path, path: &str) -> Option<SourceIdentity> {
    provenance_of(root, path).and_then(|e| e.identity)
}

/// **下载落盘后记一笔**（`fetch_cloud_preset` 的两条落盘支都走它）。
///
/// `identity` 只在**真的从云端取回了字节**时给；盘上字节本来就在、这次只是复用时
/// 传 `None` —— 那些字节可能是别的源取的，拿当前源填进去就是编历史。
pub fn record_download(
    root: &Path,
    path: &str,
    identity: Option<&SourceIdentity>,
    cloud_file: &str,
    stamp: &CatalogStamp<'_>,
    source_sha256: Option<&str>,
) -> Result<(), AppError> {
    let _guard = write_lock();
    let mut ledger = CopiesLedger {
        entries: load_provenance(root),
    };
    ledger.entries.retain(|e| e.path != path);
    ledger.entries.push(LocalProvenance {
        path: path.to_owned(),
        kind: OriginKind::Download,
        identity: identity.cloned(),
        origin_path: None,
        cloud_file: Some(cloud_file.to_owned()),
        catalog_id: stamp.catalog_id.map(str::to_owned),
        serial: stamp.serial,
        revision: Some(stamp.revision.to_owned()),
        published_at: stamp.published_at.map(str::to_owned),
        source_sha256: source_sha256.map(str::to_owned),
        at: now_secs(),
    });
    save(&copies_path(root), &ledger)
}

/// **更新把这份文件推进到新基线之后**，把账上的目录四格与摘要跟着推。
///
/// 身份 / 母本 / 云端文件名原样保留：更新换的是"对应哪一版"，不是"从哪来"。
/// 账上没有这一条（老文件第一次更新）⇒ 按下载记一条 —— 内容确实来自当前云端。
pub fn record_baseline(
    root: &Path,
    path: &str,
    identity: Option<&SourceIdentity>,
    cloud_file: &str,
    stamp: &CatalogStamp<'_>,
    source_sha256: Option<&str>,
) -> Result<(), AppError> {
    let _guard = write_lock();
    let mut ledger = CopiesLedger {
        entries: load_provenance(root),
    };
    let previous = ledger.entries.iter().find(|e| e.path == path).cloned();
    ledger.entries.retain(|e| e.path != path);
    ledger.entries.push(LocalProvenance {
        path: path.to_owned(),
        kind: previous.as_ref().map_or(OriginKind::Download, |e| e.kind),
        identity: previous
            .as_ref()
            .and_then(|e| e.identity.clone())
            .or_else(|| identity.cloned()),
        origin_path: previous.as_ref().and_then(|e| e.origin_path.clone()),
        cloud_file: previous
            .as_ref()
            .and_then(|e| e.cloud_file.clone())
            .or_else(|| Some(cloud_file.to_owned())),
        catalog_id: stamp.catalog_id.map(str::to_owned),
        serial: stamp.serial,
        revision: Some(stamp.revision.to_owned()),
        published_at: stamp.published_at.map(str::to_owned),
        source_sha256: source_sha256.map(str::to_owned),
        /* 更新不是"重新落盘"：这份文件当初落地的那一刻不跟着变 */
        at: previous.as_ref().map_or_else(now_secs, |e| e.at),
    });
    save(&copies_path(root), &ledger)
}

/// **另存为一份新的**：母本账上有多少格，副本就继承多少格（母本没有 ⇒ 副本也没有）。
///
/// 副本没有新的下载事件 —— `at` 是拷贝时刻，`originPath` 指向母本。
pub fn record_copy(
    root: &Path,
    path: &str,
    mother_path: &str,
    mother: Option<&LocalProvenance>,
) -> Result<(), AppError> {
    let _guard = write_lock();
    let mut ledger = CopiesLedger {
        entries: load_provenance(root),
    };
    ledger.entries.retain(|e| e.path != path);
    ledger.entries.push(LocalProvenance {
        path: path.to_owned(),
        kind: OriginKind::Copy,
        identity: mother.and_then(|e| e.identity.clone()),
        origin_path: Some(mother_path.to_owned()),
        cloud_file: mother.and_then(|e| e.cloud_file.clone()),
        catalog_id: mother.and_then(|e| e.catalog_id.clone()),
        serial: mother.and_then(|e| e.serial),
        revision: mother.and_then(|e| e.revision.clone()),
        published_at: mother.and_then(|e| e.published_at.clone()),
        source_sha256: mother.and_then(|e| e.source_sha256.clone()),
        at: now_secs(),
    });
    save(&copies_path(root), &ledger)
}

/// **导入**：账上只记"它是导入来的"，没有源身份 / 目录四格（外部文件不来自云端）。
pub fn record_import(root: &Path, path: &str) -> Result<(), AppError> {
    let _guard = write_lock();
    let mut ledger = CopiesLedger {
        entries: load_provenance(root),
    };
    ledger.entries.retain(|e| e.path != path);
    ledger.entries.push(LocalProvenance {
        path: path.to_owned(),
        kind: OriginKind::Import,
        identity: None,
        origin_path: None,
        cloud_file: None,
        catalog_id: None,
        serial: None,
        revision: None,
        published_at: None,
        source_sha256: None,
        at: now_secs(),
    });
    save(&copies_path(root), &ledger)
}

/// 哪几份是用户复制出来的（设置页「复制出来的也跟着云端更新」关着时要把它们排除）
pub fn copy_paths(root: &Path) -> std::collections::HashSet<String> {
    load_provenance(root)
        .into_iter()
        .filter(|e| e.kind == OriginKind::Copy)
        .map(|e| e.path)
        .collect()
}

/// 文件改名时把条目一起挪：自己改了名 `path` 跟走，母本改了名 `originPath` 跟走
/// （账记的是历史，路径是现在）。
pub fn repath(root: &Path, from: &str, to: &str) -> Result<(), AppError> {
    let _guard = write_lock();
    let mut entries = load_provenance(root);
    if !entries
        .iter()
        .any(|e| e.path == from || e.origin_path.as_deref() == Some(from))
    {
        return Ok(());
    }
    for e in &mut entries {
        if e.path == from {
            e.path = to.to_owned();
        }
        if e.origin_path.as_deref() == Some(from) {
            e.origin_path = Some(to.to_owned());
        }
    }
    save(&copies_path(root), &CopiesLedger { entries })
}

/// 这份用户文件不在了：把它的来路整条移除。
///
/// 不删的话，下一次同路径出现一份**手工放进来**的文件会继承旧的来源账 ——
/// 那是"元数据复活"，与 delivery 复活是同一类错误。
pub fn remove(root: &Path, path: &str) -> Result<(), AppError> {
    let _guard = write_lock();
    let mut entries = load_provenance(root);
    let before = entries.len();
    entries.retain(|e| e.path != path);
    if entries.len() == before {
        return Ok(());
    }
    save(&copies_path(root), &CopiesLedger { entries })
}

/// 从回收站恢复：把删那一刻的来路原样写回（同路径先清掉旧的，避免两份）。
pub fn restore(root: &Path, entry: &LocalProvenance) -> Result<(), AppError> {
    let _guard = write_lock();
    let mut entries = load_provenance(root);
    entries.retain(|e| e.path != entry.path);
    entries.push(entry.clone());
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

    fn stamp() -> CatalogStamp<'static> {
        CatalogStamp {
            catalog_id: Some("support-ease"),
            serial: Some(7),
            revision: "rev-7",
            published_at: Some("2026-10-27T07:25:50Z"),
        }
    }

    /// 下载：目录四格在下载那一刻定格；改名跟着走
    #[test]
    fn a_download_keeps_the_stamp_and_follows_its_path() {
        let root = tempfile::tempdir().unwrap();
        let path = "presets-local/A1.toml";
        assert!(
            provenance_of(root.path(), path).is_none(),
            "没记过就是不知道"
        );
        record_download(
            root.path(),
            path,
            Some(&gh()),
            "A1.toml",
            &stamp(),
            Some("sha-a"),
        )
        .unwrap();
        record_download(
            root.path(),
            path,
            Some(&gh()),
            "A1.toml",
            &stamp(),
            Some("sha-a"),
        )
        .unwrap();
        assert_eq!(load_provenance(root.path()).len(), 1, "同一路径不留两条");
        let e = provenance_of(root.path(), path).unwrap();
        assert_eq!(e.kind, OriginKind::Download);
        assert_eq!(e.identity, Some(gh()));
        assert_eq!(e.published_at.as_deref(), Some("2026-10-27T07:25:50Z"));
        assert_eq!(e.serial, Some(7));

        repath(root.path(), path, "presets-local/B1.toml").unwrap();
        assert!(provenance_of(root.path(), path).is_none(), "旧名字不再有人");
        assert_eq!(
            provenance_of(root.path(), "presets-local/B1.toml")
                .unwrap()
                .path,
            "presets-local/B1.toml"
        );
    }

    /// 拷贝：继承母本的源身份与目录四格，但母本路径指向母本；母本改名后链接跟着走
    #[test]
    fn a_copy_inherits_the_mother_and_follows_her_name() {
        let root = tempfile::tempdir().unwrap();
        let mother = "presets-local/A1.toml";
        record_download(
            root.path(),
            mother,
            Some(&gh()),
            "A1.toml",
            &stamp(),
            Some("sha-a"),
        )
        .unwrap();
        let m = provenance_of(root.path(), mother).unwrap();
        record_copy(root.path(), "presets-local/A1 copy.toml", mother, Some(&m)).unwrap();

        let c = provenance_of(root.path(), "presets-local/A1 copy.toml").unwrap();
        assert_eq!(c.kind, OriginKind::Copy);
        assert_eq!(c.origin_path.as_deref(), Some(mother));
        assert_eq!(c.identity, Some(gh()));
        assert_eq!(c.published_at.as_deref(), Some("2026-10-27T07:25:50Z"));

        repath(root.path(), mother, "presets-local/A1-new.toml").unwrap();
        assert_eq!(
            provenance_of(root.path(), "presets-local/A1 copy.toml")
                .unwrap()
                .origin_path
                .as_deref(),
            Some("presets-local/A1-new.toml")
        );
    }

    /// 旧格式（只有 path + identity + at）读成 Unknown：不替它挑一个来路
    #[test]
    fn an_old_entry_reads_as_unknown() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("run")).unwrap();
        atomic_write(
            &root.path().join(COPIES_FILE),
            br#"{"entries":[{"path":"presets-local/A1.toml","identity":{"platform":"github","repo":"o/r","gitRef":"main"},"at":1}]}"#,
        )
        .unwrap();
        let e = provenance_of(root.path(), "presets-local/A1.toml").unwrap();
        assert_eq!(e.kind, OriginKind::Unknown);
        assert_eq!(e.identity, Some(gh()), "旧账里的源身份是当时记的，不丢");
        assert!(e.published_at.is_none(), "旧账没有发布时刻 —— 未知");
    }

    /// 更新推进基线：身份不动，目录四格与摘要前进
    #[test]
    fn advancing_the_baseline_keeps_the_origin() {
        let root = tempfile::tempdir().unwrap();
        let path = "presets-local/A1.toml";
        record_copy(root.path(), path, "presets-local/mother.toml", None).unwrap();
        let next = CatalogStamp {
            catalog_id: Some("support-ease"),
            serial: Some(8),
            revision: "rev-8",
            published_at: Some("2026-10-29T04:54:13Z"),
        };
        record_baseline(root.path(), path, None, "A1.toml", &next, Some("sha-b")).unwrap();
        let e = provenance_of(root.path(), path).unwrap();
        assert_eq!(e.kind, OriginKind::Copy, "更新不改来路");
        assert_eq!(e.origin_path.as_deref(), Some("presets-local/mother.toml"));
        assert_eq!(e.serial, Some(8));
        assert_eq!(e.published_at.as_deref(), Some("2026-10-29T04:54:13Z"));
    }

    /// 坏档按空账：读不出来不许让任何判定失败，更不许覆盖别的东西
    #[test]
    fn a_corrupt_ledger_reads_as_empty() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("run")).unwrap();
        atomic_write(&root.path().join(NOTES_FILE), b"{ not json").unwrap();
        atomic_write(&root.path().join(COPIES_FILE), b"]").unwrap();
        assert!(load_notes(root.path()).is_empty());
        assert!(load_provenance(root.path()).is_empty());
        assert!(provenance_of(root.path(), "presets-local/A1.toml").is_none());
        /* 空账之上还能正常记账（坏档不挡路） */
        note_seen(root.path(), &gh(), None, "rev-a").unwrap();
        assert_eq!(load_notes(root.path()).len(), 1);
    }
}
