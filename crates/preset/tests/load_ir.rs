//! `load_ir` 的判据 —— 2026-10-31 起改成 **SupportEase 自有标准**（不再依赖 MKP-SR 的旧产物）。
//!
//! ## 为什么改
//!
//! 原来这条文件里有一条「预设 → IR → 12 步 → 输出，与 `mkp-sr` 参考物逐字节相同」的整链判据。
//! 那份参考物由**来源仓库的 CLI** 生成、本仓无法复现，于是它实际上把「那一刻的参数值」
//! 一起钉死了：参数源（`presets/registry`）一改，输出就变，CI 必红，而且只能去外部重出参考物。
//! 独立之后这条依赖必须断掉 —— 参数要能持续更新，判据要能继续发现真实的行为错误。
//!
//! ## 现在的分层（每一层各答一问）
//!
//! | 层 | 判据 | 回答什么 | 参数改了会怎样 |
//! |---|---|---|---|
//! | 数据一致 | `dataLabel`/`build.rs`/`variants.rs`/`gen-presets --check` | 源 → 配方 → 入库产物 → 夹具基线是否同源 | **跟着重算**（允许） |
//! | 映射 | [`preset_maps_into_ir_field_by_field`] | 预设里的每一格是否**原样**进 IR | 自动跟随（期望值独立从 TOML 算） |
//! | 行为 | [`changing_offset_z_shifts_every_paint_block_by_the_same_amount`] 等 | 参数是否真的作用到输出 | 自动跟随（差值口径，不写死数值） |
//! | 结构 | [`the_chain_is_deterministic_and_structurally_sound`] | 输出稳定、标记成对、坐标不越界 | 不受影响 |
//! | 回归 | [`preset_chain_matches_the_support_ease_golden`] | **有没有未经确认的字节变化** | **不受影响**（输入是冻结件） |
//! | 管线回归 | `crates/postprocess/tests/{end_to_end,process_with_ir}.rs` | 管线字节有没有变（冻结 config 路径） | 不受影响 |
//!
//! ## 自有 golden 的规则（职责 / 谁能改 / 什么时候改）
//!
//! 完整规格见 `crates/postprocess/tests/reference/README.md` 的
//! 「SupportEase 自有整链 golden」一节。三条硬规矩先写在这里：
//!
//! 1. **输入是冻结件**：`tests/fixtures/presets-frozen/A1-standard.frozen.toml`。
//!    它属于测试资产，`gen-presets --sync-baseline` **不碰**它，参数更新也不改它 ——
//!    所以参数怎么变都不会要求重录这份 golden。
//! 2. **不自动重写**：只有显式设 `MKPSE_RECORD_PRESET_GOLDEN=1` 才会写盘，且不挂在
//!    任何自动路径上（不是 `--sync-baseline`、不是生成、不是发布）。
//! 3. **它是回归检测器，不是「正确」的裁判**：红了先看差异是「映射/管线改了」还是
//!    「有人动了冻结输入」；确认想要之后才重录，并把原因写进提交信息。
//!
//! ## 证据等级（别把差别抹平）
//!
//! - 自有 golden：**自比自**，只能证明「没变」，不证明「对」；
//! - 映射 / 行为 / 结构三条：期望值由测试**独立**从预设 TOML 或差值规则算出，
//!   证明「这一格确实生效了」，但仍不是「工艺正确」的背书（工艺正确要靠真机）；
//! - 冻结 config 的两条外部参考判据仍然保留，它们守的是「搬过来的管线没变」。
//!
//! 伪随机序列是**进程级**状态（见 `crates/postprocess/tests/process_with_ir.rs` 文件头），
//! 所以每条整链用例都要先 reset 并串行（本文件用一把锁）。

// 测试写临时文件、造夹具是正当的：写盘纪律管的是**生产代码**
// （与源码扫描断言只看 `#[cfg(test)]` 之前那部分同一口径）。
#![allow(clippy::disallowed_methods)]

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use postprocess::diag::CancelToken;
use postprocess::pipeline::{self, IrProcessRequest, NoProgress};

