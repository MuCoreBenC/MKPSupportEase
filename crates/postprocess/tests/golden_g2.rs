//! G2 —— pass1+pass2 全链输出逐字节 golden（Task 3.3 落判据，Task 12 转绿）。
//!
//! 诚实边界（Task 12.6 会如实登记）：42274.2 是**单色单挤出**切片，
//! expected 里擦料塔相关计数为 0 ⇒ 本判据不覆盖换料塔端到端
//! （design.md §六）。它钉住的是 pass1→pass2 主链的其余全部行为。

mod golden_common;

use golden_common as gc;

/// 全链：first_pass（IR fixture）→ second_pass（消耗式吃 Pass1Output）。
fn run_pass2_chain(input: &str) -> Vec<String> {
    let content: Vec<String> = gc::split_lines(input);
    let ir_json = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/ir/golden_42274_2.json"),
    )
    .expect("读不到 IR fixture（Task 4.2 的导出物）");
    let mut ir_data: postprocess::ir::Ir =
        serde_json::from_str(&ir_json).expect("IR JSON 反序列化失败");
    let toml_machine = ir_data.machine.machine_type.clone();
    let mut progress = |_pct: f64, _msg: String| {};
    let p1 = postprocess::postproc::pass1::first_pass(
        &content,
        &mut ir_data,
        &toml_machine,
        &mut progress,
        &postprocess::diag::CancelToken::new(),
        postprocess::postproc::cancel::DEFAULT_CANCEL_CHECK_INTERVAL,
    )
    .expect("first_pass 失败");
    let final_tower_height = p1.stats.max_z_height;
    let mut pass2_stats = Default::default();
    let p2 = postprocess::postproc::pass2::second_pass(
        p1,
        &mut ir_data,
        final_tower_height,
        &toml_machine,
        &mut progress,
        &mut pass2_stats,
        &postprocess::diag::CancelToken::new(),
        postprocess::postproc::cancel::DEFAULT_CANCEL_CHECK_INTERVAL,
    )
    .expect("second_pass 失败");
    p2.lines
}

#[test]
fn g2_pass2_chain_matches_golden() {
    let input = gc::read_golden("42274.2.gcode");
    let expected = gc::split_lines(&gc::read_golden("42274.2.pass2.expected.gcode"));

    let actual = run_pass2_chain(&input);

    if gc::update_golden_requested() {
        let mut text = actual.join("\n");
        text.push('\n');
        gc::write_golden("42274.2.pass2.expected.gcode", &text);
        return;
    }
    gc::compare_line_wise("G2 pass1+pass2", &expected, &actual);
}
