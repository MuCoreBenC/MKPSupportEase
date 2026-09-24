//! 斜肋（Diagonal Rib）生成（对照 tower/rib.go 逐行复刻）。

use super::constants::*;
use super::geometry::{
    Point, find_closest_point_index, get_limit_depth_by_height, polygon_offset, polygon_walk,
    remove_duplicate_points, rib_section,
};
use crate::gcode::{format_e, format_speed};

#[allow(clippy::too_many_arguments)]
pub fn generate_rib_gcode(
    layer_count: i64,
    current_z_height: f64,
    layer_height: f64,
    first_layer_height: f64,
    tower_height: f64,
    wiper_x: f64,
    wiper_y: f64,
    travel_speed: f64,
    wipe_tower_print_speed: f64,
    retract_length: f64,
    nozzle_diameter: f64,
    rib_extra_length: f64,
    rib_width: f64,
    rib_fillet_wall: bool,
    first_layer_flow_ratio: f64,
    rib_bottom_fill_style: &str,
    bed_center_x: f64,
    bed_center_y: f64,
    next_model_z: f64,
    safe_z_offset: f64,
    bbox_min_x: f64,
    bbox_min_y: f64,
) -> Option<Vec<String>> {
    let _ = rib_bottom_fill_style; // 旧侧签名携带但生成路径未消费（逐字保持）
    if rib_width <= 0.0 || tower_height <= 0.0 {
        return None;
    }

    let width = TOWER_OUTER_BOUND - TOWER_INNER_BOUND;
    let depth = TOWER_OUTER_BOUND - TOWER_INNER_BOUND;
    let diagonal = (width * width + depth * depth).sqrt();
    let mut rib_length = diagonal.max(diagonal + rib_extra_length);
    let min_depth = get_limit_depth_by_height(tower_height);
    rib_length = rib_length.max(min_depth * (2.0f64).sqrt());
    rib_length = diagonal.max(rib_length);
    let rib_width = rib_width.min(width.min(depth) / 2.0);

    let max_extra_len = (0.0f64).max(rib_length - diagonal) / 2.0;
    let extra_len = (tower_height - current_z_height).abs() / tower_height * max_extra_len;

    let x_off = wiper_x - bbox_min_x;
    let y_off = wiper_y - bbox_min_y;

    let lh = if layer_count == 0 {
        first_layer_height
    } else {
        layer_height
    };

    let line_width = nozzle_diameter * LINE_WIDTH_MULTIPLIER;
    let filament_area = std::f64::consts::PI * FILAMENT_RADIUS * FILAMENT_RADIUS;

    let is_first_layer = layer_count == 0;
    generate_rib_paths(
        width,
        depth,
        rib_width,
        extra_len,
        rib_length,
        rib_fillet_wall,
        x_off,
        y_off,
        current_z_height,
        lh,
        line_width,
        filament_area,
        travel_speed,
        wipe_tower_print_speed,
        retract_length,
        first_layer_flow_ratio,
        is_first_layer,
        layer_count,
        bed_center_x,
        bed_center_y,
        next_model_z,
        safe_z_offset,
    )
}

/// `calcBrimLoops`：brim/chamfer 圈数（非首层随 layerCount 递减收敛）。
pub fn calc_brim_loops(spacing: f64, is_first_layer: bool, layer_count: i64) -> i64 {
    let mut brim_loops = ((RIB_BRIM_WIDTH + spacing / 2.0) / spacing) as i64;
    if !is_first_layer {
        let chamfer_loops = (RIB_MAX_CHAMFER_WIDTH / spacing) as i64;
        brim_loops = brim_loops.min(chamfer_loops) - layer_count;
    }
    if brim_loops < 0 {
        brim_loops = 0;
    }
    brim_loops
}

