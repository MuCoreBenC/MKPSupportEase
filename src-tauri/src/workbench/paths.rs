//! 工作台的数据根 —— 指向**仓库里的开发数据**，不是 `appDataDir`。
//!
//! 这是与客户端最根本的一处差别。客户端的两层数据根（`fsx::paths`）在用户机器上，
//! 因为那是用户的数据；工作台的数据在**仓库里**，因为那是开发源数据，要跟着 git 走、
//! 要能被 review、要能被别人到工作台上看明白。
//!
//! 根怎么定位：
//! 1. 环境变量 `MKPSE_REPO_DIR` —— 给"仓库不在默认位置"的情况留的口子；
//! 2. 否则取 `CARGO_MANIFEST_DIR` 的上一级。它由 cargo 在**编译时**填，填的是
//!    编译这份代码的那台机器上的仓库路径 —— 不是写死的绝对路径，换机器编译就换值。
//!
//! 这条只在 workbench feature 下存在，所以"指向仓库"这个假设成立：工作台永远是从
//! 仓库里编出来、在仓库旁边跑的开发工具，不会被装到用户机器上。
//!
//! 所有相对路径仍然过 [`crate::fsx::paths::resolve_in`] 的三道防穿越判据 —— 工作台也不该
//! 因为"反正是开发者自己用"就允许 `../../` 写到仓库外面去。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::AppError;
use crate::fsx::paths::resolve_in;

/// 开发源数据目录名（仓库根下）
const WORKBENCH_DIR: &str = "workbench";
/// 预设根目录名（仓库根下）。**唯一的预设真相源**
const PRESETS_DIR: &str = "presets";
/// 沙箱目录名（`workbench/` 底下，点号打头，与 `.draft` / `.trash` / `.snapshots` 同一族）
pub const SANDBOX_DIR: &str = ".sandbox";
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

/// 测试模式（沙箱）开着吗。
///
/// **进程内一份**：开机时从 `<appDataDir>/sandbox.json` 读进来装一次
/// （`app::sandbox::install_at_startup`），之后由那颗开关改（`wb_sandbox_set`）。
/// 做成模块级状态、而不是"每个取根的函数多收一个参数"，是因为**每一处取根的地方
/// 都该自动跟着走** —— 见 [`data_root`]：整套数据根只有那一个开关。
static SANDBOX: AtomicBool = AtomicBool::new(false);

/// 测试模式开着吗。
pub fn sandbox_on() -> bool {
    SANDBOX.load(Ordering::Relaxed)
}

/// 切测试模式。**只切模式，不动文件** —— 沙箱那棵树的建 / 清 / 拷由
/// `app::sandbox` 那几条命令负责（它们才需要 AppHandle 去写模式档）。
pub fn set_sandbox(on: bool) {
    SANDBOX.store(on, Ordering::Relaxed);
}

/// 沙箱根：`<repo>/workbench/.sandbox`。**测试模式下整套数据都住这儿**
/// —— `presets/` 的整棵副本 + 沙箱自己的 `workbench/`。
///
/// 放在 `workbench/` 里、点号打头，与 `.draft` / `.trash` / `.snapshots` 同一族：
/// 它是**本机状态**（不入库，`.gitignore` 一行挡住），删掉整个目录是安全的。
pub fn sandbox_root() -> PathBuf {
    repo_root().join(WORKBENCH_DIR).join(SANDBOX_DIR)
}

/// **会话数据根** —— 现在这一整套数据住在哪：正式 = `<repo>`；测试 = 沙箱根。
///
/// ★ 它是"整套切根"的**唯一**一处开关：`presets/`（源数据 + 交付产物 + 资产载荷）
/// 与 `workbench/`（草稿 / 台账 / 回收站）全部由它派生。生成与审计早就是从会话根
/// 派生的（`delivery_root_at(ctx.presets.root())`），所以换根对它们天然成立。
fn data_root() -> PathBuf {
    data_root_at(&repo_root(), sandbox_on())
}

/// 会话数据根的**纯派生版**（测试用：不碰那个模块级开关）。
fn data_root_at(repo: &Path, sandbox: bool) -> PathBuf {
    if sandbox {
        repo.join(WORKBENCH_DIR).join(SANDBOX_DIR)
    } else {
        repo.to_path_buf()
    }
}

