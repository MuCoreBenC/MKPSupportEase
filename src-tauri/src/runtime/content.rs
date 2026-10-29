//! **云端原文内容仓库**：一份内容只存一份，按 SHA-256 寻址。
//!
//! # 它回答什么
//!
//! 「云端某一版长什么样」—— 三方更新要读用户实际基于的那一版，恢复默认要读云端原文，
//! 目录索引要按 path 找到内容。**内容与路径无关**：两个云端路径内容相同就共用一份；
//! 云端发布新版就是新的 SHA，旧内容自然成为历史版本。
//!
//! # 与目录索引的关系
//!
//! `run/delivery-index.json` 记「当前这台机器手上是哪一版的 path → sha」；
//! 这里只按 sha 存字节。**目录换代只换索引，不动内容** —— 旧版因此不会因为
//! 换目录或删除用户文件而丢失。
//!
//! # 幂等与校验
//!
//! 落点 `<appDataDir>/content/<sha256>.toml`。摘要就是内容：
//! - 同一版写多少次都只落一份，第二次一个字节不动；
//! - 读出时**字节与名字对不上就当没有**（不拿可疑字节冒充云端原文）。
//!
//! # 清理
//!
//! 内容不是"当前版本就留、否则删"：同一个 SHA 可能同时被当前目录、用户血统、
//! 回收记录、逐参数更新账引用。清理前由 [`gc`] 统计引用，只删没人引用且超过保留期的。

use postprocess::diag::messages;

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::AppError;

/// 内容仓库的目录名。相对**内部根**（`appDataDir`）。
pub const CONTENT_DIR: &str = "content";

/// 保留策略：最多留多少份、最多留多少天（两者先到先清）。
pub const KEEP_COUNT: usize = 20;
pub const KEEP_DAYS: u64 = 30;

/// 内容仓库：`<appDataDir>/content`
pub fn content_dir(internal_root: &Path) -> PathBuf {
    internal_root.join(CONTENT_DIR)
}

/// 一份内容的落点：`<appDataDir>/content/<sha256>.toml`
pub fn content_path(internal_root: &Path, sha256: &str) -> PathBuf {
    content_dir(internal_root).join(format!("{}.toml", sha256.trim().to_ascii_lowercase()))
}

/// 摘要必须是一个真的 sha256，才允许拿来拼路径。
///
/// **这不是"顺手校验"**：它同时是防穿越的一道闸 —— 摘要来自用户文件头里的那行注释，
/// 而那是用户能改的东西。放一个 `../../x` 进来就会写到用户根外面去。
pub fn check_sha(sha256: &str) -> Result<&str, AppError> {
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
pub fn exists(internal_root: &Path, sha256: &str) -> bool {
    match check_sha(sha256) {
        Ok(s) => content_path(internal_root, s).is_file(),
        Err(_) => false,
    }
}

/// 落一份内容。**幂等**：已经有就一个字节不动。
///
/// 返回 `true` = 这次真的写了一份新的。落不上不该让下载失败 ——
/// 调用方按"附加信息"处理（与事件账、出处账同一条口径）。
pub fn ensure(internal_root: &Path, sha256: &str, bytes: &[u8]) -> Result<bool, AppError> {
    let s = check_sha(sha256)?;
    let target = content_path(internal_root, s);
    if target.is_file() {
        return Ok(false);
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            AppError::io(messages::text(
                "ui.runtime.content.505",
                &[("v1", &parent.display().to_string())],
            ))
            .with_detail(e.to_string())
        })?;
    }
    crate::fsx::atomic::atomic_write(&target, bytes)?;
    Ok(true)
}

