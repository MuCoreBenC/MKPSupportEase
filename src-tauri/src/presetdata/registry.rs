//! 字段定义与界面布局 —— `presets/registry/param_registry.toml` + `presets/layout_schema.toml`。
//!
//! **这一层是字段定义的 SSOT。** b04 Task 9 之前它住在 `upstream/registry.rs`
//! （读 `content/*.json`，也就是别人的构建产物）；那两个 JSON 本来就是从这两个 TOML
//! 构建出来的，所以这次搬家是**纯移动 + 换 loader**，模型一个字段都没改。
//!
//! # 两个文件互补，不是副本
//!
//! | 面 | 文件 | 承载什么 |
//! |---|---|---|
//! | 分组元数据 | `param_registry.toml` 的 `[[tabs]]` | **中文名、排序、图标**（`layout_schema` 全文没有这三样） |
//! | 参数摆放 | `layout_schema.toml` 的 `tabs[].sections[].items[]` | 哪个参数落在哪个 section、**section 级可见性与自定义组件** |
//! | 参数自带归属 | `[params.layout]` | `{order, sectionId}`，参数自己声明它属于哪个 section |
//!
//! 所以「参数 → section」这件事**有两处声明**（`params[].layout.sectionId` 与
//! `items[].paramKey`），实测两处一致，但**数据本身没有任何门禁保证它们不漂移** ——
//! 这就是 [`ParamRegistry::check_consistency`] 存在的理由。
//!
//! 两个文件由**同一个** [`ParamRegistry`] 持有，而不是各建一个结构：
//! 「参数 → section」的一致性要同时看两边才查得出来，分成两个所有者就没有任何时刻
//! 能把它们放在一起查。
//!
//! # 排序以 `layout.order` 为准，不用 `items[].id`
//!
//! `item.id` 形如 `item_0_0_1`，末位数字**不是顺序**：`space_offset` 的实际序列是
//! `item_0_0_1`(offset.x) → `item_0_0_0`(offset.y) → `item_0_0_2`(offset.z)。
//! 拿它当顺序会把 X 和 Y 调个头，而且调错了界面上看不出来。
//!
//! # 27 个 section 里只有 16 个装参数
//!
//! `internal` 一个 section 都没有，`settings` 下 11 个全是 `component` 占位（那是客户端
//! 设置页，不是配方）。矩阵按 27 个建分类会多出 12 个永远为空的页签 —— 见
//! [`ParamRegistry::param_tabs`]。
//!
//! # 同一份文本解析两遍，这是刻意的
//!
//! - `DocumentMut`：保原文与排版，**写回靠它**
//! - serde 反序列化：拿到好用的模型，**读靠它**
//!
//! 56 KB 解析两遍的代价可以忽略，换来的是"改一个字段不会重排整个文件"。

use postprocess::diag::messages;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use toml_edit::DocumentMut;

use crate::error::AppError;
use crate::fsx::atomic::atomic_write;

/* ---------- 值与控件 ---------- */

/// `valueType`。实测分布 float 43 / string 15 / bool 12 / int 4
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValueType {
    Float,
    Int,
    Bool,
    /// 数据里写的是 `"string"`；这里叫 `Text` 只为躲开 `String` 这个名字
    #[serde(rename = "string")]
    Text,
}

/// `uiComponent`。实测源里是五种；第六种 `text` 是**编辑器给的** ——
/// 字符串参数清掉可选项之后总得有个诚实的落点（自由文本），不能逼人挂着
/// 分段控件配空表
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UiComponent {
    Number,
    Switch,
    Segmented,
    Select,
    Gcode,
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Universal,
    MachineSpecific,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VariantMode {
    Shared,
    PerVariant,
}

/// 可见性条件。**实测形状恒定**：三键、无嵌套、无 and/or 数组
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowWhen {
    pub key: String,
    pub op: ShowOp,
    /// **类型混杂**：有 `true`（bool）、`"off"`（string），也有字符串化的数字 `"0"`。
    /// 比较前要按被指向字段的 `valueType` 归一化 —— 见 `domain::visibility`
    pub value: Value,
}

/// 实测只有三种：`eq` 38 / `neq` 3 / `gt` 2
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShowOp {
    Eq,
    Neq,
    Gt,
}

/// 枚举项。**`deprecated` 是选项级的** —— 参数本身没废弃、某个选项废弃了，
/// 只读参数级标记会漏掉这种
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Choice {
    pub label: String,
    /// 英文显示名（2026-10-26 双语注册表）。空 = 这条还没译，界面回落 `label`
    #[serde(default)]
    pub label_en: String,
    /// 日文显示名（2026-10-31 四册）。空 = 回落中文
    #[serde(default)]
    pub label_ja: String,
    /// 繁中显示名（2026-10-31 四册）。空 = 回落中文
    #[serde(default)]
    pub label_tw: String,
    pub value: Value,
    /// 全局弃用：所有机型、所有版本都不许再写这一档
    #[serde(default)]
    pub deprecated: bool,
    /// **只在这些机型上**弃用。空 = 不按机型弃用（与 `machineFilter` 同一口径）。
    /// 与 `deprecated` 是**叠加**关系，不是二选一：全局那一勾压过这份清单
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "comma_separated"
    )]
    pub deprecated_machines: Vec<String>,
}

impl Choice {
    /// 这一档在这台机型上还能不能写。`deprecated_machines` 为空 = 只看全局那一勾
    pub fn deprecated_for(&self, machine_id: &str) -> bool {
        self.deprecated || self.deprecated_machines.iter().any(|m| m == machine_id)
    }
}

/// 参数自己声明的界面归属
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParamLayout {
    /// 可以是小数（实测有 `0.5`）
    pub order: f64,
    pub section_id: String,
}

/// 一个参数的完整定义。
///
/// 刻意**不读** `mergeable` / `selectable`：实测 74/74 全为 true，零区分度。
/// 读进来再不给它们任何行为，就是在界面上摆一个永远为真的字段 —— 那是假的。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParamDef {
    /// 全局唯一主键，形如 `toolhead.offset.x`
    pub key: String,
    /// Go/TS 侧配置结构体的字段名
    pub config_key: String,
    /// 写进 `presets/mkp/*.toml` 的键名
    pub toml_key: String,
    pub json_key: String,
    pub label: String,
    /// 英文显示名（2026-10-26 双语注册表）。空 = 回落 `label`
    #[serde(default)]
    pub label_en: String,
    /// 日文显示名（2026-10-31 四册）。空 = 回落中文
    #[serde(default)]
    pub label_ja: String,
    /// 繁中显示名（2026-10-31 四册）。空 = 回落中文
    #[serde(default)]
    pub label_tw: String,
    pub desc: String,
    /// 英文说明。空 = 回落 `desc`
    #[serde(default)]
    pub desc_en: String,
    /// 日文说明（2026-10-31 四册）。空 = 回落中文
    #[serde(default)]
    pub desc_ja: String,
    /// 繁中说明（2026-10-31 四册）。空 = 回落中文
    #[serde(default)]
    pub desc_tw: String,
    /// 生成 TOML 时写的行内注释
    pub toml_comment: String,
    pub value_type: ValueType,
    pub ui_component: UiComponent,
    /// **出厂默认，单值**（74/74 都有）。三层里最底那一层就是它
    pub default_value: Value,
    pub scope: Scope,
    /// 数据域分区（= key 前缀，`wiping` / `toolhead`）。**不是界面分组**
    pub section: String,
    pub layout: ParamLayout,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub step: Option<f64>,
    #[serde(default)]
    pub unit: Option<String>,
    /// 父参数（层级折叠用）。与 `show_when` 数量相同但语义独立
    #[serde(default)]
    pub parent_key: Option<String>,
    #[serde(default)]
    pub show_when: Option<ShowWhen>,
    #[serde(default)]
    pub choices: Vec<Choice>,
    #[serde(default)]
    pub variant_mode: Option<VariantMode>,
    /// 源数据里是**逗号分隔字符串**（`"A1,A1_MINI,A2L"`），这里拆成数组。
    /// 空 = 不限机型
    #[serde(default, deserialize_with = "comma_separated")]
    pub machine_filter: Vec<String>,
    /// 实测 7 条为 true，**没有 false 占位**，所以缺省即 false。
    /// **全局弃用**：所有机型、所有版本都不许再写
    #[serde(default)]
    pub deprecated: bool,
    /// **只在这些机型上**弃用（空 = 不按机型弃用）。与 `deprecated` 叠加：
    /// `deprecated = true` 时这份清单没有额外含义，但不清空它 —— 撤掉全局那一勾，
    /// 原来点名的机型还得继续弃用
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "comma_separated"
    )]
    pub deprecated_machines: Vec<String>,
    /// 按机型/版本的**值**覆盖。键有两种形态：`"P1S"`（纯机型）与 `"P1S:LITE"`（版本）。
    /// 它**不属于出厂层** —— 由 `domain::variants` 按三步归并消化进机型基底与版本覆盖（doc §3.3）
    #[serde(default)]
    pub machine_variants: BTreeMap<String, Value>,

    /// **这一版跟不跟基底**（2026-11-03）。与 `machine_variants` 并列、互不推导。
    ///
    /// 键只有一种形状：`"A1:STANDARD"`（见 [`super::follows::uid_of`]）。
    /// **表里没有这个键 = 跟随**，只有 `false`（不跟随）会落盘 ——
    /// 跟随是绝大多数格子的常态，写进文件会凭空多出几百行。
    ///
    /// 为什么它必须单独存在：值与状态是两件事。写了 `3.85` 的版本，
    /// 可能是"跟着基底变过来的"，也可能是"自己定在 3.85" —— 靠值反推这一条
    /// 正是作者报的「显示不跟随、改基底却跟着变」。判据唯一实现是
    /// [`super::follows::is_following`]，徽章 / 写值 / 日志三处共用。
    ///
    /// ★ **不进 catalog**（2026-11-03）：交付目录与 `catalog.generated.json` 是
    /// **客户端**读的那一份，客户端只认值、不关心"这一行是不是影子"。把它序列化
    /// 出去会让每次改这条状态都动交付字节（闸要求发布才能提交），而客户端一个字节
    /// 也用不上。`skip_serializing` 只关出口，读（TOML / 旧 catalog）照旧。
    #[serde(default, skip_serializing)]
    pub follows: BTreeMap<String, bool>,

    /// 按机型/版本的**范围**覆盖。这两张表是字段定义的机型特化，**不是配方值**，
    /// 所以不能走 `machine_variants` 那套归并（doc §3.2）
    #[serde(default)]
    pub machine_min_variants: BTreeMap<String, Value>,
    #[serde(default)]
    pub machine_max_variants: BTreeMap<String, Value>,
    #[serde(default)]
    pub merge_group: Option<String>,
    #[serde(default)]
    pub pinned: bool,
    /// **该机型/变体没写值就整行不落盘**（2026-11-10）。
    ///
    /// 只对「某些机型用不到」的字段有意义：A 系列的落笔/收笔骨架只动 X，
    /// 等待位 Y 对它不适用。不标这个的话，出厂默认 `0` 会落进 TOML，界面上
    /// 就多出一格永远不该被改的 Y —— 而那是绝对坐标，改错就是把工具头甩到零点。
    /// 标了之后只有机型基底/版本**真的写了值**才落盘。
    #[serde(default)]
    pub omit_when_unset: bool,
    #[serde(default)]
    pub serialization: Option<Serialization>,
    /// **哪几册译文跟着这份中文一起过期了**（2026-11-02）。
    ///
    /// # 它记的是"事实"，不是"语气"
    ///
    /// 作者的口径：「我改了中文，但是其他没有改，这种能不能记录下来，让 AI 去改」——
    /// 界面上看不出「英文还是旧中文的译本」，只能靠人记；所以把它写成**数据**：
    /// `label` / `desc` 一改，剩下那三册的 `labelX` / `descX` 就登记在这里，
    /// 直到有人把译文重写了才摘掉。
    ///
    /// - 登记发生在 [`ParamDef::mark_translations_stale`]，调用点只有
    ///   [`ParamRegistry::set_param_meta`] 一处 —— 界面上没有第二个地方能改中文。
    /// - 客户端**不读它**（显示仍是 `labelX` 原样，缺了回落中文）。它是给工作台与
    ///   AI 看的待办：`scripts/stale-translations.mjs` 按它出一份清单。
    /// - 空 = 没问题。空数组**不落盘**（`skip_serializing_if`），所以老文件一个字节不变。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stale_translations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Serialization {
    pub subfields_order: Vec<String>,
}

impl ParamDef {
    /// 这个字段在这台机型上存不存在。
    ///
    /// `machine_filter` 为空 = 不限机型。**这是"不适用"的唯一判据** ——
    /// 与"被上级条件关着"是两件不同的事（前者根本没有这一项，后者看得见改不动）
    pub fn applies_to(&self, machine_id: &str) -> bool {
        self.machine_filter.is_empty() || self.machine_filter.iter().any(|m| m == machine_id)
    }

    /// 这条参数在这台机型上是不是「不许再写」。
    ///
    /// 两条路取或：**全局弃用**（所有机型）与**点名弃用**（只这些机型）。
    /// 与「不适用」是两件不同的事：`applies_to` 为假 = 这台机器根本没这一项，
    /// 这里为真 = 有这一项，但值只能读、不能再改
    pub fn deprecated_for(&self, machine_id: &str) -> bool {
        self.deprecated || self.deprecated_machines.iter().any(|m| m == machine_id)
    }

    /// 中文这几格变了之后，把**跟着一起过期**的译文登记进 `stale_translations`。
    ///
    /// `before` 是改之前那一份（同一份中文 + 那一份译文）。判据三条，缺一不可：
    ///
    /// 1. **中文真的变了** —— 没变就不该往台账上添一条（改个单位、挪个分组不算）。
    /// 2. **那一册有译文** —— 没译的册（空串）没有"过期"可言，它只是「还没译」；
    ///    旧译本换新中文之后**留着不删**（空串才是"删掉"），所以必须看一眼旧值：
    ///    旧值在中文变化的那一刻就成了"旧中文的译本"。
    /// 3. **译文本身这一笔没被重写** —— 同一次保存里既改了中文、又填了新译文，
    ///    那就是已经译好了，不该记一笔待办（作者：「我改了所有的都可以用公共的方法」）。
    ///
    /// 只登记、不清账：摘标记由 `set_param_meta` 里"这次真的重写了那一册"负责。
    pub fn mark_translations_stale(&mut self, before: &ParamDef) {
        let label_changed = self.label != before.label;
        let desc_changed = self.desc != before.desc;
        if !label_changed && !desc_changed {
            return;
        }
        for (changed, now, old, key_name) in [
            (label_changed, &self.label_en, &before.label_en, "labelEn"),
            (label_changed, &self.label_ja, &before.label_ja, "labelJa"),
            (label_changed, &self.label_tw, &before.label_tw, "labelTw"),
            (desc_changed, &self.desc_en, &before.desc_en, "descEn"),
            (desc_changed, &self.desc_ja, &before.desc_ja, "descJa"),
            (desc_changed, &self.desc_tw, &before.desc_tw, "descTw"),
        ] {
            // 中文没动这一格（或者译文这一笔被重写了）⇒ 这一册不欠账
            if !changed || now != old {
                continue;
            }
            if old.trim().is_empty() || self.stale_translations.iter().any(|x| x == key_name) {
                continue;
            }
            self.stale_translations.push(key_name.to_owned());
        }
    }

    /// **这一笔改了哪几格**（2026-11-02，给写盘日志用）。
    ///
    /// `ParamMetaEdit` 是整包提交（二十几格），逐格手写对比迟早漏一格，所以这里
    /// **扫全表**。只列真的变了的：定义框点「确认」但一个字没改时 `fields=` 是空的，
    /// 那一笔在日志里就长成"没有字段差异"，一眼能看出它不是一次改动。
    ///
    /// 键名用文件里的写法（`labelEn` / `staleTranslations`），与 `param_registry.toml`
    /// 对齐 —— 排查时可以直接照着去 `diff`。
    pub fn summary(&self, before: &ParamDef) -> Vec<crate::logfmt::FieldChange> {
        use crate::logfmt::{note, note_bool, note_opt, FieldChange};

        let mut out: Vec<FieldChange> = Vec::new();
        let s = |v: &str| v.to_owned();
        note(&mut out, "label", &before.label, &self.label);
        note(&mut out, "labelEn", &before.label_en, &self.label_en);
        note(&mut out, "labelJa", &before.label_ja, &self.label_ja);
        note(&mut out, "labelTw", &before.label_tw, &self.label_tw);
        note(&mut out, "desc", &before.desc, &self.desc);
        note(&mut out, "descEn", &before.desc_en, &self.desc_en);
        note(&mut out, "descJa", &before.desc_ja, &self.desc_ja);
        note(&mut out, "descTw", &before.desc_tw, &self.desc_tw);
        note(
            &mut out,
            "valueType",
            format!("{:?}", before.value_type),
            format!("{:?}", self.value_type),
        );
        note(
            &mut out,
            "uiComponent",
            format!("{:?}", before.ui_component),
            format!("{:?}", self.ui_component),
        );
        note(
            &mut out,
            "defaultValue",
            before.default_value.to_string(),
            self.default_value.to_string(),
        );
        note_opt(
            &mut out,
            "unit",
            json_opt(&before.unit),
            json_opt(&self.unit),
        );
        note_opt(&mut out, "min", num_opt(before.min), num_opt(self.min));
        note_opt(&mut out, "max", num_opt(before.max), num_opt(self.max));
        note_opt(&mut out, "step", num_opt(before.step), num_opt(self.step));
        note_opt(
            &mut out,
            "parentKey",
            json_opt(&before.parent_key),
            json_opt(&self.parent_key),
        );
        note_opt(
            &mut out,
            "showWhen",
            before
                .show_when
                .as_ref()
                .map(|w| Value::String(format!("{} {} {}", w.key, show_op_text(w.op), w.value))),
            self.show_when
                .as_ref()
                .map(|w| Value::String(format!("{} {} {}", w.key, show_op_text(w.op), w.value))),
        );
        note_bool(&mut out, "deprecated", before.deprecated, self.deprecated);
        note(
            &mut out,
            "deprecatedMachines",
            before.deprecated_machines.join(","),
            self.deprecated_machines.join(","),
        );
        note(
            &mut out,
            "machineFilter",
            before.machine_filter.join(","),
            self.machine_filter.join(","),
        );
        note(
            &mut out,
            "choices",
            choice_digest(&before.choices),
            choice_digest(&self.choices),
        );
        /* 分组与组内顺序住在**另一份文件**（`layout_schema.toml`），但它们是
        定义框同一笔提交的一部分 —— 日志里不分文件，只分字段 */
        note(
            &mut out,
            "sectionId",
            s(&before.layout.section_id),
            s(&self.layout.section_id),
        );
        note(
            &mut out,
            "order",
            before.layout.order.to_string(),
            self.layout.order.to_string(),
        );
        note(
            &mut out,
            "staleTranslations",
            before.stale_translations.join(","),
            self.stale_translations.join(","),
        );
        out
    }
}

/// 可空字符串 → 日志里的 `Option<Value>`（`None` = 这一格没填）
fn json_opt(v: &Option<String>) -> Option<Value> {
    v.as_ref().map(|s| Value::String(s.clone()))
}

/// 可空数字同上（`3.0` 与 `3` 印成同一个样子，别因为格式差异报假改动）
fn num_opt(v: Option<f64>) -> Option<Value> {
    v.map(|n| {
        if n.fract() == 0.0 && n.abs() < 1e15 {
            Value::String(format!("{}", n as i64))
        } else {
            Value::String(n.to_string())
        }
    })
}

/// [`ShowOp`] 的人话（日志里写 `eq` 比 `Eq` 好读）
pub fn show_op_text(op: ShowOp) -> &'static str {
    match op {
        ShowOp::Eq => "eq",
        ShowOp::Neq => "neq",
        ShowOp::Gt => "gt",
    }
}

/// 可选项整表压成一行，`foo=1|bar=2`；顺带带上弃用标记才有区分度
fn choice_digest(choices: &[Choice]) -> String {
    choices
        .iter()
        .map(|c| {
            let mut one = format!("{}={}", c.label, c.value);
            if c.deprecated {
                one.push_str("(停用)");
            }
            one
        })
        .collect::<Vec<_>>()
        .join("|")
}

/// 「保存这一趟顺手新建一个分组」—— 定义框分组下拉里每个页签右边那个「＋」。
///
/// # 为什么跟着定义一起交，而不是先建一个空组
///
/// 建了又取消的话，数据里会留下一个**谁也删不掉**的空组（这里没有删分组的入口）。
/// 所以它和参数定义是**同一笔**：`set_param_meta` 一次性把分组与参数的位置都写进
/// 两份文件；取消 = 一个字节都没动。
///
/// # id 由前端给
///
/// 撤销 / 重做会把同一笔载荷**原样重放**，那时这个分组已经在了 —— 所以下游的语义
/// 是「确保这个分组在」（见 [`ParamRegistry::ensure_section`]），不是「再造一个」。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewSection {
    /// 新分组的 id（`param_registry` 与 `layout_schema` 两侧共用；只许 a-z / 0-9 / `_`）
    pub id: String,
    /// 挂在哪个页签下（页签必须已经在注册表里）
    pub tab_id: String,
    /// 中文显示名（必填；另外三册留空 = 回落中文，与分组元数据同一口径）
    pub label: String,
}

/// 同一页签里往前 / 往后挪一格。**位置是数据**，所以由后端挪、前端只说方向
/// （参数那条路是前端算 `order` 中点，两边不一样：分组的显示顺序在**数组**里）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SectionMove {
    Up,
    Down,
}

/// 一次「分组」编辑的载荷 —— `wb_set_section_meta`。`None` = 这一格不动。
///
/// 改的是 `param_registry.toml` 的 `[[tabs.sections]]`：名字落在 `label`，
/// 位置落在**数组次序**（客户端 `tabsOf` 按数组排，不读 `order`）与 `order`
/// （工作台卡片按它排）两处 —— 见 [`ParamRegistry::set_section_meta`]。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionMetaEdit {
    /// 新名字（`None` = 不动）。**只写中文那一册**，另外三册留在原处不动
    #[serde(default)]
    pub label: Option<String>,
    /// 往哪边挪一格（`None` = 不动）
    #[serde(default, rename = "move")]
    pub move_dir: Option<SectionMove>,
    /// **顺手新建这一组**（`None` = 不建）。分组抽屉那颗「＋ 新增分组」用它。
    ///
    /// 为什么挂在 `SectionMetaEdit` 上而不是另开一条命令：新建与改名/换位是
    /// **同一类结构编辑**，同一套事务、同一套撤销、同一套"只进草稿"。另开一条命令
    /// 会多出一种「建了但还没命名」的中间态，数据里也得为它留个位置。
    ///
    /// 语义与参数定义框那条 `ParamMetaEdit::new_section` 一致：**幂等** ——
    /// 撤销 / 重做把同一笔载荷原样重放时，这一组已经在，按「确保它在」处理。
    #[serde(default)]
    pub new_section: Option<NewSection>,
}

/// 一次「参数定义」编辑的载荷 —— `wb_set_param_meta` 整包提交（模态框一次
/// 保存改的可能不止一格）。`None` 的意思是**这一格清空**（文件里删键），不是「不动」。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParamMetaEdit {
    pub label: String,
    /// 英文显示名（2026-10-26 双语注册表）。**`None` = 不动**（工作台的参数编辑器
    /// 还没编这一格，它也读不到这一格）；`Some("")` = 清掉这条的英文。
    #[serde(default)]
    pub label_en: Option<String>,
    pub desc: String,
    /// 英文说明，语义同上。
    #[serde(default)]
    pub desc_en: Option<String>,
    /// 日 / 繁中两册，语义同 `label_en`（`None` = 不动）。2026-10-31 四册化。
    #[serde(default)]
    pub label_ja: Option<String>,
    #[serde(default)]
    pub label_tw: Option<String>,
    #[serde(default)]
    pub desc_ja: Option<String>,
    #[serde(default)]
    pub desc_tw: Option<String>,
    pub unit: Option<String>,
    pub value_type: ValueType,
    pub ui_component: UiComponent,
    pub default_value: Value,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: Option<f64>,
    pub parent_key: Option<String>,
    pub show_when: Option<ShowWhen>,
    pub deprecated: bool,
    /// **只在这些机型上弃用**（与 `deprecated` 叠加，空 = 不按机型弃用）
    #[serde(default)]
    pub deprecated_machines: Vec<String>,
    /// 可选项整表（`[[params.choices]]`）。字符串枚举在模态框里编辑；
    /// 其他类型身上挂的几条（bool 的开/关叫法、数字的预设档）界面不给编辑、
    /// 原样带回 —— 这里不区分，整表照写
    #[serde(default)]
    pub choices: Vec<Choice>,
    /// 适用机型。空 = 不限机型（与 [`ParamDef::machine_filter`] 同一口径）
    #[serde(default)]
    pub machine_filter: Vec<String>,
    /// 搬到哪个分组（`layout_schema.toml` 的 section id）。
    /// `None` = 不动 —— 旧载荷没有这一格时，不能把它读成「搬回某个默认组」。
    /// 带 `new_section` 时这一格被忽略（目标是那个新分组）
    #[serde(default)]
    pub section_id: Option<String>,
    /// 这一笔顺手新建的分组（`None` = 不建）。见 [`NewSection`]
    #[serde(default)]
    pub new_section: Option<NewSection>,
    /// **组内顺序**（`[params.layout].order`）。`None` = 不动。
    ///
    /// 界面上是「第 n / m 位」那对上下箭头；客户端按同一个值排同一组里的参数，
    /// 所以它是数据，不是界面状态。搬组时不给就落到目标组末尾
    #[serde(default)]
    pub order: Option<f64>,
}

/* ---------- 分组元数据（中文名与顺序的唯一权威） ---------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionMeta {
    pub id: String,
    pub label: String,
    /// 英文显示名（空 = 回落 `label`）
    #[serde(default)]
    pub label_en: String,
    /// 日文显示名（2026-10-31 四册，空 = 回落中文）
    #[serde(default)]
    pub label_ja: String,
    /// 繁中显示名（2026-10-31 四册，空 = 回落中文）
    #[serde(default)]
    pub label_tw: String,
    #[serde(default)]
    pub description: Option<String>,
    /// 英文说明（空 = 回落 `description`）
    #[serde(default)]
    pub description_en: Option<String>,
    /// 日文说明（2026-10-31 四册，空 = 回落中文）
    #[serde(default)]
    pub description_ja: Option<String>,
    /// 繁中说明（2026-10-31 四册，空 = 回落中文）
    #[serde(default)]
    pub description_tw: Option<String>,
    pub order: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TabMeta {
    pub id: String,
    pub label: String,
    /// 英文显示名（空 = 回落 `label`）
    #[serde(default)]
    pub label_en: String,
    /// 日文显示名（空 = 回落中文）
    #[serde(default)]
    pub label_ja: String,
    /// 繁中显示名（空 = 回落中文）
    #[serde(default)]
    pub label_tw: String,
    pub order: f64,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub sections: Vec<SectionMeta>,
}

/// `param_registry.toml` 的反序列化目标
///
/// `pub(crate)` + `Serialize`（2026-11-03）：[`super::normalize`] 要拿它做
/// **无损检查** —— 重排前后各读一遍、逐字段比。给它单独写一套解析就等于
/// 在仓里立了第二个真值，比出来的"一致"毫无意义。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegistryFile {
    #[serde(default)]
    pub(crate) updated: String,
    #[serde(default)]
    pub(crate) tabs: Vec<TabMeta>,
    #[serde(default)]
    pub(crate) params: Vec<ParamDef>,
}

impl RegistryFile {
    /// 从**文本**解析（不从盘上读）。规范化那一侧要用它比对两份字节。
    ///
    /// 与 [`Registry::load_from`] 共用 `toml_edit::de` 这一条路 —— 两边读法不一致的
    /// 话，"数据一样"这个结论就只对我们自己的读法成立。
    pub(crate) fn parse_str(text: &str) -> Result<Self, AppError> {
        let mut file: Self = toml_edit::de::from_str(text).map_err(|e| {
            AppError::corrupted(messages::text(
                "ui.presetdata.registry.292",
                &[("v1", "<内存>")],
            ))
            .with_detail(e.to_string())
        })?;
        file.normalize_typed_values();
        Ok(file)
    }

    /// **值的写法跟着 `valueType` 走**：`int` 参数里「整值浮点」归一成整数。
    ///
    /// 源数据里有 `defaultValue = 1.0`（TOML 浮点）而 `valueType = "int"` 的历史写法。
    /// 不归一的话 `Layers::effective_recipe()` 会把这个 `1.0` 原样交给写值层，
    /// 而 [`to_toml_value`] 对 `int` 只认 `as_i64()` —— 复制配方因为一个值被判
    /// "类型对不上"就整批拒绝，一项都写不进去。
    ///
    /// **只收整值**：`1.5` 原样保留，那是真的写错了，写值层会如实报错 ——
    /// 这一层不是兜底。`Float` / `Bool` / `Text` 与 `showWhen.value` 都不动
    /// （后者有自己的比较归一，见 `domain::visibility`）。
    ///
    /// 两条解析入口（[`Self::parse_str`] 与 [`ParamRegistry::load_from`]）都必须调它：
    /// 只归一一边的话，同一份文件在两个入口里会长出两种数字。
    pub(crate) fn normalize_typed_values(&mut self) {
        for p in &mut self.params {
            if p.value_type != ValueType::Int {
                continue;
            }
            coerce_integral_number(&mut p.default_value);
            for v in p.machine_variants.values_mut() {
                coerce_integral_number(v);
            }
            for c in &mut p.choices {
                coerce_integral_number(&mut c.value);
            }
        }
    }
}

/// JSON number 的 `1.0` 收成 `1`（只对**整值**）。非数、非整值、本来就整数：原样不动。
///
/// `serde_json::Value` 把 `1` 与 `1.0` 当成两个不相等的值，而注册表里它们是同一个数。
/// 归一放在**读入**这一层，`effective_recipe` / 产物 / 指纹 / 复制配方就都看同一份模型。
fn coerce_integral_number(v: &mut Value) {
    if v.is_i64() || v.is_u64() {
        return;
    }
    let Some(f) = v.as_f64() else {
        return;
    };
    if !f.is_finite() || f.fract() != 0.0 {
        return;
    }
    *v = Value::from(f as i64);
}

/// 原文里一处「`int` 参数却写成了小数」。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawIntLiteral {
    pub key: String,
    /// `""` = `defaultValue`；其它 = `machineVariants` 的 owner（`A1` / `A1:FAST`）
    pub owner: String,
    /// 原文写法（`1.0`）
    pub literal: String,
    /// 整值浮点（`1.0`）：`presets normalize` 能写正；`false` 是真小数，得先改数值
    pub integral: bool,
}

fn push_if_float(out: &mut Vec<RawIntLiteral>, key: &str, owner: &str, v: &toml_edit::Value) {
    let Some(f) = v.as_float() else {
        return;
    };
    out.push(RawIntLiteral {
        key: key.to_owned(),
        owner: owner.to_owned(),
        /* `Display` 会把前缀的空格一起印出来（` 1.0`），消息里要的是原文 */
        literal: v.to_string().trim().to_owned(),
        integral: f.is_finite() && f.fract() == 0.0,
    });
}

