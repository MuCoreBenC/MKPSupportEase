//! **回收站**：删掉的用户预设进这里，而不是真的消失。
//!
//! # 一个条目里有什么
//!
//! ```text
//! recovery/<id>/
//!   <file_name>                 用户删掉的那一份（删那一刻的字节）
//!   <file_name>.origin.toml     它的原文镜像（「恢复默认值」退回去的那份）
//!   recovery.json               { path, file_name, deleted_at, provenance, has_origin }
//! ```
//!
//! 恢复 = 把文件与镜像搬回原位 + 把来源账本写回去，**不自动设为使用中**。
//! 条目超过 [`KEEP_DAYS`] 天或超过 [`KEEP_COUNT`] 份时按最旧清理；清理只动回收目录，
//! 不碰 `content/`（内容是否还在由 [`super::content::gc`] 的引用检查决定）。
//!
//! # 为什么不在删除时就删内容
//!
//! 同一份云端原文可能还被别的用户预设、回收条目或待处理更新引用。
//! 删除只负责"这份用户文件不在了"，内容去留交给引用检查。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::AppError;

use super::catalog::hex;
use super::source_ledger::LocalProvenance;

/// 回收区的目录名。相对**内部根**。
pub const RECOVERY_DIR: &str = "recovery";

/// 保留策略：30 天或 20 份，先到先清。
pub const KEEP_DAYS: u64 = 30;
pub const KEEP_COUNT: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryEntry {
    /// 目录名（时间戳 + 路径短哈希）
    pub id: String,
    /// 原相对路径（`presets-local/xxx.toml`）
    pub path: String,
    pub file_name: String,
    /// 删除时刻（UTC epoch 秒）
    pub deleted_at: u64,
    /// 删除那一刻的来源账（恢复时原样写回）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<LocalProvenance>,
    /// 有没有一起收进来的原文镜像
    pub has_origin: bool,
}

pub fn recovery_dir(root: &Path) -> PathBuf {
    root.join(RECOVERY_DIR)
}

fn entry_dir(root: &Path, id: &str) -> PathBuf {
    recovery_dir(root).join(id)
}

fn manifest_path(root: &Path, id: &str) -> PathBuf {
    entry_dir(root, id).join("recovery.json")
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn short_hash(path: &str) -> String {
    hex(&Sha256::digest(path.as_bytes()))[..8].to_owned()
}

/// 读一条回收记录。
pub fn read(root: &Path, id: &str) -> Result<Option<RecoveryEntry>, AppError> {
    let bytes = match std::fs::read(manifest_path(root, id)) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(AppError::io("回收记录读不出来").with_detail(e.to_string()));
        }
    };
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|e| AppError::corrupted("回收记录坏了").with_detail(e.to_string()))
}

/// 列出所有回收条目（最新删除的在前）。
pub fn list(root: &Path) -> Vec<RecoveryEntry> {
    let dir = recovery_dir(root);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out: Vec<RecoveryEntry> = entries
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| {
            let id = e.file_name().to_string_lossy().into_owned();
            read(root, &id).ok().flatten()
        })
        .collect();
    out.sort_by(|a, b| {
        b.deleted_at
            .cmp(&a.deleted_at)
            .then_with(|| a.id.cmp(&b.id))
    });
    out
}

/// **把一份用户文件收进回收站**。
///
/// 顺序：先写回收副本与清单，成功之后再删原文件。任何一步写失败都保留原文件；
/// 删原文件失败时把刚建的回收目录清掉，不留下"两边都有"的半成品。
pub fn create(
    root: &Path,
    user_root: &Path,
    path: &str,
    provenance: Option<LocalProvenance>,
) -> Result<RecoveryEntry, AppError> {
    super::local::check_local_prefix(path)?;
    let source = crate::fsx::paths::resolve_in(user_root, path)?;
    let bytes = std::fs::read(&source)
        .map_err(|e| AppError::io("要删除的预设读不出来").with_detail(e.to_string()))?;

    let origin = super::baseline::read_origin_bytes(root, path)
        .ok()
        .flatten();

    let deleted_at = now_secs();
    let mut id = format!("{deleted_at}-{}", short_hash(path));
    let mut n = 1;
    while entry_dir(root, &id).exists() {
        id = format!("{deleted_at}-{}-{n}", short_hash(path));
        n += 1;
    }
    let dir = entry_dir(root, &id);
    std::fs::create_dir_all(&dir)
        .map_err(|e| AppError::io("回收目录建不出来").with_detail(e.to_string()))?;

    let file_name = source
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.rsplit('/').next().unwrap_or("preset.toml").to_owned());
    let file_dst = dir.join(&file_name);
    crate::fsx::atomic::atomic_write(&file_dst, &bytes)?;
    if let Some(origin_bytes) = &origin {
        crate::fsx::atomic::atomic_write(
            &dir.join(format!("{file_name}.origin.toml")),
            origin_bytes,
        )?;
    }

    let entry = RecoveryEntry {
        id: id.clone(),
        path: path.to_owned(),
        file_name,
        deleted_at,
        provenance,
        has_origin: origin.is_some(),
    };
    let manifest = serde_json::to_vec_pretty(&entry)
        .map_err(|e| AppError::io("回收记录写不出来").with_detail(e.to_string()))?;
    crate::fsx::atomic::atomic_write(&manifest_path(root, &id), &manifest)?;

    /* 清单落好之后才动原文件；失败就把刚建的目录撤掉 */
    if let Err(e) = std::fs::remove_file(&source) {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(AppError::io("预设删不掉（原文件还在）").with_detail(e.to_string()));
    }
    let _ = super::baseline::remove_origin_copy(root, path);
    Ok(entry)
}

