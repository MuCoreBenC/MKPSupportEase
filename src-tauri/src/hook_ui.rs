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

use tauri::{AppHandle, Emitter as _, Manager as _};

use crate::error::AppError;
use crate::hook_ipc::{
    self, FactsPayload, FinishedPayload, FromHook, ProgressPayload, QuestionPayload, ToHook,
};

/// 一条进度（推给界面，也存进快照）。
pub const PROGRESS_EVENT: &str = "postprocess-progress";
/// 开场那几个事实（机型 / 耗材 / 大小 / 喷嘴 / 层高）—— 界面左栏那张表。
pub const FACTS_EVENT: &str = "postprocess-facts";
/// 要用户回答一个问题（目前只有"机型不匹配还跑不跑"）。
pub const QUESTION_EVENT: &str = "postprocess-question";
/// 跑完了（成功 / 失败 / 取消都走这一条），带上退出码。
pub const FINISHED_EVENT: &str = "postprocess-finished";
/// 新的一趟开始了（界面据此把上一趟的结论与"我关过了"一起清掉）。
pub const STARTED_EVENT: &str = "postprocess-started";
/// 延后的打印时间**补算完了** —— 结论那一格里空着的「预估打印时间」由它补上。
///
/// 只在算出来时发（算不出来就不发：界面那一格继续空着，不许拿 0 凑数）。
pub const PRINT_TIME_EVENT: &str = "postprocess-print-time";

/// 一条进度长什么样（与 Rust 侧 [`ProgressPayload`] 一一对应；前端只认字段名）。
pub type ProgressDto = ProgressPayload;
/// 开场那几个事实长什么样。
pub type FactsDto = FactsPayload;
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

/// 延后的打印时间补算完（[`PRINT_TIME_EVENT`]）—— 结论里那一格由它补上。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintTimeDto {
    /// 预估打印秒数（与管线内同一口径：同一个 `printtime::estimate` + 同一组 Options）
    pub total_seconds: f64,
}

/// 窗口起来时读的那份**全量快照**（进度 / 待答的问句 / 结论都在里面）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSnapshot {
    pub preset_name: String,
    pub gcode_name: String,
    /// 开场那几个事实（机型 / 耗材 / 大小 / 喷嘴 / 层高）—— 还没收到就是 `None`
    /// （窗口开得比钩子早、或者那一趟在报事实之前就失败了）
    pub facts: Option<FactsDto>,
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
    facts: Option<FactsDto>,
    question: Option<QuestionDto>,
    progress: Option<ProgressDto>,
    finished: Option<FinishedDto>,
}

