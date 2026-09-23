//! config 步两层强校验（Task 23，补 Task 14 的漏移植）。
//!
//! 契约来源（逐字对照）：
//! - `config.ValidateConfig`（config.go:54）：6 条硬检查，先跑；
//! - `preset.ValidateAgainstRegistry`（validate_registry.go:30）：按 param_registry
//!   全参数强校验，后跑。Go 侧 CLI 与 GUI 两条装配路径都生效（main.go:152-156 /
//!   app_lifecycle startup()）；Rust 侧 registry 是嵌入快照**恒就绪**，不存在
//!   Go 的「registry 未就绪跳过」分支（登记差异，非缺口）。
//!
//! 两层是**双层关系**，不许合并：与已移植的 `ir::build::validateRanges` 也是双层
//! （Go 同样双层）——本层吃 TOML 原值（速度是 mm/s、无单位换算），IR 层吃换算后
//! 值；bounds 数值不同（如 WiperX 这里 [-40,260]，registry [10,226]，IR 层不查）。
//!
//! 刻意复刻的谓词细节（validate_registry.go:72-108）：
//! - 只有 `valueType=="string" && choices 非空` 才查枚举——**bool 带 choices 不校验**
//!   （快照里 FirstPenRevitalizationFlag/PrimeEveryLayer 等 bool 均带 choices，
//!   若错查枚举，全部真实预设会因 "false" ∉ {off,on} 而误报）；
//! - 只有 `valueType∈{int,float} && (min||max)` 才查范围；非数值面（string/bool）
//!   落到范围分支时**跳过**（asFloat64 !ok ⇒ 不校验）；
//! - 零值豁免：string 的 ""、数值的 0（含 int 0）视为「未设置」；
//! - 消息格式：choices 用 `param %s value %q is not in allowed choices [%s]`，
//!   范围用 ParamRangeError SSOT（`param %s value %v is out of range [%s, %s]`，
//!   缺端格式化为空串——formatBound 的 nil 分支）。
//!
//! **bug-compatible 键名漂移（实测登记）**：registry 侧 10 个 `GlueZCompensation*`
//! 参数的 configKey 是全拼，而 Go `DtoToConfigMap` 里的键是缩写（`GlueZCompBedFL`
//! 等）⇒ Go 的强校验**从未真正校验过这 10 个参数**（map 查不到即 continue）。
//! 本模块的 arm 表按 DtoToConfigMap 命名、查不到返回 None ⇒ 行为逐字一致。
//! 用例 `registry_skips_glue_z_compensation_family_like_go` 锁住该行为。

use crate::model::TomlConfig;
use crate::registry::{EffectiveRange, ParamEntry, Registry};
use postprocess::diag::PostprocError;

/// Go fmt `%v`/`%g` 对 float64 的最短定点形式（值域内与 Rust `{}` 逐字一致；
/// 与 ir::build 的 go_float_str 同一约定——本 crate 不依赖 ir，各自持有）。
fn go_float(v: f64) -> String {
    format!("{v}")
}

fn invalid(message: String) -> PostprocError {
    PostprocError::InvalidConfig { message }
}

/// `DtoToConfigMap`（dto_mapper.go:16）的值形态——Go 侧 map[string]any 的类型面。
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigVal {
    F64(f64),
    I64(i64),
    Str(String),
    Bool(bool),
}

/// Go `fmt.Sprint(val)`：枚举校验时把 cfg 值与 choice 值拉到同一字符串面。
fn sprint(v: &ConfigVal) -> String {
    match v {
        ConfigVal::F64(f) => go_float(*f),
        ConfigVal::I64(i) => i.to_string(),
        ConfigVal::Str(s) => s.clone(),
        ConfigVal::Bool(b) => b.to_string(),
    }
}

/// Go `choiceValueToString`：choice.Value 的字符串面（nil → ""，Rust 无 nil 面）。
fn choice_value_to_string(v: &toml::Value) -> String {
    match v {
        toml::Value::String(s) => s.clone(),
        toml::Value::Boolean(b) => b.to_string(),
        toml::Value::Integer(i) => i.to_string(),
        toml::Value::Float(f) => go_float(*f),
        other => format!("{other:?}"),
    }
}

