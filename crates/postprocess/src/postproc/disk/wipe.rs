// 圆盘擦拭生成器（对照 internal/disk/wipe.go + sdk.go 逐行移植）。
//
// 结构：geometry 子块（ConvexHull/ComputeNormals，对照 internal/geometry/convex_hull.go）
// → 选位与评分（GenerateDiskWipe 主体）→ 三种连接线 → Complex Wipe → SDK 段拆分。
// pass2 对本模块的调用点：;Pre-glue preparation（连接线提前）与
// ;Prepare for next tower（handlePrepareNextTower，绘制+退出段延后）。
#![allow(
    clippy::too_many_lines,
    clippy::collapsible_if,
    clippy::question_mark,
    clippy::ptr_arg
)] // Go 同形结构，不改写（let-else 与 Go 的 early-return 同形）

use crate::gcode::{format_e, format_python_float, format_speed, format_speed_int, math_round};
use crate::ir::{Ir, num_strip};

use crate::postproc::machine_dims::get_machine_dimensions;
use crate::postproc::tower::FILAMENT_DIAMETER;

pub(crate) const MM_PER_MINUTE: f64 = crate::postproc::offset::MM_PER_MINUTE;

// ---- 常量表（wipe.go:15-78） ----
pub const DISK_DIAMETER: f64 = 4.5;
pub const DISK_RADIUS: f64 = DISK_DIAMETER / 2.0;
pub const CHECK_RADIUS: f64 = DISK_RADIUS + 1.0;
pub const EXTENSION_LENGTH: f64 = 4.0;
pub const N_GON: usize = 10;

pub const WIPE_COLLISION_THRESHOLD: i64 = 3;
pub const WIPE_LAYER_DIFF_THRESHOLD: f64 = 5.0;
pub const WIPE_HISTORY_MAX_ENTRIES: usize = 19;
pub const WIPE_Z_LIFT_HEIGHT: f64 = 0.2;
pub const WIPE_RETRACT_SPEED: i64 = 1500;
pub const WIPE_DISK_INIT_SPEED: f64 = 500.0;
pub const WIPE_FILL_SPEED: f64 = 6000.0;
pub const WIPE_TPU_FILL_SPEED: f64 = 1800.0;
pub const WIPE_FILL_LAYER_COEFF: f64 = 0.4;
pub const WIPE_RADIUS_TOLERANCE: f64 = 0.01;
pub const WIPE_MIN_LINE_LENGTH: f64 = 0.05;
pub const WIPE_OUTER_RING_MIN_SEGS: f64 = 32.0;
pub const WIPE_OUTER_RING_SEG_LENGTH: f64 = 0.5;
pub const WIPE_SPEED_CAP: f64 = 200.0;
pub const WIPE_RETRACT_RATIO: f64 = 0.7;
pub const WIPE_END_RETRACT_RATIO: f64 = 0.3;
pub const WIPE_COOL_Z_LIFT: f64 = 0.3;
pub const WIPE_COOL_Z_SPEED: i64 = 1200;
pub const WIPE_PLA_COOL_TEMP: i64 = 180;
pub const WIPE_ABS_COOL_TEMP: i64 = 210;
pub const WIPE_PETG_COOL_TEMP: i64 = 210;
pub const WIPE_CONNECTION_LAYER_COEFF: f64 = 0.7;
pub const WIPE_TARGET_LENGTH_EXTRA: f64 = 2.0;
pub const WIPE_PLA_TEMP_OFFSET: f64 = 40.0;
pub const WIPE_ABS_TEMP_OFFSET: f64 = 30.0;
pub const WIPE_Z_ACCUM_THRESHOLD: f64 = 0.20;
pub const WIPE_Z_ACCUM_STEP: f64 = 0.06;
pub const WIPE_Z_MICRO_ADJUST: f64 = 0.03;
pub const WIPE_WIPE_SPEED: i64 = 1800;
pub const WIPE_FIRST_LAYER_EXTRA_LEN: f64 = 2.0;
pub const WIPE_REINFORCE_SHORTEN: f64 = 2.0;
pub const WIPE_REINFORCE_Z_ADJUST: f64 = 0.1;
/// 阶梯首个正方形起点偏移系数（× NozzleDiameter；0.0 对齐 Python）。
pub const WIPE_SUPPORT_OFFSET_COEFF: f64 = 0.0;
/// 圆盘覆盖率阈值：≥ 此值视为"直接压在上方"（用加强连接线）。
pub const WIPE_DISK_OVERLAP_THRESHOLD: f64 = 0.25;
/// 圆盘末段 WIPE 单次移动距离（沿末段走线方向）。
pub const WIPE_DISTANCE: f64 = 1.0;
/// G3 螺旋退出加速度。
pub const WIPE_EXIT_ACCELERATION: i64 = 6000;
/// 直线连接线圆盘是否生成螺旋（内部实验开关，默认 false）。
pub const SPIRAL_ON_STRAIGHT_CONNECTION: bool = false;

pub const ANGLE_OFFSETS: [f64; 3] = [0.0, 30.0, -30.0];

// ---- 几何（internal/geometry/convex_hull.go） ----

