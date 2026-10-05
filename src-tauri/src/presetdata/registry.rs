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
    /// 源数据里是**逗号分隔字符串**（`"A1,A1_MINI,A2L"`），这里拆成数组。
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

/// 一次「参数定义」编辑的载荷 —— `wb_set_param_meta` 整包提交（模态框一次
/// 保存改的可能不止一格）。`None` 的意思是**这一格清空**（文件里删键），不是「不动」。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParamMetaEdit {
    pub label: String,
    pub desc: String,
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
    /// 可选项整表（`[[params.choices]]`）。字符串枚举在模态框里编辑；
    /// 其他类型身上挂的几条（bool 的开/关叫法、数字的预设档）界面不给编辑、
    /// 原样带回 —— 这里不区分，整表照写
    #[serde(default)]
    pub choices: Vec<Choice>,
    /// 适用机型。空 = 不限机型（与 [`ParamDef::machine_filter`] 同一口径）
    #[serde(default)]
    pub machine_filter: Vec<String>,
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

/// `param_registry.toml` 的反序列化目标
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegistryFile {
    #[serde(default)]
    updated: String,
    #[serde(default)]
    tabs: Vec<TabMeta>,
    #[serde(default)]
    params: Vec<ParamDef>,
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
        let parsed: RegistryFile = toml_edit::de::from_str(&text).map_err(|e| {
            AppError::corrupted(format!("{} 的结构对不上字段定义模型", file.display()))
                .with_detail(e.to_string())
        })?;

        let layout_file = root.join("layout_schema.toml");
        let layout_text = super::read(&layout_file)?;
        let layout_doc = super::parse_text(&layout_text, &layout_file)?;
        let layout: LayoutFile = toml_edit::de::from_str(&layout_text).map_err(|e| {
            AppError::corrupted(format!("{} 的结构对不上布局模型", layout_file.display()))
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

    /// 数据本身没有这道门禁，所以在这里立三条。
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
            return Err(AppError::corrupted("同一个参数被放进了多个 section")
                .with_detail(format!("重复的 paramKey：{}", dup.join("、"))));
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
            return Err(AppError::corrupted("布局里引用了字段定义里没有的参数")
                .with_detail(format!("悬空的 paramKey：{}", dangling.join("、"))));
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
                let by_items: Vec<&str> = s.items.iter().map(|i| i.param_key.as_str()).collect();
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
    pub fn deprecated_choice_values(&self, key: &str) -> BTreeSet<String> {
        let Some(p) = self.param(key) else {
            return BTreeSet::new();
        };
        let mut out = BTreeSet::new();
        for c in &p.choices {
            if c.deprecated {
                out.insert(json_key(&c.value));
            }
        }
        for c in &p.choices {
            // 被这一档放开的参数：showWhen 指向本参数、op = eq、值就是这一档
            let gated: Vec<&ParamDef> = self
                .params
                .iter()
                .filter(|q| {
                    q.show_when.as_ref().is_some_and(|sw| {
                        sw.key == key
                            && sw.op == ShowOp::Eq
                            && json_key(&sw.value) == json_key(&c.value)
                    })
                })
                .collect();
            if !gated.is_empty() && gated.iter().all(|q| q.deprecated) {
                out.insert(json_key(&c.value));
            }
        }
        out
    }

    /// 这个值是不是该参数的一个**已弃用选项**。命中给回选项的中文名 ——
    /// 写值那道闸（`patch::validate`）的错误消息要点名是哪一档。
    /// `value = null`（挂回继承）不在这里判：删键不是写值。
    pub fn deprecated_choice_hit(&self, key: &str, value: &Value) -> Option<String> {
        if value.is_null() {
            return None;
        }
        let dead = self.deprecated_choice_values(key);
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
        let at = *self
            .index
            .get(key)
            .ok_or_else(|| AppError::invalid_argument(format!("字段定义里没有 {key}")))?;
        check_owner(owner)?;
        let item = to_toml_value(value, self.params[at].value_type, key)?;

        let t = self.param_table_mut(at, key)?;
        let table = t
            .entry("machineVariants")
            .or_insert(toml_edit::Item::Table(toml_edit::Table::new()))
            .as_table_mut()
            .ok_or_else(|| AppError::corrupted(format!("{key} 的 machineVariants 不是一张表")))?;
        table[owner] = item;

        self.params[at]
            .machine_variants
            .insert(owner.to_owned(), value.clone());
        Ok(())
    }

    /// 「清空」一个机型/版本的值 = **把那个键整行删掉**，不是写 `0` 或空串。
    ///
    /// 这一条与元字段那边同一个道理：没有这一行 = 「跟着上一层走」，
    /// 而写一个值 = 「我在这一层特意定了它」。两件事在界面上必须分得开。
    ///
    /// 删到这张表为空时，**连表头一起删** —— 留一个空的
    /// `[params.machineVariants]` 在文件里，读起来像"这里有机型差异"，而其实没有
    pub fn clear_variant(&mut self, key: &str, owner: &str) -> Result<(), AppError> {
        let at = *self
            .index
            .get(key)
            .ok_or_else(|| AppError::invalid_argument(format!("字段定义里没有 {key}")))?;
        check_owner(owner)?;

        let t = self.param_table_mut(at, key)?;
        let empty_now = match t.get_mut("machineVariants").and_then(|i| i.as_table_mut()) {
            Some(table) => {
                table.remove(owner);
                table.is_empty()
            }
            None => false,
        };
        if empty_now {
            t.remove("machineVariants");
        }

        self.params[at].machine_variants.remove(owner);
        Ok(())
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
        let at = *self
            .index
            .get(key)
            .ok_or_else(|| AppError::invalid_argument(format!("字段定义里没有 {key}")))?;

        let label = edit.label.trim();
        if label.is_empty() {
            return Err(AppError::invalid_argument("名称不能是空的"));
        }
        for (name, v) in [("最小", edit.min), ("最大", edit.max)] {
            if let Some(v) = v {
                if !v.is_finite() {
                    return Err(AppError::invalid_argument(format!(
                        "{name}值得是个有限数字，见到 {v}"
                    )));
                }
            }
        }
        if let (Some(min), Some(max)) = (edit.min, edit.max) {
            if min > max {
                return Err(AppError::invalid_argument(format!(
                    "范围反了：最小 {min} 比最大 {max} 还大"
                )));
            }
        }
        if let Some(s) = edit.step {
            if !s.is_finite() || s <= 0.0 {
                return Err(AppError::invalid_argument(format!(
                    "步进得是正数，见到 {s}"
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
                return Err(AppError::invalid_argument(format!(
                    "{key} 的出厂默认 {v} 与值类型 {}（{}）对不上",
                    value_type_str(t),
                    match t {
                        ValueType::Float | ValueType::Int => "要数字",
                        ValueType::Bool => "要 true/false",
                        ValueType::Text => "要字符串",
                    }
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
                return Err(AppError::invalid_argument(format!(
                    "开关只认 true/false —— {} 的参数用步进框（数字）",
                    value_type_str(edit.value_type)
                )));
            }
            UiComponent::Number
                if !matches!(edit.value_type, ValueType::Float | ValueType::Int) =>
            {
                return Err(AppError::invalid_argument(format!(
                    "步进框只配数字（float/int）—— 这条是 {}",
                    value_type_str(edit.value_type)
                )));
            }
            UiComponent::Segmented | UiComponent::Select if edit.value_type != ValueType::Text => {
                return Err(AppError::invalid_argument(format!(
                    "分段/下拉只配字符串枚举 —— 这条是 {}",
                    value_type_str(edit.value_type)
                )));
            }
            UiComponent::Segmented | UiComponent::Select if edit.choices.is_empty() => {
                return Err(AppError::invalid_argument(
                    "分段/下拉得先有可选项 —— 先在「可选项」里加几条",
                ));
            }
            UiComponent::Gcode if edit.value_type != ValueType::Text => {
                return Err(AppError::invalid_argument(format!(
                    "G-code 编辑器只配字符串 —— 这条是 {}",
                    value_type_str(edit.value_type)
                )));
            }
            UiComponent::Gcode if !edit.choices.is_empty() => {
                return Err(AppError::invalid_argument(
                    "G-code 是自由代码，不吃可选项 —— 要分段就换分段控件",
                ));
            }
            UiComponent::Text => {
                if edit.value_type != ValueType::Text {
                    return Err(AppError::invalid_argument(format!(
                        "文本框只配字符串 —— 这条是 {}，数字用步进框",
                        value_type_str(edit.value_type)
                    )));
                }
                if !edit.choices.is_empty() {
                    return Err(AppError::invalid_argument(
                        "文本框不吃可选项 —— 要分段/下拉就换控件，要自由输入就清空可选项",
                    ));
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
                return Err(AppError::invalid_argument(format!(
                    "第 {n} 条可选项没写名字"
                )));
            }
            let vkey = json_key(&c.value);
            if c.value.is_null() || vkey.is_empty() {
                return Err(AppError::invalid_argument(format!(
                    "第 {n} 条可选项（{}）没有值",
                    c.label.trim()
                )));
            }
            if !seen.insert(vkey) {
                return Err(AppError::invalid_argument(format!(
                    "第 {n} 条可选项（{}）的值跟前面重复了",
                    c.label.trim()
                )));
            }
        }
        if edit.value_type == ValueType::Text && !edit.choices.is_empty() {
            let want = json_key(&edit.default_value);
            if !edit.choices.iter().any(|c| json_key(&c.value) == want) {
                return Err(AppError::invalid_argument(format!(
                    "出厂默认 {want:?} 不在这条参数的可选项里 —— 枚举的默认值得是选项之一"
                )));
            }
        }
        if let Some(pk) = &edit.parent_key {
            if pk == key {
                return Err(AppError::invalid_argument(format!(
                    "{key} 不能自己是自己的父项"
                )));
            }
            let parent = self
                .param(pk)
                .ok_or_else(|| AppError::invalid_argument(format!("父项 {pk} 不在字段定义里")))?;
            if parent.parent_key.is_some() {
                return Err(AppError::invalid_argument(format!(
                    "{pk} 自己就是别人的子项 —— 层级只有两级，不能再往下挂"
                )));
            }
            if self
                .params
                .iter()
                .any(|p| p.parent_key.as_deref() == Some(key))
            {
                return Err(AppError::invalid_argument(format!(
                    "{key} 自己已经挂着子项 —— 不能再去当别人的子项"
                )));
            }
        }
        if let Some(sw) = &edit.show_when {
            if sw.key == key {
                return Err(AppError::invalid_argument(format!(
                    "{key} 的前置条件不能指着自己 —— 那永远不成立"
                )));
            }
            if !self.index.contains_key(&sw.key) {
                return Err(AppError::invalid_argument(format!(
                    "前置条件指向的 {} 不在字段定义里",
                    sw.key
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
        let choices_arr = if edit.choices.is_empty() {
            None
        } else {
            let mut arr = toml_edit::ArrayOfTables::new();
            for (i, c) in edit.choices.iter().enumerate() {
                let mut tbl = toml_edit::Table::new();
                tbl["value"] = choice_value_item(&c.value, key, i + 1)?;
                tbl["label"] = toml_edit::value(c.label.trim());
                if c.deprecated {
                    tbl["deprecated"] = toml_edit::value(true);
                }
                arr.push(tbl);
            }
            Some(toml_edit::Item::ArrayOfTables(arr))
        };
        // 适用机型：trim、去空、去重。空 = 不限机型（文件里删键）
        let machine_filter = {
            let mut out: Vec<String> = Vec::new();
            for id in &edit.machine_filter {
                let id = id.trim();
                if !id.is_empty() && !out.iter().any(|x| x == id) {
                    out.push(id.to_owned());
                }
            }
            out
        };

        /* ---------- 文件。到这里全是不会失败的赋值 ---------- */
        let t = self.param_table_mut(at, key)?;
        t["label"] = toml_edit::value(label);
        t["desc"] = toml_edit::value(edit.desc.as_str());
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

        /* ---------- 内存 ---------- */
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
        p.choices = edit.choices;
        p.machine_filter = machine_filter;
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
            .ok_or_else(|| AppError::corrupted(format!("{file} 的 params 不是表数组")))?;
        let n = arr.len();
        let t = arr
            .get_mut(at)
            .ok_or_else(|| AppError::corrupted(format!("{file} 只有 {n} 个 params 块")))?;
        // 反查一次 key：下标错位的话，值会被写到**另一个字段**上，而那是静默的
        let here = t.get("key").and_then(|v| v.as_str()).unwrap_or_default();
        if here != key {
            return Err(AppError::corrupted(format!(
                "{file} 的第 {at} 个 params 块是 {here}，不是 {key}"
            ))
            .with_detail("内存里的顺序与文件对不上了，这一次写入已放弃"));
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
}

/// 校验 owner 的形状 —— 非空、不含空白
fn check_owner(owner: &str) -> Result<(), AppError> {
    if owner.trim().is_empty() || owner.contains(char::is_whitespace) {
        return Err(AppError::invalid_argument(format!(
            "owner 不合法：{owner:?} —— 要么是机型 id（A1），要么是机型:版本（A1:FAST）"
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
    let bad_type =
        || AppError::invalid_argument(format!("{key} 的值 {v} 与它的 valueType（{vt:?}）对不上"));
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
            AppError::invalid_argument(format!("{key} 的前置条件值 {v} 不是有限数字"))
        }),
        _ => Err(AppError::invalid_argument(format!(
            "{key} 的前置条件值只能是字符串 / 数字 / 布尔，见到 {v}"
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
                AppError::invalid_argument(format!("{key} 第 {n} 条可选项的值 {v} 不是有限数字"))
            }),
        },
        _ => Err(AppError::invalid_argument(format!(
            "{key} 第 {n} 条可选项的值只能是字符串 / 数字 / 布尔，见到 {v}"
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

    /// 74 条字段全读到，而且关键字段真的落进模型 ——
    /// 只断言条数的话，一个全是默认值的空壳也能通过
    #[test]
    fn all_seventy_four_params_load_with_their_fields() {
        let Some(r) = real() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        assert_eq!(r.params().len(), 74, "字段条数");
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
            "desc",
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
            "mergeGroup",
            "pinned",
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
        // 真数据的口径（实测）：纯机型键**不加引号**（`P1S = 1.1`），
        // 带冒号的键只能加引号（`'P1S:LITE' = 1.1`）。写回要跟着这个口径
        assert!(text.contains("A1 = 3.0"), "float 字段没写成 3.0");
        assert!(text.contains("A1 = 3\n"), "int 字段被写成了浮点");
        assert!(
            !text.contains("'A1' = ") && !text.contains("\"A1\" = "),
            "纯机型键被加上了引号 —— 真数据里它是裸键"
        );

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

        p.apply_values(&[]).expect("空批次是合法的");

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
            desc: String::new(),
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
            choices: Vec::new(),
            machine_filter: Vec::new(),
        }
    }

    /// 一条可选项（测试里手写三件套太啰嗦）
    fn choice(label: &str, value: Value, deprecated: bool) -> Choice {
        Choice {
            label: label.to_owned(),
            value,
            deprecated,
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
