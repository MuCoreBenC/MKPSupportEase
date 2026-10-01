//! 首笔活化（对照 processor/revitalization_block.go 逐行移植）。
//!
//! pass2 第二个 CHANGE_LAYER 触发（首层全部 feature 打完后）；
//! 校准模式强制禁用。四象限小螺旋复用 tower::generate_mini_spiral_gcode。

use crate::diag::PostprocError;
use crate::gcode::{format_float, format_speed, format_z};
use crate::ir::Ir;

use crate::postproc::pass1::{LineSink, MM_PER_MINUTE, ProcessStats};
use crate::postproc::pass1::{calc_glue_z, mount_toolhead, restore_fan_state, unmount_toolhead};
use crate::postproc::tower;

/// `emitFirstPenRevitalization`（revitalization_block.go:32）。
#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_first_pen_revitalization(
    sink: &mut LineSink,
    ir_data: &mut Ir,
    current_layer_height: f64,
    final_tower_height: f64,
    stats: Option<&mut ProcessStats>,
    has_following_glue_block: bool,
) -> Result<(), PostprocError> {
    sink.push_str(";First Pen Revitalization Start");

    // 1. 抬 Z 到安全高度（与 mountToolhead 的 L801 Z 一致）
    let mut safe_z = current_layer_height + ir_data.toolhead.z_offset + 3.0;
    if current_layer_height < 3.0 {
        safe_z = current_layer_height + ir_data.toolhead.z_offset + 4.0;
    }
    sink.push_str(&format!(
        "G1 F{}",
        format_speed(ir_data.machine.travel_speed * MM_PER_MINUTE)
    ));
    sink.push_str(&format!("G1 Z{}", format_z(safe_z)));

    // 2. 切换下笔（skipFinalZ=true：Z 已在安全高度）
    mount_toolhead(
        sink,
        ir_data,
        current_layer_height,
        current_layer_height,
        true,
    );

    // 3. 坐标转换
    let bbox = tower::calculate_tower_bbox(
        &ir_data.wiping.outer_structure,
        ir_data.sheath.base_expand,
        ir_data.rib.extra_length,
        ir_data.rib.width,
        final_tower_height,
        ir_data.machine.nozzle_diameter,
        ir_data.machine.first_layer_height,
    );
    let x_off = ir_data.wiping.wiper_x - bbox.min_x;
    let y_off = ir_data.wiping.wiper_y - bbox.min_y;

    // 4. 四角补偿 Z（wiper 中心，4 象限共用）
    let comp_z = calc_glue_z(
        0.0,
        ir_data.wiping.wiper_x,
        ir_data.wiping.wiper_y,
        ir_data,
        None,
    );

    // 5. 4 象限小螺旋（不挤出；skipInitialSafeZ=true）
    let mut mini_spiral_xy_lines: Vec<String> = Vec::new();
    for q_idx in 0..4i64 {
        let lines = tower::generate_mini_spiral_gcode(
            q_idx,
            current_layer_height,
            ir_data.wiping.wiper_x,
            ir_data.wiping.wiper_y,
            bbox.min_x,
            bbox.min_y,
            ir_data.machine.travel_speed * MM_PER_MINUTE,
            ir_data.machine.nozzle_diameter,
            ir_data.tower.safe_z_offset,
            ir_data.toolhead.x_offset,
            ir_data.toolhead.y_offset,
            ir_data.toolhead.z_offset,
            ir_data.glue.z_offset,
            comp_z,
            true,
        );
        if let Some(lines) = lines {
            for line in lines {
                sink.push_str(&line);
                let trimmed = line.trim();
                if trimmed.starts_with("G1 ") && trimmed.contains('X') && trimmed.contains('Y') {
                    mini_spiral_xy_lines.push(trimmed.to_string());
                }
            }
        }
    }

    // 6. 无后续涂胶块时：收笔 + 恢复风扇 + 移动到擦料塔打印入口
    if !has_following_glue_block {
        sink.push_str(&format!(
            "G1 F{}",
            format_speed(ir_data.machine.travel_speed * MM_PER_MINUTE)
        ));
        let mut revit_finished_z =
            current_layer_height + ir_data.toolhead.z_offset + ir_data.glue.z_lift_height;
        if current_layer_height < 3.0 {
            revit_finished_z = current_layer_height
                + ir_data.toolhead.z_offset
                + ir_data.glue.z_lift_height_first_layers;
        }
        sink.push_str(&format!("G1 Z{}", format_z(revit_finished_z)));
        sink.push_str(&format!(";Lift-z:{}", format_z(revit_finished_z)));

        let machine_type = ir_data.machine.machine_type.clone();
        unmount_toolhead(sink, ir_data);
        restore_fan_state(sink, ir_data, &machine_type);

        // 喷嘴坐标（不加工具头偏移；M017：禁止坐标回退）
        let entry_x = tower::TOWER_LAYER_ENTRY_X + x_off;
        let entry_y = tower::TOWER_LAYER_ENTRY_Y + y_off;
        sink.push_str(&format!(
            "G1 X{} Y{}",
            format_float(entry_x),
            format_float(entry_y)
        ));
    }

    // 范围检查：4 象限小螺旋机器坐标是否超出机型运动范围（警告不阻断）
    check_tower_path_range(
        ir_data,
        &mini_spiral_xy_lines,
        "first_pen_revitalization",
        "first_pen_revitalization_range_check",
        "首笔活化的路径超过工具头的最大运动范围",
    );

    sink.push_str(";First Pen Revitalization End");

    if let Some(stats) = stats {
        stats.glue_cutter_changes += 1;
        stats.glue_layer_count += 1;
    }

    Ok(())
}

