//! 边缘带支撑免抬的回归样例（真实样本，不是合成几何）。
//!
//! - 输入：`tests/fixtures/glue/edge_skip_sample.gcode`（MKP 支撑测试件，切片器原版）
//! - 预设：入库产物 `A1_MINI-fast261111`（出厂 `glue_edge_skip_margin = 0.5`）
//! - 只改一个数，跑三档：`0`（关闭）/ `0.5`（出厂）/ `2.0`（更保守）
//!
//! 判据：
//! 1. **只做减法**：`0` 与 `0.5` 的产物，剔除所有 `G1 Z` 行后逐行相同 ——
//!    路径、挤出、速度、温度都不许动，差的只有抬升。
//! 2. **0 是关闭**：0 的抬笔次数必须比 0.5 多；2.0 不比 0.5 少。
//! 3. 三档的 `(行数, Z 行数, 抬笔次数)` 与 `fixtures/glue/edge_skip_summary.txt` 一致。
//!    这份摘要就是"不同参数的效果"；重录要显式：
//!    `MKPSE_UPDATE_EDGE_SKIP_SUMMARY=1 cargo test -p mkpse-preset --test edge_skip_margin -- --nocapture`
#![allow(clippy::disallowed_methods)]

mod common;

use std::path::PathBuf;

use postprocess::diag::CancelToken;
use postprocess::ir::Ir;

/// 三档：关闭 / 出厂 / 更保守。
const MARGINS: [f64; 3] = [0.0, 0.5, 2.0];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn preset_ir() -> Ir {
    let path = root().join("crates/preset/assets/presets/A1_MINI-fast261111.toml");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读不到 {}：{e}", path.display()));
    let cfg: preset::TomlConfig = toml::from_str(&text).expect("预设 TOML 解析失败");
    let mut ir = preset::build(&cfg, None, preset::CalibrationExecMode::Fallback)
        .expect("预设映射到 IR 失败");
    /* 机型尺寸不进预设 TOML：客户端入口用 `machine_dims` 填，这里给同一份（A1_MINI 190×180） */
    ir.machine.max_x = 190.0;
    ir.machine.max_y = 180.0;
    ir.machine.glue_max_x = 190.0;
    ir.machine.glue_max_y = 180.0;
    if ir.machine.machine_type.is_empty() {
        ir.machine.machine_type = "A1_MINI".to_owned();
    }
    ir
}

fn run(margin: f64, input: &[String]) -> Vec<String> {
    common::install_tables();
    let mut ir = preset_ir();
    ir.glue.edge_skip_margin = margin;
    let machine = ir.machine.machine_type.clone();
    let mut progress = |_pct: f64, _msg: String| {};
    postprocess::postproc::pass1::first_pass(
        input,
        &mut ir,
        &machine,
        &mut progress,
        &CancelToken::new(),
        postprocess::postproc::cancel::DEFAULT_CANCEL_CHECK_INTERVAL,
    )
    .expect("first_pass 失败")
    .lines
}

fn z_lines(lines: &[String]) -> usize {
    lines.iter().filter(|l| l.starts_with("G1 Z")).count()
}

/// 抬笔次数：Z 比上一句高 0.5 的次数（`Glue.EdgeLift` 是 0.5）。
fn lift_count(lines: &[String]) -> usize {
    let mut prev: Option<f64> = None;
    let mut n = 0usize;
    for line in lines {
        let Some(rest) = line.strip_prefix("G1 Z") else {
            continue;
        };
        let Ok(z) = rest.trim().parse::<f64>() else {
            continue;
        };
        if prev.is_some_and(|p| (z - p - 0.5).abs() < 1e-3) {
            n += 1;
        }
        prev = Some(z);
    }
    n
}

fn summary(rows: &[(f64, usize, usize, usize)]) -> String {
    let mut text = String::from(
        "# 边缘带支撑免抬：真实样本三档的产物摘要（MKP_support_test_models / A1_MINI fast261111）\n",
    );
    text.push_str("# margin lines z_lines lifts\n");
    for (margin, lines, z, lifts) in rows {
        text.push_str(&format!("{margin:.2} {lines} {z} {lifts}\n"));
    }
    text
}

/// 出厂就是"打开、0.5" —— 这条钉住默认档，别有人把它改成 0。
#[test]
fn the_factory_preset_ships_half_a_millimetre() {
    let ir = preset_ir();
    assert!(
        (ir.glue.edge_skip_margin - 0.5).abs() < 1e-9,
        "出厂预设应当是 0.5（默认打开），实际 {}",
        ir.glue.edge_skip_margin
    );
}

#[test]
fn edge_skip_examples() {
    let path = root().join("crates/preset/tests/fixtures/glue/edge_skip_sample.gcode");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读不到 {}：{e}", path.display()));
    let input: Vec<String> = text.lines().map(str::to_owned).collect();

    let outs: Vec<(f64, Vec<String>)> = MARGINS.iter().map(|m| (*m, run(*m, &input))).collect();

    /* 1) 只做减法：0 与 0.5 剔掉 Z 行后逐行相同 */
    let off: Vec<&String> = outs[0]
        .1
        .iter()
        .filter(|l| !l.starts_with("G1 Z"))
        .collect();
    let on: Vec<&String> = outs[1]
        .1
        .iter()
        .filter(|l| !l.starts_with("G1 Z"))
        .collect();
    assert_eq!(off.len(), on.len(), "0 与 0.5 的非 Z 行数不同");
    for (i, (a, b)) in off.iter().zip(on.iter()).enumerate() {
        assert_eq!(a, b, "第 {i} 行非 Z 内容不同：{a:?} vs {b:?}");
    }

    /* 2) 单调：0（关闭）抬得最多；值越大越保守，抬得越多，但不会超过关闭 */
    let counts: Vec<usize> = outs.iter().map(|(_, l)| lift_count(l)).collect();
    assert!(
        counts[0] > counts[2],
        "0（关闭）应当比 2.0（更保守）抬得多：{counts:?}"
    );
    assert!(
        counts[2] > counts[1],
        "2.0 应当比 0.5 抬得多（越大越保守）：{counts:?}"
    );

    /* 3) 摘要：三档的行数 / Z 行数 / 抬笔次数 */
    let rows: Vec<(f64, usize, usize, usize)> = outs
        .iter()
        .map(|(m, l)| (*m, l.len(), z_lines(l), lift_count(l)))
        .collect();
    let want = summary(&rows);
    let out = root().join("crates/preset/tests/fixtures/glue/edge_skip_summary.txt");

    if std::env::var_os("MKPSE_UPDATE_EDGE_SKIP_SUMMARY").is_some() {
        std::fs::write(&out, &want).expect("写摘要失败");
        println!("已重录 {}：\n{want}", out.display());
        return;
    }

    let got = std::fs::read_to_string(&out).unwrap_or_else(|e| {
        panic!(
            "读不到摘要 {}：{e}（首次请用 MKPSE_UPDATE_EDGE_SKIP_SUMMARY=1 生成）",
            out.display()
        )
    });
    assert_eq!(want, got, "三档摘要变了；确认差异合理后再重录");
}
