//! `build()` —— TomlConfig → Ir 的唯一翻译点（对照 ir/build.go 逐段移植）。
//!
//! 顺序与 Go 一字不差：弃用检查 → 零值→注册表默认 → 逐字段映射 → 兼容迁移 →
//! 派生/回退 → 范围校验（fail fast）→ 零值兜底 clamp → 枚举归一化。
//!
//! 与旧侧的刻意差异（逐条登记）：
//! - **无 sink 参数**：旧侧 sink 是 GUI 装配层的回退管控通道，sink=nil 时全部
//!   检查跳过、行为 = 本实现（CLI 链路即为 nil）。Phase 3 需要时再加参数，
//!   语义分支已在旧源码锁定。
//! - **单位换算 `* 60.0` 只发生在此文件**（gatecheck `speed_unit_conversion_only_in_ir`）。
//! - 弃用检查的 cfg 取值不走反射：快照里 deprecated 的 8 个 configKey 逐一
//!   手写 arm；无 arm 的 key 视同「反射找不到 → 跳过」（Go 对无此字段的
//!   configKey 同样 continue）。
//!
//! 搬入 mkp-ssr 时的改动**恰好两处**（其余逐字不动）：
//! - 下面那段 use：来源 crate 名 → 本仓库的模块路径。
//! - 删掉本文件里的 `fill_defaults` 与 `num_strip`：内核
//!   `crates/postprocess/src/ir/defaults.rs` 已有**逐字相同**的两份（搬入时实测 `diff` 退 0，
//!   24 行与 30 行完全一致）。同一个函数留两份 = 同一个换算在两处发生（AGENTS.md §1.1），
//!   迟早漂移，而漂移的表现是「改了一处、另一处照旧绿」。

use crate::registry::Registry;
use crate::{IntOrFloat, TomlConfig};
use postprocess::diag::PostprocError;
use postprocess::ir::types::*;
// `num_strip` 从内核拿（本文件原来那份已删）。
// **`fill_defaults` 刻意不 import**：`build()` 不调它 —— 时机是「G-code 元数据提取之后」，
// 那是编排层（pipeline）的事。import 进来只会得到一条 unused 警告，
// 而警告在 -D warnings 下就是错误。
use postprocess::ir::num_strip;
use std::collections::HashMap;

/// exec_mode 三值（`""` = CLI 回退读 TOML，process.go:219 语义）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CalibrationExecMode {
    /// `""`：回退读 TOML 的 calibration_execution_mode。
    #[default]
    Fallback,
    PrintThenCalibrate,
    DirectCalibrate,
}

impl CalibrationExecMode {
    pub fn from_cli(s: &str) -> Self {
        match s {
            "print_then_calibrate" => Self::PrintThenCalibrate,
            "direct_calibrate" => Self::DirectCalibrate,
            // 空串与非法值都走回退（CLI 透传层已限三值）
            _ => Self::Fallback,
        }
    }
}

