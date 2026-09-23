//! serde 读面 —— 键名与 `dto/config.go` 的 toml 标签逐字一致。
//!
//! `deny_unknown_fields`：真实预设里出现结构体没有的键 = 解析失败。
//! 这把 tasks.md 6.1 的「键名逐字对照」从人工核对变成机械判据。

use serde::Deserialize;

/// `tower_wipe_speed` 的 Go 类型是 `any`（弃用字段，int 或 float 都要接受，
/// 迁移逻辑在 ir/build.go:976-992，Rust 侧由 Task 7 处理）。
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum IntOrFloat {
    I64(i64),
    F64(f64),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ToolheadOffset {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ToolheadConfig {
    pub speed_limit: f64,
    pub offset: ToolheadOffset,
    /// 大小写混排键名（不是 snake_case），逐字对照 dto 标签。
    #[serde(rename = "MKP_retract", default)]
    pub mkp_retract: f64,
    #[serde(default)]
    pub custom_mount_gcode: String,
    #[serde(default)]
    pub custom_unmount_gcode: String,
    #[serde(default)]
    pub first_pen_revitalization_flag: bool,
    #[serde(default)]
    pub l803_leak_prevent_flag: bool,
    #[serde(default)]
    pub glue_z_offset: f64,
    #[serde(default)]
    pub prime_length: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WipingConfig {
    pub have_wiping_components: String,
    pub wiper_x: f64,
    pub wiper_y: f64,
    pub wipetower_speed: f64,
    pub nozzle_cooling_flag: bool,
    pub user_dry_time: i64,
    pub support_extrusion_multiplier: f64,
    #[serde(default)]
    pub tower_extrude_ratio: f64,
    pub interface_ironing_flag: bool,
    #[serde(default)]
    pub fan_speed: f64,
    #[serde(default)]
    pub small_feature_factor: f64,
    #[serde(default)]
    pub sheath_base_expand: f64,
    #[serde(default)]
    pub sheath_wall_width: f64,
    #[serde(default)]
    pub sheath_enable_height: f64,
    #[serde(default)]
    pub sheath_converge_layers: i64,
    #[serde(default)]
    pub ironing_e_sum_threshold: f64,
    #[serde(default)]
    pub ironing_mode: String,
    #[serde(default)]
    pub use_ironing_path: bool,
    #[serde(default)]
    pub ironing_coverage_threshold: f64,
    #[serde(default)]
    pub ironing_suppress_expand: f64,
    #[serde(default)]
    pub ironing_suppress_expand_mode: String,
    #[serde(default)]
    pub prime_trigger_layers: i64,
    #[serde(default)]
    pub prime_enabled: bool,
    #[serde(default)]
    pub disk_stagger_mode: String,
    #[serde(default)]
    pub disk_stagger_angle: f64,
    #[serde(default)]
    pub disk_stagger_max_angle: f64,
    #[serde(default)]
    pub disk_stagger_swing_mode: String,
    #[serde(default)]
    pub glue_pass_count: i64,
    #[serde(default)]
    pub min_hop_distance: f64,
    #[serde(default)]
    pub prime_min_path_length: f64,
    #[serde(default)]
    pub prime_per_model: bool,
    #[serde(default)]
    pub prime_every_layer: bool,
    #[serde(default)]
    pub glue_z_lift_height: f64,
    #[serde(default)]
    pub glue_z_lift_height_first_layers: f64,
    #[serde(default)]
    pub glue_inter_model_lift_height: f64,
    #[serde(default)]
    pub outer_structure: String,
    #[serde(default)]
    pub rib_extra_length: f64,
    #[serde(default)]
    pub rib_width: f64,
    #[serde(default)]
    pub rib_fillet_wall: bool,
    #[serde(default)]
    pub disable_front_cover_alarm: bool,
    /// 键名 `disable_3rd_layer_clog_detect` 以数字开头段，逐字对照。
    #[serde(rename = "disable_3rd_layer_clog_detect", default)]
    pub disable_3rd_layer_clog_detect: bool,
    #[serde(default)]
    pub disable_timelapse: bool,
    #[serde(default)]
    pub xy_calibration_mode: String,
    #[serde(default)]
    pub z_calibration_mode: String,
    #[serde(default)]
    pub calibration_execution_mode: String,
    #[serde(default)]
    pub min_support_interface_enabled: bool,
    #[serde(default)]
    pub glue_sparse_ratio: f64,
    #[serde(rename = "glue_z_compensation_enabled", default)]
    pub glue_z_comp_enabled: bool,
    #[serde(rename = "glue_z_compensation_mode", default)]
    pub glue_z_comp_mode: String,
    #[serde(rename = "glue_z_compensation_bed_fl", default)]
    pub glue_z_comp_bed_fl: f64,
    #[serde(rename = "glue_z_compensation_bed_fr", default)]
    pub glue_z_comp_bed_fr: f64,
    #[serde(rename = "glue_z_compensation_bed_bl", default)]
    pub glue_z_comp_bed_bl: f64,
    #[serde(rename = "glue_z_compensation_bed_br", default)]
    pub glue_z_comp_bed_br: f64,
    #[serde(rename = "glue_z_compensation_patch_fl", default)]
    pub glue_z_comp_patch_fl: f64,
    #[serde(rename = "glue_z_compensation_patch_fr", default)]
    pub glue_z_comp_patch_fr: f64,
    #[serde(rename = "glue_z_compensation_patch_bl", default)]
    pub glue_z_comp_patch_bl: f64,
    #[serde(rename = "glue_z_compensation_patch_br", default)]
    pub glue_z_comp_patch_br: f64,
    #[serde(default)]
    pub fast_tower_mode: String,
    #[serde(default)]
    pub tower_glue_layer_speed: f64,
    #[serde(default)]
    pub tower_wipe_speed: Option<IntOrFloat>,
    #[serde(default)]
    pub tower_sheath_speed: f64,
    #[serde(default)]
    pub tower_rib_speed: String,
    #[serde(default)]
    pub tower_rib_speed_value: f64,
    #[serde(default)]
    pub tower_custom_layer_height: String,
    #[serde(default)]
    pub tower_wipe_mode: String,
    #[serde(default)]
    pub tower_safe_z_offset: f64,
    #[serde(default)]
    pub tower_travel_speed: f64,
    #[serde(default)]
    pub tower_first_layer_flow: f64,
}

/// 一份**用户副本**的血统：它是从哪一份、哪个版本拷出来的。
///
/// 三项各自 `Option`：老副本可能只有一半（比如手工拷的只有 `based_on`）。
/// **缺谁就是谁不知道，不许拿默认值假装知道** —— 基线摘要缺失时差异只能保守地
/// 按「两边都变了」算，那只是多问一句；假装知道会丢用户的值。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Lineage {
    /// 来源，形如 `mkp/A1MF_260628.toml`（角色目录 + 文件名，**不写绝对路径** ——
    /// 数据根可能被搬走，绝对路径会失效）。
    pub based_on: Option<String>,
    /// 建副本那一刻，来源文件头的 `# release_time`（给人看的版本号）。
    pub based_on_release_time: Option<String>,
    /// 建副本那一刻，来源文件**全文的 sha256**（给程序判「官方变了没有」）。
    pub based_on_sha256: Option<String>,
}

impl Lineage {
    /// 三项全空 = 没有血统（不是「血统未知」）。
    pub fn is_empty(&self) -> bool {
        self.based_on.is_none()
            && self.based_on_release_time.is_none()
            && self.based_on_sha256.is_none()
    }
}

/// **一份预设的身份**：哪台机器的哪个变体。
///
/// # 为什么需要一把「钥匙」
///
/// 同一台机器的同一个变体，在用户机器上可能有**三份文件**：
/// `mkp/A1MF.toml`（云端下发）、`builtin/A1_MINI-fast.toml`（我们随版本带的）、
/// `mine/…`（他自己的副本）。实测前两者**内容完全相同**（sha 都是 `115e061f…`），
/// 只有文件名不同 —— 而用户脑子里的单位是「A1 mini 的 fast 预设」，不是「三个文件」。
///
/// 所以：**文件名与路径是实现细节，`(机型, 变体)` 才是身份。**
/// 列表归一、来源回退、副本命名三处都用这一把钥匙，绝不各写一遍
/// （上一轮的教训：`offset` 的 x/y/z 共用 `tomlKey`，按错的键索引会静默拿到别人的量程）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PresetKey {
    /// 规范机型名（`A1_MINI` / `P1S`…）。归一不出来时**保留原样**，
    /// 绝不折成空串 —— 那会把两台不认识的机器混成一把钥匙。
    pub machine: String,
    /// 变体，小写。**缺失就是缺失**：老预设没有 `# variant:` 时是 `None`，
    /// 而 `None` 与 `Some("standard")` 是**两把不同的钥匙**（不猜它属于哪个变体，
    /// 猜错会把两台机器的参数混成一条）。
    pub variant: Option<String>,
}

impl PresetKey {
    pub fn of(file: &PresetFile) -> Self {
        Self::new(&file.machine, file.variant.as_deref())
    }

    pub fn new(machine: &str, variant: Option<&str>) -> Self {
        let raw = machine.trim();
        let canonical = mkp_pp::postproc::machine_dims::normalize_to_canonical(raw);
        Self {
            machine: if canonical.is_empty() {
                raw.to_string()
            } else {
                canonical
            },
            variant: variant
                .map(|v| v.trim().to_ascii_lowercase())
                .filter(|v| !v.is_empty()),
        }
    }

    /// 这把钥匙对应的文件名：`A1_MINI-fast.toml`（无变体 ⇒ `A1_MINI.toml`）。
    ///
    /// 与内置产物同一套命名 —— **语义即钥匙即文件名**，于是「同一个变体的副本」
    /// 在磁盘上也只会有一个名字，不会出现 `A1MF.toml` 与 `A1_MINI-fast.toml` 两份同参数副本。
    pub fn file_name(&self) -> String {
        match &self.variant {
            Some(v) => format!("{}-{}.toml", self.machine, v),
            None => format!("{}.toml", self.machine),
        }
    }

    /// 给人看的一行：`A1_MINI fast`。
    pub fn label(&self) -> String {
        match &self.variant {
            Some(v) => format!("{} {}", self.machine, v),
            None => self.machine.clone(),
        }
    }
}

/// 读入结果：serde 面 + 头注释两个非 TOML 字段 + 原文（写回保注释的底）。
#[derive(Debug, Clone, PartialEq)]
pub struct PresetFile {
    pub config: TomlConfig,

    /// `# release_time: <v>` 头注释（可能缺失）。
    pub release_time: Option<String>,
    /// `# machine: <v>` 头注释；缺失即硬错误（见 lib.rs 文档）。
    pub machine: String,
    /// `# variant: <v>` 头注释（可能缺失）。
    ///
    /// **缺失不是错误**：老预设可能没有这一行，那时按机型/变体的区间覆盖直接回落全局。
    /// 值保持文件里的原样（实测是小写，如 `fastv3.3`），大小写归一在查表那一侧做
    /// （`Registry::lookup_range_for`）—— 这里不改用户文件里的字。
    pub variant: Option<String>,
    /// `# based_on*` 三行（可能缺失）。**今天磁盘上所有预设都没有这三行** ⇒ `None`，
    /// 界面按「没有血统」显示；它只出现在我们自己建的副本上。
    pub lineage: Option<Lineage>,
    /// 原始文本（CRLF 未归一的形态，供写回）。
    pub raw: String,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TomlConfig {
    pub toolhead: ToolheadConfig,
    pub wiping: WipingConfig,
}
