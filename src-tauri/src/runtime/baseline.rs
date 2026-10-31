//! **原文镜像**（`baseline-local/`）：这份用户文件自己最初的样子。
//!
//! # 它回答什么
//!
//! 「我这份预设，第一次落到盘上时长什么样」—— 编辑器那颗「恢复默认值」退的就是它；
//! 「我改了哪几项 / 这份少哪些参数」也拿它当基准。
//!
//! # 为什么按路径镜像
//!
//! 拷贝 / 导入进来的那两份在云端内容仓库里**没有对应**（它们的 `based_on_sha256`
//! 指的是更上一层的云端原文，不是自己）。所以按**用户根的相对路径**镜像一份：
//! `<appDataDir>/baseline-local/presets-local/<原文件名>`，谁在"第一次落到盘上"
//! 那一刻存一次，谁就永远答得出"它最初的样子"。
//!
//! # 与 `content/` 不是一回事
//!
//! `content/<sha>.toml` 回答"**云端某一版**长什么样"；这里回答"**这份用户文件自己**
//! 最初长什么样"。两条线各管各的，互不依赖。

use postprocess::diag::messages;

use std::path::{Path, PathBuf};

use crate::error::AppError;

/// 原文镜像区的目录名。相对**内部根**（`appDataDir`）。
pub const ORIGIN_DIR: &str = "baseline-local";

/// 原文镜像区：`<appDataDir>/baseline-local`（**按用户根的相对路径镜像**）
pub fn origin_dir(internal_root: &Path) -> PathBuf {
    internal_root.join(ORIGIN_DIR)
}

/// 一份原文镜像的落点：`<appDataDir>/baseline-local/<rel>`（`rel` 形如
/// `presets-local/A1 涂胶.toml`）。
///
/// 路径过**与读用户文件同一道闸**（[`super::local::check_local_prefix`]）——
/// 镜像直接复用那个相对路径，闸必须同源，不然 `..` 就顺着镜像写出去了。
pub fn origin_path(internal_root: &Path, rel: &str) -> Result<PathBuf, AppError> {
    super::local::check_local_prefix(rel)?;
    Ok(origin_dir(internal_root).join(rel))
}

/// 存一份原文镜像。**幂等**：已经有就一个字节不动（`false`）。
///
/// 调用时机只有两种（各自保证存下来的是"它最初的样子"）：
/// - **新文件落盘之后**（下载 / 导入 / 拷贝 / 另存）—— 刚写进去的就是原文；
/// - **老文件写盘之前**（编辑器保存 / 参数写回）—— 那一刻盘上还是原文。
///
/// 读不到源文件就如实报错：调用方按"附加信息"处理（与事件账、出处账同一条口径），
/// 不让镜像的问题连累主流程。
pub fn ensure_origin_copy(
    internal_root: &Path,
    user_root: &Path,
    rel: &str,
) -> Result<bool, AppError> {
    let target = origin_path(internal_root, rel)?;
    if target.is_file() {
        return Ok(false);
    }
    let src = crate::fsx::paths::resolve_in(user_root, rel)?;
    let bytes = std::fs::read(&src).map_err(|e| {
        AppError::io(messages::text(
            "ui.runtime.baseline.371",
            &[("v1", &src.display().to_string())],
        ))
        .with_detail(e.to_string())
    })?;
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            AppError::io(messages::text(
                "ui.runtime.baseline.372",
                &[("v1", &parent.display().to_string())],
            ))
            .with_detail(e.to_string())
        })?;
    }
    crate::fsx::atomic::atomic_write(&target, &bytes)?;
    Ok(true)
}

/// 读一份原文镜像的正文。没有 ⇒ `None`（**正常状态**：这个功能上线之前就有的文件还没存过）。
pub fn read_origin_copy(internal_root: &Path, rel: &str) -> Result<Option<String>, AppError> {
    Ok(read_origin_bytes(internal_root, rel)?.and_then(|b| String::from_utf8(b).ok()))
}

/// 读一份原文镜像的原始字节（回收站搬运用，不要求 UTF-8）。
pub fn read_origin_bytes(internal_root: &Path, rel: &str) -> Result<Option<Vec<u8>>, AppError> {
    let path = origin_path(internal_root, rel)?;
    match std::fs::read(&path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(AppError::io(messages::text(
            "ui.runtime.baseline.373",
            &[("v1", &path.display().to_string())],
        ))
        .with_detail(e.to_string())),
    }
}

