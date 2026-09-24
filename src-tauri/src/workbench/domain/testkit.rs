//! 只在测试里编译的夹具：**一份上游 + 一份我们自己的 `presets/`**。
//!
//! 为什么要一份共享夹具：`derive` 与 `preview` 都需要一整套自洽的上游
//! （字段定义 + 布局 + 机型清单 + 资源清单，四份还要过得了那一串一致性断言）。
//! 各自造一份的话，两边的夹具会慢慢分岔，然后"在 derive 里过、在 preview 里不过"
//! 变成一个查不出原因的现象。
//!
//! 夹具刻意**把真上游那几种形状都放了一个**，因为这些正是容易出错的地方：
//!
//! | 形状 | 放在哪 | 想覆盖什么 |
//! |---|---|---|
//! | 版本键值不同 | `toolhead.offset.x` 在 A1 的两个版本上不同 | 归并留覆盖 |
//! | 版本键值相同 | 同一字段在 P1S 的唯一版本上 | 归并上提到基底 |
//! | `machineFilter` | `toolhead.only_p1s` 只给 P1S | 「不适用」 |
//! | `showWhen` | `wiping.child` 吊在 `wiping.mode` 上 | 「看得见改不动」 |
//! | `gcode` | `toolhead.script` | 单元格第二分支 + 拒绝批量 |
//! | 什么都没有 | A2L（无尺寸、无产物、无机型差异） | 「暂无资源」 |
//!
//! # 两份数据，一张清单（b04 Task 8）
//!
//! 机型与版本的**清单**现在来自 `presets/machines/*.toml`（我们自己的数据，可写），
//! 产物与套餐还在上游。两边说的必须是同一批机型 —— 所以 [`Fixture::load`] 末尾有一条
//! **构造时**的断言在盯：手写的上游 JSON 与生成的 presets TOML 一旦分岔就立刻 panic，
//! 而不是等某个测试以"派生结果少一行"的形式失败。
//!
//! `registry/param_registry.toml` 与 `layout_schema.toml` 不手写第二遍：
//! 它们由上游那两份 JSON **序列化成 TOML** 得到（两边键名本来就一样，
//! 见 `presets/registry.rs` 的模块文档）。一份来源两种格式，不会分岔。

use std::path::Path;

use crate::workbench::presets::Presets;
use crate::workbench::upstream::Upstream;

use super::patch::CatalogMachine;