/* ---------- 参数摆放 ---------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutItem {
    pub id: String,
    pub param_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutSection {
    pub id: String,
    #[serde(default)]
    pub items: Vec<LayoutItem>,
    /// 整块自定义组件（`settings` 下 11 个 section 都靠它）
    #[serde(default)]
    pub component: Option<String>,
    /// **section 级可见性** —— 只有这个文件里有，`param_registry` 没有
    #[serde(default)]
    pub show_when: Option<ShowWhen>,
    #[serde(default)]
    pub layout: Option<String>,
    #[serde(default)]
    pub width: Option<String>,
    #[serde(default)]
    pub visible: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutTab {
    pub id: String,
    #[serde(default)]
    pub sections: Vec<LayoutSection>,
}

#[derive(Debug, Deserialize)]
struct LayoutFile {
    #[serde(default)]
    tabs: Vec<LayoutTab>,
}

/* ---------- 合起来的字段定义 ---------- */

/// 字段定义表。74 条字段 + 页签/分组元信息 + 参数摆放
pub struct ParamRegistry {
    params: Vec<ParamDef>,
    tabs: Vec<TabMeta>,
    layout: Vec<LayoutTab>,
    updated: String,
    /// key → params 下标
    index: HashMap<String, usize>,
    doc: DocumentMut,
    file: PathBuf,
    layout_doc: DocumentMut,
    layout_file: PathBuf,
    /// 定义指纹的**一次性缓存**（2026-10-02；2026-10-05 起只算定义，见
    /// [`schema_fingerprint`](Self::schema_fingerprint) 的注）。
    ///
    /// # 为什么要有它
    ///
    /// 指纹进有效配方的哈希（见 `Layers::fingerprint`），而 `Layers::fingerprint()`
    /// 在 `book_view` / `build_rows` 里按版本被调 O(版本数) 次
    /// —— 真数据实测：一次 8.5ms（序列化 74 条定义 + 整本布局再 SHA256），
    /// 27 次就是 230ms，`wb_book` 于是卡住 320ms（作者："像被冻结住了"）。
    ///
    /// 但**这张表构造完就不再变**（唯一改它的路径是 `load_from` 重建整个 `ParamRegistry`），
    /// 一份会话里对所有机型 / 所有版本都是**同一个值** —— 重算纯属浪费。
    /// 缓存成 `OnceLock`：`schema_fingerprint()` 仍是 `&self`，返回值一字不变，只是不再重算。
    schema_fingerprint_cache: std::sync::OnceLock<String>,
}

/// 手写而不是 `#[derive(Debug)]`：派生版会把 74 条参数定义连同布局整本打印出来，
/// 而 `unwrap_err()` 失败时打的就是它 —— 真正的失败原因会被冲到几千行之外
impl std::fmt::Debug for ParamRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ParamRegistry({} 参数 / {} tab / updated {})",
            self.params.len(),
            self.tabs.len(),
            self.updated
        )
    }
}

impl ParamRegistry {
    pub fn load_from(root: &Path) -> Result<Self, AppError> {
        let file = root.join("registry").join("param_registry.toml");
        let text = super::read(&file)?;
        let doc = super::parse_text(&text, &file)?;
        let mut parsed: RegistryFile = toml_edit::de::from_str(&text).map_err(|e| {
            AppError::corrupted(messages::text(
                "ui.presetdata.registry.292",
                &[("v1", &file.display().to_string())],
            ))
            .with_detail(e.to_string())
        })?;
        /* 与 `RegistryFile::parse_str` 同一条归一 —— 两个入口各读一份文件，
        只归一一边的话同一份数字会有两种写法 */
        parsed.normalize_typed_values();

        let layout_file = root.join("layout_schema.toml");
        let layout_text = super::read(&layout_file)?;
        let layout_doc = super::parse_text(&layout_text, &layout_file)?;
        let layout: LayoutFile = toml_edit::de::from_str(&layout_text).map_err(|e| {
            AppError::corrupted(messages::text(
                "ui.presetdata.registry.293",
                &[("v1", &layout_file.display().to_string())],
            ))
            .with_detail(e.to_string())
        })?;

        let index = parsed
            .params
            .iter()
            .enumerate()
            .map(|(i, p)| (p.key.clone(), i))
            .collect();

        let out = Self {
            params: parsed.params,
            tabs: parsed.tabs,
            layout: layout.tabs,
            updated: parsed.updated,
            index,
            doc,
            file,
            layout_doc,
            layout_file,
            schema_fingerprint_cache: std::sync::OnceLock::new(),
        };
        out.check_consistency()?;
        Ok(out)
    }

    /// 数据本身没有这道门禁，所以在这里立四条。
    ///
    /// 它们不成立时的后果都是**静默的**：矩阵少几行、或多出永远为空的分类、或把 X 和 Y
    /// 调个头 —— 没有一条会自己报错。所以判据必须在加载时就拦住，而不是等界面看起来怪。
    pub fn check_consistency(&self) -> Result<(), AppError> {
        /*
         * ④ section id 全局唯一。
         *
         * 2026-10-20 的实况：`calibration` 同时挂在「更多」与「软件设置」下，
         * 于是 `param_tabs` 按 id 收留、「tab_of_section」按第一个命中归属 ——
         * 工作台「软件设置」有计数没有行，客户端却把它单列一个页签。
         * 同一个 id 有两个页签归属时，界面说什么都可能是错的，所以这里直接拒。
         */
        let mut seen_sections: BTreeMap<&str, &str> = BTreeMap::new();
        for t in &self.layout {
            for s in &t.sections {
                if let Some(prev) = seen_sections.insert(s.id.as_str(), t.id.as_str()) {
                    return Err(AppError::corrupted(messages::text(
                        "ui.presetdata.registry.294",
                        &[("v1", &s.id.to_string())],
                    ))
                    .with_detail(format!(
                        "先出现在 {prev}，又出现在 {} —— 「参数属于哪个页签」会有两个答案",
                        t.id
                    )));
                }
            }
        }

        // ① params[].key 与 layout 的 paramKey 严格双射
        let param_keys: BTreeSet<&str> = self.params.iter().map(|p| p.key.as_str()).collect();
        let mut layout_keys: Vec<&str> = Vec::new();
        for t in &self.layout {
            for s in &t.sections {
                for it in &s.items {
                    layout_keys.push(it.param_key.as_str());
                }
            }
        }
        let layout_set: BTreeSet<&str> = layout_keys.iter().copied().collect();
        if layout_keys.len() != layout_set.len() {
            let mut seen = BTreeSet::new();
            let dup: Vec<&str> = layout_keys
                .iter()
                .copied()
                .filter(|k| !seen.insert(*k))
                .collect();
            return Err(
                AppError::corrupted(messages::text("ui.presetdata.registry.295", &[])).with_detail(
                    ::postprocess::diag::messages::text(
                        "ui.presetdata.registry.why_dup_key",
                        &[("list", &dup.join(", "))],
                    ),
                ),
            );
        }
        let missing: Vec<&str> = param_keys.difference(&layout_set).copied().collect();
        if !missing.is_empty() {
            return Err(
                AppError::corrupted(messages::text("ui.presetdata.registry.296", &[])).with_detail(
                    ::postprocess::diag::messages::text(
                        "ui.presetdata.registry.why_missing_key",
                        &[("list", &missing.join(", "))],
                    ),
                ),
            );
        }
        let dangling: Vec<&str> = layout_set.difference(&param_keys).copied().collect();
        if !dangling.is_empty() {
            return Err(
                AppError::corrupted(messages::text("ui.presetdata.registry.297", &[])).with_detail(
                    ::postprocess::diag::messages::text(
                        "ui.presetdata.registry.why_dangling_key",
                        &[("list", &dangling.join(", "))],
                    ),
                ),
            );
        }

        // ② 每个 section 两侧的参数数逐个相等
        let mut by_section: BTreeMap<&str, usize> = BTreeMap::new();
        for p in &self.params {
            *by_section.entry(p.layout.section_id.as_str()).or_insert(0) += 1;
        }
        for t in &self.layout {
            for s in &t.sections {
                if s.items.is_empty() {
                    continue;
                }
                let declared = by_section.get(s.id.as_str()).copied().unwrap_or(0);
                if declared != s.items.len() {
                    return Err(AppError::corrupted(messages::text(
                        "ui.presetdata.registry.298",
                        &[("v1", &s.id.to_string())],
                    ))
                    .with_detail(format!(
                        "layout 里 {} 个，params[].layout.sectionId 指过来 {} 个",
                        s.items.len(),
                        declared
                    )));
                }
            }
        }

        // ③ 每个 section 的 items 顺序 == 按 layout.order 升序
        for t in &self.layout {
            for s in &t.sections {
                if s.items.is_empty() {
                    continue;
                }
                let mut expected: Vec<&ParamDef> = self
                    .params
                    .iter()
                    .filter(|p| p.layout.section_id == s.id)
                    .collect();
                expected.sort_by(|a, b| a.layout.order.total_cmp(&b.layout.order));
                let by_order: Vec<&str> = expected.iter().map(|p| p.key.as_str()).collect();
                let by_items: Vec<&str> = s.items.iter().map(|i| i.param_key.as_str()).collect();
                if by_order != by_items {
                    return Err(AppError::corrupted(messages::text(
                        "ui.presetdata.registry.299",
                        &[("v1", &s.id.to_string())],
                    ))
                    .with_detail(format!(
                        "按 order：{}；按 items：{}",
                        by_order.join(" → "),
                        by_items.join(" → ")
                    )));
                }
            }
        }

        Ok(())
    }

    pub fn params(&self) -> &[ParamDef] {
        &self.params
    }

    /// **这个版本有没有自己的参数正文**（b05 Task 14.4 / doc §4.3 第 6 步）。
    ///
    /// 判据：`machineVariants` 里存在 `{机型}:{版本}` 形状的键。
    /// **只看这一张表** —— min/max 那两张是字段定义的机型特化，不是配方值
    /// （见 `machine_min_variants` 的字段注释）。`false` = 纯继承基底 ——
    /// 不是错误（defaults 兜底照样渲染），但界面上要标「参数源待补」：
    /// 模板复制（14.5）之后这里翻成 true。
    pub fn version_has_variants(&self, uid: &str) -> bool {
        self.params
            .iter()
            .any(|p| p.machine_variants.contains_key(uid))
    }

    pub fn param(&self, key: &str) -> Option<&ParamDef> {
        self.index.get(key).map(|i| &self.params[*i])
    }

    pub fn tabs(&self) -> &[TabMeta] {
        &self.tabs
    }

    /// 参数摆放（`layout_schema.toml` 的 `tabs[].sections[].items[]`）。
    ///
    /// 给**新世界 catalog 的构建**用：catalog 的 definition 要带上完整布局，
    /// 客户端只读 catalog 就能渲染界面（总纲判据 4 的数据基础）
    pub fn layout(&self) -> &[LayoutTab] {
        &self.layout
    }

    /// `param_registry.toml` 的 `updated`，给界面显示
    pub fn updated(&self) -> &str {
        &self.updated
    }

    /// 这台机型**真实看得到**的字段，按 `layout.order` 升序。
    ///
    /// 两道过滤：`machineFilter` 排除的不属于这台机型；`deprecated` 的不再显示、不进产物。
    /// 归并机型基底时也走这一条（doc §3.3 的 `visibleKeys`）—— 被排除的字段不该出现在
    /// 它的基底里。算法在 [`crate::presetdata::resolve::visible_keys_of`]（与 catalog
    /// 消费端共用），这里只是适配
    pub fn visible_keys(&self, machine_id: &str) -> Vec<&str> {
        super::resolve::visible_keys_of(&self.params, machine_id)
    }

    /// 参数台 / 矩阵**这一屏**的行：与 [`Self::visible_keys`] 唯一的差别是
    /// **弃用的参数不隐藏**（C14 §五：作者明确要「看得见 + 有那种感觉」——
    /// 藏起来会变成「明明有却找不到」；界面上划红线 + 禁用 + 徽章，值照旧读得到）。
    ///
    /// 生成侧（配方归并、参数正文）仍走 [`Self::visible_keys`]：弃用的参数
    /// 不进产物，那是上游标记的另一半含义，两边不许混。
    pub fn desk_keys(&self, machine_id: &str) -> Vec<&str> {
        let mut hit: Vec<&ParamDef> = self
            .params
            .iter()
            .filter(|p| p.applies_to(machine_id))
            .collect();
        hit.sort_by(|a, b| a.layout.order.total_cmp(&b.layout.order));
        hit.into_iter().map(|p| p.key.as_str()).collect()
    }

    /// 这个参数的选项里，哪些**已经弃用** —— 给控件的选项划线、给写值那道闸用（C14 §五）。
    ///
    /// # 判据是推出来的，不是上游标的
    ///
    /// 注册表的选项级 `deprecated` 上游基本不标；但有些档位通向的东西已经全没了，
    /// 规则很直白：**选这一档（`showWhen` 指向它、`op = eq`）之后，能被它放开的
    /// 参数全都弃用了（且至少放开一条）**。要求「至少放开一条」是防 `every`
    /// 把空集读成「全都」，那样没接任何子项的普通选项会被误判。
    ///
    /// `neq` / `gt` 不算「放开」：选这一档时那些参数是被**关掉**的，不构成
    /// 「选它会通向弃用」。「至少放开一条」的实测结果全表只命中一档：
    /// `外围结构 = 护套` —— 它放开的 5 条全被上游标了。
    ///
    /// 上游自己标的选项级 `deprecated` 照旧算数：两路判据**并集**。
    ///
    /// # 这是「全机型并集」，写闸不许用它
    ///
    /// 一份定义在一台机器上弃用、在另一台上还能写，是这一版新增的能力。
    /// 上面这条并集只回答「有没有任何地方不能写」，给注册表视图划线；
    /// **判某一台机型**走 [`Self::deprecated_choice_values_for`]。
    pub fn deprecated_choice_values(&self, key: &str) -> BTreeSet<String> {
        let Some(p) = self.param(key) else {
            return BTreeSet::new();
        };
        let mut out = BTreeSet::new();
        for c in &p.choices {
            // 全局弃用与点名弃用在「全机型并集」里都算（只要有一台不能写）
            if c.deprecated || !c.deprecated_machines.is_empty() {
                out.insert(json_key(&c.value));
            }
        }
        for c in &p.choices {
            // 被这一档放开的参数：showWhen 指向本参数、op = eq、值就是这一档
            let gated = self.gated_by(key, &c.value);
            if !gated.is_empty()
                && gated
                    .iter()
                    .all(|q| q.deprecated || !q.deprecated_machines.is_empty())
            {
                out.insert(json_key(&c.value));
            }
        }
        out
    }

    /// 这台机型上，这个参数哪些选项**不能写** —— 写闸与参数台格子的唯一判据。
    ///
    /// 与全机型并集同一条规则，只是「弃用」按 `deprecated_for(machine)` 算：
    /// 显式标注（全局或点名这台机型）∪ 推定（选它之后放开的子参数**在这台机型上**
    /// 全都弃用了，且至少放开一条）。
    pub fn deprecated_choice_values_for(&self, key: &str, machine_id: &str) -> BTreeSet<String> {
        let Some(p) = self.param(key) else {
            return BTreeSet::new();
        };
        let mut out = BTreeSet::new();
        for c in &p.choices {
            if c.deprecated_for(machine_id) {
                out.insert(json_key(&c.value));
            }
        }
        for c in &p.choices {
            let gated = self.gated_by(key, &c.value);
            if !gated.is_empty() && gated.iter().all(|q| q.deprecated_for(machine_id)) {
                out.insert(json_key(&c.value));
            }
        }
        out
    }

    /// 被「本参数 = 这一档」放开的参数（`showWhen` 指向本参数、`op = eq`、值相同）
    fn gated_by(&self, key: &str, value: &Value) -> Vec<&ParamDef> {
        self.params
            .iter()
            .filter(|q| {
                q.show_when.as_ref().is_some_and(|sw| {
                    sw.key == key && sw.op == ShowOp::Eq && json_key(&sw.value) == json_key(value)
                })
            })
            .collect()
    }

    /// 这个值是不是该参数的一个**已弃用选项**。命中给回选项的中文名 ——
    /// 写值那道闸（`patch::validate`）的错误消息要点名是哪一档。
    /// `value = null`（挂回继承）不在这里判：删键不是写值。
    ///
    /// `machine_id` = 这次要写到哪台机型上（版本层由调用方先把 uid 换成机型）。
    pub fn deprecated_choice_hit(
        &self,
        key: &str,
        value: &Value,
        machine_id: &str,
    ) -> Option<String> {
        if value.is_null() {
            return None;
        }
        let dead = self.deprecated_choice_values_for(key, machine_id);
        let p = self.param(key)?;
        let k = json_key(value);
        if !dead.contains(&k) {
            return None;
        }
        p.choices
            .iter()
            .find(|c| json_key(&c.value) == k)
            .map(|c| c.label.clone())
    }

    /// 该参数各选项的弃用标记，**与 `choices` 一一对齐** ——
    /// 给 `wb_registry` 的 `ChoiceView` 用。上游标的与推出来的取并集
    /// （判据见 [`Self::deprecated_choice_values`]）
    pub fn choice_deprecated(&self, key: &str) -> Vec<bool> {
        let dead = self.deprecated_choice_values(key);
        self.param(key).map_or_else(Vec::new, |p| {
            p.choices
                .iter()
                .map(|c| dead.contains(&json_key(&c.value)))
                .collect()
        })
    }

    /// 装参数的 section id（实测 16 个）。
    ///
    /// 判据是**布局里有没有 items**，而不是"section 在不在 tabs 里声明过" ——
    /// 27 个都声明过，其中 11 个是 `component` 占位（客户端设置页），1 个 tab 连
    /// section 都没有。按 27 个建矩阵分类会多出 12 个永远为空的页签。
    pub fn param_section_ids(&self) -> BTreeSet<&str> {
        self.layout
            .iter()
            .flat_map(|t| t.sections.iter())
            .filter(|s| !s.items.is_empty())
            .map(|s| s.id.as_str())
            .collect()
    }

    /// 矩阵的分类页签：**只留装参数的那些**，中文名与顺序取 `[[tabs]]`
    /// （`layout_schema` 全文没有 label / order / icon）。
    ///
    /// 返回里每个 tab 的 sections 也已过滤过，所以调用方拿到的就是可渲染的那一份。
    pub fn param_tabs(&self) -> Vec<TabMeta> {
        let keep = self.param_section_ids();
        let mut out: Vec<TabMeta> = self
            .tabs
            .iter()
            .map(|t| TabMeta {
                sections: t
                    .sections
                    .iter()
                    .filter(|s| keep.contains(s.id.as_str()))
                    .cloned()
                    .collect(),
                ..t.clone()
            })
            .filter(|t| !t.sections.is_empty())
            .collect();
        out.sort_by(|a, b| a.order.total_cmp(&b.order));
        out
    }

    /// section 级 `showWhen`（整组隐藏）。只有 `layout_schema` 有这个
    pub fn section_show_when(&self, section_id: &str) -> Option<&ShowWhen> {
        self.layout
            .iter()
            .flat_map(|t| t.sections.iter())
            .find(|s| s.id == section_id)
            .and_then(|s| s.show_when.as_ref())
    }

    /// 这个 section 属于哪个 tab。矩阵按分类过滤时要用。
    ///
    /// 查的是 `layout_schema` 而不是 `[[tabs]]`：两处都声明了归属，
    /// 但参数摆在哪个 tab 下是布局说的（`[[tabs]]` 给的是中文名与顺序）
    pub fn tab_of_section(&self, section_id: &str) -> Option<&str> {
        self.layout
            .iter()
            .find(|t| t.sections.iter().any(|s| s.id == section_id))
            .map(|t| t.id.as_str())
    }

    /// 一个 section 的中文名与组内序。**中文名与顺序的唯一权威是 `[[tabs]]`**
    /// （`layout_schema` 全文没有 label / order）。
    ///
    /// 这里不过滤"装不装参数"：调用方要的是元数据本身，过滤是 `param_tabs` 的事
    pub fn section_meta(&self, section_id: &str) -> Option<&SectionMeta> {
        self.tabs
            .iter()
            .flat_map(|t| t.sections.iter())
            .find(|s| s.id == section_id)
    }

    /// 页签的先后。**查不到给 `f64::MAX`，不是 0** ——
    /// 多一个没在 `[[tabs]]` 里声明的页签时，它该排到最后，而不是插到最前面
    pub fn tab_order(&self, tab_id: &str) -> f64 {
        self.tabs
            .iter()
            .find(|t| t.id == tab_id)
            .map_or(f64::MAX, |t| t.order)
    }

    /// 定义指纹（schema fingerprint）：只哈希**字段定义**，不哈希**配方值**。
    ///
    /// # 定义与数据的分界（2026-10-05 定死，改这里之前先读完）
    ///
    /// 一次真事故立的规矩：作者在参数台改了 `A1:FASTV3.3` 一个值 —— 值覆盖就落在
    /// 这张表的 `[params.machineVariants]` 里 —— 旧版把**整张表**哈希进指纹，
    /// 于是 9 份产物的 uuid 全变、全部「待更新」，而其他 8 份的正文一个字节都不该动。
    ///
    /// 给字段归类只认一句判据：
    ///
    /// > 改了它，是**所有引用这条定义的产物**都该重出生成（定义），
    /// > 还是只有**握着这个值的那个版本**（数据）？
    ///
    /// - **定义**（进本指纹）：key / tomlKey / 类型 / 出厂默认 / 约束（min·max·choices）/
    ///   适用机型 / 弃用 / 注释 / 布局，以及 `machineMinVariants` / `machineMaxVariants`
    ///   —— 它们是**定义的机型特化**，不是配方值（见 [`ParamDef`] 那边的注）。
    /// - **数据**（不进本指纹）：`machineVariants` 的值覆盖。每个版本的有效值本来就
    ///   单独进 [`crate::presetdata::resolve::Layers::fingerprint`] 的 `recipe`，
    ///   这里再算一遍，只会让改了别家值的那一版把全体版本都带成「待更新」。
    ///
    /// 结构性保证：把整条 [`ParamDef`] 序列化后剥掉 `machineVariants` —— 以后给
    /// `ParamDef` **新加字段默认进定义指纹**（这是安全的默认）；想加"数据"只能走
    /// `machineVariants`，而它恰好被剥掉。谁也别再随手往指纹里塞字段。
    ///
    /// **算一次就缓存**（见 `schema_fingerprint_cache` 字段的注）：这张表构造完不再变，
    /// 一份会话里对谁都是同一个值，而它在 `book_view` / `build_rows` 里被按版本反复取。
    pub fn schema_fingerprint(&self) -> String {
        self.schema_fingerprint_cache
            .get_or_init(|| {
                use sha2::{Digest, Sha256};
                // 剥掉值覆盖：定义是全体版本共享的，值是每个版本自己的
                let defs: Vec<Value> = self
                    .params
                    .iter()
                    .map(|p| {
                        let mut v = serde_json::to_value(p).unwrap_or_default();
                        if let Some(obj) = v.as_object_mut() {
                            obj.remove("machineVariants");
                        }
                        v
                    })
                    .collect();
                let payload = serde_json::json!({
                    "params": defs,
                    "tabs": &self.tabs,
                    "layout": &self.layout,
                });
                let mut h = Sha256::new();
                h.update(serde_json::to_vec(&payload).unwrap_or_default());
                format!("{:x}", h.finalize())
            })
            .clone()
    }

    /// **原文里 `int` 参数却写成了小数**的那些值（`defaultValue`、`machineVariants`、`choices`）。
    ///
    /// 模型在读入时已经归一（见 [`RegistryFile::normalize_typed_values`]），所以
    /// **只有原文 token 还留着旧写法**。发布闸要看的是「源文件写对了没有」，
    /// 不是「读得对不对」—— 所以这一条从 `doc` 上扫，而不是从 `params` 上扫。
    ///
    /// 只认标准表（`[[params]]` / `[params.machineVariants]`）：那是
    /// `presets normalize` 写出来的形状，也是仓库里唯一在用的形状。
    pub fn raw_int_literals(&self) -> Vec<RawIntLiteral> {
        let Some(params) = self.doc.get("params").and_then(|i| i.as_array_of_tables()) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for t in params.iter() {
            if t.get("valueType").and_then(|v| v.as_str()) != Some("int") {
                continue;
            }
            let Some(key) = t.get("key").and_then(|v| v.as_str()) else {
                continue;
            };
            if let Some(v) = t.get("defaultValue").and_then(|i| i.as_value()) {
                push_if_float(&mut out, key, "", v);
            }
            if let Some(mv) = t.get("machineVariants") {
                if let Some(tbl) = mv.as_table() {
                    for (owner, item) in tbl.iter() {
                        if let Some(v) = item.as_value() {
                            push_if_float(&mut out, key, owner, v);
                        }
                    }
                } else if let Some(it) = mv.as_inline_table() {
                    for (owner, v) in it.iter() {
                        push_if_float(&mut out, key, owner, v);
                    }
                }
            }
            if let Some(choices) = t.get("choices").and_then(|i| i.as_array_of_tables()) {
                for (i, c) in choices.iter().enumerate() {
                    if let Some(v) = c.get("value").and_then(|x| x.as_value()) {
                        push_if_float(&mut out, key, &format!("choices[{i}]"), v);
                    }
                }
            }
        }
        out.sort_by(|a, b| (&a.key, &a.owner).cmp(&(&b.key, &b.owner)));
        out
    }

    /* ---------- 写回 ---------- */

    /// 写一个机型 / 版本的值覆盖，落在 `[params.machineVariants]` 里。
    ///
    /// # 为什么值就写在这张表里，不另开一层
    ///
    /// doc §1 定过：**不做覆盖层**。我们就是这份数据的编辑器，改一个值就是改它，
    /// 而不是在旁边记一条"我把它改成了别的"。多一层的代价是两处真相 ——
    /// 而「界面显示的是哪一个」这种问题在多一层之后永远说不清。
    ///
    /// `owner` 两种形状，与这张表的键形状一一对应：
    ///
    /// ```text
    /// "A1"        整台机型（机型基底那一层）
    /// "A1:FAST"   某一个版本（版本覆盖那一层）
    /// ```
    ///
    /// **这一层只查形状**（非空、单个冒号、不含空白）。「这个机型/版本真的存在吗」
    /// 要跨文件才查得出来，在 [`super::Presets::set_variant`]；
    /// 写进一个不存在的键的后果很硬 —— 下一次加载会被跨文件断言判成 `Corrupted`，
    /// 整个工作台起不来。
    pub fn set_variant(&mut self, key: &str, owner: &str, value: &Value) -> Result<(), AppError> {
        let at = *self.index.get(key).ok_or_else(|| {
            AppError::invalid_argument(messages::text(
                "ui.presetdata.registry.300",
                &[("key", key)],
            ))
        })?;
        check_owner(owner)?;

        /* ★ **这里刻意不做「同值不落键」**（2026-11-02）。
         *
         * 那条归一化是对的（见 `app::storage::normalize_same_as_inherited`），
         * 但**不能放在这一层**：`set_variant` 还被「复制配方 / 新建版本」用，
         * 那些场景里"显式写一个与继承值相同的值"正是**独立快照**的语义 ——
         * 删了它，副本就会跟着基底一起变（`copied_recipe_is_an_independent_snapshot`
         * 与 `a_machine_level_edit_rewrites_the_promoted_version_keys` 钉着这两条）。
         *
         * 所以归一化只放在「用户改一格值」那条路上。 */
        let item = to_toml_value(value, self.params[at].value_type, key)?;

        let t = self.param_table_mut(at, key)?;
        let table = t
            .entry("machineVariants")
            .or_insert(toml_edit::Item::Table(toml_edit::Table::new()))
            .as_table_mut()
            .ok_or_else(|| {
                AppError::corrupted(messages::text(
                    "ui.presetdata.registry.301",
                    &[("key", key)],
                ))
            })?;
        table[owner] = item;

        self.params[at]
            .machine_variants
            .insert(owner.to_owned(), value.clone());
        Ok(())
    }

    /// 这一层**继承来的值** = 它自己不钉时算出来的那个。
    ///
    /// - 机型层（`A1`）：继承 `defaultValue`；
    /// - 版本层（`A1:STANDARD`）：继承机型层那一格，机型层也没钉就继承 `defaultValue`。
    ///
    /// 给「用户改值」那条路做同值归一化用（`app::storage::normalize_same_as_inherited`）。
    /// **不要**拿它去改 [`Self::set_variant`] —— 那条路还被复制配方用。
    pub fn inherited_value_for(&self, key: &str, owner: &str) -> Option<Value> {
        let at = *self.index.get(key)?;
        let p = &self.params[at];
        match owner.split_once(':') {
            Some((machine, _)) => Some(
                p.machine_variants
                    .get(machine)
                    .cloned()
                    .unwrap_or_else(|| p.default_value.clone()),
            ),
            None => Some(p.default_value.clone()),
        }
    }

    /// 「清空」一个机型/版本的值 = **把那个键整行删掉**，不是写 `0` 或空串。
    ///
    /// 这一条与元字段那边同一个道理：没有这一行 = 「跟着上一层走」，
    /// 而写一个值 = 「我在这一层特意定了它」。两件事在界面上必须分得开。
    ///
    /// 删到这张表为空时，**连表头一起删** —— 留一个空的
    /// `[params.machineVariants]` 在文件里，读起来像"这里有机型差异"，而其实没有
    pub fn clear_variant(&mut self, key: &str, owner: &str) -> Result<(), AppError> {
        let at = *self.index.get(key).ok_or_else(|| {
            AppError::invalid_argument(messages::text(
                "ui.presetdata.registry.302",
                &[("key", key)],
            ))
        })?;
        check_owner(owner)?;

        /* ★ **"原文里真有覆盖"= 这张表里还有别的机型键**（2026-11-02）。
        作者的原始要求：「70 → 75 → 70 之后，文件应恢复到原始字节状态，不应因为格式
        归一化产生『未保存 1』」；后来又补了一条口子：临时建的表保存一轮之后也不能
        变成"原文就有"。
        两版判据都试过（打开时的键集合 / 打开时的表内容），都会被**自己上一轮漏出来的
        空表头**喂大。现在只看**当前** doc：表里还有别的键才算"原文真有这一层"；
        只剩这一个键 ⇒ 这张表就是这个参数一个人写的，清空后整段删。 */
        let siblings = self
            .param_table_mut(at, key)
            .ok()
            .and_then(|t| t.get("machineVariants")?.as_table().map(|m| m.len()))
            .unwrap_or(0);
        let keep_empty = siblings > 1;

        let t = self.param_table_mut(at, key)?;
        let left = match t.get_mut("machineVariants").and_then(|i| i.as_table_mut()) {
            Some(table) => {
                table.remove(owner);
                table.len()
            }
            None => return Ok(()),
        };
        /* 表空了：还有别的覆盖时留表头（下面常挂着作者写的说明注释），
        否则整段删掉 —— 这就是"临时建出来的表不许留在文件里"。 */
        if left == 0 && !keep_empty {
            t.remove("machineVariants");
        }

        self.params[at].machine_variants.remove(owner);
        Ok(())
    }