/// 单一翻译点。`registry` 传 [`None`] 时退回硬编码默认（与 Go nil registry
/// 分支逐字一致）；Phase 1 的 CLI 恒传 [`Some`]（嵌入快照）。
pub fn build(
    cfg: &TomlConfig,
    registry: Option<&Registry>,
    exec_mode: CalibrationExecMode,
) -> Result<Ir, PostprocError> {
    if let Some(reg) = registry {
        check_deprecated_params(reg, cfg)?;
    }
    // checkZeroDefaults：Go 在 sink==nil 时直接返回 nil —— 本实现无 sink，等价跳过。
    let defaults: Option<HashMap<String, toml::Value>> = registry.map(|r| r.defaults_index());

    let mut ir = new_default_ir();

    // ---- Toolhead ----
    ir.toolhead.max_speed = cfg.toolhead.speed_limit * 60.0;
    ir.toolhead.x_offset = cfg.toolhead.offset.x;
    ir.toolhead.y_offset = cfg.toolhead.offset.y;
    ir.toolhead.z_offset = cfg.toolhead.offset.z;
    ir.toolhead.custom_mount_gcode = split_gcode_lines(&cfg.toolhead.custom_mount_gcode);
    ir.toolhead.custom_unmount_gcode = split_gcode_lines(&cfg.toolhead.custom_unmount_gcode);
    ir.toolhead.nozzle_switch_temperature = 0.0;
    ir.toolhead.glue_z_offset = cfg.toolhead.glue_z_offset;
    ir.toolhead.prime_length = cfg.toolhead.prime_length;
    if ir.toolhead.prime_length == 0.0 {
        ir.toolhead.prime_length = reg_float(&defaults, "PrimeLength", 100.0);
    }
    ir.toolhead.mkp_retract = cfg.toolhead.mkp_retract;
    ir.toolhead.first_pen_revitalization_flag = cfg.toolhead.first_pen_revitalization_flag;

    // ---- Wiping ----
    ir.wiping.preferred_mode = cfg.wiping.have_wiping_components.clone();
    ir.wiping.wiper_x = cfg.wiping.wiper_x;
    ir.wiping.wiper_y = cfg.wiping.wiper_y;
    ir.wiping.tower_print_speed = cfg.wiping.wipetower_speed;
    ir.wiping.nozzle_cooling = cfg.wiping.nozzle_cooling_flag;
    ir.wiping.fan_speed = cfg.wiping.fan_speed;
    ir.wiping.small_feature_factor = cfg.wiping.small_feature_factor;
    ir.wiping.tower_extrude_ratio = cfg.wiping.tower_extrude_ratio;
    ir.wiping.user_dry_time = cfg.wiping.user_dry_time;
    ir.wiping.support_extrusion_multiplier = cfg.wiping.support_extrusion_multiplier;

    // ---- Tower ----
    ir.tower.extrude_ratio = cfg.wiping.tower_extrude_ratio;
    ir.tower.safe_z_offset = cfg.wiping.tower_safe_z_offset;
    if ir.tower.safe_z_offset == 0.0 {
        ir.tower.safe_z_offset = reg_float(&defaults, "TowerSafeZOffset", 0.7);
    }
    ir.tower.travel_speed = cfg.wiping.tower_travel_speed;
    if ir.tower.travel_speed == 0.0 {
        ir.tower.travel_speed = reg_float(&defaults, "TowerTravelSpeed", 300.0);
    }
    ir.tower.fast_mode = normalize_fast_tower_mode(&cfg.wiping.fast_tower_mode).to_string();
    ir.tower.glue_layer_speed = cfg.wiping.tower_glue_layer_speed;
    if ir.tower.glue_layer_speed == 0.0 {
        ir.tower.glue_layer_speed = reg_float(&defaults, "TowerGlueLayerSpeed", 35.0);
    }
    ir.tower.sheath_speed = cfg.wiping.tower_sheath_speed;
    if ir.tower.sheath_speed == 0.0 {
        if let Some(v) = tower_wipe_speed_float(cfg.wiping.tower_wipe_speed) {
            ir.tower.sheath_speed = v;
        } else {
            ir.tower.sheath_speed = reg_float(&defaults, "TowerSheathSpeed", 40.0);
        }
    }
    let mut rib_speed_mode = cfg.wiping.tower_rib_speed.clone();
    if rib_speed_mode.is_empty() {
        if let Some(v) = tower_wipe_speed_float(cfg.wiping.tower_wipe_speed)
            && v > 0.0
        {
            rib_speed_mode = "custom".to_string();
        }
        if rib_speed_mode.is_empty() {
            rib_speed_mode = reg_string(&defaults, "TowerRibSpeed", "follow");
        }
    }
    if rib_speed_mode != "follow" && rib_speed_mode != "custom" {
        rib_speed_mode = "follow".to_string();
    }
    ir.tower.rib_speed_mode = rib_speed_mode;
    ir.tower.rib_speed_value = cfg.wiping.tower_rib_speed_value;
    if ir.tower.rib_speed_value == 0.0 {
        if let Some(v) = tower_wipe_speed_float(cfg.wiping.tower_wipe_speed) {
            ir.tower.rib_speed_value = v;
        } else {
            ir.tower.rib_speed_value = reg_float(&defaults, "TowerRibSpeedValue", 40.0);
        }
    }
    ir.tower.custom_layer_height =
        normalize_tower_custom_layer_height(&cfg.wiping.tower_custom_layer_height).to_string();
    let mut tower_wipe_mode = cfg.wiping.tower_wipe_mode.clone();
    if tower_wipe_mode.is_empty() {
        tower_wipe_mode = reg_string(&defaults, "TowerWipeMode", "auto");
    }
    ir.tower.wipe_mode = tower_wipe_mode;
    ir.tower.first_layer_flow = cfg.wiping.tower_first_layer_flow;
    if ir.tower.first_layer_flow == 0.0 {
        ir.tower.first_layer_flow = reg_float(&defaults, "TowerFirstLayerFlow", 1.0);
    }

    // ---- Disk ----
    ir.disk.stagger_mode = normalize_disk_stagger_mode(&cfg.wiping.disk_stagger_mode).to_string();
    ir.disk.stagger_angle = cfg.wiping.disk_stagger_angle;
    if ir.disk.stagger_angle == 0.0 {
        ir.disk.stagger_angle = reg_float(&defaults, "DiskStaggerAngle", 5.0);
    }
    ir.disk.stagger_max_angle = cfg.wiping.disk_stagger_max_angle;
    if ir.disk.stagger_max_angle == 0.0 {
        ir.disk.stagger_max_angle = reg_float(&defaults, "DiskStaggerMaxAngle", 45.0);
    }
    ir.disk.swing_mode =
        normalize_disk_stagger_swing_mode(&cfg.wiping.disk_stagger_swing_mode).to_string();

    // ---- Ironing ----
    let mut ironing_mode = cfg.wiping.ironing_mode.clone();
    if ironing_mode.is_empty() {
        ironing_mode = if cfg.wiping.interface_ironing_flag {
            "auto".to_string()
        } else {
            "off".to_string()
        };
    }
    ir.ironing.mode = ironing_mode;
    ir.ironing.use_path = cfg.wiping.use_ironing_path;
    ir.ironing.coverage_threshold = cfg.wiping.ironing_coverage_threshold;
    if ir.ironing.coverage_threshold > 0.0 && ir.ironing.coverage_threshold <= 1.0 {
        ir.ironing.coverage_threshold *= 100.0;
    }
    ir.ironing.suppress_expand = cfg.wiping.ironing_suppress_expand;
    ir.ironing.suppress_expand_mode =
        normalize_ironing_suppress_expand_mode(&cfg.wiping.ironing_suppress_expand_mode)
            .to_string();
    ir.ironing.e_sum_threshold = cfg.wiping.ironing_e_sum_threshold;
    if ir.ironing.e_sum_threshold == 0.0 {
        ir.ironing.e_sum_threshold = reg_float(&defaults, "IroningESumThreshold", 7.9);
    }
    ir.ironing.extrude_ratio = 0.0;

    // ---- Outer structure SSOT ----
    ir.wiping.outer_structure = cfg.wiping.outer_structure.clone();
    if ir.wiping.outer_structure.is_empty() || ir.wiping.outer_structure == "none" {
        ir.wiping.outer_structure = "brim".to_string();
    }
    ir.sheath.base_expand = cfg.wiping.sheath_base_expand;
    if ir.sheath.base_expand == 0.0 {
        ir.sheath.base_expand = reg_float(&defaults, "SheathBaseExpand", 5.0);
    }
    ir.sheath.wall_width = cfg.wiping.sheath_wall_width;
    if ir.sheath.wall_width == 0.0 {
        ir.sheath.wall_width = reg_float(&defaults, "SheathWallWidth", 0.8);
    }
    ir.sheath.enable_height = cfg.wiping.sheath_enable_height;
    if ir.sheath.enable_height == 0.0 {
        ir.sheath.enable_height = reg_float(&defaults, "SheathEnableHeight", 70.0);
    }
    ir.sheath.converge_layers = cfg.wiping.sheath_converge_layers;
    if ir.sheath.converge_layers == 0 {
        ir.sheath.converge_layers = reg_int(&defaults, "SheathConvergeLayers", 50);
    }

    // ---- Glue ----
    ir.glue.z_offset = cfg.toolhead.glue_z_offset;
    ir.glue.sparse_ratio = cfg.wiping.glue_sparse_ratio;
    ir.glue.pass_count = cfg.wiping.glue_pass_count;
    if ir.glue.pass_count == 0 {
        ir.glue.pass_count = reg_int(&defaults, "GluePassCount", 1);
    }
    ir.glue.z_lift_height = cfg.wiping.glue_z_lift_height;
    ir.glue.z_lift_height_first_layers = cfg.wiping.glue_z_lift_height_first_layers;
    if ir.glue.z_lift_height_first_layers == 0.0 {
        ir.glue.z_lift_height_first_layers =
            reg_float(&defaults, "GlueZLiftHeightFirstLayers", 6.0);
    }
    ir.glue.inter_model_lift_height = cfg.wiping.glue_inter_model_lift_height;
    if ir.glue.inter_model_lift_height == 0.0 {
        ir.glue.inter_model_lift_height = reg_float(&defaults, "GlueInterModelLiftHeight", 2.0);
    }
    // MinHopDistance 兼容迁移 → GlueZLiftHeight
    if ir.glue.z_lift_height == 0.0 && cfg.wiping.min_hop_distance > 0.0 {
        ir.glue.z_lift_height = cfg.wiping.min_hop_distance;
    }
    ir.glue.z_comp_enabled = cfg.wiping.glue_z_comp_enabled;
    ir.glue.z_comp_mode = cfg.wiping.glue_z_comp_mode.clone();
    if ir.glue.z_comp_mode.is_empty() {
        ir.glue.z_comp_mode = "bed".to_string();
    }
    ir.glue.z_comp_bed = ZCompCorners {
        front_left: cfg.wiping.glue_z_comp_bed_fl,
        front_right: cfg.wiping.glue_z_comp_bed_fr,
        back_left: cfg.wiping.glue_z_comp_bed_bl,
        back_right: cfg.wiping.glue_z_comp_bed_br,
    };
    ir.glue.z_comp_patch = ZCompCorners {
        front_left: cfg.wiping.glue_z_comp_patch_fl,
        front_right: cfg.wiping.glue_z_comp_patch_fr,
        back_left: cfg.wiping.glue_z_comp_patch_bl,
        back_right: cfg.wiping.glue_z_comp_patch_br,
    };

    // ---- Prime ----
    ir.prime.length = ir.toolhead.prime_length;
    ir.prime.enabled = cfg.wiping.prime_enabled;
    ir.prime.trigger_layers = cfg.wiping.prime_trigger_layers;
    ir.prime.min_path_length = cfg.wiping.prime_min_path_length;
    if ir.prime.min_path_length == 0.0 {
        ir.prime.min_path_length = reg_float(&defaults, "PrimeMinPathLength", 5.0);
    }
    ir.prime.per_model = cfg.wiping.prime_per_model;
    ir.prime.every_layer = cfg.wiping.prime_every_layer;

    // ---- Rib ----
    ir.rib.extra_length = cfg.wiping.rib_extra_length;
    ir.rib.width = cfg.wiping.rib_width;
    ir.rib.fillet_wall = cfg.wiping.rib_fillet_wall;

    // ---- Machine / Filament ----
    ir.machine.min_support_interface_enabled = cfg.wiping.min_support_interface_enabled;
    ir.filament.filament_type = "PETG".to_string();
    ir.filament.slicer = "BambuStudio".to_string();

    // ---- Safety ----
    ir.safety.l803_leak_prevent = cfg.toolhead.l803_leak_prevent_flag;
    ir.safety.disable_front_cover_alarm = cfg.wiping.disable_front_cover_alarm;
    ir.safety.disable_3rd_layer_clog_detect = cfg.wiping.disable_3rd_layer_clog_detect;
    ir.safety.disable_timelapse = cfg.wiping.disable_timelapse;
    ir.safety.unsafe_close = true;
    ir.safety.allow_proceed = false;
    ir.safety.xy_calibration =
        normalize_xy_calibration_mode(&cfg.wiping.xy_calibration_mode).to_string();
    ir.safety.z_calibration =
        normalize_z_calibration_mode(&cfg.wiping.z_calibration_mode).to_string();
    let exec_str = match exec_mode {
        CalibrationExecMode::Fallback => {
            normalize_calibration_execution_mode(&cfg.wiping.calibration_execution_mode).to_string()
        }
        CalibrationExecMode::PrintThenCalibrate => "print_then_calibrate".to_string(),
        CalibrationExecMode::DirectCalibrate => "direct_calibrate".to_string(),
    };
    ir.safety.calibration_execution_mode = exec_str;

    ir.meta.current_selected_preset = "X1".to_string();

    // ---- G-code 回退派生 ----
    if !ir.safety.l803_leak_prevent
        && ir
            .toolhead
            .custom_mount_gcode
            .iter()
            .any(|l| l.contains(";L803") || l.contains("L803"))
    {
        ir.safety.l803_leak_prevent = true;
    }
    for line in &ir.toolhead.custom_unmount_gcode {
        if line.contains("G4") {
            ir.gcode.wait_for_drying_command = line.trim().to_string();
            break;
        }
    }

    let original_mkp_retract = ir.toolhead.mkp_retract;
    let mut mkp_retract = original_mkp_retract;
    if mkp_retract == 0.0 {
        for line in &ir.toolhead.custom_mount_gcode {
            if line.contains("G1 E") {
                let nums = num_strip(line);
                if nums.len() >= 2 {
                    mkp_retract = nums[1];
                    break;
                }
            }
        }
    }
    if mkp_retract > 0.0 {
        mkp_retract = -mkp_retract;
    }
    ir.toolhead.mkp_retract = mkp_retract;
    ir.gcode.mkp_retract = mkp_retract;
    ir.gcode.custom_gcode_e_ownership = original_mkp_retract == 0.0;

    let mut mkp_extrude = ((mkp_retract.abs() + 0.5) * 100.0).round() / 100.0;
    if ir.gcode.custom_gcode_e_ownership {
        for line in &ir.toolhead.custom_unmount_gcode {
            if line.contains("G1 E") {
                let nums = num_strip(line);
                if nums.len() >= 2 && nums[1] > 0.0 && mkp_extrude != 0.0 {
                    mkp_extrude = nums[1];
                }
                break;
            }
        }
    }
    ir.gcode.mkp_extrude = mkp_extrude;

    for line in &ir.toolhead.custom_unmount_gcode {
        if line.contains("G1 E") && !line.contains("Wipe") {
            ir.gcode.refill_extrude_command = line.trim().to_string();
            break;
        }
    }

    validate_ranges(&ir, registry)?;

    // B 类零值兜底 clamp（仅 2 处，见 build.go 尾部分类注释）
    ir.wiping.tower_extrude_ratio = clamp_float(ir.wiping.tower_extrude_ratio, 0.1, 3.0);
    ir.glue.z_lift_height = clamp_float(ir.glue.z_lift_height, 5.0, 10.0);
    ir.safety.xy_calibration = normalize_xy_calibration_mode(&ir.safety.xy_calibration).to_string();
    ir.safety.z_calibration = normalize_z_calibration_mode(&ir.safety.z_calibration).to_string();
    ir.safety.calibration_execution_mode =
        normalize_calibration_execution_mode(&ir.safety.calibration_execution_mode).to_string();
    ir.tower.extrude_ratio = ir.wiping.tower_extrude_ratio;

    Ok(ir)
}

