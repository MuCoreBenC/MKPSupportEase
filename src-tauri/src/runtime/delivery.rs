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
//!
//! # 这一份我们认得出吗（第三圈第 6 层）
//!
//! 盘上有字节 ≠ 那字节是官方内容。`stale` 只说"与目录不一致"，而那可能是两种完全不同的事：
//!
//! ```text
//! 盘上这一份 vs 目录登记的当前版本
//!   ├─ 逐字节相同      → [`FileTrust::Current`]   就是当前这一版
//!   ├─ 与 [`archive/`] 里某一版相同 → [`FileTrust::OldVersion`]  认得出它是哪一版（旧版）
//!   └─ 哪儿都对不上    → [`FileTrust::Unknown`]   这台机器上查不出它属于哪一版
//! ```
//!
//! **`stale` 与"云端有没有更新"是两件事**：前者问"本机这份是不是我们认可的官方内容"，
//! 后者是 [`super::update::check`] 的事（比目录指纹）。混成一句"需更新"，用户既不知道
//! 自己的文件是不是被改过，也不知道该不该等更新 —— 所以这一段单独给 [`trust_entries`]。

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::AppError;
use crate::fsx::atomic::atomic_write;
use crate::fsx::paths::resolve_in;

use super::catalog::hex;
use super::catalog::CatalogFile;
use super::paths::{archive_dir, CATALOG_FILE};
use super::preset_events::{self, PresetEvent};

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
///
/// **目录没给期望值时**（随包 bootstrap 目录，`sha256 = None`）盘上有的都按 `Current` 算：
/// 这一侧的目录没资格为它的字节背书，说 `Stale` 就是凭空造一个"与目录不符"
/// （用户会去查一个根本不存在的差异）。真正的判定等 OTA 目录生效后那次下载。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileOnDisk {
    Absent,
    Current,
    Stale,
}

pub fn file_status(internal_root: &Path, file: &CatalogFile) -> FileOnDisk {
    match std::fs::read(internal_root.join(&file.path)) {
        Ok(bytes) => status_of(file, &bytes),
        Err(_) => FileOnDisk::Absent,
    }
}

/// [`file_status`] 的字节已在手的版本（`entries_in_status` 一份字节只用读一次）
fn status_of(file: &CatalogFile, bytes: &[u8]) -> FileOnDisk {
    match file.expected_sha() {
        Some(want) if hex(&Sha256::digest(bytes)) == want => FileOnDisk::Current,
        Some(_) => FileOnDisk::Stale,
        None => FileOnDisk::Current,
    }
}

/// SHA 对不上时的那条错。**两种成因要给两种话**（2026-10-04）。
///
/// 本机目录的 revision 与远端不同 → 多半是目录过期（OTA 没跑 / 网络不通），该说
/// "目录可能过期，检查更新"；相同却字节不符 → 才是源头坏件。判定用**远端目录的
/// revision**；取不到远端（离线）就退回保守说法 —— 不拿本机目录当"远端"，那会把
/// "没联网"误报成"文件坏了"。判定只在这一处做，界面只负责展示。
fn sha_mismatch_error(internal_root: &Path, file: &CatalogFile, got: &str) -> AppError {
    let stale = remote_revision(internal_root)
        .zip(local_revision(internal_root))
        .filter(|(remote, local)| remote != local);
    AppError::sha_mismatch(sha_mismatch_message(file, got, stale))
}

/// 那句话本身 —— 与"怎么拿到两个 revision"分开，于是它能被单测逐字钉住
/// （取 revision 要联网，这台机器上测不了；说话不该跟着一起测不了）。
fn sha_mismatch_message(file: &CatalogFile, got: &str, stale: Option<(String, String)>) -> String {
    let why = match stale {
        Some((remote, local)) => format!(
            "本地目录停在 revision {local}，云端已经是 {remote} —— \
             先点「检查更新」刷新目录再下载（目录过期时拿旧 SHA 比对，必然对不上）"
        ),
        // 取不到远端（没配源 / 离线）时照保守说：**不拿本机目录当"远端"**，
        // 那会把"没联网"误报成"文件坏了"
        None => "文件在源头就被改过或传坏了".to_owned(),
    };
    format!(
        "{} 的内容对不上（期望 SHA {}，拿到 {}）—— {why}",
        file.file_name,
        file.expected_sha().unwrap_or("<目录未登记>"),
        got
    )
}

/// 本机 registry 里那份 catalog 的 revision（读不到就 `None`）
fn local_revision(internal_root: &Path) -> Option<String> {
    let bytes = std::fs::read(super::paths::catalog_file(internal_root)).ok()?;
    super::Catalog::parse(&bytes).ok().map(|c| c.revision)
}

/// 数据源那一侧 `catalog.json` 的 revision。**取不到一律 `None`**（没配源 / 离线）——
/// 判定失败就退回保守说法，绝不让"没联网"看起来像"文件坏了"
fn remote_revision(internal_root: &Path) -> Option<String> {
    let resolved = super::source::resolve_source(internal_root).ok()?;
    let bytes = super::net::get_catalog(&resolved.catalog_url().ok()?).ok()?;
    super::Catalog::parse(&bytes).ok().map(|c| c.revision)
}

