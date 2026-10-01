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
//! # 用户那份与官方的关系：**血统写在文件里**
//!
//! 它从哪一份官方、哪一版拷出来的（`# based_on*` 三行）随文件走，见 [`super::lineage`]。
//! 于是"官方换版了、你这份还是基于旧版"这件事，**换一台电脑也认得出** ——
//! 不需要程序另记一份账（作者 2026-10-02 定的原则：文件本身的信息随文件走）。
//!
//! # 读在上面，写只有两条（都是"写我自己的文件"）
//!
//! 这一层读用户目录（列出 / 认类别 / 读正文 / 读血统）；**写用户根只有两个函数**，
//! 对应两条**不同**的事，不许合并成一个"存一下"：
//!
//! ```text
//! commit_draft  另存：官方那份改出来的 → presets-mine/<原名>（已修改）.toml（第 5 层）
//! save_back     写回自己：我那份打开再存 → 同一个路径，不产生第二份（第 8 层）
//! ```
//!
//! 两条共用一句话：**官方原件（`mkp/` 与下载区）一概不碰**。区别只在"落点是谁"与
//! "那三行血统从哪来"：另存是新的一份、血统从**来源**算；写回还是同一份、血统
//! **照抄文件里原来那三行**（出处没变 —— 见 [`super::lineage::rewrite_keeping_lineage`]）。
//! 于是"改我那份 → 保存"不会产出 `（已修改）2.toml`，也不会把出处改成"基于我自己"。

use std::path::Path;

use crate::error::AppError;
use crate::fsx::paths::MINE_DIR;

use super::catalog::Catalog;
use super::lineage::{self, Lineage};

/// 读血统时最多看文件头这么多字节。
///
/// 三行血统住在**文件头注释块**里（头几行），所以够用；而用户目录是**用户自己的地盘**，
/// 他可能往里扔一个几百 MB 的文件 —— 为了读三行注释把整份读进内存不是这一层该干的事。
pub const LINEAGE_READ_LIMIT: u64 = 8 * 1024;

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
    /// 它从哪一份官方、哪一版拷出来的（文件头那三行）。`None` = 这份没有血统
    /// （手工拷的、或别的程序写出来的）—— **不是错误**
    pub lineage: Option<Lineage>,
}

/// 这份用户文件**基于的官方版本**现在怎么样了。
///
/// 三档只回答一个问题：**它当初基于的那一版，和目录里现在这一版是不是同一份** ——
/// 不回答"这份用户文件好不好"（它是用户自己的文件，从来不是"坏文件"）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BasedOn {
    /// 基于目录里**当前**那一版（官方没换版）
    Current,
    /// 基于官方的**旧版**：官方已经换新版了。**这份用户文件照常能用、能改** ——
    /// 它只是"从旧版派生"的
    Outdated,
    /// 说不清：没有血统，或血统指的那一份已经不在目录里（换源 / 下线 / 改过名）
    Unknown,
}

/// 读一份文件的头注释（最多 [`LINEAGE_READ_LIMIT`] 字节）拿血统。
///
/// 读不出来（不是文本、没权限、文件太大被截断而三行不在前面）⇒ `None`：
/// 这一层不为此报错 —— 血统是**附加信息**，缺了不影响这份文件被认成用户的预设。
pub fn lineage_of_file(path: &Path) -> Option<Lineage> {
    let bytes = read_head(path, LINEAGE_READ_LIMIT)?;
    lineage::parse_lineage_from_content(&String::from_utf8_lossy(&bytes))
}

/// 只读前 `limit` 字节（不把整个文件读进内存）
fn read_head(path: &Path, limit: u64) -> Option<Vec<u8>> {
    use std::io::Read;
    let file = std::fs::File::open(path).ok()?;
    let mut buf = Vec::new();
    file.take(limit).read_to_end(&mut buf).ok()?;
    Some(buf)
}

