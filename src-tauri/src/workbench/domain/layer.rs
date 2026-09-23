//! 三层取值与来源层（doc §3.1、§3.6）。
//!
//! ```text
//! 出厂默认（param_registry.defaultValue，全局单值）
//!   ← 机型基底（machineVariants 的裸键，比如 `A1`）
//!     ← 版本覆盖（machineVariants 的版本键，比如 `A1:FAST`）
//! ```
//!
//! # 一张表，不是一个加一个（b04 Task 12）
//!
//! 机型层与版本层的**唯一真源**是 `presets/registry/param_registry.toml` 里每个字段的
//! `machineVariants`：裸键（`"A1"`）归机型层，带冒号（`"A1:FAST"`）归版本层。
//! **工作台改出来的值回写进同一张表**，所以"看"和"改"是同一份东西。
//!
//! 在此之前还有第二处：我们自造的 `workbench/machines|versions/*.json`。
//! 于是**每层各有两半**（上面这表里的 + 我们 json 里的），查找就得排五档：
//!
//! ```text
//!   我们的版本 → 表里的版本 → 我们的机型 → 表里的机型 → 出厂
//! ```
//!
//! 那份 json 在[b04 Task 12]删掉（盘上本来一个文件都没有，见 REPORT §1.2），
//! 五档于是退回三档。这一轮所有注释里的"那一半"都失去了对象。
//!
//! ## 由此翻掉的两条旧约定
//!
//! 1. **「挂回继承」现在一次退到出厂默认。** 以前它退的是"上游那一半"
//!    （把 A1 的 X 轴偏移挂回继承会回到 `-1`），现在会回到全局默认 `0`。
//!    `-1` 与 `0` 都是 registry 里的真值，区别只在于它记在哪一层；
//!    这一层不再替任何人留一手。
//! 2. **「没法把某一项压回出厂默认」这条代价没有了。** 它是上一层的第二那半
//!    逼出来的：那一半写不了"删除"，只能写值。现在 `clear_variant` 直接删键。
//!
//! # 不适用 vs 看得见改不动
//!
//! 每层都是稀疏表，**键在不在就是脱钩的载体**。所以：
//!
//! - 没有单独的「脱钩」标记。脱钩天然是按项算的，不是按版本算的。
//! - 删键 = 挂回继承（doc §4.1 那条 `value: None`）。
//!
//! 反过来说，**"这一层没有这个键"和"这一层把它设成了空值"是两件事**，不能混：
//! 有两个字段（`toolhead.custom_mount_gcode` / `custom_unmount_gcode`）的出厂默认
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
//! 某个参数被标了 `deprecated`、或者这台机型被从某个参数的 `machineFilter` 里摘掉之后，
//! `machineVariants` 里那个值**还在表里，但再也不会进任何产物**。这是典型的静默失效：
//! 界面上什么都看不到，而"我明明改过"这件事会一直是错的。
//! [`Layers::orphan_keys`] 把它们列出来。

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::Serialize;
use serde_json::Value;

use crate::workbench::presets::ParamRegistry as Registry;

/// 一层的稀疏覆盖表。`BTreeMap` 不只是为了好看 ——
/// 指纹要按稳定顺序算，`HashMap` 每次进程的遍历顺序都不同
pub type Overrides = BTreeMap<String, Value>;

/// 空表的共享借用。给「机型列没有版本层」用，免得每个调用点自己造一个再借
pub fn no_overrides() -> &'static Overrides {
    static EMPTY: OnceLock<Overrides> = OnceLock::new();
    EMPTY.get_or_init(Overrides::new)
}

/// 可写的两层。出厂层不在这里：它是全局单值，不是某一台机器的东西
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
/// 借用而不持有：两张负载分别属于 registry 的归并结果与其上的草稿合成结果，
/// 复制一份进来就等于又造了一个"当前状态"的副本（doc §1 第四条铁律不许）
pub struct Layers<'a> {
    registry: &'a Registry,
    machine_id: &'a str,
    /// 机型层：`machineVariants` 的裸键（`A1`）
    base: &'a Overrides,
    /// 版本层：`machineVariants` 的版本键（`A1:FAST`）。只看机型那一列时是空表
    over: &'a Overrides,
}

impl<'a> Layers<'a> {
    pub fn new(
        registry: &'a Registry,
        machine_id: &'a str,
        base: &'a Overrides,
        over: &'a Overrides,
    ) -> Self {
        Self {
            registry,
            machine_id,
            base,
            over,
        }
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
    /// 查找顺序就是三档：版本 → 机型 → 出厂（doc §3.1）
    pub fn effective(&self, key: &str) -> Option<ValueOrigin<'a>> {
        if !self.applies(key) {
            return None;
        }
        for (table, origin) in [(self.over, Origin::Version), (self.base, Origin::Machine)] {
            if let Some(v) = table.get(key) {
                return Some(ValueOrigin { value: v, origin });
            }
        }
        self.registry.param(key).map(|p| ValueOrigin {
            value: &p.default_value,
            origin: Origin::Factory,
        })
    }

