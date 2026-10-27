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
use postprocess::pipeline::{self, IrProcessRequest, NoProgress, ProgressEvent, ProgressSink};
use postprocess::postproc::calibration;
use postprocess::postproc::disk::{CollisionFacts, TowerAsk, TowerAskOutcome};

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
    pub const CALIB_MODE_QUESTION: &str = "hook.calib_mode_question";
    pub const CALIB_STOPPED: &str = "hook.calib_stopped";
    /* 挪塔那一问（2026-10-18）：设置页「智能塔位置」开着且真撞了才问 */
    pub const TOWER_RELOCATION_QUESTION: &str = "hook.tower_relocation_question";
    /// 挪完了、复检通过（成功句）
    pub const TOWER_RELOCATED_OK: &str = "hook.tower_relocated_ok";
    /// 挪完了、复检**仍然撞** ⇒ 停下（第二道闸）
    pub const TOWER_RELOCATED_STILL: &str = "hook.tower_relocated_still";
    /// 用户选了自己挪模型 / 那一问超时 ⇒ 停下
    pub const TOWER_RELOCATE_DECLINED: &str = "hook.tower_relocate_declined";
    /* 提问模态框上的按钮（2026-10-26）：按钮上的字同样是「用户读到的一句话」，
    英文界面下曾经整颗按钮是中文 —— 所以它们也进文案表，不写字面量 */
    pub const CHOICE_TOWER_RELOCATE: &str = "hook.choice_tower_relocate";
    pub const CHOICE_TOWER_RELOCATE_HINT: &str = "hook.choice_tower_relocate_hint";
    pub const CHOICE_TOWER_MODEL: &str = "hook.choice_tower_model";
    pub const CHOICE_TOWER_MODEL_HINT: &str = "hook.choice_tower_model_hint";
    pub const CHOICE_CALIB_PRINT: &str = "hook.choice_calib_print";
    pub const CHOICE_CALIB_PRINT_HINT: &str = "hook.choice_calib_print_hint";
    pub const CHOICE_CALIB_DIRECT: &str = "hook.choice_calib_direct";
    pub const CHOICE_CALIB_DIRECT_HINT: &str = "hook.choice_calib_direct_hint";
    /// 板上根本没有不撞的塔位 ⇒ 停下
    pub const TOWER_RELOCATE_UNAVAILABLE: &str = "hook.tower_relocate_unavailable";
    /* 校准件那一趟的两种成功结论（`print` = 打印件保留 / `direct` = 去掉打印件）——
     * 界面与报告读同一组键，见 `hook::calib_done_line` */
    pub const CALIB_DONE_PRINT: &str = "hook.calib_done_print";
    pub const CALIB_DONE_DIRECT: &str = "hook.calib_done_direct";
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
    keys::CALIB_MODE_QUESTION,
    keys::CALIB_STOPPED,
    keys::CALIB_DONE_PRINT,
    keys::CALIB_DONE_DIRECT,
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
    /// **碰撞那一份事实**（2026-10-18；只有 `E_GCODE_COLLISION_001` 那一趟有）。
    ///
    /// 它不进文案、不进码 —— 是给执行记录那一格用的（报告页据此给建议 + 画俯视图）。
    /// 别的失败一律 `None`（于是"这条失败记录有没有俯视图"是数据说了算）。
    ///
    /// ★ `Box`：这一份带几百个冲突点，内联会把 `HookError` 撑到几百字节 ——
    /// 于是每个 `Result<Outcome, HookError>` 都跟着变大（clippy `result_large_err`
    /// 当场红，内核那个 `PostprocError::Collision` 也是同一个原因装箱）。
    collision: Option<Box<postprocess::postproc::disk::collision::CollisionFacts>>,
}

impl HookError {
    pub fn bad_input(reason: Reason) -> Self {
        Self {
            kind: HookErrorKind::BadInput,
            reason,
            collision: None,
        }
    }

    pub fn failed(reason: Reason) -> Self {
        Self {
            kind: HookErrorKind::Failed,
            reason,
            collision: None,
        }
    }

    /// 「停下」这一档：用户按的停止 / Ctrl-C / 看门狗叫停 —— 界面说「已停止」，
    /// 不是「失败」（名字不叫 `cancelled` 是因为它和下面的判定 [`Self::cancelled`]
    /// 会撞名：Rust 的同名方法不允许两个签名）。
    pub fn stopped(reason: Reason) -> Self {
        Self {
            kind: HookErrorKind::Cancelled,
            reason,
            collision: None,
        }
    }

