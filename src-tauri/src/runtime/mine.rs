//! **用户线**：用户自己的预设文件（`~/Documents/SupportEase/presets-mine/`）。
//!
//! ```text
//! 官方线（程序管，住 appDataDir）        用户线（用户自己要看要拷，住 Documents）
//!   mkp/ 下载的官方原件                  presets-mine/ 用户自己的那一份
//!   archive/ 换下来的官方旧版本
//! ```
//!
//! 两条线**不许混**（总纲 §1③「预设 TOML 的一生」）：
//!
//! - 官方原件**不可变**：只有云端换版本能替换它；
//! - 用户自己那份**从官方另存出来**，从此与云端脱钩，**永远不回写官方原件**；
//! - 用户什么都没改 = 什么都没发生（浏览 / 使用 / 关开不产生文件）。
//!
//! # 为什么它住 Documents 而不是 appDataDir
//!
//! 这一份是**给用户自己看、自己拷、自己留的**（见 `fsx::paths` 的模块注释：内部根不放
//! Documents 是因为 iCloud 会把文件驱逐成占位 stub，那是程序管理的数据不能待的地方）。
//!
//! # 这里只有读
//!
//! 「临时编辑 → 保存 → 用户文件」是**下一层**的事：今天的 `presets-mine/` 里只有
//! 用户手动放进去的东西，所以真机上多半是空的 —— **空是合法状态，不是错误**。
//! 这一层只回答"用户自己有哪些文件、是哪一类、正文是什么"。

use std::path::Path;

use crate::error::AppError;
use crate::fsx::paths::MINE_DIR;

/// 用户自己的一份文件。**盘就是底账**（与下载区、归档区同一套规矩）：扫盘得到，不记账本 ——
/// 账本一定会和盘漂移，而这一份的主人就是用户，他随时可能在 Finder 里动它。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MineFile {
    /// 相对**用户根**的路径（`presets-mine/A1-fast.toml`）—— 读正文时把它交回来
    pub path: String,
    /// 文件名。用户自己的文件**没有"交付身份"**：云端没有它，所以没有 SHA、不属于任何版本
    pub file_name: String,
    pub size: u64,
    /// 最后改动时刻（UTC epoch 秒）。`None` = 文件系统没给（不是 0，也不编一个）
    pub modified_unix: Option<u64>,
    /// 认得出是哪一类就给（见 [`kind_of`]）；**认不出是 `None`**，不猜
    pub kind: Option<&'static str>,
}

/// 按扩展名认类别。**只认 `.toml`（MKP 预设）**。
///
/// 切片器那两类（`bbs_profile` / `orca_profile`）都是 `.json` ——
/// 光看扩展名分不出是哪一种，所以**照实认不出**，不替用户猜一个。
/// 认不出的那种在界面上照样列出来（任何类型档下都列），只是不声称它是哪一类。
pub fn kind_of(file_name: &str) -> Option<&'static str> {
    file_name
        .to_ascii_lowercase()
        .ends_with(".toml")
        .then_some("mkp_preset")
}

/// 用户自己的文件有哪些（**按路径升序** —— `read_dir` 的顺序是文件系统说的，不稳定，
/// 界面要一个每次刷新都一样的表）。
///
/// 子目录也扫（用户可能自己分文件夹放）。目录不存在 = 一份都还没有：空表。
pub fn mine_files(user_root: &Path) -> Vec<MineFile> {
    let mut out = Vec::new();
    let mut stack = vec![user_root.join(MINE_DIR)];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue; // 目录不存在 / 读不动：当作"这一支没有东西"，不让整条读失败
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
            let Ok(meta) = entry.metadata() else { continue };
            let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let rel = path
                .strip_prefix(user_root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            out.push(MineFile {
                path: rel,
                kind: kind_of(&file_name),
                file_name,
                size: meta.len(),
                modified_unix: meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs()),
            });
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

/// 用户另存出来的那一份叫什么：`<原名（不含后缀）>（已修改）<后缀>`。
///
/// 例如 `A1-fast.toml` → `A1-fast（已修改）.toml`。用全角括号：这是**给人看的名字**，
/// 用户要在 Finder 里一眼认出"这是我改过的那一份"；半角括号在文件名里太像代码。
///
/// 再存一次**还是这个名字**（覆盖它自己）—— 用户改的就是"我那份"，不该越存越多。
pub fn edited_name(source_file_name: &str) -> String {
    let path = Path::new(source_file_name);
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| source_file_name.to_owned());
    match path.extension() {
        Some(ext) => format!("{stem}（已修改）.{}", ext.to_string_lossy()),
        None => format!("{stem}（已修改）"),
    }
}

