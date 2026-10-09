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

/// 看门狗：整趟最多跑多久；多久没有任何进展就当卡住（照成熟版：30 分钟 / 10 分钟）。
const WATCHDOG_TOTAL: Duration = Duration::from_secs(30 * 60);
const WATCHDOG_IDLE: Duration = Duration::from_secs(10 * 60);
/// 看门狗多久看一次（它只是"偶尔抬头看一眼" —— 真正的取消仍走内核那条协作线）。
const WATCHDOG_TICK: Duration = Duration::from_secs(5);

/// 看门狗的判据。**纯函数**：真跑一次 30 分钟才能验的东西不算判据。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchdogVerdict {
    /// 正常（还在动，也没超时）
    Fine,
    /// 太久没有任何进展
    Idle,
    /// 整趟超时
    Overrun,
}

/// 现在该不该收手。
///
/// 先判**整趟超时**：走完了 30 分钟这件事比"最后 10 分钟没动"更硬。
pub fn watchdog_verdict(
    now: Instant,
    started: Instant,
    last_progress: Instant,
    idle_limit: Duration,
    total_limit: Duration,
) -> WatchdogVerdict {
    if now.duration_since(started) >= total_limit {
        return WatchdogVerdict::Overrun;
    }
    if now.duration_since(last_progress) >= idle_limit {
        return WatchdogVerdict::Idle;
    }
    WatchdogVerdict::Fine
}

/// 跑完之后窗口留多久再看一眼结果 —— 然后自动退，切片器才走得下去。
///
/// - 成功：几秒就够（切片器在等，用户不需要读什么）；
/// - **取消：一闪而过** —— 按钮是他自己按的，多留一秒都是"点了停止它还在那儿"（作者原话：
///   「他应该立刻，马上，一秒钟都不耽误的就停止」）；
/// - 失败：留久一点读原因。想立刻放切片器走，直接把这扇窗关掉即可
///   （关窗会带着这一趟的退出码退，见 [`crate::run_inner`] 末尾那段）。
const HOLD_OK: Duration = Duration::from_secs(4);
const HOLD_CANCELLED: Duration = Duration::from_millis(900);
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
    /// **原因**（人话）。码已经从这里剥掉了 —— 整句码塞给用户，他读不到到底哪儿不对
    /// （见 [`crate::hook::describe`]）
    pub message: String,
    /// 稳定错误码（`E_*_NNN`，报问题时带上它）。取消与没有码的几句是 `None`
    pub code: Option<String>,
    /// 停在哪一阶段（内核的阶段 id，界面转中文名）；还没出过进度时是 `None`
    pub stage: Option<String>,
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

struct Shared {
    question: Option<QuestionPayload>,
    /// 用户的答复（`None` = 还没答）
    answer: Option<bool>,
    progress: Option<ProgressPayload>,
    finished: Option<FinishedPayload>,
    /// 最近一条进度是什么时候（看门狗靠它判"卡住了"）
    last_progress_at: Instant,
    /// 停止的理由（用户按的 / 看门狗收的）—— 界面要说清是哪一种
    stop_reason: Option<String>,
}

impl Default for Shared {
    fn default() -> Self {
        Self {
            question: None,
            answer: None,
            progress: None,
            finished: None,
            last_progress_at: Instant::now(),
            stop_reason: None,
        }
    }
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

    /// 界面那颗「停止」：请求取消。
    ///
    /// **顺带把等在那一问上的线程放开**（答"不跑"）—— 成熟版同一条：取消要能打断
    /// "等你决定"那个等待，否则用户按了停，进程还杵在那儿等答复。
    pub fn stop(&self) {
        self.stop_with("已停止 —— 这一盘没有做后处理，原文件没有被改动（G-code 没有被覆盖）");
    }

    /// 带理由地停（看门狗用它，界面那颗「停止」用上面那句）。
    pub fn stop_with(&self, why: &str) {
        self.cancel.cancel();
        let mut g = self.shared.lock().expect("锁没坏");
        if g.answer.is_none() {
            g.answer = Some(false);
        }
        if g.stop_reason.is_none() {
            g.stop_reason = Some(why.to_owned());
        }
        self.answered.notify_all();
    }

    /// 停止的理由（没有就是 `None`）。
    fn stop_reason(&self) -> Option<String> {
        self.shared.lock().expect("锁没坏").stop_reason.clone()
    }

    /// 最近一条进度是什么时候（看门狗判"卡住了没有"）。
    fn last_progress_at(&self) -> Instant {
        self.shared.lock().expect("锁没坏").last_progress_at
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
        g.last_progress_at = Instant::now();
    }

    fn record_finished(&self, payload: FinishedPayload) {
        let mut g = self.shared.lock().expect("锁没坏");
        g.finished = Some(payload);
        /* 结论已出：那一问不可能还有人在等，清掉免得快照里挂着一条过期的问句 */
        g.question = None;
        self.done.notify_all();
    }

    /// 最近一条进度所在的阶段 —— 失败时界面用它说"停在哪一步"。
    pub fn last_step(&self) -> Option<String> {
        self.shared
            .lock()
            .expect("锁没坏")
            .progress
            .as_ref()
            .map(|p| p.step.clone())
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

        /* 看门狗：偶尔抬头看一眼"还在动吗"。它只**请求取消**（内核那条协作线负责真的停），
        理由留给界面说清是哪一种停（用户按的 / 超时收的） */
        let watched = run.clone();
        std::thread::spawn(move || watch(watched));

        let done = hook::run(
            run.job(),
            &mut sink,
            &run.cancel_token(),
            MismatchPolicy::Ask(&mut asker),
        );
        let payload = finished_payload(
            &done,
            started.elapsed().as_millis(),
            run.last_step(),
            run.stop_reason(),
        );
        let hold = if payload.ok {
            HOLD_OK
        } else if payload.cancelled {
            HOLD_CANCELLED
        } else {
            HOLD_BAD
        };
        run.record_finished(payload.clone());
        let _ = app.emit(FINISHED_EVENT, payload);

        /* stderr 那一行（切片器会把 stderr 给用户看）与退出码走同一条路 —— 两处口径不许各写一份 */
        let code = hook::report(done);

        std::thread::sleep(hold);
        app.exit(i32::from(code));
    });
}

