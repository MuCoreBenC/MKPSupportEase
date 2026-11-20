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

/* ---------- 自动检查软件更新（2026-11-05）：设置页「软件更新」那一块 ---------- */

/// **自动检查**开着吗（出厂 = 开）。`None` ⇒ 开，界面不拿 `null` 自己猜默认档。
#[tauri::command]
pub async fn get_auto_check_update(app: AppHandle) -> Result<bool, AppError> {
    traced("getAutoCheckUpdate", |_| {
        let root = internal_root(&app)?;
        runtime::app_state::auto_check_update(&root)
    })
}

/// 开关自动检查。落盘成功后广播一声 —— 前端那一格显示的永远是盘上真账。
///
/// 关掉只影响「拉目录之后顺路查」；设置页手动点「重新检查」照旧可用。
#[tauri::command]
pub async fn set_auto_check_update(app: AppHandle, enabled: bool) -> Result<bool, AppError> {
    traced("setAutoCheckUpdate", |_| {
        let root = internal_root(&app)?;
        let got = runtime::app_state::set_auto_check_update(&root, enabled)?;
        notify_app_state(&app);
        Ok(got)
    })
}

/* ---------- 更新渠道（2026-11-20）：设置页「更新渠道」那一格 ---------- */

/// 用户选的更新渠道：`stable` / `preview`。**没选过 = `stable`**。
///
/// ★ 它住盘上而不是前端：自动检查（`runtime::update_check`）要按这条挑清单，
///   只记在前端的话，重启就忘、自动检查永远只查正式版。
#[tauri::command]
pub async fn get_update_channel(app: AppHandle) -> Result<String, AppError> {
    traced("getUpdateChannel", |_| {
        let root = internal_root(&app)?;
        Ok(channel_wire(runtime::app_state::update_channel(&root)?))
    })
}

/// 换更新渠道：`stable` / `preview`。认不出就整次拒绝（不静默写成正式版）。
#[tauri::command]
pub async fn set_update_channel(app: AppHandle, channel: String) -> Result<String, AppError> {
    traced("setUpdateChannel", |_| {
        let root = internal_root(&app)?;
        let parsed = channel_parse(&channel)?;
        let got = runtime::app_state::set_update_channel(&root, parsed)?;
        notify_app_state(&app);
        Ok(channel_wire(got))
    })
}

fn channel_wire(c: runtime::release_info::UpdateChannel) -> String {
    match c {
        runtime::release_info::UpdateChannel::Stable => "stable",
        runtime::release_info::UpdateChannel::Preview => "preview",
    }
    .to_owned()
}