    /// 碰撞那一趟的事实（**只有** `E_GCODE_COLLISION_001` 会带）——
    /// 报告页据此给"往哪挪多少"的建议并画俯视图。
    pub fn collision(&self) -> Option<&postprocess::postproc::disk::collision::CollisionFacts> {
        self.collision.as_ref().map(|b| b.as_ref())
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

    /// 「用户自己按的停止」那一条：Ctrl-C 与界面上的停止按钮都归它。
    ///
    /// ★ 2026-10-17 改口径：以前它是**无码**的（"那是他的选择，不是错误"），于是日志里
    /// 那一行是 `code=""` —— 恰恰是最常出现的那一行搜不到东西。现在给
    /// `W_SYS_CANCELLED_001`：仍然是"停下"不是"故障"（`W_` 打头、退出码与原来一样），
    /// 但照着码能查到它是什么。见 `messages.toml` 表头那三档前缀。
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
                /* 碰撞那一份事实**原样搭过来**（它不在文案里，只能在转换这一层接住） */
                let collision = other.collision_facts().cloned().map(Box::new);
                let key = other.code();
                let reason = other
                    .params()
                    .into_iter()
                    .fold(Reason::new(key), |r, (name, value)| r.with(name, value));
                let mut e = HookError::failed(reason);
                e.collision = collision;
                e
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
    /// 结果读数（界面「结果 · 成功」那几格）—— 见 [`result_facts`]。
    pub result: crate::hook_ipc::ResultFacts,
    /// 校准件那一趟选了哪种跑法（`print` / `direct`）；不是校准件 = `None`。
    /// 结论里带上它，界面与报告才能把两种成功分开说。
    pub calib_choice: Option<String>,
    /// **挪塔记录**（2026-10-18）：这一趟挪过塔就是 `Some`（`verified` 一定是 `true` ——
    /// 挪完仍撞那一趟走的是错误出口）。结论句与归档都读它。
    pub relocation: Option<crate::archive::RelocationRecord>,
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

    /// 校准件那一问：选「先打印，再校准」/「直接校准」。
    ///
    /// 返回被选中的 id（`print` / `direct`）；`None` = 没有界面 / 超时 / 用户按了停止 ——
    /// 由调用方决定收场（**停下**，与机型那一问同一条）。
    ///
    /// 默认实现给"没有界面"那一档用：不会替用户决定，一律 `None`。
    fn ask_calibration(&mut self, _question: &str) -> Option<String> {
        None
    }
}

/// **把内核那条"撞了"换成这一问的三句话之一**（2026-10-18 方案 B）。
///
/// 内核只回 `Collision`，它不知道用户答了什么 —— 而"自己挪 / 摆不下 / 挪了还撞"
/// 这三句的分派依据正是**这一趟落下的记录**（交互侧知识），所以留在这里。
/// ★ 句子与参数与改造前一字不差（码在 `messages.toml`，这里只挑键）。
fn tower_failure_error(
    record: &crate::archive::RelocationRecord,
    facts: CollisionFacts,
) -> HookError {
    let reason = if record.verified == Some(false) {
        /* 挪了还撞：把这趟的四个数如实说出来（与改造前同一串参数） */
        let to = record.to.unwrap_or(record.from);
        Reason::new(keys::TOWER_RELOCATED_STILL)
            .with("fromX", format!("{:.1}", record.from[0]))
            .with("fromY", format!("{:.1}", record.from[1]))
            .with("toX", format!("{:.1}", to[0]))
            .with("toY", format!("{:.1}", to[1]))
            .with("moved", format!("{:.1}", record.moved_mm.unwrap_or(0.0)))
            .with("count", facts.conflict_count.to_string())
    } else if record.choice == "unavailable" {
        Reason::new(keys::TOWER_RELOCATE_UNAVAILABLE)
            .with("count", facts.conflict_count.to_string())
    } else {
        Reason::new(keys::TOWER_RELOCATE_DECLINED).with("count", facts.conflict_count.to_string())
    };
    let mut err = HookError::failed(reason);
    err.collision = Some(Box::new(facts));
    err
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

impl<'a> UiLine<'a> {
    fn send(&self, msg: crate::hook_ipc::FromHook) {
        match self {
            Self::Send(client) => client.send(&msg),
            Self::Silent => {}
        }
    }

    /// 那条通道（`Silent` 时没有）。给"机型那一问之后还要再问一句"用 ——
    /// `MismatchPolicy` 里那次 `&mut` 借用到不了下面。
    fn client(&self) -> Option<&'a crate::hook_ipc::Client> {
        match self {
            Self::Send(client) => Some(client),
            Self::Silent => None,
        }
    }
}

/// 开场那几个事实（界面左栏那张表）。**从 IR 与输入正文里取，不重新解析任何东西**。
fn facts_from(
    ir: &postprocess::ir::Ir,
    raw: &str,
    file_size_bytes: u64,
) -> crate::hook_ipc::FactsPayload {
    /* 输入头里那几格**读文件那一刻就认**（2026-10-14 作者：「它怎么就不显示那一个喷嘴层高…
    肯定获取得到」）—— 喷嘴 / 层高 / 切片软件版本都写在头的 `;` 注释里，不必等 pass1。
    原来它们只在 pass1 的逐行循环里认，而这张表**开场就推给界面**（pass1 还没跑），
    于是真机上一直是 `—`，跑完也不补。
    取值口径（谁优先）：**这份文件自报的**（头行）> IR（预设里的；真预设的 `[toolhead]`
    段本来就不含这两项，所以通常是 0）> `0`（界面摆 `—`，见 `baseGroup`）。 */
    let head = postprocess::postproc::head_facts::read_head_facts(raw);
    crate::hook_ipc::FactsPayload {
        gcode_machine: ir.machine.gcode_machine_type.clone(),
        preset_machine: ir.machine.machine_type.clone(),
        filament_type: ir.filament.filament_type.clone(),
        file_size_bytes,
        nozzle_diameter: head.nozzle_diameter.unwrap_or(ir.machine.nozzle_diameter),
        layer_height: head.layer_height.unwrap_or(ir.machine.typical_layer_height),
        /* 这两格给「切片与预估」那一栏（跑着的时候就摆）：都来自预设这份 IR ——
         * 涂胶速度的换算与归档同一条（`archive::glue_speed_of`） */
        glue_speed: Some(crate::archive::glue_speed_of(ir)),
        /* 切片软件：**带版本号的完整串**头里就有（`; BambuStudio 02.08.02.60`）——
        跑着的时候那格就摆它，与跑完 `result.slicer` 同形同源（作者 2026-10-14 问的那句）。
        头里没有（别的切片器 / 被裁过）⇒ 退回预设里那个名字，比空着强 */
        slicer: Some(
            head.slicer_version
                .unwrap_or_else(|| ir.filament.slicer.clone()),
        ),
        /* 切片器自报的预估总时长（输入头里 `; total estimated time` 那一行）：
         * 跑着的时候那一格摆它；跑完换成内核重算的（`result.printTimeSeconds`） */
        slicer_print_time_seconds: postprocess::postproc::printtime::slicer_estimated_seconds(raw),
    }
}

/// 结果读数（界面「结果 · 成功」那几格）—— **从内核的产物里搬，不新算**：
/// IR 终态（笔偏移 / 涂胶速度）+ pass1 统计（涂胶总长 / 切片软件）+ 打印时间。
///
/// 打印时间只有"这一趟自己算完"时才有值；**延后那一趟是 `None`**
/// （钩子必须立刻退出，切片器在等退出码）—— 界面等 `hook_ui` 那条
/// `postprocess-print-time` 事件把它补回来。
fn result_facts(done: &postprocess::pipeline::ProcessResult) -> crate::hook_ipc::ResultFacts {
    let ir = &done.detail_facts.ir;
    crate::hook_ipc::ResultFacts {
        pen_x: ir.toolhead.x_offset,
        pen_y: ir.toolhead.y_offset,
        pen_z: ir.toolhead.z_offset,
        glue_speed: crate::archive::glue_speed_of(ir),
        print_time_seconds: done.print_time.as_ref().map(|pt| pt.total_seconds),
        glue_length_mm: done.stats.pass1.glue_total_length,
        slicer: done.stats.pass1.slicer_version.clone(),
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
/// `ui`：只给界面看的那一条（开场那条 `Hello`，事实跟着它一起走），没有界面时传 [`UiLine::Silent`]。
pub fn run(
    job: &HookJob,
    sink: &mut dyn ProgressSink,
    cancel: &CancelToken,
    mismatch: MismatchPolicy<'_>,
    defer_print_time: bool,
    ui: UiLine<'_>,
) -> Result<Outcome, HookError> {
    let started = Instant::now();
    /* 挪塔这一趟的记录（2026-10-18）：走到那一问才填；成功 / 失败都随归档落盘。
    声明放最前面 —— 早退的那几处（预设错 / 机型不匹配 / 校准超时）也要能传它。 */
    let mut relocation: Option<crate::archive::RelocationRecord> = None;

    /* ★ **开场那一条 `Hello` 连事实一起发**（作者 2026-10-14：「需要一起带过来」）——
    所以它不能像原来那样在 [`run_with_channel`] 一进门就发（那时还没读文件、没有事实），
    而是搬到这里：在"事实算得出来 / 算不出来"的**第一个分岔点**上发**唯一一条**
    （`hello_sent` 兜底：几个早期出口谁先到谁发，后面的不再重复）。

    四个早期出口（预设不在 / G-code 不在 / 读不动 / 载不动）**都要喊一声** ——
    不然界面根本不开窗，用户只能对着切片器那句退出码发呆。
    那一刻表里全是 `—`（`facts: None`），但"这一趟失败了"那句人话摆得出来。 */
    let mut hello_sent = false;
    let mut say_hello = |facts: Option<crate::hook_ipc::FactsPayload>| {
        if hello_sent {
            return;
        }
        hello_sent = true;
        ui.send(crate::hook_ipc::FromHook::Hello {
            preset_name: file_name_of(&job.toml),
            gcode_name: file_name_of(&job.gcode),
            facts,
        });
    };

    // 前置检查（廉价）先做，且要说人话：这两句会出现在切片器的错误对话框里
    if !job.toml.is_file() {
        say_hello(None);
        return Err(HookError::bad_input(
            Reason::new(keys::PRESET_MISSING).with("path", job.toml.display().to_string()),
        ));
    }
    if !job.gcode.is_file() {
        say_hello(None);
        return Err(HookError::bad_input(
            Reason::new(keys::GCODE_MISSING).with("path", job.gcode.display().to_string()),
        ));
    }

    // 输入正文要读进内存：预设 → IR 的最后一步（机型 + 禁区）要拿它比 G-code 自报的机型
    let raw = match std::fs::read_to_string(&job.gcode) {
        Ok(raw) => raw,
        Err(e) => {
            say_hello(None);
            return Err(HookError::bad_input(
                Reason::new(keys::GCODE_UNREADABLE)
                    .with("path", job.gcode.display().to_string())
                    .with("detail", e.to_string()),
            ));
        }
    };

    // 记账起点（本地时刻）：归档记录里的 `startedAt`。读文件之后、干活之前 ——
    // 口径是「这一趟从哪一刻开始」，不是进程启动时刻。
    let started_at = crate::archive::now_local();

    // 归档第一笔：**原件先备份**。原地覆盖模式下原文件会被处理后的内容盖掉，
    // 这一笔是它唯一的副本。失败只降级成警告：产物优先（见 `crate::archive` 模块头）。
    let mut archive_warnings: Vec<String> = Vec::new();
    let archived = archive_begin(&job.gcode, &raw, &mut archive_warnings);

    // 预设 → IR：唯一入口（`crates/preset`）。预设里的参数错、机型不认识都在这一步报出来
    let mut ir = match preset::load_ir(&job.toml, Some(&raw)) {
        Ok(ir) => ir,
        Err(e) => {
            /* 界面得开窗把这次失败摆出来（真实原因多半就在下面那句话里）—— 表里全是 `—` */
            say_hello(None);
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
                Vec::new(),
                None,
                relocation.as_ref(),
                // 这一笔在身份探测之前（预设都读不出来）⇒ 空探测："没有身份"是事实，
                // 不是"没去查"。
                &postprocess::gcode::IdentityProbe::default(),
            );
            return Err(err);
        }
    };

    /*
     * 把用户在设置页里点出来的那几档**覆盖**到 IR 上（2026-10-17）。
     *
     * ★ 位置在这里、不能挪：`preset::load_ir` 刚把预设里的值铺满整个 IR，
     *   早一步写会被它盖掉；晚一步写（进管线之后）就没用了。
     *
     * ★ 读不出来（状态文件坏了 / 目录不在）**不阻断这一趟**：偏好只是"额外怎么做"，
     *   缺了它按出厂档跑仍然是对的 —— 与 `usage` 那个账同一个口径（提示而非用户资产）。
     *   记录也照落，只是那几档是出厂值。
     */
    if let Some(root) = crate::fsx::paths::internal_root_headless() {
        if let Ok(prefs) = crate::runtime::app_state::resolved_postproc_prefs(&root) {
            crate::runtime::postproc_prefs::apply_to_ir(&mut ir, &prefs);
        }
    }

    /*
     * 一开场把左栏那张表要的事实（机型 / 耗材 / 大小 / 喷嘴 / 层高）**连开场那一条一起**报过去
     * （2026-10-14 起并成一条：界面上框一出现就是完整形态，值不会在眼前换一轮）。
     *
     * ★ 位置就在**机型核对之前**：这一刻两个机型都在手上，而下面那一问正是拿它俩比 ——
     *   用户看着表上那两行答"要不要继续"才答得明白（旧世代那屏也是这么摆的）。
     */
    say_hello(Some(facts_from(&ir, &raw, raw.len() as u64)));

    /*
     * 校准件身份探测（2026-10-18）：**必须在这里跑，且只跑这一次**。
     *
     * 证据（`origin.txt` / 原始 3MF）全在切片器的临时目录里，G-code 一被搬走就够不着。
     * 所以刚拿到路径时读一次，结果一直带到执行记录里 —— 以后复盘按记录看，
     * 不重新读盘（用户把原始 3MF 删了也解释得清当时发生了什么）。
     *
     * **身份只有一处真值**：原始 3MF 的 `MKPSE:Calibration`（真机实测：Bambu Studio
     * 另存后仍在）。G-code 里那行 `;MKPCalibration …` 整条链已删除 —— 某些版本的
     * Bambu Studio 存一次就把 `custom_gcode_per_layer.xml` 丢掉，拿它当判据会
     * "模型带身份、程序却按普通模型跑"。**G-code 里即便还留着旧注释，也一律忽略。**
     */
    let identity_probe = postprocess::gcode::probe_model_identity(&job.gcode);

    // 机型核对：预设说一台、切片器那份自报另一台 —— 维度与边界全按预设算，
    // 闷头跑出来的是垃圾（旧世代那次实测：A1 的图 + A1 mini 的预设，
    // 报的是"模型超出打印边界 X：185.9mm"，把真正的原因埋在了一句听不懂的话里）。
    let preset_machine = ir.machine.machine_type.clone();
    let gcode_machine = ir.machine.gcode_machine_type.clone();
    /* 学习账要用：机型不符的那一趟**不算**（见下面成功出口那段） */
    let machine_mismatch = is_machine_mismatch(&preset_machine, &gcode_machine);
    if machine_mismatch {
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
                        /* 管线还没进（预设错 / 停在问句），没有步骤账 */
                        Vec::new(),
                        None,
                        relocation.as_ref(),
                        &identity_probe,
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
                    /* 管线还没进（停在机型那一问），没有步骤账 */
                    Vec::new(),
                    None,
                    relocation.as_ref(),
                    &identity_probe,
                );
                return Err(err);
            }
        }
    }

    /*
     * 校准件那一问（2026-10-17）：**排在机型那一问之后**（识别先后，作者定的）——
     * 机型那问答"停"就已经 return 了，走不到这里。
     *
     * 两个选项对**模型**的后果不同：
     *   · `print`  =「先打印，再校准」→ 什么都不改，就按预设里写的那套跑（打印件保留）；
     *   · `direct` =「直接校准」→ 去掉打印件，只留校准路径（内核按
     *     `calibration_execution_mode == "direct_calibrate"` 分派，见 `postproc::calibration`）。
     *
     * 所以这个选择必须**写回 IR**再进管线 —— 界面只是把它问出来，真正的差别在内核里。
     * 超时不答 = 停下（与机型那一问同一条口径）。
     */
    let mut calib_choice: Option<String> = None;
    /*
     * `mismatch` 在机型那一段已经被 `match` 用掉了（`MismatchPolicy::Ask(&mut asker)` 是一次
     * 独占借用）。所以这里**再问一次那条线有没有界面** —— 与 `MismatchPolicy` 同一个判据
     * （`ui.connected()`），不需要把借用续命到下面。
     */
    /*
     * 身份定案：**只有一处来源** —— 3MF 里那条 `MKPSE:Calibration`。
     *
     * `None`（模型没标记 / 读不到）⇒ `Mode::None`，整盘按普通模型跑、不弹校准问句。
     * **不猜**：不看正文里的旧注释、不看尺寸、不看文件名（2026-10-18 删掉的那条
     * G-code 标记链就是"看了会给出错答案"的例子）。
     */
    let calib_mode = mode_of(&identity_probe);
    if let (true, Some(client)) = (
        postprocess::postproc::calibration::is_any(calib_mode),
        ui.client(),
    ) {
        let mut calib_asker = ChannelAsker { client };
        let kind = postprocess::postproc::calibration::mode_display_name(calib_mode);
        {
            let asker = &mut calib_asker;
            let question = Reason::new(keys::CALIB_MODE_QUESTION)
                .with("kind", kind)
                .text();
            let answer = asker.ask_calibration(&question);
            /*
             * ★★★ 这里比的是**选项 id**（界面上那颗按钮回给钩子的 `id`），不是管线常量。
             *
             * 选项 id 与管线常量是**两套不同的字**：
             *   · 选项 id（`hook_ipc::CHOICE_CALIB_*`）= `"print"` / `"direct"` —— 界面回的是这个；
             *   · 管线常量（`ir::CALIBRATION_EXEC_*`）  = `"print_then_calibrate"` / `"direct_calibrate"`
             *     —— 写进 IR 给内核分派的是这个。
             *
             * 第一版拿 `answer` 直接跟 `CALIBRATION_EXEC_DIRECT_CALIBRATE` 比，`"direct"` 永远
             * 不等于 `"direct_calibrate"` ⇒ 无论选哪颗都掉进下面那个兜底臂，"直接校准"被当成
             * "先打印，再校准"跑（作者真机：选了直接校准，切出来还是有打印件）。
             * 判据见 `tests::a_direct_choice_really_writes_direct_into_the_ir`。
             */
            match answer.as_deref() {
                Some(id) => {
                    ir.safety.calibration_execution_mode = calib_exec_mode_of(id).to_owned();
                    calib_choice = Some(calib_choice_of(id).to_owned());
                }
                /* 超时 / 没有界面 / 按了停止：停下，原文件一字不动 */
                None => {
                    let err =
                        HookError::stopped(Reason::new(keys::CALIB_STOPPED).with("kind", kind));
                    archive_failure(
                        &archived,
                        job,
                        &preset_machine,
                        &conclusion_line(&err),
                        true,
                        raw.len() as u64,
                        started_at,
                        started,
                        /* 管线还没进（预设错 / 停在问句），没有步骤账 */
                        Vec::new(),
                        None,
                        relocation.as_ref(),
                        &identity_probe,
                    );
                    return Err(err);
                }
            }
        }
    }

    /*
     * **第 6 步那一问的钩子侧**（2026-10-18 方案 B）：内核在 `Step::Collision` 撞了会
     * 回调它 —— 界面交互、选择记账都留在这边，内核只认 `TowerAsk` 那条缝。
     *
     * 只在**装了界面 + 设置页开关开着**时才递进去：
     *   · 没界面（无头 / 静默档）/ 开关关着 ⇒ `None` ⇒ 内核走"没人问"那条短路，
     *     行为与改造前一字不差；
     *   · 递进去之后，等不到答复由这边收成 `Declined`（超时）或 `Cancelled`（按了停止）。
     *
     * ★ 这里**不再**预检：擦料决策 / 碰撞扫描都只由管线各跑一次。
     */
    let mut tower_ask = if ir.prefs.auto_tower_position {
        ui.client().map(|client| ChannelTowerAsk {
            client,
            cancel,
            record: None,
        })
    } else {
        None
    };

    // 12 步管线。`output_path: None` = 原地覆盖（`.part` + rename，见内核 pipeline 文档）
    /* 步骤记账包在真正的进度出口外面：失败 / 取消时把"跑到哪一步"写进记录
    （成功路径用内核自己那份 step_timings，这份记账用不上） */
    let mut steps = StepRecorder {
        inner: sink,
        current: None,
        done: Vec::new(),
    };
    let ask: Option<&mut dyn TowerAsk> = tower_ask.as_mut().map(|a| a as &mut dyn TowerAsk);
    let outcome = if defer_print_time {
        pipeline::process_with_ir_deferred_print_time_with_tower_ask(
            IrProcessRequest {
                gcode_path: job.gcode.clone(),
                ir,
                output_path: None,
                // 身份从钩子带进管线：**管线不许自己再认一遍**（认也认不出来，G-code 里没有）
                calibration: calib_mode,
            },
            ask,
            &mut steps,
            cancel,
        )
    } else {
        pipeline::process_with_ir_with_tower_ask(
            IrProcessRequest {
                gcode_path: job.gcode.clone(),
                ir,
                output_path: None,
                // 身份从钩子带进管线：**管线不许自己再认一遍**（认也认不出来，G-code 里没有）
                calibration: calib_mode,
            },
            ask,
            &mut steps,
            cancel,
        )
    };
    /* 问句那一侧的记账**先取出来**：下面失败 / 成功两条路都要用它（挑句子、写归档） */
    relocation = tower_ask.as_mut().and_then(ChannelTowerAsk::take_record);
    let done = match outcome {
        Ok(done) => done,
        Err(err) => {
            let mut e = HookError::from(err);
            /*
             * **撞塔那一问落下的三种收场**（2026-10-18 方案 B）：内核统一回 `Collision`，
             * 但它不知道用户答了什么 —— "自己挪 / 板上摆不下 / 挪了还撞"这三句的挑法
             * 只能看这一趟落下的记录。事实（撞在哪、建议）仍用错误里那份。
             */
            if let Some(record) = relocation.as_ref() {
                if let Some(facts) = e.collision().cloned() {
                    e = tower_failure_error(record, facts);
                }
            }
            let failed_pipeline = steps.finish(if e.cancelled() {
                PipelineStatus::Cancelled
            } else {
                PipelineStatus::Error
            });
            /* 碰撞那一趟把"撞在哪 + 建议"一起归档（报告页据此给建议、画俯视图）；
            别的失败 `None` —— 那一格照旧是 `null` */
            let collision = e.collision().cloned();
            archive_failure(
                &archived,
                job,
                &preset_machine,
                &conclusion_line(&e),
                e.cancelled(),
                raw.len() as u64,
                started_at,
                started,
                failed_pipeline,
                collision,
                relocation.as_ref(),
                &identity_probe,
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
            relocation.as_ref(),
            &identity_probe,
        );
    }

    /* 结果读数先摘下来（下面 `done` 被逐格搬走：warnings / output_path 都是 move）——
     * 它就是界面「结果 · 成功」那几格，值全取自内核产物，见 `result_facts` */
    let result = result_facts(&done);

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

    /*
     * **学习账记一次**（2026-10-17 晚定口径，作者：「不是使用就增加，是后处理才加」
     * 「机型不符、后面失败不算」）。
     *
     * 位置就在成功出口：走到这儿说明这一趟**真的跑成了**。机型不符的那一趟跳过
     * （哪怕用户选择"继续"，它用的也是一份对不上机型的预设，不该给那份加分）。
     *
     * 钥匙 = 切片器给的绝对路径 → `local:` / `cloud:` 前缀（[`runtime::usage::key_of_path`]）；
     * 落在两个根之外的 TOML（用户放桌面）就不记。写不进去只记日志 ——
     * 这一趟后处理绝不能因为学习账失败而失败。
     */
    if !machine_mismatch {
        if let (Some(internal), Some(user)) = (
            crate::fsx::paths::internal_root_headless(),
            crate::fsx::paths::user_root_headless(),
        ) {
            if let Some(key) = crate::runtime::usage::key_of_path(&internal, &user, &job.toml) {
                if let Err(err) = crate::runtime::usage::bump(&internal, &key) {
                    eprintln!("[usage] 记一次后处理失败（不影响这一趟）：{err}");
                }
            }
        }
    }

    Ok(Outcome {
        output: done.output_path,
        elapsed_ms: started.elapsed().as_millis(),
        warnings,
        deferred_print_time,
        result,
        calib_choice,
        relocation,
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
    relocation: Option<&crate::archive::RelocationRecord>,
    // 校准件身份探测结果（在机型核对之前那一次读盘拿到的，见 `probe_model_identity`）
    identity: &postprocess::gcode::IdentityProbe,
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

    /*
     * **参数快照**（2026-10-21）：把"这一刻预设 TOML 里写着什么"读出来存进记录。
     *
     * 为什么不留着"回头再读"：用户切完往往接着改预设（真机就是这样把擦料塔从 20 改到
     * 101 的），过了这一刻，那份 TOML 已经不在了 —— 报告里显示的就会是"现在的值"，
     * 而用户问的恰恰是"当时那一盘用的是哪个值"。
     *
     * 读不出来不算错：快照整组不摆，记录其余部分照写（与 `detail.glue/tower` 同一口径）。
     */
    let preset_text = std::fs::read_to_string(&job.toml).ok();
    let preset_catalog = crate::fsx::paths::internal_root_headless()
        .and_then(|root| crate::runtime::load_released_catalog(&root).ok());
    let preset_params = preset_text
        .as_deref()
        .zip(preset_catalog.as_ref())
        .and_then(|(text, catalog)| {
            crate::presetdata::params::read_param_values(text, &catalog.registry.params).ok()
        });

    /*
     * **这一份预设里读不出来的参数**（2026-10-21 作者：「报告页面能看到这些对应的错误」）：
     * 与它最初的原文镜像比，缺的项列出来 —— 缺项不报错，但那一项会退回默认值，
     * 结果一定不是用户想的那样。读不到镜像 / 不在用户根 ⇒ 空（不编）。
     */
    let preset_missing: Vec<String> = (|| {
        let root = crate::fsx::paths::internal_root_headless()?;
        let user = crate::fsx::paths::user_root_headless()?;
        let rel = crate::runtime::usage::key_of_path(&root, &user, &job.toml)?
            .strip_prefix("local:")
            .map(str::to_owned)?;
        let origin = crate::runtime::baseline::read_origin_copy(&root, &rel).ok()??;
        let catalog = preset_catalog.as_ref()?;
        let text = preset_text.as_deref()?;
        let lost =
            crate::presetdata::params::missing_param_keys(&origin, text, &catalog.registry.params)
                .ok()?;
        Some(lost.into_iter().map(|(_, label)| label).collect())
    })()
    .unwrap_or_default();

    // 报告字段面（`detail.glue/tower/slicer` + `pipeline[].detail`）的素材 ——
    // 全部是已经在手的事实，报告组装只做搬运（见 `archive::ReportFacts`）。
    let preset_name = file_name_of(&job.toml);
    let output_bytes = std::fs::metadata(&done.output_path).map(|m| m.len()).ok();
    let facts = crate::archive::ReportFacts {
        preset_params: preset_params.as_ref(),
        preset_missing: &preset_missing,
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
        calibration_receipt: done.calibration_receipt.as_ref(),
    };
    let pipeline: Vec<PipelineStep> = done
        .step_timings
        .iter()
        .map(|t| {
            PipelineStep::from_timing(
                t,
                PipelineStatus::Complete,
                None,
                facts.step_detail_lines(t.step),
            )
        })
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
        // 证据链：**成功那一趟也写**（原来这些值在 `resolve_profile` 之后就被消费掉了，
        // 成功时一个字都不留 —— "为什么这次用了加长版的基准"永远无法解释）。
        calibration: calibration_facts(identity, done),
        /* 参数快照：预设 TOML 那一刻的值 + IR 终态（见 `archive::ParamSnapshot`） */
        snapshot: facts.snapshot(),
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
        /* 拉起这一趟的那个切片器进程（父进程）—— 引导层靠它认"我刚才切的那一份" */
        slicer_pid: crate::parent::pid(),
        invocation: crate::archive::invocation(),
        warnings: done.warnings.clone(),
        error: None,
        error_key: None,
        error_args: None,
        cancelled: false,
        pipeline,
        /* 成功那一趟没有碰撞事实（撞了就走不到这里）—— 这一格恒 `null` */
        collision: None,
        /* 挪过塔就记一笔（成功那趟也可能挪过 —— 那正是"挪完复检通过"） */
        relocation: relocation.cloned(),
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
    // 已经跑到的那几步（`StepRecorder` 记的账）。管线还没进（预设错 / 停在问句）就给空。
    pipeline: Vec<PipelineStep>,
    // 碰撞那一趟的事实（`HookError::collision()`）—— 不是碰撞就是 `None`。
    collision: Option<postprocess::postproc::disk::collision::CollisionFacts>,
    // 挪塔那一趟的记录（2026-10-18）—— 没走到那一问就是 `None`。
    relocation: Option<&crate::archive::RelocationRecord>,
    // 校准件身份探测结果。**失败也要留**（作者："不能只在成功时产生"）——
    // 身份冲突就把两边的原值留在记录里，那正是复盘要看的东西。
    identity: &postprocess::gcode::IdentityProbe,
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
        /* 失败 / 取消也记：这一趟同样是**那个**切片器拉起来的（见成功那一处） */
        slicer_pid: crate::parent::pid(),
        invocation: crate::archive::invocation(),
        warnings: Vec::new(),
        error: Some(error.to_string()),
        /* 认得出是哪一条就把键也记下 —— 归档里存事实，词由读的那一刻决定（2026-10-27）。
        带占位符的句子（值已经填进去了）认不出来就留 `None`，读的时候原样摆，不猜。 */
        error_key: postprocess::diag::messages::key_of_exact_text(error).map(str::to_owned),
        error_args: None,
        cancelled,
        pipeline,
        collision,
        relocation: relocation.cloned(),
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
            calibration: calibration_facts_failed(identity),
            /* 失败 / 取消没有 IR 终态 ⇒ 快照整组不摆（不编） */
            snapshot: None,
        },
    };
    if let Err(e) = crate::archive::write_meta(paths, &meta) {
        tracing::warn!(error = %e, "执行记录写入失败（失败/取消那一条）");
    }
}

