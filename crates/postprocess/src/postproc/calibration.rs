// 校准检测与插入（对照 internal/calibration/mode.go + engine/helpers.go 的
// InsertCalibrationGcode / filterModelMotionPaths / MinimizeStartupGcode 家族）。
//
// 偏离登记（tasks.md 13.3）：Go 侧 InsertCalibrationGcode 会**再读一次输出文件**
// 再原子写回；Rust 改为在内存行序列上插入后返回，落盘由 engine write 步统一
// 执行 —— 行为等价、少两次 IO。
//
// Go 里这些函数挂在 Engine（helpers.go），但它们是纯内容变换、无编排逻辑，
// 按「算法全下沉 postproc」原则落在本模块；engine 只调用。
//
// **校准身份从哪来（2026-10-18 定案）**：只有一个地方 —— 原始 3MF 的
// `MKPSE:Calibration`（`gcode::model_identity`），由钩子顺着 `origin.txt` 读出来，
// 经 [`crate::pipeline::IrProcessRequest::calibration`] 传进管线。
//
// 那条 `DetectMode`（mode.go:23，按 G-code 里的标记串认校准模式）**已整条删除**。
// 删的理由不是简化，是**它会给出错答案**：标记住在 `Metadata/custom_gcode_per_layer.xml`
// 里，某些版本的 Bambu Studio 存一次就把这个条目丢掉（真机核对：原件
// `BambuStudio-02.05.00.66` 有、被 `02.08.02.60` 存过一遍的那份没有）。于是同一份模型
// 换个版本另存一遍，G-code 里就一个字都没有，管线便把校准件当普通模型跑。
//
// **G-code 里即便还留着旧注释，也一律忽略。** 几何只参与认档
// （`calib_models::find_profile`），永远不能决定"是不是校准件"。
#![allow(
    clippy::collapsible_if,
    clippy::needless_range_loop,
    clippy::too_many_lines,
    clippy::if_same_then_else
)]

use crate::diag::PostprocError;
use crate::gcode::{
    MARKER_FILAMENT_END, MARKER_GLUE_FINISHED, MARKER_PREPARE_NEXT_TOWER, MARKER_WIPE_END,
    MARKER_WIPE_START, filter_e_moves, has_slicer_comment_prefix, remove_final_lower_z,
};
use crate::ir::{CALIBRATION_EXEC_DIRECT_CALIBRATE, Ir};

use crate::postproc::disk::{BBox, CentroidResult, generate_calibration_gcode_with_centroid_rec};

/// `Mode`（calibration/mode.go:11）。
///
/// 缺省是 `None`（普通模型）—— 与"没查到身份"是同一件事，所以让 `Default` 取它，
/// 调用点不用写 `Mode::None` 这种重复。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    None,
    ZOffset,
    XyPrecise,
    XyRough,
    Repetition,
}

/// Z 校准的涂胶总长常量（mode.go:21）。
pub const Z_CALIBRATION_GLUE_TOTAL_LENGTH: f64 = 2431.32;

/// 身份串 → `Mode`。**全仓唯一的解析出口**。
///
/// 认的是 `mode_to_calibration_string` 那四个稳定 id（`ZOffset` / `Precise` / `Rough` /
/// `Repetition`）。3MF 的 `MKPSE:Calibration` 与档位表的 `mode` 数组都只写这几个字，
/// 也只经这里变成 `Mode` —— 前端与存档不许自己拼。
///
/// 不认 `XY`：那是几何分组的旧口径（一块板服务 Precise / Rough 两个身份），
/// 身份层面从来没有 `XY` 这个模式。大小写不敏感、首尾空白忽略。
pub fn parse_mode_id(id: &str) -> Option<Mode> {
    let t = id.trim();
    if t.eq_ignore_ascii_case("ZOffset") {
        return Some(Mode::ZOffset);
    }
    if t.eq_ignore_ascii_case("Precise") {
        return Some(Mode::XyPrecise);
    }
    if t.eq_ignore_ascii_case("Rough") {
        return Some(Mode::XyRough);
    }
    if t.eq_ignore_ascii_case("Repetition") {
        return Some(Mode::Repetition);
    }
    None
}

pub fn is_any(m: Mode) -> bool {
    m != Mode::None
}

/// 校准件的人类可读名字（`Z 偏移校准` / `XY 精度校准` / `XY 粗校准` / `重复性校准`）。
///
/// 与 [`mode_to_calibration_string`] 分工不同：那个是**给内核与报告用的稳定 id**
/// （`ZOffset` / `Precise` …），这个只是**问句正文里那一小截字**。
pub fn mode_display_name(m: Mode) -> &'static str {
    match m {
        Mode::ZOffset => "Z 偏移校准",
        Mode::XyPrecise => "XY 精度校准",
        Mode::XyRough => "XY 粗校准",
        Mode::Repetition => "重复性校准",
        Mode::None => "",
    }
}

