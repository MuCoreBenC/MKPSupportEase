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
/// 资产根的名字（`public/` 下）。见 [`assets_root`]
const ASSET_DIR: &str = "assets";
/// 上游预设仓库的目录名
const UPSTREAM_DIR: &str = "mkpse-presets";

/// 开发源数据根下首次启动就建齐的子目录。**这份清单是唯一的** ——
/// `store::bootstrap` 引用它而不是再抄一遍（两处写死同一份清单迟早漂移）。
///
/// `capability` 已随第三版去掉 —— 第二版那份能力定义是我自己按 registry 手写的，
/// 校验的其实是"配方有没有超出我以为客户端支持的范围"（doc §12），不是真兼容性。
///
/// `bbs/` 也去了（b05 Task 14.7 裁决，2026-09-24）：全仓核实工作台对它**零读写**
/// —— BBS 资产的职责在资产域（`public/assets/bbs/` + `presets/assets.toml` 条目），
/// 交付子树是 `dist-presets/assets/bbs/`。bootstrap 不再为一个没有职责的目录占位。
/// 剩下四个各司其职：`machines/` 存值的草稿（身份清单来自 `presets/machines/*.toml`）、
/// `.draft/` 是会话草稿（gitignore）、`.trash/` 是版本与残留回收站、
/// `.snapshots/` 存生成快照（`wb_generate` 写，`wb_revert_preview` 读 ——
/// 「恢复配方」靠它，不是只写不读）。
pub const WORKBENCH_DIRS: [&str; 4] = ["machines", ".draft", ".trash", ".snapshots"];

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

/// 资产文件的根：`<repo>/public/assets/`（b05 Task 8 定的约定）。
///
/// # 为什么在 `public/` 下面
///
/// 这些文件要能被前端**按 URL 取**（机型图、图标、模型），而 vite 的 `public/` 是唯一
/// "原样进产物、按路径直通"的目录 —— 前端拿到的是 `/assets/printers/a1.webp`。
///
/// 取 `public/` 下的一个子根而不是 `public/` 本身：那里还有 hero 图之类的**界面素材**，
/// 两类东西混在一层，`path` 就说不清"这条资产属于谁管"。约定是「我们管的资产全在
/// `public/assets/` 里，别的 `public/` 文件不许被 `presets/assets.toml` 引用」。
///
/// # 目录按需建
///
/// 与 [`workbench_root`] / [`dist_root`] 同一口径。而且这里**必须**存在：防穿越
/// （[`resolve_in`]）的第三道要比真实路径，根不存在的话每条路径都会解析失败 ——
/// "还没搬过资产"要落成一个真实存在的空目录（`public/assets/.gitkeep` 占着），
/// 而不是一个查不出来的状态。
pub fn assets_root() -> Result<PathBuf, AppError> {
    let root = repo_root().join("public").join(ASSET_DIR);
    std::fs::create_dir_all(&root).map_err(|e| {
        AppError::io(format!("建不出资产目录：{}", root.display())).with_detail(e.to_string())
    })?;
    Ok(root)
}

/* ---------- 上游预设仓库（只读） ---------- */

/// 上游 `mkpse-presets`，**只读**。
///
/// 三级定位，第一个命中的就用：
/// 1. `MKPSE_PRESETS_DIR` 环境变量 —— 仓库不在默认位置时的口子
/// 2. `<repo>/../mkpse-next-v3/mkpse-presets` —— 当前工作区的实际布局
/// 3. `<repo>/../mkpse-presets` —— 上游那边解除 submodule 后的推荐布局
///
/// **返回 `None` 是一个有意义的状态，不是错误值**：这时工作台不该启动业务，而该显示
/// "找不到 mkpse-presets，试过哪几个位置"（doc §15 第一条）。用空数据装成能跑，
/// 会让人以为上游是空的 —— 那和"读不出来"是两件完全不同的事。
///
/// 判据是**标志文件**而不是 `is_dir()`：只看目录存在的话，一个同名空目录就能骗过定位，
/// 而真正的失败会推迟到第一次读 `param_registry.json` 才暴露（报错指向受害者而非真因）。
pub fn upstream_root() -> Option<PathBuf> {
    fn marked(p: PathBuf) -> Option<PathBuf> {
        if p.join("content").join("param_registry.json").is_file() {
            Some(p)
        } else {
            None
        }
    }

    if let Some(dir) = std::env::var_os("MKPSE_PRESETS_DIR") {
        // 显式指定的路径不做兜底：指错了要当场看见，不能悄悄退回猜的那两个
        return marked(PathBuf::from(dir));
    }

    let parent = repo_root().parent().map(Path::to_path_buf)?;
    marked(parent.join("mkpse-next-v3").join(UPSTREAM_DIR))
        .or_else(|| marked(parent.join(UPSTREAM_DIR)))
}

