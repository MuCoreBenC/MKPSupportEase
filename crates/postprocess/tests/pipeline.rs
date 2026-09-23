//! 12 步编排的判据（来源：`mkp-sr` 的 `crates/engine/tests/pipeline.rs`）。
//!
//! **判据资产的来路必须说清**，否则下一个人会以为这份 TOML 是手写的：
//! `tests/fixtures/config/A1.toml` 是从**既有的** IR JSON 判据资产
//! `tests/fixtures/ir/build9/A1.json`（= 来源仓库 `ir::build()` 吃
//! `tests/fixtures/presets/A1.toml` 的输出）用 `config::to_toml` 派生出来的，
//! 派生器与 `mkp-pp init` 是同一条路。[`fixture_is_derived_from_the_ir_json`]
//! 每轮都重新核对这条派生关系 —— 手改 TOML 或改动 JSON 都会让它红。
//!
//! **为什么不用 `golden_42274_2.json`（G1/G2 喂给 pass1 的那份 IR）**：实测它跑不完整条链。
//! 那份 IR 的 `Machine` 运动范围与涂胶区是 **±999**（字节判据直接喂 pass1、从不经过
//! machine-detect 步，所以那个宽到不真实的范围一直没被覆盖）。走完整管线时第 3 步会用
//! **真实机型表**盖掉它，于是涂胶路径落在 `GlueMinY = 0` 之外 ⇒
//! `E_GCODE_BOUNDARY_001`（Y 约 -0.0mm）。那是真的越界，**不放宽范围检查、不改判据资产**，
//! 换一份本来就是从真实预设 build 出来的 IR。
//!
//! 来源仓库那 6 条判据里，**3 条随预设体系一起消失**（缺预设文件 / `# machine:` 头
//! 缺失 / registry 坏枚举 —— 本项目没有预设文件、没有头注释、没有参数注册表）。
//! 换成 3 条对着**新的**失败面的判据：缺配置文件 / 坏 TOML / 机型名不认识。

use mkp_pp::config;
use mkp_pp::diag::CancelToken;
use mkp_pp::ir::{Ir, fill_defaults};
use mkp_pp::pipeline::{ProcessRequest, ProgressEvent, Step, assert_step_order, process};

fn repo_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn golden_input() -> std::path::PathBuf {
    repo_root().join("tests/golden/42274.2.gcode")
}

fn ir_json_path() -> std::path::PathBuf {
    repo_root().join("tests/fixtures/ir/build9/A1.json")
}

fn config_fixture() -> std::path::PathBuf {
    repo_root().join("tests/fixtures/config/A1.toml")
}

/// 派生时施加的**唯一一处**改动：`Machine.MachineType = "A1"`。
///
/// 理由（不是随手填的）：来源仓库的 `ir::build()` **从不设置**这个字段 —— 机型名来自
/// 预设文件的 `# machine:` 头注释，由 engine 在 config 步单独赋值
/// （`crates/engine/src/lib.rs:206`）。本项目没有头注释这个概念，机型名就是配置里的
/// 一个普通字段，所以派生器必须把它填上，否则这份 fixture 是一份**跑不起来**的配置。
const FIXTURE_MACHINE_TYPE: &str = "A1";

fn ir_from_json() -> Ir {
    let raw = std::fs::read_to_string(ir_json_path()).expect("IR JSON 判据资产必须在");
    let mut ir: Ir = serde_json::from_str(&raw).expect("IR JSON 必须能反序列化");
    ir.machine.machine_type = FIXTURE_MACHINE_TYPE.to_string();
    ir
}