    /// 改一个版本的**跟随状态**，落在 `[params.follows]` 里。
    ///
    /// `on = true`（跟随）**删掉那个键**：文件的默认形状就是跟随，写 `= true`
    /// 只是多一行噪音，而且会让"表里没有"与"写着 true"变成两种同义写法
    /// （同一条判据两个源）。`on = false` 才写 `= false`。
    ///
    /// 表被删空时**连表头一起删** —— 与 [`Self::clear_variant`] 同一口径：
    /// 留一个空的 `[params.follows]` 读起来像"这里有状态"，而其实一条都没有。
    ///
    /// **只改状态，不碰值**：值同步是调用方（`app::storage` 那条写值路）的事，
    /// 它会在**同一次** `apply_values` 里把两件事一起落盘。
    pub fn set_following(&mut self, key: &str, uid: &str, on: bool) -> Result<(), AppError> {
        let at = *self.index.get(key).ok_or_else(|| {
            AppError::invalid_argument(messages::text(
                "ui.presetdata.registry.300",
                &[("key", key)],
            ))
        })?;
        check_owner(uid)?;

        let t = self.param_table_mut(at, key)?;
        if on {
            let left = match t.get_mut("follows").and_then(|i| i.as_table_mut()) {
                Some(table) => {
                    table.remove(uid);
                    table.len()
                }
                None => 0,
            };
            if left == 0 {
                t.remove("follows");
            }
        } else {
            let table = t
                .entry("follows")
                .or_insert(toml_edit::Item::Table(toml_edit::Table::new()))
                .as_table_mut()
                .ok_or_else(|| {
                    AppError::corrupted(messages::text(
                        "ui.presetdata.registry.301",
                        &[("key", key)],
                    ))
                })?;
            table[uid] = toml_edit::value(false);
        }

        super::follows::set_following(&mut self.params[at], uid, on);
        Ok(())
    }

    /// 删掉某个 owner（`机型:版本`）在**四张表**里的全部记录，返回动过的字段 key。
    ///
    /// 只有「删版本」会调它：版本没了，这些键就指向一个不存在的对象，而
    /// [`super::Presets::check_cross_consistency`] 下一次加载会把整份数据判成
    /// `Corrupted`（整个工作台起不来）。四张表必须一起清 —— 漏一张就等于没删干净：
    ///
    /// | 表 | 装什么 |
    /// |---|---|
    /// | `machineVariants` | 这一版钉过的值 |
    /// | `machineMinVariants` / `machineMaxVariants` | 范围的机型特化 |
    /// | `follows` | 这一版跟不跟基底 |
    ///
    /// 删空的表连表头一起删，与 [`Self::clear_variant`] / [`Self::set_following`]
    /// 同一口径（留一个空表读起来像"这里有记录"）。
    pub fn drop_owner(&mut self, owner: &str) -> Result<Vec<String>, AppError> {
        let mut touched = Vec::new();
        for at in 0..self.params.len() {
            let p = &self.params[at];
            let had = p.machine_variants.contains_key(owner)
                || p.machine_min_variants.contains_key(owner)
                || p.machine_max_variants.contains_key(owner)
                || p.follows.contains_key(owner);
            if !had {
                continue;
            }
            let key = p.key.clone();

            let t = self.param_table_mut(at, &key)?;
            for field in [
                "machineVariants",
                "machineMinVariants",
                "machineMaxVariants",
                "follows",
            ] {
                /* **两种形状都要认**：`machineVariants` / `follows` 是真表
                （`[params.machineVariants]`），min/max 在源文件里是**内联表**
                （`machineMinVariants = { "A1:FAST" = 10.0 }`）。只处理前者的话，
                删完 `A1:FAST` 在 min 里还留着，下一次加载照样判 Corrupted。 */
                let empty = match t.get_mut(field) {
                    Some(item) => {
                        if let Some(tbl) = item.as_table_mut() {
                            tbl.remove(owner);
                            tbl.is_empty()
                        } else if let Some(inline) = item.as_inline_table_mut() {
                            inline.remove(owner);
                            inline.is_empty()
                        } else {
                            false
                        }
                    }
                    None => false,
                };
                if empty {
                    t.remove(field);
                }
            }

            let p = &mut self.params[at];
            p.machine_variants.remove(owner);
            p.machine_min_variants.remove(owner);
            p.machine_max_variants.remove(owner);
            p.follows.remove(owner);
            touched.push(key);
        }
        Ok(touched)
    }

    /// 把一个 owner（`机型:版本`）在**四张表**里的键改名，返回动过的字段 key。
    ///
    /// 给「改版本 id」用：`A1:FAST` 要跟着搬成 `A1:NEW1`，否则那几项值会指向
    /// 一个不存在的版本 —— 下一次加载 [`super::Presets::check_cross_consistency`]
    /// 当场判 `Corrupted`，与 [`Self::drop_owner`] 那条注释是同一类事故。
    ///
    /// 表与内联表两种形状都要认（min / max 在源文件里是 `machineMinVariants = { ... }`）。
    /// 新键追加在该表的末尾：`toml_edit` 的 `Table` 保序但换不了键位，一次改名把那一行
    /// 挪到同字段的末尾可以接受（改主键本来就是大动）。
    pub fn rename_owner(
        &mut self,
        old_owner: &str,
        new_owner: &str,
    ) -> Result<Vec<String>, AppError> {
        let mut touched = Vec::new();
        for at in 0..self.params.len() {
            let p = &self.params[at];
            let had = p.machine_variants.contains_key(old_owner)
                || p.machine_min_variants.contains_key(old_owner)
                || p.machine_max_variants.contains_key(old_owner)
                || p.follows.contains_key(old_owner);
            if !had {
                continue;
            }
            let key = p.key.clone();

            let t = self.param_table_mut(at, &key)?;
            for field in [
                "machineVariants",
                "machineMinVariants",
                "machineMaxVariants",
                "follows",
            ] {
                if let Some(item) = t.get_mut(field) {
                    if let Some(tbl) = item.as_table_mut() {
                        if let Some(v) = tbl.remove(old_owner) {
                            tbl[new_owner] = v;
                        }
                    } else if let Some(inline) = item.as_inline_table_mut() {
                        if let Some(v) = inline.remove(old_owner) {
                            inline.insert(new_owner, v);
                        }
                    }
                }
            }

            let p = &mut self.params[at];
            if let Some(v) = p.machine_variants.remove(old_owner) {
                p.machine_variants.insert(new_owner.to_owned(), v);
            }
            if let Some(v) = p.machine_min_variants.remove(old_owner) {
                p.machine_min_variants.insert(new_owner.to_owned(), v);
            }
            if let Some(v) = p.machine_max_variants.remove(old_owner) {
                p.machine_max_variants.insert(new_owner.to_owned(), v);
            }
            if let Some(v) = p.follows.remove(old_owner) {
                p.follows.insert(new_owner.to_owned(), v);
            }
            touched.push(key);
        }
        Ok(touched)
    }

    /* ---------- 定义编辑（2026-10-03，作者：「弃用是谁决定的？我没办法改」） ---------- */

    /// 改一条参数的**定义**（名称 / 说明 / 单位 / 值类型 / 控件 / 范围 / 步进 /
    /// 出厂默认 / 属于 / 前置条件 / 弃用 / 可选项 / 适用机型），就地写进 `[[params]]` 本体。
    ///
    /// # 为什么不走 `machineVariants` 那条路
    ///
    /// 值覆盖（[`Self::set_variant`]）改的是「某台机型上它是多少」，定义改的是
    /// 「它是什么」。后者直接改 `[[params]]` 的字段本身 —— 与 doc §1「我们就是
    /// 这份数据的编辑器」同一个道理：不加覆盖层。
    ///
    /// # 校验（全在动文件之前 —— 校验不过就不许碰到一半）
    ///
    ///   · 名称不能为空；步进得是正数；min ≤ max（缺一边不限）；
    ///   · `defaultValue` 与 `valueType` 对得上（复用 [`to_toml_value`] 的口径 ——
    ///     写不进 TOML 的默认值就是非法的）；枚举参数的默认值得是选项之一；
    ///   · **控件 × 值类型的组合得画得出真的**：bool→开关、float/int→步进框、
    ///     string→分段/下拉（要有可选项）/文本（不能带可选项）/G-code ——
    ///     开关挂在 float 上那种「点了不切换」的假控件当场拒收；
    ///   · 可选项每条得有名字、值不能空、值不能重复（下拉里两行一个键是假选项）；
    ///   · `parentKey` 得存在、不能是自己、父项自己不能再有父项（层级只有两级）、
    ///     已经有儿子挂着的不能再去当儿子；
    ///   · `showWhen.key` 得存在且不能指着自己。**值不校验** —— 实测里就有
    ///     字符串化的数字（`"0"`），`visibility` 比较前会按被指向字段的类型归一。
    ///   · `machineFilter` 只做 trim / 去空 / 去重 —— 机型号打没打对注册表管不着
    ///     （界面给的是勾选框，这层错不到哪去；孤儿键有 preflight 提示兜着）。
    ///
    /// 值类型的改动**不迁移**已有的 machineVariants：类型是「它是什么」，
    /// 改了类型旧值就按新类型解释 —— 这是作者自己在模态框里按的按钮。
    pub fn set_param_meta(&mut self, key: &str, edit: ParamMetaEdit) -> Result<(), AppError> {
        let at = *self.index.get(key).ok_or_else(|| {
            AppError::invalid_argument(messages::text(
                "ui.presetdata.registry.303",
                &[("key", key)],
            ))
        })?;

        let label = edit.label.trim();
        if label.is_empty() {
            return Err(AppError::invalid_argument(messages::text(
                "ui.presetdata.registry.304",
                &[],
            )));
        }
        for (name, v) in [("最小", edit.min), ("最大", edit.max)] {
            if let Some(v) = v {
                if !v.is_finite() {
                    return Err(AppError::invalid_argument(messages::text(
                        "ui.presetdata.registry.305",
                        &[("name", name), ("v", &v.to_string())],
                    )));
                }
            }
        }
        if let (Some(min), Some(max)) = (edit.min, edit.max) {
            if min > max {
                return Err(AppError::invalid_argument(messages::text(
                    "ui.presetdata.registry.306",
                    &[("min", &min.to_string()), ("max", &max.to_string())],
                )));
            }
        }
        if let Some(s) = edit.step {
            if !s.is_finite() || s <= 0.0 {
                return Err(AppError::invalid_argument(messages::text(
                    "ui.presetdata.registry.307",
                    &[("s", &s.to_string())],
                )));
            }
        }
        // 出厂默认先过类型门（`to_toml_value` 只把数字当类型看 —— 字符串挂到
        // float 上它也照收，那里管的是「写得进 TOML」，这里管的是「类型说得通」）
        match (&edit.default_value, edit.value_type) {
            (Value::Number(_), ValueType::Float) => {}
            (Value::Number(n), ValueType::Int) if n.is_i64() || n.is_u64() => {}
            (Value::Bool(_), ValueType::Bool) => {}
            (Value::String(_), ValueType::Text) => {}
            (v, t) => {
                /* 值类型要求的三个短语本身也是文案（它们会拼进上面那句的括号里） */
                let want = match t {
                    ValueType::Float | ValueType::Int => {
                        messages::text("ui.presetdata.registry.want_number", &[])
                    }
                    ValueType::Bool => messages::text("ui.presetdata.registry.want_bool", &[]),
                    ValueType::Text => messages::text("ui.presetdata.registry.want_text", &[]),
                };
                return Err(AppError::invalid_argument(messages::text(
                    "ui.presetdata.registry.308",
                    &[
                        ("key", key),
                        ("v", &v.to_string()),
                        ("v1", value_type_str(t)),
                        ("v2", want.as_str()),
                    ],
                )));
            }
        }
        // 再按新类型试写一遍 —— 非整数的 int 当场拒
        let default_item = to_toml_value(&edit.default_value, edit.value_type, key)?;
        // 控件 × 值类型的合法组合（实测数据里也只有这几种搭配）。不合法的组合
        // 不是「先存着以后再说」—— 画出来的控件是假的：开关挂在 float 上点了
        // 只会弹提示不切换（作者：「它实际上根本没有换一套」），当场拒收
        match edit.ui_component {
            UiComponent::Switch if edit.value_type != ValueType::Bool => {
                return Err(AppError::invalid_argument(messages::text(
                    "ui.presetdata.registry.309",
                    &[("v1", value_type_str(edit.value_type))],
                )));
            }
            UiComponent::Number
                if !matches!(edit.value_type, ValueType::Float | ValueType::Int) =>
            {
                return Err(AppError::invalid_argument(messages::text(
                    "ui.presetdata.registry.310",
                    &[("v1", value_type_str(edit.value_type))],
                )));
            }
            UiComponent::Segmented | UiComponent::Select if edit.value_type != ValueType::Text => {
                return Err(AppError::invalid_argument(messages::text(
                    "ui.presetdata.registry.311",
                    &[("v1", value_type_str(edit.value_type))],
                )));
            }
            UiComponent::Segmented | UiComponent::Select if edit.choices.is_empty() => {
                return Err(AppError::invalid_argument(messages::text(
                    "ui.presetdata.registry.312",
                    &[],
                )));
            }
            UiComponent::Gcode if edit.value_type != ValueType::Text => {
                return Err(AppError::invalid_argument(messages::text(
                    "ui.presetdata.registry.313",
                    &[("v1", value_type_str(edit.value_type))],
                )));
            }
            UiComponent::Gcode if !edit.choices.is_empty() => {
                return Err(AppError::invalid_argument(messages::text(
                    "ui.presetdata.registry.314",
                    &[],
                )));
            }
            UiComponent::Text => {
                if edit.value_type != ValueType::Text {
                    return Err(AppError::invalid_argument(messages::text(
                        "ui.presetdata.registry.315",
                        &[("v1", value_type_str(edit.value_type))],
                    )));
                }
                if !edit.choices.is_empty() {
                    return Err(AppError::invalid_argument(messages::text(
                        "ui.presetdata.registry.316",
                        &[],
                    )));
                }
            }
            _ => {}
        }
        // 可选项这一表自己的门：每条得有名字、值不能空、值不能重复。
        // 重复的值在下拉里是两行一个键 —— 选哪个都落到同一档，是假选项
        let mut seen = std::collections::HashSet::new();
        for (i, c) in edit.choices.iter().enumerate() {
            let n = i + 1;
            if c.label.trim().is_empty() {
                return Err(AppError::invalid_argument(messages::text(
                    "ui.presetdata.registry.317",
                    &[("n", &n.to_string())],
                )));
            }
            let vkey = json_key(&c.value);
            if c.value.is_null() || vkey.is_empty() {
                return Err(AppError::invalid_argument(messages::text(
                    "ui.presetdata.registry.318",
                    &[("n", &n.to_string()), ("v1", c.label.trim())],
                )));
            }
            if !seen.insert(vkey) {
                return Err(AppError::invalid_argument(messages::text(
                    "ui.presetdata.registry.319",
                    &[("n", &n.to_string()), ("v1", c.label.trim())],
                )));
            }
        }
        if edit.value_type == ValueType::Text && !edit.choices.is_empty() {
            let want = json_key(&edit.default_value);
            if !edit.choices.iter().any(|c| json_key(&c.value) == want) {
                return Err(AppError::invalid_argument(messages::text(
                    "ui.presetdata.registry.320",
                    &[("want", &want.to_string())],
                )));
            }
        }
        if let Some(pk) = &edit.parent_key {
            if pk == key {
                return Err(AppError::invalid_argument(messages::text(
                    "ui.presetdata.registry.321",
                    &[("key", key)],
                )));
            }
            let parent = self.param(pk).ok_or_else(|| {
                AppError::invalid_argument(messages::text(
                    "ui.presetdata.registry.322",
                    &[("pk", &pk.to_string())],
                ))
            })?;
            if parent.parent_key.is_some() {
                return Err(AppError::invalid_argument(messages::text(
                    "ui.presetdata.registry.323",
                    &[("pk", &pk.to_string())],
                )));
            }
            if self
                .params
                .iter()
                .any(|p| p.parent_key.as_deref() == Some(key))
            {
                return Err(AppError::invalid_argument(messages::text(
                    "ui.presetdata.registry.324",
                    &[("key", key)],
                )));
            }
        }
        if let Some(sw) = &edit.show_when {
            if sw.key == key {
                return Err(AppError::invalid_argument(messages::text(
                    "ui.presetdata.registry.325",
                    &[("key", key)],
                )));
            }
            if !self.index.contains_key(&sw.key) {
                return Err(AppError::invalid_argument(messages::text(
                    "ui.presetdata.registry.326",
                    &[("v1", &sw.key.to_string())],
                )));
            }
        }
        // 前置条件子表整块先备好（值转换是会失败的 —— 失败就什么都不许动）
        let show_when_item = match &edit.show_when {
            Some(sw) => {
                let mut s = toml_edit::Table::new();
                s["key"] = toml_edit::value(sw.key.as_str());
                s["op"] = toml_edit::value(show_op_str(sw.op));
                s["value"] = scalar_item(&sw.value, key)?;
                Some(toml_edit::Item::Table(s))
            }
            None => None,
        };
        let unit = edit
            .unit
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned);
        // 可选项整表先备好（值转换是会失败的 —— 失败就什么都不许动）。
        // 「缺一行 = false」（与弃用同一条口径）—— 不弃用的不写 deprecated 键
        let choices: Vec<Choice> = edit
            .choices
            .iter()
            .map(|c| Choice {
                label: c.label.clone(),
                /* 载荷里没带英文就沿用文件里同一档的英文（同一个 value 对得上）——
                否则工作台保存一次，这一组选项的英文全丢 */
                label_en: if c.label_en.trim().is_empty() {
                    self.params[at]
                        .choices
                        .iter()
                        .find(|old| json_key(&old.value) == json_key(&c.value))
                        .map(|old| old.label_en.clone())
                        .unwrap_or_default()
                } else {
                    c.label_en.clone()
                },
                // 日 / 繁中同一条口径：载荷没带就沿用文件里同一档的旧值，
                // 否则工作台保存一次，这两册全丢（2026-10-31）
                label_ja: if c.label_ja.trim().is_empty() {
                    self.params[at]
                        .choices
                        .iter()
                        .find(|old| json_key(&old.value) == json_key(&c.value))
                        .map(|old| old.label_ja.clone())
                        .unwrap_or_default()
                } else {
                    c.label_ja.clone()
                },
                label_tw: if c.label_tw.trim().is_empty() {
                    self.params[at]
                        .choices
                        .iter()
                        .find(|old| json_key(&old.value) == json_key(&c.value))
                        .map(|old| old.label_tw.clone())
                        .unwrap_or_default()
                } else {
                    c.label_tw.clone()
                },
                value: c.value.clone(),
                deprecated: c.deprecated,
                deprecated_machines: normalize_machines(&c.deprecated_machines),
            })
            .collect();
        let choices_arr = if choices.is_empty() {
            None
        } else {
            let mut arr = toml_edit::ArrayOfTables::new();
            for (i, c) in choices.iter().enumerate() {
                let mut tbl = toml_edit::Table::new();
                tbl["value"] = choice_value_item(&c.value, key, i + 1)?;
                tbl["label"] = toml_edit::value(c.label.trim());
                if !c.label_en.trim().is_empty() {
                    tbl["labelEn"] = toml_edit::value(c.label_en.trim());
                }
                if !c.label_ja.trim().is_empty() {
                    tbl["labelJa"] = toml_edit::value(c.label_ja.trim());
                }
                if !c.label_tw.trim().is_empty() {
                    tbl["labelTw"] = toml_edit::value(c.label_tw.trim());
                }
                if c.deprecated {
                    tbl["deprecated"] = toml_edit::value(true);
                }
                if !c.deprecated_machines.is_empty() {
                    tbl["deprecatedMachines"] = toml_edit::value(c.deprecated_machines.join(","));
                }
                arr.push(tbl);
            }
            Some(toml_edit::Item::ArrayOfTables(arr))
        };
        // 适用机型 / 弃用机型：trim、去空、去重。空 = 不限（文件里删键）
        let machine_filter = normalize_machines(&edit.machine_filter);
        let deprecated_machines = normalize_machines(&edit.deprecated_machines);

        /*
         * 所属分组：**两份文件必须一起动**。
         *
         * `param_registry.toml` 这边只说「我在哪个 section」（`[params.layout]`），
         * 真正决定界面上摆在哪的是 `layout_schema.toml` 的 `tabs[].sections[].items[]`。
         * `check_consistency` 第②③条会双侧核对 —— 漏改一边，整份数据下次加载就起不来。
         * 所以这里先在**不改任何东西**的前提下把要改的全算好：目标组合不合法、
         * 新 item id 撞名，都在这一步就拒掉。
         */
        /*
         * 新分组（分组下拉里那个「＋」）先建出来，后面就只剩「搬组」一条路 ——
         * 两份文件的写法不必写第二遍。它会一并进 `self.doc` / `self.layout_doc`，
         * 但**只在整笔成功时才落盘**（`write_back` 在调用方那一层），取消/报错 = 没建。
         * 已经在时是空操作：撤销 / 重做会把同一笔载荷原样重放一次。
         */
        if let Some(fresh) = &edit.new_section {
            self.ensure_section(&fresh.tab_id, &fresh.id, &fresh.label)?;
        }
        let current_section = self.params[at].layout.section_id.clone();
        let target_section = match &edit.new_section {
            /* 目标是刚建的那个 —— 已经在里头的话不算「搬」（同组 = 不动） */
            Some(fresh) => {
                let id = fresh.id.trim();
                (id != current_section).then(|| id.to_owned())
            }
            None => match edit.section_id.as_deref().map(str::trim) {
                Some(s) if !s.is_empty() && s != current_section => Some(s.to_owned()),
                _ => None,
            },
        };
        let requested_order = match edit.order {
            Some(o) if !o.is_finite() => {
                return Err(AppError::invalid_argument(messages::text(
                    "ui.presetdata.registry.327",
                    &[("o", &o.to_string())],
                )));
            }
            other => other,
        };
        let moved_order = match &target_section {
            Some(target) => {
                let section = self
                    .layout
                    .iter()
                    .flat_map(|t| t.sections.iter())
                    .find(|s| &s.id == target)
                    .ok_or_else(|| {
                        AppError::invalid_argument(messages::text(
                            "ui.presetdata.registry.328",
                            &[("target", &target.to_string())],
                        ))
                    })?;
                if section.component.is_some() {
                    return Err(AppError::invalid_argument(messages::text(
                        "ui.presetdata.registry.329",
                        &[("target", &target.to_string())],
                    )));
                }
                // 目标顺序：给了就照给的（界面会按目标组算好）；没给就放到组末
                match requested_order {
                    Some(o) => Some(o),
                    None => {
                        let max = self
                            .params
                            .iter()
                            .filter(|p| p.layout.section_id == *target && p.key != key)
                            .map(|p| p.layout.order)
                            .fold(None, |acc: Option<f64>, o| {
                                Some(acc.map_or(o, |a| a.max(o)))
                            });
                        Some(max.map_or(1.0, |m| m + 1.0))
                    }
                }
            }
            None => None,
        };
        /*
         * 要写进 `[params.layout].order` 的新值：
         *   · 搬组 → 目标组算好的那个；
         *   · 没搬组、但给的顺序与现值不同 → 就用它（上下箭头走这一条）；
         *   · 其余 = 不动（旧载荷不带 `order`，不许被动重排）。
         */
        let new_order = match target_section.as_ref() {
            /* 搬组：目标组算好的那个（显式给的顺序 / 组末） */
            Some(_) => moved_order,
            /* 没搬组、只是上下移 */
            None => match requested_order {
                Some(o) if o != self.params[at].layout.order => Some(o),
                _ => None,
            },
        };
        let moved_item_id = target_section
            .as_ref()
            .map(|_| next_layout_item_id(&self.layout_doc, key));
        // 旧 item 得真的在（不在就是数据已经漂了，别把参数写成「两边都没有」）
        if target_section.is_some()
            && !self
                .layout
                .iter()
                .flat_map(|t| t.sections.iter())
                .find(|s| s.id == current_section)
                .is_some_and(|s| s.items.iter().any(|i| i.param_key == key))
        {
            return Err(AppError::corrupted(messages::text(
                "ui.presetdata.registry.330",
                &[
                    ("current_section", &current_section.to_string()),
                    ("key", key),
                ],
            ))
            .with_detail(::postprocess::diag::messages::text(
                "ui.presetdata.registry.why_layout_mismatch",
                &[],
            )));
        }

        /* ---------- 文件。到这里全是不会失败的赋值 ---------- */
        let t = self.param_table_mut(at, key)?;
        t["label"] = toml_edit::value(label);
        t["desc"] = toml_edit::value(edit.desc.as_str());
        /* 英文两格：载荷没给就原样留着（工作台编辑器还编不到它们，别让保存把它们抹掉） */
        if let Some(v) = &edit.label_en {
            if v.trim().is_empty() {
                t.remove("labelEn");
            } else {
                t["labelEn"] = toml_edit::value(v.trim());
            }
        }
        if let Some(v) = &edit.desc_en {
            if v.trim().is_empty() {
                t.remove("descEn");
            } else {
                t["descEn"] = toml_edit::value(v.trim());
            }
        }
        /* 日 / 繁中：与上面两格同一套语义（None = 不动、空串 = 删键） */
        for (field, key_name) in [
            (&edit.label_ja, "labelJa"),
            (&edit.label_tw, "labelTw"),
            (&edit.desc_ja, "descJa"),
            (&edit.desc_tw, "descTw"),
        ] {
            if let Some(v) = field {
                if v.trim().is_empty() {
                    t.remove(key_name);
                } else {
                    t[key_name] = toml_edit::value(v.trim());
                }
            }
        }
        match &unit {
            Some(u) => t["unit"] = toml_edit::value(u.as_str()),
            None => {
                t.remove("unit");
            }
        }
        t["valueType"] = toml_edit::value(value_type_str(edit.value_type));
        t["uiComponent"] = toml_edit::value(ui_component_str(edit.ui_component));
        t["defaultValue"] = default_item;
        match edit.min {
            Some(m) => t["min"] = toml_edit::value(m),
            None => {
                t.remove("min");
            }
        }
        match edit.max {
            Some(m) => t["max"] = toml_edit::value(m),
            None => {
                t.remove("max");
            }
        }
        match edit.step {
            Some(s) => t["step"] = toml_edit::value(s),
            None => {
                t.remove("step");
            }
        }
        match &edit.parent_key {
            Some(pk) => t["parentKey"] = toml_edit::value(pk.as_str()),
            None => {
                t.remove("parentKey");
            }
        }
        match show_when_item {
            Some(item) => {
                t.insert("showWhen", item);
            }
            None => {
                t.remove("showWhen");
            }
        }
        // 「缺一行 = false」（实测真数据里没有 false 占位）—— 弃用解除就删键
        if edit.deprecated {
            t["deprecated"] = toml_edit::value(true);
        } else {
            t.remove("deprecated");
        }
        if deprecated_machines.is_empty() {
            t.remove("deprecatedMachines");
        } else {
            t["deprecatedMachines"] = toml_edit::value(deprecated_machines.join(","));
        }
        // 可选项与适用机型都是「整表换」—— 增删改哪一条走的都是同一条路
        match choices_arr {
            Some(item) => {
                t["choices"] = item;
            }
            None => {
                t.remove("choices");
            }
        }
        if machine_filter.is_empty() {
            t.remove("machineFilter");
        } else {
            t["machineFilter"] = toml_edit::value(machine_filter.join(","));
        }
        // 搬组 / 调顺序：参数自己的 `[params.layout]` 跟着走（布局文件那一半在下面）
        if let Some(order) = new_order {
            let lt = t
                .get_mut("layout")
                .and_then(toml_edit::Item::as_table_mut)
                .ok_or_else(|| {
                    AppError::corrupted(messages::text(
                        "ui.presetdata.registry.331",
                        &[("key", key)],
                    ))
                })?;
            if let Some(target) = &target_section {
                lt["sectionId"] = toml_edit::value(target.as_str());
            }
            lt["order"] = toml_edit::value(order);
        }

        /* ---------- 布局文件：把条目从旧组搬到新组 ---------- */
        if let (Some(target), Some(item_id)) = (&target_section, moved_item_id) {
            layout_take_item(&mut self.layout_doc, &current_section, key)?;
            layout_push_item(&mut self.layout_doc, target, &item_id, key)?;
            for section in self.layout.iter_mut().flat_map(|t| t.sections.iter_mut()) {
                if section.id == current_section {
                    section.items.retain(|i| i.param_key != key);
                } else if section.id == *target {
                    section.items.push(LayoutItem {
                        id: item_id.clone(),
                        param_key: key.to_owned(),
                    });
                }
            }
            let p = &mut self.params[at];
            p.layout.section_id = target.clone();
            if let Some(order) = moved_order {
                p.layout.order = order;
            }
        } else if let Some(order) = new_order {
            self.params[at].layout.order = order;
        }

        /*
         * 顺序变了（搬组或上下移）：`items[]` 得跟着重排。
         * `check_consistency` 第③条要求两边一致 —— 不重排的话，下一次加载整份数据起不来。
         */
        if new_order.is_some() {
            let section = target_section
                .clone()
                .unwrap_or_else(|| self.params[at].layout.section_id.clone());
            self.layout_reorder_section(&section)?;
        }

