//! `mkpse-pp` —— 唯一可执行产物，纯终端，无前端。
//!
//! ```text
//! mkpse-pp run <in.gcode> -c <config.toml> [-o out.gcode] [--set K=V]...
//! mkpse-pp init [-o config.toml]
//! mkpse-pp dump-ir -c <config.toml> [--set K=V]... [--gcode <in.gcode>]
//! mkpse-pp check -c <config.toml> [--set K=V]...
//! ```
//!
//! **两条通道分得很死**：进度 / 摘要 / 警告 / 日志 → **stderr**；
//! 机器要消费的东西（`run` 的结果路径、`init` 的配置、`dump-ir` 的 JSON）→ **stdout**。
//! 于是 `out=$(mkpse-pp run a.gcode -c c.toml)` 拿到的一定是路径，不掺一个字。
//!
//! 退出码：`0` 成功 / `1` 处理失败 / `2` 输入或配置错 / `130` 用户取消
//! （128+SIGINT 的 shell 惯例；来源仓库同款，理由是让脚本区分「用户按了 Ctrl-C」
//! 与「跑失败了」）。
//!
//! **1 与 2 怎么分 —— 这里刻意不按错误变体分类**：`PostprocError::InvalidConfig`
//! 既被配置解析用、也被 postproc 深处的边界检查用（实测：`E_GCODE_BOUNDARY_001`
//! 就是 `InvalidConfig`），按变体分会把「模型超出打印边界」报成「配置错」。
//! 改按**阶段**分：`run` 先做一次前置校验（存在性 + 配置解析，与 `check` 同一条路），
//! 失败 ⇒ 2；之后进管线，任何失败 ⇒ 1，取消 ⇒ 130。
//! **代价如实登记**：前置校验到管线第 2 步之间有一个极小窗口，
//! 若此刻配置文件被改坏，那次配置错会被报成退出码 1。

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use postprocess::config;
use postprocess::diag::{CancelToken, PostprocError};
use postprocess::pipeline::{self, ProcessRequest, ProgressEvent};

mod logging {
    //! 日志装配。**subscriber 只在这里装一次**，库里任何地方都不装。
    //!
    //! 只有 stderr 一条通道：来源仓库那套文件日志（`tracing-appender` + 非阻塞 writer +
    //! `_guard` 生命周期）**刻意不搬** —— 这是个终端程序，要留档 `2> mkpse-pp.log` 就够了，
    //! 多一条落盘通道就多一处「日志到底写哪去了」的问题。

    use tracing_subscriber::EnvFilter;

    /// `level` 是**缺省**级别，`RUST_LOG` 优先（`EnvFilter` 的既有语义，不自造）。
    ///
    /// 返回 Err 而不是 panic：日志装不上不该让后处理跑不了。
    pub fn init(level: &str) -> Result<(), String> {
        let filter = EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new(level))
            .map_err(|e| format!("日志级别 {level:?} 无效：{e}"))?;
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(std::io::stderr)
            .try_init()
            .map_err(|e| e.to_string())
    }
}

const EXIT_FAILED: u8 = 1;
const EXIT_BAD_INPUT: u8 = 2;
const EXIT_CANCELLED: u8 = 130;