    /// 这一层有没有**自己钉着**这个键 = 「挂回继承」点得下去吗。
    ///
    /// 「自己钉着」就是 `machineVariants` 里真的有一条形如 `A1`（机型层）
    /// 或 `A1:FAST`（版本层）的键。**注意连带：删掉它之后这个值是退到上一层** ——
    /// 版本层删完有可能还落在机型层上，不是一步退到出厂默认。
    ///
    /// 它和 `effective(...).origin` 不是一回事：版本层钉着某个键时
    /// `has_own(Machine, key)` 仍然可能为真（机型层写着，只是被盖住了）
    pub fn has_own(&self, level: Level, key: &str) -> bool {
        match level {
            Level::Machine => self.base.contains_key(key),
            Level::Version => self.over.contains_key(key),
        }
    }

    /// 这一层钉着几项。以前这一列的台词是「N 项（自有 M 项）」——
    /// 那时候每层有两半，得分开数。**现在每层只有一张表，钉着几项就是几项**，
    /// 所以这一个数同时是界面上的「项数」和「自有项数」
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

    /// 写着、但再也不会进任何产物的键。
    ///
    /// 两种来法：这个参数被标了 `deprecated` /
    /// 这台机型被从这个参数的 `machineFilter` 里摘掉了。
    /// 两种的表现都一样 —— 表里有值，产物里没有，界面上没人提。
    ///
    /// 两种表的键都查：它们就是同一张 `machineVariants` 拆出来的
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

        let l =         Layers::new(&reg, "A1", &base, &over);
        let r = l.effective("toolhead.offset.x").unwrap();
        assert_eq!(*r.value, serde_json::json!(2));
        assert_eq!(r.origin, Origin::Version);

        let empty = Overrides::new();
        let l =         Layers::new(&reg, "A1", &base, &empty);
        let r = l.effective("toolhead.offset.x").unwrap();
        assert_eq!(*r.value, serde_json::json!(1));
        assert_eq!(r.origin, Origin::Machine);

        let l =         Layers::new(&reg, "A1", &empty, &empty);
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
        let l =         Layers::new(&reg, "A1", &empty, &empty);
        let r = l.effective("toolhead.custom_mount_gcode").unwrap();
        assert_eq!(*r.value, serde_json::json!(""));
        assert_eq!(r.origin, Origin::Factory);