pub fn need_collision_check(m: Mode) -> bool {
    m == Mode::None
}

pub fn need_glue_boundary(m: Mode) -> bool {
    m == Mode::None
}

/// `IsDynamic`（mode.go:47）：动态校准（PresetExec 路径）判定。
pub fn is_dynamic(m: Mode, ir_data: &Ir) -> bool {
    match m {
        Mode::ZOffset => ir_data.safety.z_calibration == "new",
        Mode::XyPrecise | Mode::XyRough => ir_data.safety.xy_calibration == "new",
        Mode::Repetition => true,
        Mode::None => false,
    }
}

/// `ModeToCalibrationString`（helpers.go:57）。
pub fn mode_to_calibration_string(m: Mode) -> &'static str {
    match m {
        Mode::ZOffset => "ZOffset",
        Mode::XyPrecise => "Precise",
        Mode::XyRough => "Rough",
        Mode::Repetition => "Repetition",
        Mode::None => "",
    }
}

const CALIBRATION_RETRACT_MM: f64 = 2.0;

fn is_new_calibration_mode(mode_str: &str, model_bbox: Option<&BBox>) -> bool {
    if model_bbox.is_none() {
        return false;
    }
    matches!(mode_str, "Precise" | "Rough" | "ZOffset")
}

/// **校准这一趟「用了什么」的回执**（2026-10-18，执行记录用）。
///
/// 为什么需要它：`resolve_profile` 认出的档、实测尺寸、容差全都在
/// [`insert_calibration_gcode`] **内部**就被消费掉了，成功那趟一个字都不出门 ——
/// 于是"为什么这次用了加长版的基准"永远无法解释。这个结构把那几个事实**带出来**，
/// 但它**不参与任何计算**：里面每个值都是同一次调用里已经在手的东西。
///
/// 全字段可空：经典分支（按机型固定坐标走）根本不认档 ⇒ `profile_id` 为 `None`；
/// 认档失败 ⇒ 三个 geometry 字段都是 `None`（错误本身照旧报出去）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CalibrationReceipt {
    /// 实测挤出包围盒（只有 New 变体与 Repetition 拿得到；经典分支没有）。
    pub measured: Option<BBox>,
    /// 命中的档 id（`presets/calibration_models.toml` 的 `[[models]] id`）。
    pub profile_id: Option<String>,
    /// 该档的尺寸容差（mm）。
    pub tolerance: Option<f64>,
    /// 该档声明的标称外形（宽, 高）。
    pub footprint: Option<(f64, f64)>,
}

impl CalibrationReceipt {
    /// 从认出来的那一档填一次（认档成功才有）。
    pub(crate) fn with_profile(
        measured: &BBox,
        profile: &crate::postproc::calib_models::ModelProfile,
    ) -> Self {
        Self {
            measured: Some(*measured),
            profile_id: Some(profile.id.clone()),
            tolerance: Some(profile.tolerance),
            footprint: Some((profile.footprint_w, profile.footprint_h)),
        }
    }
}

/// `InsertCalibrationGcode`（helpers.go:162）——内存行序列版（13.3 偏离）。
/// 返回插入后的行序列；落盘（.part + rename）由 engine write 步执行。
///
/// 不关心回执的调用方（CLI / 判据）走这一张脸：与
/// [`insert_calibration_gcode_with_receipt`] 传 `None` **完全等价**。
#[allow(clippy::too_many_arguments)]
pub fn insert_calibration_gcode(
    content: Vec<String>,
    mode: Mode,
    ir_data: &mut Ir,
    original_bbox: Option<BBox>,
    centroid: CentroidResult,
    z_dwell_sec: i64,
    xy_dwell_sec: i64,
) -> Result<Vec<String>, PostprocError> {
    insert_calibration_gcode_with_receipt(
        content,
        mode,
        ir_data,
        original_bbox,
        centroid,
        z_dwell_sec,
        xy_dwell_sec,
        None,
    )
}

