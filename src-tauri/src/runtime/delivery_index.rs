//! **当前云端目录在本机的那一张索引**：`path → 手上这一份的 sha`。
//!
//! # 为什么需要它
//!
//! 取消 `delivery/` 里的文件副本之后，"这一份在不在本机、是哪一版"不能再问盘上
//! 有没有同名文件 —— 那正是"磁盘上恰好有同名文件就冒充下载成功"的来源。
//! 索引只回答两件事：
//!
//! - 这台机器**手上**是哪一版（`files[path].sha256`）；
//! - 当前目录是哪一代（`revision / catalogId / publishSerial / publishedAt`）。
//!
//! 字节本身在 [`super::content`] 里按 SHA 存一份；索引不存内容。
//!
//! # 换代 / 换源
//!
//! 目录换代**只改身份四格**，`files` 由 [`sync`] 按新目录收口：
//! - 新目录里有的 path：若 `content/<sha>` 已在，补一条（共享内容直接认）；
//! - 新目录里没有的 path：删掉索引条目（内容是否还在由引用清理判，不在这里删）。
//!
//! 因此换代不会丢内容，也不会把旧版冒充成当前版。
//!
//! ★ **"内容已在"只在同一条线、同一代目录里算数**（2026-10-29）：`content/` 里有这个
//! SHA 只说明这份内容在本机（规则⑦：同内容一份），**不说明它是这条线这一代取的**。
//! 换代 / 换源之后先不认领，等真实下载走 [`record_file`] 记一笔（规则⑧：来源只由
//! 真实操作记录）。换源那一刻磁盘上留着的旧条目**不删** —— 那些字节确实在，
//! 只是它们代表"另外那条线上的一版"，与目录登记值不同就自然判成 `Stale`。

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use postprocess::diag::messages;

use super::content;

/// 索引落点：内部根的 `run/` 下。
const INDEX_FILE: &str = "run/delivery-index.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    /// 手上这一份的 SHA-256（`content/<sha>.toml`）
    pub sha256: String,
    /// 字节数（目录登记值；拿不到就不写）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    /// 这一份落到本机的时刻（UTC epoch 秒；只给排查，不上界面）
    pub at: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryIndex {
    #[serde(default)]
    pub revision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publish_serial: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published_at: Option<String>,
    /// 这份索引是按**哪个数据源**建立的（[`super::source_identity`] 的 key）。
    ///
    /// 它只服务一道判据：换源之后**不许**拿"`content/` 里有同 SHA"去认领新目录里的空位
    /// （那会把"从 A 源下过"冒充成"B 源这一份已下"）。`None` = 认不出 / 老索引没有这格，
    /// 那种情况下拿不出反证，按"同一条线"处理。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// 这一代目录被本机认下的时刻（**只在身份四格变化时**前进，不是每次收口都写）
    #[serde(default)]
    pub seen_at: u64,
    /// path（相对内部根，`delivery/mkp/presets/…`）→ 手上那一份
    #[serde(default)]
    pub files: BTreeMap<String, Entry>,
}

fn index_path(root: &Path) -> std::path::PathBuf {
    root.join(INDEX_FILE)
}

/// 读索引。读不到 / 坏档 ⇒ 空索引（它是可重建的本地映射，不让读失败拦住启动）。
pub fn load(root: &Path) -> DeliveryIndex {
    let Ok(bytes) = std::fs::read(index_path(root)) else {
        return DeliveryIndex::default();
    };
    serde_json::from_slice(&bytes).unwrap_or_else(|e| {
        tracing::warn!("目录索引读不出来，按空索引处理：{e}");
        DeliveryIndex::default()
    })
}

fn save(root: &Path, index: &DeliveryIndex) -> Result<(), AppError> {
    let bytes = serde_json::to_vec_pretty(index).map_err(|e| {
        AppError::io(messages::text("ui.storage.index_write_failed", &[]))
            .with_detail(e.to_string())
    })?;
    crate::fsx::atomic::atomic_write(&index_path(root), &bytes)
}

