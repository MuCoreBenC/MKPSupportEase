//! **开发台的演出脚本** —— 把后处理那一屏该走的几条消息**按节奏演一遍**。
//!
//! # 它是从哪来的
//!
//! 原来这段整个住在 `src/bin/hook-sim.rs` 里（假钩子 CLI）。工作台的「开发台」要做
//! 同一件事（点一下按钮就看那一屏），于是把**演出本身**抽到这里，两边共用一份：
//!
//! ```text
//!             ┌──────────── 演出（本模块：drive + Sink）────────────┐
//!             │  hello → facts → progress… → [question] → finished  │
//!             └───────────┬───────────────────────────┬─────────────┘
//!                         │                           │
//!        hook-sim CLI（连客户端那扇窗）        工作台开发台（同进程直接喂）
//!        Sink = hook_ipc::Client              Sink = HookView::apply
//! ```
//!
//! **两条腿走的是同一份脚本** —— 所以"假钩子演出来的那一屏"与"开发台上演出来的那一屏"
//! 长得一样，改文案 / 加一档只需要改这一处。
//!
//! # 红线（照旧）
//!
//! 它**不读 G-code、不跑管线、不落任何记录** —— 归档是钩子进程的活。所以它给的文件名
//! 明摆着是假的（`（假钩子）演示.gcode`），报告页不会因为它多出一条。
//!
//! # 为什么它**不再**只在 workbench 档里（2026-10-14 纠正）
//!
//! 2026-10-14 作者纠正了开发台的形态：**工作台构建里没有客户端窗口**
//! （`tauri.workbench.conf.json` 的 `app.windows` 是空数组），所以"在工作台里点按钮、
//! 让**客户端窗口**显示那一屏"必然要**跨进程**：工作台只发一条"演哪一屏"，**演出发生在
//! 客户端进程**（收消息 → `hook_ui` 显示）。于是这一份演出脚本要进**客户端构建**，
//! 不能只挂在 workbench 档里。
//!
//! 但它**不是产品行为的一部分**：客户端侧唯一的入口是那条 `dev-scene` 通道消息
//! （只有工作台会发；真钩子从不发它），正常使用下它一个字节都不会跑。

use std::time::{Duration, Instant};

use crate::hook::HookError;
use crate::hook_ipc::{FactsPayload, FinishedPayload, FromHook, ProgressPayload, ResultFacts};
use postprocess::diag::CancelToken;

/// **演出脚本**：`(步骤 id, 步内比例, 那一行话)`。
///
/// 步骤 id 与内核的 `Step` 一一对应（界面按它换中文名，见 `src/app/postprocess/steps.ts`），
/// 那几行话**照抄内核自己那 12 条 `timer.begin`**（`crates/postprocess/src/pipeline/mod.rs`）——
/// 演出来的那一屏与真跑时长得一样，改样式时才对得上。
pub fn script() -> Vec<(&'static str, f32, String)> {
    /* 两条长 pass 自己再切几刀：真跑时它们的进度条就是这么走的 */
    let pass = |id: &'static str, name: &str| -> Vec<(&'static str, f32, String)> {
        [10u32, 35, 62, 88, 100]
            .iter()
            .map(|p| (id, *p as f32 / 100.0, format!("{name}: {p}%")))
            .collect()
    };

    let mut out: Vec<(&'static str, f32, String)> = vec![
        ("input", 1.0, "正在读取文件...".to_owned()),
        ("config", 1.0, "配置已就绪（IR 由调用方给出）".to_owned()),
        ("machine-detect", 1.0, "识别机型...".to_owned()),
        ("parse", 1.0, "解析 G-code...".to_owned()),
        ("support", 1.0, "支撑面检测...".to_owned()),
        ("collision", 1.0, "碰撞检测...".to_owned()),
        ("calib-check", 1.0, "校准模型检测...".to_owned()),
    ];
    out.extend(pass("pass1", "第一遍扫描"));
    out.extend(pass("pass2", "第二遍扫描"));
    out.push(("calibration", 1.0, "校准 G-code 插入...".to_owned()));
    out.push(("write", 1.0, "正在写入文件...".to_owned()));
    out.push(("printtime", 1.0, "打印时间估算...".to_owned()));
    out
}