/// 重新生成配置 fixture。**具名 ignore，不是静默跳过**：
/// 只有在刻意要重新派生时才跑 `cargo test --test pipeline -- --ignored regenerate`。
///
/// 平时不跑的理由：判据文件被测试自己覆写，就再也测不出「文件被改坏了」。
#[test]
#[ignore = "手动派生工具：cargo test --test pipeline -- --ignored regenerate_config_fixture_from_ir_json"]
fn regenerate_config_fixture_from_ir_json() {
    let toml_text = config::to_toml(&ir_from_json());
    let path = config_fixture();
    std::fs::create_dir_all(path.parent().expect("fixture 必有父目录")).expect("建目录");
    std::fs::write(&path, &toml_text).expect("写 fixture");
    println!("已写入 {} （{} 字节）", path.display(), toml_text.len());
}

/// 派生关系判据：TOML fixture 读回来 == JSON 判据资产（都过一遍 `fill_defaults`）。
///
/// 这条同时兜住两个方向的漂移：有人手改了 TOML，或者有人改了 JSON 忘了重新派生。
#[test]
fn fixture_is_derived_from_the_ir_json() {
    let from_toml = config::load(&config_fixture()).expect("fixture 必须能读");
    let mut from_json = ir_from_json();
    fill_defaults(&mut from_json);
    assert_eq!(
        from_toml, from_json,
        "配置 fixture 与 IR JSON 判据资产不一致 ⇒ 跑一次 \
         `cargo test --test pipeline -- --ignored regenerate_config_fixture_from_ir_json`"
    );
    assert_eq!(
        from_toml.machine.machine_type, FIXTURE_MACHINE_TYPE,
        "fixture 必须自带机型名，否则它是一份跑不起来的配置"
    );
}

#[test]
fn full_pipeline_runs_and_orders_steps() {
    let out_path = std::env::temp_dir().join("mkp-pp-e2e-test.gcode");
    let req = ProcessRequest {
        gcode_path: golden_input(),
        config_path: config_fixture(),
        overrides: Vec::new(),
        output_path: Some(out_path.clone()),
    };
    let mut events = Vec::new();
    let mut sink = |e: ProgressEvent| events.push((e.step, e.fraction_in_step));
    let result = process(req, &mut sink, &CancelToken::new()).expect("全链应成功");

    // 有序子序列 + 必含
    assert_step_order(&result.executed_steps);
    // 该输入无校准 ⇒ CalibCheck / Calibration 缺席是合法子序列
    assert!(!result.executed_steps.contains(&Step::CalibCheck));
    // printtime 填了字段
    let pt = result.print_time.expect("print_time 应被填充");
    assert!(
        pt.total_seconds > 0.0,
        "打印时间应为正，得 {}",
        pt.total_seconds
    );
    // sha256 是 64 位十六进制
    assert_eq!(result.input_sha256.len(), 64);
    assert!(result.input_sha256.chars().all(|c| c.is_ascii_hexdigit()));
    // 输出文件已写入，且两个 hooks 的痕迹都在
    let out_text = std::fs::read_to_string(&out_path).expect("输出文件应存在");
    assert!(out_text.contains(";Pre-glue preparation"));
    assert!(
        out_text.contains("begin stage retract"),
        "默认偏好收笔块应在"
    );
    // 事件只带 step + 步内 fraction（类型层面保证）
    assert!(
        events.iter().any(|(s, f)| *s == Step::Pass1 && f.is_some()),
        "pass1 应发步内比例"
    );
    let _ = std::fs::remove_file(&out_path);
}

/// **执行过的每一步都必须发过进度事件**。
///
/// 来源仓库的起点红：step 12（printtime）只 `executed.push`、从不 `emit` ⇒ 最后一帧
/// 永远停在「写入文件」，看起来像卡死（实际已成功）。这条把「有执行、无事件」
/// 变成机械可查的失败。
#[test]
fn every_executed_step_also_emits_progress() {
    let out_path = std::env::temp_dir().join("mkp-pp-step-event-parity.gcode");
    let req = ProcessRequest {
        gcode_path: golden_input(),
        config_path: config_fixture(),
        overrides: Vec::new(),
        output_path: Some(out_path.clone()),
    };
    let mut seen: Vec<Step> = Vec::new();
    let mut sink = |e: ProgressEvent| {
        if !seen.contains(&e.step) {
            seen.push(e.step);
        }
    };
    let result = process(req, &mut sink, &CancelToken::new()).expect("全链应成功");
    let missing: Vec<&Step> = result
        .executed_steps
        .iter()
        .filter(|s| !seen.contains(s))
        .collect();
    assert!(
        missing.is_empty(),
        "这些步骤执行了却没发进度事件（调用方无从得知它们发生过）：{missing:?}"
    );
    assert!(
        seen.contains(&Step::PrintTime),
        "printtime 必须有事件，否则进度永远到不了 100%"
    );
    let _ = std::fs::remove_file(&out_path);
}

