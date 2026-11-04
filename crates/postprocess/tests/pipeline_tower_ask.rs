//! 第 6 步「撞了就问」的判据（2026-10-18 方案 B）。
//!
//! 这一组钉的是**四条语义**，不是"能跑"：
//!   1. **没人问 ⇒ 一字不差**：老入口与"递了 `None`"的新入口拿到同一份碰撞事实；
//!   2. **问了并挪**：自动摆出来的位置通过复检，管线继续跑完，`verified(None)` 被调到；
//!   3. **复检不过**：回调给一个仍然撞的位置 ⇒ `Err(Collision)`（不许假装成功），
//!      `verified(Some(..))` 带着"挪后的事实"；
//!   4. **等待中取消**：回调返回 `Cancelled` ⇒ `Err(Cancelled)`（不是 `Collision`）；
//!   5. **进度不回退**：整趟事件的 `(步序 + 步内比例)` 单调不减，且第 6 步只出现一次。
//!
//! 输入是一份手写的最小 G-code（带 `; FEATURE: Support` 让擦料决策走 tower 档），
//! 模型点故意压在当前塔框里 —— 这样第 6 步一定撞，且撞的是几何而不是环境。

// 测试写临时文件、造夹具是正当的：写盘纪律管的是**生产代码**
// （与 `tests/cli.rs` / `tests/pipeline.rs` 同一条豁免，理由同一句）。
#![allow(clippy::disallowed_methods)]

use std::path::PathBuf;

use postprocess::diag::{CancelToken, PostprocError};
use postprocess::ir::{Ir, TowerRelocateModeIr};
use postprocess::pipeline::{
    self, IrProcessRequest, ProgressEvent, ProgressSink, STEPS, Step,
    process_with_ir_with_tower_ask,
};
use postprocess::postproc::disk::{
    CollisionFacts, TowerAsk, TowerAskOutcome, detect_tower_collision,
};

/// 最小可跑输入：支撑标记（擦料决策 ⇒ tower）+ 一个落在塔框里的挤出点。
///
/// 塔框 = `wiper(20,20)` + brim local `[2.96, 37.04]²` ⇒ `22.96..57.04`，所以 `X30 Y30` 必撞。
const GCODE: &str = "\
; FEATURE: Support
G90
G1 Z0.2
G1 X30 Y30 E0.5
G1 X120 Y120 E0.6
G1 X140 Y60 E0.7
";

fn ir_at(wiper: [f64; 2]) -> Ir {
    let mut ir = Ir::default();
    ir.machine.machine_type = "A1".to_string();
    ir.machine.nozzle_diameter = 0.4;
    ir.machine.first_layer_height = 0.2;
    ir.machine.typical_layer_height = 0.2;
    ir.wiping.preferred_mode = "tower".to_string();
    ir.wiping.wiper_x = wiper[0];
    ir.wiping.wiper_y = wiper[1];
    ir.tower.use_towers = true;
    ir.sheath.base_expand = 5.0;
    ir
}

fn write_input(tag: &str) -> PathBuf {
    /* ★ 每个用例换一份独有输入：管线**原地覆盖**，同名文件会被上一轮的产物顶掉 */
    let path = std::env::temp_dir().join(format!("mkpse-pp-tower-ask-{tag}.gcode"));
    std::fs::write(&path, GCODE).expect("写临时输入");
    path
}

/// 假回调：按构造时说的那样答，并把被调次数 / 复检回告记下来。
struct FakeAsk {
    reply: Reply,
    asked: usize,
    verified: Vec<Option<CollisionFacts>>,
}

enum Reply {
    /// 用内核算出来的候选位置（正常"自动挪塔"）
    UseCandidate,
    /// 用一个**仍然会撞**的位置
    MoveTo([f64; 2]),
    Decline,
    Cancel,
    Fail,
}

impl FakeAsk {
    fn new(reply: Reply) -> Self {
        Self {
            reply,
            asked: 0,
            verified: Vec::new(),
        }
    }
}