/// 只读一份内容的正文。没有 ⇒ `None`。
///
/// **字节与名字对不上也当没有**（并记一条告警）：宁可回退，也不许拿一份来历不明的
/// 字节冒充"云端默认"。这里不报错 —— 内容缺失是一个**正常状态**
/// （导入的、从用户预设拷贝出来的那两份本来就没有云端原文）。
pub fn read(internal_root: &Path, sha256: &str) -> Result<Option<String>, AppError> {
    let s = check_sha(sha256)?;
    let path = content_path(internal_root, s);
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(AppError::io(messages::text(
                "ui.runtime.content.506",
                &[("v1", &path.display().to_string())],
            ))
            .with_detail(e.to_string()))
        }
    };
    let got = super::catalog::hex(&Sha256::digest(&bytes));
    if got != s {
        tracing::warn!(want = %s, got = %got, "内容的字节与它的名字对不上，不当它是云端原文");
        return Ok(None);
    }
    match String::from_utf8(bytes) {
        Ok(text) => Ok(Some(text)),
        Err(_) => {
            tracing::warn!(sha = %s, "内容不是 UTF-8 文本，读不出来");
            Ok(None)
        }
    }
}

/// 只读一份内容的原始字节（下载管道用，不要求 UTF-8）。
pub fn read_bytes(internal_root: &Path, sha256: &str) -> Option<Vec<u8>> {
    let s = check_sha(sha256).ok()?;
    let bytes = std::fs::read(content_path(internal_root, s)).ok()?;
    let got = super::catalog::hex(&Sha256::digest(&bytes));
    if got != s {
        tracing::warn!(want = %s, got = %got, "内容的字节与它的名字对不上，不当它是可用内容");
        return None;
    }
    Some(bytes)
}

/// 内容仓库里都有哪些版本（按文件名升序）。**给云端版本列表用**。
pub fn shas(internal_root: &Path) -> Vec<String> {
    let dir = content_dir(internal_root);
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

/// 某份内容最后一次落盘的时刻（UTC epoch 秒）。清理时"最旧优先"用它。
fn modified_unix(internal_root: &Path, sha: &str) -> Option<u64> {
    let meta = std::fs::metadata(content_path(internal_root, sha)).ok()?;
    meta.modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}

/// 删一份内容。**只允许在引用检查通过后调用**（见 [`gc`]）。
pub fn remove(internal_root: &Path, sha256: &str) -> Result<bool, AppError> {
    let s = check_sha(sha256)?;
    match std::fs::remove_file(content_path(internal_root, s)) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(AppError::io(messages::text(
            "ui.runtime.content.507",
            &[("v1", &content_path(internal_root, s).display().to_string())],
        ))
        .with_detail(e.to_string())),
    }
}

/// **换代之前把旧索引仍引用的内容确认已入仓库**。
///
/// 缺的按 `archive/<path>`（换版时被换下的那一份）→ `<root>/<path>`（迁移还没跑完的
/// 旧交付面）顺序补写回 `content/`；两处都没有、或字节与索引记的 SHA 对不上 ⇒ 只告警，
/// **不编字节**。
///
/// 为什么非有这一步：换代只换索引，目录里没有的 path 会从索引里被删掉。若这一版在
/// `content/` 里没有、只躺在 `archive/` 里，换完这一代就再没有"它曾经是哪一版"的记录 ——
/// 旧版不是被删了，是被忘了。所以顺序永远是**先入库、再换索引**。
///
/// 返回这次补写了几份。
pub fn ensure_referenced(
    internal_root: &Path,
    index: &super::delivery_index::DeliveryIndex,
) -> Result<usize, AppError> {
    let mut recovered = 0usize;
    for (path, entry) in &index.files {
        if exists(internal_root, &entry.sha256) {
            continue;
        }
        let Some(bytes) = archived_version(internal_root, path, &entry.sha256) else {
            tracing::warn!(
                path = %path,
                "索引记着这一版，但内容仓库与归档里都没有它 —— 换代之后这一版无法再恢复"
            );
            continue;
        };
        if ensure(internal_root, &entry.sha256, &bytes)? {
            recovered += 1;
        }
    }
    if recovered > 0 {
        tracing::info!(recovered, "换代前把旧版本补进了内容仓库");
    }
    Ok(recovered)
}