/// 一份文件走完整条管道：取回 → SHA 校验 → 大小校验 → 防穿越 → 归档旧份 → 原子落盘。
///
/// # 落点 = `file.path`（唯一路径语义，2026-10-04）
///
/// `file.path` 相对**内部根**，就是 [`CatalogFile::path`]。它同时是"云端取哪"
/// （`baseUrl + path`）与"本地放哪"（`<appDataDir>/<path>`）—— **没有第二套映射**。
/// 所以这里 `resolve_in(internal_root, &file.path)` 得到的就是最终落点，
/// 函数自己**不拼任何目录名**。
///
/// 归档同理：`archive/<file.path>`（结构与交付面同形）。
///
/// 校验只在**目录给了期望值**时做：随包 bootstrap 目录不登记期望值
/// （见 [`CatalogFile`]），那就照收 —— 拿一个空 SHA"比一比"看着像校验、其实什么都没验，
/// 还会把好文件判成坏的。真正的校验等 OTA 目录生效后那次下载。
pub fn deliver(
    internal_root: &Path,
    file: &CatalogFile,
    source: &dyn Source,
) -> Result<PathBuf, AppError> {
    let bytes = source.fetch(file)?;

    // 校验在落盘之前：期望值来自 catalog（发布时对真字节算的），字节不对就不碰盘
    if let Some(want) = file.expected_sha() {
        let got = hex(&Sha256::digest(&bytes));
        if got != want {
            return Err(sha_mismatch_error(internal_root, file, &got));
        }
    }
    if let Some(want) = file.expected_size() {
        if bytes.len() as u64 != want {
            return Err(AppError::corrupted(format!(
                "{} 的大小对不上（目录记 {} 字节，拿到 {} 字节）",
                file.file_name,
                want,
                bytes.len()
            )));
        }
    }

    // 防穿越：catalog 是我们自己写的，但路径判据不信任任何来源（总纲 §1③）
    let target = resolve_in(internal_root, &file.path)?;

    // 更新与归档：旧份与目录不一样才叫"换版本"，复制进 archive/（保留最早一份），
    // 然后原子换新。第一次下载没有旧份，直接落。
    let old = std::fs::read(&target).ok();
    let is_update = match file.expected_sha() {
        Some(want) => matches!(&old, Some(b) if hex(&Sha256::digest(b)) != want),
        // 没有期望值：旧份字节与"带来这份"是否一样由内容自己说了算
        None => matches!(&old, Some(b) if b.as_slice() != bytes.as_slice()),
    };
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

    /* ★ 事件定格（`preset_events` 硬规则②）：写盘成功的那一刻记事件，此后任何扫描、
     * 任何云端换代都不改它。界面上「下载时间 / 替换时间」从这里来 —— mtime 不再上界面。
     * 记不上不影响这次下载本身（附加信息，与 provenance 同一条口径）；
     * 同字节重放（旧份在、字节一致）：什么都没发生 —— 不记事件。 */
    let new_sha = sha_hex(&bytes);

    /* ★ **隐藏 baseline**（2026-10-08 作者改判）：每一版官方预设都留一份不可见的
     * 基准快照（[`super::baseline`]）。按内容摘要寻址 ⇒ 幂等、每版一份、不覆盖；
     * 它是参数页「恢复默认」的取值依据，也是用户那份血统里 `based_on_sha256`
     * 指向的东西。**与归档（`archive/`）是两件事**：归档服务交付四态判定，
     * 这一份服务恢复默认 —— 两条线各管各的。
     * 落不上**不影响这次下载**（附加信息，与事件账、出处账同一条口径）：
     * 缺了只表现为"恢复默认"回退到出厂值。 */
    if file.kind == super::catalog::kind::PRESET {
        if let Err(e) = super::baseline::ensure_baseline(internal_root, &new_sha, &bytes) {
            tracing::warn!("baseline 没落上（{}）：{}", new_sha, e.message);
        }
    }

    match old.as_deref() {
        // 第一次落盘：这是「下载」
        None => preset_events::append(
            internal_root,
            PresetEvent::DeliveryDownloaded {
                file: file.path.clone(),
                sha256: new_sha,
                revision: local_revision(internal_root),
                at: preset_events::now(),
            },
        ),
        // 旧份被换掉：这是「替换」。旧份那版属于哪代目录，趁它还在盘上查一把 ——
        // 查不出（链建立之前的版本 / 来路不明）就 null，不猜（硬规则③）
        Some(old_bytes) if is_update => {
            let old_sha = sha_hex(old_bytes);
            preset_events::append(
                internal_root,
                PresetEvent::DeliveryReplaced {
                    file: file.path.clone(),
                    old_revision: generation_of_sha(internal_root, &old_sha).map(|g| g.revision),
                    old_sha256: old_sha,
                    new_sha256: Some(new_sha),
                    new_revision: local_revision(internal_root),
                    at: preset_events::now(),
                },
            )
        }
        Some(_) => {}
    }
    Ok(target)
}

/* ---------- 版本身份（字节指纹 → 哪一代目录、那一代何时发布） ---------- */

/// 一份字节的**版本出身**：它登记在哪一代目录里、那一代什么时候发布的。
///
/// `published_at` 是 `ReleasePublished` 事件的读法（不落账，反查目录与版本链）；
/// 查不到 = `None` —— 链建立（2026-10-06）之前的版本谁也不记得，照实「未知」，不猜。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Generation {
    pub revision: String,
    pub published_at: Option<String>,
}

/// 字节指纹 → 出身的索引。扫描范围 = 当前目录 + 单槽归档目录 + 版本链（`archive/catalogs/`），
/// **按各代目录的 mtime 从旧到新**排 —— 同一份字节在几代里都登记过时（发出去又收回重发），
/// 以**最早**登记它的那一代为它的发布（版本的出生，不是它最近一次被提起）。
pub fn generation_index(internal_root: &Path) -> std::collections::HashMap<String, Generation> {
    /* 收集所有能当证据的目录字节，带上各自的 mtime（读不到的排最后，仍参与 ——
    排序只是为了让"最早登记"赢，不是可信度判定） */
    let mut candidates: Vec<(u64, Vec<u8>)> = Vec::new();
    let push = |path: std::path::PathBuf, out: &mut Vec<(u64, Vec<u8>)>| {
        if let Ok(bytes) = std::fs::read(&path) {
            let at = modified_unix_of(&path).unwrap_or(u64::MAX);
            out.push((at, bytes));
        }
    };
    if let Ok(entries) = std::fs::read_dir(super::release::catalog_chain_dir(internal_root)) {
        let mut chain: Vec<_> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .collect();
        chain.sort();
        for p in chain {
            push(p, &mut candidates);
        }
    }
    push(
        archive_dir(internal_root).join(CATALOG_FILE),
        &mut candidates,
    );
    push(super::paths::catalog_file(internal_root), &mut candidates);
    candidates.sort_by_key(|(at, _)| *at);

    let mut index = std::collections::HashMap::new();
    for (_, bytes) in candidates {
        let Ok(catalog) = super::Catalog::parse(&bytes) else {
            continue;
        };
        for f in &catalog.files {
            /* sha 是 None 的条目（随包 bootstrap 不登记）认不出任何字节 —— 跳过 */
            if let Some(sha) = &f.sha256 {
                index.entry(sha.clone()).or_insert_with(|| Generation {
                    revision: catalog.revision.clone(),
                    published_at: catalog.published_at.clone(),
                });
            }
        }
    }
    index
}

/// 单次查一把（多数调用方一次只问一份字节；批量请自建 [`generation_index`]）
pub fn generation_of_sha(internal_root: &Path, sha256: &str) -> Option<Generation> {
    generation_index(internal_root).get(sha256).cloned()
}

/// 字节的 SHA256 hex —— 事件账与版本出身对号用的键
fn sha_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/* ---------- 事件账的读入（含一次性建账） ---------- */

/// 读事件账；账还不存在时先做一次 **bootstrap** 把盘上存量记进来。
///
/// # 建账把哪些存量记进来（记的时刻从哪来）
///
/// - **下载区**：只记**认得出出身**的字节（SHA 对得上某代目录）—— 时刻 = 文件 mtime
///   （它落进下载区那一刻，下载管道写盘后没人动它就是下载时刻）；认不出的（被改过 /
///   链之前的）**不记**，界面照实「未知」—— 正是硬规则③，不许拿 mtime 冒充业务时间；
/// - **归档区**：被换下来的旧字节记一条「替换」—— 时刻 = 归档文件的 mtime。
///   这个 mtime 不是猜：归档是**原子写新文件**（`fsx::atomic_write`），写它的那一刻
///   就是换版那一刻，mtime 直接就是那次 [`PresetEvent::DeliveryReplaced`] 的 `at`。
///
/// bootstrap 之后账只追加（[`preset_events::append`]），任何人都不再回写。
pub fn load_events(internal_root: &Path, catalog: &super::Catalog) -> Vec<PresetEvent> {
    if !preset_events::is_initialized(internal_root) {
        bootstrap_events(internal_root, catalog);
    }
    preset_events::load(internal_root)
}

