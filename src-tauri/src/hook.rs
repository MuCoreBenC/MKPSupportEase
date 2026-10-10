//! 后处理钩子 —— 切片器导出 G-code 时调的**那一次**（`--Toml` + `--Gcode`，认法见 [`crate::args`]）。
//!
//! # 一个 exe 两种角色
//!
//! 本程序既是界面，也是切片器里注册的"后处理脚本"：**带 `--Toml/--Gcode` 时不开第二个进程，
//! 而是把那一趟挂到界面上跑**（进度模态框 + 一颗"停止"，见 [`crate::hook_ui`]），跑完带
//! 退出码退 —— 切片器按退出码决定这一盘能不能继续（弹窗里那句 `Error code: N` 就是这个 N）。
//! 可执行物**就是主程序自己**（首页「复制后处理脚本」那一整条命令由
//! [`crate::ipc::get_post_process_command`] 现拼 —— 可执行物与预设两段都是本机真值）
//! —— 成熟版 `mkpsupporte` 与旧世代 `mkp-ssr` 都是这个形状。
//!
//! # 这一层管什么（**钩子进程里没有窗口**）
//!
//! 参数以外的**全部行为**都在这里：前置检查 → 预设 → IR → 机型不匹配那一问 → 12 步管线
//! （原地覆盖，`.part` + rename 原子替换）→ 结论 + 退出码。窗口住在**界面那个常驻进程**里，
//! 这一侧只往通道里写事件（[`crate::hook_ipc`]）：跑完**立刻退**（切片器一秒都不多等），
//! 结果留在界面那扇窗上等下一次切片。
//!
//! 本模块只认两个抽象 —— [`ProgressSink`]（进度往哪推）与 [`Asker`]（问题问谁）：
//! 有界面时是 [`ChannelSink`] / [`ChannelAsker`]，没有界面时是 [`NoProgress`] /
//! [`MismatchPolicy::Refuse`]（**不替用户决定**）。
//!
//! # 执行记录与备份（2026-10-09 起）
//!
//! 每跑一次落三件套：`<用户根>/gcode_history/<日期>/<名>_{_original.gcode, .gcode, _meta.json}`
//! （形状与成熟版一致，落点住本应用自己的数据根 —— 见 [`crate::archive`]）。
//! **原件在处理前先备份**：原地覆盖之后原文件就变样了，这一笔是它唯一的副本；
//! 失败与取消也留记录（`error` / `cancelled`），报告页扫 `*_meta.json`。
//!
//! # 还没做的（按成熟版的形状接着做，别当成"以后再说"）
//!
//! - **看门狗**：成熟版有 30 分钟上限、心跳丢失、10 分钟无进展就取消、卡死快照。
//!   我们这一版只有"用户在界面上按停止"与 Ctrl-C。
//!
//! # 退出码与 stderr 是对外契约
//!
//! 切片器只看得见**退出码**与 **stderr**：成功 `0`、处理失败或用户取消 `1`、
//! 输入/预设错 `2`。stderr **只写结论**：切片器把整段塞进它的错误对话框，
//! 多写一行就是把唯一有用的那行往下推（旧世代实测过 30 行 INFO 埋掉真正原因）。

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use postprocess::diag::{CancelToken, PostprocError};
use postprocess::pipeline::{self, IrProcessRequest, NoProgress, ProgressSink};

use crate::archive::{ArchivePaths, Detail, FileFact, Meta, PipelineStatus, PipelineStep};
use crate::args::HookJob;

/// 成功。
pub const EXIT_OK: u8 = 0;
/// 跑到一半失败，或**用户按了停止**（成熟版把"取消"也归 1，我们跟它）。
pub const EXIT_FAILED: u8 = 1;
/// 输入或预设本身不可用（文件不在、预设读不出来、机型不匹配而没人回答）。
pub const EXIT_BAD_INPUT: u8 = 2;

/// 一次钩子作业为什么没成。
#[derive(Debug)]
pub enum HookError {
    /// 输入/预设不可用 —— 还没碰过用户的文件，原文件一字未动。
    BadInput(String),
    /// 处理失败 —— 内核的 `.part` 已经丢掉，原文件没被覆盖（write 步之前就返回了）。
    Failed(String),
    /// 用户（或 Ctrl-C）在半路叫停 —— 同上，原文件没被覆盖。
    Cancelled(String),
}

impl HookError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::BadInput(_) => EXIT_BAD_INPUT,
            Self::Failed(_) | Self::Cancelled(_) => EXIT_FAILED,
        }
    }

    /// 界面据此决定那一屏说什么：取消不是"失败"。
    pub fn cancelled(&self) -> bool {
        matches!(self, Self::Cancelled(_))
    }
}

impl std::fmt::Display for HookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadInput(why) | Self::Failed(why) | Self::Cancelled(why) => f.write_str(why),
        }
    }
}

impl std::error::Error for HookError {}

/// 成功产物。
#[derive(Debug, Clone)]
pub struct Outcome {
    /// 处理后的 G-code 落点（原地覆盖时就是输入那一份）。
    pub output: PathBuf,
    pub elapsed_ms: u128,
    /// 内核给的警告：**逐条给用户看**（不是日志）—— 比如"这一盘没有支撑"。
    pub warnings: Vec<String>,
    /// 打印时间估算**交出去**了（这一趟没算）：交给常驻进程补全。
    /// `None` = 这一趟已经自己算完（或压根不需要）。
    pub deferred_print_time: Option<DeferredPrintTime>,
}

/// 一条"待补全的打印时间"。
#[derive(Debug, Clone)]
pub struct DeferredPrintTime {
    /// 记录 id（= `_meta.json` 文件名去掉后缀，全树唯一）。
    pub record_id: String,
    /// 输出 G-code 的落点（估算要读它）。
    pub output: PathBuf,
}

/// 问用户一件事（目前只有"机型不匹配怎么办"）。
///
/// 有没有界面由调用方给：界面那一半住 [`crate::hook_ui`]（弹一屏、等答复、超时算"不跑"），
/// 没有界面时用 [`MismatchPolicy::Refuse`] —— **不猜**。
pub trait Asker {
    /// 返回 `true` = 继续跑，`false` = 不跑。
    fn confirm(&mut self, question: &str) -> bool;
}

