//! IPC 边界：前端能调的全部命令。
//!
//! 每个命令都走 [`traced`] 包一层：生成 trace id → 开 span → 调业务 → 给错误盖上同一个 id。
//! 于是界面上显示的 traceId 与日志里的 span 是同一个值，按 id 能把一次调用的全过程捞出来。
//!
//! **这一份只剩两条锚在"应用本身"上的命令**：校准板清单（[`get_calib_models`]）与
//! 打开模型文件（[`open_model`]）。两件都读**真实磁盘**：模型是 catalog 登记的三个
//! 交付文件（`assets/models/*.3mf`），本机有没有以盘为准；打开走系统默认程序
//! （`.3mf` 关联的切片器）。旧实现里"只记日志"的空壳与恒 `ready` 的假状态都已删掉。
//! 业务数据（预设 / 参数 / 下载区 / 使用中…）全在 [`presets`] / [`mine`] / [`catalog`]
//! 那几个模块里，读的是 catalog 与数据根 —— 与 `src/api/contract.ts` 一一对应。
//!
//! **2026-10-02 清扫**：首圈的 `get_preset`（一张 v023 时代的硬编码表，`preset_of`）删除 ——
//! 首页与校准页早已改走文件体系（`getVersionFiles` + `getMachineParams`），
//! 它只剩测试与注册清单在引用，是"平行真相"的残留。
//!
//! **2026-10-08 校准改判**：`save_offsets`（三轴落 `index/offsets.json`，只写不读的孤岛）
//! 退役 —— 校准值随**用户工作副本**走（`ipc::mine::save_preset_calibration` 写进那份
//! TOML 的 `offset_x/y/z`），落点与读写口都在用户线那一族里。

pub mod catalog;
/// **通用导入入口**（第十二层）：看落点（`stage_import`）与提交（`commit_import`）
pub mod import;
pub mod mine;
/// **逐参数「官方更新」的两条命令**（2026-10-09）：读三方账（我 / 官方旧值 / 官方新值）
/// 与落采用·保持的决定。官方那一版默认值住在隐藏 baseline 里，用户看不见它。
pub mod param_sync;
/// **baseline 的两条命令**（2026-10-08）：云端版本列表（已下载 / 新版本，**不是过时判定**）
/// 与「恢复默认」的基准值。baseline 是隐藏内部存储，用户看不见它。
pub mod preset_baseline;
/// **对比台的两条命令**（2026-10-08）：读一份用户预设的参数 / 把改过的几项写回它自己。
/// **只在用户自己的预设之间对比** —— 官方基线不进对比台。
pub mod preset_params;
pub mod presets;
/// **报告页的读口**：后处理执行报告与历史（`gcode_history/*_meta.json`，只读）
pub mod report;
/// **应用内更新 + 打开外链**（2026-10-05 第五刀）。`open_url` 是修「查看更新点了没反应」
/// 那个 bug 的：webview 没 opener 权限，外链一律过命令
pub mod update;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::error::AppError;
use crate::obs::tracing::new_trace_id;

/// **AppState 变更事件**（`docs/APP-STATE.md` §7）：任何写命令成功改了
/// `run/app-state.json` 之后广播。前端唯一的 AppState 客户端接住它重读整份状态、
/// 再通知订阅的页面 —— 这是"改了就推"的唯一通道，页面自己不持有状态副本。
pub const APP_STATE_EVENT: &str = "app-state-changed";

/// 广播一次 AppState 变更（尽力而为：界面不在也不影响写本身的成功）
pub fn notify_app_state(app: &AppHandle) {
    use tauri::Emitter as _;
    let _ = app.emit(APP_STATE_EVENT, ());
}

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
    /// 交付文件名（catalog 登记的那个）。**取回 / 打开都认它** —— 与下载管道同一个口径
    pub file_name: String,
    /// 本机那份的真实大小。`None` = 本机还没有这份文件 —— **不编一个假大小**（旧实现
    /// 写死 "284 KB" 那种，界面看着像有文件，其实一个字节都不在盘上）
    pub size: Option<String>,
    /// 本机有没有这份文件（下载区或旧缓存里找得到就算有），以磁盘为唯一权威
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

/* ---------- 校准模型（三个 3mf） ---------- */

/// 界面里的模型 id → 交付文件名。**id 是契约**（首页 / 校准页发的就是这三个），
/// 文件名是 catalog 的口径（「取回」走下载管道、「打开」找盘上那份，都认它）。
///
/// 映射只有这一处。id 对不上就报"没这个模型"，不猜、不静默少一项。
const MODEL_DEFS: [(&str, &str, &str, &str); 3] = [
    (
        "z",
        "Z 轴校准",
        "校准喷嘴高度与第一层，先打这个",
        "ZOffset_Calibration.3mf",
    ),
    (
        "xy",
        "XY 校准",
        "校准平面内的偏移，Z 轴之后打",
        "Precise_Calibration.3mf",
    ),
    (
        "test-models",
        "支撑测试",
        "校准完打这个看支撑效果",
        "MKP_support_test_models.3mf",
    ),
];

