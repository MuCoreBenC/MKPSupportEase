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
/// 用户根下首次启动就建齐的子目录
const USER_DIRS: [&str; 3] = ["exports", "reports", MINE_DIR];

/// 用户根在**应用数据目录**下的那一个子目录（`<appDataDir>/user`）。
///
/// ★ 它与内部根**并排**，不在内部根里再套一层 `user/…` 之外的东西 ——
/// 两个根都是 `appDataDir` 的一级子树，谁也不会误把对方的文件当自己的。
const USER_ROOT_DIR: &str = "user";

pub fn internal_root(app: &AppHandle) -> Result<PathBuf, AppError> {
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