static SEQUENCE: Mutex<()> = Mutex::new(());

fn lock_sequence() -> MutexGuard<'static, ()> {
    match SEQUENCE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn core_path(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../postprocess/{rel}"))
}

/// 数据源那一侧的预设（**会**随参数更新而变）。行为/映射判据用它。
fn live_preset() -> PathBuf {
    core_path("tests/fixtures/presets/A1-standard.toml")
}

/// 自有 golden 的**冻结**输入（参数更新不动它）。
fn frozen_preset() -> PathBuf {
    core_path("tests/fixtures/presets-frozen/A1-standard.frozen.toml")
}

fn golden_output() -> PathBuf {
    core_path("tests/golden/42274.2.e2e-A1.support-ease.gcode")
}

fn live_text() -> String {
    std::fs::read_to_string(live_preset()).expect("读实时预设夹具")
}

/// 读实时预设并改一格。**头注释（`# machine:`）必须保住** —— 序列化只留正文。
fn live_preset_with<F: FnOnce(&mut toml::Value)>(f: F) -> String {
    let text = live_text();
    let head: String = text
        .lines()
        .take_while(|l| l.starts_with('#') || l.trim().is_empty())
        .map(|l| format!("{l}\n"))
        .collect();
    let mut value: toml::Value = toml::from_str(&text).expect("夹具不是合法 TOML");
    f(&mut value);
    format!("{head}{}", toml::to_string(&value).expect("写回 TOML"))
}

/// 整链跑一遍，返回**完整的**输出正文（不剔标记行 —— 那是我们自己的格式，本来就是判据面）。
///
/// 调用方必须持有 [`lock_sequence`]：里面会 reset 进程级伪随机序列。
fn run_chain(preset_path: &Path, tag: &str) -> String {
    postprocess::gcode::reset_pseudo_random();
    let dir = tempfile::tempdir().expect("临时目录");
    let work = dir.path().join("input.gcode");
    std::fs::copy(core_path("tests/golden/42274.2.gcode"), &work).expect("复制输入");
    let raw = std::fs::read_to_string(&work).expect("读输入");
    let mut ir = preset::load_ir(preset_path, Some(&raw)).expect("预设 → IR 必须成功");
    // 用户偏好（`Ir.prefs`）的两颗钉子：参考档是「抬升 Z=20 + 跳过震动检测」。
    ir.prefs.begin_stage_retract_z_lift_mm = 20;
    ir.prefs.skip_vibration_calibration = true;
    pipeline::process_with_ir_with_tower_ask(
        IrProcessRequest {
            gcode_path: work.clone(),
            ir,
            output_path: None,
            calibration: Default::default(),
        },
        None,
        &mut NoProgress,
        &CancelToken::new(),
    )
    .unwrap_or_else(|e| panic!("整链必须成功（{tag}）: {e}"));
    std::fs::read_to_string(&work).expect("读回原地输出")
}

fn write_temp_preset(text: &str, tag: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("临时目录");
    let path = dir.path().join(format!("{tag}.toml"));
    std::fs::write(&path, text).expect("写临时预设");
    (dir, path)
}

// ---------------------------------------------------------------------------
// 映射层：预设里的每一格，独立算一遍，再与 IR 对照
// ---------------------------------------------------------------------------

