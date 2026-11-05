//! 参数台 v2 的只读数据面（2026-11-02，`docs/WORKBENCH-SHEET-V2-PROTOCOL.md`）。
//!
//! # 与旧 `wb_matrix` 的唯一区别
//!
//! **这里不返回「当前值」。**
//!
//! 旧 `Cell` 同时带着 `raw`（现在是多少）、`rest`（改之前是多少）、`dirty`（改没改）。
//! 三者挤在一个结构里，于是任何一处口径漂移都会变成「徽章乱跳」——
//! 作者连报三次、换了两种修法都没解决，根子就在这里。
//!
//! 新页只拿到 [`SheetCell::saved`]（保存值）+ [`SheetCell::saved_pinned`]（那一层钉没钉），
//! 「现在显示什么」由前端算 `draft ?? saved`。**结构上没有第二个"当前值"字段可填**，
//! 所以不可能再出现"值和保存值其实是同一份数据"那种事。
//!
//! # 为什么不另算一遍三层
//!
//! 层运算的唯一实现是 `presetdata::resolve::effective_of`，它已经被
//! [`Book::matrix`] 用到了（`Book` 拿 `with_rest(&ctx.base)` 之后，`Cell.rest`
//! 就是 BASE 的有效值、`Cell.rest_pinned` 就是 BASE 这一层钉没钉）。
//! 这里**只做投影**，不重算 —— 两份实现迟早会不一致，那正是旧页的病。
//!
//! # 显示文本为什么必须在这里拼
//!
//! `Cell.text` 是**当前值**（编辑树那份）的格式化结果，不是保存值的 ——
//! 2026-11-02 这一版先把 `saved_text` 错接到它上面过：编辑一格之后徽章上写的
//! 会变成"刚打进去的新值"，看着像"没改过"。所以保存值的文本由这里按
//! `wording::value_text_for` **重新格式化 `rest`** 得到，源码只有一个。

use serde::Serialize;
use serde_json::Value;

use crate::error::AppError;
use crate::presetdata::registry::Choice;
use crate::presetdata::{Level, Origin, UiComponent, ValueType};
use crate::workbench::domain::derive::ColRef;
use crate::workbench::domain::patch::variant_key;
use crate::workbench::domain::wording as w;

use super::Ctx;

/// 一列：机型基底（`version_uid` 为空）或某个版本。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetCol {
    pub key: String,
    pub machine_id: String,
    pub version_uid: Option<String>,
    /// 写值进哪一层。**前端拿它拼 `Patch::setValue`** —— 让前端从
    /// `version_uid === null` 反推是允许的，但那样每多一处推断就多一次弄反的机会，
    /// 而弄反 = 静默打破继承（`derive.rs` 的 `Col::level` 是同一句注释）。
    pub level: Level,
    pub label: String,
}

/// 一行的定义。**只有定义，没有值** —— 值在 [`SheetCell`]。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetDef {
    pub key: String,
    pub label: String,
    pub desc: String,
    pub unit: Option<String>,
    pub section_id: String,
    pub section_label: String,
    pub tab_id: Option<String>,
    pub depth: u8,
    pub parent_key: Option<String>,
    pub parent_label: Option<String>,
    /// 行头那句「受「X」控制」。**与当前值无关**，格子变灰之前就已经在那儿
    pub control_note: Option<String>,
    pub gcode: bool,
    /// 全局弃用（所有机型）。**这一列弃没弃用看 [`SheetCell::deprecated`]** ——
    /// 弃用可以只对某些机型生效，拿这一格判会把「A1 上退场」说成「全都退场」
    pub deprecated: bool,
    pub value_type: ValueType,
    pub ui_component: UiComponent,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: Option<f64>,
    /// 出厂默认的显示文本（后端格式化）。右栏「出厂值」那一格读它
    pub default_text: String,
    pub choices: Vec<Choice>,
}

/// 一格。**只有保存态**，没有「当前值」。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetCell {
    pub key: String,
    /// `A1`（机型基底列）或 `A1/STANDARD`（版本列）
    pub col_key: String,
    /// 保存值 = BASE 在这一格的有效值（版本覆盖 → 机型基底 → 出厂）。
    /// `None` 只有一种情况：这一项在这台机型上不适用 / 已弃用 ⇒ **不可写**
    pub saved: Option<Value>,
    /// BASE 里**这一层**自己钉着吗。
    ///
    /// 只决定「还原」怎么写（协议 §3）：钉着 ⇒ 写回 `saved`；没钉 ⇒ 删键挂回继承。
    /// **不要**拿它判脏 —— 归并会把同值上提到机型基底，那一刻"这一层没钉"而"值确实变了"。
    pub saved_pinned: bool,
    /// 这一格挂哪档徽章：**跟不跟基底**（`follow` / `own`，基底列是 `null`）。
    ///
    /// 与 `saved_pinned` 不同源：`saved_pinned` 读 BASE（盘上有没有这一行），
    /// 这一项读**编辑树** —— 刚写完就该看见「不跟随」，不必等保存。
    pub badge: Option<crate::workbench::domain::derive::ParamStatus>,
    /// 保存值的显示文本（后端格式化；前端不再拼字符串 —— 拼错会像视频里那样落成 `0`）
    pub saved_text: String,
    /// 保存值是从哪一层来的（出厂 → 机型基底 → 版本覆盖）。
    ///
    /// **只回答"哪张表里写着它"**，不是第二次算值 —— 顺序与
    /// `derive::Book::base_effective_at` 一致，两处都只有三分支。
    pub saved_origin: Origin,
    pub editable: bool,
    /// 这一列这台机型上**已弃用**（全局与点名弃用合起来判）
    pub deprecated: bool,
    /// 这一列这台机型上**不许再选**的那几档（含推定）
    pub dead_choices: Vec<String>,
    /// 点开灰格子时的**整句**（能跳的说「由「X」控制，需 Y」）
    pub blocked_note: Option<String>,
    /// 行上那一句**短提示**（「要 X 才可改」）。与 `blocked_note` 同源不同场合
    pub blocked_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParamSheet {
    pub cols: Vec<SheetCol>,
    pub defs: Vec<SheetDef>,
    pub cells: Vec<SheetCell>,
    pub total_rows: usize,
    pub note: Option<String>,
    pub empty_reason: Option<String>,
}

