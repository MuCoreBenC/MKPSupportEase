//! **原文镜像**（`baseline-local/`）：这份用户文件**最近一次与云端某一版
//! 逐字节一致地落盘**时的原样。
//!
//! # 它回答什么
//!
//! 「恢复默认值该回到哪一版」—— 参数页底栏与「编辑文件」那颗读**同一份字节**；
//! 「我改了哪几项 / 这份少哪些参数」也拿它当基准。
//!
//! # 什么时候写
//!
//! - 下载 / 另存为云端副本 / 导入：落盘之后（文件就是那一版的原样）；
//! - 更新合并：**只在**剥掉来源段后与云端原文一致时（逐项全部采用）；
//! - 手动编辑保存：**不写** —— 手改出来的不是默认值。
//!
//! # 为什么不是"最初那份"
//!
//! 最初那份答不了"云端更新过之后默认值该是哪一版"；而用户点了更新却保留自己的值时，
//! 那一版也不该被当成默认（他没采用）。口径因此是「最后一次真正用上的云端版」。
//!
//! # 与 `content/` 不是一回事
//!
//! `content/<sha>.toml` 按 SHA 存云端各版原文；这里只存这一份文件当前认的那一版。
//! 升级前只有"最初那份"的老文件，由读取侧按尾部末条 `sha256` 从 `content/` 迁移。

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

/// **覆盖写**一份原文镜像：把盘上这份用户文件当前的内容记成"默认"。
///
/// 调用时机（见模块头）：下载 / 另存为云端副本 / 导入落盘之后，以及更新合并**且**
/// 剥掉来源段后与云端原文一致时。手改保存**不调** —— 手改出来的不是默认值。
///
/// 读不到源文件就如实报错：调用方按"附加信息"处理（与事件账、出处账同一条口径），
/// 不让镜像的问题连累主流程。
pub fn refresh_origin_copy(
    internal_root: &Path,
    user_root: &Path,
    rel: &str,
) -> Result<(), AppError> {
    let target = origin_path(internal_root, rel)?;
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
    Ok(())
}

/// 读默认值时选哪一份正文（三分支，见 [`pick`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    /// 镜像就是这一份现在认的那版（最近一次云端落盘）
    Mirror,
    /// 镜像停在"最初那份"，用 `content/` 里那一版（老档迁移）
    Content,
    /// 只有镜像，没有可迁移的云端原文
    MirrorOnly,
    /// 什么都没有 —— 按钮禁用
    Nothing,
}

/// 选正文：**镜像与这份文件现在认的那版一致就用镜像**；否则若有云端原文就迁；
/// 都没有才退回镜像 / 没有。`has_content` 由调用方按 `want_sha` 查过 `content/` 给出。
pub fn pick(mirror_sha: Option<&str>, want_sha: Option<&str>, has_content: bool) -> Pick {
    match (mirror_sha, want_sha) {
        (Some(m), Some(w)) if m == w => Pick::Mirror,
        (_, Some(_)) if has_content => Pick::Content,
        (Some(_), _) => Pick::MirrorOnly,
        (None, _) => Pick::Nothing,
    }
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

    /// **覆盖写**：后一次取得的那版就是新的默认（"最后一次真正用上的云端版"）
    #[test]
    fn refresh_overwrites_the_origin_copy() {
        let internal = tempfile::tempdir().unwrap();
        let user = tempfile::tempdir().unwrap();
        let rel = "presets-local/A1 涂胶.toml";
        put_user_file(user.path(), rel, "# 云端 A\n[toolhead]\noffset_y = 22.4\n");

        refresh_origin_copy(internal.path(), user.path(), rel).unwrap();
        assert_eq!(
            read_origin_copy(internal.path(), rel).unwrap().as_deref(),
            Some("# 云端 A\n[toolhead]\noffset_y = 22.4\n")
        );

        /* 云端发 B、用户全部采用之后落盘：镜像跟着换成 B */
        put_user_file(user.path(), rel, "# 云端 B\n[toolhead]\noffset_y = 20\n");
        refresh_origin_copy(internal.path(), user.path(), rel).unwrap();
        assert_eq!(
            read_origin_copy(internal.path(), rel).unwrap().as_deref(),
            Some("# 云端 B\n[toolhead]\noffset_y = 20\n"),
            "默认跟着最后一次取得的那版走"
        );
    }

    /// 没有镜像 ⇒ `None`（正常状态）；源文件不在 ⇒ refresh 报错（调用方按附加信息处理）
    #[test]
    fn no_origin_copy_is_none_and_a_missing_source_is_an_error() {
        let internal = tempfile::tempdir().unwrap();
        let user = tempfile::tempdir().unwrap();
        let rel = "presets-local/没有过的.toml";
        assert_eq!(read_origin_copy(internal.path(), rel).unwrap(), None);
        assert!(refresh_origin_copy(internal.path(), user.path(), rel).is_err());
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
        refresh_origin_copy(internal.path(), user.path(), old).unwrap();
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

    /* ---------- 读默认值时的三分支（老档迁移的判据） ---------- */

    /// 镜像与这份文件现在认的那版一致 ⇒ 直接用镜像
    #[test]
    fn the_matching_mirror_is_the_default() {
        assert_eq!(pick(Some("a"), Some("a"), true), Pick::Mirror);
    }

    /// 镜像停在"最初那份"、云端原文还留着 ⇒ 迁移那一版
    #[test]
    fn a_stale_mirror_migrates_from_content() {
        assert_eq!(pick(Some("old"), Some("new"), true), Pick::Content);
    }

    /// 迁移取不到 ⇒ 退回镜像，如实标"最初那份"
    #[test]
    fn without_content_it_falls_back_to_the_mirror() {
        assert_eq!(pick(Some("old"), Some("new"), false), Pick::MirrorOnly);
    }

    /// 镜像与云端原文都没有 ⇒ 没有可恢复的（按钮禁用），绝不回落出厂值
    #[test]
    fn nothing_to_restore_is_nothing() {
        assert_eq!(pick(None, Some("new"), false), Pick::Nothing);
        assert_eq!(pick(None, None, false), Pick::Nothing);
    }
}
