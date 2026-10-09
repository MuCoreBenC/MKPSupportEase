//! 钩子模式下的**界面那一半**：把那一趟挂到窗口上 —— 推进度、问一句、按"停止"、跑完带退出码退。
//!
//! # 为什么要有这一半
//!
//! 切片器那 95% 那一步（"正在运行后处理脚本"）在等**这个进程**退出。所以钩子那一趟不能闷头跑，
//! 也不能开着窗口等用户关：**进度要看得见、卡住要停得下来、跑完自己退**。
//! 形状照成熟版 `mkpsupporte`（它的主窗口 + `ProcessingModal` + 那颗"停止"）。
//!
//! # 谁跑、谁看
//!
//! ```text
//! 主线程            Tauri 事件循环（窗口、模态框、那颗"停止"）
//! 后台线程（本模块） hook::run(...) —— 进度推事件、问句等答复、跑完 app.exit(码)
//! ```
//!
//! 两个方向的通道各一条，**都是旁路，不参与成败判定**：
//!
//! - 出去：[`PROGRESS_EVENT`]（进度）/ [`QUESTION_EVENT`]（要用户答一句）/ [`FINISHED_EVENT`]（结论）
//! - 进来：`ipc::postprocess` 那三条命令（读快照 / 取消 / 答复）
//!
//! # 三条别改坏的地方
//!
//! 1. **快照必须有**：窗口起来时那一趟可能已经跑到一半、甚至已经跑完（早期的失败尤其快）。
//!    只靠事件的话，那一屏会永远停在"准备中" —— 成熟版踩过（spec `fix-gui-postprocess-activate-ux R3`
//!    写的就是"早期校验失败也必须先出模态框"）。所以 [`PROGRESS_EVENT`] 与 [`FINISHED_EVENT`]
//!    的每一份都同时存进 [`HookRun`]，由 `ipc::postprocess::get_post_process_run` 一次给全。
//! 2. **等答复有上限**：没人答（窗口没起来 / 用户走开了）就是**不跑**，不是默认继续 ——
//!    替用户决定"机型不匹配也照跑"是这一层最不该做的事。
//! 3. **退出码由这一趟决定**，不是界面上那颗按钮：切片器只认进程退出码（`0` / `1` / `2`），
//!    与 stderr 那一行结论一起构成它对外的全部契约（见 [`crate::hook`]）。

use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter as _};

use postprocess::diag::CancelToken;
use postprocess::pipeline::{ProgressEvent, ProgressSink};

use crate::args::HookJob;
use crate::hook::{self, Asker, MismatchPolicy};

/// 进度（每一条内含阶段 id / 步内比例 / 一句消息）。**全局百分比不在这里** ——
/// 阶段名与权重是界面的事（内核 `pipeline::progress` 的模块头写着这条边界）。
pub const PROGRESS_EVENT: &str = "postprocess-progress";
/// 要用户回答一个问题（目前只有"机型不匹配还跑不跑"）。
pub const QUESTION_EVENT: &str = "postprocess-question";
/// 跑完了（成功 / 失败 / 取消都走这一条），带上退出码。
pub const FINISHED_EVENT: &str = "postprocess-finished";

/// 等用户答复的上限。到点 = **不跑**（见模块头第 2 条）。
const ANSWER_TIMEOUT: Duration = Duration::from_secs(120);

/// 跑完之后窗口留多久再看一眼结果 —— 然后自动退，切片器才走得下去。
/// 成功短（切片器在等）、失败与取消长（那句话要读）。
const HOLD_OK: Duration = Duration::from_secs(4);
const HOLD_BAD: Duration = Duration::from_secs(20);

/// 一条进度（推给界面，也存进快照）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressPayload {
    /// 阶段 id（内核的稳定标识：`input` / `pass1` …）—— 中文名归界面
    pub step: String,
    /// 步内比例 `0.0..=1.0`；`None` = 这一步此刻给不出比例
    pub fraction_in_step: Option<f32>,
    pub message: String,
}

/// 一个问题（推给界面，也存进快照）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionPayload {
    /// 问题种类（界面按它决定给什么按钮）：`machine-mismatch` = 继续 / 停下
    pub kind: String,
    pub text: String,
}

