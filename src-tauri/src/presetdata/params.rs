//! 参数值的**只读抽取**与**结构保真批量写回** —— 对比台与「恢复默认」共用的一份算法。
//!
//! # 为什么要有这一层
//!
//! 参数页走的是「草稿」那条链（`run/draft-preset.json` 里的整份正文 + 三层取值）。
//! 对比台要的是另一件事：**直接从一份具体的预设文件里把参数读出来**（读 2~3 份做对照），
//! 以及**把改过的几项写回它自己**。前者不该经过草稿，后者必须与参数页同一套保真纪律。
//!
//! 所以这里只有两个纯函数：抽值（[`read_param_values`]）与批量改值
//! （[`apply_param_edits`]，逐项转调 [`super::patch::patch_preset_toml`]）。
//! **一切写盘都不在这里**（落点闸、原子写在 IPC 那一层），这一层只算文本。
//!
//! # 值的形态
//!
//! 抽出来的值是**参数控件看得懂的字符串**（`-1.5` / `true` / `standard` / 多行 G-code），
//! 不是 TOML 字面量（不带引号、不带 `"""`）—— 前端控件直接吃，不需要懂 TOML。
//! 形态由注册表的 `valueType` 决定（与写回那条路同一张表，所以读与写永远对得上）。
//!
//! # 只认标量
//!
//! 表 / 数组 / 认不出的形态**跳过**（不返回、也不许改）：一个参数对应一个标量字段，
//! 这是 `patch.rs` 那一侧就定下的边界。

use postprocess::diag::messages;

use std::collections::BTreeMap;

use toml_edit::{DocumentMut, Item};

use crate::error::AppError;

use super::patch::{patch_preset_toml, FieldEdit};
use super::registry::{ParamDef, ValueType};

/// 从一份预设正文里按注册表抽出**这份文件里真的有**的那批参数值。
///
/// 没有那一段 / 没有那个键的字段**不出现在结果里**（"这份预设不含这个参数"与
/// "这个参数是空值"是两件事，界面要分得开）。
pub fn read_param_values(
    raw: &str,
    defs: &[ParamDef],
) -> Result<BTreeMap<String, String>, AppError> {
    let doc: DocumentMut = raw.parse().map_err(|e| {
        AppError::corrupted(messages::text("ui.presetdata.params.282", &[]))
            .with_detail(format!("{e}"))
    })?;
    let mut out = BTreeMap::new();
    for def in defs {
        let Some(table) = doc.get(&def.section).and_then(Item::as_table_like) else {
            continue;
        };
        let Some(item) = table.get(&def.toml_key) else {
            continue;
        };
        if let Some(text) = value_text(item, def.value_type) {
            out.insert(def.key.clone(), text);
        }
    }
    Ok(out)
}

/// 两份正文之间的**参数级差异**：后者（草稿）现在写着、与前者（来源）不一样的那几项。
///
/// 判据是**值**（与 [`read_param_values`] 同一套形态），不是字节 —— 注释、空行、
/// 键序、血统那三行都不进账。参数页那条链用它把"盘上草稿改了什么"交回给刚冷启动的界面
/// （内存里的草稿层随进程没了，见 `ipc::local::read_preset_draft_changes`）。
///
/// 前者没有、后者有的键**算差异**（`patch_preset_toml` 会补写缺的键）；
/// 两份都没有的键自然不在结果里。
pub fn diff_param_values(
    before: &str,
    after: &str,
    defs: &[ParamDef],
) -> Result<BTreeMap<String, String>, AppError> {
    let base = read_param_values(before, defs)?;
    let now = read_param_values(after, defs)?;
    Ok(now
        .into_iter()
        .filter(|(key, value)| base.get(key) != Some(value))
        .collect())
}