/// 演到哪一档（与真钩子的三种收场对应：成功 / 失败 / 取消，外加"那一问"）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Success,
    Fail,
    Cancel,
    Question,
}

impl Mode {
    /// 命令行 / 界面上的那个 id（`success` / `fail` / `cancel` / `question`）。
    pub const ALL: [Mode; 4] = [Mode::Success, Mode::Fail, Mode::Cancel, Mode::Question];

    pub fn id(self) -> &'static str {
        match self {
            Mode::Success => "success",
            Mode::Fail => "fail",
            Mode::Cancel => "cancel",
            Mode::Question => "question",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Mode::Success => "成功",
            Mode::Fail => "失败（模型超出打印边界）",
            Mode::Cancel => "已停止",
            Mode::Question => "机型不匹配那一问",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "success" => Some(Mode::Success),
            "fail" => Some(Mode::Fail),
            "cancel" => Some(Mode::Cancel),
            "question" => Some(Mode::Question),
            _ => None,
        }
    }
}

/// 挪塔那一问的假几何（2026-10-18，开发台演那一屏用）—— 数值照真归档那一份的形状编，
/// 键名一个不错（前端读的就是 `CollisionFacts` 的 camelCase）。
fn relocate_geo() -> crate::hook_ipc::RelocateGeoPayload {
    use postprocess::postproc::disk::collision::{Box4, CollisionFacts, TowerShape};
    crate::hook_ipc::RelocateGeoPayload {
        current: [78.0, 91.0],
        candidate: Some([16.4, 90.0]),
        collision: Some(CollisionFacts {
            kind: "model".to_owned(),
            target: "model".to_owned(),
            direction: "右上".to_owned(),
            required_move_x: 32.6,
            required_move_y: 32.6,
            conflict_count: 97,
            conflict_points: vec![
                [74.2, 88.4],
                [75.1, 89.2],
                [73.8, 87.6],
                [76.4, 90.1],
                [74.9, 88.9],
            ],
            tower_bbox: Box4 {
                x_min: 70.0,
                x_max: 87.04,
                y_min: 84.0,
                y_max: 101.04,
            },
            model_bbox: Box4 {
                x_min: 40.0,
                x_max: 96.0,
                y_min: 26.0,
                y_max: 88.0,
            },
            /* 一个模型一个框（真机那趟是内核按占用格切出来的，这里演两件） */
            model_boxes: vec![
                Box4 {
                    x_min: 40.0,
                    x_max: 96.0,
                    y_min: 26.0,
                    y_max: 58.0,
                },
                Box4 {
                    x_min: 52.0,
                    x_max: 90.0,
                    y_min: 62.0,
                    y_max: 88.0,
                },
            ],
            /* 俯视图画的是轮廓（真机由内核算凸包；这里演两件不规则的） */
            model_polygons: vec![
                vec![
                    [40.0, 26.0],
                    [70.0, 22.0],
                    [96.0, 32.0],
                    [92.0, 58.0],
                    [58.0, 58.0],
                    [40.0, 44.0],
                ],
                vec![[52.0, 62.0], [90.0, 62.0], [92.0, 88.0], [52.0, 88.0]],
            ],
            printable: Box4 {
                x_min: 0.0,
                x_max: 256.0,
                y_min: 0.0,
                y_max: 256.0,
            },
            tower: Some(TowerShape {
                outer_structure: "brim".to_owned(),
                body_mm: 19.58,
                rib_width: 3.0,
                rib_extra_length: 4.0,
                rib_fillet_wall: false,
                sheath_base_expand: 5.0,
            }),
            forbidden_zones: Vec::new(),
            advice_code: "E_GCODE_COLLISION_002".to_owned(),
            advice_text: "建议：把模型往左上挪 —— X 至少 32.6mm，Y 至少 32.6mm".to_owned(),
            arrow_from: [68.0, 57.0],
            arrow_vec: [32.6, 32.6],
        }),
    }
}

