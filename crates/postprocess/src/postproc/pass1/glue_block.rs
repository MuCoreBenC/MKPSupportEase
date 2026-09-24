//! 涂胶块发射（对照 processor/glue_block.go + pass1_emit.go 的 Z 补偿与微抬）。

// 逐字对照 glue_block.go：嵌套 if 与 Go 同形（涂胶块的分支链本身就近百行）。
#![allow(
    clippy::collapsible_if,
    clippy::needless_range_loop,
    clippy::too_many_lines,
    clippy::manual_clamp
)]

use std::collections::HashMap;

use super::ZKey;

use crate::diag::PostprocError;
use crate::gcode::{
    format_e, format_float, format_python_float, format_speed, format_z, get_e_value, has_e_param,
    replace_e_value,
};
use crate::ir::{Ir, num_strip};
use crate::postproc::marks;

use super::LineSink;
use super::model_iface::{IfaceBbox, split_iface_by_model, total_path_length};
use super::scan::{find_seg_start_cmd, parse_xy_from_g1, strip_z_and_e};
use super::stages::{MkpStage, stage_comment_line};

/// Z 补偿包围盒每侧外扩量（pass1_emit.go ZCompExpandMM，±2mm/侧）。
const Z_COMP_EXPAND_MM: f64 = 2.0;

/// `calcGlueZ`：按 4 角双线性插值计算 (x, y) 处的涂胶 Z 补偿量。
pub(crate) fn calc_glue_z(
    base_z: f64,
    x: f64,
    y: f64,
    ir_data: &Ir,
    patch_bbox: Option<IfaceBbox>,
) -> f64 {
    if !ir_data.glue.z_comp_enabled {
        return base_z;
    }
    let (corners, mut bbox) = if ir_data.glue.z_comp_mode == "patch" {
        match patch_bbox {
            Some(b) => (&ir_data.glue.z_comp_patch, b),
            None => (
                &ir_data.glue.z_comp_bed,
                IfaceBbox {
                    min_x: ir_data.machine.min_x,
                    max_x: ir_data.machine.max_x,
                    min_y: ir_data.machine.min_y,
                    max_y: ir_data.machine.max_y,
                },
            ),
        }
    } else {
        (
            &ir_data.glue.z_comp_bed,
            IfaceBbox {
                min_x: ir_data.machine.min_x,
                max_x: ir_data.machine.max_x,
                min_y: ir_data.machine.min_y,
                max_y: ir_data.machine.max_y,
            },
        )
    };
    // 全 0 快速返回（常见「未配置补偿」场景）
    if corners.front_left == 0.0
        && corners.front_right == 0.0
        && corners.back_left == 0.0
        && corners.back_right == 0.0
    {
        return base_z;
    }
    // 外扩 2mm（每侧 +2，总 +4）
    bbox.min_x -= Z_COMP_EXPAND_MM;
    bbox.max_x += Z_COMP_EXPAND_MM;
    bbox.min_y -= Z_COMP_EXPAND_MM;
    bbox.max_y += Z_COMP_EXPAND_MM;
    let dx = bbox.max_x - bbox.min_x;
    let dy = bbox.max_y - bbox.min_y;
    if dx == 0.0 || dy == 0.0 {
        return base_z;
    }
    let mut xn = (x - bbox.min_x) / dx;
    let mut yn = (y - bbox.min_y) / dy;
    if xn < 0.0 {
        xn = 0.0;
    } else if xn > 1.0 {
        xn = 1.0;
    }
    if yn < 0.0 {
        yn = 0.0;
    } else if yn > 1.0 {
        yn = 1.0;
    }
    // 双线性插值（yn=0 → Front，yn=1 → Back）
    let comp = corners.front_left * (1.0 - xn) * (1.0 - yn)
        + corners.front_right * xn * (1.0 - yn)
        + corners.back_left * (1.0 - xn) * yn
        + corners.back_right * xn * yn;
    base_z + comp
}

/// `applyMicroLift`：段间跳转 / 支撑面微抬（pass1_emit.go:88）。
fn apply_micro_lift(
    sink: &mut LineSink,
    jump_cmd: &str,
    ir_data: &Ir,
    current_layer_height: f64,
    last_layer_height: f64,
) -> Result<(), PostprocError> {
    let lift_height = ir_data.glue.inter_model_lift_height;
    sink.push_str(&format!(
        "G1 F{}",
        format_speed(ir_data.machine.travel_speed * super::MM_PER_MINUTE)
    ));
    sink.push_str(&format!(
        "G1 Z{}",
        format_z(current_layer_height + ir_data.toolhead.z_offset + lift_height)
    ));
    let processed = crate::postproc::offset::process_gcode_offset(
        jump_cmd,
        ir_data.toolhead.x_offset,
        ir_data.toolhead.y_offset,
        ir_data.toolhead.z_offset + lift_height,
        crate::gcode::OffsetMode::Normal,
        ir_data,
    )?;
    sink.push_str(&processed);
    sink.push_str(&format!(
        "G1 Z{}",
        format_z(last_layer_height + ir_data.toolhead.z_offset + ir_data.glue.z_offset)
    ));
    sink.push_str(&format!(
        "G1 F{}",
        format_speed(ir_data.toolhead.max_speed * ir_data.wiping.small_feature_factor)
    ));
    Ok(())
}