        // 版本上显式写了空串 → 有值，来源版本，且 has_own 为真
        let over = overrides(&[("toolhead.custom_mount_gcode", serde_json::json!(""))]);
        let l =         Layers::new(&reg, "A1", &empty, &over);
        let r = l.effective("toolhead.custom_mount_gcode").unwrap();
        assert_eq!(r.origin, Origin::Version, "写了空串也是写过");
        assert!(l.has_own(Level::Version, "toolhead.custom_mount_gcode"));
    }

    /// `machineFilter` 排除的字段：不进有效配方，进 not_applicable
    #[test]
    fn machine_filtered_key_is_absent_not_defaulted() {
        let (_d, reg) = registry();
        let empty = Overrides::new();

        let a1 =         Layers::new(&reg, "A1", &empty, &empty);
        assert!(a1.effective("toolhead.only_p1s").is_none(), "不该退回出厂默认");
        assert!(!a1.applies("toolhead.only_p1s"));
        assert_eq!(a1.not_applicable(), vec!["toolhead.only_p1s"]);
        assert!(!a1
            .effective_recipe()
            .contains_key("toolhead.only_p1s"));

        let p1s =         Layers::new(&reg, "P1S", &empty, &empty);
        assert_eq!(*p1s.effective("toolhead.only_p1s").unwrap().value, serde_json::json!(9));
        assert!(p1s.not_applicable().is_empty());
    }

    /// 上游根本没有这个 key → `None`，不是 panic，也不是造一个值
    #[test]
    fn unknown_key_is_none() {
        let (_d, reg) = registry();
        let empty = Overrides::new();
        let l =         Layers::new(&reg, "A1", &empty, &empty);
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
        let l =         Layers::new(&reg, "A1", &base, &over);

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
        let l =         Layers::new(&reg, "A1", &base, &over);

        assert_eq!(
            l.orphan_keys(),
            vec!["toolhead.deleted_upstream", "toolhead.only_p1s"],
            "两层各出现一次的 only_p1s 只该报一次"
        );
        // 同一份数据在 P1S 上只剩真正被删的那个
        let l =         Layers::new(&reg, "P1S", &base, &over);
        assert_eq!(l.orphan_keys(), vec!["toolhead.deleted_upstream"]);
    }

    /// 有效配方按 key 排序且只含看得见的项
    #[test]
    fn effective_recipe_is_stable_and_filtered() {
        let (_d, reg) = registry();
        let empty = Overrides::new();
        let l =         Layers::new(&reg, "A1", &empty, &empty);
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

    /// 三级查找的完整顺序：版本 → 机型 → 出厂。
    ///
    /// b04 Task 12 之前这里是五档（每层各有"上游给的"与"我们写的"两半）。
    /// 现在每层只有一张表，中间那两档连同它们的来源一起没有了
    #[test]
    fn lookup_order_is_version_then_machine_then_factory() {
        let (_d, reg) = registry();
        let k = "toolhead.offset.x";
        let base = overrides(&[(k, serde_json::json!(1))]);
        let over = overrides(&[(k, serde_json::json!(2))]);
        let empty = Overrides::new();

        let cases: [(&Overrides, &Overrides, i64, Origin); 3] = [
            (&base, &over, 2, Origin::Version),
            (&base, &empty, 1, Origin::Machine),
            (&empty, &empty, 0, Origin::Factory),
        ];
        for (b, o, want, origin) in cases {
            let l = Layers::new(&reg, "A1", b, o);
            let r = l.effective(k).unwrap();
            assert_eq!(*r.value, serde_json::json!(want));
            assert_eq!(r.origin, origin);
        }
    }

    /// **挂回继承是逐层退的**：删掉版本层那一项，下面是机型层（不是出厂默认）；
    /// 机型层也没有了才轮到出厂默认。
    ///
    /// 这一条钉的是 b04 Task 12 翻掉的那条旧约定：以前每层有两半，
    /// 删掉"我们写的"会露出"上游给的"。现在删到空就是出厂默认
    #[test]
    fn detaching_falls_back_one_level_at_a_time() {
        let (_d, reg) = registry();
        let k = "toolhead.offset.x";
        let empty = Overrides::new();
        let base = overrides(&[(k, serde_json::json!(-1))]);
        let over = overrides(&[(k, serde_json::json!(5))]);

        // 两层都钉着 → 版本赢
        let l = Layers::new(&reg, "A1", &base, &over);
        assert_eq!(*l.effective(k).unwrap().value, serde_json::json!(5));
        assert!(l.has_own(Level::Version, k));

        // 删掉版本层那一项 → 露出机型层的 -1，**不是**出厂的 0
        let l = Layers::new(&reg, "A1", &base, &empty);
        assert_eq!(
            *l.effective(k).unwrap().value,
            serde_json::json!(-1),
            "这一步是这整条判据存在的理由"
        );
        assert_eq!(l.effective(k).unwrap().origin, Origin::Machine);

        // 机型层那一项也没了 → 才是出厂默认
        let l = Layers::new(&reg, "A1", &empty, &empty);
        assert_eq!(l.effective(k).unwrap().origin, Origin::Factory);
        assert!(!l.has_own(Level::Machine, k), "没钉着就不该给出「挂回继承」");
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
        let l = Layers::new(&reg, "A1", &base, &empty);
        assert_eq!(l.own_count(Level::Machine), 1, "only_p1s 在 A1 上不适用");
        let l = Layers::new(&reg, "P1S", &base, &empty);
        assert_eq!(l.own_count(Level::Machine), 2);
    }

    /// 指纹：值变了要变；**改了字段定义、值一个没动，也要变**
    #[test]
    fn fingerprint_reacts_to_values_and_to_the_field_definitions() {
        let (_d, reg) = registry();
        let empty = Overrides::new();
        let a =         Layers::new(&reg, "A1", &empty, &empty).fingerprint();

        let base = overrides(&[("toolhead.offset.x", serde_json::json!(1))]);
        let b =         Layers::new(&reg, "A1", &base, &empty).fingerprint();
        assert_ne!(a, b, "值变了指纹要变");

        // 同样的值换台机型也要变 —— 不同机型的产物是不同文件
        let c =         Layers::new(&reg, "P1S", &base, &empty).fingerprint();
        assert_ne!(b, c);

        // 只改字段定义（给 offset.x 换个 tomlKey），值一个没动
        let (_d2, reg2) = registry_with_toml_key("off_x_v2");
        let only_x = overrides(&[("toolhead.offset.x", serde_json::json!(1))]);
        let changed_defs =         Layers::new(&reg2, "A1", &only_x, &empty).fingerprint();
        let same_values =         Layers::new(&reg, "A1", &only_x, &empty).fingerprint();
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