/// 一次演出的参数（命令行与界面两处共用同一份）。

#[derive(Debug, Clone)]
pub struct Options {
    pub mode: Mode,
    /// 每条进度之间的间隔
    pub delay: Duration,
    /// 走到这一步就停住不动（截图用）；`None` = 一路演完
    pub hold_at: Option<String>,
    /// 演出里要不要问那句「校准件怎么跑」，以及**问了之后选什么**。
    ///
    /// - `Some(true)`  = 问，且演成"用户选了「直接校准」"；
    /// - `Some(false)` = 问，演成"用户选了「先打印，再校准」"（或没人答 → 收成已停止）；
    /// - `None`        = 不问（普通那几档）。
    pub calib_question: Option<bool>,
    /// 那一趟的 `calib_choice`（`print` / `direct`）；不走提问段的档位也可以直接给。
    pub calib_choice: Option<&'static str>,

    /// 成功那一档**带不带结果读数**（`result` 那一包）。
    ///
    /// `false` = 老记录那种：跑成了，但盘上没有那次的结果读数 ⇒ 结果那一屏**没有**
    /// 指标条与「擦料与胶水消耗」那几格（2026-10-17 开发台加的那一档就要看这个）。
    pub metrics: bool,

    /// 成功那一档要不要演**好几条**警告（2026-10-17 开发台「成功 · 好几行警告」）：
    /// `true` = 摆四条警告，验框身被撑高时下面那张事实表不被挤坏。
    pub many_warnings: bool,

    /// **挪塔那一问**（2026-10-18）：演出"模型与擦料塔撞上了"那一屏 ——
    /// 带画布几何（两幅图 / 坐标 / 候选塔位）。`true` = 问，没人答就收成已停止。
    pub tower_relocate: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            mode: Mode::Success,
            delay: Duration::from_millis(700),
            hold_at: None,
            calib_question: None,
            calib_choice: None,
            metrics: true,
            many_warnings: false,
            tower_relocate: false,
        }
    }
}

/// **演出的出口** —— 一条线，两条实现。
///
/// `send` 永不失败（写不出去就丢弃，与 [`crate::hook_ipc::Client`] 同一条纪律）；
/// `ask` 返回用户对那一问的答复（没有观众 / 超时 = `false`，不替用户决定）。
pub trait Sink {
    fn send(&self, msg: &FromHook);
    fn ask(&self, question: &str) -> bool;

    /// 校准件那一问（2026-10-17）：`question` + 选项表（`(id, label, hint)`），
    /// 返回被选中的 id；`None` = 没人答（演出里就是"超时"那一档）。
    ///
    /// 默认 `None`：只有实现了"真的一问"的那一侧才需要它。
    fn ask_calibration(&self, _question: &str, _choices: &[(&str, &str, &str)]) -> Option<String> {
        None
    }

    /// **挪塔那一问**（2026-10-18）：与校准件那一问同一条路，只是**带画布几何**。
    /// 默认 `None` = 没人答（演出里按"停下"收场）。
    fn ask_relocate(
        &self,
        _question: &str,
        _geo: crate::hook_ipc::RelocateGeoPayload,
    ) -> Option<String> {
        None
    }
}

