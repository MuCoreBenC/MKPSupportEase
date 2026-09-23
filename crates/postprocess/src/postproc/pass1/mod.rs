//! `postproc::pass1` —— 第一遍扫描（对照 processor/pass1.go 逐行移植）。
//!
//! 设计偏离（design.md §3.2 已拍板）：Go 把输出写 temp 文件再由 pass2 读回；
//! Rust 端 pass1 把行序列留在内存（`Pass1Output::lines`）按值交给 pass2，
//! 不落 temp。Go 已写好但生产未接线的 passIR 零拷贝路径即此意。
//!
//! 行为规格：pass1.go 的单循环状态机（熨平移除 / 支撑面收集 / 涂胶块发射 /
//! 元数据提取 / 风扇状态跟踪）。字节判据 G1（21,837 行）。

// 状态机逐字对照 pass1.go：嵌套 if 与手工索引保持与 Go 同形，便于逐段核对。
#![allow(
    clippy::collapsible_if,
    clippy::needless_range_loop,
    clippy::too_many_lines,
    clippy::if_same_then_else
)]

mod classifier;
mod delete_wipe;
mod glue_block;
mod metadata;
mod model_iface;
mod scan;
mod stages;
mod validity;

pub(crate) use glue_block::{
    calc_glue_z, custom_unmount_gcode_has_extrude, mount_toolhead, restore_fan_state,
    unmount_toolhead,
};

use std::collections::HashMap;

use crate::diag::PostprocError;
use crate::gcode::{format_e, has_slicer_comment_prefix, match_slicer_comment, math_round};
use crate::ir::{Ir, num_strip};

use classifier::{LineClass, classify_line};
use glue_block::emit_glue_block;
use metadata::filament_type_from_line;
use scan::{
    detect_variable_layer_height, find_prev_position_cmd, strip_e_from_g1, trim_end_idx_before_wipe,
};
use validity::check_validity_interface_set;

/// 速度发射点的 mm/s→mm/min 换算（offset.rs MM_PER_MINUTE 同一登记）。
pub(crate) const MM_PER_MINUTE: f64 = crate::postproc::offset::MM_PER_MINUTE;

/// pendingLines 刷新阈值（pass1.go:106）。
const PENDING_FLUSH_THRESHOLD: usize = 32;

/// 行收集器：Go 的 bufio.Writer + '\n' 在此等价为按行累积。
/// 直接写（涂胶块 / 回抽）与 flushPending 的相对顺序即输出顺序，逐调用保持。
#[derive(Default)]
pub(crate) struct LineSink {
    pub lines: Vec<String>,
}

impl LineSink {
    pub(crate) fn push_str(&mut self, line: &str) {
        self.lines.push(line.to_string());
    }
    pub(crate) fn push_owned(&mut self, line: String) {
        self.lines.push(line);
    }
}

/// f64 键的哈希包装（Go 侧是 map[float64]；Rust 的 f64 无 Eq 不能直接作键）。
/// -0.0 归一为 0.0，保证 Hash 与 Eq 一致；层高域不出现 NaN。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZKey(pub f64);

impl Eq for ZKey {}

impl std::hash::Hash for ZKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let bits = if self.0 == 0.0 {
            0.0f64.to_bits()
        } else {
            self.0.to_bits()
        };
        bits.hash(state);
    }
}

impl PartialOrd for ZKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

/// `LayerHeightIndex`（state.go）：每层涂胶上下文（pass2 消费）。
#[derive(Debug, Clone, PartialEq)]
pub struct LayerHeightIndex {
    pub last_layer_height: f64,
    pub pre_glue_xy: String,
    pub pre_glue_xy_no_offset: String,
    pub layer_thickness: f64,
}

/// 塔层高警告（state.go TowerLayerHeightWarning）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TowerLayerHeightWarning {
    pub z: f64,
    #[serde(rename = "layerHeight")]
    pub layer_height: f64,
    #[serde(rename = "maxLH")]
    pub max_lh: f64,
}

/// `ProcessStats`（state.go）。
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProcessStats {
    #[serde(rename = "interfacesFound")]
    pub interfaces_found: i64,
    #[serde(rename = "glueLinesAdded")]
    pub glue_lines_added: i64,
    #[serde(rename = "towerLayers")]
    pub tower_layers: i64,
    #[serde(rename = "glueTotalLength")]
    pub glue_total_length: f64,
    #[serde(rename = "primeTotalLength")]
    pub prime_total_length: f64,
    #[serde(rename = "glueCutterChanges")]
    pub glue_cutter_changes: i64,
    #[serde(rename = "glueLayerCount")]
    pub glue_layer_count: i64,
    #[serde(rename = "ironingFallbackLayers")]
    pub ironing_fallback_layers: i64,
    #[serde(rename = "towerHeight")]
    pub tower_height: f64,
    #[serde(rename = "totalZHeightLayers")]
    pub total_z_height_layers: i64,
    #[serde(rename = "variableLayerHeight")]
    pub variable_layer_height: bool,
    #[serde(rename = "originalPrintTime")]
    pub original_print_time: String,
    #[serde(rename = "totalLayerNumber")]
    pub total_layer_number: i64,
    #[serde(rename = "maxZHeight")]
    pub max_z_height: f64,
    #[serde(rename = "slicerVersion")]
    pub slicer_version: String,
    #[serde(rename = "towerLayerHeightWarnings")]
    pub tower_layer_height_warnings: Vec<TowerLayerHeightWarning>,
}

/// pass1 产物：行序列 + 统计 + 层索引（11.6：不落 temp，按值交给 pass2）。
pub struct Pass1Output {
    pub lines: Vec<String>,
    pub stats: ProcessStats,
    pub layer_index: HashMap<ZKey, LayerHeightIndex>,
}

