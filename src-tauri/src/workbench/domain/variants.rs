//! `machineVariants` 的三步归并（doc §3.3）。
//!
//! # 它在整件事里的位置
//!
//! 上游的 `source/machines/*.toml` **一个参数覆盖块都没有**（实测：机型实体 TOML 只有
//! 身份字段 + `[dimensions]` + `[[versions]]`）。机型的参数差异全在
//! `param_registry.json` 的 `params[].machineVariants` 这张表里，键有两种形态：
//!
//! ```text
//! "P1S"        纯机型键
//! "P1S:LITE"   版本键（= catalog 的 machineKey）
//! ```
//!
//! 这张表**不属于出厂层**。出厂层是全局单值 `defaultValue`（决议 2）；
//! `machineVariants` 是"这台机器的驻点在哪、挂载指令怎么写"，属于机型/版本那两层。
//! 所以首次加载时要把它消化进去（doc §3.4 决议 A）。
//!
//! **不消化的后果很硬**：那 5 个有这张表的字段会全部退回全局 `defaultValue`，
//! 于是 6 台机型的喷嘴偏移与装卸胶箱 G-code 变成同一个值 —— A1 的 `-1` 和 P1S 的
//! `-25.9` 会一起变成 `0`。
//!
//! # 三步
//!
//! ```text
//! 对每台机型，按 visible_keys(机型)（已按 machineFilter + deprecated 过滤）遍历：
//!   1. 纯机型键（"P1S"）                      → 直接进机型基底
//!   2. 版本键（"P1S:LITE"）：
//!        这台机型**所有**版本都写了、且值完全相同 → 上提到机型基底，版本上不留覆盖
//!        否则                                  → 留在各版本的覆盖里
//!   3. 没有这张表的字段（69/74）                → 什么都不写，靠继承出厂默认
//! ```
//!
//! 第 2 步的上提是这里的主要产出：实测 18 项机型基底里，绝大多数来自上提，
//! 而不是来自纯机型键（纯机型键只有 3 个实例，全在 `toolhead.offset.z` 上）。
//!
//! # 刻意不做更激进的压缩
//!
//! "取多数值当基底、少数派留覆盖"要替人挑一个「标准版本」当基准 —— 那是凭空造的判断，
//! 而且会让「这一项我没动过」变成一句假话。**只在全体一致时才上提。**
//!
//! # 无损
//!
//! 对任意 `(机型, 版本, key)`，归并前后的有效值完全一致，只是记在哪一层不同。
//! [`tests::merging_is_lossless_on_real_upstream`] 逐项验这件事 ——
//! 因为"无损"是这套归并唯一不可让的性质：它一旦不成立，用户会看到某台机器的偏移
//! 悄悄变了，而没有任何一步会报错。
//!
//! 注意**无损本身不足以说明归并是对的**：把所有值都塞进版本覆盖、基底留空，
//! 同样是无损的。所以结构性的判据（什么时候该上提）另有一组单测盯着。

use std::collections::BTreeMap;

use serde_json::Value;

use crate::workbench::presets::registry::ParamDef;
use crate::workbench::presets::ParamRegistry as Registry;

use super::layer::Overrides;

/// 归并结果：一台机型的两层初值
#[derive(Debug, Clone, Default)]
pub struct Digested {
    /// 机型基底
    pub base: Overrides,
    /// 版本 id → 版本覆盖。**每个版本都有一项**（可能是空表），
    /// 这样调用方遍历时不用分"没有这个版本"和"这个版本没覆盖"
    pub versions: BTreeMap<String, Overrides>,
}

impl Digested {
    /// 覆盖项总数。doc §3.3 那张表的「覆盖项」一列
    pub fn override_count(&self) -> usize {
        self.versions.values().map(BTreeMap::len).sum()
    }
}

