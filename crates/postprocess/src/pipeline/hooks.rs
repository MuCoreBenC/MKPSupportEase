//! write 步的三个收尾 Hooks（对照 gcodesvc 的 startup_common.go /
//! begin_stage_retract*.go / bed_glue.go 的调用面）。
//!
//! 偏好默认值与 Go preferences/service.go:165-180 逐字一致：
//! `SkipVibrationCalibration=true / BeginStageRetractEnabled=true /
//! BeginStageRetractZLiftMm=20 / BedGlueEnabled=false / GlueBottomEnabled=false`。
//! Phase 1 CLI 无 preferences 通道， hooks 恒按默认值执行。
//!
//! bed_glue.go（607 行生成器）默认关且无任何字节参考 —— 刻意未移植本体，
//! 钩子点保留为 no-op（登记于 HONEST-BOUNDARIES）。

use crate::gcode::{format_python_float, has_e_param};
use crate::ir::Ir;

/// `applySkipVibrationCalibration`（startup_common.go:218）：删除 M970*/M974 行。
pub fn apply_skip_vibration_calibration(lines: Vec<String>, skip: bool) -> Vec<String> {
    if !skip {
        return lines;
    }
    lines
        .into_iter()
        .filter(|line| {
            let trimmed = line.trim();
            let code_part = code_before_comment(trimmed);
            if code_part.is_empty() {
                return true;
            }
            match code_part.split_whitespace().next() {
                Some(cmd) => {
                    let cmd = cmd.to_uppercase();
                    !(cmd.starts_with("M970") || cmd == "M974")
                }
                None => true,
            }
        })
        .collect()
}

fn code_before_comment(trimmed: &str) -> &str {
    match trimmed.find(';') {
        Some(idx) => trimmed[..idx].trim(),
        None => trimmed,
    }
}

/// P1/X1 共用收笔 G-code（begin_stage_retract_custom.go:8，从 p1mkp 样本提取）。
const BEGIN_STAGE_RETRACT_P1X1: &[&str] = &[
    "M290 X40 Y40 Z2.6666666",
    "G90",
    "G28 X Y",
    "G1 F10000",
    "G1 X250 Y216",
    "G1 X256",
    "G1 Y215",
    "G1 F6000",
    "G1 Y200",
    "G1 F12000",
    "G1 X250",
    "G91",
];

/// A1 mini 收笔 G-code。
const BEGIN_STAGE_RETRACT_A1_MINI: &[&str] =
    &["G28 X", "G1 X191 F10000", "G1 X20 F10000", "G1 F5000"];

/// A1 收笔 G-code。
const BEGIN_STAGE_RETRACT_A1: &[&str] = &["G28 X", "G1 X273 F10000", "G1 X20 F10000", "G1 F5000"];

fn custom_begin_stage_retract_gcode(machine_type: &str) -> Option<&'static [&'static str]> {
    match machine_type {
        "P1S" | "X1C" => Some(BEGIN_STAGE_RETRACT_P1X1),
        "A1_MINI" => Some(BEGIN_STAGE_RETRACT_A1_MINI),
        "A1" => Some(BEGIN_STAGE_RETRACT_A1),
        _ => None,
    }
}

/// `filterUnmountGcodeLines`（startup_common.go:253，pass1 规则复用）。
fn filter_unmount_gcode_lines(unmount_lines: &[&str], fan_speed: f64) -> Vec<String> {
    let mut result = Vec::with_capacity(unmount_lines.len());
    for ul in unmount_lines {
        let ul = ul.trim();
        if ul.is_empty() || ul.contains(";Wipe") || ul.contains(";Brush") {
            continue;
        }
        if ul.contains("M106 S[AUTO]") {
            result.push(format!("M106 S{}", format_python_float(fan_speed)));
            continue;
        }
        if ul.contains("M106 P1 S[AUTO]") {
            result.push(format!("M106 P1 S{}", format_python_float(fan_speed)));
            continue;
        }
        if has_e_param(ul) {
            continue;
        }
        result.push(ul.to_string());
    }
    result
}

/// `extractZHomingSequence`（startup_common.go:309）：动态提取 Z 归零前置序列。
fn extract_z_homing_sequence(lines: &[String]) -> Option<Vec<String>> {
    let z_home_idx = lines
        .iter()
        .position(|l| l.trim().starts_with("G28 Z P0 T"))?;

    let mut g0_lines: Vec<String> = Vec::new();
    let lower_bound = z_home_idx.saturating_sub(20);
    for i in (lower_bound..z_home_idx).rev() {
        let trimmed = lines[i].trim();
        if trimmed.starts_with("G0 X") || trimmed.starts_with("G0 Y") {
            g0_lines.insert(0, lines[i].clone());
        } else if !g0_lines.is_empty() {
            break;
        }
    }
    if g0_lines.is_empty() {
        return None;
    }

    let g28_line = lines[lower_bound..z_home_idx]
        .iter()
        .rev()
        .find(|l| {
            let t = l.trim();
            t == "G28 X" || t == "G28 X Y"
        })
        .cloned();

    let mut seq: Vec<String> = Vec::new();
    if let Some(g) = g28_line {
        seq.push(g);
    }
    seq.extend(g0_lines);
    seq.push(lines[z_home_idx].clone());
    Some(seq)
}

