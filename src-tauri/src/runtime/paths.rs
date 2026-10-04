//! 新数据世界的落点 —— 内部根下的几样东西。
//!
//! # ★ 交付文件的落点不再在这里（2026-10-04 作者裁决 C/甲）
//!
//! 交付文件（下载来的资产 / 渲染产物）的落点由 **`catalog.path` 自己说**：
//!
//! ```text
//! 本地落点 = <appDataDir>/<catalog.path>
//! ```
//!
//! 也就是说**一个字段同时管"云端取哪"和"本地放哪"**，客户端不再按 kind
//! 猜目录（旧的 `mkp/<kind>/…` 那套是第二套路由规则，已废）。
//! 想算某个交付文件的落点，用 [`super::catalog`] 那一侧的值直接 `root.join(path)`，
//! **不要**在这里另立一个函数。
//!
//! 这里只剩"与 catalog 无关的内部固定物"：说明书、归档根、程序自己的状态文件。
//!
//! 全是纯函数（拿 `&Path` 不拿 `AppHandle`）：释放逻辑与判据测试都不需要真的应用。

use std::path::{Path, PathBuf};

/// 运行时说明书的文件名。相对**内部根**（`appDataDir`）
pub const CATALOG_FILE: &str = "catalog.json";

/// 归档区的目录名。相对**内部根**。换版本时旧份进这里，不删（总纲十问 #9）
pub const ARCHIVE_DIR: &str = "archive";

/// 数据源设置的文件名。相对**内部根**，与使用中指针同一个目录：
/// 两者都是"程序自己产生的持久状态"，储一处、写法一套（见 `state.rs` 的规则）
pub const SOURCE_FILE: &str = "run/preset-source.json";

/// 运行时说明书：`<appDataDir>/catalog.json`
pub fn catalog_file(root: &Path) -> PathBuf {
    root.join(CATALOG_FILE)
}

/// 归档区：`<appDataDir>/archive`。
///
/// **内部结构与交付面同形**：`archive/<catalog.path>`（例如
/// `archive/dist/mkp/presets/A1-fast.toml`、`archive/assets/bbs/…/x.json`）。
/// 同形是刻意的 —— 归档里这份是谁，去掉前缀就是答案。
pub fn archive_dir(root: &Path) -> PathBuf {
    root.join(ARCHIVE_DIR)
}

/// 交付文件的本地落点：`<appDataDir>/<catalog.path>`。
///
/// 这是"唯一路径语义"在本地这一侧的**唯一实现处**。云端那一侧由
/// `join_url(baseUrl, path)` 实现，两者用**同一个 `path` 值**。
pub fn released_file(root: &Path, catalog_path: &str) -> PathBuf {
    root.join(catalog_path)
}
