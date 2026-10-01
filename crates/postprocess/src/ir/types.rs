//! IR 数据结构 —— 对照 `ir/types.go`（15 个子结构 + 顶层 IR）。
//!
//! JSON 键 = Go 字段名（PascalCase，无 json tag 的 MarshalIndent 输出）。
//! 刻意**不加 serde default**：Task 7.2 的闸门要求「Go 侧 IR JSON 的每个字段
//! 都必须能被这组结构吃下，缺一个即失败」。唯一的宽松点是 Go 的 nil 切片/
//! map 会序列化成 `null`——`null_to_empty` 只接受**存在但为 null** 的字段，
//! 字段缺失依旧报错。

use crate::diag::Diagnostic;
use serde::{Deserialize, Serialize};

/// Go 的 nil 切片/map 序列化为 `null`；Rust 侧收成空集合（字段缺失仍报错）。
fn null_to_empty<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    let opt: Option<Vec<T>> = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

fn null_to_empty_map<'de, D, K, V>(
    deserializer: D,
) -> Result<std::collections::BTreeMap<K, V>, D::Error>
where
    D: serde::Deserializer<'de>,
    K: Deserialize<'de> + Ord,
    V: Deserialize<'de>,
{
    let opt: Option<std::collections::BTreeMap<K, V>> = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

// ---- 枚举词汇（typed string；IR 存原始串，取值集合见常量） ----

pub const WIPING_MODE_NONE: &str = "none";
pub const WIPING_MODE_TOWER: &str = "tower";
pub const WIPING_MODE_DISK: &str = "disk";
pub const WIPING_MODE_FALSE: &str = "false";

pub const IRONING_MODE_OFF: &str = "off";
pub const IRONING_MODE_AUTO: &str = "auto";

pub const IRONING_SUPPRESS_EXPAND_MODE_BBOX: &str = "bbox";

pub const DISK_STAGGER_MODE_OFF: &str = "off";
pub const DISK_STAGGER_MODE_ON: &str = "on";
pub const DISK_STAGGER_MODE_AUTO: &str = "auto";

pub const DISK_STAGGER_SWING_MODE_SPIRAL: &str = "spiral";
pub const DISK_STAGGER_SWING_MODE_OSCILLATE: &str = "oscillate";

pub const FAST_TOWER_MODE_FAST: &str = "fast";
pub const FAST_TOWER_MODE_FOLLOW: &str = "follow";
pub const FAST_TOWER_MODE_CUSTOM: &str = "custom";

pub const TOWER_RIB_SPEED_FOLLOW: &str = "follow";
pub const TOWER_RIB_SPEED_CUSTOM: &str = "custom";

pub const TOWER_CUSTOM_LAYER_HEIGHT_MIN: &str = "min";
pub const TOWER_CUSTOM_LAYER_HEIGHT_MAX: &str = "max";
pub const TOWER_CUSTOM_LAYER_HEIGHT_AUTO: &str = "auto";

pub const TOWER_WIPE_MODE_OFF: &str = "off";
pub const TOWER_WIPE_MODE_ON: &str = "on";
pub const TOWER_WIPE_MODE_AUTO: &str = "auto";

pub const XY_CALIBRATION_DEFAULT: &str = "default";
pub const XY_CALIBRATION_NEW: &str = "new";

pub const Z_CALIBRATION_DEFAULT: &str = "default";
pub const Z_CALIBRATION_NEW: &str = "new";

pub const CALIBRATION_EXEC_PRINT_THEN_CALIBRATE: &str = "print_then_calibrate";
pub const CALIBRATION_EXEC_DIRECT_CALIBRATE: &str = "direct_calibrate";

pub const RIB_BOTTOM_FILL_GRID: &str = "grid";
pub const RIB_BOTTOM_FILL_SPIRAL: &str = "spiral";

pub const SCHEMA_VERSION: i32 = 1;

/// 禁区多边形（对照 geometry.Polygon = [][]float64）。
pub type Polygon = Vec<Vec<f64>>;

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ToolheadIr {
    pub x_offset: f64,
    pub y_offset: f64,
    pub z_offset: f64,
    /// = speed_limit * 60（mm/s → mm/min，全仓唯一换算点，gatecheck 钉住）
    pub max_speed: f64,
    #[serde(deserialize_with = "null_to_empty")]
    pub custom_mount_gcode: Vec<String>,
    #[serde(deserialize_with = "null_to_empty")]
    pub custom_unmount_gcode: Vec<String>,
    pub nozzle_switch_temperature: f64,
    pub glue_z_offset: f64,
    pub prime_length: f64,
    #[serde(rename = "MKPRetract")]
    pub mkp_retract: f64,
    pub first_pen_revitalization_flag: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct WipingIr {
    pub preferred_mode: String,
    pub needed: bool,
    pub mode: String,
    pub wiper_x: f64,
    pub wiper_y: f64,
    pub tower_print_speed: f64,
    pub nozzle_cooling: bool,
    pub fan_speed: f64,
    pub small_feature_factor: f64,
    pub tower_extrude_ratio: f64,
    pub user_dry_time: i64,
    pub support_extrusion_multiplier: f64,
    pub outer_structure: String,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct TowerIr {
    pub use_towers: bool,
    pub extrude_ratio: f64,
    pub safe_z_offset: f64,
    pub travel_speed: f64,
    pub fast_mode: String,
    pub glue_layer_speed: f64,
    pub sheath_speed: f64,
    pub rib_speed_mode: String,
    pub rib_speed_value: f64,
    pub custom_layer_height: String,
    pub wipe_mode: String,
    pub first_layer_flow: f64,
    pub layer_count: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct DiskIr {
    pub stagger_mode: String,
    pub stagger_angle: f64,
    pub stagger_max_angle: f64,
    pub swing_mode: String,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct IroningIr {
    pub mode: String,
    pub use_path: bool,
    #[serde(rename = "ESumThreshold")]
    pub e_sum_threshold: f64,
    pub coverage_threshold: f64,
    pub suppress_expand: f64,
    pub suppress_expand_mode: String,
    pub extrude_ratio: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ZCompCorners {
    pub front_left: f64,
    pub front_right: f64,
    pub back_left: f64,
    pub back_right: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct GlueIr {
    pub z_offset: f64,
    pub sparse_ratio: f64,
    pub pass_count: i64,
    pub z_lift_height: f64,
    pub z_lift_height_first_layers: f64,
    pub inter_model_lift_height: f64,
    pub z_comp_enabled: bool,
    pub z_comp_mode: String,
    pub z_comp_bed: ZCompCorners,
    pub z_comp_patch: ZCompCorners,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PrimeIr {
    pub enabled: bool,
    pub length: f64,
    pub trigger_layers: i64,
    pub min_path_length: f64,
    pub per_model: bool,
    pub every_layer: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct SheathIr {
    pub base_expand: f64,
    pub wall_width: f64,
    pub enable_height: f64,
    pub converge_layers: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct RibIr {
    pub extra_length: f64,
    pub width: f64,
    pub fillet_wall: bool,
    pub bottom_fill_style: String,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct MachineIr {
    pub max_x: f64,
    pub min_x: f64,
    pub max_y: f64,
    pub min_y: f64,
    pub glue_max_y: f64,
    pub glue_min_y: f64,
    pub glue_max_x: f64,
    pub glue_min_x: f64,
    pub first_layer_height: f64,
    pub typical_layer_height: f64,
    pub nozzle_diameter: f64,
    pub travel_speed: f64,
    pub first_layer_speed: f64,
    pub wall_print_speed: f64,
    pub retract_length: f64,
    pub support_interface_speed: f64,
    pub tree_support: bool,
    pub min_support_interface_enabled: bool,
    pub machine_type: String,
    pub gcode_machine_type: String,
    #[serde(deserialize_with = "null_to_empty")]
    pub forbidden_zones: Vec<Polygon>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct GCodeIr {
    #[serde(rename = "MKPRetract")]
    pub mkp_retract: f64,
    #[serde(rename = "MKPExtrude")]
    pub mkp_extrude: f64,
    pub refill_extrude_command: String,
    pub wait_for_drying_command: String,
    pub custom_gcode_e_ownership: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct FilamentIr {
    #[serde(rename = "Type")]
    pub filament_type: String,
    pub slicer: String,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct SafetyIr {
    #[serde(rename = "L803LeakPrevent")]
    pub l803_leak_prevent: bool,
    pub disable_front_cover_alarm: bool,
    #[serde(rename = "Disable3rdLayerClogDetect")]
    pub disable_3rd_layer_clog_detect: bool,
    pub disable_timelapse: bool,
    pub unsafe_close: bool,
    pub allow_proceed: bool,
    #[serde(rename = "XYCalibration")]
    pub xy_calibration: String,
    #[serde(rename = "ZCalibration")]
    pub z_calibration: String,
    pub calibration_execution_mode: String,
    pub is_calibration_mode: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct MetaIr {
    pub preset_name: String,
    pub new_preset_name: String,
    pub update_date: String,
    pub current_selected_preset: String,
    pub mail: String,
    pub temp_z_offset_calibr: f64,
    pub temp_x_offset_calibr: f64,
    pub temp_y_offset_calibr: f64,
}

/// 底面涂胶块（pass1 生成、pass2 插入）。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BottomGlueEntry {
    pub target_z: f64,
    #[serde(deserialize_with = "null_to_empty")]
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct StateIr {
    pub inconsistent_count: i64,
    pub this_layer_revitalization: bool,
    pub glue_bottom_enabled: bool,
    pub glue_bottom_dedup_mode: String,
    #[serde(deserialize_with = "null_to_empty")]
    pub bottom_glue_entries: Vec<BottomGlueEntry>,
    /// Go 侧 map[float64]bool（JSON 字符串键）；导出时刻必为空，见
    /// tests/reference/README.md 的影子结构说明。
    #[serde(deserialize_with = "null_to_empty_map")]
    pub support_layer_z_set: std::collections::BTreeMap<String, bool>,
    pub current_fan_speed: f64,
    pub current_fan_speed_set: bool,
}

/// 顶层 IR —— 唯一业务真相源。`build()` 返回后即已合法、已归一化、已换算。
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Ir {
    pub schema_version: i64,
    pub toolhead: ToolheadIr,
    pub wiping: WipingIr,
    pub tower: TowerIr,
    pub disk: DiskIr,
    pub ironing: IroningIr,
    pub glue: GlueIr,
    pub prime: PrimeIr,
    pub sheath: SheathIr,
    pub rib: RibIr,
    pub machine: MachineIr,
    #[serde(rename = "GCode")]
    pub gcode: GCodeIr,
    pub filament: FilamentIr,
    pub safety: SafetyIr,
    pub meta: MetaIr,
    pub state: StateIr,
    #[serde(deserialize_with = "null_to_empty")]
    pub warnings: Vec<String>,
    #[serde(deserialize_with = "null_to_empty")]
    pub warning_diagnostics: Vec<Diagnostic>,
}
