//! 配方正文的判据（K-G2 / K-G3 / K-G4）。
//!
//! 2026-11-04：旧 **K-G1**（配方渲染与现役 9 份逐字节相同）随 `gen-presets` 一起退役。
//! 现役产物由工作台按新头部生成（`# machine` / `# variant` / `# generated_at`，
//! 没有 `uuid` / `release_time`），而老配方渲染仍带那两行 —— 两边再逐字节比必然是假红。
//! 正式产物的判据归工作台 `presets diff` / `check:artifacts` 与
//! `tests/builtin_presets_match_dir.rs`（含客户端读取面闸门）。
//!
//! 这里保留的是**配方正文本身**还健康：幂等、渲染出的预设能被读取面消费、
//! 每条覆盖真的生效。配方正文仍被工作台预检（`wb_preflight`）当机型清单用。

// 测试写临时文件、造夹具是正当的：写盘纪律管的是**生产代码**
// （与源码扫描断言只看 `#[cfg(test)]` 之前那部分同一口径）。
#![allow(clippy::disallowed_methods)]

use preset::recipe::{Recipe, render};

const RECIPE: &str = include_str!("../assets/preset_recipes.toml");

/// K-G2：幂等 —— 连渲染两次结果相同。
///
/// 这条挡的是「uuid 或时间偷偷溜进生成路径」：那种代码第一次跑也是绿的，
/// 只有比较两次才看得出来。（Task 4 的脚本门禁从另一侧再守一次。）
#[test]
fn kg2_rendering_twice_gives_the_same_bytes() {
    let recipe = Recipe::parse(RECIPE).expect("配方");
    for (machine, variant) in recipe.combos() {
        let a = render(&recipe, &machine, &variant).expect("第一次");
        let b = render(&recipe, &machine, &variant).expect("第二次");
        assert_eq!(a, b, "K-G2 红：{machine}:{variant} 两次渲染结果不同");
    }
    println!("K-G2 绿：9 份各渲染两次，结果相同");
}

/// K-G3：产物是**能用的预设** —— `read_preset` + `load_ir` 都过得去。
///
/// 字节对不等于能用：字节对只说明「与现在那份一样」，而那份能不能被本程序消费
/// 要由读取面与九步流程说话。
#[test]
fn kg3_generated_presets_are_usable() {
    let recipe = Recipe::parse(RECIPE).expect("配方");
    let dir = tempfile::tempdir().expect("临时目录");
    let mut checked = 0usize;

    for (machine, variant) in recipe.combos() {
        let text = render(&recipe, &machine, &variant).expect("渲染");
        let out = dir.path().join(format!("{machine}-{variant}.toml"));
        std::fs::write(&out, &text).expect("写产物");

        let file = preset::read_preset(&out)
            .unwrap_or_else(|e| panic!("K-G3 红：{machine}:{variant} 读不回来：{e}"));
        assert_eq!(file.machine, machine, "K-G3 红：机型头写错了");
        assert_eq!(file.variant.as_deref(), Some(variant.as_str()));
        assert_eq!(
            file.release_time.as_deref(),
            Some(recipe.release_time.as_str())
        );
        preset::load_ir(&out, None)
            .unwrap_or_else(|e| panic!("K-G3 红：{machine}:{variant} load_ir 失败：{e}"));
        checked += 1;
    }
    assert_eq!(checked, 9);
    println!("K-G3 绿：9 份产物都能 read_preset + load_ir，头三项对得上");
}

/// K-G4：**每一条覆盖都真的生效了** —— 逐字段回读产物里的值。
///
/// 计划里这条写的是「没有孤儿覆盖（写错机型名会静默无效）」。实测那件事已经被 K-G5
/// 拦在解析阶段（机型/变体不存在就报错），所以只查「键被查过」的判据是空转。
/// 改成查**应用结果**：产物里那一项的值必须等于覆盖给的值。
///
/// 它挡的是另一类错：路径拆错（`toolhead.offset.x` 落到 `offset` 整表上）、
/// 整值写进了错的段、或某条覆盖被 `effective()` 漏掉。
#[test]
fn kg4_every_override_field_actually_lands_in_the_product() {
    let recipe = Recipe::parse(RECIPE).expect("配方");
    let mut fields_total = 0usize;
    let mut read_back = 0usize;

    for (machine, variant) in recipe.combos() {
        let text = render(&recipe, &machine, &variant).expect("渲染");
        let parsed: toml::Value = toml::from_str(&text).expect("产物是合法 TOML");
        // 顺手确认一件事：生成的**云端**预设不该带副本的血统三行
        let file = preset::read_preset_from_bytes(text.clone()).expect("产物能读");
        assert!(
            file.lineage.is_none(),
            "K-G4 红：云端预设不该带 `# based_on*`"
        );

        for (path, want) in recipe.effective(&machine, &variant) {
            fields_total += 1;
            if path == "uuid" {
                let line = format!("# uuid: {}", want.as_str().expect("uuid 是字符串"));
                assert!(
                    text.lines().any(|l| l.trim() == line),
                    "K-G4 红：{machine}:{variant} 的产物里找不到 `{line}`"
                );
                continue;
            }
            let got = lookup(&parsed, &path).unwrap_or_else(|| {
                panic!("K-G4 红：{machine}:{variant} 的产物里没有 `{path}` 这一项")
            });
            assert!(
                same_number_or_eq(&want, &got),
                "K-G4 红：{machine}:{variant} 的 `{path}` 没生效 —— \
                 覆盖给 {want:?}，产物是 {got:?}"
            );
            read_back += 1;
        }
    }
    assert!(
        fields_total >= 20,
        "K-G4 红：只查到 {fields_total} 条覆盖字段，判据在空转"
    );
    println!("K-G4 绿：{fields_total} 条覆盖字段全部生效（其中 {read_back} 条回读了 TOML 值）");
}

/// 按 `段.键[.内层]` 取值。
fn lookup(root: &toml::Value, path: &str) -> Option<toml::Value> {
    let mut cur = root;
    for part in path.split('.') {
        cur = cur.get(part)?;
    }
    Some(cur.clone())
}

/// 值相等；整数与浮点跨类型也算相等（`24` 与 `24.0` 是同一个数，**形态**是 K-G1 的事）。
fn same_number_or_eq(a: &toml::Value, b: &toml::Value) -> bool {
    match (a, b) {
        (toml::Value::Integer(x), toml::Value::Float(y))
        | (toml::Value::Float(y), toml::Value::Integer(x)) => (*x as f64 - *y).abs() < f64::EPSILON,
        _ => a == b,
    }
}