/// 保存值是**从哪一层**来的：出厂 → 机型基底 → 版本覆盖，与
/// `derive::Book::base_effective_at` 同一顺序。
///
/// 只回答"哪张表里写着它"，不重算值 —— 值只有 `effective_of` 一个实现。
fn saved_origin_of(ctx: &Ctx, col: &crate::workbench::domain::derive::Col, key: &str) -> Origin {
    let Some(p) = ctx.base.registry.param(key) else {
        return Origin::Factory;
    };
    if col.level == Level::Version
        && p.machine_variants
            .contains_key(&variant_key(Level::Version, &col.key))
    {
        return Origin::Version;
    }
    if p.machine_variants
        .contains_key(&variant_key(Level::Machine, &col.machine_id))
    {
        return Origin::Machine;
    }
    Origin::Factory
}

/// 参数台 v2 的整屏数据。**只读**：不写盘、不动会话状态。
pub(super) fn build(
    ctx: &Ctx,
    cols: &[ColRef],
    tab: Option<&str>,
    query: &str,
    base_machine: Option<&str>,
) -> ParamSheet {
    /* ★ `book_of` 里带着 `with_rest(&ctx.base)` —— 这一屏的全部要害：
    `Cell.rest` 与 `Cell.rest_pinned` 从它取。漏掉它，`rest` 会退回编辑树，
    于是「保存值」永远等于「当前值」。**别再手写 `Book::new`**（见 `book_of` 的注释）。 */
    let m = super::book_of(ctx).matrix(cols, tab, query, base_machine);

    let sheet_cols: Vec<SheetCol> = m
        .cols
        .iter()
        .map(|c| SheetCol {
            key: c.key.clone(),
            machine_id: c.machine_id.clone(),
            version_uid: c.version_uid.clone(),
            level: c.level,
            label: c.label.clone(),
        })
        .collect();

    let mut defs = Vec::with_capacity(m.rows.len());
    let mut cells = Vec::with_capacity(m.rows.len() * m.cols.len());

    for row in &m.rows {
        let Some(p) = ctx.presets.registry.param(&row.key) else {
            continue;
        };
        defs.push(SheetDef {
            key: row.key.clone(),
            label: row.label.clone(),
            desc: row.desc.clone(),
            unit: row.unit.clone(),
            section_id: row.section_id.clone(),
            section_label: row.section_label.clone(),
            tab_id: row.tab_id.clone(),
            depth: row.depth,
            parent_key: row.parent_key.clone(),
            parent_label: row.parent_label.clone(),
            control_note: row.control_note.clone(),
            gcode: row.gcode,
            deprecated: row.deprecated,
            value_type: p.value_type,
            ui_component: p.ui_component,
            min: p.min,
            max: p.max,
            step: p.step,
            default_text: w::value_text(p, &p.default_value),
            choices: p.choices.clone(),
        });

        for (i, cell) in row.cells.iter().enumerate() {
            let Some(col) = m.cols.get(i) else { continue };
            /* 保存值的显示文本：**从 `rest` 重新格式化**，不能用 `cell.text`
            （那是当前值）。见文件头「显示文本为什么必须在这里拼」。 */
            let saved_text = match (&cell.rest, col) {
                (Some(v), _) if w::is_gcode(p) => w::gcode_text(v).1,
                (Some(v), _) => w::value_text_for(p, v, &col.machine_id),
                (None, _) => String::new(),
            };
            cells.push(SheetCell {
                key: row.key.clone(),
                col_key: col.key.clone(),
                saved: cell.rest.clone(),
                saved_pinned: cell.rest_pinned,
                badge: cell.badge,
                saved_text,
                saved_origin: saved_origin_of(ctx, col, &row.key),
                editable: cell.editable,
                deprecated: cell.deprecated,
                dead_choices: cell.dead_choices.clone(),
                blocked_note: cell.blocked_note.clone(),
                blocked_hint: cell.blocked_hint.clone(),
            });
        }
    }

    ParamSheet {
        cols: sheet_cols,
        defs,
        cells,
        total_rows: m.total_rows,
        note: m.note,
        empty_reason: m.empty_reason,
    }
}

/// `wb_param_sheet` 的本体（抽出来是为了可测）。
pub(super) fn sheet_in(
    ctx: &Ctx,
    cols: Vec<ColRef>,
    tab: Option<String>,
    query: Option<String>,
    base_machine: Option<String>,
) -> Result<ParamSheet, AppError> {
    if cols.is_empty() {
        return Err(AppError::invalid_argument("参数台至少要看一列"));
    }
    Ok(build(
        ctx,
        &cols,
        tab.as_deref(),
        query.as_deref().unwrap_or_default(),
        base_machine.as_deref(),
    ))
}
