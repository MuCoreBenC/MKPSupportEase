//! param_registry 快照的解析面（[[params]] + [[tabs]]），双消费方：
//! - `ir::build`：零值默认 / 范围校验 / 弃用检查（`defaults_index` / `lookup_range`）；
//! - Phase 3 参数 UI（Task 20）：标签/描述/控件类型/布局/选项等全量元数据
//!   （`Serialize` 即 IPC 载荷面；显示名与分组在这里是**数据**不是 UI 逻辑）。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChoiceEntry {
    /// Go 侧是 any：字符串/布尔/数字都真实存在（快照实测：true / 0.0 / 50.0 / 90.0）。
    #[serde(default = "choice_value_default")]
    pub value: toml::Value,
    #[serde(default)]
    pub deprecated: bool,
    /// 选项显示名（UI segmented 控件的文案；旧侧有则带上）。
    #[serde(default)]
    pub label: String,
}

/// `[[params.layout]]`：UI 布局锚（sectionId 引用 tabs.sections.id，order 排序）。
/// rename 对 serde 双向生效：读 TOML 快照的 `sectionId`，IPC 载荷亦发 `sectionId`
/// （Task 27 修复：漏 rename 时 section_id 恒为空串——分组全丢且词表断言真空转）。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LayoutEntry {
    #[serde(rename = "sectionId", default)]
    pub section_id: String,
    #[serde(default)]
    pub order: f64,
}

/// `[[params.machineVariants]]`：机型变体值（键如 `A1:FAST`）。
/// 值类型不限：实测快照里既有 G-code 多行字符串也有数值（`'A1:FAST' = -0.7`）。
pub type MachineVariants = std::collections::BTreeMap<String, toml::Value>;

/// `[params.machineMinVariants]` / `[params.machineMaxVariants]`：**按机型/变体的区间覆盖**。
///
/// 键与 [`MachineVariants`] 同形（`机型:变体`，如 `P1S:LITE`），但值只可能是数字，
/// 所以单独起一个别名而不是复用 `toml::Value` —— 类型说清了，调用方就不用到处 `as_f64`。
///
/// **这两张表在这一轮之前是被 serde 静默丢弃的**（`ParamEntry` 没有对应字段，
/// 又没开 `deny_unknown_fields`）。也就是说「P1S 的 `offset.y` 区间与 A1 不同」
/// 这件事在程序里根本不存在。实测只有 5 组（挂在 `toolhead.offset.y` / `toolhead.offset.z` /
/// `wiping.wiper_x` / `wiping.wiper_y` / `wiping.user_dry_time` 下）。
pub type MachineBounds = std::collections::BTreeMap<String, f64>;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ParamEntry {
    #[serde(rename = "key")]
    pub param_key: String,
    #[serde(rename = "configKey", default)]
    pub config_key: String,
    #[serde(rename = "defaultValue", default = "default_value_zero")]
    pub default_value: toml::Value,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub deprecated: bool,
    #[serde(default)]
    pub choices: Vec<ChoiceEntry>,

    // ---- Task 20 起：参数 UI 的元数据面（全部带默认，快照缺字段不炸解析）----
    /// 中文显示名。
    #[serde(default)]
    pub label: String,
    /// 说明文案（UI tooltip）。
    #[serde(default)]
    pub desc: String,
    /// 单位（mm / mm/s …）。
    #[serde(default)]
    pub unit: String,
    /// TOML 键名（写入面；空 = 该参数不可经 TOML 编辑）。
    #[serde(rename = "tomlKey", default)]
    pub toml_key: String,
    /// 值类型：float / string / bool（快照实测词表）。
    #[serde(rename = "valueType", default)]
    pub value_type: String,
    /// 控件类型：number / gcode / switch / segmented（快照实测词表）。
    #[serde(rename = "uiComponent", default)]
    pub ui_component: String,
    /// number 控件步进。
    #[serde(default)]
    pub step: Option<f64>,
    /// TOML 表名（toolhead / wiping）——写入时的落点。
    #[serde(default)]
    pub section: String,
    /// universal / machine_specific。
    #[serde(default)]
    pub scope: String,
    /// per_variant / shared。
    #[serde(rename = "variantMode", default)]
    pub variant_mode: String,
    /// 机型过滤（逗号分隔，如 `A1,A1_MINI,A2L,P1S,P2S,X1C`）。
    #[serde(rename = "machineFilter", default)]
    pub machine_filter: String,
    /// UI 置顶标记。
    #[serde(default)]
    pub pinned: bool,
    /// 写回时的行内注释文案（快照信息源；实际写回由 toml_edit decor 保留原注释）。
    #[serde(rename = "tomlComment", default)]
    pub toml_comment: String,
    #[serde(default)]
    pub selectable: bool,
    #[serde(default)]
    pub mergeable: bool,
    #[serde(default)]
    pub layout: LayoutEntry,
    #[serde(rename = "machineVariants", default)]
    pub machine_variants: MachineVariants,
    /// 按机型/变体的区间覆盖（见 [`MachineBounds`]）。
    #[serde(rename = "machineMinVariants", default)]
    pub machine_min_variants: MachineBounds,
    #[serde(rename = "machineMaxVariants", default)]
    pub machine_max_variants: MachineBounds,
}

