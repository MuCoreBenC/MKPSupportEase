//! Task 13 判据：校准检测（DetectMode）与插入（insert_calibration_gcode）。
//!
//! 字节锚点：生成器本体已由 reference_calibration.rs（Task 9.5，四模式逐字节）
//! 钉住；插入编排（filter/minimize/插入点选择）在旧侧**无字节参考**
//! （Task 4.3 只导出了生成器对），此处为行为判据 —— 如实登记，不假称字节级。
//! 三种 exec_mode 各一条（13.5）。

use mkp_pp::ir::Ir;
use mkp_pp::postproc::calibration::{Mode, detect_mode, insert_calibration_gcode};
use mkp_pp::postproc::disk::{BBox, CentroidResult};

fn fixture_ir() -> Ir {
    let json = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/ir/golden_42274_2.json"),
    )
    .expect("读不到 IR fixture");
    serde_json::from_str(&json).expect("IR 反序列化失败")
}

fn lines(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

fn sample_bbox() -> Option<BBox> {
    Some(BBox {
        x_min: 50.0,
        x_max: 150.0,
        y_min: 50.0,
        y_max: 150.0,
    })
}

fn centroid() -> CentroidResult {
    CentroidResult {
        centroid_x: 100.0,
        centroid_y: 100.0,
        valid: true,
    }
}

#[test]
fn detect_mode_priority_matrix() {
    assert_eq!(
        detect_mode(&lines(&[
            "; total layer number: 1/1",
            "; Precise Calibration"
        ])),
        Mode::XyPrecise
    );
    assert_eq!(detect_mode(&lines(&["; Rough Calibration"])), Mode::XyRough);
    assert_eq!(
        detect_mode(&lines(&["; ZOffset Calibration"])),
        Mode::ZOffset
    );
    assert_eq!(
        detect_mode(&lines(&["; LShape Repetition"])),
        Mode::Repetition
    );
    assert_eq!(detect_mode(&lines(&["G1 X1 Y1"])), Mode::None);
    // Go 逐行判定（外层行、内层标记）：首行命中 ZOffset 即返回，
    // 不是「标记种类优先级」—— 单行多标记才是 Precise > Rough > ZOffset > Repetition。
    assert_eq!(
        detect_mode(&lines(&["; ZOffset Calibration", "; Precise Calibration"])),
        Mode::ZOffset
    );
    assert_eq!(
        detect_mode(&lines(&[
            "; line has Precise Calibration and ZOffset Calibration"
        ])),
        Mode::XyPrecise
    );
}

/// print_then_calibrate / ""：不删模型路径，校准段插到 "; filament end gcode" 之后。
#[test]
fn insert_after_print_mode_keeps_model_paths() {
    let mut ir = fixture_ir();
    ir.safety.calibration_execution_mode = "print_then_calibrate".to_string();
    let content = lines(&[
        "; CONFIG_BLOCK_START",
        "; total layer number: 1/1",
        "; CONFIG_BLOCK_END",
        "; FEATURE: Outer wall",
        "G1 X80.0 Y80.0 E0.5",
        "; filament end gcode",
        "M104 S0",
    ]);
    let out = insert_calibration_gcode(
        content,
        Mode::ZOffset,
        &mut ir,
        sample_bbox(),
        centroid(),
        2,
        3,
    )
    .expect("插入失败");
    // 插入点：filament end 之后、M104 S0 之前出现校准运行时块
    let idx_end = out
        .iter()
        .position(|l| l.contains("; filament end gcode"))
        .unwrap();
    let idx_runtime = out
        .iter()
        .position(|l| l == "; Calibration Runtime: retract and cool down")
        .expect("应插入 Calibration Runtime 块");
    assert!(idx_runtime > idx_end);
    assert!(out.contains(&"G92 E0".to_string()));
    assert!(out.iter().any(|l| l.starts_with("G1 E-2.0 F1800")));
    // 模型移动保留（非 direct 模式不删路径）
    assert!(out.iter().any(|l| l == "G1 X80.0 Y80.0 E0.5"));
}

/// direct_calibrate：删模型路径 + 启动段最小化 + E 移动过滤。
#[test]
fn direct_calibrate_filters_model_paths() {
    let mut ir = fixture_ir();
    ir.safety.calibration_execution_mode = "direct_calibrate".to_string();
    let content = lines(&[
        ";===== machine: A1 mini",
        "M140 S60",
        "M190 S60",
        ";===== prepare print temperature and material ==========",
        "M104 S220",
        ";===== prepare print temperature and material end =====",
        "; MACHINE_START_GCODE_END",
        "; FEATURE: Outer wall",
        "G1 X80.0 Y80.0 E0.5",
        "; filament end gcode",
    ]);
    let out = insert_calibration_gcode(
        content,
        Mode::ZOffset,
        &mut ir,
        sample_bbox(),
        centroid(),
        2,
        3,
    )
    .expect("插入失败");
    // 模型挤出移动被删（direct 模式）
    assert!(!out.iter().any(|l| l == "G1 X80.0 Y80.0 E0.5"));
    // M190 被删、M140 降温到 S1（A1_MINI 启动段最小化）
    assert!(!out.iter().any(|l| l.starts_with("M190")));
    assert!(out.iter().any(|l| l.starts_with("M140 S1")));
    // prepare 区高温 M104 被删
    assert!(!out.iter().any(|l| l == "M104 S220"));
    // 校准块仍在
    assert!(
        out.iter()
            .any(|l| l == "; Calibration Runtime: retract and cool down")
    );
}

/// 空串 exec_mode 与 print_then_calibrate 同路（回退归一在 ir::build，此处验证行为一致）。
#[test]
fn empty_exec_mode_behaves_like_print_then() {
    let content = lines(&[
        "; FEATURE: Outer wall",
        "G1 X80.0 Y80.0 E0.5",
        "; filament end gcode",
    ]);
    let mut ir_a = fixture_ir();
    ir_a.safety.calibration_execution_mode = String::new();
    let mut ir_b = fixture_ir();
    ir_b.safety.calibration_execution_mode = "print_then_calibrate".to_string();
    let out_a = insert_calibration_gcode(
        content.clone(),
        Mode::ZOffset,
        &mut ir_a,
        sample_bbox(),
        centroid(),
        2,
        3,
    )
    .unwrap();
    let out_b = insert_calibration_gcode(
        content,
        Mode::ZOffset,
        &mut ir_b,
        sample_bbox(),
        centroid(),
        2,
        3,
    )
    .unwrap();
    assert_eq!(out_a, out_b);
    assert!(
        out_a.iter().any(|l| l == "G1 X80.0 Y80.0 E0.5"),
        "两种模式都保留模型路径"
    );
}

/// 无 filament end 标记：校准块追加到文件末尾（新模式先加 retract 块）。
#[test]
fn insert_without_filament_end_appends() {
    let mut ir = fixture_ir();
    let content = lines(&["; CONFIG_BLOCK_START", "x = 1", "; CONFIG_BLOCK_END"]);
    let out = insert_calibration_gcode(content, Mode::Repetition, &mut ir, None, centroid(), 2, 3)
        .expect("插入失败");
    // Repetition 非 isNewMode（bbox=None）⇒ 无 retract 块，只有校准行
    assert!(!out.iter().any(|l| l.contains("Calibration Runtime")));
    assert!(out.len() > 3, "校准行已追加");
}

/// 振动检测移除（mech mode 区内的 M970/M974）。
#[test]
fn vibration_detection_removed_in_mech_mode() {
    let content = lines(&[
        ";===== mech mode fast check",
        "M970 S1",
        "M974 X1",
        "M104 S220",
        ";===== other region",
        "M970 S9",
    ]);
    let out = mkp_pp::postproc::calibration::remove_vibration_detection_from_mech_mode(&content);
    assert!(
        !out.iter()
            .any(|l| l.starts_with("M970 S1") || l == "M974 X1"),
        "区内 M970/M974 删除"
    );
    assert!(out.iter().any(|l| l == "M104 S220"), "区内其他命令保留");
    assert!(out.iter().any(|l| l == "M970 S9"), "区外 M970 保留");
}

/// 形变检测的错误码必须原样穿过插入层。
///
/// `insert_calibration_gcode` 里那道 `map_err` 会把生成器的错误统统包成
/// `E_CAL_FAILED_001`；形变检测两个码是例外 —— 它们是用户能照做的提示
/// （「把校准模型转回来」），退化成「校准失败」等于把提示变成故障。
#[test]
fn mismatch_code_survives_the_insert_layer() {
    let mut ir = fixture_ir();
    ir.safety.xy_calibration = "new".to_string();
    let content = lines(&["; CONFIG_BLOCK_START", "; CONFIG_BLOCK_END"]);
    // sample_bbox 是 100×100，XY 图案要求 50.5×50.5 ⇒ 检测二命中
    let err = insert_calibration_gcode(
        content,
        Mode::XyPrecise,
        &mut ir,
        sample_bbox(),
        centroid(),
        2,
        3,
    )
    .expect_err("尺寸不符应当报错");
    assert_eq!(err.code(), "E_CAL_MISMATCH_002", "码被插入层吃掉了: {err}");
    assert!(err.to_string().contains("请勿缩放校准模型"), "{err}");
}

/// 反面：非形变的生成失败仍然要拿到 `E_CAL_FAILED_001`（穿透不能穿过头）。
#[test]
fn other_calibration_failures_still_get_the_generic_code() {
    let mut ir = fixture_ir();
    ir.machine.machine_type = "UNKNOWN_PRINTER".to_string();
    let content = lines(&["; CONFIG_BLOCK_START", "; CONFIG_BLOCK_END"]);
    // 机型认不出 ⇒ 经典分支拿不到机型维度 ⇒ 生成器返回 Ok(None) ⇒ 走「无输出」那条
    let err = insert_calibration_gcode(content, Mode::XyPrecise, &mut ir, None, centroid(), 2, 3)
        .expect_err("未知机型应当报错");
    assert_eq!(err.code(), "E_CAL_FAILED_001", "{err}");
}