/// 从归档区（或迁移前的旧交付面）里捞这一版字节。**SHA 对得上才要**。
fn archived_version(internal_root: &Path, path: &str, sha256: &str) -> Option<Vec<u8>> {
    let want = check_sha(sha256).ok()?;
    let archive = internal_root.join(super::paths::ARCHIVE_DIR);
    for base in [archive, internal_root.to_path_buf()] {
        let Ok(file) = crate::fsx::paths::resolve_in(&base, path) else {
            continue;
        };
        let Ok(bytes) = std::fs::read(&file) else {
            continue;
        };
        if super::catalog::hex(&Sha256::digest(&bytes)) == want {
            return Some(bytes);
        }
    }
    None
}

/// **清理内容仓库**：只删"没人引用、且超过保留期"的版本。
///
/// 引用来源（任一成立就留）：
/// 1. 当前目录索引里任一 `path → sha`（见 [`super::delivery_index`]）；
/// 2. 任一用户正式预设血统里的 `based_on_sha256`；
/// 3. 任一回收记录带来的 `LocalProvenance.source_sha256`；
/// 4. 逐参数「云端更新」账里的水位 `sha256`（[`pending_update_shas`]）；
/// 5. 用户文件自己此刻的字节（下载件带血统头，通常由 2 覆盖）。
///
/// 数量闸（[`KEEP_COUNT`]）与时间闸（[`KEEP_DAYS`]）**只看无人引用的**内容：
/// 被引用的旧版本再老也不删 —— 那是三方更新的依据。
pub fn gc(internal_root: &Path) -> Result<usize, AppError> {
    let mut referenced = super::delivery_index::referenced_shas(internal_root);
    referenced.extend(super::recovery::referenced_shas(internal_root));
    referenced.extend(local_lineage_shas(internal_root));
    referenced.extend(pending_update_shas(internal_root)?);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let mut orphans: Vec<(u64, String)> = shas(internal_root)
        .into_iter()
        .filter(|sha| !referenced.contains(sha))
        .map(|sha| (modified_unix(internal_root, &sha).unwrap_or(0), sha))
        .collect();
    orphans.sort();

    let mut removed = 0usize;
    let too_old = KEEP_DAYS.saturating_mul(24 * 60 * 60);
    for (at, sha) in orphans {
        let over_count = shas(internal_root).len() > KEEP_COUNT;
        let over_age = now.saturating_sub(at) > too_old;
        if !over_count && !over_age {
            continue;
        }
        if remove(internal_root, &sha)? {
            removed += 1;
        }
    }
    Ok(removed)
}

/// 逐参数「云端更新」账里记的水位 SHA（`appState.paramDecisions[*][*].sha256`）。
///
/// 那是「这个参数我处理到哪一版」：下次云端再改，要比的就是它。删掉那一版内容，
/// 等于把用户的处理史抹成"从没处理过"，云端下次发布又把这些项全翻出来。
///
/// 读不出来就**报错**（调用方据此放弃这次清理）：这本账说不清时，删哪一份都可能是错的。
fn pending_update_shas(
    internal_root: &Path,
) -> Result<std::collections::HashSet<String>, AppError> {
    let state = super::app_state::load(internal_root)?;
    Ok(state
        .param_decisions
        .values()
        .flat_map(|book| book.values())
        .filter_map(|d| check_sha(&d.sha256).ok())
        .map(|s| s.to_ascii_lowercase())
        .collect())
}

