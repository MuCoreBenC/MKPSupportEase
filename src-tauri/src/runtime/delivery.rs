//! 公共下载管道（总纲第④层的客户端那一半）。
//!
//! 总纲定的规矩：**不管送的是 TOML、JSON、模型还是图片，物流只认七个问题**——
//! 它是谁 → 从哪里来 → SHA 是多少 → 下载到哪里 → 是否已经有了 → 更新怎么办 → 旧的去哪。
//! 这条管道把七个问题全包了：前四个是 [`deliver`]，"是否已有"看 [`FileOnDisk`]，
//! "更新怎么办 / 旧的去哪"是**归档**——换新之前旧份进 `archive/`，不删（总纲十问 #9）。
//! 以后 Preset、模型、BBS 配置的下载**都走这一条路**，业务不许自己再发明一套
//! （那是"半年后又变成现在这个样子"的防波堤）。
//!
//! # 源是可插拔的
//!
//! [`Source`] 是管道唯一的口子。第一圈只有 [`LocalDirSource`]（开发/测试：
//! 本地目录镜像，文件名与 catalog 一一对应）；真云端来了加一个网络实现，
//! **管道一行不改**。在装好的产品里既没有仓库也没有云端，所以下载命令会诚实地说
//! 还没接——不假装成功。
//!
//! # 不记账本
//!
//! "已经下载了哪些 / 哪些过时了"不单独记账：[`FileOnDisk`] 拿 catalog 当期望值、
//! **盘当底账**（在、且 SHA 对得上 = Current；在但 SHA 不一样 = Stale，就是"有更新"）。
//! 任何账本都会和盘漂移——旧世界"缓存的是定义、不是文件在不在"是同一课。
//!
//! # 校验在落盘之前
//!
//! SHA 或大小对不上就整个拒绝，`mkp/` 里不会出现半份坏文件——写盘走
//! [`crate::fsx::atomic`]，坏字节连碰盘的机会都没有。
//!
//! # 更新与归档
//!
//! 同一份文件的字节变了（catalog 更新带来新版本），再走一遍 [`deliver`] 就是更新：
//! 旧份先复制进 `archive/`（保留**最早**一份——归档槽不覆盖，要完整历史等需要的那天），
//! 然后原子换新。删除永远不是更新的一部分。

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::AppError;
use crate::fsx::atomic::atomic_write;
use crate::fsx::paths::resolve_in;

use super::catalog::hex;
use super::catalog::CatalogFile;
use super::paths::archive_dir;

/// 下载源：catalog 里登记的那份文件从哪里拿。
///
/// 第一圈的 catalog 没有 URL 字段（云端地址是第二圈的事），所以源实现自己知道
/// 怎么按 `file_name` 找到字节。等 manifest/URL 进 catalog，这个 trait 的入参
/// 再扩——实现换、管道不换。
pub trait Source {
    fn fetch(&self, file: &CatalogFile) -> Result<Vec<u8>, AppError>;
}

/// 开发/测试源：本地目录镜像。目录里放与 catalog 同名的文件，比如开发机上就是
/// `crates/preset/assets/presets/`。
pub struct LocalDirSource<'a> {
    root: &'a Path,
}

impl<'a> LocalDirSource<'a> {
    pub fn new(root: &'a Path) -> Self {
        Self { root }
    }
}

impl Source for LocalDirSource<'_> {
    fn fetch(&self, file: &CatalogFile) -> Result<Vec<u8>, AppError> {
        std::fs::read(self.root.join(&file.file_name)).map_err(|e| {
            AppError::not_found(format!("下载源里没有 {}", file.file_name))
                .with_detail(e.to_string())
        })
    }
}

/// 盘上那份文件与目录的关系。**盘当底账**，这三分态就是全部真相：
/// - `Absent` 没下载过（合法状态，不是错误）；
/// - `Current` 在，且 SHA 与目录一致；
/// - `Stale` 在，但字节与目录不一样——目录更新带来新版本，或文件被手动动过。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileOnDisk {
    Absent,
    Current,
    Stale,
}