/// 夹具里的一个版本：`(版本 id, 版本名, tag)`
type FixtureVersion = (&'static str, &'static str, &'static str);
/// 夹具里的一台机型：`(机型 id, defaultBundle, 版本列表)`
type FixtureMachine = (&'static str, &'static str, &'static [FixtureVersion]);

/// 夹具的机型清单。
///
/// **一处定义**：presets 夹具的 TOML、以及测试里那份 `Committed` 的清单都从这里长出来
pub const FIXTURE_MACHINES: &[FixtureMachine] = &[
    (
        "A1",
        "A1_default",
        &[("STANDARD", "标准版", "推荐"), ("FAST", "高速版", "热门")],
    ),
    ("A2L", "", &[("STANDARD", "标准版", "")]),
    ("P1S", "", &[("LITE", "精简版", "推荐")]),
];

/// 上面那份清单的 `Committed.catalog` 形态。干净仓库里它就是全部的清单
pub fn fixture_catalog() -> Vec<CatalogMachine> {
    FIXTURE_MACHINES
        .iter()
        .map(|(id, bundle, versions)| CatalogMachine {
            id: (*id).to_owned(),
            display: (*id).to_owned(),
            icon: Some("a1".to_owned()),
            default_bundle: Some((*bundle).to_owned()).filter(|s| !s.is_empty()),
            has_dimensions: has_dimensions(id),
            version_ids: versions.iter().map(|(v, _, _)| (*v).to_owned()).collect(),
        })
        .collect()
}

/// A2L **刻意没有尺寸** —— 真上游就是这样，而「这台还没配尺寸」是界面上要说出来的一档
fn has_dimensions(id: &str) -> bool {
    id != "A2L"
}

pub struct Fixture {
    /// 临时目录要活到测试结束，所以持有它
    _dir: tempfile::TempDir,
    pub up: Upstream,
    /// 我们自己那份数据。**清单的来源**，而且是可写的
    pub presets: Presets,
}

impl Fixture {
    pub fn load() -> Self {
        let dir = tempfile::tempdir().unwrap();
        write_all(dir.path());
        write_presets(&dir.path().join("presets"));
        let up = Upstream::load_from(dir.path()).expect("夹具本身应该是自洽的");
        let presets =
            Presets::load_from(&dir.path().join("presets")).expect("presets 夹具应该读得通");
        check_same_catalog(&up, &presets);
        Self {
            _dir: dir,
            up,
            presets,
        }
    }

    /// 把临时目录连同两份数据一起交出去。
    ///
    /// **要写 presets 的测试必须用这个**：`presets` 会被写回磁盘，
    /// 那个临时目录得活过那次写。直接 `f.presets` 搬走的话目录已经被删了，
    /// 而症状是一条"建不出文件"的错，离原因很远
    pub fn into_parts(self) -> (tempfile::TempDir, Upstream, Presets) {
        (self._dir, self.up, self.presets)
    }
}

/// **构造时**的对齐断言：上游那份手写 JSON 与 presets 夹具说的必须是同一批机型与版本。
///
/// 分岔的后果不会以"夹具不对"的形式出现，而是"派生结果少了一行"或者
/// "产物状态莫名是未生成" —— 那种失败要查很久才回到这里
fn check_same_catalog(up: &Upstream, presets: &Presets) {
    let ours: Vec<(String, Vec<String>)> = presets
        .catalog
        .machines()
        .iter()
        .map(|m| {
            (
                m.id.clone(),
                m.versions.iter().map(|v| v.id.clone()).collect(),
            )
        })
        .collect();
    let theirs: Vec<(String, Vec<String>)> = up
        .catalog
        .machines()
        .iter()
        .map(|m| {
            (
                m.id.clone(),
                m.versions.iter().map(|v| v.id.clone()).collect(),
            )
        })
        .collect();
    assert_eq!(
        ours, theirs,
        "夹具的两份数据分岔了：presets 说 {ours:?}，上游说 {theirs:?}"
    );
    assert_eq!(
        ours.len(),
        FIXTURE_MACHINES.len(),
        "FIXTURE_MACHINES 与实际生成的机型数不一致"
    );
    // 尺寸那一档也要对齐：`dimensions_missing` 现在读 presets，而
    // `issues` 那边还在看上游（尺寸页是后面的任务）。两边说的不一样时，
    // 界面上会出现「这台没尺寸」和「这台有尺寸」同时成立
    for m in presets.catalog.machines() {
        let upstream_has = up
            .catalog
            .machine(&m.id)
            .is_some_and(|x| x.dimensions.is_some());
        assert_eq!(
            m.has_dimensions, upstream_has,
            "{} 的尺寸：presets 说 {}，上游说 {upstream_has}",
            m.id, m.has_dimensions
        );
    }
}

fn w(root: &Path, rel: &str, v: &serde_json::Value) {
    crate::fsx::atomic::atomic_write_json(&root.join(rel), v).unwrap();
}

fn write_all(root: &Path) {
    // 字段定义与布局**只写 TOML**（b04 Task 9：上游那两份 JSON 已经没人读了）；
    // 下面 `write_presets` 会从同一份 JSON 转出来
    w(root, "content/machine_catalog.json", &catalog());
    w(root, "manifest.json", &manifest());
    w(root, "content/fallback_registry.json", &fallback());
}

/// 我们自己那份 `presets/`。
///
/// 机型文件手写（它的形状就是被测对象之一），字段定义与布局**从上游那两份 JSON 转过来** ——
/// 同一份内容手写两遍就是给分岔留门
fn write_presets(root: &Path) {
    let t = |rel: &str, text: String| {
        crate::fsx::atomic::atomic_write(&root.join(rel), text.as_bytes()).unwrap();
    };

    t(
        "brands.toml",
        "[[brands]]\nid = 'Bambu Lab'\nname = '拓竹 (Bambu Lab)'\nlogo = 'bambu-logo.png'\n"
            .to_owned(),
    );
    // 资产定义（b05 Task 8）：两条 —— 一条图片（归 A1）、一条切片器预设（归 P1S），
    // 两种类型都走到。`path` 指向夹具资产根里**不存在**的文件是合法的：
    // 存在性不是加载期的事（条目与文件一起在 Task 9 落地）
    t(
        "assets.toml",
        "[[assets]]\n\
         id = 'a1-image'\n\
         type = 'image'\n\
         machineId = 'A1'\n\
         name = 'A1 外观图'\n\
         path = 'printers/a1.webp'\n\
         \n\
         [[assets]]\n\
         id = 'a1-icon'\n\
         type = 'icon'\n\
         machineId = 'A1'\n\
         name = 'A1 图标'\n\
         path = 'icons/a1.svg'\n\
         \n\
         [[assets]]\n\
         id = 'p1s-icon'\n\
         type = 'icon'\n\
         machineId = 'P1S'\n\
         name = 'P1S 图标'\n\
         path = 'icons/p1s.svg'\n\
         \n\
         [[assets]]\n\
         id = 'a1-extra-image'\n\
         type = 'image'\n\
         machineId = 'A1'\n\
         name = 'A1 备选图（没人引用）'\n\
         path = 'printers/a1-extra.webp'\n\
         \n\
         [[assets]]\n\
         id = 'a1-bbs-04-020'\n\
         type = 'slicerProfile'\n\
         machineId = 'A1'\n\
         name = 'A1 0.4 喷头 0.20 层高'\n\
         path = 'bbs/A1/process.json'\n\
         slicer = 'bbs'\n\
         profile = 'process'\n\
         \n\
         [[assets]]\n\
         id = 'p1s-bbs-02-010'\n\
         type = 'slicerProfile'\n\
         machineId = 'P1S'\n\
         name = 'P1S 0.2 喷头 0.10 层高'\n\
         path = 'bbs/P1S/process.json'\n\
         slicer = 'bbs'\n\
         profile = 'process'\n"
            .to_owned(),
    );
    // 套餐定义（b05 Task 10）：一条，被夹具里 A1 的 `defaultBundle` 与
    // 两个版本的 `recommendedBundle` 引用着。`assetRefs` 里的 BBS 是 10.8 那条
    // 「成套配发」判据要用的形状；p1s-bbs-02-010 刻意**没有**套餐引用 ——
    // 「没人引用的 BBS 删得掉」要用它
    t(
        "bundles.toml",
        "[[bundles]]\n\
         id = 'A1_default'\n\
         display = '官方推荐'\n\
         machineId = 'A1'\n\
         assetRefs = ['a1-bbs-04-020']\n\
         updatedAt = '2026-07-12'\n"
            .to_owned(),
    );
    for (id, bundle, versions) in FIXTURE_MACHINES {
        let mut s = String::new();
        s.push_str(&format!("id = '{id}'\n"));
        s.push_str(&format!("display = '{id}'\n"));
        s.push_str("brand = 'Bambu Lab'\n");
        // 图标是**资产 id**（b05 Task 9 改的引用形式）：P1S 用自己的那份，
        // 其余借用 a1 那份 —— 与真数据里「P2S / X1C 借 p1s-icon」同一形状
        s.push_str(if *id == "P1S" {
            "icon = 'p1s-icon'\n"
        } else {
            "icon = 'a1-icon'\n"
        });
        // A1 有一张机型图（`a1-image` 指着它）—— 反查与删除守卫那条判据要用；
        // 其余机型不给图，与真数据里 A2L 没有图同一形状
        if *id == "A1" {
            s.push_str("image = 'a1-image'\n");
        }
        if !bundle.is_empty() {
            s.push_str(&format!("defaultBundle = '{bundle}'\n"));
        }
        if *id == "A1" {
            s.push_str("externalAliases = ['A1C']\n");
        }
        // 这一层只看「有没有 `[dimensions]`」（`presets/catalog.rs` 没给尺寸建细模型），
        // 所以放一格就够；A2L 刻意不放
        if has_dimensions(id) {
            s.push_str("\n[dimensions]\nbedSize = { width = 256, depth = 256 }\n");
        }
        for (vid, name, tag) in *versions {
            s.push_str(&format!("\n[[versions]]\nid = '{vid}'\nname = '{name}'\n"));
            // 空 tag **不写这一行**（写成 '' 会读成「填过，填了个空」）
            if !tag.is_empty() {
                s.push_str(&format!("tag = '{tag}'\n"));
            }
            // A1 的每一版都推荐同一条套餐 —— 与真数据同形状（b05 Task 10），
            // `check_bundle_refs` 的机型侧检查靠它有东西可查
            if *id == "A1" {
                s.push_str("recommendedBundle = 'A1_default'\n");
            }
        }
        t(&format!("machines/{id}.toml"), s);
    }

    t("registry/param_registry.toml", as_toml(&params()));
    t("layout_schema.toml", as_toml(&layout()));
}

/// 把一份 JSON 原样转成 TOML。两边的键名一样（camelCase），所以这是纯格式转换
fn as_toml(v: &serde_json::Value) -> String {
    toml::to_string(v).expect("夹具那两份 JSON 应该都能表示成 TOML")
}

fn params() -> serde_json::Value {
    serde_json::json!({
        "params": [
            {
                "key": "toolhead.offset.x", "configKey": "XOffset",
                "tomlKey": "offset", "jsonKey": "x",
                "label": "X 轴偏移", "desc": "喷嘴在 X 上的偏移", "tomlComment": "笔尖偏移",
                "valueType": "float", "uiComponent": "number", "defaultValue": 0,
                "scope": "machine_specific", "section": "toolhead",
                "layout": { "order": 1, "sectionId": "space" },
                "unit": "mm", "min": -50, "max": 50, "step": 0.05,
                // A1 两个版本值不同 → 留覆盖；P1S 唯一版本 → 上提到基底
                "machineVariants": {
                    "A1:STANDARD": -1,
                    "A1:FAST": -0.7,
                    "P1S:LITE": -25.9
                }
            },
            // y 与 z 与 x **共享 tomlKey**：渲染时要合成内联表
            // `offset = { x = …, y = …, z = … }`，成员名取 jsonKey。
            // 这是上游真实的形状（toolhead.offset.x/y/z 的 tomlKey 都是 offset）
            {
                "key": "toolhead.offset.y", "configKey": "YOffset",
                "tomlKey": "offset", "jsonKey": "y",
                "label": "Y 轴偏移", "desc": "", "tomlComment": "笔尖偏移",
                "valueType": "float", "uiComponent": "number", "defaultValue": 18.6,
                "scope": "machine_specific", "section": "toolhead",
                "layout": { "order": 1.1, "sectionId": "space" },
                "unit": "mm", "min": -50, "max": 50, "step": 0.05
            },
            {
                "key": "toolhead.offset.z", "configKey": "ZOffset",
                "tomlKey": "offset", "jsonKey": "z",
                "label": "Z 轴偏移", "desc": "", "tomlComment": "笔尖偏移",
                "valueType": "float", "uiComponent": "number", "defaultValue": 4,
                "scope": "machine_specific", "section": "toolhead",
                "layout": { "order": 1.2, "sectionId": "space" },
                "unit": "mm", "min": 0, "max": 20, "step": 0.05
            },
            {
                "key": "toolhead.only_p1s", "configKey": "OnlyP1S",
                "tomlKey": "only_p1s", "jsonKey": "only_p1s",
                "label": "只有 P1S 有的项", "desc": "", "tomlComment": "",
                "valueType": "float", "uiComponent": "number", "defaultValue": 9,
                "scope": "machine_specific", "section": "toolhead",
                "layout": { "order": 2, "sectionId": "space" },
                "machineFilter": "P1S"
            },
            {
                "key": "toolhead.script", "configKey": "Script",
                "tomlKey": "script", "jsonKey": "script",
                "label": "装载 G-code", "desc": "", "tomlComment": "",
                "valueType": "string", "uiComponent": "gcode", "defaultValue": "",
                "scope": "universal", "section": "toolhead",
                "layout": { "order": 3, "sectionId": "space" }
            },
            // 下面两条的 `layout.order` **刻意和 space 组的号段重叠**（1.05 / 1.5 落在
            // offset.x 的 1 与 only_p1s 的 2 之间）。上游真实数据就是这样：order 是
            // section **内部**的序号，跨组必然撞号。拿它当全局键排序就会把两组洗成一团 ——
            // `rows_are_grouped_by_section_not_interleaved` 靠这个形状才测得到东西
            {
                "key": "wiping.mode", "configKey": "Mode",
                "tomlKey": "mode", "jsonKey": "mode",
                "label": "擦拭部件", "desc": "", "tomlComment": "",
                "valueType": "string", "uiComponent": "segmented", "defaultValue": "tower",
                "scope": "universal", "section": "wiping",
                "layout": { "order": 1.05, "sectionId": "wipe" },
                "choices": [
                    { "label": "擦料塔", "value": "tower" },
                    { "label": "圆盘擦拭", "value": "disk" }
                ]
            },
            {
                "key": "wiping.child", "configKey": "Child",
                "tomlKey": "child", "jsonKey": "child",
                "label": "塔位置 X", "desc": "", "tomlComment": "",
                "valueType": "float", "uiComponent": "number", "defaultValue": 20,
                "scope": "universal", "section": "wiping",
                "layout": { "order": 1.5, "sectionId": "wipe" },
                "parentKey": "wiping.mode",
                "showWhen": { "key": "wiping.mode", "op": "eq", "value": "tower" }
            }
        ],
        "tabs": [
            { "id": "offset", "label": "偏移", "order": 10, "icon": "wrench",
              "sections": [{ "id": "space", "label": "空间偏移", "order": 100 }] },
            { "id": "wiping", "label": "擦料", "order": 20, "icon": "layers",
              "sections": [{ "id": "wipe", "label": "擦料方式", "order": 0 }] }
        ],
        "updated": "2026-01-01 00:00:00"
    })
}

fn layout() -> serde_json::Value {
    serde_json::json!({
        "tabs": [
            { "id": "offset", "sections": [{ "id": "space", "items": [
                { "id": "i0", "paramKey": "toolhead.offset.x" },
                { "id": "i0y", "paramKey": "toolhead.offset.y" },
                { "id": "i0z", "paramKey": "toolhead.offset.z" },
                { "id": "i1", "paramKey": "toolhead.only_p1s" },
                { "id": "i2", "paramKey": "toolhead.script" }
            ] }] },
            { "id": "wiping", "sections": [{ "id": "wipe", "items": [
                { "id": "i3", "paramKey": "wiping.mode" },
                { "id": "i4", "paramKey": "wiping.child" }
            ] }] }
        ]
    })
}

fn dims() -> serde_json::Value {
    serde_json::json!({
        "bedSize": { "width": 256, "depth": 256 },
        "movementRange": { "minX": -40, "maxX": 260, "minY": 0, "maxY": 255, "maxZ": 999 },
        "glueArea": { "glueMinX": 0, "glueMaxX": 255, "glueMinY": 0, "glueMaxY": 265, "wipeX": 20 },
        "calibration": {
            "lShapeBaseX": 68.21, "lShapeBaseY": 126.373,
            "xLineX": 114.523, "xLineY": 104.83, "xLineYEnd": 114.83,
            "yLineX": 104.53, "yLineXEnd": 114.53, "yLineY": 112.83,
            "zStartX": 68.21, "zStartY": 126.373
        },
        "flags": { "gcodeMarker": ";===== machine: X", "hasSecondFan": false },
        "edgeZone": 10
    })
}

fn catalog() -> serde_json::Value {
    serde_json::json!({
        "brands": [{ "id": "Bambu Lab", "name": "拓竹 (Bambu Lab)", "logo": "bambu-logo.png" }],
        "models": { "Bambu Lab": [
            { "id": "A1", "externalAliases": ["A1C"],
              "versions": [{ "id": "STANDARD" }, { "id": "FAST" }] },
            { "id": "A2L", "externalAliases": [], "versions": [{ "id": "STANDARD" }] },
            { "id": "P1S", "externalAliases": ["P1"], "versions": [{ "id": "LITE" }] }
        ] },
        "dimensions": { "A1": dims(), "P1S": dims() },
        "forbiddenZones": {}
    })
}

fn asset(id: &str, rtype: &str, machine: &str, file: &str, sha: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id, "resourceType": rtype, "category": "",
        "machineId": machine, "fileName": file,
        "relativePath": format!("presets/mkp/{file}"),
        "sha256": sha, "size": 4299, "updatedAt": "2026-01-01T00:00:00Z"
    })
}

