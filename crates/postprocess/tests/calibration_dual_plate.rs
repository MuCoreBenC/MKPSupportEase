//! 同盘多校准件：按对象分段识别，Z 先、XY 后依次插入。
//!
//! 真实样本（`Calibration_Combined.3mf` 一盘摆 Z 板 + XY 板）里 G-code 的
//! `; start printing object, unique label id: N` / `; stop printing object` 就是天然边界 ——
//! 不依赖两块板之间留空隙。这里用同样的分段形态造合成输入：
//!
//! | label | 外形 | 命中档 |
//! |---|---|---|
//! | 105 | 119.51 × 15.27 | `zoffset-ext` ⇒ `ZOffset` |
//! | 94 | 55.58 × 55.58 | `xy-square-55` ⇒ `XyPrecise` |
//!
//! 覆盖四件事：单 Z、单 XY、两块、以及「两块贴在一起」。

mod common;

use postprocess::ir::Ir;
use postprocess::postproc::calib_models::{
    CalibVerdicts, ModelProfile, decide_calibrations, detect_plates,
};
use postprocess::postproc::calibration::{CalibrationTarget, Mode, insert_calibration_blocks};
use postprocess::postproc::disk::{BBox, CentroidResult};

fn fixture_ir() -> Ir {
    common::install_tables();
    let json = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/ir/golden_42274_2.json"),
    )
    .expect("读不到 IR fixture");
    let mut ir: Ir = serde_json::from_str(&json).expect("IR 反序列化失败");
    ir.safety.calibration_execution_mode = "print_then_calibrate".to_string();
    ir.safety.z_calibration = "new".to_string();
    ir.safety.xy_calibration = "new".to_string();
    /* 落笔 / 收笔模板：夹具 IR 是普通模型那一份，没有这两段。 */
    ir.toolhead.custom_mount_gcode = lines(&["L801", "G1 X168 F30000"]);
    ir.toolhead.custom_unmount_gcode = lines(&["G1 X168 F30000", "L802", "G92 E0"]);
    ir
}

fn lines(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

fn profiles() -> &'static [ModelProfile] {
    common::install_tables();
    postprocess::postproc::machine_dims::tables()
        .calibration_models()
        .expect("档表没装")
}

/// 造一段对象区间：矩形四角的挤出点就是这块板的实测外形。
fn object_segment(label: &str, x: f64, y: f64, w: f64, h: f64) -> Vec<String> {
    vec![
        format!("; start printing object, unique label id: {label}"),
        format!("G1 X{x:.3} Y{y:.3} E0.1"),
        format!("G1 X{:.3} Y{y:.3} E0.2", x + w),
        format!("G1 X{:.3} Y{:.3} E0.3", x + w, y + h),
        format!("G1 X{x:.3} Y{:.3} E0.4", y + h),
        format!("; stop printing object, unique label id: {label}"),
    ]
}

fn z_plate() -> Vec<String> {
    object_segment("105", 30.20, 121.00, 119.51, 15.27)
}

fn xy_plate() -> Vec<String> {
    object_segment("94", 63.48, 42.79, 55.58, 55.58)
}

fn plate_lines(plates: &[Vec<String>]) -> Vec<String> {
    let mut out = vec!["G90".to_string()];
    for p in plates {
        out.extend(p.iter().cloned());
    }
    out
}

/// 两块插入目标：Z 板与 XY 板，各带自己的包围盒。
fn dual_targets() -> Vec<CalibrationTarget> {
    let z_bbox = BBox {
        x_min: 30.20,
        x_max: 149.71,
        y_min: 121.00,
        y_max: 136.27,
    };
    let xy_bbox = BBox {
        x_min: 63.48,
        x_max: 119.06,
        y_min: 42.79,
        y_max: 98.37,
    };
    let centroid = |b: &BBox| CentroidResult {
        centroid_x: (b.x_min + b.x_max) / 2.0,
        centroid_y: (b.y_min + b.y_max) / 2.0,
        valid: true,
    };
    vec![
        CalibrationTarget {
            mode: Mode::ZOffset,
            bbox: Some(z_bbox),
            centroid: centroid(&z_bbox),
        },
        CalibrationTarget {
            mode: Mode::XyPrecise,
            bbox: Some(xy_bbox),
            centroid: centroid(&xy_bbox),
        },
    ]
}

/// 单 Z：只认出一块，列表长度 1。
#[test]
fn a_lone_z_plate_is_still_recognised() {
    let content = plate_lines(&[z_plate()]);
    let plates = detect_plates(&content, profiles());
    assert_eq!(plates.len(), 1, "{plates:?}");
    assert_eq!(plates[0].mode, Mode::ZOffset);
    assert_eq!(
        decide_calibrations(false, Mode::Combined, &plates, None, profiles()),
        CalibVerdicts::Calibrations(vec![Mode::ZOffset])
    );
}

/// 单 XY：同样只认出一块。
#[test]
fn a_lone_xy_plate_is_still_recognised() {
    let content = plate_lines(&[xy_plate()]);
    let plates = detect_plates(&content, profiles());
    assert_eq!(plates.len(), 1, "{plates:?}");
    assert_eq!(plates[0].mode, Mode::XyPrecise);
    assert_eq!(
        decide_calibrations(false, Mode::Combined, &plates, None, profiles()),
        CalibVerdicts::Calibrations(vec![Mode::XyPrecise])
    );
}