/// 机型不匹配时怎么处置。
pub enum MismatchPolicy<'a> {
    /// 有界面：问一句（超时 / 不答 = 不跑）。
    Ask(&'a mut dyn Asker),
    /// 没界面：**不替用户决定**，直接报错说清两边机型（退出码 2）。
    Refuse,
}

/// 跑一次：前置检查 → 预设 → IR → 机型核对 → 12 步管线（**原地覆盖**）。
///
/// `sink` 是进度出口（界面推事件 / 无界面时 [`NoProgress`]）；
/// `cancel` 是协作取消线（内核每个步骤入口都查它，长 pass 内也按固定间隔查）——
/// 取消落在 write 步**之前**，所以原文件不会被写坏（这一条是内核的判据，不是这里的承诺）。
///
/// `defer_print_time`：**必须确认有界面接手才传 `true`** —— 打印时间是纯报告数据，
/// 延后能省几百毫秒；但没人接手时传 `true` 会让那条记录永远停在"正在估算"。
pub fn run(
    job: &HookJob,
    sink: &mut dyn ProgressSink,
    cancel: &CancelToken,
    mismatch: MismatchPolicy<'_>,
    defer_print_time: bool,
) -> Result<Outcome, HookError> {
    let started = Instant::now();

    // 前置检查（廉价）先做，且要说人话：这两句会出现在切片器的错误对话框里
    if !job.toml.is_file() {
        return Err(HookError::BadInput(format!(
            "预设文件不存在：{}",
            job.toml.display()
        )));
    }
    if !job.gcode.is_file() {
        return Err(HookError::BadInput(format!(
            "输入 G-code 不存在：{}",
            job.gcode.display()
        )));
    }

    // 输入正文要读进内存：预设 → IR 的最后一步（机型 + 禁区）要拿它比 G-code 自报的机型
    let raw = std::fs::read_to_string(&job.gcode).map_err(|e| {
        HookError::BadInput(format!("读不到输入 G-code：{}（{e}）", job.gcode.display()))
    })?;

    // 记账起点（本地时刻）：归档记录里的 `startedAt`。读文件之后、干活之前 ——
    // 口径是「这一趟从哪一刻开始」，不是进程启动时刻。
    let started_at = crate::archive::now_local();

    // 归档第一笔：**原件先备份**。原地覆盖模式下原文件会被处理后的内容盖掉，
    // 这一笔是它唯一的副本。失败只降级成警告：产物优先（见 `crate::archive` 模块头）。
    let mut archive_warnings: Vec<String> = Vec::new();
    let archived = archive_begin(&job.gcode, &raw, &mut archive_warnings);

    // 预设 → IR：唯一入口（`crates/preset`）。预设里的参数错、机型不认识都在这一步报出来
    let ir = match preset::load_ir(&job.toml, Some(&raw)) {
        Ok(ir) => ir,
        Err(e) => {
            let message = format!("预设不可用 [{}]：{e}", e.code());
            archive_failure(
                &archived,
                job,
                "",
                &message,
                false,
                raw.len() as u64,
                started_at,
                started,
            );
            return Err(HookError::BadInput(message));
        }
    };

    // 机型核对：预设说一台、切片器那份自报另一台 —— 维度与边界全按预设算，
    // 闷头跑出来的是垃圾（旧世代那次实测：A1 的图 + A1 mini 的预设，
    // 报的是"模型超出打印边界 X：185.9mm"，把真正的原因埋在了一句听不懂的话里）。
    let preset_machine = ir.machine.machine_type.clone();
    let gcode_machine = ir.machine.gcode_machine_type.clone();
    if is_machine_mismatch(&preset_machine, &gcode_machine) {
        let question = format!(
            "这份 G-code 是 {gcode_machine} 切出来的，而选中的预设是 {preset_machine} 的 —— \
             维度与打印边界会按预设算，跑出来的可能是错的。还继续吗？"
        );
        match mismatch {
            MismatchPolicy::Ask(asker) => {
                if !asker.confirm(&question) {
                    let message = format!(
                        "已停下（机型不匹配：G-code 是 {gcode_machine}，预设是 {preset_machine}）—— \
                         原文件没有被改动"
                    );
                    archive_failure(
                        &archived,
                        job,
                        &preset_machine,
                        &message,
                        true,
                        raw.len() as u64,
                        started_at,
                        started,
                    );
                    return Err(HookError::Cancelled(message));
                }
            }
            MismatchPolicy::Refuse => {
                let message = format!(
                    "机型不匹配：G-code 是 {gcode_machine}，预设是 {preset_machine} —— \
                     换一份对应机型的预设再跑"
                );
                archive_failure(
                    &archived,
                    job,
                    &preset_machine,
                    &message,
                    false,
                    raw.len() as u64,
                    started_at,
                    started,
                );
                return Err(HookError::BadInput(message));
            }
        }
    }

    // 12 步管线。`output_path: None` = 原地覆盖（`.part` + rename，见内核 pipeline 文档）
    let outcome = if defer_print_time {
        pipeline::process_with_ir_deferred_print_time(
            IrProcessRequest {
                gcode_path: job.gcode.clone(),
                ir,
                output_path: None,
            },
            sink,
            cancel,
        )
    } else {
        pipeline::process_with_ir(
            IrProcessRequest {
                gcode_path: job.gcode.clone(),
                ir,
                output_path: None,
            },
            sink,
            cancel,
        )
    };
    let done = match outcome {
        Ok(done) => done,
        Err(err) => {
            let e = report_error(err);
            archive_failure(
                &archived,
                job,
                &preset_machine,
                &e.to_string(),
                e.cancelled(),
                raw.len() as u64,
                started_at,
                started,
            );
            return Err(e);
        }
    };

    // 归档第二笔：复制输出 + 写成功记录（失败只降级成警告）
    if let Some(paths) = &archived {
        archive_success(
            paths,
            job,
            &preset_machine,
            &done,
            raw.len() as u64, // 输入大小 = 原文字节数（盘上那份此刻已经是输出了）
            raw.lines().count() as u64,
            started_at,
            started,
            &mut archive_warnings,
        );
    }

    let mut warnings = done.warnings;
    warnings.extend(archive_warnings);

    // 延后的打印时间：把"该补谁"交给界面（记录 id + 输出路径，都很小）。
    let deferred_print_time = if let Some(paths) = &archived {
        (done.print_time.is_none()).then(|| DeferredPrintTime {
            record_id: record_id_of(paths),
            output: done.output_path.clone(),
        })
    } else {
        None
    };

    Ok(Outcome {
        output: done.output_path,
        elapsed_ms: started.elapsed().as_millis(),
        warnings,
        deferred_print_time,
    })
}

