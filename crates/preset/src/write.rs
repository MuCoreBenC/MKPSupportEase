//! 预设 TOML 的**写值**面。
//!
//! # 三条硬规矩（违反其一就是 bug，判据逐条守）
//!
//! 1. **只改已存在的键的值** —— 不新增、不删除。`model.rs` 全体带
//!    `deny_unknown_fields`，写出任何多余键，下次自己就读不回来（`read_preset` 直接失败）。
//! 2. **不碰任何 decor** —— 注释、空行、键序都是用户的东西。实测真实预设约
//!    **70 条注释 / 100 行**（5 个用户预设：整行 7 + 行尾 61~63），几乎每个键都带中文行尾注释。
//!    「读成结构体再序列化回去」会把它们全抹掉，所以这里用 `toml_edit` 的
//!    `DocumentMut` 就地换值，而不是 `toml::to_string`。
//! 3. **输出必须能被 `read_preset_from_bytes` + `load_ir` 读回** —— 这一步由调用方
//!    （`src-tauri` 的 `preset_apply`）执行，本模块只保证不越权改动。
//!
//! 第 1 条**只对用户的预设成立**。`recipe_edit::set_override` 改的是我们自己的配方
//! （`preset_recipes.toml`），那边**允许新增一行** —— 给新机型补覆盖值本来就得新增。
//! 两个模块的立场相反是有意的，判据也不同；别把一边的结论搬到另一边。
//!
//! # 形态不许漂移
//!
//! 同样的值有多种合法写法，写回时**保持用户原来那种**，否则 diff 里会出现一堆
//! 与用户无关的改动：
//!
//! - `offset = { x = -0.5, y = 24, z = 4 }` 是 **inline table**：只能进它内部改，
//!   整块替换会吞掉行尾的 `# 笔尖偏移`；
//! - `custom_mount_gcode = """…"""` 是**多行字面量**：写回仍用 `"""`，
//!   不塌成带 `\n` 的单行串（语义等价，但文件会面目全非）；
//! - `speed_limit = 70` 是**整数**：不写成 `70.0`（`tower_wipe_speed` 更是
//!   `IntOrFloat` 双形态，写错形态连类型都变了）。

use postprocess::diag::PostprocError;
use toml_edit::{DocumentMut, Formatted, Item, TableLike, Value};

/// 一个编辑值。**已经按 `valueType` 解析好**：解析与量程校验是调用方（表单层）的事，
/// 这里只负责「原样写进去且不破坏形态」。
#[derive(Debug, Clone, PartialEq)]
pub enum EditValue {
    Float(f64),
    Int(i64),
    Bool(bool),
    Str(String),
}

/// 一次编辑：`section` + `key` 定位。
///
/// `key` 允许两段式（`offset.x`）—— 那是 inline table 里的字段，全预设只有 `offset` 一个。
#[derive(Debug, Clone, PartialEq)]
pub struct Edit {
    pub section: String,
    pub key: String,
    pub value: EditValue,
}

impl Edit {
    pub fn new(section: &str, key: &str, value: EditValue) -> Self {
        Self {
            section: section.to_string(),
            key: key.to_string(),
            value,
        }
    }
}

fn invalid(message: String) -> PostprocError {
    PostprocError::InvalidConfig { message }
}

