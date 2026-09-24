// pass2 —— 第二遍扫描（对照 processor/pass2.go 逐行移植）。
//
// 消耗式：`second_pass(p1: Pass1Output, ...)` 按值吃 pass1 产物（12.1，无 Option）。
// 输出为内存行序列；`.part` + rename 的落盘属 engine write 步（design §3.2/§八），
// 行为等价、少一层文件往返 —— 该偏离写入收口报告。
//
// 依赖 disk::wipe（圆盘擦拭，Task 12 随 pass2 落地，design §3.2 的
// 「wipe.go 随 pass2 落地」决定）。
#![allow(
    clippy::collapsible_if,
    clippy::needless_range_loop,
    clippy::too_many_lines,
    clippy::if_same_then_else
)]

mod revitalization;

use std::collections::VecDeque;

use crate::diag::PostprocError;
use crate::gcode::{
    format_e, format_python_float, format_speed, format_speed_int, format_z,
    has_slicer_comment_prefix, match_slicer_comment, math_round,
};
use crate::ir::{Ir, num_strip};

use crate::postproc::disk::wipe::{
    CollisionLayer, DiskPosition, DiskWipeRequest, default_disk_wipe_options,
    generate_disk_wipe_segment,
};
use crate::postproc::offset::process_gcode_offset;
use crate::postproc::pass1::{LineSink, MM_PER_MINUTE, Pass1Output, ProcessStats};
use crate::postproc::tower;

pub(crate) const TOWER_SUGGESTED_LAYER_COEFF: f64 = 0.7;
pub(crate) const TOWER_MIN_LAYER_COEFF: f64 = 0.2;
pub(crate) const TOWER_HEIGHT_THRESHOLD: f64 = 0.4;
pub(crate) const TOWER_FIRST_LAYER_REF: f64 = 0.2;
pub(crate) const TOWER_RETRACT_DIFF: f64 = 0.31;
pub const TOWER_COLLISION_KEEP_LAYERS: i64 = 5;
pub const TOWER_COLLISION_MAX_KEEP_LAYERS: i64 = 20;
pub(crate) const TOWER_COLLISION_TARGET_Z_RANGE: f64 = 0.8;
pub(crate) const TOWER_HEIGHT_COMP_OFFSET: f64 = 2.0;

/// `towerCollisionKeepLayers`（pass2.go:39）：按层高自适应碰撞窗口容量。
pub(crate) fn tower_collision_keep_layers(typical_layer_height: f64) -> usize {
    if typical_layer_height <= 0.0 {
        return TOWER_COLLISION_KEEP_LAYERS as usize;
    }
    let n = (TOWER_COLLISION_TARGET_Z_RANGE / typical_layer_height).ceil() as i64;
    if n < TOWER_COLLISION_KEEP_LAYERS {
        TOWER_COLLISION_KEEP_LAYERS as usize
    } else if n > TOWER_COLLISION_MAX_KEEP_LAYERS {
        TOWER_COLLISION_MAX_KEEP_LAYERS as usize
    } else {
        n as usize
    }
}

/// `CalcSuggestedLH`（extrusion.go）：塔层高建议值（fast/custom/follow 三模式）。
#[allow(clippy::too_many_arguments)]
pub(crate) fn calc_suggested_lh(
    current_z: f64,
    last_z: f64,
    nozzle_diameter: f64,
    has_next_tj: bool,
    fast_tower_mode: &str,
    custom_layer_height: &str,
    next_glue_layer_distance: i64,
) -> f64 {
    let max_lh = (TOWER_SUGGESTED_LAYER_COEFF * nozzle_diameter * 1000.0).round() / 1000.0;
    let min_lh = (TOWER_MIN_LAYER_COEFF * nozzle_diameter * 1000.0).round() / 1000.0;
    let clamp_lh = |lh: f64| lh.clamp(min_lh, max_lh);

    match fast_tower_mode {
        "fast" => {
            if has_next_tj || next_glue_layer_distance <= 1 {
                clamp_lh(((current_z - last_z) * 1000.0).round() / 1000.0)
            } else {
                max_lh
            }
        }
        "custom" => match custom_layer_height {
            "min" => min_lh,
            "max" => max_lh,
            "auto" => {
                if has_next_tj {
                    clamp_lh(((current_z - last_z) * 1000.0).round() / 1000.0)
                } else {
                    max_lh
                }
            }
            _ => max_lh,
        },
        _ => clamp_lh(((current_z - last_z) * 1000.0).round() / 1000.0),
    }
}

/// `cachedLine`（pass2.go:60）：塔层模板行的预计算形态。
#[derive(Default, Clone)]
struct CachedLine {
    is_placeholder: bool,
    placeholder_key: &'static str,
    is_g1_with_e: bool,
    offset_applied: String,
    template_e: f64,
    template_line: String,
    is_g1_no_e: bool,
    offset_applied_ne: String,
    is_g1_f9600: bool,
    is_g1_f9600c: bool,
    is_g92_e0: bool,
    is_g1_e_static: bool,
    static_e_line: &'static str,
    is_travel_f30000: bool,
    offset_applied_t: String,
    raw_line: String,
}

/// `precomputeTowerLayer`（pass2.go:79）。
fn precompute_tower_layer(
    ir_data: &Ir,
    final_tower_height: f64,
) -> Result<Vec<CachedLine>, PostprocError> {
    let template_lines = tower::get_wiping_gcode_lines();
    let bbox = tower::calculate_tower_bbox(
        &ir_data.wiping.outer_structure,
        ir_data.sheath.base_expand,
        ir_data.rib.extra_length,
        ir_data.rib.width,
        final_tower_height,
        ir_data.machine.nozzle_diameter,
        ir_data.machine.first_layer_height,
    );
    let x_off = ir_data.wiping.wiper_x - bbox.min_x;
    let y_off = ir_data.wiping.wiper_y - bbox.min_y;
    let mut result = Vec::with_capacity(template_lines.len());
    for tpl_line in &template_lines {
        let mut cl = CachedLine::default();
        if tpl_line.contains("G1 F9600C") {
            cl.is_g1_f9600c = true;
        } else if tpl_line.contains("G1 F9600") {
            cl.is_g1_f9600 = true;
        } else if tpl_line.contains("TOWER_ZP_ST") {
            cl.raw_line = tpl_line.clone();
        } else if tpl_line.contains("NOZZLE_HEIGHT_ADJUST") {
            cl.is_placeholder = true;
            cl.placeholder_key = "NOZZLE_HEIGHT_ADJUST";
        } else if tpl_line.contains("EXTRUDER_REFILL") {
            cl.is_placeholder = true;
            cl.placeholder_key = "EXTRUDER_REFILL";
        } else if tpl_line.contains("EXTRUDER_RETRACT") {
            cl.is_placeholder = true;
            cl.placeholder_key = "EXTRUDER_RETRACT";
        } else if tpl_line.contains("G1 E-.21 F5400") {
            cl.is_g1_e_static = true;
            cl.static_e_line = "G1 E-.21 F5400";
        } else if tpl_line.contains("G1 E.3 F5400") {
            cl.is_g1_e_static = true;
            cl.static_e_line = "G1 E.3 F5400";
        } else if tpl_line.contains("G92 E0") {
            cl.is_g92_e0 = true;
        } else if tpl_line.contains("G1 ") && tpl_line.contains("F30000") {
            cl.is_travel_f30000 = true;
            cl.offset_applied_t = process_gcode_offset(
                tpl_line,
                x_off,
                y_off,
                0.0,
                crate::gcode::OffsetMode::Tower,
                ir_data,
            )?;
        } else if tpl_line.contains("G1 ") && tpl_line.contains('E') {
            cl.is_g1_with_e = true;
            cl.template_line = tpl_line.trim().to_string();
            if let Some(e_idx) = tpl_line.rfind('E') {
                let mut e_str = &tpl_line[e_idx + 1..];
                if let Some(space_idx) = e_str.find(' ') {
                    e_str = &e_str[..space_idx];
                }
                if let Ok(v) = e_str.parse::<f64>() {
                    cl.template_e = v;
                }
            }
            cl.offset_applied = process_gcode_offset(
                tpl_line,
                x_off,
                y_off,
                0.0,
                crate::gcode::OffsetMode::Tower,
                ir_data,
            )?;
        } else if tpl_line.contains("G1 ") {
            cl.is_g1_no_e = true;
            cl.offset_applied_ne = process_gcode_offset(
                tpl_line,
                x_off,
                y_off,
                0.0,
                crate::gcode::OffsetMode::Tower,
                ir_data,
            )?;
        } else {
            cl.raw_line = tpl_line.clone();
        }
        result.push(cl);
    }
    Ok(result)
}

