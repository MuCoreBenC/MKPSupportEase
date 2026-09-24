//! G1 —— FirstPass 输出逐字节 golden（Task 3.3 落判据，Task 11 转绿）。
//!
//! 输入 `42274.2.gcode` + Task 4.2 导出的 IR fixture（golden_test.go:38-46 的
//! 7 项覆写已含在导出里）。判据：先比行数再逐行，零容差、不许 Skip。
//! 附 11.7 哨兵：`M1031 S1` 出现次数 ≤ 20（RISK-002 回归锚）。

mod golden_common;

use golden_common as gc;

fn run_first_pass(input: &str) -> Vec<String> {
    let content: Vec<String> = gc::split_lines(input);
    let ir_json = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/ir/golden_42274_2.json"),
    )
    .expect("读不到 IR fixture（Task 4.2 的导出物）");
    let mut ir_data: postprocess::ir::Ir =
        serde_json::from_str(&ir_json).expect("IR JSON 反序列化失败");
    let mut progress = |_pct: f64, _msg: String| {};
    let toml_machine = ir_data.machine.machine_type.clone();
    let out = postprocess::postproc::pass1::first_pass(
        &content,
        &mut ir_data,
        &toml_machine,
        &mut progress,
        &postprocess::diag::CancelToken::new(),
        postprocess::postproc::cancel::DEFAULT_CANCEL_CHECK_INTERVAL,
    )
    .expect("first_pass 失败");
    out.lines
}

#[test]
fn g1_first_pass_matches_golden() {
    let input = gc::read_golden("42274.2.gcode");
    let expected = gc::split_lines(&gc::read_golden("42274.2.expected.gcode"));

    let actual = run_first_pass(&input);

    if gc::update_golden_requested() {
        let mut text = actual.join("\n");
        text.push('\n');
        gc::write_golden("42274.2.expected.gcode", &text);
        return;
    }
    gc::compare_line_wise("G1 FirstPass", &expected, &actual);

    // 11.7 哨兵：RISK-002 修复后 M1031 S1 应大幅减少（buggy 时代 88，golden 实测 0）
    let m1031_count = actual.iter().filter(|l| l.contains("M1031 S1")).count();
    assert!(
        m1031_count <= 20,
        "M1031 S1 计数 {m1031_count} > 20 —— 疑似 RISK-002（裸熨平路径）回归"
    );
}
