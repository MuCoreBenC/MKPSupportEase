//! 12 步编排 —— 只回答「现在做哪一步、成功后去哪一步」，算法一行都不放这里。
//!
//! 12 步：input / config / machine-detect / parse / support / collision /
//! calib-check / pass1 / pass2 / calibration / write / printtime。
//!
//! 来源：`mkp-sr` 的 `crates/engine/src/lib.rs`（498 行）。**只有第 2 步换了**，
//! 其余 11 步逐行照搬（路径前缀 `mkp_postproc::` → `crate::postproc::` 等）。
//!
//! 与来源的差异，逐条登记（不写在提交信息里就会被忘掉，所以钉在代码里）：
//!
//! 1. **第 2 步整段换成 `config::load_with_overrides`**。来源那 6 个动作
//!    （`read_preset` / `validate_config` / `load_param_registry` /
//!    `validate_against_registry` / `ir::build` / 两行 `ir.meta`·`ir.machine` 赋值）
//!    连同它们背后 1,130 行 TOML→Ir 映射一起不再存在 —— 配置文件的结构**就是** `Ir`。
//! 2. **`ProcessRequest.exec_mode` 删除**。来源用 `CalibrationExecMode::from_cli`
//!    把 CLI 的三值串归一后交给 `ir::build`；现在 `Safety.CalibrationExecutionMode`
//!    是配置里的一个普通字段，要改就 `--set Safety.CalibrationExecutionMode=...`。
//!    它在全仓**只有一个消费点**（`calibration.rs:135` 与常量比较），所以少一层归一
//!    不会产生分叉。
//! 3. **`ir.meta.preset_name` 不再被覆写**。来源用 `preset_path.file_name()`
//!    （带扩展名，是实测事实）盖掉它；现在这个字段由用户自己在配置里写，
//!    编排层没有资格改用户写下的值。
//! 4. **机型未命中从「静默继续」改成「报错」**（这是收紧，不是照搬）。
//!    `normalize_to_canonical` 对未知别名返回 `""`（`machine_dims.rs:148-153`），
//!    来源只检查预设里那个串非空 ⇒ 写错机型名会一路走到底、维度全 0、
//!    输出是垃圾但退出码是 0。这里改成 `MissingMachine` 并点名那个写错的值。
//! 5. **`progress_file` 不搬**（跨进程进度状态文件只服务 GUI attach），
//!    `DetailFacts` 精简到 4 个字段（见该结构体的文档，逐个写明搬去哪了）。
//! 6. **多出一张公开脸 `process_with_ir`**（mkp-ssr 加的，`mkpse-pp` 时期没有）。
//!    钩子路径的 IR 来自预设映射（`mkp-preset`）而不是 TOML 配置文件，所以第 2 步
//!    需要一个「IR 已经在手上」的入口。**12 步实现只有一份**（私有 `run`），
//!    两张脸的差别仅在 [`IrSource`] 这一个枚举上 —— 复制一条管线出来就等于
//!    有两条会各自漂移的产品路径。判据：拆之前拆之后 G2 与整链参考仍字节相等。

pub mod hooks;
pub mod progress;

pub use progress::{NoProgress, ProgressEvent, ProgressSink, STEPS, Step};

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::config;
use crate::diag::{CancelToken, PostprocError};
use crate::ir::Ir;

/// write hooks 的偏好默认值（Go preferences/service.go:165-180；CLI 无偏好通道）。
pub const DEFAULT_SKIP_VIBRATION: bool = true;
pub const DEFAULT_BEGIN_STAGE_RETRACT: bool = true;
pub const DEFAULT_BEGIN_STAGE_RETRACT_Z_LIFT_MM: i64 = 20;
/// Z/XY 校准停留秒数（application_defaults [calibration] SSOT：Z=2, XY=3）。
pub const DEFAULT_Z_DWELL_SEC: i64 = 2;
pub const DEFAULT_XY_DWELL_SEC: i64 = 3;

