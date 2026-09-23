//! G3 —— 擦料塔三入口的参数矩阵 golden（Task 3.3 落判据，Task 8 转绿）。
//!
//! 用例矩阵逐条译自旧 `tower_matrix_golden_test.go`（含共用常量与 23 条用例的
//! 全部标量入参）；块渲染 / `<nil>` vs `<empty>` 语义见 `golden_common`。
//! 浮点纪律：所有入参与旧侧逐字相同；实现侧禁止 FMA 融合写法（gatecheck 已钉）。

mod golden_common;

use golden_common as gc;

// —— 矩阵共用固定参数（tower_matrix_golden_test.go 顶部 const 块逐字对照）——
const TM_NOZZLE: f64 = 0.4;
const TM_LAYER_HEIGHT: f64 = 0.2;
const TM_FIRST_LAYER_H: f64 = 0.2;
const TM_WIPER_X: f64 = 20.0;
const TM_WIPER_Y: f64 = 20.0;
const TM_TRAVEL_SPEED: f64 = 9000.0;
const TM_PRINT_SPEED: f64 = 3600.0;
const TM_RETRACT: f64 = 1.5;
const TM_FLOW_RATIO: f64 = 1.2;
const TM_NEXT_MODEL_Z: f64 = 0.4;
const TM_SAFE_Z_OFFSET: f64 = 0.6;
const TM_BBOX_MIN_X: f64 = 5.96;
const TM_BBOX_MIN_Y: f64 = 5.96;
const TM_WALL_WIDTH: f64 = 1.0;
const TM_CONVERGE_LAYERS: i64 = 5;
const TM_ENABLE_HEIGHT: f64 = 20.0;
const TM_MODEL_TALL_HEIGHT: f64 = 50.0;
/// tower/constants.go:7-8（rib 用例把塔心作为入参传入）
const TOWER_CENTER_X: f64 = 20.0;
const TOWER_CENTER_Y: f64 = 20.0;

// —— 三个入口接真实实现（Task 8）。签名保持 Option<Vec<String>>
// （nil 与空切片必须可区分）。——
use mkp_pp::postproc::tower::{
    generate_mini_spiral_gcode, generate_rib_gcode, generate_sheath_gcode,
};

fn sheath(
    name: &str,
    layer_count: i64,
    outer: &str,
    model_total_height: f64,
    base_expand: f64,
) -> (String, Option<Vec<String>>) {
    (
        format!("sheath/{name}"),
        generate_sheath_gcode(
            layer_count,
            layer_count as f64 * TM_LAYER_HEIGHT + TM_FIRST_LAYER_H,
            TM_LAYER_HEIGHT,
            TM_FIRST_LAYER_H,
            TM_WIPER_X,
            TM_WIPER_Y,
            TM_TRAVEL_SPEED,
            TM_PRINT_SPEED,
            TM_RETRACT,
            TM_NOZZLE,
            outer,
            base_expand,
            TM_ENABLE_HEIGHT,
            TM_CONVERGE_LAYERS,
            TM_WALL_WIDTH,
            model_total_height,
            TM_FLOW_RATIO,
            TM_NEXT_MODEL_Z,
            TM_SAFE_Z_OFFSET,
            TM_BBOX_MIN_X,
            TM_BBOX_MIN_Y,
        ),
    )
}

#[allow(clippy::too_many_arguments)]
fn rib(
    name: &str,
    layer_count: i64,
    tower_height: f64,
    rib_width: f64,
    rib_extra_len: f64,
    fillet_wall: bool,
    bottom_style: &str,
) -> (String, Option<Vec<String>>) {
    (
        format!("rib/{name}"),
        generate_rib_gcode(
            layer_count,
            layer_count as f64 * TM_LAYER_HEIGHT + TM_FIRST_LAYER_H,
            TM_LAYER_HEIGHT,
            TM_FIRST_LAYER_H,
            tower_height,
            TM_WIPER_X,
            TM_WIPER_Y,
            TM_TRAVEL_SPEED,
            TM_PRINT_SPEED,
            TM_RETRACT,
            TM_NOZZLE,
            rib_extra_len,
            rib_width,
            fillet_wall,
            TM_FLOW_RATIO,
            bottom_style,
            TOWER_CENTER_X,
            TOWER_CENTER_Y,
            TM_NEXT_MODEL_Z,
            TM_SAFE_Z_OFFSET,
            TM_BBOX_MIN_X,
            TM_BBOX_MIN_Y,
        ),
    )
}