/// 同 [`insert_calibration_gcode`]，但把「用了什么」写进 `receipt`。
///
/// 回执里的值全是同一次调用里已经在手的事实，**不新增任何计算**；
/// `receipt = None` 时行为与旧入口逐字节相同（reference 判据因此不受影响）。
#[allow(clippy::too_many_arguments)]
pub fn insert_calibration_gcode_with_receipt(
    content: Vec<String>,
    mode: Mode,
    ir_data: &mut Ir,
    original_bbox: Option<BBox>,
    centroid: CentroidResult,
    z_dwell_sec: i64,
    xy_dwell_sec: i64,
    receipt: Option<&mut CalibrationReceipt>,
) -> Result<Vec<String>, PostprocError> {
    crate::ir::fill_defaults(ir_data);
    let machine_type_str = ir_data.machine.machine_type.clone();

    let model_bbox = original_bbox;

    let mode_str = mode_to_calibration_string(mode);
    let generated = generate_calibration_gcode_with_centroid_rec(
        mode_str,
        &machine_type_str,
        ir_data,
        model_bbox.as_ref(),
        centroid,
        z_dwell_sec,
        xy_dwell_sec,
        receipt,
    );
    let calibration_lines = generated
        .map_err(|e| match e {
            // 形变检测（`E_CAL_MISMATCH_001/002`）的码是对外契约，不许被
            // `E_CAL_FAILED_001` 吃掉 —— 前者告诉用户「把校准模型转回来」，
            // 后者只能让人去翻日志。
            PostprocError::CalibrationMismatch { .. } => e,
            // 这里只留**诊断细节**：「校准代码没有生成出来」那句与码都在表里
            // （`messages.toml` 的 E_CAL_FAILED_001），正文里再写一遍码就是自我引用。
            other => PostprocError::Calibration {
                message: format!("mode={mode_str}：{other}"),
            },
        })?
        .ok_or_else(|| PostprocError::Calibration {
            message: format!("无输出，mode={mode_str}"),
        })?;

    let is_direct_calibrate =
        ir_data.safety.calibration_execution_mode == CALIBRATION_EXEC_DIRECT_CALIBRATE;
    let is_new_mode = is_new_calibration_mode(mode_str, model_bbox.as_ref());

    let mut output = if is_direct_calibrate {
        let preserve =
            machine_type_str == "P1S" || machine_type_str == "A1" || machine_type_str == "A1_MINI";
        let filtered = filter_model_motion_paths(&content, preserve);
        minimize_startup_gcode(filtered, &machine_type_str)
    } else if ir_data.prefs.skip_vibration_calibration {
        /* ★ 2026-10-17：这一支原来是**无条件**跑的 —— 也就是说"跳过震动检测"从来没听过
        用户的设置，一直被当成开着。现在按设置页那一档来（出厂关 ⇒ 保留 M970/M974）。 */
        remove_vibration_detection_from_mech_mode(&content)
    } else {
        content
    };

    let insert_idx = output
        .iter()
        .position(|l| l.contains(MARKER_FILAMENT_END) && !l.contains('='));

    match insert_idx {
        None => {
            if is_new_mode {
                output.push("; Calibration Runtime: retract and cool down".to_string());
                output.push("G92 E0".to_string());
                output.push(format!("G1 E-{CALIBRATION_RETRACT_MM:.1} F1800"));
                output.push("G92 E0".to_string());
                output.push("M104 S30".to_string());
            }
            output.extend(calibration_lines);
        }
        Some(idx) => {
            let mut new_lines: Vec<String> = Vec::new();
            if is_new_mode {
                new_lines.push("; Calibration Runtime: retract and cool down".to_string());
                new_lines.push("G92 E0".to_string());
                new_lines.push(format!("G1 E-{CALIBRATION_RETRACT_MM:.1} F1800"));
                new_lines.push("G92 E0".to_string());
                new_lines.push("M104 S30".to_string());
            }
            new_lines.extend(calibration_lines);
            let mut result = Vec::with_capacity(output.len() + new_lines.len());
            result.extend_from_slice(&output[..idx + 1]);
            result.extend(new_lines);
            result.extend_from_slice(&output[idx + 1..]);
            output = result;
        }
    }

    if is_new_mode {
        output = remove_final_lower_z(&output);
    }

    if is_direct_calibrate {
        output = filter_e_moves(&output);
    }

    Ok(output)
}

// ---- filterModelMotionPaths（helpers.go:526） ----