/// Go `asFloat64`：非数值面返回 None ⇒ 范围分支跳过。
fn as_f64(v: &ConfigVal) -> Option<f64> {
    match v {
        ConfigVal::F64(f) => Some(*f),
        ConfigVal::I64(i) => Some(*i as f64),
        _ => None,
    }
}

/// Go `formatBound`：nil → ""（开放端在消息里显示为空串）。
fn format_bound(b: Option<f64>) -> String {
    b.map_or_else(String::new, go_float)
}

/// dto.TomlConfig → configKey 值（`DtoToConfigMap` 逐键对照，77 键）。
/// 未知 configKey 返回 None = Go map 查不到 ⇒ 该参数跳过。
/// `TowerFirstLayerFlow` 的 ADR-007 零值替换（0→1.0）与 Go 同在此发生。
pub fn config_value(cfg: &TomlConfig, config_key: &str) -> Option<ConfigVal> {
    let t = &cfg.toolhead;
    let w = &cfg.wiping;
    Some(match config_key {
        // ---- toolhead（11 键）----
        "XOffset" => ConfigVal::F64(t.offset.x),
        "YOffset" => ConfigVal::F64(t.offset.y),
        "ZOffset" => ConfigVal::F64(t.offset.z),
        "MaxSpeed" => ConfigVal::F64(t.speed_limit),
        "MKPRetract" => ConfigVal::F64(t.mkp_retract),
        "FirstPenRevitalizationFlag" => ConfigVal::Bool(t.first_pen_revitalization_flag),
        "L803LeakPreventFlag" => ConfigVal::Bool(t.l803_leak_prevent_flag),
        "CustomMountGcode" => ConfigVal::Str(t.custom_mount_gcode.clone()),
        "CustomUnmountGcode" => ConfigVal::Str(t.custom_unmount_gcode.clone()),
        "GlueZOffset" => ConfigVal::F64(t.glue_z_offset),
        "PrimeLength" => ConfigVal::F64(t.prime_length),
        // ---- wiping（66 键）----
        "UseWipingTowers" => ConfigVal::Str(w.have_wiping_components.clone()),
        "WiperX" => ConfigVal::F64(w.wiper_x),
        "WiperY" => ConfigVal::F64(w.wiper_y),
        "WipeTowerPrintSpeed" => ConfigVal::F64(w.wipetower_speed),
        "TowerExtrudeRatio" => ConfigVal::F64(w.tower_extrude_ratio),
        "NozzleCoolingFlag" => ConfigVal::Bool(w.nozzle_cooling_flag),
        "FanSpeed" => ConfigVal::F64(w.fan_speed),
        "UserDryTime" => ConfigVal::I64(w.user_dry_time),
        "SupportExtrusionMultiplier" => ConfigVal::F64(w.support_extrusion_multiplier),
        "SmallFeatureFactor" => ConfigVal::F64(w.small_feature_factor),
        "OuterStructure" => ConfigVal::Str(w.outer_structure.clone()),
        "SheathBaseExpand" => ConfigVal::F64(w.sheath_base_expand),
        "SheathWallWidth" => ConfigVal::F64(w.sheath_wall_width),
        "SheathEnableHeight" => ConfigVal::F64(w.sheath_enable_height),
        "SheathConvergeLayers" => ConfigVal::I64(w.sheath_converge_layers),
        "IroningESumThreshold" => ConfigVal::F64(w.ironing_e_sum_threshold),
        "IroningMode" => ConfigVal::Str(w.ironing_mode.clone()),
        "UseIroningPath" => ConfigVal::Bool(w.use_ironing_path),
        "IroningCoverageThreshold" => ConfigVal::F64(w.ironing_coverage_threshold),
        "IroningSuppressExpand" => ConfigVal::F64(w.ironing_suppress_expand),
        "IroningSuppressExpandMode" => ConfigVal::Str(w.ironing_suppress_expand_mode.clone()),
        "PrimeTriggerLayers" => ConfigVal::I64(w.prime_trigger_layers),
        "PrimeEnabled" => ConfigVal::Bool(w.prime_enabled),
        "DiskStaggerMode" => ConfigVal::Str(w.disk_stagger_mode.clone()),
        "DiskStaggerAngle" => ConfigVal::F64(w.disk_stagger_angle),
        "DiskStaggerMaxAngle" => ConfigVal::F64(w.disk_stagger_max_angle),
        "DiskStaggerSwingMode" => ConfigVal::Str(w.disk_stagger_swing_mode.clone()),
        "GluePassCount" => ConfigVal::I64(w.glue_pass_count),
        "GlueZLiftHeight" => ConfigVal::F64(w.glue_z_lift_height),
        "GlueZLiftHeightFirstLayers" => ConfigVal::F64(w.glue_z_lift_height_first_layers),
        "GlueInterModelLiftHeight" => ConfigVal::F64(w.glue_inter_model_lift_height),
        "PrimeMinPathLength" => ConfigVal::F64(w.prime_min_path_length),
        "PrimePerModel" => ConfigVal::Bool(w.prime_per_model),
        "PrimeEveryLayer" => ConfigVal::Bool(w.prime_every_layer),
        "RibExtraLength" => ConfigVal::F64(w.rib_extra_length),
        "RibWidth" => ConfigVal::F64(w.rib_width),
        "RibFilletWall" => ConfigVal::Bool(w.rib_fillet_wall),
        "DisableFrontCoverAlarm" => ConfigVal::Bool(w.disable_front_cover_alarm),
        "Disable3rdLayerClogDetect" => ConfigVal::Bool(w.disable_3rd_layer_clog_detect),
        "DisableTimelapse" => ConfigVal::Bool(w.disable_timelapse),
        "XYCalibrationMode" => ConfigVal::Str(w.xy_calibration_mode.clone()),
        "ZCalibrationMode" => ConfigVal::Str(w.z_calibration_mode.clone()),
        "CalibrationExecutionMode" => ConfigVal::Str(w.calibration_execution_mode.clone()),
        "MinSupportInterfaceEnabled" => ConfigVal::Bool(w.min_support_interface_enabled),
        "GlueSparseRatio" => ConfigVal::F64(w.glue_sparse_ratio),
        "GlueZCompEnabled" => ConfigVal::Bool(w.glue_z_comp_enabled),
        "GlueZCompMode" => ConfigVal::Str(w.glue_z_comp_mode.clone()),
        "GlueZCompBedFL" => ConfigVal::F64(w.glue_z_comp_bed_fl),
        "GlueZCompBedFR" => ConfigVal::F64(w.glue_z_comp_bed_fr),
        "GlueZCompBedBL" => ConfigVal::F64(w.glue_z_comp_bed_bl),
        "GlueZCompBedBR" => ConfigVal::F64(w.glue_z_comp_bed_br),
        "GlueZCompPatchFL" => ConfigVal::F64(w.glue_z_comp_patch_fl),
        "GlueZCompPatchFR" => ConfigVal::F64(w.glue_z_comp_patch_fr),
        "GlueZCompPatchBL" => ConfigVal::F64(w.glue_z_comp_patch_bl),
        "GlueZCompPatchBR" => ConfigVal::F64(w.glue_z_comp_patch_br),
        // Go 侧经 FastTowerModeToString：string 输入原样返回（bool 分支 dto 不触达）
        "FastTowerMode" => ConfigVal::Str(w.fast_tower_mode.clone()),
        "TowerGlueLayerSpeed" => ConfigVal::F64(w.tower_glue_layer_speed),
        "TowerSheathSpeed" => ConfigVal::F64(w.tower_sheath_speed),
        "TowerRibSpeed" => ConfigVal::Str(w.tower_rib_speed.clone()),
        "TowerRibSpeedValue" => ConfigVal::F64(w.tower_rib_speed_value),
        "TowerCustomLayerHeight" => ConfigVal::Str(w.tower_custom_layer_height.clone()),
        "TowerWipeMode" => ConfigVal::Str(w.tower_wipe_mode.clone()),
        "TowerSafeZOffset" => ConfigVal::F64(w.tower_safe_z_offset),
        "TowerTravelSpeed" => ConfigVal::F64(w.tower_travel_speed),
        // ADR-007：0 视为未设置，进校验面的值是 registry 默认 1.0（Go 在 map 构造时替换）
        "TowerFirstLayerFlow" => ConfigVal::F64(if w.tower_first_layer_flow == 0.0 {
            1.0
        } else {
            w.tower_first_layer_flow
        }),
        "InterfaceIroningFlag" => ConfigVal::Bool(w.interface_ironing_flag),
        _ => return None,
    })
}

