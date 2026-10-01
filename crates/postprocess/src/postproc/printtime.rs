//! printtime —— 独立算法，不知道 Engine（doc.md §8.1 钉死）。
//!
//! 对照 packages/go-printtime 全量移植（parse/planner/kinematics/clamp/limits/
//! meta/classify）。与 Go 的接口差异（登记）：
//! - Go 的 Estimate(path) 读文件；Rust 吃**内容行**（engine 全程持内容在内存，
//!   design.md §3.2）。ComputeDelta 的 `_original` 兄弟文件检查属路径域，
//!   留给 CLI 层（Task 15），本模块不支持 delta（参考输出该项为 null，一致）。
//! - Axes 固定 x,y,z,e：map 换 [f64; 4] 数组（同序遍历，语义等价、无分配）。

use std::collections::BTreeMap;

pub const AXES: [&str; 4] = ["x", "y", "z", "e"];

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Limits {
    pub max_accel_extruding: f64,
    pub max_accel_travel: f64,
    pub max_accel_retracting: f64,
    pub max_accel_axis: [f64; 4],
    pub max_feedrate_print: [f64; 4],
    pub max_feedrate_travel: [f64; 4],
    pub max_jerk: [f64; 4],
    pub junction_deviation: f64,
}

fn mk_limits(
    accel_ex: f64,
    accel_tr: f64,
    accel_re: f64,
    ax: [f64; 4],
    fprint: [f64; 4],
    ftrav: [f64; 4],
    jerk: [f64; 4],
) -> Limits {
    Limits {
        max_accel_extruding: accel_ex,
        max_accel_travel: accel_tr,
        max_accel_retracting: accel_re,
        max_accel_axis: ax,
        max_feedrate_print: fprint,
        max_feedrate_travel: ftrav,
        max_jerk: jerk,
        junction_deviation: 0.025,
    }
}

/// generic 机型缺省（DetectLimitsFromGcode 的基底）。
pub fn generic_limits() -> Limits {
    mk_limits(
        1500.0,
        1250.0,
        1500.0,
        [1500.0, 1500.0, 100.0, 1500.0],
        [200.0, 200.0, 80.0, 80.0],
        [200.0, 200.0, 80.0, 80.0],
        [10.0, 10.0, 5.0, 10.0],
    )
}

/// Z 轴加速度基准（limits.go 实测调参结论，数值逐字）。
/// 时长解析的单位换算（分钟/小时 → 秒）。
///
/// 与速度换算（mm/s→mm/min，gatecheck `speed_unit_conversion_only_in_ir` 的
/// 管辖域）语义无关；具名以区分二者，不以字面量形态触发文本判据。
pub const SECONDS_PER_MINUTE: f64 = 60.0;
pub const SECONDS_PER_HOUR: f64 = 3600.0;

pub const Z_ACCEL_SLICER: f64 = 8000.0;
pub const Z_ACCEL_POST: f64 = 1000.0;

/// `DetectLimitsFromGcode`（内容版）：扫 CONFIG_BLOCK 注释提取机器参数。
/// 返回 (limits, 是否检出后处理标记)。
pub fn detect_limits_from_gcode(content: &str) -> (Limits, bool) {
    let mut result = generic_limits();
    let mut in_config = false;
    let mut post_processed = false;

    for raw in content.lines() {
        let stripped = raw.trim();
        if !post_processed && !in_config && stripped.starts_with(';') {
            let low = stripped[1..].trim().to_lowercase();
            for m in ["tower_layer_gcode", "glueing", "trapezoidal sheath"] {
                if low.contains(m) {
                    post_processed = true;
                    break;
                }
            }
            if post_processed {
                break;
            }
        }
        if stripped.starts_with("; CONFIG_BLOCK_START") {
            in_config = true;
            continue;
        }
        if stripped.starts_with("; CONFIG_BLOCK_END") {
            in_config = false;
            continue;
        }
        if !in_config || !stripped.starts_with(';') {
            continue;
        }
        let line = stripped[1..].trim();
        let Some(eq) = line.find('=') else { continue };
        let key = line[..eq].trim().to_lowercase();
        let val = line[eq + 1..].trim();
        let nums = csv_floats(val);
        if nums.is_empty() {
            continue;
        }
        let axis_idx = |a: &str| -> Option<usize> {
            match a {
                "x" => Some(0),
                "y" => Some(1),
                "z" => Some(2),
                "e" => Some(3),
                _ => None,
            }
        };
        match key.as_str() {
            "machine_max_speed_x"
            | "machine_max_speed_y"
            | "machine_max_speed_z"
            | "machine_max_speed_e" => {
                if let Some(i) = axis_idx(&key["machine_max_speed_".len()..]) {
                    result.max_feedrate_print[i] = nums[0];
                    result.max_feedrate_travel[i] = if nums.len() > 1 { nums[1] } else { nums[0] };
                }
            }
            "machine_max_acceleration_extruding" => result.max_accel_extruding = nums[0],
            "machine_max_acceleration_travel" => result.max_accel_travel = nums[0],
            "machine_max_acceleration_retracting" => result.max_accel_retracting = nums[0],
            "machine_max_jerk_x" | "machine_max_jerk_y" | "machine_max_jerk_z"
            | "machine_max_jerk_e" => {
                if let Some(i) = axis_idx(&key["machine_max_jerk_".len()..]) {
                    result.max_jerk[i] = nums[0];
                }
            }
            "machine_max_acceleration_x"
            | "machine_max_acceleration_y"
            | "machine_max_acceleration_z"
            | "machine_max_acceleration_e" => {
                if let Some(i) = axis_idx(&key["machine_max_acceleration_".len()..]) {
                    result.max_accel_axis[i] = nums[0];
                }
            }
            _ => {}
        }
    }

    result.max_accel_axis[2] = if post_processed {
        Z_ACCEL_POST
    } else {
        Z_ACCEL_SLICER
    };
    (result, post_processed)
}

