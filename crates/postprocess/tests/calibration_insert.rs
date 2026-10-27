//! Task 13 判据：校准插入（insert_calibration_gcode）。
//!
//! 字节锚点：生成器本体已由 reference_calibration.rs（Task 9.5，四模式逐字节）
//! 钉住；插入编排（filter/minimize/插入点选择）在旧侧**无字节参考**
//! （Task 4.3 只导出了生成器对），此处为行为判据 —— 如实登记，不假称字节级。
//! 三种 exec_mode 各一条（13.5）。

use postprocess::ir::Ir;
use postprocess::postproc::calibration::{Mode, insert_calibration_gcode, minimize_startup_gcode};
use postprocess::postproc::disk::{BBox, CentroidResult};

fn fixture_ir() -> Ir {
    let json = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/ir/golden_42274_2.json"),
    )
    .expect("读不到 IR fixture");
    serde_json::from_str(&json).expect("IR 反序列化失败")
}

fn lines(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

fn sample_bbox() -> Option<BBox> {
    Some(BBox {
        x_min: 50.0,
        x_max: 150.0,
        y_min: 50.0,
        y_max: 150.0,
    })
}

fn centroid() -> CentroidResult {
    CentroidResult {
        centroid_x: 100.0,
        centroid_y: 100.0,
        valid: true,
    }
}

/// print_then_calibrate / ""：不删模型路径，校准段插到 "; filament end gcode" 之后。
#[test]
fn insert_after_print_mode_keeps_model_paths() {
    let mut ir = fixture_ir();
    ir.safety.calibration_execution_mode = "print_then_calibrate".to_string();
    let content = lines(&[
        "; CONFIG_BLOCK_START",
        "; total layer number: 1/1",
        "; CONFIG_BLOCK_END",
        "; FEATURE: Outer wall",
        "G1 X80.0 Y80.0 E0.5",
        "; filament end gcode",
        "M104 S0",
    ]);
    let out = insert_calibration_gcode(
        content,
        Mode::ZOffset,
        &mut ir,
        sample_bbox(),
        centroid(),
        2,
        3,
    )
    .expect("插入失败");
    // 插入点：filament end 之后、M104 S0 之前出现校准运行时块
    let idx_end = out
        .iter()
        .position(|l| l.contains("; filament end gcode"))
        .unwrap();
    let idx_runtime = out
        .iter()
        .position(|l| l == "; Calibration Runtime: retract and cool down")
        .expect("应插入 Calibration Runtime 块");
    assert!(idx_runtime > idx_end);
    assert!(out.contains(&"G92 E0".to_string()));
    assert!(out.iter().any(|l| l.starts_with("G1 E-2.0 F1800")));
    // 模型移动保留（非 direct 模式不删路径）
    assert!(out.iter().any(|l| l == "G1 X80.0 Y80.0 E0.5"));
}

/// direct_calibrate：删模型路径 + 启动段最小化 + E 移动过滤。
#[test]
fn direct_calibrate_filters_model_paths() {
    let mut ir = fixture_ir();
    ir.safety.calibration_execution_mode = "direct_calibrate".to_string();
    let content = lines(&[
        ";===== machine: A1 mini",
        "M140 S60",
        "M190 S60",
        ";===== prepare print temperature and material ==========",
        "M104 S220",
        ";===== prepare print temperature and material end =====",
        "; MACHINE_START_GCODE_END",
        "G1 F9000",
        "; CUSTOM_GCODE",
        "G1 X127.693 Y124.368 F42000",
        "G1 Z.4",
        "G1 Z.2",
        "; FEATURE: Outer wall",
        "G1 F3000",
        "G1 X80.0 Y80.0 E0.5",
        "G1 F10342.643",
        "; filament end gcode",
    ]);
    let out = insert_calibration_gcode(
        content,
        Mode::ZOffset,
        &mut ir,
        sample_bbox(),
        centroid(),
        2,
        3,
    )
    .expect("插入失败");
    // 模型挤出移动被删（direct 模式）
    assert!(!out.iter().any(|l| l == "G1 X80.0 Y80.0 E0.5"));
    // 模型区里「只改速度、不动位置」的 F-only 行一并清掉（移动没了，速度提示就是垃圾）
    assert!(
        !out.iter().any(|l| l == "G1 F3000" || l == "G1 F10342.643"),
        "模型区里不该留下 F-only 行：{out:?}"
    );
    // 区外的 F-only 行**必须留着**：它改的是后续移动的模态进给，删了就是改了真机速度
    assert!(
        out.iter().any(|l| l == "G1 F9000"),
        "模型区外的 G1 F9000 被误删：{out:?}"
    );
    // `; CUSTOM_GCODE` 之后的「对象起点空驶 + 把 Z 压到首层高」也要清：
    // 模型路径删光之后它就是「跑到板子正中点一下」，真机上笔/喷嘴会蹭板
    assert!(
        !out.iter()
            .any(|l| l.contains("X127.693") || l == "G1 Z.4" || l == "G1 Z.2"),
        "对象起点空驶没被清掉：{out:?}"
    );
    // M190 被删、M140 降温到 S1（A1_MINI 启动段最小化）
    assert!(!out.iter().any(|l| l.starts_with("M190")));
    assert!(out.iter().any(|l| l.starts_with("M140 S1")));
    // prepare 区高温 M104 被删
    assert!(!out.iter().any(|l| l == "M104 S220"));
    // 校准块仍在
    assert!(
        out.iter()
            .any(|l| l == "; Calibration Runtime: retract and cool down")
    );
}

/// A1 直接校准：`G28` 抬 Z 保命，归零之后那一整段「去清洗位 + 落 Z + 升温」全部删掉。
///
/// 原样跑的话：归零结束时 Z 就在 0（喷嘴贴板），紧接着 `G1 X108 Y-0.5` 从板中心横穿
/// 到前缘 —— 真机上喷嘴（和笔架）直接刮过去；而那段清洗线在模板里本来就是注释掉的，
/// 所以"走到位"之后什么也不做。直接校准不打印，这一整段没有存在意义。
#[test]
fn a1_direct_mode_drops_the_pre_purge_block_and_keeps_the_lift() {
    let content = lines(&[
        ";===== machine: A1",
        ";===== wipe nozzle",
        ";===== wipe nozzle end",
        "M104 S1",
        ";===== home after wipe mouth=====",
        "    M1002 gcode_claim_action : 13",
        "    G28",
        ";===== home after wipe mouth end =====",
        "G1 X108.000 Y-0.500 F30000",
        "G1 Z0.300 F1200",
        "M400",
        "G2814 Z0.32",
        "M104 S245 ; prepare to print",
        ";===== nozzle load line",
        ";G1 X88 Y-0.5 F20000",
        ";===== nozzle load line end",
        ";===== extrude cali test",
        "    M109 S245",
    ]);

    let a1 = minimize_startup_gcode(content.clone(), "A1");
    let pos = |needle: &str| a1.iter().position(|l| l.trim() == needle);
    let g28 = pos("G28").expect("G28 应保留");
    let lift = pos("G1 Z10 F1200").expect("G28 之后应插入抬 Z");
    assert!(g28 < lift, "抬 Z 必须在 G28 之后：{a1:?}");

    for gone in [
        "G1 X108.000 Y-0.500 F30000",
        "G1 Z0.300 F1200",
        "G2814 Z0.32",
    ] {
        assert!(
            !a1.iter().any(|l| l.trim() == gone),
            "直接校准不该保留 {gone}：{a1:?}"
        );
    }
    assert!(
        !a1.iter().any(|l| l.contains("M104 S245")),
        "直接校准不该升到打印温度：{a1:?}"
    );
    // 后段标记必须还在，否则下游的 load line / extrude cali 处理会整段失联
    assert!(a1.iter().any(|l| l.contains(";===== nozzle load line")));
    assert!(a1.iter().any(|l| l.contains(";===== extrude cali test")));

    // P1 走的是另一条裁剪路径：同样不吃这段准备动作，但不该拿到 A1 那条抬 Z
    let p1 = minimize_startup_gcode(content, "P1S");
    assert!(
        !p1.iter().any(|l| l.trim() == "G1 X108.000 Y-0.500 F30000"),
        "P1S 也不该保留这段准备动作：{p1:?}"
    );
    assert!(
        !p1.iter().any(|l| l.trim() == "G1 Z10 F1200"),
        "P1S 不该借用 A1 那条抬 Z：{p1:?}"
    );
}

/// 振动补偿被删之后，为它摆位的那些移动必须一起走 —— 否则真机上就是「跑到中间干等」。
/// 但 `G0 X25 Y175 ; find a soft place to home` 与 `G28 Z` 是给 Z 归零找软地的，留着。
#[test]
fn mech_mode_moves_leave_together_with_the_vibration_commands() {
    let content = lines(&[
        ";===== machine: A1 mini",
        ";===== mech mode fast check============================",
        "M1002 gcode_claim_action : 3",
        "G0 X25 Y175 F20000 ; find a soft place to home",
        "G28 Z P0 T300; home z with low precision,permit 300deg temperature",
        "G29.2 S0 ; turn off ABL",
        "M104 S170",
        "G1 Z5 F3000",
        "G1 X90 Y-1 F30000",
        "M400 P200",
        "M970.3 Q1 A7 K0 O2",
        "M974 Q1 S2 P0",
        "G1 X90 Y0 Z5 F30000",
        "M970 Q0 A10 B50 C90 H15 K0 M20 O3",
        "M974 Q0 S2 P0",
        "M975 S1",
        "G1 F30000",
        "G1 X-1 Y10",
        "G28 X ; re-home XY",
        ";===== bed leveling ==================================",
        "G29 A1 X21 Y72 I136 J34",
        ";===== bed leveling end ================================",
        "; TAIL_MARK",
    ]);

    let out = minimize_startup_gcode(content, "A1_MINI");
    assert!(
        out.iter().any(|l| l.contains("G0 X25 Y175")),
        "找软地的定位要留：{out:?}"
    );
    assert!(
        out.iter().any(|l| l.contains("G28 Z P0 T300")),
        "Z 归零要留：{out:?}"
    );
    for gone in ["G1 Z5 F3000", "G1 X-1 Y10"] {
        assert!(
            !out.iter().any(|l| l.trim() == gone),
            "振动测量摆位 {gone} 该删：{out:?}"
        );
    }
    assert!(!out.iter().any(|l| l.contains("X90 Y-1")));
    assert!(!out.iter().any(|l| l.contains("X90 Y0 Z5")));
    assert!(
        !out.iter()
            .any(|l| l.starts_with("M970") || l.starts_with("M974")),
        "M970/M974 本来就不该留：{out:?}"
    );
    // 区间边界要正确收口：后面的内容既不被卷进 mech 段，也不被整段吞掉
    assert!(
        out.iter().any(|l| l.contains("TAIL_MARK")),
        "mech 段之后的尾巴被吞了：{out:?}"
    );
}

/// P1 直接校准：擦嘴之后的自检空跑与"提前升温"整块不要，但清洗线段落必须接住。
#[test]
fn p1_direct_mode_drops_the_mech_check_and_print_preheat() {
    let content = lines(&[
        ";===== machine: P1S-0.4",
        ";===== wipe nozzle =====",
        "M109 S225",
        ";===== wipe nozzle end =====",
        "M104 S1",
        ";===== home after wipe mouth=====",
        ";===== home after wipe mouth end =====",
        "M106 P2 S100 ; turn on big fan",
        "M104 S245 ; set extrude temp earlier, to reduce wait time",
        "G1 X128 Y128 Z10 F20000",
        "G1 X128 Y128 Z10 F20000",
        "G28 X ; re-home XY",
        ";===== nozzle load line ===============================",
        "M104 S30",
        "M400",
        "; MACHINE_START_GCODE_END",
    ]);

    let p1 = minimize_startup_gcode(content, "P1S");
    assert!(
        p1.iter().any(|l| l.contains(";===== nozzle load line")),
        "清洗线段落标记必须保留：{p1:?}"
    );
    assert!(
        !p1.iter().any(|l| l.contains("M104 S245")),
        "直接校准不该提前升到打印温度：{p1:?}"
    );
    assert!(
        !p1.iter().any(|l| l.contains("X128 Y128 Z10")),
        "mech mode 自检的两次空跑该删：{p1:?}"
    );
    assert!(
        !p1.iter().any(|l| l.trim().starts_with("G28 X")),
        "自检之后的 re-home 一并删：{p1:?}"
    );
    assert!(
        p1.iter().any(|l| l.trim() == "M104 S30"),
        "清洗线段的降温要留下：{p1:?}"
    );
}

/// X1C 与 P1S 共用同一条起始段裁剪（来源仓库 helpers.go:991 就是这么分的）。
/// 差别只在 `preserve_nozzle_load_line`：X1C 不在那个白名单里，清洗线段落连标记
/// 都会被更早删掉 —— 所以这里的出口判据不能只认 `;===== nozzle load line`。
#[test]
fn x1c_shares_the_p1_cut_without_leaking_past_the_boundary() {
    let content = lines(&[
        ";===== machine: X1C-0.4",
        ";===== wipe nozzle =====",
        ";===== wipe nozzle end =====",
        "M104 S1",
        ";===== home after wipe mouth=====",
        ";===== home after wipe mouth end =====",
        "M104 S245 ; set extrude temp earlier",
        "G1 X128 Y128 Z10 F20000",
        "G28 X ; re-home XY",
        ";========turn off light and wait extrude temperature =============",
        "M106 S0 ; turn off fan",
        "; MACHINE_START_GCODE_END",
        "; filament start gcode",
    ]);

    let out = minimize_startup_gcode(content, "X1C");
    assert!(
        !out.iter().any(|l| l.contains("M104 S245")),
        "X1C 也要砍掉提前升温：{out:?}"
    );
    assert!(
        !out.iter().any(|l| l.contains("X128 Y128 Z10")),
        "X1C 也要砍掉自检空跑：{out:?}"
    );
    // 边界之后的内容不许被吞掉
    assert!(
        out.iter()
            .any(|l| l.contains("turn off light and wait extrude temperature")),
        "边界标记被吞了：{out:?}"
    );
    assert!(
        out.iter().any(|l| l.trim() == "M106 S0 ; turn off fan"),
        "边界之后的关风扇被吞了：{out:?}"
    );
    assert!(
        out.iter().any(|l| l.contains("; filament start gcode")),
        "启动段之外的内容被吞了：{out:?}"
    );
}

/// 直接校准：A1 启动段里带缩进的 `M109 S245` 必须变成**不阻塞**的 `M104 S30`。
/// 只改温度不改命令会留下 `M109 S30`，机器会停在那里等喷嘴降温（2026-10-19 真机）。
#[test]
fn direct_mode_never_leaves_a_blocking_m109_cooldown() {
    let mut ir = fixture_ir();
    ir.safety.calibration_execution_mode = "direct_calibrate".to_string();
    let content = lines(&[
        ";===== machine: A1 mini =====",
        ";===== extrude cali test ===============================",
        "M400",
        "    M900 S",
        "    M900 C",
        "    G90",
        "    M83",
        "    M109 S220",
        "    G91",
        "    G90",
        "    M400",
        "M900 R",
        "; MACHINE_START_GCODE_END",
        // 纯 E 行（省略整数位）：direct 模式必须被 filter_e_moves 删掉
        "G1 E.8 F1800",
        "; filament end gcode",
    ]);
    let out = insert_calibration_gcode(
        content,
        Mode::ZOffset,
        &mut ir,
        sample_bbox(),
        centroid(),
        2,
        3,
    )
    .expect("插入失败");

    assert!(
        out.iter().any(|l| l == "    M104 S30"),
        "带缩进的 M109 应改成不阻塞的 M104 S30"
    );
    assert!(
        !out.iter().any(|l| l.trim() == "M109 S30"),
        "不许留下阻塞式 M109 S30"
    );
    assert!(
        !out.iter().any(|l| l.trim() == "G1 E.8 F1800"),
        "纯 E 行（E.8）应被 filter_e_moves 删掉"
    );
}

/// 空串 exec_mode 与 print_then_calibrate 同路（回退归一在 ir::build，此处验证行为一致）。
#[test]
fn empty_exec_mode_behaves_like_print_then() {
    let content = lines(&[
        "; FEATURE: Outer wall",
        "G1 X80.0 Y80.0 E0.5",
        "; filament end gcode",
    ]);
    let mut ir_a = fixture_ir();
    ir_a.safety.calibration_execution_mode = String::new();
    let mut ir_b = fixture_ir();
    ir_b.safety.calibration_execution_mode = "print_then_calibrate".to_string();
    let out_a = insert_calibration_gcode(
        content.clone(),
        Mode::ZOffset,
        &mut ir_a,
        sample_bbox(),
        centroid(),
        2,
        3,
    )
    .unwrap();
    let out_b = insert_calibration_gcode(
        content,
        Mode::ZOffset,
        &mut ir_b,
        sample_bbox(),
        centroid(),
        2,
        3,
    )
    .unwrap();
    assert_eq!(out_a, out_b);
    assert!(
        out_a.iter().any(|l| l == "G1 X80.0 Y80.0 E0.5"),
        "两种模式都保留模型路径"
    );
}

/// 无 filament end 标记：校准块追加到文件末尾（新模式先加 retract 块）。
#[test]
fn insert_without_filament_end_appends() {
    let mut ir = fixture_ir();
    let content = lines(&["; CONFIG_BLOCK_START", "x = 1", "; CONFIG_BLOCK_END"]);
    let out = insert_calibration_gcode(content, Mode::Repetition, &mut ir, None, centroid(), 2, 3)
        .expect("插入失败");
    // Repetition 非 isNewMode（bbox=None）⇒ 无 retract 块，只有校准行
    assert!(!out.iter().any(|l| l.contains("Calibration Runtime")));
    assert!(out.len() > 3, "校准行已追加");
}

/// 振动检测移除（mech mode 区内的 M970/M974）。
#[test]
fn vibration_detection_removed_in_mech_mode() {
    let content = lines(&[
        ";===== mech mode fast check",
        "M970 S1",
        "M974 X1",
        "M104 S220",
        ";===== other region",
        "M970 S9",
    ]);
    let out =
        postprocess::postproc::calibration::remove_vibration_detection_from_mech_mode(&content);
    assert!(
        !out.iter()
            .any(|l| l.starts_with("M970 S1") || l == "M974 X1"),
        "区内 M970/M974 删除"
    );
    assert!(out.iter().any(|l| l == "M104 S220"), "区内其他命令保留");
    assert!(out.iter().any(|l| l == "M970 S9"), "区外 M970 保留");
}

/// 形变检测的错误码必须原样穿过插入层。
///
/// `insert_calibration_gcode` 里那道 `map_err` 会把生成器的错误统统包成
/// `E_CAL_FAILED_001`；形变检测两个码是例外 —— 它们是用户能照做的提示
/// （「把校准模型转回来」），退化成「校准失败」等于把提示变成故障。
#[test]
fn mismatch_code_survives_the_insert_layer() {
    let mut ir = fixture_ir();
    ir.safety.xy_calibration = "new".to_string();
    let content = lines(&["; CONFIG_BLOCK_START", "; CONFIG_BLOCK_END"]);
    // sample_bbox 是 100×100，XY 图案要求 50.5×50.5 ⇒ 检测二命中
    let err = insert_calibration_gcode(
        content,
        Mode::XyPrecise,
        &mut ir,
        sample_bbox(),
        centroid(),
        2,
        3,
    )
    .expect_err("尺寸不符应当报错");
    assert_eq!(err.code(), "E_CAL_MISMATCH_002", "码被插入层吃掉了: {err}");
    assert!(err.to_string().contains("请勿缩放校准模型"), "{err}");
}

/// 反面：非形变的生成失败仍然要拿到 `E_CAL_FAILED_001`（穿透不能穿过头）。
#[test]
fn other_calibration_failures_still_get_the_generic_code() {
    let mut ir = fixture_ir();
    ir.machine.machine_type = "UNKNOWN_PRINTER".to_string();
    let content = lines(&["; CONFIG_BLOCK_START", "; CONFIG_BLOCK_END"]);
    // 机型认不出 ⇒ 经典分支拿不到机型维度 ⇒ 生成器返回 Ok(None) ⇒ 走「无输出」那条
    let err = insert_calibration_gcode(content, Mode::XyPrecise, &mut ir, None, centroid(), 2, 3)
        .expect_err("未知机型应当报错");
    assert_eq!(err.code(), "E_CAL_FAILED_001", "{err}");
}