/// 第一层：`ValidateConfig`（config.go:54）——6 条硬检查，消息逐字。
pub fn validate_config(cfg: &TomlConfig) -> Result<(), PostprocError> {
    let t = &cfg.toolhead;
    let w = &cfg.wiping;
    // **这里只剩一条，而且它不是区间。**
    //
    // 它守的是「注册表零值豁免」的补集：`validate_registry_entry` 在 `f == 0.0` 时直接放过
    // （照抄 Go，本轮不改），所以 `speed_limit = 0` 只有这一条拦得住 ——
    // 而 0 速度会让后续管线得到无意义的进给。**这条里不许出现区间数字。**
    if t.speed_limit <= 0.0 {
        return Err(invalid(format!(
            "Toolhead.SpeedLimit must be > 0, got {}",
            go_float(t.speed_limit)
        )));
    }
    // 原来这里还有 5 条硬编码区间，spec `ranges-from-registry` 删掉了它们。
    // 删的理由是实测对照（doc §1.1）：**没有一条比注册表严**，而两道校验都会跑，
    // 所以生效的区间本来就是注册表那一份 —— 这 5 条要么重复、要么是更宽的死代码：
    //
    // | 键 | 原来写死的 | 注册表 | 关系 |
    // |---|---|---|---|
    // | `offset.x` | `\|x\| ≤ 50` | `-50..50` | 逐值相同 |
    // | `offset.y` | `\|y\| ≤ 50` | `-50..50` | 逐值相同 |
    // | `offset.z` | `\|z\| ≤ 20` | `0..20`   | 注册表更严（不许负） |
    // | `wiper_x`  | `-40..260`   | `10..226`（A1_MINI 上 `10..150`） | 注册表更严 |
    // | `wiper_y`  | `0..265`     | `10..226`（A1_MINI 上 `10..150`） | 注册表更严 |
    //
    // 区间数值现在只有一处真源：`param_registry.toml`。门禁
    // `scripts/check_hardcoded_ranges.py` 守着「它们不许长回来」。
    tracing::info!(
        "配置校验通过: SpeedLimit={}, Offset=({},{},{}), Wiper=({},{})",
        go_float(t.speed_limit),
        go_float(t.offset.x),
        go_float(t.offset.y),
        go_float(t.offset.z),
        go_float(w.wiper_x),
        go_float(w.wiper_y)
    );
    Ok(())
}

