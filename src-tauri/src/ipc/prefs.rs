//! **界面偏好**（2026-10-12「整套客户端做深色模式」那一刀开的）：外壳级、跨重启记着的
//! 用户选择。头一格就是[主题]。
//!
//! 2026-10-13 添了第二格：**执行记录的自动清理开关**（[`get_auto_clean_history`] /
//! [`set_auto_clean_history`]）—— 它同样是"用户点出来的一档偏好"，不是一份业务数据；
//! 真正动手清理的是 [`crate::retention`]，这一层只管把这一档写下来 / 读回去。
//!
//! 两格都落在 `run/app-state.json`（`theme` / `autoCleanHistory`）——
//! [`crate::runtime::app_state`] 是它唯一的读写入口，这一层只管"线上形状 ↔ 盘上形状"
//! 的翻译与写成功之后的那次广播。
//!
//! 为什么单开一个模块、不塞进 [`super::catalog`]：那一位管的是**数据从哪儿来**
//! （源地址 / 目录 / 下载），主题跟数据无关。混在一起会让"这条命令要不要联网"这种
//! 问题只能靠通读代码来回答；这里两条命令都不联网、也不碰文件系统（除那一格状态）。
//!
//! 写成功走 [`super::notify_app_state`] 广播 —— 前端不用"切完手动刷新"，
//! 与 AppState 其它格是同一条通道（`docs/APP-STATE.md` §7）。

use tauri::AppHandle;

use crate::error::AppError;
use crate::fsx::paths::internal_root;
use crate::runtime;
use crate::runtime::app_state::ThemePref;

use super::{notify_app_state, traced};

/// 当前主题偏好。`null` = **还没选过** —— 界面按"跟随系统"（这一档不落盘）。
///
/// ★ `null` 与 `"system"` 意思不同：前者是"从没选过"，后者是"用户明确点了跟随系统"。
/// 界面处理一样（都跟随系统），但盘上要分得开 —— 将来若改出厂默认档，
/// "没选过"这一格还能被重新解释（旧档照读，不迁移、不升号）。
#[tauri::command]
pub async fn get_theme(app: AppHandle) -> Result<Option<String>, AppError> {
    traced("getTheme", |_| {
        let root = internal_root(&app)?;
        Ok(runtime::app_state::theme(&root)?
            .map(ThemePref::wire)
            .map(str::to_owned))
    })
}

/// 换主题档位：`light` / `dark` / `system`。**先认字符串再落盘** ——
/// 认不出就整次拒绝（不静默写成浅色：那会让界面显示的和盘上的不是一回事）。
#[tauri::command]
pub async fn set_theme(app: AppHandle, theme: String) -> Result<String, AppError> {
    traced("setTheme", |_| {
        let root = internal_root(&app)?;
        let pref = ThemePref::parse(&theme).ok_or_else(|| {
            AppError::invalid_argument(format!(
                "认不出的主题：{theme}（只能是 light / dark / system）"
            ))
        })?;
        runtime::app_state::set_theme(&root, pref)?;
        notify_app_state(&app);
        Ok(pref.wire().to_owned())
    })
}

/// 后处理记录的**自动清理**开着吗（2026-10-13）。
///
/// ★ 回的是**解析后的 `bool`**：盘上那一格"没选过"是 `None`（= 开着），
/// 而"没选过"该怎么解释是**后端的事** —— 界面不自己拿 `null` 去猜默认值
/// （猜出来的默认值迟早与 [`crate::retention`] 实际执行的那一档漂开）。
#[tauri::command]
pub async fn get_auto_clean_history(app: AppHandle) -> Result<bool, AppError> {
    traced("getAutoCleanHistory", |_| {
        let root = internal_root(&app)?;
        runtime::app_state::auto_clean_history(&root)
    })
}

/// 开关自动清理。**点了就是点了**（与主题同一条口径：单选不用再要一颗确认键），
/// 落盘成功后广播一声 —— 前端那一格显示的永远是盘上真账。
#[tauri::command]
pub async fn set_auto_clean_history(app: AppHandle, enabled: bool) -> Result<bool, AppError> {
    traced("setAutoCleanHistory", |_| {
        let root = internal_root(&app)?;
        let got = runtime::app_state::set_auto_clean_history(&root, enabled)?;
        notify_app_state(&app);
        Ok(got)
    })
}
