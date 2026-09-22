//! 继承解析 —— 从「机型基底 + 版本覆盖」算出**有效配方**，并给它一个指纹。
//!
//! 三条规则，顺序就是优先级：
//! 1. 版本覆盖了这个字段 → 用版本的值，来源标 `Override`
//! 2. 否则机型基底有 → 用基底的值，来源标 `Base`
//! 3. 两边都没有 → 这个字段**在这个机型上不适用**，不进有效配方
//!
//! 第 3 条里"机型基底决定字段适用性"是刻意的：适用性不该再单独维护一张表，
//! 否则"基底里有没有"和"表里写没写"两处会漂移。
//!
//! 由此还长出一个必须被看见的情况：**版本覆盖了一个基底没有的字段**。
//! 它不是错误（版本刚从别的机型移过来时就是这样），但也不能进 TOML ——
//! 所以它进 [`Effective::orphans`]，界面标灰"新机型不适用"，值原样留在 JSON 里不丢。
//!
//! 指纹（[`Effective::hash`]）是状态判定唯一的判据。它的输入**必须包含全局配置的指纹**：
//! 否则改了字段定义或回退规则，产物明明已经过期，状态却还显示"已生成"。

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use crate::workbench::model::{sha256_of, Fallback, Machine, Registry, Version};

/// 值从哪来。界面靠它区分"继承值"与"本版覆盖"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Origin {
    Base,
    Override,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueOrigin {
    pub value: Value,
    pub origin: Origin,
    /// 基底里的值。`origin == Override` 时界面要显示"清除覆盖后会变成什么"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_value: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Effective {
    /// 字段 key → 值与来源。BTreeMap 保证顺序只由键决定
    pub values: BTreeMap<String, ValueOrigin>,
    /// 版本覆盖了、但当前机型基底里没有的字段。保留不丢，但不写进产物
    pub orphans: Vec<String>,
    /// 覆盖里出现了字段定义里根本没有的 key。这是真问题，出货检查会报
    pub unknown_keys: Vec<String>,
    /// 状态判定唯一的判据
    pub hash: String,
}

impl Effective {
    /// 有效配方为空 = 这个版本**还没写配方**。
    ///
    /// doc §8 把这种状态叫"未配置"，举的例子是 A2L：配方本上有名字，下面一行没写。
    /// 判据刻意落在"有效配方为空"而不是"版本覆盖为空" —— 后者会把
    /// "完全继承机型基底"的正常版本（比如 A1 标准版）也误判成未配置。
    pub fn is_unconfigured(&self) -> bool {
        self.values.is_empty()
    }

    /// 只要值，不要来源。生成 TOML 与校验都用这一份
    pub fn plain(&self) -> BTreeMap<String, Value> {
        self.values
            .iter()
            .map(|(k, v)| (k.clone(), v.value.clone()))
            .collect()
    }
}

