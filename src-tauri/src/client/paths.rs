//! 正式客户端的数据根 —— 指向 `appDataDir`，**不是仓库**。
//!
//! 工作台那一侧（`workbench::paths`）读的是 `<repo>/presets`，因为那是开发源数据，
//! 要跟着 git 走。客户端这一侧读的是**用户机器上的正式数据**，用户装完根本没有仓库。
//! 两边**永不互相读写**，即使跑在同一台开发机上。
//!
//! 根的位置：`<appDataDir>/presets` —— 与仓库 `presets/` **同构**（同样的 `machines/`、
//! `assets.toml`、`bundles.toml`、`registry/`），因为喂给它的解析代码是同一份
//! [`crate::presetdata::Presets::load_from`]。
//!
//! # 第一次启动铺什么
//!
//! 定义类文件来自**编译期打进二进制**的内置默认（[`super::defaults`]），见
//! [`seed_if_absent`]。**只铺缺失的** —— 已有的一份一个字节都不动。

use std::path::{Path, PathBuf};

use tauri::AppHandle;

use crate::error::AppError;
use crate::fsx::atomic::atomic_write;
use crate::fsx::paths::internal_root;

use super::defaults::FILES;

/// 预设数据在程序数据根下的目录名
const PRESETS_DIR: &str = "presets";
/// 交付产物区（MKP 预设本体）的子目录名。**初始为空** —— 用户从云端下载或自己导入
const MKP_DIR: &str = "mkp";

/// 预设数据根：`<appDataDir>/presets`。喂给 [`crate::presetdata::Presets::load_from`] 的就是它。
pub fn presets_root(app: &AppHandle) -> Result<PathBuf, AppError> {
    Ok(internal_root(app)?.join(PRESETS_DIR))
}

/// 用户产物区：`<appDataDir>/presets/mkp`。下载 / 导入的 MKP 预设落这里
pub fn mkp_dir(root: &Path) -> PathBuf {
    root.join(MKP_DIR)
}

/// 一次释放的结果。给启动日志用
#[derive(Debug, Default)]
pub struct SeedReport {
    /// 这次真铺下去的（相对路径）
    pub written: Vec<&'static str>,
    /// 已经在了、有内容也与内置默认一致的
    pub kept: Vec<&'static str>,
    /// 已经在了、但内容与内置默认**不同**的。**只报不覆盖**（见 [`seed_if_absent`]）
    pub drifted: Vec<&'static str>,
}

impl SeedReport {
    /// 一行给日志看的话
    pub fn summary(&self) -> String {
        let mut s = format!(
            "铺了 {} 份，保留 {} 份",
            self.written.len(),
            self.kept.len()
        );
        if !self.drifted.is_empty() {
            s.push_str(&format!(
                "；{} 份与内置默认不同：{}",
                self.drifted.len(),
                self.drifted.join("、")
            ));
        }
        s
    }
}

/// 把内置默认的定义类文件铺进预设根。**只铺缺失的**。
///
/// # 覆盖策略：本轮不覆盖
///
/// 已有的一份**保持原样**，只比对内容（"指纹"）：不同就记进 [`SeedReport::drifted`]。
/// 这说明这份定义要么被用户改过、要么是上一次构建留下的旧版本 —— 两种都可能，
/// 而在没有真正升级需求之前，**替用户决定哪一份该赢是不负责任的**。
/// 覆盖策略等云端发布带上版本号那一轮再收紧。
///
/// # 顺带建出空的 `mkp/`
///
/// 那是用户产物区，**一个文件都不放**。建出目录是为了让数据根自解释：
/// 用户打开它能看到"我的预设放这儿"。
pub fn seed_if_absent(root: &Path) -> Result<SeedReport, AppError> {
    let mut report = SeedReport::default();
    for (rel, text) in FILES {
        let path = root.join(rel);
        match std::fs::read(&path) {
            Ok(existing) if existing == text.as_bytes() => report.kept.push(rel),
            // 在了但内容不同：只记一笔，不动盘
            Ok(_) => report.drifted.push(rel),
            // 读不到（多半是还没有）就铺一份。真读不了（权限）时下面的写会报出真原因
            Err(_) => {
                atomic_write(&path, text.as_bytes())?;
                report.written.push(rel);
            }
        }
    }
    std::fs::create_dir_all(mkp_dir(root)).map_err(|e| {
        AppError::io(format!("建不出预设产物目录：{}", mkp_dir(root).display()))
            .with_detail(e.to_string())
    })?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeds_every_default_file() {
        let d = tempfile::tempdir().unwrap();
        let report = seed_if_absent(d.path()).unwrap();

        assert!(report.drifted.is_empty(), "空目录里不该有「不同」的东西");
        assert_eq!(report.written.len(), FILES.len(), "每一份都该铺下去");
        for (rel, text) in FILES {
            let got = std::fs::read_to_string(d.path().join(rel))
                .unwrap_or_else(|e| panic!("{rel} 没铺出来：{e}"));
            assert_eq!(got, *text, "{rel} 铺出来的内容与内置默认不一致");
        }

        // 用户产物区：目录在，文件一个都没有
        let mkp = mkp_dir(d.path());
        assert!(mkp.is_dir(), "mkp/ 该被建出来");
        assert_eq!(
            std::fs::read_dir(&mkp).unwrap().count(),
            0,
            "mkp/ 初始必须是空的"
        );
    }

    /// 这条盯的是**内置清单与解析层期望的结构一致**。
    /// 少一份定义，客户端第一屏就是空的（或直接加载失败）—— 而那种错在真机上查起来最费劲
    #[test]
    fn seeding_yields_a_loadable_presets_tree() {
        let d = tempfile::tempdir().unwrap();
        seed_if_absent(d.path()).unwrap();

        let p = crate::presetdata::Presets::load_from(d.path()).expect("铺完该读得起来");
        assert_eq!(p.catalog.machines().len(), 5, "5 台机型");
        assert_eq!(p.bundles.items().len(), 5, "5 份套餐");
        assert!(!p.registry.params().is_empty(), "字段定义该读得到");
    }

    #[test]
    fn never_overwrites_an_existing_file() {
        let d = tempfile::tempdir().unwrap();
        let rel = "brands.toml";
        let mine = "# 我自己改过的\n";
        atomic_write(&d.path().join(rel), mine.as_bytes()).unwrap();

        let report = seed_if_absent(d.path()).unwrap();

        assert_eq!(
            std::fs::read_to_string(d.path().join(rel)).unwrap(),
            mine,
            "已有的定义文件一个字节都不许动"
        );
        assert!(report.drifted.contains(&rel), "与内置默认不同要说出来");
        assert!(!report.written.contains(&rel), "又写了一遍就是覆盖了");
        // 其余的照铺
        assert_eq!(report.written.len(), FILES.len() - 1);
    }
}