/// `writeGlueLines`（pass1_emit.go:120）：发射一个段的涂胶移动。
/// 注意：稀疏化分支会**修改** `lines`（Go 同名行为，调用方负责传拷贝）。
fn write_glue_lines(
    sink: &mut LineSink,
    lines: &mut Vec<String>,
    ir_data: &Ir,
    current_layer_height: f64,
    last_layer_height: f64,
) -> Result<(), PostprocError> {
    use crate::gcode::MARKER_MICRO_LIFT;
    use crate::gcode::{MARKER_ZJUMP_INTERNAL, MARKER_ZJUMP_START};

    if ir_data.machine.nozzle_diameter <= 0.3 && ir_data.glue.sparse_ratio > 0.0 {
        let mut move_indices: Vec<usize> = Vec::new();
        for (i, l) in lines.iter().enumerate() {
            let tl = l.trim();
            if tl.starts_with("G1 ")
                && !tl.contains("G1 F")
                && (tl.contains('X') || tl.contains('Y'))
            {
                move_indices.push(i);
            }
        }
        if move_indices.len() > 2 {
            let step = 100.0 / ir_data.glue.sparse_ratio;
            let mut keep_set = std::collections::HashSet::new();
            let mut k = 0.0f64;
            while k < move_indices.len() as f64 {
                let mut idx = k as usize;
                if idx >= move_indices.len() {
                    idx = move_indices.len() - 1;
                }
                keep_set.insert(move_indices[idx]);
                k += step;
            }
            keep_set.insert(move_indices[0]);
            let last = *move_indices.last().unwrap();
            keep_set.insert(last);
            for &mi in move_indices.iter().rev() {
                if !keep_set.contains(&mi) {
                    lines.remove(mi);
                }
            }
        }
    }

    // Patch 模式 Z 补偿 bbox 预计算（每层一个总 bbox 的降级策略）
    let mut layer_bbox: Option<IfaceBbox> = None;
    if ir_data.glue.z_comp_enabled && ir_data.glue.z_comp_mode == "patch" {
        let mut bx = IfaceBbox {
            min_x: f64::MAX,
            max_x: f64::MIN,
            min_y: f64::MAX,
            max_y: f64::MIN,
        };
        for l in lines.iter() {
            let tl = l.trim();
            if !tl.starts_with("G1 ") {
                continue;
            }
            let (x, y, ok) = parse_xy_from_g1(tl);
            if !ok {
                continue;
            }
            if tl.contains('X') {
                bx.min_x = bx.min_x.min(x);
                bx.max_x = bx.max_x.max(x);
            }
            if tl.contains('Y') {
                bx.min_y = bx.min_y.min(y);
                bx.max_y = bx.max_y.max(y);
            }
        }
        if bx.max_x > bx.min_x && bx.max_y > bx.min_y {
            layer_bbox = Some(bx);
        }
    }

    // prevX/prevY 维护当前涂胶笔位置（G1 可能只含单轴）
    let mut prev_x = 0.0;
    let mut prev_y = 0.0;

    let mut idx = 0usize;
    while idx < lines.len() {
        let il = lines[idx].trim().to_string();
        if il.contains(MARKER_ZJUMP_START) {
            let mut next_start_idx: isize = -1;
            for ni in idx + 1..lines.len() {
                if lines[ni].contains("G1 X") || lines[ni].contains("G1 Y") {
                    next_start_idx = ni as isize;
                    break;
                }
            }
            if next_start_idx < 0 {
                idx += 1;
                continue;
            }
            let nsi = next_start_idx as usize;
            if ir_data.glue.inter_model_lift_height > 0.0 {
                sink.push_str(&format!(
                    "G1 F{}",
                    format_speed(ir_data.machine.travel_speed * super::MM_PER_MINUTE)
                ));
                sink.push_str(&format!(
                    "G1 Z{}",
                    format_z(
                        current_layer_height
                            + ir_data.toolhead.z_offset
                            + ir_data.glue.inter_model_lift_height
                    )
                ));
                let jump_cmd = strip_z_and_e(lines[nsi].trim());
                let processed = crate::postproc::offset::process_gcode_offset(
                    &jump_cmd,
                    ir_data.toolhead.x_offset,
                    ir_data.toolhead.y_offset,
                    ir_data.toolhead.z_offset + ir_data.glue.inter_model_lift_height,
                    crate::gcode::OffsetMode::Normal,
                    ir_data,
                )?;
                sink.push_str(&processed);
                sink.push_str(&format!(
                    "G1 Z{}",
                    format_z(last_layer_height + ir_data.toolhead.z_offset + ir_data.glue.z_offset)
                ));
                sink.push_str(&format!(
                    "G1 F{}",
                    format_speed(ir_data.toolhead.max_speed * ir_data.wiping.small_feature_factor)
                ));
            } else {
                let jump_cmd = strip_z_and_e(lines[nsi].trim());
                if jump_cmd.contains('X') || jump_cmd.contains('Y') {
                    apply_micro_lift(
                        sink,
                        &jump_cmd,
                        ir_data,
                        current_layer_height,
                        last_layer_height,
                    )?;
                }
            }
            idx = nsi + 1;
            continue;
        }
        if il.contains(MARKER_ZJUMP_INTERNAL) {
            let feature_idx = il.find("; FEATURE:").or_else(|| il.find(";FEATURE:"));
            if let Some(fi) = feature_idx {
                sink.push_str(&format!(";Feature transition: {}", &il[fi..]));
            }
            let mut next_start_idx: isize = -1;
            for ni in idx + 1..lines.len() {
                if lines[ni].contains("G1 X") || lines[ni].contains("G1 Y") {
                    next_start_idx = ni as isize;
                    break;
                }
            }
            if next_start_idx < 0 {
                idx += 1;
                continue;
            }
            let nsi = next_start_idx as usize;
            let jump_cmd = strip_z_and_e(lines[nsi].trim());
            if jump_cmd.contains('X') || jump_cmd.contains('Y') {
                apply_micro_lift(
                    sink,
                    &jump_cmd,
                    ir_data,
                    current_layer_height,
                    last_layer_height,
                )?;
            }
            idx = nsi + 1;
            continue;
        }
        if il.contains(MARKER_MICRO_LIFT) {
            let mut next_start_idx: isize = -1;
            for ni in idx + 1..lines.len() {
                if lines[ni].contains("G1 X") || lines[ni].contains("G1 Y") {
                    next_start_idx = ni as isize;
                    break;
                }
            }
            if next_start_idx < 0 {
                idx += 1;
                continue;
            }
            let nsi = next_start_idx as usize;
            let jump_cmd = strip_z_and_e(lines[nsi].trim());
            if jump_cmd.contains('X') || jump_cmd.contains('Y') {
                apply_micro_lift(
                    sink,
                    &jump_cmd,
                    ir_data,
                    current_layer_height,
                    last_layer_height,
                )?;
            }
            idx = nsi + 1;
            continue;
        }
        if il.starts_with("G1 ") && !il.contains("G1 F") {
            let mut glue_line = il.clone();
            if glue_line.contains(" E") {
                let e_idx = glue_line.find(" E").unwrap();
                let after_e = &glue_line[e_idx + 2..];
                let mut end_of_e = 0;
                for (ci, ch) in after_e.bytes().enumerate() {
                    if ch == b' ' || ch == b';' {
                        end_of_e = ci;
                        break;
                    }
                }
                if end_of_e > 0 {
                    glue_line = format!("{}{}", &glue_line[..e_idx], &after_e[end_of_e..]);
                } else {
                    glue_line = glue_line[..e_idx].to_string();
                }
            }
            if glue_line.contains('X') || glue_line.contains('Y') {
                let (x, y, ok) = parse_xy_from_g1(&glue_line);
                if ok {
                    if glue_line.contains('X') {
                        prev_x = x;
                    }
                    if glue_line.contains('Y') {
                        prev_y = y;
                    }
                }
                let comp_z = calc_glue_z(0.0, prev_x, prev_y, ir_data, layer_bbox);
                let processed = crate::postproc::offset::process_gcode_offset(
                    &glue_line,
                    ir_data.toolhead.x_offset,
                    ir_data.toolhead.y_offset,
                    ir_data.toolhead.z_offset + ir_data.glue.z_offset + comp_z,
                    crate::gcode::OffsetMode::Normal,
                    ir_data,
                )?;
                sink.push_str(&processed);
            }
        } else if il.contains("M204") {
            sink.push_str(&il);
        }
        idx += 1;
    }
    Ok(())
}