/// 删除模型区域的移动命令（direct_calibrate 模式），保留涂胶/活化/擦拭/收尾段。
pub fn filter_model_motion_paths(
    content: &[String],
    preserve_nozzle_load_line: bool,
) -> Vec<String> {
    let mut filtered: Vec<String> = Vec::with_capacity(content.len());
    let mut in_glue_region = false;
    let mut in_gluepen_region = false;
    let mut in_wipe_region = false;
    let mut in_model_region = false;
    let mut in_end_region = false;
    let mut in_nozzle_load_region = false;
    let mut in_config_block = false;
    let mut in_skippable_region = false;
    let mut found_first_non_custom_feature = false;

    for line in content {
        let trimmed = line.trim();

        if trimmed.contains("; CONFIG_BLOCK_START") {
            in_config_block = true;
            filtered.push(line.clone());
            continue;
        }
        if trimmed.contains("; CONFIG_BLOCK_END") {
            in_config_block = false;
            filtered.push(line.clone());
            continue;
        }
        if in_config_block {
            filtered.push(line.clone());
            continue;
        }

        if trimmed.contains(";===== nozzle load line") {
            in_nozzle_load_region = true;
            if preserve_nozzle_load_line {
                filtered.push(line.clone());
            }
            continue;
        }

        if in_nozzle_load_region {
            if trimmed.starts_with(";=====") {
                in_nozzle_load_region = false;
            } else {
                if preserve_nozzle_load_line {
                    let (processed_line, keep) = should_preserve_nozzle_load_line_line(line);
                    if keep {
                        filtered.push(processed_line);
                    }
                }
                continue;
            }
        }

        if trimmed.contains(MARKER_PREPARE_NEXT_TOWER) {
            in_glue_region = true;
        }
        if trimmed.contains(";Glueing Started") {
            in_glue_region = true;
        }
        if trimmed.contains(MARKER_GLUE_FINISHED) {
            in_glue_region = false;
        }
        if trimmed.contains(";Gluepen Revitalization Start") {
            in_gluepen_region = true;
        }
        if trimmed.contains(";Gluepen Revitalization End") {
            in_gluepen_region = false;
        }
        if trimmed.contains(MARKER_WIPE_START) {
            in_wipe_region = true;
        }
        if trimmed.contains(MARKER_WIPE_END) {
            in_wipe_region = false;
        }
        if trimmed.contains(MARKER_FILAMENT_END) && !trimmed.contains('=') {
            in_end_region = true;
        }

        if !in_glue_region
            && !in_gluepen_region
            && !in_wipe_region
            && !in_end_region
            && !in_model_region
            && !found_first_non_custom_feature
            && has_slicer_comment_prefix(trimmed, "FEATURE:")
            && !trimmed.starts_with("; FEATURE: Custom")
        {
            found_first_non_custom_feature = true;
            in_model_region = true;
        }

        if in_model_region
            && has_slicer_comment_prefix(trimmed, "FEATURE:")
            && trimmed.starts_with("; FEATURE: Custom")
        {
            in_model_region = false;
        }

        if in_glue_region || in_gluepen_region || in_wipe_region || in_end_region {
            filtered.push(line.clone());
            continue;
        }

        if in_model_region {
            if has_slicer_comment_prefix(trimmed, "SKIPPABLE_START") {
                in_skippable_region = true;
            }
            if has_slicer_comment_prefix(trimmed, "SKIPPABLE_END") {
                in_skippable_region = false;
            }
            if !in_skippable_region {
                if is_model_skeleton_line(trimmed) {
                    continue;
                }
                if is_model_motion_line(line) {
                    continue;
                }
            }
        }

        filtered.push(line.clone());
    }
    filtered
}

// ---- removeVibrationDetectionFromMechMode（helpers.go:645） ----

pub fn remove_vibration_detection_from_mech_mode(lines: &[String]) -> Vec<String> {
    let mut result: Vec<String> = Vec::with_capacity(lines.len());
    let mut in_mech_mode = false;

    for line in lines {
        let trimmed = line.trim();

        if trimmed.contains(";===== mech mode fast check") {
            in_mech_mode = true;
            result.push(line.clone());
            continue;
        }

        if in_mech_mode
            && (trimmed.starts_with(";=====") || trimmed.contains("; MACHINE_START_GCODE_END"))
        {
            in_mech_mode = false;
        }

        if in_mech_mode {
            let code_part = code_before_comment(trimmed);
            if !code_part.is_empty() {
                let mut fields = code_part.split_whitespace();
                if let Some(cmd) = fields.next() {
                    let cmd = cmd.to_uppercase();
                    if cmd.starts_with("M970") || cmd == "M974" {
                        continue;
                    }
                }
            }
        }

        result.push(line.clone());
    }
    result
}

// ---- 启动段最小化（helpers.go:684-1000） ----

pub fn minimize_startup_gcode(lines: Vec<String>, machine_type: &str) -> Vec<String> {
    match machine_type {
        "P1S" => minimize_startup_gcode_p1(lines),
        "A1" | "A1_MINI" => minimize_startup_gcode_a1(lines),
        "X1C" => minimize_startup_gcode_p1(lines), // X1 复用 P1（helpers.go:991）
        _ => lines,
    }
}