fn channel_parse(raw: &str) -> Result<runtime::release_info::UpdateChannel, AppError> {
    match raw.trim() {
        "stable" => Ok(runtime::release_info::UpdateChannel::Stable),
        "preview" => Ok(runtime::release_info::UpdateChannel::Preview),
        other => Err(AppError::invalid_argument(messages::text(
            "ui.ipc.prefs.493",
            &[("channel", other)],
        ))),
    }
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

/// **使用一份预设之后自动复制它的后处理脚本**（出厂 = 开）。
///
/// ★ 回的是**解析后的 `bool`**，与上面那条同一条口径（盘上"没选过"是 `None`）。
///
/// 读者**只有前端那一处**（`src/app/postprocess/autoCopy.ts`）：换完「使用中」之后
/// 要不要顺手抄一遍脚本，是界面的事 —— 后端不读这一格，也不因为这一格改任何产物。
#[tauri::command]
pub async fn get_auto_copy_script(app: AppHandle) -> Result<bool, AppError> {
    traced("getAutoCopyScript", |_| {
        let root = internal_root(&app)?;
        runtime::app_state::auto_copy_script(&root)
    })
}

/// 开关「使用之后自动复制后处理脚本」。
///
/// 落盘后照常广播（AppState 的写命令一律如此）—— 这一格有消费者，
/// 只是不在后端：前端那一条 effect 读的是后端真值，不自己留副本。
#[tauri::command]
pub async fn set_auto_copy_script(app: AppHandle, enabled: bool) -> Result<bool, AppError> {
    traced("setAutoCopyScript", |_| {
        let root = internal_root(&app)?;
        let got = runtime::app_state::set_auto_copy_script(&root, enabled)?;
        notify_app_state(&app);
        Ok(got)
    })
}

/// **下载之后自动设为使用中**（出厂 = 开）。
///
/// 读者只有预设页那一处：在「云端 → MKP 配置」点「下载」之后，要不要接着走
/// 首页那颗按钮的第二步（`applyActivePreset`）。后端不读这一格。
#[tauri::command]
pub async fn get_auto_use_after_download(app: AppHandle) -> Result<bool, AppError> {
    traced("getAutoUseAfterDownload", |_| {
        let root = internal_root(&app)?;
        runtime::app_state::auto_use_after_download(&root)
    })
}

/// 开关「下载之后自动设为使用中」。
#[tauri::command]
pub async fn set_auto_use_after_download(app: AppHandle, enabled: bool) -> Result<bool, AppError> {
    traced("setAutoUseAfterDownload", |_| {
        let root = internal_root(&app)?;
        let got = runtime::app_state::set_auto_use_after_download(&root, enabled)?;
        notify_app_state(&app);
        Ok(got)
    })
}

/* ---------- 调试窗「默认点位」（2026-11-18） ---------- */

/// 「收落笔点位测试」窗里那张参照表的一行：一台机型的一个版本。
///
/// # 数据源是随包预设的 `[toolhead]` 段
///
/// `preset::BUILTIN_PRESETS`（`build.rs` 按 `crates/preset/assets/presets/` 生成，
/// 与工作台产物同一批字节）。这里**不抄任何数字**：版本 id、落笔位、收笔位、
/// 等待位（含 Y）都从预设文本里解析。
///
/// 以前这张表是前端手写的快照，2026-11-18 作者实测指出三处已经漂了：
/// A1 的收笔位走到 274、A1 mini 261111 的落笔位是 182、P 系列的等待位 Y（7 / 0）
/// 压根没进那张表。手抄的东西没有门禁，过期了看不出来。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugAnchorRow {
    /// 版本 id：预设文件名里 `机型-` 之后那一段（`fast261111`）
    pub variant: String,
    pub mount_wait_x: f64,
    /// 预设里没写这一格 ⇒ `null`（界面上显示「跟随驻点位」）
    pub mount_wait_y: Option<f64>,
    pub mount_x: f64,
    pub unmount_wait_x: f64,
    /// 预设里没写这一格 ⇒ `null`
    pub unmount_wait_y: Option<f64>,
    pub unmount_x: f64,
}

/// 一台机型的全部版本（组内顺序 = 随包预设的文件名顺序）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugAnchorMachine {
    pub machine: String,
    pub rows: Vec<DebugAnchorRow>,
}

/// 调试窗那张参照表：从随包预设现读。
#[tauri::command]
pub fn get_debug_anchors() -> Result<Vec<DebugAnchorMachine>, AppError> {
    traced("getDebugAnchors", |_| {
        Ok(debug_anchors_from(preset::BUILTIN_PRESETS))
    })
}

/// `[toolhead]` 里的一格数字：整数与浮点都收（预设里 `pen_mount_wait_y = 7` 是整数）。
fn anchor_number(doc: &toml_edit::DocumentMut, key: &str) -> Option<f64> {
    let item = doc.get("toolhead")?.as_table()?.get(key)?;
    item.as_float()
        .or_else(|| item.as_integer().map(|v| v as f64))
}

