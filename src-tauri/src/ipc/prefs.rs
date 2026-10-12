//! **界面偏好**（2026-10-12「整套客户端做深色模式」那一刀开的）：外壳级、跨重启记着的
//! 用户选择。头一格就是[主题]。
//!
//! 主题落在 `run/app-state.json` 的 `theme` 格 —— [`crate::runtime::app_state`] 是它
//! 唯一的读写入口，这一层只管"字符串 ↔ 枚举"的翻译与写成功之后的那次广播。
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