/// mm/min → mm/s 的**逆**换算分母。
///
/// 为什么会有一次逆换算：`Toolhead.MaxSpeed` 在 IR 里是 **mm/min**，而展示面要的
/// 涂胶速度是 **mm/s**。本仓库「IR 之后不再读 TOML」⇒ 只能从 mm/min 折回去。
/// 全仓唯一逆换算点就是 [`header_facts`] 之外的 `glue_speed_mm_per_s()`。
pub const MM_PER_MINUTE: f64 = 60.0;

/// pass 循环内协作取消检查的**产品默认间隔**。再导出 postproc 的唯一定义，
/// 刻意不在这里重写 `from_millis(100)`（那会变成第二处字面量）。
pub use crate::postproc::cancel::DEFAULT_CANCEL_CHECK_INTERVAL;

/// 处理请求。
pub struct ProcessRequest {
    pub gcode_path: PathBuf,
    /// TOML 配置文件；其结构**就是** `Ir`（见 [`crate::config`]）。
    pub config_path: PathBuf,
    /// `--set A.B=value` 覆盖项，按给出顺序应用（后写覆盖先写）。
    pub overrides: Vec<String>,
    /// None / 等于输入路径 → 原地覆盖（.part + rename）。
    pub output_path: Option<PathBuf>,
}

/// 处理请求的另一张脸：**IR 已经在调用方手里**。
///
/// 存在理由（mkp-ssr 的钩子路径）：用户的预设 TOML 是 snake_case 的用户参数
/// （`[toolhead]` / `[wiping]`），不是 IR；它先经 `mkp-preset` 的映射变成 `Ir`，
/// 再进这条管线。让调用方把 IR 序列化成临时 TOML 再喂 [`ProcessRequest`] 是**错的**：
/// 那等于让同一个换算在两处发生（AGENTS.md §1.1），还平白多一次 IR→TOML→IR 往返。
///
/// **第 3 步（机型维度 + 禁区 + G-code 自报机型）仍然会跑**，即便调用方已经填过 ——
/// [`fill_machine_facts`] 对同一份机型表与同一份输入是幂等的，重跑一次换来
/// 「进管线的 IR 一定与这份输入匹配」，比省一次查表值。
pub struct IrProcessRequest {
    pub gcode_path: PathBuf,
    /// 已经过配置解析（或预设映射）的 IR。
    pub ir: Ir,
    /// None / 等于输入路径 → 原地覆盖（.part + rename）。
    pub output_path: Option<PathBuf>,
}

/// 第 2 步的 IR 从哪来 —— 私有，两张公开脸各自对应一个变体。
///
/// 用枚举而不是「先算 IR 再调统一入口」的理由是**顺序**：12 步里读输入是第 1 步、
/// 配置是第 2 步，先把配置解析提到管线之外会让 `Config` 事件早于 `Input` 发出，
/// 于是 [`assert_step_order`] 与进度展示都会看到一条乱序的步骤流。
enum IrSource {
    /// TOML 配置文件（结构 = IR）+ `--set` 覆盖项。CLI 走这条。
    Config {
        path: PathBuf,
        overrides: Vec<String>,
    },
    /// 调用方给的 IR。装箱是因为 `Ir` 很大，不装箱会让整个枚举按最大变体对齐。
    Prebuilt(Box<Ir>),
}

/// 管线的内部作业单：两张公开脸都归一到这里。
struct Job {
    gcode_path: PathBuf,
    output_path: Option<PathBuf>,
    ir_source: IrSource,
}

/// 处理产物（printtime 只填字段，不作为 transform）。
pub struct ProcessResult {
    pub output_path: PathBuf,
    pub stats: Stats,
    pub print_time: Option<crate::postproc::printtime::Result>,
    pub warnings: Vec<String>,
    /// 实际执行到的步骤，供顺序断言与展示。
    pub executed_steps: Vec<Step>,
    pub input_sha256: String,
    pub detail_facts: DetailFacts,
}