/// **这份用户文件是从哪一份官方派生的、那一版现在还在不在**。
///
/// 血统里存的是 `based_on`（角色目录 + 文件名，形如 `mkp/presets/A1-standard.toml`）
/// 与 `based_on_sha256`（那时候官方那一版**全文**的摘要）。拿它跟目录里现在登记的那一份比：
///
/// - 摘一样 ⇒ [`BasedOn::Current`]（官方没换版）
/// - 摘不一样 ⇒ [`BasedOn::Outdated`]（官方换版了）—— **这就是第七层要说的那件事**
/// - 目录里已经没有它 ⇒ [`BasedOn::Unknown`]（说不出新旧）
///
/// 只看血统里记的那一份，**不看用户文件自己的字节**：用户改过的东西必然与官方不同，
/// 拿它去比只会得出"永远不一样"这种废话。
pub fn based_on(catalog: &Catalog, lineage: Option<&Lineage>) -> BasedOn {
    let Some(lineage) = lineage else {
        return BasedOn::Unknown;
    };
    let Some(source) = lineage.based_on.as_deref() else {
        return BasedOn::Unknown;
    };
    let known = catalog
        .files
        .iter()
        .find(|f| f.path == source)
        .or_else(|| catalog.files.iter().find(|f| f.file_name == source));
    let Some(file) = known else {
        return BasedOn::Unknown;
    };
    match lineage.based_on_sha256.as_deref() {
        Some(sha) if sha == file.sha256 => BasedOn::Current,
        Some(_) => BasedOn::Outdated,
        /* 有来源没摘要：比不了 ⇒ 说不清新旧（缺的那项就是"不知道"） */
        None => BasedOn::Unknown,
    }
}

/// 血统指的那一份官方，现在**对应哪台机型的哪个版本**（界面要说人话用）。
/// 说不清就 `None` —— 不猜。
pub fn source_of<'a>(
    catalog: &'a Catalog,
    lineage: Option<&Lineage>,
) -> Option<&'a super::catalog::CatalogFile> {
    let source = lineage?.based_on.as_deref()?;
    catalog
        .files
        .iter()
        .find(|f| f.path == source)
        .or_else(|| catalog.files.iter().find(|f| f.file_name == source))
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
                lineage: lineage_of_file(&path),
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
///
/// 写下去的是**副本的形状**：`text` + 头注释块里三行血统（[`super::lineage::make_copy`]）——
/// `based_on` = 来源那份在**交付根里的相对路径**（`mkp/presets/A1-standard.toml`，
/// 与工作台建副本的"角色目录 + 文件名"同一形状）。于是这份文件拷到哪台电脑上都说得清
/// 自己从哪来、基于哪一版（第七层：「官方换版了、你这份还是基于旧版」就靠它判）。
pub fn commit_draft(
    user_root: &Path,
    source_file_name: &str,
    based_on: &str,
    text: &str,
) -> Result<Committed, AppError> {
    let file_name = edited_name(source_file_name);
    let rel = format!("{MINE_DIR}/{file_name}");
    /* 名字是从目录里的文件名派生的，但仍然过一遍防穿越闸 */
    let target = crate::fsx::paths::resolve_in(user_root, &rel)?;
    let replaced = target.exists();
    let body = super::lineage::make_copy(text, based_on);
    crate::fsx::atomic::atomic_write(&target, body.as_bytes())?;
    Ok(Committed {
        path: rel,
        file_name,
        size: body.len() as u64,
        replaced,
    })
}

/// **把编辑后的正文写回它自己**（第八层）：**同一个路径、同一份文件，不产生第二份**。
///
/// 与 [`commit_draft`]（另存）只有两处不同：
///
/// - **落点是原来那条路径**，不是派生的新名字 —— 用户改的就是"我那份"，
///   所以不会出现 `（已修改）2.toml` / `（再次修改）.toml` 这种越改越多的名字；
/// - **三行血统照抄文件里原来那三行**（出处没变，见
///   [`super::lineage::rewrite_keeping_lineage`]）—— 保存之后它仍然说得清
///   "我从哪一版官方派生"，也不会因为重算摘要变成"基于我自己改过的字节"。
///   这份本来就没有血统（手工拷的 / 别的程序写出来的）⇒ 照实不写，**不编一个出处**。
///
/// 编辑期间这份被移走 / 删掉了 ⇒ **拒绝并说清**：不去别处新建一份（那不是用户点的
/// 那个"保存"），也不假装存成了。
pub fn save_back(user_root: &Path, rel: &str, text: &str) -> Result<Committed, AppError> {
    check_mine_prefix(rel)?;
    let target = crate::fsx::paths::resolve_in(user_root, rel)?;
    if !target.is_file() {
        return Err(AppError::not_found(format!(
            "{rel} 已经不在原来的位置了（可能被移走或删掉了）—— 没有动别的地方"
        )));
    }
    /* 血统是**这份文件自己的属性**：先读出来再照抄回去（读不出来 = 它本来就没有） */
    let lineage = lineage_of_file(&target);
    let body = super::lineage::rewrite_keeping_lineage(text, lineage.as_ref());
    crate::fsx::atomic::atomic_write(&target, body.as_bytes())?;
    Ok(Committed {
        path: rel.to_owned(),
        file_name: target
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| rel.to_owned()),
        size: body.len() as u64,
        /* 写的就是原来那一份所在的位置：当然是覆盖 */
        replaced: true,
    })
}