fn minimize_startup_gcode_p1(lines: Vec<String>) -> Vec<String> {
    let mut result: Vec<String> = Vec::with_capacity(lines.len());
    let mut in_startup_region = false;
    let mut in_prepare_region = false;
    let mut in_wipe_region = false;
    let mut in_bed_leveling_region = false;
    let mut in_home_after_wipe_region = false;
    let mut in_config_block = false;

    for line in lines {
        let trimmed = line.trim();

        if trimmed.contains("; CONFIG_BLOCK_START") {
            in_config_block = true;
            result.push(line.clone());
            continue;
        }
        if trimmed.contains("; CONFIG_BLOCK_END") {
            in_config_block = false;
            result.push(line.clone());
            continue;
        }
        if in_config_block {
            result.push(line.clone());
            continue;
        }

        if trimmed.contains(";===== machine:") {
            in_startup_region = true;
            result.push(line.clone());
            continue;
        }

        if trimmed.contains("; MACHINE_START_GCODE_END") {
            in_startup_region = false;
            in_prepare_region = false;
            in_wipe_region = false;
            in_bed_leveling_region = false;
            in_home_after_wipe_region = false;
            result.push(line.clone());
            continue;
        }

        if !in_startup_region {
            result.push(line.clone());
            continue;
        }

        if trimmed.contains(";===== prepare print temperature and material ==========") {
            in_prepare_region = true;
            result.push(line.clone());
            continue;
        }
        if trimmed.contains(";===== prepare print temperature and material end =====") {
            in_prepare_region = false;
            result.push(line.clone());
            continue;
        }

        if trimmed.contains(";===== wipe nozzle =====") {
            in_wipe_region = true;
            result.push(line.clone());
            continue;
        }
        if trimmed.contains(";===== wipe nozzle end ====") {
            in_wipe_region = false;
            result.push(line.clone());
            result.push("M104 S1".to_string());
            continue;
        }

        if in_wipe_region {
            result.push(line.clone());
            continue;
        }

        if trimmed.contains(";===== bed leveling =====") {
            in_bed_leveling_region = true;
            continue;
        }
        if trimmed.contains(";===== bed leveling end =====") {
            in_bed_leveling_region = false;
            continue;
        }
        if in_bed_leveling_region {
            continue;
        }

        if trimmed.contains(";===== home after wipe mouth =====")
            || trimmed.contains(";===== home after wipe mouth=====")
        {
            in_home_after_wipe_region = true;
            result.push(line.clone());
            continue;
        }
        if trimmed.contains(";===== home after wipe mouth end =====")
            || trimmed.contains(";===== home after wipe mouth end=====")
        {
            in_home_after_wipe_region = false;
            result.push(line.clone());
            continue;
        }
        if in_home_after_wipe_region {
            if is_judge_flag_line(trimmed) || is_m622_line(trimmed) || is_m623_line(trimmed) {
                continue;
            }
            result.push(line.clone());
            continue;
        }

        if let Some(processed) = minimize_startup_line(&line, trimmed, in_prepare_region) {
            result.push(processed);
        }
    }

    let result = remove_judge_flag_blocks(result, &["extrude_cali_flag"]);
    remove_vibration_detection_from_mech_mode(&result)
}

