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

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ToolheadConfig {
    pub speed_limit: f64,
    /*
     * 笔尖偏移三轴：**三个独立字段**（2026-10-02 从内联表 `offset = { x, y, z }` 拆出来）。
     *
     * 旧形状要在客户端侧养一套"多条参数共享一个 tomlKey、再靠 jsonKey 区分子字段"的机制 ——
     * 而真实语义就是三个独立参数，那套机制等于为一份历史文件在**新项目**里留兼容层。
     * 现在没有存量用户、文件全部自动生成，所以直接定成一项一字段：
     * 参数 key `toolhead.offset.x` ↔ TOML `offset_x`，一一对应，没有中间层。
     */
    #[serde(default)]
    pub offset_x: f64,
    #[serde(default)]
    pub offset_y: f64,
    #[serde(default)]
    pub offset_z: f64,
    /// 大小写混排键名（不是 snake_case），逐字对照 dto 标签。
    #[serde(rename = "MKP_retract", default)]
    pub mkp_retract: f64,
    /// 收笔时回填的挤出量。`0` = 按 `|回抽| + 0.5` 自动，非 0 = 用它（联动关闭）。
    #[serde(rename = "MKP_extrude", default)]
    pub mkp_extrude: f64,
    #[serde(default)]
    pub custom_mount_gcode: String,
    #[serde(default)]
    pub custom_unmount_gcode: String,
    /// 落笔的等待位 X（落笔骨架里的 `{wait}`）。
    #[serde(default)]
    pub pen_mount_wait_x: f64,
    /// 落笔的等待位 Y（骨架里的 `{wait_y}`）。A 系列不用，留 0。
    #[serde(default)]
    pub pen_mount_wait_y: f64,
    /// 落笔位 X（落笔骨架里的 `{target}`）。
    #[serde(default)]
    pub pen_mount_x: f64,
    /// 落笔动作行的速度（`{speed}`，mm/min）。
    #[serde(default)]
    pub pen_mount_speed: f64,
    /// 收笔的等待位 X（收笔骨架里的 `{wait}`）。
    #[serde(default)]
    pub pen_unmount_wait_x: f64,
    /// 收笔的等待位 Y（`{wait_y}`）。A 系列不用，留 0。
    #[serde(default)]
    pub pen_unmount_wait_y: f64,
    /// 收笔位 X（收笔骨架里的 `{target}`）。
    #[serde(default)]
    pub pen_unmount_x: f64,
    /// 收笔动作行的速度（`{speed}`，mm/min）。
    #[serde(default)]
    pub pen_unmount_speed: f64,
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
    /* ---- 预涂胶（2026-11-05 新增，作者：预涂胶速度/段间横移/预热温度/等待/擦笔占位） ---- */
    /// 预涂胶速度档：`follow`（跟随机型移动速度）/ `glue`（涂胶速度限制）/ `custom`。
    #[serde(default)]
    pub bed_glue_speed_mode: String,
    /// `custom` 档下的速度（mm/s）。
    #[serde(default)]
    pub bed_glue_speed: f64,
    /// 段间横移速度档：同上三档。
    #[serde(default)]
    pub glue_travel_speed_mode: String,
    /// `custom` 档下的段间横移速度（mm/s）。
    #[serde(default)]
    pub glue_travel_speed: f64,
    /// 涂胶前热床预热温度（°C）—— 原来是写死的 50。
    #[serde(default)]
    pub bed_glue_preheat_temp: f64,
    /// 涂完胶、笔抬起之后先停多久（秒）；0 = 不停。
    #[serde(default)]
    pub bed_glue_wait_seconds: f64,
    /// 擦笔模式：`off` / `auto` / `custom`。**预留参数，当前版本不产生动作。**
    #[serde(default)]
    pub tower_pen_wipe_mode: String,
    /// 擦笔间隔层数（`custom` 档用）。**预留参数。**
    #[serde(default)]
    pub tower_pen_wipe_interval: i64,
    /// 换笔脱层（z-hop）高度；0 时由 `build.rs` 回落到注册表默认 1.0。
    #[serde(default)]
    pub swap_hop_height: f64,
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

/// **一条来源版本**：某一个云端版本在哪、什么时候发的、内容摘要是什么。
///
/// 同一个形状两处用：`origin`（我从哪来）与 `history[]`（我经历过哪几版）。
///
/// 各项都是 `Option`：**不知道就不写**，绝不拿相邻的字段顶
/// （`AGENTS.md` §6 的「两格不许互相顶替」在这里是硬规矩）。
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default, rename_all = "snake_case")]
pub struct SourceRef {
    /// 怎么来的：`download`（下载）/ `duplicate`（用户手动复制）/ `copy`（内部拷贝）/
    /// `import`（导入）/ `builtin`（随包）。**只有 `duplicate` 带 `mother`。**
    pub kind: Option<String>,
    /// 用户手动复制时的母本（相对用户根）。内部自动拷贝**不写**。
    pub mother: Option<String>,
    /// 云端文件路径（`mkp/presets/A1-fast.toml`），不写绝对路径。
    pub file: Option<String>,
    /// 平台短名（`github` / `gitee` / 自建源主机名）。
    pub platform: Option<String>,
    /// `owner/repo`。
    pub repo: Option<String>,
    /// 分支 / ref（`main`）。
    pub branch: Option<String>,
    /// **这一版的发布时间**（目录 `catalog.publishedAt`）。这是「来源版本时间」的唯一来源。
    pub at: Option<String>,
    /// 这一版云端原文的 sha256（判「云端后来变了没有」）。
    pub sha256: Option<String>,
    /// 发布线（`catalogId`）。
    pub catalog: Option<String>,
    /// 发布序号（`publishSerial`，判「发布有没有往前走」）。
    pub serial: Option<u64>,
    /// 目录指纹。
    pub revision: Option<String>,
}

