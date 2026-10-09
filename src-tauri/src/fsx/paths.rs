//! 两层数据根 + 防穿越。
//!
//! **为什么分两层**：程序自己管的东西（云端原件、归档、索引、日志、运行状态）放内部根；
//! 用户自己要看要拷的东西（预设副本、导出、报告）放**用户根**。
//!
//! ★ **两个根都住 `appDataDir`（作者 2026-10-05 拍：用户根从 `~/Documents/SupportEase`
//! 搬进程序自己的文件夹）**。两个理由，都是真机踩出来的：
//! 1. macOS 的 `~/Documents` 受 TCC 保护 —— 程序第一次进去会弹「要访问你的文稿文件夹」，
//!    而这个程序**根本不需要用户的 Documents**（2026-10-05：用户点开预设页就被问了一次）；
//! 2. 开了「桌面与文档」iCloud 同步后，Documents 里的文件会被驱逐成占位 stub ——
//!    读出来内容不对，会把 Preset 的 SHA 失效判定变成误报。
//!
//! 用户根住在 `<appDataDir>/user`：它**仍然是独立的一层**（路径语义、防穿越都不变），
//! 只是**落点**从系统目录换成程序目录。用户想拿自己的预设，去「在 Finder 中显示」。
//!
//! **防穿越**：所有相对路径都必须经过 [`resolve_in`]。拒绝绝对路径、拒绝 `..`、拼完还要再确认
//! 结果仍在根内（符号链接能绕过前两条，所以第三条是必须的）。

use std::path::{Component, Path, PathBuf};

use tauri::{AppHandle, Manager};

use crate::error::AppError;

/// 内部根下首次启动就建齐的子目录
///
/// `index/` 2026-10-08 退役：它唯一住过的东西是 `offsets.json`（校准偏移的孤岛，
/// 只写不读）—— 校准值改随**用户工作副本**走之后这个目录没有消费者了。
/// 盘上遗留的空目录不清理（用户自己删）。
const INTERNAL_DIRS: [&str; 4] = ["cloud", "archive", "logs", "run"];
/// **用户自己的预设**住的那个子目录。`pub`：用户线那两条读（`runtime::mine`）要认它，
/// 字面量只许有这一处
pub const MINE_DIR: &str = "presets-mine";

/// 后处理执行归档的那棵树：`<用户根>/gcode_history`。
///
/// 目录形状照成熟版（`mkpsupporte` 的 `gcode_history`）：
/// `<日期>/<名>_<时刻>{_original.gcode, .gcode, _meta.json}` 三件套。
/// 落点住在**本应用自己的用户根**（`appDataDir/user`）—— 不碰 `~/Documents`
/// （理由见模块头）。`pub`：归档（`crate::archive`）与报告页读口
/// （`crate::ipc::report`）都要认它，字面量只许有这一处。
pub const GCODE_HISTORY_DIR: &str = "gcode_history";

/// 用户根下首次启动就建齐的子目录
const USER_DIRS: [&str; 4] = ["exports", "reports", MINE_DIR, GCODE_HISTORY_DIR];

/// 用户根在**应用数据目录**下的那一个子目录（`<appDataDir>/user`）。
///
/// ★ 它与内部根**并排**，不在内部根里再套一层 `user/…` 之外的东西 ——
/// 两个根都是 `appDataDir` 的一级子树，谁也不会误把对方的文件当自己的。
const USER_ROOT_DIR: &str = "user";

/// 应用标识符 —— **与 `tauri.conf.json` 里那个 `identifier` 必须一字不差**
/// （判据 `the_identifier_matches_tauri_conf` 钉着；改配置不改这里，那条判据立刻红）。
///
/// 为什么要有这个常量：**钩子进程不是 Tauri 应用**（它没有 `AppHandle`，只有"干活 + 退出码"），
/// 而它要算内部根来读端点文件、写执行记录 —— 只能靠这个 + 平台规则自己算。
/// 在运行时解析 `tauri.conf.json` 也是一条路，但那样"两处会漂"就没有东西拦着。
pub const APP_IDENTIFIER: &str = "SupportEase";

/// 数据根的环境变量开关（整个盖掉算出来的那个）—— 探针与判据用，**界面的两条入口也认它**，
/// 这样"钩子"与"界面"在同一棵树上看同一份端点文件。
pub const DATA_ROOT_ENV: &str = "MKP_SUPPORT_EASE_DATA";