/// 3MF 里那条身份 → `Mode`。**全仓唯一把身份字变成 `Mode` 的地方**（钩子这一侧）。
///
/// 解析器只有 `calibration::parse_mode_id` 一个；`gcode::model_identity` 那层是叶子模块
/// （不许引用本 crate 其他模块，见 `tests/boundaries.rs`），所以它只交出原值，由这里解析。
/// 认不出（没写、写错、写成旧口径 `XY`）一律 `Mode::None` —— **不猜**。
fn mode_of(identity: &postprocess::gcode::IdentityProbe) -> calibration::Mode {
    identity
        .identity_3mf
        .as_deref()
        .and_then(calibration::parse_mode_id)
        .unwrap_or(calibration::Mode::None)
}

/// 校准件验收那一组（`detail.calibration`）—— **只搬运，不判定**。
///
/// 两边素材都在手上：`identity` 是机型核对之前那一次探测（3MF 身份 + G-code 标记 + 判定），
/// `done.calibration_receipt` 是第 7 步 dry-run 认档的结果（实测尺寸 / 命中档 / 容差）。
///
/// 普通模型那一趟：探测是 `Absent`、回执是 `None` ⇒ 整组**不写**（返回 `None`），
/// 报告页自然不摆这一块。旧记录也一样（读到 `null`）。
fn calibration_facts(
    identity: &postprocess::gcode::IdentityProbe,
    done: &postprocess::pipeline::ProcessResult,
) -> Option<crate::archive::CalibrationFacts> {
    let receipt = done.calibration_receipt.as_ref();
    // 既没身份、又没回执 ⇒ 这就是一盘普通模型，别给报告页添一块空表。
    if identity.verdict == postprocess::gcode::IdentityVerdict::Absent && receipt.is_none() {
        return None;
    }
    /* 「最终用的身份」取**内核实际跑的那个**（`detail_facts.calibration_mode`），
    不是 3MF 里那个原值 —— 两者理应相同（钩子把 `calib_mode` 传进管线），
    但记录要回答的是"实际执行了什么"，所以记内核那一份。 */
    let ran = done.detail_facts.calibration_mode.as_str();
    Some(build_calibration_facts(
        identity,
        receipt,
        (!ran.is_empty()).then_some(ran),
    ))
}