/// 出口事实集 —— **只搬运，不新算**。
///
/// 来源仓库这里有 11 个字段，其中 7 个是 GUI 详情面板专用。本项目没有那个面板，
/// **逐个写明搬去哪了**（删掉不写等于让下一个人以为它从来不存在）：
///
/// | 来源字段 | 现在在哪 |
/// |---|---|
/// | `glue_speed_mm_per_s` | 删。它是 `Toolhead.MaxSpeed / 60` 的纯派生值，`dump-ir` 的输出里有 `MaxSpeed`，除一下就是 |
/// | `machine_preset_name` | 移到 [`header_facts`]（`dump-ir --gcode <in>` 用它） |
/// | `process_preset_name` | 同上 |
/// | `slicer_ironing_enabled` | 同上 |
/// | `three_mf_file_name` | 同上 |
/// | `layer_index_len` | **删，且 `dump-ir` 也给不出**：它需要 pass1 的产物，而 `dump-ir` 不跑 pass1。它唯一的消费者是 GUI 的 `totalLayers` 在 `TotalLayerNumber == 0` 时的回退分支，CLI 没有这个展示面 |
///
/// 剩下这 4 个留着，因为它们是**这条链上才有的事实**，别处无法重算。
pub struct DetailFacts {
    /// 全量 IR 的**最终态**（12 步里被就地修改过）。
    pub ir: crate::ir::Ir,
    /// `WipingApplyResult.support_fallback`。
    pub support_fallback: bool,
    /// 同上：`WipingApplyResult.fallback_objects`。
    pub fallback_objects: Vec<String>,
    /// `ModeToCalibrationString(calibMode)`——**无条件**取一次（非动态模式下也有值）。
    pub calibration_mode: String,
}

/// 输入 G-code 头部刮取出来的 4 个事实。
///
/// 这些**只读输入、不碰 IR**，所以从管线里提出来单独成函数：`dump-ir --gcode <in>`
/// 复用它，管线本身不再需要它们（来源仓库把它们塞进 `DetailFacts` 是因为 GUI 要）。
pub struct HeaderFacts {
    pub machine_preset_name: String,
    pub process_preset_name: String,
    pub slicer_ironing_enabled: bool,
    pub three_mf_file_name: String,
}

/// 头部刮取。`path` 只用于 `; filename_format` 推 3MF 名时的兜底。
pub fn header_facts(content: &[String], path: &str) -> HeaderFacts {
    HeaderFacts {
        machine_preset_name: crate::gcode::machine_preset_name(content),
        process_preset_name: crate::gcode::process_preset_name(content),
        slicer_ironing_enabled: crate::gcode::slicer_ironing_enabled(content),
        three_mf_file_name: crate::gcode::three_mf_file_name(content, path),
    }
}

/// 第 2 步的本体：读配置 + 机型名归一 + 未命中即报错。
///
/// **提成公开函数的唯一理由**：`dump-ir` / `check` 两个子命令必须看到与 `run`
/// **完全同一条**配置解析路径。让 CLI 自己再拼一遍 `load + normalize + 检查`
/// 就是造第二个真相源 —— 那种漂移的表现是「check 说没问题、run 却报错」。
pub fn config_ir(config_path: &Path, overrides: &[String]) -> Result<Ir, PostprocError> {
    let mut ir = config::load_with_overrides(config_path, overrides)?;
    let raw_machine = ir.machine.machine_type.clone();
    ir.machine.machine_type = crate::postproc::machine_dims::normalize_to_canonical(&raw_machine);
    if ir.machine.machine_type.is_empty() {
        // 收紧点（见模块文档差异 4）：来源在这里会带着空机型继续跑完。
        return Err(PostprocError::MissingMachine {
            path: format!(
                "{}（Machine.MachineType = {raw_machine:?} 不是已知机型）",
                config_path.display()
            ),
        });
    }
    // 第二道：别名认识 ≠ 尺寸表里有。两张表实测不一致（aliasMap 6 个规范名、
    // machine_dimensions 5 个，差 `A2L`），而尺寸表未命中会安静地留下 0×0 的运动范围 ——
    // `check` 曾因此对一台 0×0 的机器说「配置可用」，真跑才在边界检查处失败、报错点离真因很远。
    if !crate::postproc::machine_dims::has_machine_dimensions(&ir.machine.machine_type) {
        return Err(PostprocError::MissingMachine {
            path: format!(
                "{}（Machine.MachineType = {raw_machine:?} 归一为 {:?}，但内置机型尺寸表里没有它 —— \
                 别名表认识这个名字、尺寸表却没有对应条目，属于内置数据不一致，请换一个机型）",
                config_path.display(),
                ir.machine.machine_type
            ),
        });
    }
    Ok(ir)
}

