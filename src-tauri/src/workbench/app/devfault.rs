//! **开发台 · 故障注入的三条命令**（工作台这一侧的薄壳）。
//!
//! 档位表与"怎么造那条错"在 [`crate::devfault`]（**无条件编译**）—— 造错发生在
//! **客户端进程**里，而客户端是普通构建（不带 `workbench` feature）。这里只做三件事：
//! 报清单 / 装 / 卸。
//!
//! # 怎么过线（与 `devsim::wb_dev_trigger` 同一条路）
//!
//! 工作台窗口里**没有客户端窗口**（`tauri.workbench.conf.json` 的 `app.windows = []`），
//! 所以只能跨进程，走 hook 那条 loopback：
//!
//! ```text
//!   工作台 ──invoke──► wb_dev_fault_arm(id) ──loopback──► 客户端进程
//!                                                            │ 客户端按 id 自己造那条错
//!                                                            ▼
//!                                            emit `dev-fault` → 前端记下「方法 → 错」
//!                                                            │
//!                                            src/api/bridge.ts 的 call() 查表 → reject
//! ```
//!
//! ★ 跨进程只传**一个 id**：造错在那一侧做，于是演示出来的与真机走同一份代码。

use crate::devfault::{self, DevFault};
use crate::error::AppError;
use crate::fsx::paths;
use crate::hook_ipc::{Client, FromHook};
use postprocess::diag::CancelToken;

/// 连客户端那条 loopback（**客户端那棵树**，不是工作台这棵）。
///
/// 与 `devsim::wb_dev_trigger` 那个 ★★ 同一个坑：`app_data_dir()` 给的是工作台自己
/// 那棵（`…/SupportEase-Workbench`），而客户端监听器绑在 `…/SupportEase`
/// （`fsx::paths::APP_IDENTIFIER`）。发错树 = 消息石沉大海。
fn to_client() -> Result<Client, AppError> {
    let Some(root) = paths::internal_root_headless() else {
        return Err(AppError::internal("算不出客户端内部数据根"));
    };
    let cancel = CancelToken::new();
    let client = Client::connect_or_headless(&root, &cancel);
    if !client.connected() {
        return Err(AppError::not_found(
            "客户端窗口没启动 —— 先跑 npm run tauri dev",
        ));
    }
    Ok(client)
}

/// 开发台能装哪些故障 —— 界面照它摆按钮。
#[tauri::command(async)]
pub fn wb_dev_faults() -> Result<Vec<DevFault>, AppError> {
    crate::ipc::traced("wb_dev_faults", |_| Ok(devfault::faults()))
}

/// **装一档故障**：让客户端窗口里所有走这个方法的界面从此真的报这一条错。
#[tauri::command]
pub fn wb_dev_fault_arm(id: String) -> Result<(), AppError> {
    crate::ipc::traced("wb_dev_fault_arm", |_| {
        let row = devfault::faults()
            .into_iter()
            .find(|r| r.id == id)
            .ok_or_else(|| AppError::invalid_argument(format!("不认识的故障：{id}")))?;
        to_client()?.send(&FromHook::DevFault { fault: Some(id) });
        tracing::info!(method = %row.method, "开发台：已装故障");
        Ok(())
    })
}

/// **卸下故障**：客户端恢复正常。
#[tauri::command]
pub fn wb_dev_fault_clear() -> Result<(), AppError> {
    crate::ipc::traced("wb_dev_fault_clear", |_| {
        to_client()?.send(&FromHook::DevFault { fault: None });
        tracing::info!("开发台：已卸故障");
        Ok(())
    })
}
