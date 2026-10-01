//! 后厨工作台的 Rust 侧。
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
            .title("SupportEase 后厨工作台")
            .inner_size(1360.0, 900.0)
            .min_inner_size(900.0, 560.0)
            .resizable(true)
            .build()
            .map_err(|e| AppError::internal("建不出工作台窗口").with_detail(e.to_string()))?;

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
    /// 交付产物目录（仓库里的 `presets/dist/`）
    pub dist: String,
}