// `fill_defaults` **在这里被删掉了**（同上）：内核 `crates/postprocess/src/ir/defaults.rs`
// 有逐字相同的一份（实测 `diff` 退 0、各 24 行）。它刻意不在 `build()` 内调用 ——
// 时机是「G-code 元数据提取之后」，那是编排层的事。

fn new_default_ir() -> Ir {
    Ir {
        schema_version: SCHEMA_VERSION as i64,
        wiping: WipingIr {
            small_feature_factor: 1.0,
            support_extrusion_multiplier: 1.0,
            ..Default::default()
        },
        tower: TowerIr {
            safe_z_offset: 0.7,
            travel_speed: 300.0,
            fast_mode: FAST_TOWER_MODE_FAST.to_string(),
            rib_speed_mode: TOWER_RIB_SPEED_FOLLOW.to_string(),
            custom_layer_height: TOWER_CUSTOM_LAYER_HEIGHT_AUTO.to_string(),
            wipe_mode: TOWER_WIPE_MODE_OFF.to_string(),
            first_layer_flow: 1.0,
            ..Default::default()
        },
        ironing: IroningIr {
            e_sum_threshold: 7.9,
            mode: IRONING_MODE_OFF.to_string(),
            suppress_expand_mode: IRONING_SUPPRESS_EXPAND_MODE_BBOX.to_string(),
            ..Default::default()
        },
        sheath: SheathIr {
            base_expand: 5.0,
            wall_width: 0.8,
            enable_height: 70.0,
            converge_layers: 50,
        },
        disk: DiskIr {
            stagger_mode: DISK_STAGGER_MODE_OFF.to_string(),
            stagger_angle: 5.0,
            stagger_max_angle: 45.0,
            swing_mode: DISK_STAGGER_SWING_MODE_SPIRAL.to_string(),
        },
        rib: RibIr {
            width: 8.0,
            fillet_wall: true,
            bottom_fill_style: RIB_BOTTOM_FILL_GRID.to_string(),
            ..Default::default()
        },
        glue: GlueIr {
            sparse_ratio: 50.0,
            ..Default::default()
        },
        prime: PrimeIr {
            length: 100.0,
            trigger_layers: 30,
            ..Default::default()
        },
        safety: SafetyIr {
            unsafe_close: true,
            ..Default::default()
        },
        meta: MetaIr {
            current_selected_preset: "X1".to_string(),
            ..Default::default()
        },
        ..Default::default()
    }
}

