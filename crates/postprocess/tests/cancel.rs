//! 用户取消语义（端到端，真跑不纸面）。来源：`mkp-sr` 的 `crates/engine/tests/cancel.rs`。
//!
//! - a) 预取消：第一个 checkpoint 即 Cancelled，原地输入字节不变、无输出、无 `.part`
//! - b) pass1 中途取消：Err(Cancelled) 且 Pass1 进度事件停在触发点附近
//!   （**不许出现高比例事件** —— 证明循环内真的停了，不是跑完在下一边界被截）
//! - c) pass2 中途取消：同型
//! - d) 显式 `--out` 下取消同样不落任何文件
//!
//! **b/c 两条为什么走 `process_with_cancel_interval(..., Duration::ZERO)`**：
//! 产品的协作检查是 100ms **时间基**节流，而「pass1 全程秒级」这个前提在快机器上是假的
//! —— 来源仓库在自托管 amd64runner 上实测同一测试二进制 4 条用例合计 **0.25s**，
//! 100ms 窗一次都没到点 ⇒ 循环内一次都没查取消位 ⇒ pass 跑完、取消只被下一个步骤边界的
//! checkpoint 截住 ⇒ 事件最高比例 0.97 / 0.99，判据**确定性假红**。注入
//! `Duration::ZERO` 让每轮都真查，于是「停在触发点附近」变成与机器速度无关的事实；
//! **阈值 0.60 / 0.90 一个字没放宽**。产品默认仍是 100ms
//! （`DEFAULT_CANCEL_CHECK_INTERVAL`），由 `process` 传入，本文件不动它。

use std::path::{Path, PathBuf};
use std::time::Duration;

use mkp_pp::diag::{CancelToken, PostprocError};
use mkp_pp::pipeline::{
    ProcessRequest, ProgressEvent, ProgressSink, Step, process, process_with_cancel_interval,
};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn golden_input() -> PathBuf {
    repo_root().join("tests/golden/42274.2.gcode")
}

/// 与 `tests/pipeline.rs` 同一份配置 fixture（派生关系由那边的判据钉住，这里只用）。
fn config_a1() -> PathBuf {
    repo_root().join("tests/fixtures/config/A1.toml")
}

/// 记录事件 + 条件触取消的 sink：emit 时若谓词命中就拉闸（模拟用户在 ~x% 时点取消）。
struct TriggerSink {
    events: Vec<(Step, Option<f32>)>,
    token: CancelToken,
    when: fn(Step, Option<f32>) -> bool,
}

impl ProgressSink for TriggerSink {
    fn emit(&mut self, event: ProgressEvent) {
        self.events.push((event.step, event.fraction_in_step));
        if (self.when)(event.step, event.fraction_in_step) {
            self.token.cancel();
        }
    }
}

impl TriggerSink {
    fn max_fraction(&self, step: Step) -> f32 {
        self.events
            .iter()
            .filter(|(s, _)| *s == step)
            .filter_map(|(_, f)| *f)
            .fold(0.0_f32, f32::max)
    }

    fn any_step(&self, step: Step) -> bool {
        self.events.iter().any(|(s, _)| *s == step)
    }
}

/// 原地模式的输入副本 + 其原始字节（取消后必须逐字节相同）。
fn inplace_copy(tag: &str) -> (PathBuf, Vec<u8>) {
    let path = std::env::temp_dir().join(format!("mkp-pp-cancel-{tag}.gcode"));
    std::fs::copy(golden_input(), &path).expect("复制输入失败");
    let pristine = std::fs::read(&path).expect("读副本失败");
    (path, pristine)
}

fn part_path_of(p: &Path) -> PathBuf {
    let mut s = p.as_os_str().to_os_string();
    s.push(".part");
    PathBuf::from(s)
}