/// `FirstPass`：第一遍扫描（pass1.go:41）。
///
/// `ir_data` 会被就地修改（机型维度填充 / 元数据提取 / State 运行态 /
/// FillDefaults）——与 Go 共享单个 `*ir.IR` 的语义一致。
/// `progress` 收到 (0..100 百分比, 消息)，Go 每 500 行且整数百分比变化时触发。
/// `cancel` 协作取消：主循环顶部时间基检查（Task 17.2，对照
/// pass1.go:167-180 每轮调 cancelChecker.Check），未取消路径零行为变化。
/// `cancel_check_interval` 由调用方注入：产品默认 100ms（engine 的
/// `DEFAULT_CANCEL_CHECK_INTERVAL`），判据注入极小值以摆脱墙钟依赖。
#[allow(clippy::too_many_lines)]
pub fn first_pass(
    content: &[String],
    ir_data: &mut Ir,
    toml_machine: &str,
    progress: &mut dyn FnMut(f64, String),
    cancel: &crate::diag::CancelToken,
    cancel_check_interval: std::time::Duration,
) -> Result<Pass1Output, PostprocError> {
    let mut layer_height_index: HashMap<ZKey, LayerHeightIndex> = HashMap::new();
    let mut layer_feature_map: HashMap<ZKey, bool> = HashMap::new();
    let mut stats = ProcessStats::default();
    let mut sink = LineSink::default();
    let mut cancel_checker = crate::postproc::cancel::CancelChecker::with_interval(
        std::time::Instant::now(),
        cancel_check_interval,
    );

    let mut copy_flag = false;
    let mut copy_flag_moisture = false;
    let mut copy_flag_fallback = false;
    let mut copy_flag_ironing = false;
    let mut has_fallback_data = false;
    let mut act_flag = false;
    let mut current_layer_height = 0.0f64;
    let mut last_layer_height = 0.0f64;
    let mut layer_thickness = 0.0f64;
    let mut last_xy_command = String::new();
    let mut last_xy_command_fe_flag = true;
    let mut start_idx = 0usize;
    let mut start_idx_moisture = 0usize;
    let mut start_idx_fallback = 0usize;
    let mut start_idx_ironing = 0usize;
    let mut start_pos_cmd = String::new();
    let mut start_pos_cmd_moisture = String::new();
    let mut start_pos_cmd_fallback = String::new();
    let mut start_pos_cmd_ironing = String::new();
    let mut iface: Vec<String> = Vec::new();
    let mut iface_moisture: Vec<String> = Vec::new();
    let mut iface_fallback: Vec<String> = Vec::new();
    let mut iface_ironing: Vec<String> = Vec::new();
    let mut xy_moisture_command = String::new();
    let mut find_last_xy_flag = true;
    let mut ironing_removal_flag = false;
    let mut has_ironing_feature = false;
    let mut has_ironing_data = false;
    let mut in_support_ironing_section = false;
    let mut e_sum: f64;
    let mut this_layer_too_low = false;
    let mut valid_count = 0i64;
    let mut invalid_count = 0i64;
    let mut param_count = 0i64;
    let mut last_progress: i64 = -1;
    let mut z_height_values: Vec<f64> = Vec::new();
    let mut pending_lines: Vec<String> = Vec::new();

    let content_len = content.len();

    ir_data.filament.slicer = "BambuStudio".to_string();

    // toml 机型校验 + 维度填充（pass1.go:150）：未命中 zero-value 跳过（M017）
    if toml_machine.is_empty() || toml_machine == "UNKNOWN" {
        tracing::warn!(
            machine = toml_machine,
            "FirstPass toml 机型缺失或未知，跳过维度填充"
        );
    } else {
        let dims = crate::postproc::machine_dims::get_machine_dimensions(toml_machine);
        if dims == crate::postproc::machine_dims::MachineDimensions::default() {
            tracing::warn!(
                machine = toml_machine,
                "FirstPass toml 机型未在 catalog 中注册，跳过维度填充"
            );
        } else {
            ir_data.machine.max_x = dims.movement_range.max_x;
            ir_data.machine.min_x = dims.movement_range.min_x;
            ir_data.machine.max_y = dims.movement_range.max_y;
            ir_data.machine.min_y = dims.movement_range.min_y;
            ir_data.machine.glue_max_y = dims.glue_area.glue_max_y;
            tracing::info!(machine = toml_machine, "机型维度填充（toml）");
        }
    }

    // flushPending：把 pendingLines 前 len-keep 行写入 sink。
    fn flush_pending(pending: &mut Vec<String>, sink: &mut LineSink, keep: usize) {
        let flush_count = pending.len().saturating_sub(keep);
        if flush_count == 0 {
            return;
        }
        for pl in pending.drain(..flush_count) {
            sink.push_owned(pl);
        }
    }

    // dropPendingAfterLastWipeEnd：删掉最后一个 "; WIPE_END" 之后的 pending 行。
    fn drop_pending_after_last_wipe_end(pending: &mut Vec<String>) -> usize {
        for k in (0..pending.len()).rev() {
            if pending[k].trim() == "; WIPE_END" {
                let dropped = pending.len() - 1 - k;
                pending.truncate(k + 1);
                return dropped;
            }
        }
        0
    }

    for i in 0..content_len {
        // 协作取消（Task 17.2）：100ms 时间基节流，未取消时是一次时间比较
        cancel_checker.check(cancel, std::time::Instant::now())?;
        let line = &content[i];
        let trimmed = line.trim().to_string();
        let class = classify_line(line);

        if i % 500 == 0 {
            let progress_pct = i as f64 / content_len as f64 * 100.0;
            if progress_pct as i64 != last_progress {
                progress(progress_pct, format!("第一遍扫描: {:.0}%", progress_pct));
                last_progress = progress_pct as i64;
            }
        }

        if trimmed.starts_with("; BambuStudio") && !has_ironing_feature && !ir_data.ironing.use_path
        {
            ir_data.filament.slicer = "BambuStudio".to_string();
        }

        if (trimmed.contains("G1 X") || trimmed.contains("G1 Y"))
            && !trimmed.contains('E')
            && last_xy_command_fe_flag
        {
            last_xy_command = trimmed.clone();
        }

        if (trimmed.contains("G1 X") || trimmed.contains("G1 Y"))
            && trimmed.contains('E')
            && !last_xy_command_fe_flag
            && copy_flag
        {
            if let Some(e_idx) = trimmed.find(" E") {
                last_xy_command = trimmed[..e_idx].trim().to_string();
            } else {
                last_xy_command = trimmed.clone();
            }
        }

        if match_slicer_comment(&trimmed, "Z_HEIGHT: ") {
            last_layer_height = current_layer_height;
            let nums = num_strip(&trimmed);
            if let Some(&n) = nums.first() {
                current_layer_height = n;
                z_height_values.push(current_layer_height);
            }
            layer_thickness = current_layer_height - last_layer_height;
            ir_data.state.inconsistent_count += 1;
            stats.total_z_height_layers += 1;
        }

        if trimmed.contains("; model printing time:") {
            for part in trimmed.splitn(2, ';') {
                let part = part.trim();
                if part.starts_with("model printing time:") {
                    stats.original_print_time = part
                        .strip_prefix("model printing time:")
                        .unwrap_or(part)
                        .trim()
                        .to_string();
                }
            }
        }

        if stats.total_layer_number == 0 && trimmed.contains("; total layer number:") {
            let nums = num_strip(&trimmed);
            if let Some(&n) = nums.first() {
                stats.total_layer_number = n as i64;
            }
        }

        if stats.max_z_height == 0.0 && trimmed.contains("; max_z_height:") {
            let nums = num_strip(&trimmed);
            if let Some(&n) = nums.first() {
                stats.max_z_height = n;
            }
        }

        if stats.slicer_version.is_empty()
            && (trimmed.contains("; BambuStudio ")
                || trimmed.contains("; OrcaSlicer ")
                || trimmed.contains("; PrusaSlicer "))
        {
            stats.slicer_version = trimmed.strip_prefix("; ").unwrap_or(&trimmed).to_string();
        }

        if param_count < 9 {
            if trimmed.contains("; travel_speed =") {
                let nums = num_strip(&trimmed);
                if let Some(&n) = nums.first() {
                    ir_data.machine.travel_speed = n;
                }
                param_count += 1;
            }
            if trimmed.contains("; nozzle_diameter = ") {
                let nums = num_strip(&trimmed);
                if let Some(&n) = nums.first() {
                    ir_data.machine.nozzle_diameter = n;
                }
                param_count += 1;
            }
            if trimmed.contains("; initial_layer_print_height =") {
                let nums = num_strip(&trimmed);
                if let Some(&n) = nums.first() {
                    ir_data.machine.first_layer_height = n;
                }
                param_count += 1;
            }
            if trimmed.contains("; layer_height = ") {
                let nums = num_strip(&trimmed);
                if let Some(&n) = nums.first() {
                    ir_data.machine.typical_layer_height = n;
                }
                param_count += 1;
            }
            if trimmed.contains("; initial_layer_speed =") {
                let nums = num_strip(&trimmed);
                if let Some(&n) = nums.first() {
                    ir_data.machine.first_layer_speed = n;
                }
                param_count += 1;
            }
            if trimmed.contains("; retraction_length = ") && ir_data.machine.retract_length == 0.0 {
                let nums = num_strip(&trimmed);
                if let Some(&n) = nums.first() {
                    ir_data.machine.retract_length = n;
                }
                param_count += 1;
            }
            if trimmed.contains("; filament_retraction_length = ") {
                let nums = num_strip(&trimmed);
                if let Some(&n) = nums.first() {
                    ir_data.machine.retract_length = n;
                }
            }
            if trimmed.contains("; nozzle_temperature = ") {
                let nums = num_strip(&trimmed);
                if let Some(&n) = nums.first() {
                    ir_data.toolhead.nozzle_switch_temperature = n;
                }
                param_count += 1;
            }
            let ft = filament_type_from_line(&trimmed);
            if !ft.is_empty() {
                ir_data.filament.filament_type = ft;
                param_count += 1;
            }
            if ir_data.filament.filament_type.is_empty()
                && trimmed.contains("; filament_settings_id ")
            {
                if trimmed.contains("PETG") || trimmed.contains("petg") {
                    ir_data.filament.filament_type = "PETG".to_string();
                } else {
                    ir_data.filament.filament_type = "PLA".to_string();
                }
                param_count += 1;
            }
            if trimmed.contains("; outer_wall_speed =") {
                let nums = num_strip(&trimmed);
                if let Some(&n) = nums.first() {
                    ir_data.machine.wall_print_speed = n;
                }
                param_count += 1;
            }
            if trimmed.contains("; support_interface_speed = ") {
                let nums = num_strip(&trimmed);
                if let Some(&n) = nums.first() {
                    ir_data.machine.support_interface_speed = n;
                }
            }
            if trimmed.contains("; enable_support_ironing = ") {
                let nums = num_strip(&trimmed);
                if let Some(&n) = nums.first() {
                    if n == 1.0 {
                        has_ironing_feature = true;
                    }
                }
            }
            if trimmed.contains("; support_type = ") {
                ir_data.machine.tree_support = trimmed.contains("tree");
            }
        }

        // 风扇状态跟踪（Phase 2：保存 / 恢复）：记录输入 G-code 的 M106 运行态，
        // 绝不写 Wiping.FanSpeed（配置值）。
        if trimmed.contains("M106 S") && !trimmed.contains("P1") && !trimmed.contains('[') {
            let nums = num_strip(&trimmed);
            if nums.len() >= 2 {
                ir_data.state.current_fan_speed = nums[1];
                ir_data.state.current_fan_speed_set = true;
            }
        }
        if trimmed.contains("M106 P1 S") {
            let nums = num_strip(&trimmed);
            if nums.len() >= 3 {
                ir_data.state.current_fan_speed = nums[2];
                ir_data.state.current_fan_speed_set = true;
            }
        }

        if class == LineClass::Feature
            && (has_slicer_comment_prefix(&trimmed, "FEATURE: Outer wall")
                || has_slicer_comment_prefix(&trimmed, "FEATURE: Inner wall")
                || has_slicer_comment_prefix(&trimmed, "FEATURE: Internal infill")
                || has_slicer_comment_prefix(&trimmed, "FEATURE: Bridge")
                || has_slicer_comment_prefix(&trimmed, "FEATURE: Top surface"))
        {
            layer_feature_map.insert(ZKey(current_layer_height), true);
        }

        if class == LineClass::Feature
            && has_slicer_comment_prefix(&trimmed, "FEATURE: Support interface")
        {
            copy_flag = true;
            start_idx = i;
            copy_flag_moisture = true;
            start_idx_moisture = i;
            copy_flag_fallback = true;
            start_idx_fallback = i;
            last_xy_command_fe_flag = false;
            start_pos_cmd = find_prev_position_cmd(content, i);
            start_pos_cmd_moisture = start_pos_cmd.clone();
            start_pos_cmd_fallback = start_pos_cmd.clone();
            if find_last_xy_flag {
                let mut j = i as isize - 1;
                while j > 0 {
                    if content[j as usize].contains("G1 X") && !content[j as usize].contains('E') {
                        let mut xy_line = content[j as usize].trim().to_string();
                        if let Some(z_idx) = xy_line.find('Z') {
                            xy_line.truncate(z_idx);
                        }
                        xy_moisture_command = xy_line;
                        find_last_xy_flag = false;
                        break;
                    }
                    j -= 1;
                }
            }
            ir_data
                .state
                .support_layer_z_set
                .insert(layer_key(current_layer_height), true);
        }

        if class == LineClass::Feature
            && has_slicer_comment_prefix(&trimmed, "FEATURE:")
            && !has_slicer_comment_prefix(&trimmed, "FEATURE: Support interface")
            && copy_flag_moisture
        {
            copy_flag_moisture = false;
            let mut end_idx_moisture = i;
            for j in start_idx_moisture..end_idx_moisture {
                if content[j].contains("M620 S") {
                    end_idx_moisture = j;
                    break;
                }
            }
            if !scan::is_support_related_feature(&trimmed) {
                end_idx_moisture =
                    trim_end_idx_before_wipe(content, start_idx_moisture, end_idx_moisture);
            }
            if end_idx_moisture > start_idx_moisture {
                if !iface_moisture.is_empty() {
                    iface_moisture.push(crate::gcode::MARKER_ZJUMP_START.to_string());
                }
                let slice = &content[start_idx_moisture..end_idx_moisture];
                let mut dw_result = if ir_data.machine.min_support_interface_enabled {
                    delete_wipe::delete_wipe(slice)
                } else {
                    delete_wipe::delete_wipe_continuous(slice)
                };
                if !start_pos_cmd_moisture.is_empty() {
                    let mut with_pos = vec![start_pos_cmd_moisture.clone()];
                    with_pos.extend(dw_result);
                    dw_result = with_pos;
                }
                start_pos_cmd_moisture = String::new();
                iface_moisture.extend(dw_result);
            }
        }

        if has_slicer_comment_prefix(&trimmed, "FEATURE:")
            && !has_slicer_comment_prefix(&trimmed, "FEATURE: Support interface")
            && copy_flag_fallback
        {
            copy_flag_fallback = false;
            let mut end_idx_fallback = i;
            for k in start_idx_fallback..end_idx_fallback {
                if content[k].contains("M620 S") {
                    end_idx_fallback = k;
                    break;
                }
            }
            if !scan::is_support_related_feature(&trimmed) {
                end_idx_fallback =
                    trim_end_idx_before_wipe(content, start_idx_fallback, end_idx_fallback);
            }
            if end_idx_fallback > start_idx_fallback {
                let slice = &content[start_idx_fallback..end_idx_fallback];
                let mut dw_result = if ir_data.machine.min_support_interface_enabled {
                    delete_wipe::delete_wipe(slice)
                } else {
                    delete_wipe::delete_wipe_continuous(slice)
                };
                if !start_pos_cmd_fallback.is_empty() {
                    let mut with_pos = vec![start_pos_cmd_fallback.clone()];
                    with_pos.extend(dw_result);
                    dw_result = with_pos;
                }
                start_pos_cmd_fallback = String::new();
                iface_fallback = dw_result;
                if check_validity_interface_set(&iface_fallback) {
                    has_fallback_data = true;
                    valid_count += 1;
                } else {
                    iface_fallback.clear();
                    invalid_count += 1;
                }
            }
        }

        if copy_flag && match_slicer_comment(&trimmed, "CHANGE_LAYER") {
            in_support_ironing_section = false;
            if ironing_removal_flag {
                ironing_removal_flag = false;
                append_prev_xy(content, i, &mut pending_lines);
            }
            if !act_flag {
                let mut end_idx = i;
                end_idx = trim_end_idx_before_wipe(content, start_idx, end_idx);
                if end_idx > start_idx {
                    if !iface.is_empty() {
                        iface.push(crate::gcode::MARKER_ZJUMP_START.to_string());
                    }
                    let iface_slice =
                        delete_wipe::strip_skippable_blocks(&content[start_idx..end_idx]);
                    let mut dw_result = if ir_data.machine.min_support_interface_enabled {
                        delete_wipe::delete_wipe(&iface_slice)
                    } else {
                        delete_wipe::delete_wipe_continuous(&iface_slice)
                    };
                    if !start_pos_cmd.is_empty() {
                        let mut with_pos = vec![start_pos_cmd.clone()];
                        with_pos.extend(dw_result);
                        dw_result = with_pos;
                    }
                    start_pos_cmd = String::new();
                    iface.extend(dw_result);
                }
                if check_validity_interface_set(&iface) {
                    act_flag = true;
                    valid_count += 1;
                } else {
                    iface.clear();
                    iface_moisture.clear();
                    copy_flag_moisture = false;
                    copy_flag_fallback = false;
                    invalid_count += 1;
                }
            }
            start_idx = i;
            copy_flag = false;
        }

        if has_slicer_comment_prefix(&trimmed, "FEATURE:")
            && !has_slicer_comment_prefix(&trimmed, "FEATURE: Support interface")
            && !has_slicer_comment_prefix(&trimmed, "FEATURE: Ironing")
            && !has_slicer_comment_prefix(&trimmed, "FEATURE: Support ironing")
            && copy_flag
            && (!has_ironing_feature
                || (!ir_data.ironing.use_path && ir_data.ironing.mode == "off"))
        {
            in_support_ironing_section = false;
            if ironing_removal_flag {
                ironing_removal_flag = false;
                append_prev_xy(content, i, &mut pending_lines);
            }
            copy_flag = false;
            if !act_flag {
                let mut end_idx = i;
                for j in start_idx..end_idx {
                    if content[j].contains("M620 S") {
                        end_idx = j;
                        break;
                    }
                }
                if !scan::is_support_related_feature(&trimmed) {
                    end_idx = trim_end_idx_before_wipe(content, start_idx, end_idx);
                }
                if end_idx > start_idx {
                    if !iface.is_empty() {
                        iface.push(crate::gcode::MARKER_ZJUMP_START.to_string());
                    }
                    let slice = &content[start_idx..end_idx];
                    let mut dw_result = if ir_data.machine.min_support_interface_enabled {
                        delete_wipe::delete_wipe(slice)
                    } else {
                        delete_wipe::delete_wipe_continuous(slice)
                    };
                    if !start_pos_cmd.is_empty() {
                        let mut with_pos = vec![start_pos_cmd.clone()];
                        with_pos.extend(dw_result);
                        dw_result = with_pos;
                    }
                    start_pos_cmd = String::new();
                    iface.extend(dw_result);
                }
                if check_validity_interface_set(&iface) {
                    act_flag = true;
                    valid_count += 1;
                } else {
                    iface.clear();
                    iface_moisture.clear();
                    copy_flag_moisture = false;
                    copy_flag_fallback = false;
                    invalid_count += 1;
                }
            }
        }

        if has_slicer_comment_prefix(&trimmed, "FEATURE: Ironing")
            && ir_data.ironing.mode == "off"
            && (has_ironing_feature || ir_data.ironing.use_path)
        {
            has_ironing_data = true;
            copy_flag = false;
            copy_flag_ironing = true;
            start_idx_ironing = i;
            start_pos_cmd_ironing = find_prev_position_cmd(content, i);
            last_xy_command_fe_flag = false;
            e_sum = 0.0;
            for temp_line in content.iter().skip(i + 1) {
                if has_slicer_comment_prefix(temp_line, "FEATURE:")
                    && !has_slicer_comment_prefix(temp_line, "FEATURE: Ironing")
                {
                    break;
                }
                if (temp_line.contains("G1 X") || temp_line.contains("G1 Y"))
                    && temp_line.contains('E')
                {
                    if let Some(e_idx) = temp_line.find('E') {
                        if e_idx + 1 < temp_line.len() {
                            let e_char = temp_line.as_bytes()[e_idx + 1];
                            let e_num_str = if e_char.is_ascii_digit() {
                                temp_line[e_idx + 1..].to_string()
                            } else if e_char == b'.' {
                                format!("0{}", &temp_line[e_idx + 1..])
                            } else {
                                String::new()
                            };
                            if !e_num_str.is_empty() {
                                let nums = num_strip(&e_num_str);
                                if let Some(&n) = nums.first() {
                                    e_sum += n.abs();
                                }
                            }
                        }
                    }
                }
            }
            e_sum = math_round(e_sum, 3);
            if ir_data.ironing.mode != "off" && e_sum < 0.1 {
                this_layer_too_low = true;
            }
            if this_layer_too_low && e_sum > 0.15 {
                this_layer_too_low = false;
            }
            if ir_data.ironing.mode == "off" {
                ironing_removal_flag = true;
                sink.push_str(&format!(
                    "G1 E-{}",
                    format_e(ir_data.machine.retract_length)
                ));
            } else if ir_data.ironing.mode == "auto" && e_sum > ir_data.ironing.e_sum_threshold {
                ironing_removal_flag = true;
                sink.push_str(&format!(
                    "G1 E-{}",
                    format_e(ir_data.machine.retract_length)
                ));
            }
        }

        // M1031 S1 ;IRONING_EXTRUSIONS_START：前瞻 1-5 行确认是否支撑面熨平
        if trimmed.starts_with("M1031 S1") && trimmed.contains("IRONING_EXTRUSIONS_START") {
            let mut is_support_ironing = false;
            let lookahead_end = (i + 5).min(content.len() - 1);
            for t in content.iter().take(lookahead_end + 1).skip(i + 1) {
                let t = t.trim();
                if t.contains("FEATURE: Support ironing") {
                    is_support_ironing = true;
                    break;
                }
                if t.contains("IRONING_EXTRUSIONS_END") {
                    break;
                }
            }
            if !is_support_ironing && in_support_ironing_section && ir_data.ironing.mode == "off" {
                is_support_ironing = true;
            }
            if is_support_ironing && ir_data.ironing.mode == "off" {
                drop_pending_after_last_wipe_end(&mut pending_lines);
                ironing_removal_flag = true;
                // 跳过当前 M1031 S1 行（不进 pendingLines；Go 的 continue 同时
                // 跳过本行末尾的刷新阈值检查，这里以真实 continue 复刻）
                continue;
            }
        }

        if has_slicer_comment_prefix(&trimmed, "FEATURE: Support ironing") {
            has_ironing_feature = true;
            has_ironing_data = true;
            in_support_ironing_section = true;
            if copy_flag {
                let end_idx = i;
                if end_idx > start_idx {
                    if !iface.is_empty() {
                        iface.push(crate::gcode::MARKER_ZJUMP_START.to_string());
                    }
                    let slice = &content[start_idx..end_idx];
                    let mut dw_result = if ir_data.machine.min_support_interface_enabled {
                        delete_wipe::delete_wipe(slice)
                    } else {
                        delete_wipe::delete_wipe_continuous(slice)
                    };
                    if !start_pos_cmd.is_empty() {
                        let mut with_pos = vec![start_pos_cmd.clone()];
                        with_pos.extend(dw_result);
                        dw_result = with_pos;
                    }
                    start_pos_cmd = String::new();
                    iface.extend(dw_result);
                }
                if check_validity_interface_set(&iface) {
                    act_flag = true;
                    valid_count += 1;
                } else {
                    iface.clear();
                    invalid_count += 1;
                }
            }
            copy_flag = false;
            if ir_data.ironing.mode != "off" || ir_data.ironing.use_path {
                copy_flag_ironing = true;
                start_idx_ironing = i;
                start_pos_cmd_ironing = find_prev_position_cmd(content, i);
                last_xy_command_fe_flag = false;
            }
            e_sum = 0.0;
            for temp_line in content.iter().skip(i + 1) {
                if has_slicer_comment_prefix(temp_line, "FEATURE:")
                    && !has_slicer_comment_prefix(temp_line, "FEATURE: Support ironing")
                {
                    break;
                }
                if (temp_line.contains("G1 X") || temp_line.contains("G1 Y"))
                    && temp_line.contains('E')
                {
                    if let Some(e_idx) = temp_line.find('E') {
                        if e_idx + 1 < temp_line.len() {
                            let e_char = temp_line.as_bytes()[e_idx + 1];
                            let e_num_str = if e_char.is_ascii_digit() {
                                temp_line[e_idx + 1..].to_string()
                            } else if e_char == b'.' {
                                format!("0{}", &temp_line[e_idx + 1..])
                            } else {
                                String::new()
                            };
                            if !e_num_str.is_empty() {
                                let nums = num_strip(&e_num_str);
                                if let Some(&n) = nums.first() {
                                    e_sum += n.abs();
                                }
                            }
                        }
                    }
                }
            }
            e_sum = math_round(e_sum, 3);
            if e_sum < 0.1 {
                this_layer_too_low = true;
            }
            if this_layer_too_low && e_sum > 0.15 {
                this_layer_too_low = false;
            }
        }

        if copy_flag_ironing && match_slicer_comment(&trimmed, "CHANGE_LAYER") {
            let end_idx = i;
            if end_idx > start_idx_ironing {
                if !iface_ironing.is_empty() {
                    iface_ironing.push(crate::gcode::MARKER_ZJUMP_START.to_string());
                }
                let slice = &content[start_idx_ironing..end_idx];
                let mut dw_result = if ir_data.machine.min_support_interface_enabled {
                    delete_wipe::delete_wipe(slice)
                } else {
                    delete_wipe::delete_wipe_continuous(slice)
                };
                if !start_pos_cmd_ironing.is_empty() {
                    let mut with_pos = vec![start_pos_cmd_ironing.clone()];
                    with_pos.extend(dw_result);
                    dw_result = with_pos;
                }
                start_pos_cmd_ironing = String::new();
                iface_ironing.extend(dw_result);
            }
            copy_flag_ironing = false;
            start_idx_ironing = i;
        }

        if has_slicer_comment_prefix(&trimmed, "FEATURE:")
            && !has_slicer_comment_prefix(&trimmed, "FEATURE: Ironing")
            && !has_slicer_comment_prefix(&trimmed, "FEATURE: Support ironing")
            && !has_slicer_comment_prefix(&trimmed, "FEATURE: Support interface")
            && copy_flag
            && has_ironing_feature
            && (ir_data.ironing.use_path || ir_data.ironing.mode != "off")
        {
            in_support_ironing_section = false;
            pending_lines.push(";Find different feature:".to_string());
            if ironing_removal_flag {
                ironing_removal_flag = false;
                append_prev_xy(content, i, &mut pending_lines);
            }
            copy_flag = false;
            let mut end_idx = i;
            for j in start_idx..end_idx {
                if content[j].contains("M620 S") {
                    end_idx = j;
                    break;
                }
            }
            if !scan::is_support_related_feature(&trimmed) {
                end_idx = trim_end_idx_before_wipe(content, start_idx, end_idx);
            }
            if end_idx > start_idx {
                if !iface.is_empty() {
                    iface.push(crate::gcode::MARKER_ZJUMP_START.to_string());
                }
                let iface_slice = delete_wipe::strip_skippable_blocks(&content[start_idx..end_idx]);
                let mut dw_result = if ir_data.machine.min_support_interface_enabled {
                    delete_wipe::delete_wipe(&iface_slice)
                } else {
                    delete_wipe::delete_wipe_continuous(&iface_slice)
                };
                if !start_pos_cmd.is_empty() {
                    let mut with_pos = vec![start_pos_cmd.clone()];
                    with_pos.extend(dw_result);
                    dw_result = with_pos;
                }
                start_pos_cmd = String::new();
                iface.extend(dw_result);
            }
            if check_validity_interface_set(&iface) {
                act_flag = true;
                valid_count += 1;
            } else {
                iface.clear();
                invalid_count += 1;
            }
        }

        if copy_flag_ironing
            && has_slicer_comment_prefix(&trimmed, "FEATURE:")
            && !has_slicer_comment_prefix(&trimmed, "FEATURE: Support ironing")
            && !has_slicer_comment_prefix(&trimmed, "FEATURE: Ironing")
        {
            let mut end_idx = i;
            for j in start_idx_ironing..end_idx {
                if content[j].contains("M620 S") {
                    end_idx = j;
                    break;
                }
            }
            if !scan::is_support_related_feature(&trimmed) {
                end_idx = trim_end_idx_before_wipe(content, start_idx_ironing, end_idx);
            }
            if end_idx > start_idx_ironing {
                if !iface_ironing.is_empty() {
                    iface_ironing.push(crate::gcode::MARKER_ZJUMP_START.to_string());
                }
                let slice = &content[start_idx_ironing..end_idx];
                let mut dw_result = if ir_data.machine.min_support_interface_enabled {
                    delete_wipe::delete_wipe(slice)
                } else {
                    delete_wipe::delete_wipe_continuous(slice)
                };
                if !start_pos_cmd_ironing.is_empty() {
                    let mut with_pos = vec![start_pos_cmd_ironing.clone()];
                    with_pos.extend(dw_result);
                    dw_result = with_pos;
                }
                start_pos_cmd_ironing = String::new();
                iface_ironing.extend(dw_result);
            }
            copy_flag_ironing = false;
        }

        if act_flag && trimmed.contains("; layer num/total_layer_count") {
            act_flag = false;
            // 涂胶块前先刷新 pendingLines，避免上一层末尾特征被劈开
            flush_pending(&mut pending_lines, &mut sink, 0);

            ir_data.state.this_layer_revitalization = false;
            let saved_inconsistent_count = ir_data.state.inconsistent_count;
            if !ir_data.prime.enabled {
                // 润笔总开关关闭，跳过
            } else if ir_data.prime.every_layer {
                ir_data.state.this_layer_revitalization = true;
            } else if ir_data.prime.trigger_layers > 0
                && ir_data.state.inconsistent_count >= ir_data.prime.trigger_layers
            {
                ir_data.state.this_layer_revitalization = true;
                ir_data.state.inconsistent_count = 0;
            }

            if ir_data.ironing.use_path {
                if !has_ironing_data || iface_ironing.is_empty() {
                    if !iface_moisture.is_empty() {
                        iface = iface_moisture.clone();
                        if !xy_moisture_command.is_empty() {
                            last_xy_command = xy_moisture_command.clone();
                        }
                    }
                    tracing::info!("无熨烫特征数据，使用支撑面路径");
                } else if ir_data.ironing.coverage_threshold == 0.0 {
                    if !iface_moisture.is_empty() {
                        iface = model_iface::build_per_model_iface(
                            &iface_ironing,
                            &iface_moisture,
                            0.0,
                            ir_data.ironing.suppress_expand,
                            &ir_data.ironing.suppress_expand_mode,
                        );
                        tracing::info!("熨烫覆盖阈值=0，按区域混合熨烫/支撑面");
                    } else {
                        iface = model_iface::convert_internal_jumps(&iface_ironing);
                        tracing::info!("熨烫覆盖阈值=0，无支撑面数据，直接使用熨烫面");
                    }
                    if !xy_moisture_command.is_empty() {
                        last_xy_command = xy_moisture_command.clone();
                    }
                } else {
                    if !iface_moisture.is_empty() {
                        iface = model_iface::build_per_model_iface(
                            &iface_ironing,
                            &iface_moisture,
                            ir_data.ironing.coverage_threshold,
                            ir_data.ironing.suppress_expand,
                            &ir_data.ironing.suppress_expand_mode,
                        );
                        tracing::info!(
                            threshold = ir_data.ironing.coverage_threshold,
                            "按区域混合熨烫/支撑面"
                        );
                    } else if has_fallback_data {
                        iface = iface_fallback.clone();
                        tracing::info!("无支撑面数据但有回退数据，使用回退路径");
                    } else {
                        iface = model_iface::convert_internal_jumps(&iface_ironing);
                        tracing::info!("无支撑面数据且无回退数据，使用熨烫面");
                    }
                    if !xy_moisture_command.is_empty() {
                        last_xy_command = xy_moisture_command.clone();
                    }
                }
                has_fallback_data = false;
                iface_fallback.clear();
                iface_moisture.clear();
            } else if has_ironing_feature && ir_data.ironing.mode == "off" {
                if !iface_moisture.is_empty() {
                    iface = iface_moisture.clone();
                }
                iface_moisture.clear();
                if !xy_moisture_command.is_empty() {
                    last_xy_command = xy_moisture_command.clone();
                }
                find_last_xy_flag = true;
            } else if ir_data.ironing.use_path
                && ir_data.ironing.mode != "off"
                && has_ironing_data
                && !iface_ironing.is_empty()
            {
                if ir_data.ironing.coverage_threshold == 0.0 {
                    if !iface_moisture.is_empty() {
                        iface = model_iface::build_per_model_iface(
                            &iface_ironing,
                            &iface_moisture,
                            0.0,
                            ir_data.ironing.suppress_expand,
                            &ir_data.ironing.suppress_expand_mode,
                        );
                    } else {
                        iface = model_iface::convert_internal_jumps(&iface_ironing);
                    }
                    if !xy_moisture_command.is_empty() {
                        last_xy_command = xy_moisture_command.clone();
                    }
                } else {
                    if !iface_moisture.is_empty() {
                        iface = model_iface::build_per_model_iface(
                            &iface_ironing,
                            &iface_moisture,
                            ir_data.ironing.coverage_threshold,
                            ir_data.ironing.suppress_expand,
                            &ir_data.ironing.suppress_expand_mode,
                        );
                    } else if has_fallback_data {
                        iface = iface_fallback.clone();
                    } else {
                        iface = model_iface::convert_internal_jumps(&iface_ironing);
                    }
                    if !xy_moisture_command.is_empty() {
                        last_xy_command = xy_moisture_command.clone();
                    }
                }
                iface_moisture.clear();
            } else if !ir_data.ironing.use_path && has_ironing_feature {
                if !iface_moisture.is_empty() {
                    iface = iface_moisture.clone();
                }
                iface_moisture.clear();
                if !xy_moisture_command.is_empty() {
                    last_xy_command = xy_moisture_command.clone();
                }
                find_last_xy_flag = true;
            } else if ir_data.ironing.mode != "off" && saved_inconsistent_count > 5 {
                if !iface_moisture.is_empty() {
                    iface = iface_moisture.clone();
                }
                iface_moisture.clear();
                if !xy_moisture_command.is_empty() {
                    last_xy_command = xy_moisture_command.clone();
                }
                find_last_xy_flag = true;
            } else if this_layer_too_low {
                sink.push_str(";Warning, Too low extrusion for ironing segment.");
                if !iface_moisture.is_empty() {
                    iface = iface_moisture.clone();
                }
                if !xy_moisture_command.is_empty() {
                    last_xy_command = xy_moisture_command.clone();
                }
                iface_moisture.clear();
            } else {
                if !iface_moisture.is_empty() {
                    iface = iface_moisture.clone();
                    if !xy_moisture_command.is_empty() {
                        last_xy_command = xy_moisture_command.clone();
                    }
                    find_last_xy_flag = true;
                }
                iface_moisture.clear();
            }
            if iface.is_empty()
                && has_fallback_data
                && (ir_data.ironing.mode != "off" || has_ironing_feature)
            {
                iface = iface_fallback.clone();
                if !xy_moisture_command.is_empty() {
                    last_xy_command = xy_moisture_command.clone();
                }
                has_fallback_data = false;
                iface_fallback.clear();
            } else {
                has_fallback_data = false;
                iface_fallback.clear();
            }
            this_layer_too_low = false;
            has_ironing_data = false;
            iface_ironing.clear();
            copy_flag_moisture = false;
            copy_flag_fallback = false;

            // 底面涂胶：主涂胶前深拷贝 iface（write_glue_lines 会改输入）
            let mut bottom_glue_iface_copy: Option<Vec<String>> = None;
            if ir_data.state.glue_bottom_enabled && last_layer_height > 0.0 {
                let skip = ir_data.state.glue_bottom_dedup_mode == "dedup"
                    && ir_data
                        .state
                        .support_layer_z_set
                        .contains_key(&layer_key(last_layer_height - layer_thickness));
                if !skip {
                    bottom_glue_iface_copy = Some(iface.clone());
                }
            }

            // 主涂胶
            emit_glue_block(
                &mut sink,
                &iface,
                ir_data,
                current_layer_height,
                last_layer_height,
                toml_machine,
                &layer_feature_map,
                &mut stats,
            )?;

            // 底面涂胶（比主涂胶低一个层高）
            if let Some(bottom_copy) = bottom_glue_iface_copy {
                let mut buf = LineSink::default();
                emit_glue_block(
                    &mut buf,
                    &bottom_copy,
                    ir_data,
                    last_layer_height,
                    last_layer_height - layer_thickness,
                    toml_machine,
                    &layer_feature_map,
                    &mut stats,
                )?;
                ir_data
                    .state
                    .bottom_glue_entries
                    .push(crate::ir::BottomGlueEntry {
                        target_z: last_layer_height,
                        lines: buf.lines,
                    });
            }

            let pre_glue_xy = crate::postproc::offset::process_gcode_offset(
                &last_xy_command,
                ir_data.toolhead.x_offset,
                ir_data.toolhead.y_offset,
                ir_data.toolhead.z_offset + ir_data.glue.z_lift_height,
                crate::gcode::OffsetMode::Normal,
                ir_data,
            )?;
            let pre_glue_xy_no_offset = crate::postproc::offset::process_gcode_offset(
                &last_xy_command,
                0.0,
                0.0,
                ir_data.toolhead.z_offset + ir_data.glue.z_lift_height,
                crate::gcode::OffsetMode::Normal,
                ir_data,
            )?;
            layer_height_index.insert(
                ZKey(current_layer_height),
                LayerHeightIndex {
                    last_layer_height,
                    pre_glue_xy,
                    pre_glue_xy_no_offset,
                    layer_thickness,
                },
            );

            iface.clear();
            last_xy_command_fe_flag = true;
        }

        if has_slicer_comment_prefix(&trimmed, "FEATURE:")
            && !has_slicer_comment_prefix(&trimmed, "FEATURE: Ironing")
            && !has_slicer_comment_prefix(&trimmed, "FEATURE: Support ironing")
            && ironing_removal_flag
        {
            in_support_ironing_section = false;
            ironing_removal_flag = false;
            append_prev_xy(content, i, &mut pending_lines);
        }

        if !ironing_removal_flag {
            pending_lines.push(line.clone());
        } else {
            let mut should_skip = false;
            if trimmed.starts_with(';') {
                if trimmed.contains("Support ironing")
                    || trimmed.contains("Ironing removed")
                    || trimmed.contains("FEATURE: Support ironing")
                    || trimmed.contains("LINE_WIDTH")
                    || trimmed.contains("LAYER_HEIGHT")
                    || trimmed.contains("Current Layer Thickness")
                    || trimmed == "; WIPE_START"
                    || trimmed == "; WIPE_END"
                {
                    should_skip = true;
                    if trimmed == "; WIPE_END" {
                        ironing_removal_flag = false;
                    }
                }
            } else if trimmed.starts_with("M1031") {
                should_skip = true;
            } else if trimmed.starts_with("M204") {
                should_skip = true;
            } else if trimmed.starts_with("M73") {
                should_skip = true;
            } else if trimmed == "G17" {
                should_skip = true;
            } else if trimmed.starts_with("G1 ")
                || trimmed.starts_with("G2 ")
                || trimmed.starts_with("G3 ")
            {
                should_skip = true;
            }
            if !should_skip {
                pending_lines.push(line.clone());
            }
        }

        if pending_lines.len() > PENDING_FLUSH_THRESHOLD {
            flush_pending(&mut pending_lines, &mut sink, 0);
        }
    }

    flush_pending(&mut pending_lines, &mut sink, 0);
    crate::ir::fill_defaults(ir_data);
    tracing::info!(valid = valid_count, invalid = invalid_count, "Pass1完成");
    stats.variable_layer_height = detect_variable_layer_height(&z_height_values);

    Ok(Pass1Output {
        lines: sink.lines,
        stats,
        layer_index: layer_height_index,
    })
}

