//! 校准 G-code 生成（对照 disk/generator.go 的经典三生成器 + calibeSing +
//! zOffsetSingLines，以及 generator_new.go 的两个 New 变体）。
//!
//! 经典分支拿机型固定坐标（`machine_dims` 的 `calibration.*`）；New 变体拿
//! 模型 BBox 左下角 + 固定图案偏移，即「固定模板 + 动态原点」，图案跟着模型走。

use crate::diag::PostprocError;
use crate::gcode::{OffsetMode, filter_e_moves, format_float, format_speed, format_z};
use crate::ir::Ir;
use crate::postproc::machine_dims::get_machine_dimensions;
use crate::postproc::offset::{MM_PER_MINUTE, process_gcode_offset};

/// home 行含 E0.001 虚拟挤出标记（BBS Viewer 兼容）。
const CALIBRATION_HOME_LINE: &str =
    "G1 X100 Y100 Z10 E0.001 F3000 ; MKPSE: virtual extrusion for BBS G-code viewer compatibility";

const CALIBE_SING: &str = "\
;Calibration Start Music
M17
M400 S1
M1006 S1
M1006 L70 M70 N99
M1006 A53 B19 L38 C0 D20 E53 F19 N69
M1006 A0 B1 C0 D1 E0 F1
M1006 A52 B19 L52 C0 D19 E52 F19 N69
M1006 A53 B19 L38 C0 D20 E53 F19 N69
M1006 A52 B1 L52 C0 D1 E0 F1
M1006 A52 B18 L52 C0 D19 E52 F19 N69
M1006 A53 B1 L38 C0 D1 E52 F1 N69
M1006 A53 B18 L38 C0 D19 E53 F19 N69
;Tick 103, Time 1 sec
M73 P0 R2
M1006 A48 B19 L69 C0 D20 E48 F19 N31
M1006 A0 B1 C0 D1 E0 F1
M1006 A51 B19 L69 C0 D19 E51 F19 N69
M1006 A51 B1 L69 C0 D1 E51 F1 N69
M1006 A49 B19 L69 C0 D19 E49 F19 N31
M1006 A46 B1 L69 C22 D1 M42 E49 F1 N31
M1006 A46 B19 L69 C22 D18 M42 E46 F19 N45
M1006 A46 B1 L69 C0 D1 E46 F1 N45
M1006 A46 B18 L69 C29 D19 M10 E46 F19 N45
;Tick 203, Time 2 sec
M73 P1 R2
M1006 A0 B1 C29 D1 M10 E46 F1 N45
M1006 A0 B19 C34 D19 M62 E0 F19
M1006 A37 B1 L53 C34 D1 M62 E0 F1
M1006 A37 B18 L53 C0 D19 E37 F19 N31
M1006 A0 B1 C0 D1 E37 F1 N31
M1006 A41 B19 L57 C0 D19 E41 F19 N38
M1006 A41 B1 L57 C0 D1 E46 F1 N45
M1006 A46 B19 L69 C0 D19 E46 F18 N45
M1006 A48 B20 L69 C17 D19 M37 E48 F20 N31
;Tick 303, Time 3 sec
M73 P2 R2
M1006 A48 B1 L69 C0 D1 E48 F1 N31
M1006 A48 B18 L69 C29 D19 M10 E48 F18 N31
M1006 A0 B20 C33 D19 M45 E0 F20
M1006 A0 B1 C0 D1 E41 F1 N38
M1006 A41 B19 L57 C0 D19 E41 F18 N38
M1006 A41 B1 L57 C0 D1 E45 F1 N45
M1006 A45 B19 L69 C0 D19 E45 F18 N45
M1006 A48 B1 L69 C0 D1 E0 F1
M1006 A48 B18 L69 C0 D19 E48 F19 N31
;Tick 403, Time 4 sec";

const Z_OFFSET_SING_LINES: &[&str] = &[
    "G1 X8.757 Y9.583 E.03587",
    "G1 X8.224 Y9.583",
    "G1 X9.583 Y8.224 E.05904",
    "G1 X9.583 Y7.691",
    "G1 X7.691 Y9.583 E.08221",
    "G1 X7.157 Y9.583",
    "G1 X9.583 Y7.158 E.10538",
    "G1 X9.583 Y6.624",
    "G1 X6.624 Y9.583 E.12856",
    "G1 X6.091 Y9.583",
    "G1 X9.583 Y6.091 E.15173",
    "G1 X9.583 Y5.558",
    "G1 X5.558 Y9.583 E.1749",
    "G1 X5.024 Y9.583",
    "G1 X9.583 Y5.024 E.19807",
    "G1 X9.583 Y4.491",
    "G1 X4.491 Y9.583 E.22125",
    "G1 X3.958 Y9.583",
    "G1 X9.583 Y3.958 E.24442",
    "G1 X9.583 Y3.425",
    "G1 X3.425 Y9.583 E.26759",
    "G1 X2.891 Y9.583",
    "G1 X9.583 Y2.891 E.29076",
    "G1 X9.583 Y2.358",
    "G1 X2.358 Y9.583 E.31394",
    "G1 X1.825 Y9.583",
    "G1 X9.583 Y1.825 E.33711",
    "G1 X9.583 Y1.292",
    "G1 X1.292 Y9.583 E.36028",
    "G1 X.758 Y9.583",
    "G1 X9.583 Y.758 E.38345",
    "G1 X9.38 Y.428",
    "G1 X.431 Y9.377 E.38886",
    "G1 X.417 Y8.857",
    "G1 X8.847 Y.428 E.36631",
    "G1 X8.314 Y.427",
    "G1 X.417 Y8.324 E.34316",
    "G1 X.417 Y7.791",
    "G1 X7.782 Y.426 E.32002",
    "G1 X7.249 Y.426",
    "G1 X.417 Y7.257 E.29688",
    "G1 X.417 Y6.724",
    "G1 X6.717 Y.425 E.27373",
    "G1 X6.184 Y.424",
    "G1 X.417 Y6.191 E.25059",
    "G1 X.417 Y5.658",
    "G1 X5.651 Y.424 E.22744",
    "G1 X5.119 Y.423",
    "G1 X.417 Y5.124 E.2043",
    "G1 X.417 Y4.591",
    "G1 X4.586 Y.422 E.18116",
    "G1 X4.054 Y.422",
    "G1 X.417 Y4.058 E.15801",
    "G1 X.417 Y3.525",
    "G1 X3.521 Y.421 E.13487",
    "G1 X2.988 Y.42",
    "G1 X.417 Y2.991 E.11172",
    "G1 X.417 Y2.458",
    "G1 X2.456 Y.42 E.08858",
    "G1 X1.923 Y.419",
    "G1 X.417 Y1.925 E.06544",
    "G1 X.417 Y1.392",
    "G1 X1.391 Y.418 E.04229",
    "G1 X.858 Y.418",
    "G1 X.417 Y.858 E.01915",
];

/// Repetition 模式对模型 BBox 的偏移（generator_new.go:21-22）。
/// Z New 变体也用这两个值当原点偏移。
const Z_PATTERN_OFFSET_X: f64 = 0.079;
const Z_PATTERN_OFFSET_Y: f64 = 3.755;

/// Z 校准模型的标定尺寸与方块布局（generator_new.go:23-26）。
const Z_PATTERN_MODEL_WIDTH: f64 = 119.58;
const Z_PATTERN_MODEL_HEIGHT: f64 = 13.063;
const Z_SQUARE_SPACING: f64 = 11.0;
const Z_SQUARE_COUNT: usize = 11;

/// XY 图案模板（generator_new.go:28-39）。全部是相对 BBox 左下角的偏移。
///
/// `xyPatternLineLength = 10.0` **不搬**：Go 侧声明了但全文没有任何引用
/// （线长是由 start/end 两个偏移隐含的）。搬过来只会多一个没人用的常量。
const XY_PATTERN_LINE_SPACING: f64 = 4.0;
const XY_PATTERN_H_LINE_OFFSET_Y: f64 = 10.49;
const XY_PATTERN_H_LINE_START_X: f64 = 0.5;
const XY_PATTERN_H_LINE_END_X: f64 = 10.5;
const XY_PATTERN_V_LINE_OFFSET_X: f64 = 10.5;
const XY_PATTERN_V_LINE_START_Y: f64 = 0.49;
const XY_PATTERN_V_LINE_END_Y: f64 = 10.49;
const XY_PATTERN_MODEL_WIDTH: f64 = 50.5;
const XY_PATTERN_MODEL_HEIGHT: f64 = 50.5;

/// 校准模式动态定位用的 2D 包围盒（disk.BBox）。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BBox {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CentroidResult {
    pub centroid_x: f64,
    pub centroid_y: f64,
    pub valid: bool,
}

/// 机型字符串 → Canonical Machine ID（对照 ParseMachineType）。
pub fn parse_machine_type(raw: &str) -> &'static str {
    let raw = raw.trim();
    match raw {
        "A1_MINI" | "A1 mini" | "A1mini" | "A1Mini" | "A1MINI" => "A1_MINI",
        "P1" | "P1S" | "P1SC" | "P1C" => "P1S",
        "X1" | "X1C" | "X1E" | "X1 Carbon" => "X1C",
        "A1" => "A1",
        _ => {
            if raw.contains("A1 mini") || raw.contains("A1mini") || raw.contains("A1Mini") {
                "A1_MINI"
            } else if raw.contains("X1") {
                "X1C"
            } else if raw.contains("P1") {
                "P1S"
            } else {
                ""
            }
        }
    }
}