/// 钩子进程那侧的内部根（没有 `AppHandle` 时用）。
///
/// 平台规则与 Tauri 的 `app_data_dir()` 对齐（Windows `%APPDATA%\<标识符>`、
/// macOS `~/Library/Application Support/<标识符>`、Linux `$XDG_DATA_HOME/<标识符>`）。
/// 算不出来 / 建不出来 ⇒ `None`：钩子那侧**没有界面也不影响干活**，只是这次没人看进度。
pub fn internal_root_headless() -> Option<PathBuf> {
    if let Some(over) = std::env::var_os(DATA_ROOT_ENV) {
        let root = PathBuf::from(over);
        ensure_dirs(&root, &INTERNAL_DIRS).ok()?;
        return Some(root);
    }
    let base = if cfg!(windows) {
        PathBuf::from(std::env::var_os("APPDATA")?)
    } else if cfg!(target_os = "macos") {
        PathBuf::from(std::env::var_os("HOME")?).join("Library/Application Support")
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share"))
    };
    let root = base.join(APP_IDENTIFIER);
    ensure_dirs(&root, &INTERNAL_DIRS).ok()?;
    Some(root)
}

pub fn internal_root(app: &AppHandle) -> Result<PathBuf, AppError> {
    /* 探针开关：整个盖掉数据根（钩子那侧认同一个变量 —— 见 `internal_root_headless`） */
    if let Some(over) = std::env::var_os(DATA_ROOT_ENV) {
        let root = PathBuf::from(over);
        ensure_dirs(&root, &INTERNAL_DIRS)?;
        return Ok(root);
    }
    let root = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::io("找不到应用数据目录").with_detail(e.to_string()))?;
    ensure_dirs(&root, &INTERNAL_DIRS)?;
    Ok(root)
}

/// 用户根：`<appDataDir>/user`。
///
/// ★ **不碰 `~/Documents`** —— 那个目录在 macOS 上受 TCC 保护（一进去就弹「要访问你的
/// 文稿文件夹」），而这里放的是程序替用户存的文件，不是用户自己摆在文稿里的东西。
pub fn user_root(app: &AppHandle) -> Result<PathBuf, AppError> {
    let root = internal_root(app)?.join(USER_ROOT_DIR);
    ensure_dirs(&root, &USER_DIRS)?;
    Ok(root)
}

/// 用户根（**钩子进程**那条路：没有 `AppHandle`，只有平台规则 + 环境变量覆盖）。
///
/// 与 [`user_root`] 同一个落点、同一套子目录；算不出来 / 建不出来 ⇒ `None` ——
/// 与 [`internal_root_headless`] 同一个理由：归档不该让后处理跑不起来
/// （调用方把 `None` 降级成一条警告，见 `crate::archive` 的接入点）。
pub fn user_root_headless() -> Option<PathBuf> {
    let root = internal_root_headless()?.join(USER_ROOT_DIR);
    ensure_dirs(&root, &USER_DIRS).ok()?;
    Some(root)
}

fn ensure_dirs(root: &Path, subs: &[&str]) -> Result<(), AppError> {
    for sub in subs {
        std::fs::create_dir_all(root.join(sub)).map_err(|e| {
            AppError::io(format!("建不出数据目录：{}", root.join(sub).display()))
                .with_detail(e.to_string())
        })?;
    }
    Ok(())
}

/// 纯函数版本的解析，便于单测（不需要 AppHandle）。
///
/// 三道判据，缺一道都能被绕过：
/// 1. 拒绝绝对路径 —— `/etc/passwd` 直接过掉拼接；
/// 2. 拒绝 `..` 与其他非普通分量 —— `a/../../x`；
/// 3. 拼完之后**用规范化后的真实路径**再确认仍在根内 —— 符号链接指向根外时前两条都看不出来。
pub fn resolve_in(root: &Path, rel: &str) -> Result<PathBuf, AppError> {
    check_relative(rel)?;

    let joined = root.join(Path::new(rel));

    /* 第三道：比真实路径。
    只有存在的那部分能 canonicalize，所以拿"最深的已存在祖先"来比 ——
    目标文件还不存在（第一次写）是正常情况，不能因此判越界。 */
    let anchor = deepest_existing(&joined);
    let real_anchor = anchor
        .canonicalize()
        .map_err(|e| AppError::io("路径解析失败").with_detail(e.to_string()))?;
    let real_root = root
        .canonicalize()
        .map_err(|e| AppError::io("数据目录不可用").with_detail(e.to_string()))?;

    if !real_anchor.starts_with(&real_root) {
        return Err(
            AppError::permission_denied("路径不能越过数据目录").with_detail(format!(
                "{} 实际指向 {}",
                joined.display(),
                real_anchor.display()
            )),
        );
    }

    Ok(joined)
}

/// 前两道闸，**不需要根**：非空、非绝对、无 `..` 与其他非普通分量。
///
/// [`resolve_in`] 在它之上再加第三道（比真实路径）。单独拎出来是给"加载期校验"
/// 用的场合：那时还没有数据根（比如客户端这一轮只释放定义、不释放资产文件本体），
/// 而"路径不能是 `../x`"这条判据与根在不在无关。
pub fn check_relative(rel: &str) -> Result<(), AppError> {
    let rel_path = Path::new(rel);

    if rel.is_empty() {
        return Err(AppError::invalid_argument("路径不能为空"));
    }

    if rel_path.is_absolute() {
        return Err(AppError::permission_denied("只接受相对路径")
            .with_detail(format!("拒绝绝对路径：{rel}")));
    }

    for c in rel_path.components() {
        match c {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir => {
                return Err(AppError::permission_denied("路径不能越过数据目录")
                    .with_detail(format!("含 .. 分量：{rel}")))
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(AppError::permission_denied("只接受相对路径")
                    .with_detail(format!("含根分量：{rel}")))
            }
        }
    }

    Ok(())
}