/// 结论（推给界面，也存进快照）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinishedPayload {
    pub ok: bool,
    /// 用户按了停止（或 Ctrl-C）—— 不是"失败"，界面说法不一样
    pub cancelled: bool,
    /// 一句话结论（与 stderr 那一行同一句）
    pub message: String,
    /// 给切片器的退出码
    pub exit_code: u8,
    pub elapsed_ms: u128,
    /// 产物落点（成功时是输入那一份 —— 原地覆盖）
    pub output: Option<String>,
    pub warnings: Vec<String>,
}

/// 一次跑的**全量快照**：窗口起来时先读它，之后跟着事件走。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSnapshot {
    pub preset_name: String,
    pub gcode_name: String,
    pub question: Option<QuestionPayload>,
    pub progress: Option<ProgressPayload>,
    pub finished: Option<FinishedPayload>,
}

/// Tauri `manage` 那一格里放的东西。普通模式（从桌面图标起来的）里是 `None` ——
/// 界面据此知道"现在没有在跑后处理"，那三条命令也就不会被调。
pub struct HookSlot(pub Option<Arc<HookRun>>);

/// 钩子那一趟与界面共享的那点状态。
pub struct HookRun {
    job: HookJob,
    cancel: CancelToken,
    shared: Mutex<Shared>,
    /// 答复到达 / 取消时唤醒等在那一问上的后台线程
    answered: Condvar,
    /// 结论落地时唤醒等它的人（用户把窗关了之后那一小段收尾，见 [`HookRun::wait_finished`]）
    done: Condvar,
}

#[derive(Default)]
struct Shared {
    question: Option<QuestionPayload>,
    /// 用户的答复（`None` = 还没答）
    answer: Option<bool>,
    progress: Option<ProgressPayload>,
    finished: Option<FinishedPayload>,
}

impl HookRun {
    pub fn new(job: HookJob) -> Self {
        Self {
            job,
            cancel: CancelToken::new(),
            shared: Mutex::new(Shared::default()),
            answered: Condvar::new(),
            done: Condvar::new(),
        }
    }

    pub fn job(&self) -> &HookJob {
        &self.job
    }

    /// 给内核的取消线（它每步入口都查）。
    pub fn cancel_token(&self) -> CancelToken {
        self.cancel.clone()
    }

    /// 界面那颗"停止"：请求取消。
    ///
    /// **顺带把等在那一问上的线程放开**（答"不跑"）—— 成熟版同一条：取消要能打断
    /// "等你决定"那个等待，否则用户按了停，进程还杵在那儿等答复。
    pub fn stop(&self) {
        self.cancel.cancel();
        let mut g = self.shared.lock().expect("锁没坏");
        if g.answer.is_none() {
            g.answer = Some(false);
        }
        self.answered.notify_all();
    }

    /// 用户对那一问的答复（`keep = true` 继续跑）。
    pub fn answer(&self, keep: bool) {
        let mut g = self.shared.lock().expect("锁没坏");
        g.answer = Some(keep);
        self.answered.notify_all();
    }

    /// 全量快照（窗口起来时读一次）。
    pub fn snapshot(&self) -> RunSnapshot {
        let g = self.shared.lock().expect("锁没坏");
        RunSnapshot {
            preset_name: file_name_of(&self.job.toml),
            gcode_name: file_name_of(&self.job.gcode),
            question: g.question.clone(),
            progress: g.progress.clone(),
            finished: g.finished.clone(),
        }
    }

    /// 问一句：先推给界面，再等答复（超时 / 被停止 ⇒ `false`）。
    fn ask(&self, app: &AppHandle, question: &str) -> bool {
        let payload = QuestionPayload {
            kind: "machine-mismatch".to_owned(),
            text: question.to_owned(),
        };
        {
            let mut g = self.shared.lock().expect("锁没坏");
            g.question = Some(payload.clone());
            g.answer = None;
        }
        let _ = app.emit(QUESTION_EVENT, payload);

        let deadline = Instant::now() + ANSWER_TIMEOUT;
        let mut g = self.shared.lock().expect("锁没坏");
        while g.answer.is_none() {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            let (guard, _) = self.answered.wait_timeout(g, left).expect("锁没坏");
            g = guard;
        }
        /* 到点没答 = 不跑（模块头第 2 条）。`wait_timeout` 超时也走这一支 */
        let answer = g.answer.unwrap_or(false);
        g.question = None;
        answer
    }

