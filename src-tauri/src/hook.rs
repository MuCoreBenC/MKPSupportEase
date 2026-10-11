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
//!
//! ★ **切片器那一框到底吃什么**（2026-10-11 查了它的源码，见
//! `docs/HOOK-ERRORS-AND-MESSAGES.md`）：macOS 上它把钩子的 **stderr 整段**摆在
//! `Output:` 那一段里（所以"少写一行"是真的看得见的差别）；**Windows 上它根本不接管道**
//! （`bInheritHandles=false`，那个 `std_err` 形参压根没用），只显示命令 + `Error code: N`。
//! 于是"让切片器多显示 / 少显示"这件事，在 macOS 上靠我们写什么、在 Windows 上**做不到**。
//!
//! # 文案住哪
//!
//! **不在这里**。所有用户可见的句子都在 `postprocess/messages.toml`
//! （[`Reason`] + [`keys`]），这一层只决定"给谁看哪一面"：
//! 切片器看 `slicer` 那一面、界面与执行记录看 `text` 那一面。
//!
//! # 日志
//!
//! 这一趟的日志**单开一条流**：`<内部根>/logs/hook.log.<日期>`
//! （[`crate::obs::hook_log`]）——与界面那条 `supportease.log.<日期>` 分家，
//! 理由见那个模块头。失败时的结论行后面会跟一行**日志路径**（表里 `hook.log_path`）。

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use postprocess::diag::{messages, CancelToken, PostprocError};
use postprocess::pipeline::{self, IrProcessRequest, NoProgress, ProgressSink};

use crate::archive::{ArchivePaths, Detail, FileFact, Meta, PipelineStatus, PipelineStep};
use crate::args::HookJob;

/// 成功。
pub const EXIT_OK: u8 = 0;
/// 跑到一半失败，或**用户按了停止**（成熟版把"取消"也归 1，我们跟它）。
pub const EXIT_FAILED: u8 = 1;
/// 输入或预设本身不可用（文件不在、预设读不出来、机型不匹配而没人回答）。
pub const EXIT_BAD_INPUT: u8 = 2;

/// 本层用到的**文案键**。句子本体住 `postprocess/messages.toml`，这里只引用名字。
///
/// ★ 为什么不就地写字符串常量：判据要把它们挨个拿去表里问"这条在不在"
/// （见 [`ALL_KEYS`]）。敲错一个键的表现是**用户眼前的框里**出现
/// `［未登记的文案 hook.xxx］`，那种错不该靠眼睛在 2000 行里找。
mod keys {
    pub const PRESET_MISSING: &str = "hook.preset_missing";
    pub const GCODE_MISSING: &str = "hook.gcode_missing";
    pub const GCODE_UNREADABLE: &str = "hook.gcode_unreadable";
    pub const MISMATCH_QUESTION: &str = "hook.mismatch_question";
    pub const MISMATCH_STOPPED: &str = "hook.mismatch_stopped";
    pub const MISMATCH_REFUSE: &str = "hook.mismatch_refuse";
    pub const CANCELLED: &str = "hook.cancelled";
    pub const STALLED: &str = "hook.stalled";
    pub const OVERRUN: &str = "hook.overrun";
    pub const OK: &str = "hook.ok";
    pub const WARNING: &str = "hook.warning";
    pub const ARCHIVE_UNAVAILABLE: &str = "hook.archive_unavailable";
    pub const ARCHIVE_BACKUP_FAILED: &str = "hook.archive_backup_failed";
    pub const ARCHIVE_OUTPUT_FAILED: &str = "hook.archive_output_failed";
    pub const ARCHIVE_FACT_FAILED: &str = "hook.archive_fact_failed";
    pub const ARCHIVE_SHA_FAILED: &str = "hook.archive_sha_failed";
    pub const ARCHIVE_META_FAILED: &str = "hook.archive_meta_failed";
    pub const LINE_FAILED: &str = "hook.line.failed";
    pub const LINE_FAILED_NOCODE: &str = "hook.line.failed_nocode";
    pub const LINE_STOPPED: &str = "hook.line.stopped";
    pub const LINE_CANCELLED: &str = "hook.line.cancelled";
    pub const LOG_PATH: &str = "hook.log_path";
}

/// 本层用到的**全部**键（判据遍历它）。
///
/// 加新键时**这里也要加** —— 这是本文件唯一一处"两处都改"的地方，判据
/// `every_key_the_hook_uses_exists_in_the_table` 负责咬住"表里有没有这条"，
/// 而"这个数组有没有漏"由 reviewer 看 diff（漏了只是那条键少一层保护，不会撒谎）。
#[cfg(test)]
const ALL_KEYS: &[&str] = &[
    keys::PRESET_MISSING,
    keys::GCODE_MISSING,
    keys::GCODE_UNREADABLE,
    keys::MISMATCH_QUESTION,
    keys::MISMATCH_STOPPED,
    keys::MISMATCH_REFUSE,
    keys::CANCELLED,
    keys::STALLED,
    keys::OVERRUN,
    keys::OK,
    keys::WARNING,
    keys::ARCHIVE_UNAVAILABLE,
    keys::ARCHIVE_BACKUP_FAILED,
    keys::ARCHIVE_OUTPUT_FAILED,
    keys::ARCHIVE_FACT_FAILED,
    keys::ARCHIVE_SHA_FAILED,
    keys::ARCHIVE_META_FAILED,
    keys::LINE_FAILED,
    keys::LINE_FAILED_NOCODE,
    keys::LINE_STOPPED,
    keys::LINE_CANCELLED,
    keys::LOG_PATH,
];

