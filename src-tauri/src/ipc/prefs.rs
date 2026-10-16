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

use tauri::{AppHandle, Manager};

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
///
/// ★ 落盘之后要顺手把**原生那一层**也换掉（`chrome::repaint_corner_fill`）：
/// 圆角窗外那一圈填色是 AppKit 画的，CSS 碰不到它（见 `chrome.rs` 里那段注释）。
/// 不重画的话，用户在设置里换档之后，那一圈会留着上一次的颜色 —— 深色界面上就是一条白光。
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
        if let Some(win) = app.get_webview_window("main") {
            crate::chrome::repaint_corner_fill(&win, resolved_dark(Some(pref), &win));
        }
        /*
         * ★ 工作台那扇窗（只在 `workbench` feature 下存在）也要跟着翻（2026-10-16）。
         * 它没有"圆角填色"那一层（那扇窗用系统原生装饰，不是无边框玻璃）——
         * 它要换的是**原生外观**：标题栏 / 滚动条 / 原生控件。少这一步，
         * 深色档下就是"页面是深色、标题栏还浅着"。
         */
        #[cfg(feature = "workbench")]
        if let Some(win) = app.get_webview_window(crate::workbench::WINDOW_LABEL) {
            apply_native_theme(&win, Some(pref));
        }
        Ok(pref.wire().to_owned())
    })
}

/// **现在到底是不是深色** —— 原生那一层与 CSS 用的是同一条规则：
/// 先看**用户选的那一档**，只有 `system`（或"还没选过"）才去问**原生外观**。
///
/// 「跟随系统」这一档问原生外观是**同源**而不是第二个判据：CSS 那侧的 `prefers-color-scheme`
/// 在 macOS 上读的正是窗口的外观（见 `src/app/state/theme.ts` 的 `resolveDark`）。
///
/// ★★ 为什么不能直接读 `webview.effectiveAppearance()`：那玩意儿**只跟系统外观走** ——
/// 用户选"深色"而系统是浅色时，它会答"浅色"（作者那台就是这种组合，2026-10-15 提交前补的洞）。
pub fn resolved_dark<R: tauri::Runtime>(
    pref: Option<ThemePref>,
    win: &tauri::WebviewWindow<R>,
) -> bool {
    match pref {
        Some(ThemePref::Dark) => true,
        Some(ThemePref::Light) => false,
        _ => matches!(win.theme(), Ok(tauri::Theme::Dark)),
    }
}

/// **原生外观**该设成哪一档 —— 返回值直接喂 [`tauri::WebviewWindow::set_theme`]。
///
/// ★ `None` 不是"不设"，是**交回给系统**：用户选了「跟随系统」（或还没选过）时，
/// 窗口外观就该跟着系统走。硬设成"系统现在这一档"反而把它**钉死**了 ——
/// 之后系统切另一档，它就再也不动。
pub fn native_theme(pref: Option<ThemePref>) -> Option<tauri::Theme> {
    match pref {
        Some(ThemePref::Dark) => Some(tauri::Theme::Dark),
        Some(ThemePref::Light) => Some(tauri::Theme::Light),
        _ => None,
    }
}

/// 把一扇窗的**原生那一层**换到 `pref` 那一档（2026-10-16 工作台做深色时加的）。
///
/// 客户端主窗口那一条"原生"是**圆角外的填色**（[`crate::chrome::repaint_corner_fill`]）
/// —— 它无边框，圆角是自己画的；工作台那扇窗是**系统原生装饰**，CSS 只画得到内容区，
/// 所以它的"原生那一层"是 `set_theme`：**标题栏 / 滚动条 / 原生控件**跟着一起翻。
/// 少这一步，工作台在深色档下就是"页面深色、标题栏浅色"。
pub fn apply_native_theme<R: tauri::Runtime>(
    win: &tauri::WebviewWindow<R>,
    pref: Option<ThemePref>,
) {
    let _ = win.set_theme(native_theme(pref));
}

/// 读盘上那一档（给启动时算原生外观用；`None` = 还没选过 ⇒ 跟随系统）。
pub fn theme_pref(app: &AppHandle) -> Option<ThemePref> {
    internal_root(app)
        .ok()
        .and_then(|root| runtime::app_state::theme(&root).ok())
        .flatten()
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