// ---- 注册表默认 helpers（regFloat/regInt/regString 逐字对照） ----

fn reg_float(defaults: &Option<HashMap<String, toml::Value>>, key: &str, fallback: f64) -> f64 {
    let Some(d) = defaults else { return fallback };
    match d.get(key) {
        Some(toml::Value::Float(f)) => *f,
        Some(toml::Value::Integer(i)) => *i as f64,
        _ => fallback,
    }
}

fn reg_int(defaults: &Option<HashMap<String, toml::Value>>, key: &str, fallback: i64) -> i64 {
    let Some(d) = defaults else { return fallback };
    match d.get(key) {
        Some(toml::Value::Float(f)) => *f as i64,
        Some(toml::Value::Integer(i)) => *i,
        _ => fallback,
    }
}

fn reg_string(
    defaults: &Option<HashMap<String, toml::Value>>,
    key: &str,
    fallback: &str,
) -> String {
    let Some(d) = defaults else {
        return fallback.to_string();
    };
    match d.get(key) {
        Some(toml::Value::String(s)) => s.clone(),
        _ => fallback.to_string(),
    }
}

// ---- 弃用检查 ----

/// cfg 值的 Go-%v 字符串面（弃用比较用）。
fn go_sprint(cfg: &TomlConfig, config_key: &str) -> Option<String> {
    let w = &cfg.wiping;
    Some(match config_key {
        "DisableFrontCoverAlarm" => w.disable_front_cover_alarm.to_string(),
        "GlueZCompensationEnabled" => w.glue_z_comp_enabled.to_string(),
        "OuterStructure" => w.outer_structure.clone(),
        "SheathBaseExpand" => go_float_str(w.sheath_base_expand),
        "SheathConvergeLayers" => w.sheath_converge_layers.to_string(),
        "SheathEnableHeight" => go_float_str(w.sheath_enable_height),
        "SheathWallWidth" => go_float_str(w.sheath_wall_width),
        "TowerSheathSpeed" => go_float_str(w.tower_sheath_speed),
        _ => return None, // 无 arm = 反射找不到 → 跳过（Go 同）
    })
}