/// 一条「原因」：**文案键 + 具名参数**，句子按面现场渲染（界面 / 切片器 / 记录 / 日志）。
///
/// # 为什么不是一个拼好的 `String`（旧形态）
///
/// 同一条错误要在几个面上说不同的话（表里的 `text` 与 `slicer`）。拼成字符串之后就只剩
/// **反着拆**这一条路 —— 旧版 `split_code` 就是干这个的：扫全文取"最后一个像 `E_*_NNN`
/// 的 token"当码、再裁掉前缀当原因。它是 2026-10-09 那个 bug（界面只显示错误码、不显示
/// 原因）的补丁，而它自己又带来了新噪音（两层码糊在一句里，`处理失败 [E_CFG_INVALID_001]：
/// 配置无效: E_GCODE_BOUNDARY_001: …` —— 外层是变体名、里层才是真因，用户两边都读不懂）。
///
/// 现在：**码与参数一路是结构化的**，句子只在要给人看的那一刻从表里渲出来。
#[derive(Debug, Clone, PartialEq)]
pub struct Reason {
    key: &'static str,
    params: Vec<(&'static str, String)>,
}

impl Reason {
    pub fn new(key: &'static str) -> Self {
        Self {
            key,
            params: Vec::new(),
        }
    }

    /// 带上一个占位符的值（表里 `{名字}`）。
    pub fn with(mut self, name: &'static str, value: impl Into<String>) -> Self {
        self.params.push((name, value.into()));
        self
    }

    pub fn key(&self) -> &'static str {
        self.key
    }

    /// 对外稳定错误码 —— **由表里那条自己说了算**：条目写了 `code` 就用它，
    /// 键本身就是码（`E_*`）就按键算，零件条目（`boundary.*`）没有码。
    ///
    /// ★ 于是"这一档给不给码"是**数据**，不是散在各处的 if：
    /// 用户自己按的停止（`hook.cancelled`）没写 `code` ⇒ 界面不摆码；
    /// 看门狗那两条写了 ⇒ 摆（那种停不是用户按的，报问题要能对上号）。
    pub fn code(&self) -> Option<String> {
        messages::code_of(self.key)
    }

    /// 本程序里给用户看的那一句（界面模态框、执行记录、日志）。
    pub fn text(&self) -> String {
        messages::text(self.key, &self.as_refs())
    }

    /// 给**切片器**那一行的口径（表里的 `slicer`，缺省 = [`Self::text`]）。
    pub fn slicer(&self) -> String {
        messages::slicer(self.key, &self.as_refs())
    }

    fn as_refs(&self) -> Vec<(&str, &str)> {
        self.params.iter().map(|(k, v)| (*k, v.as_str())).collect()
    }
}

/// 三档：谁的责任（退出码与界面说法都按它分，见模块头）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HookErrorKind {
    /// 输入/预设不可用 —— 还没碰过用户的文件，原文件一字未动。
    BadInput,
    /// 处理失败 —— 内核的 `.part` 已经丢掉，原文件没被覆盖（write 步之前就返回了）。
    Failed,
    /// 用户（或 Ctrl-C / 看门狗）在半路叫停 —— 同上，原文件没被覆盖。
    Cancelled,
}

/// 一次钩子作业为什么没成。**文案不属于这里** —— 它只带 [`Reason`]（键 + 参数）。
#[derive(Debug, Clone)]
pub struct HookError {
    kind: HookErrorKind,
    reason: Reason,
}

impl HookError {
    pub fn bad_input(reason: Reason) -> Self {
        Self {
            kind: HookErrorKind::BadInput,
            reason,
        }
    }

    pub fn failed(reason: Reason) -> Self {
        Self {
            kind: HookErrorKind::Failed,
            reason,
        }
    }

    /// 「停下」这一档：用户按的停止 / Ctrl-C / 看门狗叫停 —— 界面说「已停止」，
    /// 不是「失败」（名字不叫 `cancelled` 是因为它和下面的判定 [`Self::cancelled`]
    /// 会撞名：Rust 的同名方法不允许两个签名）。
    pub fn stopped(reason: Reason) -> Self {
        Self {
            kind: HookErrorKind::Cancelled,
            reason,
        }
    }

    pub fn exit_code(&self) -> u8 {
        match self.kind {
            HookErrorKind::BadInput => EXIT_BAD_INPUT,
            HookErrorKind::Failed | HookErrorKind::Cancelled => EXIT_FAILED,
        }
    }

    /// 界面据此决定那一屏说什么：取消不是"失败"。
    pub fn cancelled(&self) -> bool {
        self.kind == HookErrorKind::Cancelled
    }