/// 开发源数据根。**只建目录，不写任何配方** —— 见 doc §7
pub fn workbench_root() -> Result<PathBuf, AppError> {
    let root = data_root().join(WORKBENCH_DIR);
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
    delivery_root_at(&presets_root_path())
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
    assets_root_at(&presets_root_path())
}

/// 预设数据的根：`<repo>/presets`。**唯一的预设真相源** ——
/// 不 fallback、不猜路径、没有第二候选：运行时只认这一个 workspace 数据根。
///
/// 判据是**标志文件**而不是 `is_dir()`：一个同名空目录不该骗过定位，
/// 否则真正的失败会推迟到第一次读机型文件才暴露。
pub fn presets_root() -> Option<PathBuf> {
    let p = presets_root_path();
    p.join("registry")
        .join("param_registry.toml")
        .is_file()
        .then_some(p)
}

/// 预设根的**纯派生**（不探测）。定位探测归 [`presets_root`]；
/// 生成 / 审计 / 发布这类**拿着会话里的 `presets.root()`** 找交付根的调用方，
/// 必须走下面两个 `*_at` —— 派生只有这一处，`delivery_root_path` / `assets_root_path`
/// 也只是它的真仓库特例。
pub fn presets_root_path() -> PathBuf {
    data_root().join(PRESETS_DIR)
}

/// **正式**那份预设根（仓库里的 `presets/`），与测试模式无关。
///
/// 给沙箱那两条命令用：拷的来源永远是这一份 —— 无论当前模式是什么，
/// 都不靠"先切旗帜再读根"的时序去保证（那种写法迟早被人改坏）。
pub fn real_presets_root_path() -> PathBuf {
    repo_root().join(PRESETS_DIR)
}

/// **正式**那份配方本（仓库里的 `workbench/`），与测试模式无关。
pub fn real_workbench_root() -> PathBuf {
    repo_root().join(WORKBENCH_DIR)
}

/// 交付目录相对**预设根**的落点。生成写盘、发布定稿、审计只读
/// 三方都从这一处派生 —— 与 [`crate::workbench::domain::derive`] 的
/// `product_on_disk` 同一条规矩：根用 `presets.root()`，不许第二处自拼。
pub fn delivery_root_at(presets_root: &Path) -> PathBuf {
    presets_root.join(DELIVERY_SUBDIR)
}

/// 资产根相对**预设根**的落点（见 [`delivery_root_at`] 的同源理由）。
pub fn assets_root_at(presets_root: &Path) -> PathBuf {
    presets_root.join(ASSET_DIR)
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

    /// **测试模式把整套数据根挪到沙箱那棵树里**，正式根一个字节都不沾它
    #[test]
    fn the_sandbox_moves_the_whole_data_root() {
        let repo = Path::new("/tmp/repo");
        assert_eq!(data_root_at(repo, false), repo, "正式：数据根就是仓库根");
        assert_eq!(
            data_root_at(repo, true),
            repo.join("workbench").join(SANDBOX_DIR),
            "测试：整棵挪到 workbench/{SANDBOX_DIR} 底下"
        );
        /* 预设根由数据根派生 —— 换了它，`presets/` 跟着走 */
        assert_eq!(
            data_root_at(repo, true).join("presets"),
            repo.join("workbench").join(SANDBOX_DIR).join("presets")
        );
    }

    /// 默认（正式）时那几个根与以前**一字不差**：沙箱是"加"上去的一条路，
    /// 不是把原来那条改掉
    #[test]
    fn the_real_roots_are_untouched_while_the_sandbox_is_off() {
        assert!(!sandbox_on(), "测试进程里默认不该是测试模式");
        assert_eq!(presets_root_path(), repo_root().join("presets"));
        assert_eq!(real_presets_root_path(), repo_root().join("presets"));
        assert_eq!(real_workbench_root(), repo_root().join("workbench"));
        assert_eq!(
            sandbox_root(),
            repo_root().join("workbench").join(SANDBOX_DIR)
        );
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
