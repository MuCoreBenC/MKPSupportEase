//! M3 的判据：`presets/` 是尺寸 / 别名 / 禁区的**唯一来源**。
//!
//! ## 这条判据在守什么
//!
//! 搬进来的时候（M2a/M2b）这三样数据还躺在内核自己的 `assets/*.json` 里 ——
//! 那是源仓库的**编译期快照**。在那个仓库里它成立：机型集是构建期常量，换机型集 = 出新版本。
//! 在我们这里不成立：`presets/machines/*.toml` 是**可编辑的唯一真源**，工作台就靠改它交付。
//! 两份并存的话，改完尺寸再生成，产物用的还是旧几何 —— **而且不会报错**。
//!
//! 所以 M3 把数据源换成 `presets/`，旧快照**降级成基线**（搬到
//! `tests/reference/legacy_snapshot/`）。它现在的唯一用途就是这条判据：
//! **证明 presets 能逐字段复现它**。
//!
//! ## 为什么非要跟旧快照比，而不是只证明"读出来非空"
//!
//! 那 125 个值里有一批是**真机验证过的**（校准坐标、涂胶区、行程、边距），
//! 内核的全部 golden 判据都建立在这些值上。只证明"能从 presets 读出一张表"是不够的 ——
//! 读出来的值必须与那些让 golden 全绿的值**逐字段相同**，否则"产物字节等价"是空话。
//!
//! 反过来，若哪天 presets 里改了尺寸，这条判据会红：那是**故意的** ——
//! 改尺寸是改产品行为，应当同时审 golden，而不是让它静默漂过去。

use std::collections::HashMap;
use std::path::PathBuf;

use postprocess::postproc::machine_dims::{MachineDimensions, ZonePolygon, load_presets_dir};

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn presets_dir() -> PathBuf {
    crate_root().join("../../presets")
}

fn baseline(name: &str) -> String {
    let path = crate_root()
        .join("tests/reference/legacy_snapshot")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("读不到基线 {}（{e}）—— 判据已空转", path.display()))
}

/// 尺寸：5 台机器 × 25 字段逐字段相等。
#[test]
fn presets_reproduce_the_legacy_dimension_table() {
    let tables = load_presets_dir(&presets_dir()).expect("presets 应当读得通");

    let expected: HashMap<String, MachineDimensions> =
        serde_json::from_str(&baseline("machine_dimensions.json")).expect("基线不是合法 JSON");

    // 反空转哨兵：基线与读出来的都要是 5 台 —— 两边都空的话下面那个断言是白过的
    assert_eq!(expected.len(), 5, "基线里的机器数变了，判据的扫描面要重算");
    assert_eq!(
        tables.dimensions().len(),
        5,
        "presets 里带 [dimensions] 的机器是 {} 台（期望 5）",
        tables.dimensions().len()
    );

    // 逐台比对（`MachineDimensions` 是 PartialEq，字段一个不少）
    for (id, want) in &expected {
        let got = tables
            .dimensions()
            .get(id)
            .unwrap_or_else(|| panic!("presets 里缺 {id} 的 [dimensions]"));
        assert_eq!(got, want, "{id} 的尺寸与真机验证过的基线不一致");
    }
}

/// 别名：23 条映射逐条相等，键都是大写。
#[test]
fn presets_reproduce_the_legacy_alias_map() {
    let tables = load_presets_dir(&presets_dir()).expect("presets 应当读得通");

    let raw: serde_json::Value =
        serde_json::from_str(&baseline("machine_catalog_extra.json")).expect("基线不是合法 JSON");
    let expected: HashMap<String, String> = raw["aliasMap"]
        .as_object()
        .expect("基线的 aliasMap 段不见了")
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                v.as_str().expect("aliasMap 的值应当是字符串").to_string(),
            )
        })
        .collect();

    assert_eq!(expected.len(), 23, "基线里的别名数变了，判据的扫描面要重算");
    assert_eq!(
        tables.alias_map().len(),
        23,
        "从 presets 读出来的别名是 {} 条（期望 23）",
        tables.alias_map().len()
    );
    assert_eq!(tables.alias_map(), &expected, "别名映射与旧快照不一致");
}

/// 禁区：3 台机器的多边形逐点相等。
#[test]
fn presets_reproduce_the_legacy_forbidden_zones() {
    let tables = load_presets_dir(&presets_dir()).expect("presets 应当读得通");

    let raw: serde_json::Value =
        serde_json::from_str(&baseline("machine_catalog_extra.json")).expect("基线不是合法 JSON");
    let expected: HashMap<String, Vec<ZonePolygon>> =
        serde_json::from_value(raw["forbiddenZones"].clone())
            .expect("基线的 forbiddenZones 段读不出来");

    assert_eq!(
        expected.len(),
        3,
        "基线里的禁区机型数变了，判据的扫描面要重算"
    );
    assert_eq!(
        tables.forbidden_zones().len(),
        3,
        "presets/forbidden_zones/ 里有 {} 个文件（期望 3）",
        tables.forbidden_zones().len()
    );
    assert_eq!(
        tables.forbidden_zones(),
        &expected,
        "禁区多边形与旧快照不一致（逐点比）"
    );
}