// Go 源码原样带着若干「赋值后未读」的 last_x/last_y 写回（Go 编译器不警告）。
// 逐行复刻保留它们以维持与旧实现的行级对应，不做「顺手清理」。
#[allow(unused_assignments, clippy::let_and_return)]
#[allow(clippy::too_many_arguments)]
fn generate_rib_paths(
    width: f64,
    depth: f64,
    rib_width: f64,
    extra_len: f64,
    rib_length: f64,
    rib_fillet_wall: bool,
    x_off: f64,
    y_off: f64,
    current_z_height: f64,
    lh: f64,
    line_width: f64,
    filament_area: f64,
    travel_speed: f64,
    wipe_tower_print_speed: f64,
    retract_length: f64,
    first_layer_flow_ratio: f64,
    is_first_layer: bool,
    layer_count: i64,
    bed_center_x: f64,
    bed_center_y: f64,
    next_model_z: f64,
    safe_z_offset: f64,
) -> Option<Vec<String>> {
    let mut spacing = line_width - lh * (1.0 - std::f64::consts::PI / 4.0);
    if spacing < 0.1 {
        spacing = 0.1;
    }

    let mut flow_multiplier = 1.0;
    if layer_count == 0 {
        flow_multiplier = first_layer_flow_ratio;
    }

    let brim_loops = calc_brim_loops(spacing, is_first_layer, layer_count);
    let num_loops = 1 + brim_loops as usize;

    let diag = (width * width + depth * depth).sqrt();
    let current_rib_length = diag + 2.0 * extra_len;

    let outer_poly = rib_section(width, depth, current_rib_length, rib_width, rib_fillet_wall);

    if outer_poly.len() < 3 {
        return None;
    }

    let mut loops: Vec<Vec<Point>> = Vec::with_capacity(num_loops);

    let mut current_poly = polygon_offset(&outer_poly, spacing / 2.0);
    current_poly = remove_duplicate_points(&current_poly, 0.01);
    for i in 0..num_loops {
        if i > 0 {
            current_poly = polygon_offset(&current_poly, spacing);
            current_poly = remove_duplicate_points(&current_poly, 0.01);
            if current_poly.len() < 3 {
                break;
            }
        }
        loops.push(current_poly.clone());
    }

    if loops.is_empty() {
        return None;
    }

    let offset_x = TOWER_INNER_BOUND + x_off;
    let offset_y = TOWER_INNER_BOUND + y_off;

    // 智能起点：找最靠近床中心的多边形顶点（从面向打印区的一侧进入/离开）
    let local_center_x = bed_center_x - offset_x;
    let local_center_y = bed_center_y - offset_y;
    let closest_idx = find_closest_point_index(&loops[0], local_center_x, local_center_y);
    if closest_idx > 0 {
        for lp in loops.iter_mut() {
            let n = lp.len();
            if closest_idx < n {
                let mut rotated = vec![Point::default(); n];
                for (j, r) in rotated.iter_mut().enumerate() {
                    *r = lp[(j + closest_idx) % n];
                }
                *lp = rotated;
            }
        }
    }

    // 离开方向：塔中心 → 床中心
    let tower_center_x = width / 2.0;
    let tower_center_y = depth / 2.0;
    let mut exit_dx = local_center_x - tower_center_x;
    let mut exit_dy = local_center_y - tower_center_y;
    let exit_len = (exit_dx * exit_dx + exit_dy * exit_dy).sqrt();
    if exit_len > 0.001 {
        exit_dx /= exit_len;
        exit_dy /= exit_len;
    } else {
        exit_dx = 1.0;
        exit_dy = 0.0;
    }

    let (print_speed, print_accel, travel_accel) = if layer_count < TOWER_SPEED_LIMIT_LAYERS {
        (wipe_tower_print_speed.min(3000.0), 500.0, 6000.0)
    } else {
        (wipe_tower_print_speed.min(5400.0), 6000.0, 10000.0)
    };

    let mut lines: Vec<String> = Vec::new();
    lines.push("; === Diagonal Rib Start ===".to_string());
    lines.push(format!(
        "; RibLength: {rib_length:.3}mm RibWidth: {rib_width:.3}mm ExtraLen: {extra_len:.3}mm"
    ));
    lines.push(format!("; LINE_WIDTH: {line_width:.3}"));
    lines.push(format!("M204 S{}", travel_accel as i64));
    lines.push(format!("G1 F{}", format_speed(travel_speed)));
    lines.push(format!(
        "G1 Z{:.3} ;Safe Z",
        current_z_height.max(next_model_z) + safe_z_offset
    ));

    let extrusion_per_mm = lh * spacing / filament_area * flow_multiplier;

    let start_pt = loops[0][0];
    let machine_x = start_pt.x + offset_x;
    let machine_y = start_pt.y + offset_y;
    lines.push(format!("G1 X{machine_x:.3} Y{machine_y:.3}"));
    lines.push(format!("G1 Z{current_z_height:.3} ;Print Z"));
    lines.push(format!(
        "G1 E{} F{}",
        format_e(retract_length),
        RETRACT_SPEED
    ));
    lines.push(format!("M204 S{}", print_accel as i64));
    lines.push(format!("G1 F{}", format_speed(print_speed)));

    for (loop_idx, lp) in loops.iter().enumerate() {
        if loop_idx > 0 {
            lines.push(format!("M204 S{}", travel_accel as i64));
            let start_pt = lp[0];
            let mx = start_pt.x + offset_x;
            let my = start_pt.y + offset_y;
            lines.push(format!("G1 X{mx:.3} Y{my:.3}"));
            lines.push(format!("M204 S{}", print_accel as i64));
        }

        for i in 1..lp.len() {
            let dx = lp[i].x - lp[i - 1].x;
            let dy = lp[i].y - lp[i - 1].y;
            let seg_length = (dx * dx + dy * dy).sqrt();
            let seg_e = seg_length * extrusion_per_mm;
            let mx = lp[i].x + offset_x;
            let my = lp[i].y + offset_y;
            lines.push(format!("G1 X{mx:.3} Y{my:.3} E{seg_e:.5}"));
        }

        let closing_dx = lp[0].x - lp[lp.len() - 1].x;
        let closing_dy = lp[0].y - lp[lp.len() - 1].y;
        let closing_len = (closing_dx * closing_dx + closing_dy * closing_dy).sqrt();
        if closing_len > 0.01 {
            let closing_e = closing_len * extrusion_per_mm;
            let mx = lp[0].x + offset_x;
            let my = lp[0].y + offset_y;
            lines.push(format!("G1 X{mx:.3} Y{my:.3} E{closing_e:.5}"));
        }
    }

    let last_loop = &loops[loops.len() - 1];
    let mut last_pt = last_loop[0];
    lines.push("; === Diagonal Rib End ===".to_string());

    let wipe_pts = polygon_walk(last_loop, 0, TOWER_WIPE_DISTANCE);
    if !wipe_pts.is_empty() {
        lines.push("; WIPE_START".to_string());
        lines.push(format!("G1 F{TOWER_WIPE_SPEED}"));
        lines.push(format!("M204 S{TOWER_WIPE_ACCEL}"));
        let mut total_wipe_len = 0.0;
        let mut prev_pt = last_pt;
        for p in &wipe_pts {
            let dx = p.x - prev_pt.x;
            let dy = p.y - prev_pt.y;
            total_wipe_len += dx.hypot(dy);
            prev_pt = *p;
        }
        if total_wipe_len > 0.0 {
            let mut cum_len = 0.0;
            let mut prev_pt = last_pt;
            let mut prev_e = 0.0;
            for p in &wipe_pts {
                let dx = p.x - prev_pt.x;
                let dy = p.y - prev_pt.y;
                let seg_len = dx.hypot(dy);
                cum_len += seg_len;
                let cur_e = -retract_length * cum_len / total_wipe_len;
                let seg_e = cur_e - prev_e;
                let mx = p.x + offset_x;
                let my = p.y + offset_y;
                lines.push(format!("G1 X{mx:.3} Y{my:.3} E{seg_e:.5}"));
                prev_pt = *p;
                prev_e = cur_e;
            }
            last_pt = wipe_pts[wipe_pts.len() - 1];
        }
        lines.push("; WIPE_END".to_string());
    }

    lines.push(format!(
        "G1 E-{TOWER_FINAL_RETRACT:.5} F{TOWER_FINAL_RETRACT_SPEED}"
    ));
    lines.push(format!("M204 S{}", travel_accel as i64));
    lines.push("G17".to_string());
    lines.push(format!(
        "G3 Z{:.3} I{SPIRAL_LIFT_RADIUS:.3} J0 P1 F{SPIRAL_LIFT_SPEED}",
        current_z_height + SPIRAL_LIFT_Z_HEIGHT
    ));

    let exit_x = last_pt.x + exit_dx * WIPE_EXIT_OFFSET + offset_x;
    let exit_y = last_pt.y + exit_dy * WIPE_EXIT_OFFSET + offset_y;
    lines.push(format!(
        "G1 X{exit_x:.3} Y{exit_y:.3} F{}",
        format_speed(travel_speed)
    ));

    let _ = start_pt; // Go 侧 startPt 后续未再读取（保持变量存在以对照）
    Some(lines)
}