/// 第 3 步的本体：机型维度 + 禁区 + G-code 里自报的机型。
///
/// `raw_gcode = None`（`dump-ir` 不带输入时）⇒ `gcode_machine_type` 填 `"UNKNOWN"`，
/// 与「输入里检测不到机型」走**同一条**兜底，不新造第三种取值。
pub fn fill_machine_facts(ir: &mut Ir, raw_gcode: Option<&str>) {
    let machine_type = ir.machine.machine_type.clone();
    crate::postproc::machine_dims::fill_machine_dims_from_toml(&machine_type, ir);
    crate::postproc::machine_dims::populate_forbidden_zones(ir, &machine_type);
    ir.machine.gcode_machine_type = match raw_gcode {
        Some(raw) => crate::postproc::machine_dims::detect_machine_from_gcode(raw),
        None => String::new(),
    };
    if ir.machine.gcode_machine_type.is_empty() {
        ir.machine.gcode_machine_type = "UNKNOWN".to_string();
    }
}

/// 第 2 + 第 3 步 = `dump-ir` / `check` 眼里的「完整 IR」。
///
/// 到这一步为止的 IR 是**纯配置侧事实**（配置 + 默认值兜底 + 机型表）；
/// 再往后（校准检测 / 擦拭决策 / pass1 / pass2）就要吃输入 G-code 并改 IR，
/// 那些是 `run` 的事，`dump-ir` 刻意**不做** —— 否则它就等于跑一遍后处理。
pub fn resolve_ir(
    config_path: &Path,
    overrides: &[String],
    raw_gcode: Option<&str>,
) -> Result<Ir, PostprocError> {
    let mut ir = config_ir(config_path, overrides)?;
    fill_machine_facts(&mut ir, raw_gcode);
    Ok(ir)
}

/// 两遍扫描统计的合并视图（pass2 的塔警告并入 pass1 统计）。
pub struct Stats {
    pub pass1: crate::postproc::pass1::ProcessStats,
    pub tower_height: f64,
}

/// 12 步管线主体。每步失败立即返回（不允许 Warn 后继续）。
///
/// `cancel` 协作取消：**每步入口**一道 checkpoint（11 处）；**最后一道在 write 步入口**
/// ——开始落盘就写完，落盘后（含 printtime）不再响应取消。
pub fn process(
    request: ProcessRequest,
    sink: &mut dyn ProgressSink,
    cancel: &CancelToken,
) -> Result<ProcessResult, PostprocError> {
    process_with_cancel_interval(request, sink, cancel, DEFAULT_CANCEL_CHECK_INTERVAL)
}

/// 同 [`process`]，但 IR 由调用方给出（钩子 / GUI 走这条）。
pub fn process_with_ir(
    request: IrProcessRequest,
    sink: &mut dyn ProgressSink,
    cancel: &CancelToken,
) -> Result<ProcessResult, PostprocError> {
    process_with_ir_and_cancel_interval(request, sink, cancel, DEFAULT_CANCEL_CHECK_INTERVAL)
}