/// 失败 / 取消那一趟：尺寸与档位拿不到（管线可能没跑到第 7 步），
/// 但**身份那一半照写** —— 冲突、标记缺失、原始模型被移走，恰恰都是失败时最该看的。
fn calibration_facts_failed(
    identity: &postprocess::gcode::IdentityProbe,
) -> Option<crate::archive::CalibrationFacts> {
    if identity.verdict == postprocess::gcode::IdentityVerdict::Absent {
        return None;
    }
    Some(build_calibration_facts(identity, None, None))
}

fn build_calibration_facts(
    identity: &postprocess::gcode::IdentityProbe,
    receipt: Option<&postprocess::postproc::calibration::CalibrationReceipt>,
    mode: Option<&str>,
) -> crate::archive::CalibrationFacts {
    let mode = mode.filter(|m| !m.is_empty()).map(str::to_string);
    crate::archive::CalibrationFacts {
        origin: identity.origin.clone(),
        identity_3mf: identity.identity_3mf.clone(),
        verdict: identity.verdict.as_str().to_string(),
        note: identity.note.clone(),
        measured: receipt
            .and_then(|r| r.measured)
            .map(|b| [b.x_min, b.x_max, b.y_min, b.y_max]),
        profile_id: receipt.and_then(|r| r.profile_id.clone()),
        tolerance: receipt.and_then(|r| r.tolerance),
        footprint: receipt.and_then(|r| r.footprint).map(|(w, h)| [w, h]),
        mode,
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
            /* 这一档以前不带码，日志里是 `code=""`。现在每一句都带码（作者 2026-10-17：
            「就算信息不全，也能通过错误码找到对应的」）——
            `hook.line.cancelled` 自己就声明了 `W_LINE_CANCELLED_001`。 */
            let code = messages::code_of(keys::LINE_CANCELLED).unwrap_or_default();
            let p = params(&reason.slicer(), &code);
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

/// 界面回的那个**选项 id** → 写进 IR 的**管线常量**。
///
/// ★★★ 这两套字**不一样**，别把它们当同一个（2026-10-17 真机事故）：
///
/// | 谁 | `print` 那一支 | `direct` 那一支 |
/// | --- | --- | --- |
/// | 选项 id（`hook_ipc::CHOICE_CALIB_*`，界面回给钩子的） | `"print"` | `"direct"` |
/// | 管线常量（`postprocess::ir::CALIBRATION_EXEC_*`，写进 IR 的） | `"print_then_calibrate"` | `"direct_calibrate"` |
///
/// 拿 `"direct"` 去跟 `"direct_calibrate"` 比永远是 false —— 那样写出来的结果是**两颗按钮
/// 都按"先打印，再校准"跑**（作者选的"直接校准"，切出来还是有打印件）。
///
/// 认不出的 id 一律按 `print`：那是"什么都不改"的那一支，也是预设原本就写着的那种。
pub fn calib_exec_mode_of(choice_id: &str) -> &'static str {
    if choice_id == crate::hook_ipc::CHOICE_CALIB_DIRECT {
        postprocess::ir::CALIBRATION_EXEC_DIRECT_CALIBRATE
    } else {
        postprocess::ir::CALIBRATION_EXEC_PRINT_THEN_CALIBRATE
    }
}

/// 选项 id → 结论里那个 `calib_choice`（`"print"` / `"direct"`，界面与报告按它分文案）。
pub fn calib_choice_of(choice_id: &str) -> &'static str {
    if choice_id == crate::hook_ipc::CHOICE_CALIB_DIRECT {
        "direct"
    } else {
        "print"
    }
}

