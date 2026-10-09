//! 工作台的 Rust 侧。
//!
//! **整个模块只在 `workbench` feature 下存在。** `lib.rs` 里的 `mod workbench;` 挂着
//! `#[cfg(feature = "workbench")]`，所以默认构建连编译都不会碰这些文件 —— 给用户的
//! 二进制里搜不到下面任何一个命令名。
//!
//! # 当前状态
//!
//! 第二版那 30 多个 `wb_*` 命令与整个 domain 层已全部摘除（doc §6：旧契约作废），
//! 新架构按三层重排：
//!
//! - [`presets`] 预设真相源（`<repo>/presets/*.toml`，读它也写它）
//! - [`domain`] 规则与派生（三层取值、归并、可见性、patch、状态、预览、文案）
//! - [`app`] IPC 入口与落盘（**写只有 `wb_apply_draft` 一条**）
//!
//! 留用的底座：[`paths`]（数据根）/ [`clock`]（时间戳唯一来源）/
//! [`store`]（原子写、id 白名单、通用 JSON IO、草稿/快照/回收站的落盘位置）。
//!
//! **只剩一个 workspace 数据根**：`<repo>/presets`。旧的 `upstream/`（只读别人的
//! `mkpse-presets/content/*.json`）连同它的三级定位、环境变量、fallback 与设置项
//! 一起退休了 —— 不 fallback、不猜路径，运行时只认这一个根。

pub mod app;
pub mod clock;
pub mod domain;
pub mod paths;
/// 预设真相源。**它不住在这里** —— 代码搬去了 [`crate::presetdata`]（客户端也要读预设，
/// 而这一整棵子树是 feature gate 的），这里只是把它**别名**回 `presets`：
/// 工作台侧几十处 `crate::workbench::presets::X` 因此一个都不用改。
///
/// 数据根仍然是工作台自己的那一个（`<repo>/presets`），经 [`load_presets`] 给进去 ——
/// **代码共用一份，数据根两套**。
pub use crate::presetdata as presets;
pub mod store;

/// 两端一致性判据：客户端那份「血统三行」与这里那份必须逐字节一样（**只在测试里编**）
#[cfg(test)]
mod lineage_parity;

use serde::Serialize;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::error::AppError;

/// 读工作台的预设数据根：`<repo>/presets`。
///
/// 这是工作台与客户端**唯一**的分岔点：两边跑同一份 [`presetdata`] 代码，
/// 只是根不同 —— 客户端给 `appDataDir/presets`，工作台给仓库。
///
/// 资产**载荷**根（`<repo>/public/assets`）只在这一档挂上：客户端这一轮只释放定义、
/// 不释放文件本体，所以它那边 [`presets::Assets::present`] 一律为 false。
pub fn load_presets() -> Result<crate::presetdata::Presets, AppError> {
    let root = paths::presets_root().ok_or_else(|| {
        AppError::not_found("找不到工作台的预设数据根 presets/").with_detail(
            "期望 <repo>/presets/registry/param_registry.toml 存在 —— \
             工作台的数据根指向**仓库里的开发源数据**，不是 appDataDir"
                .to_owned(),
        )
    })?;
    let mut p = crate::presetdata::Presets::load_from(&root)?;
    if let Ok(asset_root) = paths::assets_root() {
        p.set_asset_root(&asset_root);
    }
    Ok(p)
}

/// 工作台窗口的标签。与客户端的 `main` 分开，`get_webview_window` 拿的是各自那一个
const WINDOW_LABEL: &str = "workbench";

/// 窗口标题。**测试模式那一份带前缀** —— 标题栏是唯一一处"切到别的窗口也还看得见"
/// 的地方（任务栏、Alt+Tab、截图里都在），作者要的"时时刻刻知道我在哪一边"就靠它兜底。
fn window_title() -> String {
    if paths::sandbox_on() {
        format!("{SANDBOX_TITLE_PREFIX}SupportEase 工作台")
    } else {
        "SupportEase 工作台".to_owned()
    }
}

/// 测试模式下标题的前缀（一眼分得开）
const SANDBOX_TITLE_PREFIX: &str = "【测试模式】";

/// 把窗口标题按**现在**的模式写一遍。
///
/// 开窗时用 [`window_title`]，切换模式时由那几条命令调这里 —— 标题不是启动一次的
/// 快照，它得跟着模式走，否则人会对着一个说"正式"的标题栏看沙箱。
pub fn refresh_window_title(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(WINDOW_LABEL) {
        let _ = win.set_title(&window_title());
    }
}

/// 开工作台窗口。已经开着就聚焦，不重复开第二个。
///
/// 刻意**不带** transparent / titleBarStyle=Overlay 这些客户端的窗口花活：
/// 工作台是密集表格界面，原生标题栏反而省事，也不需要为它再写一套拖拽与缩放边。
pub fn open_window(app: &AppHandle) -> Result<(), AppError> {
    if let Some(win) = app.get_webview_window(WINDOW_LABEL) {
        let _ = win.set_focus();
        return Ok(());
    }

    let win =
        WebviewWindowBuilder::new(app, WINDOW_LABEL, WebviewUrl::App("workbench.html".into()))
            .title(window_title())
            .inner_size(1360.0, 900.0)
            .min_inner_size(900.0, 560.0)
            .resizable(true)
            /*
             * ★ **先藏着，等前端首帧就绪再露面**（作者 2026-10-07）：
             * dev 下前端首次加载要把整棵模块树编译一遍（以分钟计），窗口若当场就露，
             * 人看到的是"一块白板先出现、然后干等"—— 作者原话"不要先出现白屏然后再等"。
             * 露面那一半在前端入口（`src/workbench/main.tsx` 首帧后调 `window.show()`）；
             * 这里同时挂**兜底**（见下）：前端因任何原因没来敲（编译错误 / JS 崩），
             * 藏着的窗口不会有任何报错出口 —— 到点无条件 show，宁可白板也不能永不出现。
             */
            .visible(false)
            .build()
            .map_err(|e| AppError::internal("建不出工作台窗口").with_detail(e.to_string()))?;

    {
        let win = win.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(60));
            let _ = win.show();
        });
    }

    /* 草稿在内存里，磁盘上只有崩溃快照。落盘三个时机里的两个挂在窗口事件上：
    失焦（去别的窗口了，这会儿写不碍事）与关闭前（最后一次机会）。
    第三个是「空闲 2 秒」，由下面那个计时器管。 */
    win.on_window_event(|e| match e {
        tauri::WindowEvent::Focused(false) | tauri::WindowEvent::CloseRequested { .. } => {
            app::flush_now();
        }
        _ => {}
    });

    // 第三个时机是「空闲 2 秒」，由一个每秒醒一次的线程管。
    // 放在窗口创建之后：没有工作台窗口时这件事根本不该发生
    app::spawn_flusher();

    tracing::info!("工作台窗口已打开");
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Roots {
    /// 开发源数据根（仓库里的 `workbench/`）
    pub workbench: String,
    /// 预设真相源（仓库里的 `presets/`）—— 机型 / 参数 / 套餐 / 资产 / 交付产物都在它下面
    pub presets: String,
    /// 交付目录（仓库里的 `presets/delivery/`）
    pub delivery: String,
}
