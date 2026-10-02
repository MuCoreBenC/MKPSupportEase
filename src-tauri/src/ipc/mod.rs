//! IPC 边界：前端能调的全部命令。
//!
//! 每个命令都走 [`traced`] 包一层：生成 trace id → 开 span → 调业务 → 给错误盖上同一个 id。
//! 于是界面上显示的 traceId 与日志里的 span 是同一个值，按 id 能把一次调用的全过程捞出来。
//!
//! **这一份只剩三条锚在"应用本身"上的命令**（校准三件套）：校准板清单是静态结构数据；
//! 三轴偏移真的落盘（[`save_offsets`]，`fsx::atomic` 的第一个真实调用点）；
//! 打开测试模型只记日志（"尚未实现下载与打开"，见 [`open_model`]）。
//! 业务数据（预设 / 参数 / 下载区 / 使用中…）全在 [`presets`] / [`mine`] / [`catalog`]
//! 那几个模块里，读的是 catalog 与数据根 —— 与 `src/api/contract.ts` 一一对应。
//!
//! **2026-10-02 清扫**：首圈的 `get_preset`（一张 v023 时代的硬编码表，`preset_of`）删除 ——
//! 首页与校准页早已改走文件体系（`getVersionFiles` + `getMachineParams`），
//! 它只剩测试与注册清单在引用，是"平行真相"的残留。

pub mod catalog;
/// **通用导入入口**（第十二层）：看落点（`stage_import`）与提交（`commit_import`）
pub mod import;
pub mod mine;
pub mod presets;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::error::AppError;
use crate::fsx::atomic::atomic_write_json;
use crate::fsx::paths::{resolve, Root};
use crate::obs::tracing::new_trace_id;

/// 三轴偏移，单位 mm
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Axes {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalibModel {
    pub id: String,
    pub name: String,
    pub desc: String,
    pub size: String,
    pub ready: bool,
}

/// 命令包装：trace id 贯穿 span 与错误。
///
/// 写成高阶函数而不是宏：宏能少写一点字，但报错信息会指向宏展开后的位置，
/// 调试时要多绕一道。这里的重复只有一行。
///
/// `pub(crate)` 是为了让 `workbench` 模块也走同一层 —— 工作台出错时同样要能按
/// traceId 去日志里捞，不该为它再造一套包装。
pub(crate) fn traced<T>(
    name: &'static str,
    f: impl FnOnce(&str) -> Result<T, AppError>,
) -> Result<T, AppError> {
    let trace_id = new_trace_id();
    let span = tracing::info_span!("ipc", command = name, trace_id = %trace_id);
    let _guard = span.enter();

    match f(&trace_id) {
        Ok(v) => {
            tracing::info!("ok");
            Ok(v)
        }
        Err(e) => {
            // 错误进日志一次（带 detail），给前端的那份 message 保持可展示
            tracing::warn!(code = ?e.code, detail = ?e.detail, "{}", e.message);
            Err(e.with_trace(&trace_id))
        }
    }
}

/* ---------- 静态数据 ---------- */

fn calib_models() -> Vec<CalibModel> {
    [
        ("z", "Z 轴校准", "校准喷嘴高度与第一层，先打这个", "284 KB"),
        ("xy", "XY 校准", "校准平面内的偏移，Z 轴之后打", "377 KB"),
        ("sup", "支撑测试", "校准完打这个看支撑效果", "3.2 MB"),
    ]
    .into_iter()
    .map(|(id, name, desc, size)| CalibModel {
        id: id.into(),
        name: name.into(),
        desc: desc.into(),
        size: size.into(),
        ready: true,
    })
    .collect()
}

/* ---------- 三个命令 ---------- */

/// 把三轴偏移写回配置。**原子写的第一个真实调用点**
#[tauri::command]
pub fn save_offsets(app: AppHandle, axes: Axes) -> Result<(), AppError> {
    traced("save_offsets", |_| {
        let path = resolve(&app, Root::Internal, "index/offsets.json")?;
        atomic_write_json(&path, &axes)?;
        tracing::info!(path = %path.display(), "偏移已落盘");
        Ok(())
    })
}

#[tauri::command]
pub fn get_calib_models() -> Result<Vec<CalibModel>, AppError> {
    traced("get_calib_models", |_| Ok(calib_models()))
}

/// 让壳去打开模型文件。真实实现要先查缓存、没有再下载 —— 这一轮只记日志
#[tauri::command]
pub fn open_model(model_id: String) -> Result<(), AppError> {
    traced("open_model", |_| {
        if model_id.trim().is_empty() {
            return Err(AppError::invalid_argument("没给模型 id"));
        }
        tracing::info!(model_id = %model_id, "请求打开模型（尚未实现下载与打开）");
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calib_models_have_three_entries() {
        assert_eq!(calib_models().len(), 3);
    }
}
