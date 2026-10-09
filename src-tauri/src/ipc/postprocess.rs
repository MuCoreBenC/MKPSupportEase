//! 钩子模式那三条命令 —— 界面看那一趟、停那一趟、答那一问。
//!
//! 这一趟本身住 [`crate::hook_ui`]（后台线程 + 一条取消线 + 一个问句的等待）。
//! **普通模式里没有对象**：从桌面图标起来时那一格是 `None`，[`get_post_process_run`] 照实答 `None`
//! （"没有在跑后处理"不是错误），另两条则会报"现在没有在跑后处理" —— 它们只有界面上那条模态框会调。

use tauri::State;

use crate::error::AppError;
use crate::hook_ui::{HookRun, HookSlot, RunSnapshot};

use super::traced;

/// 这一趟在不在。不在就报错说清（**不静默什么都不做**：界面点了"停止"却什么都没发生，
/// 用户会以为停了，而那一趟还在跑）。
fn running(slot: &HookSlot) -> Result<&std::sync::Arc<HookRun>, AppError> {
    slot.0
        .as_ref()
        .ok_or_else(|| AppError::not_found("现在没有在跑后处理"))
}

/// 钩子那一趟的**全量快照**：窗口起来时读一次。
///
/// 进度 / 待答的问句 / 结论都在里面 —— 只有事件是不够的：窗口起来时那一趟可能已经跑到一半、
/// 甚至已经跑完（早期的失败尤其快），只靠事件那一屏会永远停在"准备中"（见 [`crate::hook_ui`] 模块头）。
#[tauri::command]
pub fn get_post_process_run(slot: State<'_, HookSlot>) -> Result<Option<RunSnapshot>, AppError> {
    traced("getPostProcessRun", |_| Ok(slot.0.as_ref().map(|run| run.snapshot())))
}

/// 界面那颗「停止」：请求取消。
///
/// 取消落在内核的**步骤边界**上（写盘前还有最后一道），所以按下去之后原文件不会被写坏；
/// 顺带把"等你回答机型那一问"的等待也放开（按停止 = 不跑）。
#[tauri::command]
pub fn cancel_post_process(slot: State<'_, HookSlot>) -> Result<(), AppError> {
    traced("cancelPostProcess", |_| {
        running(&slot)?.stop();
        Ok(())
    })
}

/// 回答"机型不匹配还跑不跑"：`keep = true` 继续，`false` 停下。
#[tauri::command]
pub fn answer_post_process_mismatch(
    slot: State<'_, HookSlot>,
    keep: bool,
) -> Result<(), AppError> {
    traced("answerPostProcessMismatch", |_| {
        running(&slot)?.answer(keep);
        Ok(())
    })
}