/// [`get_debug_anchors`] 的领域体：输入是 `(文件名, 全文)`，判据可以直接喂假数据。
fn debug_anchors_from(files: &[(&str, &str)]) -> Vec<DebugAnchorMachine> {
    let mut out: Vec<DebugAnchorMachine> = Vec::new();
    for (name, text) in files {
        let Some((machine, variant)) = split_preset_name(name) else {
            continue;
        };
        let Ok(doc) = text.parse::<toml_edit::DocumentMut>() else {
            continue;
        };
        /* 落笔位 / 收笔位是必填（缺了这份就不是这一套笔预设）；等待位缺 = 跟随目标位 */
        let (Some(mount_x), Some(unmount_x)) = (
            anchor_number(&doc, "pen_mount_x"),
            anchor_number(&doc, "pen_unmount_x"),
        ) else {
            continue;
        };
        let row = DebugAnchorRow {
            variant,
            mount_wait_x: anchor_number(&doc, "pen_mount_wait_x").unwrap_or(mount_x),
            mount_wait_y: anchor_number(&doc, "pen_mount_wait_y"),
            mount_x,
            unmount_wait_x: anchor_number(&doc, "pen_unmount_wait_x").unwrap_or(unmount_x),
            unmount_wait_y: anchor_number(&doc, "pen_unmount_wait_y"),
            unmount_x,
        };
        match out.iter_mut().find(|m| m.machine == machine) {
            Some(m) => m.rows.push(row),
            None => out.push(DebugAnchorMachine {
                machine,
                rows: vec![row],
            }),
        }
    }
    out.sort_by(|a, b| a.machine.cmp(&b.machine));
    out
}

/// `A1_MINI-fast261111.toml` → (`A1_MINI`, `fast261111`)
fn split_preset_name(name: &str) -> Option<(String, String)> {
    let stem = name.strip_suffix(".toml")?;
    let (machine, variant) = stem.rsplit_once('-')?;
    if machine.is_empty() || variant.is_empty() {
        return None;
    }
    Some((machine.to_owned(), variant.to_owned()))
}

#[cfg(test)]
mod debug_anchor_tests {
    use super::*;

    /// 随包预设里每一份都出一行 —— 少一份就是这张表又漏了一台机器。
    #[test]
    fn every_builtin_preset_yields_a_row() {
        let anchors = debug_anchors_from(preset::BUILTIN_PRESETS);
        let total: usize = anchors.iter().map(|m| m.rows.len()).sum();
        assert_eq!(
            total,
            preset::BUILTIN_PRESETS.len(),
            "有预设没进参照表：{anchors:?}"
        );
    }

    /// P 系列的等待位 Y 必须来自文件（作者 2026-11-18：表里原来只有 X，Y 全是 0）。
    #[test]
    fn the_p_series_wait_y_comes_from_the_file() {
        let anchors = debug_anchors_from(preset::BUILTIN_PRESETS);
        let p1s = anchors
            .iter()
            .find(|m| m.machine == "P1S")
            .expect("随包预设里应当有 P1S");
        assert!(
            p1s.rows.iter().all(|r| r.mount_wait_y == Some(7.0)),
            "P1S 的落笔等待位 Y 应读成 7：{:?}",
            p1s.rows
        );
        assert!(
            anchors
                .iter()
                .filter(|m| m.machine.starts_with("A1"))
                .all(|m| m.rows.iter().all(|r| r.mount_wait_y.is_none())),
            "A 系列预设没写等待位 Y ⇒ 该是 null（跟随驻点位）"
        );
    }

    /// 前端按 camelCase 取值 —— 这一条钉住 DTO 的线上形状。
    #[test]
    fn the_dto_speaks_camel_case() {
        let json = serde_json::to_value(DebugAnchorRow {
            variant: "fast261111".into(),
            mount_wait_x: 256.0,
            mount_wait_y: None,
            mount_x: 267.0,
            unmount_wait_x: 256.0,
            unmount_wait_y: None,
            unmount_x: 274.0,
        })
        .expect("序列化");
        for key in [
            "mountWaitX",
            "mountWaitY",
            "mountX",
            "unmountWaitX",
            "unmountWaitY",
            "unmountX",
        ] {
            assert!(json.get(key).is_some(), "缺 {key}：{json}");
        }
    }
}
/* ---------- 「恢复默认」的第二半（2026-11-18） ---------- */