/// 这一项现在是不是**被上提到了基底** = 这台机型的每个版本都写了同一个值。
///
/// 判定与 [`digest`] 里那一步共用同一份实现，**不许各写一遍**：
/// 写回那边（[`crate::workbench::app::storage::plan_value_edits`]）要知道
/// 一次机型层的改动该落到几个键上 —— 只写裸键会被这一组版本键盖住，
/// 于是用户点了保存、值又变回去，而没有任何一步报错。
pub fn promoted_to_base(param: &ParamDef, machine_id: &str, version_ids: &[String]) -> bool {
    let table = &param.machine_variants;
    if table.is_empty() || version_ids.is_empty() {
        return false;
    }
    let per_version: Vec<Option<&Value>> = version_ids
        .iter()
        .map(|id| table.get(&format!("{machine_id}:{id}")))
        .collect();
    let first = per_version.first().copied().flatten();
    per_version.iter().all(|v| v.is_some()) && per_version.iter().all(|v| *v == first)
}

/// 把一台机型的 `machineVariants` 消化成机型基底 + 版本覆盖。
///
/// 收的是**机型 id + 版本 id 列表**而不是某一层的机型对象：这张表只认这两样
/// （键是 `"P1S"` 与 `"P1S:LITE"`），收一个具体类型只会把这个纯函数绑在某个 loader 上。
/// b04 Task 8 把清单从上游换成 `presets/machines/*.toml` 时，这里因此一个字都不用改。
///
/// 纯函数：同样的输入永远得到同样的输出，不碰文件、不看时间
pub fn digest(registry: &Registry, machine_id: &str, version_ids: &[String]) -> Digested {
    let mut out = Digested {
        base: Overrides::new(),
        versions: version_ids
            .iter()
            .map(|id| (id.clone(), Overrides::new()))
            .collect(),
    };

    for key in registry.visible_keys(machine_id) {
        let Some(param) = registry.param(key) else {
            continue;
        };
        let table = &param.machine_variants;
        if table.is_empty() {
            continue; // 第 3 步：靠继承
        }

        // 第 1 步：纯机型键
        if let Some(v) = table.get(machine_id) {
            out.base.insert(key.to_owned(), v.clone());
        }

        // 第 2 步：版本键。`机型:版本` 就是 catalog 里那个 `machineKey`
        let per_version: Vec<Option<&Value>> = version_ids
            .iter()
            .map(|id| table.get(&format!("{machine_id}:{id}")))
            .collect();

        let first = per_version.first().copied().flatten();
        if promoted_to_base(param, machine_id, version_ids) {
            // 上提。**刻意允许它盖掉第 1 步写下的纯机型值**：
            // 每个版本都有更具体的键，那个纯机型值本来就到不了任何版本，
            // 留着它反而会让基底显示一个没人在用的数
            if let Some(v) = first {
                out.base.insert(key.to_owned(), v.clone());
            }
        } else {
            for (ver, value) in version_ids.iter().zip(per_version) {
                if let Some(v) = value {
                    out.versions
                        .entry(ver.clone())
                        .or_default()
                        .insert(key.to_owned(), v.clone());
                }
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::domain::layer::{Layers, Origin};
    use crate::workbench::paths;

    /// 造一份字段定义 + 一台机型的版本清单。字段定义走真的反序列化路径，
    /// 免得夹具和真数据在字段名上悄悄分岔。
    ///
    /// 机型只出 id 与版本 id：`digest` 认的就这两样（见它的文档），
    /// 所以这里不再拼一整份 catalog + manifest 只为了造一个机型对象
    fn fixture(versions: &[&str], variants: serde_json::Value) -> (Registry, Vec<String>) {
        let d = tempfile::tempdir().unwrap();
        let root = d.path();

        let params = serde_json::json!({
            "params": [{
                "key": "toolhead.offset.x", "configKey": "X", "tomlKey": "off_x",
                "jsonKey": "off_x", "label": "X 轴偏移", "desc": "", "tomlComment": "",
                "valueType": "float", "uiComponent": "number", "defaultValue": 0,
                "scope": "machine_specific", "section": "toolhead",
                "layout": { "order": 1, "sectionId": "s1" },
                "machineVariants": variants
            }],
            "tabs": [{ "id": "t1", "label": "页签一", "order": 10,
                       "sections": [{ "id": "s1", "label": "分组一", "order": 0 }] }],
            "updated": "2026-01-01 00:00:00"
        });
        let layout = serde_json::json!({
            "tabs": [{ "id": "t1", "sections": [{
                "id": "s1", "items": [{ "id": "i0", "paramKey": "toolhead.offset.x" }]
            }] }]
        });

        let reg =
            crate::workbench::presets::registry::load_from_json_fixture(root, &params, &layout)
                .unwrap();
        (reg, versions.iter().map(|v| (*v).to_owned()).collect())
    }

    /// 三个版本值都一样 → **上提到基底，版本上一条覆盖都不留**
    #[test]
    fn unanimous_version_keys_are_promoted_to_base() {
        let (reg, vs) = fixture(
            &["STANDARD", "FAST", "FASTV3.3"],
            serde_json::json!({ "A1:STANDARD": -1, "A1:FAST": -1, "A1:FASTV3.3": -1 }),
        );
        let d = digest(&reg, "A1", &vs);
        assert_eq!(d.base["toolhead.offset.x"], serde_json::json!(-1));
        assert_eq!(d.override_count(), 0, "全体一致就不该留覆盖");
    }

    /// 值不一致 → 各留覆盖，基底不写。
    /// **不做"取多数当基底"** —— 那要替人挑一个标准版本
    #[test]
    fn differing_version_keys_stay_as_overrides() {
        let (reg, vs) = fixture(
            &["STANDARD", "FAST", "FASTV3.3"],
            serde_json::json!({ "A1:STANDARD": -1, "A1:FAST": -0.7, "A1:FASTV3.3": 0.1 }),
        );
        let d = digest(&reg, "A1", &vs);
        assert!(
            !d.base.contains_key("toolhead.offset.x"),
            "基底不该猜一个值"
        );
        assert_eq!(d.override_count(), 3);
        assert_eq!(
            d.versions["FAST"]["toolhead.offset.x"],
            serde_json::json!(-0.7)
        );
    }

    /// **少一个版本就不算一致**：两个版本写了同一个值、第三个没写，
    /// 上提会让第三个版本从"继承出厂默认"变成"继承那个值" —— 那是改了它的有效值
    #[test]
    fn partial_coverage_is_not_unanimous_even_if_values_agree() {
        let (reg, vs) = fixture(
            &["STANDARD", "FAST", "FASTV3.3"],
            serde_json::json!({ "A1:STANDARD": -1, "A1:FAST": -1 }),
        );
        let d = digest(&reg, "A1", &vs);
        assert!(
            !d.base.contains_key("toolhead.offset.x"),
            "上提会把没写的那个版本的有效值也改掉"
        );
        assert_eq!(d.override_count(), 2);
        assert!(!d.versions["FASTV3.3"].contains_key("toolhead.offset.x"));
    }

    /// 纯机型键直接进基底
    #[test]
    fn pure_machine_key_goes_to_base() {
        let (reg, vs) = fixture(&["STANDARD", "FAST"], serde_json::json!({ "A1": 1.1 }));
        let d = digest(&reg, "A1", &vs);
        assert_eq!(d.base["toolhead.offset.x"], serde_json::json!(1.1));
        assert_eq!(d.override_count(), 0);
    }

    /// 两种键**同时存在且值相同** —— 这是 `toolhead.offset.z` 在 P1S/P2S/X1C 上的真实形状。
    /// 结果只有基底一项，不该重复记两处
    #[test]
    fn both_key_forms_with_the_same_value_collapse_to_one_base_entry() {
        let (reg, vs) = fixture(
            &["STANDARD"],
            serde_json::json!({ "A1": 1.1, "A1:STANDARD": 1.1 }),
        );
        let d = digest(&reg, "A1", &vs);
        assert_eq!(d.base["toolhead.offset.x"], serde_json::json!(1.1));
        assert_eq!(d.base.len(), 1);
        assert_eq!(d.override_count(), 0);
    }

    /// 两种键同时存在但**值不同**：版本键更具体，赢。
    /// 真数据现在没有这种形状（实测两边都是 1.1），所以这条钉的是"万一出现时怎么办"，
    /// 而不是在描述现状
    #[test]
    fn version_key_wins_over_pure_machine_key_when_they_disagree() {
        let (reg, vs) = fixture(
            &["STANDARD"],
            serde_json::json!({ "A1": 4, "A1:STANDARD": 1.1 }),
        );
        let d = digest(&reg, "A1", &vs);
        assert_eq!(
            d.base["toolhead.offset.x"],
            serde_json::json!(1.1),
            "那个纯机型值到不了任何版本，留着只会在基底上显示一个没人用的数"
        );
    }

    /// 没有这张表的字段什么都不写 —— 靠继承（69/74 都是这种）
    #[test]
    fn params_without_the_table_write_nothing() {
        let (reg, vs) = fixture(&["STANDARD"], serde_json::json!({}));
        let d = digest(&reg, "A1", &vs);
        assert!(d.base.is_empty());
        assert_eq!(d.override_count(), 0);
        // 但版本项要在，调用方才好遍历
        assert!(d.versions.contains_key("STANDARD"));
    }

    /// 键指向别的机型时这台机型什么都不拿到 —— A2L 就是这样（0 基底 0 覆盖）
    #[test]
    fn keys_for_other_machines_are_ignored() {
        let (reg, vs) = fixture(
            &["STANDARD"],
            serde_json::json!({ "P1S": 1.1, "P1S:LITE": 1.1 }),
        );
        let d = digest(&reg, "A1", &vs);
        assert!(d.base.is_empty(), "这是 A2L 那一栏为什么是 0/0");
        assert_eq!(d.override_count(), 0);
    }

    /* ---------- 真数据（`<repo>/presets/`） ---------- */

    fn real() -> Option<crate::workbench::presets::Presets> {
        let root = paths::presets_root()?;
        Some(crate::workbench::presets::Presets::load_from(&root).expect("presets 读不通"))
    }

    /// 一台机型的版本 id 列表
    fn vids_of(m: &crate::workbench::presets::Machine) -> Vec<String> {
        m.versions.iter().map(|v| v.id.clone()).collect()
    }

    /// **无损**：对每个 `(机型, 版本, key)`，归并后按三层解析出来的值
    /// 必须等于直接查 `machineVariants` 得到的值。
    ///
    /// 这是这套归并唯一不可让的性质 —— 它不成立时用户会看到某台机器的偏移悄悄变了，
    /// 而没有任何一步会报错
    #[test]
    fn merging_is_lossless_on_real_data() {
        let Some(p) = real() else {
            eprintln!("没定位到 <repo>/presets，这条对齐检查未执行（不是通过）");
            return;
        };

        let mut checked = 0usize;
        for m in p.catalog.machines() {
            let d = digest(&p.registry, &m.id, &vids_of(m));
            for v in &m.versions {
                let over = &d.versions[&v.id];
                // 归并结果进的是「数据给的那一半」（doc §3.6）—— 它不落盘
                let layers = Layers::new(&p.registry, &m.id, &d.base, over);
                for key in p.registry.visible_keys(&m.id) {
                    let param = p.registry.param(key).unwrap();
                    // 归并前这个 (机型, 版本, key) 本来该是什么值
                    let machine_key = format!("{}:{}", m.id, v.id);
                    let want = param
                        .machine_variants
                        .get(&machine_key)
                        .or_else(|| param.machine_variants.get(&m.id))
                        .unwrap_or(&param.default_value);
                    let got = layers.effective(key).expect("有效配方里少了一项").value;
                    assert_eq!(
                        got, want,
                        "{}/{} 的 {key} 归并前后不一致 —— 这会让用户的偏移悄悄变了",
                        m.id, v.id
                    );
                    checked += 1;
                }
            }
        }
        assert!(checked > 0, "一项都没查，判据在空转");
    }

    /// doc §3.3 那张口径表的判据。
    ///
    /// **这一条会随数据更新而红**，而且那不算 bug：它钉的是「我对当前数据算出来的
    /// 口径」，`machineVariants` 改了它就该红。红了之后要做的是重算那张表、
    /// 更新 doc，而不是把断言删掉。
    ///
    /// 为什么值得留一条会腐烂的判据：无损**不足以**说明归并是对的 ——
    /// 把所有值塞进版本覆盖、基底留空，同样无损。上提这一步只有靠口径才看得出来。
    #[test]
    fn real_data_matches_the_documented_split() {
        let Some(p) = real() else {
            eprintln!("没定位到 <repo>/presets，这条对齐检查未执行（不是通过）");
            return;
        };

        let want: BTreeMap<&str, (usize, usize)> = [
            ("A1", (1, 9)),
            ("A1_MINI", (2, 6)),
            ("A2L", (0, 0)),
            ("P1S", (5, 0)),
            ("P2S", (5, 0)),
            ("X1C", (5, 0)),
        ]
        .into_iter()
        .collect();

        let mut total = (0usize, 0usize);
        for m in p.catalog.machines() {
            let d = digest(&p.registry, &m.id, &vids_of(m));
            let got = (d.base.len(), d.override_count());
            total.0 += got.0;
            total.1 += got.1;
            if let Some(w) = want.get(m.id.as_str()) {
                assert_eq!(
                    got, *w,
                    "{} 的基底/覆盖口径与 doc §3.3 那张表不一致（表里是 {w:?}）—— \
                     数据改过就重算那张表，别删这条断言",
                    m.id
                );
            }
        }
        assert_eq!(total, (18, 15), "合计与 doc §3.3 不一致");

        // A1 基底里那一项的身份也钉住：表里写的是 custom_mount_gcode
        let a1 = digest(
            &p.registry,
            "A1",
            &vids_of(p.catalog.machine("A1").unwrap()),
        );
        assert!(
            a1.base.contains_key("toolhead.custom_mount_gcode"),
            "A1 基底那一项应该是 custom_mount_gcode，实际是 {:?}",
            a1.base.keys().collect::<Vec<_>>()
        );

        // A2L 的 0/0 单独说一句：它**不是**"读失败了"，是数据没给它写任何机型差异
        let a2l = digest(
            &p.registry,
            "A2L",
            &vids_of(p.catalog.machine("A2L").unwrap()),
        );
        assert!(a2l.base.is_empty() && a2l.override_count() == 0);
        assert_eq!(a2l.versions.len(), 1, "它有一个版本，只是没有机型差异");

        // 顺带验一件与 A2L 有关的事：参数**不缺**，全部落到出厂默认
        let empty = Overrides::new();
        let layers = Layers::new(&p.registry, "A2L", &empty, &empty);
        let keys = p.registry.visible_keys("A2L");
        assert!(!keys.is_empty(), "A2L 一个参数都看不到就说明过滤过头了");
        for key in keys {
            let r = layers
                .effective(key)
                .expect("A2L 的参数应该落到出厂默认，不是没有");
            assert_eq!(r.origin, Origin::Factory);
        }
    }

    /// **换 loader 的一次性对齐判据**（b04 Task 9.3）。
    ///
    /// 三层取值刚从上游那份 `content/param_registry.json` 换成我们的
    /// `presets/registry/param_registry.toml`。两份数据本该等值（后者是前者的源），
    /// 但"本该"不是判据 —— 值在换 loader 时悄悄变了的话，用户看到的是某台机器的偏移
    /// 莫名不一样，而没有任何一步会报错。
    ///
    /// # 一处**已知且刻意保留**的差异：`0` 与 `0.0`
    ///
    /// 实测 42 条字段的 `defaultValue` 在旧 JSON 里是整数（`0`），在 TOML 里是浮点（`0.0`）。
    /// 那是**旧 JSON 丢了信息**：源文件写的就是 `0.0`（`valueType = 'float'`），
    /// 是构建那一步按 Go 的 `json.Marshal` 把它印成了 `0`。换回 TOML 等于把精度找回来。
    ///
    /// 所以这条判据按**数值**比，不按 JSON 表示比；但差异不许被悄悄吞掉：
    /// 表示不同的那些会被数出来，数目为 0 时反而说明这条判据失去了对象。
    ///
    /// 已知的下游影响只有一处：`fingerprint()` 变了，于是所有版本在换源后会被判成
    /// 「待生成」一次。那是对的 —— 字段定义的来源真的换了。
    ///
    /// **它是临时的** —— Task 12 把上游整个删掉时，这一条跟着删
    #[test]
    fn the_toml_and_the_retired_json_agree_on_every_value() {
        let (Some(p), Some(up_root)) = (real(), paths::upstream_root()) else {
            eprintln!("没同时定位到 presets 与上游，这条对齐检查未执行（不是通过）");
            return;
        };
        let raw = std::fs::read_to_string(up_root.join("content").join("param_registry.json"))
            .expect("读得到那份旧 JSON");
        let json: serde_json::Value = serde_json::from_str(&raw).expect("合法 JSON");
        let params = json["params"].as_array().expect("params 是数组");

        assert_eq!(
            params.len(),
            p.registry.params().len(),
            "两份数据的字段数不等"
        );

        /// 等值：数字按 f64 比（`0` 与 `0.0` 算相等），其余按原样比
        fn same(a: &Value, b: &Value) -> bool {
            match (a.as_f64(), b.as_f64()) {
                (Some(x), Some(y)) => x == y,
                _ => a == b,
            }
        }

        let mut compared = 0usize;
        let mut repr_only = 0usize;
        for jp in params {
            let key = jp["key"].as_str().expect("每条都有 key");
            let ours = p
                .registry
                .param(key)
                .unwrap_or_else(|| panic!("TOML 里没有 {key}"));

            let theirs_default = &jp["defaultValue"];
            assert!(
                same(&ours.default_value, theirs_default),
                "{key} 的出厂默认不一致：TOML {:?} / 旧 JSON {theirs_default:?}",
                ours.default_value
            );
            if &ours.default_value != theirs_default {
                repr_only += 1;
            }
            compared += 1;

            for (name, ours_table) in [
                ("machineVariants", &ours.machine_variants),
                ("machineMinVariants", &ours.machine_min_variants),
                ("machineMaxVariants", &ours.machine_max_variants),
            ] {
                let theirs = jp.get(name).and_then(|v| v.as_object());
                let theirs_len = theirs.map_or(0, serde_json::Map::len);
                assert_eq!(ours_table.len(), theirs_len, "{key} 的 {name} 键数不一致");
                for (k, v) in ours_table {
                    let want = theirs
                        .and_then(|m| m.get(k))
                        .unwrap_or_else(|| panic!("{key} 的 {name} 里旧数据没有键 {k}"));
                    assert!(
                        same(v, want),
                        "{key} 的 {name}[{k}] 值不一致：{v:?} / {want:?}"
                    );
                    if v != want {
                        repr_only += 1;
                    }
                    compared += 1;
                }
            }
        }
        assert!(compared > 0, "一个值都没比对到，判据在空转");
        // 反空转的另一半：那 42 条整数/浮点差异**确实存在**。
        // 一天它变成 0，说明比对对象换了（或者两份数据里有一份被谁改过），要重新看这条
        assert!(
            repr_only > 0,
            "一条表示差异都没有 —— 上面那段关于 `0` 与 `0.0` 的说明已经失去对象，该重写了"
        );
    }
}