fn manifest() -> serde_json::Value {
    serde_json::json!({
        "manifestVersion": 2,
        "version": "",
        "minimumClient": "",
        "channel": "stable",
        "updated": "2026-01-01T00:00:00Z",
        "forceUpdate": false,
        "assets": [
            asset("a1_mkp_standard", "mkp_preset", "A1", "A1.toml", "aaa"),
            asset("a1_mkp_fast", "mkp_preset", "A1", "A1F.toml", "bbb"),
            asset("p1s_mkp_lite", "mkp_preset", "P1S", "P1.toml", "ccc"),
            {
                "id": "a1_bbs_04", "resourceType": "bbs_profile", "category": "process",
                "machineId": "A1", "fileName": "MKPProcess A1 0.4 0.20.json",
                "relativePath": "presets/bbs/Process/0.4mm/MKPProcess A1 0.4 0.20.json",
                "sha256": "ddd", "size": 1332, "updatedAt": "2026-01-01T00:00:00Z"
            },
            {
                "id": "img_a1", "resourceType": "image", "category": "machines",
                "machineId": "", "fileName": "a1.webp",
                "relativePath": "assets/machines/a1.webp",
                "sha256": "eee", "size": 21740, "updatedAt": "2026-01-01T00:00:00Z"
            }
        ],
        "bundles": [{
            "id": "A1_default", "display": "官方推荐", "machineId": "A1",
            "assetRefs": ["a1_bbs_04"]
        }],
        "machines": {
            "A1": {
                "id": "A1", "display": "A1", "brand": "Bambu Lab",
                "icon": "a1", "image": "a1.webp", "defaultBundle": "A1_default",
                "dimensions": dims(),
                "versions": [
                    { "id": "STANDARD", "name": "标准版", "machineKey": "A1:STANDARD",
                      "tag": "推荐", "description": "官方标准配置",
                      "mkpPresetAssetId": "a1_mkp_standard", "recommendedBundle": "A1_default" },
                    { "id": "FAST", "name": "高速版", "machineKey": "A1:FAST",
                      "tag": "热门", "description": "",
                      "mkpPresetAssetId": "a1_mkp_fast", "recommendedBundle": "A1_default" }
                ]
            },
            // A2L：四处皆空 —— 无尺寸、无产物、无套餐、无机型差异
            "A2L": {
                "id": "A2L", "display": "A2L", "brand": "Bambu Lab",
                "icon": "a1", "image": "", "defaultBundle": "",
                "dimensions": null,
                "versions": [{
                    "id": "STANDARD", "name": "标准版", "machineKey": "A2L:STANDARD",
                    "tag": "", "description": "",
                    "mkpPresetAssetId": "", "recommendedBundle": ""
                }]
            },
            "P1S": {
                "id": "P1S", "display": "P1S", "brand": "Bambu Lab",
                "icon": "p1s", "image": "p1s.webp", "defaultBundle": "",
                "dimensions": dims(),
                "versions": [{
                    "id": "LITE", "name": "精简版", "machineKey": "P1S:LITE",
                    "tag": "推荐", "description": "",
                    "mkpPresetAssetId": "p1s_mkp_lite", "recommendedBundle": ""
                }]
            }
        },
        "tracks": [{ "baseVersion": "0.0.4", "channel": "stable", "latest": "0.0.4" }],
        "contentFiles": []
    })
}

fn fallback() -> serde_json::Value {
    serde_json::json!({
        "version": 1,
        "updated": "2026-01-01 00:00:00",
        "guide": "# 这是什么？\n测试夹具。\n\n# 怎么改？\n禁止手改 content/*.json。\n",
        "fallbacks": [{
            "id": "support_fallback_to_tower", "category": "override",
            "trigger": "仅有支撑面无支撑体", "from": "disk", "to": "tower",
            "enabled": true, "severity": "info",
            "desc": "强制改为擦料塔，保证支撑面能涂胶。", "reportField": "supportFallback"
        }]
    })
}