pub fn resolve(reg: &Registry, fb: &Fallback, m: &Machine, v: &Version) -> Effective {
    let mut values = BTreeMap::new();
    let mut orphans = Vec::new();

    for f in &reg.fields {
        let base = m.base.get(&f.key);
        match (v.overrides.get(&f.key), base) {
            (Some(val), Some(b)) => {
                values.insert(
                    f.key.clone(),
                    ValueOrigin {
                        value: val.clone(),
                        origin: Origin::Override,
                        base_value: Some(b.clone()),
                    },
                );
            }
            // 覆盖了，但这个机型的基底里没有这个字段
            (Some(_), None) => orphans.push(f.key.clone()),
            (None, Some(b)) => {
                values.insert(
                    f.key.clone(),
                    ValueOrigin {
                        value: b.clone(),
                        origin: Origin::Base,
                        base_value: None,
                    },
                );
            }
            (None, None) => {}
        }
    }

    // 覆盖里有字段定义里没有的 key —— 与 orphan 是两回事：orphan 是"字段存在但这个机型没有"，
    // 这个是"根本没有这个字段"。后者只可能来自手改文件或字段被删，必须报出来
    let mut unknown_keys: Vec<String> = v
        .overrides
        .keys()
        .filter(|k| reg.field(k).is_none())
        .cloned()
        .collect();
    unknown_keys.sort_unstable();
    orphans.sort_unstable();

    /* 指纹的输入。四样都要进：
       - registry / fallback 指纹：改了全局配置，所有受影响的版本要一起转"待生成"
       - values：配方本身
       - orphans：它今天不写进 TOML，但"这个版本有几个不适用的覆盖"是产物之外的事实，
         把它算进去的理由是——版本移回原机型时 orphan 会重新变成有效值，
         那一刻产物必须被判为过期 */
    let payload = serde_json::json!({
        "registry": reg.fingerprint(),
        "fallback": fb.fingerprint(),
        "values": values.iter().map(|(k, v)| (k.clone(), v.value.clone())).collect::<BTreeMap<_, _>>(),
        "orphans": &orphans,
    });

    Effective {
        values,
        orphans,
        unknown_keys,
        hash: sha256_of(&payload),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::model::{builtin_fallback, builtin_registry, BbsBinding, Params};

    fn machine(base: &[(&str, Value)]) -> Machine {
        Machine {
            id: "A1".into(),
            display_name: "A1".into(),
            base: base
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
            default_bbs: vec![],
        }
    }

    fn version(overrides: &[(&str, Value)]) -> Version {
        Version {
            id: "std".into(),
            display_name: "标准版".into(),
            machine_id: "A1".into(),
            overrides: overrides
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
            bbs: BbsBinding::Inherit,
        }
    }

    #[test]
    fn override_wins_and_records_base_value() {
        let reg = builtin_registry();
        let fb = builtin_fallback();
        let m = machine(&[("toolhead.z_offset", serde_json::json!(0.1))]);
        let v = version(&[("toolhead.z_offset", serde_json::json!(0.15))]);

        let eff = resolve(&reg, &fb, &m, &v);
        let got = &eff.values["toolhead.z_offset"];
        assert_eq!(got.value, serde_json::json!(0.15));
        assert_eq!(got.origin, Origin::Override);
        assert_eq!(got.base_value, Some(serde_json::json!(0.1)));
    }

    #[test]
    fn unset_field_inherits_from_base() {
        let reg = builtin_registry();
        let fb = builtin_fallback();
        let m = machine(&[("toolhead.offset_x", serde_json::json!(256.0))]);
        let v = version(&[]);

        let eff = resolve(&reg, &fb, &m, &v);
        let got = &eff.values["toolhead.offset_x"];
        assert_eq!(got.value, serde_json::json!(256.0));
        assert_eq!(got.origin, Origin::Base);
        assert!(got.base_value.is_none(), "继承值不需要再记一份基底值");
    }

    /// 基底没有这个字段时，覆盖值不进有效配方，但要被点名
    #[test]
    fn override_without_base_becomes_orphan() {
        let reg = builtin_registry();
        let fb = builtin_fallback();
        let m = machine(&[("toolhead.z_offset", serde_json::json!(0.1))]);
        let v = version(&[("motion.accel", serde_json::json!(12000))]);

        let eff = resolve(&reg, &fb, &m, &v);
        assert!(!eff.values.contains_key("motion.accel"));
        assert_eq!(eff.orphans, vec!["motion.accel"]);
    }

    #[test]
    fn override_of_unknown_field_is_reported() {
        let reg = builtin_registry();
        let fb = builtin_fallback();
        let m = machine(&[]);
        let v = version(&[("toolhead.mustard", serde_json::json!(1))]);

        let eff = resolve(&reg, &fb, &m, &v);
        assert_eq!(eff.unknown_keys, vec!["toolhead.mustard"]);
    }

    /// "未配置"看的是有效配方空不空，不是覆盖空不空
    #[test]
    fn unconfigured_is_about_effective_not_overrides() {
        let reg = builtin_registry();
        let fb = builtin_fallback();

        // A2L 那种：基底空、覆盖也空 → 未配置
        let empty = resolve(&reg, &fb, &machine(&[]), &version(&[]));
        assert!(empty.is_unconfigured());

        // A1 标准版那种：覆盖空但基底有值 → **不是**未配置
        let inherited = resolve(
            &reg,
            &fb,
            &machine(&[("toolhead.z_offset", serde_json::json!(0.1))]),
            &version(&[]),
        );
        assert!(!inherited.is_unconfigured());
    }

    /// 改字段定义必须让 hash 变 —— 否则产物过期了状态还显示"已生成"
    #[test]
    fn hash_reacts_to_registry_change() {
        let fb = builtin_fallback();
        let m = machine(&[("toolhead.z_offset", serde_json::json!(0.1))]);
        let v = version(&[]);

        let before = resolve(&builtin_registry(), &fb, &m, &v).hash;

        let mut reg2 = builtin_registry();
        reg2.fields[1].step = Some(0.05); // 只改一个步进
        let after = resolve(&reg2, &fb, &m, &v).hash;

        assert_ne!(before, after);
    }

    /// 改回退规则同理
    #[test]
    fn hash_reacts_to_fallback_change() {
        let reg = builtin_registry();
        let m = machine(&[("toolhead.z_offset", serde_json::json!(0.1))]);
        let v = version(&[]);

        let before = resolve(&reg, &builtin_fallback(), &m, &v).hash;

        let mut fb2 = builtin_fallback();
        fb2.rules.push(crate::workbench::model::FallbackRule {
            when_missing: "toolhead.z_offset".into(),
            use_field: "toolhead.offset_x".into(),
            note: "测试".into(),
        });
        let after = resolve(&reg, &fb2, &m, &v).hash;

        assert_ne!(before, after);
    }

    /// 同样的内容、不同的书写顺序 → 同一个 hash。
    /// 这条不成立的话，每次保存都可能凭空出现"待生成"
    #[test]
    fn hash_is_stable_across_insertion_order() {
        let reg = builtin_registry();
        let fb = builtin_fallback();

        let mut a: Params = BTreeMap::new();
        a.insert("toolhead.z_offset".into(), serde_json::json!(0.1));
        a.insert("toolhead.offset_x".into(), serde_json::json!(256.0));

        let mut b: Params = BTreeMap::new();
        b.insert("toolhead.offset_x".into(), serde_json::json!(256.0));
        b.insert("toolhead.z_offset".into(), serde_json::json!(0.1));

        let ma = Machine {
            base: a,
            ..machine(&[])
        };
        let mb = Machine {
            base: b,
            ..machine(&[])
        };
        let v = version(&[]);

        assert_eq!(
            resolve(&reg, &fb, &ma, &v).hash,
            resolve(&reg, &fb, &mb, &v).hash
        );
    }

    /// 值变了 hash 必须变 —— 否则上面那条是空转
    #[test]
    fn hash_changes_when_a_value_changes() {
        let reg = builtin_registry();
        let fb = builtin_fallback();
        let m = machine(&[("toolhead.z_offset", serde_json::json!(0.1))]);

        let a = resolve(&reg, &fb, &m, &version(&[])).hash;
        let b = resolve(
            &reg,
            &fb,
            &m,
            &version(&[("toolhead.z_offset", serde_json::json!(0.15))]),
        )
        .hash;
        assert_ne!(a, b);
    }
}
