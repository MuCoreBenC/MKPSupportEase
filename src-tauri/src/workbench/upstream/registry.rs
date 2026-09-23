//! `param_registry.json` + `layout_schema.json` —— 字段定义的 SSOT。
//!
//! # 两个文件互补，不是副本
//!
//! | 面 | 文件 | 承载什么 |
//! |---|---|---|
//! | 分组元数据 | `param_registry.tabs[]` | **中文名、排序、图标**（`layout_schema` 全文没有这三样） |
//! | 参数摆放 | `layout_schema.tabs[].sections[].items[]` | 哪个参数落在哪个 section、**section 级可见性与自定义组件** |
//! | 参数自带归属 | `params[].layout` | `{order, sectionId}`，参数自己声明它属于哪个 section |
//!
//! 所以「参数 → section」这件事**有两处声明**（`params[].layout.sectionId` 与
//! `items[].paramKey`），实测两处一致，但**上游没有任何门禁保证它们不漂移** ——
//! 这就是 [`Registry::check_consistency`] 存在的理由。
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
//! [`Registry::param_tabs`]。

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::error::AppError;
use crate::workbench::paths;
use crate::workbench::store::read_json;

const REGISTRY_REL: &str = "content/param_registry.json";
const LAYOUT_REL: &str = "content/layout_schema.json";

/* ---------- 值与控件 ---------- */

/// 上游的 `valueType`。实测分布 float 43 / string 15 / bool 12 / int 4
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValueType {
    Float,
    Int,
    Bool,
    /// 上游写的是 `"string"`；这里叫 `Text` 只为躲开 `String` 这个名字
    #[serde(rename = "string")]
    Text,
}

/// 上游的 `uiComponent`。**实测只有这五种**，没有第六种
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UiComponent {
    Number,
    Switch,
    Segmented,
    Select,
    Gcode,
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
    pub value: Value,
    #[serde(default)]
    pub deprecated: bool,
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
    pub desc: String,
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
    /// 上游是**逗号分隔字符串**（`"A1,A1_MINI,A2L"`），这里拆成数组。
    /// 空 = 不限机型
    #[serde(default, deserialize_with = "comma_separated")]
    pub machine_filter: Vec<String>,
    /// 实测 7 条为 true，**没有 false 占位**，所以缺省即 false
    #[serde(default)]
    pub deprecated: bool,
    /// 按机型/版本的**值**覆盖。键有两种形态：`"P1S"`（纯机型）与 `"P1S:LITE"`（版本）。
    /// 它**不属于出厂层** —— 由 `domain::variants` 按三步归并消化进机型基底与版本覆盖（doc §3.3）
    #[serde(default)]
    pub machine_variants: BTreeMap<String, Value>,
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
    #[serde(default)]
    pub serialization: Option<Serialization>,
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
}

/* ---------- 分组元数据（中文名与顺序的唯一权威） ---------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionMeta {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    pub order: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TabMeta {
    pub id: String,
    pub label: String,
    pub order: f64,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub sections: Vec<SectionMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ParamRegistryFile {
    params: Vec<ParamDef>,
    #[serde(default)]
    tabs: Vec<TabMeta>,
    #[serde(default)]
    updated: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LayoutSchemaFile {
    #[serde(default)]
    tabs: Vec<LayoutTab>,
}

/* ---------- 合起来的字段定义 ---------- */

pub struct Registry {
    params: Vec<ParamDef>,
    tabs: Vec<TabMeta>,
    layout: Vec<LayoutTab>,
    /// `param_registry.json` 的 `updated`，给界面显示
    pub updated: String,
    /// key → params 下标
    index: HashMap<String, usize>,
}

/// 手写而不是 `#[derive(Debug)]`：派生版会把 74 条参数定义连同布局整本打印出来，
/// 而 `unwrap_err()` 失败时打的就是它 —— 真正的失败原因会被冲到几千行之外
impl std::fmt::Debug for Registry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Registry({} 参数 / {} tab / updated {})",
            self.params.len(),
            self.tabs.len(),
            self.updated
        )
    }
}