    pub fn reason(&self) -> &Reason {
        &self.reason
    }

    /// 「用户自己按的停止」那一条：Ctrl-C 与界面上的停止按钮都归它（**无码** ——
    /// 那是他的选择，不是错误；见 [`Reason::code`]）。
    pub fn user_cancelled() -> Self {
        Self::stopped(Reason::new(keys::CANCELLED))
    }

    /// 把"哪一档"改成**输入/预设错**（退出码 2）。
    ///
    /// 为什么需要它：同一个内核错误会出现在两个位置 —— 预设加载那一步失败是
    /// **输入不可用**（`E_TOML_PARSE_001` / `E_CFG_PARSE_001` 都从那儿出），
    /// 而管线里跑出来的失败是**处理失败**。退出码按**发生的位置**分、不按错误类型分
    /// （与内核 `main.rs` 模块头同一条口径：按变体分会把"模型超出打印边界"报成"配置错"）。
    pub fn as_bad_input(mut self) -> Self {
        self.kind = HookErrorKind::BadInput;
        self
    }

    /// 界面那一屏要的两件：**原因**（人话）与**稳定错误码**。
    ///
    /// 旧版在这里做字符串拆解（[`Reason`] 的文档里登记了那段历史）。现在两件都是现成的：
    /// 原因按面渲一遍，码问表要 —— 于是"人话里夹着码"这种形态**构造不出来**。
    pub fn describe(&self) -> (String, Option<String>) {
        (self.reason.text(), self.reason.code())
    }
}

/// 内核错误 → 钩子错误：**码与参数原样搭过来**（换面重渲时用的是同一份事实）。
///
/// 内核那个 `Cancelled` 例外：它只说"有人叫停了"，而这一层知道那是用户按的还是 Ctrl-C ——
/// 统一说 [`keys::CANCELLED`] 那句（两边本来就是同一件事：这一盘没做，文件没动）。
impl From<PostprocError> for HookError {
    fn from(e: PostprocError) -> Self {
        match e {
            PostprocError::Cancelled { .. } => HookError::user_cancelled(),
            other => {
                let key = other.code();
                let reason = other
                    .params()
                    .into_iter()
                    .fold(Reason::new(key), |r, (name, value)| r.with(name, value));
                HookError::failed(reason)
            }
        }
    }
}

impl std::fmt::Display for HookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.reason.text())
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

/// 「只给界面看」的那几条消息往哪儿去（与 [`MismatchPolicy`] 同一个形状：有没有界面在这儿分）。
///
/// 为什么不并进 `sink`：`sink` 是**内核的**进度出口（`ProgressSink`，回答"现在在做什么"），
/// 而"这台机型 / 这种耗材 / 这份多大"是界面左栏那张表要摆的东西 —— 内核不该知道有那张表。
pub enum UiLine<'a> {
    /// 有界面：推过去（`run_with_channel` 给的就是它）
    Send(&'a crate::hook_ipc::Client),
    /// 没有界面：什么都不做（判据 / 极端情况那条路）
    Silent,
}

impl UiLine<'_> {
    fn send(&self, msg: crate::hook_ipc::FromHook) {
        match self {
            Self::Send(client) => client.send(&msg),
            Self::Silent => {}
        }
    }
}

/// 开场那几个事实（界面左栏那张表）。**从 IR 与输入正文里取，不重新解析任何东西**。
fn facts_from(ir: &postprocess::ir::Ir, file_size_bytes: u64) -> crate::hook_ipc::FactsPayload {
    crate::hook_ipc::FactsPayload {
        gcode_machine: ir.machine.gcode_machine_type.clone(),
        preset_machine: ir.machine.machine_type.clone(),
        filament_type: ir.filament.filament_type.clone(),
        file_size_bytes,
        nozzle_diameter: ir.machine.nozzle_diameter,
        layer_height: ir.machine.typical_layer_height,
    }
}