/// 回溯上一条 G1 X/Y 行、剥 E 后追加进 pending（pass1.go 四处重复的小段）。
fn append_prev_xy(content: &[String], i: usize, pending: &mut Vec<String>) {
    let mut k = i as isize - 1;
    while k >= 0 {
        let c = &content[k as usize];
        if c.contains("G1 X") || c.contains("G1 Y") {
            let xy_temp = strip_e_from_g1(c.trim());
            pending.push(xy_temp);
            break;
        }
        k -= 1;
    }
}

/// State.SupportLayerZSet 的 f64 键序列化（Go map[float64] 的 JSON 键形态，
/// 'g' 最短表示）。写读同源，键相等 ⇔ 字符串相等（±0 除外，层高不为 -0）。
fn layer_key(z: f64) -> String {
    format_g_shortest(z)
}

/// Go `strconv.FormatFloat(v, 'g', -1, 64)` 的等价输出（覆盖层高量级）。
fn format_g_shortest(v: f64) -> String {
    // Rust {} 对 f64 就是最短往返表示；'g' 风格的指数阈值与 Go 不同
    // （Go：exponent < -4 || exponent >= 21 才用科学计数法），层高量级
    // （0~1000）两种表示一致，直接用十进制最短表示。
    let s = format!("{v}");
    s
}