/// `buildBeginStageRetractBlock`（startup_common.go:366）。
fn build_begin_stage_retract_block(
    unmount_lines: &[&str],
    fan_speed: f64,
    homing_seq: Option<Vec<String>>,
    z_lift: i64,
) -> Vec<String> {
    if unmount_lines.is_empty() {
        return Vec::new();
    }
    let filtered = filter_unmount_gcode_lines(unmount_lines, fan_speed);
    if filtered.is_empty() {
        return Vec::new();
    }
    let mut block = vec![";===== begin stage retract =================".to_string()];
    if let Some(seq) = homing_seq {
        block.extend(seq);
    }
    let z_lift = if !(20..=100).contains(&z_lift) {
        20
    } else {
        z_lift
    };
    block.extend([
        "G91".to_string(),
        "G1 Z10 F600".to_string(),
        "G90".to_string(),
        format!("G1 Z{z_lift} F600"),
    ]);
    block.extend(filtered);
    block.push(";===== begin stage retract end =============".to_string());
    block
}

/// `applyBeginStageRetractToFile`（service.go:456，内存行版）：
/// 第一个 "; FEATURE: Custom" 前插入收笔块；机型未定义则跳过。
pub fn apply_begin_stage_retract(
    lines: Vec<String>,
    ir_data: &Ir,
    enabled: bool,
    z_lift_mm: i64,
) -> Vec<String> {
    if !enabled {
        return lines;
    }
    let toml_machine = ir_data.machine.machine_type.as_str();
    let Some(unmount_lines) = custom_begin_stage_retract_gcode(toml_machine) else {
        tracing::warn!(machine = toml_machine, "机型未定义收笔 G-code，跳过收笔块");
        return lines;
    };
    let homing_seq = extract_z_homing_sequence(&lines);
    let block = build_begin_stage_retract_block(
        unmount_lines,
        ir_data.wiping.fan_speed,
        homing_seq,
        z_lift_mm,
    );
    if block.is_empty() {
        return lines;
    }
    let insert_idx = lines
        .iter()
        .position(|l| l.contains("; FEATURE: Custom"))
        .unwrap_or(0);
    let mut result = Vec::with_capacity(lines.len() + block.len());
    result.extend_from_slice(&lines[..insert_idx]);
    result.extend(block);
    result.extend_from_slice(&lines[insert_idx..]);
    result
}

/// `ApplyPrePrintBedGlue`：BedGlueEnabled 默认 false —— 钩子点保留，本体未移植
/// （默认关 + 无字节参考，见模块文档）。
pub fn apply_pre_print_bed_glue(lines: Vec<String>, _ir_data: &Ir) -> Vec<String> {
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn skip_vibration_removes_m970_m974() {
        let out = apply_skip_vibration_calibration(
            lines(&["M970 S1", "M974 X1", "M104 S220", "G1 X1 ; M970 in comment"]),
            true,
        );
        assert_eq!(out, lines(&["M104 S220", "G1 X1 ; M970 in comment"]));
        // skip=false 原样
        let out = apply_skip_vibration_calibration(lines(&["M970 S1"]), false);
        assert_eq!(out, lines(&["M970 S1"]));
    }

    #[test]
    fn homing_sequence_extracted() {
        let content = lines(&["G28 X", "G0 X100 Y100", "G28 Z P0 T12345", "G1 X1 Y1"]);
        let seq = extract_z_homing_sequence(&content).unwrap();
        assert_eq!(seq, lines(&["G28 X", "G0 X100 Y100", "G28 Z P0 T12345"]));
    }

    #[test]
    fn retract_block_inserted_before_custom() {
        let mut ir = Ir::default();
        ir.machine.machine_type = "A1_MINI".to_string();
        ir.wiping.fan_speed = 255.0;
        let content = lines(&["; HEADER", "; FEATURE: Custom", "G1 X1"]);
        let out = apply_begin_stage_retract(content, &ir, true, 20);
        let idx_retract = out
            .iter()
            .position(|l| l.contains("begin stage retract ===="))
            .unwrap();
        let idx_custom = out
            .iter()
            .position(|l| l.contains("; FEATURE: Custom"))
            .unwrap();
        assert!(idx_retract < idx_custom);
        assert!(out.iter().any(|l| l == "G1 Z20 F600"));
    }

    #[test]
    fn retract_unknown_machine_skips() {
        let mut ir = Ir::default();
        ir.machine.machine_type = "P2S".to_string();
        let content = lines(&["; FEATURE: Custom"]);
        let out = apply_begin_stage_retract(content, &ir, true, 20);
        assert_eq!(out.len(), 1);
    }
}