/// 第二层：`ValidateAgainstRegistry`（validate_registry.go:30）——按 registry
/// 参数序迭代（Go 遍历 reg.Params 数组，确定性序），Key/ConfigKey 空壳跳过，
/// dto 未映射的 configKey 跳过（不报错）。
///
/// **区间来自注册表的「有效区间」**（spec `ranges-from-registry`）：
/// `机型:变体` 的覆盖优先，没有覆盖才用全局 `min`/`max`。
/// `machine` 要是**规范名**（`load_ir` 第 1.5 步先算好），`variant` 是文件头 `# variant:` 的原值；
/// 两者任一拿不到都只是回落全局 —— 校验只会更宽，不会因为机型识别不出来而误拦。
pub fn validate_against_registry(
    cfg: &TomlConfig,
    reg: &Registry,
    machine: &str,
    variant: Option<&str>,
) -> Result<(), PostprocError> {
    for entry in &reg.params {
        if entry.param_key.is_empty() || entry.config_key.is_empty() {
            continue;
        }
        let Some(val) = config_value(cfg, &entry.config_key) else {
            continue;
        };
        let range = reg.lookup_range_for(&entry.config_key, machine, variant);
        validate_registry_entry(entry, &val, range)?;
    }
    tracing::info!(count = reg.params.len(), "参数注册表强校验通过");
    Ok(())
}