/// [`process_with_ir`] 的判据缝，理由同 [`process_with_cancel_interval`]。
pub fn process_with_ir_and_cancel_interval(
    request: IrProcessRequest,
    sink: &mut dyn ProgressSink,
    cancel: &CancelToken,
    cancel_check_interval: std::time::Duration,
) -> Result<ProcessResult, PostprocError> {
    run(
        Job {
            gcode_path: request.gcode_path,
            output_path: request.output_path,
            ir_source: IrSource::Prebuilt(Box::new(request.ir)),
        },
        sink,
        cancel,
        cancel_check_interval,
    )
}

/// 判据缝：显式指定 pass 循环内协作检查的节流间隔。
///
/// 存在理由（照搬来源的教训，不许删）：「取消停在循环内、而不是跑完在下一个步骤边界
/// 被截住」这条判据，用 100ms 时间窗去测一条墙钟可能只有几十毫秒的 pass，结果由
/// **机器速度**决定 ⇒ 判据确定性假红。判据传 `Duration::ZERO` ⇒ 每轮都真查 ⇒
/// 断言恢复成机器无关的确定性事实，且阈值一个字不用放宽。
///
/// **产品调用方（CLI）不得使用本函数**，走 [`process`] 即可。
pub fn process_with_cancel_interval(
    request: ProcessRequest,
    sink: &mut dyn ProgressSink,
    cancel: &CancelToken,
    cancel_check_interval: std::time::Duration,
) -> Result<ProcessResult, PostprocError> {
    run(
        Job {
            gcode_path: request.gcode_path,
            output_path: request.output_path,
            ir_source: IrSource::Config {
                path: request.config_path,
                overrides: request.overrides,
            },
        },
        sink,
        cancel,
        cancel_check_interval,
    )
}