/// 读用户自己那份的正文（**只认 `presets-mine/`**，见 [`check_mine_prefix`]）。
///
/// 不是 UTF-8 就如实报错 —— 用户自己的文件也一样，读不出来就说读不出来，不装成空正文。
pub fn read_text(user_root: &Path, rel: &str) -> Result<String, AppError> {
    check_mine_prefix(rel)?;
    let target = crate::fsx::paths::resolve_in(user_root, rel)?;
    let bytes = std::fs::read(&target)
        .map_err(|_| AppError::not_found(format!("找不到 {rel} —— 它可能已经被移走或删掉了")))?;
    String::from_utf8(bytes)
        .map_err(|_| AppError::corrupted(format!("{rel} 不是 UTF-8 文本，这一条读不出来")))
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

    /// 造一份目录登记（SHA/大小都对得上），用来试「基于的官方那一版现在是什么样」
    fn entry_bytes(name: &str, content: &str) -> super::super::catalog::CatalogFile {
        super::super::catalog::CatalogFile {
            kind: "mkp_preset".to_owned(),
            file_name: name.to_owned(),
            path: format!("mkp/presets/{name}"),
            machine_id: "A1".to_owned(),
            version_id: "STANDARD".to_owned(),
            sha256: lineage::sha256_hex(content),
            size: content.len() as u64,
        }
    }

    fn catalog_with(files: Vec<super::super::catalog::CatalogFile>) -> Catalog {
        Catalog {
            catalog_schema: super::super::catalog::CATALOG_SCHEMA,
            revision: "test".to_owned(),
            files,
            ..Catalog::default()
        }
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
    ///
    /// 写下去的是**副本的形状**：正文 + 头注释里三行血统（剪掉三行必须与正文逐字节相同）。
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

        let label = "mkp/presets/A1-fast.toml";
        let done = commit_draft(user.path(), "A1-fast.toml", label, "涂胶宽度 = 1.4").unwrap();
        assert_eq!(done.path, "presets-mine/A1-fast（已修改）.toml");
        assert!(!done.replaced, "第一次另存没有盖掉谁");
        let saved = std::fs::read_to_string(user.path().join(&done.path)).unwrap();
        assert_eq!(
            crate::runtime::lineage::strip_lineage_for_compare(&saved),
            "涂胶宽度 = 1.4",
            "剪掉血统三行就是用户改过的正文"
        );
        let got = crate::runtime::lineage::parse_lineage_from_content(&saved).expect("该带上血统");
        assert_eq!(got.based_on.as_deref(), Some(label), "说得出是从哪一份拷的");

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

        /* 再存一次：还是同一个名字，盖掉它自己（血统也还是三行，不叠加） */
        let again = commit_draft(user.path(), "A1-fast.toml", label, "涂胶宽度 = 1.5").unwrap();
        assert!(again.replaced, "第二次是覆盖");
        assert_eq!(mine_files(user.path()).len(), 1, "不会越存越多");
        let saved = std::fs::read_to_string(user.path().join(&again.path)).unwrap();
        assert_eq!(
            crate::runtime::lineage::strip_lineage_for_compare(&saved),
            "涂胶宽度 = 1.5"
        );
        assert_eq!(saved.matches("# based_on:").count(), 1, "血统不叠加");
    }

    /// 另存出来的那份，接着就能被用户线列出来、血统也读得回来（一条链的收尾连上了）
    #[test]
    fn the_committed_copy_shows_up_in_mine_files() {
        let user = tempfile::tempdir().unwrap();
        let label = "mkp/presets/A1-standard.toml";
        let done = commit_draft(user.path(), "我的 A1 涂胶.toml", label, "涂胶宽度 = 1.3").unwrap();

        let listed = mine_files(user.path());
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].path, done.path);
        assert_eq!(listed[0].kind, Some("mkp_preset"), ".toml 认得出");
        assert_eq!(
            listed[0]
                .lineage
                .as_ref()
                .and_then(|l| l.based_on.as_deref()),
            Some(label),
            "列出来的时候就把血统读出来了"
        );
        assert!(check_mine_prefix(&done.path).is_ok(), "落点在那一格里");
    }

    /* ---------- 写回自己（第八层：改我那份 → 保存） ---------- */

    /// **这一层的核心不变式**：改我那份 → 保存 = **写回同一份** ——
    /// 不产生 `（已修改）2.toml`，血统还是当初那三行（出处没变）
    #[test]
    fn saving_back_writes_the_same_file_and_keeps_the_lineage() {
        let user = tempfile::tempdir().unwrap();
        let label = "mkp/presets/A1-standard.toml";
        let done = commit_draft(user.path(), "A1-standard.toml", label, "涂胶宽度 = 1.0").unwrap();
        let lineage = lineage_of_file(&user.path().join(&done.path)).expect("副本该有血统");
        let edited = "涂胶宽度 = 1.4";

        let back = save_back(user.path(), &done.path, edited).unwrap();

        assert_eq!(back.path, done.path, "还是同一条路径，没有第二份");
        assert_eq!(back.file_name, done.file_name);
        assert!(back.replaced, "写的就是原来那一份所在的位置");
        assert_eq!(mine_files(user.path()).len(), 1, "不会越存越多");

        let after = std::fs::read_to_string(user.path().join(&back.path)).unwrap();
        assert_eq!(
            lineage::strip_lineage_for_compare(&after),
            edited,
            "正文就是改过的那份"
        );
        assert_eq!(
            lineage_of_file(&user.path().join(&back.path)),
            Some(lineage),
            "出处那三行一个字都没变 —— 不会变成\"基于我自己改过的字节\""
        );
    }

    /// **什么都没改就什么都没发生**：打开又保存，文件逐字节不变
    #[test]
    fn saving_back_an_untouched_file_changes_nothing() {
        let user = tempfile::tempdir().unwrap();
        let done = commit_draft(
            user.path(),
            "A1-standard.toml",
            "mkp/presets/A1-standard.toml",
            "涂胶宽度 = 1.0",
        )
        .unwrap();
        let before = std::fs::read(user.path().join(&done.path)).unwrap();
        let body = lineage::strip_lineage_for_compare(&String::from_utf8_lossy(&before));

        save_back(user.path(), &done.path, &body).unwrap();

        assert_eq!(
            std::fs::read(user.path().join(&done.path)).unwrap(),
            before,
            "打开又保存：逐字节不变"
        );
    }

    /// 本来就没有血统的那份（手工拷的 / 别的程序写的）写回时**不编一个出处**
    #[test]
    fn saving_back_a_file_without_lineage_invents_none() {
        let user = tempfile::tempdir().unwrap();
        write(user.path(), "presets-mine/我的.toml", "涂胶宽度 = 1.0");

        let back = save_back(user.path(), "presets-mine/我的.toml", "涂胶宽度 = 1.2").unwrap();

        let after = std::fs::read_to_string(user.path().join(&back.path)).unwrap();
        assert_eq!(after, "涂胶宽度 = 1.2", "没有血统就不写血统");
        assert_eq!(lineage_of_file(&user.path().join(&back.path)), None);
    }

    /// 编辑期间这份被移走 / 删掉了 ⇒ **拒绝并说清**，不去别处新建一份
    #[test]
    fn saving_back_refuses_when_the_file_is_gone() {
        let user = tempfile::tempdir().unwrap();
        let e = save_back(user.path(), "presets-mine/已经不在了.toml", "涂胶 = 1").unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::NotFound);
        assert!(
            !user.path().join("presets-mine/已经不在了.toml").exists(),
            "没有顺手建一份出来"
        );
    }

    /// 写回也只认用户自己那一格：`exports/`、`../` 一概不碰
    #[test]
    fn saving_back_stays_in_the_mine_dir() {
        let user = tempfile::tempdir().unwrap();
        assert!(save_back(user.path(), "exports/x.toml", "涂胶 = 1").is_err());
        assert!(save_back(user.path(), "../x.toml", "涂胶 = 1").is_err());
    }

    /* ---------- 基于官方哪一版（第七层：官方换版了没有） ---------- */

    /// 目录里登记着来源那一份，摘要一样 ⇒ 基于当前版；官方换了版 ⇒ 旧版
    #[test]
    fn the_official_update_shows_up_as_based_on_an_old_version() {
        let v1 = entry_bytes("A1-standard.toml", "官方第一版");
        let mut v2 = entry_bytes("A1-standard.toml", "官方第二版");
        let based_on_v1 = Lineage {
            based_on: Some("mkp/presets/A1-standard.toml".to_owned()),
            based_on_release_time: None,
            based_on_sha256: Some(lineage::sha256_hex("官方第一版")),
        };

        /* 目录里还是 v1：基于当前版 */
        assert_eq!(
            based_on(&catalog_with(vec![v1.clone()]), Some(&based_on_v1)),
            BasedOn::Current
        );

        /* 官方换到 v2：同一份用户文件变成「基于旧版」—— 而它不是坏文件 */
        v2.path = v1.path.clone();
        assert_eq!(
            based_on(&catalog_with(vec![v2.clone()]), Some(&based_on_v1)),
            BasedOn::Outdated,
            "官方换版了，用户那份还是基于旧版"
        );
        assert_eq!(
            source_of(&catalog_with(vec![v2]), Some(&based_on_v1)).map(|f| f.machine_id.as_str()),
            Some("A1"),
            "界面要说得出它是哪台机型哪一版改出来的"
        );
    }

    /// 说不清那几档：没有血统 / 血统里没摘要 / 来源已经不在目录里（换源、下线）
    #[test]
    fn unknown_when_the_source_cannot_be_resolved() {
        let catalog = catalog_with(vec![entry_bytes("A1-standard.toml", "官方")]);
        assert_eq!(based_on(&catalog, None), BasedOn::Unknown, "没有血统");

        let no_sha = Lineage {
            based_on: Some("mkp/presets/A1-standard.toml".to_owned()),
            based_on_release_time: None,
            based_on_sha256: None,
        };
        assert_eq!(
            based_on(&catalog, Some(&no_sha)),
            BasedOn::Unknown,
            "记了来源没记摘要 —— 比不了就是比不了"
        );

        let gone = Lineage {
            based_on: Some("mkp/presets/A1-gone.toml".to_owned()),
            based_on_release_time: None,
            based_on_sha256: Some(lineage::sha256_hex("官方")),
        };
        assert_eq!(
            based_on(&catalog, Some(&gone)),
            BasedOn::Unknown,
            "来源不在目录里（换源或下线）"
        );
        assert!(source_of(&catalog, Some(&gone)).is_none());
    }

    /// 读血统只看文件头那一小段：一个几百 MB 的文件不许为了让界面读三行注释就整份读进来
    #[test]
    fn lineage_is_read_from_the_head_only() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("big.toml");
        let mut text = String::from("# based_on: mkp/presets/A1-standard.toml\n");
        text.push_str(&"x".repeat((LINEAGE_READ_LIMIT as usize) * 2));
        crate::fsx::atomic::atomic_write(&path, text.as_bytes()).unwrap();

        assert!(
            lineage_of_file(&path).is_some(),
            "三行在文件头，头 8 KB 里就读得到"
        );
    }

    /// 不是文本 / 读不出来 ⇒ 没有血统（**不是错误**：血统是附加信息）
    #[test]
    fn an_unreadable_head_is_just_no_lineage() {
        let root = tempfile::tempdir().unwrap();
        assert!(lineage_of_file(&root.path().join("根本没有这个文件")).is_none());
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