fn spiral(name: &str, quadrant: i64, skip_safe_z: bool) -> (String, Option<Vec<String>>) {
    (
        format!("mini_spiral/{name}"),
        generate_mini_spiral_gcode(
            quadrant,
            0.6,
            TM_WIPER_X,
            TM_WIPER_Y,
            TM_BBOX_MIN_X,
            TM_BBOX_MIN_Y,
            TM_TRAVEL_SPEED,
            TM_NOZZLE,
            TM_SAFE_Z_OFFSET,
            0.0,
            0.0,
            0.0,
            0.0,
            0.6,
            skip_safe_z,
        ),
    )
}

/// 23 条用例（译自 towerMatrixCases()，顺序无关——比对按块点名）。
fn all_cases() -> Vec<(String, Option<Vec<String>>)> {
    vec![
        // —— Sheath 分支矩阵 ——
        sheath("first_layer_sheath", 0, "sheath", TM_MODEL_TALL_HEIGHT, 2.0),
        sheath("first_layer_brim", 0, "brim", TM_MODEL_TALL_HEIGHT, 2.0),
        sheath("first_layer_rib_clamp", 0, "rib", TM_MODEL_TALL_HEIGHT, 2.0),
        sheath(
            "layer1_sheath_rect_lowaccel",
            1,
            "sheath",
            TM_MODEL_TALL_HEIGHT,
            2.0,
        ),
        sheath(
            "layer3_sheath_rect_highaccel",
            3,
            "sheath",
            TM_MODEL_TALL_HEIGHT,
            4.0,
        ),
        sheath("layer1_brim_nil", 1, "brim", TM_MODEL_TALL_HEIGHT, 2.0),
        sheath("layer1_sheath_short_nil", 1, "sheath", 10.0, 2.0),
        sheath(
            "layer5_converged_nil",
            5,
            "sheath",
            TM_MODEL_TALL_HEIGHT,
            2.0,
        ),
        sheath(
            "layer4_expansion_below_min_nil",
            4,
            "sheath",
            TM_MODEL_TALL_HEIGHT,
            0.4,
        ),
        // —— Rib 分支矩阵（mid_tower_h250 已被旧侧移出字节 golden，勿加回）——
        rib(
            "first_layer_fillet",
            0,
            100.0,
            2.0,
            10.0,
            true,
            "concentric",
        ),
        rib(
            "first_layer_nofillet",
            0,
            100.0,
            2.0,
            10.0,
            false,
            "concentric",
        ),
        rib(
            "non_first_layer_fillet",
            2,
            100.0,
            2.0,
            10.0,
            true,
            "concentric",
        ),
        rib("short_tower_h5", 0, 5.0, 2.0, 0.0, true, "concentric"),
        rib("tall_tower_h350", 0, 350.0, 2.0, 0.0, true, "concentric"),
        rib("ribwidth_zero_nil", 0, 100.0, 0.0, 10.0, true, "concentric"),
        rib(
            "towerheight_zero_nil",
            0,
            0.0,
            2.0,
            10.0,
            true,
            "concentric",
        ),
        // —— MiniSpiral 分支矩阵 ——
        spiral("quadrant0", 0, false),
        spiral("quadrant1", 1, false),
        spiral("quadrant2", 2, false),
        spiral("quadrant3", 3, false),
        spiral("quadrant0_skip_safez", 0, true),
        spiral("out_of_range_neg_nil", -1, false),
        spiral("out_of_range_high_nil", 4, false),
    ]
}

#[test]
fn g3_tower_matrix_matches_golden() {
    let cases = all_cases();
    // 反空转 + 名字唯一（与旧侧同判法）
    assert!(!cases.is_empty(), "矩阵为空 —— golden 退化为空转，视为失败");
    let mut names: Vec<&str> = cases.iter().map(|(n, _)| n.as_str()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), cases.len(), "用例名重复，golden 块会碰撞");

    let mut actual_text = String::new();
    for (name, lines) in &cases {
        actual_text.push_str(&gc::render_case_block(name, lines.as_deref()));
    }

    if gc::update_golden_requested() {
        gc::write_golden("tower_matrix.golden", &actual_text);
        return;
    }

    let expected = gc::parse_case_blocks(&gc::read_golden("tower_matrix.golden"));
    let actual = gc::parse_case_blocks(&actual_text);
    gc::compare_case_blocks("G3 tower matrix", &expected, &actual);
}
