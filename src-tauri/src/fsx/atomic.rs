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

/// Windows 上把目标路径转成 `\\?\` 前缀的 verbatim 形式。
///
/// 解决什么：文件名上限按**字节**算（`runtime::local::NAME_MAX_BYTES = 255`），
/// 而 Windows 默认的 `MAX_PATH` 是 **260 字符**的整条路径 —— 用户目录一长，
/// `presets-local/<255 字节的名字>` 就整体越界，`rename` 报
/// “系统找不到指定的路径 (os error 3)”，复制/改名直接失败。
/// 加 verbatim 前缀后这条路径不再受 260 限制（NT 命名空间本身支持 32K）。
///
/// 只对**父目录**做 `canonicalize`（目标文件还不存在，不能对它做），
/// 拿到的已经是 verbatim 形式，直接拼文件名即可。任何一步失败就退回原路径 ——
/// 长路径这次写不成，但不会让原本能成的短路径变坏。
#[cfg(windows)]
fn verbatim(path: &Path) -> std::path::PathBuf {
    let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
        return path.to_path_buf();
    };
    match parent.canonicalize() {
        Ok(dir) => dir.join(name),
        Err(_) => path.to_path_buf(),
    }
}

#[cfg(not(windows))]
fn verbatim(path: &Path) -> std::path::PathBuf {
    path.to_path_buf()
}

/// 原子写。目标目录不存在会自动建。
///
/// ★ **内容没变就不落盘**（2026-10-05 作者裁定：只读不写，连修改时间都不许动）。
/// git 不看 mtime，但人看 —— 「我只是打开了预演，文件时间怎么全变了」是实打实的困惑。
/// 跳过是安全的：写盘的唯一目的是让目标等于这串字节，它已经相等了。
#[allow(clippy::disallowed_methods)] // 本文件是那个唯一的洞
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    if let Ok(old) = std::fs::read(verbatim(path)) {
        if old == bytes {
            return Ok(());
        }
    }

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

    tmp.persist(verbatim(path))?;

    /* 父目录也要 fsync：rename 之后"这个名字指向哪个 inode"还可能只在缓存里。
    目录打不开（罕见）不算写失败 —— 数据已经在盘上了，只是这一步保证弱一点。 */
    if let Ok(dir) = File::open(parent) {
        let _ = dir.sync_all();
    }

    Ok(())
}

/// **流式原子写**（2026-10-17，给备份那一处）：给一个 `&mut File` 让调用方边读边写，
/// 写完由本函数负责 `sync_all` → `persist` → fsync 父目录。
///
/// 为什么要有这一条：`atomic_write` 收的是**整串字节**，而备份那种东西
/// （执行归档三件套能到几百 MB）不该先整个进内存再落盘。
/// 规矩与它**一字不差** —— 同样是"临时文件 → 落盘 → 同分区 rename"，
/// 所以写盘出口仍然只有这一个文件。
#[allow(clippy::disallowed_methods)] // 本文件是那个唯一的洞
pub fn atomic_write_stream(
    path: &Path,
    write: impl FnOnce(&mut File) -> Result<(), AppError>,
) -> Result<(), AppError> {
    let parent = path
        .parent()
        .ok_or_else(|| AppError::invalid_argument("写入路径没有父目录"))?;

    std::fs::create_dir_all(parent).map_err(|e| {
        AppError::io(format!("建不出目录：{}", parent.display())).with_detail(e.to_string())
    })?;

    let mut tmp = tempfile::NamedTempFile::new_in(parent).map_err(|e| {
        AppError::io("创建临时文件失败").with_detail(format!("{} / {e}", parent.display()))
    })?;

    write(tmp.as_file_mut())?;

    tmp.as_file()
        .sync_all()
        .map_err(|e| AppError::io("临时文件落盘失败").with_detail(e.to_string()))?;
    tmp.persist(verbatim(path))?;

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

    /// ★ 同字节重写必须**整个跳过**：内容不变，连修改时间都不许动
    /// （2026-10-05 作者裁定：只读不写。真机踩过：no-op 的生成把交付根
    /// 全部摸了一遍，用户看到「我只是看了一眼预演」）。
    #[test]
    fn identical_bytes_do_not_touch_the_file_at_all() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("a.txt");
        atomic_write(&p, b"same").unwrap();
        let before = std::fs::metadata(&p).unwrap().modified().unwrap();

        // 留出时钟前进的余量：若跳过失效（真的重写了），mtime 必然不同
        std::thread::sleep(std::time::Duration::from_millis(1100));
        atomic_write(&p, b"same").unwrap();
        assert_eq!(
            std::fs::metadata(&p).unwrap().modified().unwrap(),
            before,
            "同字节重写不许动 mtime"
        );

        // 真写入：内容变、mtime 变
        atomic_write(&p, b"changed").unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"changed");
        assert_ne!(
            std::fs::metadata(&p).unwrap().modified().unwrap(),
            before,
            "真写入要更新 mtime"
        );
    }

    /// 流式那一档（备份 zip 用）：边读边写，落出来的与一次性写的一模一样，且不留临时文件
    #[test]
    fn streaming_writes_land_the_same_bytes_and_leave_no_temp_files() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("big.bin");
        atomic_write_stream(&p, |f| {
            f.write_all(b"part-1-")
                .map_err(|e| AppError::io("写不了").with_detail(e.to_string()))?;
            f.write_all(b"part-2")
                .map_err(|e| AppError::io("写不了").with_detail(e.to_string()))
        })
        .unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"part-1-part-2");

        let leftovers: Vec<_> = std::fs::read_dir(d.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n != "big.bin")
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