fn bootstrap_events(internal_root: &Path, catalog: &super::Catalog) {
    let generations = generation_index(internal_root);
    let mut events = Vec::new();

    /* 归档存量先记：每份旧字节记一条「替换」（新份是什么，同位交付文件还在就补上）。
     * 它同时解释了**盘上那份新字节的来历** —— 所以下面下载区的扫描要让着它 */
    for a in archived_files(internal_root) {
        let Some(at) = a.modified_unix else {
            continue;
        };
        let Some(file) = a
            .path
            .strip_prefix(&format!("{}/", super::paths::ARCHIVE_DIR))
        else {
            continue;
        };
        let file = file.to_owned();
        let new_sha = std::fs::read(internal_root.join(&file))
            .ok()
            .map(|bytes| sha_hex(&bytes))
            .filter(|sha| *sha != a.sha256);
        let new_revision = new_sha
            .as_deref()
            .and_then(|sha| generations.get(sha))
            .map(|g| g.revision.clone());
        events.push(PresetEvent::DeliveryReplaced {
            file,
            old_revision: generations.get(&a.sha256).map(|g| g.revision.clone()),
            old_sha256: a.sha256.clone(),
            new_sha256: new_sha,
            new_revision,
            at,
        });
    }

    /* 下载区存量：认得出出身的记「下载」—— 但**来历已经被替换事件解释过的不再记**
     * （盘上那份就是那次替换换上去的新字节；一份字节只有一种来路，硬规则①） */
    for f in &catalog.files {
        let Ok(bytes) = std::fs::read(internal_root.join(&f.path)) else {
            continue;
        };
        let sha = sha_hex(&bytes);
        if preset_events::arrival_of(&events, &f.path, &sha).is_some() {
            continue;
        }
        let Some(g) = generations.get(&sha) else {
            continue; // 认不出的字节不记账（硬规则③）
        };
        let Some(at) = modified_unix_of(&internal_root.join(&f.path)) else {
            continue;
        };
        events.push(PresetEvent::DeliveryDownloaded {
            file: f.path.clone(),
            sha256: sha,
            revision: Some(g.revision.clone()),
            at,
        });
    }

    preset_events::init(internal_root, events);
}

/* ---------- 归档区（旧版本留档） ---------- */

/// 归档区里的一份旧版本。**盘就是底账**（与下载区同一套规矩）：扫盘得到，不记账本。
///
/// 归档是**官方版本生命周期**的一部分（换版本时旧份进 `archive/`，不删），
/// **不是用户修改历史** —— 用户改出来的东西是另一条线（另存成另一份文件），
/// 永远不回写官方原件。所以这里只有"官方旧版本"，没有"谁在什么时候改了什么"。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedFile {
    /// 相对内部根的路径（`archive/delivery/mkp/presets/A1-fast.toml`）—— 世界里唯一的键
    pub path: String,
    /// 文件名。与它对应的交付文件**同名**：换版本换的是字节，不是名字
    pub file_name: String,
    pub size: u64,
    /// 这份旧字节的指纹。事件账（`DeliveryReplaced.old_sha256`）与版本出身
    /// （哪一代目录发布过它）都靠它对号
    pub sha256: String,
    /// 这份旧版本**被换下来的时刻**（UTC epoch 秒）。归档没有单独的"归档时刻"这一回事——
    /// 写这份文件的时刻就是它（原子写新文件，mtime 即写入时刻）。
    /// `None` = 文件系统没给（不是 0，也不编一个）。
    /// **它不再直接上界面**：界面读事件账的 `DeliveryReplaced.at`（建账时从这里补记），
    /// 「云端发布」另有一格（版本出身反查）—— 两个时间各是各，不许互相顶替
    pub modified_unix: Option<u64>,
}

/// 归档区里现在有什么（按路径升序 —— `read_dir` 的顺序是文件系统说的，不稳定，
/// 界面要一个每次刷新都一样的表）。
///
/// 归档区不存在 = 还没归档过东西（合法状态，不是错误）：返回空表。
/// **只列，不动盘**：这一层不提供删除、不提供恢复。
pub fn archived_files(internal_root: &Path) -> Vec<ArchivedFile> {
    let chain_dir = super::release::catalog_chain_dir(internal_root);
    let memory_catalog = archive_dir(internal_root).join(CATALOG_FILE);
    let mut out = Vec::new();
    let mut stack = vec![archive_dir(internal_root)];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue; // 目录不存在 / 读不动：当作"这一支没有东西"，不让整条读失败
        };
        for entry in entries.flatten() {
            let path = entry.path();
            /* 目录（catalog.json）与版本链（catalogs/）是**版本记忆** —— 换下来的是
             * "目录"，不是"旧份"。它们进这里会把记忆文件当成旧版本列出来 */
            if path == memory_catalog || path == chain_dir {
                continue;
            }
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
            /* 指纹现在就算好（旧字节反正是要读的）：调用方拿它对事件账、对版本出身 */
            let sha256 = std::fs::read(&path)
                .map(|bytes| sha_hex(&bytes))
                .unwrap_or_default();
            out.push(ArchivedFile {
                path: rel,
                file_name: path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                size: meta.len(),
                sha256,
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

/* ---------- 删除（作者裁决 2026-10-06：一切皆可删） ---------- */

/// 删除本机下载区里的一份官方交付文件。
///
/// 为什么可以删：删了它回到「未下载」，随时可以从云端重新下载 —— 字节有目录 SHA
/// 锚定，**零数据损失**。此前的「不在这里删」是把"对不上目录时该修"错当成了"不许动"。
///
/// - 认 `file_name`（目录的键，与下载 / 应用同一套口径）；
/// - **正在使用 / 有草稿不归这里管**：使用中指针与草稿由调用方（ipc）先撤下 ——
///   文件层只管文件；
/// - 幂等：本来就不在（已经删过 / 被外部动了）也算成功 —— 目标状态就是"不在"；
/// - **事件账与归档不动**：`DeliveryDownloaded` / `DeliveryReplaced` 是历史事实，
///   不是这份文件的附属；以后重新下载，新事件追加，读侧取最后一条。
pub fn delete_downloaded(
    internal_root: &Path,
    catalog: &super::Catalog,
    file_name: &str,
) -> Result<(), AppError> {
    let file = catalog
        .files
        .iter()
        .find(|f| f.file_name == file_name)
        .ok_or_else(|| AppError::not_found(format!("目录里没有 {file_name}")))?;
    let target = resolve_in(internal_root, &file.path)?;
    remove_file_idempotent(&target, file_name)
}

/// 删除归档区里的一份旧版本（入参 = [`archived_files`] 给的那条相对路径）。
///
/// 允许删，但代价要讲清（界面确认框的事）：**云端只有最新版，这一版删了就找不回**。
/// 版本链（`archive/catalogs/`）与事件账**不动** —— 它们是历史事实；只是删掉的
/// 那份字节从此不在归档清单里。归档"保留最早一份、不覆盖"是**写侧**策略，与
/// "允许用户删"不冲突。
pub fn delete_archived(internal_root: &Path, path: &str) -> Result<(), AppError> {
    let prefix = format!("{}/", super::paths::ARCHIVE_DIR);
    if !path.starts_with(&prefix) {
        return Err(AppError::invalid_argument(format!(
            "只认归档区（{prefix}…）里的路径，别的不是旧版本"
        )));
    }
    let target = resolve_in(internal_root, path)?;
    remove_file_idempotent(&target, path)
}

/// 删一个文件；不存在 = 已是目标状态，照实成功（幂等）
fn remove_file_idempotent(target: &Path, what: &str) -> Result<(), AppError> {
    match std::fs::remove_file(target) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(AppError::io(format!("{what} 删不掉")).with_detail(e.to_string())),
    }
}

/* ---------- 这一份我们认得出吗（第三圈第 6 层：SHA 报警） ---------- */
///
/// 四个答案，**只回答"本机这份是不是我们认可的官方内容"**——不掺"云端有没有更新"
/// （那是 [`super::update::check`] 的问题，比的是目录指纹，与本机的字节无关）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileTrust {
    /// 盘上没有这一份（合法状态，不是错误）
    Absent,
    /// 字节与目录登记一致 —— 就是当前这一版
    Current,
    /// 字节与我们认得的某一版**旧**官方一致 —— 认得出它是哪一版
    OldVersion,
    /// 与目录、归档、被归档的旧目录都对不上 —— 这台机器上查不出它属于哪一版
    Unknown,
}

/// 一份被认出来的旧版本。**证据在哪**要说出来：界面拿它去开抽屉看正文，
/// 读者也能一眼看出"凭什么说它是旧版"。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownVersion {
    pub sha256: String,
    /// 归档区里躺着这份字节时给（`archive/delivery/mkp/presets/A1-fast.toml`）。
    /// 只被旧目录登记过、归档里没有它字节的那种是 `None` —— 那样同样认得出，
    /// 只是没有"可以看正文"这一档
    pub archived_path: Option<String>,
}

