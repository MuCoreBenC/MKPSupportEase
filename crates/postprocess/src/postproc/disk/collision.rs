//! 碰撞检测（对照 disk/collision.go 逐行）。诊断（logInfo）在旧侧是旁路输出，
//! Rust 侧结果结构体已经携带全部字段，调用方自取。

use crate::ir::Ir;
use crate::postproc::geom::{self, BBox as GBBox, MoveDirection, Polygon};
use crate::postproc::tower::calculate_tower_bbox;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CollisionType {
    #[default]
    None,
    Model,
    PrimeTower,
    Both,
    /// 擦料塔落在机型额外禁区内
    Forbidden,
    /// 模型无足够空间，建议调整擦料塔
    WipeTowerNeeded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AdjustmentTarget {
    #[default]
    Model,
    WipeTower,
}

#[derive(Debug, Clone, Copy)]
pub struct MoveAttempt {
    pub direction: MoveDirection,
    pub move_x: f64,
    pub move_y: f64,
    pub collision: bool,
    pub out_of_bounds: bool,
}

#[derive(Debug, Clone, Default)]
pub struct CollisionResult {
    pub has_collision: bool,
    pub conflict_count: i64,
    pub direction: MoveDirection,
    pub direction_cn: String,
    pub required_move_x: f64,
    pub required_move_y: f64,
    pub collision_type: CollisionType,
    pub adjustment_target: AdjustmentTarget,
    pub attempts: Vec<MoveAttempt>,
    /// 实际冲突点采样（调试/模拟用）
    pub conflict_points: Vec<[f64; 2]>,
}

/// `DetectTowerCollision`：检测擦料塔包围盒与模型/原塔挤出点的冲突。
/// `UseTowers=false` 时直接返回空结果（无 MKP 擦料塔，跳过检测）。
pub fn detect_tower_collision(content: &[String], ir_data: &Ir) -> CollisionResult {
    let mut result = CollisionResult::default();

    if !ir_data.tower.use_towers {
        return result;
    }

    let tower_bbox = calculate_tower_collision_bbox(ir_data);

    let tower_poly: Polygon = vec![
        vec![tower_bbox.x_min, tower_bbox.y_min],
        vec![tower_bbox.x_max, tower_bbox.y_min],
        vec![tower_bbox.x_max, tower_bbox.y_max],
        vec![tower_bbox.x_min, tower_bbox.y_max],
    ];

    for zone in &ir_data.machine.forbidden_zones {
        if geom::polygons_intersect(&tower_poly, zone) {
            result.has_collision = true;
            result.collision_type = CollisionType::Forbidden;
            result.conflict_count = 1;
            result.direction_cn = "擦料塔位置落在机型额外禁区内".to_string();
            let (dx, dy) = geom::min_translation_vector(&tower_poly, zone);
            result.required_move_x = dx;
            result.required_move_y = dy;
            return result;
        }
    }

    detect_tower_collision_all_layers(content, &tower_bbox, ir_data)
}

/// `isExtrusionMove`：G0/G1/G2/G3 行含 " E" 且 rest 有 X/Y，NumStrip 取 [1]/[2]。
fn is_extrusion_move(line: &str) -> Option<(f64, f64)> {
    let trimmed = line.trim();
    if !trimmed.contains(" E") {
        return None;
    }
    let is_motion = [("G0", 3), ("G1", 3), ("G2", 3), ("G3", 3)]
        .iter()
        .any(|(pfx, _)| {
            // 零分配等价形态（Task 18.2）：原 starts_with(&format!("{pfx} "))×2
            // 是每行两次堆分配；语义 = pfx 后紧跟空格或制表符
            trimmed
                .strip_prefix(pfx)
                .is_some_and(|r| r.starts_with(' ') || r.starts_with('\t'))
        });
    if !is_motion {
        return None;
    }
    let code_end = 2;
    let rest = &trimmed[code_end..];
    if !rest.contains('X') || !rest.contains('Y') {
        return None;
    }
    let nums = num_strip(trimmed);
    if nums.len() >= 3 {
        Some((nums[1], nums[2]))
    } else {
        None
    }
}

/// ir.NumStrip 的本模块内联（与 ir::build 的实现同一语义，独立副本避免跨层引用）。
fn num_strip(line: &str) -> Vec<f64> {
    let mut result = Vec::with_capacity(8);
    let mut buf = String::with_capacity(64);
    let mut has_dot = false;
    fn flush(buf: &mut String, has_dot: &mut bool, out: &mut Vec<f64>) {
        if !buf.is_empty()
            && buf.as_str() != "-"
            && buf.as_str() != "."
            && let Ok(v) = buf.parse::<f64>()
        {
            out.push(v);
        }
        buf.clear();
        *has_dot = false;
    }
    for c in line.bytes() {
        if c == b'-' && buf.is_empty() {
            buf.push('-');
        } else if c.is_ascii_digit() {
            buf.push(c as char);
        } else if c == b'.' && !has_dot && !buf.is_empty() {
            buf.push('.');
            has_dot = true;
        } else {
            flush(&mut buf, &mut has_dot, &mut result);
        }
    }
    flush(&mut buf, &mut has_dot, &mut result);
    result
}

fn detect_tower_collision_all_layers(
    content: &[String],
    tower_bbox: &GBBox,
    ir_data: &Ir,
) -> CollisionResult {
    let mut result = CollisionResult::default();

    let mut processed_count: i64 = 0;
    let mut model_conflict_count: i64 = 0;
    let mut prime_tower_conflict_count: i64 = 0;
    let mut in_prime_tower = false;
    let mut conflict_points: Vec<[f64; 2]> = Vec::new();

    let mut model_bbox_tracking = false;
    let (mut model_min_x, mut model_max_x, mut model_min_y, mut model_max_y) = (0.0, 0.0, 0.0, 0.0);
    let mut prime_tower_bbox_tracking = false;
    let (mut pt_min_x, mut pt_max_x, mut pt_min_y, mut pt_max_y) = (0.0, 0.0, 0.0, 0.0);

    let total_lines = content.len();
    let (sample_interval, max_conflict_points) = if total_lines > 10000 {
        (500i64, 500usize)
    } else if total_lines > 5000 {
        (150i64, 200usize)
    } else {
        (50i64, 200usize)
    };
    let max_cal_points = if total_lines > 10000 {
        500usize
    } else if total_lines > 5000 {
        150usize
    } else {
        100usize
    };
    let mut cal_points: Vec<[f64; 2]> = Vec::new();
    let _ = &mut cal_points;
    cal_points.reserve(max_cal_points);

    for line in content {
        let trimmed = line.trim();

        if crate::gcode::has_slicer_comment_prefix(trimmed, "FEATURE") {
            in_prime_tower =
                crate::gcode::has_slicer_comment_prefix(trimmed, "FEATURE: Prime tower");
            continue;
        }

        let Some((x_coord, y_coord)) = is_extrusion_move(line) else {
            continue;
        };

        processed_count += 1;

        if processed_count % sample_interval == 0 && cal_points.len() < max_cal_points {
            cal_points.push([x_coord, y_coord]);
        }

        let in_collision = x_coord >= tower_bbox.x_min
            && x_coord <= tower_bbox.x_max
            && y_coord >= tower_bbox.y_min
            && y_coord <= tower_bbox.y_max;

        if in_collision && conflict_points.len() < max_conflict_points {
            conflict_points.push([x_coord, y_coord]);
        }

        if in_prime_tower {
            if !prime_tower_bbox_tracking {
                prime_tower_bbox_tracking = true;
                pt_min_x = x_coord;
                pt_max_x = x_coord;
                pt_min_y = y_coord;
                pt_max_y = y_coord;
            } else {
                pt_min_x = pt_min_x.min(x_coord);
                pt_max_x = pt_max_x.max(x_coord);
                pt_min_y = pt_min_y.min(y_coord);
                pt_max_y = pt_max_y.max(y_coord);
            }
            if in_collision {
                prime_tower_conflict_count += 1;
            }
        } else {
            if !model_bbox_tracking {
                model_bbox_tracking = true;
                model_min_x = x_coord;
                model_max_x = x_coord;
                model_min_y = y_coord;
                model_max_y = y_coord;
            } else {
                model_min_x = model_min_x.min(x_coord);
                model_max_x = model_max_x.max(x_coord);
                model_min_y = model_min_y.min(y_coord);
                model_max_y = model_max_y.max(y_coord);
            }
            if in_collision {
                model_conflict_count += 1;
            }
        }
    }

    result.conflict_count = model_conflict_count + prime_tower_conflict_count;

    let has_model_collision = model_conflict_count > 0 && model_bbox_tracking;
    let has_prime_tower_collision = prime_tower_conflict_count > 0 && prime_tower_bbox_tracking;

    if has_model_collision || has_prime_tower_collision {
        result.has_collision = true;

        let mut ref_bbox = GBBox::default();
        if has_model_collision {
            ref_bbox = GBBox {
                x_min: model_min_x,
                x_max: model_max_x,
                y_min: model_min_y,
                y_max: model_max_y,
            };
            result.collision_type = CollisionType::Model;
        }
        if has_prime_tower_collision {
            let pt_bbox = GBBox {
                x_min: pt_min_x,
                x_max: pt_max_x,
                y_min: pt_min_y,
                y_max: pt_max_y,
            };
            if has_model_collision {
                ref_bbox.x_min = ref_bbox.x_min.min(pt_bbox.x_min);
                ref_bbox.x_max = ref_bbox.x_max.max(pt_bbox.x_max);
                ref_bbox.y_min = ref_bbox.y_min.min(pt_bbox.y_min);
                ref_bbox.y_max = ref_bbox.y_max.max(pt_bbox.y_max);
                result.collision_type = CollisionType::Both;
            } else {
                ref_bbox = pt_bbox;
                result.collision_type = CollisionType::PrimeTower;
            }
        }

        let bed_bbox = GBBox {
            x_min: ir_data.machine.min_x,
            x_max: ir_data.machine.max_x,
            y_min: ir_data.machine.min_y,
            y_max: ir_data.machine.max_y,
        };
        result.conflict_points = conflict_points.clone();
        result =
            resolve_model_adjustment(result, &ref_bbox, tower_bbox, &bed_bbox, &conflict_points);
    }

    result
}

/// `resolveModelAdjustment`：基于实际冲突点算各方向最小退出距离。
fn resolve_model_adjustment(
    mut result: CollisionResult,
    ref_bbox: &GBBox,
    tower_bbox: &GBBox,
    bed_bbox: &GBBox,
    conflict_points: &[[f64; 2]],
) -> CollisionResult {
    let directions = build_adjustment_directions(ref_bbox, tower_bbox);

    let mut scored: Vec<(MoveDirection, f64, f64, f64)> = Vec::new();
    for dir_ in directions {
        let (dx, dy) = dir_.delta();
        let t = min_exit_distance(conflict_points, tower_bbox, dx, dy);
        if t <= 0.0 || t.is_nan() || t.is_infinite() {
            continue;
        }
        let moved = ref_bbox.translated(dx * t, dy * t);
        if !bed_bbox.contains(&moved) {
            continue;
        }
        scored.push((dir_, dx, dy, t));
    }

    scored.sort_by(|a, b| a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal));

    const EXIT_EPSILON: f64 = 0.05; // mm，确保冲突点严格离开 tower 边界
    let mut attempts: Vec<MoveAttempt> = Vec::with_capacity(3);
    for (i, s) in scored.iter().enumerate() {
        if i >= 3 {
            break;
        }
        let (_, dx, dy, t) = *s;
        let t_with_epsilon = t + EXIT_EPSILON;
        let move_x = geom::ceil_to_01(t_with_epsilon * dx.abs());
        let move_y = geom::ceil_to_01(t_with_epsilon * dy.abs());
        let moved = ref_bbox.translated(dx * t_with_epsilon, dy * t_with_epsilon);
        let out_of_bounds = !bed_bbox.contains(&moved);
        let collision = has_conflicts_after_move(
            conflict_points,
            tower_bbox,
            dx * t_with_epsilon,
            dy * t_with_epsilon,
        );
        attempts.push(MoveAttempt {
            direction: scored[i].0,
            move_x,
            move_y,
            collision,
            out_of_bounds,
        });
        if !out_of_bounds && !collision {
            result.direction = scored[i].0;
            result.direction_cn = scored[i].0.as_str_cn().to_string();
            result.required_move_x = move_x;
            result.required_move_y = move_y;
            result.adjustment_target = AdjustmentTarget::Model;
            result.attempts = attempts;
            return result;
        }
    }

    result.direction = MoveDirection::None;
    result.direction_cn = String::new();
    result.required_move_x = 0.0;
    result.required_move_y = 0.0;
    result.collision_type = CollisionType::WipeTowerNeeded;
    result.adjustment_target = AdjustmentTarget::WipeTower;
    result.attempts = attempts;
    result
}

