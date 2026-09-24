//! 写回面的判据：K-P1（零编辑往返）/ K-P2（单键改值）/ K-P3（拒绝面）。
//!
//! **K-P1 是这一轮最强的一条**：`apply_edits(raw, &[])` 的输出必须与输入**逐字节相同**。
//! 它一次性守住「注释、空行、键序、行尾风格、数字形态」全部五件事 ——
//! 任何一处漂移都会让字节比较失败，不需要为每件事单独写断言。
//! 覆盖面是仓库里的 9 份 fixture 预设（真实用户预设的同构副本，含约 70 条注释 / 100 行）。
//!
//! K-P2 用「行级 diff 恰好一行」+ 「结构比较除该键外全等」双验：
//! 前者说明文件没被重排，后者说明没有别的键被顺手改掉（文本 diff 会被格式化差异淹没）。

use std::path::PathBuf;

// 基线目录只认 `generate::fixtures_dir()` 那一处（理由见 `tests/recipe.rs`）
use preset::generate::fixtures_dir;
use preset::model::TomlConfig;
use preset::write::{Edit, EditValue, apply_edits};

fn presets() -> Vec<PathBuf> {
    let dir = fixtures_dir();
    let mut out: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("读 fixtures/presets")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    out.sort();
    assert!(
        out.len() >= 9,
        "反空转：fixture 预设至少 9 份，实测 {}",
        out.len()
    );
    out
}

fn parse(text: &str) -> TomlConfig {
    toml::from_str(text).expect("写回的文本必须还能被读面吃下")
}

#[test]
fn kp1_zero_edits_round_trip_is_byte_identical() {
    let mut checked = 0;
    let mut comments = 0;
    for p in presets() {
        let raw = std::fs::read_to_string(&p).expect("读预设");
        let out = apply_edits(&raw, &[]).expect("零编辑写回");
        assert_eq!(
            out,
            raw,
            "K-P1 红：{} 零编辑写回后字节变了（注释/键序/形态之一被动过）",
            p.display()
        );
        comments += raw.lines().filter(|l| l.contains('#')).count();
        checked += 1;
    }
    assert!(
        comments > 300,
        "反空转：这批预设应当注释很密，实测 {comments} 行带 #"
    );
    println!("K-P1 绿：{checked} 份预设零编辑往返逐字节相同（共 {comments} 行带注释）");
}

#[test]
fn kp2_one_edit_changes_exactly_one_line() {
    let path = fixtures_dir().join("A1.toml");
    let raw = std::fs::read_to_string(&path).expect("读预设");
    let before = parse(&raw);

    let out = apply_edits(
        &raw,
        &[Edit::new("toolhead", "speed_limit", EditValue::Float(65.0))],
    )
    .expect("改一个键");

    // ① 行级：只有一行不同，且新旧行都能指名
    let diff: Vec<(usize, &str, &str)> = raw
        .lines()
        .zip(out.lines())
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(i, (a, b))| (i + 1, a, b))
        .collect();
    assert_eq!(raw.lines().count(), out.lines().count(), "行数不许变");
    assert_eq!(
        diff.len(),
        1,
        "K-P2 红：期望恰好 1 行不同，实测 {} 行：{diff:?}",
        diff.len()
    );
    let (lineno, old_line, new_line) = diff[0];
    assert!(
        new_line.contains("speed_limit = 65") && !new_line.contains("65.0"),
        "K-P2 红：整数形态漂移了 —— 原来是 `{old_line}`，写成了 `{new_line}`"
    );
    assert!(
        new_line.contains('#') && old_line.contains('#'),
        "K-P2 红：那一行的行尾注释没了 —— 原来 `{old_line}`，现在 `{new_line}`"
    );

    // ② 结构：除被改的键外，其余键逐一相同
    let after = parse(&out);
    assert_eq!(after.toolhead.speed_limit, 65.0);
    let mut expect = before.clone();
    expect.toolhead.speed_limit = 65.0;
    assert_eq!(
        after, expect,
        "K-P2 红：除 speed_limit 之外还有别的键被改了"
    );

    println!(
        "K-P2 绿：第 {lineno} 行 `{}` → `{}`，其余 {} 行逐字节不变，结构比较只差那一个键",
        old_line.trim(),
        new_line.trim(),
        raw.lines().count() - 1
    );
}

