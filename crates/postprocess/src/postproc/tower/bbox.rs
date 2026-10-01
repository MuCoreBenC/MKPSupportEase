//! 塔包围盒（对照 tower/bbox.go 逐行复刻）。碰撞检测宁可误报不漏报。

use super::constants::*;
use super::geometry::{get_limit_depth_by_height, rib_section};
use super::rib::calc_brim_loops as calc_brim_loops_from;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TowerBBox {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl TowerBBox {
    pub fn width(&self) -> f64 {
        self.max_x - self.min_x
    }
    pub fn height(&self) -> f64 {
        self.max_y - self.min_y
    }
}

/// `CalculateTowerBBox`：按外围结构动态计算局部坐标包围盒。
#[allow(clippy::too_many_arguments)]
pub fn calculate_tower_bbox(
    outer_structure: &str,
    sheath_base_expand: f64,
    rib_extra_length: f64,
    rib_width: f64,
    tower_height: f64,
    nozzle_diameter: f64,
    first_layer_height: f64,
) -> TowerBBox {
    let base_min = TOWER_INNER_BOUND;
    let base_max = TOWER_OUTER_BOUND;

    match outer_structure {
        "rib" => {
            let width = base_max - base_min;
            let depth = base_max - base_min;
            let diagonal = (width * width + depth * depth).sqrt();
            let mut rib_length = diagonal.max(diagonal + rib_extra_length);
            if tower_height > 0.0 {
                let min_depth = get_limit_depth_by_height(tower_height);
                rib_length = rib_length.max(min_depth * (2.0f64).sqrt());
            }
            rib_length = diagonal.max(rib_length);

            let max_extra_len = (0.0f64).max(rib_length - diagonal) / 2.0;

            let line_width = nozzle_diameter * LINE_WIDTH_MULTIPLIER;
            let mut spacing = line_width - first_layer_height * (1.0 - std::f64::consts::PI / 4.0);
            if spacing < 0.1 {
                spacing = 0.1;
            }
            let brim_loops = calc_brim_loops_from(spacing, true, 0);
            let brim_offset = spacing / 2.0 + brim_loops as f64 * spacing;

            let max_reach = rib_length + brim_offset + max_extra_len + 2.0;

            let eff_rib_width = rib_width.min(width.min(depth) / 2.0);
            let outer_poly = rib_section(
                width,
                depth,
                diagonal + 2.0 * max_extra_len,
                eff_rib_width,
                false,
            );
            let mut p_min_x = outer_poly[0].x;
            let mut p_min_y = outer_poly[0].y;
            for p in &outer_poly[1..] {
                if p.x < p_min_x {
                    p_min_x = p.x;
                }
                if p.y < p_min_y {
                    p_min_y = p.y;
                }
            }
            const POLY_OFFSET_SAFETY: f64 = 0.5;
            let bbox_min_x = base_min + p_min_x - brim_offset - POLY_OFFSET_SAFETY;
            let bbox_min_y = base_min + p_min_y - brim_offset - POLY_OFFSET_SAFETY;
            TowerBBox {
                min_x: bbox_min_x,
                min_y: bbox_min_y,
                max_x: bbox_min_x + max_reach,
                max_y: bbox_min_y + max_reach,
            }
        }
        "sheath" | "brim" => {
            let expansion = (0.0f64).max(sheath_base_expand);
            let line_width = nozzle_diameter * LINE_WIDTH_MULTIPLIER;
            let half_width = line_width / 2.0;
            const SAFETY_MARGIN: f64 = 2.0;
            TowerBBox {
                min_x: base_min - expansion - half_width - SAFETY_MARGIN,
                min_y: base_min - expansion - half_width - SAFETY_MARGIN,
                max_x: base_max + expansion + half_width + SAFETY_MARGIN,
                max_y: base_max + expansion + half_width + SAFETY_MARGIN,
            }
        }
        _ => TowerBBox {
            min_x: base_min,
            min_y: base_min,
            max_x: base_max,
            max_y: base_max,
        },
    }
}
