//! Task 10.3 判据：printtime 数值比对（tests/reference/printtime/）。
//!
//! 输入 = G2 vendored 副本（与 tests/golden/42274.2.pass2.expected.gcode 字节相同），
//! 选项 = {compute_delta:true, startup_overhead_seconds:240}（与旧侧 process.go:1104 同参）。
//! 数值策略：**逐位相等**。原策略是「不等则退为相对误差 1e-9」，Task 8.4e 查清那唯一
//! 一条退档（`TotalSeconds` 差 1 ULP）的真因是**参考文件的解析层**而非算法（serde_json
//! 默认 feature 的浮点快路径），修掉解析后 5 项全部逐位相等 ⇒ 容差分支已删。

use postprocess::postproc::printtime::{Options, estimate};

fn load(rel: &str) -> String {
    std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)).unwrap()
}

#[test]
fn printtime_matches_go_reference() {
    let content = load("tests/reference/printtime/input.gcode");
    let expected: serde_json::Value =
        serde_json::from_str(&load("tests/reference/printtime/expected_result.json")).unwrap();

    let opts = Options {
        compute_delta: true,
        startup_overhead_seconds: 240.0,
    };
    let got = estimate(&content, &opts);

    let cases = [
        ("TotalSeconds", got.total_seconds),
        ("ToolMovementSeconds", got.tool_movement_seconds),
        ("PrepOverheadSeconds", got.prep_overhead_seconds),
        ("StartupOverheadSeconds", got.startup_overhead_seconds),
        (
            "BambuAnchorSeconds",
            got.bambu_anchor_seconds.unwrap_or(0.0),
        ),
    ];
    for (key, value) in cases {
        let want = expected[key].as_f64().unwrap();
        // **Task 8.4e 判据升级（这是收紧不是放宽）**：原判据是「先逐位相等，不等则退到
        // 相对误差 < 1e-9」，Task 10 当时实测 `TotalSeconds` 落在退档分支上（记为「与 Go
        // 参考差 1 ULP」）。那 1 ULP 的**真因不在算法**：`expected_result.json` 是用
        // serde_json 读的，而它默认 feature 下的浮点解析走「有效数字 as f64 ÷ 10^k」快
        // 路径，17 位有效数字上可差 1 ULP（等价复现：`11637377359379175f64 / 1e13`）。
        // 根 `Cargo.toml` 打开 `float_roundtrip` 后参考按 IEEE 最近舍入解析，本表 5 项
        // **全部逐位相等** ⇒ 容差分支不再需要，删掉它（留着等于给未来的真差异开后门）。
        assert_eq!(
            value.to_bits(),
            want.to_bits(),
            "{key}: got {value} (bits {:x})，want {want} (bits {:x})；相对误差 {}。\
             逐位不等就是真差异——先归因（算法顺序 / FMA / 参考解析），不要加容差",
            value.to_bits(),
            want.to_bits(),
            (value - want).abs() / want.max(1e-12)
        );
    }
    assert_eq!(got.segments, expected["Segments"].as_i64().unwrap());
    assert_eq!(
        got.post_process_delta_seconds.is_none(),
        expected["PostProcessDeltaSeconds"].is_null(),
        "无 _original 兄弟 ⇒ delta 两侧都为 null"
    );
}

/// 10.4 反空转：空内容不得静默返回 0 秒——锚点/开销为 0 合法，但 segments 为 0
/// 且分类表为空是可区分的零值形态（引擎层可据此判断）。
#[test]
fn empty_input_is_distinguishable_zero() {
    let got = estimate("; nothing\n", &Options::default());
    assert_eq!(got.segments, 0);
    assert_eq!(got.total_seconds, 0.0);
    assert!(got.by_type.is_empty(), "零段输入不得虚构分类统计");
}
