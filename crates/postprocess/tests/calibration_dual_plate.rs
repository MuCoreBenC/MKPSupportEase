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
    let targets = vec![
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
    ];
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
    assert_eq!(
        out.iter()
            .filter(|l| l.as_str() == "; Calibration Runtime: retract and cool down")
            .count(),
        2,
        "两块各带一段运行时头"
    );
}