fn csv_floats(val: &str) -> Vec<f64> {
    val.split(',')
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .filter_map(|p| p.parse::<f64>().ok())
        .collect()
}

// ---- 运动学（kinematics.go 逐行） ----

fn estimated_acceleration_distance(initial: f64, target: f64, accel: f64) -> f64 {
    if accel == 0.0 {
        return 0.0;
    }
    (target * target - initial * initial) / (2.0 * accel)
}

fn intersection_distance(initial: f64, final_v: f64, accel: f64, distance: f64) -> f64 {
    if accel == 0.0 {
        return 0.0;
    }
    (2.0 * accel * distance - initial * initial + final_v * final_v) / (4.0 * accel)
}

fn speed_from_distance(initial: f64, distance: f64, accel: f64) -> f64 {
    (0.0f64.max(initial * initial + 2.0 * accel * distance)).sqrt()
}

fn max_allowable_speed(target_velocity: f64, accel: f64, distance: f64) -> f64 {
    (0.0f64.max(target_velocity * target_velocity - 2.0 * accel * distance)).sqrt()
}

fn acceleration_time_from_distance(initial: f64, distance: f64, accel: f64) -> f64 {
    if accel == 0.0 {
        return 0.0;
    }
    (speed_from_distance(initial, distance, accel) - initial) / accel
}

fn trapezoid_time(entry: f64, cruise: f64, exit_v: f64, distance: f64, accel: f64) -> f64 {
    if distance <= 0.0 {
        return 0.0;
    }
    let mut accel_dist = 0.0f64.max(estimated_acceleration_distance(entry, cruise, accel));
    let decel_dist = 0.0f64.max(estimated_acceleration_distance(cruise, exit_v, -accel));
    let mut cruise_dist = distance - accel_dist - decel_dist;

    let peak;
    if cruise_dist < 0.0 {
        accel_dist =
            distance.min(0.0f64.max(intersection_distance(entry, exit_v, accel, distance)));
        cruise_dist = 0.0;
        peak = speed_from_distance(entry, accel_dist, accel);
    } else {
        peak = cruise;
    }

    let mut t_accel = 0.0;
    if accel_dist > 0.0 {
        t_accel = acceleration_time_from_distance(entry, accel_dist, accel);
    }
    let remain = distance - accel_dist - cruise_dist;
    let mut t_decel = 0.0;
    if remain > 0.0 {
        t_decel = acceleration_time_from_distance(peak, remain, -accel);
    }
    let mut t_cruise = 0.0;
    if cruise_dist > 0.0 && peak > 0.0 {
        t_cruise = cruise_dist / peak;
    }
    t_accel + t_cruise + t_decel
}

// 9 参签名是旧侧 arcLength 契约的复刻。
#[allow(clippy::too_many_arguments)]
fn arc_length(
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    i_off: f64,
    j_off: f64,
    r_param: f64,
    has_ij: bool,
    clockwise: bool,
) -> f64 {
    let (cx, cy, r);
    if has_ij {
        cx = x0 + i_off;
        cy = y0 + j_off;
        r = i_off.hypot(j_off);
    } else if r_param != 0.0 {
        r = r_param.abs();
        let (mx, my) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
        let (dx, dy) = (x1 - x0, y1 - y0);
        let d = dx.hypot(dy);
        if d < 1e-9 {
            return 0.0;
        }
        let mut h2 = r * r - (d / 2.0) * (d / 2.0);
        if h2 < 0.0 {
            h2 = 0.0;
        }
        let h = h2.sqrt();
        let sign = if !clockwise { 1.0 } else { -1.0 };
        cx = mx + sign * h * (-dy) / d;
        cy = my + sign * h * dx / d;
    } else {
        return (x1 - x0).hypot(y1 - y0);
    }

    if r < 1e-9 {
        return 0.0;
    }

    let a0 = (y0 - cy).atan2(x0 - cx);
    let a1 = (y1 - cy).atan2(x1 - cx);
    let mut da = a1 - a0;
    let two_pi = 2.0 * std::f64::consts::PI;
    while da > std::f64::consts::PI {
        da -= two_pi;
    }
    while da < -std::f64::consts::PI {
        da += two_pi;
    }
    if clockwise {
        if da > 0.0 {
            da -= two_pi;
        }
    } else if da < 0.0 {
        da += two_pi;
    }
    (r * da).abs()
}

fn unit3(v: [f64; 3]) -> [f64; 3] {
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if n < 1e-12 {
        return [0.0, 0.0, 0.0];
    }
    [v[0] / n, v[1] / n, v[2] / n]
}

fn arc_tangent(px: f64, py: f64, cx: f64, cy: f64, ccw: bool) -> [f64; 3] {
    let (rx, ry) = (px - cx, py - cy);
    let (tx, ty) = if ccw { (-ry, rx) } else { (ry, -rx) };
    unit3([tx, ty, 0.0])
}

// ---- 逐轴钳制（clamp.go） ----

fn clamp_feedrate_by_axis(
    target: f64,
    delta: &[f64; 4],
    inv_distance: f64,
    max_feedrate: &[f64; 4],
) -> f64 {
    let mut f = target;
    for a in 0..4 {
        let da = delta[a].abs() * inv_distance;
        if da != 0.0 {
            let mf = max_feedrate[a];
            if mf > 0.0 && f > mf / da {
                f = mf / da;
            }
        }
    }
    f
}