fn default_value_zero() -> toml::Value {
    toml::Value::Integer(0)
}

fn choice_value_default() -> toml::Value {
    toml::Value::String(String::new())
}

/// `[[tabs.sections]]`。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SectionEntry {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub order: f64,
}

/// `[[tabs]]`。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct TabEntry {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub order: f64,
    #[serde(default)]
    pub sections: Vec<SectionEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Registry {
    pub params: Vec<ParamEntry>,
    #[serde(default)]
    pub tabs: Vec<TabEntry>,
}

/// 从嵌入快照解析。反空转：0 条即 panic（快照损坏要响，不能静默空表）。
pub fn load_param_registry() -> Registry {
    let reg: Registry = toml::from_str(crate::PARAM_REGISTRY_TOML)
        .expect("嵌入的 param_registry 快照不是合法 TOML");
    assert!(
        !reg.params.is_empty(),
        "param_registry 快照解析出 0 条 params —— 反空转失败"
    );
    assert!(
        !reg.tabs.is_empty(),
        "param_registry 快照解析出 0 条 tabs —— UI 布局面反空转失败"
    );
    reg
}

/// 区间从哪来。**必须可见**：否则「这台机型专有的区间」与「其实回落到全局了」
/// 在界面上和判据里长得一模一样。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RangeSource {
    /// 命中 `[params.machineMinVariants]` / `machineMaxVariants` 的 `机型:变体`
    MachineVariant,
    /// 回落到全局 `min` / `max`
    Global,
}

/// 一个参数在**某台机型 + 某个变体**下的有效区间。
///
/// 两半各自 `Option`：覆盖表只给了一半时，另一半回落全局（实测那 5 组都是成对给的，
/// 但类型上不该假设它）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct EffectiveRange {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub source: RangeSource,
}

impl ParamEntry {
    /// 这一条参数在**某台机型 + 某个变体**下的有效区间。
    ///
    /// 解析规则只写在这里一处：[`Registry::lookup_range_for`] 只是「先按 configKey 找到条目，
    /// 再问它」的薄壳。编辑器按注册表的 `key`（`段.键`）找条目（`offset` 的 x/y/z
    /// **共用一个 tomlKey**，只能按 `key` 找），拿到条目后走这个方法 ——
    /// 于是「校验用的区间」与「界面显示的区间」是同一段代码算出来的，不会各算一遍。
    pub fn effective_range(&self, machine: &str, variant: Option<&str>) -> Option<EffectiveRange> {
        let key = variant.map(|v| format!("{}:{}", machine, v.to_ascii_uppercase()));
        let overridden = key.as_deref().map(|k| {
            (
                self.machine_min_variants.get(k).copied(),
                self.machine_max_variants.get(k).copied(),
            )
        });

        match overridden {
            // 命中任意一半就算「按机型」——另一半回落全局，来源仍标 MachineVariant，
            // 因为「这条区间被机型影响过」是用户该知道的事实。
            Some((min, max)) if min.is_some() || max.is_some() => Some(EffectiveRange {
                min: min.or(self.min),
                max: max.or(self.max),
                source: RangeSource::MachineVariant,
            }),
            _ if self.min.is_some() || self.max.is_some() => Some(EffectiveRange {
                min: self.min,
                max: self.max,
                source: RangeSource::Global,
            }),
            _ => None,
        }
    }
}

impl Registry {
    /// configKey → defaultValue（仅非空 configKey 入索引；对照 registryDefaults）。
    pub fn defaults_index(&self) -> std::collections::HashMap<String, toml::Value> {
        self.params
            .iter()
            .filter(|p| !p.config_key.is_empty())
            .map(|p| (p.config_key.clone(), p.default_value.clone()))
            .collect()
    }

    /// 按 configKey 查范围（Min/Max 齐全才返回）。
    ///
    /// **这个函数一个字不动**：`build.rs` 的 IR 域兜底逻辑（`validate_ranges`）在用它，
    /// 那一层的清理不在本轮范围内。新代码请用 [`Registry::lookup_range_for`]。
    pub fn lookup_range(&self, config_key: &str) -> Option<(f64, f64)> {
        let p = self.params.iter().find(|p| p.config_key == config_key)?;
        Some((p.min?, p.max?))
    }