/// 12 步管线的**唯一实现**。两张公开脸（[`process`] / [`process_with_ir`]）都归到这里，
/// 差别只在第 2 步的 IR 从哪来；其余 11 步一个字都不分叉 ——
/// 分叉了就等于有两条会各自漂移的产品路径。
fn run(
    job: Job,
    sink: &mut dyn ProgressSink,
    cancel: &CancelToken,
    cancel_check_interval: std::time::Duration,
) -> Result<ProcessResult, PostprocError> {
    let Job {
        gcode_path,
        output_path,
        ir_source,
    } = job;
    /// 步骤边界检查点：命中即中止，只返回、不写部分输出
    /// （原地模式的原文件因此保证不动）。
    fn checkpoint(cancel: &CancelToken) -> Result<(), PostprocError> {
        if cancel.is_cancelled() {
            return Err(PostprocError::Cancelled {
                message: "用户已取消后处理".to_string(),
            });
        }
        Ok(())
    }

    let mut executed: Vec<Step> = Vec::new();

    // ---- step 1: input —— 唯一一次读输入文件 + sha256 ----
    checkpoint(cancel)?;
    emit(sink, Step::Input, "正在读取文件...");
    executed.push(Step::Input);
    let raw = std::fs::read_to_string(&gcode_path).map_err(|e| PostprocError::Io {
        op: "read_gcode",
        source: e,
    })?;
    let input_sha256 = sha256_hex(raw.as_bytes());
    let content: Vec<String> = raw
        .lines()
        .map(|l| l.trim_end_matches('\r').to_string())
        .collect();

    // ---- step 2: config —— 读 TOML（结构 = Ir）+ 覆盖项 + 机型归一 ----
    // 与来源的唯一结构性差异：这里**没有**预设读取、两层校验与 1,130 行映射。
    // 「缺键即错」由 serde（无 `#[serde(default)]`）承担，判据在 `config.rs`。
    checkpoint(cancel)?;
    executed.push(Step::Config);
    let mut ir = match ir_source {
        IrSource::Config { path, overrides } => {
            emit(sink, Step::Config, "正在加载配置...");
            config_ir(&path, &overrides)?
        }
        // 文案与 Config 分支**刻意不同**：这一步在这条路径上确实没有读任何文件，
        // 写成一样的「正在加载配置...」等于让进度条撒谎。
        IrSource::Prebuilt(ir) => {
            emit(sink, Step::Config, "配置已就绪（IR 由调用方给出）");
            *ir
        }
    };

    // ---- step 3: machine-detect —— 维度填充 + 禁区 ----
    checkpoint(cancel)?;
    emit(sink, Step::MachineDetect, "识别机型...");
    executed.push(Step::MachineDetect);
    let machine_type = ir.machine.machine_type.clone();
    fill_machine_facts(&mut ir, Some(&raw));

    // ---- step 4: parse —— 校准模式检测 ----
    checkpoint(cancel)?;
    emit(sink, Step::Parse, "解析 G-code...");
    executed.push(Step::Parse);
    let calib_mode = crate::postproc::calibration::detect_mode(&content);
    ir.safety.is_calibration_mode = crate::postproc::calibration::is_any(calib_mode);
    // **无条件**取一次校准模式串（只在 is_dynamic 分支里算 ⇒ 非动态模式下出口没有
    // 这个事实）。零新计算，只是提早取。
    let calibration_mode = crate::postproc::calibration::mode_to_calibration_string(calib_mode);

    // ---- step 5: support —— DecideWiping → ApplyWipingDecision ----
    checkpoint(cancel)?;
    emit(sink, Step::Support, "支撑面检测...");
    executed.push(Step::Support);
    // preferred 的来源：来源仓库取 `preset.config.wiping.have_wiping_components`，
    // 而 `ir::build.rs:74` 是 `ir.wiping.preferred_mode = cfg.…have_wiping_components`
    // 的**纯恒等拷贝**（无归一化，Task 8 实测确认）⇒ 从 IR 取同一个值，
    // 而不是把 TOML 结构重新拉进来。钉住这条恒等的判据在
    // `tests/pipeline_wiping_source.rs`。
    let preferred = ir.wiping.preferred_mode.clone();
    let decision = crate::postproc::support::decide_wiping(&preferred, &content);
    let wiping_applied = crate::postproc::support::apply_wiping_decision(&mut ir, &decision);

    // ---- step 6: collision ----
    checkpoint(cancel)?;
    emit(sink, Step::Collision, "碰撞检测...");
    executed.push(Step::Collision);
    if crate::postproc::calibration::need_collision_check(calib_mode) {
        let collision = crate::postproc::disk::detect_tower_collision(&content, &ir);
        if collision.has_collision {
            return Err(PostprocError::Collision {
                message: "E_GCODE_COLLISION_001: 模型与擦料塔位置冲突".to_string(),
            });
        }
    }

    // ---- step 7: calib-check —— 仅动态校准：BBox + 质心 + dry-run ----
    checkpoint(cancel)?;
    if crate::postproc::calibration::is_dynamic(calib_mode, &ir) {
        emit(sink, Step::CalibCheck, "校准模型检测...");
        executed.push(Step::CalibCheck);
        let bbox = crate::postproc::disk::compute_bbox(&content)?;
        let centroid = centroid_of(&content);
        let mode_str = crate::postproc::calibration::mode_to_calibration_string(calib_mode);
        crate::postproc::disk::generate_calibration_gcode_with_centroid(
            mode_str,
            &machine_type,
            &ir,
            Some(&bbox),
            centroid,
            DEFAULT_Z_DWELL_SEC,
            DEFAULT_XY_DWELL_SEC,
        )?;
    }

    // ---- step 8: pass1 ----
    checkpoint(cancel)?;
    emit(sink, Step::Pass1, "第一遍扫描：收集信息...");
    executed.push(Step::Pass1);
    let toml_machine = machine_type.clone();
    let mut p1_fwd = |pct: f64, msg: String| {
        sink.emit(ProgressEvent {
            step: Step::Pass1,
            fraction_in_step: Some((pct / 100.0).clamp(0.0, 1.0) as f32),
            message: msg,
        });
    };
    let p1 = crate::postproc::pass1::first_pass(
        &content,
        &mut ir,
        &toml_machine,
        &mut p1_fwd,
        cancel,
        cancel_check_interval,
    )?;
    let final_tower_height = p1.stats.max_z_height;
    let pass1_stats = p1.stats.clone();

    // ---- step 9: pass2（消耗式吃 Pass1Output）----
    checkpoint(cancel)?;
    emit(sink, Step::Pass2, "第二遍扫描：应用修改...");
    executed.push(Step::Pass2);
    let mut p2_fwd = |pct: f64, msg: String| {
        sink.emit(ProgressEvent {
            step: Step::Pass2,
            fraction_in_step: Some((pct / 100.0).clamp(0.0, 1.0) as f32),
            message: msg,
        });
    };
    let mut pass2_stats = crate::postproc::pass1::ProcessStats::default();
    let p2 = crate::postproc::pass2::second_pass(
        p1,
        &mut ir,
        final_tower_height,
        &toml_machine,
        &mut p2_fwd,
        &mut pass2_stats,
        cancel,
        cancel_check_interval,
    )?;
    let tower_height = p2.tower_height;
    let mut out_lines = p2.lines;

    // ---- step 10: calibration —— 校准插入（内存行序列）----
    checkpoint(cancel)?;
    if crate::postproc::calibration::is_any(calib_mode) {
        emit(sink, Step::Calibration, "校准 G-code 插入...");
        executed.push(Step::Calibration);
        let (bbox, centroid) = if crate::postproc::calibration::is_dynamic(calib_mode, &ir) {
            (
                Some(crate::postproc::disk::compute_bbox(&content)?),
                centroid_of(&content),
            )
        } else {
            (None, crate::postproc::disk::CentroidResult::default())
        };
        out_lines = crate::postproc::calibration::insert_calibration_gcode(
            out_lines,
            calib_mode,
            &mut ir,
            bbox,
            centroid,
            DEFAULT_Z_DWELL_SEC,
            DEFAULT_XY_DWELL_SEC,
        )?;
    }

    // ---- step 11: write —— 三个 hooks + .part + rename ----
    // 最后一道取消检查点：此后开始落盘，写就写完。
    checkpoint(cancel)?;
    emit(sink, Step::Write, "正在写入文件...");
    executed.push(Step::Write);
    out_lines = hooks::apply_begin_stage_retract(
        out_lines,
        &ir,
        DEFAULT_BEGIN_STAGE_RETRACT,
        DEFAULT_BEGIN_STAGE_RETRACT_Z_LIFT_MM,
    );
    out_lines = hooks::apply_skip_vibration_calibration(out_lines, DEFAULT_SKIP_VIBRATION);
    out_lines = hooks::apply_pre_print_bed_glue(out_lines, &ir);

    // MKP 标记协议 v1 收尾：ev 按落盘行序重编号 + 文件头 ;MKP_INFO + 文件尾 ;MKP_STATS。
    // 必须是最后一道行变换 —— STATS 数的就是落盘的那份，STATS_MATCH 靠这个成立。
    out_lines = crate::postproc::marks::finalize(out_lines, &machine_type);

    let final_output_path = match &output_path {
        Some(p) if p != &gcode_path => p.clone(),
        _ => gcode_path.clone(),
    };
    write_atomic(&final_output_path, &out_lines)?;

    // ---- step 12: printtime —— 不是 transform，只填字段 ----
    // 执行过的步骤必须发事件：来源这一步以前只 push 不 emit ⇒ 最后一帧永远停在
    // 「写入文件」，完成后看起来像卡死。
    emit(sink, Step::PrintTime, "打印时间估算...");
    executed.push(Step::PrintTime);
    let mut out_text = out_lines.join("\n");
    out_text.push('\n');
    let print_time = crate::postproc::printtime::estimate(
        &out_text,
        &crate::postproc::printtime::Options {
            compute_delta: true,
            startup_overhead_seconds: 240.0,
        },
    );

    Ok(ProcessResult {
        output_path: final_output_path,
        stats: Stats {
            pass1: merge_stats(pass1_stats, pass2_stats),
            tower_height,
        },
        print_time: Some(print_time),
        warnings: ir.warnings.clone(),
        executed_steps: executed,
        input_sha256,
        detail_facts: DetailFacts {
            ir,
            support_fallback: wiping_applied.support_fallback,
            fallback_objects: wiping_applied.fallback_objects,
            calibration_mode: calibration_mode.to_string(),
        },
    })
}

