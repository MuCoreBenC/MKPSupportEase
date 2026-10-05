//! 工作台的数据根 —— 指向**仓库里的开发数据**，不是 `appDataDir`。
//!
//! 这是与客户端最根本的一处差别。客户端的两层数据根（`fsx::paths`）在用户机器上，
//! 因为那是用户的数据；工作台的数据在**仓库里**，因为那是开发源数据，要跟着 git 走、
//! 要能被 review、要能被别人来"后厨参观"。
//!
//! 根怎么定位：
//! 1. 环境变量 `MKPSE_REPO_DIR` —— 给"仓库不在默认位置"的情况留的口子；
//! 2. 否则取 `CARGO_MANIFEST_DIR` 的上一级。它由 cargo 在**编译时**填，填的是
//!    编译这份代码的那台机器上的仓库路径 —— 不是写死的绝对路径，换机器编译就换值。
//!
//! 这条只在 workbench feature 下存在，所以"指向仓库"这个假设成立：工作台永远是从
//! 仓库里编出来、在仓库旁边跑的开发工具，不会被装到用户机器上。
//!
//! 所有相对路径仍然过 [`crate::fsx::paths::resolve_in`] 的三道防穿越判据 —— 后厨也不该
//! 因为"反正是开发者自己用"就允许 `../../` 写到仓库外面去。

use std::path::{Path, PathBuf};

use crate::error::AppError;
use crate::fsx::paths::resolve_in;

/// 开发源数据目录名（仓库根下）
const WORKBENCH_DIR: &str = "workbench";
/// 预设根目录名（仓库根下）。**唯一的预设真相源**
const PRESETS_DIR: &str = "presets";
/// 交付目录的子目录名（`presets/` 下）。人维护 `presets/*.toml`，机器生成 `presets/delivery/*`：
/// 源与交付各占一层，一眼分得清哪个是手写的（`RESOURCE-ADDRESSING-ROADMAP.md` §2.1）。
pub(crate) const DELIVERY_SUBDIR: &str = "delivery";
/// 交付根下的**产品资源区**：与客户端 `catalog.path` 里的那一段同名同形
/// （B 类是 `delivery/mkp/presets/…`，见 `app::delivery` 模块头）。
/// 住在 paths 是因为**读侧也要用**（生成状态兜底要 stat 磁盘上的产物），不能只让写侧认得。
pub const MKP_DIR: &str = "mkp";
/// MKP 产物在交付根里的子目录（**相对交付根**：`catalog.path` = `delivery/` + 这一格）
pub const MKP_PRESETS_DIR: &str = "mkp/presets";
/// 资产根的名字（`presets/` 下）。见 [`assets_root`]
const ASSET_DIR: &str = "assets";

/// 开发源数据根下首次启动就建齐的子目录。**这份清单是唯一的** ——
/// `store::bootstrap` 引用它而不是再抄一遍（两处写死同一份清单迟早漂移）。
///
/// `capability` 已随第三版去掉 —— 第二版那份能力定义是我自己按 registry 手写的，
/// 校验的其实是"配方有没有超出我以为客户端支持的范围"（doc §12），不是真兼容性。
pub const WORKBENCH_DIRS: [&str; 5] = ["machines", "bbs", ".draft", ".trash", ".snapshots"];

