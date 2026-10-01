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

/* ---------- 归档区（旧版本留档） ---------- */

/// 归档区里的一份旧版本。**盘就是底账**（与下载区同一套规矩）：扫盘得到，不记账本。
///
/// 归档是**官方版本生命周期**的一部分（换版本时旧份进 `archive/`，不删），
/// **不是用户修改历史** —— 用户改出来的东西是另一条线（另存成另一份文件），
/// 永远不回写官方原件。所以这里只有"官方旧版本"，没有"谁在什么时候改了什么"。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedFile {
    /// 相对内部根的路径（`archive/mkp/presets/A1-fast.toml`）—— 世界里唯一的键
    pub path: String,
    /// 文件名。与它对应的交付文件**同名**：换版本换的是字节，不是名字
    pub file_name: String,
    pub size: u64,
    /// 这份旧版本**被换下来的时刻**（UTC epoch 秒）。归档没有单独的"归档时刻"这一回事——
    /// 写这份文件的时刻就是它。`None` = 文件系统没给（不是 0，也不编一个）
    pub modified_unix: Option<u64>,
}

/// 归档区里现在有什么（按路径升序 —— `read_dir` 的顺序是文件系统说的，不稳定，
/// 界面要一个每次刷新都一样的表）。
///
/// 归档区不存在 = 还没归档过东西（合法状态，不是错误）：返回空表。
/// **只列，不动盘**：这一层不提供删除、不提供恢复。
pub fn archived_files(internal_root: &Path) -> Vec<ArchivedFile> {
    let mut out = Vec::new();
    let mut stack = vec![archive_dir(internal_root)];
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
            let rel = path
                .strip_prefix(internal_root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            out.push(ArchivedFile {
                path: rel,
                file_name: path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
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

/* ---------- 一次多份（第二圈：并发） ---------- */

/// 多份同时下载时走几条道。**固定上限而不是每份一条**：下载源是别人的服务器，
/// 一次开几十个连接既没快多少，又很容易被当成滥用拦掉
pub const CONCURRENCY: usize = 4;

/// 多份下载里某一份的结局。**按份给结局**——一份失败不许拖累其它份，
/// 也不许被抹成一句"下载失败"（界面要说出是哪份、因为什么）
#[derive(Debug, Clone)]
pub struct FileOutcome {
    pub file_name: String,
    pub ok: bool,
    /// 失败原因。**成功了就是空的**——不塞一个"成功了"进去充数
    pub message: String,
}

/// 一次取多份：**并发在这一层做，不丢给前端**。
///
/// 前端自己并发的话，"失败了几份、失败在哪儿"要由它逐条 patch 出来，那份汇总
/// 既不完整也不一致（它的并发版本和 socket 版本还会打架）。这里发的每一份都走
/// **同一个 [`deliver`]**，幂等且落点互不相同，所以并行是安全的。
///
/// `on_each` 在每一份有结局时立刻回调（顺序是完成顺序）；返回值则是**请求顺序**
/// ——界面要一个稳定的行序，不要一个随网络抖动的顺序。
pub fn deliver_all(
    internal_root: &Path,
    files: &[CatalogFile],
    source: &(dyn Source + Sync),
    on_each: &(dyn Fn(&FileOutcome) + Send + Sync),
) -> Vec<FileOutcome> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc::channel;

    if files.is_empty() {
        return Vec::new();
    }
    let lanes = files.len().clamp(1, CONCURRENCY);
    let next = AtomicUsize::new(0);
    let (tx, rx) = channel::<(usize, FileOutcome)>();

    std::thread::scope(|scope| {
        for _lane in 0..lanes {
            let tx = tx.clone();
            let next = &next;
            scope.spawn(move || loop {
                let index = next.fetch_add(1, Ordering::SeqCst);
                let Some(file) = files.get(index) else {
                    break;
                };
                let outcome = match deliver(internal_root, file, source) {
                    Ok(target) => FileOutcome {
                        file_name: target
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| file.file_name.clone()),
                        ok: true,
                        message: String::new(),
                    },
                    Err(e) => FileOutcome {
                        file_name: file.file_name.clone(),
                        ok: false,
                        message: e.message,
                    },
                };
                on_each(&outcome);
                if tx.send((index, outcome)).is_err() {
                    break;
                }
            });
        }
    });
    drop(tx);

    let mut done: Vec<(usize, FileOutcome)> = rx.into_iter().collect();
    done.sort_by_key(|(index, _)| *index);
    done.into_iter().map(|(_, outcome)| outcome).collect()
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
            files,
            ..Catalog::default()
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

    /* ---------- 归档区（旧版本留档的可见性） ---------- */

    /// 归档区不存在 = 还没归档过东西：空表，不是错误（新装的机器就是这个状态）
    #[test]
    fn archived_files_is_empty_before_anything_was_archived() {
        let root = tempfile::tempdir().unwrap();
        assert!(archived_files(root.path()).is_empty());
    }

    /// **盘当底账**：换版本之后它列得出那一份旧版本（路径 / 文件名 / 大小 / 时刻），
    /// 而且按它给的路径能读回**原字节**（列出来的东西与归档里那份是同一份）
    #[test]
    fn archived_files_lists_what_the_update_pushed_aside() {
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
        assert!(
            archived_files(root.path()).is_empty(),
            "第一次下载没有旧版本 —— 归档只在换版本时产生"
        );

        deliver(
            root.path(),
            &v2,
            &MemorySource("版本二".as_bytes().to_vec()),
        )
        .unwrap();

        let got = archived_files(root.path());
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].path, "archive/mkp/A1-standard.toml");
        assert_eq!(got[0].file_name, "A1-standard.toml", "与交付文件同名");
        assert_eq!(
            got[0].size,
            "版本一".len() as u64,
            "大小是**旧份**的字节数，不是新的"
        );
        assert!(got[0].modified_unix.is_some(), "写这份文件的时刻要带上");
        assert_eq!(
            std::fs::read(root.path().join(&got[0].path)).unwrap(),
            "版本一".as_bytes(),
            "按这条读给的路径读回来的是旧版本的原字节"
        );
    }

    /// 归档槽**保留最早一份**（不覆盖）—— 所以连升两版之后，列出来仍然只有一份、
    /// 而且是最早那版。这条同时把"归档 ≠ 每次更新的历史"钉在判据里
    #[test]
    fn archived_files_reflects_the_single_archive_slot() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(mkp_root(&root)).unwrap();
        for content in ["版本一", "版本二", "版本三"] {
            let file = entry("A1-standard.toml", content.as_bytes());
            deliver(
                root.path(),
                &file,
                &MemorySource(content.as_bytes().to_vec()),
            )
            .unwrap();
        }

        let got = archived_files(root.path());
        assert_eq!(
            got.len(),
            1,
            "归档槽只有一个（最早那份），不是每次更新的历史"
        );
        assert_eq!(
            std::fs::read(root.path().join(&got[0].path)).unwrap(),
            "版本一".as_bytes()
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

    /* ---------- 批量（第二圈） ---------- */

    /// 按 [`content_of`] 那条约定吐字节的源：批量用例里"全都成功"的那一档
    struct HonestSource;
    impl Source for HonestSource {
        fn fetch(&self, file: &CatalogFile) -> Result<Vec<u8>, AppError> {
            Ok(content_of(file))
        }
    }

    /// 测试里的唯一约定：一份文件的内容就是"<文件名> 的内容"。
    /// 条目按它造 SHA，下面的源也按它吐字节——**两边同一条约定**，不要各写一套
    fn content_of(file: &CatalogFile) -> Vec<u8> {
        format!("{} 的内容", file.file_name).as_bytes().to_vec()
    }

    fn batch(names: &[&str]) -> Vec<CatalogFile> {
        let files: Vec<CatalogFile> = names.iter().map(|n| entry(n, &[])).collect();
        // size/sha 由内容算出来，所以先造壳再回填
        files
            .into_iter()
            .map(|mut file| {
                let bytes = content_of(&file);
                file.sha256 = hex(&Sha256::digest(&bytes));
                file.size = bytes.len() as u64;
                file
            })
            .collect()
    }

    /// 挑一份给错的字节（**长度还一模一样**）——用来造"批量里某一份坏档"
    struct PickySource<'a> {
        bad_for: &'a str,
    }
    impl Source for PickySource<'_> {
        fn fetch(&self, file: &CatalogFile) -> Result<Vec<u8>, AppError> {
            if file.file_name != self.bad_for {
                return Ok(content_of(file));
            }
            // 长度对齐，只有内容不对：真正要靠 SHA 那道闸拦住的那种坏档
            Ok(content_of(file).iter().map(|_| b'x').collect())
        }
    }

    /// 让某一特别慢：用来证明"返回结果按请求顺序，而不是按快慢"
    struct SlowSource {
        slow_ms: u64,
    }
    impl Source for SlowSource {
        fn fetch(&self, file: &CatalogFile) -> Result<Vec<u8>, AppError> {
            if !file.file_name.ends_with("0.toml") {
                return Ok(content_of(file));
            }
            std::thread::sleep(std::time::Duration::from_millis(self.slow_ms));
            Ok(content_of(file))
        }
    }

    fn setup_batch(names: &[&str]) -> (tempfile::TempDir, Vec<CatalogFile>) {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(mkp_root(&root)).unwrap();
        (root, batch(names))
    }

    /// 五份一起下：**每份都在盘上**（途经同一个 `deliver`，幂等与校验照旧）
    #[test]
    fn batch_delivers_every_file() {
        let (root, files) = setup_batch(&["0.toml", "1.toml", "2.toml", "3.toml", "4.toml"]);

        let order = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let record = {
            let order = order.clone();
            move |o: &FileOutcome| order.lock().expect("锁坏了").push(o.file_name.clone())
        };

        let outcomes = deliver_all(root.path(), &files, &HonestSource, &record);

        assert_eq!(outcomes.len(), 5);
        assert!(outcomes.iter().all(|o| o.ok), "全都该成功：{outcomes:?}");
        for file in &files {
            assert_eq!(
                std::fs::read(root.path().join(&file.path)).unwrap(),
                content_of(file),
                "{} 落到盘上",
                file.file_name
            );
        }
        assert!(
            outcomes.iter().all(|o| o.message.is_empty()),
            "成功不带评语"
        );
        assert_eq!(order.lock().expect("锁坏了").len(), 5, "每一份都回调了一次");
    }

    /// 返回的是**请求顺序**：界面那张表的行序不该跟着网络抖动变
    #[test]
    fn batch_result_keeps_the_requested_order() {
        let (root, files) = setup_batch(&["0.toml", "1.toml", "2.toml", "3.toml"]);

        let outcomes = deliver_all(root.path(), &files, &SlowSource { slow_ms: 150 }, &|_| {});

        let got: Vec<&str> = outcomes.iter().map(|o| o.file_name.as_str()).collect();
        assert_eq!(got, vec!["0.toml", "1.toml", "2.toml", "3.toml"]);
    }

    /// 一份坏档不拖累别人：**其它三份照常落盘**，坏的那份单独带着原因回来
    #[test]
    fn batch_isolates_a_bad_file_from_the_others() {
        let (root, files) = setup_batch(&["0.toml", "1.toml", "2.toml", "3.toml"]);

        let outcomes = deliver_all(
            root.path(),
            &files,
            &PickySource { bad_for: "2.toml" },
            &|_| {},
        );

        let bad = outcomes.iter().find(|o| !o.ok).expect("该有一份失败");
        assert_eq!(bad.file_name, "2.toml");
        assert!(!bad.message.is_empty(), "失败要说原因");
        assert_eq!(
            outcomes.iter().filter(|o| o.ok).count(),
            3,
            "其余三份不受影响"
        );
        assert!(
            !root.path().join("mkp/2.toml").exists(),
            "坏字节不落盘这条对批量同样成立"
        );
        assert!(root.path().join("mkp/0.toml").exists());
    }

    /// 什么都没要 = 什么都不做，也不炸
    #[test]
    fn empty_batch_is_a_no_op() {
        let (root, files) = setup_batch(&[]);
        let outcomes = deliver_all(root.path(), &files, &HonestSource, &|_| {});
        assert!(outcomes.is_empty());
    }

    // ---- 测试小工具 ----

    fn mkp_root(dir: &tempfile::TempDir) -> PathBuf {
        super::super::paths::mkp_dir(dir.path())
    }
}
