//! **一次性存储迁移**：旧 `delivery/` 副本 + `baseline/<sha>` → `content/<sha>` + 目录索引。
//!
//! 规则（作者 2026-10-29）：
//! 1. 先把所有旧字节**写进** `content/`，全部成功之后才删旧目录；
//! 2. 任何一步失败 ⇒ **保留原状**，不写完成标记，下次启动再试；
//! 3. 只迁 `delivery/`（云端交付面）；`assets/`（模型 / 图标 / 切片器配置）不动；
//! 4. `archive/` 是版本证据，不动。
//!
//! 完成标记 `run/storage-v2.json`：存在即不再跑。

use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::AppError;
use postprocess::diag::messages;

use super::catalog::hex;
use super::{content, delivery_index};

const MARKER: &str = "run/storage-v2.json";
const DELIVERY_DIR: &str = "delivery";
const LEGACY_BASELINE_DIR: &str = "baseline";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Marker {
    at: u64,
}

/// 跑一次迁移。返回 `true` = 这次真的迁了（不是重复跑）。
pub fn run(root: &Path) -> Result<bool, AppError> {
    if root.join(MARKER).is_file() {
        return Ok(false);
    }

    let mut migrated: Vec<(String, String, Option<u64>, u64)> = Vec::new();

    /* ① 旧 baseline（按 SHA 命名）→ content；先只写不删 */
    let baseline = root.join(LEGACY_BASELINE_DIR);
    if let Ok(entries) = std::fs::read_dir(&baseline) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if content::check_sha(stem).is_err() {
                continue;
            }
            let bytes = std::fs::read(&path).map_err(|e| {
                AppError::io(messages::text("ui.migrate.baseline_read_failed", &[]))
                    .with_detail(e.to_string())
            })?;
            if hex(&Sha256::digest(&bytes)) != stem.to_ascii_lowercase() {
                tracing::warn!(file = %path.display(), "旧基准的字节与文件名对不上，跳过");
                continue;
            }
            content::ensure(root, stem, &bytes)?;
        }
    }

    /* ② 旧 delivery 副本 → content + 索引；先只写不删 */
    let delivery = root.join(DELIVERY_DIR);
    collect_delivery_files(root, &delivery, &mut migrated)?;
    for (rel, sha, size, at) in &migrated {
        let bytes = std::fs::read(root.join(rel)).map_err(|e| {
            AppError::io(messages::text("ui.migrate.delivery_read_failed", &[]))
                .with_detail(e.to_string())
        })?;
        content::ensure(root, sha, &bytes)?;
        delivery_index::record_file(root, rel, sha, *size)?;
        let _ = at;
    }

    /* ③ 用当前目录收口索引（识别共享 SHA、丢掉目录里没有的旧 path） */
    let catalog = super::load_released_catalog(root)?;
    delivery_index::sync(root, &catalog)?;

    /* ④ 全部写成功之后，才删旧目录（删不动只告警，不影响已完成的内容迁移） */
    if delivery.exists() {
        if let Err(e) = std::fs::remove_dir_all(&delivery) {
            tracing::warn!("旧 delivery 目录没删干净（内容已入库）：{e}");
        }
    }
    if baseline.exists() {
        if let Err(e) = std::fs::remove_dir_all(&baseline) {
            tracing::warn!("旧 baseline 目录没删干净（内容已入库）：{e}");
        }
    }

    let marker = serde_json::to_vec_pretty(&Marker {
        at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    })
    .map_err(|e| {
        AppError::io(messages::text("ui.migrate.marker_write_failed", &[]))
            .with_detail(e.to_string())
    })?;
    crate::fsx::atomic::atomic_write(&root.join(MARKER), &marker)?;

    tracing::info!(
        files = migrated.len(),
        "存储迁移完成：delivery/baseline → content"
    );
    Ok(true)
}

fn collect_delivery_files(
    root: &Path,
    dir: &Path,
    out: &mut Vec<(String, String, Option<u64>, u64)>,
) -> Result<(), AppError> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            collect_delivery_files(root, &path, out)?;
            continue;
        }
        if !kind.is_file() {
            continue;
        }
        let bytes = std::fs::read(&path).map_err(|e| {
            AppError::io(messages::text("ui.migrate.delivery_read_failed", &[]))
                .with_detail(e.to_string())
        })?;
        let sha = hex(&Sha256::digest(&bytes));
        let rel = path
            .strip_prefix(root)
            .map_err(|e| {
                AppError::io(messages::text("ui.migrate.path_outside_root", &[]))
                    .with_detail(e.to_string())
            })?
            .to_string_lossy()
            .replace('\\', "/");
        let at = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        out.push((rel, sha, Some(bytes.len() as u64), at));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fsx::atomic::atomic_write;

    #[test]
    fn legacy_delivery_and_baseline_move_into_content_once() {
        let d = tempfile::tempdir().unwrap();
        let text = b"# preset\nspeed = 1\n";
        let sha = hex(&Sha256::digest(text));
        let old = d.path().join("delivery/mkp/presets/A1.toml");
        std::fs::create_dir_all(old.parent().unwrap()).unwrap();
        atomic_write(&old, text).unwrap();
        std::fs::create_dir_all(d.path().join("baseline")).unwrap();
        atomic_write(&d.path().join(format!("baseline/{sha}.toml")), text).unwrap();

        /* 目录不存在也要能迁：迁移在 catalog 读不出来时不该毁数据 */
        std::fs::create_dir_all(d.path().join("run")).unwrap();
        atomic_write(
            &d.path().join("catalog.json"),
            br#"{"catalogSchema":1,"revision":"r1","files":[]}"#,
        )
        .unwrap();

        assert!(run(d.path()).unwrap());
        assert_eq!(
            content::read(d.path(), &sha).unwrap().as_deref(),
            Some("# preset\nspeed = 1\n")
        );
        assert!(!old.exists(), "旧 delivery 文件已删");
        assert!(!d.path().join("baseline").exists(), "旧 baseline 目录已删");
        assert!(!run(d.path()).unwrap(), "第二次是空转");
    }
}
