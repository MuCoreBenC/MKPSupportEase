//! **隐藏 baseline 存储**：每一版下载过的云端预设，在这里留一份不可见的基准快照。
//!
//! # 它是谁的、给谁用
//!
//! 用户世界里**只有「本地预设」**（`presets-local/`，见 [`super::local`]）。这一份是
//! **系统内部数据**，不是产品功能 —— 用户看不到它、列表里没有它、不能选它、
//! 不能删它、它也不参与任何"新版本 / 过时"的判断。它只有一个职责：
//!
//! > 回答「这份本地预设，当初那一版云端默认值是什么」。
//!
//! 具体兑现成两件事：
//!
//! - **逐参数「云端更新」的三方账**（2026-10-09）：交付预设那一版这一项是什么值，
//!   就从这儿读（`ipc::param_sync`）—— 与我那份文件里写着的、目录里当前版的值一起，
//!   凑出「我 / 云端旧值 / 云端新值」；
//! - **血统关系**：`based_on_sha256` 指向的就是这一份（云端版本的唯一身份）。
//!
//! # 为什么按内容摘要寻址
//!
//! 落点是 `<appDataDir>/baseline/<sha256>.toml`。摘要就是内容，于是：
//!
//! - **天然幂等**：同一版下多少次都只落一份，第二次一个字节不动；
//! - **每版一份、永不覆盖**：没有"最新是谁"这回事，也就不需要任何版本生命周期管理；
//! - **O(1) 取回**：血统里那串摘要直接就是路径，不需要扫目录、不需要索引账本。
//!
//! # 不做垃圾回收
//!
//! 用户删掉自己那份之后，基准可以暂时留下（几 KB 而已，而且关系还在）。真要做 GC
//! 是另一件事 —— 现在为它引入一套"引用计数 / 生命周期"会把简单的事做复杂。
//!
//! # 与归档（`archive/`）不是一回事
//!
//! `archive/` 服务的是**交付四态判定**（盘上这份认得出是哪一版吗，见
//! [`super::delivery::trust_entries`]）；这一份服务的是**逐参数云端更新的三方账**。
//! 两条线各管各的，互不依赖。
//!
//! # 原文镜像（`baseline-local/`）—— 同一族、另一件事
//!
//! 上面的摘库回答"**云端某一版**长什么样"；镜像区回答"**这份用户文件自己最初**长什么样"
//! （编辑文件模态框那颗「恢复默认值」退的就是它，作者 2026-10-16：「用户下载进来的文件是
//! 存在 baseline 的，那它导入的或者复制出的文件可能也需要做一个这种工作」）。
//!
//! 为什么不能复用摘库：副本、导入进来那两份在摘库里**根本没有对应**（它们的
//! `based_on_sha256` 指的是更上一层的云端原文，不是自己）。所以按**路径**镜像一份：
//! `<appDataDir>/baseline-local/presets-local/<原文件名>`，谁在"第一次落到盘上"那一刻存一次，
//! 谁就永远答得出"它最初的样子"。

use postprocess::diag::messages;

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::AppError;

/// 基准区的目录名。相对**内部根**（`appDataDir`）。
pub const BASELINE_DIR: &str = "baseline";

/// 基准区：`<appDataDir>/baseline`
pub fn baseline_dir(internal_root: &Path) -> PathBuf {
    internal_root.join(BASELINE_DIR)
}

/// 一份基准的落点：`<appDataDir>/baseline/<sha256>.toml`
pub fn baseline_path(internal_root: &Path, sha256: &str) -> PathBuf {
    baseline_dir(internal_root).join(format!("{}.toml", sha256.trim().to_ascii_lowercase()))
}