/// **不许**用生产映射函数的结果当期望值：期望值一律从 TOML 独立读出来、按口径算。
#[test]
fn preset_maps_into_ir_field_by_field() {
    let text = live_text();
    let t: toml::Value = toml::from_str(&text).expect("夹具不是合法 TOML");
    // TOML 里 `4` 与 `4.0` 是两种类型 —— 期望值一律取数，不要求作者写成浮点
    let num = |v: &toml::Value| {
        v.as_float()
            .or_else(|| v.as_integer().map(|i| i as f64))
            .unwrap_or_else(|| panic!("必须是数字，实测 {v:?}"))
    };

    let offset_x = num(&t["toolhead"]["offset_x"]);
    let offset_y = num(&t["toolhead"]["offset_y"]);
    let offset_z = num(&t["toolhead"]["offset_z"]);
    let speed_limit = num(&t["toolhead"]["speed_limit"]);
    let ironing_mode = t["wiping"]["ironing_mode"].as_str().expect("必须是字符串");
    let tower_ratio = num(&t["wiping"]["tower_extrude_ratio"]);
    let machine = text
        .lines()
        .find_map(|l| l.trim().strip_prefix("# machine:"))
        .map(str::trim)
        .expect("夹具必须带 `# machine:` 头");

    let ir = preset::load_ir(&live_preset(), None).expect("预设 → IR 必须成功");

    assert_eq!(
        ir.toolhead.x_offset, offset_x,
        "toolhead.offset_x 没有原样进 IR"
    );
    assert_eq!(
        ir.toolhead.y_offset, offset_y,
        "toolhead.offset_y 没有原样进 IR"
    );
    assert_eq!(
        ir.toolhead.z_offset, offset_z,
        "toolhead.offset_z 没有原样进 IR"
    );
    // 唯一换算点：mm/s → mm/min（全仓只在映射层做一次）
    assert_eq!(
        ir.toolhead.max_speed,
        speed_limit * 60.0,
        "speed_limit 必须按 ×60 换成 mm/min"
    );
    assert_eq!(
        ir.ironing.mode, ironing_mode,
        "wiping.ironing_mode 没有原样进 IR"
    );
    assert_eq!(
        ir.tower.extrude_ratio, tower_ratio,
        "wiping.tower_extrude_ratio 没有原样进 IR"
    );
    assert_eq!(
        ir.machine.machine_type, machine,
        "机型必须归一成预设声明的那个"
    );
    assert_eq!(
        ir.meta.preset_name, "A1-standard.toml",
        "preset_name 必须是带扩展名的文件名"
    );

    // 反空转：机型维度不能是 0×0（那是「安静跑完」的失败形态）
    assert!(
        ir.machine.max_x > 100.0 && ir.machine.max_y > 100.0 && ir.machine.glue_max_x > 100.0,
        "机型维度没填上：max=({}, {}) glue_max_x={}",
        ir.machine.max_x,
        ir.machine.max_y,
        ir.machine.glue_max_x
    );
}

// ---------------------------------------------------------------------------
// 行为层：只改一格，看输出差在哪 —— 期望值是「差值」，不是一个写死的字节
// ---------------------------------------------------------------------------

fn paint_block_z(output: &str) -> Vec<f64> {
    let lines: Vec<&str> = output.lines().collect();
    lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.starts_with(";MKP_BEGIN paint"))
        .filter_map(|(i, _)| {
            lines[i + 1..]
                .iter()
                .take(8)
                .find(|l| l.starts_with("G1 Z"))
                .and_then(|l| l[4..].trim().parse::<f64>().ok())
        })
        .collect()
}

/// `offset_z` 加多少，每个涂胶块的开头 Z 就抬多少 —— 一个都不能漏。
#[test]
fn changing_offset_z_shifts_every_paint_block_by_the_same_amount() {
    let _seq = lock_sequence();
    let parsed: toml::Value = toml::from_str(&live_text()).expect("夹具不是合法 TOML");
    let base_z = parsed["toolhead"]["offset_z"]
        .as_float()
        .expect("夹具里必须有 toolhead.offset_z");
    let bumped = live_preset_with(|v| {
        v["toolhead"]["offset_z"] = toml::Value::Float(base_z + 0.5);
    });
    let (_d1, base_path) = write_temp_preset(&live_text(), "offset-base");
    let (_d2, bump_path) = write_temp_preset(&bumped, "offset-bump");

    let a = run_chain(&base_path, "offset-base");
    let b = run_chain(&bump_path, "offset-bump");

    let (za, zb) = (paint_block_z(&a), paint_block_z(&b));
    // 反空转：至少得有涂胶块，否则这条断言是白过的
    assert!(
        za.len() >= 5,
        "涂胶块只有 {} 个 —— 这条判据没覆盖到实际涂胶，视为失败",
        za.len()
    );
    assert_eq!(za.len(), zb.len(), "改 offset_z 不该改变涂胶块的数量");
    for (i, (x, y)) in za.iter().zip(zb.iter()).enumerate() {
        let delta = y - x;
        assert!(
            (delta - 0.5).abs() < 1e-6,
            "第 {i} 个涂胶块的 Z 没跟着 offset_z 走：{x} → {y}（差 {delta}）"
        );
    }
}