/// 仓库根。见本模块文档的两级回退
pub fn repo_root() -> PathBuf {
    if let Some(dir) = std::env::var_os("MKPSE_REPO_DIR") {
        return PathBuf::from(dir);
    }
    // CARGO_MANIFEST_DIR = <repo>/src-tauri
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// 开发源数据根。**只建目录，不写任何配方** —— 见 doc §7
pub fn workbench_root() -> Result<PathBuf, AppError> {
    let root = repo_root().join(WORKBENCH_DIR);
    ensure_dirs(&root, &WORKBENCH_DIRS)?;
    Ok(root)
}

/// 交付目录：`<repo>/presets/delivery`。发布动作才会往里写，读状态时不需要它存在。
/// **生成器的唯一出口**（`RESOURCE-ADDRESSING-ROADMAP.md` 铁律 ②）。
pub fn delivery_root() -> Result<PathBuf, AppError> {
    let root = delivery_root_path();
    std::fs::create_dir_all(&root).map_err(|e| {
        AppError::io(format!("建不出发布目录：{}", root.display())).with_detail(e.to_string())
    })?;
    Ok(root)
}

/// 交付根的**只读**定位（不建目录）：读状态用 —— 只有生成 / 发布才需要它存在。
/// 路径与 [`delivery_root`] 同一处算出，不许第二处自拼。
pub fn delivery_root_path() -> PathBuf {
    repo_root().join(PRESETS_DIR).join(DELIVERY_SUBDIR)
}

/// 把相对路径解析到开发源数据根内，越界一律 `PERMISSION_DENIED`
pub fn resolve(rel: &str) -> Result<PathBuf, AppError> {
    resolve_in(&workbench_root()?, rel)
}

/// 把相对路径解析到交付目录内
pub fn resolve_delivery(rel: &str) -> Result<PathBuf, AppError> {
    resolve_in(&delivery_root()?, rel)
}

/// 资产文件的根：`<repo>/public/assets/`（b05 Task 8 定的约定）。
///
/// # 为什么在 `public/` 下面
///
/// 这些文件要能被前端**按 URL 取**（图标、模型、BBS 配置），而 vite 的 `public/` 是唯一
/// "原样进产物、按路径直通"的目录 —— 前端拿到的是 `/assets/icons/a1.svg` 这种。
///
/// 取 `public/` 下的一个子根而不是 `public/` 本身：`public/` 里还有 BBS 页元数据那类
/// **界面数据**，两类东西混在一层，`path` 就说不清"这条资产属于谁管"。约定是「我们管的资产全在
/// `public/assets/` 里，别的 `public/` 文件不许被 `presets/assets.toml` 引用」。
///
/// 2026-10-03 从 `public/assets` 搬到 `presets/assets`（作者：「在 assets 吧，到时候 3mf
/// 也要放」—— 产品数据资源住一起）：现在这个根下是台账管的全部分类
/// `bbs/` `icons/` `models/` `printers/`。
/// **`printers/`（整机图）是 `bundled` 档**：台账登记、工作台可管，但**不进云端交付**
/// —— 到客户端靠构建期复制（`scripts/copy-assets.mjs`）。
///
/// # 目录按需建
///
/// 与 [`workbench_root`] / [`delivery_root`] 同一口径。而且这里**必须**存在：防穿越
/// （[`resolve_in`]）的第三道要比真实路径，根不存在的话每条路径都会解析失败 ——
/// "还没搬过资产"要落成一个真实存在的空目录（`public/assets/.gitkeep` 占着），
/// 而不是一个查不出来的状态。
pub fn assets_root() -> Result<PathBuf, AppError> {
    let root = assets_root_path();
    std::fs::create_dir_all(&root).map_err(|e| {
        AppError::io(format!("建不出资产目录：{}", root.display())).with_detail(e.to_string())
    })?;
    Ok(root)
}

/// 资产根的**只读**定位（不建目录）—— 判据与闸用它。
///
/// 与 [`delivery_root_path`] 同一条理由：**读一件事不该顺手造出一个目录**。
/// 发布闸明写「只读：不写盘」，所以它必须走这一条而不是 [`assets_root`]。
/// 路径与 [`assets_root`] 同一处算出，不许第二处自拼。
pub fn assets_root_path() -> PathBuf {
    repo_root().join(PRESETS_DIR).join(ASSET_DIR)
}

/// 预设数据的根：`<repo>/presets`。**唯一的预设真相源** ——
/// 不 fallback、不猜路径、没有第二候选：运行时只认这一个 workspace 数据根。
///
/// 判据是**标志文件**而不是 `is_dir()`：一个同名空目录不该骗过定位，
/// 否则真正的失败会推迟到第一次读机型文件才暴露。
pub fn presets_root() -> Option<PathBuf> {
    let p = repo_root().join(PRESETS_DIR);
    p.join("registry")
        .join("param_registry.toml")
        .is_file()
        .then_some(p)
}

fn ensure_dirs(root: &Path, subs: &[&str]) -> Result<(), AppError> {
    for sub in subs {
        let dir = root.join(sub);
        std::fs::create_dir_all(&dir).map_err(|e| {
            AppError::io(format!("建不出工作台目录：{}", dir.display())).with_detail(e.to_string())
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 仓库根下必须有 package.json —— 如果这条不成立，说明 `CARGO_MANIFEST_DIR`
    /// 的上一级不是仓库根，后面所有路径都会指错地方
    #[test]
    fn repo_root_points_at_the_repository() {
        assert!(
            repo_root().join("package.json").is_file(),
            "仓库根定位错了：{}",
            repo_root().display()
        );
    }

    #[test]
    fn ensures_subdirs_without_writing_files() {
        let d = tempfile::tempdir().unwrap();
        ensure_dirs(d.path(), &WORKBENCH_DIRS).unwrap();
        for sub in WORKBENCH_DIRS {
            assert!(d.path().join(sub).is_dir(), "{sub} 没建出来");
        }
        // 只建目录，一个文件都不该写
        let files: Vec<_> = walk_files(d.path());
        assert!(files.is_empty(), "不该写文件，却写了：{files:?}");
    }

    #[test]
    fn rejects_escaping_relative_path() {
        let d = tempfile::tempdir().unwrap();
        let e = resolve_in(d.path(), "../outside.json").unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::PermissionDenied);
    }

    /// **唯一预设根**：定位得到时那个目录必须真能读到 `registry/param_registry.toml`。
    ///
    /// 刻意**不断言**"一定找得到" —— 那会把测试绑在这台机器的工作区布局上。
    /// 这里断言的是**不变式**：返回 Some 时标志文件必须在。
    #[test]
    fn presets_root_is_either_marked_or_none() {
        if let Some(p) = presets_root() {
            assert!(
                p.join("registry").join("param_registry.toml").is_file(),
                "定位到了 {} 但里面没有 registry/param_registry.toml",
                p.display()
            );
        }
    }

    /// 交付目录落在**预设根里面**：`<repo>/presets/delivery`。
    /// 源与产物同一棵树，人维护的 `presets/*.toml` 与机器生成的 `presets/delivery/*` 一眼分得开
    #[test]
    fn delivery_lives_under_presets() {
        let d = delivery_root().unwrap();
        assert_eq!(
            d.file_name().and_then(|s| s.to_str()),
            Some("delivery"),
            "交付目录该是 presets/delivery，实测 {}",
            d.display()
        );
        assert_eq!(
            d.parent()
                .and_then(|p| p.file_name())
                .and_then(|s| s.to_str()),
            Some("presets"),
            "发布目录的上一级该是 presets，实测 {}",
            d.display()
        );
    }

    fn walk_files(dir: &Path) -> Vec<PathBuf> {
        let mut out = Vec::new();
        let Ok(entries) = std::fs::read_dir(dir) else {
            return out;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(walk_files(&p));
            } else {
                out.push(p);
            }
        }
        out
    }
}