/// 演一遍。**阻塞**（每条进度之间睡 `delay`），调用方负责放到后台线程里。
///
/// `cancel` 中途置位就收成"已停止"（与真钩子按「停止」同一条）。
/// 返回最终是"被取消"还是"自己走到头"。
pub fn drive(sink: &dyn Sink, opts: &Options, cancel: &CancelToken) -> bool {
    /* Ctrl-C / 界面那颗「停止」都走**协作取消**（与真钩子同一条）：
    那一屏会收到"已取消"而不是就此卡住 */

    /* 机型：只有"那一问"档故意让两边不一样（真跑时那一问就是这么来的） */
    let (gcode_machine, preset_machine) = match opts.mode {
        Mode::Question => ("A1", "A1_MINI"),
        _ => ("A1_MINI", "A1_MINI"),
    };
    /* 事实**并进开场那一条**（2026-10-14 起真钩子也这么发，见 [`FromHook::Hello`]）——
    演出与真跑走同一份脚本，界面上"框一出现就是完整形态"的样子两边才一致 */
    sink.send(&FromHook::Hello {
        preset_name: "（假钩子）A1_MINI-fast.toml".to_owned(),
        gcode_name: "（假钩子）演示.gcode".to_owned(),
        facts: Some(FactsPayload {
            gcode_machine: gcode_machine.to_owned(),
            preset_machine: preset_machine.to_owned(),
            filament_type: "PETG".to_owned(),
            file_size_bytes: 3_355_443,
            nozzle_diameter: 0.4,
            layer_height: 0.2,
            /* 这三格是"跑着那一扇"右栏要摆的（涂胶速度 / 切片软件 / 切片器自报的预估时长）——
            真钩子分别从 IR 与输入头里取，这里给一份与归档演示同一批的值 */
            glue_speed: Some(70.0),
            slicer: Some("BambuStudio".to_owned()),
            slicer_print_time_seconds: Some(12_175.0), // 3h 22m 55s
        }),
    });

    /* 「那一问」在真钩子里排在管线的**前面**（机型核对先做）—— 这里同一个位置 */
    if opts.mode == Mode::Question {
        /* 与 `messages.toml` 的 `hook.mismatch_question` 同一句（演出来的那一屏要和真跑的一样；
        2026-10-17 作者嫌原来那句"维度与打印边界"太像内核口径，改成了这一句短的） */
        let keep = sink.ask("G-code 是 A1，预设是 A1_MINI —— 机型对不上，结果可能出错。还继续吗？");
        if !keep {
            return finish(
                sink,
                Mode::Question,
                true,
                120,
                opts.metrics,
                opts.many_warnings,
                opts.calib_choice,
            );
        }
    }

    /*
     * 校准件那一问（2026-10-17）：真钩子排在机型那一问**之后**、进管线之前 —— 这里同一个位置。
     * 演出来的选择直接写进结论（`calib_choice`），界面据此把两种成功分开说。
     */
    let mut calib_choice: Option<&'static str> = opts.calib_choice;
    if opts.calib_question.is_some() {
        let asked = sink.ask_calibration(
            "这份 G-code 是Z 偏移校准件。保留打印件一起后处理，还是只跑校准？",
            &[
                ("print", "先打印，再校准", "打印件保留"),
                ("direct", "直接校准", "去掉打印件，只走校准路径"),
            ],
        );
        match asked.as_deref() {
            Some("direct") => calib_choice = Some("direct"),
            /* 不答 / 选了「先打印」：都按"模型原样"跑；不答则**停下**（与真钩子同一条口径） */
            Some(_) => calib_choice = Some("print"),
            None => {
                return finish(
                    sink,
                    opts.mode,
                    true,
                    120,
                    opts.metrics,
                    opts.many_warnings,
                    None,
                )
            }
        }
    }

    /*
     * **挪塔那一问**（2026-10-18）：真钩子里它住内核第 6 步（`Step::Collision`）——
     * 开发台是"不跑管线、只演那一屏"，所以顺序仍按演示走。答 `relocate` 就往下演
     * （塔挪了），别的收成已停止。
     */
    if opts.tower_relocate {
        let asked = sink.ask_relocate(
            "擦料塔与模型发生碰撞，是否自动调整擦料塔位置？",
            relocate_geo(),
        );
        if asked.as_deref() != Some("relocate") {
            return finish(
                sink,
                opts.mode,
                true,
                120,
                opts.metrics,
                opts.many_warnings,
                None,
            );
        }
    }

    let started = Instant::now();
    let mut cancelled = false;
    for (step, fraction, message) in script() {
        if cancel.is_cancelled() {
            cancelled = true;
            break;
        }
        /* 「失败 / 取消」两档演到一半就收手 —— 真跑时也是半路停下的，后面的步骤没走 */
        if (opts.mode == Mode::Fail && step == "pass2")
            || (opts.mode == Mode::Cancel && step == "calibration")
        {
            cancelled = opts.mode == Mode::Cancel;
            break;
        }
        sink.send(&FromHook::Progress(ProgressPayload {
            step: step.to_owned(),
            fraction_in_step: Some(fraction),
            message,
        }));
        std::thread::sleep(opts.delay);

        if opts.hold_at.as_deref() == Some(step) {
            /* 停住不动（截图用）：等取消线被置位（界面那颗「停止」/ Ctrl-C） */
            while !cancel.is_cancelled() {
                std::thread::sleep(Duration::from_millis(200));
            }
            cancelled = true;
            break;
        }
    }

    finish(
        sink,
        opts.mode,
        cancelled,
        started.elapsed().as_millis() as u64,
        opts.metrics,
        opts.many_warnings,
        calib_choice,
    )
}