/// 除了**目录登记的当前那一版**之外，这台机器还认得这一份文件的哪几版。
///
/// 三个来源，都是"我们亲手发出去的字节"：
///
/// 1. `archive/` 里那份旧文件 —— 换版本时被换下来的那一份；
/// 2. 被归档的旧目录（`archive/catalog.json`）里登记的同一份文件 ——
///    它记着更早那些版本的字节指纹，所以归档槽被后来的版本占了也还能认出更早那一版；
/// 3. **目录版本链**（`archive/catalogs/`，2026-10-06）—— 每一次换代归档的那份旧目录。
///    曾经只有来源 1+2：单槽保留最早一份，官方连发两版中间那版的指纹就永久丢了，
///    用户盘上正规的旧版本会被误判成「内容异常」（2026-10-06 实测 7 份假警报）。
///    链是**追加**的（[`super::release::release_bytes`] 每次换代都进一份），
///    所以"这台机器见过的每一版目录"都在这里。
///
/// **没有第四个来源**：盘上的字节自己不算证据（那正是要判的东西）。
pub fn other_known_versions(internal_root: &Path, file: &CatalogFile) -> Vec<KnownVersion> {
    let mut out = Vec::new();

    let rel = format!("{}/{}", super::paths::ARCHIVE_DIR, file.path);
    if let Ok(bytes) = std::fs::read(internal_root.join(&rel)) {
        out.push(KnownVersion {
            sha256: hex(&Sha256::digest(&bytes)),
            archived_path: Some(rel),
        });
    }

    /* 归档目录（单槽 + 版本链）里登记的指纹。读不出来 / 是更未来的代次的那份 =>
    当作"没有这一档证据"，不让整条判据失败 */
    let mut catalog_paths = vec![internal_root
        .join(super::paths::ARCHIVE_DIR)
        .join(CATALOG_FILE)];
    if let Ok(entries) = std::fs::read_dir(super::release::catalog_chain_dir(internal_root)) {
        let mut chain: Vec<_> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .collect();
        chain.sort();
        catalog_paths.extend(chain);
    }
    for path in catalog_paths {
        let Ok(old_bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(old) = super::Catalog::parse(&old_bytes) else {
            continue;
        };
        let hit = old
            .files
            .iter()
            .find(|f| f.path == file.path)
            .or_else(|| old.files.iter().find(|f| f.file_name == file.file_name));
        /*
         * 旧目录那条要给得出指纹才算证据：随包 bootstrap 目录不登记它（`sha256 = None`），
         * 一条没有指纹的登记认不出任何字节 —— 跳过，而不是拿 `None` 去比（那会把它当成
         * "与任何字节都不同"的假证据，把盘上文件误判成 `Unknown`）。
         */
        if let Some(sha) = hit.and_then(|entry| entry.sha256.clone()) {
            out.push(KnownVersion {
                sha256: sha,
                archived_path: None,
            });
        }
    }
    out
}

/// 盘上这一份是哪一版（[`FileTrust`] 四档），以及认得出时**证据在哪**。
fn inspect(internal_root: &Path, file: &CatalogFile) -> (FileTrust, Option<String>) {
    let Ok(bytes) = std::fs::read(internal_root.join(&file.path)) else {
        return (FileTrust::Absent, None);
    };
    let got = hex(&Sha256::digest(&bytes));
    let Some(want) = file.expected_sha() else {
        /*
         * 目录没登记期望值（随包 bootstrap 目录）：盘上有就算 `Current` ——
         * 这一侧的目录没资格为它的字节背书，说 `Unknown` 会让每一份已下载文件
         * 都进"报警表"（`trust_entries`），那是几百条假警报。
         * 字节的权威判定归 OTA 目录生效后那次下载 / `file_status`。
         */
        return (FileTrust::Current, None);
    };
    if got == want {
        return (FileTrust::Current, None);
    }
    for known in other_known_versions(internal_root, file) {
        if known.sha256 == got {
            return (FileTrust::OldVersion, known.archived_path);
        }
    }
    (FileTrust::Unknown, None)
}

pub fn trust_of(internal_root: &Path, file: &CatalogFile) -> FileTrust {
    inspect(internal_root, file).0
}

/// 一份"认得出 / 认不出"的交代。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTrustEntry {
    pub file_name: String,
    pub trust: FileTrust,
    /// `OldVersion` 且归档里有它字节时的路径（界面据此这样说得出正文）；其余是 `None`
    pub archived_path: Option<String>,
}

/// 目录登记的每一份都过一遍，**只列有事的**：盘上没有（还没下载）和与目录一致的那两种
/// 不在"报警表"里 —— 它们没有问题，列出来只会把真正要处理的那几份淹掉。
pub fn trust_entries(internal_root: &Path, catalog: &super::Catalog) -> Vec<FileTrustEntry> {
    catalog
        .files
        .iter()
        .filter_map(|file| {
            let (trust, archived_path) = inspect(internal_root, file);
            match trust {
                FileTrust::Absent | FileTrust::Current => None,
                FileTrust::OldVersion | FileTrust::Unknown => Some(FileTrustEntry {
                    file_name: file.file_name.clone(),
                    trust,
                    archived_path,
                }),
            }
        })
        .collect()
}

/// 取出**可以当依据**的官方正文：只有盘上这一份与目录登记逐字节一致时才给。
///
/// 「临时编辑」那条链的第一步就在于此：改的来源必须是当前这一版官方原件。
/// 被改过（SHA 对不上）的那些不给 —— 它的字节存疑，修复它的动作是**重新下载**，
/// 不许拿它当原文去改（改完另存成用户文件，等于把可疑内容洗成"我改过的那一份"）。
pub fn official_text(internal_root: &Path, file: &CatalogFile) -> Result<String, AppError> {
    let bytes = std::fs::read(internal_root.join(&file.path)).map_err(|_| {
        AppError::not_found(format!("{} 还没下载到本机 —— 先下载，再改", file.file_name))
    })?;
    // 只有目录登记了期望值才比对：随包 bootstrap 目录不登记它，读得出来的正文就是可用正文
    if let Some(want) = file.expected_sha() {
        if hex(&Sha256::digest(&bytes)) != want {
            return Err(AppError::sha_mismatch(format!(
                "{} 盘上这一份与目录登记的字节不一致 —— 先用「重新下载」把它换成干净的官方版，再改",
                file.file_name
            )));
        }
    }
    String::from_utf8(bytes)
        .map_err(|_| AppError::corrupted(format!("{} 不是 UTF-8 文本，改不了", file.file_name)))
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
    entries_in_status(internal_root, catalog, FileOnDisk::Current)
        .into_iter()
        .map(|e| e.file_name)
        .collect()
}