/// 熨烫打开只允许**多出**熨烫的挤出移动，原有行一个都不许被改写。
#[test]
fn turning_ironing_on_only_adds_the_ironing_moves() {
    let _seq = lock_sequence();
    let off = live_preset_with(|v| {
        v["wiping"]["ironing_mode"] = toml::Value::String("off".to_owned());
    });
    let (_d1, off_path) = write_temp_preset(&off, "ironing-off");
    let (_d2, auto_path) = write_temp_preset(&live_text(), "ironing-auto");

    let with_ironing = run_chain(&auto_path, "ironing-auto");
    let without = run_chain(&off_path, "ironing-off");

    assert_ne!(with_ironing, without, "关掉熨烫后输出居然一个字节都没变");
    assert!(
        with_ironing.contains("IRONING_EXTRUSIONS_START"),
        "熨烫打开时输出里必须有熨烫段（锚点 IRONING_EXTRUSIONS_START）"
    );
    assert!(
        !without.contains("IRONING_EXTRUSIONS_START"),
        "熨烫关掉后不该再有熨烫段"
    );

    // 关掉熨烫后的每一行，必须原样出现在打开熨烫的输出里（= 只删不改）
    let long: Vec<&str> = with_ironing.lines().collect();
    let mut cursor = 0usize;
    for line in without.lines() {
        match long[cursor..].iter().position(|l| *l == line) {
            Some(at) => cursor += at + 1,
            None => panic!("关熨烫的输出里有一行在开熨烫的输出里找不到：{line}"),
        }
    }
}

/// 塔的挤出比：`E` 按比例缩放，坐标与其余行一个字不动。
///
/// 这份整链输入不产塔（`useTowers` 关着），所以差分放在**偏移层**做 —— 比例真正被消费的地方。
#[test]
fn tower_extrude_ratio_scales_only_the_e_values() {
    use postprocess::gcode::OffsetMode;
    use postprocess::postproc::offset::process_gcode_offset;

    let mut ir = preset::load_ir(&live_preset(), None).expect("预设 → IR");
    let line = "G1 X10 Y10 E2";

    ir.tower.extrude_ratio = 1.0;
    let full =
        process_gcode_offset(line, 0.0, 0.0, 0.0, OffsetMode::Tower, &ir).expect("偏移必须成功");
    ir.tower.extrude_ratio = 0.5;
    let half =
        process_gcode_offset(line, 0.0, 0.0, 0.0, OffsetMode::Tower, &ir).expect("偏移必须成功");

    let e_of = |s: &str| -> f64 {
        s.split_whitespace()
            .find_map(|w| w.strip_prefix('E'))
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or_else(|| panic!("输出里没有 E：{s}"))
    };
    assert!(
        (e_of(&full) - 2.0).abs() < 1e-9,
        "比例 1.0 时 E 应原样：{full}"
    );
    assert!(
        (e_of(&half) - 1.0).abs() < 1e-9,
        "比例 0.5 时 E 应减半：{half}"
    );
    // 坐标不动
    assert!(
        half.contains("X10") && half.contains("Y10"),
        "比例不该改坐标：{half}"
    );
}

// ---------------------------------------------------------------------------
// 结构层（辅助）：确定性 + 标记成对 + 坐标不越界
// ---------------------------------------------------------------------------