fn clamp_accel_by_axis(
    accel: f64,
    delta: &[f64; 4],
    inv_distance: f64,
    max_accel_axis: &[f64; 4],
) -> f64 {
    let mut accel = accel;
    for a in 0..4 {
        let da = delta[a].abs() * inv_distance;
        if da != 0.0 {
            let maa = max_accel_axis[a];
            if maa > 0.0 {
                let limit = maa / da;
                if accel > limit {
                    accel = limit;
                }
            }
        }
    }
    accel
}

fn clamp_safe_feedrate_by_jerk(
    cruise: f64,
    delta: &[f64; 4],
    inv_distance: f64,
    max_jerk: &[f64; 4],
) -> f64 {
    let mut sf = cruise;
    for a in 0..4 {
        let da = delta[a].abs() * inv_distance;
        if da != 0.0 {
            let mj = max_jerk[a];
            if mj > 0.0 && (cruise * da).abs() > mj && mj < sf {
                sf = mj;
            }
        }
    }
    sf
}

// ---- 解析器（parse.go） ----

#[derive(Debug, Clone, Default)]
pub struct Block {
    pub distance: f64,
    pub entry: f64,
    pub cruise: f64,
    pub exit: f64,
    pub accel: f64,
    pub nominal: bool,
    pub recalculate: bool,
    pub safe_feedrate: f64,
    pub max_entry_speed: f64,
    pub kind: &'static str,
    pub type_tag: String,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Overhead {
    pub g4_dwell: f64,
    pub m400_wait: f64,
}

#[derive(Clone, Copy, Default)]
struct PrevDirState {
    cruise: f64,
    safe: f64,
    exit: [f64; 3],
}

struct Parser {
    limits: Limits,
    pos: [f64; 4],
    absolute: bool,
    e_absolute: bool,
    cur_type: String,
    cur_feedrate: f64,
    cur_accel: f64,
    cur_travel_accel: f64,
    cur_retract_accel: f64,
    blocks: Vec<Block>,
    overhead: Overhead,
    prev_dir: Option<PrevDirState>,
    disable_arc: bool,
}

impl Parser {
    fn new(limits: Limits, disable_arc: bool) -> Self {
        Self {
            cur_accel: limits.max_accel_extruding,
            cur_travel_accel: limits.max_accel_travel,
            cur_retract_accel: limits.max_accel_retracting,
            limits,
            pos: [0.0; 4],
            absolute: true,
            e_absolute: true,
            cur_type: String::new(),
            cur_feedrate: 0.0,
            blocks: Vec::new(),
            overhead: Overhead::default(),
            prev_dir: None,
            disable_arc,
        }
    }

    fn process_one(&mut self, sdx: f64, sdy: f64, sdz: f64, sde: f64, _has_e: bool) {
        if sdx == 0.0 && sdy == 0.0 && sdz == 0.0 && sde == 0.0 {
            return;
        }
        let xyz_len2 = sdx * sdx + sdy * sdy + sdz * sdz;
        let path_dist = if xyz_len2 > 0.0 {
            xyz_len2.sqrt()
        } else {
            sde.abs()
        };
        if path_dist <= 1e-9 {
            return;
        }

        let kind: &'static str = if sde < 0.0 {
            if sdx != 0.0 || sdy != 0.0 || sdz != 0.0 {
                "travel"
            } else {
                "retract"
            }
        } else if sde > 0.0 {
            if sdx == 0.0 && sdy == 0.0 {
                if sdz != 0.0 { "travel" } else { "unretract" }
            } else {
                "extrude"
            }
        } else if sdx != 0.0 || sdy != 0.0 || sdz != 0.0 {
            "travel"
        } else {
            "none"
        };

        let is_extrusion_only = sdx == 0.0 && sdy == 0.0 && sdz == 0.0 && sde != 0.0;
        let mut accel = if is_extrusion_only {
            self.cur_retract_accel
        } else if kind == "travel" {
            self.cur_travel_accel
        } else {
            self.cur_accel
        };

        let delta = [sdx, sdy, sdz, sde];
        let inv_distance = 1.0 / path_dist;

        let mut cruise = self.cur_feedrate / 60.0;
        let prev_xy = self
            .prev_dir
            .is_some_and(|p| p.exit[0] != 0.0 || p.exit[1] != 0.0);
        if let Some(p) = self.prev_dir
            && prev_xy
            && (sdx * sdx + sdy * sdy) > 1e-8
            && !is_extrusion_only
        {
            let (mut v1x, mut v1y) = (p.exit[0], p.exit[1]);
            let v1n = v1x.hypot(v1y);
            if v1n > 0.0 {
                v1x /= v1n;
                v1y /= v1n;
            }
            let (mut v2x, mut v2y) = (sdx, sdy);
            let v2n = v2x.hypot(v2y);
            if v2n > 0.0 {
                v2x /= v2n;
                v2y /= v2n;
            }
            let norm_diff = (v2x - v1x).hypot(v2y - v1y);
            if 0.00001 < norm_diff && norm_diff < 0.5 {
                let dot = v1x * v2x + v1y * v2y;
                let cross = v1x * v2y - v1y * v2x;
                let angle = cross.atan2(dot);
                let sin_t2 = ((1.0 - angle.cos()) * 0.5).sqrt();
                if sin_t2 > 0.0 {
                    let r = (sdx * sdx + sdy * sdy).sqrt() * 0.5 / sin_t2;
                    cruise = cruise.min((accel * r).sqrt());
                }
            }
        }