#[test]
fn kp2b_inline_table_and_multiline_keep_their_shape() {
    let path = fixtures_dir().join("A1MF.toml");
    let raw = std::fs::read_to_string(&path).expect("读预设");

    // inline table 内部改一个分量：整块不许被重写，行尾注释要留着
    let out = apply_edits(
        &raw,
        &[Edit::new("toolhead", "offset.z", EditValue::Float(4.5))],
    )
    .expect("改 inline table 分量");
    let line = out
        .lines()
        .find(|l| l.trim_start().starts_with("offset ="))
        .expect("offset 那一行还在");
    assert!(
        line.contains("{ x =") && line.contains("z = 4.5") && line.contains('#'),
        "K-P2b 红：inline table 形态或行尾注释被破坏：`{line}`"
    );
    assert_eq!(parse(&out).toolhead.offset.z, 4.5);

    // 多行 G-code：写回仍是 `"""` 字面量，且读回来的内容与写进去的一致
    let gcode = "G92 E0\nG1 E-5 F1800\n";
    let out2 = apply_edits(
        &raw,
        &[Edit::new(
            "toolhead",
            "custom_mount_gcode",
            EditValue::Str(gcode.to_string()),
        )],
    )
    .expect("改多行串");
    assert!(
        out2.contains("custom_mount_gcode = \"\"\"\nG92 E0\nG1 E-5 F1800\n\"\"\""),
        "K-P2b 红：多行字面量塌成了单行转义串"
    );
    assert_eq!(parse(&out2).toolhead.custom_mount_gcode, gcode);

    println!("K-P2b 绿：inline table 与多行字面量的形态都保住了");
}

#[test]
fn kp3_refuses_new_keys_missing_sections_and_bad_values() {
    let path = fixtures_dir().join("A1.toml");
    let raw = std::fs::read_to_string(&path).expect("读预设");

    let cases: Vec<(&str, Edit, &str)> = vec![
        (
            "新增键",
            Edit::new("toolhead", "brand_new_key", EditValue::Float(1.0)),
            "拒绝新增",
        ),
        (
            "不存在的段",
            Edit::new("nosuch", "speed_limit", EditValue::Float(1.0)),
            "没有 [nosuch] 段",
        ),
        (
            "NaN",
            Edit::new("toolhead", "speed_limit", EditValue::Float(f64::NAN)),
            "NaN/Inf",
        ),
        (
            "Inf",
            Edit::new("toolhead", "speed_limit", EditValue::Float(f64::INFINITY)),
            "NaN/Inf",
        ),
        (
            "多行串里带三引号",
            Edit::new(
                "toolhead",
                "custom_mount_gcode",
                EditValue::Str("G1\n\"\"\"\n".to_string()),
            ),
            "多行字面量装不下",
        ),
        (
            "单行串收到换行",
            Edit::new(
                "wiping",
                "have_wiping_components",
                EditValue::Str("tower\ndisk".to_string()),
            ),
            "原本是单行字符串",
        ),
    ];

    for (name, edit, expect_msg) in cases {
        let err = apply_edits(&raw, &[edit]).expect_err(&format!("{name} 必须被拒绝"));
        let msg = err.to_string();
        assert!(
            msg.contains(expect_msg),
            "K-P3 红：{name} 的报错没说清原因。期望包含 `{expect_msg}`，实测 `{msg}`"
        );
        println!("  {name} → {msg}");
    }
    println!("K-P3 绿：6 类越权/非法编辑各拒绝一次，报错都点明了原因");
}