/// **可编辑面的真源**：预设 TOML 里程序认得的全部键。
///
/// 为什么清单在这里、而不是从注册表来：注册表是**外部资产**（会随云端更新变），
/// 而 `model.rs` 的 serde 键才是「程序到底能读回什么」的事实
/// （全体 `deny_unknown_fields`）。注册表只提供元数据（类型/量程/中文名）。
///
/// 计数：**77 个 serde 键 → 79 行**（`offset` 是 inline table，展成 x/y/z 三行）。
/// 判据 K-P6 用**双向差集**把这份清单与 `model.rs` 咬住：任一边加了字段都会红。
pub const EDITABLE_KEYS: &[(&str, &str)] = &[
    ("toolhead", "speed_limit"),
    ("toolhead", "offset.x"),
    ("toolhead", "offset.y"),
    ("toolhead", "offset.z"),
    ("toolhead", "MKP_retract"),
    ("toolhead", "custom_mount_gcode"),
    ("toolhead", "custom_unmount_gcode"),
    ("toolhead", "first_pen_revitalization_flag"),
    ("toolhead", "l803_leak_prevent_flag"),
    ("toolhead", "glue_z_offset"),
    ("toolhead", "prime_length"),
    ("wiping", "have_wiping_components"),
    ("wiping", "wiper_x"),
    ("wiping", "wiper_y"),
    ("wiping", "wipetower_speed"),
    ("wiping", "nozzle_cooling_flag"),
    ("wiping", "user_dry_time"),
    ("wiping", "support_extrusion_multiplier"),
    ("wiping", "tower_extrude_ratio"),
    ("wiping", "interface_ironing_flag"),
    ("wiping", "fan_speed"),
    ("wiping", "small_feature_factor"),
    ("wiping", "sheath_base_expand"),
    ("wiping", "sheath_wall_width"),
    ("wiping", "sheath_enable_height"),
    ("wiping", "sheath_converge_layers"),
    ("wiping", "ironing_e_sum_threshold"),
    ("wiping", "ironing_mode"),
    ("wiping", "use_ironing_path"),
    ("wiping", "ironing_coverage_threshold"),
    ("wiping", "ironing_suppress_expand"),
    ("wiping", "ironing_suppress_expand_mode"),
    ("wiping", "prime_trigger_layers"),
    ("wiping", "prime_enabled"),
    ("wiping", "disk_stagger_mode"),
    ("wiping", "disk_stagger_angle"),
    ("wiping", "disk_stagger_max_angle"),
    ("wiping", "disk_stagger_swing_mode"),
    ("wiping", "glue_pass_count"),
    ("wiping", "min_hop_distance"),
    ("wiping", "prime_min_path_length"),
    ("wiping", "prime_per_model"),
    ("wiping", "prime_every_layer"),
    ("wiping", "glue_z_lift_height"),
    ("wiping", "glue_z_lift_height_first_layers"),
    ("wiping", "glue_inter_model_lift_height"),
    ("wiping", "outer_structure"),
    ("wiping", "rib_extra_length"),
    ("wiping", "rib_width"),
    ("wiping", "rib_fillet_wall"),
    ("wiping", "disable_front_cover_alarm"),
    ("wiping", "disable_3rd_layer_clog_detect"),
    ("wiping", "disable_timelapse"),
    ("wiping", "xy_calibration_mode"),
    ("wiping", "z_calibration_mode"),
    ("wiping", "calibration_execution_mode"),
    ("wiping", "min_support_interface_enabled"),
    ("wiping", "glue_sparse_ratio"),
    ("wiping", "glue_z_compensation_enabled"),
    ("wiping", "glue_z_compensation_mode"),
    ("wiping", "glue_z_compensation_bed_fl"),
    ("wiping", "glue_z_compensation_bed_fr"),
    ("wiping", "glue_z_compensation_bed_bl"),
    ("wiping", "glue_z_compensation_bed_br"),
    ("wiping", "glue_z_compensation_patch_fl"),
    ("wiping", "glue_z_compensation_patch_fr"),
    ("wiping", "glue_z_compensation_patch_bl"),
    ("wiping", "glue_z_compensation_patch_br"),
    ("wiping", "fast_tower_mode"),
    ("wiping", "tower_glue_layer_speed"),
    ("wiping", "tower_wipe_speed"),
    ("wiping", "tower_sheath_speed"),
    ("wiping", "tower_rib_speed"),
    ("wiping", "tower_rib_speed_value"),
    ("wiping", "tower_custom_layer_height"),
    ("wiping", "tower_wipe_mode"),
    ("wiping", "tower_safe_z_offset"),
    ("wiping", "tower_travel_speed"),
    ("wiping", "tower_first_layer_flow"),
];

/// 一个键在某份预设文件里的现状。
///
/// `present == false` 是**常态**而不是错误：7 个 `deprecated` 键在真实预设里根本不出现，
/// 而本模块**不许新增键**，所以它们只能是「文件里没有 ⇒ 不可编辑」。
#[derive(Debug, Clone, PartialEq)]
pub struct KeySnapshot {
    pub section: String,
    pub key: String,
    pub present: bool,
    /// 当前值，按文件里**实际写的形态**给（整数就是 `Int`，不擅自升 `Float`）。
    pub value: Option<EditValue>,
    /// 是否多行字面量（`"""…"""`）—— 表单据此选 gcode 控件。
    pub multiline: bool,
}