        /* ---------- 内存 ---------- */
        /* ★ **改前整份**留一份（2026-11-02）：日志要说出"这一笔改了哪几格"，
        逐格手抄二十几次迟早漏一格，而且文件面已经被上面改掉了 —— 那一份只在内存里。
        取在这里而不是函数开头：中间那些校验失败时它一次都用不上。 */
        let before_def = self.params[at].clone();
        /*
         * 中文与六格译文**改前**各留一份：下面把它们换掉之后，判「中文变了没有」
         * 「旧译文有没有」全靠这几份（见 `mark_translations_stale`）。
         */
        let before_label = self.params[at].label.clone();
        let before_desc = self.params[at].desc.clone();
        let before_langs = (
            self.params[at].label_en.clone(),
            self.params[at].label_ja.clone(),
            self.params[at].label_tw.clone(),
            self.params[at].desc_en.clone(),
            self.params[at].desc_ja.clone(),
            self.params[at].desc_tw.clone(),
        );
        let p = &mut self.params[at];
        p.label = label.to_owned();
        p.desc = edit.desc;
        p.unit = unit;
        p.value_type = edit.value_type;
        p.ui_component = edit.ui_component;
        p.default_value = edit.default_value;
        p.min = edit.min;
        p.max = edit.max;
        p.step = edit.step;
        p.parent_key = edit.parent_key;
        p.show_when = edit.show_when;
        p.deprecated = edit.deprecated;
        p.deprecated_machines = deprecated_machines;
        p.choices = choices;
        p.machine_filter = machine_filter;
        /*
         * 译文台账（2026-11-02）：中文改了、那一册没跟着重写 ⇒ 记一笔「这册过期」。
         * 反过来：这一笔**真的重写了**哪一册（给了非空值且与旧译文不同）⇒ 摘掉那一笔。
         *
         * 判「重写」只看"给了非空的、跟旧的不一样"：撤销 / 重做把同一笔载荷原样重放时
         * 值没变，不清账也不添账（幂等）；空串是"删掉这一册"，不是"重译好了"。
         */
        let stale_before = p.stale_translations.clone();
        for (field, current, key_name) in [
            (&edit.label_en, &mut p.label_en, "labelEn"),
            (&edit.label_ja, &mut p.label_ja, "labelJa"),
            (&edit.label_tw, &mut p.label_tw, "labelTw"),
            (&edit.desc_en, &mut p.desc_en, "descEn"),
            (&edit.desc_ja, &mut p.desc_ja, "descJa"),
            (&edit.desc_tw, &mut p.desc_tw, "descTw"),
        ] {
            if let Some(v) = field {
                let v = v.trim();
                /* 真重写了（给了非空的、跟旧译文不一样）⇒ 这一册不欠账了 */
                if !v.is_empty() && v != current.trim() {
                    p.stale_translations.retain(|x| x != key_name);
                }
                /* 内存也得跟着换 —— 上面那段只写了文件，`reload` 之前读内存的那条路要一致 */
                *current = v.to_owned();
            }
        }
        /*
         * 中文改了、那一册没跟着重写 ⇒ 记一笔「这册过期」。
         * `before` 取的是**旧中文 + 旧的译文**（内存里那两份已经被换掉了），
         * 靠它判"中文真的变了"与"译文本来就有"。
         */
        let before = ParamDef {
            label: before_label,
            desc: before_desc,
            label_en: before_langs.0,
            label_ja: before_langs.1,
            label_tw: before_langs.2,
            desc_en: before_langs.3,
            desc_ja: before_langs.4,
            desc_tw: before_langs.5,
            stale_translations: Vec::new(),
            ..p.clone()
        };
        p.mark_translations_stale(&before);
        if p.stale_translations != stale_before {
            let list = p.stale_translations.clone();
            let t = self.param_table_mut(at, key)?;
            if list.is_empty() {
                t.remove("staleTranslations");
            } else {
                let mut arr = toml_edit::Array::new();
                for v in &list {
                    arr.push(v.as_str());
                }
                t["staleTranslations"] = toml_edit::value(arr);
            }
        }
        /* **这一笔改了哪几格**写进日志（2026-11-02）。以前这条命令在日志里只有一句
        「编辑已进事务 files=1 ops=1」，看不出改的是说明还是出厂默认 ——
        作者排查「我改了没反应」时最缺的正是这一句。 */
        let fields = self.params[at].summary(&before_def);
        if !fields.is_empty() {
            tracing::info!(
                key = %key,
                count = fields.len(),
                fields = %crate::logfmt::render(&fields),
                "参数定义已改"
            );
        }
        Ok(())
    }

    /// **确保这个分组在**：`param_registry.toml` 的 `[[tabs.sections]]` 与
    /// `layout_schema.toml` 的 `[[tabs.sections]]`（`items = []`）各补一条，内存跟着加。
    ///
    /// 已经在了就当无事发生 —— 撤销 / 重做会把同一笔载荷原样重放，那时它已经建好了。
    /// **但不许**借这条路改名或改归属：落在别的页签、或那个 id 是个整块组件，一律拒。
    /// 分组名只有新建的那一版算数（改名是另一件事，这条路上不做）。
    pub(crate) fn ensure_section(
        &mut self,
        tab_id: &str,
        id: &str,
        label: &str,
    ) -> Result<(), AppError> {
        let id = id.trim();
        let label = label.trim();
        /*
         * id 会进两份 TOML、也会进客户端 catalog —— 只收 a-z / 0-9 / `_`。
         * 中文标签没法直接当 id（那不是排版问题，是引用问题），所以界面上是
         * 「名字随便写、id 自动取 custom_N」。
         */
        if id.is_empty()
            || id.len() > 48
            || !id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            return Err(AppError::invalid_argument(messages::text(
                "ui.presetdata.registry.345",
                &[("id", id)],
            )));
        }
        if label.is_empty() || label.chars().count() > 24 {
            return Err(AppError::invalid_argument(messages::text(
                "ui.presetdata.registry.346",
                &[],
            )));
        }
        if !self.tabs.iter().any(|t| t.id == tab_id) {
            return Err(AppError::invalid_argument(messages::text(
                "ui.presetdata.registry.347",
                &[("tab", tab_id)],
            )));
        }
        // 同一个 id 全局只能有一个归属（`check_consistency` ④同一口径）
        if let Some((here, sec)) = self.layout.iter().find_map(|t| {
            t.sections
                .iter()
                .find(|s| s.id == id)
                .map(|s| (t.id.as_str(), s))
        }) {
            if sec.component.is_some() || here != tab_id {
                return Err(AppError::invalid_argument(messages::text(
                    "ui.presetdata.registry.348",
                    &[("id", id), ("tab", here)],
                )));
            }
            return Ok(());
        }

        // 顺序取该页签里最大 +1：新组挂在那个页签的末尾
        let order = self.tabs.iter().find(|t| t.id == tab_id).map_or(0.0, |t| {
            t.sections.iter().map(|s| s.order).fold(0.0_f64, f64::max)
        }) + 1.0;

        /* ---------- 注册表文件：`[[tabs.sections]]` 追加一条 ---------- */
        let tabs = self
            .doc
            .get_mut("tabs")
            .and_then(toml_edit::Item::as_array_of_tables_mut)
            .ok_or_else(|| {
                AppError::corrupted(messages::text(
                    "ui.presetdata.registry.349",
                    &[("tab", tab_id)],
                ))
            })?;
        let tab = find_table_mut(tabs, tab_id).ok_or_else(|| {
            AppError::corrupted(messages::text(
                "ui.presetdata.registry.349",
                &[("tab", tab_id)],
            ))
        })?;
        let mut tbl = toml_edit::Table::new();
        tbl["id"] = toml_edit::value(id);
        tbl["label"] = toml_edit::value(label);
        tbl["order"] = toml_edit::value(order);
        push_section_table(tab, tbl);

        /* ---------- 布局文件：同 id 的 section，`items = []` ---------- */
        let ltabs = self
            .layout_doc
            .get_mut("tabs")
            .and_then(toml_edit::Item::as_array_of_tables_mut)
            .ok_or_else(|| {
                AppError::corrupted(messages::text(
                    "ui.presetdata.registry.350",
                    &[("tab", tab_id)],
                ))
            })?;
        let ltab = find_table_mut(ltabs, tab_id).ok_or_else(|| {
            AppError::corrupted(messages::text(
                "ui.presetdata.registry.350",
                &[("tab", tab_id)],
            ))
        })?;
        let mut ltbl = toml_edit::Table::new();
        ltbl["id"] = toml_edit::value(id);
        /* 空组在真数据里就是内联空数组；`layout_push_item` 认这个形状 */
        ltbl["items"] = toml_edit::value(toml_edit::Array::new());
        push_section_table(ltab, ltbl);

        /* ---------- 内存两半 ---------- */
        if let Some(t) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            t.sections.push(SectionMeta {
                id: id.to_owned(),
                label: label.to_owned(),
                label_en: String::new(),
                label_ja: String::new(),
                label_tw: String::new(),
                description: None,
                description_en: None,
                description_ja: None,
                description_tw: None,
                order,
            });
        }
        if let Some(t) = self.layout.iter_mut().find(|t| t.id == tab_id) {
            t.sections.push(LayoutSection {
                id: id.to_owned(),
                items: Vec::new(),
                component: None,
                show_when: None,
                layout: None,
                width: None,
                visible: None,
            });
        }
        Ok(())
    }

    /// **删掉一个空分组**（2026-11-01 作者：「空的时候可以删除，比如说我建了一个新的，
    /// 不小心建多了也可以删除」）。
    ///
    /// # 只删「空」的
    ///
    /// 「空」= **没有任何参数归它**（注册表 `[params.layout].sectionId` 一条都不指向它）。
    /// 这是唯一能安全删的情形：有参数的分组一删，那些参数就成了没有家的孤儿（客户端那边
    /// 会凭空少一截），要删得先把参数搬走 —— 那是另一件事，不在这一笔下。
    ///
    /// 顺带挡住两类"看起来能删、其实不能"的：
    ///
    ///   · `component` 占位（设置页那 11 张卡、`tower-map`）—— 那是**功能卡**，
    ///     不是参数分组，删了等于删功能；
    ///   · 带 `showWhen` 的组 —— 条件分组是排版契约的一部分，空也只说明「这一档没放参数」。
    ///
    /// # 两份文件一起删
    ///
    /// `param_registry.toml` 的 `[[tabs.sections]]` 与 `layout_schema.toml` 的同 id
    /// section 都要去掉，否则两份文件的 section 集合对不上，「所属分组」下拉与
    /// `param_tabs()` 会各说各的。**只按文本块删**（与 [`swap_section_blocks`] 同一个
    /// 理由）：`toml_edit` 挪 `Table` 会丢 span / 丢嵌套的 `items` 子表。
    ///
    /// 删完把这个页签的 `order` 重排成 `0..n-1` —— 客户端按数组次序排，中间留个号
    /// 不影响显示，但下一次"挪一格"是按新次序重写的，留个空洞只会让 diff 更乱。
    ///
    /// # 译文台账的逃生通道（2026-11-02）
    ///
    /// 台账在 [`ParamRegistry::set_param_meta`] 里**只在值真的变了**时摘（那是撤销/重做
    /// 幂等的前提）。但译文可以由别处写好 —— 直接改文件、或由 AI 按
    /// `scripts/stale-translations.mjs` 的清单一次改一批 —— 那种"改"不经过后端，
    /// 值明明是对的、标记却永远摘不掉。
    ///
    /// 所以另开一条**显式**的路：「我叫你清哪几格，就清哪几格，不看你改了没有」。
    /// 它不是放宽判据 —— `set_param_meta` 那条老规矩一个字没动，撤销/重做照样幂等；
    /// 这里要求调用方**点名**，是因为"替你判定哪几格已经译好了"在前端做不到
    /// （前端没有"旧中文"，判不出译文是不是跟着那一版写的）。
    ///
    /// `keys` 里的名字必须在六册之内，写错一个就整笔拒收（不做"能清几个算几个"）。
    pub fn clear_stale_translations(
        &mut self,
        key: &str,
        keys: &[String],
    ) -> Result<Vec<String>, AppError> {
        let at = *self.index.get(key).ok_or_else(|| {
            AppError::invalid_argument(messages::text(
                "ui.presetdata.registry.303",
                &[("key", key)],
            ))
        })?;
        const NAMES: [&str; 6] = [
            "labelEn", "labelJa", "labelTw", "descEn", "descJa", "descTw",
        ];
        // 先整批校验：名字写错了不许"清了一半"再报错
        for k in keys {
            if !NAMES.contains(&k.as_str()) {
                return Err(AppError::invalid_argument(messages::text(
                    "ui.presetdata.registry.356",
                    &[("k", k), ("names", &NAMES.join(" / "))],
                )));
            }
        }
        let held: Vec<String> = keys.to_vec();
        let p = &mut self.params[at];
        p.stale_translations.retain(|x| !held.contains(x));
        let left = p.stale_translations.clone();

        let t = self.param_table_mut(at, key)?;
        if left.is_empty() {
            t.remove("staleTranslations");
        } else {
            let mut arr = toml_edit::Array::new();
            for v in &left {
                arr.push(v.as_str());
            }
            t["staleTranslations"] = toml_edit::value(arr);
        }
        Ok(left)
    }

    pub fn remove_section(&mut self, section_id: &str) -> Result<(), AppError> {
        let id = section_id.trim();
        let ti = self
            .tabs
            .iter()
            .position(|t| t.sections.iter().any(|s| s.id == id))
            .ok_or_else(|| {
                AppError::invalid_argument(messages::text(
                    "ui.presetdata.registry.351",
                    &[("id", id)],
                ))
            })?;
        let tab_id = self.tabs[ti].id.clone();
        let at = self.tabs[ti]
            .sections
            .iter()
            .position(|s| s.id == id)
            .expect("上面刚判过");

        /* ---------- 三道门：有参数 / 组件卡 / 条件组 ---------- */
        let used = self
            .params
            .iter()
            .filter(|p| p.layout.section_id == id)
            .count();
        if used > 0 {
            return Err(AppError::invalid_argument(messages::text(
                "ui.presetdata.registry.354",
                &[
                    ("label", self.tabs[ti].sections[at].label.as_str()),
                    ("n", &used.to_string()),
                ],
            )));
        }
        let is_component = self
            .layout
            .iter()
            .flat_map(|t| t.sections.iter())
            .find(|s| s.id == id)
            .is_some_and(|s| s.component.is_some() || s.show_when.is_some());
        if is_component {
            return Err(AppError::invalid_argument(messages::text(
                "ui.presetdata.registry.355",
                &[("label", self.tabs[ti].sections[at].label.as_str())],
            )));
        }

        /* ---------- 两份文件：按文本块删（先文件、再内存，删失败就一个字没动） ---------- */
        remove_section_block(&mut self.doc, &tab_id, at)?;
        if let Some(lti) = self.layout.iter().position(|t| t.id == tab_id) {
            if let Some(lat) = self.layout[lti].sections.iter().position(|s| s.id == id) {
                remove_section_block(&mut self.layout_doc, &tab_id, lat)?;
                self.layout[lti].sections.remove(lat);
            }
        }

        /* ---------- 内存 + 重排 order ---------- */
        self.tabs[ti].sections.remove(at);
        for (j, sec) in self.tabs[ti].sections.iter_mut().enumerate() {
            sec.order = j as f64;
        }
        let orders: Vec<f64> = self.tabs[ti].sections.iter().map(|s| s.order).collect();
        set_section_orders(&mut self.doc, &tab_id, &orders);
        Ok(())
    }

    /// **改一个分组的名字 / 位置**（参数台卡片头那颗可点的标题与上下箭头，2026-11-01）。
    ///
    /// # 位置到底是哪一个字段
    ///
    /// 两个读者、两把尺子，这里一次写完、保证同序：
    ///
    ///   · 客户端 `tabsOf` 按 `[[tabs.sections]]` 的**数组次序**排（它不读 `order`）；
    ///   · 工作台卡片按 `SectionMeta.order` 排（`derive::row_sort_key`）。
    ///
    /// 所以挪一格 = 数组里换两位 + 把这个页签的 `order` 按新次序重排成 `0..n-1`
    /// （既有数据本来就是连续整数，重排不会平白改动没挪的组）。
    /// `layout_schema.toml` 的同名数组跟着换 —— 那份不决定显示顺序，但「所属分组」
    /// 下拉按它的次序出，两个文件不该各说各的。
    ///
    /// # 只改中文名
    ///
    /// 另外三册（`labelEn` / `labelJa` / `labelTw`）留在原处：把它们一起改成中文名
    /// 等于把译文弄丢，而这里没有编辑译文的格子。
    pub fn set_section_meta(
        &mut self,
        section_id: &str,
        edit: SectionMetaEdit,
    ) -> Result<(), AppError> {
        /*
         * 顺带新建（抽屉里那颗「＋ 新增分组」）：**先建，再按常规往下改名 / 换位**。
         * `ensure_section` 幂等（已经在就空操作），所以撤销 / 重做重放同一笔没问题。
         */
        if let Some(fresh) = &edit.new_section {
            self.ensure_section(&fresh.tab_id, &fresh.id, &fresh.label)?;
        }
        let id = section_id.trim();
        let ti = self
            .tabs
            .iter()
            .position(|t| t.sections.iter().any(|s| s.id == id))
            .ok_or_else(|| {
                AppError::invalid_argument(messages::text(
                    "ui.presetdata.registry.351",
                    &[("id", id)],
                ))
            })?;
        /*
         * 带 `component` 的分组（`tower-map` 那张、设置页那 11 张）**与普通分组同权**：
         * 改名、换位都放行（2026-11-01 作者：「组件卡跟别的一样呀，只是它的样子不一样
         * 而已」）。名字落在 `[[tabs.sections]].label`，位置落在数组次序 —— 与有没有
         * 画布无关，别在数据层替界面决定"哪些能改"。
         */
        let tab_id = self.tabs[ti].id.clone();
        let at = self.tabs[ti]
            .sections
            .iter()
            .position(|s| s.id == id)
            .expect("上面刚判过");

        /* ---------- 改名 ---------- */
        if let Some(label) = edit.label.as_deref() {
            let label = label.trim();
            if label.is_empty() || label.chars().count() > 24 {
                return Err(AppError::invalid_argument(messages::text(
                    "ui.presetdata.registry.346",
                    &[],
                )));
            }
            if label != self.tabs[ti].sections[at].label {
                self.tabs[ti].sections[at].label = label.to_owned();
                if let Some(tbl) = section_table_mut(&mut self.doc, &tab_id, id) {
                    tbl.insert("label", toml_edit::value(label));
                }
            }
        }

        /* ---------- 换位置 ---------- */
        if let Some(dir) = edit.move_dir {
            let n = self.tabs[ti].sections.len();
            let to = match dir {
                SectionMove::Up => at.checked_sub(1),
                SectionMove::Down => (at + 1 < n).then_some(at + 1),
            };
            let Some(to) = to else {
                return Err(AppError::invalid_argument(messages::text(
                    "ui.presetdata.registry.353",
                    &[("label", self.tabs[ti].sections[at].label.as_str())],
                )));
            };
            /*
             * **先改文件、再改内存**：文本块对调失败就直接报错退出，内存一个字没动，
             * 上层也不会 `write_back` —— 不会出现"内存换了、盘上没换"的半截状态。
             * （2026-11-01 的血泪：反过来的顺序 + 换表内容的算法，把布局文件搅错了。）
             */
            swap_section_blocks(&mut self.doc, &tab_id, at, to)?;
            /* 布局那份按 id 找自己的位置（两份的数组未必一一对应：组件占位只在布局里） */
            let layout_swap = self
                .layout
                .iter()
                .position(|t| t.id == tab_id)
                .and_then(|lti| {
                    let lat = self.layout[lti].sections.iter().position(|s| s.id == id)?;
                    let lto = match dir {
                        SectionMove::Up => lat.checked_sub(1),
                        SectionMove::Down => {
                            (lat + 1 < self.layout[lti].sections.len()).then_some(lat + 1)
                        }
                    }?;
                    Some((lti, lat, lto))
                });
            if let Some((_, lat, lto)) = layout_swap {
                swap_section_blocks(&mut self.layout_doc, &tab_id, lat, lto)?;
            }

            /* 两份文件都改完，才动内存 */
            self.tabs[ti].sections.swap(at, to);
            if let Some((lti, lat, lto)) = layout_swap {
                self.layout[lti].sections.swap(lat, lto);
            }

            /* order 跟着数组位置走 —— 卡片头与客户端读的是同一次改动 */
            for (j, sec) in self.tabs[ti].sections.iter_mut().enumerate() {
                sec.order = j as f64;
            }
            let orders: Vec<f64> = self.tabs[ti].sections.iter().map(|s| s.order).collect();
            set_section_orders(&mut self.doc, &tab_id, &orders);
        }

        Ok(())
    }

    /// 把一个 section 的 `items[]` 重排成与 `[params.layout].order` 一致（文件 + 内存两半）。
    ///
    /// 改顺序、搬组之后都要过这一道：`check_consistency` 第③条要求
    /// 「items 顺序 == 按 order 升序」，漏了它下一次加载整份数据就起不来。
    /// 同 order 时保持注册表里的原有次序（稳定排序）。
    fn layout_reorder_section(&mut self, section_id: &str) -> Result<(), AppError> {
        let mut pairs: Vec<(&str, f64)> = self
            .params
            .iter()
            .filter(|p| p.layout.section_id == section_id)
            .map(|p| (p.key.as_str(), p.layout.order))
            .collect();
        pairs.sort_by(|a, b| a.1.total_cmp(&b.1));
        let want: Vec<String> = pairs.iter().map(|(k, _)| (*k).to_owned()).collect();

        layout_reorder_items(&mut self.layout_doc, section_id, &want);

        // 内存那一半：按 want 里的位次排，不在表里的（理论上没有）沉到最后
        let pos: BTreeMap<&str, usize> = want
            .iter()
            .enumerate()
            .map(|(i, k)| (k.as_str(), i))
            .collect();
        for section in self.layout.iter_mut().flat_map(|t| t.sections.iter_mut()) {
            if section.id == section_id {
                section
                    .items
                    .sort_by_key(|i| pos.get(i.param_key.as_str()).copied().unwrap_or(usize::MAX));
            }
        }
        Ok(())
    }

    /// 文件里第 `at` 个 `[[params]]` 块。
    ///
    /// 下标通用是因为 `self.params` 与文件里的顺序来自**同一次解析**；
    /// 不在文件里按 key 再找一遍是刻意的：两处各查一次就有两个答案的可能
    fn param_table_mut(&mut self, at: usize, key: &str) -> Result<&mut toml_edit::Table, AppError> {
        let file = self.file.display().to_string();
        let arr = self
            .doc
            .get_mut("params")
            .and_then(toml_edit::Item::as_array_of_tables_mut)
            .ok_or_else(|| {
                AppError::corrupted(messages::text(
                    "ui.presetdata.registry.332",
                    &[("file", &file.to_string())],
                ))
            })?;
        let n = arr.len();
        let t = arr.get_mut(at).ok_or_else(|| {
            AppError::corrupted(messages::text(
                "ui.presetdata.registry.333",
                &[("file", &file.to_string()), ("n", &n.to_string())],
            ))
        })?;
        // 反查一次 key：下标错位的话，值会被写到**另一个字段**上，而那是静默的
        let here = t.get("key").and_then(|v| v.as_str()).unwrap_or_default();
        if here != key {
            return Err(AppError::corrupted(messages::text(
                "ui.presetdata.registry.334",
                &[
                    ("file", &file.to_string()),
                    ("at", &at.to_string()),
                    ("here", here),
                    ("key", key),
                ],
            ))
            .with_detail(::postprocess::diag::messages::text(
                "ui.presetdata.registry.why_order_mismatch",
                &[],
            )));
        }
        Ok(t)
    }

    pub fn file(&self) -> &Path {
        &self.file
    }

    pub fn layout_file(&self) -> &Path {
        &self.layout_file
    }

    /// 写回用的文本。**没改过就逐字节等于读进来那份**
    pub fn to_toml(&self) -> String {
        self.doc.to_string()
    }

    pub fn layout_to_toml(&self) -> String {
        self.layout_doc.to_string()
    }

    /// 布局里一共排了多少个条目 —— 它应当等于"可见字段数"
    pub fn layout_item_count(&self) -> usize {
        self.layout
            .iter()
            .flat_map(|t| t.sections.iter())
            .map(|s| s.items.len())
            .sum()
    }

    pub fn write_back(&self) -> Result<(), AppError> {
        atomic_write(&self.file, self.to_toml().as_bytes())?;
        atomic_write(&self.layout_file, self.layout_to_toml().as_bytes())
    }

    /// 能装参数的分组（`component` 整块组件除外），按布局里的页签顺序排好。
    ///
    /// 给「所属分组」下拉用：`(tab_id, tab_label, section_id, section_label)`。
    /// **不过滤空组** —— 把一个组搬空之后还得能搬回去。
    pub fn layout_sections(&self) -> Vec<(String, String, String, String)> {
        self.layout_sections_all()
            .into_iter()
            .filter(|(_, _, _, _, component)| component.is_none())
            .map(|(tab_id, tab_label, id, label, _)| (tab_id, tab_label, id, label))
            .collect()
    }

    /// 布局里的**全部分组**，含 `component` 整块组件卡（最后那一格是组件名）。
    ///
    /// 分组抽屉要它：组件卡也有卡头、也有参数（`tower-map` 那张），点得开才说得清
    /// 它在哪儿。**「所属分组」下拉仍然只认 `layout_sections()`** ——
    /// 后端 `move_param` 拒收组件卡（`ui.presetdata.registry.329`），下拉里就不该摆它。
    pub fn layout_sections_all(&self) -> Vec<(String, String, String, String, Option<String>)> {
        let mut out = Vec::new();
        for lt in &self.layout {
            let Some(meta) = self.tabs.iter().find(|t| t.id == lt.id) else {
                continue;
            };
            /*
             * 抽屉这侧的分组顺序，必须与参数台卡片那侧（`derive::row_sort_key` 用的
             * 是 `SectionMeta.order`）完全一致 —— 否则「支撑」在左边是第 6 个、在右侧
             * 抽屉里是第 3 个，作者 2026-11-01 就是被这个不一致搞晕的。
             * 两份文件里，注册表那份带 `order`，而且换位置时我们把它重排成了数组下标，
             * 所以这里按注册表里的 `order` 排序（找不到就排到最后），两处就永远对齐了。
             */
            let mut rows: Vec<(f64, String, String, String)> = lt
                .sections
                .iter()
                .map(|s| {
                    let order = meta
                        .sections
                        .iter()
                        .find(|m| m.id == s.id)
                        .map_or(f64::INFINITY, |m| m.order);
                    let label = meta
                        .sections
                        .iter()
                        .find(|m| m.id == s.id)
                        .map_or_else(|| s.id.clone(), |m| m.label.clone());
                    (
                        order,
                        s.id.clone(),
                        label,
                        s.component.clone().unwrap_or_default(),
                    )
                })
                .collect();
            rows.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            for (_, id, label, component) in rows {
                out.push((
                    lt.id.clone(),
                    meta.label.clone(),
                    id,
                    label,
                    if component.is_empty() {
                        None
                    } else {
                        Some(component)
                    },
                ));
            }
        }
        out
    }
}

/// 机型清单归一：trim、去空、去重。`machineFilter` 与 `deprecatedMachines` 同一套
fn normalize_machines(ids: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for id in ids {
        let id = id.trim();
        if !id.is_empty() && !out.iter().any(|x| x == id) {
            out.push(id.to_owned());
        }
    }
    out
}

/// 布局文件里一个 section 的表（`tabs[].sections[]` 里 id 命中的那一节）
/// 在 `[[tabs]]` 里按 `id` 找那一张表（`ensure_section` 用）
fn find_table_mut<'a>(
    tabs: &'a mut toml_edit::ArrayOfTables,
    id: &str,
) -> Option<&'a mut toml_edit::Table> {
    tabs.iter_mut()
        .find(|t| t.get("id").and_then(|v| v.as_str()) == Some(id))
}

/// 往某个页签的 `sections` 末尾追加一张表。
///
/// 缺键、或那一格不是表数组时**就地换成表数组** —— 与 `layout_push_item` 同一套写法。
fn push_section_table(tab: &mut toml_edit::Table, tbl: toml_edit::Table) {
    match tab.get_mut("sections") {
        Some(item) if item.is_array_of_tables() => {
            if let Some(arr) = item.as_array_of_tables_mut() {
                arr.push(tbl);
            }
        }
        Some(item) => {
            let mut arr = toml_edit::ArrayOfTables::new();
            arr.push(tbl);
            *item = toml_edit::Item::ArrayOfTables(arr);
        }
        None => {
            let mut arr = toml_edit::ArrayOfTables::new();
            arr.push(tbl);
            tab.insert("sections", toml_edit::Item::ArrayOfTables(arr));
        }
    }
}

/// 某个页签里某一张 `[[tabs.sections]]` 表（注册表与布局两份文件同一套写法）。
/// 找不到就 `None` —— 调用方多半只是"这一份里没有"，不是错误
fn section_table_mut<'a>(
    doc: &'a mut DocumentMut,
    tab_id: &str,
    section_id: &str,
) -> Option<&'a mut toml_edit::Table> {
    let tabs = doc.get_mut("tabs")?.as_array_of_tables_mut()?;
    let tab = find_table_mut(tabs, tab_id)?;
    let arr = tab.get_mut("sections")?.as_array_of_tables_mut()?;
    arr.iter_mut()
        .find(|t| t.get("id").and_then(|v| v.as_str()) == Some(section_id))
}

/// 把某个页签里 `[[tabs.sections]]` 的第 `from` / `to` **整块**对调。
///
/// # 为什么按文本块换，而不是挪 `Table`
///
/// `toml_edit` 0.22 里 section 表**自带嵌套的 `[[tabs.sections.items]]`**（一个
/// `ArrayOfTables` 套着另一个）。两条路都试过，都会坏数据：
///
///   · `mem::take` 出来再 `push` 回去 —— 表带着旧 span，序列化时**丢表**；
///   · 把两张表的**内容**对调 —— `items` 这个子表留在原地，只有 `id` 换过去，
///     于是 A 的名字挂上了 B 的参数。**2026-11-01 真发生过**：布局文件被搅成
///     `custom_1` 名下挂着 `safety_advanced` 的 7 条参数，`presets check` 报
///     「两侧的参数数对不上」。
///
/// 换文本块最稳：块里的注释、`items`、空行都原样跟着走，解析结果不可能错位。
/// 代价是要把整个文档重新 parse 一次 —— 但这条命令本来就不在热路径上。
fn swap_section_blocks(
    doc: &mut DocumentMut,
    tab_id: &str,
    from: usize,
    to: usize,
) -> Result<(), AppError> {
    if from == to {
        return Ok(());
    }
    let text = doc.to_string();
    let region = tab_region(&text, tab_id).ok_or_else(|| {
        AppError::corrupted(messages::text(
            "ui.presetdata.registry.349",
            &[("tab", tab_id)],
        ))
    })?;
    let (head, body) = text.split_at(region.start);
    let (chunk, tail) = body.split_at(region.len());

    /* 切块：每个 `[[tabs.sections]]` 是一块的开始，块到下一个开始（或区域尾）为止 */
    let marker = "[[tabs.sections]]";
    let mut cuts: Vec<usize> = Vec::new();
    let mut at = 0;
    while let Some(i) = chunk[at..].find(marker) {
        cuts.push(at + i);
        at += i + marker.len();
    }
    if from >= cuts.len() || to >= cuts.len() {
        return Err(AppError::corrupted(messages::text(
            "ui.presetdata.registry.349",
            &[("tab", tab_id)],
        )));
    }
    let mut blocks: Vec<&str> = Vec::new();
    for (i, cut) in cuts.iter().enumerate() {
        let end = cuts.get(i + 1).copied().unwrap_or(chunk.len());
        blocks.push(&chunk[*cut..end]);
    }
    blocks.swap(from, to);

    let new_chunk = format!("{}{}{}", &chunk[..cuts[0]], blocks.concat(), "");
    let new_text = format!("{head}{new_chunk}{tail}");
    *doc = new_text.parse::<DocumentMut>().map_err(|e| {
        AppError::corrupted(messages::text(
            "ui.presetdata.registry.349",
            &[("tab", tab_id), ("id", &e.to_string())],
        ))
    })?;
    Ok(())
}

