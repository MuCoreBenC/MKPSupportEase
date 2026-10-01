//! 释放口：把安装包里嵌着的 catalog 铺进内部根（层② → 层③）。
//!
//! # 释放策略（第二圈起：这就是"升级"本身）
//!
//! - 盘上没有 → 铺一份（首次启动）；
//! - 盘上与随包**逐字节一致** → 不动（最常见：同一个版本再开一次）；
//! - 盘上与随包**不同** → 旧份归档进 `archive/catalog.json`，换上随包新份。
//!
//! 第三条是**程序管理的说明书**的应有之义：catalog 的唯一主人是程序（总纲 §3），
//! 用户没有"自己那份 catalog"一说——它与旧世界铺源 TOML 时的"只报不覆盖"刻意分手，
//! 那条路随旧世界退役了。软件升级带来新 catalog 时，旧份归档、新份生效，这就是更新
//! 流程本身；归档槽保留**最早**一份，可回溯、不静默丢。换 catalog 之后文件层面的
//! 旧版本怎么办，是 [`super::delivery`] 的事（Stale → 重走管道 → 旧文件归档）。
//!
//! # 顺带建出空的 `mkp/`
//!
//! 下载区初始为空是**判据**（铁律 3），不是巧合：建出目录是为了自解释，
//! 一个文件都不放。

use std::path::Path;

use crate::error::AppError;
use crate::fsx::atomic::atomic_write;

use super::paths::{archive_dir, catalog_file, mkp_dir};
use super::EMBEDDED_CATALOG;

#[derive(Debug, Default)]
pub struct ReleaseReport {
    /// 这次真写下去的（首次启动、文件被删、或随包带来了新版本）
    pub written: bool,
    /// 盘上的旧份被归档了（= 这是一次升级）
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

pub fn release_catalog(root: &Path) -> Result<ReleaseReport, AppError> {
    release_bytes(root, EMBEDDED_CATALOG)
}

/// 把**任意一份合法 catalog 字节**释放进数据根——[`release_catalog`]（随包那份）与
/// 远端更新（`apply_remote_update`，工作台发布的 `dist/catalog.json`）共用的唯一入口。
///
/// 先验后写：解析不过的字节连盘都不碰（坏目录不许替换好目录）。
/// 升级语义见模块头：盘上不同 → 旧份归档、新份生效。
pub fn release_bytes(root: &Path, bytes: &[u8]) -> Result<ReleaseReport, AppError> {
    // 先验后写的第一闸：格式不对的目录没有资格上盘
    super::Catalog::parse(bytes)?;

    std::fs::create_dir_all(mkp_dir(root)).map_err(|e| {
        AppError::io(format!("建不出下载区：{}", mkp_dir(root).display()))
            .with_detail(e.to_string())
    })?;

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
        let r = release_catalog(d.path()).unwrap();
        assert!(r.written && !r.archived);
        assert_eq!(
            std::fs::read(catalog_file(d.path())).unwrap(),
            EMBEDDED_CATALOG,
            "铺下去的就是随包那份"
        );

        // 已在且一致：一个字节不动
        let r = release_catalog(d.path()).unwrap();
        assert!(!r.written && !r.archived);
    }

    /// 升级：盘上那份与随包不同 → 旧份进 archive/，盘上换成随包新份
    #[test]
    fn drift_archives_the_old_catalog_then_replaces() {
        let d = tempfile::tempdir().unwrap();
        release_catalog(d.path()).unwrap();

        // 模拟"上一个版本装的旧 catalog"（或被手动动过的——同一个处理）
        let old = "# 旧版本的 catalog\n".as_bytes();
        atomic_write(catalog_file(d.path()).as_path(), old).unwrap();

        let r = release_catalog(d.path()).unwrap();
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
        release_catalog(d.path()).unwrap();

        let older = "# 更旧的 catalog\n".as_bytes();
        atomic_write(catalog_file(d.path()).as_path(), older).unwrap();
        release_catalog(d.path()).unwrap(); // 第一次升级：older 进归档

        let newer = "# 次旧的 catalog\n".as_bytes();
        atomic_write(catalog_file(d.path()).as_path(), newer).unwrap();
        release_catalog(d.path()).unwrap(); // 第二次升级：归档槽已占，不覆盖

        assert_eq!(
            std::fs::read(archive_dir(d.path()).join("catalog.json")).unwrap(),
            older,
            "历史不被后浪抹掉"
        );
    }

    /// 铁律 3 的正面表达：释放只动 catalog 和目录，下载区里一个文件都没有
    #[test]
    fn mkp_dir_is_created_empty() {
        let d = tempfile::tempdir().unwrap();
        release_catalog(d.path()).unwrap();
        let mkp = mkp_dir(d.path());
        assert!(mkp.is_dir(), "下载区该被建出来");
        assert_eq!(
            std::fs::read_dir(&mkp).unwrap().count(),
            0,
            "下载区初始必须是空的"
        );
    }
}