impl SourceRef {
    /// 三样齐了才能判「谁新」：发布线 + 序号 + 内容摘要。
    pub fn has_release_baseline(&self) -> bool {
        self.catalog.is_some() && self.serial.is_some() && self.sha256.is_some()
    }
}

/// 一份预设的**溯源记录**（文件尾部 `[mkp_source]`）。
///
/// - `origin`：我从哪来；
/// - `history`：只追加的履历，每接受 / 下载一版云端就多一条，**旧条目不改**。
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default, rename_all = "snake_case")]
pub struct SourceProvenance {
    /// 结构版本（今天只有 1）。
    pub schema: u32,
    pub origin: Option<SourceRef>,
    #[serde(default)]
    pub history: Vec<SourceRef>,
}

impl SourceProvenance {
    /// 一条都没有 = 这份文件没有来源记录（手工拖进来 / 老文件）。
    pub fn is_empty(&self) -> bool {
        self.origin.is_none() && self.history.is_empty()
    }

    /// **来源版本**：履历最后一条（最新接受的那一版）。
    ///
    /// 判「我这份基于哪一版」用它 —— 不是 `origin`（那是「我从哪来」，
    /// 复制出来的文件两者可能不同）。
    pub fn latest_version(&self) -> Option<&SourceRef> {
        self.history.last().or(self.origin.as_ref())
    }
}

/// **一份预设的身份**：哪台机器的哪个变体。
///
/// # 为什么需要一把「钥匙」
///
/// 同一台机器的同一个变体，在用户机器上可能有**三份文件**：
/// `mkp/A1MF.toml`（云端下发）、`builtin/A1_MINI-fast.toml`（我们随版本带的）、
/// `local/…`（他自己的副本）。实测前两者**内容完全相同**（sha 都是 `115e061f…`），
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
        let canonical = postprocess::postproc::machine_dims::normalize_to_canonical(raw);
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
    ///
    /// **有变体那一档转调 [`crate::generate::file_name`]**（b05 Task 5）：
    /// 以前这里自己拼一遍，而拼出来的东西要和产物同名 —— 那是两份独立实现，
    /// 中间没有编译器（`docs/ARCHITECTURE.md` §10.3 要求五个使用点共用一个函数）。
    /// 无变体那一档是本函数自己的形状：老预设没有 `# variant:`，规则（身份 → 名字）
    /// 在这一档上不成立，所以它是显式写出来的例外，不是漏掉。
    pub fn file_name(&self) -> String {
        match &self.variant {
            Some(v) => crate::generate::file_name(&self.machine, v),
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

    /// `# release_time: <v>` 头注释（可能缺失）—— **配方自己写的版本标记**。
    ///
    /// 新产物只写 `generated_at` 不写这一行 ⇒ `None`（不知道就是不知道，
    /// 不拿写盘时刻顶，见 `AGENTS.md` §6）。
    pub release_time: Option<String>,
    /// `# generated_at: <v>` 头注释（可能缺失）—— **这份字节什么时候落盘**。
    ///
    /// 它不是发布时刻、也不是配方版本时间；单独成一个字段，禁止拿去顶 `release_time`。
    pub generated_at: Option<String>,
    /// `# machine: <v>` 头注释；缺失即硬错误（见 lib.rs 文档）。
    pub machine: String,
    /// `# variant: <v>` 头注释（可能缺失）。
    ///
    /// **缺失不是错误**：老预设可能没有这一行，那时按机型/变体的区间覆盖直接回落全局。
    /// 值保持文件里的原样（实测是小写，如 `fastv3.3`），大小写归一在查表那一侧做
    /// （`Registry::lookup_range_for`）—— 这里不改用户文件里的字。
    pub variant: Option<String>,
    /// 文件尾部 `[mkp_source]` 溯源记录（可能缺失）。
    ///
    /// **详情页显示的来源信息一律以它为准**（2026-11-04 作者裁定）：
    /// 不再用 `run/` 下的账去顶。没有记录 ⇒ `None`，界面照实「未知」
    /// （用户手动拖进来的文件只有落地时间，来源就是未知）。
    pub provenance: Option<SourceProvenance>,
    /// 原始文本（CRLF 未归一的形态，供写回）。
    pub raw: String,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TomlConfig {
    pub toolhead: ToolheadConfig,
    pub wiping: WipingConfig,
}