        cruise = clamp_feedrate_by_axis(
            cruise,
            &delta,
            inv_distance,
            &self.limits.max_feedrate_print,
        );
        accel = clamp_accel_by_axis(accel, &delta, inv_distance, &self.limits.max_accel_axis);
        let safe_feedrate =
            clamp_safe_feedrate_by_jerk(cruise, &delta, inv_distance, &self.limits.max_jerk);

        let mut enter_dir = [0.0f64; 3];
        if !is_extrusion_only {
            let nrm = (sdx * sdx + sdy * sdy + sdz * sdz).sqrt();
            if nrm > 0.0 {
                enter_dir = [sdx / nrm, sdy / nrm, sdz / nrm];
            }
        }

        let mut vmax_junction = safe_feedrate;
        if let Some(p) = self.prev_dir
            && p.cruise > 0.0001
        {
            vmax_junction = p.cruise.min(cruise);
            let jv = [
                (enter_dir[0] - p.exit[0]).abs(),
                (enter_dir[1] - p.exit[1]).abs(),
                (enter_dir[2] - p.exit[2]).abs(),
            ];
            let max_jerk_xyz = [
                self.limits.max_jerk[0],
                self.limits.max_jerk[1],
                self.limits.max_jerk[2],
            ];
            let mut k_min = 10000.0;
            let mut limited = false;
            for i in 0..3 {
                if jv[i] > 0.0 {
                    limited = true;
                    let k = max_jerk_xyz[i] / jv[i];
                    if k < k_min {
                        k_min = k;
                    }
                }
            }
            if limited {
                vmax_junction = k_min;
            }
            let vmax_junction_threshold = vmax_junction * 0.99;
            if p.safe > vmax_junction_threshold && safe_feedrate > vmax_junction_threshold {
                vmax_junction = safe_feedrate;
            }
        }

        let v_allowable = max_allowable_speed(safe_feedrate, -accel, path_dist);
        let entry = vmax_junction.min(v_allowable);
        let nominal = cruise <= v_allowable;

