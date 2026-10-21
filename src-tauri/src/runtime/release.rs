//! 释放口：把安装包里嵌着的 catalog 铺进内部根（层② → 层③）。
//!
//! # 两个动作，两条路（2026-10-04 拆开 —— 混用会造成"启动回退 OTA"）
//!
//! catalog 的生命周期是 **bootstrap → OTA → 当前**。随包那份只是**起点**，
//! 不是最终资源（铁律 4：运行时只认自己的运行时数据）。所以写盘分两个动作：
//!
//! | 函数 | 谁调 | 语义 |
//! |---|---|---|
//! | [`ensure_released`] | 启动（`lib.rs::setup`）、读不到时的兜底 | **对齐**：按「谁更新」判（见下），同版本一个字节不动 |
//! | [`release_bytes`] | [`super::update`] 的应用侧（`apply_remote_update`） | **升级**：盘上不同 → 旧份归档、新份生效 |
//!
//! ★ 启动**绝不能**走 [`release_bytes`]：那会让每次启动都用随包那份覆盖掉
//! 用户 OTA 拿到的新目录，下载随即 SHA 不匹配。详见 [`ensure_released`] 的注释。
//!
//! 归档发生在 [`release_bytes`] 与 [`ensure_released`] 的升级分支里（同一份
//! [`archive_current`]），分两层：
//!
//! - **版本链** `archive/catalogs/<revision>.json`：每一次换代都**追加**一份旧目录 ——
//!   这是"这份文件我们认得出是哪一版"的全部记忆。曾经过的弯路：只留一个槽
//!   （保留最早）时，连发两版中间那版的指纹就永久丢了 —— 用户上午下的货、下午换目录，
//!   晚上盘上那几份全部被误判成「内容异常」（2026-10-06 实测 7 份假警报，那批字节
//!   在发布历史里都是正规版本）；
//! - **单槽** `archive/catalog.json`：保留最早一份（行为不变，老判据还在读它）。
//!
//! 换 catalog 之后文件层面的旧版本怎么办，是 [`super::delivery`] 的事
//! （Stale → 重走管道 → 旧文件归档）。
//!
//! # 交付面初始为空（2026-10-04 改口径）
//!
//! 铁律 3 仍然成立：**用户没下载的，目录里就没有**。但实现方式变了 ——
//! 以前是"建出一个空的 `mkp/` 目录"来体现这一条；现在交付文件的落点由
//! `catalog.path` 决定（唯一路径语义），**没有一个固定的"下载区根"可以提前建**。
//!
//! 所以这里**不预建任何交付目录**：目录在 [`super::delivery::deliver`] 落盘时
//! 按需 `create_dir_all`。判据也从"`mkp/` 是空的"改成"下载前一个交付文件都没有"
//! （见 `release.rs` 的测试）。

use std::path::Path;

use crate::error::AppError;
use crate::fsx::atomic::atomic_write;

use super::paths::{archive_dir, catalog_file};
use super::EMBEDDED_CATALOG;

/// 版本链目录：`archive/catalogs/`。每一次换代把**被换下的那份**追加进来，
/// 文件名用那一份自己的 revision（[`super::catalog::Catalog::revision`]）。
pub fn catalog_chain_dir(root: &Path) -> std::path::PathBuf {
    archive_dir(root).join("catalogs")
}

/// revision → 链内文件名。revision 是发布侧生成的指纹（hex），但这一层不信任任何
/// 来源的形状：只留文件名安全的那几个字符，其余换 `_`；清完是空的就用 `unknown`。
fn chain_file_name(revision: &str) -> String {
    let safe: String = revision
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if safe.is_empty() {
        "unknown".to_owned()
    } else {
        safe
    }
}

#[derive(Debug, Default)]
pub struct ReleaseReport {
    /// 这次真写下去的（首次启动 / 文件被删 → [`ensure_released`]；升级 → [`release_bytes`]）
    pub written: bool,
    /// 盘上的旧份被归档了（= 这是一次升级 —— **只会**发生在 [`release_bytes`]）
    pub archived: bool,
}

