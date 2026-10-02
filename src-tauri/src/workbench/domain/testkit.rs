//! 只在测试里编译的夹具：**一份我们自己的 `presets/`**。
//!
//! 为什么要一份共享夹具：`derive` 与 `preview` 都需要一整套自洽的预设数据
//! （字段定义 + 布局 + 机型清单 + 资源 + 套餐）。各自造一份的话，两边的夹具会慢慢分岔，
//! 然后"在 derive 里过、在 preview 里不过"变成一个查不出原因的现象。
//!
//! 夹具刻意**把真数据那几种形状都放了一个**，因为这些正是容易出错的地方：
//!
//! | 形状 | 放在哪 | 想覆盖什么 |
//! |---|---|---|
//! | 版本键值不同 | `toolhead.offset.x` 在 A1 的两个版本上不同 | 归并留覆盖 |
//! | 版本键值相同 | 同一字段在 P1S 的唯一版本上 | 归并上提到基底 |
//! | `machineFilter` | `toolhead.only_p1s` 只给 P1S | 「不适用」 |
//! | `showWhen` | `wiping.child` 吊在 `wiping.mode` 上 | 「看得见改不动」 |
//! | `gcode` | `toolhead.script` | 单元格第二分支 + 拒绝批量 |
//! | 什么都没有 | A2L（无尺寸、无配方、无套餐） | 「暂无资源」 |
//!
//! # 只有一个预设数据根
//!
//! 机型与版本的**清单**来自 `presets/machines/*.toml`，字段定义与布局来自
//! `presets/registry/param_registry.toml` 与 `presets/layout_schema.toml` ——
//! 全都在 `<root>/presets/` 这一棵树里（**唯一的预设真相源**，旧的上游 JSON 已退休）。

use std::path::Path;

use crate::workbench::presets::Presets;

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
    /// 我们那份数据。**唯一的数据源**，而且是可写的
    pub presets: Presets,
}

impl Fixture {
    pub fn load() -> Self {
        let dir = tempfile::tempdir().unwrap();
        write_presets(&dir.path().join("presets"));
        let presets =
            Presets::load_from(&dir.path().join("presets")).expect("presets 夹具应该读得通");
        assert_eq!(
            presets.catalog.machines().len(),
            FIXTURE_MACHINES.len(),
            "FIXTURE_MACHINES 与实际生成的机型数不一致"
        );
        Self { _dir: dir, presets }
    }

    /// 把临时目录连同数据一起交出去。
    ///
    /// **要写 presets 的测试必须用这个**：`presets` 会被写回磁盘，
    /// 那个临时目录得活过那次写。直接 `f.presets` 搬走的话目录已经被删了，
    /// 而症状是一条"建不出文件"的错，离原因很远
    pub fn into_parts(self) -> (tempfile::TempDir, Presets) {
        (self._dir, self.presets)
    }
}