/// 改名：镜像跟着搬家。没有镜像 ⇒ `false`（没什么可搬的，不是错误）。
pub fn move_origin_copy(
    internal_root: &Path,
    from_rel: &str,
    to_rel: &str,
) -> Result<bool, AppError> {
    let from = origin_path(internal_root, from_rel)?;
    if !from.is_file() {
        return Ok(false);
    }
    let to = origin_path(internal_root, to_rel)?;
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            AppError::io(messages::text(
                "ui.runtime.baseline.374",
                &[("v1", &parent.display().to_string())],
            ))
            .with_detail(e.to_string())
        })?;
    }
    std::fs::rename(&from, &to).map_err(|e| {
        AppError::io(messages::text(
            "ui.runtime.baseline.375",
            &[("v1", &from.display().to_string())],
        ))
        .with_detail(e.to_string())
    })?;
    Ok(true)
}

/// 删除：镜像一起走（没有镜像 ⇒ `false`，不是错误）。
pub fn remove_origin_copy(internal_root: &Path, rel: &str) -> Result<bool, AppError> {
    let path = origin_path(internal_root, rel)?;
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(AppError::io(messages::text(
            "ui.runtime.baseline.376",
            &[("v1", &path.display().to_string())],
        ))
        .with_detail(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 在用户根里摆一份文件（测试用真路径，两道闸都照着真的走）
    fn put_user_file(user_root: &Path, rel: &str, text: &str) {
        let path = crate::fsx::paths::resolve_in(user_root, rel).unwrap();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        crate::fsx::atomic::atomic_write(&path, text.as_bytes()).unwrap();
    }

    /// 第一次存原文；之后无论源文件怎么改，镜像**一个字节不动**（"最初那份"）
    #[test]
    fn the_origin_copy_is_written_once_and_never_touched() {
        let internal = tempfile::tempdir().unwrap();
        let user = tempfile::tempdir().unwrap();
        let rel = "presets-local/A1 涂胶.toml";
        put_user_file(user.path(), rel, "# 最初\n[toolhead]\noffset_y = 22.4\n");

        assert!(ensure_origin_copy(internal.path(), user.path(), rel).unwrap());
        put_user_file(user.path(), rel, "# 后来改过\n");
        assert!(
            !ensure_origin_copy(internal.path(), user.path(), rel).unwrap(),
            "已经有就不该再写"
        );
        assert_eq!(
            read_origin_copy(internal.path(), rel).unwrap().as_deref(),
            Some("# 最初\n[toolhead]\noffset_y = 22.4\n"),
            "镜像里永远是第一次那一份"
        );
    }

    /// 没有镜像 ⇒ `None`（正常状态）；源文件不在 ⇒ ensure 报错（调用方按附加信息处理）
    #[test]
    fn no_origin_copy_is_none_and_a_missing_source_is_an_error() {
        let internal = tempfile::tempdir().unwrap();
        let user = tempfile::tempdir().unwrap();
        let rel = "presets-local/没有过的.toml";
        assert_eq!(read_origin_copy(internal.path(), rel).unwrap(), None);
        assert!(ensure_origin_copy(internal.path(), user.path(), rel).is_err());
    }

    /// 越界路径全被拒：镜像是**同一道闸**（前缀 + 相对路径形状）
    #[test]
    fn the_origin_copy_refuses_paths_outside_the_local_dir() {
        let d = tempfile::tempdir().unwrap();
        for bad in ["", "../x.toml", "exports/x.toml", "/etc/passwd"] {
            assert!(origin_path(d.path(), bad).is_err(), "{bad} 该被拒");
        }
    }

    /// 改名搬家 + 删除清理（都没有镜像时如实答 `false`）
    #[test]
    fn the_origin_copy_moves_with_a_rename_and_dies_with_the_file() {
        let internal = tempfile::tempdir().unwrap();
        let user = tempfile::tempdir().unwrap();
        let old = "presets-local/A1 涂胶.toml";
        let new = "presets-local/A1 涂胶-高速版.toml";
        put_user_file(user.path(), old, "# 最初\n");

        assert!(
            !move_origin_copy(internal.path(), old, new).unwrap(),
            "没有镜像时搬家是 false"
        );
        ensure_origin_copy(internal.path(), user.path(), old).unwrap();
        assert!(move_origin_copy(internal.path(), old, new).unwrap());
        assert_eq!(read_origin_copy(internal.path(), old).unwrap(), None);
        assert_eq!(
            read_origin_copy(internal.path(), new).unwrap().as_deref(),
            Some("# 最初\n")
        );

        assert!(remove_origin_copy(internal.path(), new).unwrap());
        assert!(!remove_origin_copy(internal.path(), new).unwrap());
        assert_eq!(read_origin_copy(internal.path(), new).unwrap(), None);
    }
}