/// Go fmt 的 %v 对 float64 用 'g' 最短形式：5.0→"5"、0.8→"0.8"。
/// Rust `{}` 对 f64 同为最短定点（无指数），值域内逐字一致。
fn go_float_str(v: f64) -> String {
    format!("{v}")
}

fn value_go_sprint(v: &toml::Value) -> String {
    match v {
        toml::Value::Float(f) => go_float_str(*f),
        toml::Value::Integer(i) => i.to_string(),
        toml::Value::String(s) => s.clone(),
        toml::Value::Boolean(b) => b.to_string(),
        other => format!("{other:?}"),
    }
}

fn check_deprecated_params(registry: &Registry, cfg: &TomlConfig) -> Result<(), PostprocError> {
    for entry in &registry.params {
        if entry.config_key.is_empty() {
            continue;
        }
        let Some(cfg_str) = go_sprint(cfg, &entry.config_key) else {
            continue;
        };
        if entry.deprecated {
            // 零值（未显式设置）跳过：bool false / 数值 0 / 空串
            let is_zero = cfg_str == "false"
                || cfg_str == "0"
                || (cfg_str == "0.0" && !cfg_str.is_empty() && cfg_str.parse::<f64>() == Ok(0.0))
                || cfg_str.is_empty();
            if is_zero {
                continue;
            }
            if cfg_str != value_go_sprint(&entry.default_value) {
                return Err(PostprocError::InvalidConfig {
                    message: format!(
                        "参数 {} 使用了已弃用的值 {}（默认值 {}）。请从 TOML 中移除该参数或改为有效值",
                        entry.param_key,
                        cfg_str,
                        value_go_sprint(&entry.default_value)
                    ),
                });
            }
            continue;
        }
        for choice in &entry.choices {
            if choice.deprecated && cfg_str == value_go_sprint(&choice.value) {
                return Err(PostprocError::InvalidConfig {
                    message: format!(
                        "参数 {} 使用了已弃用的选项值 {}。请从 TOML 中移除该参数或改为有效值",
                        entry.param_key,
                        value_go_sprint(&choice.value)
                    ),
                });
            }
        }
    }
    Ok(())
}

// ---- 范围校验（validateRanges 逐条对照，35 float + 4 int + 枚举） ----

