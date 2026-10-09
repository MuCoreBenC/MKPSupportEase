//! **界面那一侧**：把钩子推过来的那一趟显示出来（进度 / 问一句 / 结论）。
//!
//! # 谁跑、谁看（与旧世代 `mkp-ssr` 同一分工）
//!
//! ```text
//! 钩子进程（切片器起的，跑完就退）        界面进程（常驻：窗口留着、下次复用）
//!   hook::run_with_channel(...)  ──NDJSON──►  hook_ipc::Listener::serve
//!                                                  │  本模块：存快照 + 推事件
//!                                                  ▼
//!                                            模态框（进度 / 问句 / 结论）
//!                                                  │  取消 / 答复
//!   ◄──────────────────────────────────────────────┘
//! ```
//!
//! 窗口**不住在钩子进程里**（那样"跑完不自动关"就等于"切片器一直卡着"）——
//! 理由与四条通道纪律写在 [`crate::hook_ipc`] 的模块头。
//!
//! # 三条别改坏的地方
//!
//! 1. **快照必须有**：界面起来时那一趟可能已经跑到一半、甚至已经跑完（早期的失败尤其快）。
//!    只靠事件的话，那一屏会永远停在"准备中" —— 所以每条消息都同时存进 [`HookView`]，
//!    窗口起来先读一次全量快照（`ipc::postprocess::get_post_process_run`），之后才跟事件。
//! 2. **结论留在屏上**：跑完之后**不自动清**，也不自动关窗 —— 窗口留着、下次切片复用
//!    （旧世代同一条：结果态留到下一次 `progress` 顶掉它）。用户看完自己关掉那一屏。
//! 3. **界面说了不算**：退出码与成败判定都在钩子那一侧（切片器只认进程退出码）；
//!    这里那颗「停止」只是往通道里写一条 `cancel`，由钩子决定怎么收手。

use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter as _};

use crate::error::AppError;
use crate::hook_ipc::{self, FinishedPayload, FromHook, ProgressPayload, QuestionPayload, ToHook};

/// 一条进度（推给界面，也存进快照）。
pub const PROGRESS_EVENT: &str = "postprocess-progress";
/// 要用户回答一个问题（目前只有"机型不匹配还跑不跑"）。
pub const QUESTION_EVENT: &str = "postprocess-question";
/// 跑完了（成功 / 失败 / 取消都走这一条），带上退出码。
pub const FINISHED_EVENT: &str = "postprocess-finished";
/// 新的一趟开始了（界面据此把上一趟的结论与"我关过了"一起清掉）。
pub const STARTED_EVENT: &str = "postprocess-started";

/// 一条进度长什么样（与 Rust 侧 [`ProgressPayload`] 一一对应；前端只认字段名）。
pub type ProgressDto = ProgressPayload;
/// 一个问题长什么样。
pub type QuestionDto = QuestionPayload;
/// 结论长什么样。
pub type FinishedDto = FinishedPayload;

/// 新一趟的开场。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartedDto {
    pub preset_name: String,
    pub gcode_name: String,
}

/// 窗口起来时读的那份**全量快照**（进度 / 待答的问句 / 结论都在里面）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSnapshot {
    pub preset_name: String,
    pub gcode_name: String,
    pub question: Option<QuestionDto>,
    pub progress: Option<ProgressDto>,
    pub finished: Option<FinishedDto>,
}

/// Tauri `manage` 那一格里放的东西（命令从它那三条读 / 写）。
pub struct HookSlot(pub Arc<HookView>);

/// 界面这一侧那一趟的状态。
pub struct HookView {
    shared: Mutex<Shared>,
    /// 回写线（取消 / 答复）：界面一起来就装上，之后一直用同一条
    writer: Mutex<Option<hook_ipc::Handle>>,
}

#[derive(Default)]
struct Shared {
    preset_name: Option<String>,
    gcode_name: Option<String>,
    question: Option<QuestionDto>,
    progress: Option<ProgressDto>,
    finished: Option<FinishedDto>,
}