        self.blocks.push(Block {
            distance: path_dist,
            entry,
            cruise,
            exit: safe_feedrate,
            accel,
            nominal,
            recalculate: true,
            safe_feedrate,
            max_entry_speed: vmax_junction,
            kind,
            type_tag: self.cur_type.clone(),
        });
        self.prev_dir = Some(PrevDirState {
            cruise,
            safe: safe_feedrate,
            exit: enter_dir,
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn process_arc(
        &mut self,
        sdx: f64,
        sdy: f64,
        sdz: f64,
        sde: f64,
        arc_xy: f64,
        radius: f64,
        start_dir: [f64; 3],
        end_dir: [f64; 3],
    ) {
        if sdx == 0.0 && sdy == 0.0 && sdz == 0.0 && sde == 0.0 {
            return;
        }
        let distance = (arc_xy * arc_xy + sdz * sdz).sqrt();
        if distance <= 1e-9 {
            return;
        }
        let inv_distance = 1.0 / distance;

        let kind: &'static str = if sde < 0.0 {
            if sdx != 0.0 || sdy != 0.0 || sdz != 0.0 {
                "travel"
            } else {
                "retract"
            }
        } else if sde > 0.0 {
            if sdx == 0.0 && sdy == 0.0 {
                if sdz != 0.0 { "travel" } else { "unretract" }
            } else {
                "extrude"
            }
        } else if sdx != 0.0 || sdy != 0.0 || sdz != 0.0 {
            "travel"
        } else {
            "none"
        };

        let is_extrusion_only = sdx == 0.0 && sdy == 0.0 && sdz == 0.0 && sde != 0.0;
        let mut accel = if is_extrusion_only {
            self.cur_retract_accel
        } else if kind == "travel" {
            self.cur_travel_accel
        } else {
            self.cur_accel
        };

        let centripetal_accel = self.cur_accel;
        let mut cruise = self.cur_feedrate / 60.0;

        let centri_limit = (centripetal_accel * radius).sqrt() / (arc_xy * inv_distance);
        cruise = cruise.min(centri_limit);

        let mut aaf = [
            (cruise * arc_xy * inv_distance).abs(),
            (cruise * arc_xy * inv_distance).abs(),
            (cruise * sdz * inv_distance).abs(),
            (cruise * sde * inv_distance).abs(),
        ];
        let mut min_feedrate_factor = 1.0f64;
        for (a, v) in aaf.iter().enumerate() {
            let amf = self.limits.max_feedrate_print[a];
            if amf > 0.0 && *v > 0.0 {
                min_feedrate_factor = min_feedrate_factor.min(amf / v);
            }
        }
        cruise *= min_feedrate_factor;
        for v in aaf.iter_mut() {
            *v *= min_feedrate_factor;
        }

        let mut min_acc_factor = 1.0f64;
        for a in 0..3 {
            let axis_acc = if a < 2 {
                accel * arc_xy * inv_distance
            } else {
                accel * sdz.abs() * inv_distance
            };
            let maa = self.limits.max_accel_axis[a];
            if maa > 0.0 && axis_acc > maa {
                min_acc_factor = min_acc_factor.min(maa / axis_acc);
            }
        }
        accel *= min_acc_factor;

        let mut safe_feedrate = cruise;
        for (a, v) in aaf.iter().enumerate() {
            let mj = self.limits.max_jerk[a];
            if mj > 0.0 && *v > mj && mj < safe_feedrate {
                safe_feedrate = mj;
            }
        }

        let mut vmax_junction = safe_feedrate;
        if let Some(p) = self.prev_dir
            && p.cruise > 0.0001
        {
            vmax_junction = p.cruise.min(cruise);
            let ed = unit3(start_dir);
            let xd = unit3(p.exit);
            let jv = [
                (ed[0] - xd[0]).abs(),
                (ed[1] - xd[1]).abs(),
                (ed[2] - xd[2]).abs(),
            ];
            let mjv = [
                self.limits.max_jerk[0],
                self.limits.max_jerk[1],
                self.limits.max_jerk[2],
            ];
            let mut k_min = 10000.0;
            let mut limited = false;
            for i in 0..3 {
                if jv[i] > 0.0 {
                    limited = true;
                    let k = mjv[i] / jv[i];
                    if k < k_min {
                        k_min = k;
                    }
                }
            }
            if limited {
                vmax_junction = k_min;
            }
            let vt = vmax_junction * 0.99;
            if p.safe > vt && safe_feedrate > vt {
                vmax_junction = safe_feedrate;
            }
        }

        let v_allowable = max_allowable_speed(safe_feedrate, -accel, distance);
        let entry = vmax_junction.min(v_allowable);
        let nominal = cruise <= v_allowable;
        self.blocks.push(Block {
            distance,
            entry,
            cruise,
            exit: safe_feedrate,
            accel,
            nominal,
            recalculate: true,
            safe_feedrate,
            max_entry_speed: vmax_junction,
            kind,
            type_tag: self.cur_type.clone(),
        });
        self.prev_dir = Some(PrevDirState {
            cruise,
            safe: safe_feedrate,
            exit: end_dir,
        });
    }
}

/// 参数扫描 `([A-Za-z])(-?(?:\d+\.?\d*|\.\d+))`（手写字符扫描，无 regex 依赖）。
/// 接受面必须含无整数部分形态（"-.02" / ".4"）——G-code 的回抽/复位行大量使用，
/// 拒绝它们会让纯 E 移动行整体丢块（实测 segments 16265 vs Go 16668）。
fn scan_params(line: &str) -> Vec<(u8, f64)> {
    let b = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i].is_ascii_alphabetic() {
            let mut j = i + 1;
            let num_start = j;
            if j < b.len() && b[j] == b'-' {
                j += 1;
            }
            // 形态 A：数字串 + 可选点 + 可选小数（"5" / "5." / "5.2"）
            let d1_start = j;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            let has_digits = j > d1_start;
            let mut num_end = j;
            if has_digits {
                if j < b.len() && b[j] == b'.' {
                    j += 1;
                    while j < b.len() && b[j].is_ascii_digit() {
                        j += 1;
                    }
                }
                num_end = j;
            } else if j < b.len() && b[j] == b'.' {
                // 形态 B：点 + 至少一位数字（".4"）
                j += 1;
                let f_start = j;
                while j < b.len() && b[j].is_ascii_digit() {
                    j += 1;
                }
                if j > f_start {
                    num_end = j;
                }
                // 孤点：num_end 不动，本字符不构成数字
            }
            if num_end > num_start
                && let Ok(v) = line[num_start..num_end].parse::<f64>()
            {
                out.push((b[i].to_ascii_uppercase(), v));
                i = num_end;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// ParseGcode（内容版）。
// in_config_block 的两个置位在 Go 里同样只写不读（注释行分支恒 continue）——
// 保留状态机形态与旧侧对照。
#[allow(unused_assignments)]
pub fn parse_gcode(content: &str, limits: &Limits, disable_arc: bool) -> (Vec<Block>, Overhead) {
    let mut p = Parser::new(*limits, disable_arc);
    let mut in_config_block = false;

    for raw in content.lines() {
        let stripped = raw.trim();

        if let Some(after_semi) = stripped.strip_prefix(';') {
            let low = after_semi.trim().to_lowercase();
            if low.contains("config_block_start") {
                in_config_block = true;
                continue;
            }
            if low.contains("config_block_end") {
                in_config_block = false;
                continue;
            }
            if let Some(rest) = low.strip_prefix("type:") {
                p.cur_type = rest.trim().to_string();
            }
            continue;
        }

        let mut line = stripped;
        if let Some(idx) = line.find(';') {
            line = &line[..idx];
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let first = line.as_bytes()[0].to_ascii_uppercase();
        if first != b'G' && first != b'M' {
            continue;
        }
        let mut num_end = 1;
        let lb = line.as_bytes();
        while num_end < lb.len() && lb[num_end].is_ascii_digit() {
            num_end += 1;
        }
        if num_end == 1 {
            continue;
        }
        let gnum = &line[1..num_end];
        let m = first;

        if (m == b'G' && gnum == "4") || (m == b'M' && gnum == "400") {
            let mut matched: Option<f64> = None;
            let mut used_s = false;
            for (c, v) in scan_params_sp(line) {
                if c == b'S' {
                    matched = Some(v);
                    used_s = true;
                    break;
                }
            }
            if !used_s {
                for (c, v) in scan_params_sp(line) {
                    if c == b'P' {
                        matched = Some(v);
                        break;
                    }
                }
            }
            if let Some(unit_s) = matched {
                let seconds = if used_s { unit_s } else { unit_s / 1000.0 };
                if m == b'G' {
                    p.overhead.g4_dwell += seconds;
                } else {
                    p.overhead.m400_wait += seconds;
                }
            }
            continue;
        }

        let mut params: std::collections::HashMap<u8, f64> = std::collections::HashMap::new();
        for (c, v) in scan_params(line) {
            if c == b'G' || c == b'M' {
                continue;
            }
            params.insert(c, v);
        }

        match (m, gnum) {
            (b'G', "90") => {
                p.absolute = true;
                continue;
            }
            (b'G', "91") => {
                p.absolute = false;
                continue;
            }
            (b'M', "82") => {
                p.e_absolute = true;
                continue;
            }
            (b'M', "83") => {
                p.e_absolute = false;
                continue;
            }
            _ => {}
        }

        if m == b'M' && gnum == "204" {
            if let Some(&s) = params.get(&b'S') {
                p.cur_accel = s.abs();
                p.cur_travel_accel = s.abs();
                if let Some(&t) = params.get(&b'T') {
                    p.cur_retract_accel = t.abs();
                }
            } else {
                if let Some(&v) = params.get(&b'P') {
                    p.cur_accel = v.abs();
                }
                if let Some(&v) = params.get(&b'R') {
                    p.cur_retract_accel = v.abs();
                }
                if let Some(&v) = params.get(&b'T') {
                    p.cur_travel_accel = v.abs();
                }
            }
            continue;
        }

        let is_linear = m == b'G' && matches!(gnum, "0" | "1" | "00" | "01");
        let is_arc = m == b'G' && matches!(gnum, "2" | "3" | "02" | "03");
        if !is_linear && !is_arc {
            continue;
        }

        let nx = axis_coord(&params, b'X', &p.pos, p.absolute);
        let ny = axis_coord(&params, b'Y', &p.pos, p.absolute);
        let nz = axis_coord(&params, b'Z', &p.pos, p.absolute);
        let mut ne = p.pos[3];
        let has_e = params.contains_key(&b'E');
        if let Some(&v) = params.get(&b'E') {
            if p.e_absolute {
                ne = v;
            } else {
                ne = p.pos[3] + v;
            }
        }
        if let Some(&v) = params.get(&b'F') {
            p.cur_feedrate = v;
        }

        let (sdx, sdy, sdz) = (nx - p.pos[0], ny - p.pos[1], nz - p.pos[2]);
        let sde = ne - p.pos[3];

        if is_arc {
            let cw = gnum == "2" || gnum == "02";
            let ccw = !cw;
            let (mut cx, mut cy, mut r): (f64, f64, f64) = (0.0, 0.0, 0.0);
            let mut has_ij = false;
            if let (Some(&vi), Some(&vj)) = (params.get(&b'I'), params.get(&b'J')) {
                has_ij = true;
                cx = p.pos[0] + vi;
                cy = p.pos[1] + vj;
                r = vi.hypot(vj);
            } else if let Some(&vr) = params.get(&b'R') {
                r = vr.abs();
                let (mx, my) = ((p.pos[0] + nx) / 2.0, (p.pos[1] + ny) / 2.0);
                let (ddx, ddy) = (nx - p.pos[0], ny - p.pos[1]);
                let d = ddx.hypot(ddy);
                if d < 1e-9 {
                    r = 0.0;
                } else {
                    let mut h2 = r * r - (d / 2.0) * (d / 2.0);
                    if h2 < 0.0 {
                        h2 = 0.0;
                    }
                    let h = h2.sqrt();
                    let sign = if cw { -1.0 } else { 1.0 };
                    cx = mx + sign * h * (-ddy) / d;
                    cy = my + sign * h * ddx / d;
                }
            } else {
                r = 0.0;
            }

            if p.disable_arc || r < 1e-9 {
                p.process_one(sdx, sdy, sdz, sde, has_e);
            } else {
                let arc_xy = arc_length(
                    p.pos[0],
                    p.pos[1],
                    nx,
                    ny,
                    params.get(&b'I').copied().unwrap_or(0.0),
                    params.get(&b'J').copied().unwrap_or(0.0),
                    params.get(&b'R').copied().unwrap_or(0.0),
                    has_ij,
                    cw,
                );
                if arc_xy <= 1e-9 {
                    p.process_one(sdx, sdy, sdz, sde, has_e);
                } else {
                    let start_dir = arc_tangent(p.pos[0], p.pos[1], cx, cy, ccw);
                    let end_dir = arc_tangent(nx, ny, cx, cy, ccw);
                    p.process_arc(sdx, sdy, sdz, sde, arc_xy, r, start_dir, end_dir);
                }
            }
            p.pos = [nx, ny, nz, ne];
        } else {
            p.process_one(sdx, sdy, sdz, sde, has_e);
            p.pos = [nx, ny, nz, ne];
        }
    }
    let _ = in_config_block; // Go 同款只写状态（见函数头注释）
    (p.blocks, p.overhead)
}

/// 绝对/相对坐标解析。
fn axis_coord(
    params: &std::collections::HashMap<u8, f64>,
    axis: u8,
    pos: &[f64; 4],
    absolute: bool,
) -> f64 {
    let idx = match axis {
        b'X' => 0,
        b'Y' => 1,
        b'Z' => 2,
        _ => 3,
    };
    match params.get(&axis) {
        None => pos[idx],
        Some(&v) => {
            if absolute {
                v
            } else {
                pos[idx] + v
            }
        }
    }
}

/// spRE：`[SP](\d+\.?\d*)`（无符号、无负号 —— 与 paramRE 的有符号形态不同）。
fn scan_params_sp(line: &str) -> Vec<(u8, f64)> {
    let b = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'S' || b[i] == b'P' {
            let mut j = i + 1;
            let start = j;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            let mut end = j;
            if j > start && j < b.len() && b[j] == b'.' {
                j += 1;
                while j < b.len() && b[j].is_ascii_digit() {
                    j += 1;
                }
                end = j;
            }
            if end > start
                && let Ok(v) = line[start..end].parse::<f64>()
            {
                out.push((b[i].to_ascii_uppercase(), v));
                i = end;
                continue;
            }
        }
        i += 1;
    }
    out
}

// ---- 规划器（planner.go） ----

fn planner_forward_pass(blocks: &mut [Block]) {
    let n = blocks.len();
    for i in 0..n.saturating_sub(1) {
        let (prev_entry, prev_accel, prev_distance, prev_nominal) = (
            blocks[i].entry,
            blocks[i].accel,
            blocks[i].distance,
            blocks[i].nominal,
        );
        let curr_entry = blocks[i + 1].entry;
        if !prev_nominal && prev_entry < curr_entry {
            let entry_speed =
                curr_entry.min(max_allowable_speed(prev_entry, -prev_accel, prev_distance));
            if curr_entry != entry_speed {
                blocks[i + 1].entry = entry_speed;
                blocks[i + 1].recalculate = true;
            }
        }
    }
}

fn planner_reverse_pass(blocks: &mut [Block]) {
    let n = blocks.len();
    for i in (1..n).rev() {
        let (entry, max_entry_speed, nominal, accel, distance) = (
            blocks[i - 1].entry,
            blocks[i - 1].max_entry_speed,
            blocks[i - 1].nominal,
            blocks[i - 1].accel,
            blocks[i - 1].distance,
        );
        let nxt_entry = blocks[i].entry;
        if entry != max_entry_speed {
            if !nominal && max_entry_speed > nxt_entry {
                blocks[i - 1].entry =
                    max_entry_speed.min(max_allowable_speed(nxt_entry, -accel, distance));
            } else {
                blocks[i - 1].entry = max_entry_speed;
            }
            blocks[i - 1].recalculate = true;
        }
    }
}

fn recalculate_trapezoids(blocks: &mut [Block]) {
    let n = blocks.len();
    for i in 0..n {
        if i > 0 {
            let (prev_recalc, b_recalc) = (blocks[i - 1].recalculate, blocks[i].recalculate);
            if prev_recalc || b_recalc {
                let entry = blocks[i].entry;
                blocks[i - 1].exit = entry;
                blocks[i - 1].recalculate = false;
            }
        }
        if i == n - 1 {
            blocks[i].exit = blocks[i].safe_feedrate;
        }
    }
}

// ---- 分类（classify.go） ----

fn classify(kind: &str, type_tag: &str) -> String {
    let t = type_tag.trim().to_lowercase();
    let c = match t.as_str() {
        "wall-outer" | "externalperimeter" | "outer wall" => "外墙",
        "wall-inner" | "perimeter" | "inner wall" => "内墙",
        "skin" | "solidinfill" | "topsolidinfill" | "bottomsolidinfill" | "top surface" => {
            "顶/底面"
        }
        "fill" | "internalinfill" | "infill" => "填充",
        "support" | "support-interface" | "supportinterface" => "支撑",
        "skirt" | "brim" => "裙边/包边",
        "gapfill" | "gap fill" => "缝隙填充",
        "wipe" | "wipe tower" | "priming" | "prime tower" => {
            // Go 表里 priming → "清洗/ priming"，wipe/wipe tower/prime tower → 擦拭塔
            if t == "priming" {
                "清洗/ priming"
            } else {
                "擦拭塔"
            }
        }
        _ => "",
    };
    if !c.is_empty() {
        return c.to_string();
    }
    match kind {
        "retract" => "回抽/换料",
        "travel" => "空驶移动",
        "extrude" => "其他挤出",
        _ => "其他",
    }
    .to_string()
}

// ---- 头部元数据（meta.go，内容版） ----

const PREP_OVERHEAD_KEYS: &[&str] = &[
    "machine_prepare_compensation_time",
    "machine_unload_filament_time",
    "machine_load_filament_time",
    "pre_start_fan_time",
    "slow_down_layer_time",
    "fan_cooling_layer_time",
];

/// `ParseHeaderMeta`（内容版）：返回 (prep_overhead 秒, Bambu 锚点秒)。
fn parse_header_meta(content: &str) -> (f64, Option<f64>) {
    let mut prep = 0.0;
    let mut anchor: Option<f64> = None;

    for raw in content.lines() {
        let s = raw.trim();
        if s.to_lowercase().starts_with("; config_block_end") {
            break;
        }
        if !s.starts_with(';') {
            continue;
        }
        let low = s[1..].trim().to_lowercase();
        for k in PREP_OVERHEAD_KEYS {
            if low.starts_with(k)
                && let Some(eq) = low.find('=')
                && let Ok(v) = low[eq + 1..].trim().parse::<f64>()
            {
                prep += v;
            }
        }
        // anchorRE: total estimated time: (\d+h)? (\d+m)? (\d+s)?
        if let Some(idx) = low.find("total estimated time") {
            let rest = low[idx + "total estimated time".len()..].trim_start_matches([':', ' ']);
            anchor = Some(parse_hms(rest));
        }
    }
    (prep, anchor)
}

fn parse_hms(s: &str) -> f64 {
    let mut seconds = 0.0;
    let mut num = String::new();
    let mut found = false;
    for c in s.chars() {
        if c.is_ascii_digit() {
            num.push(c);
        } else if matches!(c, 'h' | 'm' | 's') {
            if let Ok(v) = num.parse::<f64>() {
                found = true;
                seconds += match c {
                    'h' => v * SECONDS_PER_HOUR,
                    'm' => v * SECONDS_PER_MINUTE,
                    _ => v,
                };
            }
            num.clear();
        } else if c == ' ' {
            num.clear(); // "16m 48s" 的组分隔空格：继续扫
        } else {
            break; // 其他字符（; 等）终止，与 Go 正则的最左匹配边界一致
        }
    }
    let _ = found;
    seconds
}

// ---- 结果与入口（estimate.go） ----

/// 为什么带 `Serialize`：这份估算要原样进历史档案（`gcode_history/*_meta.json`
/// 的 `detail.printTime`）。让 `src-tauri` 手写一遍字段映射等于把同一组键名写两处，
/// 而漏掉一个字段是静默的（JSON 里少个键没人会报错）。
/// **只加 `Serialize`，不加 `Deserialize`**：档案是只写的出口，
/// 读回来的一侧用 `serde_json::Value` 就够（历史档案不需要还原成计算结构）。
#[derive(Debug, Clone, Copy, Default, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryStat {
    pub seconds: f64,
    pub count: i64,
    pub percent: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Options {
    /// Go 的 ComputeDelta 需要路径域的 `_original` 兄弟文件检查，留给 CLI 层；
    /// 本模块恒不计算 delta（参考输出该项为 null）。
    pub compute_delta: bool,
    pub startup_overhead_seconds: f64,
}

/// `Serialize` 的理由同 [`CategoryStat`]。
///
/// `by_type` 的键是 [`classify`] 给的**中文类别名**（「外墙」「非运动等待(G4/M400)」…），
/// 落进 JSON 就是对象键，原样、不转义 —— 这不违反「中文步骤名只许在 UI 一处」那条：
/// 那条管的是 12 步的**步骤名**（`pipeline/progress.rs:6`），而这些类别名是
/// 打印时间估算自己的分类结果，本来就只在这一处产生。
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Result {
    pub total_seconds: f64,
    pub tool_movement_seconds: f64,
    pub prep_overhead_seconds: f64,
    pub startup_overhead_seconds: f64,
    pub bambu_anchor_seconds: Option<f64>,
    pub post_process_delta_seconds: Option<f64>,
    pub segments: i64,
    pub by_type: BTreeMap<String, CategoryStat>,
}

/// `Estimate`（内容版）：options.startup_overhead_seconds 即 CLI 的 240。
pub fn estimate(content: &str, opts: &Options) -> Result {
    let (limits, _) = detect_limits_from_gcode(content);

    let (mut blocks, overhead) = parse_gcode(content, &limits, false);
    planner_forward_pass(&mut blocks);
    planner_reverse_pass(&mut blocks);
    recalculate_trapezoids(&mut blocks);

    let mut total = 0.0;
    let mut by_type: BTreeMap<String, CategoryStat> = BTreeMap::new();
    for b in &blocks {
        let t = trapezoid_time(b.entry, b.cruise, b.exit, b.distance, b.accel);
        total += t;
        let cat = classify(b.kind, &b.type_tag);
        let d = by_type.entry(cat).or_default();
        d.seconds += t;
        d.count += 1;
    }

    let non_motion = overhead.g4_dwell + overhead.m400_wait;
    total += non_motion;
    if non_motion > 0.0 {
        let d = by_type
            .entry("非运动等待(G4/M400)".to_string())
            .or_default();
        d.seconds = non_motion;
        d.count = (overhead.g4_dwell > 0.0) as i64 + (overhead.m400_wait > 0.0) as i64;
    }

    let base = total;
    for d in by_type.values_mut() {
        d.percent = d.seconds / base * 100.0;
    }

    let (prep, anchor) = parse_header_meta(content);
    let mut total = total + prep;
    let delta: Option<f64> = None; // 内容域无兄弟文件（见 Options 文档）

    let startup = opts.startup_overhead_seconds;
    total += startup;

    Result {
        total_seconds: total,
        tool_movement_seconds: total - prep - startup,
        prep_overhead_seconds: prep,
        startup_overhead_seconds: startup,
        bambu_anchor_seconds: anchor,
        post_process_delta_seconds: delta,
        segments: blocks.len() as i64,
        by_type,
    }
}

#[cfg(test)]
mod serialize_tests {
    use super::*;

    /// 档案（`gcode_history/*_meta.json` 的 `detail.printTime`）咬着这 8 个键。
    ///
    /// 为什么要专门钉：`None` 在 serde 默认行为下落成 `null`（**不是缺键**），
    /// 而「缺键」与「值为 null」对读侧是两件事 —— 缺键会让人以为版本不对，
    /// null 才表达「这一项本来就算不出来」（`postProcessDeltaSeconds` 恒 null，见 [`Options`]）。
    #[test]
    fn the_archive_gets_all_eight_keys_in_camel_case() {
        let mut by_type = BTreeMap::new();
        by_type.insert(
            "外墙".to_string(),
            CategoryStat {
                seconds: 12.5,
                count: 3,
                percent: 25.0,
            },
        );
        let r = Result {
            total_seconds: 1474.89,
            tool_movement_seconds: 1200.0,
            prep_overhead_seconds: 34.89,
            startup_overhead_seconds: 240.0,
            bambu_anchor_seconds: None,
            post_process_delta_seconds: None,
            segments: 12345,
            by_type,
        };
        let v: serde_json::Value = serde_json::to_value(&r).expect("序列化");
        let obj = v.as_object().expect("必须是对象");
        let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec![
                "bambuAnchorSeconds",
                "byType",
                "postProcessDeltaSeconds",
                "prepOverheadSeconds",
                "segments",
                "startupOverheadSeconds",
                "toolMovementSeconds",
                "totalSeconds",
            ],
            "档案里的键集变了 —— 读侧与前端都咬着这 8 个"
        );
        assert!(
            obj["bambuAnchorSeconds"].is_null(),
            "算不出来要落 null，不许缺键"
        );
        // 中文类别名原样做对象键（不转义、不改名）
        assert_eq!(v["byType"]["外墙"]["count"], serde_json::json!(3));
        assert_eq!(v["byType"]["外墙"]["percent"], serde_json::json!(25.0));
    }
}