impl ReleaseReport {
    /// 一行给日志看的话
    pub fn summary(&self) -> String {
        match (self.written, self.archived) {
            (true, true) => "catalog 已更新（旧份归档进 archive/）".to_owned(),
            (true, false) => "catalog 已释放".to_owned(),
            (false, _) => "catalog 已是最新".to_owned(),
        }
    }
}

/// 启动时把**随包那份**与**盘上那份**对齐 —— 只升不降，比较的是版本号不是时间。
///
/// # 判定表（2026-10-21）
///
/// | 盘上状态 | 动作 |
/// |---|---|
/// | 没有文件 | 铺随包那份 |
/// | schema 比本程序新 | **不动**（降级场景：旧 exe 读不懂新版数据，别毁它） |
/// | 字节完全一样 | 不动（幂等） |
/// | 读得出来且 `revision` 相同 | 不动（同内容不同排版） |
/// | 盘上 `data_version` **比随包新** | **不动**（装回旧版：新版数据是资产，不降级） |
/// | 来自发布/OTA（`published_at` 有值）且 `data_version >=` 随包 | 不动（云端优先） |
/// | 其余（随包铺的旧份 / 随包版本更高 / 文件坏了） | 旧份归档 + 铺随包那份 |
///
/// # 为什么不是「盘上有就不动」（2026-10-04 的老规矩）
///
/// 那条规矩在当时是对的：`release_bytes` 的"盘上不同就换"会让每次启动把用户 OTA 拿到的
/// 新目录盖回随包那份，下载随即拿旧 SHA 比云端新文件、永远 `SHA_MISMATCH`。但它同时
/// 让「程序换了、参数定义更新了」永远进不来 —— 改了注册表、重装客户端，界面还是旧的。
///
/// 现在用两条**正交**的信息代替拍脑袋：
/// - `revision`（内容指纹）回答「一不一样」，一样就一个字节不动；
/// - `data_version`（由应用版本号算出，见 [`super::catalog::app_version_code`]）回答
///   「谁更新」，只在**来自于发布/OTA 的那份不落后**时才让位；
/// - `published_at` 当**出身证明**：有值 = 这份是发布出去的（云端权威），没值 = 这份是
///   随包铺的（随包可以继续升它）。不解析时间、不读系统时钟。
///
/// 判据（每一条都钉一个分支）：`fresh_install_releases_embedded` /
/// `same_version_is_a_noop` / `bundled_newer_upgrades_and_archives` /
/// `ota_newer_is_never_overwritten` / `older_exe_keeps_newer_disk_catalog`
/// （最后一条的「格式更新 + 界面用随包渲染」在 `runtime::tests`）。
pub fn ensure_released(root: &Path) -> Result<ReleaseReport, AppError> {
    // 先验后写：随包那份也要过解析器（构造上它一定合法，这里是防"包被换过"）
    let embedded = super::Catalog::parse(EMBEDDED_CATALOG)?;
    let path = catalog_file(root);

    let Ok(old) = std::fs::read(&path) else {
        // 读不到（多半是还没有）就铺一份。真读不了（权限）时写会报出真原因
        atomic_write(&path, EMBEDDED_CATALOG)?;
        return Ok(ReleaseReport {
            written: true,
            archived: false,
        });
    };
    if old == EMBEDDED_CATALOG {
        return Ok(ReleaseReport::default());
    }

    // 盘上的那份格式比本程序新（降级装了旧版）：一个字节都别动。上层用随包那份渲染。
    if schema_of(&old).is_some_and(|v| v > super::catalog::CATALOG_SCHEMA) {
        return Ok(ReleaseReport::default());
    }

    match super::Catalog::parse(&old) {
        Ok(disk) => {
            // 内容一样（排版不同也算一样）⇒ 不动
            if disk.revision == embedded.revision {
                return Ok(ReleaseReport::default());
            }
            // **盘上那份比随包新**（装回了旧版）⇒ 不动。
            //
            // 上面那条 schema 闸只拦得住「格式都变了」那种降级；同一代格式里新版本改的
            // 是值 / 字段 / 布局，旧程序读得懂，于是会走到这里来。那份是用户已经拿到手的
            // 新版数据，把它换回随包的旧份 = 让一次降级吃掉升级成果（再装回新版时也回不去了）。
            // 旧程序照旧读得动它 —— 读不动的情况在上面（schema）与下面（坏文件）各有去处。
            if disk.data_version > embedded.data_version {
                return Ok(ReleaseReport::default());
            }
            // 发布/OTA 来的、且不比随包旧 ⇒ 云端优先，不动
            if disk.published_at.is_some() && disk.data_version >= embedded.data_version {
                return Ok(ReleaseReport::default());
            }
            let archived = archive_current(root, &old)?;
            atomic_write(&path, EMBEDDED_CATALOG)?;
            Ok(ReleaseReport {
                written: true,
                archived,
            })
        }
        // 读不懂（坏文件）：换成自己那份。坏的归档留着，排查时还看得见
        Err(_) => {
            let archived = archive_current(root, &old)?;
            atomic_write(&path, EMBEDDED_CATALOG)?;
            Ok(ReleaseReport {
                written: true,
                archived,
            })
        }
    }
}