/// 这一条**只**证明「稳」和「结构合法」，不证明任何参数正确。
///
/// 刻意**不**比较输入输出的行数：后处理合法地增删指令（熨烫、抬升、换笔），
/// 行数守恒在这里不是不变量。
#[test]
fn the_chain_is_deterministic_and_structurally_sound() {
    let _seq = lock_sequence();
    let a = run_chain(&live_preset(), "det-1");
    let b = run_chain(&live_preset(), "det-2");
    assert_eq!(a, b, "同一输入跑两遍输出不一致 —— 有未复位的时间/随机状态");

    let begins = a.matches(";MKP_BEGIN").count();
    let ends = a.matches(";MKP_END").count();
    assert!(begins > 0, "输出里一个 MKP 区间都没有 —— 判据空转");
    assert_eq!(
        begins, ends,
        "MKP 区间没有成对：BEGIN {begins} / END {ends}"
    );

    assert!(a.starts_with("; HEADER_BLOCK_START"), "输出头被改坏了");
    assert!(a.contains("; EXECUTABLE_BLOCK_START"), "可执行块头不见了");

    let ir = preset::load_ir(&live_preset(), None).expect("预设 → IR");
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    let mut checked = 0usize;
    // 只扫**涂胶块内部**的移动：换笔块里的 `custom_mount_gcode` 是机型固定指令
    // （`G1 X267` 这类装笔动作），它们的坐标不属于模型行程，拿机型行程去卡会假红。
    let mut in_paint = false;
    for line in a.lines() {
        if line.starts_with(";MKP_BEGIN paint") {
            in_paint = true;
            continue;
        }
        if line.starts_with(";MKP_END") {
            in_paint = false;
            continue;
        }
        if !in_paint {
            continue;
        }
        let Some(rest) = line.strip_prefix("G1 ") else {
            continue;
        };
        let mut x = None;
        let mut y = None;
        for word in rest.split_whitespace() {
            if let Some(v) = word.strip_prefix('X').and_then(|v| v.parse::<f64>().ok()) {
                x = Some(v);
            }
            if let Some(v) = word.strip_prefix('Y').and_then(|v| v.parse::<f64>().ok()) {
                y = Some(v);
            }
        }
        if let Some(v) = x {
            min_x = min_x.min(v);
            max_x = max_x.max(v);
            checked += 1;
        }
        if let Some(v) = y {
            min_y = min_y.min(v);
            max_y = max_y.max(v);
        }
    }
    assert!(checked > 1000, "只扫到 {checked} 条 G1 X —— 解析口径坏了");
    // 留 1mm 容差：Z 抬升与换笔段的移动点按规则会贴边
    assert!(
        min_x >= ir.machine.min_x - 1.0 && max_x <= ir.machine.max_x + 1.0,
        "X 越界：实测 [{min_x}, {max_x}]，机型允许 [{}, {}]",
        ir.machine.min_x,
        ir.machine.max_x
    );
    assert!(
        min_y >= ir.machine.min_y - 1.0 && max_y <= ir.machine.max_y + 1.0,
        "Y 越界：实测 [{min_y}, {max_y}]，机型允许 [{}, {}]",
        ir.machine.min_y,
        ir.machine.max_y
    );
}

// ---------------------------------------------------------------------------
// 回归层：SupportEase 自有的整链 golden（冻结输入，不随参数更新）
// ---------------------------------------------------------------------------

