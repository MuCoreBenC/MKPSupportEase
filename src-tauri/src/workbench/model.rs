//! 开发源数据的形状。
//!
//! 三类东西，权限完全不同，所以类型也分开：
//! - [`Registry`] / [`Fallback`] —— 全局配置。本稿**只读**，没有任何写入路径
//! - [`Machine`] —— 机型基底：基础参数写在这
//! - [`Version`] —— 版本：**只写和机型基底的差别**
//!
//! 两处刻意的选型：
//!
//! 1. 值统一用 [`serde_json::Value`] 而不是各字段一个 Rust 类型。字段定义是数据
//!    （`registry.json`），不是代码；把它编进类型系统就等于每加一个参数都要改 Rust。
//!    代价是类型校验从编译期挪到运行期 —— 那正是 [`crate::workbench::capability`] 要做的事。
//!
//! 2. 参数表用 [`BTreeMap`]。它保证序列化顺序只由键决定，所以"同样的内容"永远得出
//!    同一个 hash。用 `HashMap` 的话顺序随进程随机，状态判定会凭空出现"待生成"。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// 参数表：字段 key → 值
pub type Params = BTreeMap<String, Value>;

/// 字段的值类型。校验与 TOML 序列化都看它
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueType {
    Float,
    Int,
    Bool,
    Text,
    /// 多行 G-code。与 Text 的差别只在界面用哪种输入框、TOML 里用哪种字符串写法
    Gcode,
}

/// 一个字段的定义 —— 配方本上的一个栏位。
///
/// 这不只是 UI 表头：它决定 TOML 怎么写、值怎么校验、区间与步进是多少。
/// 所以本稿不开放编辑（doc §9.8）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldDef {
    /// 全局唯一，形如 `toolhead.z_offset`。机型基底与版本覆盖都用它做键
    pub key: String,
    pub label: String,
    pub desc: String,
    pub value_type: ValueType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
    /// TOML 里的分节名
    pub section: String,
    /// TOML 里的键名。与 `key` 的后半段通常相同，但不强制 —— 交付格式不该被内部键名绑住
    pub toml_key: String,
    /// 界面排序。用 f64 是为了以后往两条之间插一行不必重排整表
    pub order: f64,
}

/// 字段定义全表
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Registry {
    pub schema_version: u32,
    pub fields: Vec<FieldDef>,
}

impl Registry {
    pub fn field(&self, key: &str) -> Option<&FieldDef> {
        self.fields.iter().find(|f| f.key == key)
    }

    /// 全表指纹。进有效配方的 hash —— 否则改了字段定义，产物不会变成"待生成"
    pub fn fingerprint(&self) -> String {
        sha256_of(self)
    }
}

/// 一条回退规则：某个字段缺失时，用哪个字段顶上，并在产物里注明。
///
/// 骨架阶段规则表是空的 —— **空表和"没有这个机制"是两件事**，出货检查会把
/// "引用了不存在的回退规则"报出来，所以这张表必须真实存在。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FallbackRule {
    pub when_missing: String,
    pub use_field: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Fallback {
    pub schema_version: u32,
    pub rules: Vec<FallbackRule>,
}

impl Fallback {
    pub fn fingerprint(&self) -> String {
        sha256_of(self)
    }
}

/// 机型：一类打印机的基础做法
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Machine {
    pub id: String,
    pub display_name: String,
    /// 机型基底。版本没覆盖的字段从这里取
    #[serde(default)]
    pub base: Params,
    /// 这个机型默认配的 BBS 清单
    #[serde(default)]
    pub default_bbs: Vec<String>,
}

/// 版本级 BBS 绑定
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum BbsBinding {
    /// 继承机型默认
    #[default]
    Inherit,
    /// 脱钩：**完全独立的一份清单**，不在机型默认上做增删。
    /// BBS 是外部导入的文件不是配方字段，脱钩就明确由自己负责 —— 增删式继承的规则
    /// 一旦引入，"这个版本到底带哪几瓶"就要靠心算
    Own { ids: Vec<String> },
}