fn build_adjustment_directions(ref_bbox: &GBBox, tower_bbox: &GBBox) -> Vec<MoveDirection> {
    let x_overlap = ref_bbox.x_min < tower_bbox.x_max && ref_bbox.x_max > tower_bbox.x_min;
    let y_overlap = ref_bbox.y_min < tower_bbox.y_max && ref_bbox.y_max > tower_bbox.y_min;

    if x_overlap && y_overlap {
        vec![
            MoveDirection::LeftFront,
            MoveDirection::RightFront,
            MoveDirection::LeftBack,
            MoveDirection::RightBack,
        ]
    } else if x_overlap {
        vec![MoveDirection::Left, MoveDirection::Right]
    } else if y_overlap {
        vec![MoveDirection::Front, MoveDirection::Back]
    } else {
        Vec::new()
    }
}

fn has_conflicts_after_move(
    conflict_points: &[[f64; 2]],
    tower_bbox: &GBBox,
    move_x: f64,
    move_y: f64,
) -> bool {
    for p in conflict_points {
        let x = p[0] + move_x;
        let y = p[1] + move_y;
        if x >= tower_bbox.x_min
            && x <= tower_bbox.x_max
            && y >= tower_bbox.y_min
            && y <= tower_bbox.y_max
        {
            return true;
        }
    }
    false
}

