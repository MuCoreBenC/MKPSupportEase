//! 两层数据根 + 防穿越。
//!
//! **为什么分两层**：程序自己管的东西（云端原件、归档、索引、日志、运行状态）放 `appDataDir`；
//! 用户自己要看要拷的东西（预设副本、导出、报告）放 Documents 下的 `SupportEase/`。
//! 不把前者也塞进 Documents 的原因很具体：macOS 开了「桌面与文档」iCloud 同步后，
//! Documents 里的文件会被驱逐成占位 stub —— 读出来内容不对，会把 Preset 的 SHA 失效判定
//! 变成误报。程序管理的数据不能放在一个会被系统悄悄搬走的地方。
//!
//! **防穿越**：所有相对路径都必须经过 [`resolve`]。拒绝绝对路径、拒绝 `..`、拼完还要再确认
//! 结果仍在根内（符号链接能绕过前两条，所以第三条是必须的）。

use std::path::{Component, Path, PathBuf};

use tauri::{AppHandle, Manager};

use crate::error::AppError;

/// 两个数据根。`Root` 而不是直接传 `&Path`：调用点写的是"内部"还是"用户"，
/// 而不是一个能被随手替换成任意目录的路径。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Root {
    /// `appDataDir` —— 程序管理，用户不该手动进去改
    Internal,
    /// `~/Documents/SupportEase` —— 给用户看的
    User,
}

/// 内部根下首次启动就建齐的子目录
const INTERNAL_DIRS: [&str; 5] = ["cloud", "archive", "index", "logs", "run"];
/// 用户根下首次启动就建齐的子目录
const USER_DIRS: [&str; 3] = ["exports", "reports", "presets-mine"];

/// 用户可见目录的名字。与窗口标题一致 —— 这个目录是给用户看的，就该叫产品名
const USER_DIR_NAME: &str = "SupportEase";

pub fn internal_root(app: &AppHandle) -> Result<PathBuf, AppError> {
    let root = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::io("找不到应用数据目录").with_detail(e.to_string()))?;
    ensure_dirs(&root, &INTERNAL_DIRS)?;
    Ok(root)
}

pub fn user_root(app: &AppHandle) -> Result<PathBuf, AppError> {
    let root = app
        .path()
        .document_dir()
        .map_err(|e| AppError::io("找不到「文档」目录").with_detail(e.to_string()))?
        .join(USER_DIR_NAME);
    ensure_dirs(&root, &USER_DIRS)?;
    Ok(root)
}

pub fn root_path(app: &AppHandle, root: Root) -> Result<PathBuf, AppError> {
    match root {
        Root::Internal => internal_root(app),
        Root::User => user_root(app),
    }
}

/// 把相对路径解析成根内的绝对路径。越界一律 `PERMISSION_DENIED`
pub fn resolve(app: &AppHandle, root: Root, rel: &str) -> Result<PathBuf, AppError> {
    resolve_in(&root_path(app, root)?, rel)
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

    let joined = root.join(rel_path);

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
        let p = resolve_in(d.path(), "index/offsets.json").unwrap();
        assert!(p.starts_with(d.path()));
        assert!(p.ends_with("index/offsets.json"));
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

    /// 目标文件还不存在是正常情况（第一次写），不能判成越界
    #[test]
    fn allows_not_yet_existing_file() {
        let d = root();
        assert!(resolve_in(d.path(), "a/b/c/new.json").is_ok());
    }
}