/// `checkTowerPathRange`（revitalization_block.go:135）。
fn check_tower_path_range(
    ir_data: &mut Ir,
    gcode_lines: &[String],
    source: &'static str,
    operation: &str,
    source_label: &str,
) {
    // M017：机型维度未识别时跳过检测，不回退默认机型
    if ir_data.machine.max_x == 0.0
        && ir_data.machine.min_x == 0.0
        && ir_data.machine.max_y == 0.0
        && ir_data.machine.min_y == 0.0
    {
        return;
    }

    let movement_range = crate::gcode::MachineMovementRange {
        min_x: ir_data.machine.min_x,
        max_x: ir_data.machine.max_x,
        min_y: ir_data.machine.min_y,
        max_y: ir_data.machine.max_y,
    };

    let violations = crate::gcode::check_xy_range(
        gcode_lines,
        movement_range,
        source,
        ir_data.wiping.wiper_x,
        ir_data.wiping.wiper_y,
        ir_data.toolhead.x_offset,
        ir_data.toolhead.y_offset,
    );

    for v in violations {
        let msg = format_range_warning_message(&v, source_label);
        ir_data.warnings.push(msg.clone());
        ir_data
            .warning_diagnostics
            .push(crate::diag::Diagnostic::warning(operation, msg));
    }
}

/// `formatRangeWarningMessage`（revitalization_block.go:170）。
fn format_range_warning_message(v: &crate::gcode::RangeViolation, source_label: &str) -> String {
    let (dir_cn, limit_label, axis) = match v.direction {
        "X+" => ("X+", "机型最大 X", "X"),
        "X-" => ("X-", "机型最小 X", "X"),
        "Y+" => ("Y+", "机型最大 Y", "Y"),
        "Y-" => ("Y-", "机型最小 Y", "Y"),
        _ => ("X+", "机型最大 X", "X"),
    };
    // 坐标量级下 Rust `{}` 与 Go `%g` 输出一致（最短往返表示、无尾零）
    format!(
        "{source_label}，向 {dir_cn} 方向移动（实际 {axis}={}，{limit_label}={}）",
        v.actual_value, v.machine_limit
    )
}