/// `customMountGcodeHasRetract`（glue_block.go:471）：custom_mount_gcode 含负 E 指令。
pub(crate) fn custom_mount_gcode_has_retract(ir_data: &Ir) -> bool {
    ir_data
        .toolhead
        .custom_mount_gcode
        .iter()
        .any(|line| get_e_value(line).is_some_and(|v| v < 0.0))
}

/// `customUnmountGcodeHasExtrude`（glue_block.go:483；pass2 的 prepare-next-tower 消费）。
pub(crate) fn custom_unmount_gcode_has_extrude(ir_data: &Ir) -> bool {
    ir_data
        .toolhead
        .custom_unmount_gcode
        .iter()
        .any(|line| get_e_value(line).is_some_and(|v| v > 0.0))
}

/// `mountToolhead`（glue_block.go:376）：custom_mount_gcode 切换下笔。
pub(crate) fn mount_toolhead(
    sink: &mut LineSink,
    ir_data: &Ir,
    current_layer_height: f64,
    last_layer_height: f64,
    skip_final_z: bool,
) {
    sink.push_str(";Mounting Toolhead");
    sink.push_str(&stage_comment_line(MkpStage::MountToolhead, ""));
    for mount_line in &ir_data.toolhead.custom_mount_gcode {
        let ml = mount_line.trim();
        if ml.is_empty() {
            continue;
        }
        if ml.contains("L801") {
            let mut l801_z = current_layer_height + ir_data.toolhead.z_offset + 3.0;
            if current_layer_height < 3.0 {
                l801_z = current_layer_height + ir_data.toolhead.z_offset + 4.0;
            }
            sink.push_str(&format!("G1 Z{};L801", format_z(l801_z)));
        } else if !ir_data.gcode.custom_gcode_e_ownership && has_e_param(ml) {
            sink.push_str(&replace_e_value(ml, ir_data.gcode.mkp_retract));
        } else {
            sink.push_str(ml);
        }
    }
    sink.push_str(";Toolhead Mounted");
    if !skip_final_z {
        sink.push_str(&format!(
            "G1 Z{}",
            format_z(last_layer_height + ir_data.toolhead.z_offset + 3.0)
        ));
    }
}

