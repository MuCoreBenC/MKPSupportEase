//! **自动检查软件更新**（2026-11-05）：挂在「客户端成功拉到云端目录」之后，顺路查一次
//! [`crate::runtime::release_info`]。
//!
//! ## 三条硬规矩
//!
//! 1. **不进启动 / 首屏路径**（铁律 2：启动零网络）。入口只有一处：
//!    [`crate::ipc::catalog::check_remote_update`] 成功返回前。拉目录本来就要联网，
//!    搭这一趟，不新开定时轮询 —— 用户多了也不会把公共仓库打爆。
//! 2. **本会话只查一次**。进程内一个 `AtomicBool`；开关关着时不占这个名额。
//!    查失败（离线 / 没发过版 / 源没配）把名额放回去，后面联网了还能再试。
//! 3. **失败一律静默**。这一层是"顺路"，不是功能：不许弹错、不许把目录检查
//!    拖成失败。有新版才推一个事件，由前端右上角通知决定怎么显示。
//!
//! 版本比较、地址解析、release.json 的形状都不在这里重写 —— 走
//! [`fetch_update`] 这一个口子；设置页手动「重新检查」也调同一个口子。

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Emitter as _};

use crate::error::AppError;
use crate::fsx::paths::internal_root;
use crate::runtime::{app_state, net, release_info, source};

/// 前端监听的事件名（顶层只挂一次监听）。
pub const UPDATE_AVAILABLE_EVENT: &str = "software-update-available";

/// 本会话是否已经查过（`true` 之前的那次不论成功失败都算一次成功的"不再打扰"）。
static CHECKED: AtomicBool = AtomicBool::new(false);

/// 推给前端的那点信息。**版本号以外都不带** —— 通知只负责"有新版、点我过去"，
/// 详情（说明 / 下载地址 / 更新按钮）由设置页「软件更新」那一块自己读。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAvailable {
    pub version: String,
}

/// 取一份「有没有新版本」的结论。
///
/// 地址从 Source Manifest 的 `release_url()` 来（不许从 base_url 上跳一级去猜），
/// 版本比较用 [`release_info::to_update`]（`-dev` 后缀忽略、严格新才算）。
/// 设置页手动检查与自动顺路查**共用这一处**，避免两条链各读一份 release.json。
pub fn fetch_update(root: &Path) -> Result<release_info::SoftwareUpdate, AppError> {
    let resolved = source::resolve_source(root)?;
    let release_url = resolved.release_url()?;
    let bytes = net::get_release(&release_url)?;
    let info = release_info::parse(&bytes)?;
    Ok(release_info::to_update(&info))
}

/// 拉目录成功之后调用：**能查就查，不能查就当没这回事**。
///
/// 全程不 `unwrap`、不向上抛；调用方不必处理返回值。
pub fn maybe_check_after_catalog(app: &AppHandle) {
    /* ① 开关：盘上"没选过"= 开（解释权在 app_state，这里不自己猜） */
    let root = match internal_root(app) {
        Ok(root) => root,
        Err(e) => {
            tracing::debug!("自动检查更新跳过（拿不到数据根）：{}", e.message);
            return;
        }
    };
    let enabled = match app_state::auto_check_update(&root) {
        Ok(enabled) => enabled,
        Err(e) => {
            tracing::debug!("自动检查更新跳过（偏好读不回来）：{}", e.message);
            return;
        }
    };
    if !enabled {
        return;
    }

    /* ② 本会话去重：先抢名额再联网，两个窗口同时拉目录也只打一次 */
    if CHECKED.swap(true, Ordering::AcqRel) {
        return;
    }

    /* ③ 查 —— 失败静默，且把名额放回去（离线时别把这辈子的一次机会用掉） */
    match fetch_update(&root) {
        Ok(update) if update.has_update => {
            let payload = UpdateAvailable {
                version: update.latest_version,
            };
            if let Err(e) = app.emit(UPDATE_AVAILABLE_EVENT, payload) {
                tracing::warn!("自动检查更新：通知事件没发出去：{e}");
            }
        }
        Ok(_) => {} // 已是最新
        Err(e) => {
            tracing::debug!("自动检查更新静默跳过：{}", e.message);
            CHECKED.store(false, Ordering::Release);
        }
    }
}