fn round3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

fn append_calibe_sing(mut output: Vec<String>, mt: &str) -> Vec<String> {
    if mt != "A1" && mt != "A1_MINI" {
        return output;
    }
    for line in CALIBE_SING.split('\n') {
        if !line.trim().is_empty() {
            output.push(line.to_string());
        }
    }
    output
}

/// `GenerateCalibrationGCodeWithCentroid`（template_dir 恒空 —— Rust 侧无运行时
/// 模板目录，走内嵌回退，与参考导出同一分支）。
///
/// 四模式分派。`Precise/Rough/ZOffset` 三个模式各有经典与 New 两条路：
/// **IR 里写着 `"new"` 且拿到了模型 BBox 才走 New**。缺 BBox 就落经典 ——
/// 那是非动态路径（`is_dynamic` 为假时上游传 `None`），不是错误。
pub fn generate_calibration_gcode_with_centroid(
    mode: &str,
    machine_type_str: &str,
    ir_data: &Ir,
    model_bbox: Option<&BBox>,
    centroid: CentroidResult,
    z_dwell_sec: i64,
    xy_dwell_sec: i64,
) -> Result<Option<Vec<String>>, PostprocError> {
    let mt = parse_machine_type(machine_type_str);
    let z_dwell_ms = z_dwell_sec * 1000;
    let xy_dwell_ms = xy_dwell_sec * 1000;
    match mode {
        "Precise" | "Rough" => {
            if ir_data.safety.xy_calibration == "new"
                && let Some(bbox) = model_bbox
            {
                return generate_xy_calibration_new(mode, mt, ir_data, bbox, centroid, xy_dwell_ms);
            }
            generate_xy_calibration(mode, mt, ir_data, xy_dwell_ms)
        }
        "ZOffset" => {
            if ir_data.safety.z_calibration == "new"
                && let Some(bbox) = model_bbox
            {
                return generate_z_offset_calibration_new(mt, ir_data, bbox, centroid, z_dwell_ms);
            }
            generate_z_offset_calibration(mt, ir_data, z_dwell_ms)
        }
        "Repetition" => generate_repetition_calibration(mt, ir_data, model_bbox),
        _ => Ok(None),
    }
}

fn mount_unmount_lines(ir_data: &Ir) -> (Vec<String>, Vec<String>) {
    let mount = ir_data
        .toolhead
        .custom_mount_gcode
        .iter()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    let unmount = ir_data
        .toolhead
        .custom_unmount_gcode
        .iter()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    (mount, unmount)
}

fn generate_xy_calibration(
    mode: &str,
    mt: &str,
    ir_data: &Ir,
    _xy_dwell_ms: i64,
) -> Result<Option<Vec<String>>, PostprocError> {
    let mut output: Vec<String> = Vec::new();

    let dims = get_machine_dimensions(mt);
    if dims.calibration.y_line_x == 0.0 {
        return Ok(None);
    }

    output.push("; XY Offset Calibration".to_string());
    output.push(CALIBRATION_HOME_LINE.to_string());
    output.push(";Rising nozzle to avoid collision".to_string());
    output.push(format!(
        "G1 Z{}",
        format_z(round3(
            ir_data.machine.first_layer_height + ir_data.toolhead.z_offset + 6.0
        ))
    ));
    output.push(";Mounting Toolhead".to_string());
    let (mount, unmount) = mount_unmount_lines(ir_data);
    output.extend(mount);
    output.push(";Toolhead Mounted".to_string());
    output = append_calibe_sing(output, mt);

    let mut cali_accumulate = match mode {
        "Precise" => -1.0,
        "Rough" => -2.5,
        _ => 0.0,
    };

    let mut offset_accumulate = 0.0f64;
    for _ in 0..11 {
        output.push(format!(
            "G1 F{}",
            format_speed(ir_data.machine.travel_speed * MM_PER_MINUTE)
        ));
        let origin_cali_line = format!(
            "G1 X{} Y{}",
            format_float(round3(dims.calibration.y_line_x)),
            format_float(round3(dims.calibration.y_line_y + offset_accumulate))
        );
        let offset_result = process_gcode_offset(
            &origin_cali_line,
            ir_data.toolhead.x_offset,
            ir_data.toolhead.y_offset + cali_accumulate,
            ir_data.toolhead.z_offset + ir_data.glue.z_offset,
            OffsetMode::Calibration,
            ir_data,
        )?;
        output.push(offset_result);
        output.push(format!(
            "G1 Z{}",
            format_z(round3(
                ir_data.machine.first_layer_height + ir_data.toolhead.z_offset + 3.0
            ))
        ));
        if ir_data.toolhead.max_speed > 10.0 {
            output.push("G1 F300".to_string());
        } else {
            output.push(format!("G1 F{}", format_speed(ir_data.toolhead.max_speed)));
        }
        let draw_line = format!(
            "G1 X{} Y{} Z{}",
            format_float(round3(dims.calibration.y_line_x_end)),
            format_float(round3(dims.calibration.y_line_y + offset_accumulate)),
            format_z(round3(
                ir_data.machine.first_layer_height
                    + ir_data.toolhead.z_offset
                    + ir_data.glue.z_offset
            ))
        );
        let offset_result = process_gcode_offset(
            &draw_line,
            ir_data.toolhead.x_offset,
            ir_data.toolhead.y_offset + cali_accumulate,
            0.0,
            OffsetMode::Calibration,
            ir_data,
        )?;
        output.push(offset_result);
        offset_accumulate += 4.0;
        match mode {
            "Precise" => cali_accumulate += 0.2,
            "Rough" => cali_accumulate += 0.5,
            _ => {}
        }
        output.push(format!(
            "G1 Z{}",
            format_z(round3(
                ir_data.machine.first_layer_height + ir_data.toolhead.z_offset + 3.0
            ))
        ));
    }

    let mut offset_accumulate = 0.0f64;
    let mut cali_accumulate = match mode {
        "Precise" => -1.0,
        "Rough" => -2.5,
        _ => 0.0,
    };

    for _ in 0..11 {
        output.push(format!(
            "G1 F{}",
            format_speed(ir_data.machine.travel_speed * MM_PER_MINUTE)
        ));
        let origin_cali_line = format!(
            "G1 X{} Y{}",
            format_float(round3(dims.calibration.x_line_x + offset_accumulate)),
            format_float(round3(dims.calibration.x_line_y))
        );
        let offset_result = process_gcode_offset(
            &origin_cali_line,
            ir_data.toolhead.x_offset + cali_accumulate,
            ir_data.toolhead.y_offset,
            ir_data.toolhead.z_offset + ir_data.glue.z_offset,
            OffsetMode::Calibration,
            ir_data,
        )?;
        output.push(offset_result);
        output.push(format!(
            "G1 Z{}",
            format_z(round3(
                ir_data.machine.first_layer_height + ir_data.toolhead.z_offset + 3.0
            ))
        ));
        if ir_data.toolhead.max_speed > 10.0 {
            output.push("G1 F300".to_string());
        } else {
            output.push(format!("G1 F{}", format_speed(ir_data.toolhead.max_speed)));
        }
        let draw_line = format!(
            "G1 X{} Y{} Z{}",
            format_float(round3(dims.calibration.x_line_x + offset_accumulate)),
            format_float(round3(dims.calibration.x_line_y_end)),
            format_z(round3(
                ir_data.machine.first_layer_height
                    + ir_data.toolhead.z_offset
                    + ir_data.glue.z_offset
            ))
        );
        let offset_result = process_gcode_offset(
            &draw_line,
            ir_data.toolhead.x_offset + cali_accumulate,
            ir_data.toolhead.y_offset,
            0.0,
            OffsetMode::Calibration,
            ir_data,
        )?;
        output.push(offset_result);
        offset_accumulate += 4.0;
        match mode {
            "Precise" => cali_accumulate += 0.2,
            "Rough" => cali_accumulate += 0.5,
            _ => {}
        }
        output.push(format!(
            "G1 Z{}",
            format_z(round3(
                ir_data.machine.first_layer_height + ir_data.toolhead.z_offset + 3.0
            ))
        ));
    }

    output.push(";Unmounting Toolhead".to_string());
    output.extend(unmount);
    output.push(";Toolhead Unmounted".to_string());
    output = append_calibe_sing(output, mt);
    output.push("G1 X100 Y100 Z100".to_string());

    Ok(Some(output))
}