/// 跑一次：前置检查 → 预设 → IR → 机型核对 → 12 步管线（**原地覆盖**）。
///
/// `sink` 是进度出口（界面推事件 / 无界面时 [`NoProgress`]）；
/// `cancel` 是协作取消线（内核每个步骤入口都查它，长 pass 内也按固定间隔查）——
/// 取消落在 write 步**之前**，所以原文件不会被写坏（这一条是内核的判据，不是这里的承诺）。
///
/// `defer_print_time`：**必须确认有界面接手才传 `true`** —— 打印时间是纯报告数据，
/// 延后能省几百毫秒；但没人接手时传 `true` 会让那条记录永远停在"正在估算"。
///
/// `ui`：只给界面看的那几条（目前只有开场的事实那一条），没有界面时传 [`UiLine::Silent`]。
pub fn run(
    job: &HookJob,
    sink: &mut dyn ProgressSink,
    cancel: &CancelToken,
    mismatch: MismatchPolicy<'_>,
    defer_print_time: bool,
    ui: UiLine<'_>,
) -> Result<Outcome, HookError> {
    let started = Instant::now();

    // 前置检查（廉价）先做，且要说人话：这两句会出现在切片器的错误对话框里
    if !job.toml.is_file() {
        return Err(HookError::bad_input(
            Reason::new(keys::PRESET_MISSING).with("path", job.toml.display().to_string()),
        ));
    }
    if !job.gcode.is_file() {
        return Err(HookError::bad_input(
            Reason::new(keys::GCODE_MISSING).with("path", job.gcode.display().to_string()),
        ));
    }

    // 输入正文要读进内存：预设 → IR 的最后一步（机型 + 禁区）要拿它比 G-code 自报的机型
    let raw = std::fs::read_to_string(&job.gcode).map_err(|e| {
        HookError::bad_input(
            Reason::new(keys::GCODE_UNREADABLE)
                .with("path", job.gcode.display().to_string())
                .with("detail", e.to_string()),
        )
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
            /* 码与参数**原样搭过来**（`From<PostprocError>`）：句子由表渲染，
            这一层不再有 `预设不可用 [码]：…` 那种自己拼的前缀（那正是两层码的来路）。
            档位改成"输入不可用"：这一步失败原文件一字未动，退出码该是 2 */
            let err = HookError::from(e).as_bad_input();
            archive_failure(
                &archived,
                job,
                "",
                &conclusion_line(&err),
                false,
                raw.len() as u64,
                started_at,
                started,
            );
            return Err(err);
        }
    };

    /*
     * 一开场先把左栏那张表要的事实报过去（机型 / 耗材 / 大小 / 喷嘴 / 层高）。
     *
     * ★ 位置就在**机型核对之前**：这一刻两个机型都在手上，而下面那一问正是拿它俩比 ——
     *   用户看着表上那两行答"要不要继续"才答得明白（旧世代那屏也是这么摆的）。
     */
    ui.send(crate::hook_ipc::FromHook::Facts(facts_from(
        &ir,
        raw.len() as u64,
    )));

    // 机型核对：预设说一台、切片器那份自报另一台 —— 维度与边界全按预设算，
    // 闷头跑出来的是垃圾（旧世代那次实测：A1 的图 + A1 mini 的预设，
    // 报的是"模型超出打印边界 X：185.9mm"，把真正的原因埋在了一句听不懂的话里）。
    let preset_machine = ir.machine.machine_type.clone();
    let gcode_machine = ir.machine.gcode_machine_type.clone();
    if is_machine_mismatch(&preset_machine, &gcode_machine) {
        /* 三句（问句 / 停下 / 拒绝）共用同一份事实：两个机型名 */
        let with_machines = |key: &'static str| {
            Reason::new(key)
                .with("gcode", gcode_machine.clone())
                .with("preset", preset_machine.clone())
        };
        let question = with_machines(keys::MISMATCH_QUESTION).text();
        match mismatch {
            MismatchPolicy::Ask(asker) => {
                if !asker.confirm(&question) {
                    let err = HookError::stopped(with_machines(keys::MISMATCH_STOPPED));
                    archive_failure(
                        &archived,
                        job,
                        &preset_machine,
                        &conclusion_line(&err),
                        true,
                        raw.len() as u64,
                        started_at,
                        started,
                    );
                    return Err(err);
                }
            }
            MismatchPolicy::Refuse => {
                let err = HookError::bad_input(with_machines(keys::MISMATCH_REFUSE));
                archive_failure(
                    &archived,
                    job,
                    &preset_machine,
                    &conclusion_line(&err),
                    false,
                    raw.len() as u64,
                    started_at,
                    started,
                );
                return Err(err);
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
            let e = HookError::from(err);
            archive_failure(
                &archived,
                job,
                &preset_machine,
                &conclusion_line(&e),
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
        warnings.push(messages::text(keys::ARCHIVE_UNAVAILABLE, &[]));
        return None;
    };
    match crate::archive::begin(&root, gcode, raw, crate::archive::now_local()) {
        Ok(paths) => Some(paths),
        Err(e) => {
            warnings.push(messages::text(
                keys::ARCHIVE_BACKUP_FAILED,
                &[("detail", &e.to_string())],
            ));
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
            warnings.push(messages::text(
                keys::ARCHIVE_OUTPUT_FAILED,
                &[("detail", &e.to_string())],
            ));
        }
        match crate::archive::sha256_of(&done.output_path) {
            Ok(sha) => match crate::archive::file_fact(&done.output_path, None, sha) {
                Ok(fact) => Some(fact),
                Err(e) => {
                    warnings.push(messages::text(
                        keys::ARCHIVE_FACT_FAILED,
                        &[("detail", &e.to_string())],
                    ));
                    None
                }
            },
            Err(e) => {
                warnings.push(messages::text(
                    keys::ARCHIVE_SHA_FAILED,
                    &[("detail", &e.to_string())],
                ));
                None
            }
        }
    };

    // 报告字段面（`detail.glue/tower/slicer` + `pipeline[].detail`）的素材 ——
    // 全部是已经在手的事实，报告组装只做搬运（见 `archive::ReportFacts`）。
    let preset_name = file_name_of(&job.toml);
    let output_bytes = std::fs::metadata(&done.output_path).map(|m| m.len()).ok();
    let facts = crate::archive::ReportFacts {
        preset_name: &preset_name,
        input_lines,
        input_bytes: input_size_bytes,
        input_sha256: &done.input_sha256,
        output_bytes,
        in_place: done.output_path == job.gcode,
        ir: &done.detail_facts.ir,
        stats: &done.stats.pass1,
        tower_height: done.stats.tower_height,
        header: &done.header_facts,
        calibration_mode: &done.detail_facts.calibration_mode,
    };
    let pipeline: Vec<PipelineStep> = done
        .step_timings
        .iter()
        .map(|t| PipelineStep::from_timing(t, PipelineStatus::Complete, facts.step_detail(t.step)))
        .collect();
    let detail = Detail {
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
        estimated_print_seconds: done.print_time.as_ref().map(|pt| pt.total_seconds),
        glue: Some(facts.glue()),
        tower: Some(facts.tower()),
        slicer: Some(facts.slicer()),
    };
    let meta = Meta {
        schema_version: Meta::SCHEMA_VERSION,
        started_at: crate::archive::display_stamp(started_at),
        finished_at: crate::archive::display_stamp(crate::archive::now_local()),
        elapsed_ms: started.elapsed().as_millis() as u64,
        preset_name,
        machine_type: machine.to_string(),
        preset_path: job.toml.display().to_string(),
        gcode_path: job.gcode.display().to_string(),
        // 3MF 模型名：内核在 parse 步取的那一格（G-code 还在临时目录里时读的）。
        // 失败了也有 —— 见 `archive_failure`。
        model_name: done.header_facts.model_name.clone(),
        invocation: crate::archive::invocation(),
        warnings: done.warnings.clone(),
        error: None,
        cancelled: false,
        pipeline,
        detail,
    };
    if let Err(e) = crate::archive::write_meta(paths, &meta) {
        warnings.push(messages::text(
            keys::ARCHIVE_META_FAILED,
            &[("detail", &e.to_string())],
        ));
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
        // 失败 / 取消没有 IR 终态（没有 `HeaderFacts`），模型名**单独取一次** ——
        // G-code 还在切片器临时目录里，这一格照样拿得到（列表里那条失败记录也该有它）。
        model_name: postprocess::gcode::model_name(&job.gcode),
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
            estimated_print_seconds: None,
            // 失败 / 取消没有 IR 终态 ⇒ 三组分包整组不写（页面口径：没给就不摆）
            glue: None,
            tower: None,
            slicer: None,
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

/// **结论那一行**：切片器的 stderr、执行记录的 `error` 字段共用这一处。
///
/// 三档分得开：失败且**有码** ⇒ `处理失败：<原因>［<码>］`；失败但没码（理论分支，表里
/// 也留了一条）⇒ `处理失败：<原因>`；取消/看门狗 ⇒ `已取消：…` / `已停止：…［码］`。
///
/// ★ 这里**一个字都不写死**：四个框子都在 `messages.toml`（`hook.line.*`）。
/// 曾经这一行是 `处理失败 [外层码]：外层变体名: 里层码: 真原因` ——
/// 用户在一句话里读到两个码，谁也看不出哪个才是原因（2026-10-09 作者点名的那条）。
pub fn conclusion_line(err: &HookError) -> String {
    let reason = err.reason();
    let params = |reason_text: &str, code: &str| {
        vec![
            ("reason", reason_text.to_string()),
            ("code", code.to_string()),
        ]
    };
    match (err.cancelled(), reason.code()) {
        (false, Some(code)) => {
            let p = params(&reason.slicer(), &code);
            messages::text(keys::LINE_FAILED, &refs(&p))
        }
        (false, None) => {
            let p = params(&reason.slicer(), "");
            messages::text(keys::LINE_FAILED_NOCODE, &refs(&p))
        }
        (true, Some(code)) => {
            let p = params(&reason.slicer(), &code);
            messages::text(keys::LINE_STOPPED, &refs(&p))
        }
        (true, None) => {
            let p = params(&reason.slicer(), "");
            messages::text(keys::LINE_CANCELLED, &refs(&p))
        }
    }
}

/// 成功那一行（界面结论与 stderr 同一句）。**公开**是给假钩子
/// （`bin/hook-sim.rs`）用的：演示屏上的字因此与真跑一次一模一样，不再是手抄一份。
pub fn success_line(output: &Path, elapsed_ms: u128) -> String {
    messages::text(
        keys::OK,
        &[
            ("output", &output.display().to_string()),
            ("ms", &elapsed_ms.to_string()),
        ],
    )
}

/// 失败/取消时往 stderr 写的**那几行**（**纯函数**，判据咬得住"日志路径那一行在不在"）。
///
/// 第一行是结论（切片器框里就它）；第二行（可选）是这一趟的日志落点。
pub fn failure_lines(err: &HookError, log: Option<&Path>) -> Vec<String> {
    let mut lines = vec![conclusion_line(err)];
    if let Some(path) = log {
        lines.push(messages::text(
            keys::LOG_PATH,
            &[("path", &path.display().to_string())],
        ));
    }
    lines
}

/// `Vec<(&str, String)>` → 渲染要的 `&[(&str, &str)]`（两条渲染路径都过这里，不各写一遍）。
fn refs<'a>(params: &'a [(&'static str, String)]) -> Vec<(&'static str, &'a str)> {
    params.iter().map(|(k, v)| (*k, v.as_str())).collect()
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

    /* 内部根（钩子这侧没有 AppHandle，靠标识符自己算）：两处要用 —— 端点文件与日志目录。
    算不出来 ⇒ 一条"没有界面、也没有日志"的线：照旧干活（见 `obs::hook_log` 模块头） */
    let root = crate::fsx::paths::internal_root_headless();
    let log = root
        .as_ref()
        .and_then(|root| crate::obs::hook_log::init(&root.join("logs")));
    tracing::info!(
        input = %job.gcode.display(),
        preset = %job.toml.display(),
        "后处理开始"
    );

    let client = match &root {
        Some(root) => crate::hook_ipc::Client::connect_or_spawn(root, &cancel),
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
    let done = run(
        job,
        &mut sink,
        &cancel,
        mismatch,
        client.connected(),
        UiLine::Send(&client),
    );
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

    /* stderr 那一行（切片器会把 stderr 给用户看）与退出码走同一条路 —— 两处口径不许各写一份。
    日志路径只在**真有那份日志**时给（装不上就别指一个不存在的地方） */
    ExitCode::from(report(done, log.as_deref()))
}

/// 没有通道的那条路（判据 / 极端情况）：跑一次，结论只写 stderr。
pub fn run_and_report(job: &HookJob) -> ExitCode {
    let cancel = CancelToken::new();
    let mut sink = NoProgress;
    /* 这条路同样单开一条日志流（判据走它，也能顺手验"日志装得上"） */
    let log = crate::fsx::paths::internal_root_headless()
        .and_then(|root| crate::obs::hook_log::init(&root.join("logs")));
    tracing::info!(
        input = %job.gcode.display(),
        preset = %job.toml.display(),
        "后处理开始（无界面那条路）"
    );
    // 没有界面 ⇒ 打印时间必须自己算完（`defer = false`），否则记录永远停在"正在估算"
    let done = run(
        job,
        &mut sink,
        &cancel,
        MismatchPolicy::Refuse,
        false,
        UiLine::Silent,
    );
    ExitCode::from(report(done, log.as_deref()))
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
    /// 看门狗叫停的理由（**键 + 参数**，与作业失败共用那张表）。
    reason: Mutex<Option<Reason>>,
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

    fn reason(&self) -> Option<Reason> {
        self.reason.lock().expect("锁没坏").clone()
    }

    fn stop_with(&self, why: Reason) {
        let mut g = self.reason.lock().expect("锁没坏");
        if g.is_none() {
            *g = Some(why);
        }
    }
}

/// 看门狗那一圈：到点就**请求取消**并留下理由（界面据此说清是哪一种停）。
///
/// 两种叫停各自**有自己的码**（`E_SYS_STALLED_001` / `E_SYS_TIMEOUT_001`）：
/// 与用户自己按的停止分开 —— 那一种没码（用户的选择不用报错码），
/// 这一种是**系统替用户停的**，日志与报告里要能搜到（作者 2026-10-11 点名要的
/// "处理过程中的中断"）。
fn spawn_watchdog(watch: Arc<Watch>, cancel: CancelToken) {
    std::thread::spawn(move || {
        let started = Instant::now();
        loop {
            std::thread::sleep(WATCHDOG_TICK);
            if cancel.is_cancelled() {
                return; // 已经有人叫停了（用户按的 / Ctrl-C），不用再看
            }
            let verdict = watchdog_verdict(
                Instant::now(),
                started,
                watch.last(),
                WATCHDOG_IDLE,
                WATCHDOG_TOTAL,
            );
            let reason = match verdict {
                WatchdogVerdict::Fine => continue,
                WatchdogVerdict::Idle => Reason::new(keys::STALLED)
                    .with("minutes", (WATCHDOG_IDLE.as_secs() / 60).to_string()),
                WatchdogVerdict::Overrun => Reason::new(keys::OVERRUN)
                    .with("minutes", (WATCHDOG_TOTAL.as_secs() / 60).to_string()),
            };
            /* 日志那一面：叫停这件事**当场**记下来（结论行要等管线收手才出来，
            中间隔着内核的检查间隔；卡死那一档尤其要看得出"是看门狗先动的"） */
            tracing::warn!(
                code = reason.code().unwrap_or_default(),
                "{}",
                reason.text()
            );
            watch.stop_with(reason);
            cancel.cancel();
            return;
        }
    });
}

/// 结论 → 通道里那条 [`crate::hook_ipc::FinishedPayload`]。
fn finished_payload(
    done: &Result<Outcome, HookError>,
    elapsed_ms: u128,
    stage: Option<String>,
    stop_reason: Option<Reason>,
) -> crate::hook_ipc::FinishedPayload {
    match done {
        Ok(out) => crate::hook_ipc::FinishedPayload {
            ok: true,
            cancelled: false,
            message: success_line(&out.output, out.elapsed_ms),
            code: None,
            stage,
            exit_code: EXIT_OK,
            elapsed_ms: out.elapsed_ms as u64,
            output: Some(out.output.display().to_string()),
            warnings: out.warnings.clone(),
        },
        Err(err) => {
            /* 原因与码分家：界面把原因摆正中、码摆小字。
            取消那一档优先用**停止的理由**（用户按的？看门狗收的？）——
            只说"已停止"会把"为什么停"藏起来。
            ★ 码由表决定（看门狗那两条有码，用户自己按的没有），不是这里 if 出来的 */
            let reason = match (err.cancelled(), stop_reason) {
                (true, Some(stop)) => stop,
                _ => err.reason().clone(),
            };
            crate::hook_ipc::FinishedPayload {
                ok: false,
                cancelled: err.cancelled(),
                message: reason.text(),
                code: reason.code(),
                stage,
                exit_code: err.exit_code(),
                elapsed_ms: elapsed_ms as u64,
                output: None,
                warnings: Vec::new(),
            }
        }
    }
}

/// 结论 → **stderr 那一行** + 退出码。**有界面与没界面两条路共用这一处**：
/// 两处各写一份的话，迟早有一处少一行（旧世代就是这么丢过失败原因的）。
///
/// `log`：这一趟的后处理日志落点（装不上时是 `None`）。失败/取消时在结论后面补一行
/// 「完整日志：<路径>」—— 用户报问题时把那一段发给我们，比复述症状有用得多。
/// **成功不补**：那一行会挤掉产物路径，而成功时切片器根本不弹框。
pub fn report(done: Result<Outcome, HookError>, log: Option<&Path>) -> u8 {
    match done {
        Ok(out) => {
            let line = success_line(&out.output, out.elapsed_ms);
            tracing::info!(output = %out.output.display(), ms = out.elapsed_ms, "{line}");
            eprintln!("{line}");
            // 警告各占一行：那是**用户要看的东西**，不是日志
            for w in &out.warnings {
                let line = messages::text(keys::WARNING, &[("text", w)]);
                tracing::warn!("{line}");
                eprintln!("{line}");
            }
            EXIT_OK
        }
        Err(err) => {
            let lines = failure_lines(&err, log);
            /* 日志那一面：码单独成字段（能按码检索），停/失败用不同级别 ——
            "中断"与"失败"在翻日志时是第一眼要分开的 */
            let code = err.reason().code().unwrap_or_default();
            if err.cancelled() {
                tracing::warn!(code, exit_code = err.exit_code(), "{}", lines[0]);
            } else {
                tracing::error!(code, exit_code = err.exit_code(), "{}", lines[0]);
            }
            // 失败原因**一定**要出去：切片器只会把 stderr 的文字给用户看
            for line in &lines {
                eprintln!("{line}");
            }
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

    /// 开场那条事实：从 IR 与输入字节数里取，**一格都不许错位**
    /// （两个机型最容易反：`preset` 是维度与边界按它算的那一份，`gcode` 是切片器自报的）。
    #[test]
    fn the_opening_facts_come_straight_from_the_ir() {
        let mut ir = postprocess::ir::Ir::default();
        ir.machine.machine_type = "A1_MINI".to_owned();
        ir.machine.gcode_machine_type = "A1".to_owned();
        ir.filament.filament_type = "PETG".to_owned();
        ir.machine.nozzle_diameter = 0.4;
        ir.machine.first_layer_height = 0.25;
        ir.machine.typical_layer_height = 0.2;

        let f = facts_from(&ir, 3_355_443);
        assert_eq!(f.preset_machine, "A1_MINI");
        assert_eq!(f.gcode_machine, "A1");
        assert_eq!(f.filament_type, "PETG");
        assert_eq!(f.nozzle_diameter, 0.4);
        /* 层高取**常规**那一档（旧世代那张表摆的是 0.20，不是首层 0.25） */
        assert_eq!(f.layer_height, 0.2);
        assert_eq!(f.file_size_bytes, 3_355_443);
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
            UiLine::Silent,
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
        /* 码是**结构化的**（`Reason::code`），不再靠正文里夹一串 `[E_XXX_001]` ——
        所以这条判据改成问 `describe()` 要码，顺带钉住"人话里没有码" */
        let (reason, code) = err.describe();
        assert_eq!(code.as_deref(), Some("E_TOML_PARSE_001"), "{reason}");
        assert!(
            !reason.contains("E_TOML_PARSE_001"),
            "人话里不许夹码：{reason}"
        );
        assert!(reason.contains("预设解析失败"), "{reason}");
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
            UiLine::Silent,
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
            UiLine::Silent,
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

    /// 本层用到的每个文案键都**真在表里**（敲错键的表现是用户那边的框里出现
    /// `［未登记的文案 hook.xxx］`）。
    #[test]
    fn every_key_the_hook_uses_exists_in_the_table() {
        for key in ALL_KEYS {
            assert!(
                messages::has(key),
                "{key} 在 messages.toml 里没有条目 —— 新增说法必须同时补表"
            );
        }
    }

    /// **人话里不许夹码**，且每个面说自己的那份（这条钉的是 2026-10-09 作者点名的那个
    /// bug：「停只显示了个什么错误码而已，根本就没显示正确的错误原因」）。
    ///
    /// ★ 形态变了：以前这条判据喂一个**字符串**给 `HookError`，再验"拆得对不对"；
    /// 现在没有"拆"这一步（码与原因本来就是结构化进来的），所以改成喂**真的内核错误**，
    /// 验三面各自长什么样。
    #[test]
    fn each_surface_says_its_own_thing_without_gluing_codes_into_the_sentence() {
        // 内核那条边界错误：旧形态是 `InvalidConfig` 里塞一句带码的正文（于是两层码）
        let err = HookError::from(PostprocError::Boundary {
            code: "E_GCODE_BOUNDARY_001",
            sides: "当前X：185.9mm，允许范围：≤180.0mm".to_owned(),
        });

        let (reason, code) = err.describe();
        assert_eq!(
            code.as_deref(),
            Some("E_GCODE_BOUNDARY_001"),
            "码要跟最具体的那个走"
        );
        assert!(reason.starts_with("模型超出打印边界"), "{reason}");
        assert!(!reason.contains("E_"), "人话里不许夹码：{reason}");
        assert!(
            !reason.contains("配置无效"),
            "外层那句假话（变体名）不许露头：{reason}"
        );

        // 切片器那一面 = 结论框子 + 人话 + 码
        let line = conclusion_line(&err);
        assert!(line.starts_with("处理失败："), "{line}");
        assert!(line.contains(&reason), "{line} 里该有原因");
        assert!(line.contains("［E_GCODE_BOUNDARY_001］"), "{line}");
        assert!(
            !line.contains("E_CFG_INVALID_001"),
            "外层码不该出现：{line}"
        );

        /* 表里给切片器**单独写了说法**的那一条：两面必须不一样 ——
        校准形变那串机器判据留给界面/记录/日志，切片器只看"别转我的模型" */
        let rotated = HookError::from(PostprocError::CalibrationMismatch {
            code: "E_CAL_MISMATCH_001",
            message: "expected=1x2 actual=2x1".to_owned(),
        });
        let (ui, _) = rotated.describe();
        let slicer_line = conclusion_line(&rotated);
        assert!(ui.contains("expected=1x2"), "记录/界面那一面留着判据：{ui}");
        assert!(
            !slicer_line.contains("expected=1x2"),
            "切片器那一面不该看机器判据：{slicer_line}"
        );
        assert!(slicer_line.contains("请勿旋转校准模型"), "{slicer_line}");
    }

    /// 结论行分三档：**失败**（有码）、**取消/用户按的**（无码）、**停止/系统收的**（有码）。
    ///
    /// 「用户自己按的停不给码」是刻意的（那是他的选择，不是错误）；而看门狗那两种是
    /// **系统替用户停的** —— 报问题时得能对上号，所以表里给它们写了码。
    #[test]
    fn the_conclusion_line_frames_the_three_kinds() {
        let failed =
            HookError::failed(Reason::new(keys::PRESET_MISSING).with("path", "C:\\x\\A1.toml"));
        let line = conclusion_line(&failed);
        assert!(line.starts_with("处理失败："), "{line}");
        assert!(line.contains("C:\\x\\A1.toml"), "{line}");
        assert!(line.contains("［E_FS_NOT_FOUND_001］"), "{line}");

        let by_user = HookError::stopped(Reason::new(keys::CANCELLED));
        let line = conclusion_line(&by_user);
        assert!(line.starts_with("已取消："), "{line}");
        assert!(!line.contains("E_"), "用户按的停不该摆码：{line}");
        assert!(line.contains("原文件没有被改动"), "{line}");

        let by_watchdog = HookError::stopped(
            Reason::new(keys::STALLED).with("minutes", (WATCHDOG_IDLE.as_secs() / 60).to_string()),
        );
        let line = conclusion_line(&by_watchdog);
        assert!(line.starts_with("已停止："), "{line}");
        assert!(line.contains("［E_SYS_STALLED_001］"), "{line}");
    }

    /// 失败时**多给一行日志落点**（用户报问题时把它整段发给我们）；
    /// 装不上日志（`None`）时那一行不出现 —— 不指一个不存在的文件。
    #[test]
    fn a_failure_points_at_the_hook_log() {
        let err =
            HookError::failed(Reason::new(keys::PRESET_MISSING).with("path", "C:\\x\\A1.toml"));
        let with = failure_lines(
            &err,
            Some(std::path::Path::new("C:\\data\\logs\\hook.log.2026-10-11")),
        );
        assert_eq!(with.len(), 2, "{with:?}");
        assert!(with[1].starts_with("完整日志："), "{:?}", with[1]);
        assert!(with[1].contains("hook.log.2026-10-11"), "{:?}", with[1]);

        let without = failure_lines(&err, None);
        assert_eq!(without.len(), 1, "没有日志就不摆那一行：{without:?}");
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
            UiLine::Silent,
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