fn minimize_startup_gcode_a1(lines: Vec<String>) -> Vec<String> {
    let mut result: Vec<String> = Vec::with_capacity(lines.len());
    let mut in_startup_region = false;
    let mut in_prepare_region = false;
    let mut in_wipe_region = false;
    let mut in_bed_leveling_region = false;
    let mut in_home_after_wipe_region = false;
    let mut in_extrude_cali_region = false;
    let mut in_config_block = false;

    for line in lines {
        let trimmed = line.trim();

        if trimmed.contains("; CONFIG_BLOCK_START") {
            in_config_block = true;
            result.push(line.clone());
            continue;
        }
        if trimmed.contains("; CONFIG_BLOCK_END") {
            in_config_block = false;
            result.push(line.clone());
            continue;
        }
        if in_config_block {
            result.push(line.clone());
            continue;
        }

        if trimmed.contains(";===== machine:") {
            let lower = trimmed.to_lowercase();
            if lower.contains("a1") {
                in_startup_region = true;
                result.push(line.clone());
                continue;
            }
        }

        if trimmed.contains("; MACHINE_START_GCODE_END") {
            in_startup_region = false;
            in_prepare_region = false;
            in_wipe_region = false;
            in_bed_leveling_region = false;
            in_home_after_wipe_region = false;
            in_extrude_cali_region = false;
            result.push(line.clone());
            continue;
        }

        if !in_startup_region {
            result.push(line.clone());
            continue;
        }

        if in_extrude_cali_region {
            if trimmed.starts_with(";=====")
                && !trimmed.contains("extrude cali test")
                && !trimmed.contains("nozzle load line")
            {
                in_extrude_cali_region = false;
            } else {
                let (processed_line, keep) = should_preserve_nozzle_load_line_line(&line);
                if keep {
                    result.push(processed_line);
                }
                continue;
            }
        }

        if trimmed.contains(";===== prepare print temperature and material end") {
            in_prepare_region = false;
            result.push(line.clone());
            continue;
        }
        if trimmed.contains(";===== prepare print temperature and material") {
            in_prepare_region = true;
            result.push(line.clone());
            continue;
        }

        if trimmed.contains(";===== wipe nozzle end") {
            in_wipe_region = false;
            result.push(line.clone());
            result.push("M104 S1".to_string());
            continue;
        }
        if trimmed.contains(";===== wipe nozzle") {
            in_wipe_region = true;
            result.push(line.clone());
            continue;
        }

        if in_wipe_region {
            result.push(line.clone());
            continue;
        }

        if trimmed.contains(";===== bed leveling end") {
            in_bed_leveling_region = false;
            continue;
        }
        if trimmed.contains(";===== bed leveling") {
            in_bed_leveling_region = true;
            continue;
        }
        if in_bed_leveling_region {
            continue;
        }

        if trimmed.contains(";===== home after wipe mouth end") {
            in_home_after_wipe_region = false;
            result.push(line.clone());
            continue;
        }
        if trimmed.contains(";===== home after wipe mouth") {
            in_home_after_wipe_region = true;
            result.push(line.clone());
            continue;
        }
        if in_home_after_wipe_region {
            if is_judge_flag_line(trimmed) || is_m622_line(trimmed) || is_m623_line(trimmed) {
                continue;
            }
            result.push(line.clone());
            continue;
        }

        if trimmed.contains(";===== extrude cali test")
            || trimmed.contains(";===== nozzle load line")
        {
            in_extrude_cali_region = true;
            result.push(line.clone());
            continue;
        }

        if let Some(processed) = minimize_startup_line(&line, trimmed, in_prepare_region) {
            result.push(processed);
        }
    }

    let result = remove_judge_flag_blocks(result, &["extrude_cali_flag"]);
    remove_vibration_detection_from_mech_mode(&result)
}

fn minimize_startup_line(line: &str, trimmed: &str, in_prepare_region: bool) -> Option<String> {
    if is_m140_command(trimmed) {
        return Some(replace_m140_temp(line, 1));
    }
    if is_m190_command(trimmed) {
        return None;
    }
    if in_prepare_region
        && (is_m104_or_m109_with_high_temp(trimmed) || is_pure_e_extrusion(trimmed))
    {
        return None;
    }
    Some(line.to_string())
}

// ---- 小工具（helpers.go:996-1177 + 317-524） ----

fn code_before_comment(trimmed: &str) -> &str {
    match trimmed.find(';') {
        Some(idx) => trimmed[..idx].trim(),
        None => trimmed,
    }
}

fn is_judge_flag_line(trimmed: &str) -> bool {
    let code_part = code_before_comment(trimmed);
    if code_part.is_empty() {
        return false;
    }
    let mut fields = code_part.split_whitespace();
    let Some(cmd) = fields.next() else {
        return false;
    };
    match fields.next() {
        Some(second) => {
            cmd.eq_ignore_ascii_case("M1002") && second.eq_ignore_ascii_case("judge_flag")
        }
        None => false,
    }
}

fn is_m622_line(trimmed: &str) -> bool {
    let code_part = code_before_comment(trimmed);
    if code_part.is_empty() {
        return false;
    }
    code_part
        .split_whitespace()
        .next()
        .is_some_and(|c| c.eq_ignore_ascii_case("M622"))
}

fn is_m623_line(trimmed: &str) -> bool {
    let code_part = code_before_comment(trimmed);
    if code_part.is_empty() {
        return false;
    }
    code_part
        .split_whitespace()
        .next()
        .is_some_and(|c| c.eq_ignore_ascii_case("M623"))
}

fn extract_judge_flag_name(trimmed: &str) -> &str {
    let code_part = code_before_comment(trimmed);
    let mut fields = code_part.split_whitespace();
    fields.next();
    fields.next();
    fields.next().unwrap_or("")
}