/// 校准件那一趟的**那一句结论**（两种跑法都是成功，只有这句不同）。
///
/// `None` = 不是校准件这一趟 ⇒ 调用方走 [`success_line`]。键住 `messages.toml`
/// （`hook.calib_done_print` / `hook.calib_done_direct`）—— 界面与报告读的是同一组，
/// **不许在 JSX 里另写一份**。
pub fn calib_done_line(choice: Option<&str>) -> Option<String> {
    let key = match choice? {
        "print" => keys::CALIB_DONE_PRINT,
        "direct" => keys::CALIB_DONE_DIRECT,
        _ => return None,
    };
    Some(messages::text(key, &[]))
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
    /* ★ 开场那一条 `Hello` **不在这里发**（2026-10-14）：它现在**连事实一起**走一条，
    而事实要等输入读进来、预设载完才算得出来 —— 发点搬进了 [`run`]，见那儿
    "开场那一条"那段注释。 */

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

    /*
     * 归档之后顺手清一遍旧记录（2026-10-13「自动清理」那一刀）：开关开着才动，
     * **全程静默** —— 不进结论、不改退出码、失败只写日志（见 [`crate::retention`] 模块头）。
     *
     * 摆在**结论已经送出去之后**：界面先拿到结果，清理这几十毫秒不该让用户多等。
     * 也不指望界面进程来做这件事 —— 切片器拉起的这一趟**就是**"跑完了"那一刻，
     * 而界面可能开在别的页面、甚至根本没开。
     */
    crate::retention::prune_if_enabled_headless();

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

/// **管线步骤记账**（2026-10-18）：包住真正的进度出口，每一步"开始了"就记一笔，
/// 下一步开始（或这一趟收场）时结算耗时 —— 于是**失败 / 取消的那一趟也有
/// 「管线计时」**：跑到哪一步、前面几步各花了多久（作者：「失败的管线也要写出来，
/// 第一步干嘛了，比如检测机型之类的」）。
///
/// 内核成功时自己带 `step_timings`（`ProcessResult`），记账只服务失败路径；
/// 成功路径的逐字账仍以内核那份为准（含 `detail` 多行事实），这里记的不用。
struct StepRecorder<'a> {
    inner: &'a mut dyn ProgressSink,
    /// 正在跑的那一步：步骤 / 入口那句话 / 开始时刻。`None` = 还没进任何步。
    current: Option<(postprocess::pipeline::Step, String, std::time::Instant)>,
    /// 已经跑完的步骤（按执行顺序）。
    done: Vec<PipelineStep>,
}

impl StepRecorder<'_> {
    fn settle(&mut self, status: PipelineStatus) {
        if let Some((step, message, started_at)) = self.current.take() {
            self.done.push(PipelineStep {
                id: step.id(),
                message,
                elapsed_ms: started_at.elapsed().as_millis() as u64,
                status,
                /* 失败路径没有 `DetailFacts`（那要等整链成功），小字整组不写
                —— 页面口径「没给就不摆」 */
                detail: None,
                detail_lines: None,
            });
        }
    }

    /// 收场：最后停在的那一步按 `status` 结算（前面的都是跑完的）。
    fn finish(mut self, last_status: PipelineStatus) -> Vec<PipelineStep> {
        self.settle(last_status);
        self.done
    }
}

impl ProgressSink for StepRecorder<'_> {
    fn emit(&mut self, event: ProgressEvent) {
        /* 新一步的入口事件 = `fraction_in_step: None`（步内进度是 `Some`，且步号不变）。
        内核的 `StepTimer` 就是这个语义，这里照抄同一份判据 —— 不认它就会把
        pass1 的 0% / 2% / … 进度帧错当成一步一个。 */
        if event.fraction_in_step.is_none()
            && self
                .current
                .as_ref()
                .map_or(true, |(step, _, _)| *step != event.step)
        {
            self.settle(PipelineStatus::Complete);
            self.current = Some((event.step, event.message.clone(), std::time::Instant::now()));
        }
        self.inner.emit(event);
    }
}

