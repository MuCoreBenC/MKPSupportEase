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

/// **开发台**：让客户端把某一格界面显示出来（作者 2026-10-14）。
///
/// 与上面那几条**完全不同的一类** —— 那几条是"后处理那一趟"的事件流；这一条是
/// **一条指令**："把 `<scene>` 那一格界面打开"。前端一个 Provider
/// （`src/app/dev/preview.tsx`）收下它，各模态框的持有者按自己的 `scene` 认领。
///
/// 它是**纯界面触发**：不跑后处理、不读 G-code、不落记录。后处理那一组不走这一条
/// （它有自己的演出，见 [`HookView::run_preview`]）。
pub const DEV_PREVIEW_EVENT: &str = "dev-preview";

/// 一条进度长什么样（与 Rust 侧 [`ProgressPayload`] 一一对应；前端只认字段名）。
pub type ProgressDto = ProgressPayload;
/// 开场那几个事实长什么样。
pub type FactsDto = FactsPayload;
/// 一个问题长什么样。
pub type QuestionDto = QuestionPayload;
/// 结论长什么样。
pub type FinishedDto = FinishedPayload;

/// **这一趟的整份状态**（开场 / 事实 / 进度 / 那一问 / 结论）——
/// [`HookView::replay_run`] 照这一份重推；给它一个名字，免得那一串
/// `Option<…>` 到处长出来（clippy 也不会再说类型太绕）。
pub type ReplayShot = (
    StartedDto,
    Option<FactsDto>,
    Option<ProgressDto>,
    Option<QuestionDto>,
    Option<FinishedDto>,
);

/// 新一趟的开场。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartedDto {
    pub preset_name: String,
    pub gcode_name: String,
    /// 这一趟是**开发台演出来的**（不是真钩子）—— 界面据此把真业务按钮禁掉、标「预览模式」
    pub preview: bool,
}

/// 延后的打印时间补算完（[`PRINT_TIME_EVENT`]）—— 结论里那一格由它补上。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintTimeDto {
    /// 预估打印秒数（与管线内同一口径：同一个 `printtime::estimate` + 同一组 Options）
    pub total_seconds: f64,
}

/// 开发台的一条**界面指令**（[`DEV_PREVIEW_EVENT`]）：把哪一格界面显示出来。
///
/// `scene` 是稳定串（`import-collision` / `rename` / … —— 前端 `DevScene` 那套 id 的下半截）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevPreviewDto {
    pub scene: String,
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
    /// **这一屏是开发台演出来的**（不是真钩子跑出来的）。
    ///
    /// `true` 时界面把那几颗真业务按钮（停止 / 后台运行 / 强制继续 / 终止）**禁掉**
    /// 并标上「预览模式」—— 那些按钮走的是真钩子回写线，预览里点了只会如实报
    /// "现在没有在跑后处理"（作者 2026-10-14 裁定：预览就明确是预览，不许点出真业务）。
    pub preview: bool,
    /// 快照里这一份问句，事件有没有送到过界面（见 [`Shared::question_emitted`]）。
    /// `false` = 界面拿到快照后应当喊 [`HookView::replay_question`] 补推一次。
    pub question_emitted: bool,
}

/// Tauri `manage` 那一格里放的东西（命令从它那三条读 / 写）。
pub struct HookSlot(pub Arc<HookView>);

/// 界面这一侧那一趟的状态。
pub struct HookView {
    shared: Mutex<Shared>,
    /// 回写线（取消 / 答复）：界面一起来就装上，之后一直用同一条
    writer: Mutex<Option<hook_ipc::Handle>>,
    /// **开发台预览的代次**：每来一条新的演出指令 +1。
    ///
    /// 用处只有一个 —— 「那一问」那一档：预览里没有真答复线（`ask` 无人可答），
    /// 那一屏要**停在问句上给用户看**（而不是立刻收场）。演出线程拿自己那一代次
    /// 在这儿等：**下一趟预览来了**（代次变了）或**演出被取消**时才放它走。
    /// 真钩子那条线不看它。
    preview_gen: std::sync::atomic::AtomicU64,
}

#[derive(Default)]
struct Shared {
    preset_name: Option<String>,
    gcode_name: Option<String>,
    facts: Option<FactsDto>,
    question: Option<QuestionDto>,
    progress: Option<ProgressDto>,
    finished: Option<FinishedDto>,
    /// 这一屏是**开发台演出来的**（不是真钩子）—— 见 [`RunSnapshot::preview`]
    preview: bool,
    /// 这一份问句**有没有推过事件**（2026-10-17）。
    ///
    /// 为什么要有这一笔：界面首帧同时做两件事 —— 读快照（`get_post_process_run`）与订阅
    /// `postprocess-question`。钩子那边「推问句 → 等答复」是一气呵成的：推送与开始等待
    /// 之间没有间隙，于是这两条路的先后全看运气。慢的那一路（订阅还没挂上）就会**一片空白
    /// 地等** —— 用户看到的正是这个：没有那个选择框，120 秒后被超时终止。
    /// 所以快照里带着"推过没有"：没推过的，界面回来喊一声 [`HookView::replay_question`]
    /// 补推一次（只重新 emit，不动任何状态）。
    question_emitted: bool,
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

/// 把客户端主窗口抬到最前 —— **新任务开工**（[`FromHook::Hello`]）与**开发台演示开演**
/// （[`HookView::run_preview`]）两处共用同一下。
///
/// 旧世代 `mkp-ssr::progress_window` 的原话：这是"新任务抬窗那条路（unminimize + show + set_focus）"。
/// 三条约定照搬：
///
/// - **一条连接只抬一次**（调用点各自保证：真钩子住开场那一条，演示住开演那一下）——
///   `progress` 一趟上千条，每条都抬等于处理期间反复抢焦点，用户没法做别的事；
/// - **刻意不用 `always_on_top`**：常驻置顶会一直挡着切片器的界面，而这个窗口空闲时没有内容
///   —— 要的是"开工时它出现在你面前"，不是"永远压在别人上面"；
/// - 全是 `let _ =`：窗口可能已经被用户关掉（关掉不是错误，别 panic）。
///
/// 代价如实登记：`set_focus` 是**请求**，macOS 给不给焦点取决于当时的前台应用与专注模式
/// 等系统设置 —— 给不了也不重试、不循环。
fn raise_main_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
    }
}