/// 扫描用户正式预设的血统头，收集 `based_on_sha256`。
///
/// 只读每个文件的前 8KB：血统头在最前面，用户文件通常几 KB；读全量在文件多时
/// 也没问题，但这里按最保守的读法来。
fn local_lineage_shas(internal_root: &Path) -> std::collections::HashSet<String> {
    let user = internal_root.join("user");
    let mut out = std::collections::HashSet::new();
    let mut stack = vec![user.join("presets-local")];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                stack.push(path);
                continue;
            }
            if !kind.is_file() {
                continue;
            }
            if path.extension().and_then(|s| s.to_str()) != Some("toml") {
                continue;
            }
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            let head = &bytes[..bytes.len().min(8192)];
            let Ok(text) = std::str::from_utf8(head) else {
                continue;
            };
            for line in text.lines() {
                let Some(rest) = line.trim_start().strip_prefix("# based_on_sha256:") else {
                    continue;
                };
                let sha = rest.trim();
                if check_sha(sha).is_ok() {
                    out.insert(sha.to_ascii_lowercase());
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fsx::atomic::atomic_write;

    fn sha_of(text: &str) -> String {
        super::super::catalog::hex(&Sha256::digest(text.as_bytes()))
    }

    /// 幂等：第二次一个字节不动 —— 这是"每版一份、永不覆盖"的实现保证
    #[test]
    fn the_same_version_is_written_once_and_never_touched() {
        let d = tempfile::tempdir().unwrap();
        let text = "# machine: A1\n[toolhead]\noffset_x = 1\n";
        let sha = sha_of(text);

        assert!(ensure(d.path(), &sha, text.as_bytes()).unwrap());
        // 有人在盘上动过它（模拟）：再 ensure 不许写回去
        let path = content_path(d.path(), &sha);
        atomic_write(&path, b"# \xe8\xa2\xab\xe6\x94\xb9\xe8\xbf\x87\n").unwrap();
        assert!(
            !ensure(d.path(), &sha, text.as_bytes()).unwrap(),
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
        assert_eq!(read(d.path(), &sha).unwrap(), None, "还没有就是 None");
        ensure(d.path(), &sha, text.as_bytes()).unwrap();
        assert_eq!(read(d.path(), &sha).unwrap().as_deref(), Some(text));
    }

    /// 字节与名字对不上 = 当它没有（宁可回退，也不拿可疑字节当默认）
    #[test]
    fn content_whose_bytes_do_not_match_its_name_is_not_used() {
        let d = tempfile::tempdir().unwrap();
        let text = "# machine: A1\n";
        let sha = sha_of(text);
        ensure(d.path(), &sha, text.as_bytes()).unwrap();
        atomic_write(
            &content_path(d.path(), &sha),
            b"# \xe5\x88\xab\xe7\x9a\x84\n",
        )
        .unwrap();
        assert_eq!(read(d.path(), &sha).unwrap(), None);
    }

    /// 摘要形状不对一律拒绝 —— 它是用户能改的注释行，不许拿它拼路径
    #[test]
    fn a_non_sha_is_refused_before_it_touches_the_disk() {
        let d = tempfile::tempdir().unwrap();
        for bad in ["", "../../x", "abc", &"z".repeat(64)] {
            assert!(ensure(d.path(), bad, b"x").is_err(), "{bad} 该被拒");
            assert!(!exists(d.path(), bad));
        }
    }

    /// 列表按摘要给出，只认 `64 位 hex + .toml` 的文件名
    #[test]
    fn the_list_reads_back_what_is_there() {
        let d = tempfile::tempdir().unwrap();
        assert!(shas(d.path()).is_empty(), "空表");
        let a = sha_of("a");
        let b = sha_of("b");
        ensure(d.path(), &a, b"a").unwrap();
        ensure(d.path(), &b, b"b").unwrap();
        std::fs::create_dir_all(content_dir(d.path())).unwrap();
        atomic_write(&content_dir(d.path()).join("readme.txt"), b"x").unwrap();
        let mut want = vec![a.clone(), b.clone()];
        want.sort();
        assert_eq!(shas(d.path()), want);
    }

    /// 造 `n` 份内容、返回它们的 SHA（清理闸的测试用）。
    fn fill(root: &Path, n: usize) -> Vec<String> {
        let mut out = Vec::new();
        for i in 0..n {
            let text = format!("# v{i}\n");
            let sha = sha_of(&text);
            ensure(root, &sha, text.as_bytes()).unwrap();
            out.push(sha);
        }
        out
    }

    /// 把这一批 SHA 全记进逐参数「云端更新」账（= 它们都是"待处理更新"的水位）。
    fn watermark(root: &Path, shas: &[String]) {
        let entries: Vec<(String, super::super::app_state::ParamDecision)> = shas
            .iter()
            .enumerate()
            .map(|(i, sha)| {
                (
                    format!("toolhead.p{i}"),
                    super::super::app_state::ParamDecision {
                        sha256: sha.clone(),
                        kind: super::super::app_state::ParamDecisionKind::Hold,
                    },
                )
            })
            .collect();
        super::super::app_state::record_param_decisions(
            root,
            "presets-local/A1-fast.toml",
            &entries,
        )
        .unwrap();
    }

    /// **换代先入库**：旧索引还引用的那一版，内容仓库里没有、字节只在归档里 ⇒
    /// 换代之前必须补回来，否则换完索引就再没有它的记录。
    #[test]
    fn versions_still_referenced_by_the_old_index_are_pulled_back_from_the_archive() {
        let d = tempfile::tempdir().unwrap();
        let text = "# machine: A1\n[toolhead]\noffset_x = 1\n";
        let sha = sha_of(text);

        let mut index = super::super::delivery_index::DeliveryIndex::default();
        index.files.insert(
            "delivery/mkp/presets/A1-fast.toml".to_owned(),
            super::super::delivery_index::Entry {
                sha256: sha.clone(),
                size: Some(text.len() as u64),
                at: 0,
            },
        );
        /* 旧版字节只在归档里（上一代换版时被换下的那份） */
        let archived = d.path().join("archive/delivery/mkp/presets/A1-fast.toml");
        std::fs::create_dir_all(archived.parent().unwrap()).unwrap();
        atomic_write(&archived, text.as_bytes()).unwrap();

        assert_eq!(ensure_referenced(d.path(), &index).unwrap(), 1, "补了一份");
        assert_eq!(read(d.path(), &sha).unwrap().as_deref(), Some(text));
        assert_eq!(
            ensure_referenced(d.path(), &index).unwrap(),
            0,
            "已经在了就不动"
        );
    }

    /// 归档里的字节与索引记的 SHA 对不上 ⇒ 宁可不补，也不拿另一版冒充
    #[test]
    fn a_version_that_does_not_match_the_index_is_not_adopted() {
        let d = tempfile::tempdir().unwrap();
        let mut index = super::super::delivery_index::DeliveryIndex::default();
        index.files.insert(
            "delivery/mkp/presets/A1-fast.toml".to_owned(),
            super::super::delivery_index::Entry {
                sha256: sha_of("# 该是这一版\n"),
                size: None,
                at: 0,
            },
        );
        let archived = d.path().join("archive/delivery/mkp/presets/A1-fast.toml");
        std::fs::create_dir_all(archived.parent().unwrap()).unwrap();
        atomic_write(&archived, b"# \xe5\x8f\xa6\xe4\xb8\x80\xe7\x89\x88\n").unwrap();

        assert_eq!(ensure_referenced(d.path(), &index).unwrap(), 0);
        assert!(
            shas(d.path()).is_empty(),
            "字节对不上就不入库，不许编一份出来"
        );
    }

    /// 数量闸的正面：没人引用时，超出 [`KEEP_COUNT`] 的按最旧清掉
    #[test]
    fn the_count_gate_trims_versions_nobody_references() {
        let d = tempfile::tempdir().unwrap();
        let all = fill(d.path(), KEEP_COUNT + 1);
        assert_eq!(shas(d.path()).len(), KEEP_COUNT + 1);

        assert_eq!(gc(d.path()).unwrap(), 1, "只清掉超出上限的那份");
        assert_eq!(shas(d.path()).len(), KEEP_COUNT);
        /* 清掉的是哪一份由"最旧优先"定（同一秒落盘时按 SHA 排），这里只钉"少了一份" */
        assert!(all.iter().filter(|sha| !exists(d.path(), sha)).count() == 1);
    }

    /// ★ 待处理更新的水位 SHA 也算引用：那是"这个参数我处理到哪一版"，
    /// 删了内容就等于把用户的处理史抹掉，云端下次发布又把这些项全翻出来。
    #[test]
    fn a_pending_update_watermark_keeps_its_version() {
        let d = tempfile::tempdir().unwrap();
        let all = fill(d.path(), KEEP_COUNT + 1);
        watermark(d.path(), &all);

        assert_eq!(gc(d.path()).unwrap(), 0, "全被水位引用 ⇒ 一份都不许删");
        assert_eq!(shas(d.path()).len(), KEEP_COUNT + 1, "超出上限也留着");
    }
}