/// 一份 path 当前手上是哪一版。
pub fn entry(root: &Path, path: &str) -> Option<Entry> {
    load(root).files.get(path).cloned()
}

/// 记下"这一 path 手上是这一版"。目录身份由 [`sync`] 单独刷新
/// （下载管道只拿得到 `CatalogFile`，拿不到整份目录）。
pub fn record_file(
    root: &Path,
    path: &str,
    sha256: &str,
    size: Option<u64>,
) -> Result<(), AppError> {
    let _guard = write_lock();
    let mut index = load(root);
    index.files.insert(
        path.to_owned(),
        Entry {
            sha256: sha256.to_ascii_lowercase(),
            size,
            at: now_secs(),
        },
    );
    save(root, &index)
}

/// 删掉一条（不删内容；内容由引用清理决定去留）。
pub fn remove(root: &Path, path: &str) -> Result<(), AppError> {
    let _guard = write_lock();
    let mut index = load(root);
    if index.files.remove(path).is_some() {
        save(root, &index)?;
    }
    Ok(())
}

/// 写这本索引时全进程排队（2026-10-29）。
///
/// 前端现在允许**并行下载**（作者：「改成那种多发的、并行的那种操作」），于是两次
/// `deliver` 可能同时走到 [`record_file`]：各自 `load` → 插自己那条 → `save`，
/// 后写的那一份会把另一条**吃掉** —— 内容还躺在 `content/` 里，但索引上没有它，
/// 那一份等于白下（下次还要再取一遍网络）。
///
/// 锁只包住"读-改-写"这一小段，不包网络、不包写字节（那两件事本来就慢，串起来
/// 就白等）。中毒（某次写 panic）不让整条链废掉：这本账"重读一遍就能自愈"。
fn write_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// 当前生效的数据源身份（[`super::source_identity::SourceIdentity::key`]）。
///
/// 认不出来（开发构建没注入地址 / 老状态文件）= `None`：那种情况拿不出反证，
/// [`sync_for_source`] 按"同一条线"处理。
fn current_source(root: &Path) -> Option<String> {
    let source = super::app_state::preset_source(root).ok().flatten()?;
    super::source_identity::identity_of(&source).map(|i| i.key())
}

/// 按**当前目录**收口索引：更新身份四格、补上内容已在的条目（**只在同一条线同一代**，
/// 判据见下）、丢掉目录里没有的 path。
///
/// 只做映射，不动内容。返回 `true` = 这次有变化（调用方据此决定要不要广播）。
pub fn sync(root: &Path, catalog: &super::Catalog) -> Result<bool, AppError> {
    sync_for_source(root, catalog, current_source(root).as_deref())
}

