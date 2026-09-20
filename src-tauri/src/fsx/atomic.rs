//! 唯一的写盘出口。
//!
//! 直接 `std::fs::write` 的问题不是"不够优雅"，而是**断电/崩溃时会留下半个文件**：
//! 它先把目标文件截断成 0 字节，再往里写。中途挂掉，用户的预设就没了 —— 不是回到旧版本，
//! 是空文件。
//!
//! 这里的做法：同目录建临时文件 → 写完 → `sync_all`（落盘，不只是进页缓存）→ `persist`
//! （同分区内的 rename，POSIX 保证原子）→ 再 fsync 父目录（让"这个名字指向新 inode"
//! 这件事也落盘，否则目录项可能还在缓存里）。任何一步失败，目标文件都还是旧的那一份。
//!
//! 临时文件必须与目标**同目录**：跨分区 rename 会退化成 copy + unlink，原子性就没了。
//!
//! `clippy.toml` 把 `std::fs::write` / `File::create` / `tokio::fs::write` 列进
//! `disallowed-methods`，全仓只有本文件开一个 `#[allow]` 的洞。

use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::error::AppError;

/// 原子写。目标目录不存在会自动建。
#[allow(clippy::disallowed_methods)] // 本文件是那个唯一的洞
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let parent = path
        .parent()
        .ok_or_else(|| AppError::invalid_argument("写入路径没有父目录"))?;

    std::fs::create_dir_all(parent).map_err(|e| {
        AppError::io(format!("建不出目录：{}", parent.display())).with_detail(e.to_string())
    })?;

    let mut tmp = tempfile::NamedTempFile::new_in(parent).map_err(|e| {
        AppError::io("创建临时文件失败").with_detail(format!("{} / {e}", parent.display()))
    })?;

    tmp.write_all(bytes)
        .map_err(|e| AppError::io("写入临时文件失败").with_detail(e.to_string()))?;
    tmp.as_file()
        .sync_all()
        .map_err(|e| AppError::io("临时文件落盘失败").with_detail(e.to_string()))?;

    tmp.persist(path)?;

    /* 父目录也要 fsync：rename 之后"这个名字指向哪个 inode"还可能只在缓存里。
    目录打不开（罕见）不算写失败 —— 数据已经在盘上了，只是这一步保证弱一点。 */
    if let Ok(dir) = File::open(parent) {
        let _ = dir.sync_all();
    }

    Ok(())
}

/// 写 JSON。序列化失败与写盘失败都会被转成 `AppError`
pub fn atomic_write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), AppError> {
    let bytes = serde_json::to_vec_pretty(value)?;
    atomic_write(path, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_content() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("a.json");
        atomic_write(&p, b"{\"x\":1}").unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "{\"x\":1}");
    }

    #[test]
    fn creates_missing_directories() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("deep/er/still/a.txt");
        atomic_write(&p, b"hi").unwrap();
        assert!(p.exists());
    }

    #[test]
    fn overwrites_atomically_without_leaving_temp_files() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("a.txt");
        atomic_write(&p, b"old").unwrap();
        atomic_write(&p, b"new").unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "new");

        let leftovers: Vec<_> = std::fs::read_dir(d.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n != "a.txt")
            .collect();
        assert!(leftovers.is_empty(), "临时文件残留：{leftovers:?}");
    }

    #[test]
    fn json_roundtrip() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("axes.json");
        atomic_write_json(&p, &serde_json::json!({ "x": 1.5 })).unwrap();
        let back: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
        assert_eq!(back["x"], 1.5);
    }
}