/// 只看 `catalogSchema` 这一个数 —— 用来判断"盘上那份是不是比本程序新"。
///
/// `Catalog::parse` 对不认识的 schema 会整体报错，与"文件坏了"分不出来；降级这件事
/// 恰恰要分开：新版数据要保住，坏文件可以换掉。
fn schema_of(bytes: &[u8]) -> Option<u32> {
    let doc: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    doc.get("catalogSchema")?.as_u64().map(|v| v as u32)
}

/// 把**将被换下的**那份收进归档：`archive/catalog.json`（单槽，最早一份）+ 版本链
/// `archive/catalogs/<revision>`（名字撞了就补序号，别把上一份盖掉）。
///
/// 返回是否落了归档（与 [`ReleaseReport::archived`] 同一口径）。`release_bytes`（发布侧
/// 升级）与 [`ensure_released`]（启动升级）共用这一份实现 —— 两处各写一套归档，
/// 迟早会有一套漏掉版本链。
fn archive_current(root: &Path, old: &[u8]) -> Result<bool, AppError> {
    let archive_dir_path = archive_dir(root);
    std::fs::create_dir_all(&archive_dir_path).map_err(|e| {
        AppError::io(format!("建不出归档目录：{}", archive_dir_path.display()))
            .with_detail(e.to_string())
    })?;
    // 版本链：旧目录解析得出自己的 revision 就用它当文件名；解析不了也照收
    // （名字退化为 unknown-N），**字节比名字重要** —— 信任判定读的是链里所有文件的指纹
    let chain = catalog_chain_dir(root);
    std::fs::create_dir_all(&chain).map_err(|e| {
        AppError::io(format!("建不出版本链目录：{}", chain.display())).with_detail(e.to_string())
    })?;
    let old_revision = super::Catalog::parse(old)
        .ok()
        .map(|c| c.revision)
        .unwrap_or_default();
    let mut chain_name = chain_file_name(&old_revision);
    if chain.join(&chain_name).exists() {
        // 同名 revision 再换上来一次（同目录反复横跳）：补个序号，别把上一份盖掉
        let mut n = 1;
        loop {
            let candidate = format!("{chain_name}-{n}");
            if !chain.join(&candidate).exists() {
                chain_name = candidate;
                break;
            }
            n += 1;
        }
    }
    atomic_write(&chain.join(&chain_name), old)?;
    let archive = archive_dir_path.join("catalog.json");
    if !archive.exists() {
        atomic_write(&archive, old)?;
    }
    Ok(true)
}