/// 系统通知那两行字（标题 / 正文）。
///
/// 文案口径与界面上那一屏一致：**成功就说成功的那句**（校准件那两种跑法各一句），
/// 失败就把钩子给的那句话原样带上（那是用户最需要看到的）。
///
/// ★ 这里**不编码表条目**：通知是"界面之外的一句提示"，不是错误，也不是记录；
/// 真正要查的细节在界面与 `_meta.json` 里。正文仍带 G-code 名，便于对上是哪一盘。
fn notify_lines(finished: &FinishedDto, gcode_name: Option<&str>) -> (String, String) {
    const TITLE_OK: &str = "后处理完成";
    const TITLE_BAD: &str = "后处理没成";

    /* 名字从**共享状态**里取（`FinishedPayload` 本身不带文件名 —— 那一份在开场那一条里）。
    取不到就说"这一盘"，不编一个名字出来 */
    let name = match gcode_name {
        Some(n) if !n.is_empty() => n.to_owned(),
        _ => "这一盘".to_owned(),
    };

    if finished.ok {
        /* 校准件那趟：两种跑法的结论不一样（打印件留没留），与界面读同一组键 */
        let body = crate::hook::calib_done_line(finished.calib_choice.as_deref())
            .unwrap_or_else(|| format!("{name} 已经处理好了"));
        (TITLE_OK.to_owned(), body)
    } else {
        (
            TITLE_BAD.to_owned(),
            format!("{name}：{}", finished.message),
        )
    }
}

/// 现在是不是**静默档**（设置页「后处理模式」那一档）。
///
/// 静默档要做的三件事，全在"别打扰用户"这一条上：
///   · 不抬窗、不抢焦点、不置顶（本函数的两处调用点）；
///   · 前端那一屏永不自己弹（`usePostProcessRun(silent)`）；
///   · 只在右上角留一枚状态，跑完 60 秒自己消失（`PostProcessChip`）。
///
/// ★ 读不出来（状态文件坏了 / 目录不在）按 **GUI**：静默是用户主动选的"这次别打扰我"，
///   读不到时退回"看得见"那一侧 —— 免得用户以为程序没动静。
///   与 `hook_ipc::wants_display` 的兜底**同一个取向**（两处必须一致，见那条判据）。
fn silent_mode(app: &AppHandle) -> bool {
    crate::fsx::paths::internal_root(app)
        .ok()
        .and_then(|root| crate::runtime::app_state::resolved_postproc_prefs(&root).ok())
        .is_some_and(|p| p.ui_mode == crate::runtime::app_state::PostprocUiMode::Silent)
}