/// 版本：一种口味。**只存和机型基底的差别**
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub id: String,
    pub display_name: String,
    pub machine_id: String,
    /// 覆盖值。键必须是 registry 里有的字段
    #[serde(default)]
    pub overrides: Params,
    #[serde(default)]
    pub bbs: BbsBinding,
}

/// 序列化成**紧凑**JSON 再算 sha256。
///
/// 紧凑不是为了省字节，是为了"同样的内容得同一个 hash"：pretty 格式的缩进会随
/// 嵌套深度变化，而嵌套深度会随字段增减变化。
pub fn sha256_of<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    let mut h = Sha256::new();
    h.update(&bytes);
    format!("{:x}", h.finalize())
}

/// 内置的最小字段集。
///
/// **由工作台自己写出来，不从 mkpse-presets 整份搬运**（doc §9.8）。搬整份的问题不是
/// 版权也不是体积，是那份表里有大量本稿还不支持的语义（per-variant 变体、machineFilter、
/// mergeable…），搬过来就等于默认自己支持它们。
///
/// 这十条覆盖了三类值：数值（带区间与步进）、布尔、多行 G-code —— 后面校验、TOML 序列化、
/// 界面控件三条路都需要这三类各有一个真实样本。
pub fn builtin_registry() -> Registry {
    let f = |key: &str,
             label: &str,
             desc: &str,
             value_type: ValueType,
             unit: Option<&str>,
             min: Option<f64>,
             max: Option<f64>,
             step: Option<f64>,
             section: &str,
             toml_key: &str,
             order: f64| FieldDef {
        key: key.into(),
        label: label.into(),
        desc: desc.into(),
        value_type,
        unit: unit.map(Into::into),
        min,
        max,
        step,
        section: section.into(),
        toml_key: toml_key.into(),
        order,
    };

    Registry {
        schema_version: 1,
        fields: vec![
            f(
                "toolhead.mkp_retract",
                "回抽长度",
                "涂胶笔回抽长度，负值表示回抽",
                ValueType::Float,
                Some("mm"),
                Some(-50.0),
                Some(50.0),
                Some(0.1),
                "toolhead",
                "MKP_retract",
                100.0,
            ),
            f(
                "toolhead.z_offset",
                "Z 偏移",
                "涂胶笔相对喷嘴的高度差",
                ValueType::Float,
                Some("mm"),
                Some(-2.0),
                Some(2.0),
                Some(0.01),
                "toolhead",
                "z_offset",
                110.0,
            ),
            f(
                "toolhead.offset_x",
                "喷嘴偏移 X",
                "驻点的 X 坐标",
                ValueType::Float,
                Some("mm"),
                Some(-300.0),
                Some(300.0),
                Some(0.1),
                "toolhead",
                "offset_x",
                120.0,
            ),
            f(
                "toolhead.offset_y",
                "喷嘴偏移 Y",
                "驻点的 Y 坐标",
                ValueType::Float,
                Some("mm"),
                Some(-300.0),
                Some(300.0),
                Some(0.1),
                "toolhead",
                "offset_y",
                130.0,
            ),
            f(
                "toolhead.custom_mount_gcode",
                "装载胶箱 G-code",
                "挂载涂胶笔的自定义指令",
                ValueType::Gcode,
                None,
                None,
                None,
                None,
                "toolhead",
                "custom_mount_gcode",
                200.0,
            ),
            f(
                "toolhead.custom_unmount_gcode",
                "卸载胶箱 G-code",
                "卸载涂胶笔的自定义指令",
                ValueType::Gcode,
                None,
                None,
                None,
                None,
                "toolhead",
                "custom_unmount_gcode",
                210.0,
            ),
            f(
                "motion.travel_speed",
                "空走速度",
                "不挤出时的移动速度",
                ValueType::Float,
                Some("mm/s"),
                Some(10.0),
                Some(500.0),
                Some(1.0),
                "motion",
                "travel_speed",
                300.0,
            ),
            f(
                "motion.accel",
                "加速度",
                "涂胶动作的加速度上限",
                ValueType::Float,
                Some("mm/s²"),
                Some(100.0),
                Some(20000.0),
                Some(100.0),
                "motion",
                "accel",
                310.0,
            ),
            f(
                "support.line_width",
                "支撑线宽",
                "单条支撑线的宽度",
                ValueType::Float,
                Some("mm"),
                Some(0.1),
                Some(2.0),
                Some(0.01),
                "support",
                "line_width",
                400.0,
            ),
            f(
                "support.enable_brim",
                "启用裙边",
                "首层是否加一圈裙边",
                ValueType::Bool,
                None,
                None,
                None,
                None,
                "support",
                "enable_brim",
                410.0,
            ),
        ],
    }
}