impl Registry {
    /// 从真上游读。定位不到上游时是 `NOT_FOUND` 并写明试过哪里
    pub fn load() -> Result<Self, AppError> {
        let root = paths::upstream_root().ok_or_else(|| {
            AppError::not_found("找不到上游预设仓库 mkpse-presets")
                .with_detail(format!("试过：{}", paths::upstream_candidates().join("；")))
        })?;
        Self::load_from(&root)
    }

    /// 从指定的上游根读。单测用这一支，不碰真仓库
    pub fn load_from(root: &Path) -> Result<Self, AppError> {
        let reg: ParamRegistryFile = read_json(&root.join(REGISTRY_REL), "字段定义")?;
        let layout: LayoutSchemaFile = read_json(&root.join(LAYOUT_REL), "参数布局")?;

        let index = reg
            .params
            .iter()
            .enumerate()
            .map(|(i, p)| (p.key.clone(), i))
            .collect();

        let out = Self {
            params: reg.params,
            tabs: reg.tabs,
            layout: layout.tabs,
            updated: reg.updated,
            index,
        };
        out.check_consistency()?;
        Ok(out)
    }

    /// 上游没有这道门禁，所以在这里立三条。
    ///
    /// 它们不成立时的后果都是**静默的**：矩阵少几行、或多出永远为空的分类、或把 X 和 Y
    /// 调个头 —— 没有一条会自己报错。所以判据必须在加载时就拦住，而不是等界面看起来怪。
    pub fn check_consistency(&self) -> Result<(), AppError> {
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
            return Err(AppError::corrupted("同一个参数被放进了多个 section").with_detail(
                format!("重复的 paramKey：{}", dup.join("、")),
            ));
        }
        let missing: Vec<&str> = param_keys.difference(&layout_set).copied().collect();
        if !missing.is_empty() {
            return Err(
                AppError::corrupted("有参数没有被任何 section 收下（它在界面上不会出现）")
                    .with_detail(format!("缺席的 key：{}", missing.join("、"))),
            );
        }
        let dangling: Vec<&str> = layout_set.difference(&param_keys).copied().collect();
        if !dangling.is_empty() {
            return Err(
                AppError::corrupted("布局里引用了字段定义里没有的参数").with_detail(format!(
                    "悬空的 paramKey：{}",
                    dangling.join("、")
                )),
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
                    return Err(AppError::corrupted(format!(
                        "section {} 两侧的参数数对不上",
                        s.id
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
                let by_items: Vec<&str> =
                    s.items.iter().map(|i| i.param_key.as_str()).collect();
                if by_order != by_items {
                    return Err(AppError::corrupted(format!(
                        "section {} 的 items 顺序与 layout.order 不一致",
                        s.id
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

    pub fn param(&self, key: &str) -> Option<&ParamDef> {
        self.index.get(key).map(|i| &self.params[*i])
    }

    /// 这台机型**真实看得到**的字段，按 `layout.order` 升序。
    ///
    /// 两道过滤：`machineFilter` 排除的不属于这台机型；`deprecated` 的不再显示、不进产物。
    /// 归并机型基底时也走这一条（doc §3.3 的 `visibleKeys`）—— 被排除的字段不该出现在
    /// 它的基底里。
    pub fn visible_keys(&self, machine_id: &str) -> Vec<&str> {
        let mut hit: Vec<&ParamDef> = self
            .params
            .iter()
            .filter(|p| !p.deprecated && p.applies_to(machine_id))
            .collect();
        hit.sort_by(|a, b| a.layout.order.total_cmp(&b.layout.order));
        hit.into_iter().map(|p| p.key.as_str()).collect()
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

    /// 矩阵的分类页签：**只留装参数的那些**，中文名与顺序取 `param_registry.tabs`
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
    /// 查的是 `layout_schema` 而不是 `param_registry.tabs`：两处都声明了归属，
    /// 但参数摆在哪个 tab 下是布局说的（`param_registry.tabs` 给的是中文名与顺序）
    pub fn tab_of_section(&self, section_id: &str) -> Option<&str> {
        self.layout
            .iter()
            .find(|t| t.sections.iter().any(|s| s.id == section_id))
            .map(|t| t.id.as_str())
    }

    /// 一个 section 的中文名与组内序。**中文名与顺序的唯一权威是 `param_registry.tabs`**
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
    /// 上游多一个没在 `tabs` 里声明的页签时，它该排到最后，而不是插到最前面
    pub fn tab_order(&self, tab_id: &str) -> f64 {
        self.tabs
            .iter()
            .find(|t| t.id == tab_id)
            .map_or(f64::MAX, |t| t.order)
    }

    /// 全表指纹。进有效配方的 hash —— 否则改了字段定义，产物不会变成"待生成"
    pub fn fingerprint(&self) -> String {
        use sha2::{Digest, Sha256};
        let payload = serde_json::json!({
            "params": &self.params,
            "tabs": &self.tabs,
            "layout": &self.layout,
        });
        let mut h = Sha256::new();
        h.update(serde_json::to_vec(&payload).unwrap_or_default());
        format!("{:x}", h.finalize())
    }
}

/// 上游把机型清单写成 `"A1,A1_MINI,A2L"` 这种逗号串。
///
/// 拆的时候顺手 trim 并丢掉空段 —— 上游现在没有 `"A1, A1_MINI"` 这种带空格的写法，
/// 但一个多打的空格会让 `"A1_MINI"` 变成 `" A1_MINI"`，然后这台机型的字段静默消失
fn comma_separated<'de, D>(d: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw: Option<String> = Option::deserialize(d)?;
    Ok(raw
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 造一份两文件都自洽的最小上游。**测试不碰真仓库** ——
    /// 判据要在任何机器上都成立
    fn fixture(dir: &Path, params: Value, layout: Value) {
        // 走仓库唯一的写盘出口。`clippy.toml` 把 `std::fs::write` 列进了
        // disallowed-methods，测试代码也在那道门禁之内
        let content = dir.join("content");
        crate::fsx::atomic::atomic_write_json(&content.join("param_registry.json"), &params)
            .unwrap();
        crate::fsx::atomic::atomic_write_json(&content.join("layout_schema.json"), &layout).unwrap();
    }

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

    fn load(params: Value, layout: Value) -> (tempfile::TempDir, Result<Registry, AppError>) {
        let d = tempfile::tempdir().unwrap();
        fixture(d.path(), params, layout);
        let r = Registry::load_from(d.path());
        (d, r)
    }

    #[test]
    fn loads_a_consistent_pair() {
        let (p, l) = good();
        let (_d, r) = load(p, l);
        let r = r.unwrap();
        assert_eq!(r.params().len(), 2);
        assert_eq!(r.updated, "2026-01-01 00:00:00");
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
        assert!(e.detail.unwrap_or_default().contains("a.z"), "没点名真因");
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
        assert!(e.message.contains("悬空") || e.detail.unwrap_or_default().contains("ghost"));
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

        assert_eq!(r.visible_keys("A1"), vec!["a.w"], "A1 只该看到不限机型且未废弃的");
        assert_eq!(
            r.visible_keys("P1S"),
            vec!["a.w", "a.x"],
            "P1S 还该看到点名给它的那个，且按 order 升序"
        );
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
        assert_eq!(tabs[0].label, "页签一", "中文名取 param_registry.tabs");
    }

    /// 分组元数据：中文名与组内序都从 `param_registry.tabs` 来，查不到就是 `None`。
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
    /// 于是上游随手加一个 tab 就把整张矩阵的行序掀翻
    #[test]
    fn tab_order_puts_unknown_tabs_last() {
        let (p, l) = good();
        let (_d, r) = load(p, l);
        let r = r.unwrap();
        assert_eq!(r.tab_order("t1"), 10.0);
        assert_eq!(r.tab_order("ghost"), f64::MAX);
        assert!(r.tab_order("t1") < r.tab_order("ghost"));
    }

    /// 指纹：改一个 step 就要变（否则改了字段定义产物不会过期）
    #[test]
    fn fingerprint_reacts_to_any_field_change() {
        let (p, l) = good();
        let (_d1, a) = load(p.clone(), l.clone());
        let before = a.unwrap().fingerprint();

        let mut p2 = p;
        p2["params"][0]["step"] = serde_json::json!(0.05);
        let (_d2, b) = load(p2, l);
        assert_ne!(before, b.unwrap().fingerprint());
    }

    /// 定位不到上游时是 NOT_FOUND 且写明试过哪里 —— 不是静默空表
    #[test]
    fn missing_files_are_not_found() {
        let d = tempfile::tempdir().unwrap();
        let e = Registry::load_from(d.path()).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::NotFound);
    }

    /* ---------- 真上游对齐（只在定位到上游的机器上执行） ---------- */

    /// **诚实边界**：这一条在没有上游的机器上什么都不做，所以它**不是门禁**，
    /// 是本机的对齐检查。真正的门禁是上面那些跑在内联夹具上的判据 ——
    /// 它们在任何机器上都执行。
    ///
    /// 它验的是**不变式而不是条数**：`params[].key` 与布局 `paramKey` 双射、
    /// 每 section 两侧计数相等、items 顺序等于 order 升序 —— 全在
    /// `check_consistency` 里，`load()` 成功本身就是它们成立的证据。
    /// 刻意**不断言 74 / 8 / 27 这些数字**：上游一改它们就变，钉住数量只会
    /// 在下一次上游更新时产出一条与代码无关的红。
    #[test]
    fn real_upstream_loads_and_satisfies_the_invariants() {
        let Some(root) = paths::upstream_root() else {
            eprintln!("没定位到上游 mkpse-presets，这条对齐检查未执行（不是通过）");
            return;
        };
        let r = Registry::load_from(&root).expect("真上游读不出来或三条一致性断言不过");

        // 反空转：真上游不可能是空表
        assert!(!r.params().is_empty(), "读出来 0 个参数，判据已空转");
        assert!(!r.param_tabs().is_empty(), "一个装参数的 tab 都没有");

        // 我建的结构有没有把上游字段读丢：逐条比对原始 JSON 的键集与我反序列化后
        // 再序列化出来的键集。少读一个字段在这里会现形，而条数变化不会
        let raw: Value = serde_json::from_str(
            &std::fs::read_to_string(root.join(REGISTRY_REL)).unwrap(),
        )
        .unwrap();
        let known: BTreeSet<&str> = [
            "key", "configKey", "tomlKey", "jsonKey", "label", "desc", "tomlComment",
            "valueType", "uiComponent", "defaultValue", "scope", "section", "layout", "min",
            "max", "step", "unit", "parentKey", "showWhen", "choices", "variantMode",
            "machineFilter", "deprecated", "machineVariants", "machineMinVariants",
            "machineMaxVariants", "mergeGroup", "pinned", "serialization",
            // 刻意不读的两个：实测 74/74 全 true，零区分度
            "mergeable", "selectable",
        ]
        .into_iter()
        .collect();
        let mut unknown: BTreeSet<String> = BTreeSet::new();
        for p in raw["params"].as_array().unwrap() {
            for k in p.as_object().unwrap().keys() {
                if !known.contains(k.as_str()) {
                    unknown.insert(k.clone());
                }
            }
        }
        assert!(
            unknown.is_empty(),
            "上游有我没读的字段：{:?} —— 要么读进来，要么在这条清单里写明为什么不读",
            unknown
        );
    }
}