/// 把**任意一份合法 catalog 字节**释放进数据根——[`release_catalog`]（随包那份）与
/// 远端更新（`apply_remote_update`，工作台发布的 `dist/catalog.json`）共用的唯一入口。
///
/// 先验后写：解析不过的字节连盘都不碰（坏目录不许替换好目录）。
/// 升级语义见模块头：盘上不同 → 旧份归档、新份生效。
pub fn release_bytes(root: &Path, bytes: &[u8]) -> Result<ReleaseReport, AppError> {
    // 先验后写的第一闸：格式不对的目录没有资格上盘
    super::Catalog::parse(bytes)?;

    let path = catalog_file(root);
    match std::fs::read(&path) {
        // 最常见的路：同一个版本再开一次，一个字节都不动
        Ok(existing) if existing == bytes => Ok(ReleaseReport::default()),
        // 盘上有、但与带来的不同：升级（或文件被手动动过）。旧份归档——单槽保留最早一份
        // （槽位已有就不覆盖）；**版本链追加**每一份被换下的旧目录（见模块头：单槽丢中间版）
        Ok(old) => {
            let archived = archive_current(root, &old)?;
            atomic_write(&path, bytes)?;
            Ok(ReleaseReport {
                written: true,
                archived,
            })
        }
        // 读不到（多半是还没有）就铺一份。真读不了（权限）时下面的写会报出真原因
        Err(_) => {
            atomic_write(&path, bytes)?;
            Ok(ReleaseReport {
                written: true,
                archived: false,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 随包那份的数据版本。**从字节里取，不硬编码 600** —— 版本号会跟着发版走
    fn embedded_version() -> u64 {
        crate::runtime::Catalog::parse(EMBEDDED_CATALOG)
            .expect("随包那份要解析得出")
            .data_version
    }

    /// 一份**合法**的 catalog 字节：拿随包那份当底，只改这三格。
    ///
    /// 手写 `{"catalogSchema":1,…}` 是不行的：`files` 没有 `#[serde(default)]`，
    /// 解析会失败 —— 于是每一处都测成了「文件坏了」那一条路，别的分支根本走不到。
    fn catalog_bytes(revision: &str, published_at: Option<&str>, data_version: u64) -> Vec<u8> {
        let mut c = crate::runtime::Catalog::parse(EMBEDDED_CATALOG).expect("随包那份要解析得出");
        c.revision = revision.to_owned();
        c.published_at = published_at.map(str::to_owned);
        c.data_version = data_version;
        serde_json::to_vec_pretty(&c).expect("目录序列化得出来")
    }

    /// 首次装：盘上没有 → 铺随包那份（不归档，没什么可归的）
    #[test]
    fn fresh_install_releases_embedded() {
        let d = tempfile::tempdir().unwrap();

        let r = ensure_released(d.path()).unwrap();

        assert!(r.written && !r.archived);
        assert_eq!(
            std::fs::read(catalog_file(d.path())).unwrap(),
            EMBEDDED_CATALOG,
            "铺下去的就是随包那份"
        );
        assert!(
            !archive_dir(d.path()).exists(),
            "首次装不该建归档目录 —— 归档说「这里换过东西」，首次没有"
        );
    }

    /// 同版本二次安装：内容指纹一样 → 一个字节不动（幂等）
    #[test]
    fn same_version_is_a_noop() {
        let d = tempfile::tempdir().unwrap();
        ensure_released(d.path()).unwrap();

        let r = ensure_released(d.path()).unwrap();

        assert!(!r.written && !r.archived);
        assert_eq!(
            std::fs::read(catalog_file(d.path())).unwrap(),
            EMBEDDED_CATALOG
        );
    }

    /// **装了新版、盘上还是随包铺的旧份** → 升级：旧份归档、盘上换成随包。
    ///
    /// 这是 2026-10-04 那条「盘上有就不动」漏掉的那一半：改了注册表、重装客户端，
    /// 界面还是旧的，只能人工删文件。
    #[test]
    fn bundled_newer_upgrades_and_archives() {
        let d = tempfile::tempdir().unwrap();
        let bundled = embedded_version();
        assert!(
            bundled > 0,
            "随包那份必须带 dataVersion（0 = 版本化之前的老文件）"
        );

        // 盘上：随包铺的旧份（无 `publishedAt` = 出身不是发布），数据版本更旧
        let old = catalog_bytes("old-bundled-rev", None, bundled - 1);
        atomic_write(catalog_file(d.path()).as_path(), &old).unwrap();

        let r = ensure_released(d.path()).unwrap();

        assert!(r.written && r.archived, "这一份该被升级");
        assert_eq!(
            std::fs::read(catalog_file(d.path())).unwrap(),
            EMBEDDED_CATALOG,
            "盘上换成随包新份"
        );
        assert_eq!(
            std::fs::read(archive_dir(d.path()).join("catalog.json")).unwrap(),
            old,
            "旧份在归档里原样躺着，可回溯"
        );
        let chain: Vec<_> = std::fs::read_dir(catalog_chain_dir(d.path()))
            .expect("版本链目录该被建出来")
            .flatten()
            .collect();
        assert_eq!(chain.len(), 1, "版本链追加了被换下的那一份");
    }

    /// ★ **启动不覆盖 OTA 成果**（2026-10-04 修的那个 bug 的钉子）。
    ///
    /// 时序：启动铺随包一份 → 用户 OTA 拿到新目录 → 再启动。第二次启动必须
    /// **一个字节都不动**，盘上留下的还是 OTA 那份 —— 不是随包那份。
    ///
    /// 换目录之后「云端那份」的判据是 `publishedAt`（出身证明）+ 版本不落后：
    /// 只有这两条一起成立才让位，光有个不同的 `revision` 会被当成「随包铺的旧份」。
    #[test]
    fn ota_newer_is_never_overwritten() {
        let d = tempfile::tempdir().unwrap();
        ensure_released(d.path()).unwrap();

        let ota = catalog_bytes("ota-rev", Some("2026-10-20T00:00:00Z"), embedded_version());
        atomic_write(catalog_file(d.path()).as_path(), &ota).unwrap();

        let r = ensure_released(d.path()).unwrap();

        assert!(!r.written, "盘上已有更新的云端目录就不该写");
        assert_eq!(
            std::fs::read(catalog_file(d.path())).unwrap(),
            ota,
            "OTA 拿到的目录必须原样留着，绝不能被随包那份换回去"
        );
    }

    /// **装回旧版**（降级）：盘上那份是更新版本铺的 → 一个字节都不动。
    ///
    /// 不需要卸载、不需要清目录：新版数据是用户已经拿到手的资产，
    /// 换回随包旧份等于让一次降级吃掉升级成果，再装回新版也回不去了。
    #[test]
    fn older_exe_keeps_newer_disk_catalog() {
        let d = tempfile::tempdir().unwrap();
        ensure_released(d.path()).unwrap();

        let newer = catalog_bytes("newer-disk-rev", None, embedded_version() + 1);
        atomic_write(catalog_file(d.path()).as_path(), &newer).unwrap();

        let r = ensure_released(d.path()).unwrap();

        assert!(!r.written && !r.archived);
        assert_eq!(
            std::fs::read(catalog_file(d.path())).unwrap(),
            newer,
            "新版数据要保住 —— 那正是降级装回时不丢东西的关键"
        );
    }

    /// 升级：盘上那份与随包不同 → 旧份进 archive/，盘上换成随包新份。
    ///
    /// **这条只属于 [`release_bytes`]（发布侧语义），不属于启动** —— 上一条判据
    /// 咬着启动那一侧的不覆盖。
    #[test]
    fn drift_archives_the_old_catalog_then_replaces() {
        let d = tempfile::tempdir().unwrap();
        ensure_released(d.path()).unwrap();

        // 模拟"上一个版本装的旧 catalog"（或被手动动过的——同一个处理）
        let old = "# 旧版本的 catalog\n".as_bytes();
        atomic_write(catalog_file(d.path()).as_path(), old).unwrap();

        let r = release_bytes(d.path(), EMBEDDED_CATALOG).unwrap();
        assert!(r.written && r.archived, "这是一次升级");
        assert_eq!(
            std::fs::read(catalog_file(d.path())).unwrap(),
            EMBEDDED_CATALOG,
            "盘上换成随包新份"
        );
        assert_eq!(
            std::fs::read(archive_dir(d.path()).join("catalog.json")).unwrap(),
            old,
            "旧份在归档里原样躺着，可回溯"
        );
    }

    /// 归档槽保留最早一份：两次升级，archive/ 里还是第一份旧 catalog
    #[test]
    fn archive_slot_keeps_the_earliest_catalog() {
        let d = tempfile::tempdir().unwrap();
        ensure_released(d.path()).unwrap();

        let older = "# 更旧的 catalog\n".as_bytes();
        atomic_write(catalog_file(d.path()).as_path(), older).unwrap();
        release_bytes(d.path(), EMBEDDED_CATALOG).unwrap(); // 第一次升级：older 进归档

        let newer = "# 次旧的 catalog\n".as_bytes();
        atomic_write(catalog_file(d.path()).as_path(), newer).unwrap();
        release_bytes(d.path(), EMBEDDED_CATALOG).unwrap(); // 第二次升级：归档槽已占，不覆盖

        assert_eq!(
            std::fs::read(archive_dir(d.path()).join("catalog.json")).unwrap(),
            older,
            "历史不被后浪抹掉"
        );
    }

    /// 铁律 3 的正面表达（2026-10-04 改口径）：释放只动 catalog ——
    /// **交付面一个文件都不预置**。
    ///
    /// 以前这条钉"`mkp/` 目录被建出来且是空的"。唯一路径语义定下之后，
    /// 交付文件的落点由 `catalog.path` 决定、没有固定的"下载区根"可以提前建 ——
    /// 所以这条改成钉**更强的**那件事：释放完，内部根下除 `catalog.json`
    /// **没有任何交付文件**（目录也一个都不多建）。
    #[test]
    fn release_places_nothing_but_the_catalog() {
        let d = tempfile::tempdir().unwrap();
        ensure_released(d.path()).unwrap();

        let mut entries: Vec<String> = std::fs::read_dir(d.path())
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        entries.sort();
        assert_eq!(
            entries,
            vec!["catalog.json".to_owned()],
            "释放只放说明书，交付面一个文件/目录都不预置"
        );
    }

    /// ★ **启动路径不许调 `release_bytes` / `release_catalog`**（源码扫描）。
    ///
    /// `startup_never_overwrites_an_ota_catalog` 钉住的是**函数的行为**；这一条钉的是
    /// **谁在用它**：`lib.rs::setup` 里那一段必须走 `ensure_released`。
    /// 两条配合才完整 —— 只有前者，将来有人在 setup 里改回 `release_bytes` 不会被拦；
    /// 只有后者，`ensure_released` 自己实现走样了也看不出来。
    ///
    /// 扫描口径：`lib.rs` 全文里，`release::` 之后的标识符只能是 `ensure_released`。
    /// `release_bytes` 出现在注释里不算（先剥掉 `//` 起头的行）。
    #[test]
    fn the_startup_path_only_ever_ensures_never_upgrades() {
        let lib = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
        let text = std::fs::read_to_string(&lib).expect("读不到 lib.rs");
        let code: String = text
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        let mut bad: Vec<String> = Vec::new();
        let mut rest = code.as_str();
        while let Some(at) = rest.find("release::") {
            let after = &rest[at + "release::".len()..];
            let ident: String = after
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if ident != "ensure_released" {
                bad.push(ident.clone());
            }
            rest = &after[ident.len().max(1)..];
        }
        assert!(
            bad.is_empty(),
            "启动路径调了 `release::{bad:?}` —— catalog 是 OTA 数据，启动只能铺空位\
             （`ensure_released`），用升级语义会把用户 OTA 拿到的目录覆盖回随包那份"
        );
    }
}