impl TowerAsk for FakeAsk {
    fn ask(
        &mut self,
        _facts: &CollisionFacts,
        candidate: Option<[f64; 2]>,
        _current: [f64; 2],
        _tower_height_mm: f64,
    ) -> TowerAskOutcome {
        self.asked += 1;
        match &self.reply {
            Reply::UseCandidate => match candidate {
                Some(pos) => TowerAskOutcome::Moved(pos),
                None => TowerAskOutcome::Declined,
            },
            Reply::MoveTo(pos) => TowerAskOutcome::Moved(*pos),
            Reply::Decline => TowerAskOutcome::Declined,
            Reply::Cancel => TowerAskOutcome::Cancelled,
            Reply::Fail => TowerAskOutcome::Failed {
                message: "通道断了".to_string(),
            },
        }
    }

    fn verified(&mut self, after: Option<&CollisionFacts>) {
        self.verified.push(after.cloned());
    }
}

/// 记事件的 sink（顺带算"步序 + 步内比例"那个单调序列）
#[derive(Default)]
struct Recorder {
    events: Vec<(Step, Option<f32>)>,
}

impl ProgressSink for Recorder {
    fn emit(&mut self, event: ProgressEvent) {
        self.events.push((event.step, event.fraction_in_step));
    }
}

impl Recorder {
    fn key(&self) -> Vec<f32> {
        self.events
            .iter()
            .map(|(step, fraction)| {
                let index = STEPS
                    .iter()
                    .position(|s| s == step)
                    .expect("每一步都在 STEPS 里") as f32;
                index + fraction.unwrap_or(0.0)
            })
            .collect()
    }
}

fn run_with(
    tag: &str,
    ask: Option<&mut dyn TowerAsk>,
) -> Result<pipeline::ProcessResult, PostprocError> {
    run_with_mode(tag, TowerRelocateModeIr::Off, ask)
}

/// 同上，但明说这一趟是**哪一档**（2026-11-04：三档各有各的语义）。
fn run_with_mode(
    tag: &str,
    mode: TowerRelocateModeIr,
    ask: Option<&mut dyn TowerAsk>,
) -> Result<pipeline::ProcessResult, PostprocError> {
    let path = write_input(tag);
    let mut sink = Recorder::default();
    let cancel = CancelToken::new();
    let mut ir = ir_at([20.0, 20.0]);
    ir.prefs.auto_tower_position = mode;
    pipeline::process_with_ir_with_tower_ask(
        IrProcessRequest {
            gcode_path: path,
            ir,
            output_path: None,
            calibration: Default::default(),
        },
        ask,
        &mut sink,
        &cancel,
    )
}

/// **1. 没人问 = 一字不差**：老入口与"递了 `None`"的新入口，碰撞事实必须相同。
#[test]
fn no_asker_keeps_the_old_behaviour() {
    let path = write_input("no-asker-old");
    let mut sink = Recorder::default();
    let cancel = CancelToken::new();
    let old = process_with_ir_with_tower_ask(
        IrProcessRequest {
            gcode_path: path.clone(),
            ir: ir_at([20.0, 20.0]),
            output_path: None,
            calibration: Default::default(),
        },
        None,
        &mut sink,
        &cancel,
    );
    let old_facts = match old {
        Err(PostprocError::Collision { facts }) => *facts,
        Err(e) => panic!("这份输入应当撞，却是别的错：{e:?}"),
        Ok(_) => panic!("这份输入应当撞，却跑完了"),
    };

    /* 新入口、没人递回调 */
    let new = run_with("no-asker-new", None);
    let new_facts = match new {
        Err(PostprocError::Collision { facts }) => *facts,
        Err(e) => panic!("递了 None 也应当撞，却是别的错：{e:?}"),
        Ok(_) => panic!("递了 None 也应当撞，却跑完了"),
    };
    assert_eq!(
        old_facts, new_facts,
        "没人问的两条路必须拿到同一份事实（逐字段相等）"
    );
}

/// **2. 问了并挪**：用候选位置 ⇒ 管线跑完，复检干净，`verified(None)`。
#[test]
fn a_moved_tower_passes_the_recheck_and_the_pipeline_finishes() {
    let mut ask = FakeAsk::new(Reply::UseCandidate);
    let done = run_with_mode("moved", TowerRelocateModeIr::Ask, Some(&mut ask))
        .expect("挪到候选位置应当跑完");
    assert_eq!(ask.asked, 1, "只问一次");
    assert_eq!(ask.verified.len(), 1, "复检回告只来一次（挪了才复检）");
    assert!(
        ask.verified[0].is_none(),
        "复检必须干净：{:?}",
        ask.verified[0]
    );

    /* 挪后的塔位真的不撞（拿管线交出来的最终 IR 再验一遍，而不是信回调） */
    let out = std::fs::read_to_string(&done.output_path).expect("读输出");
    let lines: Vec<String> = out.lines().map(str::to_string).collect();
    let after = detect_tower_collision(&lines, &done.detail_facts.ir);
    assert!(
        !after.has_collision,
        "输出里仍然撞：{:?}",
        after.facts(&done.detail_facts.ir)
    );
}