/// `unmountToolhead`（glue_block.go:413）：;Wipe/;Brush 跳过 + [AUTO] 风扇替换。
pub(crate) fn unmount_toolhead(sink: &mut LineSink, ir_data: &Ir) {
    sink.push_str(";Unmounting Toolhead");
    for unmount_line in &ir_data.toolhead.custom_unmount_gcode {
        let ul = unmount_line.trim();
        if ul.is_empty() {
            continue;
        }
        if ul.contains(";Wipe") {
            // 跳过
        } else if ul.contains(";Brush") {
            // 跳过
        } else if ul.contains("M106 S[AUTO]") {
            sink.push_str(&format!(
                "M106 S{}",
                format_python_float(ir_data.wiping.fan_speed)
            ));
        } else if ul.contains("M106 P1 S[AUTO]") {
            sink.push_str(&format!(
                "M106 P1 S{}",
                format_python_float(ir_data.wiping.fan_speed)
            ));
        } else if !ir_data.gcode.custom_gcode_e_ownership && has_e_param(ul) {
            sink.push_str(&replace_e_value(ul, ir_data.gcode.mkp_extrude));
        } else {
            sink.push_str(ul);
        }
    }
    sink.push_str(";Toolhead Unmounted");
}

/// `restoreFanState`（glue_block.go:455）：恢复 MKP 插入前的原始风扇状态。
pub(crate) fn restore_fan_state(sink: &mut LineSink, ir_data: &Ir, machine_type: &str) {
    if !ir_data.state.current_fan_speed_set {
        return;
    }
    sink.push_str(";Restore fan state");
    let fan_speed = format_python_float(ir_data.state.current_fan_speed);
    if crate::postproc::machine_dims::get_machine_dimensions(machine_type)
        .flags
        .has_second_fan
    {
        sink.push_str(&format!("M106 P1 S{fan_speed}"));
    } else {
        sink.push_str(&format!("M106 S{fan_speed}"));
    }
}