impl HookView {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            shared: Mutex::new(Shared::default()),
            writer: Mutex::new(None),
            preview_gen: std::sync::atomic::AtomicU64::new(0),
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
                facts,
            } => {
                /* 开场与收场各一条 INFO：出问题时"这一趟到底有没有被界面收到"是第一个要问的 */
                tracing::info!(gcode = %gcode_name, preset = %preset_name, "钩子来了：这一趟开始");
                /* ★ **新任务抬窗**（作者 2026-10-14 拍板；旧世代 `mkp-ssr::progress_window`
                同一条：**第一条消息到达时抬一次**）—— 用户刚在切片器点了「导出」，正等着看
                进度，这一扇就该出现在他面前；不抬的话，他能看到的只有切片器那句退出码。
                只在这一条（新一趟的开场）里抬：`progress` 一趟发上千条，**绝不每条都抬**
                —— 那等于处理期间反复抢焦点，用户什么都干不了。
                跑完不再抢（见下面 `Finished` 那一条里那句"不抢焦点"）。

                ★★ **静默档不抬**（2026-10-17 作者：「选择了静默，就不用出现弹窗，就是不用
                   这个后处理的前端出现，缩在右上角就可以，结束也不出现`,
                   还有「也不用置顶 mkp」）：那一档用户明说了别打扰他 ——
                   抬窗（`unminimize + show + set_focus`）正是最打扰的那一下。
                   静默档下这一趟照样跑、右上角照样有状态，只是窗口不动。 */
                if !silent_mode(app) {
                    raise_main_window(app);
                }
                {
                    let mut g = self.shared.lock().expect("锁没坏");
                    /* 新一趟：上一趟的结论与问句一起清掉（旧世代：结果态留到下一次顶掉它）——
                    事实**跟着开场这一条一起落**（2026-10-14：原来它是紧跟着的单独一条，
                    界面上就是"框先开成一条、数字再一个个蹦出来"那一下闪）。 */
                    *g = Shared {
                        preset_name: Some(preset_name.clone()),
                        gcode_name: Some(gcode_name.clone()),
                        facts: facts.clone(),
                        ..Shared::default()
                    };
                }
                let _ = app.emit(
                    STARTED_EVENT,
                    StartedDto {
                        preset_name,
                        gcode_name,
                        preview: false,
                    },
                );
                /* 事实照旧单独推一条事件：前端那格表的"值"走它（快照里也有，两条路同源） */
                if let Some(facts) = facts {
                    let _ = app.emit(FACTS_EVENT, facts);
                }
            }
            FromHook::Progress(progress) => {
                self.shared.lock().expect("锁没坏").progress = Some(progress.clone());
                let _ = app.emit(PROGRESS_EVENT, progress);
            }
            FromHook::Question(question) => {
                {
                    let mut g = self.shared.lock().expect("锁没坏");
                    g.question = Some((*question).clone());
                    g.question_emitted = true;
                }
                /* 这一条是用户"答不答"的唯一入口：推过没有要留痕（见 `question_emitted`） */
                tracing::info!(
                    kind = %question.question_kind,
                    "钩子在等一个答复：问句已推给界面"
                );
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
                /* 静默档在下面两处要读（闪不闪 Dock、发不发通知的那条口子同源） */
                let silent = silent_mode(app);
                /* 通知正文要文件名：`FinishedPayload` 不带它（那一份在 `Hello` 里），
                所以在锁里取出来 —— 下面 `*finished` 会被 `emit` 移走。
                取不到（没开场就收场，理论分支）就是 `None`，通知里说"这一盘"。 */
                let gcode_name = {
                    let mut g = self.shared.lock().expect("锁没坏");
                    /* 结论已出：那一问不可能还有人在等，清掉免得界面还挂着它 */
                    g.question = None;
                    g.finished = Some((*finished).clone());
                    g.gcode_name.clone()
                };
                /* 要不要喊这一下，得在 payload 被 `emit` 带走**之前**算出来 */
                let shout = !finished.ok && !finished.cancelled;
                /* 通知要的那两行也在这儿算好（`*finished` 下一行就被移走了） */
                let notify_body =
                    (finished.ok || shout).then(|| notify_lines(&finished, gcode_name.as_deref()));
                let _ = app.emit(FINISHED_EVENT, *finished);
                /* 落完之后**喊一下这扇窗**：切片器那边只弹一句 "Error code: N"，
                用户很可能压根不知道自己这扇窗里有话要说（作者 2026-10-11）。
                Windows 闪任务栏、macOS 跳 Dock —— 只"提醒"，**不抢焦点**
                （开工那一下已经抬过窗了，见 `raise_main_window`；跑完再抢只会打断用户）。
                失败才喊：成功不用惊动谁。 */
                /* ★ 静默档也**不闪任务栏、不跳 Dock**（作者：「也不用置顶 mkp」，
                那一档整条意思是"别打扰我"）。它照样能收到系统通知（如果用户开着那格）——
                通知是"留在通知中心的一条"，闪 Dock 是"现在就要你看"，两件事。 */
                if shout && !silent {
                    if let Some(win) = app.get_webview_window("main") {
                        let _ =
                            win.request_user_attention(Some(tauri::UserAttentionType::Critical));
                    }
                }

                /* 系统通知（2026-10-17）：设置页那一格「处理完成发送系统通知」的真实现。
                以前这一格只是**存了个值**，跑完什么都不会发生 —— 那种开关不如不摆。

                ★ 只在**用户开着**时发：读不出来（状态文件坏了）按出厂档（开着）。
                ★ 只发"有结果"的那几种：成功 / 失败 / 校准那趟的两种跑法。
                  用户自己按停止的**不发** —— 他刚按的，他知道。
                ★ 不进 IR、不影响产物：纯界面行为（见 `PostprocPrefs` 头注两档的分工）。 */
                if let Some((title, body)) = notify_body {
                    let prefs = crate::fsx::paths::internal_root(app).ok().and_then(|root| {
                        crate::runtime::app_state::resolved_postproc_prefs(&root).ok()
                    });
                    /* `is_none_or` 要 Rust 1.82，本项目 MSRV 1.77 —— 用等价的 `map_or` */
                    let want = prefs.map_or(true, |p| p.notify_on_finish);
                    if want {
                        crate::notify::notify(&title, &body);
                    }
                }
            }
            /* 开发台的演出指令**不在这里演** —— 演出要"睡在步骤之间"，不能在通道的
            收消息线程里阻塞。真正的处理在监听器那一处（`lib.rs`：手里有 `Arc<HookView>`
            与 `AppHandle`，能放到后台线程里跑 `devsim::drive`）。走到这里说明有人直接
            调了 `apply`——记一条日志，什么都不做（如实，不编一屏出来）。 */
            FromHook::DevScene { scene } => {
                tracing::debug!(scene = %scene, "开发台演出指令到了 apply（该由监听器直接处理）");
            }
        }
    }

    /// **开发台那一屏**：把 `scene` 演给这一扇窗看（作者 2026-10-14）。
    ///
    /// 两类场景，走两条路：
    ///
    /// - **后处理那一组**（`success` / `fail` / `cancel` / `question`）：走与真钩子**同一个落点**
    ///   （[`Self::apply`]）—— 开演前把 `preview` 置上（界面据此换页脚、禁真业务按钮）。
    /// - **其余界面格**（导入 / 重命名 / 冲突 …）：**不演出**，只发一条
    ///   [`DEV_PREVIEW_EVENT`]，由前端那格界面的持有者自己认领（见 `src/app/dev/preview.tsx`）。
    ///
    /// 两条都先把客户端窗口**置顶**（作者：「我点那个演示的时候，就把那个客户端窗口置顶，
    /// 然后演示」）—— 唤起 + 聚焦，让用户正眼看那一屏。
    ///
    /// **不读 G-code、不跑管线、不落记录**（演出本体见 [`crate::devsim`]）。
    /// 立刻返回：演出在后台线程里按节奏走，命令 / 通道那一侧都不等它。
    pub fn run_preview(self: &Arc<Self>, app: &AppHandle, scene: &str) {
        /* 用户主动点的演示：先把窗抬到最前（与真钩子「新任务抬窗」同一下，见
        [`raise_main_window`]），再演 */
        raise_main_window(app);

        /* 后处理那一组的 slug 带前缀（`postprocess-success`）—— 剥掉再认。
        认不出来 ⇒ 它是**界面指令**：推给前端 Provider，由持有那一格界面的组件自己认领。
        这一类**不演出、不占后处理那套状态**。 */
        let tail = scene.strip_prefix("postprocess-");
        /* ★「处理中（停住不动）」「跑着被踢到后台」：走 success 的演出，但在第二遍扫描
         **停住**（见下 `hold_at`） */
        let hold_at = matches!(tail, Some("running" | "backgrounded")).then(|| "pass2".to_owned());
        /*
         * 2026-10-17 新加几档（作者：「帮我在工作台加上更多的触发按钮…比如触发红字的」）。
         * 都演在**同一扇窗**里，不另开屏：
         *   · `question-timeout` —— 那一问 + 真倒计时，到点自己收成「已停止」；
         *   · `answer-stale`   —— 那一问还挂在屏上、钩子已经没了：真机上点一下，
         *     答案会落在**标题右边**那一行（红字）。那一行是界面本地状态，所以这一档
         *     除了演出，还要把 slug 推给前端，由弹窗自己认领；
         *   · `backgrounded`   —— 跑着的时候被踢到后台（标题栏那枚胶囊 + 详情面板）。
         *     这一步是**界面自己点**「后台运行」，所以也只推 slug；
         *   · `success-no-metrics` —— 老记录那种成功：结果屏上没有读数那条。
         */
        let ask_wait =
            (tail == Some("question-timeout")).then(|| std::time::Duration::from_secs(12));
        /* 这两档要额外通知前端（红字 / 踢后台都是界面本地状态） */
        let tell_ui = matches!(tail, Some("answer-stale" | "backgrounded"));
        /* 老记录那种成功：跑成了，但没有那包读数 */
        let metrics = tail != Some("success-no-metrics");
        /* 「成功 · 好几行警告」：框身被警告撑高那一档（验下面那张事实表） */
        let many_warnings = tail == Some("success-long-warning");
        /* 挪塔那一问（2026-10-18）：演在"那一问"那一屏，带画布几何 */
        let tower_relocate = tail == Some("relocate-question");
        /*
         * 校准件那几档（2026-10-17）：
         *   · `calib-question`   —— 那一问停着（倒计时在 hero 里），选了哪个都等下一档；
         *   · `calib-timeout`    —— 那一问没人答，12 秒后**收成已停止**；
         *   · `calib-print-done` —— 答「先打印，再校准」，跑完落成功屏（模型原样）；
         *   · `calib-direct-done`—— 答「直接校准」，跑完落成功屏（只留校准路径）。
         * 这几档都是**同一扇窗**，不另开屏。
         */
        let calib_question = matches!(
            tail,
            Some("calib-question" | "calib-timeout" | "calib-print-done" | "calib-direct-done")
        );
        let calib_choice = match tail {
            Some("calib-print-done") => Some("print"),
            Some("calib-direct-done") => Some("direct"),
            _ => None,
        };
        /* 那一档要等超时收场：与 question-timeout 同一条（12 秒） */
        let ask_wait = if tail == Some("calib-timeout") {
            Some(std::time::Duration::from_secs(12))
        } else {
            ask_wait
        };
        let mode = match tail {
            Some("running" | "backgrounded" | "success-no-metrics" | "success-long-warning") => {
                Some(crate::devsim::Mode::Success)
            }
            Some("question-timeout" | "answer-stale") => Some(crate::devsim::Mode::Question),
            Some("relocate-question") => Some(crate::devsim::Mode::Success),
            /* 校准件那几档都用「那一问」的骨架演（机型两边给成一样的，免得先弹机型那一问） */
            Some("calib-question" | "calib-timeout" | "calib-print-done" | "calib-direct-done") => {
                Some(crate::devsim::Mode::Success)
            }
            other => other.and_then(crate::devsim::Mode::parse),
        };
        let Some(mode) = mode else {
            tracing::info!(scene = %scene, "开发台：把这一格界面推给客户端");
            let _ = app.emit(
                DEV_PREVIEW_EVENT,
                DevPreviewDto {
                    scene: scene.to_owned(),
                },
            );
            return;
        };

        /* 开一趟新的预览：代次 +1（上一趟若正卡在"那一问"上，会因此放行） */
        let generation = self
            .preview_gen
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            + 1;

        let me = self.clone();
        let handle = app.clone();
        std::thread::spawn(move || {
            me.begin_preview(&handle, mode);
            /* 演出：每条消息都喂回 `apply` —— 与真钩子来的那条线同一个落点 */
            let sink = PreviewSink {
                view: me.clone(),
                app: handle.clone(),
                generation,
                ask_wait,
            };
            crate::devsim::drive(
                &sink,
                &crate::devsim::Options {
                    mode,
                    delay: std::time::Duration::from_millis(700),
                    hold_at,
                    /* `Some(..)` = 要问；里面的 bool 只是"演成选哪个"，演出那侧按
                    `calib_choice` / 超时判，不看它 */
                    calib_question: calib_question.then_some(calib_choice == Some("direct")),
                    calib_choice,
                    metrics,
                    many_warnings,
                    tower_relocate,
                },
                &postprocess::diag::CancelToken::new(),
            );
            tracing::info!(scene = %mode.id(), "开发台：客户端这一屏演完了");
        });

        if tell_ui {
            /* 这两件都是**界面本地状态**（点按钮 / 点后台那一下才产生），后端造不出来 ——
            所以除了开演，还把 slug 推给前端：弹窗自己认领（红字 / 主动收起） */
            tracing::info!(scene = %scene, "开发台：这一档还要界面自己补一步");
            let _ = app.emit(
                DEV_PREVIEW_EVENT,
                DevPreviewDto {
                    scene: scene.to_owned(),
                },
            );
        }
    }

    /// 开演前的一下：置 `preview`、清上一趟、发一条"新一趟开场"。
    ///
    /// 与真钩子那条 [`FromHook::Hello`] 的分工一样（界面据此把上一趟的结论与"我关过了"
    /// 一起清掉）—— 唯一差别是这一趟带着 `preview: true`。
    fn begin_preview(&self, app: &AppHandle, mode: crate::devsim::Mode) {
        let preset_name = "（开发台预览）A1_MINI-fast.toml".to_owned();
        let gcode_name = format!("（开发台预览）{}.gcode", mode.id());
        {
            let mut g = self.shared.lock().expect("锁没坏");
            *g = Shared {
                preset_name: Some(preset_name.clone()),
                gcode_name: Some(gcode_name.clone()),
                preview: true,
                ..Shared::default()
            };
        }
        let _ = app.emit(
            STARTED_EVENT,
            StartedDto {
                preset_name,
                gcode_name,
                preview: true,
            },
        );
    }

    /// **预览演出**往这一份里送一条消息 —— 与 [`Self::apply`] 同一条落点，
    /// 但**接手开场那一条**（见 [`PreviewSink::send`]）。
    ///
    /// 与 `apply` 的唯一差别：[`FromHook::Hello`] **不落进 `apply` 那支 Hello 分支**
    /// —— 那一支会把 `preview` 一起清成 `false`（它是真钩子"新一趟"的语义）。
    /// 开场那一下由 `begin_preview` 做过了（带着播放标记），所以这里把 Hello 换成
    /// **只刷新文件名的预览开场**。
    fn apply_preview(&self, app: &AppHandle, msg: FromHook) {
        match msg {
            FromHook::Hello {
                preset_name,
                gcode_name,
                facts,
            } => {
                {
                    let mut g = self.shared.lock().expect("锁没坏");
                    g.preset_name = Some(preset_name.clone());
                    g.gcode_name = Some(gcode_name.clone());
                    g.preview = true;
                    /* 事实与开场同一条落到（与真钩子那条 [`Self::apply`] 同一口径） */
                    g.facts = facts.clone();
                }
                let _ = app.emit(
                    STARTED_EVENT,
                    StartedDto {
                        preset_name,
                        gcode_name,
                        preview: true,
                    },
                );
                if let Some(facts) = facts {
                    let _ = app.emit(FACTS_EVENT, facts);
                }
            }
            other => self.apply(app, other),
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
            preview: g.preview,
            question_emitted: g.question_emitted,
        })
    }

    /// 把快照里那一份问句**再推一次事件**（2026-10-17）。
    ///
    /// 只给"首帧那两条路谁先到"这一个竞态收尾：界面读到快照里有一问、却从没收到过
    /// 对应事件（`question_emitted == false`）时喊它一次。**不动任何状态** —— 问句本来就
    /// 还在 `shared` 里，钩子那边还在等；这里只是让界面画出来。
    ///
    /// 已经不在了（问句被结论清掉）就什么都不做 —— 界面会自己在快照回来之后重画。
    pub fn replay_question(&self, app: &AppHandle) {
        let question = self.shared.lock().expect("锁没坏").question.clone();
        if let Some(question) = question {
            tracing::info!("界面晚了一步：把在等的问句补推一次");
            let _ = app.emit(QUESTION_EVENT, question);
        }
    }

    /// **把这一趟整份重推一遍**（2026-10-18）：开场 → 事实 → 进度 → 那一问 → 结论。
    ///
    /// 补的是同一类竞态的**另一半**（那一半是 [`Self::replay_question`]）。
    /// 界面首帧同时做两件事：读快照 **与** 订阅 `postprocess-started`，谁先到全看运气 ——
    /// 中间落进那条缝里（读快照时这一趟还没来、订阅挂上时开场那一条已经 emit 完了）
    /// 就是**这一趟永远进不了界面**：`run` 恒为 `null`，而 `bind` 那几条一律
    /// `prev === null ? prev : …`，之后每条进度都被丢掉。
    ///
    /// 真机看到的正是这个（作者 2026-10-18）：点了导出，**窗口被抬起来了、屏上却没有
    /// 那一屏** —— 抬窗就住 `Hello` 那一条里、还排在 `emit` 前面，所以"窗口起来了"
    /// 不等于"事件送到了"。
    ///
    /// 只补"界面没看见的那一段"：**状态一个都不动**（`shared` 原样，钩子那边照跑）。
    /// 这一趟从没来过（`preset_name` 还是 `None`）就什么都不推 —— 那是"用户自己打开了
    /// 程序"，本来就没有这一屏。
    pub fn replay_run(&self, app: &AppHandle) {
        let Some((started, facts, progress, question, finished)) = self.replay_shot() else {
            return;
        };
        tracing::info!(gcode = %started.gcode_name, "界面晚了一步：把这一趟重推一遍");
        let _ = app.emit(STARTED_EVENT, started);
        if let Some(facts) = facts {
            let _ = app.emit(FACTS_EVENT, facts);
        }
        if let Some(progress) = progress {
            let _ = app.emit(PROGRESS_EVENT, progress);
        }
        if let Some(question) = question {
            let _ = app.emit(QUESTION_EVENT, question);
        }
        if let Some(finished) = finished {
            let _ = app.emit(FINISHED_EVENT, finished);
        }
    }

    /// 这一趟**当前整份状态**的快照（开场 / 事实 / 进度 / 那一问 / 结论）。
    /// 从没来过（`preset_name` 还是 `None`）⇒ `None` —— 只读，不动状态
    /// （[`Self::replay_run`] 就照这一份重推）。
    fn replay_shot(&self) -> Option<ReplayShot> {
        let g = self.shared.lock().expect("锁没坏");
        match (g.preset_name.clone(), g.gcode_name.clone()) {
            (Some(preset_name), Some(gcode_name)) => Some((
                StartedDto {
                    preset_name,
                    gcode_name,
                    preview: g.preview,
                },
                g.facts.clone(),
                g.progress.clone(),
                g.question.clone(),
                g.finished.clone(),
            )),
            _ => None,
        }
    }

    /// 有没有钩子连着（界面据此把「停止」变灰：点了也没对象）。
    pub fn hook_connected(&self) -> bool {
        self.writer
            .lock()
            .expect("锁没坏")
            .as_ref()
            .is_some_and(hook_ipc::Handle::connected)
    }

    /// **重置后处理状态**（2026-10-17，设置页那一颗按钮）。
    ///
    /// # 它治的是什么病
    ///
    /// 旧世代那颗按钮治的是「BambuStudio 卡在 95%」：那边是**同一个进程**握着
    /// `isGUIProcessMode` 这个布尔，进程卡住 ⇒ 那一位永远是 true ⇒ 后续请求全被拒。
    /// 我们这个架构里没有那一位 —— 钩子是**独立进程**，跑完就走，状态本来就不会残留。
    ///
    /// 但有一类残留是真会发生的：**钩子进程还在等答复，界面却已经离开**（窗口关掉、
    /// 崩溃、用户在问句上一直没搭理）。这时：
    ///   · 共享状态里挂着一条问句（或上一趟的结论）；
    ///   · 钩子那头阻塞在「等一个永远不来的答案」上，最后被 120 秒超时收成 cancelled；
    ///   · 用户打开界面看到的是一屏**属于上次的**旧内容，点「停止」也未必对得上。
    ///
    /// 这一颗按钮就干两件事，顺序不能反：
    ///
    /// 1. **先让钩子那边别再等**（`Cancel`）：能连上就发一条；连不上（它已经没了）
    ///    就跳过 —— 这时下面那步清状态就是全部。
    /// 2. **再清掉界面这一侧的共享状态**，并推一条“这一趟结束了”的事件，
    ///    让还开着的窗口把自己那屏收掉（否则它要等到下次 hello 才会变）。
    ///
    /// 返回**做了什么**（发给谁 / 清了什么）—— 设置页把它原样摆成一行回执，
    /// 这样"点了没反应"与"点了但本来就没东西"分得开。
    pub fn reset(&self, app: &AppHandle) -> Result<String, AppError> {
        /* ① 先让钩子那头别再等 */
        let cancelled = self.write(ToHook::Cancel, "这一趟已经结束了").is_ok();

        /* ② 再清界面这一侧 */
        let had_run = {
            let mut g = self.shared.lock().expect("锁没坏");
            let had = g.preset_name.is_some();
            *g = Shared::default();
            had
        };

        /* 还开着的窗口靠这条事件收掉自己那屏（只推结论那一格，界面按"已取消"处理） */
        let _ = app.emit(
            FINISHED_EVENT,
            FinishedDto {
                ok: false,
                cancelled: true,
                /* 这一行只在"设置页按了重置、还开着的窗口要收掉自己那屏"时出现，
                不是钩子那条线上的结论 —— 所以句子直接写在这里，不进码表。
                ★ 码仍然给：排查时照着它能在日志里认出"是重置收的场"。 */
                message: "后处理状态已被重置（由设置页触发）".to_owned(),
                code: Some("W_SYS_RESET_001".to_owned()),
                stage: None,
                exit_code: 0,
                elapsed_ms: 0,
                output: None,
                warnings: Vec::new(),
                result: None,
                calib_choice: None,
            },
        );

        Ok(if cancelled {
            "已让正在跑的那一趟停下，界面状态也清干净了".to_owned()
        } else if had_run {
            "没有钩子在跑，界面状态已清干净".to_owned()
        } else {
            "没有正在跑的处理，也没有留下状态 —— 不用重置".to_owned()
        })
    }

    /// 界面那颗「停止」：往通道里写一条取消。
    pub fn cancel(&self) -> Result<(), AppError> {
        /*
         * ★ 这两句是**给用户看的**（界面把 AppError 的那句话原样摆出来）——
         * 2026-10-17 作者：「不允许用这种口气说话，给用户看的」。原来写的是
         * "现在没有在问你要不要继续"，读起来像在数落人；改成陈述事实 + 不必再做什么。
         */
        self.write(ToHook::Cancel, "这一趟已经结束了，没有正在跑的处理")
    }

    /// 回答屏幕上那一问（机型 / 校准方式 …）：`id` 就是那一颗按钮的 id。
    ///
    /// `question_kind` 只做校验用：它要**对得上现在挂在屏上的那一问**，否则说明界面上
    /// 还是过期的一屏（问句已经被结论清掉 / 换成下一问了）—— 回一句"已经结束了"，
    /// 不把过期的答案塞给钩子。
    pub fn answer(&self, question_kind: &str, id: &str) -> Result<(), AppError> {
        let current = self.shared.lock().expect("锁没坏").question.clone();
        let same = current.is_some_and(|q| q.question_kind == question_kind);
        if !same {
            return Err(AppError::not_found("刚才那个问题已经结束了，不用再回答"));
        }
        self.write(
            ToHook::Answer {
                question_kind: question_kind.to_owned(),
                id: id.to_owned(),
            },
            "那一趟已经结束了，这一下没有送出去",
        )
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

/// 开发台演出的出口（[`HookView::run_preview`] 用）：每条消息喂回**同一份**
/// [`HookView`] 的 [`HookView::apply`] —— 与真钩子来的那条线同一个落点。
struct PreviewSink {
    view: Arc<HookView>,
    app: AppHandle,
    /// 这一趟演出属于哪一代（见 [`HookView::preview_gen`]）
    generation: u64,
    /// 那一问**等多久就自己收场**（`None` = 一直停着给人看）。
    ///
    /// 只有「超时被终止」那一档给 `Some(..)`：问句照常推、倒计时也照常走（界面读的是
    /// `deadline_unix_ms`），到点这一趟就返回"不跑"，于是屏上落到「已停止」——
    /// 真机上等满两分钟看到的那一屏，这里十几秒就能看一遍。
    ask_wait: Option<std::time::Duration>,
}

impl crate::devsim::Sink for PreviewSink {
    fn send(&self, msg: &FromHook) {
        /* ★ 这一趟若已被顶掉（用户又点了别的场景 ⇒ 代次变了），**这一条不再写屏**：
        否则旧趟"放行收场"那一条（已停止 / 失败）会盖住新趟刚建立的那一屏
        —— 表现就是"点了成功、屏幕上却是上一条的已停止" */
        if self
            .view
            .preview_gen
            .load(std::sync::atomic::Ordering::SeqCst)
            != self.generation
        {
            return;
        }
        /* 开场那条**换掉文件名**（`devsim` 给的是"假钩子"那套，那是 CLI 的措辞）——
        这里是开发台预览，界面标题该写"开发台预览" */
        let msg = match msg {
            FromHook::Hello { facts, .. } => FromHook::Hello {
                preset_name: "（开发台预览）A1_MINI-fast.toml".to_owned(),
                gcode_name: "（开发台预览）演示.gcode".to_owned(),
                facts: facts.clone(),
            },
            other => other.clone(),
        };
        self.view.apply_preview(&self.app, msg);
    }

    /// 预览里的「那一问」：**先把问句推上屏，再停在那一屏给用户看**。
    ///
    /// `devsim::Sink::ask` 的契约是"问一句并等答复"，**发问句是这一侧的责任**
    /// （真钩子那条线的 [`crate::hook_ipc::Client::ask`] 就是自己先 `send(Question)`
    /// 再等回写）—— 所以这里也要自己发，不然界面收不到问句那一屏。
    ///
    /// 发完就**一直等**，直到**下一趟预览来了**（代次变了）才放行 —— 那一刻这一趟收场、
    /// 把屏让给新的那一趟（与"再点另一个场景"同一条路）。作者 2026-10-14：点一下就要
    /// 看到那一屏，不能演一下就没了。
    /// 校准件那一问（开发台那几档）：把选项表照真机那样推上去，等下一档顶替。
    fn ask_calibration(&self, question: &str, choices: &[(&str, &str, &str)]) -> Option<String> {
        let deadline_unix_ms = self.ask_wait.map_or(0, |w| {
            crate::hook_ipc::unix_ms_now() + u64::try_from(w.as_millis()).unwrap_or(u64::MAX)
        });
        self.send(&FromHook::Question(Box::new(
            crate::hook_ipc::QuestionPayload {
                question_kind: crate::hook_ipc::QKIND_CALIBRATION_MODE.to_owned(),
                text: question.to_owned(),
                deadline_unix_ms,
                choices: choices
                    .iter()
                    .enumerate()
                    .map(|(i, (id, label, hint))| crate::hook_ipc::QuestionChoice {
                        id: (*id).to_owned(),
                        label: (*label).to_owned(),
                        hint: Some((*hint).to_owned()),
                        /* 第一颗是主按钮（与真钩子那一侧同一套摆法） */
                        accent: i == 0,
                    })
                    .collect(),
                geo: None, /* 开发台这两问没有画布几何（挪塔那一问才有） */
            },
        )));
        let until = self.ask_wait.map(|w| std::time::Instant::now() + w);
        loop {
            if self
                .view
                .preview_gen
                .load(std::sync::atomic::Ordering::SeqCst)
                != self.generation
            {
                /* 被下一趟顶掉 ⇒ 不答，收场让位给新的那一趟 */
                return None;
            }
            if until.is_some_and(|t| std::time::Instant::now() >= t) {
                /* 「超时」那一档：没人答 ⇒ 收成已停止（真机两分钟后就是这么收的） */
                return None;
            }
            std::thread::sleep(std::time::Duration::from_millis(150));
        }
    }

    /// **挪塔那一问**（2026-10-18）：与校准件那一问同一条路（推上去、等下一档顶替），
    /// 只是**带画布几何** —— 界面照它画两幅俯视图与坐标。
    fn ask_relocate(
        &self,
        question: &str,
        geo: crate::hook_ipc::RelocateGeoPayload,
    ) -> Option<String> {
        let deadline_unix_ms = self.ask_wait.map_or(0, |w| {
            crate::hook_ipc::unix_ms_now() + u64::try_from(w.as_millis()).unwrap_or(u64::MAX)
        });
        self.send(&FromHook::Question(Box::new(
            crate::hook_ipc::QuestionPayload {
                question_kind: crate::hook_ipc::QKIND_TOWER_RELOCATION.to_owned(),
                text: question.to_owned(),
                deadline_unix_ms,
                choices: vec![
                    crate::hook_ipc::QuestionChoice {
                        id: crate::hook_ipc::CHOICE_RELOCATE.to_owned(),
                        label: "把塔挪到不撞的位置".to_owned(),
                        hint: Some("只改这一趟，预设文件不动".to_owned()),
                        accent: true,
                    },
                    crate::hook_ipc::QuestionChoice {
                        id: crate::hook_ipc::CHOICE_MODEL.to_owned(),
                        label: "我自己去挪模型".to_owned(),
                        hint: Some("这一趟停下，原文件不动".to_owned()),
                        accent: false,
                    },
                ],
                geo: Some(geo),
            },
        )));
        let until = self.ask_wait.map(|w| std::time::Instant::now() + w);
        loop {
            if self
                .view
                .preview_gen
                .load(std::sync::atomic::Ordering::SeqCst)
                != self.generation
            {
                return None;
            }
            if let Some(deadline) = until {
                if std::time::Instant::now() >= deadline {
                    return None;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(120));
        }
    }

    fn ask(&self, question: &str) -> bool {
        /* 倒计时那一行读的就是它；`None`（默认那几档）= 0 = 不摆倒计时 */
        let deadline_unix_ms = self.ask_wait.map_or(0, |w| {
            crate::hook_ipc::unix_ms_now() + u64::try_from(w.as_millis()).unwrap_or(u64::MAX)
        });
        self.send(&FromHook::Question(Box::new(
            crate::hook_ipc::QuestionPayload {
                question_kind: crate::hook_ipc::QKIND_MACHINE_MISMATCH.to_owned(),
                text: question.to_owned(),
                deadline_unix_ms,
                choices: Vec::new(),
                geo: None,
            },
        )));
        let until = self.ask_wait.map(|w| std::time::Instant::now() + w);
        loop {
            if self
                .view
                .preview_gen
                .load(std::sync::atomic::Ordering::SeqCst)
                != self.generation
            {
                /* 被下一趟顶掉 ⇒ 不继续（收场），把屏让给新的那一趟 */
                return false;
            }
            if until.is_some_and(|t| std::time::Instant::now() >= t) {
                /* 「超时」那一档：屏上落成「已停止」（真的那一条是两分钟后这么收的） */
                return false;
            }
            std::thread::sleep(std::time::Duration::from_millis(150));
        }
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
                    question_kind: "machine-mismatch".to_owned(),
                    text: "还继续吗？".to_owned(),
                    deadline_unix_ms: 1_763_000_000_000,
                    choices: Vec::new(),
                    geo: None,
                }),
                finished: None,
                preview: false,
                question_emitted: false,
            };
        }
        let snap = view.snapshot().expect("来过就该有快照");
        assert_eq!(snap.preset_name, "A1_MINI-fast.toml");
        assert!(snap.question.is_some());
        assert!(!snap.preview, "真钩子那一趟不是预览");
        /* 没推过事件的问句：界面读到快照后要去补推（2026-10-17 首帧那个竞态） */
        assert!(!snap.question_emitted, "这一份是直接塞进 shared 的，没推过");
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
            calib_choice: None,
        });
        let snap = view.snapshot().expect("结论也在快照里");
        assert!(snap.question.is_none());
        assert_eq!(snap.finished.expect("有结论").exit_code, 1);
    }

    /// 开发台演出来的那一趟：快照带 `preview = true`（界面据此禁掉真业务按钮）
    #[test]
    fn a_preview_run_is_marked_in_the_snapshot() {
        let view = HookView::new();
        {
            let mut g = view.shared.lock().expect("锁没坏");
            *g = Shared {
                preset_name: Some("（开发台预览）A1_MINI-fast.toml".to_owned()),
                gcode_name: Some("（开发台预览）success.gcode".to_owned()),
                progress: Some(ProgressPayload {
                    step: "pass1".to_owned(),
                    fraction_in_step: Some(0.4),
                    message: "第一遍扫描: 35%".to_owned(),
                }),
                preview: true,
                ..Shared::default()
            };
        }
        let snap = view.snapshot().expect("演出来也该有快照");
        assert!(snap.preview, "预览那一趟必须打上 preview 标记");
    }

    /// 没有钩子连着时，那两条命令**报错**（不静默什么都不做）
    #[test]
    fn without_a_hook_the_buttons_report_it() {
        let view = HookView::new();
        assert!(view.cancel().is_err(), "没对象时要报错");
        assert!(
            view.answer(crate::hook_ipc::QKIND_MACHINE_MISMATCH, "keep")
                .is_err(),
            "没对象时要报错"
        );
        assert!(!view.hook_connected());
    }

    /// **补推只补"界面没看见的那一段"，状态一个都不动**（2026-10-18）。
    /// 这一趟从没来过（`preset_name` 还是 `None`）⇒ 什么都不推（那是用户自己打开
    /// 的程序，没有这一屏），且 `shared` 保持原样 —— 钉住"补推不得凭空造一屏"。
    #[test]
    fn replaying_a_run_that_never_came_does_nothing() {
        let view = HookView::new();
        assert!(
            view.replay_shot().is_none(),
            "这一趟从没来过 ⇒ 没有可补推的东西（别凭空造一屏）"
        );
    }

    /// 抬窗与送事件是两件事（作者真机那次：窗口被抬起来了、屏上却没有那一屏）
    /// ⇒ 补推要能把**整份**状态取出来：开场 + 事实 + 进度 + 那一问 + 结论。
    #[test]
    fn the_replay_shot_carries_the_whole_run() {
        let view = HookView::new();
        {
            let mut g = view.shared.lock().expect("锁没坏");
            *g = Shared {
                preset_name: Some("A1_MINI-fast.toml".to_owned()),
                gcode_name: Some("板.gcode".to_owned()),
                facts: Some(FactsPayload {
                    gcode_machine: "A1".to_owned(),
                    preset_machine: "A1".to_owned(),
                    filament_type: "PLA".to_owned(),
                    file_size_bytes: 1024,
                    nozzle_diameter: 0.4,
                    layer_height: 0.2,
                    glue_speed: None,
                    slicer: None,
                    slicer_print_time_seconds: None,
                }),
                progress: Some(ProgressPayload {
                    step: "pass1".to_owned(),
                    fraction_in_step: Some(0.5),
                    message: "第一遍处理中".to_owned(),
                }),
                question: Some(QuestionPayload {
                    question_kind: "machine-mismatch".to_owned(),
                    text: "还继续吗？".to_owned(),
                    deadline_unix_ms: 1_763_000_000_000,
                    choices: Vec::new(),
                    geo: None,
                }),
                ..Shared::default()
            };
        }
        let (started, facts, progress, question, finished) = view.replay_shot().expect("有这一趟");
        assert_eq!(started.preset_name, "A1_MINI-fast.toml");
        assert_eq!(started.gcode_name, "板.gcode");
        assert!(facts.is_some(), "事实要跟着一起补推");
        assert_eq!(progress.expect("进度要跟着").step, "pass1");
        assert_eq!(
            question.expect("那一问要跟着").question_kind,
            "machine-mismatch"
        );
        assert!(finished.is_none(), "没结论就不推结论");
    }
}
