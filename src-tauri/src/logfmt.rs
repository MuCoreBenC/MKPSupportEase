//! **写盘日志的字段拼装**（2026-11-03 从 `workbench/app/logging.rs` 提出来的那一半）。
//!
//! # 为什么住在这一层
//!
//! `presetdata` 是两档共用的数据层：工作台改定义写日志要它，客户端档**没有**工作台
//! 模块也要能编译。所以"一条改动长成一行人话"这件事只能落在既不属于工作台、
//! 也不属于客户端的中间层。工作台专属的那部分（`KeyChange` 的 `why`、超长截断策略）
//! 仍住在 `workbench/app/logging.rs`，它复用这里的 [`FieldChange`]。
//!
//! # 两种记录
//!
//! - [`FieldChange`]：结构类改动（改参数定义、改机型尺寸、换套餐成员）——
//!   没有"哪台机型哪一版"那一格，只说**字段**：`label: 涂胶速度 → 涂胶速度限制`。
//! - 值类改动（`A1:STANDARD · toolhead.offset.z: 3.9 → 3.95`）带"层"，形状在
//!   工作台那一侧，因为只有工作台有"层"这个概念。
//!
//! 两边的值印法必须一致（`None` = 跟随上一层、`null` = 空、字符串不带引号），
//! 所以 [`show`] 也只有这一份。

use serde_json::Value;

/// 值超过这个长度就截断（G-code 是主要来源）
pub const MAX_VALUE_CHARS: usize = 120;

/// 一条**改了什么**：字段名 + 旧值 + 新值。
#[derive(Debug, Clone, PartialEq)]
pub struct FieldChange {
    /// 字段名，用文件里的写法（`label` / `bedSize.y` / `members`）
    pub field: String,
    pub before: Option<Value>,
    pub after: Option<Value>,
}

impl FieldChange {
    /// 一行人话：`label: 涂胶速度 → 涂胶速度限制`
    pub fn render(&self) -> String {
        format!(
            "{}: {} → {}",
            self.field,
            show(&self.before),
            show(&self.after)
        )
    }
}

/// 日志里的值怎么印：`None` 是「跟随上一层」，`null` 是「空」，字符串不带引号。
///
/// 两件事必须分得开：前者是"这一层没有这一行"，后者是"这一行的值是空" ——
/// 混成一句的话，"删键"和"写成 null"在日志里就一样了。
pub fn show(v: &Option<Value>) -> String {
    let text = match v {
        None => return "（跟随上一层）".to_owned(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) => return "（空）".to_owned(),
        Some(other) => other.to_string(),
    };
    truncate(&text)
}

/// 超长值截断：留开头 + 标总长，避免 G-code 把日志刷爆
pub fn truncate(text: &str) -> String {
    if text.chars().count() <= MAX_VALUE_CHARS {
        return text.to_owned();
    }
    let head: String = text.chars().take(MAX_VALUE_CHARS).collect();
    format!("{head}…（共 {} 字符）", text.chars().count())
}

/// 两个值不一样时才算一条改动
pub fn note<T: ToString>(out: &mut Vec<FieldChange>, field: &str, before: T, after: T) {
    let b = before.to_string();
    let a = after.to_string();
    if b != a {
        out.push(FieldChange {
            field: field.to_owned(),
            before: Some(Value::String(b)),
            after: Some(Value::String(a)),
        });
    }
}

/// 两个 `Option` 不一样时才算一条改动（`None` 印成「（跟随上一层）」）
pub fn note_opt(
    out: &mut Vec<FieldChange>,
    field: &str,
    before: Option<Value>,
    after: Option<Value>,
) {
    if before != after {
        out.push(FieldChange {
            field: field.to_owned(),
            before,
            after,
        });
    }
}

/// 布尔格（`true` / `false` 是值不是字符串，别经过 [`note`]）
pub fn note_bool(out: &mut Vec<FieldChange>, field: &str, before: bool, after: bool) {
    if before != after {
        out.push(FieldChange {
            field: field.to_owned(),
            before: Some(Value::Bool(before)),
            after: Some(Value::Bool(after)),
        });
    }
}

/// 一批字段变化拼成一行 `fields=...`。
///
/// 上限 [`MAX_ITEMS`]：改一张整尺寸表能出二十来条，全摊开会把同一次点击的其它行
/// 挤出屏幕。
pub const MAX_ITEMS: usize = 32;

pub fn render(fields: &[FieldChange]) -> String {
    if fields.is_empty() {
        return "（无字段差异）".to_owned();
    }
    if fields.len() > MAX_ITEMS {
        return format!("（{} 处，超过 {MAX_ITEMS} 不逐条展开）", fields.len());
    }
    fields
        .iter()
        .map(FieldChange::render)
        .collect::<Vec<_>>()
        .join("；")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// 「这一层没有这一行」与「这一行的值是空」不许混成一句
    #[test]
    fn values_are_printed_distinctly() {
        assert_eq!(show(&Some(json!(3.9))), "3.9");
        assert_eq!(show(&Some(json!("wipe_tower"))), "wipe_tower");
        assert_eq!(show(&None), "（跟随上一层）");
        assert_eq!(show(&Some(json!(null))), "（空）");
    }

    /// 长值截断但保留总长 —— 找问题要的是"它变了"，不是全文
    #[test]
    fn long_values_are_truncated_with_a_length_note() {
        let gcode = "G1 X0 Y0\n".repeat(60);
        let shown = show(&Some(json!(gcode.clone())));
        assert!(shown.starts_with("G1 X0 Y0"));
        assert!(shown.contains(&format!("共 {} 字符", gcode.chars().count())));
    }

    /// 只列真的变了的；没变的一条都不许出现
    #[test]
    fn only_real_changes_are_listed() {
        let mut out = Vec::new();
        note(&mut out, "label", "涂胶速度", "涂胶速度限制");
        note(&mut out, "desc", "同一句", "同一句");
        note_bool(&mut out, "deprecated", false, true);
        assert_eq!(
            out.iter().map(|c| c.field.as_str()).collect::<Vec<_>>(),
            ["label", "deprecated"]
        );
        assert_eq!(out[0].render(), "label: 涂胶速度 → 涂胶速度限制");
        assert_eq!(
            render(&out),
            "label: 涂胶速度 → 涂胶速度限制；deprecated: false → true"
        );
    }

    /// 条数过多只报个数
    #[test]
    fn too_many_fields_are_summarized() {
        let mut out = Vec::new();
        for i in 0..MAX_ITEMS + 1 {
            note(&mut out, &format!("f{i}"), "a", "b");
        }
        assert!(render(&out).contains("超过"));
        assert!(render(&[]).contains("无"));
    }
}
