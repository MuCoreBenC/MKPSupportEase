//! 新数据世界的落点 —— 内部根下的三样东西。
//!
//! 路径只在这里定一次。业务要问"下载的文件放哪"，答案永远是
//! [`paths::mkp_dir`]，不是自己再拼一个目录名（总纲 §1③：给③新增子目录，
//! 必须先改文档再写代码）。
//!
//! 全是纯函数（拿 `&Path` 不拿 `AppHandle`）：释放逻辑与判据测试都不需要真的应用。

use std::path::{Path, PathBuf};

/// 运行时说明书的文件名。相对**内部根**（`appDataDir`）
pub const CATALOG_FILE: &str = "catalog.json";

/// 下载区的目录名。相对**内部根**。初始为空 —— 用户下载了什么才有什么
pub const MKP_DIR: &str = "mkp";

/// 运行时说明书：`<appDataDir>/catalog.json`
pub fn catalog_file(root: &Path) -> PathBuf {
    root.join(CATALOG_FILE)
}

/// 下载区：`<appDataDir>/mkp`
pub fn mkp_dir(root: &Path) -> PathBuf {
    root.join(MKP_DIR)
}