fn generate_z_offset_calibration(
    mt: &str,
    ir_data: &Ir,
    z_dwell_ms: i64,
) -> Result<Option<Vec<String>>, PostprocError> {
    let mut output: Vec<String> = Vec::new();

    let sing_lines: Vec<&str> = Z_OFFSET_SING_LINES.to_vec();

    let dims = get_machine_dimensions(mt);
    if dims.calibration.z_start_x == 0.0 {
        return Ok(None);
    }
    let fr_calibe_x_start = dims.calibration.z_start_x;
    let fr_calibe_y_start = dims.calibration.z_start_y;

    output.push("; ZOffset Calibration".to_string());
    output.push(CALIBRATION_HOME_LINE.to_string());
    output.push(";Rising nozzle to avoid collision".to_string());
    output.push(format!(
        "G1 Z{}",
        format_z(round3(
            ir_data.machine.first_layer_height + ir_data.toolhead.z_offset + 6.0
        ))
    ));
    output.push(";Mounting Toolhead".to_string());
    let (mount, unmount) = mount_unmount_lines(ir_data);
    output.extend(mount);
    output.push(";Toolhead Mounted".to_string());
    output = append_calibe_sing(output, mt);

    if mt == "A1_MINI" || mt == "A1" {
        output.push(";Floating Z Calibration".to_string());
        output.push(format!(
            "G1 F{}",
            format_speed(ir_data.machine.travel_speed * MM_PER_MINUTE)
        ));
        let align_point = format!(
            "G1 X{} Y{}",
            format_float(round3(fr_calibe_x_start + 5.0)),
            format_float(round3(fr_calibe_y_start + 5.0))
        );
        let offset_result = process_gcode_offset(
            &align_point,
            ir_data.toolhead.x_offset,
            ir_data.toolhead.y_offset,
            ir_data.toolhead.z_offset + ir_data.glue.z_offset,
            OffsetMode::Calibration,
            ir_data,
        )?;
        output.push(offset_result);
        for _ in 0..5 {
            output.push("G1 Z3.4".to_string());
            output.push("G1 Z5".to_string());
        }
    }

    let mut z_accumulate = 0.5f64;
    let mut offset_accumulate = 0.0f64;

    for _ in 0..11 {
        output.push(format!(
            "G1 F{}",
            format_speed(ir_data.machine.travel_speed * MM_PER_MINUTE)
        ));
        if sing_lines.len() > 1 {
            let offset_result = process_gcode_offset(
                sing_lines[1],
                ir_data.toolhead.x_offset + offset_accumulate + fr_calibe_x_start,
                ir_data.toolhead.y_offset + fr_calibe_y_start,
                0.0,
                OffsetMode::Calibration,
                ir_data,
            )?;
            output.push(offset_result);
        }
        output.push(format!("G1 F{}", format_speed(ir_data.toolhead.max_speed)));
        output.push(format!(
            "G1 Z{}",
            format_z(round3(
                0.4 + ir_data.toolhead.z_offset + ir_data.glue.z_offset + z_accumulate
            ))
        ));
        for sing_line in &sing_lines {
            let offset_result = process_gcode_offset(
                sing_line,
                ir_data.toolhead.x_offset + offset_accumulate + fr_calibe_x_start,
                ir_data.toolhead.y_offset + fr_calibe_y_start,
                0.0,
                OffsetMode::Calibration,
                ir_data,
            )?;
            output.push(offset_result);
        }
        z_accumulate -= 0.1;
        offset_accumulate += 11.0;
        output.push(format!(
            "G1 Z{}",
            format_z(round3(ir_data.toolhead.z_offset + 6.0))
        ));
        output.push(format!("G4 P{z_dwell_ms}"));
    }

    output.push(";Unmounting Toolhead".to_string());
    output.extend(unmount);
    output.push(";Toolhead Unmounted".to_string());
    output = append_calibe_sing(output, mt);
    output.push("G1 X100 Y100 Z100".to_string());

    Ok(Some(output))
}

fn generate_l_shape_template(base_x: f64, base_y: f64) -> Vec<String> {
    let seg_len = 10.0;
    let mut lines = Vec::new();
    lines.push(format!(
        "G1 X{} Y{}",
        format_float(round3(base_x)),
        format_float(round3(base_y))
    ));
    lines.push(format!(
        "G1 X{} Y{}",
        format_float(round3(base_x)),
        format_float(round3(base_y + seg_len * 3.0))
    ));
    lines.push(format!(
        "G1 X{} Y{}",
        format_float(round3(base_x + seg_len)),
        format_float(round3(base_y + seg_len * 3.0))
    ));
    lines.push(format!(
        "G1 X{} Y{}",
        format_float(round3(base_x + seg_len)),
        format_float(round3(base_y + seg_len * 2.0))
    ));
    lines.push(format!(
        "G1 X{} Y{}",
        format_float(round3(base_x + seg_len * 2.0)),
        format_float(round3(base_y + seg_len * 2.0))
    ));
    lines.push(format!(
        "G1 X{} Y{}",
        format_float(round3(base_x + seg_len * 2.0)),
        format_float(round3(base_y + seg_len))
    ));
    lines.push(format!(
        "G1 X{} Y{}",
        format_float(round3(base_x + seg_len * 3.0)),
        format_float(round3(base_y + seg_len))
    ));
    lines.push(format!(
        "G1 X{} Y{}",
        format_float(round3(base_x + seg_len * 3.0)),
        format_float(round3(base_y))
    ));
    lines
}

fn generate_repetition_calibration(
    mt: &str,
    ir_data: &Ir,
    model_bbox: Option<&BBox>,
) -> Result<Option<Vec<String>>, PostprocError> {
    let mut output: Vec<String> = Vec::new();

    output.push("; LShape Repetition Calibration".to_string());
    output.push(CALIBRATION_HOME_LINE.to_string());
    output.push(";Rising nozzle to avoid collision".to_string());
    output.push(format!(
        "G1 Z{}",
        format_z(round3(
            ir_data.machine.first_layer_height + ir_data.toolhead.z_offset + 6.0
        ))
    ));
    output.push(";Mounting Toolhead".to_string());
    let (mount, unmount) = mount_unmount_lines(ir_data);
    output.extend(mount);
    output.push(";Toolhead Mounted".to_string());
    output = append_calibe_sing(output, mt);
    output.push(format!(
        "G1 Z{}",
        format_z(round3(
            ir_data.machine.first_layer_height + ir_data.toolhead.z_offset + ir_data.glue.z_offset
        ))
    ));

    let (origin_x, origin_y) = if let Some(b) = model_bbox {
        (b.x_min + Z_PATTERN_OFFSET_X, b.y_min + Z_PATTERN_OFFSET_Y)
    } else {
        let dims = get_machine_dimensions(mt);
        if dims.calibration.l_shape_base_x == 0.0 {
            return Ok(None);
        }
        (
            dims.calibration.l_shape_base_x,
            dims.calibration.l_shape_base_y,
        )
    };

    let l_shape_lines = generate_l_shape_template(origin_x, origin_y);

    output.push(format!("G1 F{}", format_speed(ir_data.toolhead.max_speed)));
    for l_line in &l_shape_lines {
        if l_line.contains("G1 ") && !l_line.contains("G1 E") && !l_line.contains("G1 F") {
            let y_adjust = if mt == "A1_MINI" || mt == "A1" {
                ir_data.toolhead.y_offset
            } else {
                ir_data.toolhead.y_offset - 2.0
            };
            let offset_result = process_gcode_offset(
                l_line,
                ir_data.toolhead.x_offset,
                y_adjust,
                ir_data.toolhead.z_offset + ir_data.glue.z_offset,
                OffsetMode::Calibration,
                ir_data,
            )?;
            output.push(offset_result);
        }
    }

    output.push(";Unmounting Toolhead".to_string());
    output.extend(unmount);
    output.push(";Toolhead Unmounted".to_string());
    output = append_calibe_sing(output, mt);
    output.push("G1 X100 Y100 Z100".to_string());

    Ok(Some(output))
}

/// mount / unmount 行：trim、去空行，然后**逐行过 `filter_e_moves`**。
///
/// 与经典分支的 `mount_unmount_lines` 差一步过滤（generator_new.go:107-116 /
/// :205-214）：New 变体不许挂载脚本里的真实挤出动作混进校准图案。
fn filtered_mount_unmount_lines(ir_data: &Ir) -> (Vec<String>, Vec<String>) {
    let keep = |lines: &[String]| -> Vec<String> {
        lines
            .iter()
            .filter_map(|l| {
                let trimmed = l.trim();
                if trimmed.is_empty() {
                    return None;
                }
                filter_e_moves(&[trimmed.to_string()]).into_iter().next()
            })
            .collect()
    };
    (
        keep(&ir_data.toolhead.custom_mount_gcode),
        keep(&ir_data.toolhead.custom_unmount_gcode),
    )
}

