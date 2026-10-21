//! 后处理两条入口的判据：**钩子那条脸与 CLI 那条脸必须产出同一份字节**。
//!
//! 背景：钩子路径手里已经是 `Ir`（预设 TOML 经 `mkp-preset` 映射而来），不是 TOML
//! 配置文件，所以 `pipeline` 多了一张脸。多一张脸就多一条会漂移的产品路径，因此这条
//! 判据钉的不是「新脸能跑」，而是**新脸与旧脸字节相等** —— 与产物长什么样无关，
//! 生成逻辑怎么演进都不会让它假红。
//!
//! ## 判据资产
//!
//! - 输入 `tests/golden/42274.2.gcode`：一份真实的切片产物，当不变的输入用。
//! - 配置 `tests/fixtures/config/A1.toml`：A1 的一份完整配置。
//!   **刻意不用** `fixtures/ir/golden_42274_2.json` —— 那份带 ±999 的机器范围，
//!   喂进整链会在边界检查处报 `E_GCODE_BOUNDARY_001`，看着像搬错了，其实是选错
//!   fixture（AGENTS.md §7⑥）。

//! ## 一个必须先说清的坑：整链输出依赖**进程内**的伪随机序列状态
//!
//! `postproc::pass2` 的可变擦拭点用 `gcode::get_pseudo_random()`
//! （`G1 X15 Y2{digit}` / `G1 X20 Y1{digit}`，加上工具头偏移后就是输出里的
//! `G1 X31.547 Y4x.547`）。那个序列是**进程级全局状态**，只有
//! `gcode::reset_pseudo_random()` 能把它拨回起点。
//!
//! 后果（本判据第一版实测撞上）：同一个进程里跑第二遍整链，擦拭 Y 会不一样 ——
//! 第一版这两条断言都报「首个不同字节在 offset 126603（left=0x33 right=0x31）」，
//! 看着像 `process_with_ir` 搬错了，真因是**我自己制造的假红**（AGENTS.md §6.1）。
//! 所以每次整链前必须 reset，且两条断言要串行（并行会让两个线程交错取同一条序列）。

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use postprocess::diag::CancelToken;
use postprocess::pipeline::{self, IrProcessRequest, NoProgress, ProcessRequest};

/// 伪随机序列是进程级状态 ⇒ 跑整链必须独占它。
static SEQUENCE: Mutex<()> = Mutex::new(());

fn lock_sequence() -> MutexGuard<'static, ()> {
    // 上一个用例 panic 会让锁中毒；这里要的是「串行」，不是「毒锁传播」。
    match SEQUENCE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn golden_input() -> PathBuf {
    repo_root().join("tests/golden/42274.2.gcode")
}

fn config_a1() -> PathBuf {
    repo_root().join("tests/fixtures/config/A1.toml")
}

/// 用户偏好（`Ir.prefs`）的两颗钉子：两条脸都按同一组值跑。
///
/// 偏好化之后出厂档与旧管线常量不同（100 / 不跳 vs 20 / 跳），这里把两边都钉成
/// 同一组，判据比较的才是**入口之差**，不是「谁用了哪套默认值」。
const PREFERENCE_PINS: &[&str] = &[
    "Prefs.BeginStageRetractZLiftMm=20",
    "Prefs.SkipVibrationCalibration=true",
];

/// 把输入复制进临时目录，原地跑一遍，返回原地覆盖后的字节。
///
/// **调用方必须持有 [`lock_sequence`] 的锁**：里面会 reset 那条进程级伪随机序列。
fn run_in_place(with_ir: bool) -> Vec<u8> {
    postprocess::gcode::reset_pseudo_random();

    let dir = tempfile::tempdir().expect("临时目录");
    let work = dir.path().join("input.gcode");
    std::fs::copy(golden_input(), &work).expect("复制输入");

    let cancel = CancelToken::new();
    let mut sink = NoProgress;

    if with_ir {
        // 钩子路径：配置先变成 IR，再把 IR 交给管线。
        let raw = std::fs::read_to_string(&work).expect("读输入");
        let overrides: Vec<String> = PREFERENCE_PINS.iter().map(|s| (*s).to_string()).collect();
        let ir = pipeline::resolve_ir(&config_a1(), &overrides, Some(&raw)).expect("配置 → IR");
        /* ★ 走**产品真正用的那个**函数（`hook.rs` 调的就是它），不是 `process_with_ir`。
        2026-10-19 收敛之后，`process_with_ir` / `_and_cancel_interval` 已随
        "零产品调用方"一起删除 —— 判据若还指着它们，守的就是一条没人走的路。 */
        pipeline::process_with_ir_with_tower_ask(
            IrProcessRequest {
                gcode_path: work.clone(),
                ir,
                output_path: None, // 原地
                calibration: Default::default(),
            },
            None,
            &mut sink,
            &cancel,
        )
        .expect("process_with_ir_with_tower_ask 必须成功");
    } else {
        pipeline::process(
            ProcessRequest {
                gcode_path: work.clone(),
                config_path: config_a1(),
                overrides: PREFERENCE_PINS.iter().map(|s| (*s).to_string()).collect(),
                output_path: None, // 原地
            },
            &mut sink,
            &cancel,
        )
        .expect("process 必须成功");
    }

    std::fs::read(&work).expect("读回原地输出")
}

/// 大对象**不要**直接 `assert_eq!`：两个 668KB 的 `Vec<u8>` 不等时输出 5.2MB，
/// 真因反而被埋掉（AGENTS.md §7⑦）。先比长度，再定位首个不同字节。
fn assert_same_bytes(what: &str, left: &[u8], right: &[u8]) {
    assert!(
        left.len() == right.len(),
        "{what}: 长度不同 left={} right={}",
        left.len(),
        right.len()
    );
    if let Some(i) = left.iter().zip(right.iter()).position(|(a, b)| a != b) {
        panic!(
            "{what}: 首个不同字节在 offset {i}（left=0x{:02x} right=0x{:02x}）",
            left[i], right[i]
        );
    }
}

#[test]
fn process_with_ir_matches_the_config_face_byte_for_byte() {
    let _seq = lock_sequence();
    let via_config = run_in_place(false);
    let via_ir = run_in_place(true);
    assert_same_bytes("两张公开脸的输出", &via_config, &via_ir);
}

// 2026-10-21：原来还有一条「与整链参考物逐字节相等」的判据，已删除 ——
// 参考物由来源实现的 CLI 生成、在本仓无法复现，拿它当锚等于把判据变成自比自。
// 本仓的产物基准只认自己维护的 `tests/golden/`（见 `golden_*.rs`）。