fn remove_judge_flag_blocks(lines: Vec<String>, flag_names: &[&str]) -> Vec<String> {
    let mut result: Vec<String> = Vec::with_capacity(lines.len());
    let mut i = 0;
    while i < lines.len() {
        let line = &lines[i];
        let trimmed = line.trim();

        if is_judge_flag_line(trimmed) && flag_names.contains(&extract_judge_flag_name(trimmed)) {
            i += 1;
            while i < lines.len() && !is_m622_line(lines[i].trim()) {
                i += 1;
            }
            if i >= lines.len() {
                break;
            }
            i += 1;
            let mut depth = 1;
            while i < lines.len() && depth > 0 {
                let trimmed_inner = lines[i].trim();
                if is_m622_line(trimmed_inner) {
                    depth += 1;
                } else if is_m623_line(trimmed_inner) {
                    depth -= 1;
                }
                i += 1;
            }
            continue;
        }

        result.push(line.clone());
        i += 1;
    }
    result
}

fn is_m140_command(trimmed: &str) -> bool {
    let code_part = code_before_comment(trimmed);
    if code_part.is_empty() {
        return false;
    }
    code_part
        .split_whitespace()
        .next()
        .is_some_and(|c| c.eq_ignore_ascii_case("M140"))
}

fn is_m190_command(trimmed: &str) -> bool {
    let code_part = code_before_comment(trimmed);
    if code_part.is_empty() {
        return false;
    }
    code_part
        .split_whitespace()
        .next()
        .is_some_and(|c| c.eq_ignore_ascii_case("M190"))
}

fn is_m104_or_m109_with_high_temp(trimmed: &str) -> bool {
    let code_part = code_before_comment(trimmed);
    if code_part.is_empty() {
        return false;
    }
    let mut fields = code_part.split_whitespace();
    let Some(cmd) = fields.next() else {
        return false;
    };
    if !cmd.eq_ignore_ascii_case("M104") && !cmd.eq_ignore_ascii_case("M109") {
        return false;
    }
    for f in fields {
        let f_upper = f.to_uppercase();
        if let Some(temp_str) = f_upper.strip_prefix('S') {
            let mut temp: i64 = 0;
            for c in temp_str.chars() {
                if let Some(d) = c.to_digit(10) {
                    temp = temp * 10 + d as i64;
                } else {
                    break;
                }
            }
            if temp > 200 {
                return true;
            }
        }
    }
    false
}

fn is_pure_e_extrusion(trimmed: &str) -> bool {
    let code_part = code_before_comment(trimmed);
    if code_part.is_empty() {
        return false;
    }
    let mut fields = code_part.split_whitespace();
    let Some(cmd) = fields.next() else {
        return false;
    };
    if !cmd.eq_ignore_ascii_case("G0") && !cmd.eq_ignore_ascii_case("G1") {
        return false;
    }
    let mut has_e = false;
    let mut has_xyz = false;
    for f in fields {
        let f_upper = f.to_uppercase();
        if f_upper.starts_with('E') {
            has_e = true;
        } else if f_upper.starts_with('X') || f_upper.starts_with('Y') || f_upper.starts_with('Z') {
            has_xyz = true;
        }
    }
    has_e && !has_xyz
}

