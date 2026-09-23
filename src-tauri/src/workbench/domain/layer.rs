//! 三层取值与来源层（doc §3.1、§3.6）。
//!
//! ```text
//! 出厂默认（param_registry.defaultValue，全局单值）
//!   ← 机型基底（上游机型差异 + workbench/machines/{机型}.json）
//!     ← 版本覆盖（上游版本差异 + workbench/machines/{机型}/versions/{版本}.json）
//! ```
//!
//! # 每一层有两半：上游给的，和我们写的（doc §3.6）
//!
//! `machineVariants` 的归并结果（[`super::variants`]）**不落盘**，它是派生的。
//! 所以机型层与版本层各有两张表：
//!
//! | | 上游给的那一半 | 我们写的那一半 |
//! |---|---|---|
//! | 机型层 | `digest.base`，派生 | `machines/{机型}.json` |
//! | 版本层 | `digest.versions[版本]`，派生 | `versions/{版本}.json` |
//!
//! 界面上**仍然只有三档来源**（出厂 / 机型 / 版本）——同一层的两半算同一档。
//! 由此定下三件事：
//!
//! 1. [`Layers::has_own`] 与「挂回继承」**只看我们写的那两张表**。挂回继承 = 删掉我们的
//!    那个键，露出上游的机型差异 —— 把 A1 的 X 轴偏移挂回继承应该回到 `-1`，
//!    不该回到全局默认 `0`。
//! 2. 我们的文件**只会多一个键，不会删一个键**，所以不需要删除标记。
//! 3. 代价：**没法把某一项压回出厂默认**。想让 A1 的 `offset.x` 变成 `0`，只能显式写 `0`。
//!    这个需求很少，而为它引入删除标记会让每一层都多一种状态。
//!
//! 换来的是：上游改了 `machineVariants`，**我们没动过的项自动跟着变**。
//!
//! # 稀疏：只记"这一层自己写过的"
//!
//! 每层都是稀疏表，**键在不在就是脱钩的载体**。所以：
//!
//! - 没有单独的「脱钩」标记。脱钩天然是按项算的，不是按版本算的。
//! - 删键 = 挂回继承（doc §4.1 那条 `value: None`）。
//!
//! 反过来说，**"这一层没有这个键"和"这一层把它设成了空值"是两件事**，不能混：
//! 上游有两个字段（`toolhead.custom_mount_gcode` / `custom_unmount_gcode`）的出厂默认
//! 就是**空串**。把"空串"当成"没设过"，这两个字段会永远显示成继承，
//! 用户改成空串（= 我不要这段 G-code）的动作就保存不下来。
//!
//! # 不适用 vs 看得见改不动
//!
//! 这一层只管前者：`machineFilter` 排除掉的字段在这台机型上**根本没有这一项**，
//! 不进有效配方，单独进 [`Layers::not_applicable`]。
//!
//! 后者（被上级 `showWhen` 关着，看得见但改不动）是 [`super::visibility`] 的事 ——
//! 那个字段是有值的、会进产物的，只是现在不该让人编辑。两者在界面上说的话也不同。
//!
//! # 已经失效的键要说出来
//!
//! 上游删掉一个参数、或者把某台机型从 `machineFilter` 里摘掉之后，我们仓库里那个值
//! **还在文件里，但再也不会进任何产物**。这是典型的静默失效：界面上什么都看不到，
//! 而"我明明改过"这件事会一直是错的。[`Layers::orphan_keys`] 把它们列出来。

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::Serialize;
use serde_json::Value;

use crate::workbench::presets::ParamRegistry as Registry;

/// 一层的稀疏覆盖表。`BTreeMap` 不只是为了好看 ——
/// 指纹要按稳定顺序算，`HashMap` 每次进程的遍历顺序都不同
pub type Overrides = BTreeMap<String, Value>;

/// 空表的共享借用。给「这一层没有上游差异」用，免得每个调用点自己造一个再借
pub fn no_overrides() -> &'static Overrides {
    static EMPTY: OnceLock<Overrides> = OnceLock::new();
    EMPTY.get_or_init(Overrides::new)
}

/// 可写的两层。出厂层不在这里：它是上游的，工作台改不了
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Level {
    Machine,
    Version,
}

/// 有效值来自哪一层。**三档，比 `Level` 多一个出厂**
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Origin {
    Factory,
    Machine,
    Version,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ValueOrigin<'a> {
    pub value: &'a Value,
    pub origin: Origin,
}