/// a：token 预先置位 → 第一个 checkpoint（input 步入口）即 Cancelled。
#[test]
fn precancelled_aborts_immediately_and_touches_nothing() {
    let (input, pristine) = inplace_copy("pre");
    let token = CancelToken::new();
    token.cancel();
    let req = ProcessRequest {
        gcode_path: input.clone(),
        config_path: config_a1(),
        overrides: Vec::new(),
        output_path: None, // 原地模式：取消必须保证原文件不动
    };
    let err = match process(req, &mut |_| {}, &token) {
        Err(e) => e,
        Ok(_) => panic!("预取消必须失败，不许跑完"),
    };
    assert!(
        matches!(err, PostprocError::Cancelled { .. }),
        "实测 {err:?}"
    );
    assert_eq!(err.code(), "E_SYS_CANCELLED_001");
    assert_eq!(
        std::fs::read(&input).expect("取消后输入必须仍可读"),
        pristine,
        "原地模式取消后输入文件被改动"
    );
    assert!(!part_path_of(&input).exists(), "取消后不得残留 .part");
}

/// b：pass1 ~30% 时拉闸 → Err(Cancelled)，且 Pass1 进度事件停在低比例。
///
/// 上限 0.60 的理由：触发 0.30 + 注入 ZERO 间隔（下一轮循环顶部即查）≪ 0.60；
/// 若循环内检查失效（跑完全程才被下一边界截住），会出现 ~1.0 的事件 → 红。
#[test]
fn cancel_during_pass1_stops_inside_the_loop() {
    let (input, pristine) = inplace_copy("p1");
    let token = CancelToken::new();
    let mut sink = TriggerSink {
        events: Vec::new(),
        token: token.clone(),
        when: |s, f| s == Step::Pass1 && f.is_some_and(|v| v > 0.30),
    };
    let req = ProcessRequest {
        gcode_path: input.clone(),
        config_path: config_a1(),
        overrides: Vec::new(),
        output_path: None,
    };
    let err = match process_with_cancel_interval(req, &mut sink, &token, Duration::ZERO) {
        Err(e) => e,
        Ok(_) => panic!("pass1 中途取消必须失败"),
    };
    assert!(
        matches!(err, PostprocError::Cancelled { .. }),
        "实测 {err:?}"
    );
    let max_p1 = sink.max_fraction(Step::Pass1);
    assert!(
        max_p1 < 0.60,
        "pass1 应停在 ~30%+ε（注入 ZERO 间隔 ⇒ 下一轮循环顶部即截），实测最高 {max_p1:.2} \
         ⇒ 循环内协作检查未生效（跑到下一边界才被截）"
    );
    assert!(!sink.any_step(Step::Write), "取消后不得进入 write");
    assert!(!sink.any_step(Step::PrintTime), "取消后不得进入 printtime");
    assert_eq!(std::fs::read(&input).unwrap(), pristine, "输入被改动");
    assert!(!part_path_of(&input).exists(), "不得残留 .part");
}

/// c：pass2 ~50% 时拉闸 → 同型。pass2 是塔模式重头（含塔层块发射），上限 0.90 给足余量。
#[test]
fn cancel_during_pass2_stops_inside_the_loop() {
    let (input, pristine) = inplace_copy("p2");
    let token = CancelToken::new();
    let mut sink = TriggerSink {
        events: Vec::new(),
        token: token.clone(),
        when: |s, f| s == Step::Pass2 && f.is_some_and(|v| v > 0.50),
    };
    let req = ProcessRequest {
        gcode_path: input.clone(),
        config_path: config_a1(),
        overrides: Vec::new(),
        output_path: None,
    };
    let err = match process_with_cancel_interval(req, &mut sink, &token, Duration::ZERO) {
        Err(e) => e,
        Ok(_) => panic!("pass2 中途取消必须失败"),
    };
    assert!(
        matches!(err, PostprocError::Cancelled { .. }),
        "实测 {err:?}"
    );
    let max_p2 = sink.max_fraction(Step::Pass2);
    assert!(
        max_p2 < 0.90,
        "pass2 应停在 ~50%+ε，实测最高 {max_p2:.2} ⇒ 循环内协作检查未生效"
    );
    assert!(!sink.any_step(Step::Write), "取消后不得进入 write");
    assert_eq!(std::fs::read(&input).unwrap(), pristine, "输入被改动");
    assert!(!part_path_of(&input).exists(), "不得残留 .part");
}