/// 收场：把结论写进通道（三档与真钩子同一个形状）。返回 `cancelled`。
///
/// ★ 那几句话**不再手抄**（2026-10-11 改）：失败那句直接用**真的内核错误 + 真的文案表**
/// 渲出来（[`HookError::from`] + [`crate::hook::conclusion_line`] 那条链），成功与取消那两句
/// 走的是钩子自己那两个构造函数。抄一份的代价以前就摆在这儿 —— 演示屏上的字会与真跑
/// 慢慢漂开，而演示屏的用处正是"看起来跟真的一样"。
fn finish(
    sink: &dyn Sink,
    mode: Mode,
    cancelled: bool,
    elapsed_ms: u64,
    metrics: bool,
    many_warnings: bool,
    calib_choice: Option<&'static str>,
) -> bool {
    const OUT: &str = "C:\\Users\\wzy\\AppData\\Local\\Temp\\（假钩子）演示.gcode";
    let payload = if cancelled {
        /* 用户按的停止：**不给码**（同真钩子） */
        let err = HookError::user_cancelled();
        finished_payload(&err, "pass2", elapsed_ms)
    } else if mode == Mode::Fail {
        /* 作者 2026-10-11 实测到的那一条：模型超出打印边界（Y 181.4 > 180.0） */
        let err = HookError::from(postprocess::diag::PostprocError::Boundary {
            code: "E_GCODE_BOUNDARY_001",
            sides: postprocess::diag::messages::text(
                "boundary.clause",
                &[
                    ("axis", "Y"),
                    ("current", "181.4"),
                    (
                        "limit",
                        &postprocess::diag::messages::text(
                            "boundary.limit.max",
                            &[("value", "180.0")],
                        ),
                    ),
                ],
            ),
        });
        finished_payload(&err, "pass1", elapsed_ms)
    } else {
        FinishedPayload {
            ok: true,
            cancelled: false,
            message: crate::hook::success_line(std::path::Path::new(OUT), elapsed_ms as u128),
            code: None,
            stage: Some("printtime".to_owned()),
            exit_code: 0,
            elapsed_ms,
            output: Some(OUT.to_owned()),
            /* 警告那一行也演一条：它是"要你看一眼"的东西，排版得试到；
            `many_warnings` 那一档摆四条（验框身被撑高时事实表不被挤坏） */
            warnings: if many_warnings {
                vec![
                    "这一盘没有支撑，支撑相关的那几步按空跑".to_owned(),
                    "擦料塔位置被模型占了一半，擦料路径往里挪了 6 mm".to_owned(),
                    "这一盘的层高与预设里那两档都不一致，按预设值继续".to_owned(),
                    "涂胶起点比上一盘晚 0.4 mm，末尾可能少一小段".to_owned(),
                ]
            } else {
                vec!["这一盘没有支撑，支撑相关的那几步按空跑".to_owned()]
            },
            /* 结果读数那一包也演一份（指标条 + 擦料与胶水消耗那几格）——
            值照归档演示那一批给，好把那几格的样子一次试全。
            `metrics: false` 的那一档**不给**（老记录就是这么回来的：跑成了，但那次
            的结果读数没落盘 ⇒ 结果屏上没有那一条） */
            result: metrics.then_some(ResultFacts {
                pen_x: -0.9,
                pen_y: 23.0,
                pen_z: 4.0,
                glue_speed: 70.0,
                print_time_seconds: Some(13140.0),
                glue_length_mm: 9350.85,
                slicer: "BambuStudio 02.08.02.60".to_owned(),
            }),
            /* 校准件那几档演出：结论里也带上"选了哪种跑法" */
            calib_choice: calib_choice.map(std::string::ToString::to_string),
        }
    };
    sink.send(&FromHook::Finished(Box::new(payload)));
    cancelled
}

