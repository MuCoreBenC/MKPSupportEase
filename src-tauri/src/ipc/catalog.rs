//! 新数据世界的读口：客户端的运行时 catalog。
//!
//! 只读**释放进数据根的那一份**（`<appDataDir>/catalog.json`）——铁律 4：
//! 运行时只认自己的运行时数据，不直接读嵌进二进制的那份。
//! 盘上没有（setup 释放失败、或文件被删）就就地补一次再读：那是兜底，不是正常路径。
//!
//! 这条命令**零网络**（铁律 2）。它就是第一圈闭环的最后一环：
//! 页面 → 这里 → 运行时 catalog → 一条真实数据。

use tauri::AppHandle;

use crate::error::AppError;
use crate::fsx::paths::internal_root;
use crate::runtime;

use super::traced;

#[tauri::command]
pub async fn get_runtime_catalog(app: AppHandle) -> Result<runtime::Catalog, AppError> {
    traced("getRuntimeCatalog", |_| {
        let root = internal_root(&app)?;
        let path = runtime::paths::catalog_file(&root);
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => {
                runtime::release::release_catalog(&root)?;
                std::fs::read(&path).map_err(|e| {
                    AppError::io("catalog 释放之后仍然读不到").with_detail(e.to_string())
                })?
            }
        };
        runtime::Catalog::parse(&bytes)
    })
}
