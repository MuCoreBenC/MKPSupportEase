//! 小螺旋（第二层 4 象限不挤出润笔）生成（对照 tower/mini_spiral.go 逐行复刻）。

use super::constants::*;
use crate::gcode::{format_float, format_speed, format_z};

/// 4 个象限的局部坐标边界 [minX, minY, maxX, maxY]（与旧侧常量表同式计算）。
fn mini_spiral_quadrant_bounds() -> [[f64; 4]; 4] {
    let gap = (TOWER_OUTER_BOUND - TOWER_INNER_BOUND - MINI_SPIRAL_QUADRANT_SIZE * 2.0) / 2.0;
    [
        // 左下
        [
            TOWER_INNER_BOUND + MINI_SPIRAL_CENTER_ADJUST,
            TOWER_INNER_BOUND + MINI_SPIRAL_CENTER_ADJUST,
            TOWER_CENTER_X - gap + MINI_SPIRAL_CENTER_ADJUST,
            TOWER_CENTER_Y - gap + MINI_SPIRAL_CENTER_ADJUST,
        ],
        // 右下
        [
            TOWER_CENTER_X + gap - MINI_SPIRAL_CENTER_ADJUST,
            TOWER_INNER_BOUND + MINI_SPIRAL_CENTER_ADJUST,
            TOWER_OUTER_BOUND - MINI_SPIRAL_CENTER_ADJUST,
            TOWER_CENTER_Y - gap + MINI_SPIRAL_CENTER_ADJUST,
        ],
        // 右上
        [
            TOWER_CENTER_X + gap - MINI_SPIRAL_CENTER_ADJUST,
            TOWER_CENTER_Y + gap - MINI_SPIRAL_CENTER_ADJUST,
            TOWER_OUTER_BOUND - MINI_SPIRAL_CENTER_ADJUST,
            TOWER_OUTER_BOUND - MINI_SPIRAL_CENTER_ADJUST,
        ],
        // 左上
        [
            TOWER_INNER_BOUND + MINI_SPIRAL_CENTER_ADJUST,
            TOWER_CENTER_Y + gap - MINI_SPIRAL_CENTER_ADJUST,
            TOWER_CENTER_X - gap + MINI_SPIRAL_CENTER_ADJUST,
            TOWER_OUTER_BOUND - MINI_SPIRAL_CENTER_ADJUST,
        ],
    ]
}

/// `GenerateMiniSpiralGCode`：quadrant 0=左下 1=右下 2=右上 3=左上；越界返回 None。
// Go 源码原样带着若干「赋值后未读」的 last_x/last_y 写回（Go 编译器不警告）。
// 逐行复刻保留它们以维持与旧实现的行级对应，不做「顺手清理」。
#[allow(unused_assignments, clippy::let_and_return)]
#[allow(clippy::too_many_arguments)]
pub fn generate_mini_spiral_gcode(
    quadrant_index: i64,
    current_z_height: f64,
    wiper_x: f64,
    wiper_y: f64,
    bbox_min_x: f64,
    bbox_min_y: f64,
    travel_speed: f64,
    nozzle_diameter: f64,
    safe_z_offset: f64,
    toolhead_x_offset: f64,
    toolhead_y_offset: f64,
    toolhead_z_offset: f64,
    glue_z_offset: f64,
    comp_z: f64,
    skip_initial_safe_z: bool,
) -> Option<Vec<String>> {
    if !(0..4).contains(&quadrant_index) {
        return None;
    }

    let q = mini_spiral_quadrant_bounds()[quadrant_index as usize];
    let (sheath_min_x, sheath_min_y, sheath_max_x, sheath_max_y) = (q[0], q[1], q[2], q[3]);

    let cx = (sheath_min_x + sheath_max_x) / 2.0;
    let cy = (sheath_min_y + sheath_max_y) / 2.0;

    let x_off = wiper_x - bbox_min_x;
    let y_off = wiper_y - bbox_min_y;
    let total_x_off = x_off + toolhead_x_offset;
    let total_y_off = y_off + toolhead_y_offset;

    let line_width = nozzle_diameter * LINE_WIDTH_MULTIPLIER;

    let mut lines: Vec<String> = Vec::new();
    lines.push(format!(
        "; === Mini Spiral Start (Quadrant {quadrant_index}) ==="
    ));

    let safe_z = current_z_height + safe_z_offset + toolhead_z_offset;
    lines.push(format!("G1 F{}", format_speed(travel_speed)));
    if !skip_initial_safe_z {
        lines.push(format!("G1 Z{} ;Safe Z", format_z(safe_z)));
    }

    lines.push(format!(
        "G1 X{} Y{}",
        format_float(cx + total_x_off),
        format_float(cy + total_y_off)
    ));

    let print_z = current_z_height + toolhead_z_offset + glue_z_offset + comp_z;
    lines.push(format!("G1 Z{} ;Print Z", format_z(print_z)));

    lines.push(format!(
        "G1 E-{MINI_SPIRAL_PRE_RETRACT:.5} F{MINI_SPIRAL_PRE_RETRACT_SPEED}"
    ));

    lines.push(format!("G1 F{}", format_speed(travel_speed)));

    let mut last_x = cx;
    let mut last_y = cy;

    let mut left = cx;
    let mut right = cx;
    let mut bottom = cy;
    let mut top = cy;

    loop {
        // 段 1：水平向右
        last_x = right;
        lines.push(format!(
            "G1 X{} Y{}",
            format_float(last_x + total_x_off),
            format_float(last_y + total_y_off)
        ));

        // 段 2：垂直
        let new_bottom = (bottom - line_width).max(sheath_min_y);
        last_y = new_bottom;
        lines.push(format!(
            "G1 X{} Y{}",
            format_float(last_x + total_x_off),
            format_float(top + total_y_off)
        ));
        last_y = top;

        // 段 3：水平
        let new_right = (right + line_width).min(sheath_max_x);
        last_x = new_right;
        lines.push(format!(
            "G1 X{} Y{}",
            format_float(left + total_x_off),
            format_float(last_y + total_y_off)
        ));
        last_x = left;

        // 段 4：垂直
        let new_top = (top + line_width).min(sheath_max_y);
        last_y = new_top;
        lines.push(format!(
            "G1 X{} Y{}",
            format_float(last_x + total_x_off),
            format_float(new_bottom + total_y_off)
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

    lines.push(format!(
        "G1 E{MINI_SPIRAL_POST_RECOVER:.5} F{MINI_SPIRAL_POST_RECOVER_SPEED}"
    ));

    lines.push(format!("G1 F{}", format_speed(travel_speed)));
    lines.push(format!("G1 Z{} ;Safe Z", format_z(safe_z)));

    lines.push(format!(
        "; === Mini Spiral End (Quadrant {quadrant_index}) ==="
    ));

    Some(lines)
}
