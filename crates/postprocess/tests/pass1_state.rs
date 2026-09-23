//! pass1 状态机行为测试（对照 processor/pass1_state_test.go 逐条移植）。
//!
//! 这些测试是「行为规格」而非字节判据：钉住熨平移除标志的置位/清除/泄漏/残留、
//! copyFlag/actFlag 闭环、风扇状态跟踪与恢复。含 4 个 Go 侧已登记的 KNOWN BUG
//! （RISK-003/004/005/011）——Rust 逐字复刻，同样以测试钉住其行为。
//!
//! helper 直测的 3 条（findPrevPositionCmd 索引 0、stripSkippableBlocks 两形态）
//! 在各模块的单元测试里（scan.rs / delete_wipe.rs）。

use mkp_pp::ir::Ir;

fn fixture_ir() -> Ir {
    let json = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/ir/golden_42274_2.json"),
    )
    .expect("读不到 IR fixture");
    let mut ir: Ir = serde_json::from_str(&json).expect("IR 反序列化失败");
    // Go 的 newTestIR 不设 MachineType（runFirstPass 传空 tomlMachine → 跳过维度
    // 填充、±999 边界）。fixture 带 golden 覆写 A1_MINI，这里抹平对齐 newTestIR。
    ir.machine.machine_type = String::new();
    ir
}

