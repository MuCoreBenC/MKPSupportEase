//! 只在测试里编译的上游夹具。
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

use std::path::Path;

use crate::workbench::upstream::Upstream;

pub struct Fixture {
    /// 临时目录要活到测试结束，所以持有它
    _dir: tempfile::TempDir,
    pub up: Upstream,
}

impl Fixture {
    pub fn load() -> Self {
        let dir = tempfile::tempdir().unwrap();
        write_all(dir.path());
        let up = Upstream::load_from(dir.path()).expect("夹具本身应该是自洽的");
        Self { _dir: dir, up }
    }
}

fn w(root: &Path, rel: &str, v: &serde_json::Value) {
    crate::fsx::atomic::atomic_write_json(&root.join(rel), v).unwrap();
}

fn write_all(root: &Path) {
    w(root, "content/param_registry.json", &params());
    w(root, "content/layout_schema.json", &layout());
    w(root, "content/machine_catalog.json", &catalog());
    w(root, "manifest.json", &manifest());
    w(root, "content/fallback_registry.json", &fallback());
}

fn params() -> serde_json::Value {
    serde_json::json!({
        "params": [
            {
                "key": "toolhead.offset.x", "configKey": "OffX",
                "tomlKey": "off_x", "jsonKey": "off_x",
                "label": "X 轴偏移", "desc": "喷嘴在 X 上的偏移", "tomlComment": "",
                "valueType": "float", "uiComponent": "number", "defaultValue": 0,
                "scope": "machine_specific", "section": "toolhead",
                "layout": { "order": 1, "sectionId": "space" },
                "unit": "mm",
                // A1 两个版本值不同 → 留覆盖；P1S 唯一版本 → 上提到基底
                "machineVariants": {
                    "A1:STANDARD": -1,
                    "A1:FAST": -0.7,
                    "P1S:LITE": -25.9
                }
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
            {
                "key": "wiping.mode", "configKey": "Mode",
                "tomlKey": "mode", "jsonKey": "mode",
                "label": "擦拭部件", "desc": "", "tomlComment": "",
                "valueType": "string", "uiComponent": "segmented", "defaultValue": "tower",
                "scope": "universal", "section": "wiping",
                "layout": { "order": 4, "sectionId": "wipe" },
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
                "layout": { "order": 5, "sectionId": "wipe" },
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