/// `scaleCachedEValue`（pass2.go:148）。
fn scale_cached_e_value(line: &str, template_e: f64, ratio: f64) -> String {
    let Some(e_idx) = line.rfind('E') else {
        return line.to_string();
    };
    let new_e = math_round(template_e * ratio, 5);
    let e_str = crate::gcode::format_e_value(new_e);
    format!("{}{}", &line[..e_idx + 1], e_str)
}

/// 未来碰撞层（pass2.go:55）：预扫描收集的单层碰撞墙体。
struct FutureCollisionLayer {
    z_height: f64,
    wall_lines: Vec<String>,
}

/// pass2 产物：行序列 + 最终塔高（engine write 步消费）。
pub struct Pass2Output {
    pub lines: Vec<String>,
    pub tower_height: f64,
}

/// pass2 全部可变状态的载体（Go 用闭包共享，Rust 用结构体方法）。
struct Ctx<'a> {
    sink: LineSink,
    recent_lines: VecDeque<String>,
    ir_data: &'a mut Ir,
    toml_machine: &'a str,
    last_key: f64,
    tower_bbox: tower::TowerBBox,

    current_layer_height: f64,
    last_layer_height: f64,
    curr_max_tower_height: f64,
    last_max_tower_height: f64,
    last_tower_z: f64,
    tower_flag: bool,
    first_layer_tower_flag: bool,
    first_layer_flag: bool,
    next_tj: bool,
    remove_g3_flag: bool,
    allow_print_flag: bool,
    remove_wrap_detect_flag: bool,
    remove_timelapse_flag: bool,
    suggested_lh: f64,
    local_thickness: f64,
    adjust_support_flag: bool,
    switch_tower_type: i64,
    pen_change_flag: bool,
    more_extrude_flag: bool,
    corner_extra_flow_active: bool,
    append_support_flag: bool,
    add_collision_line_flag: bool,
    bottom_glue_inserted_this_layer: bool,
    skip_slicer_wipe: bool,
    first_layer_model_finished_flag: bool,
    skip_glue_block_mount: bool,
    skipping_mount_sequence: bool,
    end_gcode_started: bool,
    end_gcode_timelapse_block: bool,

    supports: Vec<String>,
    collision_walls: Vec<String>,
    collision_check: Vec<CollisionLayer>,
    disk_history: Vec<crate::postproc::disk::wipe::DiskHistoryEntry>,
    last_selected_disk: Option<DiskPosition>,
    pending_paint_lines: Vec<String>,
    pending_finish_lines: Vec<String>,
    cached_layer_lines: Vec<CachedLine>,
    future_collision_layers: Vec<FutureCollisionLayer>,
}

const MAX_RECENT_LINES: usize = 500;

impl<'a> Ctx<'a> {
    fn write_line(&mut self, line: &str) {
        self.sink.push_str(line);
        self.recent_lines.push_back(line.to_string());
        if self.recent_lines.len() > MAX_RECENT_LINES {
            self.recent_lines.pop_front();
        }
    }

    fn write_lines(&mut self, lines: &[String]) {
        for l in lines {
            self.write_line(l);
        }
    }

    fn write_line_offset(
        &mut self,
        gcommand: &str,
        x_offset: f64,
        y_offset: f64,
        z_offset: f64,
        mode: crate::gcode::OffsetMode,
    ) -> Result<(), PostprocError> {
        let result =
            process_gcode_offset(gcommand, x_offset, y_offset, z_offset, mode, self.ir_data)?;
        self.write_line(&result);
        Ok(())
    }

    fn tower_x_off(&self) -> f64 {
        self.ir_data.wiping.wiper_x - self.tower_bbox.min_x
    }
    fn tower_y_off(&self) -> f64 {
        self.ir_data.wiping.wiper_y - self.tower_bbox.min_y
    }

    /// `handlePrepareNextTower`（pass2.go:447）。
    fn handle_prepare_next_tower(&mut self) -> Result<(), PostprocError> {
        self.remove_g3_flag = true;
        self.more_extrude_flag = true;
        self.switch_tower_type = 1;

        if !self.collision_walls.is_empty() {
            self.collision_check
                .push(CollisionLayer::new(std::mem::take(
                    &mut self.collision_walls,
                )));
        }
        if self.collision_check.len()
            > tower_collision_keep_layers(self.ir_data.machine.typical_layer_height)
        {
            self.collision_check.remove(0);
        }

        let mut disk_req_supports_none_pending = false;
        if !self.pending_paint_lines.is_empty() {
            // pending lines 将在 tower wiping + E5.5 之后写入
            self.supports.clear();
        } else if !self.supports.is_empty() && !self.ir_data.tower.use_towers {
            disk_req_supports_none_pending = true;
        } else {
            self.supports.clear();
        }

        if disk_req_supports_none_pending {
            // 无 ;Pre-glue preparation 的情况（无换笔涂胶）：整体写入 SegmentLines。
            // 未来层（含"同层但尚未打印"的支撑墙体，>= 判定）放 collisionCheck 开头。
            let mut effective: Vec<CollisionLayer> = Vec::new();
            let mut future_layer_count = 0usize;
            for fl in &self.future_collision_layers {
                if fl.z_height >= self.current_layer_height
                    && fl.z_height <= self.current_layer_height + TOWER_COLLISION_TARGET_Z_RANGE
                {
                    effective.push(CollisionLayer::new(fl.wall_lines.clone()));
                    future_layer_count += 1;
                }
            }
            effective.extend(
                self.collision_check
                    .iter()
                    .map(|l| CollisionLayer::new(l.wall_lines.clone())),
            );
            let supports = std::mem::take(&mut self.supports);
            let req = DiskWipeRequest {
                supports: &supports,
                collision_check: &mut effective,
                disk_history: std::mem::take(&mut self.disk_history),
                last_selected_disk: self.last_selected_disk.as_ref(),
                current_layer_height: self.current_layer_height,
                last_layer_height: self.last_layer_height,
                ir: self.ir_data,
                machine_type: self.toml_machine,
                recent_lines: &self.recent_lines.iter().cloned().collect::<Vec<_>>(),
                future_layer_count,
            };
            let segment = generate_disk_wipe_segment(req, default_disk_wipe_options());
            if let Some(seg) = segment {
                self.write_lines(&seg.segment_lines.clone());
                if let Some(d) = seg.selected_disk {
                    self.last_selected_disk = Some(d);
                }
                if !seg.disk_history.is_empty() {
                    self.disk_history = seg.disk_history;
                }
            }
        }

        if self.ir_data.tower.use_towers {
            self.write_line(&format!(
                "G1 F{}",
                format_speed(self.ir_data.tower.travel_speed * MM_PER_MINUTE)
            ));
            self.write_line(&format!("G1 Z{}", format_z(self.curr_max_tower_height)));
            let variable_wipe_code = format!("G1 X15 Y2{}", crate::gcode::get_pseudo_random());
            let (x_off, y_off) = (self.tower_x_off(), self.tower_y_off());
            if self.ir_data.filament.filament_type == "PLA" {
                self.write_line_offset(
                    &variable_wipe_code,
                    x_off,
                    y_off,
                    0.0,
                    crate::gcode::OffsetMode::Normal,
                )?;
                self.write_line_offset(
                    &format!(
                        "G1 X{} Y{}",
                        crate::gcode::format_float(tower::WIPE_CENTER_X),
                        crate::gcode::format_float(tower::WIPE_CENTER_Y)
                    ),
                    x_off,
                    y_off,
                    0.0,
                    crate::gcode::OffsetMode::Normal,
                )?;
                self.write_line_offset(
                    &variable_wipe_code,
                    x_off,
                    y_off,
                    0.0,
                    crate::gcode::OffsetMode::Normal,
                )?;
                self.write_line_offset(
                    &format!(
                        "G1 X{} Y{}",
                        crate::gcode::format_float(tower::WIPE_CENTER_X),
                        crate::gcode::format_float(tower::WIPE_CENTER_Y)
                    ),
                    x_off,
                    y_off,
                    0.0,
                    crate::gcode::OffsetMode::Normal,
                )?;
            }
            self.write_line_offset(
                &variable_wipe_code,
                x_off,
                y_off,
                0.0,
                crate::gcode::OffsetMode::Normal,
            )?;
            self.write_line_offset(
                &format!(
                    "G1 X{} Y{}",
                    crate::gcode::format_float(tower::WIPE_CENTER_X),
                    crate::gcode::format_float(tower::WIPE_CENTER_Y)
                ),
                x_off,
                y_off,
                0.0,
                crate::gcode::OffsetMode::Normal,
            )?;
            self.write_line_offset(
                &variable_wipe_code,
                x_off,
                y_off,
                0.0,
                crate::gcode::OffsetMode::Normal,
            )?;
            self.write_line_offset(
                &format!(
                    "G1 X{} Y{}",
                    crate::gcode::format_float(tower::WIPE_ALT_X),
                    crate::gcode::format_float(tower::WIPE_ALT_Y)
                ),
                x_off,
                y_off,
                0.0,
                crate::gcode::OffsetMode::Normal,
            )?;
            self.write_line_offset(
                &format!("G1 X20 Y1{}", crate::gcode::get_pseudo_random()),
                x_off,
                y_off,
                0.0,
                crate::gcode::OffsetMode::Normal,
            )?;
            // UserDryTime 等待移到 Prime 之前
            if self.ir_data.wiping.user_dry_time != 0 {
                self.write_line(";User Dry Time Activated");
                self.write_line(&format!("G4 P{}", self.ir_data.wiping.user_dry_time * 1000));
            }
            if self.pen_change_flag {
                if !crate::postproc::pass1::custom_unmount_gcode_has_extrude(self.ir_data) {
                    self.write_line(&format!(
                        "G1 E{} F1500",
                        format_e(self.ir_data.gcode.mkp_extrude)
                    ));
                }
                self.pen_change_flag = false;
            }
            self.write_line(&format!(
                "G1 F{}",
                format_speed(self.ir_data.machine.travel_speed * MM_PER_MINUTE)
            ));
        }

        // Phase 4：tower wiping + 恢复挤出后写入圆盘绘制段与退出段
        if !self.pending_paint_lines.is_empty() {
            let paint = std::mem::take(&mut self.pending_paint_lines);
            self.write_lines(&paint);
        }
        if !self.pending_finish_lines.is_empty() {
            let finish = std::mem::take(&mut self.pending_finish_lines);
            self.write_lines(&finish);
            // 圆盘段用了 E 值层高，结束后恢复实际层高注释
            self.write_line(&format!(
                "; LAYER_HEIGHT: {}",
                format_python_float(self.local_thickness)
            ));
        }

        Ok(())
    }
}