/// e：**步骤边界 checkpoint 的独立判据**（本项目新增，来源仓库没有这一条）。
///
/// 为什么必须新增：NV-14 实测把 `checkpoint` 整个失效（`if false && cancel.is_cancelled()`）
/// 之后，上面 a–d **四条全绿** —— 因为它们观测到的 `Cancelled` 由 pass1/pass2
/// **循环内**的协作检查产生，与那 11 处步骤边界 checkpoint 无关。
/// 也就是说在加这条之前，「每步入口一道 checkpoint」这个设计**没有任何判据钉住**
/// （AGENTS §7② 的同款：换个机制照样绿）。
///
/// 隔离手法：把循环内检查的节流间隔设成 1 小时 ⇒ pass 内一次都不会真查 ⇒
/// 唯一还能截住取消的就是步骤边界的 checkpoint。于是在 pass1 的第一个进度事件处拉闸，
/// pass1 会跑完，**pass2 步入口**那道 checkpoint 必须把它截住。
#[test]
fn step_boundary_checkpoint_is_the_only_thing_that_can_stop_it_when_loops_are_throttled() {
    let (input, pristine) = inplace_copy("boundary");
    let token = CancelToken::new();
    let mut sink = TriggerSink {
        events: Vec::new(),
        token: token.clone(),
        when: |s, _| s == Step::Pass1,
    };
    let req = ProcessRequest {
        gcode_path: input.clone(),
        config_path: config_a1(),
        overrides: Vec::new(),
        output_path: None,
    };
    // 一小时的节流窗 ⇒ 循环内的检查在本次运行里永远到不了点。
    let err = match process_with_cancel_interval(req, &mut sink, &token, Duration::from_secs(3600))
    {
        Err(e) => e,
        Ok(_) => panic!("取消位已置 ⇒ 必须在某个步骤边界被截住（checkpoint 失效了？）"),
    };
    assert!(
        matches!(err, PostprocError::Cancelled { .. }),
        "实测 {err:?}"
    );
    // pass1 跑完了（循环内没查），所以这次截住它的一定是步骤边界那道
    assert!(sink.any_step(Step::Pass1), "pass1 应当被执行过");
    assert!(!sink.any_step(Step::Write), "取消后不得进入 write");
    assert_eq!(std::fs::read(&input).unwrap(), pristine, "输入被改动");
    assert!(!part_path_of(&input).exists(), "不得残留 .part");
}

#[test]
fn cancel_with_explicit_out_writes_neither_out_nor_part() {
    let token = CancelToken::new();
    let out = std::env::temp_dir().join("mkp-pp-cancel-explicit-out.gcode");
    let _ = std::fs::remove_file(&out);
    let mut sink = TriggerSink {
        events: Vec::new(),
        token: token.clone(),
        when: |s, f| s == Step::Pass1 && f.is_some_and(|v| v > 0.30),
    };
    let req = ProcessRequest {
        gcode_path: golden_input(),
        config_path: config_a1(),
        overrides: Vec::new(),
        output_path: Some(out.clone()),
    };
    let err = match process(req, &mut sink, &token) {
        Err(e) => e,
        Ok(_) => panic!("中途取消必须失败"),
    };
    assert!(matches!(err, PostprocError::Cancelled { .. }));
    assert!(!out.exists(), "取消后不得写出输出文件");
    assert!(!part_path_of(&out).exists(), "取消后不得残留 .part");
}