/// **从回收站恢复**：文件回到原路径、镜像回去、来源账写回，**不自动设为使用中**。
pub fn restore(root: &Path, user_root: &Path, id: &str) -> Result<RecoveryEntry, AppError> {
    let entry = read(root, id)?.ok_or_else(|| AppError::not_found("回收站里没有这一条"))?;
    super::local::check_local_prefix(&entry.path)?;
    let target = crate::fsx::paths::resolve_in(user_root, &entry.path)?;
    if target.exists() {
        return Err(AppError::invalid_argument(
            "原位置已经有一份同名预设，先改名或删掉它再恢复",
        ));
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::io("恢复目录建不出来").with_detail(e.to_string()))?;
    }
    let src = entry_dir(root, id).join(&entry.file_name);
    let bytes = std::fs::read(&src)
        .map_err(|e| AppError::io("回收副本读不出来").with_detail(e.to_string()))?;
    crate::fsx::atomic::atomic_write(&target, &bytes)?;

    if entry.has_origin {
        if let Ok(Some(origin_bytes)) =
            std::fs::read(entry_dir(root, id).join(format!("{}.origin.toml", entry.file_name)))
                .map(Some)
        {
            if let Ok(dst) = super::baseline::origin_path(root, &entry.path) {
                if let Some(parent) = dst.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = crate::fsx::atomic::atomic_write(&dst, &origin_bytes);
            }
        }
    }

    if let Some(provenance) = &entry.provenance {
        super::source_ledger::restore(root, provenance)?;
    }
    std::fs::remove_dir_all(entry_dir(root, id))
        .map_err(|e| AppError::io("回收目录删不掉").with_detail(e.to_string()))?;
    Ok(entry)
}

/// 永久删掉一条。
pub fn purge(root: &Path, id: &str) -> Result<(), AppError> {
    let dir = entry_dir(root, id);
    match std::fs::remove_dir_all(&dir) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(AppError::io("回收条目删不掉").with_detail(e.to_string())),
    }
}

/// 清空回收站（用户明确点的动作）。
pub fn empty(root: &Path) -> Result<usize, AppError> {
    let mut n = 0;
    for entry in list(root) {
        purge(root, &entry.id)?;
        n += 1;
    }
    Ok(n)
}

/// 按保留策略清理：30 天或 20 份，先到先清。
pub fn cleanup(root: &Path) -> usize {
    let now = now_secs();
    let too_old = KEEP_DAYS.saturating_mul(24 * 60 * 60);
    let mut entries = list(root);
    /* list 是新的在前；从最旧那头删 */
    entries.reverse();
    let mut removed = 0usize;
    let mut left = entries.len();
    for entry in entries {
        let over_count = left > KEEP_COUNT;
        let over_age = now.saturating_sub(entry.deleted_at) > too_old;
        if !over_count && !over_age {
            continue;
        }
        if purge(root, &entry.id).is_ok() {
            removed += 1;
            left -= 1;
        }
    }
    removed
}

/// 回收条目带来的内容引用（`content::gc` 用）。
pub fn referenced_shas(root: &Path) -> std::collections::HashSet<String> {
    list(root)
        .into_iter()
        .filter_map(|e| e.provenance.and_then(|p| p.source_sha256))
        .map(|s| s.to_ascii_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fsx::atomic::atomic_write;

    fn put(user: &Path, rel: &str, text: &str) {
        let path = crate::fsx::paths::resolve_in(user, rel).unwrap();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        atomic_write(&path, text.as_bytes()).unwrap();
    }

    #[test]
    fn delete_moves_the_file_and_restore_brings_it_back() {
        let internal = tempfile::tempdir().unwrap();
        let user = tempfile::tempdir().unwrap();
        let rel = "presets-local/A1.toml";
        put(user.path(), rel, "# mine\n");
        super::super::baseline::ensure_origin_copy(internal.path(), user.path(), rel).unwrap();

        let entry = create(internal.path(), user.path(), rel, None).unwrap();
        assert!(!crate::fsx::paths::resolve_in(user.path(), rel)
            .unwrap()
            .exists());
        assert_eq!(list(internal.path()).len(), 1);

        restore(internal.path(), user.path(), &entry.id).unwrap();
        assert_eq!(
            std::fs::read_to_string(crate::fsx::paths::resolve_in(user.path(), rel).unwrap())
                .unwrap(),
            "# mine\n"
        );
        assert!(list(internal.path()).is_empty());
        assert!(
            super::super::baseline::read_origin_copy(internal.path(), rel)
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn restore_refuses_to_overwrite_an_existing_file() {
        let internal = tempfile::tempdir().unwrap();
        let user = tempfile::tempdir().unwrap();
        let rel = "presets-local/A1.toml";
        put(user.path(), rel, "# mine\n");
        let entry = create(internal.path(), user.path(), rel, None).unwrap();
        put(user.path(), rel, "# new\n");
        assert!(restore(internal.path(), user.path(), &entry.id).is_err());
    }
}