/// 自有 golden 的比对。**红了先读差异，别直接重录**：
///
/// - 差异在映射/管线 → 那正是它要抓的东西，先确认是不是想要的改动；
/// - 差异只在参数相关的行 → 说明有人改了「冻结输入」，那违反规则，先查为什么。
///
/// 重录（唯一入口，人工动作）：
/// ```text
/// MKPSE_RECORD_PRESET_GOLDEN=1 cargo test -p mkpse-preset --test load_ir preset_chain_matches_the_support_ease_golden
/// ```
#[test]
fn preset_chain_matches_the_support_ease_golden() {
    let _seq = lock_sequence();
    let got = run_chain(&frozen_preset(), "golden");
    assert!(!got.is_empty(), "整链输出是空的 —— 判据已空转");

    if std::env::var("MKPSE_RECORD_PRESET_GOLDEN").ok().as_deref() == Some("1") {
        std::fs::write(golden_output(), got.as_bytes()).expect("写自有 golden");
        eprintln!("已重录自有 golden：{}", golden_output().display());
        return;
    }

    let want = std::fs::read_to_string(golden_output()).unwrap_or_else(|e| {
        panic!(
            "读不到自有 golden（{}）：{e}\n\
             首次建立请跑：MKPSE_RECORD_PRESET_GOLDEN=1 cargo test -p mkpse-preset --test load_ir",
            golden_output().display()
        )
    });
    assert!(!want.is_empty(), "自有 golden 是空文件 —— 判据已空转");

    if got == want {
        return;
    }
    // 不整份 assert_eq!（60 万字节会刷屏）：只报首个不同的行
    let (gl, wl): (Vec<&str>, Vec<&str>) = (got.lines().collect(), want.lines().collect());
    let at = gl
        .iter()
        .zip(wl.iter())
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| {
            panic!(
                "输出与自有 golden 行内容相同但行数不同：本侧 {} 行 / golden {} 行",
                gl.len(),
                wl.len()
            )
        });
    panic!(
        "自有 golden 不一致（第 {} 行，index {at}）——\n  本侧：{}\n  golden：{}\n\
         若是想要的改动：人工确认差异后跑 MKPSE_RECORD_PRESET_GOLDEN=1 重录，并把原因写进提交信息。\n\
         若只差参数相关行：说明冻结输入被改了，先查是谁动的。",
        at + 1,
        gl.get(at).unwrap_or(&"<本侧到此结束>"),
        wl.get(at).unwrap_or(&"<golden 到此结束>"),
    );
}

// ---------------------------------------------------------------------------
// 硬错误（沿用）
// ---------------------------------------------------------------------------

/// 写一份最小可解析的预设到临时目录，`machine` 头由调用方给。
fn minimal_preset(dir: &Path, machine_header: Option<&str>) -> PathBuf {
    let src = std::fs::read_to_string(live_preset()).expect("读基准预设");
    // 去掉原有的 `# machine:` 行，再按需要加回一条 —— 这样其余字段全是真实值，
    // 判据测的是机型这一个变量。
    let body: String = src
        .lines()
        .filter(|l| !l.trim_start().starts_with("# machine:"))
        .map(|l| format!("{l}\n"))
        .collect();
    let text = match machine_header {
        Some(m) => format!("# machine: {m}\n{body}"),
        None => body,
    };
    let path = dir.join("probe.toml");
    std::fs::write(&path, text).expect("写临时预设");
    path
}

#[test]
fn an_unknown_machine_alias_is_a_hard_error() {
    let dir = tempfile::tempdir().expect("临时目录");
    let path = minimal_preset(dir.path(), Some("NOT_A_PRINTER"));
    // **不用 expect_err**：它在 Ok 分支会把整个 Ir 用 Debug 打出来（NV-5.5 实测刷了 5KB，
    // 真因埋在里面 —— AGENTS.md §7⑦ 同款）。只打一句人话 + 两个关键字段。
    let err = match preset::load_ir(&path, None) {
        Err(e) => e,
        Ok(ir) => panic!(
            "未知机型必须失败，实测却成功了：machine_type={:?} max_x={} —— \
             这正是「0×0 的机器安静跑完」那个失败形态",
            ir.machine.machine_type, ir.machine.max_x
        ),
    };
    let msg = err.to_string();
    assert!(
        msg.contains("NOT_A_PRINTER"),
        "错误必须点名那个写错的机型，实测：{msg}"
    );
}

#[test]
fn a_missing_machine_header_is_a_hard_error() {
    let dir = tempfile::tempdir().expect("临时目录");
    let path = minimal_preset(dir.path(), None);
    let err = preset::load_ir(&path, None).expect_err("缺 `# machine:` 必须失败");
    assert_eq!(
        err.code(),
        "E_CFG_PARSE_001",
        "缺头注释的错误码是对外契约，实测 {}：{err}",
        err.code()
    );
}