/// 另存完成的结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Committed {
    /// 相对**用户根**的路径（`presets-mine/A1-fast（已修改）.toml`）
    pub path: String,
    pub file_name: String,
    pub size: u64,
    /// 盖掉了一份同名的用户文件（第二次保存就是这种）
    pub replaced: bool,
}

/// **临时编辑的收尾：把草稿另存成用户自己的文件**（`presets-mine/<原名>（已修改）<后缀>`）。
///
/// 这一刀全层的核心不变式就在这个函数里：它**只写用户根**——
/// 官方原件（`mkp/`）与下载区**一概不碰**（只有云端换版本能替换官方原件）。
/// 再存一次就是**覆盖它自己**：用户改的是"我那份"，不该越存越多。
pub fn commit_draft(
    user_root: &Path,
    source_file_name: &str,
    text: &str,
) -> Result<Committed, AppError> {
    let file_name = edited_name(source_file_name);
    let rel = format!("{MINE_DIR}/{file_name}");
    /* 名字是从目录里的文件名派生的，但仍然过一遍防穿越闸 */
    let target = crate::fsx::paths::resolve_in(user_root, &rel)?;
    let replaced = target.exists();
    crate::fsx::atomic::atomic_write(&target, text.as_bytes())?;
    Ok(Committed {
        path: rel,
        file_name,
        size: text.len() as u64,
        replaced,
    })
}

