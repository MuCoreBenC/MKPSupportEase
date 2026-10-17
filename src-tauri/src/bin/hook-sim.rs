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
//! # ★ 演出脚本搬去了 `devsim`
//!
//! 2026-10-14 起，"演一遍"这件事抽成了 `crate::devsim` —— 因为工作台的**「开发台」**
//! 要做同一件事（点一下按钮就看那一屏，不用敲命令）。两条腿共用同一份脚本：
//! 这一条 CLI 与工作台那颗按钮演出来的完全相同，不会各漂各的。
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
use std::time::Duration;

use mkp_support_ease_lib::devsim::{self, Mode, Options, Sink};
use mkp_support_ease_lib::hook_ipc::{Client, FromHook, QuestionChoice};
use postprocess::diag::CancelToken;

const USAGE: &str = "\
假钩子（开发用）：把开场 / 进度 / 那一问 / 结论演给已经开着的那扇窗看。

  cargo run --features workbench --bin hook-sim -- [选项]

  --mode success | fail | cancel | question   演到哪一档（默认 success）
  --delay <毫秒>                              每条进度之间的间隔（默认 700）
  --hold-at <步骤 id>                         走到这一步就停住不动（截图用；界面那颗
                                              「停止」或 Ctrl-C 都能收场）
  --list                                      把这 12 步列出来（拿 --hold-at 的 id）

跑之前先把界面起起来（npm run tauri dev）。它不读 G-code、不跑管线、**不落任何记录**。

★ 不想敲命令的话：工作台「设置 → 开发台」里有一排按钮，点一下就是这一条。";

struct Cli {
    opts: Options,
    list: bool,
}

impl Cli {
    fn parse(args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut opts = Options::default();
        let mut list = false;
        let mut it = args;
        while let Some(arg) = it.next() {
            match arg.as_str() {
                "--mode" => {
                    let v = it.next().ok_or("--mode 后面要跟一档")?;
                    opts.mode = Mode::parse(&v).ok_or_else(|| format!("不认识的档：{v}"))?;
                }
                "--delay" => {
                    let v = it.next().ok_or("--delay 后面要跟毫秒数")?;
                    let ms: u64 = v
                        .parse()
                        .map_err(|_| format!("--delay 要一个整数毫秒：{v}"))?;
                    opts.delay = Duration::from_millis(ms);
                }
                "--hold-at" => {
                    opts.hold_at = Some(it.next().ok_or("--hold-at 后面要跟步骤 id")?);
                }
                "--list" => list = true,
                other => return Err(format!("不认识的参数：{other}")),
            }
        }
        Ok(Self { opts, list })
    }
}

/// 把 [`devsim::Sink`] 接到 [`Client`] 上 —— CLI 那条腿。
struct ClientSink(Client);

impl Sink for ClientSink {
    fn send(&self, msg: &FromHook) {
        self.0.send(msg);
    }
    fn ask(&self, question: &str) -> bool {
        /* 机型那一问：两颗老按钮（`keep` / `stop`），超时口径由 `Client::ask` 自己定 */
        let choices = vec![
            QuestionChoice {
                id: "keep".to_owned(),
                label: "强制继续".to_owned(),
                hint: None,
                accent: false,
            },
            QuestionChoice {
                id: "stop".to_owned(),
                label: "终止".to_owned(),
                hint: None,
                accent: true,
            },
        ];
        self.0.ask(question, choices, "stop").as_deref() == Some("keep")
    }

    /// 校准件那一问（2026-10-17）：真的推到界面上去问，选项表就是演出里那两颗。
    fn ask_calibration(&self, question: &str, choices: &[(&str, &str, &str)]) -> Option<String> {
        let table = choices
            .iter()
            .map(|(id, label, hint)| QuestionChoice {
                id: (*id).to_owned(),
                label: (*label).to_owned(),
                hint: Some((*hint).to_owned()),
                accent: *id == "print",
            })
            .collect();
        self.0.ask(question, table, "stop")
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let cli = match Cli::parse(args.into_iter()) {
        Ok(c) => c,
        Err(why) => {
            eprintln!("{why}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    if cli.list {
        println!("{USAGE}\n\n这 12 步：");
        for (id, _, msg) in devsim::script() {
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
        eprintln!("（或者用工作台「设置 → 开发台」里的按钮，同一条演出）");
        return ExitCode::from(2);
    }
    eprintln!("（假钩子）连上界面了 —— 切到那扇窗看那一屏");

    let cancelled = devsim::drive(&ClientSink(client), &cli.opts, &cancel);
    let verdict = if cancelled {
        "已停止"
    } else {
        cli.opts.mode.label()
    };
    eprintln!("（假钩子）演完了：{verdict}（不落任何记录，报告页不会多一条）");
    ExitCode::SUCCESS
}
