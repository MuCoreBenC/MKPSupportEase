//! 释放口：把安装包里嵌着的 catalog 铺进内部根（层② → 层③）。
//!
//! # 两个动作，两条路（2026-10-04 拆开 —— 混用会造成"启动回退 OTA"）
//!
//! catalog 的生命周期是 **bootstrap → OTA → 当前**。随包那份只是**起点**，
//! 不是最终资源（铁律 4：运行时只认自己的运行时数据）。所以写盘分两个动作：
//!
//! | 函数 | 谁调 | 语义 |
//! |---|---|---|
//! | [`ensure_released`] | 启动（`lib.rs::setup`）、读不到时的兜底 | **只铺底**：盘上没有才写，有就一个字节不动 |
//! | [`release_bytes`] | [`super::update`] 的应用侧（`apply_remote_update`） | **升级**：盘上不同 → 旧份归档、新份生效 |
//!
//! ★ 启动**绝不能**走 [`release_bytes`]：那会让每次启动都用随包那份覆盖掉
//! 用户 OTA 拿到的新目录，下载随即 SHA 不匹配。详见 [`ensure_released`] 的注释。
//!
//! 归档只发生在 [`release_bytes`] 里：槽位保留**最早**一份，可回溯、不静默丢。
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

/// 随包那份 catalog —— 只做**铺底**，绝不覆盖盘上已有的那份。
///
/// # 为什么它不能是 [`release_bytes`]（2026-10-04 修的）
///
/// `release_bytes` 是"升级"语义（盘上不同就换成带来这份），那是发布侧
/// （`apply_remote_update`）要的。启动用同一套语义会让每次启动把用户 OTA 拿到的
/// 新目录**覆盖回随包那份**，下载随即拿旧 SHA 比云端新文件、永远 `SHA_MISMATCH`
/// （实测就是这个链路，看起来像"文件坏了"）。判据 `startup_never_overwrites_an_ota_catalog`。
///
/// 所以启动只问：**盘上有没有**。没有就铺（首次启动 / 文件被删）；有就一个字节不动 ——
/// 那一份无论来自随包还是 OTA 都是"运行时自己的数据"（铁律 4）。
pub fn ensure_released(root: &Path) -> Result<ReleaseReport, AppError> {
    let path = catalog_file(root);
    if path.exists() {
        return Ok(ReleaseReport::default());
    }
    // 先验后写：随包那份也要过解析器（构造上它一定合法，这里是防"包被换过"）
    super::Catalog::parse(EMBEDDED_CATALOG)?;
    atomic_write(&path, EMBEDDED_CATALOG)?;
    Ok(ReleaseReport {
        written: true,
        archived: false,
    })
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
        // 盘上有、但与带来的不同：升级（或文件被手动动过）。旧份归档——归档槽保留最早一份，
        // 槽位已有就不覆盖；然后换上新份
        Ok(old) => {
            let archive = archive_dir(root).join("catalog.json");
            std::fs::create_dir_all(archive.parent().expect("归档路径必有父目录")).map_err(
                |e| {
                    AppError::io(format!("建不出归档目录：{}", archive.display()))
                        .with_detail(e.to_string())
                },
            )?;
            let archived = if archive.exists() {
                true
            } else {
                atomic_write(&archive, &old)?;
                true
            };
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

    #[test]
    fn releases_then_keeps_same_version() {
        let d = tempfile::tempdir().unwrap();

        // 首次：写下去
        let r = ensure_released(d.path()).unwrap();
        assert!(r.written && !r.archived);
        assert_eq!(
            std::fs::read(catalog_file(d.path())).unwrap(),
            EMBEDDED_CATALOG,
            "铺下去的就是随包那份"
        );

        // 已在且一致：一个字节不动
        let r = ensure_released(d.path()).unwrap();
        assert!(!r.written && !r.archived);
    }

    /// ★ **启动不覆盖 OTA 成果**（2026-10-04 修的那个 bug 的钉子）。
    ///
    /// 时序：启动铺随包一份 → 用户 OTA 拿到新目录 → 再启动。第二次启动必须
    /// **一个字节都不动**，盘上留下的还是 OTA 那份 —— 不是随包那份。
    ///
    /// 以前 setup 走 `release_catalog`（升级语义），这一步会把 OTA 目录换回随包，
    /// 于是下载永远拿着旧 SHA 去比云端新文件、永远 SHA 不匹配。
    #[test]
    fn startup_never_overwrites_an_ota_catalog() {
        let d = tempfile::tempdir().unwrap();
        ensure_released(d.path()).unwrap();

        // 用户在预设页 OTA 成功：盘上换成远端那份（与随包逐字节不同）
        let ota = "{\"catalogSchema\":1,\"revision\":\"ota-rev\"}".as_bytes();
        atomic_write(catalog_file(d.path()).as_path(), ota).unwrap();

        // 再启动一次
        let r = ensure_released(d.path()).unwrap();

        assert!(!r.written, "盘上已有就不该写");
        assert_eq!(
            std::fs::read(catalog_file(d.path())).unwrap(),
            ota,
            "OTA 拿到的目录必须原样留着，绝不能被随包那份换回去"
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