fn min_exit_distance(conflict_points: &[[f64; 2]], tower_bbox: &GBBox, dx: f64, dy: f64) -> f64 {
    let mut t = 0.0;
    for p in conflict_points {
        let mut required = 0.0;
        let has_x = dx != 0.0;
        let has_y = dy != 0.0;
        if has_x && has_y {
            let mut tx = (tower_bbox.x_max - p[0]) / dx;
            if dx < 0.0 {
                tx = (tower_bbox.x_min - p[0]) / dx;
            }
            let mut ty = (tower_bbox.y_max - p[1]) / dy;
            if dy < 0.0 {
                ty = (tower_bbox.y_min - p[1]) / dy;
            }
            required = tx.max(ty);
        } else if has_x {
            required = (tower_bbox.x_max - p[0]) / dx;
            if dx < 0.0 {
                required = (tower_bbox.x_min - p[0]) / dx;
            }
        } else if has_y {
            required = (tower_bbox.y_max - p[1]) / dy;
            if dy < 0.0 {
                required = (tower_bbox.y_min - p[1]) / dy;
            }
        }
        if required > t {
            t = required;
        }
    }
    t
}

/// `calculateTowerCollisionBBox`：塔碰撞 BBox（wiperXY 为左下角）。
/// towerHeight=0 跳过 minDepth 限制（碰撞用保守估计）。
fn calculate_tower_collision_bbox(ir_data: &Ir) -> GBBox {
    let local = calculate_tower_bbox(
        &ir_data.wiping.outer_structure,
        ir_data.sheath.base_expand,
        ir_data.rib.extra_length,
        ir_data.rib.width,
        0.0,
        ir_data.machine.nozzle_diameter,
        ir_data.machine.first_layer_height,
    );
    GBBox {
        x_min: ir_data.wiping.wiper_x,
        x_max: ir_data.wiping.wiper_x + local.width(),
        y_min: ir_data.wiping.wiper_y,
        y_max: ir_data.wiping.wiper_y + local.height(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Ir;

    fn tower_ir() -> Ir {
        // wiper (20,20) + brim bbox：local [2.96, 37.04]² → machine [22.96, 57.04]²
        let mut ir = Ir::default();
        ir.tower.use_towers = true;
        ir.wiping.outer_structure = "brim".to_string();
        ir.wiping.wiper_x = 20.0;
        ir.wiping.wiper_y = 20.0;
        ir.sheath.base_expand = 5.0;
        ir.machine.nozzle_diameter = 0.4;
        ir.machine.first_layer_height = 0.2;
        ir.machine.min_x = 0.0;
        ir.machine.max_x = 256.0;
        ir.machine.min_y = 0.0;
        ir.machine.max_y = 256.0;
        ir
    }

    fn lines(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    /// 9.6 命中侧：挤出点落进塔包围盒 → has_collision + Model 类型 + 建议位移。
    #[test]
    fn collision_detected_when_model_extrudes_into_tower() {
        let ir = tower_ir();
        let content = lines(&[
            "G1 X100 Y100 E0.5 ; 远离塔",
            "G1 X30 Y30 E0.6 ; 塔内冲突点",
            "G1 X120 Y120 E0.7",
        ]);
        let r = detect_tower_collision(&content, &ir);
        assert!(r.has_collision);
        assert_eq!(r.collision_type, CollisionType::Model);
        assert!(r.conflict_count >= 1);
        assert!(!r.conflict_points.is_empty());
    }

    /// 9.6 不命中侧：模型全部远离塔 → 无冲突；且 UseTowers=false 时整体跳过。
    #[test]
    fn no_collision_when_model_away_or_towers_disabled() {
        let ir = tower_ir();
        let content = lines(&["G1 X100 Y100 E0.5", "G1 X150 Y150 E0.6"]);
        let r = detect_tower_collision(&content, &ir);
        assert!(!r.has_collision);
        assert_eq!(r.collision_type, CollisionType::None);

        let mut disabled = tower_ir();
        disabled.tower.use_towers = false;
        let r2 = detect_tower_collision(&content, &disabled);
        assert!(!r2.has_collision, "UseTowers=false 必须整体跳过检测");
    }

    /// 禁区命中：塔多边形与 ForbiddenZones 相交 → Forbidden 类型 + MTV 位移。
    #[test]
    fn forbidden_zone_collision() {
        let mut ir = tower_ir();
        ir.machine.forbidden_zones = vec![vec![
            vec![22.0, 22.0],
            vec![60.0, 22.0],
            vec![60.0, 60.0],
            vec![22.0, 60.0],
        ]];
        let content = lines(&["G1 X200 Y200 E0.5"]);
        let r = detect_tower_collision(&content, &ir);
        assert!(r.has_collision);
        assert_eq!(r.collision_type, CollisionType::Forbidden);
        assert_eq!(r.direction_cn, "擦料塔位置落在机型额外禁区内");
    }
}