/// 客户端壳那几格偏好的线上形状（`current` 与 `factory` 用同一个 DTO）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientPrefsSliceDto {
    pub theme: String,
    pub locale: String,
    pub auto_clean_history: bool,
    pub auto_check_update: bool,
    pub copies_follow_cloud: bool,
    pub auto_copy_script: bool,
    pub auto_use_after_download: bool,
    /// 数据源那一格：`gitee` / `github` / `custom:<地址>`
    pub source: String,
}

/// 「恢复默认」对话框要摆的两侧：现在是什么、出厂是什么。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientPrefsViewDto {
    pub current: ClientPrefsSliceDto,
    pub factory: ClientPrefsSliceDto,
}

/// 数据源那一格的线上说法（`custom` 带地址 —— 用户填的那一份）。
fn source_wire(root: &std::path::Path) -> Result<String, AppError> {
    use crate::runtime::source::{SourceMode, DEFAULT_MODE};
    let stored = runtime::app_state::preset_source(root)?;
    Ok(match stored {
        Some(s) if s.mode == SourceMode::Custom => {
            format!("custom:{}", s.custom_url.unwrap_or_default())
        }
        Some(s) => s.mode.wire().to_owned(),
        None => DEFAULT_MODE.wire().to_owned(),
    })
}

/// 读「恢复默认」要摆的那两列（一次读完，别让对话框发七八个请求）。
#[tauri::command]
pub async fn get_client_prefs_view(app: AppHandle) -> Result<ClientPrefsViewDto, AppError> {
    traced("getClientPrefsView", |_| {
        let root = internal_root(&app)?;
        let d = runtime::client_defaults::client_defaults();
        let current = ClientPrefsSliceDto {
            theme: runtime::app_state::theme(&root)?
                .unwrap_or(d.theme)
                .wire()
                .to_owned(),
            locale: runtime::app_state::locale(&root)?
                .unwrap_or(d.locale)
                .wire()
                .to_owned(),
            auto_clean_history: runtime::app_state::auto_clean_history(&root)?,
            auto_check_update: runtime::app_state::auto_check_update(&root)?,
            copies_follow_cloud: runtime::app_state::copies_follow_cloud(&root)?,
            auto_copy_script: runtime::app_state::auto_copy_script(&root)?,
            auto_use_after_download: runtime::app_state::auto_use_after_download(&root)?,
            source: source_wire(&root)?,
        };
        let factory = ClientPrefsSliceDto {
            theme: d.theme.wire().to_owned(),
            locale: d.locale.wire().to_owned(),
            auto_clean_history: d.auto_clean_history,
            auto_check_update: d.auto_check_update,
            copies_follow_cloud: d.copies_follow_cloud,
            auto_copy_script: d.auto_copy_script,
            auto_use_after_download: d.auto_use_after_download,
            source: crate::runtime::source::DEFAULT_MODE.wire().to_owned(),
        };
        Ok(ClientPrefsViewDto { current, factory })
    })
}

