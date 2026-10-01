//! **新数据世界**（见 `docs/DATA-ARCHITECTURE.md`）。
//!
//! 总纲把所有文件划进四层：开发仓库 → 安装包 → 用户本地 → 云端。这一模块管的是
//! **第③层（用户本地）的运行时形态**与**第②层 → 第③层的释放口**：
//!
//! ```text
//! 安装包（二进制里嵌着的 catalog.generated.json）
//!     │  首启释放（release）
//!     ▼
//! <appDataDir>/catalog.json     说明书 —— 首屏唯一数据源（铁律 2：启动零网络）
//! <appDataDir>/mkp/             下载区 —— 初始为空，用户下载了什么才有什么（铁律 3）
//! ```
//!
//! 客户端首屏（`ipc::presets` 的九条读命令）从 catalog 的 definition 出数；
//! 旧世界（include_str! 13 份源 TOML 铺进 `presets/` 的 `client/` 模块）已随换源退役
//! —— 开发文件不再成为运行时数据库（铁律 1 收口，总纲欠账 #1）。
//!
//! 名字里没有版本号（作者裁决：新系统不背旧命名的包袱）——目录格式由
//! [`catalog::CATALOG_SCHEMA`] 表达，进化靠加字段，不靠改名。

pub mod catalog;
pub mod delivery;
/// **血统三行**（用户那份是从哪份官方、哪一版拷出来改的）—— 写在文件头注释里
pub mod lineage;
/// **用户线**：用户自己的预设（`presets-mine/`）—— 与官方线（`mkp/` / `archive/`）分开
pub mod mine;
pub mod net;
pub mod paths;
pub mod release;
pub mod source;
pub mod state;
pub mod update;

pub use catalog::Catalog;

/// 读**释放进内部根的那份** catalog 的原始字节。盘上没有（setup 释放失败、或文件被删）
/// 就就地补一次再读：那是兜底，不是正常路径。
///
/// 给"按字节缓存"的消费者用（`ipc::presets` 的九条读命令）：同一份字节只 parse 一次，
/// 目录换新（升级 / 应用远端更新）后字节变，缓存自动失效——**不需要失效钩子**。
pub fn load_released_catalog_bytes(
    root: &std::path::Path,
) -> Result<Vec<u8>, crate::error::AppError> {
    let path = paths::catalog_file(root);
    match std::fs::read(&path) {
        Ok(bytes) => Ok(bytes),
        Err(_) => {
            release::release_catalog(root)?;
            std::fs::read(&path).map_err(|e| {
                crate::error::AppError::io("catalog 释放之后仍然读不到").with_detail(e.to_string())
            })
        }
    }
}

/// 读**释放进内部根的那份** catalog（铁律 4：运行时只认自己的运行时数据）。
/// 命令层（`ipc::catalog`）与将来的写路径共用这一条入口，读法只有这一份。
pub fn load_released_catalog(root: &std::path::Path) -> Result<Catalog, crate::error::AppError> {
    Catalog::parse(&load_released_catalog_bytes(root)?)
}

/// 随安装包走的那份 catalog —— 发布构建（`cargo run --bin gen-catalog`）的产物。
///
/// 编译期钉进二进制，运行时与仓库无关（用户机器上没有仓库）。它是不是最新的，
/// 由 [`catalog::tests::embedded_matches_rebuild`] 那条判据守着：
/// 重新构建的结果与这份**逐字节一致**，否则测试红 ——
/// 「没有未经审阅的变化」在构造上成立，与对照基线同一思路。
pub const EMBEDDED_CATALOG: &[u8] = include_bytes!("catalog.generated.json");

#[cfg(test)]
mod tests {
    use super::*;

    /// 嵌进二进制的那份必须是当前源 + 交付产物能重新构建出来的那一份。
    /// 改了源忘了跑 `gen-catalog`，这条就红 —— 它是层① → 层② 的失联报警。
    #[test]
    fn embedded_matches_rebuild() {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let repo = manifest.parent().expect("src-tauri 上面就是仓库根");
        let catalog = catalog::Catalog::build_from_repo(repo).expect("目录构建不出来");
        let json = catalog.to_pretty_json().expect("序列化失败");
        assert_eq!(
            json.as_bytes(),
            EMBEDDED_CATALOG,
            "仓库里的 catalog.generated.json 与重新构建的不一致 —— 跑一遍 `cargo run --bin gen-catalog`"
        );
    }

    /// 嵌入的那份本身得能读：schema 认得、机型与文件非空。
    /// （它坏了对用户的表现是第一屏空 —— 那种错在真机上最费劲，在这里先拦住。）
    #[test]
    fn embedded_parses_and_is_not_empty() {
        let catalog = Catalog::parse(EMBEDDED_CATALOG).expect("嵌进二进制的 catalog 读不出来");
        assert_eq!(catalog.catalog_schema, catalog::CATALOG_SCHEMA);
        assert!(!catalog.machines.is_empty(), "机型清单不该是空的");
        assert!(!catalog.files.is_empty(), "文件清单不该是空的");
        assert_eq!(
            catalog.files[0].kind, "mkp_preset",
            "第一圈的清单只有 MKP 预设一种"
        );
    }
}