/// 我们自己那份 `presets/`。
///
/// 机型文件手写（它的形状就是被测对象之一），字段定义与布局由下面那两份 JSON 序列化成 TOML
/// —— 同一份内容手写两遍就是给分岔留门
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
        // 夹具写的 `[dimensions]` 要与真数据**同形状**（`presets/machines/A1.toml`）：
        // 解析层现在逐格读它（`load_dimensions`），少一格就整台机型读不出来。
        // A2L 刻意不放 —— 那是「占位机型整台跳过」的判据要用的。
        if has_dimensions(id) {
            s.push_str(
                "\n[dimensions]\nedgeZone = 10.0\n\
                 \n[dimensions.bedSize]\nwidth = 256.0\ndepth = 256.0\n\
                 \n[dimensions.movementRange]\nminX = -10.0\nmaxX = 256.0\nminY = 0.0\nmaxY = 256.0\nmaxZ = 256.0\n\
                 \n[dimensions.glueArea]\nglueMinX = 0.0\nglueMaxX = 256.0\nglueMinY = 0.0\nglueMaxY = 256.0\nwipeX = 250.0\n\
                 \n[dimensions.calibration]\nlShapeBaseX = 68.21\nlShapeBaseY = 126.373\nxLineX = 114.523\nxLineY = 104.83\nxLineYEnd = 114.83\nyLineX = 104.53\nyLineXEnd = 114.53\nyLineY = 114.83\nzStartX = 68.21\nzStartY = 126.373\n\
                 \n[dimensions.flags]\ngcodeMarker = ';===== machine: TEST'\nhasSecondFan = false\n",
            );
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
                // 三轴偏移：**各自一个独立 tomlKey**（2026-10-02 从共享 `offset` 内联表拆出来）。
                // 上游真实形状已经是一项一字段，这里跟着改 —— 夹具跟注册表同形。
                "key": "toolhead.offset.x", "configKey": "XOffset",
                "tomlKey": "offset_x", "jsonKey": "offset_x",
                "label": "X 轴偏移", "desc": "喷嘴在 X 上的偏移", "tomlComment": "笔尖偏移",
                "valueType": "float", "uiComponent": "number", "defaultValue": 0.0,
                "scope": "machine_specific", "section": "toolhead",
                "layout": { "order": 1, "sectionId": "space" },
                "unit": "mm", "min": -50, "max": 50, "step": 0.05,
                // A1 两个版本值不同 → 留覆盖；P1S 唯一版本 → 上提到基底
                "machineVariants": {
                    "A1:STANDARD": -1.0,
                    "A1:FAST": -0.7,
                    "P1S:LITE": -25.9
                }
            },
            {
                "key": "toolhead.offset.y", "configKey": "YOffset",
                "tomlKey": "offset_y", "jsonKey": "offset_y",
                "label": "Y 轴偏移", "desc": "", "tomlComment": "笔尖偏移",
                "valueType": "float", "uiComponent": "number", "defaultValue": 18.6,
                "scope": "machine_specific", "section": "toolhead",
                "layout": { "order": 1.1, "sectionId": "space" },
                "unit": "mm", "min": -50, "max": 50, "step": 0.05
            },
            {
                "key": "toolhead.offset.z", "configKey": "ZOffset",
                "tomlKey": "offset_z", "jsonKey": "offset_z",
                "label": "Z 轴偏移", "desc": "", "tomlComment": "笔尖偏移",
                "valueType": "float", "uiComponent": "number", "defaultValue": 4.0,
                "scope": "machine_specific", "section": "toolhead",
                "layout": { "order": 1.2, "sectionId": "space" },
                "unit": "mm", "min": 0, "max": 20, "step": 0.05
            },
            {
                "key": "toolhead.only_p1s", "configKey": "OnlyP1S",
                "tomlKey": "only_p1s", "jsonKey": "only_p1s",
                "label": "只有 P1S 有的项", "desc": "", "tomlComment": "",
                "valueType": "float", "uiComponent": "number", "defaultValue": 9.0,
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
                "valueType": "float", "uiComponent": "number", "defaultValue": 20.0,
                "scope": "universal", "section": "wiping",
                "layout": { "order": 1.5, "sectionId": "wipe" },
                "parentKey": "wiping.mode",
                "showWhen": { "key": "wiping.mode", "op": "eq", "value": "tower" }
            },
            // 弃用参数（C14 §五）：参数台要**看得见**（划线 + 禁用），生成侧
            // （`visible_keys`）照旧不带走它 —— 两边各一条判据，这一条夹具
            // 让两边都能测
            {
                "key": "wiping.legacy", "configKey": "Legacy",
                "tomlKey": "legacy", "jsonKey": "legacy",
                "label": "旧版擦料计数", "desc": "", "tomlComment": "",
                "valueType": "float", "uiComponent": "number", "defaultValue": 7.0,
                "scope": "universal", "section": "wiping",
                "layout": { "order": 1.6, "sectionId": "wipe" },
                "deprecated": true
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
                { "id": "i4", "paramKey": "wiping.child" },
                { "id": "i5", "paramKey": "wiping.legacy" }
            ] }] }
        ]
    })
}