/// 摘要必须是一个真的 sha256，才允许拿来拼路径。
///
/// **这不是"顺手校验"**：它同时是防穿越的一道闸 —— 摘要来自用户文件头里的那行注释，
/// 而那是用户能改的东西。放一个 `../../x` 进来就会写到用户根外面去。
fn check_sha(sha256: &str) -> Result<&str, AppError> {
    let s = sha256.trim();
    let ok = s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit());
    if !ok {
        return Err(AppError::invalid_argument(messages::text(
            "ui.runtime.baseline.368",
            &[("sha256", sha256)],
        )));
    }
    Ok(s)
}

/// 本机有没有这一版云端（按内容摘要认）。
pub fn baseline_exists(internal_root: &Path, sha256: &str) -> bool {
    match check_sha(sha256) {
        Ok(s) => baseline_path(internal_root, s).is_file(),
        Err(_) => false,
    }
}

/// 落一份基准。**幂等**：已经有就一个字节不动。
///
/// 返回 `true` = 这次真的写了一份新的。落不上不该让下载失败 ——
/// 调用方按"附加信息"处理（与事件账、出处账同一条口径）。
pub fn ensure_baseline(internal_root: &Path, sha256: &str, bytes: &[u8]) -> Result<bool, AppError> {
    let s = check_sha(sha256)?;
    let target = baseline_path(internal_root, s);
    if target.is_file() {
        return Ok(false);
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            AppError::io(messages::text(
                "ui.runtime.baseline.369",
                &[("v1", &parent.display().to_string())],
            ))
            .with_detail(e.to_string())
        })?;
    }
    crate::fsx::atomic::atomic_write(&target, bytes)?;
    Ok(true)
}

/// 只读一份基准的正文。没有 ⇒ `None`。
///
/// **字节与名字对不上也当没有**（并记一条告警）：恢复默认宁可回退到出厂值，
/// 也不许拿一份来历不明的字节冒充"云端默认"。这里不报错 —— 基准缺失是
/// 一个**正常状态**（导入的、从用户预设复制出来的那两份本来就没有基准）。
pub fn read_baseline(internal_root: &Path, sha256: &str) -> Result<Option<String>, AppError> {
    let s = check_sha(sha256)?;
    let path = baseline_path(internal_root, s);
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(AppError::io(messages::text(
                "ui.runtime.baseline.370",
                &[("v1", &path.display().to_string())],
            ))
            .with_detail(e.to_string()))
        }
    };
    let got = super::catalog::hex(&Sha256::digest(&bytes));
    if got != s {
        tracing::warn!(want = %s, got = %got, "基准的字节与它的名字对不上，不当它是默认基准");
        return Ok(None);
    }
    match String::from_utf8(bytes) {
        Ok(text) => Ok(Some(text)),
        Err(_) => {
            tracing::warn!(sha = %s, "基准不是 UTF-8 文本，读不出来");
            Ok(None)
        }
    }
}