/// **3. 复检不过**：回调给一个仍然撞的位置 ⇒ 明确失败，不许生成成功结果。
#[test]
fn a_position_that_still_collides_is_refused() {
    let mut ask = FakeAsk::new(Reply::MoveTo([30.0, 30.0]));
    let outcome = run_with_mode("still", TowerRelocateModeIr::Ask, Some(&mut ask));
    match outcome {
        Err(PostprocError::Collision { facts }) => {
            assert!(
                facts.conflict_count > 0,
                "带出来的是**挪后**那份事实（仍然撞）：{facts:?}"
            );
        }
        Err(e) => panic!("复检不过必须报碰撞，却是别的错：{e:?}"),
        Ok(_) => panic!("复检不过却跑完了（不许假装成功）"),
    }
    assert_eq!(ask.verified.len(), 1);
    assert!(
        ask.verified[0].is_some(),
        "复检回告要如实说\"还撞\"：{:?}",
        ask.verified[0]
    );
}

/// **4. 等待中取消**：回调说取消 ⇒ `Cancelled`（不是碰撞、不是成功）。
#[test]
fn cancelling_while_asking_is_a_cancellation() {
    let mut ask = FakeAsk::new(Reply::Cancel);
    match run_with_mode("cancel", TowerRelocateModeIr::Ask, Some(&mut ask)) {
        Err(PostprocError::Cancelled { .. }) => {}
        Err(e) => panic!("取消必须收成 Cancelled，却是别的错：{e:?}"),
        Ok(_) => panic!("取消却跑完了"),
    }
    assert_eq!(ask.asked, 1, "取消发生在问句里");
    assert!(ask.verified.is_empty(), "没挪，就没有复检");
}

/// **4.5 用户不挪**：`Declined` ⇒ 仍是碰撞失败（不是成功、不是取消），也不复检。
#[test]
fn a_declined_answer_stops_with_a_collision() {
    let mut ask = FakeAsk::new(Reply::Decline);
    match run_with_mode("declined", TowerRelocateModeIr::Ask, Some(&mut ask)) {
        Err(PostprocError::Collision { .. }) => {}
        Err(e) => panic!("不挪必须收成碰撞，却是别的错：{e:?}"),
        Ok(_) => panic!("不挪却跑完了"),
    }
    assert_eq!(ask.asked, 1);
    assert!(ask.verified.is_empty(), "没挪，就没有复检");
}

/// **5. 交互失败**：通道坏了 ⇒ 明确失败（`Processing`），既不假装成功也不说成碰撞。
#[test]
fn a_broken_asker_is_an_explicit_failure() {
    let mut ask = FakeAsk::new(Reply::Fail);
    match run_with_mode("fail", TowerRelocateModeIr::Ask, Some(&mut ask)) {
        Err(PostprocError::Processing { message }) => assert_eq!(message, "通道断了"),
        Err(e) => panic!("交互失败必须明确报出来，却是别的错：{e:?}"),
        Ok(_) => panic!("交互失败却跑完了"),
    }
}