/// 只认 `presets-mine/` 里的东西：读正文的入参必须是 [`mine_files`] 给的那条路径起头。
///
/// 两道闸（第三道在 [`crate::fsx::paths::resolve_in`] 里，比真实路径挡符号链接）：
/// 1. **前缀**：别越到用户根下别的目录去（`exports/` `reports/` 那些不是这一层的事）；
/// 2. **相对路径的基本形状**（[`crate::fsx::paths::check_relative`]）：绝对路径、`..` 一概拒。
///
/// 两道都放在读文件**之前**：不合形状的路径不该先碰到盘。
pub fn check_mine_prefix(rel: &str) -> Result<(), AppError> {
    let prefix = format!("{MINE_DIR}/");
    if !rel.starts_with(&prefix) {
        return Err(AppError::invalid_argument(format!(
            "只读用户自己的预设（要 {prefix}… 开头，给的是 {rel}）"
        )));
    }
    crate::fsx::paths::check_relative(rel)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 造一份用户文件。写盘走仓库唯一那个出口（`clippy.toml` 禁 `std::fs::write`）
    fn write(root: &Path, rel: &str, text: &str) {
        crate::fsx::atomic::atomic_write(&root.join(rel), text.as_bytes()).unwrap();
    }

    /// 一份都没有 = 空表，不是错误（今天真机上就是这个状态：产生用户文件的是下一层）
    #[test]
    fn mine_files_is_empty_when_the_user_has_nothing() {
        let root = tempfile::tempdir().unwrap();
        assert!(mine_files(root.path()).is_empty());
    }

    /// 盘当底账：用户放进去了什么就列什么。路径**相对用户根**（`presets-mine/…`），
    /// 大小与时刻是真值；子目录里的也算
    #[test]
    fn mine_files_lists_what_the_user_put_there() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "presets-mine/A1-fast.toml", "涂胶 = 1");
        write(root.path(), "presets-mine/我的/另存.toml", "涂胶 = 2");
        write(root.path(), "exports/不该看见.json", "{}");

        let got = mine_files(root.path());
        let paths: Vec<&str> = got.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(
            paths,
            vec!["presets-mine/A1-fast.toml", "presets-mine/我的/另存.toml",],
            "按路径升序、只出 presets-mine 里的、子目录也出"
        );
        assert_eq!(got[0].file_name, "A1-fast.toml");
        assert_eq!(got[0].size, "涂胶 = 1".len() as u64);
        assert!(got[0].modified_unix.is_some(), "时刻要带上");
        assert_eq!(got[0].kind, Some("mkp_preset"), ".toml 认得出来是 MKP 预设");
    }

    /// **认不出就不认**：`.json` 可能是 bbs 也可能是 orca，光看扩展名分不出
    #[test]
    fn kind_is_only_claimed_when_the_extension_says_so() {
        assert_eq!(kind_of("A1.toml"), Some("mkp_preset"));
        assert_eq!(kind_of("A1.TOML"), Some("mkp_preset"), "扩展名大小写无所谓");
        assert_eq!(
            kind_of("Process.json"),
            None,
            "bbs 与 orca 都是 .json，分不出"
        );
        assert_eq!(kind_of("说明.md"), None);
        assert_eq!(kind_of("没有扩展名"), None);
    }

    /// 另存出来的名字：原名 + `（已修改）`，后缀留在最后
    #[test]
    fn edited_name_keeps_the_stem_and_marks_it() {
        assert_eq!(edited_name("A1-fast.toml"), "A1-fast（已修改）.toml");
        assert_eq!(
            edited_name("我的 A1 涂胶.toml"),
            "我的 A1 涂胶（已修改）.toml"
        );
        assert_eq!(edited_name("没有后缀"), "没有后缀（已修改）");
        assert_eq!(
            edited_name("a.b.toml"),
            "a.b（已修改）.toml",
            "只剥最后那一段后缀"
        );
    }

    /// **这一层的核心不变式**：另存只写用户根 —— 官方原件（`mkp/`）字节不变、
    /// 下载区里不会多出文件（临时文件不住那儿）。再存一次是覆盖它自己。
    #[test]
    fn commit_writes_the_edited_copy_and_leaves_the_official_alone() {
        let user = tempfile::tempdir().unwrap();
        let official = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(official.path().join("mkp/presets")).unwrap();
        crate::fsx::atomic::atomic_write(
            &official.path().join("mkp/presets/A1-fast.toml"),
            "涂胶宽度 = 1.0".as_bytes(),
        )
        .unwrap();
        let before = std::fs::read(official.path().join("mkp/presets/A1-fast.toml")).unwrap();
        let files_before = std::fs::read_dir(official.path().join("mkp/presets"))
            .unwrap()
            .count();

        let done = commit_draft(user.path(), "A1-fast.toml", "涂胶宽度 = 1.4").unwrap();
        assert_eq!(done.path, "presets-mine/A1-fast（已修改）.toml");
        assert!(!done.replaced, "第一次另存没有盖掉谁");
        assert_eq!(
            std::fs::read(user.path().join(&done.path)).unwrap(),
            "涂胶宽度 = 1.4".as_bytes(),
            "用户那份里是改过的正文"
        );

        assert_eq!(
            std::fs::read(official.path().join("mkp/presets/A1-fast.toml")).unwrap(),
            before,
            "官方原件一个字节都没动"
        );
        assert_eq!(
            std::fs::read_dir(official.path().join("mkp/presets"))
                .unwrap()
                .count(),
            files_before,
            "下载区里不会多出东西：临时文件不住 mkp/"
        );

        /* 再存一次：还是同一个名字，盖掉它自己 */
        let again = commit_draft(user.path(), "A1-fast.toml", "涂胶宽度 = 1.5").unwrap();
        assert!(again.replaced, "第二次是覆盖");
        assert_eq!(mine_files(user.path()).len(), 1, "不会越存越多");
        assert_eq!(
            std::fs::read(user.path().join(&again.path)).unwrap(),
            "涂胶宽度 = 1.5".as_bytes()
        );
    }

    /// 另存出来的那份，接着就能被用户线列出来、也读得回来（一条链的收尾连上了）
    #[test]
    fn the_committed_copy_shows_up_in_mine_files() {
        let user = tempfile::tempdir().unwrap();
        let done = commit_draft(user.path(), "我的 A1 涂胶.toml", "涂胶宽度 = 1.3").unwrap();

        let listed = mine_files(user.path());
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].path, done.path);
        assert_eq!(listed[0].kind, Some("mkp_preset"), ".toml 认得出");
        assert_eq!(listed[0].size, "涂胶宽度 = 1.3".len() as u64);
        assert!(check_mine_prefix(&done.path).is_ok(), "落点在那一格里");
    }

    /// 读正文只认用户自己那一格：别的地方（`exports/`、`../`）一概不碰
    #[test]
    fn only_the_mine_subdir_is_readable() {
        assert!(check_mine_prefix("presets-mine/A1.toml").is_ok());
        assert!(check_mine_prefix("exports/report.md").is_err());
        assert!(
            check_mine_prefix("presets-mine").is_err(),
            "目录本身不是一份文件"
        );
        assert!(check_mine_prefix("../secret.toml").is_err());
        assert!(check_mine_prefix("presets-mine/../../secret.toml").is_err());
    }
}