pub fn file_status(internal_root: &Path, file: &CatalogFile) -> FileOnDisk {
    match std::fs::read(internal_root.join(&file.path)) {
        Ok(bytes) if hex(&Sha256::digest(&bytes)) == file.sha256 => FileOnDisk::Current,
        Ok(_) => FileOnDisk::Stale,
        Err(_) => FileOnDisk::Absent,
    }
}

/// 一份文件走完整条管道：取回 → SHA 校验 → 大小校验 → 防穿越 → 归档旧份 → 原子落盘。
///
/// 落点是 catalog 说的算（`file.path`，相对内部根，`mkp/…`），不是调用方拼的——
/// "下载到哪里"是目录的职责。返回落盘的绝对路径。重复下载相同内容是幂等的
/// （字节一样就不折腾盘）；内容变了就是一次更新，旧份先归档。
pub fn deliver(
    internal_root: &Path,
    file: &CatalogFile,
    source: &dyn Source,
) -> Result<PathBuf, AppError> {
    let bytes = source.fetch(file)?;

    // 校验在落盘之前：期望值来自 catalog（发布时对真字节算的），字节不对就不碰盘
    let got = hex(&Sha256::digest(&bytes));
    if got != file.sha256 {
        return Err(AppError::sha_mismatch(format!(
            "{} 的内容对不上（期望 SHA {}，拿到 {}）—— 文件在源头就被改过或传坏了",
            file.file_name, file.sha256, got
        )));
    }
    if bytes.len() as u64 != file.size {
        return Err(AppError::corrupted(format!(
            "{} 的大小对不上（目录记 {} 字节，拿到 {} 字节）",
            file.file_name,
            file.size,
            bytes.len()
        )));
    }

    // 防穿越：catalog 是我们自己写的，但路径判据不信任任何来源（总纲 §1③）
    let target = resolve_in(internal_root, &file.path)?;

    // 更新与归档：旧份与目录不一样才叫"换版本"，复制进 archive/（保留最早一份），
    // 然后原子换新。第一次下载没有旧份，直接落。
    let old = std::fs::read(&target).ok();
    let is_update = matches!(&old, Some(b) if hex(&Sha256::digest(b)) != file.sha256);
    if is_update {
        let archive_target = archive_dir(internal_root).join(&file.path);
        std::fs::create_dir_all(archive_target.parent().expect("归档路径必有父目录")).map_err(
            |e| {
                AppError::io(format!("建不出归档目录：{}", archive_target.display()))
                    .with_detail(e.to_string())
            },
        )?;
        if !archive_target.exists() {
            atomic_write(
                &archive_target,
                old.as_deref().expect("is_update 蕴含旧份存在"),
            )?;
        }
    }
    if is_update || old.is_none() {
        atomic_write(&target, &bytes)?;
    }
    Ok(target)
}

/// 已经下载到本地的文件名（catalog 登记的里面，盘上是 Current 的）。
/// 不查缓存、不记账本——每次都问盘，下载完立刻看得见。
pub fn downloaded_files(internal_root: &Path, catalog: &super::Catalog) -> Vec<String> {
    files_in_status(internal_root, catalog, FileOnDisk::Current)
}

/// 有更新的文件名（盘上在，但字节与目录不一样）。
/// "更新"动作就是对这些文件再跑一遍 [`deliver`]——旧份自动归档，没有单独的更新代码路径。
pub fn stale_files(internal_root: &Path, catalog: &super::Catalog) -> Vec<String> {
    files_in_status(internal_root, catalog, FileOnDisk::Stale)
}