/// 从 path 往上找第一个真实存在的祖先
fn deepest_existing(path: &Path) -> PathBuf {
    let mut cur = path;
    loop {
        if cur.exists() {
            return cur.to_path_buf();
        }
        match cur.parent() {
            Some(p) => cur = p,
            None => return cur.to_path_buf(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn accepts_plain_relative_path() {
        let d = root();
        let p = resolve_in(d.path(), "run/app-state.json").unwrap();
        assert!(p.starts_with(d.path()));
        assert!(p.ends_with("run/app-state.json"));
    }

    /// [`APP_IDENTIFIER`] 与 `tauri.conf.json` 里的 `identifier` **不许漂**。
    ///
    /// 两边算的是同一棵树（`<appDataDir>/…`）：钩子进程靠常量自己算，界面那侧靠 Tauri 的
    /// `app_data_dir()`。漂了的表现是"钩子把端点文件写到一棵树上、界面在另一棵树上等"，
    /// 而那种失配**没有别的判据能咬住**（两边各自都对）。
    #[test]
    fn the_identifier_matches_tauri_conf() {
        let conf = include_str!("../../tauri.conf.json");
        let parsed: serde_json::Value = serde_json::from_str(conf).expect("tauri.conf.json 该是 JSON");
        let declared = parsed["identifier"]
            .as_str()
            .expect("tauri.conf.json 里该有 identifier");
        assert_eq!(
            declared, APP_IDENTIFIER,
            "两处标识符不许漂：配置里是 {declared}，常量是 {APP_IDENTIFIER}"
        );
    }

    /// 钩子那侧的内部根：探针开关（[`DATA_ROOT_ENV`]）能整个盖掉它，且四个子目录建齐
    #[test]
    fn the_headless_root_can_be_overridden() {
        let d = root();
        /* 环境变量是进程级的，这个用例自己设自己收（同一条测试进程里别的地方不读它） */
        unsafe { std::env::set_var(DATA_ROOT_ENV, d.path()) };
        let got = internal_root_headless().expect("给了开关就该算得出根");
        unsafe { std::env::remove_var(DATA_ROOT_ENV) };
        assert_eq!(got, d.path());
        for sub in INTERNAL_DIRS {
            assert!(d.path().join(sub).is_dir(), "{sub} 该建出来");
        }
    }

    #[test]
    fn rejects_parent_dir_escape() {
        let d = root();
        let e = resolve_in(d.path(), "../../etc/passwd").unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::PermissionDenied);
    }

    #[test]
    fn rejects_absolute_path() {
        let d = root();
        let e = resolve_in(d.path(), "/etc/passwd").unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::PermissionDenied);
    }

    #[test]
    fn rejects_empty_path() {
        let d = root();
        let e = resolve_in(d.path(), "").unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::InvalidArgument);
    }

    /// 符号链接：前两道判据都看不出来，只有"比真实路径"这一道拦得住
    #[cfg(unix)]
    #[test]
    fn rejects_symlink_pointing_outside() {
        let d = root();
        let outside = root();
        std::os::unix::fs::symlink(outside.path(), d.path().join("escape")).unwrap();

        let e = resolve_in(d.path(), "escape/secret.txt").unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::PermissionDenied);
    }

    /// ★ **用户根不许碰 `~/Documents`**（作者 2026-10-05 拍）。
    ///
    /// 真机代价很具体：macOS 上第一次进 Documents 会弹「要访问你的文稿文件夹」，
    /// 而这个程序根本不需要用户的文稿目录 —— 用户点开预设页就被问了一次。
    /// 这条是**源码扫描**：把系统的那个文档目录 API 写回来，它当场报红。
    ///
    /// ★ 扫**调用**（带左括号）而不是名字：只扫名字的话，这条注释自己就会把它撞红。
    #[test]
    fn the_user_root_never_touches_documents() {
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/fsx/paths.rs"))
            .expect("读得到本文件");
        // 拼出来再比：**直接写那个名字的话，这一行自己就在被扫的文本里**（自撞）。
        let needle = format!("document{}(", "_dir");
        assert!(
            !src.contains(&needle),
            "用户根不许用系统的文档目录 —— 一碰它就要弹系统授权框"
        );
        assert!(
            src.contains("USER_ROOT_DIR"),
            "用户根应该落在 appDataDir 下一个具名子目录里"
        );
    }

    /// 目标文件还不存在是正常情况（第一次写），不能判成越界
    #[test]
    fn allows_not_yet_existing_file() {
        let d = root();
        assert!(resolve_in(d.path(), "a/b/c/new.json").is_ok());
    }
}