fn lines(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// runFirstPass（pass1_state_test.go:161）：跑 first_pass，返回 (输出行, 突变后 IR)。
fn run_first_pass(gcode: &[String], ir_data: Ir) -> (Vec<String>, Ir) {
    let mut ir = ir_data;
    let toml_machine = ir.machine.machine_type.clone();
    let mut progress = |_, _: String| {};
    let out = mkp_pp::postproc::pass1::first_pass(
        gcode,
        &mut ir,
        &toml_machine,
        &mut progress,
        &mkp_pp::diag::CancelToken::new(),
        mkp_pp::postproc::cancel::DEFAULT_CANCEL_CHECK_INTERVAL,
    )
    .expect("first_pass 失败");
    (out.lines, ir)
}

fn contains_line(output: &[String], substr: &str) -> bool {
    output.iter().any(|l| l.contains(substr))
}

fn has_exact_line(output: &[String], line: &str) -> bool {
    output.iter().any(|l| l.trim() == line)
}

fn count_occurrences(output: &[String], substr: &str) -> usize {
    output.iter().filter(|l| l.contains(substr)).count()
}

// ---- ironingRemovalFlag（RISK-001/002/003/004）----

/// L556 置位 + L1029 FEATURE 清除（RISK-001/RISK-004）。
#[test]
fn ironing_removal_ironing_feature_off_mode_removes_paths() {
    let gcode = lines(&[
        "; BambuStudio 02.07.01.62",
        "; enable_support_ironing = 1",
        "; Z_HEIGHT: 0.5",
        "; FEATURE: Ironing",
        "G1 X80.0 Y80.0 E0.1",
        "; FEATURE: Outer wall",
        "G1 X100.0 Y100.0 E0.1",
    ]);
    let (output, _) = run_first_pass(&gcode, fixture_ir());
    assert!(
        !contains_line(&output, "G1 X80.0 Y80.0 E0.1"),
        "熨平路径应被删除"
    );
    assert!(contains_line(&output, "G1 E-"), "应发射回抽行 G1 E-...");
    assert!(
        contains_line(&output, "G1 X100.0 Y100.0 E0.1"),
        "Outer wall 路径应保留"
    );
    assert!(contains_line(&output, "FEATURE: Outer wall"));
}

/// M1031 S1 + 5 行内 FEATURE: Support ironing → 整块删除（RISK-002 首块）。
#[test]
fn ironing_removal_support_ironing_m1031_first_block_removed() {
    let gcode = lines(&[
        "; BambuStudio 02.07.01.62",
        "; enable_support_ironing = 1",
        "; Z_HEIGHT: 0.5",
        "M1031 S1 ;IRONING_EXTRUSIONS_START",
        "; FEATURE: Support ironing",
        ";LINE_WIDTH:0.42",
        "G1 X90.0 Y90.0 E0.01",
        "M1031 S0 ;IRONING_EXTRUSIONS_END",
        "; WIPE_START",
        "G1 X90.0 Y90.0 E-0.38",
        "; WIPE_END",
        "; FEATURE: Outer wall",
        "G1 X100.0 Y100.0 E0.1",
    ]);
    let (output, _) = run_first_pass(&gcode, fixture_ir());
    assert!(!contains_line(&output, "M1031"), "M1031 应被删除");
    assert!(!contains_line(&output, "G1 X90.0 Y90.0 E0.01"));
    assert!(!contains_line(&output, "WIPE_START"));
    assert!(!contains_line(&output, "WIPE_END"));
    assert!(!contains_line(&output, "FEATURE: Support ironing"));
    assert!(!contains_line(&output, "LINE_WIDTH"));
    assert!(contains_line(&output, "G1 X100.0 Y100.0 E0.1"));
}

/// ; WIPE_END 清除标志（RISK-004 清除路径）。
#[test]
fn ironing_removal_wipe_end_clears_flag() {
    let gcode = lines(&[
        "; BambuStudio 02.07.01.62",
        "; enable_support_ironing = 1",
        "; Z_HEIGHT: 0.5",
        "M1031 S1 ;IRONING_EXTRUSIONS_START",
        "; FEATURE: Support ironing",
        ";LINE_WIDTH:0.42",
        "G1 X90.0 Y90.0 E0.01",
        "M1031 S0 ;IRONING_EXTRUSIONS_END",
        "; WIPE_START",
        "G1 X90.0 Y90.0 E-0.38",
        "; WIPE_END",
        "G1 E-0.02",
        "M204 S5000",
        "; FEATURE: Outer wall",
        "G1 X100.0 Y100.0 E0.1",
    ]);
    let (output, _) = run_first_pass(&gcode, fixture_ir());
    assert!(
        has_exact_line(&output, "G1 E-0.02"),
        "WIPE_END 后应保留 G1 E-0.02"
    );
    assert!(contains_line(&output, "M204 S5000"));
}

/// 非熨平 FEATURE 过渡清除标志（L1029 兜底，不依赖 copyFlag）。
#[test]
fn ironing_removal_feature_transition_clears_flag() {
    let gcode = lines(&[
        "; BambuStudio 02.07.01.62",
        "; enable_support_ironing = 1",
        "; Z_HEIGHT: 0.5",
        "; FEATURE: Ironing",
        "G1 X80.0 Y80.0 E0.1",
        "; FEATURE: Outer wall",
        "G1 X100.0 Y100.0 E0.1",
    ]);
    let (output, _) = run_first_pass(&gcode, fixture_ir());
    assert!(!contains_line(&output, "G1 X80.0 Y80.0 E0.1"));
    assert!(contains_line(&output, "G1 X100.0 Y100.0 E0.1"));
}

/// RISK-002 修复：inSupportIroningSection 兜底删除第二个 M1031 块。
#[test]
fn ironing_removal_subsequent_m1031_block_leak_removal() {
    let gcode = lines(&[
        "; BambuStudio 02.07.01.62",
        "; enable_support_ironing = 1",
        "; Z_HEIGHT: 0.5",
        "M1031 S1 ;IRONING_EXTRUSIONS_START",
        "; FEATURE: Support ironing",
        ";LINE_WIDTH:0.42",
        "G1 X90.0 Y90.0 E0.01",
        "M1031 S0 ;IRONING_EXTRUSIONS_END",
        "; WIPE_START",
        "G1 X90.0 Y90.0 E-0.38",
        "; WIPE_END",
        "G1 E-0.02",
        "M204 S5000",
        "G17",
        "M1031 S1 ;IRONING_EXTRUSIONS_START",
        "G1 X95.0 Y95.0 E0.02",
        "M1031 S0 ;IRONING_EXTRUSIONS_END",
        "; FEATURE: Outer wall",
        "G1 X100.0 Y100.0 E0.1",
    ]);
    let (output, _) = run_first_pass(&gcode, fixture_ir());
    assert_eq!(
        count_occurrences(&output, "M1031 S1"),
        0,
        "两个 M1031 S1 块都应被删"
    );
    assert!(!contains_line(&output, "G1 X95.0 Y95.0 E0.02"));
    assert!(contains_line(&output, "G1 E-0.02"), "块间移动应保留");
    assert!(contains_line(&output, "M204 S5000"));
    assert!(contains_line(&output, "G17"));
    assert!(contains_line(&output, "G1 X100.0 Y100.0 E0.1"));
}

/// KNOWN BUG RISK-003：copyFlag=false 时 CHANGE_LAYER 无法清除标志。
#[test]
fn ironing_removal_change_layer_cannot_clear_flag() {
    let gcode = lines(&[
        "; BambuStudio 02.07.01.62",
        "; enable_support_ironing = 1",
        "; Z_HEIGHT: 0.5",
        "; FEATURE: Ironing",
        "G1 X80.0 Y80.0 E0.1",
        "; CHANGE_LAYER",
        "G1 X50.0 Y50.0 E0.5",
        "; FEATURE: Outer wall",
        "G1 X100.0 Y100.0 E0.1",
    ]);
    let (output, _) = run_first_pass(&gcode, fixture_ir());
    assert!(!contains_line(&output, "G1 X80.0 Y80.0 E0.1"));
    assert!(
        !has_exact_line(&output, "G1 X50.0 Y50.0 E0.5"),
        "RISK-003：CHANGE_LAYER 后带 E 的行应被跳过"
    );
    assert!(contains_line(&output, "CHANGE_LAYER"));
    assert!(contains_line(&output, "G1 X100.0 Y100.0 E0.1"));
}

/// 缺 WIPE_END：残留到下一个非熨平 FEATURE（L1029 兜底有效）。
#[test]
fn ironing_removal_missing_wipe_end_residual_to_feature() {
    let gcode = lines(&[
        "; BambuStudio 02.07.01.62",
        "; enable_support_ironing = 1",
        "; Z_HEIGHT: 0.5",
        "M1031 S1 ;IRONING_EXTRUSIONS_START",
        "; FEATURE: Support ironing",
        ";LINE_WIDTH:0.42",
        "G1 X90.0 Y90.0 E0.01",
        "M1031 S0 ;IRONING_EXTRUSIONS_END",
        "; FEATURE: Outer wall",
        "G1 X100.0 Y100.0 E0.1",
    ]);
    let (output, _) = run_first_pass(&gcode, fixture_ir());
    assert!(!contains_line(&output, "G1 X90.0 Y90.0 E0.01"));
    assert!(!contains_line(&output, "M1031"));
    assert!(contains_line(&output, "G1 X100.0 Y100.0 E0.1"));
}

/// KNOWN BUG RISK-004：缺 WIPE_END 且无后续 FEATURE → 残留至文件末尾。
#[test]
fn ironing_removal_missing_wipe_end_residual_to_file_end() {
    let gcode = lines(&[
        "; BambuStudio 02.07.01.62",
        "; enable_support_ironing = 1",
        "; Z_HEIGHT: 0.5",
        "; FEATURE: Ironing",
        "G1 X80.0 Y80.0 E0.1",
        "G1 X100.0 Y100.0 E0.1",
        "G1 X110.0 Y110.0 E0.2",
    ]);
    let (output, _) = run_first_pass(&gcode, fixture_ir());
    assert!(!contains_line(&output, "G1 X80.0 Y80.0 E0.1"));
    assert!(
        !contains_line(&output, "G1 X100.0 Y100.0 E0.1"),
        "RISK-004 残留"
    );
    assert!(
        !contains_line(&output, "G1 X110.0 Y110.0 E0.2"),
        "RISK-004 残留"
    );
}

// ---- copyFlag / actFlag（RISK-005 / RISK-010）----

#[test]
fn copy_flag_support_interface_sets_and_clears() {
    let gcode = lines(&[
        "; BambuStudio 02.07.01.62",
        ";===== machine: A1",
        "; Z_HEIGHT: 0.5",
        "; FEATURE: Support interface",
        "G1 X80.0 Y80.0 E0.1",
        "; CHANGE_LAYER",
        "; layer num/total_layer_count: 1/10",
        "; FEATURE: Outer wall",
        "G1 X100.0 Y100.0 E0.1",
    ]);
    let (output, _) = run_first_pass(&gcode, fixture_ir());
    assert!(
        contains_line(&output, ";Pre-glue preparation"),
        "有效支撑面应触发涂胶块"
    );
    assert!(contains_line(&output, "G1 X100.0 Y100.0 E0.1"));
}

#[test]
fn act_flag_invalid_interface_no_glue_block() {
    let gcode = lines(&[
        "; BambuStudio 02.07.01.62",
        ";===== machine: A1",
        "; Z_HEIGHT: 0.5",
        "; FEATURE: Support interface",
        "G1 X80.0 Y80.0",
        "; CHANGE_LAYER",
        "; layer num/total_layer_count: 1/10",
        "; FEATURE: Outer wall",
        "G1 X100.0 Y100.0 E0.1",
    ]);
    let (output, _) = run_first_pass(&gcode, fixture_ir());
    assert!(
        !contains_line(&output, ";Pre-glue preparation"),
        "无效支撑面（无挤出）不应发射涂胶块"
    );
}

#[test]
fn act_flag_layer_num_clears_flag() {
    let gcode = lines(&[
        "; BambuStudio 02.07.01.62",
        ";===== machine: A1",
        "; Z_HEIGHT: 0.5",
        "; FEATURE: Support interface",
        "G1 X80.0 Y80.0 E0.1",
        "; CHANGE_LAYER",
        "; layer num/total_layer_count: 1/10",
        "; Z_HEIGHT: 1.0",
        "; FEATURE: Support interface",
        "G1 X90.0 Y90.0 E0.2",
        "; CHANGE_LAYER",
        "; layer num/total_layer_count: 2/10",
        "; FEATURE: Outer wall",
        "G1 X100.0 Y100.0 E0.1",
    ]);
    let (output, _) = run_first_pass(&gcode, fixture_ir());
    assert_eq!(
        count_occurrences(&output, ";Pre-glue preparation"),
        2,
        "每层各一个涂胶块"
    );
}

/// KNOWN BUG RISK-005：缺 ; layer num 时 actFlag 跨层残留（陈旧 iface 出胶）。
#[test]
fn act_flag_missing_layer_num_residual() {
    let gcode = lines(&[
        "; BambuStudio 02.07.01.62",
        ";===== machine: A1",
        "; Z_HEIGHT: 0.5",
        "; FEATURE: Support interface",
        "G1 X80.0 Y80.0 E0.1",
        "; CHANGE_LAYER",
        "; layer num/total_layer_count: 1/10",
        "; Z_HEIGHT: 1.0",
        "; FEATURE: Support interface",
        "G1 X70.0 Y70.0 E0.3",
        "; CHANGE_LAYER",
        "; Z_HEIGHT: 1.5",
        "; FEATURE: Support interface",
        "G1 X60.0 Y60.0 E0.4",
        "; CHANGE_LAYER",
        "; layer num/total_layer_count: 3/10",
        "; FEATURE: Outer wall",
        "G1 X100.0 Y100.0 E0.1",
    ]);
    let (output, _) = run_first_pass(&gcode, fixture_ir());
    assert_eq!(
        count_occurrences(&output, ";Pre-glue preparation"),
        2,
        "RISK-005：层 2 接口被跳过（残留 actFlag），只有层 1 与层 3 出胶"
    );
}

/// KNOWN BUG RISK-006 前置：无效接口 → 涂胶块跳过 → 辅助标志不重置。
#[test]
fn aux_flags_invalid_interface_no_glue_block_residual() {
    let gcode = lines(&[
        "; BambuStudio 02.07.01.62",
        ";===== machine: A1",
        "; Z_HEIGHT: 0.5",
        "; FEATURE: Support interface",
        "G1 X80.0 Y80.0",
        "; CHANGE_LAYER",
        "; layer num/total_layer_count: 1/10",
        "; FEATURE: Outer wall",
        "G1 X100.0 Y100.0 E0.1",
    ]);
    let (output, _) = run_first_pass(&gcode, fixture_ir());
    assert!(!contains_line(&output, ";Pre-glue preparation"));
}

// ---- Phase 2：风扇状态跟踪与恢复 ----

#[test]
fn fan_state_tracking_m106_s() {
    let gcode = lines(&[
        "; BambuStudio 02.07.01.62",
        "; Z_HEIGHT: 0.5",
        "M106 S124.95",
        "; FEATURE: Outer wall",
        "G1 X100.0 Y100.0 E0.1",
    ]);
    let mut ir = fixture_ir();
    ir.wiping.fan_speed = 255.0;
    let (_, ir) = run_first_pass(&gcode, ir);
    assert!((ir.state.current_fan_speed - 124.95).abs() < 1e-9);
    assert!(ir.state.current_fan_speed_set);
    assert_eq!(ir.wiping.fan_speed, 255.0, "配置 SSOT 不得被覆写");
}

#[test]
fn fan_state_tracking_m106_p1_s() {
    let gcode = lines(&[
        "; BambuStudio 02.07.01.62",
        "; Z_HEIGHT: 0.5",
        "M106 P1 S100",
        "; FEATURE: Outer wall",
        "G1 X100.0 Y100.0 E0.1",
    ]);
    let mut ir = fixture_ir();
    ir.wiping.fan_speed = 255.0;
    let (_, ir) = run_first_pass(&gcode, ir);
    assert!((ir.state.current_fan_speed - 100.0).abs() < 1e-9);
    assert!(ir.state.current_fan_speed_set);
}

#[test]
fn fan_state_tracking_no_m106_set_false() {
    let gcode = lines(&[
        "; BambuStudio 02.07.01.62",
        "; Z_HEIGHT: 0.5",
        "; FEATURE: Outer wall",
        "G1 X100.0 Y100.0 E0.1",
    ]);
    let (_, ir) = run_first_pass(&gcode, fixture_ir());
    assert!(!ir.state.current_fan_speed_set);
}

#[test]
fn fan_state_tracking_m106_s0_zero_is_valid() {
    let gcode = lines(&[
        "; BambuStudio 02.07.01.62",
        "; Z_HEIGHT: 0.5",
        "M106 S0",
        "; FEATURE: Outer wall",
        "G1 X100.0 Y100.0 E0.1",
    ]);
    let (_, ir) = run_first_pass(&gcode, fixture_ir());
    assert_eq!(ir.state.current_fan_speed, 0.0);
    assert!(ir.state.current_fan_speed_set, "S0 是合法值");
}

/// restoreFanState：未置位不恢复（端到端形态：无 M106 输入 → 涂胶块无恢复行）。
#[test]
fn restore_fan_state_not_set_no_restore() {
    let gcode = lines(&[
        "; BambuStudio 02.07.01.62",
        "; Z_HEIGHT: 0.5",
        "; FEATURE: Support interface",
        "G1 X80.0 Y80.0 E0.1",
        "; CHANGE_LAYER",
        "; layer num/total_layer_count: 1/10",
    ]);
    let (output, _) = run_first_pass(&gcode, fixture_ir());
    assert!(!contains_line(&output, ";Restore fan state"));
}

/// restoreFanState：单风扇机型恢复 M106 S<原值>（端到端）。
#[test]
fn restore_fan_state_single_fan_restores_correctly() {
    let gcode = lines(&[
        "; BambuStudio 02.07.01.62",
        "; Z_HEIGHT: 0.5",
        "M106 S124.95",
        "; FEATURE: Support interface",
        "G1 X80.0 Y80.0 E0.1",
        "; CHANGE_LAYER",
        "; layer num/total_layer_count: 1/10",
    ]);
    let (output, _) = run_first_pass(&gcode, fixture_ir());
    assert!(contains_line(&output, ";Restore fan state"));
    assert!(contains_line(&output, "M106 S124.95"));
    assert!(!contains_line(&output, "M106 P1 S124.95"), "单风扇不用 P1");
}
