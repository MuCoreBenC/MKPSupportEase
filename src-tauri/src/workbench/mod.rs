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
//! - [`upstream`] 上游只读层（`mkpse-presets`，零写入函数）
//! - [`domain`] 规则与派生（三层取值、归并、可见性、patch、状态、预览、文案）
//! - [`app`] IPC 入口与落盘（**写只有 `wb_apply_draft` 一条**）
//!
//! 留用的底座：[`paths`]（三个数据根）/ [`clock`]（时间戳唯一来源）/
//! [`store`]（原子写、id 白名单、通用 JSON IO、草稿/快照/回收站的落盘位置）。
//!
//! **第二版的 model / resolve / bbs / catalog / generate / publish / preflight / state
//! 全部删掉了**，不是留着改。它们的输入面是我自己造的 10 条字段定义与两层取值，
//! 而这一版字段定义来自上游 74 条、取值是三层 —— 留着等于在一个专为消灭双轨的
//! 重做里先建一条双轨。其中有价值的部分（TOML 渲染与转义、原子替换、catalog 最后写、
//! 状态按 hash 比对）在 doc §8–§10 里逐条记着，Task 9 照着重写。

pub mod app;
pub mod clock;
pub mod domain;
pub mod paths;
pub mod store;
pub mod upstream;


use serde::Serialize;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::error::AppError;

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

    WebviewWindowBuilder::new(app, WINDOW_LABEL, WebviewUrl::App("workbench.html".into()))
        .title("SupportEase 后厨工作台")
        .inner_size(1360.0, 900.0)
        .min_inner_size(900.0, 560.0)
        .resizable(true)
        .build()
        .map_err(|e| AppError::internal("建不出工作台窗口").with_detail(e.to_string()))?;

    tracing::info!("工作台窗口已打开");
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Roots {
    /// 开发源数据根（仓库里的 `workbench/`）
    pub workbench: String,
    /// 发布目录（仓库里的 `dist-presets/`）
    pub dist: String,
    /// 上游预设仓库（只读）。定位不到时是 `None` —— **这时工作台不该启动业务**
    pub upstream: Option<String>,
}