/// 记录 id = 文件名去掉 **`_meta.json`**（与报告页读口的口径一致）。
///
/// ★ 不能图省事用 `file_stem()`：它剥的是最后一个点之后的部分，
/// 对 `63288.1_20261009_234259_meta.json` 会给出 `…_meta`，于是
/// `find_meta` 去找 `…_meta_meta.json` —— 永远找不到，记录就卡在"正在估算"。
fn record_id_of(paths: &ArchivePaths) -> String {
    paths
        .meta
        .file_name()
        .and_then(|n| {
            n.to_string_lossy()
                .strip_suffix("_meta.json")
                .map(str::to_string)
        })
        .unwrap_or_default()
}

/// 归档第一笔：原件备份。返回 `None` = 这次没有归档（没有数据根 / 写失败），
/// 原因作为警告记进 `warnings` —— **调用点照常往下跑**（产物优先，见 `crate::archive`）。
fn archive_begin(
    gcode: &std::path::Path,
    raw: &str,
    warnings: &mut Vec<String>,
) -> Option<ArchivePaths> {
    let Some(root) = crate::archive::history_root() else {
        warnings.push("归档不可用：找不到应用数据目录（这一次不落执行记录）".to_string());
        return None;
    };
    match crate::archive::begin(&root, gcode, raw, crate::archive::now_local()) {
        Ok(paths) => Some(paths),
        Err(e) => {
            warnings.push(format!("原件备份失败（这一次不落执行记录）：{e}"));
            None
        }
    }
}

/// 归档第二笔（成功）：复制输出 + 写成功记录。每一步失败都只降级成警告。
#[allow(clippy::too_many_arguments)]
fn archive_success(
    paths: &ArchivePaths,
    job: &HookJob,
    machine: &str,
    done: &postprocess::pipeline::ProcessResult,
    input_size_bytes: u64,
    input_lines: u64,
    started_at: time::OffsetDateTime,
    started: std::time::Instant,
    warnings: &mut Vec<String>,
) {
    // 延后模式下：输出的复制与 sha 都交给界面进程 —— 这条路上只写记录，不碰那 26MB。
    let deferred = done.print_time.is_none();
    let output_file = if deferred {
        None
    } else {
        if let Err(e) = crate::archive::archive_output(paths, &done.output_path) {
            warnings.push(format!("输出归档失败：{e}"));
        }
        match crate::archive::sha256_of(&done.output_path) {
            Ok(sha) => match crate::archive::file_fact(&done.output_path, None, sha) {
                Ok(fact) => Some(fact),
                Err(e) => {
                    warnings.push(format!("输出文件事实读不出来：{e}"));
                    None
                }
            },
            Err(e) => {
                warnings.push(format!("输出文件摘要算不出来：{e}"));
                None
            }
        }
    };

    let meta = Meta {
        schema_version: Meta::SCHEMA_VERSION,
        started_at: crate::archive::display_stamp(started_at),
        finished_at: crate::archive::display_stamp(crate::archive::now_local()),
        elapsed_ms: started.elapsed().as_millis() as u64,
        preset_name: file_name_of(&job.toml),
        machine_type: machine.to_string(),
        preset_path: job.toml.display().to_string(),
        gcode_path: job.gcode.display().to_string(),
        invocation: crate::archive::invocation(),
        warnings: done.warnings.clone(),
        error: None,
        cancelled: false,
        pipeline: done
            .step_timings
            .iter()
            .map(|t| PipelineStep::from_timing(t, PipelineStatus::Complete))
            .collect(),
        detail: Detail {
            input_file: input_fact(
                &job.gcode,
                input_size_bytes,
                Some(input_lines),
                &done.input_sha256,
            ),
            output_file,
            stats: serde_json::to_value(&done.stats.pass1).ok(),
            print_time: done
                .print_time
                .as_ref()
                .and_then(|pt| serde_json::to_value(pt).ok()),
            // 打印时间是空的 ⇒ 这一趟把它延后了（交给常驻进程）⇒ 先标 `computing`。
            print_time_status: if done.print_time.is_none() {
                Some(crate::archive::PRINT_TIME_COMPUTING.to_string())
            } else {
                None
            },
        },
    };
    if let Err(e) = crate::archive::write_meta(paths, &meta) {
        warnings.push(format!("执行记录写入失败：{e}"));
    }
}

/// 归档第三笔（失败 / 取消）：写 error 记录（没有输出件）。
///
/// 这里的失败只进日志（`tracing::warn`）—— 调用点马上就要把 `Err` 返回给切片器，
/// 没有一条能把警告带给用户的通道；但归档失败必须留下痕迹（排查用）。
#[allow(clippy::too_many_arguments)]
fn archive_failure(
    archived: &Option<ArchivePaths>,
    job: &HookJob,
    machine: &str,
    error: &str,
    cancelled: bool,
    input_size_bytes: u64,
    started_at: time::OffsetDateTime,
    started: std::time::Instant,
) {
    let Some(paths) = archived else { return };
    let meta = Meta {
        schema_version: Meta::SCHEMA_VERSION,
        started_at: crate::archive::display_stamp(started_at),
        finished_at: crate::archive::display_stamp(crate::archive::now_local()),
        elapsed_ms: started.elapsed().as_millis() as u64,
        preset_name: file_name_of(&job.toml),
        machine_type: machine.to_string(),
        preset_path: job.toml.display().to_string(),
        gcode_path: job.gcode.display().to_string(),
        invocation: crate::archive::invocation(),
        warnings: Vec::new(),
        error: Some(error.to_string()),
        cancelled,
        pipeline: Vec::new(),
        detail: Detail {
            input_file: input_fact(&job.gcode, input_size_bytes, None, ""),
            output_file: None,
            stats: None,
            print_time: None,
            print_time_status: None, // 失败/取消不算估算，不写状态
        },
    };
    if let Err(e) = crate::archive::write_meta(paths, &meta) {
        tracing::warn!(error = %e, "执行记录写入失败（失败/取消那一条）");
    }
}