/// 带显式源身份的 [`sync`]：判据要能摆得出"索引是 A 源建的，目录是 B 源的"这一局，
/// 所以源身份从外面递进来（线上由 [`sync`] 从 `appState.presetSource` 现算）。
pub fn sync_for_source(
    root: &Path,
    catalog: &super::Catalog,
    source: Option<&str>,
) -> Result<bool, AppError> {
    let _guard = write_lock();
    let mut index = load(root);
    let before = index.clone();

    /* 「凭内容认领」的两道闸（2026-10-29）：
     *
     * `content/` 里有这个 SHA 只说明**这份内容在本机**（规则⑦：同内容一份），
     * 不说明它是**这条线、这一代**取回来的。两道都对得上才认：
     *
     * - 同一条线：索引按当前数据源建立（换源之后不许把"从 A 源下过"当"B 源已下"）；
     * - 同一代：目录 revision 没变（换代之后要真实下载一趟，由 `record_file` 记）。
     *
     * 任一格"不知道"（老索引没记源 / 还没记过 revision）按"对得上"处理 ——
     * 那是首次建立与迁移的场景，盘上内容本来就该认。 */
    let same_line = index.source.as_deref().map_or(true, |s| Some(s) == source);
    let same_generation = index.revision.is_empty() || index.revision == catalog.revision;
    let may_adopt = same_line && same_generation;
    if !may_adopt {
        tracing::info!(
            same_line,
            same_generation,
            "换代 / 换源：本次不凭本地内容认领，等真实下载"
        );
    }

    let in_catalog: HashSet<&str> = catalog.files.iter().map(|f| f.path.as_str()).collect();
    index
        .files
        .retain(|path, _| in_catalog.contains(path.as_str()));

    for file in &catalog.files {
        let Some(sha) = file.expected_sha() else {
            /* 随包 bootstrap 目录不登记 SHA：保留已有条目，不凭内容猜 */
            continue;
        };
        if index.files.contains_key(&file.path) {
            continue;
        }
        if !may_adopt {
            continue;
        }
        /* 内容已经在（同 SHA 被别的 path 下过 / 迁移过来的）⇒ 这一份就是"已有" */
        if content::exists(root, sha) {
            let at = std::fs::metadata(content::content_path(root, sha))
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or_else(now_secs);
            index.files.insert(
                file.path.clone(),
                Entry {
                    sha256: sha.to_ascii_lowercase(),
                    size: file.expected_size(),
                    at,
                },
            );
        }
    }

    let generation_changed = index.revision != catalog.revision
        || index.catalog_id != catalog.catalog_id
        || index.publish_serial != catalog.publish_serial
        || index.published_at != catalog.published_at;

    index.revision = catalog.revision.clone();
    index.catalog_id = catalog.catalog_id.clone();
    index.publish_serial = catalog.publish_serial;
    index.published_at = catalog.published_at.clone();
    /* 认不出源就**别拿 None 盖掉**记着的那条线：那会让下一趟又放开认领 */
    if source.is_some() {
        index.source = source.map(str::to_owned);
    }
    if generation_changed {
        index.seen_at = now_secs();
    }

    if index == before {
        return Ok(false);
    }
    save(root, &index)?;
    Ok(true)
}

