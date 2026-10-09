//! 钩子那一趟的三条命令 —— 界面看它、停它、答它那一问。
//!
//! 那一趟住在**钩子进程**里（切片器起的那个，跑完就退）；界面这一侧只有一条通道
//! （[`crate::hook_ipc`]）与一份显示状态（[`crate::hook_ui`]）。这三条命令因此都只做一件事：
//! 从那份状态里读一眼 / 往通道里写一行。
//!
//! **没有对象时报错**，不静默什么都不做：界面点了「停止」却什么都没发生，用户会以为停了，
//! 而那一趟还在跑（旧世代同一条纪律）。

use tauri::State;

use crate::error::AppError;
use crate::hook_ui::{HookSlot, RunSnapshot};

use super::traced;

/// 钩子那一趟的**全量快照**：窗口起来时读一次。
///
/// 进度 / 待答的问句 / 结论都在里面 —— 只有事件是不够的：界面起来时那一趟可能已经跑到一半、
/// 甚至已经跑完（早期的失败尤其快），只靠事件那一屏会永远停在"准备中"（见 [`crate::hook_ui`]）。
/// `None` = 这一趟从来没来过（普通模式，或者这台机器还没切过片）。
#[tauri::command]
pub fn get_post_process_run(slot: State<'_, HookSlot>) -> Result<Option<RunSnapshot>, AppError> {
    traced("getPostProcessRun", |_| Ok(slot.0.snapshot()))
}

/// 界面那颗「停止」：往通道里写一条取消。
///
/// 取消落在内核的**步骤边界**上（写盘前还有最后一道），所以按下去之后原文件不会被写坏；
/// 钩子那一侧的读线程收到就置位取消线 —— 它会**打断**"等你回答机型那一问"的等待。
#[tauri::command]
pub fn cancel_post_process(slot: State<'_, HookSlot>) -> Result<(), AppError> {
    traced("cancelPostProcess", |_| slot.0.cancel())
}

/// 回答"机型不匹配还跑不跑"：`keep = true` 继续，`false` 停下。
#[tauri::command]
pub fn answer_post_process_mismatch(slot: State<'_, HookSlot>, keep: bool) -> Result<(), AppError> {
    traced("answerPostProcessMismatch", |_| slot.0.answer(keep))
}