fn centroid_of(content: &[String]) -> crate::postproc::disk::CentroidResult {
    match crate::postproc::disk::compute_extrusion_centroid(content) {
        Ok((cx, cy)) => crate::postproc::disk::CentroidResult {
            centroid_x: cx,
            centroid_y: cy,
            valid: true,
        },
        Err(_) => crate::postproc::disk::CentroidResult::default(),
    }
}

fn emit(sink: &mut dyn ProgressSink, step: Step, message: &str) {
    sink.emit(ProgressEvent {
        step,
        fraction_in_step: None,
        message: message.to_string(),
    });
}

/// `<out>.part` → rename 原子替换；rename 失败回退复制。
///
/// **写盘豁免**（`clippy::disallowed_methods`）：禁列禁 `fs::write` 的理由是
/// "会截断目标文件、崩溃时留半个文件"。这里写的是 **`.part` 临时文件**，
/// 目标文件直到 rename 那一刻都没被碰过 —— 正是禁列想要的那个模式，
/// 只是它自己就是实现，没法再转调别人。
///
/// **退役条件**：Task 19 统一写盘入口落地、且内核能用上那个入口时，改成转调它。
#[allow(clippy::disallowed_methods)]
fn write_atomic(path: &Path, lines: &[String]) -> Result<(), PostprocError> {
    let part_path = {
        let mut p = path.as_os_str().to_os_string();
        p.push(".part");
        PathBuf::from(p)
    };
    let mut text = lines.join("\n");
    text.push('\n');
    std::fs::write(&part_path, &text).map_err(|e| PostprocError::Io {
        op: "write_part",
        source: e,
    })?;
    if std::fs::rename(&part_path, path).is_err() {
        std::fs::copy(&part_path, path).map_err(|e| PostprocError::Io {
            op: "rename_fallback_copy",
            source: e,
        })?;
        let _ = std::fs::remove_file(&part_path);
    }
    Ok(())
}