/// 输入侧的文件事实：**大小由调用方给**（原文字节数），不从盘上读 ——
/// 原地覆盖之后 `job.gcode` 里已经是输出了，从盘上量出来的是输出大小
/// （踩过一次的坑，判据见 `archive::input_file_fact` 那一条回归测试）。
fn input_fact(
    gcode: &std::path::Path,
    size_bytes: u64,
    lines: Option<u64>,
    sha256: &str,
) -> FileFact {
    crate::archive::input_file_fact(gcode, size_bytes, lines, sha256.to_string())
}

/// 两边机型对不上吗。
///
/// 判据只有三条，别加码：
/// 1. G-code 那边**认得出**（`UNKNOWN` / 空串 = 认不出，那不算不匹配 —— 老 G-code 里
///    经常没有机型标记，为此拦人是另一种错）；
/// 2. 两边都非空；
/// 3. 归一后不相等（大小写与别名差异不算不匹配）。
pub fn is_machine_mismatch(preset_machine: &str, gcode_machine: &str) -> bool {
    use postprocess::postproc::machine_dims::normalize_to_canonical;
    let g = gcode_machine.trim();
    if g.is_empty() || g.eq_ignore_ascii_case("UNKNOWN") {
        return false;
    }
    let p = normalize_to_canonical(preset_machine.trim());
    let g = normalize_to_canonical(g);
    !p.is_empty() && !g.is_empty() && p != g
}

/// 内核错误 → 钩子错误。整句原样留下（界面那一屏再拆成「原因 + 码」，见 [`describe`]）。
fn report_error(e: PostprocError) -> HookError {
    match e {
        PostprocError::Cancelled { .. } => {
            HookError::Cancelled(format!("已取消 [{}]：原文件没有被改动", e.code()))
        }
        other => HookError::Failed(format!("处理失败 [{}]：{other}", other.code())),
    }
}

/// 界面那一屏要的三件：**原因**（人话）、**稳定错误码**、是不是用户取消。
///
/// # 为什么要拆
///
/// 内核那句话里经常**嵌两层码**，实测形态：
///
/// ```text
/// 处理失败 [E_CFG_INVALID_001]：配置无效: E_GCODE_BOUNDARY_001: 模型超出打印边界，当前X：185.9mm…
///            ^^^^^^^^^^^^^^^^ 外层（变体名）  ^^^^^^^^^^^^^^^^^^^^ 里层（真正的原因）
/// ```
///
/// 直接整句摆给用户，他看到的是一串码，读不到"到底哪儿不对"（2026-10-09 作者原话：
/// 「停只显示了个什么错误码而已，根本就没显示正确的错误原因」）。
/// 所以：**里层码**（更具体那个）留下给排查用，**正文**只留码后面那截人话。
/// 外层那句"配置无效"一并丢掉 —— 它常常是**变体名在说假话**（边界检查复用的就是
/// `InvalidConfig` 这个变体，见内核 `diag/error.rs` 的登记）。
pub fn describe(err: &HookError) -> (String, Option<String>) {
    match err {
        /* 取消不是"错误"：不摆码，也不摆那串 `E_SYS_CANCELLED_001` —— 用户自己按的，
        他要看的是"现在是什么状态" */
        HookError::Cancelled(_) => (
            "已停止 —— 这一盘没有做后处理，原文件没有被改动（G-code 没有被覆盖）".to_owned(),
            None,
        ),
        HookError::BadInput(text) | HookError::Failed(text) => {
            let (body, code) = split_code(text);
            (body, code)
        }
    }
}

/// 从内核那句话里剥出**最具体**的稳定错误码（`E_*_NNN`），并返回码后面的人话。
///
/// 取值规则（简单到能一眼验）：扫全文，取**最后一个**像码的 token —— 越靠里越具体；
/// 找不到就整句当人话（前置检查那几句本来就没有码）。
fn split_code(text: &str) -> (String, Option<String>) {
    /// 看着像不像一个错误码：`E_` 开头，后面全是大写字母 / 数字 / 下划线，且至少两段。
    fn code_len_at(bytes: &[u8], at: usize) -> Option<usize> {
        if bytes[at] != b'E' || bytes.get(at + 1) != Some(&b'_') {
            return None;
        }
        let mut j = at + 2;
        while j < bytes.len() {
            let c = bytes[j];
            if c.is_ascii_uppercase() || c.is_ascii_digit() || c == b'_' {
                j += 1;
            } else {
                break;
            }
        }
        let token = &bytes[at..j];
        (token.len() > 4 && token.iter().filter(|c| **c == b'_').count() >= 2).then_some(j - at)
    }

    let bytes = text.as_bytes();
    let mut found: Option<(usize, usize)> = None; // (码的起点, 码的终点)
    let mut i = 0;
    while i < bytes.len() {
        if let Some(len) = code_len_at(bytes, i) {
            found = Some((i, i + len));
            i += len;
            continue;
        }
        i += 1;
    }

    let Some((start, end)) = found else {
        return (text.to_owned(), None);
    };
    let code = text[start..end].to_owned();
    /* 正文取码**之后**那截（码前面是外层包装：`处理失败 [外层码]：配置无效: ` 之类）——
    它们对用户没有信息量，留着只会把真正的原因往后推。 */
    let mut tail = text[end..]
        .trim_start_matches([']', ':', '：', ' ', '，', '。'])
        .trim();
    if tail.is_empty() {
        /* 码后面什么都没有（整句就是个码）：那就整句当人话，别给一屏空白 */
        tail = text;
    }
    (tail.to_owned(), Some(code))
}

