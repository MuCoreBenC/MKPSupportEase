//! IPC 边界：前端能调的全部命令。
//!
//! 每个命令都走 [`traced`] 包一层：生成 trace id → 开 span → 调业务 → 给错误盖上同一个 id。
//! 于是界面上显示的 traceId 与日志里的 span 是同一个值，按 id 能把一次调用的全过程捞出来。
//!
//! **这一轮返回的是硬编码数据**（doc §8 的验收目标是"链路通"，不是"功能全"）。
//! 唯一真的落盘的是 [`save_offsets`] —— 它作为 `fsx::atomic` 的第一个真实调用点。
//! 数据形状与 `src/api/contract.ts` 一一对应，将来换成读文件时前端一行不用改。

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
pub struct Preset {
    pub name: String,
    pub path: String,
    pub axes: Axes,
    pub speed: f64,
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

/* ---------- 硬编码数据：与 src/api/mock.ts 同源，将来由文件/索引接管 ---------- */

fn preset_of(variant_id: &str) -> Option<Preset> {
    let (name, path, axes, speed) = match variant_id {
        "std" => (
            "A1M.toml",
            "C:\\Users\\WZY\\Documents\\MKPSupportSSR\\presets\\mine\\A1M.toml",
            Axes {
                x: -0.6,
                y: 22.4,
                z: 3.8,
            },
            60.0,
        ),
        "fast-old" => (
            "A1MF.toml",
            "C:\\Users\\WZY\\Documents\\MKPSupportSSR\\presets\\mine\\A1MF.toml",
            Axes {
                x: -0.8,
                y: 22.8,
                z: 3.9,
            },
            65.0,
        ),
        "fast-260628" => (
            "A1MF_260628.toml",
            "C:\\Users\\WZY\\Documents\\MKPSupportSSR\\presets\\mine\\A1MF_260628.toml",
            Axes {
                x: -0.9,
                y: 23.0,
                z: 4.0,
            },
            70.0,
        ),
        _ => return None,
    };
    Some(Preset {
        name: name.into(),
        path: path.into(),
        axes,
        speed,
    })
}

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

/* ---------- 四个命令 ---------- */

/// 取某个打印件版本对应的预设。`None` 是"没有这一份"，不是出错
#[tauri::command]
pub fn get_preset(variant_id: String) -> Result<Option<Preset>, AppError> {
    traced("get_preset", |_| {
        if variant_id.trim().is_empty() {
            return Err(AppError::invalid_argument("没给打印件版本"));
        }
        Ok(preset_of(&variant_id))
    })
}

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
    fn unknown_variant_is_none_not_error() {
        let out = get_preset("没有这个版本".into()).unwrap();
        assert!(out.is_none());
    }

    #[test]
    fn empty_variant_is_invalid_argument() {
        let e = get_preset("  ".into()).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::InvalidArgument);
        // 包装层盖过 trace id：不再是初始的 "-"
        assert_ne!(e.trace_id, "-");
    }

    #[test]
    fn calib_models_have_three_entries() {
        assert_eq!(calib_models().len(), 3);
    }
}