/// 校准模型的尺寸检测（generator_new.go:50-84 与 :241-275，两处逐字相同）。
///
/// 两道：宽高互换（疑似旋转 90°）与尺寸不符（被缩放）。容差都是 0.5 mm ——
/// 切片器的挤出宽度会让 BBox 有零点几毫米抖动，卡太死会把正常模型拦下来。
///
/// 注意 `expected_width != expected_height` 这个条件：XY 图案是 50.5×50.5 的
/// 正方形，第一道检测对 XY **恒不触发**（正方形转 90° 从 BBox 完全看不出来）。
/// 这不是漏洞，正因如此 XY 才必须靠质心判方向。
fn check_model_size(
    bbox: &BBox,
    expected_width: f64,
    expected_height: f64,
    calibration_mode: &str,
    error_source: &str,
) -> Result<(), PostprocError> {
    let actual_width = bbox.x_max - bbox.x_min;
    let actual_height = bbox.y_max - bbox.y_min;
    let context = format!(
        "expected={expected_width}x{expected_height} actual={actual_width:.4}x{actual_height:.4} \
         bbox=[{:.4},{:.4}]x[{:.4},{:.4}] calibration_mode={calibration_mode} \
         source={error_source} bbox_source=original_gcode bbox_stage=before_pass2",
        bbox.x_min, bbox.x_max, bbox.y_min, bbox.y_max,
    );

    if (actual_width - expected_height).abs() < 0.5
        && (actual_height - expected_width).abs() < 0.5
        && expected_width != expected_height
    {
        return Err(PostprocError::CalibrationMismatch {
            code: "E_CAL_MISMATCH_001",
            message: format!("检测到校准模型可能被旋转。请勿旋转校准模型。（{context}）"),
        });
    }
    if (actual_width - expected_width).abs() > 0.5 || (actual_height - expected_height).abs() > 0.5
    {
        return Err(PostprocError::CalibrationMismatch {
            code: "E_CAL_MISMATCH_002",
            message: format!(
                "检测到校准模型尺寸已变化。请勿缩放校准模型，仅允许移动位置。（{context}）"
            ),
        });
    }
    Ok(())
}

/// `generateXYCalibrationNew`（generator_new.go:47-220）：BBox 感知的 XY 校准。
///
/// 与经典 `generate_xy_calibration` 的差异（四处，都容易漏）：
/// 1. 坐标相对 `model_bbox` 左下角，**不读机型固定坐标**，因此也没有
///    「机型维度缺失就返回 `Ok(None)`」那道早退；
/// 2. 每轮末尾有 `G4 P{xy_dwell_ms}` —— 经典分支根本不消费 dwell 参数；
/// 3. 先横线（H）11 轮再竖线（V）11 轮，经典分支是先 Y 线再 X 线；
/// 4. mount/unmount 行逐行过 `filter_e_moves`。
fn generate_xy_calibration_new(
    mode: &str,
    mt: &str,
    ir_data: &Ir,
    model_bbox: &BBox,
    centroid: CentroidResult,
    xy_dwell_ms: i64,
) -> Result<Option<Vec<String>>, PostprocError> {
    check_model_size(
        model_bbox,
        XY_PATTERN_MODEL_WIDTH,
        XY_PATTERN_MODEL_HEIGHT,
        mode,
        "generate_xy_calibration_new",
    )?;

    if centroid.valid {
        check_rotation_by_centroid(
            centroid,
            model_bbox,
            -1,
            -1,
            mode,
            0.02,
            "generate_xy_calibration_new",
            true,
            0,
        )?;
    }

    let mut output: Vec<String> = Vec::new();
    output.push("; XY Offset Calibration".to_string());
    output.push(CALIBRATION_HOME_LINE.to_string());
    output.push(";Rising nozzle to avoid collision".to_string());
    output.push(format!(
        "G1 Z{}",
        format_z(round3(
            ir_data.machine.first_layer_height + ir_data.toolhead.z_offset + 6.0
        ))
    ));
    output.push(";Mounting Toolhead".to_string());
    let (mount, unmount) = filtered_mount_unmount_lines(ir_data);
    output.extend(mount);
    output.push(";Toolhead Mounted".to_string());
    output = append_calibe_sing(output, mt);

    let cali_start = match mode {
        "Precise" => -1.0,
        "Rough" => -2.5,
        _ => 0.0,
    };
    let cali_step = match mode {
        "Precise" => 0.2,
        "Rough" => 0.5,
        _ => 0.0,
    };

    // 第一段：11 条横线，Y 逐轮上移，Y 方向叠加待校准偏移。
    let mut cali_accumulate = cali_start;
    let mut offset_accumulate = 0.0f64;
    for _ in 0..11 {
        let line_y = model_bbox.y_min + XY_PATTERN_H_LINE_OFFSET_Y + offset_accumulate;
        let start_x = model_bbox.x_min + XY_PATTERN_H_LINE_START_X;
        let end_x = model_bbox.x_min + XY_PATTERN_H_LINE_END_X;

        output.push(format!(
            "G1 F{}",
            format_speed(ir_data.machine.travel_speed * MM_PER_MINUTE)
        ));
        let origin_cali_line = format!(
            "G1 X{} Y{}",
            format_float(round3(start_x)),
            format_float(round3(line_y))
        );
        output.push(process_gcode_offset(
            &origin_cali_line,
            ir_data.toolhead.x_offset,
            ir_data.toolhead.y_offset + cali_accumulate,
            ir_data.toolhead.z_offset + ir_data.glue.z_offset,
            OffsetMode::Calibration,
            ir_data,
        )?);
        output.push(format!(
            "G1 Z{}",
            format_z(round3(
                ir_data.machine.first_layer_height + ir_data.toolhead.z_offset + 3.0
            ))
        ));
        if ir_data.toolhead.max_speed > 10.0 {
            output.push("G1 F300".to_string());
        } else {
            output.push(format!("G1 F{}", format_speed(ir_data.toolhead.max_speed)));
        }
        let draw_line = format!(
            "G1 X{} Y{} Z{}",
            format_float(round3(end_x)),
            format_float(round3(line_y)),
            format_z(round3(
                ir_data.machine.first_layer_height
                    + ir_data.toolhead.z_offset
                    + ir_data.glue.z_offset
            ))
        );
        output.push(process_gcode_offset(
            &draw_line,
            ir_data.toolhead.x_offset,
            ir_data.toolhead.y_offset + cali_accumulate,
            0.0,
            OffsetMode::Calibration,
            ir_data,
        )?);
        output.push(format!("G4 P{xy_dwell_ms}"));
        offset_accumulate += XY_PATTERN_LINE_SPACING;
        cali_accumulate += cali_step;
        output.push(format!(
            "G1 Z{}",
            format_z(round3(
                ir_data.machine.first_layer_height + ir_data.toolhead.z_offset + 3.0
            ))
        ));
    }

    // 第二段：11 条竖线，X 逐轮右移，X 方向叠加待校准偏移。
    let mut cali_accumulate = cali_start;
    let mut offset_accumulate = 0.0f64;
    for _ in 0..11 {
        let line_x = model_bbox.x_min + XY_PATTERN_V_LINE_OFFSET_X + offset_accumulate;
        let start_y = model_bbox.y_min + XY_PATTERN_V_LINE_START_Y;
        let end_y = model_bbox.y_min + XY_PATTERN_V_LINE_END_Y;

        output.push(format!(
            "G1 F{}",
            format_speed(ir_data.machine.travel_speed * MM_PER_MINUTE)
        ));
        let origin_cali_line = format!(
            "G1 X{} Y{}",
            format_float(round3(line_x)),
            format_float(round3(start_y))
        );
        output.push(process_gcode_offset(
            &origin_cali_line,
            ir_data.toolhead.x_offset + cali_accumulate,
            ir_data.toolhead.y_offset,
            ir_data.toolhead.z_offset + ir_data.glue.z_offset,
            OffsetMode::Calibration,
            ir_data,
        )?);
        output.push(format!(
            "G1 Z{}",
            format_z(round3(
                ir_data.machine.first_layer_height + ir_data.toolhead.z_offset + 3.0
            ))
        ));
        if ir_data.toolhead.max_speed > 10.0 {
            output.push("G1 F300".to_string());
        } else {
            output.push(format!("G1 F{}", format_speed(ir_data.toolhead.max_speed)));
        }
        let draw_line = format!(
            "G1 X{} Y{} Z{}",
            format_float(round3(line_x)),
            format_float(round3(end_y)),
            format_z(round3(
                ir_data.machine.first_layer_height
                    + ir_data.toolhead.z_offset
                    + ir_data.glue.z_offset
            ))
        );
        output.push(process_gcode_offset(
            &draw_line,
            ir_data.toolhead.x_offset + cali_accumulate,
            ir_data.toolhead.y_offset,
            0.0,
            OffsetMode::Calibration,
            ir_data,
        )?);
        output.push(format!("G4 P{xy_dwell_ms}"));
        offset_accumulate += XY_PATTERN_LINE_SPACING;
        cali_accumulate += cali_step;
        output.push(format!(
            "G1 Z{}",
            format_z(round3(
                ir_data.machine.first_layer_height + ir_data.toolhead.z_offset + 3.0
            ))
        ));
    }

    output.push(";Unmounting Toolhead".to_string());
    output.extend(unmount);
    output.push(";Toolhead Unmounted".to_string());
    output = append_calibe_sing(output, mt);
    output.push("G1 X100 Y100 Z100".to_string());

    Ok(Some(output))
}

