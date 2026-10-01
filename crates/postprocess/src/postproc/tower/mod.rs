//! 擦料塔几何生成（对照旧仓库 `internal/tower`，按语义分模块、非文件级翻译）。
//!
//! - `constants`：常量表（逐字）
//! - `geometry`：多边形/肋截面/圆角/限深插值（逐行，浮点顺序与 Go 一字不差）
//! - `sheath` / `rib` / `spiral`：三个生成入口（G3 golden 的被测对象）
//! - `bbox`：碰撞用包围盒
//! - 刻意**不移植** `arcfit.go`：实测全仓零消费（死代码），登记于提交信息。

pub mod bbox;
pub mod constants;
pub use constants::*;
pub mod geometry;
pub mod rib;
pub mod sheath;
pub mod spiral;

mod template;

pub use bbox::{TowerBBox, calculate_tower_bbox};
pub use rib::generate_rib_gcode;
pub use sheath::{calculate_sheath_expansion, generate_sheath_gcode};
pub use spiral::generate_mini_spiral_gcode;
pub use template::get_wiping_gcode_lines;

#[cfg(test)]
mod tests {
    use super::*;

    /// 8.6 数值表：CalculateSheathExpansion 13 条（译自 TestCalculateSheathExpansion_Table，
    /// tmNozzle=0.4，converge=5，enableH=20）。
    #[test]
    fn calculate_sheath_expansion_table() {
        const CONVERGE: i64 = 5;
        const ENABLE_H: f64 = 20.0;
        const NOZZLE: f64 = 0.4;
        let cases: &[(i64, f64, &str, f64, f64)] = &[
            (0, 50.0, "sheath", 2.0, 2.0),
            (0, 50.0, "brim", 2.0, 2.0),
            (0, 50.0, "rib", 2.0, 2.0),
            (0, 5.0, "sheath", 2.0, 2.0),
            (1, 50.0, "brim", 2.0, 0.0),
            (1, 50.0, "rib", 2.0, 0.0),
            (1, ENABLE_H, "sheath", 2.0, 0.0),
            (1, 50.0, "sheath", 2.0, 1.6),
            (2, 50.0, "sheath", 2.0, 1.2),
            (4, 50.0, "sheath", 2.0, 0.4),
            (CONVERGE, 50.0, "sheath", 2.0, 0.0),
            (CONVERGE + 3, 50.0, "sheath", 2.0, 0.0),
            (4, 50.0, "sheath", 0.4, 0.0), // 0.4*(1-4/5)=0.08 < 0.4*0.5
        ];
        for (layer, model_h, outer, base, want) in cases {
            let got = calculate_sheath_expansion(
                *layer, *model_h, outer, *base, ENABLE_H, CONVERGE, NOZZLE,
            );
            assert!(
                (got - want).abs() <= 1e-9,
                "layer={layer} outer={outer} base={base}: got {got}, want {want}"
            );
        }
    }

    /// 8.6 数值表：getLimitDepthByHeight 9 条 + 单调不减扫描。
    #[test]
    fn get_limit_depth_by_height_table_and_monotonicity() {
        let cases: &[(f64, f64)] = &[
            (1.0, 5.0),
            (5.0, 5.0),
            (50.0, 5.0 + (50.0 - 5.0) / (100.0 - 5.0) * (20.0 - 5.0)),
            (100.0, 20.0),
            (175.0, 30.0),
            (250.0, 40.0),
            (300.0, 50.0),
            (350.0, 60.0),
            (1000.0, 60.0),
        ];
        for (h, want) in cases {
            let got = geometry::get_limit_depth_by_height(*h);
            assert!((got - want).abs() <= 1e-9, "h={h}: got {got}, want {want}");
        }
        let mut prev = geometry::get_limit_depth_by_height(0.0);
        let mut h = 5.0;
        while h <= 400.0 {
            let cur = geometry::get_limit_depth_by_height(h);
            assert!(
                cur >= prev,
                "深度随高度下降：h={h} 时 {cur} < 前一档 {prev}"
            );
            prev = cur;
            h += 5.0;
        }
    }

    /// 8.1：模板嵌入与切分语义（对照 embed_templates.go 的 splitTemplate）。
    #[test]
    fn tower_layer_template_is_embedded() {
        let lines = get_wiping_gcode_lines();
        assert_eq!(
            lines.len(),
            60,
            "模板 60 行（尾部空行已按 splitTemplate 剥掉）"
        );
        assert_eq!(lines[0], ";Tower_Layer_Gcode");
        assert!(lines.contains(&";WIPE_START".to_string()));
        assert!(lines.contains(&"EXTRUDER_REFILL".to_string()));
        assert!(lines.contains(&";START_HERE".to_string()));
    }

    /// bbox：rib 分支是保守正方形且 Min/Max 关系成立（对照 bbox_test.go 的判法）。
    #[test]
    fn tower_bbox_rib_is_conservative_square() {
        let bbox = calculate_tower_bbox("rib", 0.0, 10.0, 2.0, 100.0, 0.4, 0.2);
        assert!(
            (bbox.width() - bbox.height()).abs() < 1e-9,
            "rib 包围盒须为正方形"
        );
        assert!(
            bbox.min_x < TOWER_INNER_BOUND,
            "锚点必须低于塔体内界（含 brim+安全余量）"
        );
        assert!(
            bbox.width() > TOWER_OUTER_BOUND - TOWER_INNER_BOUND,
            "保守边长大于塔体"
        );
    }

    /// bbox：brim/sheath 分支四向对称扩展。
    #[test]
    fn tower_bbox_sheath_expands_symmetrically() {
        let bbox = calculate_tower_bbox("sheath", 5.0, 0.0, 2.0, 100.0, 0.4, 0.2);
        // expansion=5, lineWidth=0.5, half=0.25, safety=2 → 10.21-7.25 = 2.96
        assert!((bbox.min_x - (10.21 - 5.0 - 0.25 - 2.0)).abs() < 1e-9);
        assert!((bbox.max_x - (29.79 + 5.0 + 0.25 + 2.0)).abs() < 1e-9);
        assert_eq!(bbox.min_x, bbox.min_y);
        assert_eq!(bbox.max_x, bbox.max_y);
    }

    use constants::{TOWER_INNER_BOUND, TOWER_OUTER_BOUND};
}