    fn record_progress(&self, payload: ProgressPayload) {
        let mut g = self.shared.lock().expect("锁没坏");
        g.progress = Some(payload);
    }

    fn record_finished(&self, payload: FinishedPayload) {
        let mut g = self.shared.lock().expect("锁没坏");
        g.finished = Some(payload);
        /* 结论已出：那一问不可能还有人在等，清掉免得快照里挂着一条过期的问句 */
        g.question = None;
        self.done.notify_all();
    }

    /// 等这一趟落地（`Some(退出码)`）；超时给 `None`。
    ///
    /// **谁在等**：用户把窗关了那一刻（事件循环结束）—— 那一趟可能还在跑，
    /// 不能就这么返回 0：切片器会把 0 当成"处理成功"，接着去打印一份没处理过的 G-code。
    pub fn wait_finished(&self, timeout: Duration) -> Option<u8> {
        let deadline = Instant::now() + timeout;
        let mut g = self.shared.lock().expect("锁没坏");
        while g.finished.is_none() {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return None;
            }
            let (guard, _) = self.done.wait_timeout(g, left).expect("锁没坏");
            g = guard;
        }
        g.finished.as_ref().map(|f| f.exit_code)
    }
}

fn file_name_of(p: &std::path::Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| p.display().to_string())
}

/// 界面那一半的进度出口。推不出去**不影响这一趟** —— 与内核"进度是旁路"同一立场。
struct EventSink {
    app: AppHandle,
    run: Arc<HookRun>,
}

impl ProgressSink for EventSink {
    fn emit(&mut self, event: ProgressEvent) {
        let payload = ProgressPayload {
            step: event.step.id().to_owned(),
            fraction_in_step: event.fraction_in_step,
            message: event.message,
        };
        self.run.record_progress(payload.clone());
        let _ = self.app.emit(PROGRESS_EVENT, payload);
    }
}

/// 界面那一半的问句：弹一屏、等答复。
struct WindowAsker {
    app: AppHandle,
    run: Arc<HookRun>,
}

impl Asker for WindowAsker {
    fn confirm(&mut self, question: &str) -> bool {
        self.run.ask(&self.app, question)
    }
}

/// 起钩子那一趟（后台线程）：跑 → 推结论 → 留一会儿 → **带退出码退**。
///
/// 为什么退出码在这一层定：`app.exit(code)` 之后进程才真的结束，而切片器等的正是它。
pub fn spawn(app: &AppHandle, run: Arc<HookRun>) {
    let app = app.clone();
    std::thread::spawn(move || {
        let started = Instant::now();
        let mut sink = EventSink {
            app: app.clone(),
            run: run.clone(),
        };
        let mut asker = WindowAsker {
            app: app.clone(),
            run: run.clone(),
        };

        let done = hook::run(
            run.job(),
            &mut sink,
            &run.cancel_token(),
            MismatchPolicy::Ask(&mut asker),
        );
        let payload = finished_payload(&done, started.elapsed().as_millis());
        run.record_finished(payload.clone());
        let _ = app.emit(FINISHED_EVENT, payload);

        /* stderr 那一行（切片器会把 stderr 给用户看）与退出码走同一条路 —— 两处口径不许各写一份 */
        let code = hook::report(done);

        std::thread::sleep(if code == hook::EXIT_OK { HOLD_OK } else { HOLD_BAD });
        app.exit(i32::from(code));
    });
}

fn finished_payload(done: &Result<hook::Outcome, hook::HookError>, elapsed_ms: u128) -> FinishedPayload {
    match done {
        Ok(out) => FinishedPayload {
            ok: true,
            cancelled: false,
            message: format!("处理完成：{}（{} ms）", out.output.display(), out.elapsed_ms),
            exit_code: hook::EXIT_OK,
            elapsed_ms: out.elapsed_ms,
            output: Some(out.output.display().to_string()),
            warnings: out.warnings.clone(),
        },
        Err(err) => FinishedPayload {
            ok: false,
            cancelled: err.cancelled(),
            message: err.to_string(),
            exit_code: err.exit_code(),
            elapsed_ms,
            output: None,
            warnings: Vec::new(),
        },
    }
}