/// 通道那一侧的问句：推给界面，等它答（见 [`crate::hook_ipc::Client::ask`]）。
struct ChannelAsker<'a> {
    client: &'a crate::hook_ipc::Client,
}
impl Asker for ChannelAsker<'_> {
    fn confirm(&mut self, question: &str) -> bool {
        /* 机型那一问：不摆选项表（界面用默认那两颗），超时 = 停下（`stop`） */
        matches!(
            self.client.ask(
                crate::hook_ipc::QKIND_MACHINE_MISMATCH,
                question,
                Vec::new(),
                crate::hook_ipc::CHOICE_STOP
            ),
            Some(id) if id == crate::hook_ipc::CHOICE_KEEP
        )
    }

    /// 校准件那一问：两个选项摆给界面（`print` 主、`direct` 次），超时 = `stop`。
    fn ask_calibration(&mut self, question: &str) -> Option<String> {
        use crate::hook_ipc::{
            QuestionChoice, CHOICE_CALIB_DIRECT, CHOICE_CALIB_PRINT, CHOICE_STOP,
        };
        let choices = vec![
            QuestionChoice {
                id: CHOICE_CALIB_PRINT.to_owned(),
                label: Reason::new(keys::CHOICE_CALIB_PRINT).text(),
                hint: Some(Reason::new(keys::CHOICE_CALIB_PRINT_HINT).text()),
                accent: true,
            },
            QuestionChoice {
                id: CHOICE_CALIB_DIRECT.to_owned(),
                label: Reason::new(keys::CHOICE_CALIB_DIRECT).text(),
                hint: Some(Reason::new(keys::CHOICE_CALIB_DIRECT_HINT).text()),
                accent: false,
            },
        ];
        let answer = self.client.ask(
            crate::hook_ipc::QKIND_CALIBRATION_MODE,
            question,
            choices,
            CHOICE_STOP,
        )?;
        /* 超时回的是 `stop`（不是选项之一）⇒ 当成"没答" */
        (answer != CHOICE_STOP).then_some(answer)
    }
}

/// **第 6 步那一问的钩子侧实现**（2026-10-18 方案 B）。
///
/// 内核在 `Step::Collision` 撞了会调 [`TowerAsk::ask`]；这一份负责把问句推给界面、
/// 等答复、并把**这一趟的选择记成 `RelocationRecord`**（内核不认识归档类型）。
///
/// 为什么记账在这边而不是内核：`relocation.before` 是"调整前的位置与事实"，
/// 它属于"用户做了什么选择"这件事 —— 那是交互侧的知识，内核手上只有几何。
struct ChannelTowerAsk<'a> {
    client: &'a crate::hook_ipc::Client,
    /// 取消线：按了停止就别再把问句推上去；等待期间它是唯一的打断手段。
    cancel: &'a CancelToken,
    /// 这一趟的选择与几何 —— `run` 在管线返回后 `take_record()` 走归档。
    record: Option<crate::archive::RelocationRecord>,
}

impl ChannelTowerAsk<'_> {
    /// 把这一趟的记录取走（归档只认一份）
    fn take_record(&mut self) -> Option<crate::archive::RelocationRecord> {
        self.record.take()
    }
}