    /// **有效区间**：先按 `configKey` 找到条目，再问它在这台机型/变体下的区间
    /// （规则写在 [`ParamEntry::effective_range`]，这里不重算一遍）。
    ///
    /// - `machine` 要是**规范名**（`A1_MINI` / `P1S` / `X1C`…）—— 归一是调用方的事
    ///   （`load_ir` 在校验前先算一次）；
    /// - `variant` 是文件头 `# variant:` 的原值，大小写不敏感：注册表里的键是大写
    ///   （`P1S:LITE`），而预设文件头写的是小写（`fastv3.3`），统一大写后再查；
    /// - 两边都查不到 ⇒ `None`（那个参数就是没有量程，编辑器会说「没有量程作证」）。
    pub fn lookup_range_for(
        &self,
        config_key: &str,
        machine: &str,
        variant: Option<&str>,
    ) -> Option<EffectiveRange> {
        self.params
            .iter()
            .find(|p| p.config_key == config_key)?
            .effective_range(machine, variant)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 快照元数据面的机械判据：词表不漂移、编辑面自洽。
    /// UI 生成器信赖这些词表——出现未知控件类型时这里先红。
    #[test]
    fn ui_metadata_vocabulary_is_closed() {
        let reg = load_param_registry();
        assert!(
            reg.params.len() >= 70,
            "快照应有 ~74 条，实测 {}",
            reg.params.len()
        );
        assert!(
            reg.tabs.len() >= 8,
            "快照应有 8 个 tabs，实测 {}",
            reg.tabs.len()
        );

        let mut sections_total = 0usize;
        for tab in &reg.tabs {
            assert!(!tab.id.is_empty(), "tab 缺 id：{tab:?}");
            sections_total += tab.sections.len();
        }
        assert!(sections_total >= 8, "sections 总数过少：{sections_total}");

        // 快照实测词表（grep 盘点）：UI 生成器按它分派控件，出现未知类型先红这里
        const UI_COMPONENTS: [&str; 6] = ["number", "gcode", "switch", "segmented", "select", ""];
        const VALUE_TYPES: [&str; 5] = ["float", "string", "bool", "int", ""];
        let section_ids: Vec<String> = reg
            .tabs
            .iter()
            .flat_map(|t| t.sections.iter().map(|s| s.id.clone()))
            .collect();

        let mut with_toml_key = 0usize;
        let mut with_layout = 0usize;
        for p in &reg.params {
            assert!(
                UI_COMPONENTS.contains(&p.ui_component.as_str()),
                "{} 的 uiComponent 越出词表：{:?}",
                p.param_key,
                p.ui_component
            );
            assert!(
                VALUE_TYPES.contains(&p.value_type.as_str()),
                "{} 的 valueType 越出词表：{:?}",
                p.param_key,
                p.value_type
            );
            if p.toml_key.is_empty() {
                continue;
            }
            with_toml_key += 1;
            assert!(
                p.section == "toolhead" || p.section == "wiping",
                "{} 的 section 必须是 toolhead/wiping 之一，实测 {:?}",
                p.param_key,
                p.section
            );
            if !p.layout.section_id.is_empty() {
                // Task 27 修复后的强断言：快照实测 74/74 参数带 layout.sectionId
                // （此前 section_id 恒空串，这条断言真空转——bug 由浏览器 harness 曝出）
                with_layout += 1;
                assert!(
                    section_ids.contains(&p.layout.section_id),
                    "{} 的 layout.sectionId={} 不在任何 tab 下",
                    p.param_key,
                    p.layout.section_id
                );
            }
        }
        assert!(
            with_toml_key >= 60,
            "可 TOML 编辑的参数过少：{with_toml_key}"
        );
        assert!(
            with_layout >= 60,
            "带 layout.sectionId 的参数过少：{with_layout}（sectionId 解析是否又断了？）"
        );
    }

    /// 既有消费面（ir::build）不回归：默认索引与范围查询仍工作。
    #[test]
    fn defaults_and_ranges_still_resolve() {
        let reg = load_param_registry();
        let idx = reg.defaults_index();
        assert!(!idx.is_empty());
        // 快照实测样本：MKPRetract 默认 0.0（toolhead.MKP_retract）
        assert!(
            idx.contains_key("MKPRetract"),
            "configKey=MKPRetract 应在索引"
        );
        assert!(
            reg.lookup_range("MKPRetract").is_some(),
            "MKPRetract 应有范围"
        );
    }
}