/// `emitGlueBlock`（glue_block.go:27）：生成完整涂胶块（换笔→涂胶→擦料→准备下一塔）。
///
/// 副作用：修改 ir_data.state.this_layer_revitalization 与 stats 计数字段；
/// write_glue_lines 可能修改传入的 iface（稀疏化），调用方按 Go 语义传拷贝。
#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_glue_block(
    sink: &mut LineSink,
    iface: &[String],
    ir_data: &mut Ir,
    current_layer_height: f64,
    last_layer_height: f64,
    machine_type: &str,
    _layer_feature_map: &HashMap<ZKey, bool>,
    stats: &mut super::ProcessStats,
) -> Result<(), PostprocError> {
    // MKP 标记协议 v1：本块的事件序号。glue_cutter_changes 在下面 +1，这里先读，
    // 三个区间共享同一个值。最终 ev 会在 marks::finalize 里按落盘行序重编号。
    let ev = stats.glue_cutter_changes + 1;

    sink.push_str(";Pre-glue preparation");
    // swap#1（取笔）。END 必须在 ;Toolhead Mounted 附近闭合：pass2 的
    // skipping_mount_sequence 会丢弃这一整段，标记靠 marks::is_mark_line 穿透。
    sink.push_str(&marks::swap_begin(ev, "pen", current_layer_height));

    if ir_data.filament.filament_type.to_uppercase() != "ABS" {
        let fan_speed = format_python_float(ir_data.wiping.fan_speed);
        if crate::postproc::machine_dims::get_machine_dimensions(machine_type)
            .flags
            .has_second_fan
        {
            sink.push_str(&format!("M106 P1 S{fan_speed}"));
        } else {
            sink.push_str(&format!("M106 S{fan_speed}"));
        }
    }

    let wipe_x = crate::postproc::machine_dims::get_machine_dimensions(machine_type)
        .glue_area
        .wipe_x;
    if custom_mount_gcode_has_retract(ir_data) {
        sink.push_str(&format!(
            "G1 X{} Z{} F{}",
            format_float(wipe_x),
            format_z(current_layer_height + 1.0),
            format_speed(ir_data.machine.travel_speed * super::MM_PER_MINUTE)
        ));
    } else {
        sink.push_str(&format!(
            "G1 X{} Z{} E{} F{}",
            format_float(wipe_x),
            format_z(current_layer_height + 1.0),
            format_e(ir_data.gcode.mkp_retract),
            format_speed(ir_data.machine.travel_speed * super::MM_PER_MINUTE)
        ));
    }

    if ir_data.wiping.nozzle_cooling {
        sink.push_str(";Pervent Leakage");
        sink.push_str(&format!(
            "M104 S{:.0}",
            ir_data.toolhead.nozzle_switch_temperature - 30.0
        ));
    }

    sink.push_str(crate::gcode::MARKER_RISING_NOZZLE);
    sink.push_str(&stage_comment_line(MkpStage::RiseNozzle, ""));
    let mut lift_height = ir_data.glue.z_lift_height;
    if current_layer_height < 3.0 {
        lift_height = ir_data.glue.z_lift_height_first_layers;
    }
    sink.push_str(&format!(
        "G1 Z{}",
        format_z(current_layer_height + ir_data.toolhead.z_offset + lift_height)
    ));

    mount_toolhead(
        sink,
        ir_data,
        current_layer_height,
        last_layer_height,
        false,
    );
    sink.push_str(&marks::end("swap"));

    sink.push_str(";Glueing Started");
    sink.push_str(&stage_comment_line(MkpStage::GlueStart, ""));
    sink.push_str(&marks::paint_begin(ev, current_layer_height));
    let segments = split_iface_by_model(iface);

    stats.glue_cutter_changes += 1;
    stats.glue_layer_count += 1;
    for seg in &segments {
        stats.glue_total_length += total_path_length(seg) * ir_data.glue.pass_count as f64;
    }

    for (seg_idx, seg) in segments.iter().enumerate() {
        // Go 侧此处先累计 segESum，但该值从未被消费（死存储），未移植。

        let seg_start_cmd = find_seg_start_cmd(seg);

        if !seg_start_cmd.is_empty() {
            let repos_cmd = strip_z_and_e(&seg_start_cmd);
            if seg_idx == 0 {
                sink.push_str(&format!(
                    "G1 F{}",
                    format_speed(ir_data.machine.travel_speed * super::MM_PER_MINUTE)
                ));
                let offset_result = crate::postproc::offset::process_gcode_offset(
                    &repos_cmd,
                    ir_data.toolhead.x_offset,
                    ir_data.toolhead.y_offset,
                    ir_data.toolhead.z_offset,
                    crate::gcode::OffsetMode::Normal,
                    ir_data,
                )?;
                sink.push_str(&offset_result);
                sink.push_str(&format!(
                    "G1 Z{}",
                    format_z(last_layer_height + ir_data.toolhead.z_offset + ir_data.glue.z_offset)
                ));
                sink.push_str(&format!(
                    "G1 F{}",
                    format_speed(ir_data.toolhead.max_speed * ir_data.wiping.small_feature_factor)
                ));
            } else if ir_data.glue.inter_model_lift_height > 0.0 {
                sink.push_str(&format!(
                    "G1 F{}",
                    format_speed(ir_data.machine.travel_speed * super::MM_PER_MINUTE)
                ));
                sink.push_str(&format!(
                    "G1 Z{}",
                    format_z(
                        current_layer_height
                            + ir_data.toolhead.z_offset
                            + ir_data.glue.inter_model_lift_height
                    )
                ));
                let offset_result = crate::postproc::offset::process_gcode_offset(
                    &repos_cmd,
                    ir_data.toolhead.x_offset,
                    ir_data.toolhead.y_offset,
                    ir_data.toolhead.z_offset + ir_data.glue.inter_model_lift_height,
                    crate::gcode::OffsetMode::Normal,
                    ir_data,
                )?;
                sink.push_str(&offset_result);
                sink.push_str(&format!(
                    "G1 Z{}",
                    format_z(last_layer_height + ir_data.toolhead.z_offset + ir_data.glue.z_offset)
                ));
                sink.push_str(&format!(
                    "G1 F{}",
                    format_speed(ir_data.toolhead.max_speed * ir_data.wiping.small_feature_factor)
                ));
            } else {
                sink.push_str(&format!(
                    "G1 F{}",
                    format_speed(ir_data.machine.travel_speed * super::MM_PER_MINUTE)
                ));
                let offset_result = crate::postproc::offset::process_gcode_offset(
                    &repos_cmd,
                    ir_data.toolhead.x_offset,
                    ir_data.toolhead.y_offset,
                    ir_data.toolhead.z_offset,
                    crate::gcode::OffsetMode::Normal,
                    ir_data,
                )?;
                sink.push_str(&offset_result);
                sink.push_str(&format!(
                    "G1 F{}",
                    format_speed(ir_data.toolhead.max_speed * ir_data.wiping.small_feature_factor)
                ));
            }
        }

        if ir_data.prime.enabled
            && ir_data.state.this_layer_revitalization
            && (ir_data.prime.min_path_length == 0.0
                || total_path_length(seg) >= ir_data.prime.min_path_length)
        {
            sink.push_str(";Gluepen Revitalization Start");
            sink.push_str(&stage_comment_line(MkpStage::RevitalizationStart, ""));
            let revitalization_speed =
                ir_data.toolhead.max_speed * ir_data.wiping.small_feature_factor;
            sink.push_str(&format!("G1 F{}", format_speed(revitalization_speed)));
            let mut target_length = ir_data.prime.length;
            if target_length < 20.0 {
                target_length = 100.0;
            }
            let prime_iface = seg;
            let mut accumulated_dist = 0.0;
            let mut last_rx = f64::NAN;
            let mut last_ry = f64::NAN;
            for iface_line in prime_iface {
                let il = iface_line.trim();
                if il.contains(crate::gcode::MARKER_ZJUMP_START) {
                    break;
                }
                if !il.starts_with("G1 ") || il.contains("G1 F") {
                    continue;
                }
                let mut revit_line = il.to_string();
                if revit_line.contains(" E") {
                    let e_idx = revit_line.find(" E").unwrap();
                    let after_e = &revit_line[e_idx + 2..];
                    let mut end_of_e = 0;
                    for (ci, ch) in after_e.bytes().enumerate() {
                        if ch == b' ' || ch == b';' {
                            end_of_e = ci;
                            break;
                        }
                    }
                    if end_of_e > 0 {
                        revit_line = format!("{}{}", &revit_line[..e_idx], &after_e[end_of_e..]);
                    } else {
                        revit_line = revit_line[..e_idx].to_string();
                    }
                }
                if !revit_line.contains('X') && !revit_line.contains('Y') {
                    continue;
                }
                let mut rx = 0.0;
                let mut ry = 0.0;
                let mut has_x = false;
                let mut has_y = false;
                if let Some(x_idx) = revit_line.find('X') {
                    let mut x_end = x_idx + 1;
                    let rb = revit_line.as_bytes();
                    while x_end < rb.len() && rb[x_end] != b' ' && rb[x_end] != b';' {
                        x_end += 1;
                    }
                    let mut x_part = revit_line[x_idx..x_end].to_string();
                    if x_part.len() > 1 && x_part.as_bytes()[1] == b'.' {
                        x_part = format!("{}0{}", &x_part[..1], &x_part[1..]);
                    }
                    let x_nums = num_strip(&x_part);
                    if let Some(&n) = x_nums.first() {
                        rx = n;
                        has_x = true;
                    }
                }
                if let Some(y_idx) = revit_line.find('Y') {
                    let mut y_end = y_idx + 1;
                    let rb = revit_line.as_bytes();
                    while y_end < rb.len() && rb[y_end] != b' ' && rb[y_end] != b';' {
                        y_end += 1;
                    }
                    let mut y_part = revit_line[y_idx..y_end].to_string();
                    if y_part.len() > 1 && y_part.as_bytes()[1] == b'.' {
                        y_part = format!("{}0{}", &y_part[..1], &y_part[1..]);
                    }
                    let y_nums = num_strip(&y_part);
                    if let Some(&n) = y_nums.first() {
                        ry = n;
                        has_y = true;
                    }
                }
                if has_x && has_y {
                    if !last_rx.is_nan() && !last_ry.is_nan() {
                        let dx = rx - last_rx;
                        let dy = ry - last_ry;
                        accumulated_dist += (dx * dx + dy * dy).sqrt();
                    }
                    last_rx = rx;
                    last_ry = ry;
                    if accumulated_dist >= target_length {
                        break;
                    }
                    let processed = crate::postproc::offset::process_gcode_offset(
                        &revit_line,
                        ir_data.toolhead.x_offset,
                        ir_data.toolhead.y_offset,
                        ir_data.toolhead.z_offset + ir_data.glue.z_offset,
                        crate::gcode::OffsetMode::Normal,
                        ir_data,
                    )?;
                    sink.push_str(&processed);
                }
            }
            sink.push_str(";Gluepen Revitalization End");
            sink.push_str(&stage_comment_line(MkpStage::RevitalizationEnd, ""));
            stats.prime_total_length += accumulated_dist;
            if !ir_data.prime.per_model {
                ir_data.state.this_layer_revitalization = false;
            }
            let revit_repos_cmd = strip_z_and_e(&seg_start_cmd);
            if ir_data.glue.inter_model_lift_height > 0.0 {
                sink.push_str(&format!(
                    "G1 Z{}",
                    format_z(
                        current_layer_height
                            + ir_data.toolhead.z_offset
                            + ir_data.glue.inter_model_lift_height
                    )
                ));
                sink.push_str(&format!(
                    "G1 F{}",
                    format_speed(ir_data.machine.travel_speed * super::MM_PER_MINUTE)
                ));
                let offset_result = crate::postproc::offset::process_gcode_offset(
                    &revit_repos_cmd,
                    ir_data.toolhead.x_offset,
                    ir_data.toolhead.y_offset,
                    ir_data.toolhead.z_offset + ir_data.glue.inter_model_lift_height,
                    crate::gcode::OffsetMode::Normal,
                    ir_data,
                )?;
                sink.push_str(&offset_result);
                sink.push_str(&format!(
                    "G1 Z{}",
                    format_z(last_layer_height + ir_data.toolhead.z_offset + ir_data.glue.z_offset)
                ));
                sink.push_str(&format!(
                    "G1 F{}",
                    format_speed(ir_data.toolhead.max_speed * ir_data.wiping.small_feature_factor)
                ));
            } else {
                sink.push_str(&format!(
                    "G1 F{}",
                    format_speed(ir_data.machine.travel_speed * super::MM_PER_MINUTE)
                ));
                let offset_result = crate::postproc::offset::process_gcode_offset(
                    &revit_repos_cmd,
                    ir_data.toolhead.x_offset,
                    ir_data.toolhead.y_offset,
                    ir_data.toolhead.z_offset,
                    crate::gcode::OffsetMode::Normal,
                    ir_data,
                )?;
                sink.push_str(&offset_result);
                sink.push_str(&format!(
                    "G1 F{}",
                    format_speed(ir_data.toolhead.max_speed * ir_data.wiping.small_feature_factor)
                ));
            }
        }

        let mut seg_owned = seg.clone();
        write_glue_lines(
            sink,
            &mut seg_owned,
            ir_data,
            current_layer_height,
            last_layer_height,
        )?;

        for pass_idx in 1..ir_data.glue.pass_count {
            sink.push_str(&format!(";Glue Pass {}", pass_idx + 1));
            sink.push_str(&stage_comment_line(
                MkpStage::GluePass,
                &format!("{}", pass_idx + 1),
            ));
            let inpos_cmd = strip_z_and_e(&seg_start_cmd);
            if ir_data.glue.inter_model_lift_height > 0.0 {
                sink.push_str(&format!(
                    "G1 F{}",
                    format_speed(ir_data.machine.travel_speed * super::MM_PER_MINUTE)
                ));
                sink.push_str(&format!(
                    "G1 Z{}",
                    format_z(
                        current_layer_height
                            + ir_data.toolhead.z_offset
                            + ir_data.glue.inter_model_lift_height
                    )
                ));
                let offset_result = crate::postproc::offset::process_gcode_offset(
                    &inpos_cmd,
                    ir_data.toolhead.x_offset,
                    ir_data.toolhead.y_offset,
                    ir_data.toolhead.z_offset + ir_data.glue.inter_model_lift_height,
                    crate::gcode::OffsetMode::Normal,
                    ir_data,
                )?;
                sink.push_str(&offset_result);
                sink.push_str(&format!(
                    "G1 Z{}",
                    format_z(last_layer_height + ir_data.toolhead.z_offset + ir_data.glue.z_offset)
                ));
                sink.push_str(&format!(
                    "G1 F{}",
                    format_speed(ir_data.toolhead.max_speed * ir_data.wiping.small_feature_factor)
                ));
            } else {
                sink.push_str(&format!(
                    "G1 F{}",
                    format_speed(ir_data.machine.travel_speed * super::MM_PER_MINUTE)
                ));
                let offset_result = crate::postproc::offset::process_gcode_offset(
                    &inpos_cmd,
                    ir_data.toolhead.x_offset,
                    ir_data.toolhead.y_offset,
                    ir_data.toolhead.z_offset,
                    crate::gcode::OffsetMode::Normal,
                    ir_data,
                )?;
                sink.push_str(&offset_result);
                sink.push_str(&format!(
                    "G1 F{}",
                    format_speed(ir_data.toolhead.max_speed * ir_data.wiping.small_feature_factor)
                ));
            }
            write_glue_lines(
                sink,
                &mut seg_owned,
                ir_data,
                current_layer_height,
                last_layer_height,
            )?;
        }

        if !ir_data.prime.per_model {
            ir_data.state.this_layer_revitalization = false;
        }
    }

    sink.push_str(";Glueing Finished");
    sink.push_str(&stage_comment_line(MkpStage::GlueEnd, ""));
    sink.push_str(&format!(
        "G1 F{}",
        format_speed(ir_data.machine.travel_speed * super::MM_PER_MINUTE)
    ));
    let mut glue_finished_z =
        current_layer_height + ir_data.toolhead.z_offset + ir_data.glue.z_lift_height;
    if current_layer_height < 3.0 {
        glue_finished_z = current_layer_height
            + ir_data.toolhead.z_offset
            + ir_data.glue.z_lift_height_first_layers;
    }
    sink.push_str(&format!("G1 Z{}", format_z(glue_finished_z)));
    sink.push_str(&format!(";Lift-z:{}", format_z(glue_finished_z)));
    sink.push_str(&marks::end("paint"));

    // swap#2（放笔）。END 必须在 ;Prepare for next tower 之前 —— pass2 会在那之后
    // 灌入延迟的圆盘擦拭段（;DISK_WIPE_START/END 是别名区间），否则会嵌套。
    sink.push_str(&marks::swap_begin(ev, "print", current_layer_height));
    unmount_toolhead(sink, ir_data);
    restore_fan_state(sink, ir_data, machine_type);
    sink.push_str(&marks::end("swap"));

    if ir_data.tower.use_towers {
        let tower_bbox = crate::postproc::tower::calculate_tower_bbox(
            &ir_data.wiping.outer_structure,
            ir_data.sheath.base_expand,
            ir_data.rib.extra_length,
            ir_data.rib.width,
            stats.max_z_height,
            ir_data.machine.nozzle_diameter,
            ir_data.machine.first_layer_height,
        );
        let offset_result = crate::postproc::offset::process_gcode_offset(
            "G1 X20 Y10.19",
            ir_data.wiping.wiper_x - tower_bbox.min_x,
            ir_data.wiping.wiper_y - tower_bbox.min_y,
            current_layer_height + 3.0,
            crate::gcode::OffsetMode::Normal,
            ir_data,
        )?;
        sink.push_str(&offset_result);
        sink.push_str(";Prepare for next tower");
        if ir_data.wiping.nozzle_cooling {
            if ir_data.wiping.user_dry_time != 0 {
                sink.push_str(&format!(
                    "M104 S{:.0}",
                    ir_data.toolhead.nozzle_switch_temperature
                ));
            } else {
                sink.push_str(&format!(
                    "M109 S{:.0}",
                    ir_data.toolhead.nozzle_switch_temperature
                ));
            }
        }
    } else {
        sink.push_str(";Prepare for next tower");
        if !ir_data.gcode.refill_extrude_command.is_empty() {
            let mut cmd = ir_data.gcode.refill_extrude_command.clone();
            if !ir_data.gcode.custom_gcode_e_ownership {
                cmd = replace_e_value(&cmd, ir_data.gcode.mkp_extrude);
            }
            sink.push_str(&cmd);
            sink.push_str(&format!(
                "G1 F{}",
                format_speed(ir_data.machine.travel_speed * super::MM_PER_MINUTE)
            ));
        }
    }

    Ok(())
}