/// **钩子进程的入口**：接上通道 → 干活 → 把结论写进通道 → 立刻退。
///
/// 退出码与那一行 stderr 是切片器看得见的全部；进度与结论是**给界面那扇窗看的**，
/// 写不出去也不影响成败（产物优先于界面，见 [`crate::hook_ipc`]）。
pub fn run_with_channel(job: &HookJob) -> ExitCode {
    let cancel = CancelToken::new();
    /* Ctrl-C → 协作取消（不是硬杀）：内核在步骤边界停下，`.part` 丢掉、原文件不动。
    装不上处理器**不阻断**这一趟 —— 用户按 Ctrl-C 时最坏退回默认硬杀。 */
    if let Err(e) = ctrlc::set_handler({
        let token = cancel.clone();
        move || token.cancel()
    }) {
        eprintln!("Ctrl-C 处理器装配失败（{e}），继续但不支持协作取消");
    }

    /* 端点文件在内部根下（钩子这侧没有 AppHandle，靠标识符自己算）。
    算不出来 / 连不上 / 起不来 ⇒ 一条"没有界面"的线：照旧干活 */
    let client = match crate::fsx::paths::internal_root_headless() {
        Some(root) => crate::hook_ipc::Client::connect_or_spawn(&root, &cancel),
        None => crate::hook_ipc::Client::headless(&cancel),
    };
    client.send(&crate::hook_ipc::FromHook::Hello {
        preset_name: file_name_of(&job.toml),
        gcode_name: file_name_of(&job.gcode),
    });

    /* 看门狗：偶尔抬头看一次"还在动吗"，到点就**请求取消**并把理由写进结论 */
    let watch = Watch::new();
    spawn_watchdog(watch.clone(), cancel.clone());

    let started = Instant::now();
    let mut sink = ChannelSink {
        client: &client,
        watch: watch.clone(),
        last_step: None,
    };
    let mut asker = ChannelAsker { client: &client };
    let mismatch = if client.connected() {
        MismatchPolicy::Ask(&mut asker)
    } else {
        MismatchPolicy::Refuse
    };

    // 打印时间**只在真的有界面接手时**才交出去 —— 没人接手就自己算完，记录才完整。
    let done = run(job, &mut sink, &cancel, mismatch, client.connected());
    if let Ok(done) = &done {
        if let Some(d) = &done.deferred_print_time {
            client.send(&crate::hook_ipc::FromHook::DeferPrintTime {
                record_id: d.record_id.clone(),
                output: d.output.display().to_string(),
            });
        }
    }
    let payload = finished_payload(
        &done,
        started.elapsed().as_millis(),
        sink.last_step(),
        watch.reason(),
    );
    client.send(&crate::hook_ipc::FromHook::Finished(Box::new(payload)));

    /* stderr 那一行（切片器会把 stderr 给用户看）与退出码走同一条路 —— 两处口径不许各写一份 */
    ExitCode::from(report(done))
}

/// 没有通道的那条路（判据 / 极端情况）：跑一次，结论只写 stderr。
pub fn run_and_report(job: &HookJob) -> ExitCode {
    let cancel = CancelToken::new();
    let mut sink = NoProgress;
    // 没有界面 ⇒ 打印时间必须自己算完（`defer = false`），否则记录永远停在"正在估算"
    let done = run(job, &mut sink, &cancel, MismatchPolicy::Refuse, false);
    ExitCode::from(report(done))
}

fn file_name_of(p: &std::path::Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| p.display().to_string())
}

/// 通道那一侧的进度出口：每条进度推给界面，同时给看门狗"报个平安"。
struct ChannelSink<'a> {
    client: &'a crate::hook_ipc::Client,
    watch: Arc<Watch>,
    last_step: Option<String>,
}

impl ChannelSink<'_> {
    fn last_step(&self) -> Option<String> {
        self.last_step.clone()
    }
}

impl ProgressSink for ChannelSink<'_> {
    fn emit(&mut self, event: postprocess::pipeline::ProgressEvent) {
        let payload = crate::hook_ipc::ProgressPayload {
            step: event.step.id().to_owned(),
            fraction_in_step: event.fraction_in_step,
            message: event.message,
        };
        self.last_step = Some(payload.step.clone());
        self.watch.touch();
        self.client
            .send(&crate::hook_ipc::FromHook::Progress(payload));
    }
}

/// 通道那一侧的问句：推给界面，等它答（见 [`crate::hook_ipc::Client::ask`]）。
struct ChannelAsker<'a> {
    client: &'a crate::hook_ipc::Client,
}

impl Asker for ChannelAsker<'_> {
    fn confirm(&mut self, question: &str) -> bool {
        self.client.ask(question)
    }
}

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

/// 现在该不该收手。先判**整趟超时**：走完 30 分钟这件事比"最后 10 分钟没动"更硬。
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

/// 这一趟"还在动吗"的那点状态（进度出口报平安，看门狗读它）。
pub struct Watch {
    last: Mutex<Instant>,
    reason: Mutex<Option<String>>,
}

impl Watch {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            last: Mutex::new(Instant::now()),
            reason: Mutex::new(None),
        })
    }

    fn touch(&self) {
        *self.last.lock().expect("锁没坏") = Instant::now();
    }

    fn last(&self) -> Instant {
        *self.last.lock().expect("锁没坏")
    }

    fn reason(&self) -> Option<String> {
        self.reason.lock().expect("锁没坏").clone()
    }

    fn stop_with(&self, why: String) {
        let mut g = self.reason.lock().expect("锁没坏");
        if g.is_none() {
            *g = Some(why);
        }
    }
}

