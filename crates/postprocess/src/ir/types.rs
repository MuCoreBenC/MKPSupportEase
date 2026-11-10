//! IR 数据结构 —— 对照 `ir/types.go`（15 个子结构 + 顶层 IR）。
//!
//! JSON 键 = Go 字段名（PascalCase，无 json tag 的 MarshalIndent 输出）。
//! 刻意**不加 serde default**：Task 7.2 的闸门要求「Go 侧 IR JSON 的每个字段
//! 都必须能被这组结构吃下，缺一个即失败」。唯一的宽松点是 Go 的 nil 切片/
//! map 会序列化成 `null`——`null_to_empty` 只接受**存在但为 null** 的字段，
//! 字段缺失依旧报错。

use crate::diag::Diagnostic;
use crate::diag::messages::MessageRef;
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

/// 旧 IR / TOML 里没有 `SwapHopHeight` 时按 1.0 处理 —— 与加参数之前的硬编码一致，
/// 这样用户手上那些老预设文件不会被判成 Corrupted。
fn default_swap_hop_height() -> f64 {
    1.0
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
    /* ---- 预涂胶（2026-11-05）---- */
    /// 预涂胶速度档：`follow` / `glue` / `custom`（空串按 `follow`，见 `bed_glue::glue_feed_of`）。
    #[serde(default)]
    pub bed_glue_speed_mode: String,
    /// `custom` 档的速度（mm/s）。
    #[serde(default)]
    pub bed_glue_speed: f64,
    /// 段间横移速度档：同上三档。
    #[serde(default)]
    pub glue_travel_speed_mode: String,
    /// `custom` 档的段间横移速度（mm/s）。
    #[serde(default)]
    pub glue_travel_speed: f64,
    /// 涂胶前热床预热温度（°C）。
    #[serde(default)]
    pub bed_glue_preheat_temp: f64,
    /// 涂胶结束后等待秒数（0 = 不停）。
    #[serde(default)]
    pub bed_glue_wait_seconds: f64,
    /// 擦笔模式（**预留，不产生动作**）。
    #[serde(default)]
    pub tower_pen_wipe_mode: String,
    /// 擦笔间隔层数（**预留**）。
    #[serde(default)]
    pub tower_pen_wipe_interval: i64,
    /// 换笔脱层（z-hop）高度：只负责「离开刚打完的表面」，不是抬笔高度。
    #[serde(default = "default_swap_hop_height")]
    pub swap_hop_height: f64,
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
    /// **警告的指代**（键 + 参数），不是渲好的句子：语言到了**读它的人**手上才定。
    ///
    /// 与 `_meta.json` 的 `errorKey` 同一条口径：存事实不存句子。老的做法是在
    /// 管线里就渲成字符串，于是执行记录里那句永远是写入那一刻的语言 —— 钩子由
    /// 切片器拉起、默认简体，日文 / 英文界面上翻旧记录就会冒中文。
    ///
    /// ★ 每一条都要在 `messages.toml` 里登记：渲的时候查不到会渲成
    ///   `［未登记的文案 …］`，一眼看得见（不返回空串）。
    #[serde(deserialize_with = "null_to_empty")]
    pub warnings: Vec<MessageRef>,
    #[serde(deserialize_with = "null_to_empty")]
    pub warning_diagnostics: Vec<Diagnostic>,
    /// **用户在设置页里点出来的那几档**（2026-10-17）。
    ///
    /// 与预设（`presets-local/*.toml`）分开：那一份说的是"这台机器的工艺"，
    /// 这一格说的是"用户想额外怎么做"（跳不跳震动检测、热床涂不涂胶……）。
    /// 预设里没有这个字段 ⇒ `default()`（全按出厂），旧预设照读。
    ///
    /// ★ 它是**应用偏好写进 IR 的落点**，不是预设的一部分：
    /// `runtime::postproc_prefs::apply_to_ir` 是唯一写入点。
    /// 名字带 `prefs` 是为了在 IR 里一眼看出"这不是工艺参数"。
    #[serde(default)]
    pub prefs: PrefsIr,
}

