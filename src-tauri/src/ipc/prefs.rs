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

use postprocess::diag::messages;

use tauri::{AppHandle, Manager};

use crate::error::AppError;
use crate::fsx::paths::internal_root;
use crate::runtime;
use crate::runtime::app_state::{LocalePref, ThemePref};

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
            AppError::invalid_argument(messages::text(
                "ui.ipc.prefs.114",
                &[("theme", &theme.to_string())],
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

/// 主窗的**原生背景色** = 界面底（2026-10-22，`--bg-2` 由前端报上来）。
///
/// ★★★ 治的是什么：**macOS 上拖拽改窗口大小时，右侧/底部会露出白边**。
///
/// 这不是我们的代码问题 —— 是 WebKit 的已知呈现 bug（`WebKit#323132`，
/// 下游 `tauri-apps/tauri#14288`）。live resize 时 AppKit 同步推进 WKWebView 的
/// bounds，而 WebContent 进程的远程图层树是**异步提交**的：新图层到达前，
/// 屏幕上还是上一次提交的、旧尺寸的页面表面，盖不满新 bounds 的边就露出来了
/// （上游量到 20~60px）。Tauri 维护者已表态这个场景 **wontfix**。
///
/// 报告里列过一串"已验证无效"的手段（别再试）：`Resized` 里强推 webview frame、
/// `preservesContentDuringLiveResize`、`NSViewLayerContentsRedrawPolicy::DuringViewResize`、
/// 在 `html`/`body` 反复设背景色 —— **都没有实质改善**。
///
/// 唯一现实可行的是「**把露出来的那条边变成同色**」：设原生背景色，让白边变成界面底。
///
/// ★★ 为什么之前只设 `NSWindow.backgroundColor` 没用：露出来的那一层是
/// **WKWebView 自己的白**（`drawsBackground` 默认 true），不是窗口底。
/// Tauri 的 [`WebviewWindow::set_background_color`] 恰好两件事一起做 ——
/// ① `window.set_background_color`（窗口底）；② wry 侧把 webview 的
/// `drawsBackground` 关掉并设 `underPageBackgroundColor`。两层都盖住才没有白。
///
/// ★ 颜色**不在 Rust 里写死**：前端在 `<html data-theme>` 落地之后读 `--bg-2` 报上来，
/// 真值仍然只有 `tokens.css` 一处。
///
/// ★ 同步命令（不是 `async`）：这个 API 要碰 AppKit，Tauri 的同步命令就在主线程跑。
#[tauri::command]
pub fn set_window_backdrop(app: AppHandle, color: String) -> Result<(), AppError> {
    traced("setWindowBackdrop", |_| {
        let Some(win) = app.get_webview_window("main") else {
            return Ok(());
        };
        let Some((r, g, b)) = parse_hex(&color) else {
            tracing::warn!("窗口底色只认 #rrggbb，收到 {color:?}，跳过");
            return Ok(());
        };
        win.set_background_color(Some(tauri::window::Color(r, g, b, 255)))
            .map_err(|e| {
                AppError::internal(messages::text("ui.ipc.prefs.115", &[]))
                    .with_detail(e.to_string())
            })?;
        Ok(())
    })
}

/// `#rrggbb` → (r, g, b)。只认这一种形状 —— 调用方给的是 `getComputedStyle`
/// 读出来的 `--bg-2`，本来就是六个十六进制位；别的形状当"没给"（宁可不刷，也不猜）。
fn parse_hex(hex: &str) -> Option<(u8, u8, u8)> {
    let s = hex.trim().trim_start_matches('#');
    if s.len() != 6 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let at = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).ok();
    Some((at(0)?, at(2)?, at(4)?))
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

/* ---------- 界面语言（2026-10-25）：设置页「界面语言」那一块 ---------- */

/// 当前界面语言。`null` = **还没选过** ⇒ 界面按"跟随系统"（与 `get_theme` 同一口径）。
#[tauri::command]
pub async fn get_locale(app: AppHandle) -> Result<Option<String>, AppError> {
    traced("getLocale", |_| {
        let root = internal_root(&app)?;
        Ok(runtime::app_state::locale(&root)?
            .map(LocalePref::wire)
            .map(str::to_owned))
    })
}

/// 换界面语言：`system` / `zh-CN` / `zh-TW` / `ja` / `en`。**先认字符串再落盘** ——
/// 认不出就整次拒绝（不静默写成中文：那会让人以为自己点没反应）。
///
/// ★ 落盘成功之后有两件事要跟着做，顺序不能反：
///
///   ① **本进程内后端那几句提示换语言**（`messages::set_lang`）——
///      界面上「这次检查失败了：<后端那一句>」里的后半句就是它，不翻的话
///      英文界面上会冒出一段中文（正是 2026-10-26 那张截图里的样子）。
///   ② 广播一次 AppState：前端字典据此换一套文案，不用重启。
///
/// 翻语言这一步**只在写成功之后**：写失败却已经换了字，盘上与界面就不一致了。
#[tauri::command]
pub async fn set_locale(app: AppHandle, locale: String) -> Result<String, AppError> {
    traced("setLocale", |_| {
        let root = internal_root(&app)?;
        let pref = LocalePref::parse(&locale).ok_or_else(|| {
            AppError::invalid_argument(messages::text(
                "ui.ipc.prefs.492",
                &[("locale", &locale.to_string())],
            ))
        })?;

        runtime::app_state::set_locale(&root, pref)?;
        runtime::app_state::apply_lang(Some(pref));
        notify_app_state(&app);
        Ok(pref.wire().to_owned())
    })
}

/* ---------- 预设页偏好（2026-10-20）：设置页「预设」那一块 ---------- */

/// **复制出来的那一份跟不跟云端**（出厂 = 关）。
///
/// ★ 回的是**解析后的 `bool`**：盘上"没选过"是 `None`，由后端解释成出厂档 ——
/// 界面不自己拿 `null` 去猜（与自动清理那条同一条口径）。
///
/// 读者有两处（**同一格、两条路、一个判据**）：身份账本出口
/// （[`super::local::get_local_preset_index`]）与逐参数更新账的出口
/// （[`super::param_sync::get_preset_param_sync`]）。
#[tauri::command]
pub async fn get_copies_follow_cloud(app: AppHandle) -> Result<bool, AppError> {
    traced("getCopiesFollowCloud", |_| {
        let root = internal_root(&app)?;
        runtime::app_state::copies_follow_cloud(&root)
    })
}

/// 开关「复制出来的也跟着云端更新」。
///
/// 落盘后要广播：这一档**立刻改变**预设页云端行的三态（未获取 / 本地已有 / 有新版）
/// 与操作列那颗「更新」—— 那一态由身份账本算，而账本的出口正看这一格。
/// 不广播的话，用户切回预设页看到的还是旧账。
#[tauri::command]
pub async fn set_copies_follow_cloud(app: AppHandle, enabled: bool) -> Result<bool, AppError> {
    traced("setCopiesFollowCloud", |_| {
        let root = internal_root(&app)?;
        let got = runtime::app_state::set_copies_follow_cloud(&root, enabled)?;
        notify_app_state(&app);
        Ok(got)
    })
}

/* ---------- 后处理偏好（2026-10-17）：设置页那一整块 ---------- */

/// 读后处理偏好 —— 回的是**解析后**的值（`None` 已经解成出厂档、量程已经夹住）。
///
/// 与自动清理那一条同一条口径：**"没选过"该怎么解释是后端的事**，
/// 界面不自己拿 `null` 去猜默认值（猜出来的迟早与管线实际执行的那一档漂开）。
#[tauri::command]
pub async fn get_postproc_prefs(app: AppHandle) -> Result<serde_json::Value, AppError> {
    traced("getPostprocPrefs", |_| {
        let root = internal_root(&app)?;
        let raw = runtime::app_state::postproc_prefs(&root)?;
        let r = raw.resolved();
        Ok(serde_json::json!({
            /* 原样那一格（界面用来判断"这一项动过没有"） */
            "raw": raw,
            /* 解析后（界面直接摆这些） */
            "resolved": {
                "notifyOnFinish": r.notify_on_finish,
                "uiMode": r.ui_mode.wire(),
                "glueBottomEnabled": r.glue_bottom_enabled,
                "skipVibrationCalibration": r.skip_vibration_calibration,
                "beginStageRetractEnabled": r.begin_stage_retract_enabled,
                "beginStageRetractZLiftMm": r.begin_stage_retract_z_lift_mm,
                "zCalibrationDwellSec": r.z_calibration_dwell_sec,
                "xyCalibrationDwellSec": r.xy_calibration_dwell_sec,
                "bedGlueEnabled": r.bed_glue_enabled,
                "bedGlueAllModels": r.bed_glue_all_models,
            },
            /* 量程（界面画数字框要用，**不另写一份**在 TS 里） */
            "limits": {
                "zDwell": { "min": runtime::app_state::Z_DWELL_MIN, "max": runtime::app_state::Z_DWELL_MAX },
                "xyDwell": { "min": runtime::app_state::XY_DWELL_MIN, "max": runtime::app_state::XY_DWELL_MAX },
                "zLift": {
                    "min": runtime::app_state::BEGIN_STAGE_RETRACT_Z_LIFT_MIN,
                    "max": runtime::app_state::BEGIN_STAGE_RETRACT_Z_LIFT_MAX,
                },
            },
        }))
    })
}

/// 写后处理偏好（**整格覆盖**，见 `runtime::app_state::set_postproc_prefs` 的注释）。
///
/// 参数是一整份 `PostprocPrefs`：界面手里就有全量，改一项传全量。
/// 传 `null` 进来 = 清回出厂档（这也正是"整格覆盖"才能表达的那一档）。
#[tauri::command]
pub async fn set_postproc_prefs(
    app: AppHandle,
    prefs: runtime::app_state::PostprocPrefs,
) -> Result<serde_json::Value, AppError> {
    traced("setPostprocPrefs", |_| {
        let root = internal_root(&app)?;
        runtime::app_state::set_postproc_prefs(&root, prefs)?;
        notify_app_state(&app);
        Ok(serde_json::json!({ "ok": true }))
    })
}