/// 单 XY 且身份写的是 `Rough`：外形定「是 XY 板」，身份定哪一档。
#[test]
fn a_lone_xy_plate_takes_its_identity_from_the_3mf() {
    let content = plate_lines(&[xy_plate()]);
    let plates = detect_plates(&content, profiles());
    assert_eq!(
        decide_calibrations(false, Mode::XyRough, &plates, None, profiles()),
        CalibVerdicts::Calibrations(vec![Mode::XyRough])
    );
}

/// 两块：先 Z 后 XY（顺序由 `detect_plates` 排好）。
#[test]
fn two_plates_come_out_z_first() {
    let content = plate_lines(&[xy_plate(), z_plate()]);
    let plates = detect_plates(&content, profiles());
    assert_eq!(plates.len(), 2, "{plates:?}");
    assert_eq!(
        decide_calibrations(false, Mode::Combined, &plates, None, profiles()),
        CalibVerdicts::Calibrations(vec![Mode::ZOffset, Mode::XyPrecise])
    );
}

/// 两块紧贴（中间没有空隙）也分得开：边界是标签，不是间距。
#[test]
fn two_plates_touching_each_other_still_split() {
    let mut content = vec!["G90".to_string()];
    content.extend(object_segment("1", 0.0, 0.0, 119.51, 15.27));
    content.extend(object_segment("2", 119.51, 0.0, 55.58, 55.58));
    let plates = detect_plates(&content, profiles());
    assert_eq!(plates.len(), 2, "{plates:?}");
}

/// 插入：两块各用**自己的**包围盒，Z 块排在 XY 块前面。
#[test]
fn two_blocks_are_inserted_z_before_xy() {
    let mut ir = fixture_ir();
    let targets = dual_targets();
    let content = lines(&[
        "; FEATURE: Outer wall",
        "G1 X80.0 Y80.0 E0.5",
        "; filament end gcode",
        "M104 S0",
    ]);

    let (out, receipts) =
        insert_calibration_blocks(content, &targets, &mut ir, 2, 3).expect("插入失败");

    assert_eq!(receipts.len(), 2, "两块各一条回执");
    let z_at = out
        .iter()
        .position(|l| l == "; ZOffset Calibration")
        .expect("没有 Z 校准块");
    let xy_at = out
        .iter()
        .position(|l| l == "; XY Offset Calibration")
        .expect("没有 XY 校准块");
    assert!(z_at < xy_at, "Z 必须排在 XY 前面: {z_at} vs {xy_at}");

    /* 只在最外层各做一次：第一块落笔、最后一块收笔，中间什么都不做。
     * 「Runtime」那套（回抽 + 降温）也只给第一块 —— 第二块接着跑时笔还挂着。 */
    let count = |needle: &str| out.iter().filter(|l| l.as_str() == needle).count();
    assert_eq!(
        count("; Calibration Runtime: retract and cool down"),
        1,
        "运行时头只给第一块"
    );
    assert_eq!(count(";Mounting Toolhead"), 1, "只落一次笔");
    assert_eq!(count(";Toolhead Mounted"), 1);
    assert_eq!(count(";Unmounting Toolhead"), 1, "只收一次笔");
    assert_eq!(count("L802"), 1, "只松一次笔");
    assert_eq!(
        count(";Calibration Start Music"),
        2,
        "只留第一块的开场与最后一块的结束（各一次）"
    );
    assert_eq!(count("G1 X100 Y100 Z100"), 1, "只在最后抬到 Z100");

    /* 两块之间不许出现「收笔 / 松笔 / 抬 Z100」—— 这些是用户点出来的中间那几段。
     * 第一块的落笔与开场音乐本来就在这段区间里，不算。 */
    let between = &out[z_at..xy_at];
    for banned in [";Unmounting Toolhead", "L802", "G1 X100 Y100 Z100"] {
        assert!(
            !between.iter().any(|l| l.as_str() == banned),
            "两块之间不该有 {banned}"
        );
    }

    /* 第二块入口必须自己把 Z 放回工作高度：第一块收尾会抬到 Z100，
     * 不重新定位的话 XY 块就在 Z100 上空跑（真机 2026-11-28 撞到过）。 */
    let z_line = out[xy_at..]
        .iter()
        .find(|l| l.starts_with("G1 Z"))
        .expect("XY 块入口没有 Z 定位");
    let z: f64 = z_line[4..]
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .parse()
        .unwrap_or(f64::MAX);
    assert!(z < 20.0, "XY 块不能在 Z100 上空跑：{z_line}");
}

/// 「直接校准」一盘两块：模型路径必须整段去掉，只留两块校准。
#[test]
fn direct_calibrate_drops_the_model_for_two_plates() {
    let mut ir = fixture_ir();
    ir.safety.calibration_execution_mode = "direct_calibrate".to_string();
    let content = lines(&[
        "; CONFIG_BLOCK_START",
        "; total layer number: 1/1",
        "; CONFIG_BLOCK_END",
        "; FEATURE: Outer wall",
        "G1 X80.0 Y80.0 E0.5",
        "G1 X90.0 Y80.0 E0.6",
        "; filament end gcode",
        "M104 S0",
    ]);
    let (out, receipts) =
        insert_calibration_blocks(content, &dual_targets(), &mut ir, 2, 3).expect("插入失败");

    assert_eq!(receipts.len(), 2, "两块各一条回执");
    assert!(
        !out.iter()
            .any(|l| l.contains("X80.0 Y80.0") || l.contains("X90.0 Y80.0")),
        "模型路径没删掉: {out:?}"
    );
    assert!(out.iter().any(|l| l == "; ZOffset Calibration"), "Z 块丢了");
    assert!(
        out.iter().any(|l| l == "; XY Offset Calibration"),
        "XY 块丢了"
    );
}
