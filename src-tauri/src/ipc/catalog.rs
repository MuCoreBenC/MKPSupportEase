//! 新数据世界的读口：客户端的运行时 catalog 与下载管道的命令面。
//!
//! 读走 [`runtime::load_released_catalog`]：只读**释放进数据根的那一份**
//! （`<appDataDir>/catalog.json`）——铁律 4：运行时只认自己的运行时数据。
//!
//! 三条命令**首屏零网络**（铁律 2）；下载那一条也不在启动路径上，
//! 而且第一圈只有开发源——真云端（第二圈）来了换 [`runtime::delivery::Source`]
//! 的实现，命令与管道都不动。

use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::Digest;
use tauri::AppHandle;

use crate::error::AppError;
use crate::fsx::paths::internal_root;
use crate::runtime;

use super::traced;

#[tauri::command]
pub async fn get_runtime_catalog(app: AppHandle) -> Result<runtime::Catalog, AppError> {
    traced("getRuntimeCatalog", |_| {
        let root = internal_root(&app)?;
        runtime::load_released_catalog(&root)
    })
}

/// 第一圈的开发源：入库产物目录（`crates/preset/assets/presets`）。
///
/// 编译期钉的是**开发机的绝对路径**，运行时只做一件事：探测"这台机器有没有这个源"。
/// 开发机上它存在，管道就能端到端走通；用户机器上不存在，命令诚实说还没接。
/// 这条路径探测随第二圈真源上线一起退场——它不是架构的一部分，是脚手架。
fn dev_source_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri 上面就是仓库根")
        .join("crates/preset/assets/presets")
}

/// 把 catalog 里登记的一份文件拉进下载区（`mkp/`）。
///
/// 第一圈的口径：一次一份（`downloadFiles` 那个批量口子还留给旧世界）。
/// 字节对不上 SHA 就整个拒绝——`mkp/` 里不会出现坏文件。
#[tauri::command]
pub async fn download_runtime_file(app: AppHandle, file_name: String) -> Result<String, AppError> {
    traced("downloadRuntimeFile", |_| {
        let root = internal_root(&app)?;
        let catalog = runtime::load_released_catalog(&root)?;
        let file = catalog
            .files
            .iter()
            .find(|f| f.file_name == file_name)
            .ok_or_else(|| AppError::not_found(format!("目录里没有 {file_name}")))?;

        let dir = dev_source_dir();
        if !dir.is_dir() {
            return Err(AppError::not_implemented(
                "下载还没接：真云端在第二圈，这台机器上也没有开发源",
            ));
        }
        let target =
            runtime::delivery::deliver(&root, file, &runtime::delivery::LocalDirSource::new(&dir))?;
        Ok(target
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| file_name.clone()))
    })
}

/// 已经下载到下载区的文件名（盘就是底账：文件在且 SHA 对得上才算数）
#[tauri::command]
pub async fn get_downloaded_files(app: AppHandle) -> Result<Vec<String>, AppError> {
    traced("getDownloadedFiles", |_| {
        let root = internal_root(&app)?;
        let catalog = runtime::load_released_catalog(&root)?;
        Ok(runtime::delivery::downloaded_files(&root, &catalog))
    })
}

/// 有更新的文件名（盘上在、字节与目录不一样）。"更新"就是对这些再跑一遍下载——
/// 旧份自动归档，没有单独的更新代码路径
#[tauri::command]
pub async fn get_stale_files(app: AppHandle) -> Result<Vec<String>, AppError> {
    traced("getStaleFiles", |_| {
        let root = internal_root(&app)?;
        let catalog = runtime::load_released_catalog(&root)?;
        Ok(runtime::delivery::stale_files(&root, &catalog))
    })
}

/* ---------- 使用中指针（第一圈 ⑤：用户状态的第一个真数据） ---------- */

/// 给界面的使用中状态：指针 + 从目录反查出来的机型/版本 + 文件是否还是当时那份
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivePresetDto {
    pub file_name: String,
    pub sha256: String,
    /// 目录里已经没有这份时是空串（目录更新了、状态还在——过渡期的诚实表达）
    pub machine_id: String,
    pub version_id: String,
    /// 盘上的文件还是不是应用时刻的那份（`mkp/` 是只读区，正常恒 true）
    pub intact: bool,
}

fn active_dto(
    root: &Path,
    catalog: &runtime::Catalog,
    state: runtime::state::ActivePreset,
) -> ActivePresetDto {
    let listed = catalog
        .files
        .iter()
        .find(|f| f.file_name == state.file_name);
    ActivePresetDto {
        machine_id: listed.map(|f| f.machine_id.clone()).unwrap_or_default(),
        version_id: listed.map(|f| f.version_id.clone()).unwrap_or_default(),
        intact: runtime::state::active_matches_disk(root, &state),
        file_name: state.file_name,
        sha256: state.sha256,
    }
}

/// 当前使用的是哪一份。`null` = 还没用任何一份（合法状态，不是错误）
#[tauri::command]
pub async fn get_active_preset(app: AppHandle) -> Result<Option<ActivePresetDto>, AppError> {
    traced("getActivePreset", |_| {
        let root = internal_root(&app)?;
        let catalog = runtime::load_released_catalog(&root)?;
        match runtime::state::load_active(&root)? {
            None => Ok(None),
            Some(state) => Ok(Some(active_dto(&root, &catalog, state))),
        }
    })
}

/// 「使用这一份」：把目录里登记的某份下载文件记成使用中。全局唯一——
/// 产品规则定死了同一时刻只能有一份处于已应用状态，构造上就是"一个文件"。
#[tauri::command]
pub async fn apply_active_preset(
    app: AppHandle,
    file_name: String,
) -> Result<ActivePresetDto, AppError> {
    traced("applyActivePreset", |_| {
        let root = internal_root(&app)?;
        let catalog = runtime::load_released_catalog(&root)?;
        let file = catalog
            .files
            .iter()
            .find(|f| f.file_name == file_name)
            .ok_or_else(|| AppError::not_found(format!("目录里没有 {file_name}")))?;

        // 应用的是盘上那份：字节得真的在、且与目录对得上（没下载/被删/漂了都不许应用）
        let on_disk = std::fs::read(root.join(&file.path)).ok();
        let bytes = on_disk.ok_or_else(|| {
            AppError::not_found(format!("{file_name} 还不在本机——先下载，再使用"))
        })?;
        let digest = runtime::catalog::hex(&sha2::Sha256::digest(&bytes));
        if digest != file.sha256 {
            return Err(AppError::sha_mismatch(format!(
                "{file_name} 盘上的内容与目录对不上，拒绝应用"
            )));
        }

        let state = runtime::state::save_active(&root, file)?;
        Ok(active_dto(&root, &catalog, state))
    })
}

/// 撤销使用。幂等：本来就没在用也不报错
#[tauri::command]
pub async fn clear_active_preset(app: AppHandle) -> Result<(), AppError> {
    traced("clearActivePreset", |_| {
        let root = internal_root(&app)?;
        runtime::state::clear_active(&root)
    })
}