/// 一个 `(机型, 版本)` 的三层视图。
///
/// 借用而不持有：五张表分别属于上游、归并结果、机型文件、版本文件，
/// 复制一份进来就等于又造了一个"当前状态"的副本（doc §1 第四条铁律不许）
pub struct Layers<'a> {
    registry: &'a Registry,
    machine_id: &'a str,
    /// 机型层「上游给的那一半」：`variants::digest` 的 `base`
    upstream_base: &'a Overrides,
    /// 机型层「我们写的那一半」
    base: &'a Overrides,
    /// 版本层「上游给的那一半」：`variants::digest` 的 `versions[版本]`
    upstream_over: &'a Overrides,
    /// 版本层「我们写的那一半」
    over: &'a Overrides,
}

impl<'a> Layers<'a> {
    pub fn new(
        registry: &'a Registry,
        machine_id: &'a str,
        upstream_base: &'a Overrides,
        base: &'a Overrides,
        upstream_over: &'a Overrides,
        over: &'a Overrides,
    ) -> Self {
        Self {
            registry,
            machine_id,
            upstream_base,
            base,
            upstream_over,
            over,
        }
    }

    /// 上游没给这台机型任何差异时的简写（A2L 就是这种），也给单测用
    pub fn without_upstream(
        registry: &'a Registry,
        machine_id: &'a str,
        base: &'a Overrides,
        over: &'a Overrides,
    ) -> Self {
        Self::new(
            registry,
            machine_id,
            no_overrides(),
            base,
            no_overrides(),
            over,
        )
    }

