//! **假钩子**：把几条消息按节奏发给**已经在跑的那扇窗** —— 只给开发用。
//!
//! # 它解决什么
//!
//! 那一屏后处理模态框（进度 / 等你决定 / 结论）平时只有切片器导出 G-code 时才看得见，
//! 要看一次得真去切片器里切一遍。改它的样式、截图、验三档结论时太慢 —— 这个 bin
//! 走**和真钩子同一条线**（`hook_ipc` 的 loopback 口），把开场 / 进度 / 那一问 / 结论
//! 按你给的节奏演一遍，界面那一屏就活了。
//!
//! # 它**不**做的事（别误会）
//!
//! 不读 G-code、不跑管线、**不落任何记录**（归档是钩子的活）—— 报告页不会因为你跑它而
//! 多出一条，`gcode_history/` 一个字节都不动。所以它给的文件名也明摆着是假的
//! （`（假钩子）演示.gcode`）。**它不是产品的一部分**：`required-features = ["workbench"]`
//! 把这一份整个关在 workbench 那一档里，给用户的二进制里编不到它。
//!
//! # 用法
//!
//! ```text
//! cargo run --features workbench --bin hook-sim -- [选项]
//!
//!   --mode success | fail | cancel | question   演到哪一档（默认 success）
//!   --delay <毫秒>                              每条进度之间的间隔（默认 700）
//!   --hold-at <步骤 id>                         走到这一步就停住不动（截图用）
//!   --list                                      把这 12 步列出来（拿 id 用）
//! ```
//!
//! 跑之前先把界面起起来（`npm run tauri dev`）—— 它连的是那扇窗的 loopback 口，
//! **连不上就自己安静退出**（打印一句"没有界面在听"），一个进程都不拉起。

use std::process::ExitCode;
use std::time::{Duration, Instant};

use mkp_support_ease_lib::hook;
use mkp_support_ease_lib::hook_ipc::{
    Client, FactsPayload, FinishedPayload, FromHook, ProgressPayload, ResultFacts,
};
use postprocess::diag::CancelToken;

const USAGE: &str = "\
假钩子（开发用）：把开场 / 进度 / 那一问 / 结论演给已经开着的那扇窗看。

  cargo run --features workbench --bin hook-sim -- [选项]

  --mode success | fail | cancel | question   演到哪一档（默认 success）
  --delay <毫秒>                              每条进度之间的间隔（默认 700）
  --hold-at <步骤 id>                         走到这一步就停住不动（截图用；界面那颗
                                              「停止」或 Ctrl-C 都能收场）
  --list                                      把这 12 步列出来（拿 --hold-at 的 id）

跑之前先把界面起起来（npm run tauri dev）。它不读 G-code、不跑管线、**不落任何记录**。";