/// `validateRegistryEntry` 逐字（谓词细节见模块文档）。
///
/// `range` 是**有效区间**（按机型/变体覆盖优先）。传 `None` 表示这个参数没有量程 ⇒ 不校验。
/// 错误文案格式一个字没改（`param %s value %v is out of range [%s, %s]` 是对外契约），
/// 只是方括号里的数字现在可能来自机型覆盖。
fn validate_registry_entry(
    entry: &ParamEntry,
    val: &ConfigVal,
    range: Option<EffectiveRange>,
) -> Result<(), PostprocError> {
    if entry.value_type == "string" && !entry.choices.is_empty() {
        let vs = sprint(val);
        if vs.is_empty() {
            return Ok(()); // 零值=未设置，跳过
        }
        for c in &entry.choices {
            if choice_value_to_string(&c.value) == vs {
                return Ok(());
            }
        }
        let allowed: Vec<String> = entry
            .choices
            .iter()
            .map(|c| choice_value_to_string(&c.value))
            .collect();
        return Err(invalid(format!(
            "param {} value {:?} is not in allowed choices [{}]",
            entry.param_key,
            vs,
            allowed.join(", ")
        )));
    }

    // 区间一律来自有效区间（`range`），**不再读 entry.min/entry.max** ——
    // 那两个字段是「全局」那一档，机型覆盖优先级更高，两处各读一遍必然漂移。
    if entry.value_type == "int" || entry.value_type == "float" {
        let Some(r) = range else {
            return Ok(()); // 没有量程 ⇒ 不校验（与今天一致）
        };
        if r.min.is_none() && r.max.is_none() {
            return Ok(());
        }
        let Some(f) = as_f64(val) else {
            return Ok(()); // 非数值面不校验（Go asFloat64 !ok）
        };
        if f == 0.0 {
            return Ok(()); // 零值=未设置，跳过（Go 语义，本轮不改；补集由 validate_config 守）
        }
        if r.min.is_some_and(|m| f < m) || r.max.is_some_and(|m| f > m) {
            return Err(invalid(format!(
                "param {} value {} is out of range [{}, {}]",
                entry.param_key,
                go_float(f),
                format_bound(r.min),
                format_bound(r.max)
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::load_param_registry;

    /// 从 TOML 文本构造 config（serde 面 = engine 实际吃的同一解析路径）。
    fn cfg_from(extra_wiping: &str) -> TomlConfig {
        let text = format!(
            "[toolhead]\nspeed_limit = 200\noffset = {{ x = -1, y = 18.6, z = 4 }}\n\
             MKP_retract = 0\n\n[wiping]\nhave_wiping_components = \"tower\"\n\
             wiper_x = 20\nwiper_y = 140\nwipetower_speed = 80\n{extra_wiping}"
        );
        toml::from_str(&text).expect("测试预设应可解析")
    }

    fn msg(err: PostprocError) -> String {
        match err {
            PostprocError::InvalidConfig { message } => message,
            other => panic!("应为 InvalidConfig，实测 {other:?}"),
        }
    }

    // ---- 第一层 ValidateConfig：只剩「速度为正」一条（区间归注册表，spec `ranges-from-registry`）----

    #[test]
    fn validate_config_speed_limit_must_be_positive() {
        let mut cfg = cfg_from("");
        cfg.toolhead.speed_limit = 0.0;
        assert_eq!(
            msg(validate_config(&cfg).unwrap_err()),
            "Toolhead.SpeedLimit must be > 0, got 0"
        );
    }

    /// **K-R6：零值豁免的补集。**
    ///
    /// 注册表那道范围检查在 `f == 0.0` 时直接放过（Go 语义，本轮不改），
    /// 所以 `speed_limit = 0` 只有第一层拦得住 —— 这就是那条硬规则**留下来的全部理由**。
    /// 反过来 `wiper_x = 0` 今天是被放过的（豁免），删掉第一层的区间也不改变这一点。
    #[test]
    fn kr6_zero_exemption_complement_is_still_guarded() {
        let reg = load_param_registry();

        // 速度 0：第一层拦
        let mut zero_speed = cfg_from("");
        zero_speed.toolhead.speed_limit = 0.0;
        assert!(
            validate_config(&zero_speed).is_err(),
            "speed_limit = 0 必须被拒"
        );
        // 而第二层**放过它**（零值豁免）—— 这正是第一层不能删的证据
        validate_against_registry(&zero_speed, &reg, "A1", None)
            .expect("零值豁免：第二层放过 speed_limit = 0（Go 语义）");

        // wiper_x = 0：两层都放过（与删除硬编码之前一致）
        let mut zero_wiper = cfg_from("");
        zero_wiper.wiping.wiper_x = 0.0;
        validate_config(&zero_wiper).expect("wiper_x = 0 与今天一致：第一层不管");
        validate_against_registry(&zero_wiper, &reg, "A1", None).expect("零值豁免");
    }

    /// 原来第一层写死区间的那 5 个键，现在由**注册表**拦 —— 断言的是「拦得住」，换的是「谁拦的」。
    #[test]
    fn offset_and_wiper_are_now_rejected_by_the_registry() {
        let reg = load_param_registry();

        let mut cfg = cfg_from("");
        cfg.toolhead.offset.x = 51.0;
        assert_eq!(
            msg(validate_against_registry(&cfg, &reg, "A1", None).unwrap_err()),
            "param toolhead.offset.x value 51 is out of range [-50, 50]"
        );

        let mut cfg = cfg_from("");
        cfg.toolhead.offset.y = -50.5;
        assert_eq!(
            msg(validate_against_registry(&cfg, &reg, "A1", None).unwrap_err()),
            "param toolhead.offset.y value -50.5 is out of range [-50, 50]"
        );

        // offset.z：注册表比原来的硬编码**更严**（`0..20` vs `|z| ≤ 20`）——
        // 负值现在会被拒，这是收紧，照实断言。
        let mut cfg = cfg_from("");
        cfg.toolhead.offset.z = -0.1;
        assert_eq!(
            msg(validate_against_registry(&cfg, &reg, "A1", None).unwrap_err()),
            "param toolhead.offset.z value -0.1 is out of range [0, 20]"
        );

        let mut cfg = cfg_from("");
        cfg.wiping.wiper_x = 300.0;
        assert_eq!(
            msg(validate_against_registry(&cfg, &reg, "A1", None).unwrap_err()),
            "param wiping.wiper_x value 300 is out of range [10, 226]"
        );

        // **按机型收紧的那一条**：同样的 200 在 A1 上合法、在 A1_MINI 上越界（上限 150）
        let mut cfg = cfg_from("");
        cfg.wiping.wiper_x = 200.0;
        validate_against_registry(&cfg, &reg, "A1", Some("fast")).expect("A1 上 200 合法");
        assert_eq!(
            msg(validate_against_registry(&cfg, &reg, "A1_MINI", Some("fastv3.3")).unwrap_err()),
            "param wiping.wiper_x value 200 is out of range [10, 150]"
        );
    }

    /// 边界值本身必须通过（注册表是闭区间）。
    #[test]
    fn validate_config_boundary_values_pass() {
        let reg = load_param_registry();
        let mut cfg = cfg_from("");
        cfg.toolhead.offset.x = 50.0;
        cfg.toolhead.offset.z = 20.0;
        cfg.wiping.wiper_x = 226.0;
        cfg.wiping.wiper_y = 226.0;
        validate_config(&cfg).expect("第一层只看速度");
        validate_against_registry(&cfg, &reg, "A1", None).expect("边界值应通过");
    }

    // ---- 第二层 ValidateAgainstRegistry ----

    #[test]
    fn registry_rejects_bad_enum_with_exact_message() {
        let cfg = cfg_from("ironing_mode = \"xyz\"\n");
        let err = validate_against_registry(&cfg, &load_param_registry(), "A1", None).unwrap_err();
        assert_eq!(
            msg(err),
            "param wiping.ironing_mode value \"xyz\" is not in allowed choices [off, on, auto]"
        );
    }

    #[test]
    fn registry_rejects_out_of_range_mkpretract() {
        let mut cfg = cfg_from("");
        cfg.toolhead.mkp_retract = 60.0;
        let err = validate_against_registry(&cfg, &load_param_registry(), "A1", None).unwrap_err();
        assert_eq!(
            msg(err),
            "param toolhead.MKP_retract value 60 is out of range [-50, 50]"
        );
    }

    /// **bug-compatible 跳过（响亮登记，不是我们的 bug）**：registry 侧 10 个
    /// `GlueZCompensation*` 参数的 configKey（全拼）在 Go 的 DtoToConfigMap 里
    /// 是缩写形态（`GlueZCompBedFL`…）⇒ `values[entry.ConfigKey]` 查不到 ⇒
    /// **Go 的强校验从未真正校验过这 10 个参数**。本移植逐字复刻（arm 表按
    /// DtoToConfigMap 命名，查不到返回 None ⇒ 跳过）。若哪天旧侧修了键名漂移，
    /// 这里同步——不许先于旧侧「修好」。
    #[test]
    fn registry_skips_glue_z_compensation_family_like_go() {
        let cfg = cfg_from("glue_z_compensation_bed_fl = 0.3\n");
        assert_eq!(config_value(&cfg, "GlueZCompensationBedFL"), None);
        validate_against_registry(&cfg, &load_param_registry(), "A1", None)
            .expect("键名漂移 ⇒ Go 同样放行（bug-compatible）");
    }

    /// registry 范围可以比第一层紧：XOffset 合法于 [-50,50]（第一层过），
    /// registry 也是 [-50,50]，取 -60 双层都应拦——此处锁第二层独立承重。
    #[test]
    fn registry_rejects_offset_independent_of_layer_one() {
        let mut cfg = cfg_from("");
        cfg.toolhead.offset.x = -60.0;
        // 第一层先红（|X|>50）；把第一层挪开，直接打第二层
        let err = validate_against_registry(&cfg, &load_param_registry(), "A1", None).unwrap_err();
        assert_eq!(
            msg(err),
            "param toolhead.offset.x value -60 is out of range [-50, 50]"
        );
    }

    /// 零值豁免：XOffset=0（未设置）不得报错。
    #[test]
    fn registry_zero_values_are_exempt() {
        let mut cfg = cfg_from("");
        cfg.toolhead.offset.x = 0.0;
        cfg.wiping.glue_z_comp_bed_fl = 0.0;
        validate_against_registry(&cfg, &load_param_registry(), "A1", None).expect("零值应豁免");
    }

    /// bool 带 choices 刻意不校验（Go 谓词 string&&choices 的复刻）：
    /// FirstPenRevitalizationFlag=false 且 choices={off,on}——若错查枚举，
    /// 这里会因 "false" ∉ {off, on} 而误报。
    #[test]
    fn registry_bool_with_choices_is_not_validated() {
        let cfg = cfg_from("");
        let reg = load_param_registry();
        let entry = reg
            .params
            .iter()
            .find(|p| p.config_key == "FirstPenRevitalizationFlag")
            .expect("快照应有该参数");
        assert_eq!(entry.value_type, "bool");
        assert!(!entry.choices.is_empty(), "该参数应带 choices（词面证据）");
        let val = config_value(&cfg, "FirstPenRevitalizationFlag").unwrap();
        assert_eq!(sprint(&val), "false");
        assert_ne!(choice_value_to_string(&entry.choices[0].value), "false");
        validate_registry_entry(entry, &val, None).expect("bool 带 choices 不校验（Go 同）");
    }

    /// TowerFirstLayerFlow 的 ADR-007 替换在值面发生：0 进校验的是 1.0。
    #[test]
    fn tower_first_layer_flow_zero_maps_to_default_in_value_face() {
        let cfg = cfg_from("");
        assert_eq!(
            config_value(&cfg, "TowerFirstLayerFlow"),
            Some(ConfigVal::F64(1.0))
        );
    }

    /// 回归判据：9 份真实预设两层全过（不许为过校验改 fixture）。
    #[test]
    fn nine_real_presets_pass_both_layers() {
        let reg = load_param_registry();
        // 搬入 mkp-ssr 的路径改动：预设 fixture 在**内核那份**里
        // （crates/postprocess/tests/fixtures/presets），不在本 crate 下 —— 判据资产只留一份。
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../postprocess/tests/fixtures/presets");
        let mut count = 0;
        for entry in std::fs::read_dir(&dir).expect("fixtures 目录应存在") {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                continue;
            }
            let preset = crate::read_preset(&path).expect("真实预设应可读");
            validate_config(&preset.config)
                .unwrap_or_else(|e| panic!("第一层拦了 {}：{e:?}", path.display()));
            let canonical =
                postprocess::postproc::machine_dims::normalize_to_canonical(preset.machine.trim());
            validate_against_registry(&preset.config, &reg, &canonical, preset.variant.as_deref())
                .unwrap_or_else(|e| panic!("第二层拦了 {}：{e:?}", path.display()));
            count += 1;
        }
        assert_eq!(count, 9, "9 份真实预设必须全数参与");
    }
}