    /// 这台机型看得见的字段，按 `layout.order` 升序。
    /// 等价于 `registry.visible_keys(machine_id)` —— 留个门面，调用方不用知道过滤规则在哪
    pub fn keys(&self) -> Vec<&'a str> {
        self.registry.visible_keys(self.machine_id)
    }

    /// 这个字段在这台机型上存不存在。未知的 key 一律 false
    pub fn applies(&self, key: &str) -> bool {
        self.registry
            .param(key)
            .is_some_and(|p| !p.deprecated && p.applies_to(self.machine_id))
    }

    /// 这台机型上**不适用**的字段（被 `machineFilter` 排除的）。
    /// 界面上写「不适用」，与「看得见改不动」是两回事
    pub fn not_applicable(&self) -> Vec<&'a str> {
        self.registry
            .params()
            .iter()
            .filter(|p| !p.deprecated && !p.applies_to(self.machine_id))
            .map(|p| p.key.as_str())
            .collect()
    }

    /// 有效值 + 来源层。三层都没有（或这个字段在这台机型上不适用）返回 `None`。
    ///
    /// **`None` 与"值是空串"严格分开**：前者是"这一项不存在"，后者是一个真实的值。
    ///
    /// 查找顺序是五级，但报出来的来源只有三档（doc §3.6）
    pub fn effective(&self, key: &str) -> Option<ValueOrigin<'a>> {
        if !self.applies(key) {
            return None;
        }
        for (table, origin) in [
            (self.over, Origin::Version),
            (self.upstream_over, Origin::Version),
            (self.base, Origin::Machine),
            (self.upstream_base, Origin::Machine),
        ] {
            if let Some(v) = table.get(key) {
                return Some(ValueOrigin { value: v, origin });
            }
        }
        self.registry.param(key).map(|p| ValueOrigin {
            value: &p.default_value,
            origin: Origin::Factory,
        })
    }

    /// 这一层**我们自己写过**这个键吗 = 有没有脱钩。
    ///
    /// **只看我们写的那一张表**，不看上游的归并结果（doc §3.6 第一条）：
    /// 「挂回继承」删掉的就是这一项，删完露出的是上游那一半。
    ///
    /// 它和 `effective(...).origin` 不是一回事：版本覆盖存在时
    /// `has_own(Machine, key)` 仍然可能为真（基底写过，只是被盖住了）
    pub fn has_own(&self, level: Level, key: &str) -> bool {
        match level {
            Level::Machine => self.base.contains_key(key),
            Level::Version => self.over.contains_key(key),
        }
    }

    /// 我们在这一层写了几项。界面上「N 项自有」那个徽章
    pub fn own_count(&self, level: Level) -> usize {
        let table = match level {
            Level::Machine => self.base,
            Level::Version => self.over,
        };
        table.keys().filter(|k| self.applies(k)).count()
    }

    /// 完整的有效配方。**这是进产物、进指纹的那一份**
    pub fn effective_recipe(&self) -> BTreeMap<&'a str, &'a Value> {
        self.keys()
            .into_iter()
            .filter_map(|k| self.effective(k).map(|r| (k, r.value)))
            .collect()
    }

    /// **我们**写着、但再也不会进任何产物的键。
    ///
    /// 三种来法：上游删了这个参数 / 上游把它标了 `deprecated` /
    /// 上游把这台机型从 `machineFilter` 里摘掉了。
    /// 三种的表现都一样 —— 文件里有值，产物里没有，界面上没人提。
    ///
    /// 只查我们写的那两张表：上游归并结果里的键是派生的，上游一改它自己就没了
    pub fn orphan_keys(&self) -> Vec<&str> {
        let mut out: Vec<&str> = self
            .base
            .keys()
            .chain(self.over.keys())
            .filter(|k| !self.applies(k))
            .map(String::as_str)
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// 有效配方的指纹。产物过期判定（doc §5）用它，**不用文件时间**。
    ///
    /// 输入里含 `registry` 的指纹：改了字段定义（比如给某个参数换了 `tomlKey`）
    /// 产出的 TOML 就不一样了，而三层的值一个都没动 —— 不带上它，产物会一直显示「已生成」
    pub fn fingerprint(&self) -> String {
        use sha2::{Digest, Sha256};
        let payload = serde_json::json!({
            "registry": self.registry.fingerprint(),
            "machine": self.machine_id,
            "recipe": self.effective_recipe(),
        });
        let mut h = Sha256::new();
        h.update(serde_json::to_vec(&payload).unwrap_or_default());
        format!("{:x}", h.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::paths;

    /// 三个字段：一个普通的、一个出厂默认是**空串**的、一个只给 P1S 的
    fn registry() -> (tempfile::TempDir, Registry) {
        let d = tempfile::tempdir().unwrap();
        let param = |key: &str, order: f64, default: serde_json::Value, filter: &str| {
            serde_json::json!({
                "key": key, "configKey": "X", "tomlKey": key, "jsonKey": key,
                "label": key, "desc": "", "tomlComment": "",
                "valueType": "float", "uiComponent": "number", "defaultValue": default,
                "scope": "universal", "section": "toolhead",
                "layout": { "order": order, "sectionId": "s1" },
                "machineFilter": filter
            })
        };
        let params = serde_json::json!({
            "params": [
                param("toolhead.offset.x", 1.0, serde_json::json!(0), ""),
                param("toolhead.custom_mount_gcode", 2.0, serde_json::json!(""), ""),
                param("toolhead.only_p1s", 3.0, serde_json::json!(9), "P1S"),
                param("toolhead.gone", 4.0, serde_json::json!(1), "")
            ],
            "tabs": [{ "id": "t1", "label": "页签一", "order": 10,
                       "sections": [{ "id": "s1", "label": "分组一", "order": 0 }] }],
            "updated": "2026-01-01 00:00:00"
        });
        let layout = serde_json::json!({
            "tabs": [{ "id": "t1", "sections": [{ "id": "s1", "items": [
                { "id": "i0", "paramKey": "toolhead.offset.x" },
                { "id": "i1", "paramKey": "toolhead.custom_mount_gcode" },
                { "id": "i2", "paramKey": "toolhead.only_p1s" },
                { "id": "i3", "paramKey": "toolhead.gone" }
            ] }] }]
        });
        let r = crate::workbench::presets::registry::load_from_json_fixture(
            d.path(),
            &params,
            &layout,
        )
        .unwrap();
        (d, r)
    }

    fn overrides(pairs: &[(&str, serde_json::Value)]) -> Overrides {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), v.clone()))
            .collect()
    }

    /// 覆盖 > 基底 > 出厂，且来源层要报对
    #[test]
    fn version_beats_machine_beats_factory() {
        let (_d, reg) = registry();
        let base = overrides(&[("toolhead.offset.x", serde_json::json!(1))]);
        let over = overrides(&[("toolhead.offset.x", serde_json::json!(2))]);

        let l =         Layers::without_upstream(&reg, "A1", &base, &over);
        let r = l.effective("toolhead.offset.x").unwrap();
        assert_eq!(*r.value, serde_json::json!(2));
        assert_eq!(r.origin, Origin::Version);

        let empty = Overrides::new();
        let l =         Layers::without_upstream(&reg, "A1", &base, &empty);
        let r = l.effective("toolhead.offset.x").unwrap();
        assert_eq!(*r.value, serde_json::json!(1));
        assert_eq!(r.origin, Origin::Machine);

        let l =         Layers::without_upstream(&reg, "A1", &empty, &empty);
        let r = l.effective("toolhead.offset.x").unwrap();
        assert_eq!(*r.value, serde_json::json!(0));
        assert_eq!(r.origin, Origin::Factory);
    }

    /// **空串是一个值，不是"没设过"**。
    /// 混了的后果：那两个 G-code 字段永远显示继承，用户清空的动作保存不下来
    #[test]
    fn an_empty_string_is_a_value_not_an_absence() {
        let (_d, reg) = registry();
        let empty = Overrides::new();

        // 出厂默认本来就是空串 → 有值，来源出厂
        let l =         Layers::without_upstream(&reg, "A1", &empty, &empty);
        let r = l.effective("toolhead.custom_mount_gcode").unwrap();
        assert_eq!(*r.value, serde_json::json!(""));
        assert_eq!(r.origin, Origin::Factory);

        // 版本上显式写了空串 → 有值，来源版本，且 has_own 为真
        let over = overrides(&[("toolhead.custom_mount_gcode", serde_json::json!(""))]);
        let l =         Layers::without_upstream(&reg, "A1", &empty, &over);
        let r = l.effective("toolhead.custom_mount_gcode").unwrap();
        assert_eq!(r.origin, Origin::Version, "写了空串也是写过");
        assert!(l.has_own(Level::Version, "toolhead.custom_mount_gcode"));
    }

    /// `machineFilter` 排除的字段：不进有效配方，进 not_applicable
    #[test]
    fn machine_filtered_key_is_absent_not_defaulted() {
        let (_d, reg) = registry();
        let empty = Overrides::new();

        let a1 =         Layers::without_upstream(&reg, "A1", &empty, &empty);
        assert!(a1.effective("toolhead.only_p1s").is_none(), "不该退回出厂默认");
        assert!(!a1.applies("toolhead.only_p1s"));
        assert_eq!(a1.not_applicable(), vec!["toolhead.only_p1s"]);
        assert!(!a1
            .effective_recipe()
            .contains_key("toolhead.only_p1s"));

        let p1s =         Layers::without_upstream(&reg, "P1S", &empty, &empty);
        assert_eq!(*p1s.effective("toolhead.only_p1s").unwrap().value, serde_json::json!(9));
        assert!(p1s.not_applicable().is_empty());
    }

    /// 上游根本没有这个 key → `None`，不是 panic，也不是造一个值
    #[test]
    fn unknown_key_is_none() {
        let (_d, reg) = registry();
        let empty = Overrides::new();
        let l =         Layers::without_upstream(&reg, "A1", &empty, &empty);
        assert!(l.effective("toolhead.made_up").is_none());
        assert!(!l.applies("toolhead.made_up"));
    }

    /// `has_own` 与 `origin` 不是一回事：基底写过、被版本盖住时
    /// 基底那一项仍然"自己写过" —— 挂回继承之后露出来的就是它
    #[test]
    fn has_own_is_not_the_same_as_origin() {
        let (_d, reg) = registry();
        let base = overrides(&[("toolhead.offset.x", serde_json::json!(1))]);
        let over = overrides(&[("toolhead.offset.x", serde_json::json!(2))]);
        let l =         Layers::without_upstream(&reg, "A1", &base, &over);

        assert_eq!(l.effective("toolhead.offset.x").unwrap().origin, Origin::Version);
        assert!(l.has_own(Level::Machine, "toolhead.offset.x"), "基底那一项还在");
        assert!(l.has_own(Level::Version, "toolhead.offset.x"));
        assert!(!l.has_own(Level::Machine, "toolhead.custom_mount_gcode"));
    }

    /// 两层里写着、却再也进不了产物的键要被列出来 —— 否则"我明明改过"会一直是错的
    #[test]
    fn orphan_keys_are_reported() {
        let (_d, reg) = registry();
        let base = overrides(&[
            ("toolhead.offset.x", serde_json::json!(1)),
            ("toolhead.only_p1s", serde_json::json!(5)), // A1 上不适用
            ("toolhead.deleted_upstream", serde_json::json!(7)), // 上游已删
        ]);
        let over = overrides(&[("toolhead.only_p1s", serde_json::json!(6))]);
        let l =         Layers::without_upstream(&reg, "A1", &base, &over);

        assert_eq!(
            l.orphan_keys(),
            vec!["toolhead.deleted_upstream", "toolhead.only_p1s"],
            "两层各出现一次的 only_p1s 只该报一次"
        );
        // 同一份数据在 P1S 上只剩真正被删的那个
        let l =         Layers::without_upstream(&reg, "P1S", &base, &over);
        assert_eq!(l.orphan_keys(), vec!["toolhead.deleted_upstream"]);
    }

    /// 有效配方按 key 排序且只含看得见的项
    #[test]
    fn effective_recipe_is_stable_and_filtered() {
        let (_d, reg) = registry();
        let empty = Overrides::new();
        let l =         Layers::without_upstream(&reg, "A1", &empty, &empty);
        let keys: Vec<&str> = l.effective_recipe().into_keys().collect();
        assert_eq!(
            keys,
            vec![
                "toolhead.custom_mount_gcode",
                "toolhead.gone",
                "toolhead.offset.x"
            ],
            "BTreeMap 要给出稳定顺序，且不含 only_p1s"
        );
    }

    /// **上游给的那一半算同一档来源，但不算「自有」**（doc §3.6）。
    ///
    /// 这是整个 §3.6 的落点：挂回继承要露出上游那一半，而不是掉到出厂默认
    #[test]
    fn the_upstream_half_counts_as_the_same_origin_but_not_as_own() {
        let (_d, reg) = registry();
        let empty = Overrides::new();
        // 上游给 A1 的机型差异：offset.x = -1
        let up_base = overrides(&[("toolhead.offset.x", serde_json::json!(-1))]);

        let l = Layers::new(&reg, "A1", &up_base, &empty, &empty, &empty);
        let r = l.effective("toolhead.offset.x").unwrap();
        assert_eq!(*r.value, serde_json::json!(-1));
        assert_eq!(r.origin, Origin::Machine, "上游机型差异也是「机型」这一档");
        assert!(
            !l.has_own(Level::Machine, "toolhead.offset.x"),
            "它不是我们写的，所以不算自有，「挂回继承」该禁用"
        );
        assert_eq!(l.own_count(Level::Machine), 0);

        // 我们在机型基底上盖一个值 → 自有为真，挂回继承可点
        let ours = overrides(&[("toolhead.offset.x", serde_json::json!(5))]);
        let l = Layers::new(&reg, "A1", &up_base, &ours, &empty, &empty);
        assert_eq!(*l.effective("toolhead.offset.x").unwrap().value, serde_json::json!(5));
        assert!(l.has_own(Level::Machine, "toolhead.offset.x"));
        assert_eq!(l.own_count(Level::Machine), 1);

        // 挂回继承（删掉我们那一项）→ 回到上游的 -1，**不是**出厂的 0
        let l = Layers::new(&reg, "A1", &up_base, &empty, &empty, &empty);
        assert_eq!(
            *l.effective("toolhead.offset.x").unwrap().value,
            serde_json::json!(-1),
            "挂回继承该露出上游那一半，不该掉到出厂默认"
        );
    }

    /// 五级查找的完整顺序：我们的版本 → 上游版本 → 我们的机型 → 上游机型 → 出厂
    #[test]
    fn lookup_order_is_ours_then_upstream_within_each_layer() {
        let (_d, reg) = registry();
        let k = "toolhead.offset.x";
        let up_base = overrides(&[(k, serde_json::json!(1))]);
        let ours_base = overrides(&[(k, serde_json::json!(2))]);
        let up_over = overrides(&[(k, serde_json::json!(3))]);
        let ours_over = overrides(&[(k, serde_json::json!(4))]);
        let empty = Overrides::new();

        let cases: [(&Overrides, &Overrides, &Overrides, &Overrides, i64, Origin); 5] = [
            (&up_base, &ours_base, &up_over, &ours_over, 4, Origin::Version),
            (&up_base, &ours_base, &up_over, &empty, 3, Origin::Version),
            (&up_base, &ours_base, &empty, &empty, 2, Origin::Machine),
            (&up_base, &empty, &empty, &empty, 1, Origin::Machine),
            (&empty, &empty, &empty, &empty, 0, Origin::Factory),
        ];
        for (ub, b, uo, o, want, origin) in cases {
            let l = Layers::new(&reg, "A1", ub, b, uo, o);
            let r = l.effective(k).unwrap();
            assert_eq!(*r.value, serde_json::json!(want));
            assert_eq!(r.origin, origin);
        }
    }

    /// `own_count` 不数已经失效的键 —— 界面上「N 项自有」不该把改不到产物的项算进去
    #[test]
    fn own_count_skips_keys_that_no_longer_apply() {
        let (_d, reg) = registry();
        let empty = Overrides::new();
        let base = overrides(&[
            ("toolhead.offset.x", serde_json::json!(1)),
            ("toolhead.only_p1s", serde_json::json!(5)),
        ]);
        let l = Layers::without_upstream(&reg, "A1", &base, &empty);
        assert_eq!(l.own_count(Level::Machine), 1, "only_p1s 在 A1 上不适用");
        let l = Layers::without_upstream(&reg, "P1S", &base, &empty);
        assert_eq!(l.own_count(Level::Machine), 2);
    }

    /// 指纹：值变了要变；**改了字段定义、值一个没动，也要变**
    #[test]
    fn fingerprint_reacts_to_values_and_to_the_field_definitions() {
        let (_d, reg) = registry();
        let empty = Overrides::new();
        let a =         Layers::without_upstream(&reg, "A1", &empty, &empty).fingerprint();

        let base = overrides(&[("toolhead.offset.x", serde_json::json!(1))]);
        let b =         Layers::without_upstream(&reg, "A1", &base, &empty).fingerprint();
        assert_ne!(a, b, "值变了指纹要变");

        // 同样的值换台机型也要变 —— 不同机型的产物是不同文件
        let c =         Layers::without_upstream(&reg, "P1S", &base, &empty).fingerprint();
        assert_ne!(b, c);

        // 只改字段定义（给 offset.x 换个 tomlKey），值一个没动
        let (_d2, reg2) = registry_with_toml_key("off_x_v2");
        let only_x = overrides(&[("toolhead.offset.x", serde_json::json!(1))]);
        let changed_defs =         Layers::without_upstream(&reg2, "A1", &only_x, &empty).fingerprint();
        let same_values =         Layers::without_upstream(&reg, "A1", &only_x, &empty).fingerprint();
        assert_ne!(
            changed_defs, same_values,
            "改了 tomlKey 产出的 TOML 就不一样了，产物必须变成待生成"
        );
    }

    /// 只有一个参数、`tomlKey` 可指定的最小字段定义。给上面那条指纹判据用
    fn registry_with_toml_key(toml_key: &str) -> (tempfile::TempDir, Registry) {
        let d = tempfile::tempdir().unwrap();
        let params = serde_json::json!({
            "params": [{
                "key": "toolhead.offset.x", "configKey": "X",
                "tomlKey": toml_key, "jsonKey": "toolhead.offset.x",
                "label": "X 轴偏移", "desc": "", "tomlComment": "",
                "valueType": "float", "uiComponent": "number", "defaultValue": 0,
                "scope": "universal", "section": "toolhead",
                "layout": { "order": 1, "sectionId": "s1" }
            }],
            "tabs": [{ "id": "t1", "label": "页签一", "order": 10,
                       "sections": [{ "id": "s1", "label": "分组一", "order": 0 }] }],
            "updated": "2026-01-01 00:00:00"
        });
        let layout = serde_json::json!({
            "tabs": [{ "id": "t1", "sections": [{ "id": "s1", "items": [
                { "id": "i0", "paramKey": "toolhead.offset.x" }
            ] }] }]
        });
        let r = crate::workbench::presets::registry::load_from_json_fixture(
            d.path(),
            &params,
            &layout,
        )
        .unwrap();
        (d, r)
    }

    /// 真数据里那两个出厂默认是空串的字段**真的存在** ——
    /// 没有它们，上面那条"空串是值"的判据就是在测一个不存在的情况
    #[test]
    fn real_data_really_has_empty_string_defaults() {
        let Some(root) = paths::presets_root() else {
            eprintln!("没定位到 <repo>/presets，这条对齐检查未执行（不是通过）");
            return;
        };
        let reg = Registry::load_from(&root).unwrap();
        let blanks: Vec<&str> = reg
            .params()
            .iter()
            .filter(|p| p.default_value == serde_json::json!(""))
            .map(|p| p.key.as_str())
            .collect();
        assert!(
            blanks.contains(&"toolhead.custom_mount_gcode")
                && blanks.contains(&"toolhead.custom_unmount_gcode"),
            "出厂默认为空串的字段变了：{blanks:?}"
        );
    }
}