/// 索引里所有被引用的 SHA（清理内容时用）。
pub fn referenced_shas(root: &Path) -> HashSet<String> {
    load(root)
        .files
        .values()
        .map(|e| e.sha256.to_ascii_lowercase())
        .collect()
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::catalog::{Catalog, CatalogFile};
    use sha2::{Digest, Sha256};

    fn file(name: &str, text: &str) -> CatalogFile {
        CatalogFile {
            kind: super::super::catalog::kind::PRESET.to_owned(),
            file_name: name.to_owned(),
            path: format!("delivery/mkp/presets/{name}"),
            machine_id: "A1".to_owned(),
            version_id: "FAST".to_owned(),
            sha256: Some(super::super::catalog::hex(&Sha256::digest(text.as_bytes()))),
            size: Some(text.len() as u64),
        }
    }

    fn catalog(files: Vec<CatalogFile>, revision: &str) -> Catalog {
        Catalog {
            revision: revision.to_owned(),
            files,
            ..Catalog::default()
        }
    }

    #[test]
    fn syncing_recognizes_content_that_is_already_there() {
        let d = tempfile::tempdir().unwrap();
        let f = file("A1.toml", "x");
        content::ensure(d.path(), f.expected_sha().unwrap(), b"x").unwrap();
        let c = catalog(vec![f.clone()], "r1");
        assert!(sync(d.path(), &c).unwrap());
        assert_eq!(
            entry(d.path(), &f.path).unwrap().sha256,
            f.expected_sha().unwrap()
        );
    }

    #[test]
    fn a_new_catalog_drops_paths_it_no_longer_has_but_keeps_content() {
        let d = tempfile::tempdir().unwrap();
        let f = file("A1.toml", "x");
        let sha = f.expected_sha().unwrap().to_owned();
        content::ensure(d.path(), &sha, b"x").unwrap();
        sync(d.path(), &catalog(vec![f.clone()], "r1")).unwrap();

        let other = file("A2.toml", "y");
        sync(d.path(), &catalog(vec![other], "r2")).unwrap();
        assert!(
            entry(d.path(), &f.path).is_none(),
            "目录里没有的 path 退出索引"
        );
        assert!(content::exists(d.path(), &sha), "但内容不在这里删");
    }

    #[test]
    fn content_from_another_line_is_not_counted_as_downloaded() {
        let d = tempfile::tempdir().unwrap();
        let f = file("A1.toml", "x");
        let sha = f.expected_sha().unwrap().to_owned();
        content::ensure(d.path(), &sha, b"x").unwrap();

        /* A 源建的索引（content 里有这一份；同步会把条目认下来） */
        sync_for_source(
            d.path(),
            &catalog(vec![f.clone()], "r1"),
            Some("github#a/b@main"),
        )
        .unwrap();
        assert_eq!(entry(d.path(), &f.path).unwrap().sha256, sha);

        /* 换到 B 源：同一个 SHA、同一个 path —— 但这是"从 A 源下过"，不算 B 源已下 */
        sync_for_source(
            d.path(),
            &catalog(vec![f.clone()], "r1"),
            Some("gitee#a/b@main"),
        )
        .unwrap();
        assert!(
            entry(d.path(), &f.path).is_some(),
            "盘上确实有这一份旧条目，不删（它代表另外那条线的一版）"
        );

        /* 干净盘面：content 有、索引里没有这条 path ⇒ 换源之后不许凭内容认领 */
        let fresh = tempfile::tempdir().unwrap();
        content::ensure(fresh.path(), &sha, b"x").unwrap();
        sync_for_source(
            fresh.path(),
            &catalog(vec![], "r1"),
            Some("github#a/b@main"),
        )
        .unwrap();
        sync_for_source(
            fresh.path(),
            &catalog(vec![f.clone()], "r1"),
            Some("gitee#a/b@main"),
        )
        .unwrap();
        assert!(
            entry(fresh.path(), &f.path).is_none(),
            "换源之后不凭本地内容认领，要真实下载"
        );
    }

    #[test]
    fn content_is_not_adopted_across_generations_even_on_the_same_line() {
        let d = tempfile::tempdir().unwrap();
        let f = file("A1.toml", "x");
        let sha = f.expected_sha().unwrap().to_owned();
        content::ensure(d.path(), &sha, b"x").unwrap();
        sync_for_source(d.path(), &catalog(vec![], "r1"), Some("github#a/b@main")).unwrap();

        /* 同一代：共享内容直接认（规则⑦） */
        sync_for_source(
            d.path(),
            &catalog(vec![f.clone()], "r1"),
            Some("github#a/b@main"),
        )
        .unwrap();
        assert!(entry(d.path(), &f.path).is_some());

        /* 下一代：同样的内容、同样的 path，但要真实下载一次才记进索引 */
        let d2 = tempfile::tempdir().unwrap();
        content::ensure(d2.path(), &sha, b"x").unwrap();
        sync_for_source(d2.path(), &catalog(vec![], "r1"), Some("github#a/b@main")).unwrap();
        sync_for_source(
            d2.path(),
            &catalog(vec![f.clone()], "r2"),
            Some("github#a/b@main"),
        )
        .unwrap();
        assert!(
            entry(d2.path(), &f.path).is_none(),
            "换代之后不凭本地内容认领"
        );
    }

    #[test]
    fn recording_a_download_updates_its_sha() {
        let d = tempfile::tempdir().unwrap();
        record_file(
            d.path(),
            "delivery/mkp/presets/A1.toml",
            &"a".repeat(64),
            Some(1),
        )
        .unwrap();
        assert_eq!(
            entry(d.path(), "delivery/mkp/presets/A1.toml")
                .unwrap()
                .sha256,
            "a".repeat(64)
        );
    }
}