/// 「恢复默认」：客户端壳那几格一起清回"没选过"，并当场生效（语言 / 主题）。
///
/// 与 [`get_client_prefs_view`] 配对：那边摆两列，这边真动手。
#[tauri::command]
pub async fn reset_client_prefs(app: AppHandle) -> Result<serde_json::Value, AppError> {
    traced("resetClientPrefs", |_| {
        let root = internal_root(&app)?;
        runtime::app_state::clear_client_prefs(&root)?;
        /* 语言：出厂档要在本进程里也生效（后端那几句提示跟着换） */
        runtime::app_state::apply_lang(None);
        /* 主题：原生那一层（圆角窗外那圈填色 / 工作台标题栏）跟着回出厂 */
        let theme = runtime::client_defaults::client_defaults().theme;
        if let Some(win) = app.get_webview_window("main") {
            crate::chrome::repaint_corner_fill(&win, resolved_dark(Some(theme), &win));
        }
        #[cfg(feature = "workbench")]
        if let Some(win) = app.get_webview_window(crate::workbench::WINDOW_LABEL) {
            apply_native_theme(&win, Some(theme));
        }
        notify_app_state(&app);
        Ok(serde_json::json!({ "ok": true }))
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
        /* 出厂档：全 `null` 解析出来的那一份（与 `resolved()` 同一个函数，不另算一套） */
        let f = serde_json::from_str::<runtime::app_state::PostprocPrefs>("{}")
            .map_err(|e| AppError::internal(format!("出厂偏好解析不了：{e}")))?
            .resolved();
        Ok(serde_json::json!({
            /* 原样那一格（界面用来判断"这一项动过没有"） */
            "raw": raw,
            /* 解析后（界面直接摆这些） */
            "resolved": {
                "notifyOnFinish": r.notify_on_finish,
                "uiMode": r.ui_mode.wire(),
                /* 等回答时置顶（2026-11-17）：只影响窗口层级，不进 IR */
                "autoTopOnAsk": r.auto_top_on_ask,
                "glueBottomEnabled": r.glue_bottom_enabled,
                /* 下方支撑余量（实验，2026-11-17）：余量够的涂胶段不抬高 */
                "underSupportEnabled": r.under_support_enabled,
                "underSupportMm": r.under_support_mm,
                "skipVibrationCalibration": r.skip_vibration_calibration,
                "beginStageRetractEnabled": r.begin_stage_retract_enabled,
                "beginStageRetractZLiftMm": r.begin_stage_retract_z_lift_mm,
                /* 校准「切笔」的安全抬升量（2026-11-19）：三种校准共用一个值 */
                "calibrationSafeLiftMm": r.calibration_safe_lift_mm,
                "zCalibrationDwellSec": r.z_calibration_dwell_sec,
                "xyCalibrationDwellSec": r.xy_calibration_dwell_sec,
                "bedGlueEnabled": r.bed_glue_enabled,
                "bedGlueAllModels": r.bed_glue_all_models,
                /* 强制校准模式（2026-11-12）：只按外形认校准件，认不出就停下 */
                "forceCalibrationMode": r.force_calibration_mode,
                /* 智能塔位置三档（2026-11-04）：`off` / `auto` / `ask` */
                "autoTowerPosition": r.auto_tower_position.wire(),
                /* 调试模式（2026-11-11）：开关 + 四个动作坐标。
                 * 这一格是**手写列字段**的，结构里加了不写在这里，界面就拿到 undefined。 */
                "debugModeEnabled": r.debug_mode_enabled,
                "debugWaitX": r.debug_wait_x,
                "debugMountWaitX": r.debug_mount_wait_x,
                "debugMountX": r.debug_mount_x,
                "debugUnmountWaitX": r.debug_unmount_wait_x,
                "debugUnmountX": r.debug_unmount_x,
                "debugSpeed": r.debug_speed,
                "debugWaitY": r.debug_wait_y,
                "debugMountWaitY": r.debug_mount_wait_y,
                "debugMountY": r.debug_mount_y,
                "debugUnmountWaitY": r.debug_unmount_wait_y,
                "debugUnmountY": r.debug_unmount_y,
                "debugCustomGcode": r.debug_custom_gcode,
                "debugTravelSpeed": r.debug_travel_speed,
                "debugUseTravelSpeed": r.debug_use_travel_speed,
                "debugRepeat": r.debug_repeat,
                "debugDwellSec": r.debug_dwell_sec,
                "debugMoveY": r.debug_move_y,
            },
            /*
             * 出厂档 = 全 `null` 解析出来的那一份。
             *
             * 恢复默认的对比要用它：只写「出厂」两个字，用户看不出点完会变成开还是关、
             * 数字是多少。这里给「有出厂值的那几项」：`[Prefs]` 段的 12 项 + 通知 /
             * 显示方式 / 置顶 / 强制校准这 4 项（它们的出厂档在同一份文件的 `[Client]` 段），
             * 与 `resetDefaults` 的补丁一一对应；调试动作那组有自己那颗恢复，不在这里。
             */
            "factory": {
                "notifyOnFinish": f.notify_on_finish,
                "uiMode": f.ui_mode.wire(),
                "autoTopOnAsk": f.auto_top_on_ask,
                "forceCalibrationMode": f.force_calibration_mode,
                "skipVibrationCalibration": f.skip_vibration_calibration,
                "beginStageRetractEnabled": f.begin_stage_retract_enabled,
                "beginStageRetractZLiftMm": f.begin_stage_retract_z_lift_mm,
                "calibrationSafeLiftMm": f.calibration_safe_lift_mm,
                "zCalibrationDwellSec": f.z_calibration_dwell_sec,
                "xyCalibrationDwellSec": f.xy_calibration_dwell_sec,
                "underSupportEnabled": f.under_support_enabled,
                "underSupportMm": f.under_support_mm,
                "glueBottomEnabled": f.glue_bottom_enabled,
                "bedGlueEnabled": f.bed_glue_enabled,
                "bedGlueAllModels": f.bed_glue_all_models,
                "autoTowerPosition": f.auto_tower_position.wire(),
            },
            /* 量程（界面画数字框要用，**不另写一份**在 TS 里） */
            "limits": {
                "zDwell": { "min": runtime::app_state::Z_DWELL_MIN, "max": runtime::app_state::Z_DWELL_MAX },
                "xyDwell": { "min": runtime::app_state::XY_DWELL_MIN, "max": runtime::app_state::XY_DWELL_MAX },
                "zLift": {
                    "min": runtime::app_state::BEGIN_STAGE_RETRACT_Z_LIFT_MIN,
                    "max": runtime::app_state::BEGIN_STAGE_RETRACT_Z_LIFT_MAX,
                },
                "calibLift": {
                    "min": runtime::app_state::CALIBRATION_SAFE_LIFT_MM_MIN,
                    "max": runtime::app_state::CALIBRATION_SAFE_LIFT_MM_MAX,
                },
                "underSupport": {
                    "min": runtime::app_state::UNDER_SUPPORT_MM_MIN,
                    "max": runtime::app_state::UNDER_SUPPORT_MM_MAX,
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

/* ---------- 前端那两格偏好的出厂档（2026-11-18） ---------- */

/// 前端自己拿着的两格偏好（切片器 / 保存后的重新切片提示）在"没设过"时用什么。
///
/// ★ 这两格住在 webview 的 localStorage（`mkp.A40.slicer` / `mkp.A40.guide.show`），
///   壳读不到它们，所以出厂值只能**从这里递过去**：前端启动时问一次，之后按它兜底。
///   判据仍是 `presets/client-defaults.toml` 的 `[Client]` 段，前端不另写一份。
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientDefaultsDto {
    /// `system` / `bambu` / `orca` —— 与 `open_model` 认的那套 id 同字
    pub slicer: String,
    /// 保存参数后要不要弹「回切片器重新切片」那一屏
    pub guide_show: bool,
}

#[tauri::command]
pub async fn get_client_defaults() -> Result<ClientDefaultsDto, AppError> {
    traced("getClientDefaults", |_| {
        let d = runtime::client_defaults::client_defaults();
        Ok(ClientDefaultsDto {
            slicer: d.slicer.wire().to_owned(),
            guide_show: d.guide_show,
        })
    })
}