/// 内置的回退登记表。骨架阶段是空表 —— 见 [`FallbackRule`] 上的说明
pub fn builtin_fallback() -> Fallback {
    Fallback {
        schema_version: 1,
        rules: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_registry_keys_are_unique() {
        let reg = builtin_registry();
        let mut keys: Vec<&str> = reg.fields.iter().map(|f| f.key.as_str()).collect();
        keys.sort_unstable();
        let before = keys.len();
        keys.dedup();
        assert_eq!(before, keys.len(), "字段 key 有重复");
    }

    /// 三类值各要有真实样本，否则校验与序列化里有分支永远走不到
    #[test]
    fn builtin_registry_covers_three_value_shapes() {
        let reg = builtin_registry();
        let has = |t: ValueType| reg.fields.iter().any(|f| f.value_type == t);
        assert!(has(ValueType::Float), "缺数值样本");
        assert!(has(ValueType::Bool), "缺布尔样本");
        assert!(has(ValueType::Gcode), "缺 G-code 样本");
    }

    /// 带区间的字段必须 min < max，否则任何值都校验不过
    #[test]
    fn numeric_ranges_are_sane() {
        for f in builtin_registry().fields {
            if let (Some(min), Some(max)) = (f.min, f.max) {
                assert!(min < max, "{} 的区间反了：{min}..{max}", f.key);
            }
            if let Some(step) = f.step {
                assert!(step > 0.0, "{} 的步进不是正数", f.key);
            }
        }
    }

    /// hash 只看内容：键的书写顺序不同，指纹必须相同
    #[test]
    fn params_hash_is_order_independent() {
        let mut a: Params = BTreeMap::new();
        a.insert("b".into(), serde_json::json!(2));
        a.insert("a".into(), serde_json::json!(1));

        let mut b: Params = BTreeMap::new();
        b.insert("a".into(), serde_json::json!(1));
        b.insert("b".into(), serde_json::json!(2));

        assert_eq!(sha256_of(&a), sha256_of(&b));
    }

    /// 内容不同，指纹必须不同 —— 否则上面那条就是空转
    #[test]
    fn params_hash_changes_with_content() {
        let mut a: Params = BTreeMap::new();
        a.insert("a".into(), serde_json::json!(1));
        let mut b: Params = BTreeMap::new();
        b.insert("a".into(), serde_json::json!(2));
        assert_ne!(sha256_of(&a), sha256_of(&b));
    }

    #[test]
    fn bbs_binding_roundtrips() {
        let own = BbsBinding::Own {
            ids: vec!["BBS-01".into()],
        };
        let s = serde_json::to_string(&own).unwrap();
        assert_eq!(serde_json::from_str::<BbsBinding>(&s).unwrap(), own);
        assert_eq!(
            serde_json::from_str::<BbsBinding>(r#"{"mode":"inherit"}"#).unwrap(),
            BbsBinding::Inherit
        );
    }
}