impl TowerAsk for ChannelTowerAsk<'_> {
    fn ask(
        &mut self,
        facts: &CollisionFacts,
        candidate: Option<[f64; 2]>,
        current: [f64; 2],
        tower_height_mm: f64,
    ) -> TowerAskOutcome {
        use crate::hook_ipc::{
            QuestionChoice, CHOICE_MODEL, CHOICE_RELOCATE, CHOICE_STOP, QKIND_TOWER_RELOCATION,
        };
        /* 摆得下才给"自动挪塔"这颗按钮；摆不下 ⇒ 只给"自己去挪模型" */
        let mut choices = Vec::new();
        if candidate.is_some() {
            choices.push(QuestionChoice {
                id: CHOICE_RELOCATE.to_owned(),
                label: Reason::new(keys::CHOICE_TOWER_RELOCATE).text(),
                hint: Some(Reason::new(keys::CHOICE_TOWER_RELOCATE_HINT).text()),
                accent: true,
            });
        }
        choices.push(QuestionChoice {
            id: CHOICE_MODEL.to_owned(),
            label: Reason::new(keys::CHOICE_TOWER_MODEL).text(),
            hint: Some(Reason::new(keys::CHOICE_TOWER_MODEL_HINT).text()),
            accent: false,
        });
        let geo = crate::hook_ipc::RelocateGeoPayload {
            collision: Some(facts.clone()),
            candidate,
            current,
        };
        /* 问句只说"要不要挪" —— 冲突点数由界面上那一行单独说 */
        let question = Reason::new(keys::TOWER_RELOCATION_QUESTION).text();
        let answer = self.client.ask_with_geo(
            QKIND_TOWER_RELOCATION,
            &question,
            choices,
            CHOICE_STOP,
            Some(geo),
        );
        /*
         * ★ **取消与超时必须分开**（作者 2026-10-18）：`ask_with_geo` 两种情形都返回
         * `None`，但按了停止是"这一趟被用户掐掉"，超时是"没人答"——后者照样按碰撞收场，
         * 前者要变成 `Cancelled`（归档不得记成成功）。
         */
        if self.cancel.is_cancelled() {
            return TowerAskOutcome::Cancelled;
        }

        let unavailable = candidate.is_none();
        let relocated = answer.as_deref() == Some(CHOICE_RELOCATE) && candidate.is_some();
        let choice = if unavailable {
            "unavailable"
        } else if relocated {
            "relocate"
        } else if answer.is_some() {
            "model"
        } else {
            "timeout"
        };
        let to = if relocated { candidate } else { None };
        self.record = Some(crate::archive::RelocationRecord {
            choice: choice.to_owned(),
            from: current,
            to,
            moved_mm: to
                .map(|p| ((p[0] - current[0]).powi(2) + (p[1] - current[1]).powi(2)).sqrt()),
            before_conflict_count: facts.conflict_count,
            after_conflict_count: None,
            verified: None,
            tower_height_mm,
            before: Some(facts.clone()),
        });
        match to {
            Some(pos) => TowerAskOutcome::Moved(pos),
            None => TowerAskOutcome::Declined,
        }
    }

    /// 复检回告：内核拿改完之后的 IR 又跑了一遍检测。判据在内核，这里只记账。
    fn verified(&mut self, after: Option<&CollisionFacts>) {
        if let Some(record) = self.record.as_mut() {
            record.verified = Some(after.is_none());
            record.after_conflict_count = Some(after.map_or(0, |f| f.conflict_count));
        }
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
                /* 兜底不给空串：见 `failure_lines` 那一段的同一条理由 ——
                日志里 `code=""` 等于这一行查不到 */
                code = reason
                    .code()
                    .unwrap_or_else(|| "W_UNCLASSIFIED_001".to_owned()),
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
            /*
             * 成功那一句：**挪过塔就说挪塔**（2026-10-18 作者：「怎么移动，多少距离」）——
             * 用户读到的结论必须提到"塔被挪了"，否则他只会记得自己点过一颗按钮。
             * 挪完仍撞那一趟走的是错误出口（另一个码），到不了这里。
             */
            message: match out.relocation.as_ref().filter(|r| r.verified == Some(true)) {
                Some(r) => messages::text(
                    keys::TOWER_RELOCATED_OK,
                    &[
                        ("fromX", &format!("{:.1}", r.from[0])),
                        ("fromY", &format!("{:.1}", r.from[1])),
                        (
                            "toX",
                            &format!("{:.1}", r.to.map(|t| t[0]).unwrap_or(r.from[0])),
                        ),
                        (
                            "toY",
                            &format!("{:.1}", r.to.map(|t| t[1]).unwrap_or(r.from[1])),
                        ),
                        ("moved", &format!("{:.1}", r.moved_mm.unwrap_or(0.0))),
                    ],
                ),
                None => success_line(&out.output, out.elapsed_ms),
            },
            code: None,
            stage,
            exit_code: EXIT_OK,
            elapsed_ms: out.elapsed_ms as u64,
            output: Some(out.output.display().to_string()),
            warnings: out.warnings.clone(),
            /* 结果读数：成功才有（失败 / 取消时内核没把这几个数算出来 —— 空着） */
            result: Some(out.result.clone()),
            /* 校准件那一趟选了哪种跑法 —— 界面与报告据此分开说"两种成功" */
            calib_choice: out.calib_choice.clone(),
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
                result: None,
                calib_choice: None,
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
            /* ★ 码**不许是空串**（作者 2026-10-17：「就算信息不全，也能通过错误码找到
            对应的」）：日志里 `code=""` 等于这一行在码表里查不到。
            兜底顺序：这条 reason 自己的码 → 结论框子那个码（`hook.line.*` 各有一个）
            → 再不行才写 `W_UNCLASSIFIED_001`（判据 `every_hook_line_carries_a_code…`
            会先咬住"哪条 hook.* 没码"，所以这个兜底正常永远用不到）。 */
            let code = err
                .reason()
                .code()
                .or_else(|| {
                    let key = if err.cancelled() {
                        keys::LINE_CANCELLED
                    } else {
                        keys::LINE_FAILED
                    };
                    messages::code_of(key)
                })
                .unwrap_or_else(|| "W_UNCLASSIFIED_001".to_owned());
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
    use postprocess::pipeline::{ProgressEvent, Step};

    /// **存档里那一格必须说真话**（2026-10-19）。
    ///
    /// `detail.calibration` 是用户唯一能查的凭据：`mode` 写的是"实际跑了什么"、
    /// `profileId` 写的是"用了哪一档"。这条咬住三件事：
    /// 1. 内核给了 `ZOffset` ⇒ 存档 `mode` 就是 `ZOffset`（不是 3MF 原值，也不是空）；
    /// 2. 回执里的档位 / 容差 / 实测尺寸原样带进存档；
    /// 3. 给了空串（普通模型那一趟）⇒ `mode` 是 `null`，不是 `""`。
    ///
    /// 第 3 条防的是"把空串当身份写进记录" —— 那是另一种假数据。
    #[test]
    fn the_archive_records_the_mode_that_actually_ran() {
        use postprocess::postproc::calibration::CalibrationReceipt;
        use postprocess::postproc::disk::BBox;

        let identity = postprocess::gcode::IdentityProbe {
            identity_3mf: Some("ZOffset".to_string()),
            verdict: postprocess::gcode::IdentityVerdict::From3mf,
            ..Default::default()
        };
        let receipt = CalibrationReceipt {
            measured: Some(BBox {
                x_min: 67.921,
                x_max: 187.4321,
                y_min: 121.408,
                y_max: 137.471,
            }),
            profile_id: Some("zoffset-ext".to_string()),
            tolerance: Some(0.5),
            footprint: Some((119.58, 16.063)),
        };

        let facts = build_calibration_facts(&identity, Some(&receipt), Some("ZOffset"));
        assert_eq!(facts.mode.as_deref(), Some("ZOffset"), "实际跑的就是它");
        assert_eq!(facts.identity_3mf.as_deref(), Some("ZOffset"));
        assert_eq!(facts.profile_id.as_deref(), Some("zoffset-ext"));
        assert_eq!(facts.tolerance, Some(0.5));
        assert_eq!(facts.footprint, Some([119.58, 16.063]));
        assert_eq!(facts.measured, Some([67.921, 187.4321, 121.408, 137.471]));
        assert_eq!(facts.verdict, "from_3mf");

        /* 普通模型那一趟：内核给的是 `""` ⇒ 存档 `mode` 必须是 null，不能是空串 */
        let plain = build_calibration_facts(&identity, None, Some(""));
        assert_eq!(plain.mode, None, "空串不是身份，别原样写进记录");
        assert!(plain.profile_id.is_none());
    }

    /// **身份只从 3MF 来**（2026-10-18 用户定的硬口径）：
    /// 3MF 有身份 ⇒ 就是那个身份；没有 ⇒ `Mode::None`，**正文里写什么都不算**。
    ///
    /// 这条同时钉住"钩子交给管线的身份"这个出口（`identity_probe.mode()`），
    /// 是 `IrProcessRequest::calibration` 的唯一来源。
    #[test]
    fn identity_comes_only_from_the_3mf() {
        let with = |m: Option<&str>| postprocess::gcode::IdentityProbe {
            identity_3mf: m.map(str::to_string),
            verdict: if m.is_some() {
                postprocess::gcode::IdentityVerdict::From3mf
            } else {
                postprocess::gcode::IdentityVerdict::Absent
            },
            ..Default::default()
        };

        /* 3MF 说 ZOffset ⇒ ZOffset */
        assert_eq!(
            calibration::mode_to_calibration_string(mode_of(&with(Some("ZOffset")))),
            "ZOffset"
        );
        /* 4 个身份都认 */
        for (raw, want) in [
            ("ZOffset", "ZOffset"),
            ("Precise", "Precise"),
            ("Rough", "Rough"),
            ("Repetition", "Repetition"),
        ] {
            assert_eq!(
                calibration::mode_to_calibration_string(mode_of(&with(Some(raw)))),
                want
            );
        }

        /* 没有身份 ⇒ `None`；值乱写也 ⇒ `None`（不猜、不按尺寸凑） */
        assert!(!calibration::is_any(mode_of(&with(None))));
        assert!(!calibration::is_any(mode_of(&with(Some("随便写的")))));
        /* 旧口径 `XY` 不认 —— 身份层面从来没有这个模式 */
        assert!(!calibration::is_any(mode_of(&with(Some("XY")))));
    }

    /// **失败也有管线账**（2026-10-18，作者：「失败的管线也要写出来，第一步干嘛了」）：
    /// 记账器把"进了哪些步、各花多久、停在哪一步"记全 —— 前面的步是跑完的，
    /// 最后停在的那一步带 `error` / `cancelled` 状态。
    #[test]
    fn the_step_recorder_setstles_started_steps_and_marks_the_last_one() {
        struct Sink;
        impl ProgressSink for Sink {
            fn emit(&mut self, _: ProgressEvent) {}
        }
        let mut inner = Sink;
        let mut rec = StepRecorder {
            inner: &mut inner,
            current: None,
            done: Vec::new(),
        };

        /* 三步：input（完成）→ machine-detect（完成）→ parse（在这步失败）。
        步内进度帧（fraction Some）不许被当成新一步 —— pass1 的 0%/2%/… 帧就长这样 */
        rec.emit(ProgressEvent {
            step: Step::Input,
            fraction_in_step: None,
            message: "正在读取文件...".into(),
        });
        rec.emit(ProgressEvent {
            step: Step::MachineDetect,
            fraction_in_step: None,
            message: "识别机型...".into(),
        });
        rec.emit(ProgressEvent {
            step: Step::Parse,
            fraction_in_step: None,
            message: "解析 G-code...".into(),
        });
        rec.emit(ProgressEvent {
            step: Step::Parse,
            fraction_in_step: Some(0.4),
            message: "解析 G-code... 40%".into(),
        });

        let steps = rec.finish(PipelineStatus::Error);
        assert_eq!(steps.len(), 3, "三步都该有账：{steps:?}");
        assert_eq!(steps[0].id, "input");
        assert_eq!(steps[0].status, PipelineStatus::Complete);
        assert_eq!(steps[1].id, "machine-detect");
        assert_eq!(steps[1].status, PipelineStatus::Complete);
        assert_eq!(steps[2].id, "parse");
        assert_eq!(
            steps[2].status,
            PipelineStatus::Error,
            "最后停在的那一步要标记"
        );
        for s in &steps {
            assert!(s.elapsed_ms < 5_000, "测试里不该跑出真耗时：{s:?}");
            assert_eq!(s.detail, None, "失败路径没有 DetailFacts，小字整组不写");
        }
        assert_eq!(steps[2].message, "解析 G-code...");
    }

    /// 取消那一条：最后一步的状态是 `cancelled`（作者口径：停下不是故障）。
    #[test]
    fn a_cancelled_run_marks_the_last_step_cancelled() {
        struct Sink;
        impl ProgressSink for Sink {
            fn emit(&mut self, _: ProgressEvent) {}
        }
        let mut inner = Sink;
        let mut rec = StepRecorder {
            inner: &mut inner,
            current: None,
            done: Vec::new(),
        };
        rec.emit(ProgressEvent {
            step: Step::Input,
            fraction_in_step: None,
            message: "正在读取文件...".into(),
        });
        let steps = rec.finish(PipelineStatus::Cancelled);
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].status, PipelineStatus::Cancelled);
    }
    use crate::args::HookJob;

    /// ★★★ 校准那一问：**界面回的是选项 id**（`print` / `direct`），写进 IR 的却是管线常量
    /// （`print_then_calibrate` / `direct_calibrate`）—— 两套字不一样。
    ///
    /// 2026-10-17 真机的表现：选了「直接校准」，切出来还有打印件。原因就是把 `"direct"`
    /// 直接跟 `"direct_calibrate"` 比，两边永远不等 ⇒ 掉进兜底臂，按"先打印"跑了。
    /// 这条判据钉住**映射本身**（纯函数，不需要真跑一趟管线）。
    #[test]
    fn a_direct_choice_really_writes_direct_into_the_ir() {
        /* 界面回的那颗「直接校准」 */
        assert_eq!(
            calib_exec_mode_of(crate::hook_ipc::CHOICE_CALIB_DIRECT),
            postprocess::ir::CALIBRATION_EXEC_DIRECT_CALIBRATE,
            "选「直接校准」必须把 direct_calibrate 写进 IR —— 否则内核照旧保留打印件"
        );
        assert_eq!(
            calib_choice_of(crate::hook_ipc::CHOICE_CALIB_DIRECT),
            "direct"
        );

        /* 另一颗「先打印，再校准」 */
        assert_eq!(
            calib_exec_mode_of(crate::hook_ipc::CHOICE_CALIB_PRINT),
            postprocess::ir::CALIBRATION_EXEC_PRINT_THEN_CALIBRATE
        );
        assert_eq!(
            calib_choice_of(crate::hook_ipc::CHOICE_CALIB_PRINT),
            "print"
        );

        /* ★ 两种 id 的**字面量本来就不一样** —— 这条断言是把"为什么会有那个 bug"钉在代码里，
        谁要是哪天想拿 id 直接跟常量比，读到这里就停下了 */
        assert_ne!(
            crate::hook_ipc::CHOICE_CALIB_DIRECT,
            postprocess::ir::CALIBRATION_EXEC_DIRECT_CALIBRATE,
            "选项 id 与管线常量是两套字，不许直接相等比较"
        );

        /* 认不出的 id（老界面 / 手滑）按"什么都不改"那一支，绝不当成 direct */
        assert_eq!(
            calib_exec_mode_of("keep"),
            postprocess::ir::CALIBRATION_EXEC_PRINT_THEN_CALIBRATE
        );
        assert_eq!(calib_choice_of("keep"), "print");
    }

    fn job(toml: &std::path::Path, gcode: &std::path::Path) -> HookJob {
        HookJob {
            toml: to_path_buf(toml),
            gcode: to_path_buf(gcode),
        }
    }

    fn to_path_buf(p: &std::path::Path) -> PathBuf {
        p.to_path_buf()
    }

    /// 开场那条事实：机型 / 耗材 / 大小取 IR，**「这份文件自报的」那几格优先于 IR**，
    /// 一格都不许错位（两个机型最容易反：`preset` 是维度与边界按它算的那一份，
    /// `gcode` 是切片器自报的）。
    #[test]
    fn the_opening_facts_prefer_this_file_over_the_ir() {
        let mut ir = postprocess::ir::Ir::default();
        ir.machine.machine_type = "A1_MINI".to_owned();
        ir.machine.gcode_machine_type = "A1".to_owned();
        ir.filament.filament_type = "PETG".to_owned();
        ir.machine.nozzle_diameter = 0.4;
        ir.machine.first_layer_height = 0.25;
        ir.machine.typical_layer_height = 0.2;
        ir.toolhead.max_speed = 4_200.0; // 70 mm/s × 60（内核这一格是 mm/min）
        ir.filament.slicer = "BambuStudio".to_owned();

        /* 输入头（真 G-code 的样子）：版本串 + 自报的预估总时长 + 喷嘴 / 层高 */
        let raw = "; BambuStudio 02.06.01.55\n\
                   ; model printing time: 3h 16m 52s; total estimated time: 3h 22m 55s\n\
                   ; nozzle_diameter = 0.6\n\
                   ; layer_height = 0.12\n\
                   G1 X1\n";
        let f = facts_from(&ir, raw, 3_355_443);
        assert_eq!(f.preset_machine, "A1_MINI");
        assert_eq!(f.gcode_machine, "A1");
        assert_eq!(f.filament_type, "PETG");
        assert_eq!(f.file_size_bytes, 3_355_443);
        /* ★ 喷嘴 / 层高 / 切片软件：**这份文件自报的优先**（2026-10-14）——
        头里的 0.6 / 0.12 / 完整版本串盖过 IR 里的 0.4 / 0.2 / 名字。
        那张表**开场就要摆**（等 pass1 来不及：真机上一直是 `—`）。
        层高认的是**常规**那一档（`; layer_height`），不是首层那条 */
        assert_eq!(f.nozzle_diameter, 0.6, "头行自报的喷嘴优先于 IR");
        assert_eq!(f.layer_height, 0.12, "头行自报的层高优先于 IR");
        /* 「切片与预估」那一栏的三格（跑着的时候就摆）：
        涂胶速度 = mm/min ÷ 60（与归档 `glueSpeed` 同一处换算）；
        切片软件 = 头里的完整串；预估打印时间 = **切片器自报的**那一行 */
        assert_eq!(
            f.glue_speed,
            Some(70.0),
            "换算必须与归档同一处：{:?}",
            f.glue_speed
        );
        assert_eq!(f.slicer.as_deref(), Some("BambuStudio 02.06.01.55"));
        assert_eq!(
            f.slicer_print_time_seconds,
            Some(12_175.0),
            "3h 22m 55s 该被读成 12175 秒"
        );
        /* 头里那几行缺席（别的切片器 / 被裁过）⇒ 退回 IR；预估时长那格不摆（`None`，不是 0） */
        let bare = facts_from(&ir, "G1 X1\n", 1);
        assert_eq!(bare.nozzle_diameter, 0.4, "头行缺席 ⇒ 退回 IR");
        assert_eq!(bare.layer_height, 0.2);
        assert_eq!(bare.slicer.as_deref(), Some("BambuStudio"));
        assert_eq!(bare.slicer_print_time_seconds, None);
    }

    /// 仓库里那份夹具预设（与内置产物同构）—— 机型 A1 / 变体 fast
    fn fixture_preset(dest: &std::path::Path) {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../crates/preset/assets/presets/A1-fast.toml");
        std::fs::copy(&src, dest).expect("拷一份夹具预设");
    }

    /// **测试不许往真数据根写执行记录**（2026-10-18：报告页冒出 tmp 路径的假"失败"记录，
    /// `调用` 那格是测试二进制 —— 就是这批测试写的）。
    ///
    /// 这一层进程内的 `run` 会归档（失败 / 取消**也留记录**，不许瞒 —— 见 `archive`
    /// 模块头），归档根 = `internal_root_headless()`。于是每个要跑管线的测试先过这里，
    /// 把数据根整个盖到本进程私有的临时目录（`MKP_SUPPORT_EASE_DATA`，探针同一把钥匙）。
    /// 进程级只设一次（测试多线程，`Once` 保不竞态）；临时目录故意 `forget` ——
    /// drop 会在别的测试还在用时删根，进程退出时由 OS 收拾。
    fn isolate_data_root() {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            let dir = tempfile::tempdir().expect("测试数据根临时目录");
            std::env::set_var(crate::fsx::paths::DATA_ROOT_ENV, dir.path());
            std::mem::forget(dir);
        });
    }

    fn run_headless(toml: &std::path::Path, gcode: &std::path::Path) -> Result<Outcome, HookError> {
        isolate_data_root();
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
        isolate_data_root();
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
        isolate_data_root();
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
        /* 结果读数那一包必须搭车到 `Outcome` 上（界面「结果 · 成功」的指标条 +
        擦料与胶水消耗那几格）—— 值对不对归内核那几条判据管，这里钉的是
        **它们真的从 `ProcessResult` 搬到了这一层**（搬丢一格界面就少一格） */
        assert!(
            out.result.glue_speed > 0.0,
            "涂胶速度该从预设里来：{:?}",
            out.result
        );
        assert!(
            out.result.glue_length_mm > 0.0,
            "涂胶总长该从 pass1 统计里来：{:?}",
            out.result
        );
        assert!(
            !out.result.slicer.is_empty(),
            "切片软件那串该从输入头行刮到：{:?}",
            out.result
        );
        assert!(
            out.result.print_time_seconds.is_some(),
            "这一条没延后（headless），打印时间该在车上：{:?}",
            out.result
        );
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
        校准形变那串机器判据留给界面/记录/日志，切片器只看"别转本地模型" */
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
        /* 「停下」这一档统一说"已停止"（`hook.line.stopped`），**不是**"已取消"——
        后者是"失败但没有码"那条理论分支用的框子。 */
        assert!(line.starts_with("已停止："), "{line}");
        /* 2026-10-17 改口径：用户按的停**也给码**，方便照着码查（作者要的就是这个）。
        它不许是 `E_`（那不是故障），而是 `W_` 打头的"停下"类。 */
        assert!(
            line.contains("［W_SYS_CANCELLED_001］"),
            "用户按的停要给 W_ 码：{line}"
        );
        assert!(
            !line.contains("［E_"),
            "用户按的停不是故障，不该出现 E_ 码：{line}"
        );
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
        isolate_data_root();
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

    /// 造一条挪塔记录（只给这三句的判据用）
    fn relocation_record(choice: &str, verified: Option<bool>) -> crate::archive::RelocationRecord {
        crate::archive::RelocationRecord {
            choice: choice.to_owned(),
            from: [78.0, 91.0],
            to: verified.map(|_| [16.4, 90.0]),
            moved_mm: verified.map(|_| 61.6),
            before_conflict_count: 97,
            after_conflict_count: verified.map(|v| if v { 12 } else { 0 }),
            verified,
            tower_height_mm: 245.6,
            before: None,
        }
    }

    /// **撞塔那三种收场各挑各的句子**（2026-10-18 方案 B）。
    ///
    /// 内核统一回 `Collision`（它不知道用户答了什么），"自己挪 / 板上摆不下 / 挪了还撞"
    /// 这三句只能看这一趟落下的记录 —— 码与改造前逐字一致。
    #[test]
    fn tower_failures_keep_their_three_sentences() {
        let facts = CollisionFacts {
            conflict_count: 97,
            ..CollisionFacts::default()
        };
        assert_eq!(
            tower_failure_error(&relocation_record("model", None), facts.clone())
                .reason()
                .code(),
            Some("W_COLLISION_RELOCATE_002".to_string()),
            "答「自己挪模型」/ 超时那一句"
        );
        assert_eq!(
            tower_failure_error(&relocation_record("unavailable", None), facts.clone())
                .reason()
                .code(),
            Some("W_COLLISION_RELOCATE_003".to_string()),
            "板上摆不下那一句"
        );
        let still = tower_failure_error(&relocation_record("relocate", Some(false)), facts);
        assert_eq!(
            still.reason().code(),
            Some("W_COLLISION_RELOCATE_001".to_string()),
            "挪了还撞那一句"
        );
        assert!(
            still.collision().is_some(),
            "事实要跟着错误一起出门（报告页据此画图）"
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