/// 读一份预设里全部可编辑键的现状。
///
/// 放在本 crate 而不是应用层：**预设文件的解析只许发生在这个 crate 里**
/// （依赖方向 preset → core 由 Cargo 守，「谁能读预设」这条今天靠这个约定守）。
pub fn snapshot(raw: &str) -> Result<Vec<KeySnapshot>, PostprocError> {
    let doc: DocumentMut = raw.parse().map_err(|e| PostprocError::TomlParse {
        path: String::new(),
        message: format!("预设不是合法 TOML：{e}"),
    })?;

    let mut out = Vec::with_capacity(EDITABLE_KEYS.len());
    for (section, key) in EDITABLE_KEYS {
        let found = doc
            .get(section)
            .and_then(Item::as_table_like)
            .and_then(|t| match key.split_once('.') {
                Some((outer, inner)) => t
                    .get(outer)
                    .and_then(Item::as_table_like)
                    .and_then(|nested| nested.get(inner)),
                None => t.get(key),
            })
            .and_then(Item::as_value);

        let (present, value, multiline) = match found {
            Some(v) => {
                let multiline =
                    v.as_str().is_some() && v.to_string().trim_start().starts_with("\"\"\"");
                (true, Some(to_edit_value(v)), multiline)
            }
            None => (false, None, false),
        };
        out.push(KeySnapshot {
            section: (*section).to_string(),
            key: (*key).to_string(),
            present,
            value,
            multiline,
        });
    }
    Ok(out)
}

fn to_edit_value(v: &Value) -> EditValue {
    match v {
        Value::Integer(i) => EditValue::Int(*i.value()),
        Value::Float(f) => EditValue::Float(*f.value()),
        Value::Boolean(b) => EditValue::Bool(*b.value()),
        Value::String(s) => EditValue::Str(s.value().clone()),
        // 日期/数组/inline table 不在可编辑面里（EDITABLE_KEYS 全是标量）。
        // 真出现了就原样带成字符串，让表单标「不可编辑」，而不是静默丢掉这一行。
        other => EditValue::Str(other.to_string().trim().to_string()),
    }
}

/// 数值的**形态**跟谁走。
///
/// 两个调用方要的是两件事，混成一件必然有一方是错的：
/// - [`NumberShape::FollowOld`]（编辑器）：用户在一个浮点字段里敲 `24`，
///   写出去仍然该是 `24.0`（或原来的形态）—— 改一个值不该顺手改文件的写法。
/// - [`NumberShape::FollowNew`]（生成器）：目标预设里那一项**本来就是** `y = 24`（整数），
///   而模板里是 `y = 18.6`（浮点）。这时形态必须跟新值，否则「与现有预设逐字节相同」
///   永远差一个 `.0`（实测就是这么红的：`24` vs `24.0`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberShape {
    /// 形态跟随原值（编辑器）
    FollowOld,
    /// 形态跟随新值（生成器）
    FollowNew,
}

/// 就地改值，返回新文本。**零编辑时返回与输入逐字节相同的字符串**（判据 K-P1）。
///
/// 数值形态跟随**原值** —— 编辑器的语义。生成器要的是 [`apply_edits_exact`]。
pub fn apply_edits(raw: &str, edits: &[Edit]) -> Result<String, PostprocError> {
    apply_edits_with(raw, edits, NumberShape::FollowOld)
}

/// 就地改值，数值形态跟随**新值** —— 生成器的语义（见 [`NumberShape`]）。
pub fn apply_edits_exact(raw: &str, edits: &[Edit]) -> Result<String, PostprocError> {
    apply_edits_with(raw, edits, NumberShape::FollowNew)
}

fn apply_edits_with(
    raw: &str,
    edits: &[Edit],
    shape: NumberShape,
) -> Result<String, PostprocError> {
    let mut doc: DocumentMut = raw.parse().map_err(|e| PostprocError::TomlParse {
        path: String::new(),
        message: format!("预设不是合法 TOML，拒绝写回：{e}"),
    })?;

    for edit in edits {
        let section = doc
            .get_mut(&edit.section)
            .and_then(Item::as_table_like_mut)
            .ok_or_else(|| {
                invalid(format!(
                    "预设里没有 [{}] 段，拒绝创建（编辑器只改已有的键）",
                    edit.section
                ))
            })?;

        match edit.key.split_once('.') {
            Some((outer, inner)) => {
                let nested = section
                    .get_mut(outer)
                    .and_then(Item::as_table_like_mut)
                    .ok_or_else(|| {
                        invalid(format!(
                            "[{}] 里没有 `{outer}`，或者它不是一张表，拒绝写 `{}`",
                            edit.section, edit.key
                        ))
                    })?;
                set_value(nested, inner, &edit.value, &edit.section, &edit.key, shape)?;
            }
            None => set_value(
                section,
                &edit.key,
                &edit.value,
                &edit.section,
                &edit.key,
                shape,
            )?,
        }
    }

    Ok(doc.to_string())
}