/// `SecondPass`（pass2.go:165）：第二遍扫描。
///
/// 消耗式吃 `Pass1Output`；`final_tower_height` = pass1 stats.MaxZHeight。
/// `cancel` 协作取消：主循环顶部时间基检查（Task 17.2，对照
/// pass2.go:624-640），未取消路径零行为变化。
/// `cancel_check_interval` 由调用方注入：产品默认 100ms（engine 的
/// `DEFAULT_CANCEL_CHECK_INTERVAL`），判据注入极小值以摆脱墙钟依赖。
#[allow(clippy::too_many_arguments)]
pub fn second_pass(
    p1: Pass1Output,
    ir_data: &mut Ir,
    final_tower_height: f64,
    toml_machine: &str,
    progress: &mut dyn FnMut(f64, String),
    pass2_stats: &mut ProcessStats,
    cancel: &crate::diag::CancelToken,
    cancel_check_interval: std::time::Duration,
) -> Result<Pass2Output, PostprocError> {
    let Pass1Output {
        lines: pass1_lines,
        layer_index,
        ..
    } = p1;
    let mut cancel_checker = crate::postproc::cancel::CancelChecker::with_interval(
        std::time::Instant::now(),
        cancel_check_interval,
    );

    if toml_machine.is_empty() || toml_machine == "UNKNOWN" {
        tracing::warn!(
            machine = toml_machine,
            "SecondPass toml 机型缺失或未知，机型相关业务决策将被跳过"
        );
    }

    let tower_bbox = tower::calculate_tower_bbox(
        &ir_data.wiping.outer_structure,
        ir_data.sheath.base_expand,
        ir_data.rib.extra_length,
        ir_data.rib.width,
        final_tower_height,
        ir_data.machine.nozzle_diameter,
        ir_data.machine.first_layer_height,
    );

    // 预扫描：涂胶事件标记 + 按层收集未来碰撞墙体
    let mut has_glue_event = false;
    let mut future_collision_layers: Vec<FutureCollisionLayer> = Vec::new();
    {
        let mut cur_z = 0.0f64;
        let mut cur_walls: Vec<String> = Vec::new();
        let mut add_collision = false;
        let mut layer_started = false;
        for text in &pass1_lines {
            if text.contains(crate::gcode::MARKER_RISING_NOZZLE) {
                has_glue_event = true;
            }
            let trimmed = text.trim();
            if match_slicer_comment(trimmed, "Z_HEIGHT: ") {
                if layer_started && !cur_walls.is_empty() {
                    future_collision_layers.push(FutureCollisionLayer {
                        z_height: cur_z,
                        wall_lines: std::mem::take(&mut cur_walls),
                    });
                }
                let nums = num_strip(trimmed);
                if let Some(&n) = nums.first() {
                    cur_z = n;
                }
                cur_walls.clear();
                add_collision = false;
                layer_started = true;
                continue;
            }
            let is_collision_feature = has_slicer_comment_prefix(trimmed, "FEATURE: Outer wall")
                || has_slicer_comment_prefix(trimmed, "FEATURE: Inner wall")
                || has_slicer_comment_prefix(trimmed, "FEATURE: Top surface")
                || has_slicer_comment_prefix(trimmed, "FEATURE: Internal infill")
                || has_slicer_comment_prefix(trimmed, "FEATURE: Bridge")
                || has_slicer_comment_prefix(trimmed, "FEATURE: Support")
                || has_slicer_comment_prefix(trimmed, "FEATURE: Ironing");
            if is_collision_feature {
                add_collision = true;
            }
            if add_collision && trimmed.starts_with("G1 X") && trimmed.contains('E') {
                cur_walls.push(trimmed.to_string());
            }
            if has_slicer_comment_prefix(trimmed, "FEATURE:") && !is_collision_feature {
                add_collision = false;
            }
        }
        if layer_started && !cur_walls.is_empty() {
            future_collision_layers.push(FutureCollisionLayer {
                z_height: cur_z,
                wall_lines: cur_walls,
            });
        }
    }

    let cached_layer_lines = precompute_tower_layer(ir_data, final_tower_height)?;

    let mut last_key = 0.0f64;
    for k in layer_index.keys() {
        if k.0 > last_key {
            last_key = k.0;
        }
    }

    let mut ctx = Ctx {
        sink: LineSink::default(),
        recent_lines: VecDeque::new(),
        ir_data,
        toml_machine,
        last_key,
        tower_bbox,
        current_layer_height: 0.0,
        last_layer_height: 0.0,
        curr_max_tower_height: 0.0, // 占位，下方按 Go 初值设置
        last_max_tower_height: 0.0,
        last_tower_z: 0.0,
        tower_flag: false,
        first_layer_tower_flag: true,
        first_layer_flag: true,
        next_tj: false,
        remove_g3_flag: false,
        allow_print_flag: true,
        remove_wrap_detect_flag: false,
        remove_timelapse_flag: false,
        suggested_lh: 0.0,
        local_thickness: 0.0,
        adjust_support_flag: false,
        switch_tower_type: 0,
        pen_change_flag: false,
        more_extrude_flag: false,
        corner_extra_flow_active: false,
        append_support_flag: false,
        add_collision_line_flag: false,
        bottom_glue_inserted_this_layer: false,
        skip_slicer_wipe: false,
        first_layer_model_finished_flag: true,
        skip_glue_block_mount: false,
        skipping_mount_sequence: false,
        end_gcode_started: false,
        end_gcode_timelapse_block: false,
        supports: Vec::new(),
        collision_walls: Vec::new(),
        collision_check: Vec::new(),
        disk_history: Vec::new(),
        last_selected_disk: None,
        pending_paint_lines: Vec::new(),
        pending_finish_lines: Vec::new(),
        cached_layer_lines,
        future_collision_layers,
    };
    ctx.curr_max_tower_height = ctx.ir_data.machine.first_layer_height;
    ctx.suggested_lh = TOWER_SUGGESTED_LAYER_COEFF * ctx.ir_data.machine.nozzle_diameter;

    // 读游标 + 预读缓冲（Go 的 scanner + peekBuffer）
    let total = pass1_lines.len();
    let mut pos: usize = 0;
    let mut peek_buffer: VecDeque<String> = VecDeque::new();

    let mut last_progress: i64 = -1;
    let mut support_extrusion_count = 0i64;
    let mut line_num: i64 = 0;

    let read_next = |pos: &mut usize, peek_buffer: &mut VecDeque<String>| -> Option<String> {
        if let Some(l) = peek_buffer.pop_front() {
            return Some(l);
        }
        if *pos < pass1_lines.len() {
            let l = pass1_lines[*pos].clone();
            *pos += 1;
            Some(l)
        } else {
            None
        }
    };

    let peek_lines = |pos: &mut usize, peek_buffer: &mut VecDeque<String>, n: usize| {
        while peek_buffer.len() < n && *pos < pass1_lines.len() {
            peek_buffer.push_back(pass1_lines[*pos].clone());
            *pos += 1;
        }
        peek_buffer.iter().take(n).cloned().collect::<Vec<_>>()
    };

    while let Some(line) = read_next(&mut pos, &mut peek_buffer) {
        // 协作取消（Task 17.2）：循环顶部（pass2.go:624-640 同位——所有含
        // continue 的分支都会经过这里），100ms 时间基节流
        cancel_checker.check(cancel, std::time::Instant::now())?;
        let mut line = line;
        let trimmed = line.trim().to_string();
        line_num += 1;

        if line_num % 500 == 0 {
            let mut p = pos as f64 / total as f64 * 100.0;
            if p > 99.0 {
                p = 99.0;
            }
            if p as i64 != last_progress {
                progress(p, format!("第二遍扫描: {:.0}%", p));
                last_progress = p as i64;
            }
        }

        // 跳过涂胶块 mount 序列（首笔活化已下笔）
        if ctx.skipping_mount_sequence {
            // MKP 标记穿透：这一段整体被丢弃，但标记行必须活着，否则
            // swap#1 的 BEGIN/END 一起消失、事件只剩 paint→swap（违反 EV_SHAPE）。
            // 代价是这一次 swap 成为空区间（观看端 SWAP_HAS_MOTION warn）——
            // 取笔动作确实发生在首笔活化里，不在这个块里。
            if crate::postproc::marks::is_mark_line(&trimmed) {
                ctx.write_line(&line);
            }
            if trimmed.contains(";Toolhead Mounted") {
                ctx.skipping_mount_sequence = false;
                ctx.skip_glue_block_mount = false;
            }
            continue;
        }
        if ctx.skip_glue_block_mount && trimmed.contains(";Pre-glue preparation") {
            ctx.skipping_mount_sequence = true;
            continue;
        }

        // Phase 4：;Pre-glue preparation 之前写阶梯/连接线，绘制/退出段延后
        if trimmed.contains(crate::gcode::MARKER_PRE_GLUE)
            && !ctx.supports.is_empty()
            && !ctx.ir_data.tower.use_towers
        {
            let mut effective: Vec<CollisionLayer> = Vec::new();
            let mut future_layer_count = 0usize;
            for fl in &ctx.future_collision_layers {
                if fl.z_height >= ctx.current_layer_height
                    && fl.z_height <= ctx.current_layer_height + TOWER_COLLISION_TARGET_Z_RANGE
                {
                    effective.push(CollisionLayer::new(fl.wall_lines.clone()));
                    future_layer_count += 1;
                }
            }
            if !ctx.collision_walls.is_empty() {
                effective.push(CollisionLayer::new(ctx.collision_walls.clone()));
            }
            effective.extend(
                ctx.collision_check
                    .iter()
                    .map(|l| CollisionLayer::new(l.wall_lines.clone())),
            );
            let supports = std::mem::take(&mut ctx.supports);
            let recent: Vec<String> = ctx.recent_lines.iter().cloned().collect();
            let req = DiskWipeRequest {
                supports: &supports,
                collision_check: &mut effective,
                disk_history: std::mem::take(&mut ctx.disk_history),
                last_selected_disk: ctx.last_selected_disk.as_ref(),
                current_layer_height: ctx.current_layer_height,
                last_layer_height: ctx.last_layer_height,
                ir: ctx.ir_data,
                machine_type: ctx.toml_machine,
                recent_lines: &recent,
                future_layer_count,
            };
            let segment = generate_disk_wipe_segment(req, default_disk_wipe_options());
            if let Some(seg) = segment {
                ctx.write_lines(&seg.connection_lines.clone());
                ctx.pending_paint_lines = seg.paint_lines.clone();
                ctx.pending_finish_lines = seg.finish_lines.clone();
                if let Some(d) = seg.selected_disk {
                    ctx.last_selected_disk = Some(d);
                }
                if !seg.disk_history.is_empty() {
                    ctx.disk_history = seg.disk_history;
                }
            }
        }

        if match_slicer_comment(&trimmed, "LAYER_HEIGHT:") {
            let nums = num_strip(&trimmed);
            if let Some(&n) = nums.first() {
                ctx.local_thickness = n;
            }
            ctx.write_line(&format!(
                "; Current Layer Thickness:{}",
                format_python_float(ctx.local_thickness)
            ));
        }

        if ctx.current_layer_height > 0.3 {
            if has_slicer_comment_prefix(&trimmed, "FEATURE: Support body") {
                ctx.adjust_support_flag = true;
            }
            if has_slicer_comment_prefix(&trimmed, "FEATURE: ")
                && !has_slicer_comment_prefix(&trimmed, "FEATURE: Support body")
            {
                ctx.adjust_support_flag = false;
            }
        }

        if ctx.adjust_support_flag && trimmed.contains("G1 X") && trimmed.contains('E') {
            if let Some(e_idx) = trimmed.find('E') {
                if e_idx + 1 < trimmed.len() && trimmed.as_bytes()[e_idx + 1] != b'-' {
                    let e_value_str = &trimmed[e_idx + 1..];
                    let mut e_value = 0.0f64;
                    if !e_value_str.is_empty() && e_value_str.as_bytes()[0] == b'.' {
                        let nums = num_strip(&format!("0{e_value_str}"));
                        if let Some(&n) = nums.first() {
                            e_value = n;
                        }
                    } else if !e_value_str.is_empty() {
                        let nums = num_strip(e_value_str);
                        if let Some(&n) = nums.first() {
                            e_value = n;
                        }
                    }
                    if e_value > 0.0 {
                        let new_e_value = math_round(
                            e_value * ctx.ir_data.wiping.support_extrusion_multiplier,
                            5,
                        );
                        let e_str = crate::gcode::format_e_value(new_e_value);
                        line = format!("{}{};Adjust as support", &trimmed[..e_idx + 1], e_str);
                        support_extrusion_count += 1;
                    }
                }
            }
        }

        if crate::gcode::is_support_related_feature(&trimmed)
            || has_slicer_comment_prefix(&trimmed, "FEATURE: Support")
        {
            ctx.append_support_flag = true;
        }
        if (has_slicer_comment_prefix(&trimmed, "FEATURE: ")
            && !has_slicer_comment_prefix(&trimmed, "FEATURE: Support"))
            || has_slicer_comment_prefix(&trimmed, "FEATURE: Support ironing")
        {
            ctx.append_support_flag = false;
        }
        if ctx.append_support_flag && trimmed.starts_with("G1 X") {
            ctx.supports.push(trimmed.clone());
        }

        if match_slicer_comment(&trimmed, "Z_HEIGHT: ") {
            ctx.last_layer_height = ctx.current_layer_height;
            let nums = num_strip(&trimmed);
            if let Some(&n) = nums.first() {
                ctx.current_layer_height = n;
            }
            ctx.bottom_glue_inserted_this_layer = false;
            ctx.skip_slicer_wipe = false;

            // Go 此处对 layerHeightIndex 命中有一个空语句块（pass2.go:810），无行为。

            if ctx.ir_data.tower.use_towers
                && ctx.first_layer_flag
                && ctx.current_layer_height > 0.01
            {
                ctx.first_layer_flag = false;
            }

            let peek = peek_lines(&mut pos, &mut peek_buffer, 20);
            for pk in &peek {
                if pk.contains(";Rising Nozzle a little") {
                    ctx.next_tj = true;
                    ctx.switch_tower_type = 1;
                    ctx.pen_change_flag = true;
                    break;
                }
            }

            // 距下一涂胶层的层数（fast 模式层高预判）
            let mut next_glue_layer_distance: i64 = 9999;
            let peek_far = peek_lines(&mut pos, &mut peek_buffer, 200);
            let mut z_height_count: i64 = 0;
            for pk in &peek_far {
                if pk.contains(";Rising Nozzle a little") {
                    next_glue_layer_distance = z_height_count;
                    break;
                }
                if match_slicer_comment(pk, "Z_HEIGHT: ") {
                    z_height_count += 1;
                }
            }

            ctx.suggested_lh = calc_suggested_lh(
                ctx.current_layer_height,
                ctx.last_layer_height,
                ctx.ir_data.machine.nozzle_diameter,
                ctx.next_tj,
                &ctx.ir_data.tower.fast_mode,
                &ctx.ir_data.tower.custom_layer_height,
                next_glue_layer_distance,
            );

            ctx.last_max_tower_height = ctx.curr_max_tower_height;
            if ctx.current_layer_height < ctx.last_key + TOWER_HEIGHT_THRESHOLD {
                if ctx.curr_max_tower_height + ctx.suggested_lh < ctx.current_layer_height
                    || ctx.next_tj
                {
                    ctx.next_tj = false;
                    ctx.curr_max_tower_height =
                        math_round(ctx.curr_max_tower_height + ctx.suggested_lh, 3);
                }
                if ctx.last_max_tower_height < ctx.curr_max_tower_height {
                    ctx.tower_flag = true;
                }
            }

            // 底面涂胶块插入（pass1 预生成于 State.BottomGlueEntries）。
            // 先按值取出命中条目再写（避免与 ctx 的可变借用冲突；Go 语义不变）。
            if !ctx.ir_data.state.bottom_glue_entries.is_empty() {
                let mut hit: Option<(usize, Vec<String>)> = None;
                for (i, entry) in ctx.ir_data.state.bottom_glue_entries.iter().enumerate() {
                    if entry.target_z == ctx.current_layer_height {
                        hit = Some((i, entry.lines.clone()));
                        break;
                    }
                }
                if let Some((idx, lines)) = hit {
                    for glue_line in lines {
                        if glue_line.is_empty() {
                            continue;
                        }
                        if glue_line.contains(crate::gcode::MARKER_RISING_NOZZLE) {
                            ctx.pen_change_flag = true;
                            ctx.write_line(&glue_line);
                        } else if glue_line.contains(crate::gcode::MARKER_PREPARE_NEXT_TOWER) {
                            ctx.handle_prepare_next_tower()?;
                        } else {
                            ctx.write_line(&glue_line);
                        }
                    }
                    ctx.ir_data.state.bottom_glue_entries[idx].target_z = -1.0; // 标记已消费
                    ctx.bottom_glue_inserted_this_layer = true;
                }
            }
        }

        let was_first_layer_tower_flag = ctx.first_layer_tower_flag;

        if match_slicer_comment(&trimmed, "CHANGE_LAYER")
            && ctx.first_layer_tower_flag
            && ctx.ir_data.tower.use_towers
            && has_glue_event
        {
            ctx.first_layer_tower_flag = false;
            ctx.ir_data.tower.extrude_ratio = math_round(
                ctx.ir_data.machine.first_layer_height / TOWER_FIRST_LAYER_REF,
                3,
            );
            let sheath_lines = tower::generate_sheath_gcode(
                0,
                ctx.ir_data.machine.first_layer_height,
                ctx.ir_data.machine.first_layer_height,
                ctx.ir_data.machine.first_layer_height,
                ctx.ir_data.wiping.wiper_x,
                ctx.ir_data.wiping.wiper_y,
                ctx.ir_data.machine.travel_speed * MM_PER_MINUTE,
                ctx.ir_data.tower.sheath_speed * MM_PER_MINUTE,
                ctx.ir_data.machine.retract_length,
                ctx.ir_data.machine.nozzle_diameter,
                ctx.ir_data.wiping.outer_structure.as_str(),
                ctx.ir_data.sheath.base_expand,
                ctx.ir_data.sheath.enable_height,
                ctx.ir_data.sheath.converge_layers,
                ctx.ir_data.sheath.wall_width,
                ctx.last_key,
                ctx.ir_data.tower.first_layer_flow,
                ctx.current_layer_height,
                ctx.ir_data.tower.safe_z_offset,
                ctx.tower_bbox.min_x,
                ctx.tower_bbox.min_y,
            );
            if let Some(lines) = sheath_lines.filter(|l| !l.is_empty()) {
                ctx.write_lines(&lines);
            }
            if ctx.ir_data.wiping.outer_structure == "rib" {
                let rib_speed = if ctx.ir_data.tower.rib_speed_mode == "custom" {
                    ctx.ir_data.tower.rib_speed_value * MM_PER_MINUTE
                } else {
                    ctx.ir_data.wiping.tower_print_speed * MM_PER_MINUTE
                };
                let rib_lines = tower::generate_rib_gcode(
                    0,
                    ctx.ir_data.machine.first_layer_height,
                    ctx.ir_data.machine.first_layer_height,
                    ctx.ir_data.machine.first_layer_height,
                    final_tower_height,
                    ctx.ir_data.wiping.wiper_x,
                    ctx.ir_data.wiping.wiper_y,
                    ctx.ir_data.tower.travel_speed * MM_PER_MINUTE,
                    rib_speed,
                    ctx.ir_data.machine.retract_length,
                    ctx.ir_data.machine.nozzle_diameter,
                    ctx.ir_data.rib.extra_length,
                    ctx.ir_data.rib.width,
                    ctx.ir_data.rib.fillet_wall,
                    ctx.ir_data.tower.first_layer_flow,
                    ctx.ir_data.rib.bottom_fill_style.as_str(),
                    (ctx.ir_data.machine.min_x + ctx.ir_data.machine.max_x) / 2.0,
                    (ctx.ir_data.machine.min_y + ctx.ir_data.machine.max_y) / 2.0,
                    ctx.current_layer_height,
                    ctx.ir_data.tower.safe_z_offset,
                    ctx.tower_bbox.min_x,
                    ctx.tower_bbox.min_y,
                );
                if let Some(lines) = rib_lines.filter(|l| !l.is_empty()) {
                    ctx.write_lines(&lines);
                }
            }
            ctx.ir_data.tower.layer_count += 1;
            ctx.last_tower_z = ctx.curr_max_tower_height;
        }

        // 第二个 CHANGE_LAYER：首层模型打完 → 首笔活化（校准模式禁用）
        if !was_first_layer_tower_flag
            && ctx.first_layer_model_finished_flag
            && match_slicer_comment(&trimmed, "CHANGE_LAYER")
            && ctx.ir_data.tower.use_towers
            && ctx.ir_data.toolhead.first_pen_revitalization_flag
            && has_glue_event
            && !ctx.ir_data.safety.is_calibration_mode
        {
            ctx.first_layer_model_finished_flag = false;
            revitalization::emit_first_pen_revitalization(
                &mut ctx.sink,
                ctx.ir_data,
                ctx.ir_data.machine.first_layer_height,
                final_tower_height,
                Some(pass2_stats),
                false,
            )?;
        }

        if trimmed.contains("; update layer progress")
            && ctx.ir_data.tower.use_towers
            && ctx.tower_flag
            && !ctx.first_layer_tower_flag
            && ctx.current_layer_height != ctx.ir_data.machine.first_layer_height
        {
            ctx.tower_flag = false;
            ctx.write_line(&format!(
                "G1 F{}",
                format_speed(ctx.ir_data.machine.travel_speed * MM_PER_MINUTE)
            ));
            ctx.ir_data.tower.extrude_ratio = math_round(ctx.suggested_lh / 0.2, 3);
            let ratio = ctx.ir_data.tower.extrude_ratio;
            let safe_z = ctx.current_layer_height.max(ctx.curr_max_tower_height)
                + ctx.ir_data.tower.safe_z_offset;
            let processed = process_gcode_offset(
                "G1 X20 Y20",
                ctx.tower_x_off(),
                ctx.tower_y_off(),
                0.0,
                crate::gcode::OffsetMode::Tower,
                ctx.ir_data,
            )?;
            ctx.write_line(&format!("{} Z{}", processed, format_z(safe_z)));
            ctx.write_line("; FEATURE: Inner wall");
            ctx.write_line("; LINE_WIDTH: 0.42");
            ctx.write_line(&format!(
                ";Extruding Ratio: {}",
                format_python_float(ctx.ir_data.tower.extrude_ratio)
            ));
            ctx.write_line(&format!(
                "; LAYER_HEIGHT: {}",
                format_python_float(ctx.suggested_lh)
            ));
            for cl in &ctx.cached_layer_lines.clone() {
                if cl.is_g1_f9600 {
                    let need_speed_limit = match ctx.ir_data.tower.fast_mode.as_str() {
                        "fast" => ctx.switch_tower_type == 1,
                        "follow" => ctx.switch_tower_type <= 3,
                        "custom" => ctx.switch_tower_type == 1,
                        _ => false,
                    };
                    if need_speed_limit {
                        ctx.write_line(&format!(
                            "G1 F{}",
                            format_speed_int(
                                ctx.ir_data
                                    .wiping
                                    .tower_print_speed
                                    .min(ctx.ir_data.tower.glue_layer_speed)
                                    * MM_PER_MINUTE,
                            )
                        ));
                    } else {
                        ctx.write_line(&format!(
                            "G1 F{}",
                            format_speed(ctx.ir_data.wiping.tower_print_speed * MM_PER_MINUTE)
                        ));
                    }
                } else if cl.is_g1_f9600c {
                    let mut corner_speed =
                        ctx.ir_data.wiping.tower_print_speed * tower::TOWER_CORNER_SPEED_RATIO;
                    let need_speed_limit = match ctx.ir_data.tower.fast_mode.as_str() {
                        "fast" => ctx.switch_tower_type == 1,
                        "follow" => ctx.switch_tower_type <= 3,
                        "custom" => ctx.switch_tower_type == 1,
                        _ => false,
                    };
                    if need_speed_limit {
                        corner_speed = corner_speed.min(ctx.ir_data.tower.glue_layer_speed);
                    }
                    ctx.write_line(&format!(
                        "G1 F{}",
                        format_speed_int(corner_speed * MM_PER_MINUTE)
                    ));
                    ctx.corner_extra_flow_active = true;
                } else if !cl.raw_line.is_empty() {
                    ctx.write_line(&cl.raw_line);
                } else if cl.is_placeholder {
                    match cl.placeholder_key {
                        "NOZZLE_HEIGHT_ADJUST" => {
                            ctx.write_line(&format!(
                                "G1 Z{};Tower Z",
                                format_z(ctx.curr_max_tower_height)
                            ));
                        }
                        "EXTRUDER_REFILL" => {
                            ctx.write_line("G92 E0");
                            ctx.write_line(&format!(
                                "G1 E{}",
                                format_e(ctx.ir_data.machine.retract_length)
                            ));
                            ctx.write_line("G92 E0");
                        }
                        "EXTRUDER_RETRACT" => {
                            ctx.write_line("G92 E0");
                            ctx.write_line(&format!(
                                "G1 E-{}",
                                format_e(math_round(
                                    (ctx.ir_data.machine.retract_length - TOWER_RETRACT_DIFF).abs(),
                                    3,
                                ))
                            ));
                            ctx.write_line("G92 E0");
                        }
                        _ => {}
                    }
                } else if cl.is_g1_e_static {
                    ctx.write_line(cl.static_e_line);
                } else if cl.is_g92_e0 {
                    ctx.write_line("G92 E0");
                } else if cl.is_g1_with_e {
                    if ctx.more_extrude_flag && ctx.current_layer_height >= TOWER_HEIGHT_THRESHOLD {
                        if cl.template_line.starts_with("G1 X20 Y20 E") {
                            let processed = process_gcode_offset(
                                &format!("G1 X20 Y20 E.34 {}", crate::gcode::MARKER_MORE_EXTRUSION),
                                ctx.tower_x_off(),
                                ctx.tower_y_off(),
                                0.0,
                                crate::gcode::OffsetMode::Tower,
                                ctx.ir_data,
                            )?;
                            ctx.write_line(&processed);
                            ctx.corner_extra_flow_active = false;
                        } else if cl.template_line.starts_with("G1 X29.81 Y20 E") {
                            let processed = process_gcode_offset(
                                &format!(
                                    "G1 X29.81 Y20 E.34 {}",
                                    crate::gcode::MARKER_MORE_EXTRUSION
                                ),
                                ctx.tower_x_off(),
                                ctx.tower_y_off(),
                                0.0,
                                crate::gcode::OffsetMode::Tower,
                                ctx.ir_data,
                            )?;
                            ctx.write_line(&processed);
                            ctx.corner_extra_flow_active = false;
                        } else if cl.template_line.starts_with("G1 X10.19 Y20 E") {
                            let processed = process_gcode_offset(
                                &format!(
                                    "G1 X10.19 Y20 E.34 {}",
                                    crate::gcode::MARKER_MORE_EXTRUSION
                                ),
                                ctx.tower_x_off(),
                                ctx.tower_y_off(),
                                0.0,
                                crate::gcode::OffsetMode::Tower,
                                ctx.ir_data,
                            )?;
                            ctx.write_line(&processed);
                            ctx.corner_extra_flow_active = false;
                        } else if ctx.corner_extra_flow_active {
                            let scaled_e = cl.template_e * (1.0 + tower::TOWER_CORNER_EXTRA_FLOW);
                            ctx.write_line(&scale_cached_e_value(
                                &cl.offset_applied,
                                scaled_e,
                                ratio,
                            ));
                            ctx.corner_extra_flow_active = false;
                        } else {
                            ctx.write_line(&scale_cached_e_value(
                                &cl.offset_applied,
                                cl.template_e,
                                ratio,
                            ));
                        }
                    } else if ctx.corner_extra_flow_active {
                        let scaled_e = cl.template_e * (1.0 + tower::TOWER_CORNER_EXTRA_FLOW);
                        ctx.write_line(&scale_cached_e_value(&cl.offset_applied, scaled_e, ratio));
                        ctx.corner_extra_flow_active = false;
                    } else {
                        ctx.write_line(&scale_cached_e_value(
                            &cl.offset_applied,
                            cl.template_e,
                            ratio,
                        ));
                    }
                } else if cl.is_g1_no_e {
                    ctx.write_line(&cl.offset_applied_ne);
                } else if cl.is_travel_f30000 {
                    let replaced = cl.offset_applied_t.replacen(
                        "F30000",
                        &format!(
                            "F{}",
                            (ctx.ir_data.tower.travel_speed * MM_PER_MINUTE) as i64
                        ),
                        1,
                    );
                    ctx.write_line(&replaced);
                }
            }
            if ctx.switch_tower_type <= 3 {
                ctx.switch_tower_type += 1;
            }
            if ctx.more_extrude_flag {
                ctx.more_extrude_flag = false;
            }
            ctx.write_line(&format!(
                "G1 F{}",
                format_speed(ctx.ir_data.machine.travel_speed * MM_PER_MINUTE)
            ));
            if ctx.ir_data.tower.use_towers {
                let mut should_gen_sheath = true;
                let should_gen_rib = ctx.ir_data.wiping.outer_structure == "rib";
                match ctx.ir_data.tower.fast_mode.as_str() {
                    "fast" => should_gen_sheath = ctx.switch_tower_type == 1,
                    "follow" => should_gen_sheath = true,
                    "custom" => match ctx.ir_data.tower.wipe_mode.as_str() {
                        "on" => should_gen_sheath = true,
                        "off" => should_gen_sheath = false,
                        "auto" => should_gen_sheath = ctx.switch_tower_type == 1,
                        _ => should_gen_sheath = ctx.switch_tower_type == 1,
                    },
                    _ => {}
                }
                if should_gen_sheath {
                    let sheath_lines = tower::generate_sheath_gcode(
                        ctx.ir_data.tower.layer_count,
                        ctx.curr_max_tower_height,
                        ctx.suggested_lh,
                        ctx.ir_data.machine.first_layer_height,
                        ctx.ir_data.wiping.wiper_x,
                        ctx.ir_data.wiping.wiper_y,
                        ctx.ir_data.machine.travel_speed * MM_PER_MINUTE,
                        ctx.ir_data.tower.sheath_speed * MM_PER_MINUTE,
                        ctx.ir_data.machine.retract_length,
                        ctx.ir_data.machine.nozzle_diameter,
                        ctx.ir_data.wiping.outer_structure.as_str(),
                        ctx.ir_data.sheath.base_expand,
                        ctx.ir_data.sheath.enable_height,
                        ctx.ir_data.sheath.converge_layers,
                        ctx.ir_data.sheath.wall_width,
                        final_tower_height,
                        1.0,
                        ctx.current_layer_height,
                        ctx.ir_data.tower.safe_z_offset,
                        ctx.tower_bbox.min_x,
                        ctx.tower_bbox.min_y,
                    );
                    if let Some(lines) = sheath_lines.filter(|l| !l.is_empty()) {
                        ctx.write_lines(&lines);
                    }
                }
                if should_gen_rib {
                    let rib_speed = if ctx.ir_data.tower.rib_speed_mode == "custom" {
                        ctx.ir_data.tower.rib_speed_value * MM_PER_MINUTE
                    } else {
                        ctx.ir_data.wiping.tower_print_speed * MM_PER_MINUTE
                    };
                    let rib_lines = tower::generate_rib_gcode(
                        ctx.ir_data.tower.layer_count,
                        ctx.curr_max_tower_height,
                        ctx.suggested_lh,
                        ctx.ir_data.machine.first_layer_height,
                        final_tower_height,
                        ctx.ir_data.wiping.wiper_x,
                        ctx.ir_data.wiping.wiper_y,
                        ctx.ir_data.tower.travel_speed * MM_PER_MINUTE,
                        rib_speed,
                        ctx.ir_data.machine.retract_length,
                        ctx.ir_data.machine.nozzle_diameter,
                        ctx.ir_data.rib.extra_length,
                        ctx.ir_data.rib.width,
                        ctx.ir_data.rib.fillet_wall,
                        1.0,
                        ctx.ir_data.rib.bottom_fill_style.as_str(),
                        (ctx.ir_data.machine.min_x + ctx.ir_data.machine.max_x) / 2.0,
                        (ctx.ir_data.machine.min_y + ctx.ir_data.machine.max_y) / 2.0,
                        ctx.current_layer_height,
                        ctx.ir_data.tower.safe_z_offset,
                        ctx.tower_bbox.min_x,
                        ctx.tower_bbox.min_y,
                    );
                    if let Some(lines) = rib_lines.filter(|l| !l.is_empty()) {
                        ctx.write_lines(&lines);
                    }
                }
                ctx.ir_data.tower.layer_count += 1;
            }
            let safe_z = ctx.current_layer_height.max(ctx.curr_max_tower_height)
                + ctx.ir_data.tower.safe_z_offset;
            let leaving = process_gcode_offset(
                &format!(
                    "G1 X{} Y{}",
                    crate::gcode::format_float(tower::LEAVE_TOWER_X),
                    crate::gcode::format_float(tower::LEAVE_TOWER_Y)
                ),
                ctx.tower_x_off(),
                ctx.tower_y_off(),
                safe_z,
                crate::gcode::OffsetMode::Tower,
                ctx.ir_data,
            )?;
            ctx.write_line(&format!(
                "{} Z{} ;Leaving Wiping Tower ;MKP_STAGE: LeaveTower ; 离开擦料塔",
                leaving,
                format_z(safe_z)
            ));
            ctx.write_line(&format!(
                "; LAYER_HEIGHT: {}",
                format_python_float(ctx.local_thickness)
            ));
            ctx.remove_g3_flag = true;

            let layer_height = ctx.curr_max_tower_height - ctx.last_tower_z;
            let max_lh =
                (TOWER_SUGGESTED_LAYER_COEFF * ctx.ir_data.machine.nozzle_diameter * 1000.0)
                    .round()
                    / 1000.0;
            // epsilon 容差避免浮点减法伪影
            if layer_height > max_lh + 1e-9 {
                pass2_stats.tower_layer_height_warnings.push(
                    crate::postproc::pass1::TowerLayerHeightWarning {
                        z: ctx.curr_max_tower_height,
                        layer_height,
                        max_lh,
                    },
                );
            }
            ctx.last_tower_z = ctx.curr_max_tower_height;
        }

        if trimmed.starts_with("; layer num/total_layer_count") {
            // 空条目也追加：挤出窗口中陈旧墙体，避免假阳性碰撞
            ctx.collision_check.push(CollisionLayer::new(std::mem::take(
                &mut ctx.collision_walls,
            )));
            if ctx.collision_check.len()
                > tower_collision_keep_layers(ctx.ir_data.machine.typical_layer_height)
            {
                ctx.collision_check.remove(0);
            }
            ctx.supports.clear();
        }

        // 碰撞特征收集（所有可能形成实体表面的 FEATURE）
        let is_collision_feature = has_slicer_comment_prefix(&trimmed, "FEATURE: Outer wall")
            || has_slicer_comment_prefix(&trimmed, "FEATURE: Inner wall")
            || has_slicer_comment_prefix(&trimmed, "FEATURE: Top surface")
            || has_slicer_comment_prefix(&trimmed, "FEATURE: Internal infill")
            || has_slicer_comment_prefix(&trimmed, "FEATURE: Bridge")
            || has_slicer_comment_prefix(&trimmed, "FEATURE: Support")
            || has_slicer_comment_prefix(&trimmed, "FEATURE: Ironing");
        if is_collision_feature {
            ctx.add_collision_line_flag = true;
        }
        if ctx.add_collision_line_flag && trimmed.starts_with("G1 X") && trimmed.contains('E') {
            ctx.collision_walls.push(trimmed.clone());
        }
        if has_slicer_comment_prefix(&trimmed, "FEATURE:") && !is_collision_feature {
            ctx.add_collision_line_flag = false;
        }

        if !ctx.remove_wrap_detect_flag && !ctx.remove_timelapse_flag {
            ctx.allow_print_flag = true;
        }

        if ctx.remove_g3_flag && trimmed.contains("G1 X") && trimmed.contains('E') {
            ctx.remove_g3_flag = false;
        }

        if ctx.remove_g3_flag && trimmed.contains("G3 Z") {
            ctx.remove_g3_flag = false;
            ctx.allow_print_flag = false;
        }

        if has_slicer_comment_prefix(&trimmed, "SKIPTYPE: head_wrap_detect") {
            let should_remove = ctx.ir_data.safety.disable_3rd_layer_clog_detect
                || (ctx.toml_machine != "A1" && ctx.toml_machine != "A1_MINI");
            if should_remove {
                ctx.allow_print_flag = false;
                ctx.remove_wrap_detect_flag = true;
            }
        }

        if has_slicer_comment_prefix(&trimmed, "SKIPTYPE: timelapse")
            && ctx.ir_data.safety.disable_timelapse
        {
            ctx.allow_print_flag = false;
            ctx.remove_timelapse_flag = true;
        }

        // Bambu Studio 直接输出的延时摄影段（独有注释触发）
        if ctx.ir_data.safety.disable_timelapse
            && trimmed.contains(
                "don't support timelapse gcode in spiral_mode and by object sequence for I3 structure printer",
            )
        {
            ctx.allow_print_flag = false;
            ctx.remove_timelapse_flag = true;
        }

        if (ctx.remove_wrap_detect_flag || ctx.remove_timelapse_flag)
            && trimmed.contains("; SKIPPABLE_END")
        {
            ctx.remove_wrap_detect_flag = false;
            ctx.remove_timelapse_flag = false;
            ctx.allow_print_flag = true;
            continue;
        }

        // 机器结束 gcode 段中的延时摄影过滤
        if trimmed.contains("; MACHINE_END_GCODE_START") {
            ctx.end_gcode_started = true;
        }

        if ctx.end_gcode_started && ctx.ir_data.safety.disable_timelapse {
            if trimmed.contains("M1002 judge_flag timelapse_record_flag") {
                ctx.end_gcode_timelapse_block = true;
                continue;
            }
            if ctx.end_gcode_timelapse_block {
                if trimmed.starts_with("M623") {
                    ctx.end_gcode_timelapse_block = false;
                }
                continue;
            }
        }

        // 清除切片器原始擦拭残留（底面涂胶层插入后）
        if ctx.bottom_glue_inserted_this_layer
            && !ctx.skip_slicer_wipe
            && trimmed.starts_with("; WIPE_START")
        {
            ctx.skip_slicer_wipe = true;
        }
        if ctx.skip_slicer_wipe {
            if trimmed.contains("; layer num") {
                ctx.skip_slicer_wipe = false;
                // 不 continue，让此行正常写入
            } else {
                continue;
            }
        }

        if ctx.allow_print_flag {
            // Go 的 m630Inserted 是每轮循环内新建的局部量，检查处恒为 false —— 逐字保持
            if ctx.ir_data.safety.disable_front_cover_alarm
                && ctx.toml_machine == "P1S"
                && trimmed.contains(";===== reset machine status")
            {
                ctx.write_line(&line);
                ctx.write_line("M630 S0 P0");
                continue;
            }
            ctx.write_line(&line);
        }

        if trimmed.contains(crate::gcode::MARKER_LOWER_PENTIP) {
            ctx.write_line(&format!(
                "G1 Z{}",
                format_z(ctx.curr_max_tower_height + ctx.ir_data.toolhead.z_offset)
            ));
        }

        if trimmed.contains(crate::gcode::MARKER_SHIELD_NOZZLE) {
            let (x_off, y_off) = (ctx.tower_x_off(), ctx.tower_y_off());
            ctx.write_line_offset(
                &format!(
                    "G1 X{} Y{}",
                    crate::gcode::format_float(tower::WIPE_CENTER_X),
                    crate::gcode::format_float(tower::WIPE_CENTER_Y)
                ),
                x_off,
                y_off,
                0.0,
                crate::gcode::OffsetMode::Normal,
            )?;
            ctx.write_line(&format!("G1 Z{}", format_z(ctx.last_max_tower_height)));
            let variable_wipe_code = format!("G1 X15 Y2{}", crate::gcode::get_pseudo_random());
            if ctx.ir_data.filament.filament_type == "PLA" {
                ctx.write_line_offset(
                    &variable_wipe_code,
                    x_off,
                    y_off,
                    0.0,
                    crate::gcode::OffsetMode::Normal,
                )?;
                ctx.write_line_offset(
                    &format!(
                        "G1 X{} Y{}",
                        crate::gcode::format_float(tower::WIPE_CENTER_X),
                        crate::gcode::format_float(tower::WIPE_CENTER_Y)
                    ),
                    x_off,
                    y_off,
                    0.0,
                    crate::gcode::OffsetMode::Normal,
                )?;
                ctx.write_line_offset(
                    &variable_wipe_code,
                    x_off,
                    y_off,
                    0.0,
                    crate::gcode::OffsetMode::Normal,
                )?;
                ctx.write_line_offset(
                    &format!(
                        "G1 X{} Y{}",
                        crate::gcode::format_float(tower::WIPE_CENTER_X),
                        crate::gcode::format_float(tower::WIPE_CENTER_Y)
                    ),
                    x_off,
                    y_off,
                    0.0,
                    crate::gcode::OffsetMode::Normal,
                )?;
            }
            ctx.write_line_offset(
                &variable_wipe_code,
                x_off,
                y_off,
                0.0,
                crate::gcode::OffsetMode::Normal,
            )?;
            ctx.write_line_offset(
                &format!(
                    "G1 X{} Y{}",
                    crate::gcode::format_float(tower::WIPE_CENTER_X),
                    crate::gcode::format_float(tower::WIPE_CENTER_Y)
                ),
                x_off,
                y_off,
                0.0,
                crate::gcode::OffsetMode::Normal,
            )?;
            ctx.write_line_offset(
                &variable_wipe_code,
                x_off,
                y_off,
                0.0,
                crate::gcode::OffsetMode::Normal,
            )?;
            ctx.write_line_offset(
                &format!(
                    "G1 X{} Y{}",
                    crate::gcode::format_float(tower::WIPE_ALT_X),
                    crate::gcode::format_float(tower::WIPE_ALT_Y)
                ),
                x_off,
                y_off,
                0.0,
                crate::gcode::OffsetMode::Normal,
            )?;
            ctx.write_line_offset(
                &format!("G1 X20 Y1{}", crate::gcode::get_pseudo_random()),
                x_off,
                y_off,
                0.0,
                crate::gcode::OffsetMode::Normal,
            )?;
        }

        if trimmed.contains(crate::gcode::MARKER_LIFT_Z) {
            let nums = num_strip(&trimmed);
            if let Some(&lift_z) = nums.first() {
                let comp = math_round(ctx.curr_max_tower_height + TOWER_HEIGHT_COMP_OFFSET, 3);
                if comp > lift_z {
                    ctx.write_line(&format!("G1 Z{};Compensation", format_z(comp)));
                }
            }
        }

        if trimmed.contains(crate::gcode::MARKER_PREPARE_NEXT_TOWER) {
            ctx.handle_prepare_next_tower()?;
        }

        if trimmed.contains(crate::gcode::MARKER_ADJUST_COOLING) {
            ctx.write_line(&format!(
                "G1 Z{}",
                format_z(math_round(
                    ctx.curr_max_tower_height + TOWER_HEIGHT_COMP_OFFSET,
                    3,
                ))
            ));
        }
    }

    progress(99.0, "第二遍扫描: 99%".to_string());

    if support_extrusion_count > 0 {
        tracing::info!(count = support_extrusion_count, "支撑挤出调整");
    }

    let tower_height = ctx.curr_max_tower_height;
    Ok(Pass2Output {
        lines: ctx.sink.lines,
        tower_height,
    })
}