/// 看门狗那一圈：到点就**请求取消**并留下理由（界面据此说清是哪一种停）。
fn spawn_watchdog(watch: Arc<Watch>, cancel: CancelToken) {
    std::thread::spawn(move || {
        let started = Instant::now();
        loop {
            std::thread::sleep(WATCHDOG_TICK);
            if cancel.is_cancelled() {
                return; // 已经有人叫停了（用户按的 / Ctrl-C），不用再看
            }
            match watchdog_verdict(
                Instant::now(),
                started,
                watch.last(),
                WATCHDOG_IDLE,
                WATCHDOG_TOTAL,
            ) {
                WatchdogVerdict::Fine => {}
                WatchdogVerdict::Idle => {
                    watch.stop_with(format!(
                        "超时停止：{} 分钟没有任何进展（多半是卡住了）—— 原文件没有被改动",
                        WATCHDOG_IDLE.as_secs() / 60
                    ));
                    cancel.cancel();
                    return;
                }
                WatchdogVerdict::Overrun => {
                    watch.stop_with(format!(
                        "超时停止：这一趟超过 {} 分钟 —— 原文件没有被改动",
                        WATCHDOG_TOTAL.as_secs() / 60
                    ));
                    cancel.cancel();
                    return;
                }
            }
        }
    });
}

/// 结论 → 通道里那条 [`crate::hook_ipc::FinishedPayload`]。
fn finished_payload(
    done: &Result<Outcome, HookError>,
    elapsed_ms: u128,
    stage: Option<String>,
    stop_reason: Option<String>,
) -> crate::hook_ipc::FinishedPayload {
    match done {
        Ok(out) => crate::hook_ipc::FinishedPayload {
            ok: true,
            cancelled: false,
            message: format!(
                "处理完成：{}（{} ms）",
                out.output.display(),
                out.elapsed_ms
            ),
            code: None,
            stage,
            exit_code: EXIT_OK,
            elapsed_ms: out.elapsed_ms as u64,
            output: Some(out.output.display().to_string()),
            warnings: out.warnings.clone(),
        },
        Err(err) => {
            /* 原因与码分家：界面把原因摆正中、码摆小字（见 [`describe`]）。
            取消那一档优先用**停止的理由**（用户按的？看门狗收的？）——
            只说"已停止"会把"为什么停"藏起来 */
            let (message, code) = describe(err);
            let message = if err.cancelled() {
                stop_reason.unwrap_or(message)
            } else {
                message
            };
            crate::hook_ipc::FinishedPayload {
                ok: false,
                cancelled: err.cancelled(),
                message,
                code,
                stage,
                exit_code: err.exit_code(),
                elapsed_ms: elapsed_ms as u64,
                output: None,
                warnings: Vec::new(),
            }
        }
    }
}

/// 结论 → stderr 那一行 + 退出码。**有界面与没界面两条路共用这一处**：
/// 两处各写一份的话，迟早有一处少一行（旧世代就是这么丢过失败原因的）。
pub fn report(done: Result<Outcome, HookError>) -> u8 {
    match done {
        Ok(out) => {
            eprintln!(
                "处理完成：{} （{} ms）",
                out.output.display(),
                out.elapsed_ms
            );
            // 警告各占一行：那是**用户要看的东西**，不是日志
            for w in &out.warnings {
                eprintln!("警告：{w}");
            }
            EXIT_OK
        }
        Err(err) => {
            // 失败原因**一定**要出去：切片器只会把 stderr 的文字给用户看
            eprintln!("{err}");
            err.exit_code()
        }
    }
}

#[cfg(test)]
mod tests {
    // 判据要造真实文件（预设、G-code、归档）—— 生产代码的写盘走 `fsx::atomic`。
    #![allow(clippy::disallowed_methods)]

    use super::*;
    use crate::args::HookJob;

    fn job(toml: &std::path::Path, gcode: &std::path::Path) -> HookJob {
        HookJob {
            toml: to_path_buf(toml),
            gcode: to_path_buf(gcode),
        }
    }

    fn to_path_buf(p: &std::path::Path) -> PathBuf {
        p.to_path_buf()
    }

    /// 仓库里那份夹具预设（与内置产物同构）—— 机型 A1 / 变体 fast
    fn fixture_preset(dest: &std::path::Path) {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../crates/preset/assets/presets/A1-fast.toml");
        std::fs::copy(&src, dest).expect("拷一份夹具预设");
    }

    fn run_headless(toml: &std::path::Path, gcode: &std::path::Path) -> Result<Outcome, HookError> {
        let mut sink = NoProgress;
        run(
            &job(toml, gcode),
            &mut sink,
            &CancelToken::new(),
            MismatchPolicy::Refuse,
            false, // 判据路径没有界面接手 ⇒ 打印时间必须自己算完
        )
    }

    /// 预设不在 ⇒ 退出码 2，且**还没碰过 G-code**（这条是"宁可不做也不乱写"的钉子）
    #[test]
    fn a_missing_preset_is_bad_input() {
        let d = tempfile::tempdir().expect("临时目录");
        let gcode = d.path().join("a.gcode");
        std::fs::write(&gcode, "G28\n").expect("写输入");
        let err = run_headless(&d.path().join("nope.toml"), &gcode).expect_err("预设不在必须报错");
        assert_eq!(err.exit_code(), EXIT_BAD_INPUT);
        assert!(
            err.to_string().contains("预设文件不存在"),
            "要说人话：{err}"
        );
        assert_eq!(
            std::fs::read_to_string(&gcode).expect("读回输入"),
            "G28\n",
            "输入一个字节都不许动"
        );
    }

    /// 输入不在 ⇒ 也是 2（切片器把 G-code 路径追加错了时看到的就是这条）
    #[test]
    fn a_missing_gcode_is_bad_input() {
        let d = tempfile::tempdir().expect("临时目录");
        let toml = d.path().join("A1-fast.toml");
        fixture_preset(&toml);
        let err = run_headless(&toml, &d.path().join("nope.gcode")).expect_err("输入不在必须报错");
        assert_eq!(err.exit_code(), EXIT_BAD_INPUT);
        assert!(err.to_string().contains("输入 G-code 不存在"), "{err}");
    }