/// 有更新的文件名（盘上在，但字节与目录不一样）。
/// "更新"动作就是对这些文件再跑一遍 [`deliver`]——旧份自动归档，没有单独的更新代码路径。
pub fn stale_files(internal_root: &Path, catalog: &super::Catalog) -> Vec<String> {
    entries_in_status(internal_root, catalog, FileOnDisk::Stale)
        .into_iter()
        .map(|e| e.file_name)
        .collect()
}

/// 盘上的一份：文件名 + **这一份字节的时间**（全部来自事件，mtime 不再上界面）。
///
/// - `downloaded_unix` / `replaced_unix` —— 这份字节是「下载」进来的还是「替换」上去的
///   （[`preset_events::Arrival`]，两个事件**至多一个在**：同一份字节只有一种来路）。
///   都没有 = 认不出出身的字节（这种字节不记账，不猜 —— 硬规则③），界面照实「未知」；
/// - `published_at` —— 这份字节属于哪一代目录、那一代什么时候发布的
///   （`ReleasePublished` 的读法：反查目录与版本链）。查不到 = `None`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnDiskEntry {
    pub file_name: String,
    pub downloaded_unix: Option<u64>,
    pub replaced_unix: Option<u64>,
    pub published_at: Option<String>,
}

/// [`downloaded_files`] 带时间的那一版（界面预设页本地表的「时间」列）。
pub fn downloaded_entries(internal_root: &Path, catalog: &super::Catalog) -> Vec<OnDiskEntry> {
    entries_in_status(internal_root, catalog, FileOnDisk::Current)
}

/// [`stale_files`] 带时间的那一版（盘上那份对不上目录：旧版本的时间照样答得出 ——
/// 它是"这份字节是什么时候到我机器上的"；认不出的照实没有）。
pub fn stale_entries(internal_root: &Path, catalog: &super::Catalog) -> Vec<OnDiskEntry> {
    entries_in_status(internal_root, catalog, FileOnDisk::Stale)
}

fn entries_in_status(
    internal_root: &Path,
    catalog: &super::Catalog,
    want: FileOnDisk,
) -> Vec<OnDiskEntry> {
    let events = load_events(internal_root, catalog);
    let generations = generation_index(internal_root);
    catalog
        .files
        .iter()
        .filter_map(|f| {
            let bytes = std::fs::read(internal_root.join(&f.path)).ok()?;
            if status_of(f, &bytes) != want {
                return None;
            }
            let sha = sha_hex(&bytes);
            let arrival = preset_events::arrival_of(&events, &f.path, &sha);
            Some(OnDiskEntry {
                file_name: f.file_name.clone(),
                downloaded_unix: match arrival {
                    Some(preset_events::Arrival::Downloaded(at)) => Some(at),
                    _ => None,
                },
                replaced_unix: match arrival {
                    Some(preset_events::Arrival::Replaced(at)) => Some(at),
                    _ => None,
                },
                published_at: generations.get(&sha).and_then(|g| g.published_at.clone()),
            })
        })
        .collect()
}