/// **相对基线少了哪些参数**（2026-10-21，作者：「我改成了这样子，它都没有提示我」）。
///
/// `want` = 基线（通常是 `baseline-local/` 的原文镜像），`got` = 现在这份正文。
/// 判据是**参数值表**，不是字节 —— 所以"被粘进上一行注释里""整行删掉"都算丢。
///
/// 真机踩过：`z_calibration_mode = "new"` 被粘到 `xy_calibration_mode` 的行尾注释后面，
/// TOML **仍然合法**、语法闸一声不吭，但这一项从参数表里消失了 ——
/// 切片时会静默退回默认值，用户完全无感。
///
/// 返回 `(key, label)`，按 `defs` 的顺序（就是界面上参数页的顺序）。
pub fn missing_param_keys(
    want: &str,
    got: &str,
    defs: &[ParamDef],
) -> Result<Vec<(String, String)>, AppError> {
    let base = read_param_values(want, defs)?;
    let now = read_param_values(got, defs)?;
    Ok(defs
        .iter()
        .filter(|d| base.contains_key(&d.key) && !now.contains_key(&d.key))
        .map(|d| (d.key.clone(), d.label.clone()))
        .collect())
}

/// 一批改动**一次算完**：逐项结构保真地改，**任何一项失败就整批不落地**
/// （返回 `Err`，调用方手里的旧正文还是好的）。
pub fn apply_param_edits(
    raw: &str,
    defs: &[ParamDef],
    edits: &[FieldEdit],
) -> Result<String, AppError> {
    let mut out = raw.to_owned();
    for edit in edits {
        out = patch_preset_toml(&out, defs, edit)?;
    }
    Ok(out)
}

/// 标量 → 参数控件看得懂的字符串。表 / 数组 / 认不出的形态 ⇒ `None`（这一项跳过）。
fn value_text(item: &Item, ty: ValueType) -> Option<String> {
    let v = item.as_value()?;
    match ty {
        ValueType::Bool => v.as_bool().map(|b| b.to_string()),
        /* 整数写成整数，浮点写成浮点；数值本身照最短往返写法（`4.0` 显示成 `4`） */
        ValueType::Int => v
            .as_integer()
            .map(|i| i.to_string())
            .or_else(|| v.as_float().map(number_text)),
        ValueType::Float => v
            .as_float()
            .map(number_text)
            .or_else(|| v.as_integer().map(|i| i.to_string())),
        /* 文本与 G-code 都是字符串：给的是**内容**，不带引号 */
        ValueType::Text => v.as_str().map(|s| s.to_owned()),
    }
}