    /// 预设读不出来（不是一份预设）⇒ 2，且报的是**内核那个稳定码**
    #[test]
    fn an_unreadable_preset_is_bad_input_with_a_code() {
        let d = tempfile::tempdir().expect("临时目录");
        let toml = d.path().join("bad.toml");
        std::fs::write(&toml, "[toolhead]\n不是 TOML\n").expect("写坏预设");
        let gcode = d.path().join("a.gcode");
        std::fs::write(&gcode, "G28\n").expect("写输入");
        let err = run_headless(&toml, &gcode).expect_err("坏预设必须报错");
        assert_eq!(err.exit_code(), EXIT_BAD_INPUT);
        assert!(
            err.to_string().contains("预设不可用 [E_"),
            "带上稳定错误码：{err}"
        );
    }

    /// **机型不匹配**：预设是 A1、G-code 自报 A1 mini ⇒ 没界面时**不猜**，
    /// 报错要同时点名两边（旧世代那句"模型超出打印边界"就是这条缺位的代价）
    #[test]
    fn a_machine_mismatch_refuses_and_names_both_sides() {
        let d = tempfile::tempdir().expect("临时目录");
        let toml = d.path().join("A1-fast.toml");
        fixture_preset(&toml);
        let gcode = d.path().join("a1mini.gcode");
        std::fs::write(&gcode, "; printer_model = Bambu Lab A1 mini\nG28\n").expect("写输入");
        let err = run_headless(&toml, &gcode).expect_err("机型不匹配必须拦下");
        assert_eq!(err.exit_code(), EXIT_BAD_INPUT);
        let why = err.to_string();
        assert!(
            why.contains("A1_MINI") && why.contains("A1"),
            "两边都要点名：{why}"
        );
        assert_eq!(
            std::fs::read_to_string(&gcode).expect("读回输入"),
            "; printer_model = Bambu Lab A1 mini\nG28\n",
            "拦下时输入一个字节都不许动"
        );
    }

    /// 有界面时问一句：答"继续"就照跑（这里用坏输入当靶子 —— 只要越过了那一问，
    /// 报的就不再是"机型不匹配"）
    #[test]
    fn an_answered_question_lets_the_run_continue_past_the_mismatch_gate() {
        let d = tempfile::tempdir().expect("临时目录");
        let toml = d.path().join("A1-fast.toml");
        fixture_preset(&toml);
        let gcode = d.path().join("a1mini.gcode");
        std::fs::write(&gcode, "; printer_model = Bambu Lab A1 mini\nG28\n").expect("写输入");

        struct Yes;
        impl Asker for Yes {
            fn confirm(&mut self, q: &str) -> bool {
                /* 问句要把**两边**都说清（用的是规范机型名，与预设头里那个同一个写法） */
                assert!(
                    q.contains("A1_MINI") && q.contains("A1"),
                    "两边都要点名：{q}"
                );
                true
            }
        }
        let mut sink = NoProgress;
        let out = run(
            &job(&toml, &gcode),
            &mut sink,
            &CancelToken::new(),
            MismatchPolicy::Ask(&mut Yes),
            false,
        )
        .expect("答了「继续」就该照跑（那一问只是拦一句，不改管线）");
        assert_eq!(out.output, gcode, "落点仍是输入那一份（原地覆盖）");
    }

    /// 取消：置位之后跑，落到"取消"那一档（1）
    #[test]
    fn a_cancelled_run_is_reported_as_cancelled() {
        let d = tempfile::tempdir().expect("临时目录");
        let toml = d.path().join("A1-fast.toml");
        fixture_preset(&toml);
        let gcode = d.path().join("a.gcode");
        std::fs::write(&gcode, "G28\n").expect("写输入");

        let cancel = CancelToken::new();
        cancel.cancel(); // 先置位：管线第一步就该吐出来
        let mut sink = NoProgress;
        let err = run(
            &job(&toml, &gcode),
            &mut sink,
            &cancel,
            MismatchPolicy::Refuse,
            false,
        )
        .expect_err("取消必须报错");
        assert_eq!(err.exit_code(), EXIT_FAILED);
        assert!(err.cancelled(), "要能被界面认成取消，而不是失败：{err}");
        assert_eq!(
            std::fs::read_to_string(&gcode).expect("读回输入"),
            "G28\n",
            "取消后输入一个字节都不许动"
        );
    }

