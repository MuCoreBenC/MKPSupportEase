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

pub use calibration::{
    BBox, BlockOrnaments, CALIBRATION_SAFE_LIFT_MAX_MM, CALIBRATION_SAFE_LIFT_MIN_MM,
    CentroidResult, generate_calibration_gcode_with_centroid,
    generate_calibration_gcode_with_centroid_rec,
};
pub use collision::{
    AdjustmentTarget, BoundaryPiece, CollisionFacts, CollisionResult, CollisionType,
    ModelFootprint, ModelMapFacts, MoveAttempt, TowerAsk, TowerAskOutcome,
    auto_tower_position_for_facts, detect_tower_collision, model_footprint, model_map_facts,
    model_pieces_beyond,
};

/// processor.ComputeBBox（对照 bbox_compute.go / model_z_map.go:59）：
/// 绝对定位模式下挤出移动（G1 带 E）的 2D 包围盒；找不到有效坐标报错。
pub fn compute_bbox(lines: &[String]) -> Result<BBox, crate::diag::PostprocError> {
    compute_bbox_range(lines, 0..lines.len())
}

/// 只在一段行区间里算包围盒 —— 口径与 [`compute_bbox`] 完全一致，只是范围受限。
pub fn compute_bbox_range(
    lines: &[String],
    range: std::ops::Range<usize>,
) -> Result<BBox, crate::diag::PostprocError> {
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    let mut found = false;
    let mut relative_mode = false;
    for line in &lines[range] {
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
    compute_extrusion_centroid_range(lines, 0..lines.len())
}

/// 只在一段行区间里算挤出质心 —— 与 [`compute_extrusion_centroid`] 同一口径。
pub fn compute_extrusion_centroid_range(
    lines: &[String],
    range: std::ops::Range<usize>,
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

    for line in &lines[range] {
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

/// 一盘中按对象分段后的一块（`; start printing object, unique label id: N` 起、
/// `; stop printing object` 止）。同一个 label 出现多段时合并成一块。
#[derive(Debug, Clone, PartialEq)]
pub struct PlateSegment {
    pub label: String,
    pub bbox: BBox,
    pub centroid: CentroidResult,
}

/// 按对象标签分段，逐块算包围盒与挤出质心。
///
/// 只统计绝对定位下带 `E` 的 `G1`（与 [`compute_bbox`] 同一条规矩）。
/// 没有分段标记 ⇒ 返回空，调用方退回整盘口径。
/// 缺 `stop` 的那一段整个丢掉：宁可少一块，也不把两段算成一块。
pub fn segment_objects(lines: &[String]) -> Vec<PlateSegment> {
    struct Acc {
        label: String,
        closed: bool,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        sum_x: f64,
        sum_y: f64,
        sum_e: f64,
        found: bool,
    }

    let mut accs: Vec<Acc> = Vec::new();
    let mut active: Option<usize> = None;
    let mut relative_mode = false;
    /* 质心按 ΔE 加权：E 是整条流连续的，跨段不能重置 —— 所以这一遍就顺带算完。 */
    let mut current_e = 0.0;
    let mut has_e = false;
    let mut current_x = 0.0;
    let mut current_y = 0.0;
    let mut has_x = false;
    let mut has_y = false;

    for line in lines {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("; start printing object") {
            let label = rest.rsplit(':').next().map(str::trim).unwrap_or_default();
            if label.is_empty() {
                active = None;
                continue;
            }
            let idx = match accs.iter().position(|a| a.label == label) {
                Some(i) => i,
                None => {
                    accs.push(Acc {
                        label: label.to_string(),
                        closed: false,
                        min_x: f64::MAX,
                        min_y: f64::MAX,
                        max_x: f64::MIN,
                        max_y: f64::MIN,
                        sum_x: 0.0,
                        sum_y: 0.0,
                        sum_e: 0.0,
                        found: false,
                    });
                    accs.len() - 1
                }
            };
            active = Some(idx);
            continue;
        }
        if trimmed.starts_with("; stop printing object") {
            if let Some(i) = active.take() {
                accs[i].closed = true;
            }
            continue;
        }
        if trimmed == "G90" {
            relative_mode = false;
            continue;
        }
        if trimmed == "G91" {
            relative_mode = true;
            continue;
        }
        if !line.starts_with("G1 ") || relative_mode {
            continue;
        }
        let Some(e_val) = crate::gcode::extract_coord(line.as_bytes(), b'E') else {
            continue;
        };
        let x = crate::gcode::extract_coord(line.as_bytes(), b'X');
        let y = crate::gcode::extract_coord(line.as_bytes(), b'Y');
        if let Some(v) = x {
            current_x = v;
            has_x = true;
        }
        if let Some(v) = y {
            current_y = v;
            has_y = true;
        }
        if let Some(i) = active {
            let a = &mut accs[i];
            if let Some(v) = x {
                a.min_x = a.min_x.min(v);
                a.max_x = a.max_x.max(v);
                a.found = true;
            }
            if let Some(v) = y {
                a.min_y = a.min_y.min(v);
                a.max_y = a.max_y.max(v);
                a.found = true;
            }
            if has_e && has_x && has_y {
                let delta_e = e_val - current_e;
                if delta_e > 0.0 {
                    a.sum_x += current_x * delta_e;
                    a.sum_y += current_y * delta_e;
                    a.sum_e += delta_e;
                }
            }
        }
        current_e = e_val;
        has_e = true;
    }

    accs.into_iter()
        .filter(|a| a.closed && a.found)
        .map(|a| PlateSegment {
            label: a.label,
            bbox: BBox {
                x_min: a.min_x,
                x_max: a.max_x,
                y_min: a.min_y,
                y_max: a.max_y,
            },
            centroid: if a.sum_e > 0.0 {
                CentroidResult {
                    centroid_x: a.sum_x / a.sum_e,
                    centroid_y: a.sum_y / a.sum_e,
                    valid: true,
                }
            } else {
                CentroidResult::default()
            },
        })
        .collect()
}
