//! 碰撞检测 + 校准几何（对照 internal/disk 的 collision.go / generator.go /
//! generator_new.go）。
//!
//! wipe.go（圆盘擦拭）+ sdk.go 的段拆分已随 Task 12 pass2 落地（见 wipe.rs）。
//! generator_new.go 的两个 *New 变体（XY / ZOffset 的 BBox 动态定位）已于
//! 2026-09-08 移植，字节级参考见 tests/reference/calibration/*.new.*。
//! 刻意不移植（登记于提交信息）：
//! - connection.go——GUI 装配层。

pub mod calibration;
pub mod collision;
pub mod wipe;

pub use calibration::{BBox, CentroidResult, generate_calibration_gcode_with_centroid};
pub use collision::{
    AdjustmentTarget, CollisionResult, CollisionType, MoveAttempt, detect_tower_collision,
};

/// processor.ComputeBBox（对照 bbox_compute.go / model_z_map.go:59）：
/// 绝对定位模式下挤出移动（G1 带 E）的 2D 包围盒；找不到有效坐标报错。
pub fn compute_bbox(lines: &[String]) -> Result<BBox, crate::diag::PostprocError> {
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    let mut found = false;
    let mut relative_mode = false;
    for line in lines {
        let trimmed = line.trim();
        if trimmed == "G90" {
            relative_mode = false;
            continue;
        }
        if trimmed == "G91" {
            relative_mode = true;
            continue;
        }
        if !line.starts_with("G1 ") {
            continue;
        }
        if relative_mode {
            continue;
        }
        if crate::gcode::extract_coord(line.as_bytes(), b'E').is_none() {
            continue;
        }
        if let Some(x) = crate::gcode::extract_coord(line.as_bytes(), b'X') {
            if x < min_x {
                min_x = x;
            }
            if x > max_x {
                max_x = x;
            }
            found = true;
        }
        if let Some(y) = crate::gcode::extract_coord(line.as_bytes(), b'Y') {
            if y < min_y {
                min_y = y;
            }
            if y > max_y {
                max_y = y;
            }
            found = true;
        }
    }
    if !found || min_x > max_x {
        return Err(crate::diag::PostprocError::Processing {
            message: "无法从GCode中计算模型边界框：未找到有效的挤出坐标".to_string(),
        });
    }
    Ok(BBox {
        x_min: min_x,
        x_max: max_x,
        y_min: min_y,
        y_max: max_y,
    })
}

/// processor.ComputeExtrusionCentroid：按 ΔE 加权的挤出质心。
pub fn compute_extrusion_centroid(
    lines: &[String],
) -> Result<(f64, f64), crate::diag::PostprocError> {
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    let mut sum_e = 0.0;
    let mut current_e = 0.0;
    let mut current_x = 0.0;
    let mut current_y = 0.0;
    let mut has_e = false;
    let mut has_x = false;
    let mut has_y = false;
    let mut relative_mode = false;

    for line in lines {
        let trimmed = line.trim();
        if trimmed == "G90" {
            relative_mode = false;
            continue;
        }
        if trimmed == "G91" {
            relative_mode = true;
            continue;
        }
        if !line.starts_with("G1 ") {
            continue;
        }
        if relative_mode {
            continue;
        }
        let Some(e_val) = crate::gcode::extract_coord(line.as_bytes(), b'E') else {
            continue;
        };
        if let Some(x) = crate::gcode::extract_coord(line.as_bytes(), b'X') {
            current_x = x;
            has_x = true;
        }
        if let Some(y) = crate::gcode::extract_coord(line.as_bytes(), b'Y') {
            current_y = y;
            has_y = true;
        }
        if has_e && has_x && has_y {
            let delta_e = e_val - current_e;
            if delta_e > 0.0 {
                sum_x += current_x * delta_e;
                sum_y += current_y * delta_e;
                sum_e += delta_e;
            }
        }
        current_e = e_val;
        has_e = true;
    }

    if sum_e <= 0.0 {
        return Err(crate::diag::PostprocError::Processing {
            message: "无法从GCode中计算挤出质心：未找到有效的挤出点".to_string(),
        });
    }
    Ok((sum_x / sum_e, sum_y / sum_e))
}
