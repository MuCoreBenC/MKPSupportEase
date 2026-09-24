//! 护套（Trapezoidal Sheath）生成（对照 tower/sheath.go 逐行复刻）。

use super::constants::*;
use super::geometry::{Point, polygon_walk};
use crate::gcode::{format_e, format_speed};

pub fn calculate_sheath_expansion(
    layer_count: i64,
    model_total_height: f64,
    outer_structure: &str,
    sheath_base_expand: f64,
    sheath_enable_height: f64,
    sheath_converge_layers: i64,
    nozzle_diameter: f64,
) -> f64 {
    // SSOT 语义：outerStructure 是 "brim"/"rib"/"sheath"。
    // layerCount==0 恒生成首层 brim；"sheath" 非首层按模型高度决定；其余非首层不生成。
    let should_generate = if layer_count == 0 {
        true
    } else {
        match outer_structure {
            "sheath" => model_total_height > sheath_enable_height,
            _ => false,
        }
    };
    if !should_generate {
        return 0.0;
    }
    if layer_count == 0 {
        return sheath_base_expand;
    }
    if outer_structure == "sheath" && model_total_height <= sheath_enable_height {
        return 0.0;
    }
    if layer_count >= sheath_converge_layers {
        return 0.0;
    }
    let expansion = sheath_base_expand * (1.0 - layer_count as f64 / sheath_converge_layers as f64);
    let min_expansion = nozzle_diameter * 0.5;
    if expansion < min_expansion {
        return 0.0;
    }
    expansion
}