/// 演出脚本：`(步骤 id, 步内比例, 那一行话)`。
///
/// 步骤 id 与内核的 `Step` 一一对应（界面按它换中文名，见 `src/app/postprocess/steps.ts`），
/// 那几行话**照抄内核自己那 12 条 `timer.begin`**（`crates/postprocess/src/pipeline/mod.rs`）——
/// 演出来的那一屏与真跑时长得一样，改样式时才对得上。
fn script() -> Vec<(&'static str, f32, String)> {
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
enum Mode {
    Success,
    Fail,
    Cancel,
    Question,
}

struct Options {
    mode: Mode,
    delay: Duration,
    hold_at: Option<String>,
    list: bool,
}

impl Options {
    fn parse(args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut o = Self {
            mode: Mode::Success,
            delay: Duration::from_millis(700),
            hold_at: None,
            list: false,
        };
        let mut it = args;
        while let Some(arg) = it.next() {
            match arg.as_str() {
                "--mode" => {
                    let v = it.next().ok_or("--mode 后面要跟一档")?;
                    o.mode = match v.as_str() {
                        "success" => Mode::Success,
                        "fail" => Mode::Fail,
                        "cancel" => Mode::Cancel,
                        "question" => Mode::Question,
                        other => return Err(format!("不认识的档：{other}")),
                    };
                }
                "--delay" => {
                    let v = it.next().ok_or("--delay 后面要跟毫秒数")?;
                    let ms: u64 = v
                        .parse()
                        .map_err(|_| format!("--delay 要一个整数毫秒：{v}"))?;
                    o.delay = Duration::from_millis(ms);
                }
                "--hold-at" => {
                    o.hold_at = Some(it.next().ok_or("--hold-at 后面要跟步骤 id")?);
                }
                "--list" => o.list = true,
                other => return Err(format!("不认识的参数：{other}")),
            }
        }
        Ok(o)
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let opts = match Options::parse(args.into_iter()) {
        Ok(o) => o,
        Err(why) => {
            eprintln!("{why}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    if opts.list {
        println!("{USAGE}\n\n这 12 步：");
        for (id, _, msg) in script() {
            println!("  {id:<16}{msg}");
        }
        return ExitCode::SUCCESS;
    }

    /* Ctrl-C 也走**协作取消**（与真钩子同一条）：那一屏会收到"已取消"而不是就此卡住 */
    let cancel = CancelToken::new();
    if let Err(e) = ctrlc::set_handler({
        let token = cancel.clone();
        move || token.cancel()
    }) {
        eprintln!("Ctrl-C 处理器装不上（{e}）—— 继续，Ctrl-C 会直接杀掉这一行");
    }

    /* 端点文件在内部根下（与真钩子同一处算）：算不出来 = 这机器上还没有数据根 */
    let Some(root) = mkp_support_ease_lib::fsx::paths::internal_root_headless() else {
        eprintln!("算不出内部数据根 —— 先把界面起一次（npm run tauri dev），让它把目录建齐");
        return ExitCode::from(2);
    };
    let client = Client::connect_or_headless(&root, &cancel);
    if !client.connected() {
        eprintln!("没有界面在听（那一扇窗没开？）—— 先 npm run tauri dev，再跑这一条");
        return ExitCode::from(2);
    }
    eprintln!("（假钩子）连上界面了 —— 切到那扇窗看那一屏");

    /* 机型：只有"那一问"档故意让两边不一样（真跑时那一问就是这么来的） */
    let (gcode_machine, preset_machine) = match opts.mode {
        Mode::Question => ("A1", "A1_MINI"),
        _ => ("A1_MINI", "A1_MINI"),
    };
    client.send(&FromHook::Hello {
        preset_name: "（假钩子）A1_MINI-fast.toml".to_owned(),
        gcode_name: "（假钩子）演示.gcode".to_owned(),
    });
    client.send(&FromHook::Facts(FactsPayload {
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
    }));

    /* 「那一问」在真钩子里排在管线的**前面**（机型核对先做）—— 这里同一个位置 */
    if opts.mode == Mode::Question {
        let keep = client.ask(
            "这份 G-code 是 A1 切出来的，而选中的预设是 A1_MINI 的 —— \
             维度与打印边界会按预设算，跑出来的可能是错的。还继续吗？",
        );
        if !keep {
            return finish(&client, Mode::Question, true, 120);
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
        client.send(&FromHook::Progress(ProgressPayload {
            step: step.to_owned(),
            fraction_in_step: Some(fraction),
            message,
        }));
        std::thread::sleep(opts.delay);

        if opts.hold_at.as_deref() == Some(step) {
            eprintln!("（假钩子）停在 `{step}` 不动了 —— 界面那颗「停止」、或者 Ctrl-C 都能收场");
            while !cancel.is_cancelled() {
                std::thread::sleep(Duration::from_millis(200));
            }
            cancelled = true;
            break;
        }
    }

    finish(
        &client,
        opts.mode,
        cancelled,
        started.elapsed().as_millis() as u64,
    )
}

/// 收场：把结论写进通道（三档与真钩子同一个形状）。
///
/// ★ 那几句话**不再手抄**（2026-10-11 改）：失败那句直接用**真的内核错误 + 真的文案表**
/// 渲出来（[`hook::HookError::from`] + [`hook::conclusion_line`]），成功与取消那两句
/// 走的是钩子自己那两个构造函数。抄一份的代价以前就摆在这儿 —— 演示屏上的字会与真跑
/// 慢慢漂开，而演示屏的用处正是"看起来跟真的一样"。
fn finish(client: &Client, mode: Mode, cancelled: bool, elapsed_ms: u64) -> ExitCode {
    const OUT: &str = "C:\\Users\\wzy\\AppData\\Local\\Temp\\（假钩子）演示.gcode";
    let payload = if cancelled {
        /* 用户按的停止：**不给码**（同真钩子） */
        let err = hook::HookError::user_cancelled();
        finished_payload(&err, "pass2", elapsed_ms)
    } else if mode == Mode::Fail {
        /* 作者 2026-10-11 实测到的那一条：模型超出打印边界（Y 181.4 > 180.0） */
        let err = hook::HookError::from(postprocess::diag::PostprocError::Boundary {
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
            message: hook::success_line(std::path::Path::new(OUT), elapsed_ms as u128),
            code: None,
            stage: Some("printtime".to_owned()),
            exit_code: 0,
            elapsed_ms,
            output: Some(OUT.to_owned()),
            /* 警告那一行也演一条：它是"要你看一眼"的东西，排版得试到 */
            warnings: vec!["这一盘没有支撑，支撑相关的那几步按空跑".to_owned()],
            /* 结果读数那一包也演一份（指标条 + 擦料与胶水消耗那几格）——
            值照归档演示那一批给，好把那几格的样子一次试全 */
            result: Some(ResultFacts {
                pen_x: -0.9,
                pen_y: 23.0,
                pen_z: 4.0,
                glue_speed: 70.0,
                print_time_seconds: Some(13140.0),
                glue_length_mm: 9350.85,
                slicer: "BambuStudio 02.08.02.60".to_owned(),
            }),
        }
    };
    client.send(&FromHook::Finished(Box::new(payload)));
    eprintln!("（假钩子）演完了：{mode:?}（不落任何记录，报告页不会多一条）");
    ExitCode::SUCCESS
}

/// 失败/取消那两档的 payload：**照真钩子那条路**从错误里取原因与码（见 `hook::describe`）。
fn finished_payload(err: &hook::HookError, stage: &str, elapsed_ms: u64) -> FinishedPayload {
    let (message, code) = err.describe();
    FinishedPayload {
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