/// 补一条执行记录的打印时间（**后台线程里跑**，不占钩子那条关键路径）。
///
/// 四种情形在这里收口：
/// - 算出来 ⇒ `ready`（只动 `detail.printTime` 与 `printTimeStatus`，其余字段不动）；
/// - 输出文件读不到 / 序列化失败 ⇒ `failed`；
/// - 那条记录已经不在（被清掉了）⇒ 什么都不做；
/// - 界面进程退出 ⇒ 线程随进程一起没，记录停在 `computing` —— 界面下次打开报告页
///   会看到"正在估算"，**不假装有值**。
///
/// 返回值 = 算出来的预估秒数（**算出来了才有**）—— 调用方拿它推 [`PRINT_TIME_EVENT`]，
/// 界面上那一格由它亮起来；算不出来（标了 `failed` / 记录不在了）就是 `None`，不推。
pub(crate) fn complete_archive(
    user_root: &std::path::Path,
    record_id: &str,
    output: &str,
) -> Option<f64> {
    let history = user_root.join(crate::fsx::paths::GCODE_HISTORY_DIR);
    let Some(meta_path) = crate::archive::find_meta(&history, record_id) else {
        tracing::warn!(record = %record_id, "找不到那条执行记录，放弃补全");
        return None;
    };

    // ① 输出副本：与 `_meta.json` 同目录、名字就是 `<record_id>.gcode`
    let output_copy = meta_path.with_file_name(format!("{record_id}.gcode"));
    if let Err(e) = std::fs::copy(output, &output_copy) {
        tracing::warn!(error = %e, "输出归档复制失败，标失败");
        let _ = crate::archive::mark_print_time_failed(&meta_path);
        return None;
    }

    // ② 输出文件的事实（大小 + sha）
    let output_file = match crate::archive::sha256_of(&output_copy) {
        Ok(sha) => crate::archive::file_fact(&output_copy, None, sha).ok(),
        Err(e) => {
            tracing::warn!(error = %e, "输出摘要算不出来，标失败");
            let _ = crate::archive::mark_print_time_failed(&meta_path);
            return None;
        }
    };

    // ③ 打印时间（与管线内同一口径，补出来的数字跟自己算的一模一样）
    let text = match std::fs::read_to_string(output) {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(error = %e, "读不到输出文件，打印时间标失败");
            let _ = crate::archive::mark_print_time_failed(&meta_path);
            return None;
        }
    };
    let opts = postprocess::postproc::printtime::Options {
        compute_delta: true,
        startup_overhead_seconds: 240.0,
    };
    let estimated = postprocess::postproc::printtime::estimate(&text, &opts);
    let print_time = serde_json::to_value(&estimated).ok();

    // ④ 一次读-改-写补全（原子）
    if let Err(e) = crate::archive::complete_meta(&meta_path, output_file, print_time) {
        tracing::warn!(error = %e, "补全记录失败");
        let _ = crate::archive::mark_print_time_failed(&meta_path);
        return None;
    }
    tracing::info!(record = %record_id, "输出归档与打印时间补全完成");
    /* 算出来了 ⇒ 把秒数交回调用方（它推一条 [`PRINT_TIME_EVENT`]，那一格就亮起来） */
    Some(estimated.total_seconds)
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
            FromHook::Facts(facts) => {
                self.shared.lock().expect("锁没坏").facts = Some(facts.clone());
                let _ = app.emit(FACTS_EVENT, facts);
            }
            FromHook::Progress(progress) => {
                self.shared.lock().expect("锁没坏").progress = Some(progress.clone());
                let _ = app.emit(PROGRESS_EVENT, progress);
            }
            FromHook::Question(question) => {
                self.shared.lock().expect("锁没坏").question = Some(question.clone());
                let _ = app.emit(QUESTION_EVENT, question);
            }
            FromHook::DeferPrintTime { record_id, output } => {
                /* 打印时间**交到这一侧**：钩子已经退了（切片器在等退出码，它不能留后台任务）。
                这里是常驻进程，慢慢算不影响任何人；算完补全那条记录，
                **再把秒数推给界面**（结论窗里那一格空着 —— 补上它，用户不用跑去报告页找）。 */
                tracing::info!(record = %record_id, "接手这一趟的打印时间估算");
                let root = crate::fsx::paths::user_root(app).ok();
                let app = app.clone();
                std::thread::spawn(move || {
                    if let Some(root) = root {
                        if let Some(total_seconds) = complete_archive(&root, &record_id, &output) {
                            let _ = app.emit(PRINT_TIME_EVENT, PrintTimeDto { total_seconds });
                        }
                    }
                });
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
                /* 要不要喊这一下，得在 payload 被 `emit` 带走**之前**算出来 */
                let shout = !finished.ok && !finished.cancelled;
                let _ = app.emit(FINISHED_EVENT, *finished);
                /* 落完之后**喊一下这扇窗**：切片器那边只弹一句 "Error code: N"，
                用户很可能压根不知道自己这扇窗里有话要说（作者 2026-10-11）。
                Windows 闪任务栏、macOS 跳 Dock —— 只"提醒"，**不抢焦点**
                （焦点那一下是切片器的，抢过来只会打断用户）。失败才喊：成功不用惊动谁。 */
                if shout {
                    if let Some(win) = app.get_webview_window("main") {
                        let _ =
                            win.request_user_attention(Some(tauri::UserAttentionType::Critical));
                    }
                }
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
            facts: g.facts.clone(),
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
    // 判据要造真实文件（归档记录 + 输出体）—— 生产代码的写盘走 `fsx::atomic`。
    #![allow(clippy::disallowed_methods)]

    use super::*;

    /// 钩子把打印时间交出来之后：**这一侧**把它补上（`computing` → `ready`），
    /// 且补出来的数字与管线自己算的口径一致（同一组 Options）。
    ///
    /// 这条是解耦链路的钉子：没有它，"延后"会退化成"永远显示正在估算"。
    #[test]
    fn deferred_print_time_is_completed_by_the_display_side() {
        use time::macros::datetime;

        let d = tempfile::tempdir().unwrap();
        // 造一棵最小的 `{user_root}/gcode_history/<日期>/` 与一条 computing 记录
        let history = d.path().join(crate::fsx::paths::GCODE_HISTORY_DIR);
        let input = d.path().join("in.gcode");
        std::fs::write(&input, "G1 X1\n").unwrap();
        let paths = crate::archive::begin(
            &history,
            &input,
            "G1 X1\n",
            datetime!(2026-10-10 2:00:00 +8),
        )
        .unwrap();
        let record_id = paths
            .meta
            .file_name()
            .and_then(|n| {
                n.to_string_lossy()
                    .strip_suffix("_meta.json")
                    .map(str::to_string)
            })
            .unwrap();
        let meta = serde_json::json!({
            "startedAt": "2026-10-10 02:00:00",
            "elapsedMs": 1234,
            "detail": {"stats": {"glueLayerCount": 9}},
            "printTimeStatus": crate::archive::PRINT_TIME_COMPUTING,
        });
        std::fs::write(&paths.meta, serde_json::to_string(&meta).unwrap()).unwrap();

        // 输出文件：一段真 G-code（估算会扫它）
        let output = d.path().join("out.gcode");
        std::fs::write(&output, "G1 X10 Y10 F3000\nG1 X20 Y10 E0.5\n").unwrap();

        /* 返回值就是界面那一格要的那个数（补算完由 `PRINT_TIME_EVENT` 推过去） */
        let seconds = complete_archive(d.path(), &record_id, &output.to_string_lossy())
            .expect("补算成功要把秒数交出来");
        assert!(seconds > 0.0, "预估秒数该是个正数：{seconds}");

        let back: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&paths.meta).unwrap()).unwrap();
        assert_eq!(
            back["printTimeStatus"],
            crate::archive::PRINT_TIME_READY,
            "补算之后必须是 ready"
        );
        assert!(
            back["detail"]["printTime"]["segments"].is_number(),
            "printTime 要有内容：{:?}",
            back["detail"]["printTime"]
        );
        // 输出副本的事实也要补上（复制 + sha + 大小）
        assert!(
            back["detail"]["outputFile"]["sizeBytes"].is_number(),
            "outputFile 要有内容：{:?}",
            back["detail"]["outputFile"]
        );
        let expected_copy = paths.meta.with_file_name(format!("{record_id}.gcode"));
        assert_eq!(
            back["detail"]["outputFile"]["path"],
            expected_copy.display().to_string(),
            "输出副本的落点要与记录同目录、同名"
        );
        // 其余字段不动
        assert_eq!(back["elapsedMs"], 1234);
        assert_eq!(back["detail"]["stats"]["glueLayerCount"], 9);
    }

    /// 输出文件没了 ⇒ 必须标 `failed`，**不许留着 `computing`**。
    #[test]
    fn a_missing_output_marks_the_estimate_failed() {
        use time::macros::datetime;

        let d = tempfile::tempdir().unwrap();
        let history = d.path().join(crate::fsx::paths::GCODE_HISTORY_DIR);
        let input = d.path().join("in.gcode");
        std::fs::write(&input, "G1 X1\n").unwrap();
        let paths = crate::archive::begin(
            &history,
            &input,
            "G1 X1\n",
            datetime!(2026-10-10 2:00:00 +8),
        )
        .unwrap();
        let record_id = paths
            .meta
            .file_name()
            .unwrap()
            .to_string_lossy()
            .strip_suffix("_meta.json")
            .unwrap()
            .to_string();
        std::fs::write(
            &paths.meta,
            r#"{"elapsedMs":1,"printTimeStatus":"computing"}"#,
        )
        .unwrap();

        let missing = d.path().join("nope.gcode");
        assert!(
            complete_archive(d.path(), &record_id, &missing.to_string_lossy()).is_none(),
            "标了失败就没有秒数可返回（界面那一格继续空着，不许拿 0 凑数）"
        );

        let back: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&paths.meta).unwrap()).unwrap();
        assert_eq!(
            back["printTimeStatus"],
            crate::archive::PRINT_TIME_FAILED,
            "读不到输出就该标失败"
        );
    }

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
                facts: None,
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
            /* 失败没有读数（与真钩子同一个形状） */
            result: None,
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