fn validate_ranges(ir: &Ir, registry: Option<&Registry>) -> Result<(), PostprocError> {
    struct FloatCheck {
        name: &'static str,
        val: f64,
        config_key: &'static str,
        hard_min: f64,
        hard_max: f64,
    }
    let float_checks: Vec<FloatCheck> = vec![
        FloatCheck {
            name: "IroningESumThreshold",
            val: ir.ironing.e_sum_threshold,
            config_key: "IroningESumThreshold",
            hard_min: 0.1,
            hard_max: 100.0,
        },
        FloatCheck {
            name: "IroningCoverageThreshold",
            val: ir.ironing.coverage_threshold,
            config_key: "IroningCoverageThreshold",
            hard_min: 0.0,
            hard_max: 100.0,
        },
        FloatCheck {
            name: "IroningSuppressExpand",
            val: ir.ironing.suppress_expand,
            config_key: "IroningSuppressExpand",
            hard_min: 0.0,
            hard_max: 20.0,
        },
        FloatCheck {
            name: "GlueZOffset",
            val: ir.glue.z_offset,
            config_key: "GlueZOffset",
            hard_min: -5.0,
            hard_max: 2.0,
        },
        FloatCheck {
            name: "PrimeLength",
            val: ir.prime.length,
            config_key: "PrimeLength",
            hard_min: 0.0,
            hard_max: 500.0,
        },
        FloatCheck {
            name: "GlueZLiftHeight",
            val: ir.glue.z_lift_height,
            config_key: "GlueZLiftHeight",
            hard_min: 5.0,
            hard_max: 10.0,
        },
        FloatCheck {
            name: "GlueZLiftHeightFirstLayers",
            val: ir.glue.z_lift_height_first_layers,
            config_key: "GlueZLiftHeightFirstLayers",
            hard_min: 6.0,
            hard_max: 10.0,
        },
        FloatCheck {
            name: "GlueInterModelLiftHeight",
            val: ir.glue.inter_model_lift_height,
            config_key: "GlueInterModelLiftHeight",
            hard_min: 2.0,
            hard_max: 5.0,
        },
        FloatCheck {
            name: "PrimeMinPathLength",
            val: ir.prime.min_path_length,
            config_key: "PrimeMinPathLength",
            hard_min: 0.0,
            hard_max: 100.0,
        },
        FloatCheck {
            name: "GlueZCompBedFL",
            val: ir.glue.z_comp_bed.front_left,
            config_key: "GlueZCompBedFL",
            hard_min: -0.2,
            hard_max: 0.2,
        },
        FloatCheck {
            name: "GlueZCompBedFR",
            val: ir.glue.z_comp_bed.front_right,
            config_key: "GlueZCompBedFR",
            hard_min: -0.2,
            hard_max: 0.2,
        },
        FloatCheck {
            name: "GlueZCompBedBL",
            val: ir.glue.z_comp_bed.back_left,
            config_key: "GlueZCompBedBL",
            hard_min: -0.2,
            hard_max: 0.2,
        },
        FloatCheck {
            name: "GlueZCompBedBR",
            val: ir.glue.z_comp_bed.back_right,
            config_key: "GlueZCompBedBR",
            hard_min: -0.2,
            hard_max: 0.2,
        },
        FloatCheck {
            name: "GlueZCompPatchFL",
            val: ir.glue.z_comp_patch.front_left,
            config_key: "GlueZCompPatchFL",
            hard_min: -0.1,
            hard_max: 0.1,
        },
        FloatCheck {
            name: "GlueZCompPatchFR",
            val: ir.glue.z_comp_patch.front_right,
            config_key: "GlueZCompPatchFR",
            hard_min: -0.1,
            hard_max: 0.1,
        },
        FloatCheck {
            name: "GlueZCompPatchBL",
            val: ir.glue.z_comp_patch.back_left,
            config_key: "GlueZCompPatchBL",
            hard_min: -0.1,
            hard_max: 0.1,
        },
        FloatCheck {
            name: "GlueZCompPatchBR",
            val: ir.glue.z_comp_patch.back_right,
            config_key: "GlueZCompPatchBR",
            hard_min: -0.1,
            hard_max: 0.1,
        },
        FloatCheck {
            name: "FanSpeed",
            val: ir.wiping.fan_speed,
            config_key: "FanSpeed",
            hard_min: 0.0,
            hard_max: 255.0,
        },
        FloatCheck {
            name: "SmallFeatureFactor",
            val: ir.wiping.small_feature_factor,
            config_key: "SmallFeatureFactor",
            hard_min: 0.0,
            hard_max: 2.0,
        },
        FloatCheck {
            name: "TowerExtrudeRatio",
            val: ir.wiping.tower_extrude_ratio,
            config_key: "TowerExtrudeRatio",
            hard_min: 0.1,
            hard_max: 3.0,
        },
        FloatCheck {
            name: "SheathBaseExpand",
            val: ir.sheath.base_expand,
            config_key: "SheathBaseExpand",
            hard_min: 0.0,
            hard_max: 20.0,
        },
        FloatCheck {
            name: "SheathWallWidth",
            val: ir.sheath.wall_width,
            config_key: "SheathWallWidth",
            hard_min: 0.2,
            hard_max: 3.0,
        },
        FloatCheck {
            name: "SheathEnableHeight",
            val: ir.sheath.enable_height,
            config_key: "SheathEnableHeight",
            hard_min: 0.0,
            hard_max: 300.0,
        },
        FloatCheck {
            name: "DiskStaggerAngle",
            val: ir.disk.stagger_angle,
            config_key: "DiskStaggerAngle",
            hard_min: 0.0,
            hard_max: 30.0,
        },
        FloatCheck {
            name: "DiskStaggerMaxAngle",
            val: ir.disk.stagger_max_angle,
            config_key: "DiskStaggerMaxAngle",
            hard_min: 10.0,
            hard_max: 90.0,
        },
        FloatCheck {
            name: "MaxSpeed",
            val: ir.toolhead.max_speed,
            config_key: "",
            hard_min: 0.0,
            hard_max: 200.0 * 60.0,
        },
        FloatCheck {
            name: "TowerSafeZOffset",
            val: ir.tower.safe_z_offset,
            config_key: "TowerSafeZOffset",
            hard_min: 0.3,
            hard_max: 5.0,
        },
        FloatCheck {
            name: "TowerTravelSpeed",
            val: ir.tower.travel_speed,
            config_key: "TowerTravelSpeed",
            hard_min: 50.0,
            hard_max: 500.0,
        },
        FloatCheck {
            name: "TowerSheathSpeed",
            val: ir.tower.sheath_speed,
            config_key: "TowerSheathSpeed",
            hard_min: 5.0,
            hard_max: 200.0,
        },
        FloatCheck {
            name: "TowerRibSpeedValue",
            val: ir.tower.rib_speed_value,
            config_key: "TowerRibSpeedValue",
            hard_min: 5.0,
            hard_max: 200.0,
        },
        FloatCheck {
            name: "RibExtraLength",
            val: ir.rib.extra_length,
            config_key: "RibExtraLength",
            hard_min: 0.0,
            hard_max: 300.0,
        },
        FloatCheck {
            name: "RibWidth",
            val: ir.rib.width,
            config_key: "RibWidth",
            hard_min: 0.0,
            hard_max: 300.0,
        },
        FloatCheck {
            name: "TowerFirstLayerFlow",
            val: ir.tower.first_layer_flow,
            config_key: "TowerFirstLayerFlow",
            hard_min: 0.5,
            hard_max: 2.0,
        },
    ];
    for c in &float_checks {
        if c.val == 0.0 {
            continue; // 零值=未设置，豁免
        }
        let (min_v, max_v) = match (registry, !c.config_key.is_empty()) {
            (Some(reg), true) => reg
                .lookup_range(c.config_key)
                .unwrap_or((c.hard_min, c.hard_max)),
            _ => (c.hard_min, c.hard_max),
        };
        if c.val < min_v || c.val > max_v {
            return Err(range_error(c.name, c.val, min_v, max_v));
        }
    }

    struct IntCheck {
        name: &'static str,
        val: i64,
        config_key: &'static str,
        hard_min: i64,
        hard_max: i64,
    }
    let int_checks = [
        IntCheck {
            name: "PrimeTriggerLayers",
            val: ir.prime.trigger_layers,
            config_key: "PrimeTriggerLayers",
            hard_min: 0,
            hard_max: 999,
        },
        IntCheck {
            name: "GluePassCount",
            val: ir.glue.pass_count,
            config_key: "GluePassCount",
            hard_min: 1,
            hard_max: 10,
        },
        IntCheck {
            name: "UserDryTime",
            val: ir.wiping.user_dry_time,
            config_key: "UserDryTime",
            hard_min: 0,
            hard_max: 30,
        },
        IntCheck {
            name: "SheathConvergeLayers",
            val: ir.sheath.converge_layers,
            config_key: "SheathConvergeLayers",
            hard_min: 1,
            hard_max: 200,
        },
    ];
    for c in &int_checks {
        if c.val == 0 {
            continue;
        }
        let (min_v, max_v) = match (registry, !c.config_key.is_empty()) {
            (Some(reg), true) => {
                let (a, b) = reg
                    .lookup_range(c.config_key)
                    .unwrap_or((c.hard_min as f64, c.hard_max as f64));
                (a as i64, b as i64)
            }
            _ => (c.hard_min, c.hard_max),
        };
        if c.val < min_v || c.val > max_v {
            return Err(range_error(
                c.name,
                c.val as f64,
                min_v as f64,
                max_v as f64,
            ));
        }
    }

    if ir.rib.bottom_fill_style != RIB_BOTTOM_FILL_GRID
        && ir.rib.bottom_fill_style != RIB_BOTTOM_FILL_SPIRAL
    {
        return Err(PostprocError::InvalidConfig {
            message: format!(
                "param RibBottomFillStyle value {:?} is not in allowed choices [grid, spiral]",
                ir.rib.bottom_fill_style
            ),
        });
    }
    Ok(())
}

