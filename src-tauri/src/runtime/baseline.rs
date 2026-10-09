//! **隐藏 baseline 存储**：每一版下载过的官方预设，在这里留一份不可见的基准快照。
//!
//! # 它是谁的、给谁用
//!
//! 用户世界里**只有「我的预设」**（`presets-mine/`，见 [`super::mine`]）。这一份是
//! **系统内部数据**，不是产品功能 —— 用户看不到它、列表里没有它、不能选它、
//! 不能删它、它也不参与任何"新版本 / 过时"的判断。它只有一个职责：
//!
//! > 回答「这份我的预设，当初那一版官方默认值是什么」。
//!
//! 具体兑现成两件事：
//!
//! - **逐参数「官方更新」的三方账**（2026-10-09）：官方那一版这一项是什么值，
//!   就从这儿读（`ipc::param_sync`）—— 与我那份文件里写着的、目录里当前版的值一起，
//!   凑出「我 / 官方旧值 / 官方新值」；
//! - **血统关系**：`based_on_sha256` 指向的就是这一份（官方版本的唯一身份）。
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
//! [`super::delivery::trust_entries`]）；这一份服务的是**恢复默认**。
//! 两条线各管各的，互不依赖。

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
        return Err(AppError::invalid_argument(format!(
            "血统里的摘要不是一个 sha256：{sha256}"
        )));
    }
    Ok(s)
}

/// 本机有没有这一版官方（按内容摘要认）。
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
pub fn ensure_baseline(
    internal_root: &Path,
    sha256: &str,
    bytes: &[u8],
) -> Result<bool, AppError> {
    let s = check_sha(sha256)?;
    let target = baseline_path(internal_root, s);
    if target.is_file() {
        return Ok(false);
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            AppError::io(format!("建不出基准目录：{}", parent.display()))
                .with_detail(e.to_string())
        })?;
    }
    crate::fsx::atomic::atomic_write(&target, bytes)?;
    Ok(true)
}

/// 只读一份基准的正文。没有 ⇒ `None`。
///
/// **字节与名字对不上也当没有**（并记一条告警）：恢复默认宁可回退到出厂值，
/// 也不许拿一份来历不明的字节冒充"官方默认"。这里不报错 —— 基准缺失是
/// 一个**正常状态**（导入的、从用户预设复制出来的那两份本来就没有基准）。
pub fn read_baseline(internal_root: &Path, sha256: &str) -> Result<Option<String>, AppError> {
    let s = check_sha(sha256)?;
    let path = baseline_path(internal_root, s);
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(
                AppError::io(format!("读不到基准 {}", path.display())).with_detail(e.to_string())
            )
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
        crate::fsx::atomic::atomic_write(&path, b"# \xe8\xa2\xab\xe6\x94\xb9\xe8\xbf\x87\n").unwrap();
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
        assert_eq!(read_baseline(d.path(), &sha).unwrap(), None, "还没有就是 None");
        ensure_baseline(d.path(), &sha, text.as_bytes()).unwrap();
        assert_eq!(read_baseline(d.path(), &sha).unwrap().as_deref(), Some(text));
    }

    /// 字节与名字对不上 = 当它没有（宁可回退出厂值，也不拿可疑字节当默认）
    #[test]
    fn a_baseline_whose_bytes_do_not_match_its_name_is_not_used() {
        let d = tempfile::tempdir().unwrap();
        let text = "# machine: A1\n";
        let sha = sha_of(text);
        ensure_baseline(d.path(), &sha, text.as_bytes()).unwrap();
        crate::fsx::atomic::atomic_write(&baseline_path(d.path(), &sha), b"# \xe5\x88\xab\xe7\x9a\x84\n")
            .unwrap();
        assert_eq!(read_baseline(d.path(), &sha).unwrap(), None);
    }

    /// 摘要形状不对一律拒绝 —— 它是用户能改的注释行，不许拿它拼路径
    #[test]
    fn a_non_sha_is_refused_before_it_touches_the_disk() {
        let d = tempfile::tempdir().unwrap();
        for bad in ["", "../../x", "abc", &"z".repeat(64)] {
            assert!(ensure_baseline(d.path(), bad, b"x").is_err(), "{bad} 该被拒");
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
}