/// pass2 统计并入（塔层高警告是 pass2 产物）。
fn merge_stats(
    mut p1: crate::postproc::pass1::ProcessStats,
    p2: crate::postproc::pass1::ProcessStats,
) -> crate::postproc::pass1::ProcessStats {
    p1.tower_layer_height_warnings = p2.tower_layer_height_warnings;
    p1
}

/// 十六进制小写。
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// 内容的 sha256（小写十六进制）。
///
/// **公开的理由**：`ProcessResult.input_sha256` 就是这么算的，而应用层要给
/// 历史档案里的产物算同一种指纹（`gcode_history/*_meta.json` 的 `outputSha256`）。
/// 让应用层自己引一遍 `sha2` + 自己写一遍 hex，就是同一个算法在两处实现 ——
/// 而"两处算出来不一样"这种漂移是静默的（两个 64 位十六进制串谁也不会去比）。
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// 步骤有序子序列断言（钉顺序与必含，不绑总长度）。
pub fn assert_step_order(executed: &[Step]) {
    let mut expected_idx = 0;
    for step in executed {
        while expected_idx < STEPS.len() && STEPS[expected_idx] != *step {
            expected_idx += 1;
        }
        assert!(
            expected_idx < STEPS.len(),
            "步骤 {step:?} 不在 12 步序中或顺序违规（executed={executed:?}）"
        );
    }
    // 必含（非条件步）
    for required in [
        Step::Input,
        Step::Config,
        Step::Pass1,
        Step::Pass2,
        Step::Write,
    ] {
        assert!(
            executed.contains(&required),
            "必含步骤 {required:?} 缺失（executed={executed:?}）"
        );
    }
}