/// `[[tabs]] id = '<tab_id>'` 那一段在全文里的位置（到下一个 `[[tabs]]` 为止）
fn tab_region(text: &str, tab_id: &str) -> Option<std::ops::Range<usize>> {
    let header = "[[tabs]]";
    let mut from = 0;
    while let Some(i) = text[from..].find(header) {
        let start = from + i;
        let next = text[start + header.len()..]
            .find(header)
            .map_or(text.len(), |j| start + header.len() + j);
        let chunk = &text[start..next];
        let hit = chunk.lines().any(|l| {
            let l = l.trim();
            l == format!("id = '{tab_id}'") || l == format!("id = \"{tab_id}\"")
        });
        if hit {
            return Some(start..next);
        }
        from = next;
    }
    None
}

/// 把某个页签里第 `at` 个 `[[tabs.sections]]` **整块**删掉（文本块级，不动兄弟）。
///
/// 与 [`swap_section_blocks`] 同一套切块法：一个块从 `[[tabs.sections]]` 起到下一个
/// 同样的表头（或页签区域结束）为止 —— 嵌套的 `[[tabs.sections.items]]` 是**同一块**
/// 里的子表头，不是新的切点，所以整块删才不可能只删掉表头留下孤儿 items。
///
/// 要防的一个坑：块尾可能连着下一条 section 前的空行 / 注释，删多了会把下一条的
/// 隔行注释带走。所以删完一段后把**尾部只到最后一个非空行为止**，剩下的留给下一条。
fn remove_section_block(doc: &mut DocumentMut, tab_id: &str, at: usize) -> Result<(), AppError> {
    let text = doc.to_string();
    let region = tab_region(&text, tab_id).ok_or_else(|| {
        AppError::corrupted(messages::text(
            "ui.presetdata.registry.349",
            &[("tab", tab_id)],
        ))
    })?;
    let (head, body) = text.split_at(region.start);
    let (chunk, tail) = body.split_at(region.len());

    let marker = "[[tabs.sections]]";
    let mut cuts: Vec<usize> = Vec::new();
    let mut from = 0;
    while let Some(i) = chunk[from..].find(marker) {
        cuts.push(from + i);
        from += i + marker.len();
    }
    if at >= cuts.len() {
        return Err(AppError::corrupted(messages::text(
            "ui.presetdata.registry.349",
            &[("tab", tab_id)],
        )));
    }
    let start = cuts[at];
    let end = cuts.get(at + 1).copied().unwrap_or(chunk.len());

    /*
     * 整块删掉，**连它后面的空行一起收**。
     *
     * 早先写的是"只吃到最后一个非空行"，想给下一条留隔行空行 —— 那个想法本身没错，
     * 但它和后一句 `&chunk[start + keep..]` 合起来是错的：`keep` 停在块尾的换行处，
     * 于是剩下的 `"\n\n[[tabs.sections]]"` 被接在了**块的起点**上，多出来的空行堆在
     * 上一条与下一条之间。几百行之后 `parse` 还能过，但 `check_consistency` 那条
     * 「items 顺序 == 按 order 升序」按文本块切的时候就会被空行搅乱 ——
     * 真机表现是**删完之后命令一直不回来**（重新加载时卡在校验里）。
     *
     * 现在：块连同它后面的空白行一起收掉，下一条自己的换行由它自己带。
     */
    let mut stop = end;
    while stop < chunk.len() && (chunk.as_bytes()[stop] == b'\n' || chunk.as_bytes()[stop] == b'\r')
    {
        stop += 1;
    }
    let new_chunk = format!("{}{}", &chunk[..start], &chunk[stop..]);
    let new_text = format!("{head}{new_chunk}{tail}");
    *doc = new_text.parse::<DocumentMut>().map_err(|e| {
        AppError::corrupted(messages::text(
            "ui.presetdata.registry.349",
            &[("tab", tab_id), ("id", &e.to_string())],
        ))
    })?;
    Ok(())
}

/// 按**新次序**把这个页签每个分组的 `order` 逐张写回（`orders[i]` 对应第 i 张表）
fn set_section_orders(doc: &mut DocumentMut, tab_id: &str, orders: &[f64]) {
    let Some(tabs) = doc
        .get_mut("tabs")
        .and_then(toml_edit::Item::as_array_of_tables_mut)
    else {
        return;
    };
    let Some(tab) = find_table_mut(tabs, tab_id) else {
        return;
    };
    let Some(arr) = tab
        .get_mut("sections")
        .and_then(toml_edit::Item::as_array_of_tables_mut)
    else {
        return;
    };
    for (i, tbl) in arr.iter_mut().enumerate() {
        if let Some(o) = orders.get(i) {
            tbl.insert("order", toml_edit::value(*o));
        }
    }
}

fn layout_section_mut<'a>(
    doc: &'a mut DocumentMut,
    section_id: &str,
) -> Option<&'a mut toml_edit::Table> {
    let tabs = doc.get_mut("tabs")?.as_array_of_tables_mut()?;
    for t in tabs.iter_mut() {
        let Some(secs) = t
            .get_mut("sections")
            .and_then(toml_edit::Item::as_array_of_tables_mut)
        else {
            continue;
        };
        for s in secs.iter_mut() {
            if s.get("id").and_then(|v| v.as_str()) == Some(section_id) {
                return Some(s);
            }
        }
    }
    None
}

/// 布局文件里全局唯一的 item id。命名只要求唯一 —— 真数据里 `item_5_3_3` 那种
/// 序号并不是由 section 下标严格算出来的（`machine_specific` 又从 `_0` 起）
fn next_layout_item_id(doc: &DocumentMut, key: &str) -> String {
    let mut used: BTreeSet<&str> = BTreeSet::new();
    if let Some(tabs) = doc
        .get("tabs")
        .and_then(toml_edit::Item::as_array_of_tables)
    {
        for t in tabs.iter() {
            let Some(secs) = t
                .get("sections")
                .and_then(toml_edit::Item::as_array_of_tables)
            else {
                continue;
            };
            for s in secs.iter() {
                let Some(items) = s.get("items").and_then(toml_edit::Item::as_array_of_tables)
                else {
                    continue;
                };
                for it in items.iter() {
                    if let Some(id) = it.get("id").and_then(|v| v.as_str()) {
                        used.insert(id);
                    }
                }
            }
        }
    }
    let base = key.replace('.', "_");
    let mut n = 1;
    loop {
        let cand = format!("item_{base}_{n}");
        if !used.contains(cand.as_str()) {
            return cand;
        }
        n += 1;
    }
}

/// 从某个 section 的 `items` 里摘掉一条（不改别的条目、不动原文排版）
fn layout_take_item(
    doc: &mut DocumentMut,
    section_id: &str,
    param_key: &str,
) -> Result<(), AppError> {
    let section = layout_section_mut(doc, section_id).ok_or_else(|| {
        AppError::corrupted(messages::text(
            "ui.presetdata.registry.335",
            &[("section_id", section_id)],
        ))
    })?;
    let Some(items) = section
        .get_mut("items")
        .and_then(toml_edit::Item::as_array_of_tables_mut)
    else {
        return Err(AppError::corrupted(messages::text(
            "ui.presetdata.registry.336",
            &[("section_id", section_id)],
        )));
    };
    let mut kept = toml_edit::ArrayOfTables::new();
    let mut hit = false;
    for it in items.iter() {
        if it.get("paramKey").and_then(|v| v.as_str()) == Some(param_key) {
            hit = true;
            continue;
        }
        kept.push(it.clone());
    }
    if !hit {
        return Err(AppError::corrupted(messages::text(
            "ui.presetdata.registry.337",
            &[("section_id", section_id), ("param_key", param_key)],
        )));
    }
    *items = kept;
    Ok(())
}

/// 往某个 section 的 `items` 末尾追加一条。空组的 `items = []` 是**内联空数组**，
/// 不是表数组 —— 那种形状要先换成 `ArrayOfTables` 才塞得进表条目
fn layout_push_item(
    doc: &mut DocumentMut,
    section_id: &str,
    item_id: &str,
    param_key: &str,
) -> Result<(), AppError> {
    let section = layout_section_mut(doc, section_id).ok_or_else(|| {
        AppError::corrupted(messages::text(
            "ui.presetdata.registry.338",
            &[("section_id", section_id)],
        ))
    })?;
    let mut tbl = toml_edit::Table::new();
    tbl["id"] = toml_edit::value(item_id);
    tbl["paramKey"] = toml_edit::value(param_key);
    match section.get_mut("items") {
        Some(item) if item.is_array_of_tables() => {
            if let Some(arr) = item.as_array_of_tables_mut() {
                arr.push(tbl);
            }
        }
        Some(item) => {
            let mut arr = toml_edit::ArrayOfTables::new();
            arr.push(tbl);
            *item = toml_edit::Item::ArrayOfTables(arr);
        }
        None => {
            let mut arr = toml_edit::ArrayOfTables::new();
            arr.push(tbl);
            section.insert("items", toml_edit::Item::ArrayOfTables(arr));
        }
    }
    Ok(())
}

/// 按 `want` 给的 key 顺序重排某个 section 的 `items[]`。
///
/// **只改每格里的 `paramKey`，不搬动表格本身**。
///
/// 为什么不整组重建：`toml_edit` 的表数组条目带着**文档位置**，把克隆出来的表塞回
/// 一个新建的数组会让同一条被印两遍（2026-10-20 实测：`insert("items", …)` 之后
/// 文件里出现 6 条）。逐格换 key 既不改结构、也不动排版 —— `id` 只是唯一标识，
/// 谁配谁没有语义，换个配对不违反任何判据。
fn layout_reorder_items(doc: &mut DocumentMut, section_id: &str, want: &[String]) {
    let Some(section) = layout_section_mut(doc, section_id) else {
        return;
    };
    /* 真数据是 `[[tabs.sections.items]]`（表数组）；手写源可能是内联数组 —— 两种都认 */
    if let Some(items) = section
        .get_mut("items")
        .and_then(toml_edit::Item::as_array_of_tables_mut)
    {
        /* 数量对不上说明数据已经漂了：宁可不动，也不要把条目改成对不上号的 */
        if items.len() != want.len() {
            return;
        }
        for (i, tbl) in items.iter_mut().enumerate() {
            tbl["paramKey"] = toml_edit::value(want[i].as_str());
        }
        return;
    }
    if let Some(arr) = section
        .get_mut("items")
        .and_then(toml_edit::Item::as_array_mut)
    {
        if arr.len() != want.len() {
            return;
        }
        for (i, v) in arr.iter_mut().enumerate() {
            if let Some(t) = v.as_inline_table_mut() {
                t.insert("paramKey", toml_edit::Value::from(want[i].as_str()));
            }
        }
    }
    /* `items = []`（组件占位那种内联空数组）没有可排的，什么都不做 */
}

/// 校验 owner 的形状 —— 非空、不含空白
fn check_owner(owner: &str) -> Result<(), AppError> {
    if owner.trim().is_empty() || owner.contains(char::is_whitespace) {
        return Err(AppError::invalid_argument(messages::text(
            "ui.presetdata.registry.339",
            &[("owner", owner)],
        )));
    }
    Ok(())
}

/// 把一个 `serde_json::Value` 转成 `toml_edit::Item`。
///
/// `valueType` 决定数字的表示：`int` 写整数，其余写浮点。
/// **这一条是为了不让表示漂**：真数据里 `valueType = 'float'` 的字段写的是 `0.0`，
/// 而旧的那份 JSON 产物把它印成了 `0`（见 `domain::variants` 里那条对齐判据）。
/// 从这边写回去时要跟着 TOML 的口径，不是跟着 JSON 的。
///
/// 文本（含多行 G-code）走 `literal_str`：单引号优先，含控制字符时退回双引号 + 转义
fn to_toml_value(
    v: &serde_json::Value,
    vt: ValueType,
    key: &str,
) -> Result<toml_edit::Item, AppError> {
    let bad_type = || {
        AppError::invalid_argument(messages::text(
            "ui.presetdata.registry.340",
            &[
                ("key", key),
                ("v", &v.to_string()),
                ("vt", &format!("{vt:?}")),
            ],
        ))
    };
    match v {
        serde_json::Value::Bool(b) => Ok(toml_edit::value(*b)),
        serde_json::Value::Number(n) => match vt {
            ValueType::Int => n.as_i64().map(toml_edit::value).ok_or_else(bad_type),
            _ => n.as_f64().map(toml_edit::value).ok_or_else(bad_type),
        },
        serde_json::Value::String(s) => Ok(super::literal_str(s)),
        _ => Err(bad_type()),
    }
}

/// `ValueType` 写回文件时的原文。与 serde 的 `rename_all = "lowercase"` +
/// `Text → "string"` 一口同音 —— 手写两份就该是一份的意思
fn value_type_str(v: ValueType) -> &'static str {
    match v {
        ValueType::Float => "float",
        ValueType::Int => "int",
        ValueType::Bool => "bool",
        ValueType::Text => "string",
    }
}

/// `UiComponent` 写回文件时的原文（serde `rename_all = "lowercase"`）
fn ui_component_str(v: UiComponent) -> &'static str {
    match v {
        UiComponent::Number => "number",
        UiComponent::Switch => "switch",
        UiComponent::Segmented => "segmented",
        UiComponent::Select => "select",
        UiComponent::Gcode => "gcode",
        UiComponent::Text => "text",
    }
}

/// `ShowOp` 写回文件时的原文（serde `rename_all = "lowercase"`）
fn show_op_str(op: ShowOp) -> &'static str {
    match op {
        ShowOp::Eq => "eq",
        ShowOp::Neq => "neq",
        ShowOp::Gt => "gt",
    }
}

/// `showWhen.value` 写回文件的那一项。与 `to_toml_value` 不同：**不按某个
/// valueType 归一** —— 前置条件的值跟着被指向字段走，而实测里就有字符串化的
/// 数字（`"0"`），原样保留才是忠实转写；`visibility` 比较前会自己归一
fn scalar_item(v: &Value, key: &str) -> Result<toml_edit::Item, AppError> {
    match v {
        Value::String(s) => Ok(super::literal_str(s)),
        Value::Bool(b) => Ok(toml_edit::value(*b)),
        Value::Number(n) => n.as_f64().map(toml_edit::value).ok_or_else(|| {
            AppError::invalid_argument(messages::text(
                "ui.presetdata.registry.341",
                &[("key", key), ("v", &v.to_string())],
            ))
        }),
        _ => Err(AppError::invalid_argument(messages::text(
            "ui.presetdata.registry.342",
            &[("key", key), ("v", &v.to_string())],
        ))),
    }
}

/// 可选项的值写回文件的那一项。与 `scalar_item` 只差一处：**整数按整数写**
/// （`0` 不许漂成 `0.0`）—— 可选项是照抄原文的东西，表示一动就天天出无谓 diff
fn choice_value_item(v: &Value, key: &str, n: usize) -> Result<toml_edit::Item, AppError> {
    match v {
        Value::String(s) => Ok(super::literal_str(s)),
        Value::Bool(b) => Ok(toml_edit::value(*b)),
        Value::Number(num) => match num.as_i64() {
            Some(i) => Ok(toml_edit::value(i)),
            None => num.as_f64().map(toml_edit::value).ok_or_else(|| {
                AppError::invalid_argument(messages::text(
                    "ui.presetdata.registry.343",
                    &[("key", key), ("n", &n.to_string()), ("v", &v.to_string())],
                ))
            }),
        },
        _ => Err(AppError::invalid_argument(messages::text(
            "ui.presetdata.registry.344",
            &[("key", key), ("n", &n.to_string()), ("v", &v.to_string())],
        ))),
    }
}

/// 源数据把机型清单写成 `"A1,A1_MINI,A2L"` 这种逗号串。
///
/// 拆的时候顺手 trim 并丢掉空段 —— 现在没有 `"A1, A1_MINI"` 这种带空格的写法，
/// 但一个多打的空格会让 `"A1_MINI"` 变成 `" A1_MINI"`，然后这台机型的字段静默消失
/// `machineFilter` 的读取：**两种输入形状都认**。
///
/// TOML 源里是**逗号分隔字符串**（`"A1,A1_MINI,A2L"`）—— 手写源的形状；
/// catalog（新世界两端共用的 JSON 契约）里是**字符串数组** —— `Vec<String>` 序列化的
/// 自然产物。只认前者的话，catalog 往返（parse 自己产出的 JSON）当场断，
/// definition 就进不了共用契约 —— 这不是宽松，是序列化与反序列化必须说同一种话。
fn comma_separated<'de, D>(d: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let v: Value = Value::deserialize(d)?;
    match v {
        Value::Null => Ok(Vec::new()),
        Value::String(s) => Ok(s
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect()),
        Value::Array(items) => items
            .into_iter()
            .map(|i| match i {
                Value::String(s) => Ok(s),
                other => Err(serde::de::Error::custom(format!(
                    "machineFilter 数组里只能是字符串，见到：{other}"
                ))),
            })
            .collect(),
        other => Err(serde::de::Error::custom(format!(
            "machineFilter 要么是逗号分隔字符串，要么是字符串数组，见到：{other}"
        ))),
    }
}

/// 选项值 / `showWhen` 值的**匹配键**。两边都是注册表里的 JSON 标量，
/// 字符串直接比原文；数字归一成 f64 再比 —— `1` 与 `1.0` 在 `Value` 的相等
/// 判断里是两个数，但在注册表里说的是同一档。
fn json_key(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n
            .as_f64()
            .map(|f| f.to_string())
            .unwrap_or_else(|| n.to_string()),
        other => other.to_string(),
    }
}