/// `replaceM140Temp`：把行内所有 `(?i)S\d+` 替换为 `S<newTemp>`。
fn replace_m140_temp(line: &str, new_temp: i64) -> String {
    let bytes = line.as_bytes();
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'S' || bytes[i] == b's' {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j > i + 1 {
                out.push_str(&format!("S{new_temp}"));
                i = j;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// `ShouldPreserveNozzleLoadLineLine`（helpers.go:474）。
fn should_preserve_nozzle_load_line_line(line: &str) -> (String, bool) {
    let trimmed = line.trim();
    if trimmed.starts_with(';') {
        return (line.to_string(), true);
    }
    let code_part = code_before_comment(trimmed);
    if code_part.is_empty() {
        return (line.to_string(), true);
    }
    let mut fields = code_part.split_whitespace();
    let Some(cmd) = fields.next() else {
        return (line.to_string(), true);
    };
    if cmd.eq_ignore_ascii_case("M109") {
        let modified = replace_cmd_prefix(line, "M109", "M104");
        let modified = replace_s_numbers(&modified, 30);
        return (modified, true);
    }
    if cmd.eq_ignore_ascii_case("M104") {
        let modified = replace_s_numbers(line, 30);
        return (modified, true);
    }
    if cmd.eq_ignore_ascii_case("G0")
        || cmd.eq_ignore_ascii_case("G1")
        || cmd.eq_ignore_ascii_case("G00")
        || cmd.eq_ignore_ascii_case("G01")
    {
        for f in fields {
            let f_upper = f.to_uppercase();
            if f_upper.starts_with('X')
                || f_upper.starts_with('Y')
                || f_upper.starts_with('Z')
                || f_upper.starts_with('E')
            {
                if f_upper.len() > 1 {
                    let c = f_upper.as_bytes()[1];
                    if c == b'-' || c == b'.' || c.is_ascii_digit() {
                        return (String::new(), false);
                    }
                }
            }
        }
        return (line.to_string(), true);
    }
    (line.to_string(), true)
}

/// 行首 M109 → M104（大小写不敏感；Go `(?i)^M109` 锚在原始行首）。
fn replace_cmd_prefix(line: &str, from: &str, to: &str) -> String {
    if line.len() >= from.len() && line[..from.len()].eq_ignore_ascii_case(from) {
        format!("{to}{}", &line[from.len()..])
    } else {
        line.to_string()
    }
}

/// 行内所有 `(?i)S\d+` → `S<n>`（与 replaceM140Temp 同一正则的手写扫描器）。
fn replace_s_numbers(line: &str, new_val: i64) -> String {
    let bytes = line.as_bytes();
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'S' || bytes[i] == b's' {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j > i + 1 {
                out.push_str(&format!("S{new_val}"));
                i = j;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn is_model_skeleton_line(trimmed: &str) -> bool {
    if has_slicer_comment_prefix(trimmed, "FEATURE:") && !trimmed.starts_with("; FEATURE: Custom") {
        return true;
    }
    if has_slicer_comment_prefix(trimmed, "LINE_WIDTH:") {
        return true;
    }
    if trimmed.starts_with(";WIPE_START")
        || trimmed.starts_with("; WIPE_START")
        || trimmed.starts_with(";WIPE_END")
        || trimmed.starts_with("; WIPE_END")
    {
        return true;
    }
    let code_part = code_before_comment(trimmed);
    if code_part.is_empty() {
        return false;
    }
    let mut fields = code_part.split_whitespace();
    let Some(cmd) = fields.next() else {
        return false;
    };
    let cmd = cmd.to_uppercase();
    if cmd == "M204" || cmd == "G17" || cmd == "G017" {
        return true;
    }
    if cmd == "M73" {
        let mut has_p = false;
        let mut has_l = false;
        for f in fields {
            let f_up = f.to_uppercase();
            if f_up.starts_with('P') {
                has_p = true;
            }
            if f_up.starts_with('L') {
                has_l = true;
            }
        }
        if has_p && !has_l {
            return true;
        }
    }
    false
}

/// `isModelMotionLine`（helpers.go:317）：G0-G3 且带 X/Y/Z/E(/I/J) 数值参数。
fn is_model_motion_line(line: &str) -> bool {
    let trimmed = line.trim();
    let code_part = code_before_comment(trimmed);
    if code_part.is_empty() {
        return false;
    }
    let upper: String = code_part.to_uppercase();
    let (g_code, params_start): (String, usize) = if upper.starts_with("G0") {
        split_g_code(code_part, &upper, 0)
    } else if upper.starts_with("G1") {
        split_g_code(code_part, &upper, 1)
    } else if upper.starts_with("G2") {
        split_g_code(code_part, &upper, 2)
    } else if upper.starts_with("G3") {
        split_g_code(code_part, &upper, 3)
    } else {
        return false;
    };
    let is_g0g1 = g_code == "G0" || g_code == "G1" || g_code == "G00" || g_code == "G01";
    let is_g2g3 = g_code == "G2" || g_code == "G3" || g_code == "G02" || g_code == "G03";
    if !is_g0g1 && !is_g2g3 {
        return false;
    }
    let params = &code_part[params_start..];
    let pb = params.as_bytes();
    for i in 0..pb.len() {
        let c = pb[i].to_ascii_uppercase() as char;
        let motion_axis = c == 'X' || c == 'Y' || c == 'Z' || c == 'E';
        let arc_axis = is_g2g3 && (c == 'I' || c == 'J');
        if motion_axis || arc_axis {
            if i + 1 < pb.len() {
                let next = pb[i + 1];
                if next == b'-' || next == b'.' || next.is_ascii_digit() {
                    return true;
                }
            }
        }
    }
    false
}

fn split_g_code(code_part: &str, upper: &str, digit: u8) -> (String, usize) {
    let cb = code_part.as_bytes();
    let ch = char::from(digit + b'0');
    if cb.len() >= 3 && cb[2].is_ascii_digit() {
        // "G" + 两位数字码（如 G01 → "G01"）
        (format!("G{}", &upper[1..3]), 3)
    } else {
        (format!("G{ch}"), 2)
    }
}
