//! **身份透传判据**（2026-10-19）。
//!
//! 这个文件只咬一件事：**调用方把 `calibration` 塞进请求之后，整条管线的下游能不能
//! 看到它**。
//!
//! # 为什么单开一个文件
//!
//! 2026-10-19 出过一个很"安静"的 bug：`IrProcessRequest.calibration` 字段加好了，
//! 钩子也算对了 `ZOffset`，但 `pipeline/mod.rs` 里那几个包装函数在构造内部 `Job` 时
//! 写死了 `calibration: Mode::None` —— 调用方传什么都不算数。
//!
//! 它**编译通过、原有测试全绿**，因为：
//!
//! - 判据都在测 `resolve_identity` / `mode_of`（身份算得对不对）；
//! - 另一个端到端判据跑的是 CLI 那条 `ProcessRequest`（本来就没有身份）。
//!
//! 「算对了」和「接到了」分成两半各测一遍，中间那一跳就没人守。这个文件就是守那一跳的。
//!
//! # 守的是什么
//!
//! 请求里给什么身份 ⇒
//! 1. `DetailFacts.calibration_mode`（存档 `detail.calibration.mode` 的来源）必须是它；
//! 2. `Step::CalibCheck` 真的跑了（第 7 步 = 认档），且 `calibration_receipt` 有实测尺寸；
//! 3. 输出正文里插了校准段。
//!
//! 三条一起断言才叫"透传成立" —— 只看第一条的话，一个只回填字段、不走校准分支的
//! 实现也能骗过判据。

#![allow(clippy::disallowed_methods)]

use std::path::{Path, PathBuf};

use postprocess::diag::{CancelToken, PostprocError};
use postprocess::ir::Ir;
use postprocess::pipeline::{
    self, IrProcessRequest, ProgressEvent, ProgressSink, Step, process_with_ir_with_tower_ask,
};
use postprocess::postproc::calibration::Mode;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// 一份**尺寸对得上 `zoffset` 档**的校准件切片（在测试里现造）。
///
/// 为什么不用 `fixtures/real/` 里那份 `rib-tall-min.gcode`：那是斜肋高塔，
/// 包围盒 171 × 54 —— 拿它当 Z 板会被 `resolve_profile` 判 `E_CAL_MISMATCH_002`，
/// 判据就成了"在测档位不匹配"，不是"在测透传"。
///
/// 真校准件 1~2MB 且带用户模型，按 `fixtures/real/README.md` 的规矩不入库；
/// 这里只要**外形落在 `zoffset` 档（119.58 × 13.063，容差 0.5）内**，
/// 外壳 + 少量填充线足够让 `compute_bbox` 测出那个尺寸。
fn write_calib_gcode(path: &Path) {
    /* 外形取 119.58 × 13.0 —— 正落在 zoffset 档里（偏差 0.063 < 容差 0.5） */
    let (x0, x1) = (68.0_f64, 187.58_f64);
    let (y0, y1) = (121.0_f64, 134.0_f64);

    let mut g = String::new();
    g.push_str("; HEADER_BLOCK_START\n");
    g.push_str("; BambuStudio 02.08.02.60\n");
    g.push_str("; total layer number: 2\n");
    g.push_str("; max_z_height: 0.40\n");
    g.push_str("; HEADER_BLOCK_END\n\n");
    g.push_str("; printable_area = 0x0,256x0,256x256,0x256\n");
    g.push_str("; printer_model = Bambu Lab A1\n\n");
    g.push_str("G28\nG90\nM83\n");

    /* 两层：每层一圈矩形外壳 + 四条填充，带挤出（E 正数）才会进包围盒 */
    for layer in 0..2 {
        let z = 0.2 * (layer as f64 + 1.0);
        g.push_str(&format!("; LAYER_CHANGE\nG1 Z{z:.2} F720\n"));
        g.push_str(&format!("G1 X{x0:.3} Y{y0:.3} F6000\n"));
        for (x, y) in [(x1, y0), (x1, y1), (x0, y1), (x0, y0)] {
            g.push_str(&format!("G1 X{x:.3} Y{y:.3} E0.03 F3000\n"));
        }
        for fx in [x0 + 30.0, x0 + 60.0, x0 + 90.0] {
            g.push_str(&format!("G1 X{fx:.3} Y{y0:.3} E0.01\n"));
            g.push_str(&format!("G1 X{fx:.3} Y{y1:.3} E0.02\n"));
        }
    }
    g.push_str("\n; EXECUTABLE_BLOCK_END\n");
    std::fs::write(path, g).expect("写合成校准 G-code");
}

fn config_a1() -> PathBuf {
    repo_root().join("tests/fixtures/config/A1.toml")
}

struct NoProgress;

impl ProgressSink for NoProgress {
    fn emit(&mut self, _e: ProgressEvent) {}
}

/// 造一份"Z 校准按新型走"的 IR —— 只有 `z_calibration == "new"` 时
/// `is_dynamic` 才为真、第 7 步（认档）才会跑。
fn z_new_ir(raw: &str) -> Ir {
    let overrides = ["Safety.ZCalibration=new".to_string()];
    pipeline::resolve_ir(&config_a1(), &overrides, Some(raw)).expect("配置 → IR（A1 + Z 校准新型）")
}

fn copy_input(dir: &Path) -> PathBuf {
    let work = dir.join("input.gcode");
    write_calib_gcode(&work);
    work
}

