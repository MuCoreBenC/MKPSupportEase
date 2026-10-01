//! 释放口：把安装包里嵌着的 catalog 铺进内部根（层② → 层③）。
//!
//! # 只补缺失
//!
//! 已有的一份一个字节都不动；内容与嵌入的那份不同就**只报不覆盖** ——
//! 它要么被用户改过、要么是上一个版本装的。替用户决定哪份该赢是不负责任的，
//! 覆盖策略等真正的升级流程（第二圈：更新 / 归档）一起定。这与旧世界
//! [`crate::client::paths::seed_if_absent`] 的政策一字不差，是有意保持一致。
//!
//! # 顺带建出空的 `mkp/`
//!
//! 下载区初始为空是**判据**（铁律 3），不是巧合：建出目录是为了自解释，
//! 一个文件都不放。

use std::path::Path;

use crate::error::AppError;
use crate::fsx::atomic::atomic_write;

use super::paths::{catalog_file, mkp_dir};
use super::EMBEDDED_CATALOG;

#[derive(Debug, Default)]
pub struct ReleaseReport {
    /// 这次真写下去的（首次启动，或文件被删了）
    pub written: bool,
    /// 已经在了、但内容与嵌入的那份不同。只报，不动盘
    pub drifted: bool,
}

impl ReleaseReport {
    /// 一行给日志看的话
    pub fn summary(&self) -> String {
        match (self.written, self.drifted) {
            (true, _) => "catalog 已释放".to_owned(),
            (false, true) => "catalog 保留盘上那份（与随包的不同，只报不改）".to_owned(),
            (false, false) => "catalog 已是最新".to_owned(),
        }
    }
}

pub fn release_catalog(root: &Path) -> Result<ReleaseReport, AppError> {
    std::fs::create_dir_all(mkp_dir(root)).map_err(|e| {
        AppError::io(format!("建不出下载区：{}", mkp_dir(root).display())).with_detail(e.to_string())
    })?;

    let path = catalog_file(root);
    match std::fs::read(&path) {
        Ok(existing) => Ok(ReleaseReport {
            written: false,
            drifted: existing != EMBEDDED_CATALOG,
        }),
        // 读不到（多半是还没有）就铺一份。真读不了（权限）时下面的写会报出真原因
        Err(_) => {
            atomic_write(&path, EMBEDDED_CATALOG)?;
            Ok(ReleaseReport {
                written: true,
                drifted: false,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn releases_then_keeps_then_reports_drift() {
        let d = tempfile::tempdir().unwrap();

        // 首次：写下去
        let r = release_catalog(d.path()).unwrap();
        assert!(r.written && !r.drifted);
        assert_eq!(
            std::fs::read(catalog_file(d.path())).unwrap(),
            EMBEDDED_CATALOG,
            "铺下去的就是随包那份"
        );

        // 已在：一个字节不动
        let r = release_catalog(d.path()).unwrap();
        assert!(!r.written && !r.drifted);

        // 盘上的被改过：只报，不覆盖（字节串字面量装不下中文，用 as_bytes）
        let drifted = "# 用户改过的\n".as_bytes();
        std::fs::write(catalog_file(d.path()), drifted).unwrap();
        let r = release_catalog(d.path()).unwrap();
        assert!(!r.written && r.drifted, "不同要说出来");
        assert_eq!(
            std::fs::read(catalog_file(d.path())).unwrap(),
            drifted,
            "已有的一份一个字节都不许动"
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