/// 本机基准区里都有哪些版本（按文件名升序）。**给云端版本列表用** ——
/// 名字就是摘要，"已下载"这件事不需要另记一本账。
///
/// 读不动目录 ⇒ 空表（一份都还没下过就是空表，不是错误）。
pub fn baseline_shas(internal_root: &Path) -> Vec<String> {
    let dir = baseline_dir(internal_root);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out: Vec<String> = entries
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let stem = name.strip_suffix(".toml")?;
            check_sha(stem).ok().map(|s| s.to_ascii_lowercase())
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

/* ---------- 原文镜像（`baseline-local/`）：这份用户文件最初的样子 ---------- */

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
/// - **新文件落盘之后**（下载 / 导入 / 复制 / 另存）—— 刚写进去的就是原文；
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

/// 读一份原文镜像。没有 ⇒ `None`（**正常状态**：这个功能上线之前就有的文件还没存过）。
pub fn read_origin_copy(internal_root: &Path, rel: &str) -> Result<Option<String>, AppError> {
    let path = origin_path(internal_root, rel)?;
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(AppError::io(messages::text(
                "ui.runtime.baseline.373",
                &[("v1", &path.display().to_string())],
            ))
            .with_detail(e.to_string()))
        }
    };
    Ok(String::from_utf8(bytes).ok())
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

    fn sha_of(text: &str) -> String {
        super::super::catalog::hex(&Sha256::digest(text.as_bytes()))
    }

    /// 幂等：第二次一个字节不动 —— 这是"每版一份、永不覆盖"的实现保证
    #[test]
    fn the_same_version_is_written_once_and_never_touched() {
        let d = tempfile::tempdir().unwrap();
        let text = "# machine: A1\n[toolhead]\noffset_x = 1\n";
        let sha = sha_of(text);

        assert!(ensure_baseline(d.path(), &sha, text.as_bytes()).unwrap());
        // 有人在盘上动过它（模拟）：再 ensure 不许写回去
        let path = baseline_path(d.path(), &sha);
        crate::fsx::atomic::atomic_write(&path, b"# \xe8\xa2\xab\xe6\x94\xb9\xe8\xbf\x87\n")
            .unwrap();
        assert!(
            !ensure_baseline(d.path(), &sha, text.as_bytes()).unwrap(),
            "已经有就不该再写"
        );
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "# 被改过\n",
            "第二次 ensure 不许碰它"
        );
    }

    /// 读回来的是原样字节；没有那一份是 `None`（缺失是正常状态，不报错）
    #[test]
    fn reading_back_gives_the_bytes_or_nothing() {
        let d = tempfile::tempdir().unwrap();
        let text = "# machine: A1\n[toolhead]\noffset_x = -1 # 笔尖\n";
        let sha = sha_of(text);
        assert_eq!(
            read_baseline(d.path(), &sha).unwrap(),
            None,
            "还没有就是 None"
        );
        ensure_baseline(d.path(), &sha, text.as_bytes()).unwrap();
        assert_eq!(
            read_baseline(d.path(), &sha).unwrap().as_deref(),
            Some(text)
        );
    }

    /// 字节与名字对不上 = 当它没有（宁可回退出厂值，也不拿可疑字节当默认）
    #[test]
    fn a_baseline_whose_bytes_do_not_match_its_name_is_not_used() {
        let d = tempfile::tempdir().unwrap();
        let text = "# machine: A1\n";
        let sha = sha_of(text);
        ensure_baseline(d.path(), &sha, text.as_bytes()).unwrap();
        crate::fsx::atomic::atomic_write(
            &baseline_path(d.path(), &sha),
            b"# \xe5\x88\xab\xe7\x9a\x84\n",
        )
        .unwrap();
        assert_eq!(read_baseline(d.path(), &sha).unwrap(), None);
    }

    /// 摘要形状不对一律拒绝 —— 它是用户能改的注释行，不许拿它拼路径
    #[test]
    fn a_non_sha_is_refused_before_it_touches_the_disk() {
        let d = tempfile::tempdir().unwrap();
        for bad in ["", "../../x", "abc", &"z".repeat(64)] {
            assert!(
                ensure_baseline(d.path(), bad, b"x").is_err(),
                "{bad} 该被拒"
            );
            assert!(!baseline_exists(d.path(), bad));
        }
    }

    /// 列表按摘要给出，只认 `64 位 hex + .toml` 的文件名
    #[test]
    fn the_list_reads_back_what_is_there() {
        let d = tempfile::tempdir().unwrap();
        assert!(baseline_shas(d.path()).is_empty(), "空表");
        let a = sha_of("a");
        let b = sha_of("b");
        ensure_baseline(d.path(), &a, b"a").unwrap();
        ensure_baseline(d.path(), &b, b"b").unwrap();
        // 混一个不该被认出来的东西进去
        std::fs::create_dir_all(baseline_dir(d.path())).unwrap();
        crate::fsx::atomic::atomic_write(&baseline_dir(d.path()).join("readme.txt"), b"x").unwrap();
        let mut want = vec![a.clone(), b.clone()];
        want.sort();
        assert_eq!(baseline_shas(d.path()), want);
    }

    /* ---------- 原文镜像（`baseline-local/`） ---------- */

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