/// **6. 进度不回退**：整趟的 `(步序 + 步内比例)` 单调不减；第 6 步只出现一次。
///
/// 等待期间"停住"= 没有新事件；继续之后的下一步序号必然更大 ⇒ 序列天然单调。
/// 这条判据同时钉住"别拿前端最大值掩盖"—— 后端事件自己就得不倒退。
#[test]
fn progress_never_goes_backwards() {
    let path = write_input("progress");
    let mut sink = Recorder::default();
    let mut ask = FakeAsk::new(Reply::UseCandidate);
    let cancel = CancelToken::new();
    /* ★ 这一档必须是 `Ask`：`Off` 一律按碰撞收场（那才是"关就是关"），
    否则这条判据测的就不是"问完挪完接着跑"那条路 */
    let mut ir = ir_at([20.0, 20.0]);
    ir.prefs.auto_tower_position = TowerRelocateModeIr::Ask;
    let done = pipeline::process_with_ir_with_tower_ask(
        IrProcessRequest {
            gcode_path: path,
            ir,
            output_path: None,
            calibration: Default::default(),
        },
        Some(&mut ask),
        &mut sink,
        &cancel,
    )
    .expect("挪到候选位置应当跑完");

    let keys = sink.key();
    for pair in keys.windows(2) {
        assert!(
            pair[1] >= pair[0] - 1e-6,
            "进度倒退了：{} → {}（全部：{:?}）",
            pair[0],
            pair[1],
            keys
        );
    }
    let at_collision = sink
        .events
        .iter()
        .filter(|(step, _)| *step == Step::Collision)
        .count();
    assert!(at_collision >= 1, "第 6 步必须出现（问句就停在这一步）");
    assert!(
        done.executed_steps.contains(&Step::Collision),
        "步骤账里要能看见第 6 步（失败记录也靠它说\"停在哪\"）"
    );
}

/*
 * ============ 2026-11-04：`Auto`（"开"）那一档不问话 ============
 *
 * 作者把设置页那颗开关的口径改了：「关就是关，开就是开」「真正的开是直接替用户确认了
 * 直接移动塔的位置」。所以下面两条钉的是**同一份会撞的输入，两档两种行为**：
 *   · `Auto`：一次 `ask` 都不发生，塔照样被挪开、这一趟跑完；
 *   · `Off`：与引入这一格之前一字不差 —— 递了回调也不许问（关就是关）。
 */

/// **7. 开 = 不问、直接挪、跑完**。
///
/// ★ 这是这一代最要紧的一条：用户抱怨的正是"我没开它却还弹"，反过来"开了就不该弹"。
#[test]
fn the_auto_mode_moves_the_tower_without_asking() {
    let mut ask = FakeAsk::new(Reply::Decline);
    let done = run_with_mode("auto", TowerRelocateModeIr::Auto, Some(&mut ask))
        .expect("开那一档应当直接把塔挪开、跑完");
    assert_eq!(ask.asked, 0, "开就是开 —— 一声都不许问");
    assert_eq!(ask.verified.len(), 1, "挪了就要复检回告（归档那笔账靠它）");
    assert!(ask.verified[0].is_none(), "复检必须干净");

    /* 拿管线交出来的最终 IR 再验一遍，而不是信回告 */
    let out = std::fs::read_to_string(&done.output_path).expect("读输出");
    let lines: Vec<String> = out.lines().map(str::to_string).collect();
    let after = detect_tower_collision(&lines, &done.detail_facts.ir);
    assert!(
        !after.has_collision,
        "输出里仍然撞：{:?}",
        after.facts(&done.detail_facts.ir)
    );
}

// **8. 开 + 有没有位置都不问**：`Auto` 只有一条路 —— 有位置就挪，没位置就按
// 碰撞收场；**永远不换成"那就问一句"**（那正是上一代口径最招人烦的态）。
#[test]
fn the_auto_mode_never_falls_back_to_asking() {
    let mut ask = FakeAsk::new(Reply::UseCandidate);
    let outcome = run_with_mode("auto-loop", TowerRelocateModeIr::Auto, Some(&mut ask));
    /* 这条输入板上还有地方 ⇒ 应当跑完；要紧的是下面那两条断言 */
    if let Err(e) = &outcome {
        assert!(
            matches!(e, PostprocError::Collision { .. }),
            "开那一档只允许两种收场：跑完 / 碰撞，却是：{e:?}"
        );
    }
    assert_eq!(ask.asked, 0, "开那一档永远不问");
}

/// **9. 关 = 关**：递了回调也不许问，行为与"没人问"一字不差。
#[test]
fn the_off_mode_never_asks_even_when_an_asker_is_given() {
    let mut ask = FakeAsk::new(Reply::UseCandidate);
    match run_with_mode("off-with-asker", TowerRelocateModeIr::Off, Some(&mut ask)) {
        Err(PostprocError::Collision { .. }) => {}
        Err(e) => panic!("关那一档应当报碰撞，却是别的错：{e:?}"),
        Ok(_) => panic!("关那一档却跑完了（塔不许被动）"),
    }
    assert_eq!(ask.asked, 0, "关就是关 —— 有回调也不问");
    assert!(ask.verified.is_empty(), "没挪，就没有复检");
}