/// preset.ParamRangeError 的消息格式逐字对照（E_CFG_INVALID_001）。
fn range_error(key: &str, value: f64, min_v: f64, max_v: f64) -> PostprocError {
    PostprocError::InvalidConfig {
        message: format!(
            "param {} value {} is out of range [{}, {}]",
            key,
            go_float_str(value),
            go_float_str(min_v),
            go_float_str(max_v)
        ),
    }
}

// ---- 纯 helpers（逐字对照 build.go 尾部） ----

pub(crate) fn split_gcode_lines(s: &str) -> Vec<String> {
    s.split('\n')
        .map(|l| l.trim_end_matches('\r').to_string())
        .collect()
}

// `num_strip` **在这里被删掉了**（搬入 mkp-ssr 的第 2 处改动）：内核
// `crates/postprocess/src/ir/defaults.rs` 里有逐字相同的一份（实测 `diff` 退 0、各 30 行），
// 且 postproc 的元数据扫描本来就调那一份。本文件顶部 `use postprocess::ir::num_strip`。

fn clamp_float(v: f64, min: f64, max: f64) -> f64 {
    if v < min {
        return min;
    }
    if v > max {
        return max;
    }
    v
}

pub(crate) fn normalize_fast_tower_mode(v: &str) -> &str {
    if v.is_empty() { "fast" } else { v }
}

pub(crate) fn normalize_tower_custom_layer_height(v: &str) -> &str {
    match v {
        "min" | "max" | "auto" => v,
        _ => "auto",
    }
}

pub(crate) fn normalize_disk_stagger_mode(v: &str) -> &str {
    if v.is_empty() { "off" } else { v }
}

pub(crate) fn normalize_disk_stagger_swing_mode(v: &str) -> &str {
    if v.is_empty() { "spiral" } else { v }
}

pub(crate) fn normalize_ironing_suppress_expand_mode(v: &str) -> &str {
    if v.is_empty() { "bbox" } else { v }
}

pub(crate) fn normalize_xy_calibration_mode(v: &str) -> &str {
    match v {
        "default" | "new" => v,
        _ => "default",
    }
}

pub(crate) fn normalize_z_calibration_mode(v: &str) -> &str {
    match v {
        "default" | "new" => v,
        _ => "default",
    }
}

pub(crate) fn normalize_calibration_execution_mode(v: &str) -> &str {
    match v {
        "print_then_calibrate" | "direct_calibrate" => v,
        _ => "print_then_calibrate",
    }
}