/// 用户偏好落到 IR 上的那几个开关（见 [`Ir::prefs`]）。
///
/// **全是"真改变切出来 G-code"的那几项**；界面形态（静默/GUI）与系统通知不在这里 ——
/// 它们不影响产物，只归客户端管。
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PrefsIr {
    /// 支撑面下方一层也涂胶。
    pub glue_bottom_enabled: bool,
    /// 跳过 G-code 里的震动补偿校准检测（M970/M974）。
    pub skip_vibration_calibration: bool,
    /// 开头插入收笔序列。
    pub begin_stage_retract_enabled: bool,
    /// 收笔时在原版抬 Z 之上再补抬的相对高度（mm）。
    pub begin_stage_retract_z_lift_mm: i64,
    /// Z 轴校准停留秒数。
    pub z_calibration_dwell_sec: i64,
    /// XY 轴校准停留秒数。
    pub xy_calibration_dwell_sec: i64,
    /// 热床预涂胶（危险，出厂关）。
    pub bed_glue_enabled: bool,
    /// 热床涂胶是只涂擦料塔位置（false）还是所有模型首层（true）。
    pub bed_glue_all_models: bool,
    /// **智能塔位置**（2026-11-04 换成三档）：模型与擦料塔相撞时怎么办。
    ///
    /// - [`TowerRelocateModeIr::Ask`]（**出厂档**，2026-11-04 作者定：软件装好就是"问"）：
    ///   停下来问一句（界面侧才有"问"这件事，无界面时与 `Off` 等价）；
    /// - [`TowerRelocateModeIr::Auto`]：直接挪到自动摆出来的位置、复检、跑完这一趟；
    /// - [`TowerRelocateModeIr::Off`]：撞了就按碰撞停下 —— 塔摆哪儿是用户 / 套餐定的，
    ///   内核不自作主张挪。
    ///
    /// **只改这一趟，不落盘**。
    ///
    /// ★ `#[serde(default)]`（2026-11-04）：这一格是**客户端偏好**，不是 `.toml`
    ///   结构的一部分 —— 本仓真预设与 `preset` 解析器都没有这个键，手写的测试夹具
    ///   也不该被迫带上它。缺键 ⇒ 出厂 `Ask`（与 [`PrefsIr::default`] 同一个值，
    ///   不在这里另写一份字面量）。
    #[serde(default)]
    pub auto_tower_position: TowerRelocateModeIr,
}

impl Default for PrefsIr {
    /// 出厂档 —— **与 `runtime::app_state::ResolvedPostprocPrefs` 的默认值必须一致**
    /// （判据 `crates/postprocess` 里那条 `default_prefs_match_the_client_defaults` 咬着）。
    ///
    /// 为什么两边各写一份而不是共用：`postprocess` 是独立 crate，**不许**回头依赖
    /// 客户端（那是"内核知道界面"的反向依赖）。所以两份 + 一条判据，判据是唯一防线。
    fn default() -> Self {
        Self {
            glue_bottom_enabled: false,
            skip_vibration_calibration: false,
            begin_stage_retract_enabled: true,
            begin_stage_retract_z_lift_mm: 10,
            z_calibration_dwell_sec: 2,
            xy_calibration_dwell_sec: 3,
            bed_glue_enabled: false,
            bed_glue_all_models: false,
            auto_tower_position: TowerRelocateModeIr::Ask,
        }
    }
}

/// 「模型与擦料塔相撞时怎么办」三档 —— 内核这一侧的镜像。
///
/// ★ 与客户端 `runtime::app_state::TowerRelocateMode` **一一对应、默认一致**
///   （判据 `postproc_prefs::tests::the_two_default_sets_agree` 咬着）。
///   内核不许反向依赖客户端，所以两份 + 一条判据 —— 与 [`PrefsIr`] 其它格同一条理。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TowerRelocateModeIr {
    /// 关：撞了就按碰撞停下
    Off,
    /// 开：直接挪塔、复检、跑完（**不问**）
    Auto,
    /// 问：停下来问人（那件事住在界面侧）—— **出厂档**（2026-11-04 作者：装好就是"问"）
    #[default]
    Ask,
}
