//! 公共下载管道（第一圈骨架，总纲第④层的客户端那一半）。
//!
//! 总纲定的规矩：**不管送的是 TOML、JSON、模型还是图片，物流只认七个问题**——
//! 它是谁 → 从哪里来 → SHA 是多少 → 下载到哪里 → 是否已经有了 → 更新怎么办 → 旧的去哪。
//! 第一圈先把"从哪里来 → SHA → 下载到哪里 → 是否已有"这条最短管道立起来，
//! 更新 / 归档在第二圈接上。以后 Preset、模型、BBS 配置的下载**都走这一条路**，
//! 业务不许自己再发明一套（那是"半年后又变成现在这个样子"的防波堤）。
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
//! "已经下载了哪些"不单独记账：[`downloaded_files`] 拿 catalog 当期望值、**盘当底账**
//! （文件在且 SHA 对得上才算数）。任何账本都会和盘漂移——旧世界"缓存的是定义、
//! 不是文件在不在"是同一课。
//!
//! # 校验在落盘之前
//!
//! SHA 或大小对不上就整个拒绝，`mkp/` 里不会出现半份坏文件——写盘走
//! [`crate::fsx::atomic`]，坏字节连碰盘的机会都没有。

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::AppError;
use crate::fsx::atomic::atomic_write;
use crate::fsx::paths::resolve_in;

use super::catalog::hex;
use super::catalog::CatalogFile;

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

/// 一份文件走完整条管道：取回 → SHA 校验 → 大小校验 → 防穿越 → 原子落盘。
///
/// 落点是 catalog 说的算（`file.path`，相对内部根，`mkp/…`），不是调用方拼的——
/// "下载到哪里"是目录的职责。返回落盘的绝对路径。
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
    atomic_write(&target, &bytes)?;
    Ok(target)
}

/// 已经下载到本地的文件名（catalog 登记的里面，盘上真的有且 SHA 对得上的）。
/// 不查缓存、不记账本——每次都问盘，下载完立刻看得见。
pub fn downloaded_files(internal_root: &Path, catalog: &super::Catalog) -> Vec<String> {
    catalog
        .files
        .iter()
        .filter(|f| {
            std::fs::read(internal_root.join(&f.path))
                .map(|bytes| hex(&Sha256::digest(&bytes)) == f.sha256)
                .unwrap_or(false)
        })
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

    #[test]
    fn delivers_verified_bytes_into_mkps() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(mkp_root(&root)).unwrap();
        let content = "# A1 标准版\nspeed_limit = 60\n".as_bytes();
        let file = entry("A1-standard.toml", content);

        let target = deliver(root.path(), &file, &MemorySource(content.to_vec())).unwrap();

        assert!(target.starts_with(mkp_root(&root)), "落在下载区里");
        assert_eq!(std::fs::read(&target).unwrap(), content, "字节逐份一致");

        // 盘就是底账：下载完立刻查得到
        let catalog = empty_catalog();
        let mut catalog = catalog;
        catalog.files.push(file);
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
        let content = b"x";
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

    // ---- 测试小工具 ----

    fn mkp_root(dir: &tempfile::TempDir) -> PathBuf {
        super::super::paths::mkp_dir(dir.path())
    }

    fn empty_catalog() -> Catalog {
        Catalog {
            catalog_schema: super::super::catalog::CATALOG_SCHEMA,
            revision: "test".to_owned(),
            machines: Vec::new(),
            files: Vec::new(),
        }
    }
}
