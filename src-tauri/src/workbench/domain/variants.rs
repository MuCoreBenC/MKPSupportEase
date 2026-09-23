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

use crate::workbench::upstream::catalog::Machine;
use crate::workbench::upstream::Registry;

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

/// 把一台机型的 `machineVariants` 消化成机型基底 + 版本覆盖。
///
/// 纯函数：同样的输入永远得到同样的输出，不碰文件、不看时间
pub fn digest(registry: &Registry, machine: &Machine) -> Digested {
    let mut out = Digested {
        base: Overrides::new(),
        versions: machine
            .versions
            .iter()
            .map(|v| (v.id.clone(), Overrides::new()))
            .collect(),
    };

    for key in registry.visible_keys(&machine.id) {
        let Some(param) = registry.param(key) else {
            continue;
        };
        let table = &param.machine_variants;
        if table.is_empty() {
            continue; // 第 3 步：靠继承
        }

        // 第 1 步：纯机型键
        if let Some(v) = table.get(&machine.id) {
            out.base.insert(key.to_owned(), v.clone());
        }

        // 第 2 步：版本键
        let per_version: Vec<Option<&Value>> = machine
            .versions
            .iter()
            .map(|v| table.get(&v.machine_key))
            .collect();

        let first = per_version.first().copied().flatten();
        let unanimous = !machine.versions.is_empty()
            && per_version.iter().all(|v| v.is_some())
            && per_version.iter().all(|v| *v == first);

        if unanimous {
            // 上提。**刻意允许它盖掉第 1 步写下的纯机型值**：
            // 每个版本都有更具体的键，那个纯机型值本来就到不了任何版本，
            // 留着它反而会让基底显示一个没人在用的数
            if let Some(v) = first {
                out.base.insert(key.to_owned(), v.clone());
            }
        } else {
            for (ver, value) in machine.versions.iter().zip(per_version) {
                if let Some(v) = value {
                    out.versions
                        .entry(ver.id.clone())
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
    use crate::workbench::upstream::catalog::Catalog;
    use crate::workbench::upstream::Manifest;
    use crate::workbench::paths;

    /// 造一台机型 + 一份字段定义。两边都走真的反序列化路径，
    /// 免得夹具和真数据在字段名上悄悄分岔
    fn fixture(versions: &[&str], variants: serde_json::Value) -> (Registry, Machine) {
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
        let vers: Vec<serde_json::Value> = versions
            .iter()
            .map(|v| serde_json::json!({ "id": v }))
            .collect();
        let mvers: Vec<serde_json::Value> = versions
            .iter()
            .map(|v| {
                serde_json::json!({
                    "id": v, "name": v, "machineKey": format!("A1:{v}"),
                    "tag": "", "description": "",
                    "mkpPresetAssetId": "", "recommendedBundle": ""
                })
            })
            .collect();
        let catalog = serde_json::json!({
            "brands": [{ "id": "Bambu Lab", "name": "拓竹", "logo": "l.png" }],
            "models": { "Bambu Lab": [{ "id": "A1", "externalAliases": [], "versions": vers }] },
            "dimensions": {}, "forbiddenZones": {}
        });
        let manifest = serde_json::json!({
            "manifestVersion": 2, "channel": "stable", "updated": "2026-01-01T00:00:00Z",
            "assets": [], "bundles": [],
            "machines": { "A1": {
                "id": "A1", "display": "A1", "brand": "Bambu Lab",
                "icon": "a1", "image": "", "defaultBundle": "",
                "dimensions": null, "versions": mvers
            } },
            "tracks": [], "contentFiles": []
        });

        let w = |rel: &str, v: &serde_json::Value| {
            crate::fsx::atomic::atomic_write_json(&root.join(rel), v).unwrap()
        };
        w("content/param_registry.json", &params);
        w("content/layout_schema.json", &layout);
        w("content/machine_catalog.json", &catalog);
        w("manifest.json", &manifest);

        let reg = Registry::load_from(root).unwrap();
        let man = Manifest::load_from(root).unwrap();
        let cat = Catalog::load_from(root, &man).unwrap();
        let machine = cat.machine("A1").unwrap().clone();
        (reg, machine)
    }

    /// 三个版本值都一样 → **上提到基底，版本上一条覆盖都不留**
    #[test]
    fn unanimous_version_keys_are_promoted_to_base() {
        let (reg, m) = fixture(
            &["STANDARD", "FAST", "FASTV3.3"],
            serde_json::json!({ "A1:STANDARD": -1, "A1:FAST": -1, "A1:FASTV3.3": -1 }),
        );
        let d = digest(&reg, &m);
        assert_eq!(d.base["toolhead.offset.x"], serde_json::json!(-1));
        assert_eq!(d.override_count(), 0, "全体一致就不该留覆盖");
    }

    /// 值不一致 → 各留覆盖，基底不写。
    /// **不做"取多数当基底"** —— 那要替人挑一个标准版本
    #[test]
    fn differing_version_keys_stay_as_overrides() {
        let (reg, m) = fixture(
            &["STANDARD", "FAST", "FASTV3.3"],
            serde_json::json!({ "A1:STANDARD": -1, "A1:FAST": -0.7, "A1:FASTV3.3": 0.1 }),
        );
        let d = digest(&reg, &m);
        assert!(!d.base.contains_key("toolhead.offset.x"), "基底不该猜一个值");
        assert_eq!(d.override_count(), 3);
        assert_eq!(d.versions["FAST"]["toolhead.offset.x"], serde_json::json!(-0.7));
    }

    /// **少一个版本就不算一致**：两个版本写了同一个值、第三个没写，
    /// 上提会让第三个版本从"继承出厂默认"变成"继承那个值" —— 那是改了它的有效值
    #[test]
    fn partial_coverage_is_not_unanimous_even_if_values_agree() {
        let (reg, m) = fixture(
            &["STANDARD", "FAST", "FASTV3.3"],
            serde_json::json!({ "A1:STANDARD": -1, "A1:FAST": -1 }),
        );
        let d = digest(&reg, &m);
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
        let (reg, m) = fixture(&["STANDARD", "FAST"], serde_json::json!({ "A1": 1.1 }));
        let d = digest(&reg, &m);
        assert_eq!(d.base["toolhead.offset.x"], serde_json::json!(1.1));
        assert_eq!(d.override_count(), 0);
    }

    /// 两种键**同时存在且值相同** —— 这是 `toolhead.offset.z` 在 P1S/P2S/X1C 上的真实形状。
    /// 结果只有基底一项，不该重复记两处
    #[test]
    fn both_key_forms_with_the_same_value_collapse_to_one_base_entry() {
        let (reg, m) = fixture(
            &["STANDARD"],
            serde_json::json!({ "A1": 1.1, "A1:STANDARD": 1.1 }),
        );
        let d = digest(&reg, &m);
        assert_eq!(d.base["toolhead.offset.x"], serde_json::json!(1.1));
        assert_eq!(d.base.len(), 1);
        assert_eq!(d.override_count(), 0);
    }

    /// 两种键同时存在但**值不同**：版本键更具体，赢。
    /// 上游现在没有这种数据（实测两边都是 1.1），所以这条钉的是"万一出现时怎么办"，
    /// 而不是在描述现状
    #[test]
    fn version_key_wins_over_pure_machine_key_when_they_disagree() {
        let (reg, m) = fixture(
            &["STANDARD"],
            serde_json::json!({ "A1": 4, "A1:STANDARD": 1.1 }),
        );
        let d = digest(&reg, &m);
        assert_eq!(
            d.base["toolhead.offset.x"],
            serde_json::json!(1.1),
            "那个纯机型值到不了任何版本，留着只会在基底上显示一个没人用的数"
        );
    }

    /// 没有这张表的字段什么都不写 —— 靠继承（69/74 都是这种）
    #[test]
    fn params_without_the_table_write_nothing() {
        let (reg, m) = fixture(&["STANDARD"], serde_json::json!({}));
        let d = digest(&reg, &m);
        assert!(d.base.is_empty());
        assert_eq!(d.override_count(), 0);
        // 但版本项要在，调用方才好遍历
        assert!(d.versions.contains_key("STANDARD"));
    }

    /// 键指向别的机型时这台机型什么都不拿到 —— A2L 就是这样（0 基底 0 覆盖）
    #[test]
    fn keys_for_other_machines_are_ignored() {
        let (reg, m) = fixture(
            &["STANDARD"],
            serde_json::json!({ "P1S": 1.1, "P1S:LITE": 1.1 }),
        );
        let d = digest(&reg, &m);
        assert!(d.base.is_empty(), "这是 A2L 那一栏为什么是 0/0");
        assert_eq!(d.override_count(), 0);
    }

    /* ---------- 真上游 ---------- */

    /// **无损**：对每个 `(机型, 版本, key)`，归并后按三层解析出来的值
    /// 必须等于直接查 `machineVariants` 得到的值。
    ///
    /// 这是这套归并唯一不可让的性质 —— 它不成立时用户会看到某台机器的偏移悄悄变了，
    /// 而没有任何一步会报错
    #[test]
    fn merging_is_lossless_on_real_upstream() {
        let Some(root) = paths::upstream_root() else {
            eprintln!("没定位到上游 mkpse-presets，这条对齐检查未执行（不是通过）");
            return;
        };
        let up = crate::workbench::upstream::Upstream::load_from(&root).unwrap();

        let mut checked = 0usize;
        for m in up.catalog.machines() {
            let d = digest(&up.registry, m);
            for v in &m.versions {
                let over = &d.versions[&v.id];
                    // 归并结果进的是「上游给的那一半」（doc §3.6）—— 它不落盘
                    let empty = Overrides::new();
                    let layers =
                        Layers::new(&up.registry, &m.id, &d.base, &empty, over, &empty);
                for key in up.registry.visible_keys(&m.id) {
                    let param = up.registry.param(key).unwrap();
                    // 归并前这个 (机型, 版本, key) 本来该是什么值
                    let want = param
                        .machine_variants
                        .get(&v.machine_key)
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
    /// **这一条会随上游更新而红**，而且那不算 bug：它钉的是「我对当前上游数据算出来的
    /// 口径」，上游改了 `machineVariants` 它就该红。红了之后要做的是重算那张表、
    /// 更新 doc，而不是把断言删掉。
    ///
    /// 为什么值得留一条会腐烂的判据：无损**不足以**说明归并是对的 ——
    /// 把所有值塞进版本覆盖、基底留空，同样无损。上提这一步只有靠口径才看得出来。
    #[test]
    fn real_upstream_matches_the_documented_split() {
        let Some(root) = paths::upstream_root() else {
            eprintln!("没定位到上游 mkpse-presets，这条对齐检查未执行（不是通过）");
            return;
        };
        let up = crate::workbench::upstream::Upstream::load_from(&root).unwrap();

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
        for m in up.catalog.machines() {
            let d = digest(&up.registry, m);
            let got = (d.base.len(), d.override_count());
            total.0 += got.0;
            total.1 += got.1;
            if let Some(w) = want.get(m.id.as_str()) {
                assert_eq!(
                    got, *w,
                    "{} 的基底/覆盖口径与 doc §3.3 那张表不一致（表里是 {w:?}）—— \
                     上游改过就重算那张表，别删这条断言",
                    m.id
                );
            }
        }
        assert_eq!(total, (18, 15), "合计与 doc §3.3 不一致");

        // A1 基底里那一项的身份也钉住：表里写的是 custom_mount_gcode
        let a1 = digest(&up.registry, up.catalog.machine("A1").unwrap());
        assert!(
            a1.base.contains_key("toolhead.custom_mount_gcode"),
            "A1 基底那一项应该是 custom_mount_gcode，实际是 {:?}",
            a1.base.keys().collect::<Vec<_>>()
        );

        // A2L 的 0/0 单独说一句：它**不是**"读失败了"，是上游没给它写任何机型差异
        let a2l = digest(&up.registry, up.catalog.machine("A2L").unwrap());
        assert!(a2l.base.is_empty() && a2l.override_count() == 0);
        assert_eq!(a2l.versions.len(), 1, "它有一个版本，只是没有机型差异");

        // 顺带验一件与 A2L 有关的事：参数**不缺**，全部落到出厂默认
        let empty = Overrides::new();
        let layers = Layers::without_upstream(&up.registry, "A2L", &empty, &empty);
        let keys = up.registry.visible_keys("A2L");
        assert!(!keys.is_empty(), "A2L 一个参数都看不到就说明过滤过头了");
        for key in keys {
            let r = layers.effective(key).expect("A2L 的参数应该落到出厂默认，不是没有");
            assert_eq!(r.origin, Origin::Factory);
        }
    }
}