fn files_in_status(
    internal_root: &Path,
    catalog: &super::Catalog,
    want: FileOnDisk,
) -> Vec<String> {
    catalog
        .files
        .iter()
        .filter(|f| file_status(internal_root, f) == want)
        .map(|f| f.file_name.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::catalog::Catalog;

    /// 造一份内容为 `content` 的目录条目（SHA/大小都对得上）
    fn entry(name: &str, content: &[u8]) -> CatalogFile {
        CatalogFile {
            kind: "mkp_preset".to_owned(),
            file_name: name.to_owned(),
            path: format!("mkp/{name}"),
            machine_id: "A1".to_owned(),
            version_id: "STANDARD".to_owned(),
            sha256: hex(&Sha256::digest(content)),
            size: content.len() as u64,
        }
    }

    /// 内存源：直接吐预置的字节，测试不碰第二个临时目录
    struct MemorySource(Vec<u8>);
    impl Source for MemorySource {
        fn fetch(&self, _file: &CatalogFile) -> Result<Vec<u8>, AppError> {
            Ok(self.0.clone())
        }
    }

    fn catalog_with(files: Vec<CatalogFile>) -> Catalog {
        Catalog {
            catalog_schema: super::super::catalog::CATALOG_SCHEMA,
            revision: "test".to_owned(),
            machines: Vec::new(),
            files,
        }
    }

    #[test]
    fn delivers_verified_bytes_into_mkps() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(mkp_root(&root)).unwrap();
        let content = "# A1 standard\nspeed_limit = 60\n".as_bytes();
        let file = entry("A1-standard.toml", content);

        let target = deliver(root.path(), &file, &MemorySource(content.to_vec())).unwrap();

        assert!(target.starts_with(mkp_root(&root)), "落在下载区里");
        assert_eq!(std::fs::read(&target).unwrap(), content, "字节逐份一致");

        // 盘就是底账：下载完立刻查得到
        let catalog = catalog_with(vec![file]);
        assert_eq!(
            downloaded_files(root.path(), &catalog),
            vec!["A1-standard.toml".to_owned()]
        );
    }

    #[test]
    fn rejects_sha_mismatch_without_touching_disk() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(mkp_root(&root)).unwrap();
        let file = entry("A1-standard.toml", "真内容".as_bytes());

        let e = deliver(
            root.path(),
            &file,
            &MemorySource("假内容".as_bytes().to_vec()),
        )
        .unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::ShaMismatch);
        assert!(
            !root.path().join("mkp/A1-standard.toml").exists(),
            "校验不过就整个拒绝，坏文件不落盘"
        );
    }

    #[test]
    fn rejects_size_mismatch() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(mkp_root(&root)).unwrap();
        let mut file = entry("A1-standard.toml", "内容".as_bytes());
        file.size = 999; // 目录里记错了大小

        let e = deliver(
            root.path(),
            &file,
            &MemorySource("内容".as_bytes().to_vec()),
        )
        .unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::Corrupted);
    }

    /// catalog 的 path 字段不 trusted：`..` 一律过不了防穿越那道闸
    #[test]
    fn rejects_traversal_in_catalog_path() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(mkp_root(&root)).unwrap();
        let content = "x".as_bytes();
        let mut file = entry("A1-standard.toml", content);
        file.path = "mkp/../../escape.toml".to_owned();

        let e = deliver(root.path(), &file, &MemorySource(content.to_vec())).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::PermissionDenied);
        assert!(!root.path().join("escape.toml").exists());
    }

    #[test]
    fn missing_source_file_is_not_found() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(mkp_root(&root)).unwrap();
        let file = entry("A1-standard.toml", "内容".as_bytes());

        let e = deliver(root.path(), &file, &LocalDirSource::new(root.path())).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::NotFound);
    }

    /* ---------- 更新与归档（第二圈第一刀） ---------- */

    /// 十问 #9 的正面表达：换版本时旧份进 archive/，不删；盘换新
    #[test]
    fn update_archives_the_old_file_then_replaces() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(mkp_root(&root)).unwrap();
        let v1 = entry("A1-standard.toml", "版本一".as_bytes());
        let v2 = entry("A1-standard.toml", "版本二".as_bytes());

        deliver(
            root.path(),
            &v1,
            &MemorySource("版本一".as_bytes().to_vec()),
        )
        .unwrap();
        deliver(
            root.path(),
            &v2,
            &MemorySource("版本二".as_bytes().to_vec()),
        )
        .unwrap();

        // 盘上是新的，旧份在 archive/ 里原样躺着
        assert_eq!(
            std::fs::read(root.path().join("mkp/A1-standard.toml")).unwrap(),
            "版本二".as_bytes()
        );
        assert_eq!(
            std::fs::read(root.path().join("archive/mkp/A1-standard.toml")).unwrap(),
            "版本一".as_bytes()
        );

        // 三分态：更新完是 Current，Stale 清空
        let catalog = catalog_with(vec![v2]);
        assert_eq!(
            file_status(root.path(), &catalog.files[0]),
            FileOnDisk::Current
        );
        assert!(stale_files(root.path(), &catalog).is_empty());
    }

    /// 归档槽保留**最早**那份：连升两版，archive/ 里还是第一版——历史不被后浪抹掉
    #[test]
    fn archive_slot_keeps_the_earliest_version() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(mkp_root(&root)).unwrap();
        let v1 = entry("A1-standard.toml", "版本一".as_bytes());
        let v2 = entry("A1-standard.toml", "版本二".as_bytes());
        let v3 = entry("A1-standard.toml", "版本三".as_bytes());

        deliver(
            root.path(),
            &v1,
            &MemorySource("版本一".as_bytes().to_vec()),
        )
        .unwrap();
        deliver(
            root.path(),
            &v2,
            &MemorySource("版本二".as_bytes().to_vec()),
        )
        .unwrap();
        deliver(
            root.path(),
            &v3,
            &MemorySource("版本三".as_bytes().to_vec()),
        )
        .unwrap();

        assert_eq!(
            std::fs::read(root.path().join("archive/mkp/A1-standard.toml")).unwrap(),
            "版本一".as_bytes(),
            "归档槽不覆盖：要完整历史是以后的事，先保证最早的丢不了"
        );
    }

    /// 幂等：同内容重复下载不折腾盘（归档目录都不该出现）
    #[test]
    fn redownloading_same_bytes_is_a_no_op() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(mkp_root(&root)).unwrap();
        let file = entry("A1-standard.toml", "内容".as_bytes());

        deliver(
            root.path(),
            &file,
            &MemorySource("内容".as_bytes().to_vec()),
        )
        .unwrap();
        deliver(
            root.path(),
            &file,
            &MemorySource("内容".as_bytes().to_vec()),
        )
        .unwrap();

        assert!(
            !root.path().join("archive").exists(),
            "没有版本变化就没有归档"
        );
    }

    /// Stale 的判定：盘上的字节和目录不一样 = "有更新"，Current 清零
    #[test]
    fn stale_is_visible_when_disk_bytes_drift() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(mkp_root(&root)).unwrap();
        let file = entry("A1-standard.toml", "当时的内容".as_bytes());
        let catalog = catalog_with(vec![file.clone()]);

        deliver(
            root.path(),
            &file,
            &MemorySource("当时的内容".as_bytes().to_vec()),
        )
        .unwrap();
        assert_eq!(
            downloaded_files(root.path(), &catalog),
            vec!["A1-standard.toml".to_owned()]
        );

        // 目录换了新版本（或文件被手动动过）：同一份文件从 Current 掉进 Stale
        let new_version = entry("A1-standard.toml", "新版本".as_bytes());
        let new_catalog = catalog_with(vec![new_version]);
        assert!(downloaded_files(root.path(), &new_catalog).is_empty());
        assert_eq!(
            stale_files(root.path(), &new_catalog),
            vec!["A1-standard.toml".to_owned()]
        );
    }

    // ---- 测试小工具 ----

    fn mkp_root(dir: &tempfile::TempDir) -> PathBuf {
        super::super::paths::mkp_dir(dir.path())
    }
}