/// `generateZOffsetCalibrationNew`（generator_new.go:233-372）：BBox 感知的 Z 偏移校准。
///
/// 与经典 `generate_z_offset_calibration` 只差原点：经典读机型 `z_start_x/y`，
/// 这里用 `bbox` 左下角 + (0.079, 3.755)。其余（11 个方块、`z_accumulate` 每轮
/// −0.1、间距 11.0、A1 家族的 Floating Z、每轮尾部停留）逐行一致。
///
/// `loadCalibrationTemplate(template_dir, "z_offset_path_new.gcode")` 无 Rust 对位：
/// 本项目没有运行时模板目录，Go 侧 `template_dir` 为空时也回落到内嵌 `zOffsetSingLines`，
/// 所以这里直接用 `Z_OFFSET_SING_LINES`，与经典分支同源。
fn generate_z_offset_calibration_new(
    mt: &str,
    ir_data: &Ir,
    model_bbox: &BBox,
    centroid: CentroidResult,
    z_dwell_ms: i64,
) -> Result<Option<Vec<String>>, PostprocError> {
    check_model_size(
        model_bbox,
        Z_PATTERN_MODEL_WIDTH,
        Z_PATTERN_MODEL_HEIGHT,
        "ZOffset",
        "generate_z_offset_calibration_new",
    )?;

    if centroid.valid {
        check_rotation_by_centroid(
            centroid,
            model_bbox,
            1,
            0,
            "ZOffset",
            0.01,
            "generate_z_offset_calibration_new",
            false,
            0,
        )?;
    }

    let sing_lines: Vec<&str> = Z_OFFSET_SING_LINES.to_vec();
    let fr_calibe_x_start = model_bbox.x_min + Z_PATTERN_OFFSET_X;
    let fr_calibe_y_start = model_bbox.y_min + Z_PATTERN_OFFSET_Y;

    let mut output: Vec<String> = Vec::new();
    output.push("; ZOffset Calibration".to_string());
    output.push(CALIBRATION_HOME_LINE.to_string());
    output.push(";Rising nozzle to avoid collision".to_string());
    output.push(format!(
        "G1 Z{}",
        format_z(round3(
            ir_data.machine.first_layer_height + ir_data.toolhead.z_offset + 6.0
        ))
    ));
    output.push(";Mounting Toolhead".to_string());
    let (mount, unmount) = filtered_mount_unmount_lines(ir_data);
    output.extend(mount);
    output.push(";Toolhead Mounted".to_string());
    output = append_calibe_sing(output, mt);

    if mt == "A1_MINI" || mt == "A1" {
        output.push(";Floating Z Calibration".to_string());
        output.push(format!(
            "G1 F{}",
            format_speed(ir_data.machine.travel_speed * MM_PER_MINUTE)
        ));
        let align_point = format!(
            "G1 X{} Y{}",
            format_float(round3(fr_calibe_x_start + 5.0)),
            format_float(round3(fr_calibe_y_start + 5.0))
        );
        output.push(process_gcode_offset(
            &align_point,
            ir_data.toolhead.x_offset,
            ir_data.toolhead.y_offset,
            ir_data.toolhead.z_offset + ir_data.glue.z_offset,
            OffsetMode::Calibration,
            ir_data,
        )?);
        for _ in 0..5 {
            output.push("G1 Z3.4".to_string());
            output.push("G1 Z5".to_string());
        }
    }

    let mut z_accumulate = 0.5f64;
    let mut offset_accumulate = 0.0f64;

    for _ in 0..Z_SQUARE_COUNT {
        output.push(format!(
            "G1 F{}",
            format_speed(ir_data.machine.travel_speed * MM_PER_MINUTE)
        ));
        if sing_lines.len() > 1 {
            output.push(process_gcode_offset(
                sing_lines[1],
                ir_data.toolhead.x_offset + offset_accumulate + fr_calibe_x_start,
                ir_data.toolhead.y_offset + fr_calibe_y_start,
                0.0,
                OffsetMode::Calibration,
                ir_data,
            )?);
        }
        output.push(format!("G1 F{}", format_speed(ir_data.toolhead.max_speed)));
        output.push(format!(
            "G1 Z{}",
            format_z(round3(
                0.4 + ir_data.toolhead.z_offset + ir_data.glue.z_offset + z_accumulate
            ))
        ));
        for sing_line in &sing_lines {
            output.push(process_gcode_offset(
                sing_line,
                ir_data.toolhead.x_offset + offset_accumulate + fr_calibe_x_start,
                ir_data.toolhead.y_offset + fr_calibe_y_start,
                0.0,
                OffsetMode::Calibration,
                ir_data,
            )?);
        }
        z_accumulate -= 0.1;
        offset_accumulate += Z_SQUARE_SPACING;
        output.push(format!(
            "G1 Z{}",
            format_z(round3(ir_data.toolhead.z_offset + 6.0))
        ));
        output.push(format!("G4 P{z_dwell_ms}"));
    }

    output.push(";Unmounting Toolhead".to_string());
    output.extend(unmount);
    output.push(";Toolhead Unmounted".to_string());
    output = append_calibe_sing(output, mt);
    output.push("G1 X100 Y100 Z100".to_string());

    Ok(Some(output))
}