    /// 端到端：整链参考 G-code（A1 mini）+ 对应机型那份预设 ⇒ 跑通、**原地更新**、不留中间文件。
    ///
    /// 产物**对不对**归内核那几条判据管（这里是"这一层有没有把两件事接对"）；
    /// 钉的是三件本层的事：预设与 G-code 真的喂进了管线、落点是输入那一份、收尾干净。
    #[test]
    fn the_reference_gcode_runs_through_and_updates_the_input_in_place() {
        let d = tempfile::tempdir().expect("临时目录");
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let toml = d.path().join("A1_MINI-fast.toml");
        std::fs::copy(
            manifest.join("../crates/preset/assets/presets/A1_MINI-fast.toml"),
            &toml,
        )
        .expect("拷一份 A1 mini 预设");
        let gcode = d.path().join("ref.gcode");
        std::fs::copy(
            manifest.join("../crates/postprocess/tests/golden/42274.2.gcode"),
            &gcode,
        )
        .expect("拷一份整链参考 G-code");
        let before = std::fs::read(&gcode).expect("读原份");

        let out = run_headless(&toml, &gcode).expect("整链参考物必须跑通");
        assert_eq!(out.output, gcode, "原地覆盖：落点就是输入那一份");
        let after = std::fs::read(&gcode).expect("读产物");
        assert_ne!(before, after, "跑完之后那份字节必须变了");
        assert!(
            after.len() > before.len(),
            "后处理是往里加涂胶 / 支撑代码，产物只会更长（{} → {}）",
            before.len(),
            after.len()
        );
        let leftovers: Vec<_> = std::fs::read_dir(d.path())
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n != "ref.gcode" && n != "A1_MINI-fast.toml")
            .collect();
        assert!(leftovers.is_empty(), "中间文件残留：{leftovers:?}");
    }

    /// 界面那句话的拆法：**里层码**与**原因**分家。
    ///
    /// 这条钉的是 2026-10-09 作者点名的那个 bug：「停只显示了个什么错误码而已，
    /// 根本就没显示正确的错误原因」。
    #[test]
    fn the_display_sentence_splits_the_code_from_the_reason() {
        // 内核那句的实测形态（两层码：外层是变体名，里层才是真因）
        let err = HookError::Failed(
            "处理失败 [E_CFG_INVALID_001]：配置无效: E_GCODE_BOUNDARY_001: \
             模型超出打印边界，当前X：185.9mm，允许范围：≤180.0mm"
                .to_owned(),
        );
        let (why, code) = describe(&err);
        assert_eq!(
            code.as_deref(),
            Some("E_GCODE_BOUNDARY_001"),
            "要留**里层**那个更具体的码"
        );
        assert_eq!(why, "模型超出打印边界，当前X：185.9mm，允许范围：≤180.0mm");
        assert!(!why.contains("E_"), "人话里不许再夹码：{why}");

        // 像码的那一截**不在开头**也要认得出来（预设不可用那句前面还有自己的前缀）
        let err = HookError::BadInput(
            "预设不可用 [E_CFG_PARSE_001]：TOML 解析失败 : 这份预设比本程序新".to_owned(),
        );
        let (why, code) = describe(&err);
        assert_eq!(code.as_deref(), Some("E_CFG_PARSE_001"));
        assert!(why.starts_with("TOML 解析失败"), "{why}");

        // 没有码的那几句：原样给人话
        let err = HookError::BadInput("预设文件不存在：C:\\x\\A1.toml".to_owned());
        let (why, code) = describe(&err);
        assert!(code.is_none());
        assert!(why.contains("预设文件不存在"), "{why}");

        // 取消：**不给码**，只给状态（用户自己按的停止不是"错误"）
        let err = HookError::Cancelled("已取消 [E_SYS_CANCELLED_001]：原文件没有被改动".to_owned());
        let (why, code) = describe(&err);
        assert!(code.is_none(), "用户按的停止不该摆错误码");
        assert!(why.contains("原文件没有被改动"), "{why}");
    }

    /// **「点了停止就该立刻停」**：跑到一半置位取消，管线必须马上收手。
    ///
    /// 内核每个步骤边界查一次、长 pass 内每 100ms 查一次（`DEFAULT_CANCEL_CHECK_INTERVAL`），
    /// 所以判据给 **1 秒**是留线程调度的余量 —— 不是放宽（作者原话：
    /// 「他应该立刻，马上，一秒钟都不耽误的就停止」）。
    #[test]
    fn a_mid_run_cancel_stops_within_a_tick() {
        use std::sync::Arc;
        use std::time::Duration;

        let d = tempfile::tempdir().expect("临时目录");
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let toml = d.path().join("A1_MINI-fast.toml");
        std::fs::copy(
            manifest.join("../crates/preset/assets/presets/A1_MINI-fast.toml"),
            &toml,
        )
        .expect("拷一份预设");
        let gcode = d.path().join("ref.gcode");
        std::fs::copy(
            manifest.join("../crates/postprocess/tests/golden/42274.2.gcode"),
            &gcode,
        )
        .expect("拷一份整链参考 G-code");

        let cancel = CancelToken::new();
        let trigger = cancel.clone();
        let flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag2 = flag.clone();
        let stopper = std::thread::spawn(move || {
            // 等管线真的跑起来（第一条进度）再取消 —— 否则测的是"起点就取消"
            while !flag2.load(std::sync::atomic::Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(5));
            }
            trigger.cancel();
        });

        let mut sink = move |_e: postprocess::pipeline::ProgressEvent| {
            flag.store(true, std::sync::atomic::Ordering::SeqCst)
        };
        let started = Instant::now();
        let done = run(
            &job(&toml, &gcode),
            &mut sink,
            &cancel,
            MismatchPolicy::Refuse,
            false,
        );
        let took = started.elapsed();
        stopper.join().expect("停手那个线程不该炸");

        let err = done.expect_err("取消必须报错");
        assert!(err.cancelled(), "要落在取消那一档：{err}");
        assert!(
            took < Duration::from_secs(1),
            "取消用了 {took:?} —— 太慢（内核该在一个检查间隔内收手）"
        );
    }

    /// 看门狗判据：正常 / 没进展 / 整趟超时三档，且**整趟超时优先**
    /// （真跑 30 分钟才能验的东西不算判据，所以这里比的是时间点）
    #[test]
    fn the_watchdog_speaks_up_only_when_it_should() {
        let idle = Duration::from_secs(600);
        let total = Duration::from_secs(1800);
        let started = Instant::now();

        assert_eq!(
            watchdog_verdict(started, started, started, idle, total),
            WatchdogVerdict::Fine,
            "刚起步：什么都不说"
        );
        assert_eq!(
            watchdog_verdict(
                started + Duration::from_secs(300),
                started,
                started + Duration::from_secs(300),
                idle,
                total
            ),
            WatchdogVerdict::Fine,
            "5 分钟前还有进度：正常"
        );
        assert_eq!(
            watchdog_verdict(
                started + Duration::from_secs(700),
                started,
                started + Duration::from_secs(100),
                idle,
                total
            ),
            WatchdogVerdict::Idle,
            "最后 10 分钟一动不动：卡住"
        );
        assert_eq!(
            watchdog_verdict(
                started + Duration::from_secs(1801),
                started,
                started + Duration::from_secs(1800),
                idle,
                total
            ),
            WatchdogVerdict::Overrun,
            "超过整趟上限：哪怕刚刚还有进度也收手"
        );
    }

    /// 机型判据本身：认不出（UNKNOWN / 空）**不算**不匹配；大小写差异也不算
    #[test]
    fn the_mismatch_rule_does_not_cry_wolf() {
        assert!(!is_machine_mismatch("A1", "A1"));
        assert!(!is_machine_mismatch("A1", "a1"));
        assert!(!is_machine_mismatch("A1", "UNKNOWN"));
        assert!(!is_machine_mismatch("A1", ""));
        assert!(is_machine_mismatch("A1", "A1_MINI"));
    }
}