impl HookView {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            shared: Mutex::new(Shared::default()),
            writer: Mutex::new(None),
        })
    }

    /// 装上回写线（由 `lib.rs` 在起监听器时调一次）。
    pub fn attach_writer(&self, handle: hook_ipc::Handle) {
        *self.writer.lock().expect("锁没坏") = Some(handle);
    }

    /// 通道来了一条消息：存快照 + 推事件。
    pub fn apply(&self, app: &AppHandle, msg: FromHook) {
        match msg {
            FromHook::Hello {
                preset_name,
                gcode_name,
            } => {
                /* 开场与收场各一条 INFO：出问题时"这一趟到底有没有被界面收到"是第一个要问的 */
                tracing::info!(gcode = %gcode_name, preset = %preset_name, "钩子来了：这一趟开始");
                {
                    let mut g = self.shared.lock().expect("锁没坏");
                    /* 新一趟：上一趟的结论与问句一起清掉（旧世代：结果态留到下一次顶掉它） */
                    *g = Shared {
                        preset_name: Some(preset_name.clone()),
                        gcode_name: Some(gcode_name.clone()),
                        ..Shared::default()
                    };
                }
                let _ = app.emit(
                    STARTED_EVENT,
                    StartedDto {
                        preset_name,
                        gcode_name,
                    },
                );
            }
            FromHook::Progress(progress) => {
                self.shared.lock().expect("锁没坏").progress = Some(progress.clone());
                let _ = app.emit(PROGRESS_EVENT, progress);
            }
            FromHook::Question(question) => {
                self.shared.lock().expect("锁没坏").question = Some(question.clone());
                let _ = app.emit(QUESTION_EVENT, question);
            }
            FromHook::Finished(finished) => {
                tracing::info!(
                    exit_code = finished.exit_code,
                    ok = finished.ok,
                    cancelled = finished.cancelled,
                    "钩子那一趟结束"
                );
                {
                    let mut g = self.shared.lock().expect("锁没坏");
                    /* 结论已出：那一问不可能还有人在等，清掉免得界面还挂着它 */
                    g.question = None;
                    g.finished = Some((*finished).clone());
                }
                let _ = app.emit(FINISHED_EVENT, *finished);
            }
        }
    }

    /// 全量快照；这一趟**从来没来过**（普通模式、或者还没切片）⇒ `None`。
    pub fn snapshot(&self) -> Option<RunSnapshot> {
        let g = self.shared.lock().expect("锁没坏");
        let preset_name = g.preset_name.clone()?;
        Some(RunSnapshot {
            preset_name,
            gcode_name: g.gcode_name.clone().unwrap_or_default(),
            question: g.question.clone(),
            progress: g.progress.clone(),
            finished: g.finished.clone(),
        })
    }

    /// 有没有钩子连着（界面据此把「停止」变灰：点了也没对象）。
    pub fn hook_connected(&self) -> bool {
        self.writer
            .lock()
            .expect("锁没坏")
            .as_ref()
            .is_some_and(hook_ipc::Handle::connected)
    }

    /// 界面那颗「停止」：往通道里写一条取消。
    pub fn cancel(&self) -> Result<(), AppError> {
        self.write(ToHook::Cancel, "现在没有在跑后处理")
    }

    /// 回答"机型不匹配还跑不跑"。
    pub fn answer(&self, keep: bool) -> Result<(), AppError> {
        self.write(ToHook::Answer { keep }, "现在没有在问你要不要继续")
    }

    fn write(&self, msg: ToHook, absent: &str) -> Result<(), AppError> {
        if !self.hook_connected() {
            /* **失败必须长得像失败**：界面点了"停止"却什么都没发生，用户会以为停了，
            而那一趟还在跑（旧世代同一条：宁可报错，不许静默） */
            return Err(AppError::not_found(absent));
        }
        let guard = self.writer.lock().expect("锁没坏");
        if let Some(handle) = guard.as_ref() {
            handle.send(msg);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 快照的三态：没来过 → `None`；来了进度 → 看得到；来了结论 → 问句被清掉、结论留下
    #[test]
    fn the_snapshot_follows_the_run() {
        let view = HookView::new();
        assert!(view.snapshot().is_none(), "没来过就该是 None（不是空壳）");

        /* `apply` 要 AppHandle（推事件用），这里只走"存快照"那一半：
        把消息按通道里的顺序喂进 shared —— 与 `apply` 同一段逻辑的判据在
        `hook_ipc::tests::the_wire_carries_both_ways` 里（那条走真连接） */
        {
            let mut g = view.shared.lock().expect("锁没坏");
            *g = Shared {
                preset_name: Some("A1_MINI-fast.toml".to_owned()),
                gcode_name: Some("45600.0.gcode".to_owned()),
                progress: Some(ProgressPayload {
                    step: "pass1".to_owned(),
                    fraction_in_step: Some(0.5),
                    message: "第一遍处理中".to_owned(),
                }),
                question: Some(QuestionPayload {
                    kind: "machine-mismatch".to_owned(),
                    text: "还继续吗？".to_owned(),
                }),
                finished: None,
            };
        }
        let snap = view.snapshot().expect("来过就该有快照");
        assert_eq!(snap.preset_name, "A1_MINI-fast.toml");
        assert!(snap.question.is_some());
        assert_eq!(snap.progress.expect("有进度").step, "pass1");

        /* 结论一来：问句清掉、结论留下（界面不该还挂着一条已经不可能有人答的问题） */
        view.shared.lock().expect("锁没坏").question = None;
        view.shared.lock().expect("锁没坏").finished = Some(FinishedPayload {
            ok: false,
            cancelled: false,
            message: "模型超出打印边界".to_owned(),
            code: Some("E_GCODE_BOUNDARY_001".to_owned()),
            stage: Some("parse".to_owned()),
            exit_code: 1,
            elapsed_ms: 42,
            output: None,
            warnings: Vec::new(),
        });
        let snap = view.snapshot().expect("结论也在快照里");
        assert!(snap.question.is_none());
        assert_eq!(snap.finished.expect("有结论").exit_code, 1);
    }

    /// 没有钩子连着时，那两条命令**报错**（不静默什么都不做）
    #[test]
    fn without_a_hook_the_buttons_report_it() {
        let view = HookView::new();
        assert!(view.cancel().is_err(), "没对象时要报错");
        assert!(view.answer(true).is_err(), "没对象时要报错");
        assert!(!view.hook_connected());
    }
}