#[derive(Debug, Parser)]
#[command(
    name = "mkpse-pp",
    version,
    about = "MKP 后处理内核（G-code + 配置 → 处理后 G-code）",
    long_about = "输入一份 G-code 与一份配置文件（结构就是内核的 IR 本身），输出处理后的 G-code。\n\
                  没有预设文件、没有云端、没有本地管理：参数就在配置里改，或者用 --set 临时覆盖。"
)]
struct Cli {
    /// 日志缺省级别（trace/debug/info/warn/error）；`RUST_LOG` 优先
    #[arg(long, global = true, default_value = "info")]
    log_level: String,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// 跑后处理
    Run {
        /// 输入 G-code
        gcode: PathBuf,
        /// 配置文件（TOML，结构 = IR）
        #[arg(short, long)]
        config: PathBuf,
        /// 输出路径；省略则**原地覆盖**（`.part` + rename 原子替换）
        #[arg(short, long)]
        out: Option<PathBuf>,
        /// 临时覆盖一个配置项，如 `--set Tower.SafeZOffset=1.5`（可重复）
        #[arg(long = "set", value_name = "路径=值")]
        set: Vec<String>,
    },
    /// 生成一份完整的默认配置（每个字段都在，带段落注释）
    Init {
        /// 写到文件；省略则打到 stdout。**已存在的文件不覆盖**（会退 2）
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// 打印配置**经过默认值兜底与机型表填充之后**的完整 IR（JSON）
    ///
    /// 它和你写的配置**不一样**，这正是它存在的理由：零值兜底（层高 / 喷嘴 / 速度）
    /// 与机型表填充（运动范围 / 涂胶区 / 禁区）都发生在读配置之后，
    /// 「我改的值到底生效了吗」只能这样看。
    DumpIr {
        #[arg(short, long)]
        config: PathBuf,
        #[arg(long = "set", value_name = "路径=值")]
        set: Vec<String>,
        /// 一并给出输入 G-code ⇒ 追加 4 条头部刮取事实，并真实检测 `GcodeMachineType`
        #[arg(long)]
        gcode: Option<PathBuf>,
    },
    /// 只校验配置能不能用（不读输入、不写任何文件）
    Check {
        #[arg(short, long)]
        config: PathBuf,
        #[arg(long = "set", value_name = "路径=值")]
        set: Vec<String>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    if let Err(e) = logging::init(&cli.log_level) {
        eprintln!("日志初始化失败（{e}），继续以无日志运行");
    }
    match cli.command {
        Command::Run {
            gcode,
            config,
            out,
            set,
        } => cmd_run(&gcode, &config, out, set),
        Command::Init { out } => cmd_init(out.as_deref()),
        Command::DumpIr { config, set, gcode } => cmd_dump_ir(&config, &set, gcode.as_deref()),
        Command::Check { config, set } => cmd_check(&config, &set),
    }
}

fn cmd_run(gcode: &Path, config: &Path, out: Option<PathBuf>, set: Vec<String>) -> ExitCode {
    if let Some(code) = preflight(gcode, config, &set) {
        return code;
    }

    // Ctrl-C → CancelToken：装了 handler 之后 SIGINT 不再直接杀进程，改走协作取消
    // （pass 循环内 100ms 停下 / 步骤边界立即停），原地模式的原文件因此保证不动。
    let cancel = CancelToken::new();
    if let Err(e) = ctrlc::set_handler({
        let token = cancel.clone();
        move || token.cancel()
    }) {
        eprintln!("Ctrl-C 处理器装配失败（{e}），继续但不支持协作取消");
    }

    let request = ProcessRequest {
        gcode_path: gcode.to_path_buf(),
        config_path: config.to_path_buf(),
        overrides: set,
        output_path: out,
    };
    let mut sink = |event: ProgressEvent| match event.fraction_in_step {
        Some(f) => eprintln!(
            "[progress] {:?} {:3.0}% {}",
            event.step,
            f * 100.0,
            event.message
        ),
        None => eprintln!("[progress] {:?} {}", event.step, event.message),
    };

    match pipeline::process(request, &mut sink, &cancel) {
        Ok(result) => {
            // 结果路径 → stdout，**独占一行**（脚本消费面）
            println!("{}", result.output_path.display());
            if let Some(pt) = &result.print_time {
                eprintln!(
                    "预计打印时间：{:.1}s（tool {:.1}s / prep {:.1}s / startup {:.1}s / segments {}）",
                    pt.total_seconds,
                    pt.tool_movement_seconds,
                    pt.prep_overhead_seconds,
                    pt.startup_overhead_seconds,
                    pt.segments
                );
            }
            eprintln!(
                "统计：塔高 {:.2}mm / 涂胶层 {} / 警告 {} 条",
                result.stats.tower_height,
                result.stats.pass1.glue_layer_count,
                result.warnings.len()
            );
            for w in &result.warnings {
                eprintln!("警告：{w}");
            }
            ExitCode::SUCCESS
        }
        Err(err) if matches!(err, PostprocError::Cancelled { .. }) => {
            eprintln!("已取消 [{}]：{err}", err.code());
            ExitCode::from(EXIT_CANCELLED)
        }
        Err(err) => {
            eprintln!("处理失败 [{}]：{err}", err.code());
            ExitCode::from(EXIT_FAILED)
        }
    }
}

/// `run` 的前置校验 = 存在性 + 一次真实的配置解析（与 `check` 同一条路）。
///
/// 返回 `Some(退出码)` 表示已经报错、调用方直接退；`None` 表示可以继续。
fn preflight(gcode: &Path, config: &Path, set: &[String]) -> Option<ExitCode> {
    if !gcode.is_file() {
        eprintln!("输入 G-code 不存在：{}", gcode.display());
        return Some(ExitCode::from(EXIT_BAD_INPUT));
    }
    if !config.is_file() {
        eprintln!("配置文件不存在：{}", config.display());
        return Some(ExitCode::from(EXIT_BAD_INPUT));
    }
    // 这一次解析只为把「配置错」与「处理失败」分成两个退出码。
    // 管线自己还会再读一次配置 —— 是**同一个函数**，不是第二条解析路径。
    if let Err(err) = pipeline::config_ir(config, set) {
        eprintln!("配置不可用 [{}]：{err}", err.code());
        return Some(ExitCode::from(EXIT_BAD_INPUT));
    }
    None
}

/// `init` 子命令：把一份样例配置打到 stdout，或写到 `--out` 指定的路径。
///
/// **写盘豁免**（`clippy::disallowed_methods`）：禁列防的是"截断用户已有文件"，
/// 而这里**先判 `path.exists()` 再写**，已存在就报错退出 —— 截断在语义上不可能发生。
/// 写的也是用户在命令行上自己指定的路径，不是仓库资产或用户数据目录。
///
/// **退役条件**：Task 19 统一写盘入口落地后，改成转调它（那时 `exists()` 那道判断
/// 应该换成 `File::create_new` 那种"原子地占住新路径"的原语）。
#[allow(clippy::disallowed_methods)]
fn cmd_init(out: Option<&Path>) -> ExitCode {
    let text = config::init_toml();
    match out {
        None => {
            print!("{text}");
            let _ = std::io::stdout().flush();
            ExitCode::SUCCESS
        }
        Some(path) => {
            // **不覆盖已存在的文件**：那份文件很可能已经被用户改过，
            // 静默盖掉等于删用户数据。要重来就自己先删。
            if path.exists() {
                eprintln!(
                    "已存在，拒绝覆盖：{}（要重新生成请先自己删）",
                    path.display()
                );
                return ExitCode::from(EXIT_BAD_INPUT);
            }
            match std::fs::write(path, &text) {
                Ok(()) => {
                    eprintln!("已写入 {}（{} 字节）", path.display(), text.len());
                    println!("{}", path.display());
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("写入失败：{}（{e}）", path.display());
                    ExitCode::from(EXIT_BAD_INPUT)
                }
            }
        }
    }
}

fn cmd_dump_ir(config: &Path, set: &[String], gcode: Option<&Path>) -> ExitCode {
    // 输入 G-code 是可选的；**给了就必须能读**（给了却读不到 ⇒ 报错，不静默降级成不带）。
    let raw = match gcode {
        Some(p) => match std::fs::read_to_string(p) {
            Ok(s) => Some(s),
            Err(e) => {
                eprintln!("读不到输入 G-code：{}（{e}）", p.display());
                return ExitCode::from(EXIT_BAD_INPUT);
            }
        },
        None => None,
    };
    let ir = match pipeline::resolve_ir(config, set, raw.as_deref()) {
        Ok(ir) => ir,
        Err(err) => {
            eprintln!("配置不可用 [{}]：{err}", err.code());
            return ExitCode::from(EXIT_BAD_INPUT);
        }
    };

    let mut value = match serde_json::to_value(&ir) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("IR 序列化失败：{e}");
            return ExitCode::from(EXIT_FAILED);
        }
    };
    // 4 条头部刮取事实只在给了输入时存在。**没给输入就不写这个键** ——
    // 写一个空对象会被读成「刮出来是空的」，那是另一件事（同款理由见 config.rs 的缺键判据）。
    if let (Some(text), Some(path)) = (raw.as_deref(), gcode) {
        let content: Vec<String> = text
            .lines()
            .map(|l| l.trim_end_matches('\r').to_string())
            .collect();
        let facts = pipeline::header_facts(&content, &path.to_string_lossy());
        if let Some(obj) = value.as_object_mut() {
            obj.insert(
                "GcodeHeaderFacts".to_string(),
                serde_json::json!({
                    "MachinePresetName": facts.machine_preset_name,
                    "ProcessPresetName": facts.process_preset_name,
                    "SlicerIroningEnabled": facts.slicer_ironing_enabled,
                    "ThreeMfFileName": facts.three_mf_file_name,
                }),
            );
        }
    }
    match serde_json::to_string_pretty(&value) {
        Ok(s) => {
            println!("{s}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("IR 序列化失败：{e}");
            ExitCode::from(EXIT_FAILED)
        }
    }
}

fn cmd_check(config: &Path, set: &[String]) -> ExitCode {
    if !config.is_file() {
        eprintln!("配置文件不存在：{}", config.display());
        return ExitCode::from(EXIT_BAD_INPUT);
    }
    // 用 `resolve_ir`（第 2+3 步）而不是只做第 2 步：机型表填完才知道运动范围，
    // 而「机型名写对了没有」是唯一会让整条链改行为的字段。
    match pipeline::resolve_ir(config, set, None) {
        Ok(ir) => {
            // 报**判定依据**而不是一句「OK」：一句 OK 的可核对性等于零。
            eprintln!("配置可用：{}", config.display());
            eprintln!(
                "机型 {}（X {:.1}..{:.1} / Y {:.1}..{:.1}，涂胶区 X {:.1}..{:.1} / Y {:.1}..{:.1}）",
                ir.machine.machine_type,
                ir.machine.min_x,
                ir.machine.max_x,
                ir.machine.min_y,
                ir.machine.max_y,
                ir.machine.glue_min_x,
                ir.machine.glue_max_x,
                ir.machine.glue_min_y,
                ir.machine.glue_max_y
            );
            eprintln!(
                "生效层高 {:.3}/{:.3}mm，喷嘴 {:.2}mm，擦拭偏好 {:?}，禁区 {} 处",
                ir.machine.first_layer_height,
                ir.machine.typical_layer_height,
                ir.machine.nozzle_diameter,
                ir.wiping.preferred_mode,
                ir.machine.forbidden_zones.len()
            );
            eprintln!(
                "完整生效值（含全部字段）：mkpse-pp dump-ir -c {}",
                config.display()
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("配置不可用 [{}]：{err}", err.code());
            ExitCode::from(EXIT_BAD_INPUT)
        }
    }
}