/// 三条断言一次做完：存档说它是 ZOffset、第 7 步真跑了、正文里插了校准段。
fn assert_passthrough(
    label: &str,
    request: IrProcessRequest,
    work: &Path,
    run: impl FnOnce(IrProcessRequest) -> Result<postprocess::pipeline::ProcessResult, PostprocError>,
) {
    let done = run(request).unwrap_or_else(|e| panic!("{label}：管线失败 {e:?}"));

    // ① 存档里那个"最终用的模式"（`detail.calibration.mode` 的来源）
    assert_eq!(
        done.detail_facts.calibration_mode, "ZOffset",
        "{label}：请求给的是 ZOffset，`DetailFacts.calibration_mode` 却是「{}」——\
         包装函数把调用方的参数覆盖掉了",
        done.detail_facts.calibration_mode
    );

    // ② 第 7 步（认档）必须真的跑过，而且要留下实测尺寸
    let ran_calib_check = done.step_timings.iter().any(|s| s.step == Step::CalibCheck);
    assert!(
        ran_calib_check,
        "{label}：ZOffset + Z 校准新型 ⇒ 第 7 步认档必须跑，实际步骤：{:?}",
        done.step_timings.iter().map(|s| s.step).collect::<Vec<_>>()
    );
    let receipt = done
        .calibration_receipt
        .as_ref()
        .unwrap_or_else(|| panic!("{label}：认档回执为空 —— 第 7 步跑了却没带回事实"));
    assert!(receipt.measured.is_some(), "{label}：回执里没有实测包围盒");

    // ③ 正文里真的插了校准段（证明不只是回填了个字段）
    let out = std::fs::read_to_string(work).unwrap_or_else(|e| panic!("{label}：读输出 {e}"));
    assert!(
        out.contains("ZOffset Calibration"),
        "{label}：输出里没有校准段 —— 身份到了记录、没到正文"
    );
}

// ---------- 一次遍历所有带身份的入口 ----------
//
// 每个入口单独一条判据（而不是一个 for 循环），这样红的时候一眼看出是哪个包装函数漏了。

/// **正式主链（非延后）**：`hook.rs:863` 走的就是这个函数 —— 不是 `process_with_ir`。
///
/// ★ 这里踩过一次：第一版写的是 `process_with_ir`，它走 354 行那条；
/// 而 bug 在 388 行（`_with_tower_ask`）。判据"通过"了，坏代码照样活着。
/// 判据必须调**产品真正调用的那个函数**，不能调"名字更顺眼的那个"。
#[test]
fn the_tower_ask_wrapper_passes_the_identity_through() {
    let dir = tempfile::tempdir().unwrap();
    let work = copy_input(dir.path());
    let raw = std::fs::read_to_string(&work).unwrap();
    let cancel = CancelToken::new();
    let mut sink = NoProgress;

    assert_passthrough(
        "process_with_ir_with_tower_ask",
        IrProcessRequest {
            gcode_path: work.clone(),
            ir: z_new_ir(&raw),
            output_path: None,
            calibration: Mode::ZOffset,
        },
        &work,
        |req| process_with_ir_with_tower_ask(req, None, &mut sink, &cancel),
    );
}

/// **正式主链（延后算打印时间 + 带问句）**：`hook.rs` 的另一支。
#[test]
fn the_deferred_wrapper_with_ask_passes_the_identity_through() {
    let dir = tempfile::tempdir().unwrap();
    let work = copy_input(dir.path());
    let raw = std::fs::read_to_string(&work).unwrap();
    let cancel = CancelToken::new();
    let mut sink = NoProgress;

    assert_passthrough(
        "process_with_ir_deferred_print_time_with_tower_ask",
        IrProcessRequest {
            gcode_path: work.clone(),
            ir: z_new_ir(&raw),
            output_path: None,
            calibration: Mode::ZOffset,
        },
        &work,
        |req| {
            pipeline::process_with_ir_deferred_print_time_with_tower_ask(
                req, None, &mut sink, &cancel,
            )
        },
    );
}

/// **反证**：不给身份 ⇒ 必须真的是普通模型那一趟。
///
/// 少了这条，上面三条都可能靠"包装函数恒返回 ZOffset"这种假实现通过。
#[test]
fn no_identity_means_no_calibration_branch() {
    let dir = tempfile::tempdir().unwrap();
    let work = copy_input(dir.path());
    let raw = std::fs::read_to_string(&work).unwrap();
    let cancel = CancelToken::new();
    let mut sink = NoProgress;

    let done = process_with_ir_with_tower_ask(
        IrProcessRequest {
            gcode_path: work.clone(),
            ir: z_new_ir(&raw),
            output_path: None,
            calibration: Mode::None, // 普通模型
        },
        None,
        &mut sink,
        &cancel,
    )
    .expect("普通模型那一趟必须成功");

    assert_eq!(
        done.detail_facts.calibration_mode, "",
        "没给身份，`calibration_mode` 必须是空串（`Mode::None` 的稳定串）"
    );
    assert!(
        !done.step_timings.iter().any(|s| s.step == Step::CalibCheck),
        "普通模型不该跑第 7 步认档"
    );
    assert!(done.calibration_receipt.is_none(), "普通模型不该有认档回执");

    let out = std::fs::read_to_string(&work).unwrap();
    assert!(
        !out.contains("ZOffset Calibration"),
        "普通模型的输出里不该出现校准段"
    );
}