/// 本机那份模型文件在哪：**下载区**（catalog `path` 的落点）优先 —— 那是交付管道的
/// 口径；没有就退**旧世代缓存**（`<MKPSupportSSR>/models/`，用户机器上早就有的那三个，
/// 见 [`crate::legacy`]）。两处都没有 → `None`（"还没取回"，不是错误）。
///
/// catalog 里没有这个文件的登记时**不猜落点**：下载区那一半直接没有，只剩旧缓存可查。
fn locate_model_file(app: &AppHandle, file_name: &str) -> Result<Option<std::path::PathBuf>, AppError> {
    let internal = crate::fsx::paths::internal_root(app)?;
    let in_delivery = crate::runtime::load_released_catalog(&internal)
        .ok()
        .and_then(|catalog| {
            catalog
                .files
                .iter()
                .find(|f| f.kind == "model" && f.file_name == file_name)
                .map(|f| internal.join(&f.path))
        })
        .filter(|p| p.is_file());
    if in_delivery.is_some() {
        return Ok(in_delivery);
    }
    let legacy = crate::legacy::data_root(app)
        .map(|root| crate::legacy::models_dir(&root).join(file_name))
        .filter(|p| p.is_file());
    Ok(legacy)
}

/// 字节数 → 界面上的大小。只在**盘上真有那份文件**时给 —— 没有就 `None`，不编数。
fn size_text(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    let b = bytes as f64;
    if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

/* ---------- 两条命令 ---------- */

#[tauri::command]
pub fn get_calib_models(app: AppHandle) -> Result<Vec<CalibModel>, AppError> {
    traced("getCalibModels", |_| {
        let mut out = Vec::with_capacity(MODEL_DEFS.len());
        for (id, name, desc, file_name) in MODEL_DEFS {
            let path = locate_model_file(&app, file_name)?;
            let size = match &path {
                Some(p) => std::fs::metadata(p).ok().map(|m| size_text(m.len())),
                None => None,
            };
            out.push(CalibModel {
                id: id.to_owned(),
                name: name.to_owned(),
                desc: desc.to_owned(),
                file_name: file_name.to_owned(),
                size,
                ready: path.is_some(),
            });
        }
        Ok(out)
    })
}

/// 打开一份模型文件：用**系统默认程序**（`.3mf` 关联的切片器）打开盘上那份。
///
/// - 本机还没有这份文件 → `NOT_FOUND`，原话告诉用户先「取回」—— **不静默、不假装打开**；
/// - 系统打不开（没有关联程序那种）→ `IO` 原样冒上来，界面照实显示。
///
/// （下载是另一件事：「取回」走 `download_catalog_file`，一个动作一个命令。）
#[tauri::command]
pub fn open_model(app: AppHandle, model_id: String) -> Result<(), AppError> {
    traced("openModel", |_| {
        let id = model_id.trim();
        let Some((_, _, _, file_name)) = MODEL_DEFS.iter().find(|(mid, ..)| *mid == id) else {
            return Err(AppError::invalid_argument(format!("没这个模型：{model_id}")));
        };
        let Some(path) = locate_model_file(&app, file_name)? else {
            return Err(AppError::not_found(format!(
                "本机还没有 {file_name} —— 先在界面上「取回」，再打开"
            )));
        };
        use tauri_plugin_opener::OpenerExt as _;
        app.opener()
            .open_path(path.to_string_lossy().into_owned(), None::<&str>)
            .map_err(|e| AppError::io("系统打不开这个模型文件").with_detail(e.to_string()))?;
        tracing::info!(model_id = id, path = %path.display(), "模型已交给系统打开");
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calib_models_have_three_entries() {
        assert_eq!(MODEL_DEFS.len(), 3);
    }

    /// 界面发的三个 id 就是契约 —— 换名字要两处一起改，判据钉在这里
    #[test]
    fn model_ids_match_the_pages() {
        let ids: Vec<&str> = MODEL_DEFS.iter().map(|(id, ..)| *id).collect();
        assert_eq!(ids, ["z", "xy", "test-models"]);
    }

    #[test]
    fn size_text_only_for_real_bytes() {
        assert_eq!(size_text(512), "512 B");
        assert_eq!(size_text(284 * 1024), "284 KB");
        assert_eq!(size_text(3_250_000), "3.1 MB");
    }
}
