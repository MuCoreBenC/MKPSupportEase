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
///
/// TODO（2026-10-18 作者）：这个开关目前只在 **A1 / A1 mini** 上验证过 —— 只有
/// 这两家机器的切片器会往 G-code 里写 M970/M974 震动补偿校准序列。别的机型
/// （P1/X1 系列）用的命令集不一样，还没逐台核对：开关勾了之后该删什么、有没有
/// 对应的检测序列，以后适配一轮（逐机型实测产物，别只按命令名猜）。
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
                // 大小写不敏感比较（原实现每行一次 `to_uppercase` 堆分配）
                Some(cmd) => {
                    !(starts_with_ascii_ci(cmd, "m970") || cmd.eq_ignore_ascii_case("M974"))
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

/// ASCII 大小写不敏感的前缀判断（`prefix` 必须小写）。
fn starts_with_ascii_ci(s: &str, prefix: &str) -> bool {
    s.get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
}

/// P1S / X1C / P2S 共用收笔 G-code（`begin_stage_retract_custom.go:8`，从 p1mkp 样本提取）。
/// 收笔位在右后角：`G1 X250 Y216` → `X256` → `Y215` → `Y200` → `X250`。
/// P2S 还没有实物可验，先按 P1/X1 这一份走。
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

/// A1 mini 收笔 G-code：只动 X。`X191` 是右端收笔位（超出 180 行程，
/// 靠固件对收笔位的豁免），`X20` 退回喷嘴待命位。
///
/// X、Y 都要归零 —— 只归 X 时 Y 位置未知，走到收笔位会撞。
const BEGIN_STAGE_RETRACT_A1_MINI: &[&str] = &[
    "G28 X",
    "G28 Y",
    "G1 X191 F10000",
    "G1 X20 F10000",
    "G1 F5000",
];

/// A1 收笔 G-code：同 A1 mini，收笔位在 `X276`（行程 256，`273` 实测差一点碰不到）。
const BEGIN_STAGE_RETRACT_A1: &[&str] = &[
    "G28 X",
    "G28 Y",
    "G1 X276 F10000",
    "G1 X20 F10000",
    "G1 F5000",
];

fn custom_begin_stage_retract_gcode(machine_type: &str) -> Option<&'static [&'static str]> {
    match machine_type {
        "P1S" | "X1C" | "P2S" => Some(BEGIN_STAGE_RETRACT_P1X1),
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

/// 收笔块自己抬 Z 的合法区间（mm，**相对抬升**）。
///
/// 原厂口径（`startup_common.go:366`）：越界**不夹取，而是回落到下限** ——
/// 所以这里只提供区间，判据仍在 [`build_begin_stage_retract_block`] 里写"回落下限"。
/// 客户端那一份量程（`runtime::app_state::BEGIN_STAGE_RETRACT_Z_LIFT_*`）必须与这两个数
/// 一致，由 `runtime::postproc_prefs` 的测试咬住。
pub const BEGIN_STAGE_RETRACT_Z_LIFT_MIN_MM: i64 = 5;
pub const BEGIN_STAGE_RETRACT_Z_LIFT_MAX_MM: i64 = 30;

/// 收笔块本体。
///
/// `lift_mm` 是块首额外再抬的 Z（已回落到量程内）。用带堵转保护的 `G380 S2`
/// 而不是 `G1 Z`：`G1` 是无条件行走，Z 已经在行程顶端时照样往前送，步进电机
/// 会丢步「哒哒哒」；`G380` 碰顶就停。
fn build_begin_stage_retract_block(
    unmount_lines: &[&str],
    fan_speed: f64,
    lift_mm: i64,
) -> Vec<String> {
    if unmount_lines.is_empty() {
        return Vec::new();
    }
    let filtered = filter_unmount_gcode_lines(unmount_lines, fan_speed);
    if filtered.is_empty() {
        return Vec::new();
    }
    let mut block = vec![";===== begin stage retract =================".to_string()];
    block.extend([
        "G91".to_string(),
        format!("G380 S2 Z{lift_mm} F1200"),
        "G90".to_string(),
    ]);
    block.extend(filtered);
    block.push(";===== begin stage retract end =============".to_string());
    block
}

/// 量程回落：越界回落到下限，不夹取（原厂口径）。
fn clamp_lift(lift_mm: i64) -> i64 {
    if (BEGIN_STAGE_RETRACT_Z_LIFT_MIN_MM..=BEGIN_STAGE_RETRACT_Z_LIFT_MAX_MM).contains(&lift_mm) {
        lift_mm
    } else {
        BEGIN_STAGE_RETRACT_Z_LIFT_MIN_MM
    }
}

/// 抬 Z 段落的结束位置（插在这个下标之前 = 紧跟那条 `G90`）。
///
/// 原版起始段里 `G380 S2 Z40` / `G380 S3 Z-15` / `G90` 这一段负责把 Z 抬到床上方
/// 并校准，收尾是一条 `G90`。A1 在 `avoid end stop` 段，P1S 在 `reset machine status` 段。
fn z_lift_end_anchor(lines: &[String]) -> Option<usize> {
    let first = lines.iter().position(|l| l.trim().starts_with("G380"))?;
    lines[first..]
        .iter()
        .position(|l| l.trim() == "G90")
        .map(|offset| first + offset + 1)
}

/// 收笔块插入。
///
/// 插入点紧跟原版抬 Z 段落（`G380 … G90`）—— 那里 Z 已经在床上方、XY 归零动作
/// 由块内自带。找不到这段时退回 `; FEATURE: Custom` 之前。
pub fn apply_begin_stage_retract(
    lines: Vec<String>,
    ir_data: &Ir,
    enabled: bool,
    z_lift_mm: i64,
) -> Vec<String> {
    if !enabled {
        return lines;
    }
    /* 机型优先认 **G-code 自报的那一个**（`GcodeMachineType`）：两份可以不一样
     * （A1 配置切 A1 mini 出来的件），而收笔位是按物理行程定的 —— A1 mini 在
     * X191、A1 在 X276，认错就是撞机。
     *
     * 只有 G-code 里认不出来（空 / `UNKNOWN`）时才退回预设机型。认出来了但这个
     * 机型没有收笔位数据，就跳过并记日志，不拿另一个机型的位置顶替。 */
    let gcode_machine = ir_data.machine.gcode_machine_type.as_str();
    let preset_machine = ir_data.machine.machine_type.as_str();
    let detected = !gcode_machine.is_empty() && gcode_machine != "UNKNOWN";
    let unmount_lines = if detected {
        match custom_begin_stage_retract_gcode(gcode_machine) {
            Some(lines) => lines,
            None => {
                tracing::warn!(
                    machine = gcode_machine,
                    "G-code 机型没有收笔位数据，跳过收笔块"
                );
                return lines;
            }
        }
    } else {
        match custom_begin_stage_retract_gcode(preset_machine) {
            Some(lines) => lines,
            None => {
                tracing::warn!(
                    machine = preset_machine,
                    "预设机型没有收笔位数据，跳过收笔块"
                );
                return lines;
            }
        }
    };
    let insert_idx = z_lift_end_anchor(&lines).unwrap_or_else(|| {
        lines
            .iter()
            .position(|l| l.contains("; FEATURE: Custom"))
            .unwrap_or(0)
    });
    let block = build_begin_stage_retract_block(
        unmount_lines,
        ir_data.wiping.fan_speed,
        clamp_lift(z_lift_mm),
    );
    if block.is_empty() {
        return lines;
    }
    // **移动**而不是 `extend_from_slice`：后者对 `String` 是深拷贝整份行序列
    // （一份百万行级的输出就是一百万次分配）。
    let mut result = Vec::with_capacity(lines.len() + block.len());
    let mut tail = lines.into_iter();
    result.extend(tail.by_ref().take(insert_idx));
    result.extend(block);
    result.extend(tail);
    result
}

/* `apply_pre_print_bed_glue` 那个**空壳已删**（2026-10-17）：它一直"钩子点保留、
本体未移植"，于是设置页那一档勾了也没有任何效果。真实现现在在
[`super::bed_glue::apply_bed_glue`]，由 `pipeline/mod.rs` 的写步调用。 */

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    /// 原版起始段里抬 Z 的那一小截（`G380 … G1 Z5 … G90`）。
    fn z_lift_fixture() -> Vec<String> {
        lines(&[
            "; HEADER",
            "G91",
            "G380 S2 Z30 F1200",
            "G1 Z5 F1200",
            "G90",
            "M204 S6000",
        ])
    }

    fn ir_of(preset: &str, gcode: &str) -> Ir {
        let mut ir = Ir::default();
        ir.machine.machine_type = preset.to_string();
        ir.machine.gcode_machine_type = gcode.to_string();
        ir.wiping.fan_speed = 255.0;
        ir
    }

    /// 收笔位认 **G-code 机型**：配置写 A1、切出来的是 A1 mini，就必须用 X191。
    #[test]
    fn retract_prefers_gcode_machine_over_preset() {
        let out = apply_begin_stage_retract(z_lift_fixture(), &ir_of("A1", "A1_MINI"), true, 20);
        assert!(
            out.iter().any(|l| l == "G1 X191 F10000"),
            "应为 A1 mini 的收笔位"
        );
        assert!(
            !out.iter().any(|l| l == "G1 X276 F10000"),
            "不许用 A1 的收笔位"
        );
        // 反向同理
        let out = apply_begin_stage_retract(z_lift_fixture(), &ir_of("A1_MINI", "A1"), true, 20);
        assert!(out.iter().any(|l| l == "G1 X276 F10000"));
        assert!(!out.iter().any(|l| l == "G1 X191 F10000"));
    }

    /// G-code 里认不出机型（空 / UNKNOWN）才退回预设机型。
    #[test]
    fn retract_falls_back_to_preset_machine() {
        for unknown in ["", "UNKNOWN"] {
            let out =
                apply_begin_stage_retract(z_lift_fixture(), &ir_of("A1_MINI", unknown), true, 20);
            assert!(
                out.iter().any(|l| l == "G1 X191 F10000"),
                "gcode={unknown:?} 时应退回预设 A1_MINI"
            );
        }
    }

    /// 认出来了但这个机型没有收笔位数据 ⇒ 跳过，不拿别的机型位置顶替。
    #[test]
    fn retract_detected_machine_without_data_skips() {
        let out = apply_begin_stage_retract(z_lift_fixture(), &ir_of("A1_MINI", "H2D"), true, 20);
        assert!(!out.iter().any(|l| l.contains("begin stage retract")));
        assert_eq!(out, z_lift_fixture());
    }

    /// P2S 还没有实物，先按 P1/X1 那一份走。
    #[test]
    fn retract_p2s_shares_the_p1x1_sequence() {
        let out = apply_begin_stage_retract(z_lift_fixture(), &ir_of("P2S", "P2S"), true, 10);
        assert!(out.iter().any(|l| l == "G1 X256"), "{out:?}");
        assert!(out.iter().any(|l| l == "G28 X Y"));
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

    /// 块紧跟原版抬 Z 段（`G380 … G90`）之后，不等到预热与放音。
    #[test]
    fn retract_block_follows_the_z_lift_segment() {
        let content = lines(&[
            "; HEADER",
            "M18",
            ";=====avoid end stop =================",
            "G91",
            "G380 S2 Z40 F1200",
            "G380 S3 Z-15 F1200",
            "G90",
            ";===== reset machine status =================",
            "M204 S6000",
        ]);
        let out = apply_begin_stage_retract(content, &ir_of("A1", "A1"), true, 30);
        let pos = |needle: &str| out.iter().position(|l| l.contains(needle)).unwrap();
        let start = pos("begin stage retract ====");
        assert_eq!(out[start - 1], "G90", "必须紧跟抬 Z 段的收尾：{out:?}");
        assert!(start < pos("M204 S6000"));
    }

    /// 没有 `G380` 段落时退回 `; FEATURE: Custom` 之前。
    #[test]
    fn retract_falls_back_to_before_custom() {
        let content = lines(&["; HEADER", "M73 P0 R35", "; FEATURE: Custom", "G392 S0"]);
        let out = apply_begin_stage_retract(content, &ir_of("A1", "A1"), true, 30);
        let pos = |needle: &str| out.iter().position(|l| l.contains(needle)).unwrap();
        let start = pos("begin stage retract ====");
        assert_eq!(out[start - 1], "M73 P0 R35");
        assert!(start < pos("; FEATURE: Custom"));
    }

    /// A1 系列块：先按设置值抬一截，再 X、Y 一起归零。
    #[test]
    fn retract_lifts_then_homes_both_axes() {
        for machine in ["A1", "A1_MINI"] {
            let out =
                apply_begin_stage_retract(z_lift_fixture(), &ir_of(machine, machine), true, 5);
            let start = out
                .iter()
                .position(|l| l.contains("begin stage retract ===="))
                .unwrap();
            let end = out
                .iter()
                .position(|l| l.contains("begin stage retract end"))
                .unwrap();
            let block = &out[start..end];
            // 相对抬升用带堵转保护的 G380，不用 G1 Z（硬走）/ G28 Z（压床）
            assert!(
                block.iter().any(|l| l == "G380 S2 Z5 F1200"),
                "{machine}: {block:?}"
            );
            assert!(!block.iter().any(|l| l.starts_with("G1 Z")), "{machine}");
            assert!(!block.iter().any(|l| l.starts_with("G28 Z")), "{machine}");
            assert!(block.iter().any(|l| l == "G28 X"), "{machine}: {block:?}");
            assert!(block.iter().any(|l| l == "G28 Y"), "{machine}: {block:?}");
            // 抬升在归零之前
            let lift = block.iter().position(|l| l.starts_with("G380")).unwrap();
            let home = block.iter().position(|l| l == "G28 X").unwrap();
            assert!(lift < home, "{machine}");
        }
    }

    /// 开关关着 ⇒ 整块不插入，逐行原样返回。
    #[test]
    fn begin_stage_retract_disabled_inserts_nothing() {
        let content = z_lift_fixture();
        let out =
            apply_begin_stage_retract(content.clone(), &ir_of("A1_MINI", "A1_MINI"), false, 10);
        assert_eq!(out, content);
        assert!(!out.iter().any(|l| l.contains("begin stage retract")));
    }

    /// 起始段结构完全不认识时也照样插块（收笔是必须动作），插在文件头。
    #[test]
    fn retract_inserts_even_on_unknown_startup() {
        let content = lines(&["; HEADER", "G1 X1"]);
        let out = apply_begin_stage_retract(content, &ir_of("A1_MINI", "A1_MINI"), true, 30);
        assert!(out.iter().any(|l| l == "G1 X191 F10000"), "{out:?}");
        assert!(out.iter().any(|l| l == "G28 X"));
    }

    #[test]
    fn retract_unknown_machine_skips() {
        let mut ir = Ir::default();
        ir.machine.machine_type = "H2D".to_string();
        let content = lines(&["; FEATURE: Custom"]);
        let out = apply_begin_stage_retract(content, &ir, true, 20);
        assert_eq!(out.len(), 1);
    }
}
