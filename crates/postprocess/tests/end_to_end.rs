//! 整链判据：**`config/A1.toml` → 12 步编排 → 输出字节**，与自分基线逐字节相同。
//!
//! 这是「配置层 → 编排层 → 输出」这条链的唯一字节锁。其余判据都只覆盖一段：
//! `tests/postproc_*.rs` 覆盖 pass1/pass2 的字节，`tests/config.rs` 覆盖配置解析，
//! `tests/pipeline.rs` 覆盖步骤顺序 —— 都不能证明整链。
//!
//! ## 基线是什么
//!
//! `tests/golden/42274.2.e2e-A1.config.gcode` 是**本项目自己的**输出：
//! 同一份输入 + `tests/fixtures/config/A1.toml` 原样进管线，出来的字节。
//!
//! 它回答「整链字节有没有变」，不回答「整链输出对不对」——后者靠真机验收与人工审阅。
//!
//! ## 再生
//!
//! ```text
//! cargo test -p mkpse-postprocess --test end_to_end -- --ignored regenerate_the_chain_baseline
//! ```
//!
//! **再生前必须逐行看 diff 并说清为什么变**，原因写进提交信息。
//! 它不挂在生成、发布或任何同步动作上 —— 把现状抄成期望，判据就从「证明」退化成「自比自」。
//!
//! ## 这条判据管不到的事
//!
//! - 只跑了 A1 一个机型 / 一份输入（`42274.2.gcode`，34 个有效层）；
//! - `printtime` 只进 stderr，不进字节 ⇒ 字节相同不代表耗时估算相同。

use std::path::PathBuf;
use std::process::{Command, Output};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_mkpse-pp")
}

fn golden_input() -> PathBuf {
    repo_root().join("tests/golden/42274.2.gcode")
}

fn baseline_output() -> PathBuf {
    repo_root().join("tests/golden/42274.2.e2e-A1.config.gcode")
}

fn config_a1() -> PathBuf {
    repo_root().join("tests/fixtures/config/A1.toml")
}

fn run_cli(args: &[&str]) -> Output {
    Command::new(bin())
        .args(args)
        .output()
        .expect("CLI 二进制必须能起来")
}

/// 跑一次 `run`，返回输出文件的字节。
fn run_and_read(tag: &str, extra: &[&str]) -> Vec<u8> {
    let out_path = std::env::temp_dir().join(format!("mkpse-pp-e2e-{tag}.gcode"));
    let _ = std::fs::remove_file(&out_path);
    let input = golden_input();
    let config = config_a1();
    let mut args: Vec<String> = vec![
        "run".into(),
        input.to_string_lossy().into_owned(),
        "-c".into(),
        config.to_string_lossy().into_owned(),
        "-o".into(),
        out_path.to_string_lossy().into_owned(),
    ];
    args.extend(extra.iter().map(|s| (*s).to_string()));
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let out = run_cli(&refs);
    assert_eq!(
        out.status.code(),
        Some(0),
        "run 应当成功；stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    std::fs::read(&out_path).expect("输出文件应当存在")
}

/// 逐行比、逐行报：两个 600KB 的 Vec 直接 `assert_eq!` 会刷屏，
/// 而「字节偏移 + 附近 80 字节」在一片相同的 G-code 里指不出是哪一行。
fn compare_lines(what: &str, got: &[u8], want: &[u8]) {
    let got_lines = lines_of(got);
    let want_lines = lines_of(want);
    assert_eq!(
        got_lines.len(),
        want_lines.len(),
        "{what}: 行数不同（实际 {} 行 / 基线 {} 行）",
        got_lines.len(),
        want_lines.len()
    );
    for (i, (a, b)) in got_lines.iter().zip(want_lines.iter()).enumerate() {
        assert_eq!(a, b, "{what}: 第 {} 行与基线不同", i + 1);
    }
}

fn lines_of(bytes: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(bytes)
        .split_inclusive('\n')
        .map(str::to_owned)
        .collect()
}

/// 整链锁：配置原样进管线，出来的字节必须与基线逐字节相同。
#[test]
fn output_matches_the_chain_baseline() {
    let got = run_and_read("same", &[]);
    let want = std::fs::read(baseline_output()).expect("整链基线 fixture 应当存在");
    compare_lines("整链输出", &got, &want);
}

/// 反空转 ①：基线不是输入的副本 —— 否则上一条判据在「管线什么都不做」时也会绿。
#[test]
fn the_baseline_is_not_just_a_copy_of_the_input() {
    let input = std::fs::read(golden_input()).expect("输入 fixture 应当存在");
    let want = std::fs::read(baseline_output()).expect("整链基线 fixture 应当存在");
    assert!(input != want, "基线与输入相同 ⇒ 整链判据是空的");
    assert!(
        want.len() > input.len(),
        "基线（{} 字节）没有比输入（{} 字节）长：后处理是往里加涂胶/塔/擦拭动作的，\
         不该变短",
        want.len(),
        input.len()
    );
}

/// 反空转 ②：字节比对**对配置敏感**。
///
/// 改一个真实旋钮（涂胶遍数 1 → 2）后输出必须与基线不同；否则说明配置根本没走到管线里，
/// 上一条判据就只是在比较「两个固定文件」。
#[test]
fn the_byte_comparison_is_sensitive_to_the_config() {
    let baseline = run_and_read("baseline", &[]);
    let perturbed = run_and_read("perturbed", &["--set", "Glue.PassCount=2"]);
    let want = std::fs::read(baseline_output()).expect("整链基线 fixture 应当存在");

    // 用 assert!(a == b) 而不是 assert_eq!：后者失败时会把两个 600KB 的 Vec 原样打出来
    // （实测 5.2MB 输出，真正的原因反而被埋了）。
    assert!(baseline == want, "基线跑法应当与 fixture 一致（前置条件）");
    assert!(
        perturbed != want,
        "把 Glue.PassCount 改成 2 之后输出仍与基线一致 ⇒ 配置没进管线，整链判据是空的"
    );
}

/// 手动再生工具。**具名 ignore，不是静默跳过**：平时不跑，否则判据文件被测试自己覆写，
/// 就再也测不出「文件被改坏了」。
///
/// 写盘豁免（`clippy::disallowed_methods`）：写盘纪律管的是**生产代码** ——
/// 它的风险不在「截断半个文件」，而在「改了判据期望却没人看 diff」。
/// 兜着后者的是 `#[ignore]`（只能人手触发）与上面那条再生规矩。
#[test]
#[ignore = "手动再生：cargo test --test end_to_end -- --ignored regenerate_the_chain_baseline"]
#[allow(clippy::disallowed_methods)]
fn regenerate_the_chain_baseline() {
    let bytes = run_and_read("regen", &[]);
    let path = baseline_output();
    std::fs::write(&path, &bytes).expect("写基线");
    println!(
        "已写入 {}（{} 字节）—— 提交前必须逐行审查 diff",
        path.display(),
        bytes.len()
    );
}