fn set_value(
    table: &mut dyn TableLike,
    key: &str,
    value: &EditValue,
    section: &str,
    full_key: &str,
    shape: NumberShape,
) -> Result<(), PostprocError> {
    let slot = table.get_mut(key).ok_or_else(|| {
        invalid(format!(
            "[{section}] 里没有 `{full_key}` 这个键，拒绝新增 —— \
             预设的读面是 deny_unknown_fields，多一个键下次就读不回来了"
        ))
    })?;
    let old = slot
        .as_value()
        .ok_or_else(|| invalid(format!("[{section}] 的 `{full_key}` 不是标量，拒绝改")))?;

    let mut next = match value {
        EditValue::Float(f) => {
            if !f.is_finite() {
                return Err(invalid(format!(
                    "`{full_key}` 收到 {f}（NaN/Inf）—— TOML 表达不了，写出去也读不回来"
                )));
            }
            // 形态跟随原值（编辑器）：原来是整数、新值又恰好是整数 ⇒ 仍写整数。
            // 生成器要的是跟随新值 —— 见 `NumberShape`。
            if shape == NumberShape::FollowOld
                && matches!(old, Value::Integer(_))
                && f.fract() == 0.0
            {
                Value::Integer(Formatted::new(*f as i64))
            } else {
                Value::Float(Formatted::new(*f))
            }
        }
        EditValue::Int(i) => {
            if shape == NumberShape::FollowOld && matches!(old, Value::Float(_)) {
                Value::Float(Formatted::new(*i as f64))
            } else {
                Value::Integer(Formatted::new(*i))
            }
        }
        EditValue::Bool(b) => Value::Boolean(Formatted::new(*b)),
        EditValue::Str(s) => string_value(old, s, full_key)?,
    };

    // decor（前导空白 + 行尾注释）从旧值原样搬过来 —— 这是「不碰注释」的落点。
    *next.decor_mut() = old.decor().clone();
    *slot = Item::Value(next);
    Ok(())
}

/// 字符串写回。原来是多行字面量就仍写多行字面量。
fn string_value(old: &Value, s: &str, full_key: &str) -> Result<Value, PostprocError> {
    // 判「原来是不是多行字面量」只能看 repr（`as_str()` 拿到的是解码后的内容，
    // 单行与多行长得一样）。repr 私有，所以用 `to_string()` 看它渲染出来的开头。
    let was_multiline =
        old.as_str().is_some() && old.to_string().trim_start().starts_with("\"\"\"");

    if !was_multiline {
        if s.contains('\n') {
            return Err(invalid(format!(
                "`{full_key}` 原本是单行字符串，收到的值里有换行 —— \
                 改变字符串形态不在这一轮的范围内"
            )));
        }
        return Ok(Value::String(Formatted::new(s.to_string())));
    }

    if s.contains("\"\"\"") {
        return Err(invalid(format!(
            "`{full_key}` 是多行 G-code 串，值里出现了 `\"\"\"` —— \
             多行字面量装不下它，拒绝写（不做转义降级：那会改变文件形态）"
        )));
    }
    // `"""` 后紧跟的换行按 TOML 规范会被丢弃，所以开头补一个换行，
    // 结尾保证有换行 —— 这样 round-trip 回来的字符串与写进去的一致。
    let body = if s.ends_with('\n') {
        s.to_string()
    } else {
        format!("{s}\n")
    };
    // **让解析器造这个值**，而不是自己设 repr：`Formatted::set_repr_unchecked` 与
    // `Repr::new_unchecked` 在 toml_edit 0.22 里是 crate 私有的（实测编译报
    // 「method `set_repr_unchecked` is private」）。解析一段一行的 TOML 再把值取出来，
    // 用的全是公开 API，且形态由解析器保证合法。
    let snippet = format!("v = \"\"\"\n{body}\"\"\"\n");
    let doc: DocumentMut = snippet.parse().map_err(|e| {
        invalid(format!(
            "`{full_key}` 的新值拼不成合法的多行字面量：{e}（值里有控制字符？）"
        ))
    })?;
    let v = doc
        .get("v")
        .and_then(Item::as_value)
        .ok_or_else(|| invalid(format!("`{full_key}` 的多行字面量构造失败")))?
        .clone();
    Ok(v)
}
