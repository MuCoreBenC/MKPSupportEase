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
/// 发布目录名（仓库根下）。刻意不叫 `dist` —— 那个名字被 vite 的前端产物占着，
/// 两者混在一起会让"清一下产物"这种操作顺手删掉交付资源。
const DIST_DIR: &str = "dist-presets";

/// 开发源数据根下首次启动就建齐的子目录
const WORKBENCH_DIRS: [&str; 6] = [
    "machines",
    "bbs",
    "capability",
    ".draft",
    ".trash",
    ".snapshots",
];

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

/// 发布目录。发布动作才会往里写，读状态时不需要它存在
pub fn dist_root() -> Result<PathBuf, AppError> {
    let root = repo_root().join(DIST_DIR);
    std::fs::create_dir_all(&root).map_err(|e| {
        AppError::io(format!("建不出发布目录：{}", root.display())).with_detail(e.to_string())
    })?;
    Ok(root)
}

/// 把相对路径解析到开发源数据根内，越界一律 `PERMISSION_DENIED`
pub fn resolve(rel: &str) -> Result<PathBuf, AppError> {
    resolve_in(&workbench_root()?, rel)
}

/// 把相对路径解析到发布目录内
pub fn resolve_dist(rel: &str) -> Result<PathBuf, AppError> {
    resolve_in(&dist_root()?, rel)
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