/// 定位失败时给人看的候选路径。错误信息里要写出**试过哪里** ——
/// 只说"找不到"没法据以行动
pub fn upstream_candidates() -> Vec<String> {
    if let Some(dir) = std::env::var_os("MKPSE_PRESETS_DIR") {
        return vec![format!(
            "MKPSE_PRESETS_DIR={}",
            PathBuf::from(dir).display()
        )];
    }
    match repo_root().parent() {
        Some(parent) => vec![
            parent
                .join("mkpse-next-v3")
                .join(UPSTREAM_DIR)
                .display()
                .to_string(),
            parent.join(UPSTREAM_DIR).display().to_string(),
        ],
        None => Vec::new(),
    }
}

/// 我们自己那份预设数据的根：`<repo>/presets`。
///
/// **它和 [`upstream_root`] 是两件不同的事，别合并**：那个指向 `mkpse-presets`
/// （历史上的上游，现在只是我们搬数据的来源）；这个是**我们自己的**数据，
/// 工作台读它也写它。搬完之后只有这一份是真相。
///
/// 判据同样是**标志文件**而不是 `is_dir()`：一个同名空目录不该骗过定位，
/// 否则真正的失败会推迟到第一次读机型文件才暴露。
pub fn presets_root() -> Option<PathBuf> {
    let p = repo_root().join("presets");
    p.join("registry")
        .join("param_registry.toml")
        .is_file()
        .then_some(p)
}

/// 解析上游仓库内的相对路径。**只读用**，越界一律 `PERMISSION_DENIED`
pub fn resolve_upstream(rel: &str) -> Result<PathBuf, AppError> {
    let root = upstream_root().ok_or_else(|| {
        AppError::not_found("找不到上游预设仓库 mkpse-presets")
            .with_detail(format!("试过：{}", upstream_candidates().join("；")))
    })?;
    resolve_in(&root, rel)
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

    /// 上游定位要么给出一个**真带标志文件**的目录，要么给 `None`。
    ///
    /// 刻意**不断言**"一定找得到" —— 那会把测试绑在这台机器的工作区布局上，
    /// 换台机器或在 CI 上就变成一条与代码无关的红。这里断言的是**不变式**：
    /// 返回 Some 时那个目录必须真的能读到 `param_registry.json`。
    #[test]
    fn upstream_root_is_either_marked_or_none() {
        if let Some(p) = upstream_root() {
            assert!(
                p.join("content").join("param_registry.json").is_file(),
                "定位到了 {} 但里面没有 content/param_registry.json",
                p.display()
            );
        }
    }

    /// 定位失败时必须说得出试过哪里 —— 空清单等于只说"找不到"
    #[test]
    fn upstream_candidates_are_actionable() {
        let c = upstream_candidates();
        assert!(!c.is_empty(), "候选路径清单是空的，错误信息将无法据以行动");
        assert!(
            c.iter().all(|s| !s.trim().is_empty()),
            "候选里有空串：{c:?}"
        );
    }

    /// 找不到上游时，`resolve_upstream` 必须是 `NOT_FOUND` 且 detail 里带候选路径；
    /// 找得到时则要能解析出仓库内的路径。两种环境下这条都成立
    #[test]
    fn resolve_upstream_reports_where_it_looked() {
        match resolve_upstream("content/param_registry.json") {
            Ok(p) => assert!(p.is_file(), "解析出来的路径不是文件：{}", p.display()),
            Err(e) => {
                assert_eq!(e.code, crate::error::ErrorCode::NotFound);
                let detail = e.detail.unwrap_or_default();
                assert!(detail.contains("试过"), "错误里没写试过哪里：{detail}");
            }
        }
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