fn finished_payload(
    done: &Result<hook::Outcome, hook::HookError>,
    elapsed_ms: u128,
    stage: Option<String>,
    stop_reason: Option<String>,
) -> FinishedPayload {
    match done {
        Ok(out) => FinishedPayload {
            ok: true,
            cancelled: false,
            message: format!("处理完成：{}（{} ms）", out.output.display(), out.elapsed_ms),
            code: None,
            stage,
            exit_code: hook::EXIT_OK,
            elapsed_ms: out.elapsed_ms,
            output: Some(out.output.display().to_string()),
            warnings: out.warnings.clone(),
        },
        Err(err) => {
            /* 原因与码分家：界面把原因摆正中、码摆小字（见 `hook::describe`）。
            取消那一档优先用**停止的理由**（用户按的？看门狗收的？）——
            只说"已停止"会把"为什么停"藏起来 */
            let (message, code) = hook::describe(err);
            let message = if err.cancelled() {
                stop_reason.unwrap_or(message)
            } else {
                message
            };
            FinishedPayload {
                ok: false,
                cancelled: err.cancelled(),
                message,
                code,
                stage,
                exit_code: err.exit_code(),
                elapsed_ms,
                output: None,
                warnings: Vec::new(),
            }
        }
    }
}

/// 看门狗那一圈：偶尔抬头看一次"还在动吗 / 是不是太久了"，到点就**请求取消**并留下理由。
///
/// 为什么要有它：切片器在等**这个进程**退出，而"卡住"会长这样 —— 进程活着、界面在画、
/// 切片器永远停在 95%（2026-10-09 同一天踩过两次：钩子开窗不退、等答复没人答）。
/// 成熟版同一条：30 分钟上限 + 10 分钟无进展。
fn watch(run: Arc<HookRun>) {
    let started = Instant::now();
    loop {
        /* 睡到下一次抬头（这一趟跑完会立刻醒 —— `wait_finished` 在结论落地时被唤醒） */
        if run.wait_finished(WATCHDOG_TICK).is_some() {
            return;
        }
        match watchdog_verdict(
            Instant::now(),
            started,
            run.last_progress_at(),
            WATCHDOG_IDLE,
            WATCHDOG_TOTAL,
        ) {
            WatchdogVerdict::Fine => {}
            WatchdogVerdict::Idle => {
                run.stop_with(&format!(
                    "超时停止：{} 分钟没有任何进展（多半是卡住了）—— 原文件没有被改动",
                    WATCHDOG_IDLE.as_secs() / 60
                ));
                return;
            }
            WatchdogVerdict::Overrun => {
                run.stop_with(&format!(
                    "超时停止：这一趟超过 {} 分钟 —— 原文件没有被改动",
                    WATCHDOG_TOTAL.as_secs() / 60
                ));
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 看门狗判据：正常 / 没进展 / 整趟超时三档，且**整趟超时优先**
    /// （真跑 30 分钟才能验的东西不算判据，所以这里比的是时间点）
    #[test]
    fn the_watchdog_speaks_up_only_when_it_should() {
        let idle = Duration::from_secs(600);
        let total = Duration::from_secs(1800);
        let started = Instant::now();

        // 刚起步：什么都不说
        assert_eq!(
            watchdog_verdict(started, started, started, idle, total),
            WatchdogVerdict::Fine
        );

        // 5 分钟前有过进度：还正常
        assert_eq!(
            watchdog_verdict(
                started + Duration::from_secs(300),
                started,
                started + Duration::from_secs(300),
                idle,
                total
            ),
            WatchdogVerdict::Fine
        );

        // 最后 10 分钟一动不动：卡住
        assert_eq!(
            watchdog_verdict(
                started + Duration::from_secs(700),
                started,
                started + Duration::from_secs(100),
                idle,
                total
            ),
            WatchdogVerdict::Idle
        );

        // 超过整趟上限：哪怕刚刚还有进度也收手（这一条比"没进展"更硬）
        assert_eq!(
            watchdog_verdict(
                started + Duration::from_secs(1801),
                started,
                started + Duration::from_secs(1800),
                idle,
                total
            ),
            WatchdogVerdict::Overrun
        );
    }

    /// 快照里的那几格：进度 / 问句 / 结论 / 停的理由都读得出来
    #[test]
    fn the_snapshot_carries_what_the_window_needs() {
        let run = HookRun::new(HookJob {
            toml: std::path::PathBuf::from("C:\\p\\A1_MINI-fast.toml"),
            gcode: std::path::PathBuf::from("C:\\tmp\\45600.0.gcode"),
        });
        assert_eq!(run.snapshot().preset_name, "A1_MINI-fast.toml");
        assert_eq!(run.snapshot().gcode_name, "45600.0.gcode");
        assert!(run.snapshot().question.is_none() && run.snapshot().finished.is_none());

        run.record_progress(ProgressPayload {
            step: "pass1".to_owned(),
            fraction_in_step: Some(0.5),
            message: "第一遍处理中".to_owned(),
        });
        assert_eq!(run.last_step().as_deref(), Some("pass1"));
        assert_eq!(run.snapshot().progress.unwrap().fraction_in_step, Some(0.5));
    }
}