fn number_text(f: f64) -> String {
    if f.fract() == 0.0 && f.abs() < 1e15 {
        format!("{}", f as i64)
    } else {
        format!("{f}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presetdata::registry::ParamRegistry;

    /// 真注册表（`<repo>/presets/registry/param_registry.toml`）：`param_key` →
    /// `(section, toml_key)` 那张映射正是这里最要紧的东西，手写夹具等于自己出题
    fn reg() -> ParamRegistry {
        let root = crate::presetdata::repo_presets_root().expect("仓库预设根（测试专用）");
        ParamRegistry::load_from(&root).expect("读字段定义")
    }

    const DOC: &str = "\
# MKP 预设
# release_time: 2026-10-15 09:00:00

[toolhead]
speed_limit = 70 # 速度上限(mm/s)
offset_x = -1.5
first_pen_revitalization_flag = false
custom_mount_gcode = \"\"\"
G1 X0
G1 Y0
\"\"\"

[wiping]
mode = \"tower\"
";

    /// 抽出来的值是**控件看得懂**的形态：浮点、布尔、字符串内容、多行文本
    #[test]
    fn values_come_out_in_control_shape() {
        let got = read_param_values(DOC, reg().params()).expect("抽值");
        assert_eq!(
            got.get("toolhead.speed_limit").map(String::as_str),
            Some("70")
        );
        assert_eq!(
            got.get("toolhead.offset.x").map(String::as_str),
            Some("-1.5")
        );
        assert_eq!(
            got.get("toolhead.first_pen_revitalization_flag")
                .map(String::as_str),
            Some("false")
        );
        assert!(
            got.get("toolhead.custom_mount_gcode")
                .is_some_and(|v| v.contains("G1 X0") && v.contains("G1 Y0")),
            "多行文本给的是内容，不是带引号的字面量"
        );
    }

    /// 这份文件里没有的字段**不出现在结果里**（"不含这个参数" ≠ "这个参数是空值"）
    #[test]
    fn fields_this_file_does_not_have_are_absent() {
        let got = read_param_values(DOC, reg().params()).expect("抽值");
        assert!(
            !got.contains_key("toolhead.offset.z"),
            "这份文件没有 offset_z，就不该出现在结果里：{got:?}"
        );
    }

    /// 批量改值：**保真**（注释 / 键序 / 别的行不动），且一次算完
    #[test]
    fn a_batch_of_edits_is_applied_faithfully() {
        let out = apply_param_edits(
            DOC,
            reg().params(),
            &[
                FieldEdit::new("toolhead.offset.x", "-2.25"),
                FieldEdit::new("toolhead.speed_limit", "80"),
            ],
        )
        .expect("批量改");
        assert!(out.contains("offset_x = -2.25"), "\n{out}");
        /* `speed_limit` 在注册表里是浮点 ⇒ 渲染成 `80.0`；要紧的是**值换了、注释留着** */
        assert!(out.contains("speed_limit = 80"), "值要换\n{out}");
        assert!(out.contains("# 速度上限(mm/s)"), "注释要留着\n{out}");
        assert!(
            out.contains("# release_time: 2026-10-15 09:00:00"),
            "头注释不动\n{out}"
        );
        /* 只有那两行变了 */
        let changed: Vec<(&str, &str)> = DOC
            .lines()
            .zip(out.lines())
            .filter(|(a, b)| a != b)
            .collect();
        assert_eq!(changed.len(), 2, "只该有两行变化，实际：{changed:?}");
    }

    /// 差异只认**值**：改过的那几项回来，没动的不回来；注释 / 空行不同不算改动
    #[test]
    fn only_params_whose_value_changed_come_back() {
        let edited = DOC
            .replace("offset_x = -1.5", "offset_x = -2.25")
            .replace("# 速度上限(mm/s)", "# 速度上限（改个注释，值没动）");
        let got = diff_param_values(DOC, &edited, reg().params()).expect("算差异");
        assert_eq!(
            got.get("toolhead.offset.x").map(String::as_str),
            Some("-2.25"),
            "{got:?}"
        );
        assert!(
            !got.contains_key("toolhead.speed_limit"),
            "值没动（只换了注释）就不该算改动：{got:?}"
        );
        assert_eq!(got.len(), 1, "只该有一项：{got:?}");
    }

    /// **少了参数要说得出**（2026-10-21）：真机那次是 `z_calibration_mode` 被粘进了
    /// `xy_calibration_mode` 的行尾注释 —— TOML 仍然合法，只有按参数表比才看得见。
    #[test]
    fn params_swallowed_by_a_comment_are_reported_as_missing() {
        let want = DOC.replace("offset_x = -1.5", "offset_x = -1.5\noffset_z = 4");

        /* 整行被删 */
        let lost = missing_param_keys(&want, DOC, reg().params()).expect("算缺项");
        assert!(
            lost.iter().any(|(k, _)| k == "toolhead.offset.z"),
            "整行不见了要报：{lost:?}"
        );

        /* 被 `#` 吃掉（真机那种粘行） */
        let glued = want.replace("offset_z = 4", "# 笔尖偏移 offset_z = 4");
        let lost = missing_param_keys(&want, &glued, reg().params()).expect("算缺项");
        assert!(
            lost.iter().any(|(k, _)| k == "toolhead.offset.z"),
            "被注释掉也要报：{lost:?}"
        );

        /* 同一份：一个都不缺 */
        assert!(missing_param_keys(&want, &want, reg().params())
            .unwrap()
            .is_empty());
    }

    /// 原稿没有、草稿补上的键也算（`patch_preset_toml` 会补写缺的键）
    #[test]
    fn a_key_the_source_lacks_counts_as_changed() {
        let with_z = DOC.replace("offset_x = -1.5", "offset_x = -1.5\noffset_z = 4");
        let got = diff_param_values(DOC, &with_z, reg().params()).expect("算差异");
        assert_eq!(
            got.get("toolhead.offset.z").map(String::as_str),
            Some("4"),
            "{got:?}"
        );
    }

    /// 任何一项失败 → 整批不落地（调用方手里的旧正文还是好的）
    #[test]
    fn one_bad_edit_fails_the_whole_batch() {
        let edits = [
            FieldEdit::new("toolhead.offset.x", "-2"),
            FieldEdit::new("toolhead.speed_limit", "快"),
        ];
        assert!(apply_param_edits(DOC, reg().params(), &edits).is_err());
    }
}