pub(crate) fn tower_wipe_speed_float(v: Option<IntOrFloat>) -> Option<f64> {
    match v {
        Some(IntOrFloat::F64(f)) => Some(f),
        Some(IntOrFloat::I64(i)) => Some(i as f64),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TomlConfig;

    fn minimal_cfg() -> TomlConfig {
        TomlConfig::default()
    }

    /// 7.3：速度换算只发生在这里（speed_limit * 60）。
    #[test]
    fn max_speed_is_speed_limit_times_60() {
        let mut cfg = minimal_cfg();
        cfg.toolhead.speed_limit = 70.0;
        let ir = build(&cfg, None, CalibrationExecMode::Fallback).unwrap();
        assert_eq!(ir.toolhead.max_speed, 4200.0);
    }

    /// 7.4：splitGcodeLines —— 按 \n 切、去尾 \r、保留空行（尾串产生尾部空元素）。
    #[test]
    fn split_gcode_lines_semantics() {
        assert_eq!(
            split_gcode_lines("G92 E0\r\nG1 E-5\r\n"),
            vec!["G92 E0", "G1 E-5", ""]
        );
        assert_eq!(split_gcode_lines(""), vec![""]);
    }

    /// 7.5：零值→注册表默认（nil registry 走硬编码兜底，与 Go 同分支）。
    #[test]
    fn zero_values_get_defaults() {
        let ir = build(&minimal_cfg(), None, CalibrationExecMode::Fallback).unwrap();
        assert_eq!(ir.toolhead.prime_length, 100.0);
        assert!((ir.tower.safe_z_offset - 0.7).abs() < f64::EPSILON);
        assert_eq!(ir.tower.travel_speed, 300.0);
        assert_eq!(ir.tower.glue_layer_speed, 35.0);
        assert_eq!(ir.tower.sheath_speed, 40.0);
        assert_eq!(ir.tower.rib_speed_value, 40.0);
        assert!((ir.tower.first_layer_flow - 1.0).abs() < f64::EPSILON);
        assert!((ir.disk.stagger_angle - 5.0).abs() < f64::EPSILON);
        assert!((ir.disk.stagger_max_angle - 45.0).abs() < f64::EPSILON);
        assert!((ir.ironing.e_sum_threshold - 7.9).abs() < f64::EPSILON);
        assert!((ir.sheath.base_expand - 5.0).abs() < f64::EPSILON);
        assert!((ir.sheath.wall_width - 0.8).abs() < f64::EPSILON);
        assert_eq!(ir.glue.pass_count, 1);
        assert!(
            (ir.wiping.tower_extrude_ratio - 0.1).abs() < f64::EPSILON,
            "B 类零值兜底 clamp 0→0.1"
        );
        assert!(
            (ir.glue.z_lift_height - 5.0).abs() < f64::EPSILON,
            "B 类零值兜底 clamp 0→5"
        );
    }

    /// tower_wipe_speed 兼容迁移（int/float 双形态 → SheathSpeed/RibSpeedValue/RibMode）。
    #[test]
    fn tower_wipe_speed_migrates() {
        let mut cfg = minimal_cfg();
        cfg.wiping.tower_wipe_speed = Some(IntOrFloat::I64(40));
        let ir = build(&cfg, None, CalibrationExecMode::Fallback).unwrap();
        assert_eq!(ir.tower.sheath_speed, 40.0);
        assert_eq!(ir.tower.rib_speed_value, 40.0);
        assert_eq!(ir.tower.rib_speed_mode, "custom");
    }

    /// MKP_retract 派生链：TOML 0 + mount G1 E-5 → -5、ownership=true、
    /// unmount 正向 E 5.5 覆盖 extrude；TOML 2 → -2、ownership=false、extrude=2.5。
    #[test]
    fn mkp_retract_derivation_chain() {
        let mut cfg = minimal_cfg();
        cfg.toolhead.custom_mount_gcode = "G92 E0\nG1 E-5 F1800".into();
        cfg.toolhead.custom_unmount_gcode = "G92 E0\nG1 E5.5 F1000".into();
        let ir = build(&cfg, None, CalibrationExecMode::Fallback).unwrap();
        assert_eq!(ir.toolhead.mkp_retract, -5.0);
        assert!(ir.gcode.custom_gcode_e_ownership);
        assert_eq!(ir.gcode.mkp_extrude, 5.5);
        assert_eq!(ir.gcode.refill_extrude_command, "G1 E5.5 F1000");

        let mut cfg2 = minimal_cfg();
        cfg2.toolhead.mkp_retract = 2.0;
        cfg2.toolhead.custom_unmount_gcode = "G1 E9 F1000".into();
        let ir2 = build(&cfg2, None, CalibrationExecMode::Fallback).unwrap();
        assert_eq!(ir2.toolhead.mkp_retract, -2.0, "正值符号归一化");
        assert!(!ir2.gcode.custom_gcode_e_ownership);
        assert_eq!(
            ir2.gcode.mkp_extrude, 2.5,
            "|−2|+0.5，system ownership 不被覆盖"
        );
    }

    /// 7.7：exec_mode 三值——显式覆盖与空值回退 TOML。
    #[test]
    fn exec_mode_three_values() {
        let mut cfg = minimal_cfg();
        cfg.wiping.calibration_execution_mode = "direct_calibrate".into();
        let ir = build(&cfg, None, CalibrationExecMode::Fallback).unwrap();
        assert_eq!(ir.safety.calibration_execution_mode, "direct_calibrate");
        let ir = build(&cfg, None, CalibrationExecMode::PrintThenCalibrate).unwrap();
        assert_eq!(ir.safety.calibration_execution_mode, "print_then_calibrate");

        let mut cfg2 = minimal_cfg();
        cfg2.wiping.calibration_execution_mode = "garbage".into();
        let ir2 = build(&cfg2, None, CalibrationExecMode::Fallback).unwrap();
        assert_eq!(
            ir2.safety.calibration_execution_mode, "print_then_calibrate",
            "非法值归一化"
        );
    }

    /// 弃用检查：outer_structure="sheath"（弃用选项）→ E_CFG_INVALID_001。
    /// 注意 registry=None 时检查整体跳过（Go nil registry 同）——必须传注册表。
    #[test]
    fn deprecated_choice_fails_loudly() {
        let reg = crate::load_param_registry();
        let mut cfg = minimal_cfg();
        cfg.wiping.outer_structure = "sheath".into();
        let err = build(&cfg, Some(&reg), CalibrationExecMode::Fallback).unwrap_err();
        assert!(matches!(err, PostprocError::InvalidConfig { .. }));
        assert!(err.to_string().contains("已弃用"));
    }

    /// 范围校验 fail fast：fan_speed 300 越界 → E_CFG_INVALID_001，零值豁免。
    #[test]
    fn range_validation_fails_fast() {
        let mut cfg = minimal_cfg();
        cfg.wiping.fan_speed = 300.0;
        let err = build(&cfg, None, CalibrationExecMode::Fallback).unwrap_err();
        assert!(err.to_string().contains("out of range"), "实际: {err}");
        // 零值豁免（0=未设置）
        cfg.wiping.fan_speed = 0.0;
        assert!(build(&cfg, None, CalibrationExecMode::Fallback).is_ok());
    }

    /// 覆盖率阈值 0~1 → 0~100 的兼容放大。
    #[test]
    fn coverage_threshold_scales_0_1_to_0_100() {
        let mut cfg = minimal_cfg();
        cfg.wiping.ironing_coverage_threshold = 0.5;
        let ir = build(&cfg, None, CalibrationExecMode::Fallback).unwrap();
        assert!((ir.ironing.coverage_threshold - 50.0).abs() < f64::EPSILON);
        cfg.wiping.ironing_coverage_threshold = 55.0; // >1 不放大
        let ir = build(&cfg, None, CalibrationExecMode::Fallback).unwrap();
        assert!((ir.ironing.coverage_threshold - 55.0).abs() < f64::EPSILON);
    }

    /// 9 份真实预设在**嵌入注册表**分支下 build 也必须成功（弃用/范围不误伤）。
    ///
    /// 名单是 `{机型 id}-{版本 id 小写}` 那套规则算出来的（b05 Task 5 统一命名）；
    /// 写死成字面量是刻意的 —— 名字集合本身是要盯的东西。
    #[test]
    fn nine_presets_build_with_embedded_registry() {
        let reg = crate::load_param_registry();
        for name in [
            "A1-standard",
            "A1-fast",
            "A1-fastv3.3",
            "A1_MINI-standard",
            "A1_MINI-fast",
            "A1_MINI-fastv3.3",
            "P1S-lite",
            "P2S-standard",
            "X1C-lite",
        ] {
            // 同上：预设 fixture 只在内核那一份里，路径只认 `generate::fixtures_dir()`。
            let path = crate::generate::fixtures_dir().join(format!("{name}.toml"));
            let file = crate::read_preset(&path).unwrap();
            build(&file.config, Some(&reg), CalibrationExecMode::Fallback)
                .unwrap_or_else(|e| panic!("{name}: {e}"));
        }
    }

    /// NumStrip：孤立 -/. 不算、多数字提取按序。
    #[test]
    fn num_strip_semantics() {
        assert_eq!(num_strip("G1 E-5 F1800"), vec![1.0, -5.0, 1800.0]);
        // "E.5" 的点在缓冲空时被丢弃（Go：'.' 需 bufLen>0）→ 提取到 5
        assert_eq!(num_strip("G1 E.5 F1.2.3"), vec![1.0, 5.0, 1.2, 3.0]);
        assert!(num_strip("- . -").is_empty());
    }
}