/// `checkRotationByCentroid`（generator_new.go:374-529）：拿挤出质心相对 BBox 中心的
/// 偏移方向判断校准模型是否被旋转。
///
/// 为什么需要它：BBox 只能看出「尺寸变了」。正方形模型（XY 图案 50.5×50.5）转 90°、
/// 任何模型转 180°，BBox 都一模一样 —— 只有质心方向会反。
///
/// **四步判定顺序不可重排**：先放过（退化 BBox / 偏移太小），再依次判双轴 180°、
/// 长条 180°、正方形 90°。把「放过」挪到后面会让退化输入走进除法；把 90° 挪到
/// 180° 前面会把双轴反向误报成 90°。
///
/// Go 侧用 `errdiag.WithContext` 挂了 ~20 个结构化字段。Rust 侧没有 errdiag 这一层，
/// **登记偏差**：上下文拼进 message 文本 —— 消费方只有弹窗一行和日志一行，
/// 没有读结构化字段的人。
#[allow(clippy::too_many_arguments)]
fn check_rotation_by_centroid(
    centroid: CentroidResult,
    bbox: &BBox,
    expected_dx_sign: i32,
    expected_dy_sign: i32,
    calibration_mode: &str,
    min_offset_ratio: f64,
    error_source: &str,
    is_square_model: bool,
    // Go 侧两个调用方都传 0，此时 `magnitude_ok` 恒为 true。参数保留是为了与
    // 参考实现同形，不是为了将来某个假想需求。
    expected_relative_magnitude: i32,
) -> Result<(), PostprocError> {
    let center_x = (bbox.x_min + bbox.x_max) / 2.0;
    let center_y = (bbox.y_min + bbox.y_max) / 2.0;
    let width = bbox.x_max - bbox.x_min;
    let height = bbox.y_max - bbox.y_min;

    if width <= 0.0 || height <= 0.0 {
        return Ok(());
    }

    let dx = centroid.centroid_x - center_x;
    let dy = centroid.centroid_y - center_y;
    let dx_ratio = dx / width;
    let dy_ratio = dy / height;

    let aspect_ratio = if width / height < 1.0 {
        height / width
    } else {
        width / height
    };
    let is_long_strip = aspect_ratio >= 3.0;

    let sign_of = |v: f64| -> i32 {
        if v > 0.0 {
            1
        } else if v < 0.0 {
            -1
        } else {
            0
        }
    };
    let dx_sign = sign_of(dx);
    let dy_sign = sign_of(dy);

    // 期望符号为 0 表示「这一轴没有期望」，不参与反向判定。
    let dx_reversed = dx_sign != 0 && expected_dx_sign != 0 && dx_sign == -expected_dx_sign;
    let dy_reversed = dy_sign != 0 && expected_dy_sign != 0 && dy_sign == -expected_dy_sign;

    let dx_large_enough = dx_ratio.abs() >= min_offset_ratio;
    let dy_large_enough = dy_ratio.abs() >= min_offset_ratio;

    if !dx_large_enough && !dy_large_enough {
        // 判不出来就不判：模型太对称时宁可漏报，也不误报把人挡住。
        return Ok(());
    }

    let context = format!(
        "expected_dx_sign={expected_dx_sign} expected_dy_sign={expected_dy_sign} \
         actual_dx={dx:.4} actual_dy={dy:.4} dx_ratio={dx_ratio:.5} dy_ratio={dy_ratio:.5} \
         centroid=({:.4},{:.4}) center=({center_x:.4},{center_y:.4}) \
         bbox=[{:.4},{:.4}]x[{:.4},{:.4}] calibration_mode={calibration_mode} \
         aspect_ratio={aspect_ratio:.4} is_long_strip={is_long_strip} \
         source={error_source} bbox_source=original_gcode bbox_stage=before_pass2",
        centroid.centroid_x, centroid.centroid_y, bbox.x_min, bbox.x_max, bbox.y_min, bbox.y_max,
    );

    let diag_180 = |rotation_type: &str| PostprocError::CalibrationMismatch {
        code: "E_CAL_MISMATCH_001",
        message: format!(
            "检测到校准模型可能被旋转 180 度。请勿旋转校准模型。\
             （{context} rotation_type={rotation_type}）"
        ),
    };

    if dx_reversed && dy_reversed && dx_large_enough && dy_large_enough {
        return Err(diag_180("180_degree_both_axes"));
    }

    if is_long_strip {
        // 长条模型的短轴方向噪声大，只信长轴。
        let (long_axis_reversed, long_axis_large_enough) = if width >= height {
            (dx_reversed, dx_large_enough)
        } else {
            (dy_reversed, dy_large_enough)
        };
        if long_axis_reversed && long_axis_large_enough {
            return Err(diag_180("180_degree_long_strip"));
        }
    }

    if is_square_model && dx_reversed != dy_reversed && dx_large_enough && dy_large_enough {
        let magnitude_ok = if expected_relative_magnitude == 0 {
            true
        } else {
            let actual_magnitude = if dx.abs() > dy.abs() {
                1
            } else if dy.abs() > dx.abs() {
                -1
            } else {
                0
            };
            actual_magnitude == -expected_relative_magnitude
        };
        if magnitude_ok {
            return Err(PostprocError::CalibrationMismatch {
                code: "E_CAL_MISMATCH_001",
                message: format!(
                    "检测到校准模型可能被旋转。请勿旋转校准模型。\
                     （{context} is_square_model=true rotation_type=90_degree）"
                ),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 真机 XY 样例的 BBox（doc.md 第二节实测）。
    fn xy_sample_bbox() -> BBox {
        BBox {
            x_min: 66.023,
            x_max: 116.523,
            y_min: 66.340,
            y_max: 116.840,
        }
    }

    /// 判据用 IR：数值取自 `tests/reference/calibration/Precise.input.json`
    /// （travel 150、max_speed 12000、边界 ±999），这样本文件的手写判据与那份
    /// 参考夹具处在同一个数值语境里，两边对不上时能直接对照。
    fn test_ir() -> Ir {
        let mut ir = Ir::default();
        ir.machine.min_x = -999.0;
        ir.machine.max_x = 999.0;
        ir.machine.min_y = -999.0;
        ir.machine.max_y = 999.0;
        ir.machine.first_layer_height = 0.2;
        ir.machine.travel_speed = 150.0;
        ir.toolhead.max_speed = 12000.0;
        ir.safety.xy_calibration = "new".to_string();
        ir.safety.z_calibration = "new".to_string();
        ir
    }

    fn xy_new(mode: &str, mt: &str, ir: &Ir, bbox: &BBox) -> Vec<String> {
        generate_xy_calibration_new(mode, mt, ir, bbox, CentroidResult::default(), 3000)
            .expect("生成失败")
            .expect("New 变体不该返回空")
    }

    /// 结构与坐标：22 轮（11 横 + 11 竖），每轮 7 行，头 6 尾 3。
    /// 坐标全部相对 `bbox` 左下角，不碰机型固定坐标。
    #[test]
    fn xy_new_structure_and_coordinates() {
        let ir = test_ir();
        let b = xy_sample_bbox();
        // P1S 没有 CALIBE_SING，行数干净好数
        let out = xy_new("Precise", "P1S", &ir, &b);

        assert_eq!(out.len(), 6 + 22 * 7 + 3, "行数不对: {}", out.len());
        assert_eq!(out[0], "; XY Offset Calibration");
        assert_eq!(out[1], CALIBRATION_HOME_LINE);
        assert_eq!(out[2], ";Rising nozzle to avoid collision");
        assert_eq!(out[3], "G1 Z6.2"); // 0.2 + 0 + 6
        assert_eq!(out[4], ";Mounting Toolhead");
        assert_eq!(out[5], ";Toolhead Mounted");

        // 第一轮横线：起点 X = x_min + 0.5，Y = y_min + 10.49 + cali(-1.0)
        assert_eq!(out[6], "G1 F9000.0"); // travel 150 * 60
        assert_eq!(out[7], "G1 X66.523 Y75.83");
        assert_eq!(out[8], "G1 Z3.2"); // 0.2 + 0 + 3
        assert_eq!(out[9], "G1 F300"); // max_speed > 10 走这条
        assert_eq!(out[10], "G1 X76.523 Y75.83 Z0.2"); // 终点 X = x_min + 10.5
        assert_eq!(out[11], "G4 P3000");
        assert_eq!(out[12], "G1 Z3.2");

        // 第二轮：Y 再 +4.0（图案间距），cali 再 +0.2 ⇒ 净 +4.2
        assert_eq!(out[14], "G1 X66.523 Y80.03");

        // 第一轮竖线（第 12 轮，索引 6 + 11*7 = 83）：X = x_min + 10.5 + cali(-1.0)
        assert_eq!(out[84], "G1 X75.523 Y66.83"); // Y = y_min + 0.49
        assert_eq!(out[87], "G1 X75.523 Y76.83 Z0.2"); // 终点 Y = y_min + 10.49

        // 每轮都有停留 —— 这是 New 与经典分支最容易漏的差异（经典分支根本没有 G4）
        assert_eq!(out.iter().filter(|l| l.as_str() == "G4 P3000").count(), 22);

        assert_eq!(out[out.len() - 3], ";Unmounting Toolhead");
        assert_eq!(out[out.len() - 2], ";Toolhead Unmounted");
        assert_eq!(out[out.len() - 1], "G1 X100 Y100 Z100");
    }

    /// Rough 的起始偏移是 −2.5、步进 +0.5（Precise 是 −1.0 / +0.2）。
    #[test]
    fn xy_new_rough_uses_coarser_steps() {
        let ir = test_ir();
        let b = xy_sample_bbox();
        let out = xy_new("Rough", "P1S", &ir, &b);
        assert_eq!(out[7], "G1 X66.523 Y74.33"); // 76.83 − 2.5
        assert_eq!(out[14], "G1 X66.523 Y78.83"); // +4.0 −0.5 ⇒ +3.5
    }

    /// **交叉验证**：真机样例正好摆在机型默认校准位（bbox.x_min + 0.5 = A1_MINI 的
    /// `y_line_x`），所以 New 分支的输出应当与经典分支**逐行相同，只多出 G4 停留行**。
    ///
    /// 这条判据的价值在于：它用一份已经逐字节钉住的参考（Precise.expected.gcode）
    /// 反过来验证「固定图案偏移常量」是对的 —— 10.49 / 0.5 / 10.5 / 0.49 这些数
    /// 本来就是从机型默认坐标反推出来的。
    #[test]
    fn xy_new_equals_classic_plus_dwell_when_model_sits_at_default_spot() {
        let mut ir = test_ir();
        let b = xy_sample_bbox();
        let new_out = xy_new("Precise", "A1_MINI", &ir, &b);

        ir.safety.xy_calibration = String::new();
        let classic = generate_calibration_gcode_with_centroid(
            "Precise",
            "A1_MINI",
            &ir,
            None,
            CentroidResult::default(),
            2,
            3,
        )
        .expect("经典分支生成失败")
        .expect("经典分支返回空");

        let new_without_dwell: Vec<&String> =
            new_out.iter().filter(|l| !l.starts_with("G4 P")).collect();
        assert_eq!(
            new_without_dwell.len(),
            classic.len(),
            "去掉 G4 后行数应与经典分支一致"
        );
        for (i, (n, c)) in new_without_dwell.iter().zip(classic.iter()).enumerate() {
            assert_eq!(n.as_str(), c.as_str(), "第 {} 行与经典分支不一致", i + 1);
        }
    }

    /// 模型被缩放 ⇒ `E_CAL_MISMATCH_002`，且文案说的是人能照做的事。
    #[test]
    fn xy_new_rejects_scaled_model() {
        let ir = test_ir();
        let scaled = BBox {
            x_min: 66.023,
            x_max: 66.023 + 60.0, // 50.5 → 60.0
            y_min: 66.340,
            y_max: 66.340 + 60.0,
        };
        let err = generate_xy_calibration_new(
            "Precise",
            "P1S",
            &ir,
            &scaled,
            CentroidResult::default(),
            3000,
        )
        .unwrap_err();
        assert_eq!(err.code(), "E_CAL_MISMATCH_002");
        assert!(err.to_string().contains("请勿缩放校准模型"), "{err}");
    }

    /// 0.5 mm 以内的尺寸误差不算缩放（切片器的挤出宽度会让 BBox 有零点几毫米抖动）。
    #[test]
    fn xy_new_tolerates_half_mm_size_noise() {
        let ir = test_ir();
        let noisy = BBox {
            x_min: 66.023,
            x_max: 66.023 + 50.9, // +0.4，仍在 0.5 容差内
            y_min: 66.340,
            y_max: 66.340 + 50.2,
        };
        assert!(
            generate_xy_calibration_new(
                "Precise",
                "P1S",
                &ir,
                &noisy,
                CentroidResult::default(),
                3000
            )
            .is_ok()
        );
    }

    /// 质心判定接进来了：整体旋转 180° 的质心必须让生成失败。
    #[test]
    fn xy_new_wires_up_the_centroid_check() {
        let ir = test_ir();
        let b = xy_sample_bbox();
        let cx = (b.x_min + b.x_max) / 2.0;
        let cy = (b.y_min + b.y_max) / 2.0;
        let mirrored = CentroidResult {
            centroid_x: cx + (cx - 87.7831),
            centroid_y: cy + (cy - 82.5736),
            valid: true,
        };
        let err =
            generate_xy_calibration_new("Precise", "P1S", &ir, &b, mirrored, 3000).unwrap_err();
        assert_eq!(err.code(), "E_CAL_MISMATCH_001");

        // valid=false 时不判（调用方语义）
        let ignored = CentroidResult {
            centroid_x: mirrored.centroid_x,
            centroid_y: mirrored.centroid_y,
            valid: false,
        };
        assert!(
            generate_xy_calibration_new("Precise", "P1S", &ir, &b, ignored, 3000).is_ok(),
            "valid=false 的质心不该参与判定"
        );
    }

    /// New 变体对 mount/unmount 行**逐行过 `filter_e_moves`**（经典分支不做）。
    /// 带真实挤出的行会被整行丢掉，虚拟挤出行留着。
    #[test]
    fn xy_new_filters_e_moves_from_mount_gcode() {
        let mut ir = test_ir();
        ir.toolhead.custom_mount_gcode = vec![
            "M104 S200".to_string(),
            "G1 X10 Y10 E5".to_string(),
            "   ".to_string(),
        ];
        ir.toolhead.custom_unmount_gcode = vec!["G1 E-2".to_string(), "M104 S0".to_string()];
        let out = xy_new("Precise", "P1S", &ir, &xy_sample_bbox());

        let mounted = out.iter().position(|l| l == ";Toolhead Mounted").unwrap();
        let mount_block = &out[5..mounted];
        assert_eq!(mount_block, ["M104 S200"], "带 E 的挂载行没被过滤掉");

        let unmounting = out
            .iter()
            .position(|l| l == ";Unmounting Toolhead")
            .unwrap();
        let unmounted = out.iter().position(|l| l == ";Toolhead Unmounted").unwrap();
        assert_eq!(&out[unmounting + 1..unmounted], ["M104 S0"]);
    }

    /// 造一个 BBox：左下角 (x, y) + 宽高。
    fn bbox_at(x: f64, y: f64, w: f64, h: f64) -> BBox {
        BBox {
            x_min: x,
            x_max: x + w,
            y_min: y,
            y_max: y + h,
        }
    }

    /// 造一个相对 BBox 中心偏移 (dx, dy) 的质心。
    fn centroid_offset(b: &BBox, dx: f64, dy: f64) -> CentroidResult {
        CentroidResult {
            centroid_x: (b.x_min + b.x_max) / 2.0 + dx,
            centroid_y: (b.y_min + b.y_max) / 2.0 + dy,
            valid: true,
        }
    }

    fn xy_check(b: &BBox, c: CentroidResult) -> Result<(), PostprocError> {
        check_rotation_by_centroid(
            c,
            b,
            -1,
            -1,
            "Precise",
            0.02,
            "generate_xy_calibration_new",
            true,
            0,
        )
    }

    fn z_check(b: &BBox, c: CentroidResult) -> Result<(), PostprocError> {
        check_rotation_by_centroid(
            c,
            b,
            1,
            0,
            "ZOffset",
            0.01,
            "generate_z_offset_calibration_new",
            false,
            0,
        )
    }

    /// 真机 Z 样例的 BBox（doc.md 第二节实测）。
    fn z_sample_bbox() -> BBox {
        BBox {
            x_min: 30.131,
            x_max: 149.711,
            y_min: 84.618,
            y_max: 97.681,
        }
    }

    fn z_new(mt: &str, ir: &Ir, bbox: &BBox) -> Vec<String> {
        generate_z_offset_calibration_new(mt, ir, bbox, CentroidResult::default(), 2000)
            .expect("生成失败")
            .expect("New 变体不该返回空")
    }

    /// 结构与原点：11 个方块，原点 = bbox 左下角 + (0.079, 3.755)。
    #[test]
    fn z_new_structure_and_origin() {
        let ir = test_ir();
        let out = z_new("P1S", &ir, &z_sample_bbox()); // P1S 没有 Floating Z、没有音乐

        let per_round = Z_OFFSET_SING_LINES.len() + 6;
        assert_eq!(
            out.len(),
            6 + Z_SQUARE_COUNT * per_round + 3,
            "行数不对: {}",
            out.len()
        );
        assert_eq!(out[0], "; ZOffset Calibration");
        assert_eq!(out[3], "G1 Z6.2");
        assert_eq!(out[5], ";Toolhead Mounted");
        assert!(
            !out.iter().any(|l| l == ";Floating Z Calibration"),
            "P1S 不该有 Floating Z"
        );

        // 每个方块一次停留
        assert_eq!(
            out.iter().filter(|l| l.as_str() == "G4 P2000").count(),
            Z_SQUARE_COUNT
        );
        // 第一个方块的层高：0.4 + z_offset + glue + z_accumulate(0.5) = 0.9
        assert_eq!(out[9], "G1 Z0.9");
        // 第二个方块：z_accumulate 降到 0.4 ⇒ 0.8
        assert_eq!(out[9 + per_round], "G1 Z0.8");
        // 每轮抬回 z_offset + 6（`format_z` 对整数补 `.0`，所以是 `Z6.0` 不是 `Z6`）
        assert_eq!(out[6 + per_round - 2], "G1 Z6.0");

        assert_eq!(out[out.len() - 1], "G1 X100 Y100 Z100");
    }

    /// **交叉验证**：A1_MINI 的 `zStartX/zStartY` 恰好是 30.21 / 88.373，
    /// 而真机样例的 `bbox.x_min + 0.079` / `bbox.y_min + 3.755` 也正是这两个数。
    /// 于是 Z New 分支的输出应与经典分支**逐行完全相同**（Z 经典分支本来就有 G4，
    /// 不像 XY 那样差停留行）。
    #[test]
    fn z_new_equals_classic_when_model_sits_at_default_spot() {
        let mut ir = test_ir();
        let new_out = z_new("A1_MINI", &ir, &z_sample_bbox());

        ir.safety.z_calibration = String::new();
        let classic = generate_calibration_gcode_with_centroid(
            "ZOffset",
            "A1_MINI",
            &ir,
            None,
            CentroidResult::default(),
            2,
            3,
        )
        .expect("经典分支生成失败")
        .expect("经典分支返回空");

        assert_eq!(new_out.len(), classic.len(), "行数应与经典分支一致");
        for (i, (n, c)) in new_out.iter().zip(classic.iter()).enumerate() {
            assert_eq!(n.as_str(), c.as_str(), "第 {} 行不一致", i + 1);
        }
    }

    /// Z 图案非正方形（119.58×13.063），宽高互换 ⇒ 检测一命中，报旋转。
    /// 这条是 XY 拿不到的能力（XY 是正方形，检测一恒不触发）。
    #[test]
    fn z_new_detects_swapped_dimensions_as_rotation() {
        let ir = test_ir();
        let rotated = BBox {
            x_min: 30.131,
            x_max: 30.131 + Z_PATTERN_MODEL_HEIGHT,
            y_min: 84.618,
            y_max: 84.618 + Z_PATTERN_MODEL_WIDTH,
        };
        let err = generate_z_offset_calibration_new(
            "P1S",
            &ir,
            &rotated,
            CentroidResult::default(),
            2000,
        )
        .unwrap_err();
        assert_eq!(err.code(), "E_CAL_MISMATCH_001");
        assert!(err.to_string().contains("请勿旋转校准模型"), "{err}");
    }

    /// 模型被缩放 ⇒ `E_CAL_MISMATCH_002`。
    #[test]
    fn z_new_rejects_scaled_model() {
        let ir = test_ir();
        let scaled = BBox {
            x_min: 30.131,
            x_max: 30.131 + 100.0,
            y_min: 84.618,
            y_max: 84.618 + Z_PATTERN_MODEL_HEIGHT,
        };
        let err =
            generate_z_offset_calibration_new("P1S", &ir, &scaled, CentroidResult::default(), 2000)
                .unwrap_err();
        assert_eq!(err.code(), "E_CAL_MISMATCH_002");
    }

    /// Floating Z 只给 A1 / A1_MINI，且对齐点是原点 +5/+5。
    #[test]
    fn z_new_floating_z_is_a1_family_only() {
        let ir = test_ir();
        let a1 = z_new("A1", &ir, &z_sample_bbox());
        let idx = a1
            .iter()
            .position(|l| l == ";Floating Z Calibration")
            .expect("A1 应有 Floating Z");
        // 原点 (30.21, 88.373) + 5 ⇒ (35.21, 93.373)
        assert_eq!(a1[idx + 2], "G1 X35.21 Y93.373");
        assert_eq!(
            a1.iter().filter(|l| l.as_str() == "G1 Z3.4").count(),
            5,
            "Floating Z 应是 5 组 Z3.4/Z5"
        );

        let p1s = z_new("P1S", &ir, &z_sample_bbox());
        assert!(!p1s.iter().any(|l| l == ";Floating Z Calibration"));
    }

    /// Z 的质心判定用的是 `(dx>0, dy 无期望)` + 0.01 阈值 + 非正方形。
    #[test]
    fn z_new_wires_up_the_centroid_check() {
        let ir = test_ir();
        let b = z_sample_bbox();
        let cx = (b.x_min + b.x_max) / 2.0;
        let cy = (b.y_min + b.y_max) / 2.0;
        let mirrored = CentroidResult {
            centroid_x: cx + (cx - 93.4280),
            centroid_y: cy + (cy - 91.0501),
            valid: true,
        };
        let err = generate_z_offset_calibration_new("P1S", &ir, &b, mirrored, 2000).unwrap_err();
        assert_eq!(err.code(), "E_CAL_MISMATCH_001");
    }

    /// 分派：`xy_calibration == "new"` 且有 BBox ⇒ 走 New 变体（特征是每轮 `G4` 停留）。
    /// 这条曾经是「响亮报错」的位置，现在必须产出 G-code。
    #[test]
    fn dispatch_routes_new_flag_with_bbox_to_new_variant() {
        let ir = test_ir();
        let xy = generate_calibration_gcode_with_centroid(
            "Precise",
            "P1S",
            &ir,
            Some(&xy_sample_bbox()),
            CentroidResult::default(),
            2,
            3,
        )
        .expect("XY New 分派仍在报错")
        .expect("XY New 分派返回空");
        assert_eq!(
            xy.iter().filter(|l| l.as_str() == "G4 P3000").count(),
            22,
            "没走到 New 分支"
        );

        let z = generate_calibration_gcode_with_centroid(
            "ZOffset",
            "P1S",
            &ir,
            Some(&z_sample_bbox()),
            CentroidResult::default(),
            2,
            3,
        )
        .expect("Z New 分派仍在报错")
        .expect("Z New 分派返回空");
        assert_eq!(
            z.iter().filter(|l| l.as_str() == "G4 P2000").count(),
            Z_SQUARE_COUNT
        );
    }

    /// 没有 BBox（非动态路径）⇒ 落经典分支，即便 IR 里写着 `"new"`。
    /// XY 经典分支不消费 dwell，所以「没有 G4」正是它的指纹。
    #[test]
    fn dispatch_falls_back_to_classic_without_bbox() {
        let ir = test_ir();
        let xy = generate_calibration_gcode_with_centroid(
            "Precise",
            "A1_MINI",
            &ir,
            None,
            CentroidResult::default(),
            2,
            3,
        )
        .expect("经典分支报错")
        .expect("经典分支返回空");
        assert!(
            !xy.iter().any(|l| l.starts_with("G4 P")),
            "无 BBox 时不该走 New 分支"
        );
    }

    /// 宽或高 ≤ 0 直接放过（generator_new.go:390-392）——退化 BBox 上判方向没有意义。
    #[test]
    fn degenerate_bbox_passes() {
        let b = bbox_at(10.0, 10.0, 0.0, 50.0);
        let c = CentroidResult {
            centroid_x: 999.0,
            centroid_y: 999.0,
            valid: true,
        };
        assert!(xy_check(&b, c).is_ok());
    }

    /// 两轴偏移比都不够大 ⇒ 放过（generator_new.go:426-429）。
    /// 立场是「判不出来就不判」：宁可漏报，也不因为模型本身对称就拦住用户。
    #[test]
    fn tiny_offsets_pass_even_when_reversed() {
        let b = bbox_at(0.0, 0.0, 50.5, 50.5);
        // 期望 (-1,-1)，这里给 (+,+) 即两轴都反向，但比值只有 0.0099 < 0.02
        let c = centroid_offset(&b, 0.5, 0.5);
        assert!(xy_check(&b, c).is_ok());
    }

    /// 双轴都反向且都够大 ⇒ 180°（generator_new.go:461-465）。
    #[test]
    fn both_axes_reversed_is_180() {
        let b = bbox_at(0.0, 0.0, 50.5, 50.5);
        let c = centroid_offset(&b, 5.0, 5.0);
        let err = xy_check(&b, c).unwrap_err();
        assert_eq!(err.code(), "E_CAL_MISMATCH_001");
        assert!(err.to_string().contains("180"), "文案未提 180 度: {err}");
        assert!(
            err.to_string().contains("180_degree_both_axes"),
            "缺 rotation_type 上下文: {err}"
        );
    }

    /// 长条模型（长宽比 ≥ 3.0）只看长轴（generator_new.go:467-482）。
    /// Z 校准模型 119.58×13.063 长宽比 9.15，短轴的方向噪声很大，不能拿来判。
    #[test]
    fn long_strip_judges_only_the_long_axis() {
        let b = bbox_at(0.0, 0.0, 119.58, 13.063);
        // 期望 dx>0，这里给 dx<0（长轴反向），比值 0.042 > 0.01
        let c = centroid_offset(&b, -5.0, 0.0);
        let err = z_check(&b, c).unwrap_err();
        assert_eq!(err.code(), "E_CAL_MISMATCH_001");
        assert!(
            err.to_string().contains("180_degree_long_strip"),
            "应判为长条 180°: {err}"
        );
    }

    /// 正方形模型的单轴反向 ⇒ 90°（generator_new.go:484-526）。
    /// XY 图案是 50.5×50.5，转 90° 后 BBox 一模一样，只有质心方向能看出来。
    #[test]
    fn square_model_single_axis_reversed_is_90() {
        let b = bbox_at(0.0, 0.0, 50.5, 50.5);
        // 期望 (-1,-1)：dx 反向（+）、dy 不反向（−），两轴比值都 > 0.02
        let c = centroid_offset(&b, 5.0, -5.0);
        let err = xy_check(&b, c).unwrap_err();
        assert_eq!(err.code(), "E_CAL_MISMATCH_001");
        assert!(err.to_string().contains("90_degree"), "应判为 90°: {err}");
    }

    /// 非正方形、非长条模型的单轴反向**不判** —— `is_square_model` 为 false
    /// 时那条支路整块跳过（generator_new.go:484）。
    #[test]
    fn non_square_single_axis_reversed_passes() {
        let b = bbox_at(0.0, 0.0, 40.0, 30.0);
        // 长宽比 1.33 < 3.0，不是长条；dy 期望符号为 0（无期望）⇒ dy 不算反向
        let c = centroid_offset(&b, -5.0, 5.0);
        assert!(z_check(&b, c).is_ok());
    }

    /// 本函数不看 `valid` 字段（那是调用方的判断），真调进来也必须不 panic。
    #[test]
    fn invalid_centroid_is_the_callers_business() {
        let b = bbox_at(0.0, 0.0, 50.5, 50.5);
        let c = CentroidResult {
            centroid_x: (b.x_min + b.x_max) / 2.0 - 5.0,
            centroid_y: (b.y_min + b.y_max) / 2.0 - 5.0,
            valid: false,
        };
        // dx/dy 都与期望同向 ⇒ 放过
        assert!(xy_check(&b, c).is_ok());
    }

    /// 真机实测值（2026-09-08，doc.md 第二节）必须放过 —— 摆放正确的模型不许被拦。
    #[test]
    fn real_machine_samples_pass() {
        // XY_PLA_6m48s.gcode：bbox x[66.023,116.523] y[66.340,116.840]，质心 (87.7831, 82.5736)
        let xy = BBox {
            x_min: 66.023,
            x_max: 116.523,
            y_min: 66.340,
            y_max: 116.840,
        };
        let xy_centroid = CentroidResult {
            centroid_x: 87.7831,
            centroid_y: 82.5736,
            valid: true,
        };
        assert!(
            xy_check(&xy, xy_centroid).is_ok(),
            "实测 XY 样例被误判为旋转"
        );

        // ZOffset Calibration_PLA_9m11s.gcode：bbox x[30.131,149.711] y[84.618,97.681]，
        // 质心 (93.4280, 91.0501)
        let z = BBox {
            x_min: 30.131,
            x_max: 149.711,
            y_min: 84.618,
            y_max: 97.681,
        };
        let z_centroid = CentroidResult {
            centroid_x: 93.4280,
            centroid_y: 91.0501,
            valid: true,
        };
        assert!(z_check(&z, z_centroid).is_ok(), "实测 Z 样例被误判为旋转");
    }

    /// 实测样例整体旋转 180°（绕 BBox 中心镜像质心）必须被抓住 ——
    /// 正例放过不等于反例能抓，两头都要钉。
    #[test]
    fn real_machine_samples_rotated_180_are_caught() {
        let xy = BBox {
            x_min: 66.023,
            x_max: 116.523,
            y_min: 66.340,
            y_max: 116.840,
        };
        let cx = (xy.x_min + xy.x_max) / 2.0;
        let cy = (xy.y_min + xy.y_max) / 2.0;
        let mirrored = CentroidResult {
            centroid_x: cx + (cx - 87.7831),
            centroid_y: cy + (cy - 82.5736),
            valid: true,
        };
        let err = xy_check(&xy, mirrored).unwrap_err();
        assert_eq!(err.code(), "E_CAL_MISMATCH_001");
        assert!(err.to_string().contains("180_degree_both_axes"), "{err}");

        let z = BBox {
            x_min: 30.131,
            x_max: 149.711,
            y_min: 84.618,
            y_max: 97.681,
        };
        let zcx = (z.x_min + z.x_max) / 2.0;
        let zcy = (z.y_min + z.y_max) / 2.0;
        let z_mirrored = CentroidResult {
            centroid_x: zcx + (zcx - 93.4280),
            centroid_y: zcy + (zcy - 91.0501),
            valid: true,
        };
        let err = z_check(&z, z_mirrored).unwrap_err();
        assert_eq!(err.code(), "E_CAL_MISMATCH_001");
    }
}