/// 文件的 mtime → UTC epoch 秒。读不到 / 早于 1970 的都算"没给"——不编一个 0 出来
fn modified_unix_of(path: &Path) -> Option<u64> {
    let meta = std::fs::metadata(path).ok()?;
    meta.modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
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
            sha256: Some(hex(&Sha256::digest(content))),
            size: Some(content.len() as u64),
        }
    }

    /// 随包 bootstrap 目录那种条目：有这一份、不登记期望值
    fn entry_without_expectations(name: &str) -> CatalogFile {
        CatalogFile {
            kind: "mkp_preset".to_owned(),
            file_name: name.to_owned(),
            path: format!("mkp/{name}"),
            machine_id: "A1".to_owned(),
            version_id: "STANDARD".to_owned(),
            sha256: None,
            size: None,
        }
    }

    /// 内存源：直接吐预置的字节，测试不碰第二个临时目录
    struct MemorySource(Vec<u8>);
    impl Source for MemorySource {
        fn fetch(&self, _file: &CatalogFile) -> Result<Vec<u8>, AppError> {
            Ok(self.0.clone())
        }
    }

    fn catalog_with_rev(files: Vec<CatalogFile>, revision: &str) -> Catalog {
        Catalog {
            catalog_schema: super::super::catalog::CATALOG_SCHEMA,
            revision: revision.to_owned(),
            files,
            ..Catalog::default()
        }
    }

    fn catalog_with(files: Vec<CatalogFile>) -> Catalog {
        catalog_with_rev(files, "test")
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

    /// 「下载到本机的时刻」（2026-10-06 改为**事件账**）：`downloaded_entries` 给的
    /// `downloaded_unix` = `DeliveryDownloaded.at`（deliver 落盘成功那一刻记的）——
    /// 本地表「时间」列的真值来源。**账上没有就是 `None`**（不编，不许拿 mtime 顶）。
    #[test]
    fn downloaded_entries_carry_the_download_event() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(mkp_root(&root)).unwrap();
        let content = "# A1 standard\nspeed_limit = 60\n".as_bytes();
        let file = entry("A1-standard.toml", content);

        deliver(root.path(), &file, &MemorySource(content.to_vec())).unwrap();

        let got = downloaded_entries(root.path(), &catalog_with(vec![file]));
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].file_name, "A1-standard.toml");
        assert!(
            got[0].downloaded_unix.is_some(),
            "下载事件要给 —— 它就是界面上「下载到本机的时刻」"
        );
        assert!(got[0].replaced_unix.is_none());
    }

    /// ★ **事件定格**（2026-10-06 预设事件时间模型）：deliver 落盘成功的那一刻记事件
    /// —— 第一次是「下载」，换版是「替换」，同字节重放什么都不记。一条替换事件同时
    /// 解释两侧：归档那份旧字节的「替换时间」与盘上新字节的「到位时间」，互不顶替。
    #[test]
    fn deliver_records_events_as_they_happen() {
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
        // 同字节重放：什么都没发生，不记事件
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

        let events = preset_events::load(root.path());
        assert_eq!(events.len(), 2, "下载一条 + 替换一条");
        match &events[0] {
            PresetEvent::DeliveryDownloaded { sha256, at, .. } => {
                assert_eq!(sha256, &sha_hex("版本一".as_bytes()));
                assert!(*at > 0, "事件时刻是发生那一刻，不是 0");
            }
            other => panic!("第一条该是下载，结果是 {other:?}"),
        }
        match &events[1] {
            PresetEvent::DeliveryReplaced {
                old_sha256,
                new_sha256,
                ..
            } => {
                assert_eq!(old_sha256, &sha_hex("版本一".as_bytes()));
                assert_eq!(
                    new_sha256.as_deref(),
                    Some(sha_hex("版本二".as_bytes()).as_str())
                );
            }
            other => panic!("第二条该是替换，结果是 {other:?}"),
        }

        // 读侧：盘上这份（v2）答得出自己是「替换」上去的，不是「下载」的 —— 一个事件一个语义
        let catalog = catalog_with(vec![v2]);
        let got = downloaded_entries(root.path(), &catalog);
        assert_eq!(got.len(), 1);
        assert!(
            got[0].downloaded_unix.is_none(),
            "这份字节是替换上去的 —— 不许拿下载事件顶替"
        );
        assert!(got[0].replaced_unix.is_some());
    }

    /// ★ **建账 bootstrap**：账不存在时把盘上存量一次性记进来。
    /// - 纯下载的存量（没有归档）记「下载」，时刻 = mtime（它落盘那一刻）；
    /// - 有归档的存量记「替换」，时刻 = 归档文件的 mtime（归档是原子写新文件，
    ///   写它的那一刻就是换版那一刻）—— **同一份盘上字节只有一种来路**：被替换
    ///   事件解释过的不再记「下载」（硬规则①）；
    /// - 认不出出身的字节不记（硬规则③，不许 mtime 冒充业务时间）。
    ///
    /// 建过账之后只追加，绝不回写。
    #[test]
    fn bootstrap_seeds_the_ledger_from_disk() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(mkp_root(&root)).unwrap();
        let plain = entry("A1-standard.toml", "纯下载的存量".as_bytes());
        let replaced = entry("A1-fast.toml", "替换上去的新份".as_bytes());
        // 直接把文件放盘上（不走 deliver）：模拟"账建起来之前"的存量
        /* 写盘走全仓唯一那个出口（`clippy.toml` 禁 `std::fs::write`）—— 测试也一样 */
        crate::fsx::atomic::atomic_write(&root.path().join(&plain.path), "纯下载的存量".as_bytes())
            .unwrap();
        crate::fsx::atomic::atomic_write(
            &root.path().join(&replaced.path),
            "替换上去的新份".as_bytes(),
        )
        .unwrap();
        std::fs::create_dir_all(root.path().join("archive/mkp")).unwrap();
        crate::fsx::atomic::atomic_write(
            &root.path().join("archive/mkp/A1-fast.toml"),
            "被换下的旧份".as_bytes(),
        )
        .unwrap();
        // 目录也要在盘上：出身的证据（认不出 = 不记）
        crate::fsx::atomic::atomic_write(
            &crate::runtime::paths::catalog_file(root.path()),
            catalog_with(vec![plain.clone(), replaced.clone()])
                .to_pretty_json()
                .unwrap()
                .as_bytes(),
        )
        .unwrap();

        let catalog = catalog_with(vec![plain, replaced]);
        let got = downloaded_entries(root.path(), &catalog); // 第一次读：触发建账
        assert_eq!(got.len(), 2);
        let plain_row = got
            .iter()
            .find(|e| e.file_name == "A1-standard.toml")
            .unwrap();
        assert!(
            plain_row.downloaded_unix.is_some() && plain_row.replaced_unix.is_none(),
            "纯下载的存量记「下载」"
        );
        let replaced_row = got.iter().find(|e| e.file_name == "A1-fast.toml").unwrap();
        assert!(
            replaced_row.replaced_unix.is_some() && replaced_row.downloaded_unix.is_none(),
            "被替换事件解释过的字节只有一种来路 —— 替换，不记下载"
        );

        // 归档那份旧字节：替换事件的 at = 归档文件的 mtime
        let events = preset_events::load(root.path());
        let replaced_event = events
            .iter()
            .find(|e| matches!(e, PresetEvent::DeliveryReplaced { .. }))
            .expect("归档存量要有一条替换事件");
        match replaced_event {
            PresetEvent::DeliveryReplaced {
                file: f,
                old_sha256,
                at,
                ..
            } => {
                assert_eq!(f, "mkp/A1-fast.toml");
                assert_eq!(old_sha256, &sha_hex("被换下的旧份".as_bytes()));
                assert!(*at > 0);
            }
            _ => unreachable!(),
        }
    }

    /// 认不出出身的字节（哪一代目录都没登记过它）**不记事件** ——
    /// 下载时间照实「未知」，不许拿 mtime 顶（硬规则③）
    #[test]
    fn unrecognized_bytes_get_no_event() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(mkp_root(&root)).unwrap();
        let file = entry("A1-standard.toml", "目录登记的版本".as_bytes());
        // 盘上躺着的是来路不明的字节
        crate::fsx::atomic::atomic_write(
            &root.path().join(&file.path),
            "被人动过的字节".as_bytes(),
        )
        .unwrap();
        crate::fsx::atomic::atomic_write(
            &crate::runtime::paths::catalog_file(root.path()),
            catalog_with(vec![file.clone()])
                .to_pretty_json()
                .unwrap()
                .as_bytes(),
        )
        .unwrap();

        let got = stale_entries(root.path(), &catalog_with(vec![file]));
        assert_eq!(got.len(), 1);
        assert!(
            got[0].downloaded_unix.is_none() && got[0].replaced_unix.is_none(),
            "认不出的字节没有事件 —— 界面照实「未知」"
        );
    }

    /// ★ **目录没给期望值就不做字节校验**（2026-10-04，去 SHA 的另一半）。
    ///
    /// 随包 bootstrap 目录的条目 `sha256`/`size` 都是 `None`。这时下载**必须照收**：
    /// 这一侧的目录没资格为字节背书，拿一个空 SHA 去"比一比"看着像校验、其实什么都没验，
    /// 更糟的是会把好文件判成坏的。真正的校验等 OTA 目录生效后那次下载。
    ///
    /// 反向也钉住：**有期望值时仍然照校验**（上一条判据咬着）。
    #[test]
    fn delivers_without_checking_when_the_catalog_expects_nothing() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(mkp_root(&root)).unwrap();
        let file = entry_without_expectations("A1-standard.toml");
        let content = "随包目录不认识的字节".as_bytes();

        let target = deliver(root.path(), &file, &MemorySource(content.to_vec()))
            .expect("没有期望值就不该拒收");

        assert_eq!(
            std::fs::read(&target).unwrap(),
            content,
            "收下的是源给的字节"
        );
        // 盘上有就算"当前"：这一侧的目录说不了它是旧是漂
        assert_eq!(file_status(root.path(), &file), FileOnDisk::Current);
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
        // 拿不到远端目录（这个夹具没配源、没联网）时**退回保守说法** ——
        // 不许把"没联网"说成"目录过期"，更不许编一个 revision
        assert!(
            e.message.contains("文件在源头就被改过或传坏了"),
            "取不到远端时照保守说：{}",
            e.message
        );
        assert!(
            !e.message.contains("检查更新"),
            "没确认目录过期就不要提『检查更新』：{}",
            e.message
        );
    }

    /// ★ **目录过期与文件坏是两种话**（2026-10-04）。
    ///
    /// SHA 对不上有两种成因，用户能据以行动的动作完全不同：
    /// 目录过期 → 去刷新目录；文件真坏 → 换一台机器 / 找发布方。
    /// 以前两种共用"文件在源头就被改过或传坏了"，导致用户照着这句话查不出问题。
    ///
    /// 判据直接钉那句话本身（取 revision 要联网，测试里不碰网络）。
    #[test]
    fn stale_catalog_gets_a_different_message_than_a_bad_file() {
        let file = entry("A1-standard.toml", "真内容".as_bytes());

        // ① 两个 revision 不同 → 说"目录过期"，并给出行动建议
        let stale = sha_mismatch_message(
            &file,
            "got-sha",
            Some(("remote-rev".to_owned(), "local-rev".to_owned())),
        );
        assert!(
            stale.contains("local-rev") && stale.contains("remote-rev"),
            "两个 revision 都要摆出来：{stale}"
        );
        assert!(stale.contains("检查更新"), "要给行动建议：{stale}");
        assert!(
            !stale.contains("传坏了"),
            "已定位到目录过期，就别说文件坏了：{stale}"
        );

        // ② 拿不到远端（None）→ 保守说法，且**不提**「检查更新」
        let unknown = sha_mismatch_message(&file, "got-sha", None);
        assert!(
            unknown.contains("文件在源头就被改过或传坏了"),
            "取不到远端时照保守说：{unknown}"
        );
        assert!(
            !unknown.contains("检查更新"),
            "没据实确认就不要提『检查更新』：{unknown}"
        );
    }

    #[test]
    fn rejects_size_mismatch() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(mkp_root(&root)).unwrap();
        let mut file = entry("A1-standard.toml", "内容".as_bytes());
        file.size = Some(999); // 目录里记错了大小

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
        assert!(
            got[0].modified_unix.is_some(),
            "写这份文件的时刻要带上（bootstrap 的原料）"
        );
        assert_eq!(
            got[0].sha256,
            sha_hex("版本一".as_bytes()),
            "指纹现在就算好 —— 事件账与版本出身的对号键"
        );
        assert_eq!(
            std::fs::read(root.path().join(&got[0].path)).unwrap(),
            "版本一".as_bytes(),
            "按这条读给的路径读回来的是旧版本的原字节"
        );
    }

    /// 目录（catalog.json）与版本链（catalogs/）是**版本记忆** —— 换下来的是"目录"，
    /// 不是"旧份"，不许进归档清单（2026-10-06 加版本链时一并修掉的泄漏）
    #[test]
    fn archived_files_skip_the_catalog_memory() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("archive/catalogs")).unwrap();
        crate::fsx::atomic::atomic_write(&root.path().join("archive/catalog.json"), b"{}").unwrap();
        crate::fsx::atomic::atomic_write(&root.path().join("archive/catalogs/gen-2.json"), b"{}")
            .unwrap();
        std::fs::create_dir_all(root.path().join("archive/mkp")).unwrap();
        crate::fsx::atomic::atomic_write(
            &root.path().join("archive/mkp/A1-standard.toml"),
            "旧份".as_bytes(),
        )
        .unwrap();

        let got = archived_files(root.path());
        assert_eq!(got.len(), 1, "只有旧份本身");
        assert_eq!(got[0].file_name, "A1-standard.toml");
        assert_eq!(got[0].sha256, sha_hex("旧份".as_bytes()));
    }

    /* ---------- 删除（作者裁决 2026-10-06：一切皆可删） ---------- */

    /// 删交付区那份：盘上没了 = 回到「未下载」；**事件账不动**（历史事实，
    /// 以后重下追加新事件）；幂等 —— 再删一次也成功
    #[test]
    fn delete_downloaded_removes_only_the_disk_copy() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(mkp_root(&root)).unwrap();
        let file = entry("A1-standard.toml", "内容".as_bytes());
        deliver(
            root.path(),
            &file,
            &MemorySource("内容".as_bytes().to_vec()),
        )
        .unwrap();

        delete_downloaded(
            root.path(),
            &catalog_with(vec![file.clone()]),
            "A1-standard.toml",
        )
        .unwrap();
        assert!(downloaded_files(root.path(), &catalog_with(vec![file])).is_empty());
        // 事件账还在：删的是文件，不是历史
        assert_eq!(preset_events::load(root.path()).len(), 1);

        // 幂等：再删一次（已经不在）照实成功
        delete_downloaded(
            root.path(),
            &catalog_with(vec![entry("A1-standard.toml", b"x")]),
            "A1-standard.toml",
        )
        .unwrap();
    }

    /// 删归档那份：清单里没了；版本链与事件账不动；目录之外的路径不认（防穿越）
    #[test]
    fn delete_archived_removes_only_that_file() {
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
        assert_eq!(archived_files(root.path()).len(), 1);

        delete_archived(root.path(), "archive/mkp/A1-standard.toml").unwrap();
        assert!(archived_files(root.path()).is_empty());
        // 事件账与下载区都不动
        assert!(!preset_events::load(root.path()).is_empty());
        assert_eq!(
            std::fs::read(root.path().join("mkp/A1-standard.toml")).unwrap(),
            "版本二".as_bytes()
        );

        // 防穿越：只认归档区前缀
        assert!(delete_archived(root.path(), "mkp/A1-standard.toml").is_err());
        assert!(delete_archived(root.path(), "../外面.toml").is_err());
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

    /* ---------- 这一份我们认得出吗（第三圈第 6 层：SHA 报警） ---------- */

    /// 往内部根里写一份文件（写盘走仓库唯一那个出口）
    fn put_file(root: &Path, rel: &str, content: &str) {
        crate::fsx::atomic::atomic_write(&root.join(rel), content.as_bytes()).unwrap();
    }

    /// 造一份只用来当**被归档的旧目录**的 catalog（它登记着更早那一版的字节指纹）
    fn old_catalog_json(files: Vec<CatalogFile>) -> Vec<u8> {
        let catalog = Catalog {
            catalog_schema: super::super::catalog::CATALOG_SCHEMA,
            revision: "older".to_owned(),
            files,
            ..Catalog::default()
        };
        catalog.to_pretty_json().unwrap().into_bytes()
    }

    /// 与目录一致 = 当前这一版：报警表里不该有它
    #[test]
    fn a_clean_copy_is_current_and_not_reported() {
        let root = tempfile::tempdir().unwrap();
        let file = entry("A1-standard.toml", "官方当前版本".as_bytes());
        put_file(root.path(), "mkp/A1-standard.toml", "官方当前版本");

        let catalog = catalog_with(vec![file.clone()]);
        assert_eq!(trust_of(root.path(), &file), FileTrust::Current);
        assert!(
            trust_entries(root.path(), &catalog).is_empty(),
            "没问题就不报警"
        );
    }

    /// **认不出**：既不是目录这一版，也不是归档里那一版 —— 这台机器上查不出它属于哪一版
    #[test]
    fn tampered_bytes_are_not_recognized() {
        let root = tempfile::tempdir().unwrap();
        let file = entry("A1-standard.toml", "官方当前版本".as_bytes());
        put_file(root.path(), "mkp/A1-standard.toml", "被人动过的字节");

        assert_eq!(trust_of(root.path(), &file), FileTrust::Unknown);

        let got = trust_entries(root.path(), &catalog_with(vec![file]));
        assert_eq!(got.len(), 1, "有问题的那一份要出现在报警表里");
        assert_eq!(got[0].file_name, "A1-standard.toml");
        assert_eq!(got[0].trust, FileTrust::Unknown);
        assert!(got[0].archived_path.is_none(), "认不出就没有证据可指");
    }

    /// **认得出**：盘上这份就是归档里那一版 —— 旧版本，而且说得出证据在哪
    #[test]
    fn a_copy_of_the_archived_version_is_recognized_as_old() {
        let root = tempfile::tempdir().unwrap();
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
        /* 用户把归档里那份旧版捞回下载区（或更新还没落到这里） */
        put_file(root.path(), "mkp/A1-standard.toml", "版本一");

        assert_eq!(trust_of(root.path(), &v2), FileTrust::OldVersion);

        let got = trust_entries(root.path(), &catalog_with(vec![v2]));
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].trust, FileTrust::OldVersion);
        assert_eq!(
            got[0].archived_path.as_deref(),
            Some("archive/mkp/A1-standard.toml"),
            "证据是归档里那一份 —— 界面拿它去看旧版正文"
        );
    }

    /// 归档区里没有它的字节，但**被归档的旧目录登记过它** —— 同样认得出（认得出的那一档
    /// 不会因为归档槽被人占了就退化成"异常修改"）
    #[test]
    fn a_version_from_the_archived_catalog_is_recognized_as_old() {
        let root = tempfile::tempdir().unwrap();
        let older = entry("A1-standard.toml", "更旧的那一版".as_bytes());
        let current = entry("A1-standard.toml", "当前版本".as_bytes());
        put_file(root.path(), "mkp/A1-standard.toml", "更旧的那一版");
        crate::fsx::atomic::atomic_write(
            &root
                .path()
                .join(super::super::paths::ARCHIVE_DIR)
                .join(CATALOG_FILE),
            &old_catalog_json(vec![older]),
        )
        .unwrap();

        assert_eq!(trust_of(root.path(), &current), FileTrust::OldVersion);
        let got = trust_entries(root.path(), &catalog_with(vec![current]));
        assert_eq!(got[0].trust, FileTrust::OldVersion);
        assert!(
            got[0].archived_path.is_none(),
            "归档区里没有它的字节 —— 认得出，但没有那一版可以打开看"
        );
    }

    /// **版本链修的那个 bug 的钉子**（2026-10-06，实测 7 份假「内容异常」）：
    ///
    /// 官方连发三代（v1 → v2 → v3），用户在 v2 那一代下载了文件，随后目录一路换到 v3。
    /// 单槽归档（保留最早）会把 v2 那代目录永久丢掉 —— 单槽里只有 v1 代的目录、
    /// 归档槽里只有 v1 的字节，盘上**正规的 v2** 谁都认不出，被误判成 `Unknown`
    /// （「内容异常」）。版本链（`archive/catalogs/`）把每一代被换下的目录都留下来之后，
    /// v2 必须被认成 `OldVersion`（「旧版本 · 有更新」），而不是异常。
    #[test]
    fn a_middle_generation_stays_recognized_through_the_catalog_chain() {
        let root = tempfile::tempdir().unwrap();
        let v1 = entry("A1-fastv3.3.toml", "第一版内容".as_bytes());
        let v2 = entry("A1-fastv3.3.toml", "第二版内容".as_bytes());
        let v3 = entry("A1-fastv3.3.toml", "第三版内容".as_bytes());
        let gen1 = catalog_with_rev(vec![v1.clone()], "gen-1");
        let gen2 = catalog_with_rev(vec![v2.clone()], "gen-2");
        let gen3 = catalog_with_rev(vec![v3.clone()], "gen-3");

        // 三代目录依次上盘：release_bytes 每次把被换下的那份归档（单槽 + 版本链）
        let put_catalog = |bytes: &[u8]| {
            crate::fsx::atomic::atomic_write(&root.path().join(CATALOG_FILE), bytes).unwrap();
        };
        put_catalog(gen1.to_pretty_json().unwrap().as_bytes());
        crate::runtime::release::release_bytes(
            root.path(),
            gen2.to_pretty_json().unwrap().as_bytes(),
        )
        .unwrap();
        crate::runtime::release::release_bytes(
            root.path(),
            gen3.to_pretty_json().unwrap().as_bytes(),
        )
        .unwrap();

        // 用户盘上躺着的是 v2（在 gen-2 那一代下载的），云端已经是 v3
        put_file(root.path(), "mkp/A1-fastv3.3.toml", "第二版内容");

        assert_eq!(
            trust_of(root.path(), &v3),
            FileTrust::OldVersion,
            "中间那代的指纹在版本链里 —— 正规旧版本必须认得出，不许误报内容异常"
        );
        // 单槽里只有第一代：这条证据只可能来自链
        let got = trust_entries(root.path(), &gen3);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].trust, FileTrust::OldVersion);
    }

    /// 真正来路不明的字节（哪一代目录都没登记过它）依旧 `Unknown` ——
    /// 版本链扩的是记忆，不是把异常洗成旧版本
    #[test]
    fn bytes_outside_every_generation_are_still_unknown() {
        let root = tempfile::tempdir().unwrap();
        let v1 = entry("A1-standard.toml", "第一版内容".as_bytes());
        let current = entry("A1-standard.toml", "当前版本".as_bytes());
        let gen1 = catalog_with_rev(vec![v1.clone()], "gen-1");
        let gen2 = catalog_with_rev(vec![current.clone()], "gen-2");
        crate::fsx::atomic::atomic_write(
            &root.path().join(CATALOG_FILE),
            gen1.to_pretty_json().unwrap().as_bytes(),
        )
        .unwrap();
        crate::runtime::release::release_bytes(
            root.path(),
            gen2.to_pretty_json().unwrap().as_bytes(),
        )
        .unwrap();
        put_file(root.path(), "mkp/A1-standard.toml", "被人动过的字节");

        assert_eq!(trust_of(root.path(), &current), FileTrust::Unknown);
    }

    /// 「改这份」的入口：**SHA 对不上的字节不许当原文用**（修它的动作是重新下载）
    #[test]
    fn official_text_refuses_bytes_that_drifted() {
        let root = tempfile::tempdir().unwrap();
        let file = entry("A1-standard.toml", "官方当前版本".as_bytes());

        let missing = official_text(root.path(), &file).unwrap_err();
        assert_eq!(missing.code, crate::error::ErrorCode::NotFound);

        put_file(root.path(), "mkp/A1-standard.toml", "被人动过的字节");
        let drifted = official_text(root.path(), &file).unwrap_err();
        assert_eq!(drifted.code, crate::error::ErrorCode::ShaMismatch);

        put_file(root.path(), "mkp/A1-standard.toml", "官方当前版本");
        assert_eq!(
            official_text(root.path(), &file).unwrap(),
            "官方当前版本",
            "与目录一致的那一份才是可以改的原文"
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
                file.sha256 = Some(hex(&Sha256::digest(&bytes)));
                file.size = Some(bytes.len() as u64);
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

    /// 单测里造"交付面"目录：新语义下没有预建的下载区根，
    /// 但不少用例要先 `create_dir_all` 才能让 `deliver` 有地方写 —— 给它一个与
    /// `catalog.path` 第一段同名的地方（`assets/`、`dist/` 都行，这里取通用一点的名字）。
    fn mkp_root(dir: &tempfile::TempDir) -> PathBuf {
        let p = dir.path().join("mkp");
        let _ = std::fs::create_dir_all(&p);
        p
    }
}