// Go 源码原样带着若干「赋值后未读」的 last_x/last_y 写回（Go 编译器不警告）。
// 逐行复刻保留它们以维持与旧实现的行级对应，不做「顺手清理」。
#[allow(unused_assignments, clippy::let_and_return)]
#[allow(clippy::too_many_arguments)]
pub fn generate_sheath_gcode(
    layer_count: i64,
    current_z_height: f64,
    layer_height: f64,
    first_layer_height: f64,
    wiper_x: f64,
    wiper_y: f64,
    travel_speed: f64,
    wipe_tower_print_speed: f64,
    retract_length: f64,
    nozzle_diameter: f64,
    outer: &str,
    sheath_base_expand: f64,
    sheath_enable_height: f64,
    sheath_converge_layers: i64,
    sheath_wall_width: f64,
    model_total_height: f64,
    first_layer_flow_ratio: f64,
    next_model_z: f64,
    safe_z_offset: f64,
    bbox_min_x: f64,
    bbox_min_y: f64,
) -> Option<Vec<String>> {
    let _ = sheath_wall_width; // 旧侧签名携带但生成路径未消费（逐字保持）
    let expansion = calculate_sheath_expansion(
        layer_count,
        model_total_height,
        outer,
        sheath_base_expand,
        sheath_enable_height,
        sheath_converge_layers,
        nozzle_diameter,
    );
    if expansion == 0.0 {
        return None;
    }

    // 坐标转换：机器坐标 = 局部坐标 + (wiperX - bboxMinX, wiperY - bboxMinY)
    let x_off = wiper_x - bbox_min_x;
    let y_off = wiper_y - bbox_min_y;

    let tower_min = TOWER_INNER_BOUND;
    let tower_max = TOWER_OUTER_BOUND;

    let mut sheath_min_x = tower_min - expansion;
    let mut sheath_max_x = tower_max + expansion;
    let mut sheath_min_y = tower_min - expansion;
    let mut sheath_max_y = tower_max + expansion;

    if outer == "rib" && layer_count == 0 {
        let half_size = RIB_FIRST_LAYER_SHEATH_SIZE / 2.0;
        sheath_min_x = sheath_min_x.max(TOWER_CENTER_X - half_size);
        sheath_max_x = sheath_max_x.min(TOWER_CENTER_X + half_size);
        sheath_min_y = sheath_min_y.max(TOWER_CENTER_Y - half_size);
        sheath_max_y = sheath_max_y.min(TOWER_CENTER_Y + half_size);
    }

    let line_width = nozzle_diameter * LINE_WIDTH_MULTIPLIER;
    let filament_area = std::f64::consts::PI * FILAMENT_RADIUS * FILAMENT_RADIUS;

    let (print_accel, travel_accel) = if layer_count < TOWER_SPEED_LIMIT_LAYERS {
        (500.0, 6000.0)
    } else {
        (6000.0, 10000.0)
    };

    let mut lines: Vec<String> = Vec::new();
    lines.push("; === Trapezoidal Sheath Start ===".to_string());
    lines.push(format!("; Sheath Expansion: {expansion:.3}mm"));
    lines.push(format!("; LINE_WIDTH: {line_width:.3}"));
    lines.push(format!("M204 S{}", travel_accel as i64));
    lines.push(format!("G1 F{}", format_speed(travel_speed)));
    lines.push(format!(
        "G1 Z{:.3} ;Safe Z",
        current_z_height.max(next_model_z) + safe_z_offset
    ));

    let mut last_x;
    let mut last_y;

    let sheath_speed = wipe_tower_print_speed;

    if layer_count == 0 {
        let lh = first_layer_height;
        let spacing = line_width - lh * (1.0 - std::f64::consts::PI / 4.0);
        let cx = TOWER_CENTER_X;
        let cy = TOWER_CENTER_Y;
        last_x = cx;
        last_y = cy;
        lines.push(format!("G1 X{:.3} Y{:.3}", cx + x_off, cy + y_off));
        lines.push(format!("G1 Z{current_z_height:.3} ;Print Z"));
        lines.push(format!(
            "G1 E{} F{}",
            format_e(retract_length),
            RETRACT_SPEED
        ));
        lines.push(format!("M204 S{}", print_accel as i64));
        lines.push(format!("G1 F{}", format_speed(sheath_speed)));
        lines.push("G92 E0".to_string());

        let mut left = cx;
        let mut right = cx;
        let mut bottom = cy;
        let mut top = cy;

        loop {
            let mut seg_length = right - left;
            let mut seg_e = (seg_length * lh * spacing) / filament_area * first_layer_flow_ratio;
            last_x = right;
            lines.push(format!(
                "G1 X{:.3} Y{:.3} E{seg_e:.5}",
                last_x + x_off,
                last_y + y_off
            ));

            let new_bottom = (bottom - line_width).max(sheath_min_y);

            last_y = new_bottom;
            seg_length = top - new_bottom;
            seg_e = (seg_length * lh * spacing) / filament_area * first_layer_flow_ratio;
            lines.push(format!(
                "G1 X{:.3} Y{:.3} E{seg_e:.5}",
                last_x + x_off,
                top + y_off
            ));
            last_y = top;

            let new_right = (right + line_width).min(sheath_max_x);

            last_x = new_right;
            seg_length = new_right - left;
            seg_e = (seg_length * lh * spacing) / filament_area * first_layer_flow_ratio;
            lines.push(format!(
                "G1 X{:.3} Y{:.3} E{seg_e:.5}",
                left + x_off,
                last_y + y_off
            ));
            last_x = left;

            let new_top = (top + line_width).min(sheath_max_y);

            last_y = new_top;
            seg_length = new_top - new_bottom;
            seg_e = (seg_length * lh * spacing) / filament_area * first_layer_flow_ratio;
            lines.push(format!(
                "G1 X{:.3} Y{:.3} E{seg_e:.5}",
                last_x + x_off,
                new_bottom + y_off
            ));
            last_y = new_bottom;

            bottom = new_bottom;
            right = new_right;
            top = new_top;

            let new_left = (left - line_width).max(sheath_min_x);

            if new_left == sheath_min_x
                && new_right == sheath_max_x
                && new_bottom == sheath_min_y
                && new_top == sheath_max_y
            {
                break;
            }

            left = new_left;
            last_x = left;
        }
    } else {
        let lh = layer_height;
        let spacing = line_width - lh * (1.0 - std::f64::consts::PI / 4.0);
        last_x = sheath_min_x;
        last_y = sheath_min_y;
        lines.push(format!(
            "G1 X{:.3} Y{:.3}",
            sheath_min_x + x_off,
            sheath_min_y + y_off
        ));
        lines.push(format!("G1 Z{current_z_height:.3} ;Print Z"));
        lines.push(format!(
            "G1 E{} F{}",
            format_e(retract_length),
            RETRACT_SPEED
        ));
        lines.push(format!("M204 S{}", print_accel as i64));
        lines.push(format!("G1 F{}", format_speed(sheath_speed)));
        lines.push("G92 E0".to_string());

        let width = sheath_max_x - sheath_min_x;
        let height = sheath_max_y - sheath_min_y;
        let width_e = (width * lh * spacing) / filament_area;
        let height_e = (height * lh * spacing) / filament_area;

        last_x = sheath_max_x;
        lines.push(format!(
            "G1 X{:.3} Y{:.3} E{width_e:.5}",
            sheath_max_x + x_off,
            sheath_min_y + y_off
        ));

        last_y = sheath_max_y;
        lines.push(format!(
            "G1 X{:.3} Y{:.3} E{height_e:.5}",
            sheath_max_x + x_off,
            sheath_max_y + y_off
        ));

        last_x = sheath_min_x;
        lines.push(format!(
            "G1 X{:.3} Y{:.3} E{width_e:.5}",
            sheath_min_x + x_off,
            sheath_max_y + y_off
        ));

        last_y = sheath_min_y;
        lines.push(format!(
            "G1 X{:.3} Y{:.3} E{height_e:.5}",
            sheath_min_x + x_off,
            sheath_min_y + y_off
        ));
    }

    lines.push("; === Trapezoidal Sheath End ===".to_string());

    let outer_poly = [
        Point::new(sheath_min_x, sheath_min_y),
        Point::new(sheath_max_x, sheath_min_y),
        Point::new(sheath_max_x, sheath_max_y),
        Point::new(sheath_min_x, sheath_max_y),
    ];
    let mut start_idx = 0usize;
    if last_y > sheath_min_y + 0.01 {
        start_idx = 3;
    } else if last_x > sheath_max_x - 0.01 {
        start_idx = 1;
    } else if last_x < sheath_min_x + 0.01 && last_y > sheath_min_y + 0.01 {
        start_idx = 3;
    }

    let wipe_pts = polygon_walk(&outer_poly, start_idx, TOWER_WIPE_DISTANCE);
    if !wipe_pts.is_empty() {
        lines.push("; WIPE_START".to_string());
        lines.push(format!("G1 F{TOWER_WIPE_SPEED}"));
        lines.push(format!("M204 S{TOWER_WIPE_ACCEL}"));
        let mut total_wipe_len = 0.0;
        let mut prev_pt = Point::new(last_x, last_y);
        for p in &wipe_pts {
            let dx = p.x - prev_pt.x;
            let dy = p.y - prev_pt.y;
            total_wipe_len += dx.hypot(dy);
            prev_pt = *p;
        }
        if total_wipe_len > 0.0 {
            let mut cum_len = 0.0;
            let mut prev_pt = Point::new(last_x, last_y);
            let mut prev_e = 0.0;
            for p in &wipe_pts {
                let dx = p.x - prev_pt.x;
                let dy = p.y - prev_pt.y;
                let seg_len = dx.hypot(dy);
                cum_len += seg_len;
                let cur_e = -retract_length * cum_len / total_wipe_len;
                let seg_e = cur_e - prev_e;
                let mx = p.x + x_off;
                let my = p.y + y_off;
                lines.push(format!("G1 X{mx:.3} Y{my:.3} E{seg_e:.5}"));
                prev_pt = *p;
                prev_e = cur_e;
            }
            last_x = wipe_pts[wipe_pts.len() - 1].x;
            last_y = wipe_pts[wipe_pts.len() - 1].y;
        }
        lines.push("; WIPE_END".to_string());
    }

    lines.push(format!(
        "G1 E-{TOWER_FINAL_RETRACT:.5} F{TOWER_FINAL_RETRACT_SPEED}"
    ));
    lines.push("M204 S10000".to_string());
    lines.push("G17".to_string());
    lines.push(format!(
        "G3 Z{:.3} I{SPIRAL_LIFT_RADIUS:.3} J0 P1 F{SPIRAL_LIFT_SPEED}",
        current_z_height + SPIRAL_LIFT_Z_HEIGHT
    ));

    lines.push(format!(
        "G1 X{:.3} Y{:.3} F{}",
        last_x + WIPE_EXIT_OFFSET + x_off,
        last_y + y_off,
        format_speed(travel_speed)
    ));

    Some(lines)
}