/// 失败/取消那两档的 payload：**照真钩子那条路**从错误里取原因与码（见 `hook::describe`）。
fn finished_payload(err: &HookError, stage: &str, elapsed_ms: u64) -> FinishedPayload {
    let (message, code) = err.describe();
    FinishedPayload {
        calib_choice: None,
        ok: false,
        cancelled: err.cancelled(),
        message,
        code,
        stage: Some(stage.to_owned()),
        exit_code: err.exit_code(),
        elapsed_ms,
        output: None,
        warnings: Vec::new(),
        /* 失败 / 取消没有读数 —— 与真钩子同一个形状 */
        result: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// 只在内存里收消息的假观众 —— 判据不碰网络、不碰界面。
    #[derive(Default)]
    struct Recorder {
        seen: Mutex<Vec<FromHook>>,
        /// 那一问的答复（`None` = 不答，走"没有观众"那条）
        answer: Option<bool>,
        /// 被问到的问题（`ask` 不经过 `send`，所以单独记一笔）
        asked: Mutex<Vec<String>>,
    }

    impl Sink for Recorder {
        fn send(&self, msg: &FromHook) {
            self.seen.lock().expect("锁没坏").push(msg.clone());
        }
        fn ask(&self, question: &str) -> bool {
            self.asked.lock().expect("锁没坏").push(question.to_owned());
            self.answer.unwrap_or(false)
        }
    }

    fn kinds(seen: &[FromHook]) -> Vec<&'static str> {
        seen.iter()
            .map(|m| match m {
                FromHook::Hello { .. } => "hello",
                FromHook::Progress(_) => "progress",
                FromHook::Question(_) => "question",
                FromHook::DeferPrintTime { .. } => "defer",
                FromHook::Finished(_) => "finished",
                /* 演出自己只发前五种；这一支只为穷尽 `FromHook` */
                FromHook::DevScene { .. } => "dev-scene",
            })
            .collect()
    }

    /// 成功那一档：开场 → 事实 → 若干进度 → 结论（形如真钩子）
    #[test]
    fn the_success_run_walks_hello_to_finished() {
        let rec = Recorder::default();
        let cancelled = drive(
            &rec,
            &Options {
                delay: Duration::from_millis(0),
                ..Options::default()
            },
            &CancelToken::new(),
        );
        assert!(!cancelled, "成功那一档不该报取消");
        let seen = rec.seen.lock().expect("锁没坏");
        let k = kinds(&seen);
        assert_eq!(k.first(), Some(&"hello"));
        assert_eq!(k.last(), Some(&"finished"));
        /* 事实并进开场那一条了（2026-10-14）：hello 之后直接轮到进度，中间不再有单独的 facts */
        assert!(!k.contains(&"facts"), "不该再有单独的事实那一条：{k:?}");
        assert!(k.contains(&"progress"), "该有进度：{k:?}");
        /* 结论那一条是成功 */
        if let Some(FromHook::Finished(f)) = seen.last() {
            assert!(f.ok && !f.cancelled && f.result.is_some());
        } else {
            panic!("最后一条该是结论");
        }
    }

    /// 失败那一档：结论 `ok=false` 且带真错误码
    #[test]
    fn the_failure_run_ends_with_a_real_error_code() {
        let rec = Recorder::default();
        drive(
            &rec,
            &Options {
                mode: Mode::Fail,
                delay: Duration::from_millis(0),
                hold_at: None,
                ..Options::default()
            },
            &CancelToken::new(),
        );
        let seen = rec.seen.lock().expect("锁没坏");
        match seen.last() {
            Some(FromHook::Finished(f)) => {
                assert!(!f.ok);
                assert_eq!(f.code.as_deref(), Some("E_GCODE_BOUNDARY_001"));
                assert!(f.result.is_none(), "失败没有读数");
            }
            other => panic!("最后一条该是结论：{other:?}"),
        }
    }

    /// 取消那一档：`cancelled = true`、无码
    #[test]
    fn the_cancel_run_says_cancelled_without_a_code() {
        let rec = Recorder::default();
        drive(
            &rec,
            &Options {
                mode: Mode::Cancel,
                delay: Duration::from_millis(0),
                hold_at: None,
                ..Options::default()
            },
            &CancelToken::new(),
        );
        let seen = rec.seen.lock().expect("锁没坏");
        match seen.last() {
            Some(FromHook::Finished(f)) => {
                assert!(!f.ok && f.cancelled);
                /* 2026-10-17 改口径：主动停止也给码（`W_` 打头，不是故障）。
                演示屏与真跑一次同一个形状 —— 真钩子那边现在是 `W_SYS_CANCELLED_001`。 */
                assert_eq!(
                    f.code.as_deref(),
                    Some("W_SYS_CANCELLED_001"),
                    "用户按的停止要给 W_ 码（以前这里是 None）"
                );
            }
            other => panic!("最后一条该是结论：{other:?}"),
        }
    }

    /// "那一问"档：答复"不继续" ⇒ 开场之后直接收成已停止（连进度都没有）
    #[test]
    fn the_question_run_that_is_declined_goes_straight_to_cancelled() {
        let rec = Recorder {
            answer: Some(false),
            ..Recorder::default()
        };
        let cancelled = drive(
            &rec,
            &Options {
                mode: Mode::Question,
                delay: Duration::from_millis(0),
                hold_at: None,
                ..Options::default()
            },
            &CancelToken::new(),
        );
        assert!(cancelled, "答\"不跑\"该收成取消");
        assert_eq!(
            rec.asked.lock().expect("锁没坏").len(),
            1,
            "该问一句（机型不匹配）"
        );
        let seen = rec.seen.lock().expect("锁没坏");
        match seen.last() {
            Some(FromHook::Finished(f)) => assert!(f.cancelled),
            other => panic!("最后一条该是结论：{other:?}"),
        }
    }

    /// 演出脚本末步是 `printtime`，而那几行话与内核那 12 步同一批 id
    #[test]
    fn the_script_steps_are_the_kernel_step_ids() {
        let ids: Vec<&str> = script().iter().map(|(id, _, _)| *id).collect();
        for want in [
            "input",
            "config",
            "machine-detect",
            "parse",
            "support",
            "collision",
            "calib-check",
            "pass1",
            "pass2",
            "calibration",
            "write",
            "printtime",
        ] {
            assert!(ids.contains(&want), "脚本少了步骤 {want}：{ids:?}");
        }
    }

    /// `--mode` 的四个 id 能正反走一遍（命令行与界面共用这一份解析）
    #[test]
    fn the_mode_ids_round_trip() {
        for m in Mode::ALL {
            assert_eq!(Mode::parse(m.id()), Some(m), "{:?}", m);
        }
        assert_eq!(Mode::parse("nope"), None);
    }
}