fn cross(o: &[f64], a: &[f64], b: &[f64]) -> f64 {
    (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
}

/// `ConvexHull`：Andrew 单调链（cross<=0 弹栈，严格凸包，共线点剔除）。
pub fn convex_hull(points: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let n = points.len();
    if n < 3 {
        return points.to_vec();
    }
    let mut sorted = points.to_vec();
    sorted.sort_by(|a, b| {
        a[0].partial_cmp(&b[0])
            .unwrap()
            .then(a[1].partial_cmp(&b[1]).unwrap())
    });

    let mut lower: Vec<[f64; 2]> = Vec::new();
    for p in &sorted {
        while lower.len() >= 2 && cross(&lower[lower.len() - 2], &lower[lower.len() - 1], p) <= 0.0
        {
            lower.pop();
        }
        lower.push(*p);
    }
    let mut upper: Vec<[f64; 2]> = Vec::new();
    for p in sorted.iter().rev() {
        while upper.len() >= 2 && cross(&upper[upper.len() - 2], &upper[upper.len() - 1], p) <= 0.0
        {
            upper.pop();
        }
        upper.push(*p);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

/// `ComputeNormals`：顶点法线（相邻边法线平均 + EnsureOutward）。
pub fn compute_normals(hull_points: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let n = hull_points.len();
    if n == 0 {
        return Vec::new();
    }
    let mut center_x = 0.0;
    let mut center_y = 0.0;
    for p in hull_points {
        center_x += p[0];
        center_y += p[1];
    }
    center_x /= n as f64;
    center_y /= n as f64;

    let mut normals = Vec::with_capacity(n);
    for (i, vertex) in hull_points.iter().enumerate() {
        let prev_vertex = hull_points[(i + n - 1) % n];
        let next_vertex = hull_points[(i + 1) % n];

        let edge1 = [vertex[0] - prev_vertex[0], vertex[1] - prev_vertex[1]];
        let edge2 = [next_vertex[0] - vertex[0], next_vertex[1] - vertex[1]];

        let mut normal1 = [-edge1[1], edge1[0]];
        let mut normal2 = [-edge2[1], edge2[0]];

        let norm1 = (normal1[0] * normal1[0] + normal1[1] * normal1[1]).sqrt();
        let norm2 = (normal2[0] * normal2[0] + normal2[1] * normal2[1]).sqrt();
        if norm1 > 0.0 {
            normal1[0] /= norm1;
            normal1[1] /= norm1;
        }
        if norm2 > 0.0 {
            normal2[0] /= norm2;
            normal2[1] /= norm2;
        }

        let mut avg_normal: [f64; 2];
        if norm1 > 0.0 && norm2 > 0.0 {
            avg_normal = [
                (normal1[0] + normal2[0]) / 2.0,
                (normal1[1] + normal2[1]) / 2.0,
            ];
            let avg_norm = (avg_normal[0] * avg_normal[0] + avg_normal[1] * avg_normal[1]).sqrt();
            if avg_norm > 0.0 {
                avg_normal[0] /= avg_norm;
                avg_normal[1] /= avg_norm;
            } else {
                avg_normal = normal1;
            }
        } else if norm1 > 0.0 {
            avg_normal = normal1;
        } else if norm2 > 0.0 {
            avg_normal = normal2;
        } else {
            let dx = vertex[0] - center_x;
            let dy = vertex[1] - center_y;
            let d = (dx * dx + dy * dy).sqrt();
            if d > 0.0 {
                avg_normal = [dx / d, dy / d];
            } else {
                avg_normal = [1.0, 0.0];
            }
        }
        // EnsureOutward
        let to_center_x = center_x - vertex[0];
        let to_center_y = center_y - vertex[1];
        let dot = avg_normal[0] * to_center_x + avg_normal[1] * to_center_y;
        if dot > 0.0 {
            avg_normal[0] = -avg_normal[0];
            avg_normal[1] = -avg_normal[1];
        }
        normals.push(avg_normal);
    }
    normals
}

// ---- 类型 ----

#[derive(Debug, Clone)]
pub struct DiskPosition {
    pub vertex: [f64; 2],
    pub center: [f64; 2],
    pub normal: [f64; 2],
    pub perpendicular: [f64; 2],
    pub vertex_index: usize,
    pub offset_angle: f64,
    pub offset_idx: usize,
}

#[derive(Debug, Clone)]
pub struct DiskHistoryEntry {
    pub disk_info: Option<DiskPosition>,
    pub layer_height: f64,
    pub stagger_angle: f64,
    pub stagger_direction: i32,
    pub stick_root: Option<[f64; 2]>,
    pub is_first_layer: bool,
}

/// 碰撞层（wipe.go:241）：HullPoly 惰性计算缓存（geom::Polygon 形态）。
pub struct CollisionLayer {
    pub wall_lines: Vec<String>,
    pub hull_poly: Option<crate::postproc::geom::Polygon>,
}

impl CollisionLayer {
    pub fn new(wall_lines: Vec<String>) -> Self {
        CollisionLayer {
            wall_lines,
            hull_poly: None,
        }
    }
}

pub struct DiskWipeResult {
    pub gcode_lines: Vec<String>,
    pub selected_disk: Option<DiskPosition>,
    pub best_score: f64,
    pub is_first_layer: bool,
    pub layer_height: f64,
    pub connection_lines: Vec<String>,
    pub disk_history: Vec<DiskHistoryEntry>,
}

// ---- 小工具 ----

pub(crate) fn rotate_point(x: f64, y: f64, cx: f64, cy: f64, angle_rad: f64) -> (f64, f64) {
    let dx = x - cx;
    let dy = y - cy;
    let cos_a = angle_rad.cos();
    let sin_a = angle_rad.sin();
    (cx + dx * cos_a - dy * sin_a, cy + dx * sin_a + dy * cos_a)
}

fn compute_disk_coverage(center1: [f64; 2], center2: [f64; 2], radius: f64) -> f64 {
    let dx = center1[0] - center2[0];
    let dy = center1[1] - center2[1];
    let d = (dx * dx + dy * dy).sqrt();
    if d >= 2.0 * radius {
        return 0.0;
    }
    if d == 0.0 {
        return 1.0;
    }
    let r = radius;
    let intersect_area =
        2.0 * r * r * (d / (2.0 * r)).acos() - (d / 2.0) * (4.0 * r * r - d * d).sqrt();
    let disk_area = std::f64::consts::PI * r * r;
    intersect_area / disk_area
}

fn extract_xy(line: &str) -> Option<(f64, f64)> {
    if !line.starts_with("G1 X") || !line.contains('E') {
        return None;
    }
    let x = crate::gcode::extract_coord(line.as_bytes(), b'X')?;
    let y = crate::gcode::extract_coord(line.as_bytes(), b'Y')?;
    Some((x, y))
}

fn extract_xy_from_g1(line: &str) -> Option<(f64, f64)> {
    if !line.starts_with("G1") || !line.contains('X') || !line.contains('Y') {
        return None;
    }
    let x = crate::gcode::extract_coord(line.as_bytes(), b'X')?;
    let y = crate::gcode::extract_coord(line.as_bytes(), b'Y')?;
    Some((x, y))
}

fn last_extrude_xy_in_output(lines: &[String]) -> Option<(f64, f64)> {
    for line in lines.iter().rev() {
        if let Some(xy) = extract_xy(line) {
            return Some(xy);
        }
    }
    None
}

// computeLastExtrudeDirInOutput（wipe.go:301）在 Go 侧无消费者，未移植。

// ---- stagger 角计算（wipe.go:129） ----

fn compute_stagger_angle(
    disk_history: &[DiskHistoryEntry],
    stick_root: [f64; 2],
    current_layer_height: f64,
    last_layer_height: f64,
    ir_data: &Ir,
) -> (f64, Option<[f64; 2]>, i32) {
    if ir_data.disk.stagger_mode == "off" {
        for entry in disk_history.iter().rev() {
            let (Some(_), Some(root)) = (&entry.disk_info, entry.stick_root) else {
                continue;
            };
            let dx = (root[0] - stick_root[0]).abs();
            let dy = (root[1] - stick_root[1]).abs();
            if dx < DISK_RADIUS && dy < DISK_RADIUS {
                return (5.0, Some(root), 1);
            }
            break; // 只检查最近的一个
        }
        return (0.0, None, 1);
    }

    let mut should_stagger = false;
    if ir_data.disk.stagger_mode == "on" {
        should_stagger = true;
    } else if ir_data.disk.stagger_mode == "auto" {
        let mut layer_diff = current_layer_height;
        for entry in disk_history.iter().rev() {
            let Some(info) = &entry.disk_info else {
                continue;
            };
            let dx = (info.center[0] - stick_root[0]).abs();
            let dy = (info.center[1] - stick_root[1]).abs();
            if dx <= 2.0 && dy <= 2.0 {
                layer_diff = current_layer_height - entry.layer_height;
                break;
            }
        }
        let mut actual_layer_height = current_layer_height - last_layer_height;
        if actual_layer_height <= 0.0 {
            actual_layer_height = ir_data.machine.typical_layer_height;
        }
        if layer_diff >= actual_layer_height * 3.0 {
            should_stagger = true;
        }
    }

    if !should_stagger {
        return (0.0, None, 1);
    }

    let mut stagger_angle_deg = ir_data.disk.stagger_angle;
    if stagger_angle_deg <= 0.0 {
        stagger_angle_deg = 5.0;
    }
    let mut max_angle_deg = ir_data.disk.stagger_max_angle;
    if max_angle_deg <= 0.0 {
        max_angle_deg = 45.0;
    }

    let is_sawtooth = ir_data.disk.swing_mode == "oscillate";

    let mut last_angle = 0.0;
    let mut last_dir = 1;
    let mut matched_root: Option<[f64; 2]> = None;
    for entry in disk_history.iter().rev() {
        let (Some(_), Some(root)) = (&entry.disk_info, entry.stick_root) else {
            continue;
        };
        let dx = (root[0] - stick_root[0]).abs();
        let dy = (root[1] - stick_root[1]).abs();
        if dx <= DISK_RADIUS && dy <= DISK_RADIUS {
            last_angle = entry.stagger_angle;
            last_dir = entry.stagger_direction;
            matched_root = Some(root);
            break;
        }
    }

    let Some(matched_root) = matched_root else {
        return (0.0, Some(stick_root), 1);
    };

    if last_dir == 0 {
        last_dir = 1;
    }

    let new_angle: f64;
    let new_dir: i32;
    if is_sawtooth {
        let a = last_angle + last_dir as f64 * stagger_angle_deg;
        if a >= max_angle_deg {
            new_angle = max_angle_deg;
            new_dir = -1;
        } else if a <= 0.0 {
            new_angle = 0.0;
            new_dir = 1;
        } else {
            new_angle = a;
            new_dir = last_dir;
        }
    } else {
        let mut a = last_angle + stagger_angle_deg;
        if a > max_angle_deg {
            a -= max_angle_deg;
        }
        new_angle = a;
        new_dir = 1;
    }

    (new_angle, Some(matched_root), new_dir)
}

// ---- 选位与评分（GenerateDiskWipe 的公共前半 + PredictDiskPlacement） ----

/// 由支撑行构造候选圆盘位（凸包顶点 × 3 个角度偏移）。
fn build_disk_positions(supports: &[String]) -> Option<Vec<DiskPosition>> {
    let mut all_support_points: Vec<[f64; 2]> = Vec::new();
    for gcode_line in supports {
        if let Some((x, y)) = extract_xy(gcode_line) {
            all_support_points.push([x, y]);
        }
    }
    if all_support_points.len() < 3 {
        return None;
    }
    let hull_points = convex_hull(&all_support_points);
    if hull_points.len() < 3 {
        return None;
    }
    let mut support_center = [0.0f64, 0.0f64];
    for pt in &all_support_points {
        support_center[0] += pt[0];
        support_center[1] += pt[1];
    }
    support_center[0] /= all_support_points.len() as f64;
    support_center[1] /= all_support_points.len() as f64;

    let normals = compute_normals(&hull_points);
    let disk_offset_dist = EXTENSION_LENGTH;

    let mut disk_positions = Vec::new();
    for (i, vertex) in hull_points.iter().enumerate() {
        let normal = normals[i];
        let mut outward_dx = vertex[0] - support_center[0];
        let mut outward_dy = vertex[1] - support_center[1];
        let outward_len = (outward_dx * outward_dx + outward_dy * outward_dy).sqrt();
        if outward_len > 0.001 {
            outward_dx /= outward_len;
            outward_dy /= outward_len;
        } else {
            outward_dx = normal[0];
            outward_dy = normal[1];
        }
        for (offset_idx, a) in ANGLE_OFFSETS.iter().enumerate() {
            let angle_rad = a * std::f64::consts::PI / 180.0;
            let cos_a = angle_rad.cos();
            let sin_a = angle_rad.sin();
            let rotated_normal = [
                outward_dx * cos_a - outward_dy * sin_a,
                outward_dx * sin_a + outward_dy * cos_a,
            ];
            let disk_center = [
                vertex[0] + rotated_normal[0] * disk_offset_dist,
                vertex[1] + rotated_normal[1] * disk_offset_dist,
            ];
            let rotated_perpendicular = [-rotated_normal[1], rotated_normal[0]];
            disk_positions.push(DiskPosition {
                vertex: *vertex,
                center: disk_center,
                normal: rotated_normal,
                perpendicular: rotated_perpendicular,
                vertex_index: i,
                offset_angle: ANGLE_OFFSETS[offset_idx],
                offset_idx,
            });
        }
    }
    Some(disk_positions)
}

/// `PredictDiskPlacement`（wipe.go:334）：只做选位+评分，不生成 G-code、
/// 不追加 diskHistory（HullPoly 缓存仍会填充，与 Go 一致）。
pub fn predict_disk_placement(
    supports: &[String],
    collision_check: &mut [CollisionLayer],
) -> (Option<DiskPosition>, f64, bool) {
    let Some(disk_positions) = build_disk_positions(supports) else {
        return (None, -1.0, false);
    };
    let polygon_angles: Vec<f64> = (0..N_GON)
        .map(|i| 2.0 * std::f64::consts::PI * i as f64 / N_GON as f64)
        .collect();

    let mut selected_disk: Option<DiskPosition> = None;
    let mut best_score = -1.0;

    for disk_info in &disk_positions {
        let (cx, cy) = (disk_info.center[0], disk_info.center[1]);
        let check_poly: Vec<[f64; 2]> = polygon_angles
            .iter()
            .map(|angle| {
                [
                    cx + CHECK_RADIUS * angle.cos(),
                    cy + CHECK_RADIUS * angle.sin(),
                ]
            })
            .collect();
        let check_poly: crate::postproc::geom::Polygon =
            check_poly.iter().map(|p| p.to_vec()).collect();

        let mut collision_depths: Vec<i64> = Vec::new();
        for (depth, layer_data) in collision_check.iter_mut().enumerate() {
            if !ensure_hull(layer_data) {
                continue;
            }
            if crate::postproc::geom::polygons_intersect(
                &check_poly,
                layer_data.hull_poly.as_ref().unwrap(),
            ) {
                collision_depths.push(depth as i64 + 1);
            }
        }

        let score = if collision_depths.is_empty() {
            100.0
        } else {
            collision_depths.iter().copied().min().unwrap() as f64
        };

        if score > best_score {
            best_score = score;
            selected_disk = Some(disk_info.clone());
        }
    }

    let can_place = best_score >= WIPE_COLLISION_THRESHOLD as f64;
    (selected_disk, best_score, can_place)
}

/// 惰性计算碰撞层的凸包（wallLines → HullPoly，≥3 点）。
fn ensure_hull(layer: &mut CollisionLayer) -> bool {
    if layer.hull_poly.is_some() {
        return true;
    }
    let mut wall_points: Vec<[f64; 2]> = Vec::new();
    for wall_line in &layer.wall_lines {
        if let Some((wx, wy)) = extract_xy_from_g1(wall_line) {
            wall_points.push([wx, wy]);
        }
    }
    if wall_points.len() >= 3 {
        let wall_hull = convex_hull(&wall_points);
        if wall_hull.len() >= 3 {
            layer.hull_poly = Some(wall_hull.iter().map(|p| p.to_vec()).collect());
            return true;
        }
        false
    } else {
        false
    }
}

/// `GenerateDiskWipe`（wipe.go:489）：圆盘选位 + 绘制 + 连接线。
#[allow(clippy::too_many_arguments)]
pub fn generate_disk_wipe(
    supports: &[String],
    collision_check: &mut Vec<CollisionLayer>,
    disk_history: Vec<DiskHistoryEntry>,
    last_selected_disk: Option<&DiskPosition>,
    current_layer_height: f64,
    last_layer_height: f64,
    ir_data: &Ir,
    machine_type: &str,
    recent_lines: &[String],
    future_layer_count: usize,
) -> Option<DiskWipeResult> {
    let mut disk_history = disk_history;

    let line_width = ir_data.machine.nozzle_diameter * 1.1;
    let mut circles = (DISK_RADIUS / line_width) as i64;
    if circles < 2 {
        circles = 2;
    }

    let polygon_angles: Vec<f64> = (0..N_GON)
        .map(|i| 2.0 * std::f64::consts::PI * i as f64 / N_GON as f64)
        .collect();

    let Some(disk_positions) = build_disk_positions(supports) else {
        return None;
    };

    let mut selected_disk: Option<DiskPosition> = None;
    let mut best_score = -1.0;

    for disk_info in &disk_positions {
        let (cx, cy) = (disk_info.center[0], disk_info.center[1]);
        let check_poly: Vec<[f64; 2]> = polygon_angles
            .iter()
            .map(|angle| {
                [
                    cx + CHECK_RADIUS * angle.cos(),
                    cy + CHECK_RADIUS * angle.sin(),
                ]
            })
            .collect();
        let check_poly: crate::postproc::geom::Polygon =
            check_poly.iter().map(|p| p.to_vec()).collect();

        let mut collision_depths: Vec<i64> = Vec::new();
        let mut future_collision = false;
        for (depth, layer_data) in collision_check.iter_mut().enumerate() {
            if !ensure_hull(layer_data) {
                continue;
            }
            if crate::postproc::geom::polygons_intersect(
                &check_poly,
                layer_data.hull_poly.as_ref().unwrap(),
            ) {
                if depth < future_layer_count {
                    future_collision = true;
                }
                collision_depths.push(depth as i64 + 1);
            }
        }

        let score = if future_collision {
            0.0
        } else if collision_depths.is_empty() {
            100.0
        } else {
            collision_depths.iter().copied().min().unwrap() as f64
        };

        if score > best_score {
            best_score = score;
            selected_disk = Some(disk_info.clone());
        }
    }

    let mut output: Vec<String> = Vec::new();
    output.push(crate::gcode::MARKER_DISK_WIPE_START.to_string());
    output.push(";Find collision-disk".to_string());

    let mut is_first_layer_disk = false;
    let mut actual_layer_height = current_layer_height - last_layer_height;
    if actual_layer_height <= 0.0 {
        actual_layer_height = ir_data.machine.typical_layer_height;
    }
    let mut layer_height = math_round((actual_layer_height * 1.5).max(0.04), 2);

    if best_score < WIPE_COLLISION_THRESHOLD as f64 {
        output.push(format!(
            "; Complex Wipe - best score {:.0}, cannot satisfy layer-3 collision-free requirement",
            best_score
        ));
        let (cool_down_commands, resume_commands) = generate_complex_wipe(
            recent_lines,
            current_layer_height,
            last_layer_height,
            ir_data,
            machine_type,
        );
        if !cool_down_commands.is_empty() {
            let mut combined = cool_down_commands;
            combined.append(&mut output);
            combined.extend(resume_commands);
            output = combined;
        }
        return Some(DiskWipeResult {
            gcode_lines: output,
            selected_disk: None,
            best_score,
            is_first_layer: false,
            layer_height,
            connection_lines: Vec::new(),
            disk_history,
        });
    }

    let mut connection_lines: Vec<String> = Vec::new();

    if let Some(disk) = &selected_disk {
        if last_selected_disk.is_none() && disk_history.is_empty() {
            if last_layer_height == ir_data.machine.first_layer_height {
                layer_height = math_round((ir_data.machine.first_layer_height * 1.5).max(0.04), 2);
                is_first_layer_disk = true;
                best_score = 0.0;
            }
        }

        // 遍历 disk_history 检查上一个圆盘是否与当前圆盘重叠（从 len-1 开始）
        for entry in disk_history.iter().rev() {
            let Some(info) = &entry.disk_info else {
                continue;
            };
            let coverage = compute_disk_coverage(info.center, disk.center, DISK_RADIUS);
            if coverage >= WIPE_DISK_OVERLAP_THRESHOLD {
                let mut layer_diff = (current_layer_height - entry.layer_height).abs()
                    / ir_data.machine.typical_layer_height;
                layer_diff = layer_diff.round() as i64 as f64;
                layer_diff -= 1.0;
                if layer_diff < WIPE_LAYER_DIFF_THRESHOLD {
                    best_score = best_score.min(layer_diff);
                    if best_score == 0.0 {
                        layer_height = math_round(
                            ((current_layer_height - last_layer_height) * 1.5).max(0.04),
                            2,
                        );
                    }
                }
                break;
            }
        }

        let (center_x, center_y) = (disk.center[0], disk.center[1]);

        let stick_root = disk.vertex;
        let (stagger_angle_deg, matched_root, stagger_dir) = compute_stagger_angle(
            &disk_history,
            stick_root,
            current_layer_height,
            last_layer_height,
            ir_data,
        );
        let stagger_angle_rad = stagger_angle_deg * std::f64::consts::PI / 180.0;

        let matched_root = matched_root.unwrap_or(stick_root);
        let (root_x, root_y) = (matched_root[0], matched_root[1]);

        let (rotated_cx, rotated_cy) =
            rotate_point(center_x, center_y, root_x, root_y, stagger_angle_rad);

        disk_history.push(DiskHistoryEntry {
            disk_info: Some(disk.clone()),
            layer_height: current_layer_height,
            stagger_angle: stagger_angle_deg,
            stagger_direction: stagger_dir,
            stick_root: Some(matched_root),
            is_first_layer: false,
        });
        if disk_history.len() > WIPE_HISTORY_MAX_ENTRIES {
            disk_history.remove(0);
        }

        output.push(format!(
            "; Disk - vertex {} angle {:.0} deg",
            disk.vertex_index, disk.offset_angle
        ));
        output.push(format!("; Center: ({:.2}, {:.2})", rotated_cx, rotated_cy));
        output.push(format!(
            "; StickRoot: ({:.2}, {:.2}) stagger {:.1} deg",
            root_x, root_y, stagger_angle_deg
        ));

        let filament_area =
            std::f64::consts::PI * (FILAMENT_DIAMETER / 2.0) * (FILAMENT_DIAMETER / 2.0);

        output.push(";Paint Disk".to_string());
        output.push(format!(
            "G1 X{:.3} Y{:.3} F{}",
            rotated_cx,
            rotated_cy,
            format_speed(ir_data.machine.travel_speed * MM_PER_MINUTE)
        ));
        output.push(format!(
            "G1 Z{:.2}",
            current_layer_height + WIPE_Z_LIFT_HEIGHT
        ));
        output.push(format!(
            "M109 S{:.0}",
            ir_data.toolhead.nozzle_switch_temperature
        ));
        output.push(format!(
            "; LINE_WIDTH: {}",
            format_python_float(math_round(line_width, 3))
        ));

        let total_angle = 2.0 * std::f64::consts::PI * circles as f64;
        let steps = (total_angle * 3.0) as i64;

        output.push("G92 E0".to_string());
        output.push(format!(
            "G1 E{:.6} F{}",
            ir_data.machine.retract_length, WIPE_RETRACT_SPEED
        ));
        output.push(format!(
            "; LAYER_HEIGHT: {}",
            format_python_float(layer_height)
        ));
        output.push(format!("G1 Z{:.2}", current_layer_height));

        let mut temp_speed_disk = WIPE_DISK_INIT_SPEED;

        // 连接类型判定（diskHistory 已含当前圆盘，从 len-2 起跳过自身）
        let mut has_overlapping_disk = false;
        let mut overlap_layer_diff = 0i64;
        for entry in disk_history[..disk_history.len().saturating_sub(1)]
            .iter()
            .rev()
        {
            let Some(info) = &entry.disk_info else {
                continue;
            };
            let coverage = compute_disk_coverage(info.center, disk.center, DISK_RADIUS);
            if coverage >= WIPE_DISK_OVERLAP_THRESHOLD {
                has_overlapping_disk = true;
                overlap_layer_diff = ((current_layer_height - entry.layer_height).abs()
                    / ir_data.machine.typical_layer_height)
                    .round() as i64;
                break;
            }
        }

        let is_staircase_connection =
            best_score > 0.0 && !(has_overlapping_disk && overlap_layer_diff <= 2);
        let should_generate_spiral = if is_first_layer_disk {
            false
        } else if is_staircase_connection {
            true
        } else {
            SPIRAL_ON_STRAIGHT_CONNECTION
        };
        if should_generate_spiral {
            for step in 30..=steps {
                let t = step as f64 / steps as f64;
                let r = t * DISK_RADIUS;
                let angle = t * total_angle;
                let (rx, ry) = rotate_point(
                    center_x + r * angle.cos(),
                    center_y + r * angle.sin(),
                    root_x,
                    root_y,
                    stagger_angle_rad,
                );
                let (x, y) = (rx, ry);

                if step == 30 {
                    // 螺旋起点锚定在圆盘中心
                    output.push(format!("G1 X{:.3} Y{:.3} F500", rotated_cx, rotated_cy));
                    output.push(format!("G1 Z{:.2}", current_layer_height));
                } else {
                    let t_prev = (step - 1) as f64 / steps as f64;
                    let r_prev = t_prev * DISK_RADIUS;
                    let angle_prev = t_prev * total_angle;
                    let (x_prev, y_prev) = rotate_point(
                        center_x + r_prev * angle_prev.cos(),
                        center_y + r_prev * angle_prev.sin(),
                        root_x,
                        root_y,
                        stagger_angle_rad,
                    );
                    let length = ((x - x_prev) * (x - x_prev) + (y - y_prev) * (y - y_prev)).sqrt();
                    let extrusion = (length * layer_height * line_width) / filament_area;
                    output.push(format!(
                        "G1 X{:.3} Y{:.3} E{:.6} F{}",
                        x,
                        y,
                        extrusion,
                        format_speed_int(temp_speed_disk)
                    ));
                }
            }
        }

        if should_generate_spiral {
            output.push("G1 E-0.02 F1800".to_string());
        }

        // 平行填充：高层高（layerHeight >= 0.65*喷嘴直径）
        if layer_height >= 0.65 * ir_data.machine.nozzle_diameter {
            temp_speed_disk = if ir_data.filament.filament_type != "TPU" {
                WIPE_FILL_SPEED
            } else {
                WIPE_TPU_FILL_SPEED
            };
            output.push("; parallel fill".to_string());
            output.push(format!(
                "G1 X{:.3} Y{:.3} F{}",
                rotated_cx,
                rotated_cy,
                format_speed(ir_data.machine.travel_speed * MM_PER_MINUTE)
            ));

            let y_min = center_y - DISK_RADIUS;
            let y_max = center_y + DISK_RADIUS;
            let line_spacing = line_width;
            let num_lines = (DISK_DIAMETER / line_spacing) as i64 + 1;
            let fill_layer_height = WIPE_FILL_LAYER_COEFF * ir_data.machine.nozzle_diameter;

            for line_idx in 0..num_lines {
                let y = y_min + line_idx as f64 * line_spacing;
                if y > y_max {
                    break;
                }
                let dy = y - center_y;
                if dy.abs() > DISK_RADIUS - WIPE_RADIUS_TOLERANCE {
                    continue;
                }
                let dx = (DISK_RADIUS * DISK_RADIUS - dy * dy).sqrt();
                let x_start = center_x - dx;
                let x_end = center_x + dx;
                let (line_x_start, line_x_end) = if line_idx % 2 == 0 {
                    (x_start, x_end)
                } else {
                    (x_end, x_start)
                };
                let (rsx, rsy) = rotate_point(line_x_start, y, root_x, root_y, stagger_angle_rad);
                let (rex, rey) = rotate_point(line_x_end, y, root_x, root_y, stagger_angle_rad);
                output.push(format!(
                    "G1 X{:.3} Y{:.3} F{}",
                    rsx,
                    rsy,
                    format_speed(ir_data.machine.travel_speed * MM_PER_MINUTE)
                ));
                let line_length = (x_end - x_start).abs();
                if line_length < WIPE_MIN_LINE_LENGTH {
                    continue;
                }
                let extrusion = (line_length * fill_layer_height * line_width) / filament_area;
                output.push(format!(
                    "G1 X{:.3} Y{:.3} E{:.6} F{}",
                    rex,
                    rey,
                    extrusion,
                    format_speed_int(temp_speed_disk)
                ));
            }

            output.push("; outer ring".to_string());
            let (start_rx, start_ry) = rotate_point(
                center_x + DISK_RADIUS,
                center_y,
                root_x,
                root_y,
                stagger_angle_rad,
            );
            output.push(format!(
                "G1 X{:.3} Y{:.3} F{}",
                start_rx,
                start_ry,
                format_speed(ir_data.machine.travel_speed * MM_PER_MINUTE)
            ));

            let circumference = 2.0 * std::f64::consts::PI * DISK_RADIUS;
            let total_extrusion = (circumference * fill_layer_height * line_width) / filament_area;
            let segments =
                WIPE_OUTER_RING_MIN_SEGS.max(circumference / WIPE_OUTER_RING_SEG_LENGTH) as i64;
            let segment_extrusion = total_extrusion / segments as f64;
            for seg in 1..=segments {
                let angle = 2.0 * std::f64::consts::PI * seg as f64 / segments as f64;
                let (rx, ry) = rotate_point(
                    center_x + DISK_RADIUS * angle.cos(),
                    center_y + DISK_RADIUS * angle.sin(),
                    root_x,
                    root_y,
                    stagger_angle_rad,
                );
                output.push(format!(
                    "G1 X{:.3} Y{:.3} E{:.6} F{}",
                    rx,
                    ry,
                    segment_extrusion,
                    format_speed_int(temp_speed_disk)
                ));
            }
            output.push(format!(
                "G1 X{:.3} Y{:.3} F{}",
                rotated_cx,
                rotated_cy,
                format_speed(ir_data.machine.travel_speed * MM_PER_MINUTE)
            ));
        }

        // 平行填充：首层或低层高（layerHeight < 0.65*喷嘴直径）
        if layer_height < 0.65 * ir_data.machine.nozzle_diameter {
            temp_speed_disk = if ir_data.filament.filament_type != "TPU" {
                WIPE_FILL_SPEED
            } else {
                WIPE_TPU_FILL_SPEED
            };
            output.push("; parallel fill".to_string());
            output.push(format!(
                "G1 X{:.3} Y{:.3} F{}",
                rotated_cx,
                rotated_cy,
                format_speed(ir_data.machine.travel_speed * MM_PER_MINUTE)
            ));

            let y_min = center_y - DISK_RADIUS;
            let y_max = center_y + DISK_RADIUS;
            let line_spacing = line_width;
            let num_lines = (DISK_DIAMETER / line_spacing) as i64 + 1;
            let fill_layer_height = layer_height;

            for line_idx in 0..num_lines {
                let y = y_min + line_idx as f64 * line_spacing;
                if y > y_max {
                    break;
                }
                let dy = y - center_y;
                if dy.abs() > DISK_RADIUS - WIPE_RADIUS_TOLERANCE {
                    continue;
                }
                let dx = (DISK_RADIUS * DISK_RADIUS - dy * dy).sqrt();
                let x_start = center_x - dx;
                let x_end = center_x + dx;
                let (line_x_start, line_x_end) = if line_idx % 2 == 0 {
                    (x_start, x_end)
                } else {
                    (x_end, x_start)
                };
                let (rsx, rsy) = rotate_point(line_x_start, y, root_x, root_y, stagger_angle_rad);
                let (rex, rey) = rotate_point(line_x_end, y, root_x, root_y, stagger_angle_rad);
                output.push(format!(
                    "G1 X{:.3} Y{:.3} F{}",
                    rsx,
                    rsy,
                    format_speed(ir_data.machine.travel_speed * MM_PER_MINUTE)
                ));
                let line_length = (x_end - x_start).abs();
                if line_length < WIPE_MIN_LINE_LENGTH {
                    continue;
                }
                let extrusion = (line_length * fill_layer_height * line_width) / filament_area;
                output.push(format!(
                    "G1 X{:.3} Y{:.3} E{:.6} F{}",
                    rex,
                    rey,
                    extrusion,
                    format_speed_int(temp_speed_disk)
                ));
            }

            output.push("; outer ring".to_string());
            let (start_rx, start_ry) = rotate_point(
                center_x + DISK_RADIUS,
                center_y,
                root_x,
                root_y,
                stagger_angle_rad,
            );
            output.push(format!(
                "G1 X{:.3} Y{:.3} F{}",
                start_rx,
                start_ry,
                format_speed(ir_data.machine.travel_speed * MM_PER_MINUTE)
            ));

            let circumference = 2.0 * std::f64::consts::PI * DISK_RADIUS;
            let total_extrusion = (circumference * fill_layer_height * line_width) / filament_area;
            let segments =
                WIPE_OUTER_RING_MIN_SEGS.max(circumference / WIPE_OUTER_RING_SEG_LENGTH) as i64;
            let segment_extrusion = total_extrusion / segments as f64;
            for seg in 1..=segments {
                let angle = 2.0 * std::f64::consts::PI * seg as f64 / segments as f64;
                let (rx, ry) = rotate_point(
                    center_x + DISK_RADIUS * angle.cos(),
                    center_y + DISK_RADIUS * angle.sin(),
                    root_x,
                    root_y,
                    stagger_angle_rad,
                );
                output.push(format!(
                    "G1 X{:.3} Y{:.3} E{:.6} F{}",
                    rx,
                    ry,
                    segment_extrusion,
                    format_speed_int(temp_speed_disk)
                ));
            }
            output.push(format!(
                "G1 X{:.3} Y{:.3} F{}",
                rotated_cx,
                rotated_cy,
                format_speed(ir_data.machine.travel_speed * MM_PER_MINUTE)
            ));
        }

        // Phase 4 圆盘退出：单次 WIPE 移动（径向向内）+ 回抽 + G3 螺旋退出
        let (last_x, last_y) =
            last_extrude_xy_in_output(&output).unwrap_or((rotated_cx, rotated_cy));
        let mut dir_x = rotated_cx - last_x;
        let mut dir_y = rotated_cy - last_y;
        let dir_len = (dir_x * dir_x + dir_y * dir_y).sqrt();
        if dir_len > 1e-9 {
            dir_x /= dir_len;
            dir_y /= dir_len;
        } else {
            dir_x = 1.0;
            dir_y = 0.0;
        }
        let wipe_end_x = last_x + dir_x * WIPE_DISTANCE;
        let wipe_end_y = last_y + dir_y * WIPE_DISTANCE;
        let retract_amount = ir_data.machine.retract_length;

        output.push(crate::gcode::MARKER_WIPE_START.to_string());
        let wipe_speed =
            (MM_PER_MINUTE * WIPE_SPEED_CAP).min(MM_PER_MINUTE * ir_data.machine.wall_print_speed);
        output.push(format!(
            "G1 X{:.3} Y{:.3} E-{:.4} F{}",
            wipe_end_x,
            wipe_end_y,
            retract_amount,
            format_speed_int(wipe_speed)
        ));
        output.push(crate::gcode::MARKER_WIPE_END.to_string());

        // FinishStage
        output.push("G1 E-0.02 F1800".to_string());
        output.push(format!("M204 S{}", WIPE_EXIT_ACCELERATION));
        output.push("G17".to_string());
        let target_z = math_round(current_layer_height + WIPE_Z_LIFT_HEIGHT, 2);
        let i_offset = rotated_cx - wipe_end_x;
        let j_offset = rotated_cy - wipe_end_y;
        let travel_speed = (ir_data.machine.travel_speed * MM_PER_MINUTE) as i64;
        output.push(format!(
            "G3 Z{:.2} I{:.3} J{:.3} P1 F{}",
            target_z, i_offset, j_offset, travel_speed
        ));

        let rotated_disk = DiskPosition {
            vertex: disk.vertex,
            center: [rotated_cx, rotated_cy],
            normal: disk.normal,
            perpendicular: disk.perpendicular,
            vertex_index: disk.vertex_index,
            offset_angle: disk.offset_angle,
            offset_idx: disk.offset_idx,
        };

        if best_score > 0.0 && !(has_overlapping_disk && overlap_layer_diff <= 2) {
            connection_lines = generate_connection_lines_score_positive(
                rotated_disk.clone(),
                best_score,
                current_layer_height,
                last_layer_height,
                layer_height,
                line_width,
                ir_data,
                machine_type,
            );
        } else if is_first_layer_disk {
            connection_lines = generate_first_layer_connection(
                &rotated_disk,
                current_layer_height,
                last_layer_height,
                line_width,
                ir_data,
            );
        } else {
            connection_lines = generate_reinforcement_connection(
                &rotated_disk,
                current_layer_height,
                last_layer_height,
                line_width,
                ir_data,
            );
        }
    }

    output.push(crate::gcode::MARKER_DISK_WIPE_END.to_string());

    Some(DiskWipeResult {
        gcode_lines: output,
        selected_disk,
        best_score,
        is_first_layer: is_first_layer_disk,
        layer_height,
        connection_lines,
        disk_history,
    })
}

// ---- Complex Wipe（wipe.go:1075） ----

fn generate_complex_wipe(
    recent_lines: &[String],
    current_layer_height: f64,
    _last_layer_height: f64,
    ir_data: &Ir,
    machine_type: &str,
) -> (Vec<String>, Vec<String>) {
    let _ = machine_type;
    let mut preglue_index: isize = -1;
    for idx in (0..recent_lines.len()).rev() {
        if recent_lines[idx].starts_with(crate::gcode::MARKER_PRE_GLUE) {
            preglue_index = idx as isize;
            break;
        }
    }
    if preglue_index < 0 {
        return (Vec::new(), Vec::new());
    }
    let preglue_index = preglue_index as usize;

    let mut z_height_value: Option<f64> = None;
    for idx in (0..preglue_index).rev() {
        if crate::gcode::has_slicer_comment_prefix(&recent_lines[idx], "Z_HEIGHT:") {
            z_height_value = crate::gcode::is_z_height_line(&recent_lines[idx]);
            break;
        }
    }

    let mut layer_height_value: Option<f64> = None;
    for idx in (0..preglue_index).rev() {
        if crate::gcode::has_slicer_comment_prefix(&recent_lines[idx], "LAYER_HEIGHT:") {
            let nums = num_strip(&recent_lines[idx]);
            if let Some(&v) = nums.first() {
                layer_height_value = Some(v);
            }
            break;
        }
    }

    let cool_down_z = match (z_height_value, layer_height_value) {
        (Some(z), Some(lh)) => z + lh * 0.5,
        _ => current_layer_height,
    };

    let mut support_index: isize = -1;
    for idx in (0..preglue_index).rev() {
        if crate::gcode::has_slicer_comment_prefix(&recent_lines[idx], "FEATURE: Support interface")
        {
            support_index = idx as isize;
            break;
        }
    }

    let mut target_xy: Option<(f64, f64)> = None;
    if support_index >= 0 {
        for line in recent_lines.iter().skip(support_index as usize + 1) {
            if line.starts_with("G1")
                && line.contains('X')
                && line.contains('Y')
                && line.contains('E')
            {
                if let Some(xy) = extract_xy_from_g1(line) {
                    target_xy = Some(xy);
                }
                break;
            }
        }
    }

    let mut cool_down_commands: Vec<String> = Vec::new();
    if let Some((tx, ty)) = target_xy {
        cool_down_commands.push(format!(
            "G1 Z{:.2} F{}",
            cool_down_z + WIPE_COOL_Z_LIFT,
            WIPE_COOL_Z_SPEED
        ));
        cool_down_commands.push(format!(
            "G1 X{:.3} Y{:.3} F{}",
            tx,
            ty,
            format_speed(ir_data.machine.travel_speed * MM_PER_MINUTE)
        ));
        cool_down_commands.push(format!("G1 Z{:.2}", cool_down_z));
    }

    match ir_data.filament.filament_type.as_str() {
        "PLA" => cool_down_commands.push(format!("M109 S{}", WIPE_PLA_COOL_TEMP)),
        "ABS" => cool_down_commands.push(format!("M104 S{}", WIPE_ABS_COOL_TEMP)),
        "PETG" => cool_down_commands.push(format!("M109 S{}", WIPE_PETG_COOL_TEMP)),
        _ => {}
    }

    cool_down_commands.push(format!("G1 Z{:.2}", cool_down_z + WIPE_COOL_Z_LIFT));

    let (resume_x, resume_y) = find_next_extrude_position(recent_lines, preglue_index);
    let mut resume_commands = vec![
        format!(
            "G1 Z{:.2} F{}",
            cool_down_z + WIPE_COOL_Z_LIFT,
            WIPE_COOL_Z_SPEED
        ),
        format!(
            "G1 X{:.3} Y{:.3} F{}",
            resume_x,
            resume_y,
            format_speed(ir_data.machine.travel_speed * MM_PER_MINUTE)
        ),
        format!("G1 Z{:.2}", cool_down_z),
        format!("M109 S{:.0}", ir_data.toolhead.nozzle_switch_temperature),
    ];
    if !ir_data.gcode.refill_extrude_command.is_empty() {
        resume_commands.push(ir_data.gcode.refill_extrude_command.clone());
    } else {
        resume_commands.push(format!(
            "G1 E{} F{}",
            format_e(ir_data.machine.retract_length),
            WIPE_RETRACT_SPEED
        ));
    }

    (cool_down_commands, resume_commands)
}

fn find_next_extrude_position(recent_lines: &[String], preglue_index: usize) -> (f64, f64) {
    for line in recent_lines.iter().skip(preglue_index + 1) {
        if line.starts_with("G1") && line.contains('X') && line.contains('Y') && line.contains('E')
        {
            if let Some((x, y)) = extract_xy_from_g1(line) {
                return (x, y);
            }
        }
    }
    (0.0, 0.0)
}

// ---- 三种连接线（wipe.go:1212-1529） ----

#[allow(clippy::too_many_arguments)]
fn generate_connection_lines_score_positive(
    selected_disk: DiskPosition,
    _best_score: f64,
    current_layer_height: f64,
    last_layer_height: f64,
    _layer_height: f64,
    line_width: f64,
    ir_data: &Ir,
    machine_type: &str,
) -> Vec<String> {
    let mut connection_lines: Vec<String> = Vec::new();

    let mut temp_z = current_layer_height;
    if temp_z < ir_data.machine.first_layer_height + 0.2 {
        temp_z += 0.2;
    }

    let mut conn_actual_layer_height = current_layer_height - last_layer_height;
    if conn_actual_layer_height <= 0.0 {
        conn_actual_layer_height = ir_data.machine.typical_layer_height;
    }
    let conn_layer_height = math_round((conn_actual_layer_height * 1.5).max(0.04), 2);
    let filament_area =
        std::f64::consts::PI * (FILAMENT_DIAMETER / 2.0) * (FILAMENT_DIAMETER / 2.0);
    let nozzle_diameter = ir_data.machine.nozzle_diameter;

    let square_size = 1.0 * nozzle_diameter;
    let step_size = nozzle_diameter;

    let len_long = square_size;
    let vol_long = len_long * conn_layer_height * line_width;
    let extr_long = vol_long / filament_area;

    let len_corner = line_width;
    let vol_corner = len_corner * conn_layer_height * line_width;
    let extr_corner = vol_corner / filament_area;

    let (center_x, center_y) = (selected_disk.center[0], selected_disk.center[1]);
    let (vertex_x, vertex_y) = (selected_disk.vertex[0], selected_disk.vertex[1]);

    let center_to_vertex = [vertex_x - center_x, vertex_y - center_y];
    let ctv_length = (center_to_vertex[0] * center_to_vertex[0]
        + center_to_vertex[1] * center_to_vertex[1])
        .sqrt();
    let direction_long = [
        center_to_vertex[0] / ctv_length,
        center_to_vertex[1] / ctv_length,
    ];
    let perp_long = [-direction_long[1], direction_long[0]];

    let line_spacing = nozzle_diameter;
    let total_length = EXTENSION_LENGTH * 0.5;
    let target_length = total_length + WIPE_TARGET_LENGTH_EXTRA;
    let num_squares = (target_length / step_size).ceil() as i64;

    connection_lines.push(format!(
        "; LAYER_HEIGHT: {}",
        format_python_float(conn_layer_height)
    ));

    let filament_type = ir_data.filament.filament_type.as_str();
    if filament_type == "PLA" {
        connection_lines.push(format!(
            "M104 S{:.0}",
            ir_data.toolhead.nozzle_switch_temperature - WIPE_PLA_TEMP_OFFSET
        ));
    } else if filament_type == "ABS" || filament_type == "PETG" {
        connection_lines.push(format!(
            "M104 S{:.0}",
            ir_data.toolhead.nozzle_switch_temperature - WIPE_ABS_TEMP_OFFSET
        ));
    }

    if filament_type == "PLA" || filament_type == "PETG" {
        let fan_speed = format_python_float(ir_data.wiping.fan_speed);
        if get_machine_dimensions(machine_type).flags.has_second_fan {
            connection_lines.push(format!("M106 P1 S{fan_speed}"));
        } else {
            connection_lines.push(format!("M106 S{fan_speed}"));
        }
    }

    if num_squares > 0 {
        let start_x = vertex_x - direction_long[0] * 0.0;
        let start_y = vertex_y - direction_long[1] * 0.0;
        connection_lines.push(format!(
            "G1 X{:.3} Y{:.3} F{}",
            start_x + perp_long[0] * (line_spacing / 2.0),
            start_y + perp_long[1] * (line_spacing / 2.0),
            format_speed(ir_data.machine.travel_speed * MM_PER_MINUTE)
        ));
        connection_lines.push(format!("G1 Z{:.2}", temp_z + 0.3));
    }

    let temp_z_adjust = ir_data
        .machine
        .first_layer_height
        .max(temp_z - ir_data.machine.first_layer_height);
    connection_lines.push("; FEATURE: Top surface".to_string());
    connection_lines.push(format!("G1 Z{:.2}", temp_z_adjust));
    connection_lines.push(format!("G1 E{:.3}", ir_data.machine.retract_length));

    let mut z_accumulate = 0.0f64;
    let mut last_end1: Option<[f64; 2]> = None;

    connection_lines.push("G4 P200".to_string());

    for square_idx in 0..num_squares {
        if z_accumulate < WIPE_Z_ACCUM_THRESHOLD {
            z_accumulate += WIPE_Z_ACCUM_STEP;
        }
        let initial_offset = WIPE_SUPPORT_OFFSET_COEFF * ir_data.machine.nozzle_diameter;
        let current_offset = initial_offset + square_idx as f64 * step_size;
        if current_offset >= target_length {
            break;
        }

        let s_x = vertex_x - direction_long[0] * current_offset;
        let s_y = vertex_y - direction_long[1] * current_offset;
        let e_x = s_x - direction_long[0] * square_size;
        let e_y = s_y - direction_long[1] * square_size;

        let start1 = [
            s_x + perp_long[0] * (line_spacing / 2.0),
            s_y + perp_long[1] * (line_spacing / 2.0),
        ];
        let start2 = [
            s_x - perp_long[0] * (line_spacing / 2.0),
            s_y - perp_long[1] * (line_spacing / 2.0),
        ];
        let end1 = [
            e_x + perp_long[0] * (line_spacing / 2.0),
            e_y + perp_long[1] * (line_spacing / 2.0),
        ];
        let end2 = [
            e_x - perp_long[0] * (line_spacing / 2.0),
            e_y - perp_long[1] * (line_spacing / 2.0),
        ];

        // 流率衰减：N=1 ×1.2、N=2 ×1.1、N=3 ×1.0、N≥4 ×0.95
        let flow_factor = match square_idx {
            0 => 1.2,
            1 => 1.1,
            2 => 1.0,
            _ => 0.95,
        };
        let extr_long_scaled = extr_long * flow_factor;
        let extr_corner_scaled = extr_corner * flow_factor;

        let speed: i64 = if square_idx == 0 { 400 } else { 500 };

        connection_lines.push(format!(
            "G1 X{:.3} Y{:.3} F{}",
            start1[0],
            start1[1],
            format_speed(ir_data.machine.travel_speed * MM_PER_MINUTE)
        ));
        connection_lines.push(format!(
            "G1 X{:.3} Y{:.3} E{:.6} F{}",
            end1[0], end1[1], extr_long_scaled, speed
        ));
        connection_lines.push(format!("G1 Z{:.2}", temp_z_adjust + z_accumulate));
        connection_lines.push(format!(
            "G1 X{:.3} Y{:.3} E{:.6} F500",
            end2[0], end2[1], extr_corner_scaled
        ));
        connection_lines.push(format!(
            "G1 Z{:.2}",
            temp_z_adjust + z_accumulate - WIPE_Z_MICRO_ADJUST
        ));
        connection_lines.push(format!(
            "G1 X{:.3} Y{:.3} E{:.6} F500",
            start2[0], start2[1], extr_long_scaled
        ));
        connection_lines.push(format!(
            "G1 X{:.3} Y{:.3} E{:.6} F500",
            start1[0], start1[1], extr_corner_scaled
        ));
        connection_lines.push(format!("G1 Z{:.2}", temp_z_adjust + z_accumulate));
        last_end1 = Some(end1);
    }

    if !connection_lines.is_empty()
        && let Some(end1) = last_end1
    {
        let wipe_x = end1[0] + direction_long[0] * 1.0;
        let wipe_y = end1[1] + direction_long[1] * 1.0;
        connection_lines.push(crate::gcode::MARKER_WIPE_START.to_string());
        connection_lines.push(format!(
            "G1 X{:.3} Y{:.3} E-{:.3} F{}",
            wipe_x, wipe_y, ir_data.machine.retract_length, WIPE_WIPE_SPEED
        ));
        connection_lines.push(crate::gcode::MARKER_WIPE_END.to_string());
    }

    connection_lines
}

fn generate_first_layer_connection(
    selected_disk: &DiskPosition,
    _current_layer_height: f64,
    _last_layer_height: f64,
    line_width: f64,
    ir_data: &Ir,
) -> Vec<String> {
    let (center_x, center_y) = (selected_disk.center[0], selected_disk.center[1]);
    let (vertex_x, vertex_y) = (selected_disk.vertex[0], selected_disk.vertex[1]);

    let conn_layer_height = (ir_data.machine.first_layer_height * 1.5).max(0.04);
    let filament_area =
        std::f64::consts::PI * (FILAMENT_DIAMETER / 2.0) * (FILAMENT_DIAMETER / 2.0);
    let nozzle_diameter = ir_data.machine.nozzle_diameter;

    let total_length = EXTENSION_LENGTH + WIPE_FIRST_LAYER_EXTRA_LEN;

    let len_long = total_length;
    let extr_long = len_long * conn_layer_height * line_width / filament_area;
    let len_corner = line_width;
    let extr_corner = len_corner * conn_layer_height * line_width / filament_area;

    let center_to_vertex = [vertex_x - center_x, vertex_y - center_y];
    let ctv_length = (center_to_vertex[0] * center_to_vertex[0]
        + center_to_vertex[1] * center_to_vertex[1])
        .sqrt();
    let direction_long = [
        center_to_vertex[0] / ctv_length,
        center_to_vertex[1] / ctv_length,
    ];
    let perp_long = [-direction_long[1], direction_long[0]];

    let line_spacing = nozzle_diameter;

    let end_x = vertex_x - direction_long[0] * total_length;
    let end_y = vertex_y - direction_long[1] * total_length;

    let start1 = [
        vertex_x + perp_long[0] * (line_spacing / 2.0),
        vertex_y + perp_long[1] * (line_spacing / 2.0),
    ];
    let start2 = [
        vertex_x - perp_long[0] * (line_spacing / 2.0),
        vertex_y - perp_long[1] * (line_spacing / 2.0),
    ];
    let end1 = [
        end_x + perp_long[0] * (line_spacing / 2.0),
        end_y + perp_long[1] * (line_spacing / 2.0),
    ];
    let end2 = [
        end_x - perp_long[0] * (line_spacing / 2.0),
        end_y - perp_long[1] * (line_spacing / 2.0),
    ];

    vec![
        format!("; LAYER_HEIGHT: {}", format_python_float(conn_layer_height)),
        "; 加强连接线 - 首层".to_string(),
        "; FEATURE: Top surface".to_string(),
        format!(
            "G1 X{:.3} Y{:.3} F{}",
            start1[0],
            start1[1],
            format_speed(ir_data.machine.travel_speed * MM_PER_MINUTE)
        ),
        format!("G1 Z{:.2}", ir_data.machine.first_layer_height + 0.2),
        format!("G1 E{:.3}", ir_data.machine.retract_length),
        format!("G1 Z{:.2}", ir_data.machine.first_layer_height),
        "G4 P200".to_string(),
        format!("G1 X{:.3} Y{:.3} E{:.6} F400", end1[0], end1[1], extr_long),
        format!(
            "G1 X{:.3} Y{:.3} E{:.6} F500",
            end2[0], end2[1], extr_corner
        ),
        format!(
            "G1 X{:.3} Y{:.3} E{:.6} F500",
            start2[0], start2[1], extr_long
        ),
        format!(
            "G1 X{:.3} Y{:.3} E{:.6} F500",
            start1[0], start1[1], extr_corner
        ),
    ]
}

fn generate_reinforcement_connection(
    selected_disk: &DiskPosition,
    current_layer_height: f64,
    last_layer_height: f64,
    line_width: f64,
    ir_data: &Ir,
) -> Vec<String> {
    let mut temp_z = current_layer_height;
    if temp_z >= ir_data.machine.first_layer_height + 0.2 {
        temp_z -= WIPE_REINFORCE_Z_ADJUST;
    }

    let (center_x, center_y) = (selected_disk.center[0], selected_disk.center[1]);
    let (vertex_x, vertex_y) = (selected_disk.vertex[0], selected_disk.vertex[1]);

    let conn_layer_height = math_round(
        ((current_layer_height - last_layer_height) * 1.5).max(0.04),
        2,
    );
    let filament_area =
        std::f64::consts::PI * (FILAMENT_DIAMETER / 2.0) * (FILAMENT_DIAMETER / 2.0);
    let nozzle_diameter = ir_data.machine.nozzle_diameter;

    let total_length = (EXTENSION_LENGTH - WIPE_REINFORCE_SHORTEN).max(0.1);

    let len_long = total_length;
    let extr_long = len_long * conn_layer_height * line_width / filament_area;
    let len_corner = line_width;
    let extr_corner = len_corner * conn_layer_height * line_width / filament_area;

    let center_to_vertex = [vertex_x - center_x, vertex_y - center_y];
    let ctv_length = (center_to_vertex[0] * center_to_vertex[0]
        + center_to_vertex[1] * center_to_vertex[1])
        .sqrt();
    let direction_long = [
        center_to_vertex[0] / ctv_length,
        center_to_vertex[1] / ctv_length,
    ];
    let perp_long = [-direction_long[1], direction_long[0]];

    let line_spacing = nozzle_diameter;

    let end_x = vertex_x - direction_long[0] * total_length;
    let end_y = vertex_y - direction_long[1] * total_length;

    let start1 = [
        vertex_x + perp_long[0] * (line_spacing / 2.0),
        vertex_y + perp_long[1] * (line_spacing / 2.0),
    ];
    let start2 = [
        vertex_x - perp_long[0] * (line_spacing / 2.0),
        vertex_y - perp_long[1] * (line_spacing / 2.0),
    ];
    let end1 = [
        end_x + perp_long[0] * (line_spacing / 2.0),
        end_y + perp_long[1] * (line_spacing / 2.0),
    ];
    let end2 = [
        end_x - perp_long[0] * (line_spacing / 2.0),
        end_y - perp_long[1] * (line_spacing / 2.0),
    ];

    vec![
        format!("; LAYER_HEIGHT: {}", format_python_float(conn_layer_height)),
        format!("; 加强连接线 - 层高{:.2}mm (缩短2mm)", conn_layer_height),
        "; FEATURE: Top surface".to_string(),
        format!(
            "G1 X{:.3} Y{:.3} F{}",
            start1[0],
            start1[1],
            format_speed(ir_data.machine.travel_speed * MM_PER_MINUTE)
        ),
        format!("G1 Z{:.3}", temp_z + 0.3),
        format!("G1 E{:.3}", ir_data.machine.retract_length),
        format!("G1 Z{:.3}", temp_z),
        "G4 P200".to_string(),
        format!("G1 X{:.3} Y{:.3} E{:.6} F400", end1[0], end1[1], extr_long),
        format!(
            "G1 X{:.3} Y{:.3} E{:.6} F500",
            end2[0], end2[1], extr_corner
        ),
        format!(
            "G1 X{:.3} Y{:.3} E{:.6} F500",
            start2[0], start2[1], extr_long
        ),
        format!(
            "G1 X{:.3} Y{:.3} E{:.6} F500",
            start1[0], start1[1], extr_corner
        ),
        crate::gcode::MARKER_WIPE_START.to_string(),
        format!(
            "G1 X{:.3} Y{:.3} E-{:.3} F{}",
            end1[0],
            end1[1],
            ir_data.machine.retract_length,
            format_speed(ir_data.machine.travel_speed * MM_PER_MINUTE)
        ),
        crate::gcode::MARKER_WIPE_END.to_string(),
    ]
}

// ---- SDK 段拆分（sdk.go:29-263） ----

/// `DiskWipeRequest`（sdk.go:29）。
pub struct DiskWipeRequest<'a> {
    pub supports: &'a [String],
    pub collision_check: &'a mut Vec<CollisionLayer>,
    pub disk_history: Vec<DiskHistoryEntry>,
    pub last_selected_disk: Option<&'a DiskPosition>,
    pub current_layer_height: f64,
    pub last_layer_height: f64,
    pub ir: &'a Ir,
    pub machine_type: &'a str,
    pub recent_lines: &'a [String],
    pub future_layer_count: usize,
}

/// `DiskWipeOptions`（sdk.go:75）：零值不保证语义，用 `default_disk_wipe_options`。
#[derive(Debug, Clone, Copy)]
pub struct DiskWipeOptions {
    pub pre_extrude_compensation: f64,
    pub wipe_distance: f64,
    pub wipe_retract_ratio: f64,
}

pub fn default_disk_wipe_options() -> DiskWipeOptions {
    DiskWipeOptions {
        pre_extrude_compensation: 1.2,
        wipe_distance: 1.0,
        wipe_retract_ratio: 1.0,
    }
}

/// `DiskWipeSegment`（sdk.go:110）：Connection/Paint/Finish 拆分 + 兼容拼接。
pub struct DiskWipeSegment {
    pub segment_lines: Vec<String>,
    pub connection_lines: Vec<String>,
    pub paint_lines: Vec<String>,
    pub finish_lines: Vec<String>,
    pub disk_lines: Vec<String>,
    pub selected_disk: Option<DiskPosition>,
    pub best_score: f64,
    pub is_first_layer: bool,
    pub layer_height: f64,
    pub disk_history: Vec<DiskHistoryEntry>,
}

/// `GenerateDiskWipeSegment`（sdk.go:175）：SDK 入口。
/// opts 零值回退默认（wipe_distance/wipe_retract_ratio 与 Go 同样只回退不消费）。
pub fn generate_disk_wipe_segment(
    req: DiskWipeRequest,
    opts: DiskWipeOptions,
) -> Option<DiskWipeSegment> {
    let defaults = default_disk_wipe_options();
    let factor = if opts.pre_extrude_compensation == 0.0 {
        defaults.pre_extrude_compensation
    } else {
        opts.pre_extrude_compensation
    };

    let result = generate_disk_wipe(
        req.supports,
        req.collision_check,
        req.disk_history,
        req.last_selected_disk,
        req.current_layer_height,
        req.last_layer_height,
        req.ir,
        req.machine_type,
        req.recent_lines,
        req.future_layer_count,
    )?;

    let connection_lines = apply_pre_extrude_compensation(&result.connection_lines, factor);

    let mut wipe_start_idx: Option<usize> = None;
    for (i, line) in result.gcode_lines.iter().enumerate() {
        if line.contains(crate::gcode::MARKER_WIPE_START) {
            wipe_start_idx = Some(i);
            break;
        }
    }
    let (paint_lines, finish_lines) = match wipe_start_idx {
        Some(idx) => (
            result.gcode_lines[..idx].to_vec(),
            result.gcode_lines[idx..].to_vec(),
        ),
        None => (result.gcode_lines.clone(), Vec::new()),
    };

    let mut disk_lines = paint_lines.clone();
    disk_lines.extend(finish_lines.clone());

    let mut segment_lines = connection_lines.clone();
    segment_lines.extend(disk_lines.clone());

    Some(DiskWipeSegment {
        segment_lines,
        connection_lines,
        paint_lines,
        finish_lines,
        disk_lines,
        selected_disk: result.selected_disk,
        best_score: result.best_score,
        is_first_layer: result.is_first_layer,
        layer_height: result.layer_height,
        disk_history: result.disk_history,
    })
}

/// `applyPreExtrudeCompensation`（sdk.go:250）：首段挤出行的 E×factor。
fn apply_pre_extrude_compensation(lines: &[String], factor: f64) -> Vec<String> {
    let mut out = lines.to_vec();
    for line in out.iter_mut() {
        if extract_xy(line).is_none() {
            continue;
        }
        // 正则 `E([-0-9.]+)`：找 E 后连续的 [-0-9.] 串
        let bytes = line.as_bytes();
        let mut loc: Option<(usize, usize)> = None;
        for i in 0..bytes.len() {
            if bytes[i] == b'E' {
                let mut j = i + 1;
                while j < bytes.len()
                    && (bytes[j] == b'-' || bytes[j].is_ascii_digit() || bytes[j] == b'.')
                {
                    j += 1;
                }
                if j > i + 1 {
                    loc = Some((i, j));
                }
                break;
            }
        }
        let Some((start, end)) = loc else {
            continue;
        };
        let Ok(e_val) = line[start + 1..end].parse::<f64>() else {
            continue;
        };
        let new_e = e_val * factor;
        *line = format!("{}E{:.6}{}", &line[..start], new_e, &line[end..]);
        break;
    }
    out
}