/// `--set` 真的作用在管线上（不只是在 config 的单测里）。
#[test]
fn override_reaches_the_pipeline() {
    let out_path = std::env::temp_dir().join("mkp-pp-override.gcode");
    let req = ProcessRequest {
        gcode_path: golden_input(),
        config_path: config_fixture(),
        overrides: vec!["Meta.PresetName=从命令行改的".to_string()],
        output_path: Some(out_path.clone()),
    };
    let result = process(req, &mut |_| {}, &CancelToken::new()).expect("全链应成功");
    assert_eq!(
        result.detail_facts.ir.meta.preset_name, "从命令行改的",
        "--set 的值必须出现在出口 IR 里"
    );
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn missing_config_file_is_reported_with_code() {
    let req = ProcessRequest {
        gcode_path: golden_input(),
        config_path: repo_root().join("tests/fixtures/config/__nope__.toml"),
        overrides: Vec::new(),
        output_path: None,
    };
    let err = match process(req, &mut |_| {}, &CancelToken::new()) {
        Err(e) => e,
        Ok(_) => panic!("缺配置文件应失败"),
    };
    assert_eq!(err.code(), "E_FS_NOT_FOUND_001", "错误码保持旧字符串");
}

#[test]
fn broken_toml_aborts_with_code() {
    let tmp = std::env::temp_dir().join("mkp-pp-badtoml.toml");
    std::fs::write(&tmp, "[this is not = toml\n").expect("写临时文件");
    let req = ProcessRequest {
        gcode_path: golden_input(),
        config_path: tmp,
        overrides: Vec::new(),
        output_path: None,
    };
    let err = match process(req, &mut |_| {}, &CancelToken::new()) {
        Err(e) => e,
        Ok(_) => panic!("坏 TOML 应失败"),
    };
    assert_eq!(err.code(), "E_TOML_PARSE_001");
}

/// 机型名不认识 ⇒ 在 config 步就中止，**pass1 不执行、输出不写**。
///
/// 这是对来源仓库的**收紧**（见 `pipeline` 模块文档差异 4）：那边只检查预设里
/// 那个串非空，`normalize_to_canonical` 对未知别名返回 `""` 之后一路跑到底 ——
/// 机型维度全 0，输出是垃圾而退出码是 0。
#[test]
fn unknown_machine_type_aborts_before_pass1() {
    let out_path = std::env::temp_dir().join("mkp-pp-unknown-machine-out.gcode");
    let _ = std::fs::remove_file(&out_path);
    let req = ProcessRequest {
        gcode_path: golden_input(),
        config_path: config_fixture(),
        overrides: vec!["Machine.MachineType=NoSuchPrinter".to_string()],
        output_path: Some(out_path.clone()),
    };
    let mut steps_seen = Vec::new();
    let err = match process(
        req,
        &mut |e: ProgressEvent| steps_seen.push(e.step),
        &CancelToken::new(),
    ) {
        Err(e) => e,
        Ok(_) => panic!("未知机型应硬错误"),
    };
    assert_eq!(err.code(), "E_CFG_PARSE_001");
    assert!(
        err.to_string().contains("NoSuchPrinter"),
        "报错必须点名那个写错的机型：{err}"
    );
    assert!(!steps_seen.contains(&Step::Pass1), "失败后不得继续");
    assert!(!out_path.exists(), "失败时不得写输出");
}