/// 测试夹具：把一份 JSON 形状的字段定义 + 布局写成 TOML，再走真的 loader 读回来。
///
/// 为什么让测试写 JSON 而不是直接写 TOML：这两份数据的键名本来就一样
/// （`content/*.json` 就是从这两个 TOML 构建出来的），而 `serde_json::json!`
/// 能就地改一个键（`p["params"][0]["step"] = …`），TOML 文本做不到。
/// **生产路径永远只读 TOML**，这个入口只在 `cfg(test)` 下存在
#[cfg(test)]
pub fn load_from_json_fixture(
    root: &Path,
    params: &Value,
    layout: &Value,
) -> Result<ParamRegistry, AppError> {
    let w = |rel: &str, v: &Value| {
        let text = toml::to_string(v).expect("夹具那份 JSON 应该能表示成 TOML");
        atomic_write(&root.join(rel), text.as_bytes()).unwrap();
    };
    w("registry/param_registry.toml", params);
    w("layout_schema.toml", layout);
    ParamRegistry::load_from(root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presetdata::one_edit_only;

    /* ---------- 值的写法跟着 valueType 走（2026-11-12） ---------- */

    /// 最小注册表：三个参数，分别覆盖「int 写成浮点」「int 真的写了小数」「float 写成整数」。
    ///
    /// 只走 [`RegistryFile::parse_str`] —— 不需要布局文件，这一条只盯"读进来是什么类型"。
    const MIXED_TYPES: &str = "\
[[params]]
key = 'w.int'
configKey = 'A'
tomlKey = 'a'
jsonKey = 'a'
label = 'A'
desc = ''
tomlComment = ''
valueType = 'int'
uiComponent = 'number'
defaultValue = 1.0
scope = 'universal'
section = 'motion'

[[params.choices]]
label = 'Two'
value = 2.0

[params.machineVariants]
'A1' = 30.0

[params.layout]
sectionId = 'motion'
order = 0.0

[[params]]
key = 'w.int.bad'
configKey = 'B'
tomlKey = 'b'
jsonKey = 'b'
label = 'B'
desc = ''
tomlComment = ''
valueType = 'int'
uiComponent = 'number'
defaultValue = 1.5
scope = 'universal'
section = 'motion'

[params.layout]
sectionId = 'motion'
order = 1.0

[[params]]
key = 'w.float'
configKey = 'C'
tomlKey = 'c'
jsonKey = 'c'
label = 'C'
desc = ''
tomlComment = ''
valueType = 'float'
uiComponent = 'number'
defaultValue = 0
scope = 'universal'
section = 'motion'

[params.layout]
sectionId = 'motion'
order = 2.0
";

    /// **`int` 参数读进来必须是整数**：默认值、机型变体、选项值三处都归一；
    /// 非整值原样留着让写值层报错；`float` 参数一点都不碰。
    ///
    /// 这条咬的是真事故（2026-11-12）：`wiping.glue_pass_count = 1.0` 被原样交给
    /// `to_toml_value`，`int` 只认 `as_i64()` ⇒ 复制配方整批拒绝、一项都没写进去。
    #[test]
    fn int_values_written_as_integral_floats_are_read_back_as_integers() {
        let f = RegistryFile::parse_str(MIXED_TYPES).expect("夹具读得通");
        let p = f.params.iter().find(|p| p.key == "w.int").expect("在");

        assert_eq!(
            p.default_value,
            Value::from(1),
            "int 默认值 1.0 该读成整数 1，实测 {:?}",
            p.default_value
        );
        assert!(
            p.default_value.is_i64(),
            "必须是整数 JSON number，不是浮点：{:?}",
            p.default_value
        );
        assert_eq!(p.machine_variants.get("A1"), Some(&Value::from(30)));
        assert_eq!(p.choices[0].value, Value::from(2));

        let bad = f.params.iter().find(|p| p.key == "w.int.bad").expect("在");
        assert_eq!(
            bad.default_value,
            serde_json::json!(1.5),
            "非整值不是写法问题，是真写错了 —— 不许悄悄截断"
        );

        let fl = f.params.iter().find(|p| p.key == "w.float").expect("在");
        assert_eq!(
            fl.default_value,
            Value::from(0),
            "float 参数不在这条归一里，整数写法原样保留"
        );
    }

    /// 夹具 → `ParamRegistry`（**不走文件**）：原文 token 的闸只看 `doc`，
    /// 不需要布局那份文件，也就不必为这一条再造一棵临时树。
    fn registry_with_doc(text: &str) -> ParamRegistry {
        let file = RegistryFile::parse_str(text).expect("夹具读得通");
        let doc: toml_edit::DocumentMut = text.parse().expect("夹具是合法 TOML");
        let index = file
            .params
            .iter()
            .enumerate()
            .map(|(i, p)| (p.key.clone(), i))
            .collect();
        ParamRegistry {
            params: file.params,
            tabs: Vec::new(),
            layout: Vec::new(),
            updated: String::new(),
            index,
            doc,
            file: std::path::PathBuf::new(),
            layout_doc: toml_edit::DocumentMut::default(),
            layout_file: std::path::PathBuf::new(),
            schema_fingerprint_cache: std::sync::OnceLock::new(),
        }
    }

    /// **闸看原文**：模型已归一，只有 `doc` 还留着 `1.0` 这种写法。
    ///
    /// 三处都要报：默认值、机型变体、选项值；`1.5` 也要报（且 `integral = false`，
    /// 说明它得先改数值，normalize 写不正）。
    #[test]
    fn raw_int_literals_reports_decimal_spellings_for_int_params() {
        let reg = registry_with_doc(MIXED_TYPES);
        let bad = reg.raw_int_literals();
        assert_eq!(
            bad.len(),
            4,
            "应报 1.0 / 30.0 / 2.0 / 1.5 四处，实测 {bad:?}"
        );
        assert!(
            bad.iter().any(|b| b.key == "w.int"
                && b.owner.is_empty()
                && b.literal == "1.0"
                && b.integral),
            "默认值那一处没报到：{bad:?}"
        );
        assert!(
            bad.iter().any(|b| b.owner == "A1" && b.literal == "30.0"),
            "机型变体那一处没报到：{bad:?}"
        );
        assert!(
            bad.iter()
                .any(|b| b.owner == "choices[0]" && b.literal == "2.0"),
            "选项值那一处没报到：{bad:?}"
        );
        let non_integral = bad
            .iter()
            .find(|b| b.key == "w.int.bad")
            .expect("1.5 也是 int 写成小数，要报");
        assert!(!non_integral.integral, "1.5 不是整值，不能标成可自动修");
    }

    /// 全写成整数 ⇒ 一处都不报（闸不该误伤）。
    #[test]
    fn raw_int_literals_is_empty_when_int_values_are_integers() {
        let text = MIXED_TYPES
            .replace("defaultValue = 1.0", "defaultValue = 1")
            .replace("'A1' = 30.0", "'A1' = 30")
            .replace("value = 2.0", "value = 2")
            /* `1.5` 不是写法问题，真改成整数再验"干净" */
            .replace("defaultValue = 1.5", "defaultValue = 2");
        let reg = registry_with_doc(&text);
        let bad = reg.raw_int_literals();
        assert!(bad.is_empty(), "干净源不该报，实测 {bad:?}");
    }

    /// **真仓库一个都不能有** —— 这条就是「以后别再写回去」的闸。
    #[test]
    fn the_real_registry_has_no_decimal_spellings_for_int_params() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../presets");
        let reg = ParamRegistry::load_from(&root).expect("真仓库的注册表要读得通");
        let bad = reg.raw_int_literals();
        assert!(bad.is_empty(), "源里还有 int 参数写成小数：{bad:?}");
    }

    /// **真仓库里那四个 `int` 参数的默认值必须是整数**（源数据回归）。
    ///
    /// 它们曾经写成 `1.0 / 30.0 / 50.0 / 0.0`。归一能让运行时不再出错，但源文件
    /// 该是整数 —— 这条钉住"别再写回去"。
    #[test]
    fn the_real_registry_keeps_int_defaults_as_integers() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../presets");
        let reg = ParamRegistry::load_from(&root).expect("真仓库的注册表要读得通");
        for key in [
            "wiping.glue_pass_count",
            "wiping.prime_trigger_layers",
            "wiping.sheath_converge_layers",
            "wiping.user_dry_time",
        ] {
            let p = reg.param(key).unwrap_or_else(|| panic!("{key} 在"));
            assert!(
                p.default_value.is_i64(),
                "{key} 的 defaultValue 写成了浮点：{:?}",
                p.default_value
            );
        }
    }

    /* ---------- 改回去不许留下字节差（2026-11-02） ---------- */

    /// **70 → 75 → 70 之后，文件必须一字不差地回到原样。**
    ///
    /// 作者的原话：「改回原值之后，文件应恢复到原始字节状态，不应因为格式归一化产生
    /// 『未保存 1』」。这条咬的正是那个副产物：归一化删键之后表空了，以前会把
    /// `[params.machineVariants]` 这个**原文里就有的空表头**（下面还挂着一行说明注释）
    /// 一起删掉 ⇒ 文件少两行 ⇒ 事务记一条改动，而值一点没变。
    #[test]
    fn writing_a_value_and_clearing_it_again_restores_the_original_bytes() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../presets");
        let mut reg = ParamRegistry::load_from(&root).expect("真仓库的注册表要读得通");
        /* 这一项原文里带空表 + 一段「空表是刻意的」注释 —— 正是最容易丢表头的那一类 */
        let key = "toolhead.speed_limit";
        let owner = "A1:FASTV3.3";
        let before = reg.to_toml();
        assert!(
            before.contains("[params.machineVariants]"),
            "夹具前提变了：这一项本来该有一个空表头"
        );

        reg.set_variant(key, owner, &serde_json::json!(75.0))
            .expect("写一个与继承值不同的值");
        assert_ne!(reg.to_toml(), before, "写了值之后文件应当变了");

        reg.clear_variant(key, owner).expect("删键挂回继承");
        assert_eq!(
            reg.to_toml(),
            before,
            "改回去之后字节该一模一样 —— 差一行就会变成一条假的「未保存改动」"
        );
    }

    /// **临时建出来的表，保存一轮之后也得能删干净**（2026-11-02）。
    ///
    /// 第一版判据是"原文里有没有这张表"，于是留了个口子：
    ///
    /// ```
    /// ① 某参数原文里没有表 → set_variant 临时建表 → 写进 A1
    /// ② 保存 → reload → 重建判据，此时文件里**有**这张表 ⇒ 记成"原文就有"
    /// ③ 再改回去 → 认为"原文就有" ⇒ 留下 `[params.machineVariants]` 空表头
    /// ```
    ///
    /// 现在只有**原文里真有机型键**才留表头；空表头一律当"本次临时建的"删掉。
    #[test]
    fn a_table_built_by_this_session_is_removed_even_after_a_reload() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("registry")).unwrap();
        /* 手写一份最小注册表：一个参数，**没有** machineVariants 表 */
        crate::fsx::atomic::atomic_write(
            &root.join("registry/param_registry.toml"),
            b"updated = '2026-11-02'\n\
             \n\
             [[params]]\n\
             key = 'a.b'\n\
             label = 'B'\n\
             desc = ''\n\
             configKey = 'B'\n\
             tomlKey = 'b'\n\
             jsonKey = 'b'\n\
             tomlComment = ''\n\
             valueType = 'float'\n\
             uiComponent = 'number'\n\
             defaultValue = 70.0\n\
             section = 'motion'\n\
             scope = 'universal'\n\
             \n\
             [params.layout]\n\
             sectionId = 'motion'\n\
             order = 0.0\n",
        )
        .unwrap();
        /* 布局里得有一张 section 收下这条参数，否则一致性校验会拦（"有参数没有被任何
        section 收下"） */
        crate::fsx::atomic::atomic_write(
            &root.join("layout_schema.toml"),
            b"[[tabs]]\n\
             id = 't'\n\
             label = 'T'\n\
             \n\
             [[tabs.sections]]\n\
             id = 'motion'\n\
             label = 'Motion'\n\
             \n\
             [[tabs.sections.items]]\n\
             id = 'a.b'\n\
             key = 'a.b'\n\
             paramKey = 'a.b'\n",
        )
        .unwrap();

        let mut reg = ParamRegistry::load_from(root).expect("最小注册表要读得通");
        let clean = reg.to_toml();
        assert!(!clean.contains("machineVariants"), "前提：原文里没有这张表");

        /* ① 临时建表 + 写值 */
        reg.set_variant("a.b", "A1", &serde_json::json!(75.0))
            .unwrap();
        assert!(reg.to_toml().contains("machineVariants"));

        /* ② 保存一轮：把内容写回盘、再重新 load（正是 `Presets::reload` 做的事）。
        这里直接**写注册表文件**：`write_back` 会连布局一起写，而这份夹具的布局是手写的
        最小版，写回去会被布局模型挑出缺字段 —— 与本节要验的东西无关。
        写文件走 `fsx::atomic::atomic_write`：生产代码禁用裸写（写盘纪律），
        测试也不必破例。 */
        crate::fsx::atomic::atomic_write(
            &root.join("registry/param_registry.toml"),
            reg.to_toml().as_bytes(),
        )
        .unwrap();
        let mut reg = ParamRegistry::load_from(root).expect("写回去的那份要能再读");

        /* ③ 改回去 —— 这时表头必须整段消失 */
        reg.clear_variant("a.b", "A1").unwrap();
        let text = reg.to_toml();
        assert!(
            !text.contains("machineVariants"),
            "本次临时建出来的表，清空后要连表头一起删 —— 否则每存一轮就多一处空表头\n{text}"
        );
    }

    /// 反方向：**原文里真有覆盖**的参数，清空最后一条也要把表头留着（作者手写的注释跟着走）。
    #[test]
    fn a_table_that_really_had_values_keeps_its_header() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../presets");
        let mut reg = ParamRegistry::load_from(&root).expect("真仓库的注册表要读得通");
        let key = "toolhead.speed_limit";
        let before = reg.to_toml();
        let owner = reg
            .param(key)
            .and_then(|p| p.machine_variants.keys().next().cloned());
        let Some(owner) = owner else {
            return; /* 这一项手上没有覆盖 —— 换一条也无所谓，本条判据的前提不成立 */
        };
        reg.clear_variant(key, &owner).unwrap();
        assert!(
            reg.to_toml().contains("[params.machineVariants]"),
            "原文里真有覆盖的参数，清空后表头要留（注释跟着走）"
        );
        let _ = before;
    }

    /* ---------- 夹具上的门禁（任何机器上都执行） ---------- */

    fn param(key: &str, section: &str, order: f64) -> Value {
        serde_json::json!({
            "key": key, "configKey": "X", "tomlKey": key, "jsonKey": key,
            "label": key, "desc": "", "tomlComment": "",
            "valueType": "float", "uiComponent": "number", "defaultValue": 0,
            "scope": "universal", "section": "toolhead",
            "layout": { "order": order, "sectionId": section }
        })
    }

    fn good() -> (Value, Value) {
        (
            serde_json::json!({
                "params": [param("a.x", "s1", 2.0), param("a.y", "s1", 1.0)],
                "tabs": [{
                    "id": "t1", "label": "页签一", "order": 10,
                    "sections": [{ "id": "s1", "label": "分组一", "order": 0 }]
                }],
                "updated": "2026-01-01 00:00:00"
            }),
            // items 顺序照 order 升序：y(1.0) 在 x(2.0) 前面
            serde_json::json!({
                "tabs": [{ "id": "t1", "sections": [{
                    "id": "s1",
                    "items": [{ "id": "i0", "paramKey": "a.y" }, { "id": "i1", "paramKey": "a.x" }]
                }] }]
            }),
        )
    }

    fn load(params: Value, layout: Value) -> (tempfile::TempDir, Result<ParamRegistry, AppError>) {
        let d = tempfile::tempdir().unwrap();
        let r = load_from_json_fixture(d.path(), &params, &layout);
        (d, r)
    }

    #[test]
    fn loads_a_consistent_pair() {
        let (p, l) = good();
        let (_d, r) = load(p, l);
        let r = r.unwrap();
        assert_eq!(r.params().len(), 2);
        assert_eq!(r.updated(), "2026-01-01 00:00:00");
        assert!(r.param("a.x").is_some());
        assert!(r.param("nope").is_none());
    }

    /// 缺席：有参数没被任何 section 收下 —— 它在界面上根本不会出现
    #[test]
    fn missing_param_in_layout_is_rejected() {
        let (mut p, l) = good();
        p["params"]
            .as_array_mut()
            .unwrap()
            .push(param("a.z", "s1", 3.0));
        let (_d, r) = load(p, l);
        let e = r.unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::Corrupted);
        assert!(
            e.detail.unwrap_or_default().contains("a.z"),
            "没点名真因 —— 说明句里应当列出那个没被收下的参数"
        );
    }

    /// 悬空：布局引用了字段定义里没有的参数
    #[test]
    fn dangling_param_key_is_rejected() {
        let (p, mut l) = good();
        l["tabs"][0]["sections"][0]["items"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({ "id": "i9", "paramKey": "ghost" }));
        let (_d, r) = load(p, l);
        let e = r.unwrap_err();
        // 判定口径与语言无关：那个悬着的 key 必须出现在说明句里
        assert!(
            e.detail.unwrap_or_default().contains("ghost"),
            "说明句里应当点名那个悬空的 paramKey"
        );
    }

    /// 同一个参数被放进两个 section
    #[test]
    fn duplicate_param_key_is_rejected() {
        let (p, mut l) = good();
        l["tabs"][0]["sections"][0]["items"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({ "id": "i9", "paramKey": "a.x" }));
        let (_d, r) = load(p, l);
        assert!(r.unwrap_err().detail.unwrap_or_default().contains("a.x"));
    }

    /// items 顺序与 `layout.order` 不一致 —— 这是那种"调错了界面上看不出来"的错
    #[test]
    fn item_order_mismatch_is_rejected() {
        let (p, mut l) = good();
        let items = l["tabs"][0]["sections"][0]["items"].as_array_mut().unwrap();
        items.swap(0, 1);
        let (_d, r) = load(p, l);
        let e = r.unwrap_err();
        assert!(e.message.contains("顺序"), "{}", e.message);
    }

    /// 逗号串拆数组：trim 掉空格、丢掉空段。
    /// 一个多打的空格会让 `" A1_MINI"` 匹配不上，然后这台机型的字段静默消失
    #[test]
    fn machine_filter_is_split_and_trimmed() {
        let (mut p, l) = good();
        p["params"][0]["machineFilter"] = serde_json::json!("A1, A1_MINI ,,A2L");
        let (_d, r) = load(p, l);
        let r = r.unwrap();
        let f = &r.param("a.x").unwrap().machine_filter;
        assert_eq!(f, &["A1", "A1_MINI", "A2L"]);
    }

    /// 不带 `machineFilter` = 不限机型
    #[test]
    fn empty_machine_filter_applies_everywhere() {
        let (p, l) = good();
        let (_d, r) = load(p, l);
        let d = r.unwrap();
        let p = d.param("a.x").unwrap();
        assert!(p.machine_filter.is_empty());
        assert!(p.applies_to("A1") && p.applies_to("随便什么"));
    }

    /// `visible_keys` 两道过滤都要生效，且按 order 升序
    #[test]
    fn visible_keys_filters_and_sorts() {
        let (mut p, mut l) = good();
        p["params"][0]["machineFilter"] = serde_json::json!("P1S"); // a.x 只给 P1S
        p["params"][1]["deprecated"] = serde_json::json!(true); // a.y 废弃
                                                                // 补一个既不限机型也没废弃的，用来证明剩下的那个真的在
        p["params"]
            .as_array_mut()
            .unwrap()
            .push(param("a.w", "s1", 0.5));
        l["tabs"][0]["sections"][0]["items"] = serde_json::json!([
            { "id": "i0", "paramKey": "a.w" },
            { "id": "i1", "paramKey": "a.y" },
            { "id": "i2", "paramKey": "a.x" }
        ]);
        let (_d, r) = load(p, l);
        let r = r.unwrap();

        assert_eq!(
            r.visible_keys("A1"),
            vec!["a.w"],
            "A1 只该看到不限机型且未废弃的"
        );
        assert_eq!(
            r.visible_keys("P1S"),
            vec!["a.w", "a.x"],
            "P1S 还该看到点名给它的那个，且按 order 升序"
        );
    }

    /// 参数台这一屏**不隐藏弃用项**（C14 §五）：划线 + 禁用，不是藏起来。
    /// 与 `visible_keys` 唯一的差别就在这一条 —— 生成侧照旧看不见它们。
    #[test]
    fn desk_keys_keeps_deprecated_params_visible() {
        let (mut p, l) = good();
        p["params"][1]["deprecated"] = serde_json::json!(true); // a.y 废弃
        let (_d, r) = load(p, l);
        let r = r.unwrap();

        assert_eq!(
            r.visible_keys("A1"),
            vec!["a.x"],
            "生成侧照旧不带走弃用的参数"
        );
        assert_eq!(
            r.desk_keys("A1"),
            vec!["a.y", "a.x"],
            "参数台要看得见弃用的参数（划线那一条的前提）"
        );
    }

    /// 选项级弃用判据（C14 §五，§4-2）：**这一档放开的参数全弃用了（且至少一条）**
    /// 才算弃用。上游自己标的选项级标记照旧算数。
    #[test]
    fn deprecated_choice_is_derived_from_its_gated_family() {
        let mut p = serde_json::json!({
            "params": [
                {
                    "key": "frame.type", "configKey": "Frame", "tomlKey": "type", "jsonKey": "type",
                    "label": "外围结构", "desc": "", "tomlComment": "",
                    "valueType": "string", "uiComponent": "segmented", "defaultValue": "brim",
                    "scope": "universal", "section": "s1",
                    "layout": { "order": 1, "sectionId": "s1" },
                    "choices": [
                        { "label": "斜肋外墙", "value": "brim" },
                        { "label": "护套", "value": "sheath" },
                        { "label": "半护套", "value": "half" },
                        { "label": "旧写法", "value": "legacy", "deprecated": true }
                    ]
                },
                // half 放开 1 条、那条没弃用 → half 不标（every 不许把「还有活的」读成「全都弃用」）
                {
                    "key": "frame.half_only", "configKey": "X", "tomlKey": "ho", "jsonKey": "ho",
                    "label": "半护套专属", "desc": "", "tomlComment": "",
                    "valueType": "float", "uiComponent": "number", "defaultValue": 0,
                    "scope": "universal", "section": "s1",
                    "layout": { "order": 2.0, "sectionId": "s1" },
                    "showWhen": { "key": "frame.type", "op": "eq", "value": "half" }
                },
            ],
            "tabs": [{
                "id": "t1", "label": "页签一", "order": 10,
                "sections": [{ "id": "s1", "label": "分组一", "order": 0 }]
            }],
            "updated": "2026-01-01 00:00:00"
        });
        // 护套放开的 2 条，全弃用 → 「护套」该被推出来
        for key in ["frame.shell.speed", "frame.shell.height"] {
            p["params"].as_array_mut().unwrap().push(serde_json::json!({
                "key": key, "configKey": "X", "tomlKey": key, "jsonKey": key,
                "label": key, "desc": "", "tomlComment": "",
                "valueType": "float", "uiComponent": "number", "defaultValue": 0,
                "scope": "universal", "section": "s1",
                "layout": { "order": 3.0, "sectionId": "s1" },
                "deprecated": true,
                "showWhen": { "key": "frame.type", "op": "eq", "value": "sheath" }
            }));
        }
        // brim 放开 1 条但没弃用 → brim 不标
        p["params"].as_array_mut().unwrap().push(serde_json::json!({
            "key": "frame.brim_only", "configKey": "X", "tomlKey": "bo", "jsonKey": "bo",
            "label": "brim 专属", "desc": "", "tomlComment": "",
            "valueType": "float", "uiComponent": "number", "defaultValue": 0,
            "scope": "universal", "section": "s1",
            "layout": { "order": 4.0, "sectionId": "s1" },
            "showWhen": { "key": "frame.type", "op": "eq", "value": "brim" }
        }));
        let l = serde_json::json!({
            "tabs": [{ "id": "t1", "sections": [{
                "id": "s1",
                "items": [
                    { "id": "i0", "paramKey": "frame.type" },
                    { "id": "i1", "paramKey": "frame.half_only" },
                    { "id": "i2", "paramKey": "frame.shell.speed" },
                    { "id": "i3", "paramKey": "frame.shell.height" },
                    { "id": "i4", "paramKey": "frame.brim_only" }
                ]
            }] }]
        });
        let (_d, r) = load(p, l);
        let r = r.unwrap();

        let dead = r.deprecated_choice_values("frame.type");
        assert!(dead.contains("sheath"), "放开的 2 条全弃用 → 这一档弃用");
        assert!(dead.contains("legacy"), "上游自己标的照旧算数");
        assert!(!dead.contains("brim"), "放开的东西里还有能改的 → 不标");
        assert!(!dead.contains("half"), "空集不许被 every 读成「全都」");
        // 不存在的参数：空集，不是 panic
        assert!(r.deprecated_choice_values("ghost").is_empty());
    }

    /// 只装参数的 section 才进矩阵分类；空 section（component 占位）要被过滤掉
    #[test]
    fn param_tabs_drops_component_only_sections() {
        let (mut p, mut l) = good();
        p["tabs"].as_array_mut().unwrap().push(serde_json::json!({
            "id": "settings", "label": "软件设置", "order": 80,
            "sections": [{ "id": "appearance", "label": "外观", "order": 100 }]
        }));
        l["tabs"].as_array_mut().unwrap().push(serde_json::json!({
            "id": "settings",
            "sections": [{ "id": "appearance", "items": [], "component": "appearance-card" }]
        }));
        let (_d, r) = load(p, l);
        let r = r.unwrap();

        assert_eq!(r.param_section_ids().len(), 1, "只有 s1 装参数");
        let tabs = r.param_tabs();
        assert_eq!(tabs.len(), 1, "settings 整个 tab 都该被滤掉");
        assert_eq!(tabs[0].id, "t1");
        assert_eq!(tabs[0].label, "页签一", "中文名取 [[tabs]]");
    }

    /// 分组抽屉要的是**全部分组**（含 `component` 整块组件卡）：
    /// 组件卡的卡头也点得开，看不见就问不出"它在哪儿"。
    /// 「所属分组」下拉仍然走 `layout_sections()`（组件卡不许当搬家目标）。
    #[test]
    fn layout_sections_all_keeps_component_cards() {
        let (mut p, mut l) = good();
        p["tabs"].as_array_mut().unwrap().push(serde_json::json!({
            "id": "settings", "label": "软件设置", "order": 80,
            "sections": [{ "id": "appearance", "label": "外观", "order": 100 }]
        }));
        l["tabs"].as_array_mut().unwrap().push(serde_json::json!({
            "id": "settings",
            "sections": [{ "id": "appearance", "items": [], "component": "appearance-card" }]
        }));
        let (_d, r) = load(p, l);
        let r = r.unwrap();

        let all = r.layout_sections_all();
        let card = all
            .iter()
            .find(|(_, _, id, _, _)| id == "appearance")
            .expect("组件卡没进全量表 —— 它的卡头就点不开了");
        assert_eq!(card.4.as_deref(), Some("appearance-card"), "组件名要带出去");
        assert_eq!(card.3, "外观", "名字取 [[tabs.sections]] 的中文名");

        let plain = r.layout_sections();
        assert!(
            !plain.iter().any(|(_, _, id, _)| id == "appearance"),
            "普通那份不能把组件卡混进来 —— 后端 move_param 拒收，下拉里不许摆它"
        );
    }

    /// 分组元数据：中文名与组内序都从 `[[tabs]]` 来，查不到就是 `None`。
    /// 调用方拿 `None` 退化成显示 section id，而不是显示一个空字符串
    #[test]
    fn section_meta_falls_back_to_none_for_unknown_id() {
        let (p, l) = good();
        let (_d, r) = load(p, l);
        let r = r.unwrap();
        let s = r.section_meta("s1").expect("s1 声明过");
        assert_eq!(s.label, "分组一");
        assert_eq!(s.order, 0.0);
        assert!(r.section_meta("不存在的组").is_none());
    }

    /// 未声明的页签要排到**最后**。给 0 的话它会插到所有页签前面，
    /// 于是随手加一个 tab 就把整张矩阵的行序掀翻
    #[test]
    fn tab_order_puts_unknown_tabs_last() {
        let (p, l) = good();
        let (_d, r) = load(p, l);
        let r = r.unwrap();
        assert_eq!(r.tab_order("t1"), 10.0);
        assert_eq!(r.tab_order("ghost"), f64::MAX);
        assert!(r.tab_order("t1") < r.tab_order("ghost"));
    }

    /// 定义指纹：改一个**定义**字段就要变（否则改了字段定义产物不会过期）
    #[test]
    fn schema_fingerprint_reacts_to_definition_changes() {
        let (p, l) = good();
        let (_d1, a) = load(p.clone(), l.clone());
        let before = a.unwrap().schema_fingerprint();

        let mut p2 = p;
        p2["params"][0]["step"] = serde_json::json!(0.05);
        let (_d2, b) = load(p2, l);
        assert_ne!(before, b.unwrap().schema_fingerprint());
    }

    /// 【回归 · 2026-10-05】值覆盖（`machineVariants`）是**数据**不是定义 ——
    /// 改它不许动定义指纹。旧版把整张表哈希进指纹，作者改了 `A1:FASTV3.3`
    /// 一个值，9 份产物的 uuid 全变、全部「待更新」。
    /// 顺带钉住分界的另一半：范围覆盖（`machineMinVariants`）是**定义的机型特化**，
    /// 改它就要变。
    #[test]
    fn schema_fingerprint_ignores_value_overrides_but_not_range_overrides() {
        let (p, l) = good();
        let (_d1, a) = load(p.clone(), l.clone());
        let before = a.unwrap().schema_fingerprint();

        let mut p2 = p.clone();
        p2["params"][0]["machineVariants"] = serde_json::json!({ "A1:FASTV3.3": 70 });
        let (_d2, b) = load(p2, l.clone());
        assert_eq!(
            before,
            b.unwrap().schema_fingerprint(),
            "别家的值泼进了定义指纹 —— 指纹粒度又错了"
        );

        let mut p3 = p;
        p3["params"][0]["machineMinVariants"] = serde_json::json!({ "A1": 1 });
        let (_d3, c) = load(p3, l);
        assert_ne!(
            before,
            c.unwrap().schema_fingerprint(),
            "范围覆盖是定义的机型特化，改了要变"
        );
    }

    /// 指纹缓存（2026-10-02 的性能修复）：反复取**逐字节相同**，重载后仍跟着数据走。
    ///
    /// 守两件事：
    ///   · 缓存不许改变返回值 —— 同一个注册表取一万次都是同一个串（`book_view` 里按版本
    ///     反复取，取错一次「已生成 / 待生成」就全乱）；
    ///   · 缓存是**每份注册表各一份**，不是全局 —— 重载后数据变了，指纹必须跟着变
    ///     （否则改了字段定义，产物永远显示「已生成」）。
    #[test]
    fn fingerprint_is_cached_but_per_registry() {
        let (p, l) = good();
        let (_d1, a) = load(p.clone(), l.clone());
        let a = a.unwrap();
        let first = a.schema_fingerprint();

        // 同一个实例反复取：稳定
        for _ in 0..1000 {
            assert_eq!(a.schema_fingerprint(), first);
        }

        // 另一份、同样数据：值相同（缓存不是"随机盐"）
        let (_d2, b) = load(p.clone(), l.clone());
        assert_eq!(b.unwrap().schema_fingerprint(), first);

        // 重载后数据改了：新实例的指纹跟着变（缓存没有跨实例粘住旧值）
        let mut p2 = p;
        p2["params"][0]["step"] = serde_json::json!(0.05);
        let (_d3, c) = load(p2, l);
        assert_ne!(c.unwrap().schema_fingerprint(), first);
    }

    /// 文件不在时是 NOT_FOUND 且写明是哪个文件 —— 不是静默空表
    #[test]
    fn missing_files_are_not_found() {
        let d = tempfile::tempdir().unwrap();
        let e = ParamRegistry::load_from(d.path()).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::NotFound);
    }

    /* ---------- 真数据（只在定位到 presets/ 的机器上执行） ---------- */

    fn real() -> Option<ParamRegistry> {
        let root = crate::presetdata::repo_presets_root()?;
        Some(ParamRegistry::load_from(&root).expect("字段定义或布局读不通"))
    }

    /// 75 条字段全读到，而且关键字段真的落进模型 ——
    /// 只断言条数的话，一个全是默认值的空壳也能通过
    #[test]
    fn all_registry_params_load_with_their_fields() {
        let Some(r) = real() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        assert_eq!(r.params().len(), 92, "字段条数");
        assert!(!r.tabs().is_empty());
        assert!(!r.updated().is_empty(), "updated 没读到");

        // 一条普通数值字段：单位、范围、步进、界面归属
        let p = r.param("toolhead.MKP_retract").expect("回抽长度必须在");
        assert_eq!(p.label, "回抽长度");
        assert_eq!(p.toml_key, "MKP_retract");
        assert_eq!(p.unit.as_deref(), Some("mm"));
        assert_eq!(p.min, Some(-50.0));
        assert_eq!(p.max, Some(50.0));
        assert_eq!(p.layout.section_id, "motion");

        // 四个子表各至少有一条真数据 —— 它们是最容易在换 loader 时静默丢掉的部分
        assert!(
            r.params().iter().any(|p| !p.choices.is_empty()),
            "[[params.choices]] 一条都没读到"
        );
        assert!(
            r.params().iter().any(|p| p.show_when.is_some()),
            "[params.showWhen] 一条都没读到"
        );
        assert!(
            r.params().iter().any(|p| !p.machine_variants.is_empty()),
            "[params.machineVariants] 一条都没读到"
        );
        assert!(
            r.params().iter().any(|p| !p.machine_filter.is_empty()),
            "machineFilter 没被拆成数组"
        );
    }

    /// 布局的三层结构读通，条目数与可见字段数对得上。
    ///
    /// 三条一致性断言在 `load_from` 里已经跑过（双射 / 每组计数 / items 顺序），
    /// 所以这里只补"反空转"与"每个 paramKey 都指向真字段"
    #[test]
    fn the_layout_lists_every_visible_param_once() {
        let Some(r) = real() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        assert!(r.layout_item_count() > 0, "布局里一个条目都没有");
        assert!(!r.param_tabs().is_empty(), "一个装参数的 tab 都没有");
        assert_eq!(
            r.layout_item_count(),
            r.params().len(),
            "布局条目数与字段数不等 —— 双射断言本该先炸，它在空转"
        );
    }

    /// 往返保真：这两个文件也归这条判据管。
    ///
    /// `param_registry.toml` 是 56 KB、含多行 G-code 字符串与一堆浮点，
    /// **它是保真最容易破的地方** —— 如果哪天换回"反序列化再重新序列化"，
    /// 这条会第一个红
    #[test]
    fn registry_and_layout_survive_a_round_trip() {
        let Some(r) = real() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        for (path, round) in [
            (r.file(), r.to_toml()),
            (r.layout_file(), r.layout_to_toml()),
        ] {
            let original = std::fs::read_to_string(path).expect("刚读过的文件");
            assert_eq!(
                original,
                round,
                "{} 读进来再写出去变了。差异从第 {} 个字节开始",
                path.display(),
                original
                    .bytes()
                    .zip(round.bytes())
                    .position(|(a, b)| a != b)
                    .unwrap_or(original.len().min(round.len())),
            );
        }
    }

    /// 反空转：多行 G-code 字符串是保真的硬骨头，专门盯一条
    #[test]
    fn a_multiline_gcode_value_is_preserved_verbatim() {
        let Some(r) = real() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let p = r
            .params()
            .iter()
            .find(|p| !p.machine_variants.is_empty() && p.toml_key.contains("gcode"))
            .expect("装载/卸载 G-code 那两条必须在");
        let v = p
            .machine_variants
            .values()
            .find_map(|v| v.as_str())
            .expect("至少一个机型变体是字符串");
        assert!(v.contains('\n'), "这条本该是多行 G-code：{v:?}");
        // 原文里它是带 \n 转义的双引号字符串；写回之后那一段必须还在
        assert!(r.to_toml().contains("G92 E0"), "写回之后 G-code 内容不见了");
    }

    /// **我建的模型有没有把真数据的字段读丢。**
    ///
    /// 逐条比对文件里每个 `[[params]]` 的键集与这份清单：少读一个字段在这里会现形，
    /// 而条数变化不会。清单里那两个"刻意不读"的要写明原因，否则下一个人会以为是漏的
    #[test]
    fn every_field_in_the_real_file_is_either_read_or_listed() {
        let Some(root) = crate::presetdata::repo_presets_root() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let text = std::fs::read_to_string(root.join("registry").join("param_registry.toml"))
            .expect("读得到");
        let doc: DocumentMut = text.parse().expect("合法 TOML");
        let known: BTreeSet<&str> = [
            "key",
            "configKey",
            "tomlKey",
            "jsonKey",
            "label",
            "labelEn",
            "labelJa",
            "labelTw",
            "descEn",
            "descJa",
            "descTw",
            "desc",
            /* 译文台账（2026-11-02）：中文改了、那册没重写时登记在这 */
            "staleTranslations",
            "tomlComment",
            "valueType",
            "uiComponent",
            "defaultValue",
            "scope",
            "section",
            "layout",
            "min",
            "max",
            "step",
            "unit",
            "parentKey",
            "showWhen",
            "choices",
            "variantMode",
            "machineFilter",
            "deprecated",
            "machineVariants",
            "machineMinVariants",
            "machineMaxVariants",
            // 「这一版跟不跟基底」（2026-11-03）；读进 `ParamDef::follows`
            "follows",
            "mergeGroup",
            "pinned",
            // 「该机型没写值就整行不落盘」（2026-11-10）；读进 `ParamDef::omit_when_unset`
            "omitWhenUnset",
            "serialization",
            // 刻意不读的两个：实测 74/74 全 true，零区分度
            "mergeable",
            "selectable",
        ]
        .into_iter()
        .collect();

        let arr = doc
            .get("params")
            .and_then(|i| i.as_array_of_tables())
            .expect("[[params]] 是表数组");
        let mut unknown: BTreeSet<String> = BTreeSet::new();
        for t in arr.iter() {
            for (k, _) in t.iter() {
                if !known.contains(k) {
                    unknown.insert(k.to_owned());
                }
            }
        }
        assert!(
            unknown.is_empty(),
            "真数据里有我没读的字段：{unknown:?} —— 要么读进来，要么在这条清单里写明为什么不读"
        );
    }

    /* ---------- 写回（b04 Task 10） ---------- */

    /// **双语注册表的覆盖闸**（2026-10-26）。
    ///
    /// 参数名/说明/选项/页签/分组现在各带一份英文（`labelEn` / `descEn`）。界面在英文
    /// 下拿不到英文会**回落中文** —— 那是安全网，不是可以长期缺着的理由：漏一条，英文
    /// 界面上就冒一个中文。所以这里钉住：真数据里一条都不许空。
    ///
    /// 新增参数时这条会红 —— 补上英文再提交（翻译就写在 `[[params]]` 里 `label` 下面）。
    #[test]
    fn every_entry_has_english() {
        let Some(root) = crate::presetdata::repo_presets_root() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let r = ParamRegistry::load_from(&root).expect("真注册表读得通");

        let mut missing: Vec<String> = Vec::new();
        for p in &r.params {
            if p.label_en.trim().is_empty() {
                missing.push(format!("{}.labelEn", p.key));
            }
            if p.desc_en.trim().is_empty() {
                missing.push(format!("{}.descEn", p.key));
            }
            for c in &p.choices {
                if c.label_en.trim().is_empty() {
                    missing.push(format!("{}.choices[{}].labelEn", p.key, c.label.trim()));
                }
            }
        }
        for t in r.tabs() {
            if t.label_en.trim().is_empty() {
                missing.push(format!("tab {}.labelEn", t.id));
            }
            for s in &t.sections {
                if s.label_en.trim().is_empty() {
                    missing.push(format!("section {}.labelEn", s.id));
                }
            }
        }
        assert!(
            missing.is_empty(),
            "这些条目还没有英文（英文界面会冒出中文）：{missing:#?}"
        );
    }

    /// **四册的覆盖闸**（2026-10-31）。
    ///
    /// 作者口径：「需要单独翻译」—— MKP 自己这套涂胶/擦料塔参数 BBS 没有，日/繁中是我们
    /// 自己译的。界面在 ja / zh-TW 下拿不到那一册会**回落中文**（安全网，不是可以长期缺着
    /// 的理由）。新增参数时这条会红：补上 `labelJa` / `labelTw` 再提交。
    ///
    /// `desc` 只在原文有说明时才要求译文（有几条本来就没有说明）。
    #[test]
    fn every_entry_has_japanese_and_traditional() {
        let Some(root) = crate::presetdata::repo_presets_root() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let r = ParamRegistry::load_from(&root).expect("真注册表读得通");

        let mut missing: Vec<String> = Vec::new();
        for p in &r.params {
            if p.label_ja.trim().is_empty() {
                missing.push(format!("{}.labelJa", p.key));
            }
            if p.label_tw.trim().is_empty() {
                missing.push(format!("{}.labelTw", p.key));
            }
            if !p.desc.is_empty() && p.desc_ja.trim().is_empty() {
                missing.push(format!("{}.descJa", p.key));
            }
            if !p.desc.is_empty() && p.desc_tw.trim().is_empty() {
                missing.push(format!("{}.descTw", p.key));
            }
            for c in &p.choices {
                if c.label_ja.trim().is_empty() {
                    missing.push(format!("{}.choices[{}].labelJa", p.key, c.label.trim()));
                }
                if c.label_tw.trim().is_empty() {
                    missing.push(format!("{}.choices[{}].labelTw", p.key, c.label.trim()));
                }
            }
        }
        for t in r.tabs() {
            if t.label_ja.trim().is_empty() {
                missing.push(format!("tab {}.labelJa", t.id));
            }
            if t.label_tw.trim().is_empty() {
                missing.push(format!("tab {}.labelTw", t.id));
            }
            for s in &t.sections {
                if s.label_ja.trim().is_empty() {
                    missing.push(format!("section {}.labelJa", s.id));
                }
                if s.label_tw.trim().is_empty() {
                    missing.push(format!("section {}.labelTw", s.id));
                }
            }
        }
        assert!(
            missing.is_empty(),
            "这些条目还没有日/繁中（那两个界面会冒出中文）：{missing:#?}"
        );
    }

    /// 把两个文件复制到临时目录，**在副本上改**，不碰真数据
    fn copy_of_real() -> Option<(tempfile::TempDir, ParamRegistry)> {
        let root = crate::presetdata::repo_presets_root()?;
        let tmp = tempfile::tempdir().expect("临时目录");
        for rel in ["registry/param_registry.toml", "layout_schema.toml"] {
            let dst = tmp.path().join(rel);
            std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
            std::fs::copy(root.join(rel), &dst).unwrap();
        }
        let r = ParamRegistry::load_from(tmp.path()).expect("副本也该读得通");
        Some((tmp, r))
    }

    /// 改一个值：**只有那一处变**，而且重读盘之后值真的在。
    ///
    /// 这是「改参数值」这条写路径独有的风险 —— 它落在一个 56 KB、74 条字段的文件里，
    /// 排版被搅乱的话 diff 是一片红，真正改了什么反而看不出来
    #[test]
    fn writing_one_variant_touches_nothing_else() {
        let Some((tmp, mut r)) = copy_of_real() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let before = r.to_toml();

        r.set_variant(
            "toolhead.MKP_retract",
            "A1:STANDARD",
            &serde_json::json!(-1.5),
        )
        .expect("写得动");
        r.write_back().expect("落盘");

        let again = ParamRegistry::load_from(tmp.path()).expect("改完还得读得通");
        assert_eq!(
            again
                .param("toolhead.MKP_retract")
                .unwrap()
                .machine_variants["A1:STANDARD"],
            serde_json::json!(-1.5),
            "重读盘之后值不在 —— 内存里成了盘上没成"
        );

        let (removed, inserted) = one_edit_only(&before, &again.to_toml());
        assert!(inserted.contains("-1.5"), "新值不在插入段：{inserted:?}");
        // **别的字段一个都不许进这次改动的范围**
        assert!(
            !removed.contains("[[params]]"),
            "波及了别的字段块：{removed:?}"
        );
        assert!(
            !inserted.contains("[[params]]"),
            "插入段跨到了别的字段：{inserted:?}"
        );
    }

    /// 「清空」是**删键**：那一行整行消失，而不是写成 `0`。
    ///
    /// `0` 是一个值，意思是"我在这一层特意把它定成 0"；没有这一行才是"跟着上一层走"。
    /// 两件事在界面上必须分得开 —— 这一条与元字段那边同一个道理
    #[test]
    fn clearing_a_variant_removes_the_key_instead_of_writing_a_zero() {
        let Some((tmp, mut r)) = copy_of_real() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        // 从真数据里找一个有 `机型:版本` 键的字段来清
        let (key, owner) = r
            .params()
            .iter()
            .find_map(|p| {
                p.machine_variants
                    .keys()
                    .find(|k| k.contains(':'))
                    .map(|k| (p.key.clone(), k.clone()))
            })
            .expect("真数据里该有 `机型:版本` 形状的键 —— 没有的话这条判据失去对象");
        let others: Vec<String> = r
            .param(&key)
            .unwrap()
            .machine_variants
            .keys()
            .filter(|k| **k != owner)
            .cloned()
            .collect();

        r.clear_variant(&key, &owner).expect("清得动");
        r.write_back().expect("落盘");

        let again = ParamRegistry::load_from(tmp.path()).expect("清完还得读得通");
        let table = &again.param(&key).unwrap().machine_variants;
        assert!(!table.contains_key(&owner), "键还在：{table:?}");
        // 同一张表里别的键一个都不许少
        for k in &others {
            assert!(table.contains_key(k), "把别的键 {k} 也删了");
        }
        // 文件里不许出现"写成 0"的痕迹
        let line = format!("'{owner}' = 0");
        assert!(!again.to_toml().contains(&line), "清空被写成了 0");
    }

    /// 多行 G-code 写回来**一字不差**。
    ///
    /// 它是字符串写入最硬的一块：TOML 的字面量（单引号）不支持转义，
    /// 所以这种值只能退回双引号 + `\n`。写错的后果是产物里的 G-code 少一行，
    /// 而那要到机器上才看得出来
    #[test]
    fn a_multiline_gcode_variant_round_trips_verbatim() {
        let Some((tmp, mut r)) = copy_of_real() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let key = r
            .params()
            .iter()
            .find(|p| !p.machine_variants.is_empty() && p.toml_key.contains("gcode"))
            .map(|p| p.key.clone())
            .expect("装载/卸载 G-code 那两条必须在");
        let gcode = "G92 E0\nG1 E-2 F1800\n; 我写的一行\tTAB";

        r.set_variant(&key, "A1", &serde_json::json!(gcode))
            .expect("写得动");
        r.write_back().expect("落盘");

        let again = ParamRegistry::load_from(tmp.path()).expect("改完还得读得通");
        assert_eq!(
            again.param(&key).unwrap().machine_variants["A1"],
            serde_json::json!(gcode),
            "多行 G-code 读回来变了"
        );
    }

    /// `int` 字段写整数、`float` 字段写浮点 —— **表示不许漂**。
    ///
    /// 真数据里 `valueType = 'float'` 的默认值写的是 `0.0`；写回时印成 `0`
    /// 就等于一点点把这个文件洗成另一种风格，而且 `fingerprint` 会跟着变
    #[test]
    fn an_int_stays_an_int_and_a_float_keeps_its_point_zero() {
        let Some((tmp, mut r)) = copy_of_real() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let float_key = r
            .params()
            .iter()
            .find(|p| p.value_type == ValueType::Float)
            .map(|p| p.key.clone())
            .expect("总有一条 float");
        let int_key = r
            .params()
            .iter()
            .find(|p| p.value_type == ValueType::Int)
            .map(|p| p.key.clone())
            .expect("实测有 4 条 int");

        r.set_variant(&float_key, "A1", &serde_json::json!(3))
            .unwrap();
        r.set_variant(&int_key, "A1", &serde_json::json!(3))
            .unwrap();
        r.write_back().unwrap();

        let text =
            std::fs::read_to_string(tmp.path().join("registry/param_registry.toml")).unwrap();
        // 这条测试钉的是**值的表示**：int 留 `3`、float 留 `3.0` —— 那是数据。
        //
        // 引号不在这条的射程内（2026-11-03 作者：「没必要特殊，都用引号也可以」）。
        // 这里走的是 `write_back` 的就地写回（`toml_edit` 保留原引号风格），
        // 与 `presets normalize` 的整体重排是**两条路**：前者只改被编辑那一格，
        // 后者才重排全文。所以这里用"键 + 值"匹配，不管有没有引号。
        let has_float = text.contains("A1\" = 3.0") || text.contains("A1 = 3.0");
        let has_int = text.contains("A1\" = 3\n") || text.contains("A1 = 3\n");
        assert!(has_float, "float 字段没写成 3.0：\n{text}");
        assert!(has_int, "int 字段被写成了浮点：\n{text}");

        // 读回来两个都是 3（数值相等），但表示不同 —— 这正是这条判据要的
        let again = ParamRegistry::load_from(tmp.path()).unwrap();
        for k in [&float_key, &int_key] {
            assert_eq!(
                again.param(k).unwrap().machine_variants["A1"].as_f64(),
                Some(3.0)
            );
        }
    }

    /// **被拒之后文件一字未动。** 三条拒绝路径各有自己的话
    #[test]
    fn a_refused_write_leaves_the_file_untouched() {
        let Some((tmp, mut r)) = copy_of_real() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let path = tmp.path().join("registry/param_registry.toml");
        let before = std::fs::read_to_string(&path).unwrap();

        let no_such = r
            .set_variant("没有这个字段", "A1", &serde_json::json!(1))
            .expect_err("字段不存在");
        let bad_owner = r
            .set_variant("toolhead.MKP_retract", " ", &serde_json::json!(1))
            .expect_err("owner 空白");
        let bad_type = r
            .set_variant("toolhead.MKP_retract", "A1", &serde_json::json!([1, 2]))
            .expect_err("数组写不进去");

        let msgs = [no_such, bad_owner, bad_type].map(|e| e.message);
        for (i, a) in msgs.iter().enumerate() {
            assert!(!a.is_empty());
            for b in &msgs[i + 1..] {
                assert_ne!(a, b, "两种拒绝说了同一句话");
            }
        }
        // 三次都被拒，盘上与内存都不该有痕迹
        assert_eq!(
            before,
            std::fs::read_to_string(&path).unwrap(),
            "文件被改了"
        );
        assert_eq!(before, r.to_toml(), "内存里的文档被改了");
    }

    /// 跨文件那道门禁：**值不许写在不存在的机型/版本上**。
    ///
    /// 写进去的后果不是"多一条垃圾"，而是下一次加载被跨文件断言判成 `Corrupted` ——
    /// 整个工作台起不来。所以这一条拦的是"把数据写成自己都读不回来的状态"
    #[test]
    fn writing_a_value_onto_a_machine_that_does_not_exist_is_refused() {
        let Some(root) = crate::presetdata::repo_presets_root() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        // 整份 presets 复制一遍（机型文件也要，这一条查的就是跨文件）
        let tmp = tempfile::tempdir().unwrap();
        copy_tree(&root, tmp.path());
        let mut p = crate::presetdata::Presets::load_from(tmp.path()).expect("副本读得通");
        let before = std::fs::read_to_string(p.registry.file()).unwrap();

        let e = p
            .set_variant(
                "toolhead.MKP_retract",
                "NO_SUCH_MACHINE",
                &serde_json::json!(1),
            )
            .expect_err("机型不存在");
        assert_eq!(e.code, crate::error::ErrorCode::NotFound);
        let e2 = p
            .set_variant(
                "toolhead.MKP_retract",
                "A1:NO_SUCH_VERSION",
                &serde_json::json!(1),
            )
            .expect_err("版本不存在");
        assert_eq!(e2.code, crate::error::ErrorCode::NotFound);
        assert_ne!(e.message, e2.message, "两种不存在说了同一句话");

        assert_eq!(
            before,
            std::fs::read_to_string(p.registry.file()).unwrap(),
            "被拒之后文件被改了"
        );
        // 对照组：真机型真版本写得进去，而且改完整份数据还读得通（跨文件断言会跑）
        p.set_variant(
            "toolhead.MKP_retract",
            "A1:STANDARD",
            &serde_json::json!(-2.5),
        )
        .expect("真版本该写得进去");
        let again = crate::presetdata::Presets::load_from(tmp.path()).expect("改完整份还读得通");
        assert_eq!(
            again
                .registry
                .param("toolhead.MKP_retract")
                .unwrap()
                .machine_variants["A1:STANDARD"],
            serde_json::json!(-2.5)
        );
    }

    /// 递归复制一棵目录（夹具用）
    fn copy_tree(from: &Path, to: &Path) {
        for e in std::fs::read_dir(from).unwrap().flatten() {
            let src = e.path();
            let dst = to.join(src.file_name().unwrap());
            if src.is_dir() {
                std::fs::create_dir_all(&dst).unwrap();
                copy_tree(&src, &dst);
            } else {
                std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
                std::fs::copy(&src, &dst).unwrap();
            }
        }
    }

    /// **一批值一次落盘**（b04 Task 12.1 的前置）。
    ///
    /// 一次「保存」常常是一片改动。逐条写的代价不只是把 56 KB 写 N 遍 ——
    /// 中途失败会留下「改了前三条、没改后两条」的文件，而那种状态
    /// 没有任何判据能描述它。所以批量入口要么全成，要么一个字节都不写。
    #[test]
    fn a_batch_of_values_lands_in_one_write() {
        let Some(root) = crate::presetdata::repo_presets_root() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let tmp = tempfile::tempdir().unwrap();
        copy_tree(&root, tmp.path());
        let mut p = crate::presetdata::Presets::load_from(tmp.path()).unwrap();

        // 一批：两个机型基底 + 一个版本覆盖 + 一次清空
        p.apply_values(&[
            (
                "toolhead.MKP_retract".to_owned(),
                "A1".to_owned(),
                Some(serde_json::json!(-3.5)),
            ),
            (
                "toolhead.MKP_retract".to_owned(),
                "P1S".to_owned(),
                Some(serde_json::json!(-4.0)),
            ),
            (
                "toolhead.MKP_retract".to_owned(),
                "A1:FAST".to_owned(),
                Some(serde_json::json!(-5.0)),
            ),
        ])
        .expect("一批写得进去");

        let again = crate::presetdata::Presets::load_from(tmp.path()).expect("改完读得通");
        let t = &again
            .registry
            .param("toolhead.MKP_retract")
            .unwrap()
            .machine_variants;
        assert_eq!(t["A1"], serde_json::json!(-3.5));
        assert_eq!(t["P1S"], serde_json::json!(-4.0));
        assert_eq!(t["A1:FAST"], serde_json::json!(-5.0));

        // 清空也走同一条路
        p.apply_values(&[(
            "toolhead.MKP_retract".to_owned(),
            "A1:FAST".to_owned(),
            None,
        )])
        .expect("清得掉");
        let again = crate::presetdata::Presets::load_from(tmp.path()).unwrap();
        let t = &again
            .registry
            .param("toolhead.MKP_retract")
            .unwrap()
            .machine_variants;
        assert!(!t.contains_key("A1:FAST"), "清空没生效");
        assert_eq!(t["A1"], serde_json::json!(-3.5), "顺带把别的键改了");
    }

    /// **值与「跟不跟基底」同一次落盘**（2026-11-03 的验收点）。
    ///
    /// 分成两次写的话，中途失败会留下"值改了、状态还是跟随"——
    /// 下一次改基底就把它覆盖回去，而界面从头到尾没报过一句错。
    ///
    /// 这里盯三件事：
    /// 1. 写值 ⇒ 同一笔里 `[params.follows]` 写上 `= false`（写过就是自己的值）；
    /// 2. `follows: Some(true)` 的连带写**不会**把跟随项翻成独立值；
    /// 3. 改回跟随 = 表里删掉那个键（文件回到默认形状，不留 `= true`）。
    #[test]
    fn value_and_follow_state_go_out_in_the_same_write() {
        let Some(root) = crate::presetdata::repo_presets_root() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let tmp = tempfile::tempdir().unwrap();
        copy_tree(&root, tmp.path());
        let mut p = crate::presetdata::Presets::load_from(tmp.path()).unwrap();

        /* ① 写值：一笔里同时带 `不跟随` */
        p.apply_values(&[crate::presetdata::ValueEdit::write(
            "toolhead.MKP_retract",
            "A1:FAST",
            serde_json::json!(-5.0),
        )])
        .unwrap();
        let again = crate::presetdata::Presets::load_from(tmp.path()).unwrap();
        let q = again.registry.param("toolhead.MKP_retract").unwrap();
        assert_eq!(q.machine_variants["A1:FAST"], serde_json::json!(-5.0));
        assert!(
            !crate::presetdata::follows::is_following(q, "A1:FAST"),
            "写值与状态必须同一笔落地"
        );

        /* ② 连带写（改基底时跟着变的那些）：显式 `跟随` ⇒ 还是跟随 */
        p.apply_values(&[crate::presetdata::ValueEdit::write(
            "toolhead.MKP_retract",
            "A1:FAST",
            serde_json::json!(-6.0),
        )
        .with_follows(true)])
            .unwrap();
        let again = crate::presetdata::Presets::load_from(tmp.path()).unwrap();
        let q = again.registry.param("toolhead.MKP_retract").unwrap();
        assert_eq!(q.machine_variants["A1:FAST"], serde_json::json!(-6.0));
        assert!(
            crate::presetdata::follows::is_following(q, "A1:FAST"),
            "连带写不许把跟随项翻成独立值"
        );

        /* ③ 改回跟随 = 删键（不留 `= true` 这种同义写法） */
        p.apply_values(&[crate::presetdata::ValueEdit::keep(
            "toolhead.MKP_retract",
            "A1:FAST",
            true,
        )])
        .unwrap();
        let again = crate::presetdata::Presets::load_from(tmp.path()).unwrap();
        let q = again.registry.param("toolhead.MKP_retract").unwrap();
        assert!(
            q.follows.is_empty(),
            "跟随 = 表里没有这个键，实际：{:?}",
            q.follows
        );
        assert!(
            crate::presetdata::follows::is_following(q, "A1:FAST"),
            "跟随"
        );
        assert_eq!(
            q.machine_variants["A1:FAST"],
            serde_json::json!(-6.0),
            "只改状态不许碰值"
        );
    }

    /// **一条不合法 → 整批不写。**
    ///
    /// 这一条盯的是"部分成功"这种状态。它比失败更糟：失败能重试，
    /// 而"前三条成了、后两条没成"之后，用户看到的是一份他没打算要的数据，
    /// 而且没有任何提示说哪几条生效了
    #[test]
    fn one_bad_edit_in_a_batch_writes_nothing() {
        let Some(root) = crate::presetdata::repo_presets_root() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let tmp = tempfile::tempdir().unwrap();
        copy_tree(&root, tmp.path());
        let mut p = crate::presetdata::Presets::load_from(tmp.path()).unwrap();
        let before = std::fs::read_to_string(p.registry.file()).unwrap();

        // 第一条合法、第二条机型不存在、第三条字段不存在
        let e = p
            .apply_values(&[
                (
                    "toolhead.MKP_retract".to_owned(),
                    "A1".to_owned(),
                    Some(serde_json::json!(-9.0)),
                ),
                (
                    "toolhead.MKP_retract".to_owned(),
                    "NOPE".to_owned(),
                    Some(serde_json::json!(1)),
                ),
                (
                    "没有这个字段".to_owned(),
                    "A1".to_owned(),
                    Some(serde_json::json!(1)),
                ),
            ])
            .expect_err("整批该被拒");
        assert_eq!(
            e.code,
            crate::error::ErrorCode::NotFound,
            "先报的该是机型不存在"
        );

        assert_eq!(
            before,
            std::fs::read_to_string(p.registry.file()).unwrap(),
            "整批被拒之后盘上文件被改了"
        );
        assert_eq!(before, p.registry.to_toml(), "内存里的文档也被改了");
    }

    /// 空批次是合法的空操作 —— **不写盘**。
    ///
    /// 没有这一条的话，「保存」在没有任何值改动时也会重写一遍 56 KB，
    /// 于是文件 mtime 变了、git 里多一条噪音 diff
    #[test]
    fn an_empty_batch_does_not_touch_the_file() {
        let Some(root) = crate::presetdata::repo_presets_root() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let tmp = tempfile::tempdir().unwrap();
        copy_tree(&root, tmp.path());
        let mut p = crate::presetdata::Presets::load_from(tmp.path()).unwrap();
        let path = p.registry.file().to_path_buf();
        let before = std::fs::metadata(&path).unwrap().modified().unwrap();

        p.apply_values::<(String, String, Option<serde_json::Value>)>(&[])
            .expect("空批次是合法的");

        assert_eq!(
            before,
            std::fs::metadata(&path).unwrap().modified().unwrap(),
            "空批次动了文件的 mtime"
        );
    }

    /* ---------- 定义编辑（`wb_set_param_meta` 的数据层，2026-10-03） ---------- */

    /// 一份「改得动」的小注册表：一条 float 父项、一条挂在它下面的子项、
    /// 一条带选项的枚举 —— 定义编辑的判据在这三条上都够使了
    fn editable() -> (Value, Value) {
        let mut params = serde_json::json!({
            "params": [
                param("a.mode", "s1", 1.0),
                param("a.x", "s1", 2.0),
                param("a.y", "s1", 3.0),
            ],
            "tabs": [{
                "id": "t1", "label": "页签一", "order": 10,
                "sections": [{ "id": "s1", "label": "分组一", "order": 0 }]
            }],
            "updated": "2026-01-01 00:00:00",
        });
        // a.x 挂在 a.mode 下、被它的一档开着；a.y 是枚举
        params["params"][1]["parentKey"] = serde_json::json!("a.mode");
        params["params"][1]["showWhen"] =
            serde_json::json!({ "key": "a.mode", "op": "eq", "value": 1 });
        params["params"][2]["valueType"] = serde_json::json!("string");
        params["params"][2]["uiComponent"] = serde_json::json!("select");
        params["params"][2]["defaultValue"] = serde_json::json!("on");
        params["params"][2]["choices"] = serde_json::json!([
            { "label": "开", "value": "on" },
            { "label": "关", "value": "off" },
        ]);
        let layout = serde_json::json!({
            "tabs": [{ "id": "t1", "sections": [{
                "id": "s1",
                "items": [
                    { "id": "i0", "paramKey": "a.mode" },
                    { "id": "i1", "paramKey": "a.x" },
                    { "id": "i2", "paramKey": "a.y" },
                ],
            }] }],
        });
        (params, layout)
    }

    /// 一份「全空」的编辑载荷：只给名字，其余都是要清掉的。
    /// `choices` / `machine_filter` 同一口径 —— 空表就是清空，模态框那边
    /// 永远整表带回
    fn meta_edit(label: &str) -> ParamMetaEdit {
        ParamMetaEdit {
            label: label.to_owned(),
            label_en: None,
            label_ja: None,
            label_tw: None,
            desc: String::new(),
            desc_en: None,
            desc_ja: None,
            desc_tw: None,
            unit: None,
            value_type: ValueType::Float,
            ui_component: UiComponent::Number,
            default_value: serde_json::json!(0),
            min: None,
            max: None,
            step: None,
            parent_key: None,
            show_when: None,
            deprecated: false,
            deprecated_machines: Vec::new(),
            choices: Vec::new(),
            machine_filter: Vec::new(),
            section_id: None,
            new_section: None,
            order: None,
        }
    }

    /// 一条可选项（测试里手写三件套太啰嗦）
    fn choice(label: &str, value: Value, deprecated: bool) -> Choice {
        Choice {
            label: label.to_owned(),
            label_en: String::new(),
            label_ja: String::new(),
            label_tw: String::new(),
            value,
            deprecated,
            deprecated_machines: Vec::new(),
        }
    }

    /// 夹具临时目录里那份注册表文件
    fn to_file(root: &std::path::Path) -> std::path::PathBuf {
        root.join("registry").join("param_registry.toml")
    }

    /// 一次定义编辑：改的每一格**重读盘之后都在**，diff 只落在一个块里
    #[test]
    fn a_meta_edit_round_trips_to_disk() {
        let (p, l) = editable();
        let (d, r) = load(p, l);
        let mut r = r.unwrap();
        let before = r.to_toml();

        r.set_param_meta(
            "a.x",
            ParamMetaEdit {
                label: "横移".into(),
                desc: "改过的说明".into(),
                unit: Some("mm".into()),
                default_value: serde_json::json!(4),
                min: Some(0.0),
                max: Some(10.0),
                step: Some(0.5),
                parent_key: Some("a.mode".into()),
                show_when: Some(ShowWhen {
                    key: "a.mode".into(),
                    op: ShowOp::Neq,
                    value: serde_json::json!("off"),
                }),
                deprecated: true,
                ..meta_edit("横移")
            },
        )
        .expect("定义改得动");
        r.write_back().expect("落盘");

        let again = ParamRegistry::load_from(d.path()).expect("改完还得读得通");
        let p = again.param("a.x").unwrap();
        assert_eq!(p.label, "横移");
        assert_eq!(p.desc, "改过的说明");
        assert_eq!(p.unit.as_deref(), Some("mm"));
        assert_eq!(
            p.default_value,
            serde_json::json!(4.0),
            "float 的默认值得按浮点读回"
        );
        assert_eq!(p.min, Some(0.0));
        assert_eq!(p.max, Some(10.0));
        assert_eq!(p.step, Some(0.5));
        assert!(p.deprecated);
        assert_eq!(p.parent_key.as_deref(), Some("a.mode"));
        let sw = p.show_when.as_ref().unwrap();
        assert_eq!(sw.key, "a.mode");
        assert_eq!(sw.op, ShowOp::Neq);
        assert_eq!(sw.value, serde_json::json!("off"));

        // 邻居一格没动
        assert_eq!(again.param("a.y").unwrap().label, "a.y");

        let (removed, inserted) = one_edit_only(&before, &again.to_toml());
        assert!(
            !removed.contains("[[params]]"),
            "波及了别的字段块：{removed:?}"
        );
        assert!(
            !inserted.contains("[[params]]"),
            "插入段跨到了别的字段：{inserted:?}"
        );
    }

    /// **译文台账**（2026-11-02，作者：「我改了中文，但是其他没有改，这种能不能记录下来」）。
    ///
    /// 改中文、译文没跟着重写 ⇒ 登记到 `staleTranslations`（三册各一行、名称与说明分开）；
    /// 同一笔里把某一册重写了 ⇒ 那一册不欠账；重写的第二次保存 ⇒ 摘掉那一笔。
    /// 只改单位 / 分组（中文没动）时台账一个字节都不变。
    #[test]
    fn changing_chinese_registers_stale_translations() {
        let (mut p, l) = editable();
        p["params"][1]["labelEn"] = serde_json::json!("X offset");
        p["params"][1]["labelJa"] = serde_json::json!("X オフセット");
        p["params"][1]["descEn"] = serde_json::json!("offset");
        let (d, r) = load(p, l);
        let mut r = r.unwrap();

        // 一改：只改中文说明 —— 只有 desc 那三册欠账，label 那三册不动
        r.set_param_meta(
            "a.x",
            ParamMetaEdit {
                label: "a.x".into(),
                desc: "新的说明".into(),
                ..meta_edit("a.x")
            },
        )
        .expect("改得动");
        assert_eq!(
            r.param("a.x").unwrap().stale_translations,
            vec!["descEn".to_owned()],
            "没写译文的两册（Ja/Tw 说明）没有「过期」可言 —— 它们只是还没译"
        );
        assert!(
            r.to_toml().contains("staleTranslations"),
            "台账得落进文件，不然下次打开就丢了"
        );

        // 二改：中文名也改 —— label 那两册有译文的跟着登记
        r.set_param_meta(
            "a.x",
            ParamMetaEdit {
                label: "横移".into(),
                desc: "新的说明".into(),
                ..meta_edit("横移")
            },
        )
        .expect("改得动");
        let mut got = r.param("a.x").unwrap().stale_translations.clone();
        got.sort();
        assert_eq!(got, vec!["descEn", "labelEn", "labelJa"]);

        // 三改：同一笔里把英文说明重写了 ⇒ 那一笔摘掉（作者：「我改了所有的都可以用公共的方法」）
        r.set_param_meta(
            "a.x",
            ParamMetaEdit {
                label: "横移".into(),
                desc: "新的说明".into(),
                desc_en: Some("a brand new description".into()),
                ..meta_edit("横移")
            },
        )
        .expect("改得动");
        let mut got = r.param("a.x").unwrap().stale_translations.clone();
        got.sort();
        assert_eq!(
            got,
            vec!["labelEn", "labelJa"],
            "重写过的 descEn 不该还挂着"
        );

        // 四改：重放同一笔（撤销/重做）—— 幂等，台账不增不减
        r.set_param_meta(
            "a.x",
            ParamMetaEdit {
                label: "横移".into(),
                desc: "新的说明".into(),
                desc_en: Some("a brand new description".into()),
                ..meta_edit("横移")
            },
        )
        .expect("改得动");
        let mut got = r.param("a.x").unwrap().stale_translations.clone();
        got.sort();
        assert_eq!(got, vec!["labelEn", "labelJa"]);

        // 五改：只改单位 —— 中文没动，台账原样
        r.set_param_meta(
            "a.x",
            ParamMetaEdit {
                label: "横移".into(),
                desc: "新的说明".into(),
                unit: Some("mm".into()),
                ..meta_edit("横移")
            },
        )
        .expect("改得动");
        assert_eq!(r.param("a.x").unwrap().stale_translations.len(), 2);

        // 落盘再读回：台账是数据，不是内存里的临时标记
        r.write_back().expect("落盘");
        let again = ParamRegistry::load_from(d.path()).expect("读得通");
        let mut got = again.param("a.x").unwrap().stale_translations.clone();
        got.sort();
        assert_eq!(got, vec!["labelEn", "labelJa"]);
        assert!(
            r.to_toml().contains("staleTranslations"),
            "台账在写回之后也得留着"
        );
    }

    /// **译文台账的逃生通道**（2026-11-02）：点名清，不看你改了没有。
    ///
    /// 这条路是给"译文由别处写好"用的（直接改文件、或按待办清单整批改）——
    /// 那种改不经过 `set_param_meta`，老判据（值变了才摘）永远不认。
    /// 同时钉住两件事：**名字写错整笔拒收**（不做"能清几个算几个"）、
    /// **`set_param_meta` 那条老规矩没被放宽**（原值重存仍是幂等的）。
    #[test]
    fn stale_translations_can_be_cleared_explicitly() {
        let (mut p, l) = editable();
        p["params"][1]["labelEn"] = serde_json::json!("X offset");
        p["params"][1]["descEn"] = serde_json::json!("offset");
        p["params"][1]["staleTranslations"] = serde_json::json!(["labelEn", "descEn", "descJa"]);
        let (d, r) = load(p, l);
        let mut r = r.unwrap();

        // 名字写错 ⇒ 整笔拒收，一格都不许清
        let e = r
            .clear_stale_translations("a.x", &["labelEn".into(), "labelEN".into()])
            .unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::InvalidArgument);
        assert!(
            e.message.contains("labelEN"),
            "拒收要说清是哪一格写错了：{}",
            e.message
        );
        assert_eq!(
            r.param("a.x").unwrap().stale_translations,
            vec![
                "labelEn".to_owned(),
                "descEn".to_owned(),
                "descJa".to_owned()
            ],
            "被拒的那一笔动到了台账"
        );

        // 点名清两格 ⇒ 只剩没点的那一格；文件中同步少两个
        let left = r
            .clear_stale_translations("a.x", &["labelEn".into(), "descEn".into()])
            .expect("清得动");
        assert_eq!(left, vec!["descJa".to_owned()]);

        // 清空 ⇒ 键整个从文件里消失（与 `skip_serializing_if` 同一口径）
        r.clear_stale_translations("a.x", &["descJa".into()])
            .expect("清得动");
        r.write_back().expect("落盘");
        let again = ParamRegistry::load_from(d.path()).expect("读得通");
        assert!(again.param("a.x").unwrap().stale_translations.is_empty());
        assert!(
            !again.to_toml().contains("staleTranslations"),
            "台账清空了还留着键"
        );

        // 老判据没被放宽：原样重存（值没变）不许凭空清掉台账
        let (mut p2, l2) = editable();
        p2["params"][1]["descEn"] = serde_json::json!("offset");
        p2["params"][1]["staleTranslations"] = serde_json::json!(["descEn"]);
        let (_d2, r2) = load(p2, l2);
        let mut r2 = r2.unwrap();
        r2.set_param_meta(
            "a.x",
            ParamMetaEdit {
                desc_en: Some("offset".into()),
                ..meta_edit("a.x")
            },
        )
        .expect("改得动");
        assert_eq!(
            r2.param("a.x").unwrap().stale_translations,
            vec!["descEn".to_owned()],
            "值没变的保存不许清账 —— 撤销/重做要靠这条才幂等"
        );
    }

    /// 清空是**删键**：`None` 的格子整行消失；弃用解除不写 `false` 占位 ——
    /// 与真数据「缺一行才是 false」的口径一致
    #[test]
    fn clearing_meta_fields_removes_the_keys() {
        let (p, l) = editable();
        let (d, r) = load(p, l);
        let mut r = r.unwrap();
        // 先给 a.x 添满（parentKey / showWhen 是夹具里就有的）
        r.set_param_meta(
            "a.x",
            ParamMetaEdit {
                unit: Some("mm".into()),
                min: Some(0.0),
                max: Some(10.0),
                step: Some(0.5),
                parent_key: Some("a.mode".into()),
                show_when: Some(ShowWhen {
                    key: "a.mode".into(),
                    op: ShowOp::Eq,
                    value: serde_json::json!(1),
                }),
                deprecated: true,
                ..meta_edit("a.x")
            },
        )
        .unwrap();
        r.write_back().unwrap();

        // 再一把全清掉
        r.set_param_meta("a.x", meta_edit("a.x")).unwrap();
        r.write_back().unwrap();

        let again = ParamRegistry::load_from(d.path()).expect("清完还得读得通");
        let p = again.param("a.x").unwrap();
        assert_eq!(p.unit, None);
        assert_eq!(p.min, None);
        assert_eq!(p.max, None);
        assert_eq!(p.step, None);
        assert_eq!(p.parent_key, None);
        assert!(p.show_when.is_none());
        assert!(!p.deprecated);
        assert!(
            !again.to_toml().contains("deprecated = true"),
            "false 被写成了占位行"
        );
    }

    /// 可选项与适用机型整表换：改了要落盘（含逐条弃用）、机型清单 trim + 去重、
    /// 数字预设档的表示不漂、清空 = 删键
    #[test]
    fn choices_and_machine_filter_round_trip() {
        let (p, l) = editable();
        let (d, r) = load(p, l);
        let mut r = r.unwrap();

        // 一改：a.y 弃用「关」、加一条「半开」；机型清单带空格与重复 —— 归一
        r.set_param_meta(
            "a.y",
            ParamMetaEdit {
                value_type: ValueType::Text,
                ui_component: UiComponent::Select,
                default_value: serde_json::json!("on"),
                choices: vec![
                    choice("开", serde_json::json!("on"), false),
                    choice("关", serde_json::json!("off"), true),
                    choice("半开", serde_json::json!("half"), false),
                ],
                machine_filter: vec!["A1".into(), " P1S ".into(), "A1".into()],
                ..meta_edit("a.y")
            },
        )
        .expect("改得动");
        r.write_back().expect("落盘");

        let again = ParamRegistry::load_from(d.path()).expect("改完还得读得通");
        let py = again.param("a.y").unwrap();
        assert_eq!(py.choices.len(), 3, "加的那条没落盘");
        assert_eq!(py.choices[1].label, "关");
        assert!(py.choices[1].deprecated, "逐条弃用没落盘");
        assert!(!py.choices[0].deprecated, "没弃用的不许带 deprecated 键");
        assert_eq!(py.machine_filter, ["A1", "P1S"], "trim 与去重都没做");

        // 二改：a.x 挂两条数字预设档 —— 整数就写整数（0 不许漂成 0.0）
        r.set_param_meta(
            "a.x",
            ParamMetaEdit {
                choices: vec![
                    choice("关", serde_json::json!(0), false),
                    choice("一半", serde_json::json!(50), false),
                ],
                ..meta_edit("a.x")
            },
        )
        .unwrap();
        r.write_back().unwrap();
        let txt = std::fs::read_to_string(to_file(d.path())).unwrap();
        assert!(txt.contains("value = 0\n"), "整数预设档漂成了小数：{txt}");
        assert!(!txt.contains("value = 0.0"));

        // 三改：可选项与机型清单都清空 —— 文件里删键（控件跟着换成文本框才合法）
        r.set_param_meta(
            "a.y",
            ParamMetaEdit {
                value_type: ValueType::Text,
                ui_component: UiComponent::Text,
                default_value: serde_json::json!("自由"),
                choices: Vec::new(),
                machine_filter: Vec::new(),
                ..meta_edit("a.y")
            },
        )
        .unwrap();
        r.write_back().unwrap();

        let again = ParamRegistry::load_from(d.path()).unwrap();
        let py = again.param("a.y").unwrap();
        assert!(py.choices.is_empty(), "清了还在内存里");
        assert!(py.machine_filter.is_empty());
        let txt = again.to_toml();
        assert!(!txt.contains("machineFilter"), "机型清单清了还留着键");
        assert!(!txt.contains("'半开'"), "清掉的可选项还留在文件里");
        // a.x 的预设档不受牵连 —— 整表换只换自己那一条
        assert_eq!(again.param("a.x").unwrap().choices.len(), 2);
    }

    /// 空分组删得掉（两份文件一起删、order 重排）；有参数 / 组件卡 / 条件组拦住
    #[test]
    fn an_empty_section_can_be_removed_but_never_one_in_use() {
        let (p, l) = editable();
        let (d, r) = load(p, l);
        let mut r = r.unwrap();

        /* 先建两个空组（作者 2026-11-01：「不小心建多了也可以删除」） */
        r.ensure_section("t1", "custom_1", "多建的一").unwrap();
        r.ensure_section("t1", "custom_2", "多建的二").unwrap();
        r.write_back().unwrap();

        /* 有参数的那一组删不动 —— 参数会成孤儿 */
        let err = r.set_param_meta("a.y", meta_edit("a.y"));
        assert!(err.is_ok(), "先确认这一笔无关的编辑不受影响");
        let e = r.remove_section("s1").expect_err("s1 还有 3 条参数");
        assert!(
            e.to_string().contains("s1") || e.to_string().contains("参数"),
            "拒绝的理由要说清楚：{e}"
        );
        assert!(r.param("a.y").is_some(), "被拒之后一个字都不许动");

        /* 删空组：两份文件都要少一条，order 重排成 0..n-1 */
        r.remove_section("custom_1").expect("空组删得掉");
        r.write_back().unwrap();

        let again = ParamRegistry::load_from(d.path()).expect("删完还得读得通");
        let ids: Vec<&str> = again
            .tabs
            .iter()
            .flat_map(|t| t.sections.iter())
            .map(|s| s.id.as_str())
            .collect();
        assert!(
            !ids.contains(&"custom_1"),
            "注册表里还留着被删的组：{ids:?}"
        );
        assert!(ids.contains(&"custom_2"), "别的空组被误删了：{ids:?}");
        let layout_ids: Vec<&str> = again
            .layout
            .iter()
            .flat_map(|t| t.sections.iter())
            .map(|s| s.id.as_str())
            .collect();
        assert!(
            !layout_ids.contains(&"custom_1"),
            "布局那份还留着被删的组：{layout_ids:?}"
        );
        let orders: Vec<f64> = again.tabs[0].sections.iter().map(|s| s.order).collect();
        assert_eq!(orders, [0.0, 1.0], "order 没按新次序重排：{orders:?}");
        // 「参数一条都不许少」查的是三条参数与它们的归属，不是组数：
        // fixture 只有 s1 装参数，custom_2 是空组（param_section_ids 只数有 items 的组）。
        for key in ["a.mode", "a.x", "a.y"] {
            let p = again
                .param(key)
                .unwrap_or_else(|| panic!("参数 {key} 被删掉了"));
            assert_eq!(p.layout.section_id, "s1", "{key} 的分组被挪走了");
        }
        let s1_items = again
            .layout
            .iter()
            .flat_map(|t| t.sections.iter())
            .find(|s| s.id == "s1")
            .map(|s| s.items.len());
        assert_eq!(s1_items, Some(3), "s1 的布局条目不许少");
        assert_eq!(
            again.param_section_ids().len(),
            1,
            "空组不许被算成装参数的组"
        );
    }

    /// 弃用可以**只对某几台机型**生效：显式清单落盘、判据按机型分叉、
    /// 全机型并集仍旧认得它（注册表视图划线要用）
    #[test]
    fn deprecated_machines_round_trip_and_are_per_machine() {
        let (p, l) = editable();
        let (d, r) = load(p, l);
        let mut r = r.unwrap();
        r.set_param_meta(
            "a.y",
            ParamMetaEdit {
                value_type: ValueType::Text,
                ui_component: UiComponent::Select,
                default_value: serde_json::json!("on"),
                // 带空格、空段、重复 —— 与 machineFilter 同一套归一
                deprecated_machines: vec![" A1 ".into(), "P1S".into(), String::new(), "A1".into()],
                choices: vec![
                    choice("开", serde_json::json!("on"), false),
                    Choice {
                        deprecated_machines: vec!["A1".into()],
                        ..choice("关", serde_json::json!("off"), false)
                    },
                ],
                ..meta_edit("a.y")
            },
        )
        .expect("改得动");
        r.write_back().unwrap();

        let again = ParamRegistry::load_from(d.path()).expect("改完还得读得通");
        let py = again.param("a.y").unwrap();
        assert_eq!(
            py.deprecated_machines,
            ["A1", "P1S"],
            "trim / 去空 / 去重没做"
        );
        assert!(!py.deprecated, "点名的机型不许写成全局弃用");
        assert!(py.deprecated_for("A1") && py.deprecated_for("P1S"));
        assert!(!py.deprecated_for("X1C"), "没点名的机型被误伤");

        assert_eq!(py.choices[1].deprecated_machines, ["A1"]);
        assert!(py.choices[1].deprecated_for("A1"));
        assert!(!py.choices[1].deprecated_for("P1S"));
        assert!(!py.choices[1].deprecated, "点名机型不许写成选项级全局弃用");
        assert_eq!(
            again
                .deprecated_choice_values_for("a.y", "A1")
                .into_iter()
                .collect::<Vec<_>>(),
            ["off"]
        );
        assert!(
            again.deprecated_choice_values_for("a.y", "P1S").is_empty(),
            "P1S 上这一档还能写"
        );
        assert!(
            again.deprecated_choice_values("a.y").contains("off"),
            "全机型并集漏了这一档"
        );
        assert!(again.to_toml().contains("deprecatedMachines"));
    }

    /// 两个装参数的分组：搬组要同时改 `[params.layout]` 与 `layout_schema` 的 items
    fn editable_two_sections() -> (Value, Value) {
        let (mut params, mut layout) = editable();
        params["tabs"][0]["sections"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({ "id": "s2", "label": "分组二", "order": 1 }));
        // 空组在真数据里就是 `items = []`（内联空数组），不是表数组
        layout["tabs"][0]["sections"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({ "id": "s2", "items": [] }));
        (params, layout)
    }

    #[test]
    fn moving_a_param_to_another_section_rewrites_both_files() {
        let (p, l) = editable_two_sections();
        let (d, r) = load(p, l);
        let mut r = r.unwrap();
        r.set_param_meta(
            "a.x",
            ParamMetaEdit {
                section_id: Some("s2".into()),
                ..meta_edit("a.x")
            },
        )
        .expect("搬得动");
        r.write_back().unwrap();

        // 两边必须一起改：漏一边这里就加载不起来（check_consistency ②③）
        let again = ParamRegistry::load_from(d.path()).expect("搬完还得读得通");
        let px = again.param("a.x").unwrap();
        assert_eq!(px.layout.section_id, "s2");
        assert_eq!(px.layout.order, 1.0, "放进空组该从 1 起（组末）");
        let section_of = |id: &'static str| {
            again
                .layout()
                .iter()
                .flat_map(|t| t.sections.iter())
                .find(|s| s.id == id)
                .unwrap()
                .items
                .iter()
                .map(|i| i.param_key.as_str())
                .collect::<Vec<_>>()
        };
        assert_eq!(section_of("s2"), ["a.x"]);
        assert_eq!(section_of("s1"), ["a.mode", "a.y"], "旧组没摘干净");
    }

    /// 定义框分组下拉里那个「＋ 新增分组」：**同一笔载荷**把分组建出来、把参数放进去。
    ///
    /// 顺带钉住「重放不算第二次」—— 撤销 → 重做交回的是同一份载荷，
    /// 那时分组已经在了，`ensure_section` 得认出它就是自己建的那个。
    #[test]
    fn a_new_section_is_created_and_the_param_moves_into_it() {
        let (p, l) = editable();
        let (d, r) = load(p, l);
        let mut r = r.unwrap();
        let fresh = NewSection {
            id: "custom_1".into(),
            tab_id: "t1".into(),
            label: "试验分组".into(),
        };

        r.set_param_meta(
            "a.x",
            ParamMetaEdit {
                new_section: Some(fresh.clone()),
                ..meta_edit("a.x")
            },
        )
        .expect("建得动，也搬得动");
        r.write_back().unwrap();

        let again = ParamRegistry::load_from(d.path()).expect("建完还得读得通");
        assert_eq!(again.param("a.x").unwrap().layout.section_id, "custom_1");
        let hit = again
            .layout_sections()
            .into_iter()
            .find(|(_, _, id, _)| id == "custom_1")
            .expect("新分组没进 layout_sections —— 界面上就选不到它");
        assert_eq!(hit.0, "t1", "挂错页签");
        assert_eq!(hit.3, "试验分组");

        // 重放同一笔：不许建出第二个（否则 check_consistency ④ 下次加载就红）
        r.set_param_meta(
            "a.x",
            ParamMetaEdit {
                new_section: Some(fresh),
                ..meta_edit("a.x")
            },
        )
        .expect("重放也该没事");
        r.write_back().unwrap();
        let again = ParamRegistry::load_from(d.path()).expect("重放完还得读得通");
        let n = again
            .layout_sections()
            .iter()
            .filter(|(_, _, id, _)| id == "custom_1")
            .count();
        assert_eq!(n, 1, "同一笔载荷重放，分组建了两份");
    }

    /// id 不合规的分组**不许落盘** —— 它要进两份 TOML，还会进客户端目录
    #[test]
    fn a_bad_section_id_is_rejected() {
        let (p, l) = editable();
        let (d, r) = load(p, l);
        let mut r = r.unwrap();
        let before = r.to_toml();
        let e = r
            .set_param_meta(
                "a.x",
                ParamMetaEdit {
                    new_section: Some(NewSection {
                        id: "试验".into(),
                        tab_id: "t1".into(),
                        label: "试验分组".into(),
                    }),
                    ..meta_edit("a.x")
                },
            )
            .unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::InvalidArgument);
        assert_eq!(r.to_toml(), before, "被拒的载荷改到了文档");
        // 盘上也一个字没变
        let again = ParamRegistry::load_from(d.path()).expect("拒了也得读得通");
        assert_eq!(again.param("a.x").unwrap().layout.section_id, "s1");
    }

    /// **前端发的那个 JSON 形状**能不能解出来（2026-11-01 现场：点上下箭头一点反应
    /// 都没有，也没报错 —— 先钉死"载荷名字对不对"，别再靠推）。
    ///
    /// 前端发的是 `{ label: null, move: 'up' | 'down' }`（`workbench/api.ts` 的
    /// `SectionMetaEdit`）。这里三样都试：只有方向、只有名字、都没有。
    #[test]
    fn the_section_edit_payload_round_trips_from_the_frontend_shape() {
        let only_move: SectionMetaEdit =
            serde_json::from_value(serde_json::json!({ "label": null, "move": "up" }))
                .expect("`move: 'up'` 解不出来 —— 前端点了箭头后端就当成没改");
        assert_eq!(only_move.label, None);
        assert_eq!(only_move.move_dir, Some(SectionMove::Up));

        let down: SectionMetaEdit =
            serde_json::from_value(serde_json::json!({ "label": null, "move": "down" })).unwrap();
        assert_eq!(down.move_dir, Some(SectionMove::Down));

        let rename: SectionMetaEdit =
            serde_json::from_value(serde_json::json!({ "label": "支撑", "move": null })).unwrap();
        assert_eq!(rename.label.as_deref(), Some("支撑"));
        assert_eq!(rename.move_dir, None);

        let nothing: SectionMetaEdit =
            serde_json::from_value(serde_json::json!({ "label": null, "move": null })).unwrap();
        assert_eq!(nothing.label, None);
        assert_eq!(nothing.move_dir, None);
    }

    /// 夹具：三个分组（s1 装参数，s2 / s3 是空的 —— 空组不该被 `param_tabs` 滤掉
    /// 这件事不在这一条里，这里只借它们当"能挪的邻居"）
    fn three_sections() -> (Value, Value) {
        let (mut p, mut l) = editable();
        for (i, id) in ["s2", "s3"].iter().enumerate() {
            p["tabs"][0]["sections"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!({
                    "id": id,
                    "label": format!("分组{}", i + 2),
                    "order": (i + 1) as f64,
                }));
            l["tabs"][0]["sections"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!({ "id": id, "items": [] }));
        }
        (p, l)
    }

    /// 卡片头：**改名**与**换位置**（2026-11-01 作者：「这个标题这些能点击吧……
    /// 而且没办法去换它那个上下位置」）。
    ///
    /// 位置这件事有两个读者、两把尺子，所以一条测试同时钉住两边：
    /// 客户端按 `[[tabs.sections]]` 的**数组次序**排，工作台卡片按 `order` 排；
    /// 布局文件那份数组也跟着换（「所属分组」下拉按它出）。
    #[test]
    fn a_group_can_be_renamed_and_moved_within_its_tab() {
        let (p, l) = three_sections();
        let (d, r) = load(p, l);
        let mut r = r.unwrap();

        r.set_section_meta(
            "s2",
            SectionMetaEdit {
                label: Some("试验分组".into()),
                move_dir: None,
                new_section: None,
            },
        )
        .expect("改得动");
        r.set_section_meta(
            "s2",
            SectionMetaEdit {
                label: None,
                move_dir: Some(SectionMove::Up),
                new_section: None,
            },
        )
        .expect("挪得动");
        r.write_back().unwrap();

        let again = ParamRegistry::load_from(d.path()).expect("改完还得读得通");
        assert_eq!(again.section_meta("s2").unwrap().label, "试验分组");
        /* 注册表数组：s2 与 s1 换了位（读的是落盘那份文本，不是内存） */
        let toml = again.to_toml();
        let at = |id: &str| {
            /* 夹具写出来的引号风格不定，两种都认 */
            toml.find(&format!("id = '{id}'"))
                .or_else(|| toml.find(&format!("id = \"{id}\"")))
                .unwrap_or_else(|| panic!("{id} 在注册表里不见了"))
        };
        assert!(
            at("s2") < at("s1") && at("s1") < at("s3"),
            "注册表的数组次序没换 —— 客户端是按它排的\n{toml}"
        );
        let orders: Vec<f64> = ["s2", "s1", "s3"]
            .iter()
            .map(|id| again.section_meta(id).unwrap().order)
            .collect();
        assert_eq!(
            orders,
            [0.0, 1.0, 2.0],
            "order 没跟着数组位置重排 —— 卡片头会跟客户端各排各的"
        );
        let lorder: Vec<&str> = again
            .layout()
            .iter()
            .flat_map(|t| t.sections.iter())
            .map(|s| s.id.as_str())
            .collect();
        assert_eq!(
            lorder,
            ["s2", "s1", "s3"],
            "布局那份数组没跟着换 —— 「所属分组」下拉还是旧次序"
        );

        /* 已经在这一页的第一个了：要明说挪不动，不许默默不动 */
        let e = r
            .set_section_meta(
                "s2",
                SectionMetaEdit {
                    label: None,
                    move_dir: Some(SectionMove::Up),
                    new_section: None,
                },
            )
            .unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::InvalidArgument);
        assert!(
            e.message.contains("试验分组"),
            "报错没点名是哪个分组挪不动：{}",
            e.message
        );

        /* 名字空着也不许落盘 */
        let before = r.to_toml();
        let e = r
            .set_section_meta(
                "s1",
                SectionMetaEdit {
                    label: Some("  ".into()),
                    move_dir: None,
                    new_section: None,
                },
            )
            .unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::InvalidArgument);
        assert_eq!(r.to_toml(), before, "被拒的改名改到了文档");
    }

    /// 组内顺序：给一个新的 `order` 就换位置，**两份文件一起动**
    /// （`items` 顺序必须 == 按 order 升序，否则下次加载起不来）
    #[test]
    fn reordering_within_a_section_moves_the_layout_item_too() {
        let (p, l) = editable();
        let (d, r) = load(p, l);
        let mut r = r.unwrap();

        // a.y 本来在最后（3.0）—— 给它 0.5，排到最前
        r.set_param_meta(
            "a.y",
            ParamMetaEdit {
                order: Some(0.5),
                ..meta_edit("a.y")
            },
        )
        .expect("排得动");
        r.write_back().unwrap();

        let again = ParamRegistry::load_from(d.path()).expect("排完还得读得通");
        let keys: Vec<&str> = again
            .layout()
            .iter()
            .flat_map(|t| t.sections.iter())
            .find(|s| s.id == "s1")
            .unwrap()
            .items
            .iter()
            .map(|i| i.param_key.as_str())
            .collect();
        assert_eq!(keys, ["a.y", "a.mode", "a.x"]);
        assert_eq!(again.param("a.y").unwrap().layout.order, 0.5);
        // 别人的顺序一个都不许被顺手改掉
        assert_eq!(again.param("a.mode").unwrap().layout.order, 1.0);
        assert_eq!(again.param("a.x").unwrap().layout.order, 2.0);
    }

    #[test]
    fn moving_a_param_into_a_component_section_is_refused() {
        let (mut p, mut l) = editable();
        // 客户端设置页那种整块组件：component 有值、items 为空
        p["tabs"].as_array_mut().unwrap().push(serde_json::json!({
            "id": "t2", "label": "软件设置", "order": 80,
            "sections": [{ "id": "card", "label": "外观", "order": 1 }]
        }));
        l["tabs"].as_array_mut().unwrap().push(serde_json::json!({
            "id": "t2",
            "sections": [{ "id": "card", "items": [], "component": "appearance-card" }]
        }));
        let (_d, r) = load(p, l);
        let mut r = r.unwrap();
        let before = r.to_toml();
        let e = r
            .set_param_meta(
                "a.x",
                ParamMetaEdit {
                    section_id: Some("card".into()),
                    ..meta_edit("a.x")
                },
            )
            .expect_err("整块组件收不了参数");
        assert!(e.message.contains("整块自定义组件"), "{}", e.message);
        assert_eq!(before, r.to_toml(), "被拒之后文档被改了");
        assert_eq!(r.param("a.x").unwrap().layout.section_id, "s1");
    }

    /// 同一个 section id 落在两个页签下 —— 界面归属会有两个答案，加载就该拒
    #[test]
    fn duplicate_section_ids_are_refused_at_load() {
        let (mut p, mut l) = good();
        p["tabs"].as_array_mut().unwrap().push(serde_json::json!({
            "id": "t2", "label": "软件设置", "order": 80,
            "sections": [{ "id": "s1", "label": "校准", "order": 600 }]
        }));
        l["tabs"].as_array_mut().unwrap().push(serde_json::json!({
            "id": "t2",
            "sections": [{ "id": "s1", "items": [], "component": "calibration-card" }]
        }));
        let (_d, r) = load(p, l);
        let e = r.expect_err("重复的 section id 必须拒");
        assert!(e.message.contains("声明了两次"), "{}", e.message);
    }

    /// 控件 × 值类型的组合得画得出真的：开关挂在 float 上（点了不切换的那种）、
    /// 步进框挂在 bool 上、分段没有可选项、文本框带着可选项 —— 全拒
    #[test]
    fn meta_edit_rejects_control_combos_that_draw_fake_controls() {
        let (p, l) = editable();
        let (_d, r) = load(p, l);
        let mut r = r.unwrap();

        // 开关 + float —— 作者实测的那一手：「改成了开关，点了没切换」
        let e = r
            .set_param_meta(
                "a.x",
                ParamMetaEdit {
                    ui_component: UiComponent::Switch,
                    ..meta_edit("a.x")
                },
            )
            .unwrap_err();
        assert!(e.message.contains("开关"), "{}", e.message);

        // 步进框 + bool
        let e = r
            .set_param_meta(
                "a.x",
                ParamMetaEdit {
                    value_type: ValueType::Bool,
                    ui_component: UiComponent::Number,
                    default_value: serde_json::json!(true),
                    ..meta_edit("a.x")
                },
            )
            .unwrap_err();
        assert!(e.message.contains("步进框"), "{}", e.message);

        // 分段 + 字符串但没有可选项
        let e = r
            .set_param_meta(
                "a.y",
                ParamMetaEdit {
                    value_type: ValueType::Text,
                    ui_component: UiComponent::Segmented,
                    default_value: serde_json::json!("on"),
                    ..meta_edit("a.y")
                },
            )
            .unwrap_err();
        assert!(e.message.contains("可选项"), "{}", e.message);

        // 文本框 + 带着可选项（要么换控件，要么清空）
        let e = r
            .set_param_meta(
                "a.y",
                ParamMetaEdit {
                    value_type: ValueType::Text,
                    ui_component: UiComponent::Text,
                    default_value: serde_json::json!("on"),
                    choices: vec![choice("开", serde_json::json!("on"), false)],
                    ..meta_edit("a.y")
                },
            )
            .unwrap_err();
        assert!(e.message.contains("文本框"), "{}", e.message);

        // 可选项的值重复 —— 下拉里两行一个键是假选项
        let e = r
            .set_param_meta(
                "a.y",
                ParamMetaEdit {
                    value_type: ValueType::Text,
                    ui_component: UiComponent::Select,
                    default_value: serde_json::json!("on"),
                    choices: vec![
                        choice("开", serde_json::json!("on"), false),
                        choice("又是开", serde_json::json!("on"), false),
                    ],
                    ..meta_edit("a.y")
                },
            )
            .unwrap_err();
        assert!(e.message.contains("重复"), "{}", e.message);

        // 拒了就一格不动
        assert_eq!(r.param("a.x").unwrap().ui_component, UiComponent::Number);
        assert_eq!(r.param("a.y").unwrap().ui_component, UiComponent::Select);
    }

    /// 默认值得跟值类型对得上；枚举的默认值得是选项之一。拒了之后**一格不动**
    #[test]
    fn meta_edit_rejects_defaults_that_do_not_fit_the_type() {
        let (p, l) = editable();
        let (_d, r) = load(p, l);
        let mut r = r.unwrap();

        // float 挂个字符串默认
        let e = r
            .set_param_meta(
                "a.x",
                ParamMetaEdit {
                    default_value: serde_json::json!("x"),
                    ..meta_edit("a.x")
                },
            )
            .unwrap_err();
        assert!(e.message.contains("对不上"), "{}", e.message);

        // 枚举的默认值不在选项里（照模态框的口径：可选项整表带回）
        let e = r
            .set_param_meta(
                "a.y",
                ParamMetaEdit {
                    value_type: ValueType::Text,
                    ui_component: UiComponent::Select,
                    default_value: serde_json::json!("mid"),
                    choices: vec![
                        choice("开", serde_json::json!("on"), false),
                        choice("关", serde_json::json!("off"), false),
                    ],
                    ..meta_edit("a.y")
                },
            )
            .unwrap_err();
        assert!(e.message.contains("可选项"), "{}", e.message);

        assert_eq!(r.param("a.x").unwrap().label, "a.x", "拒了还动了内存");
        assert_eq!(r.param("a.y").unwrap().label, "a.y", "拒了还动了内存");
    }

    /// 父项判据：不存在 / 是自己 / 父项也是子项 / 自己已经挂着儿子 —— 全拒
    #[test]
    fn meta_edit_rejects_parents_that_break_two_levels() {
        let (p, l) = editable();
        let (_d, r) = load(p, l);
        let mut r = r.unwrap();

        let e = r
            .set_param_meta(
                "a.x",
                ParamMetaEdit {
                    parent_key: Some("ghost".into()),
                    ..meta_edit("a.x")
                },
            )
            .unwrap_err();
        assert!(e.message.contains("不在字段定义里"), "{}", e.message);

        let e = r
            .set_param_meta(
                "a.x",
                ParamMetaEdit {
                    parent_key: Some("a.x".into()),
                    ..meta_edit("a.x")
                },
            )
            .unwrap_err();
        assert!(e.message.contains("自己"), "{}", e.message);

        // 先让 a.y 当 a.mode 的儿子（它自己没有儿子，这一步合法），
        // 再让 a.x 挂到 a.y 下面 —— 三层了，拒
        r.set_param_meta(
            "a.y",
            ParamMetaEdit {
                parent_key: Some("a.mode".into()),
                ..meta_edit("a.y")
            },
        )
        .unwrap();
        let e = r
            .set_param_meta(
                "a.x",
                ParamMetaEdit {
                    parent_key: Some("a.y".into()),
                    ..meta_edit("a.x")
                },
            )
            .unwrap_err();
        assert!(e.message.contains("两级"), "{}", e.message);

        // a.mode 已经挂着 a.x —— 它不能再去当别人的儿子
        let e = r
            .set_param_meta(
                "a.mode",
                ParamMetaEdit {
                    parent_key: Some("a.y".into()),
                    ..meta_edit("a.mode")
                },
            )
            .unwrap_err();
        assert!(e.message.contains("子项"), "{}", e.message);
    }

    /// 前置条件得指着一条真实存在的参数，且不能指着自己；值**不校验** ——
    /// 实测里就有字符串化的数字，归一是 visibility 的事
    #[test]
    fn meta_edit_checks_show_when_but_leaves_its_value_alone() {
        let (p, l) = editable();
        let (_d, r) = load(p, l);
        let mut r = r.unwrap();

        let e = r
            .set_param_meta(
                "a.x",
                ParamMetaEdit {
                    show_when: Some(ShowWhen {
                        key: "ghost".into(),
                        op: ShowOp::Eq,
                        value: serde_json::json!(1),
                    }),
                    ..meta_edit("a.x")
                },
            )
            .unwrap_err();
        assert!(e.message.contains("不在字段定义里"), "{}", e.message);

        let e = r
            .set_param_meta(
                "a.x",
                ParamMetaEdit {
                    show_when: Some(ShowWhen {
                        key: "a.x".into(),
                        op: ShowOp::Eq,
                        value: serde_json::json!(1),
                    }),
                    ..meta_edit("a.x")
                },
            )
            .unwrap_err();
        assert!(e.message.contains("自己"), "{}", e.message);

        // 字符串化的数字照收 —— 那是实测真数据里的形状
        r.set_param_meta(
            "a.x",
            ParamMetaEdit {
                show_when: Some(ShowWhen {
                    key: "a.mode".into(),
                    op: ShowOp::Gt,
                    value: serde_json::json!("0"),
                }),
                ..meta_edit("a.x")
            },
        )
        .expect("字符串化的数字是合法的前置条件值");
    }

    /// 数值与名字的门禁：范围反了、步进非正、名称空白 —— 全拒
    #[test]
    fn meta_edit_rejects_bad_numbers_and_a_blank_label() {
        let (p, l) = editable();
        let (_d, r) = load(p, l);
        let mut r = r.unwrap();

        let e = r
            .set_param_meta(
                "a.x",
                ParamMetaEdit {
                    min: Some(5.0),
                    max: Some(1.0),
                    ..meta_edit("a.x")
                },
            )
            .unwrap_err();
        assert!(e.message.contains("范围反了"), "{}", e.message);

        let e = r
            .set_param_meta(
                "a.x",
                ParamMetaEdit {
                    step: Some(0.0),
                    ..meta_edit("a.x")
                },
            )
            .unwrap_err();
        assert!(e.message.contains("正数"), "{}", e.message);

        let e = r.set_param_meta("a.x", meta_edit("   ")).unwrap_err();
        assert!(e.message.contains("名称"), "{}", e.message);

        assert_eq!(r.param("a.x").unwrap().label, "a.x", "拒了还动了内存");
    }

    /// 真文件上的定义编辑：值覆盖丰富的块也一样只动自己那一个块
    #[test]
    fn a_meta_edit_on_the_real_file_touches_one_block_only() {
        let Some((tmp, mut r)) = copy_of_real() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let key = "toolhead.MKP_retract";
        let vt = r.param(key).unwrap().value_type;
        let uc = r.param(key).unwrap().ui_component;
        let dv = r.param(key).unwrap().default_value.clone();
        let before = r.to_toml();

        r.set_param_meta(
            key,
            ParamMetaEdit {
                label: "回抽长度（改）".into(),
                unit: Some("cm".into()),
                step: Some(0.1),
                deprecated: true,
                value_type: vt,
                ui_component: uc,
                default_value: dv,
                ..meta_edit("回抽长度（改）")
            },
        )
        .expect("真数据上改得动");
        r.write_back().expect("落盘");

        let again = ParamRegistry::load_from(tmp.path()).expect("改完还得读得通");
        let p = again.param(key).unwrap();
        assert_eq!(p.label, "回抽长度（改）");
        assert_eq!(p.unit.as_deref(), Some("cm"));
        assert_eq!(p.step, Some(0.1));
        assert!(p.deprecated);
        // 值覆盖一张不少
        assert_eq!(
            p.machine_variants,
            r.param(key).unwrap().machine_variants,
            "定义编辑动了值覆盖"
        );

        let (removed, inserted) = one_edit_only(&before, &again.to_toml());
        assert!(
            !removed.contains("[[params]]"),
            "波及了别的字段块：{removed:?}"
        );
        assert!(
            !inserted.contains("[[params]]"),
            "插入段跨到了别的字段：{inserted:?}"
        );
    }
}
