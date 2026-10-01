//! Task 18.1/18.2 性能探针（手动跑，具名 ignore——不进常规判据面）。
//!
//! 与旧仓库 `processor/bench_test.go` 的 BenchmarkProcessInPlace 同型对照：
//! 同一输入文件、同一 fixture IR（golden_42274_2.json）、每轮 fresh IR、
//! first_pass + second_pass 全链。**不含** write hooks / printtime / 塔模式
//! （fixture IR 单色）——那是 CLI 全链数字的面（tasks.md 18.1）。
//!
//! 运行（CPU 口径，抗机器负载竞争）：
//! ```sh
//! cargo test -p mkp-postproc --test perf_probe --no-run
//! B=$(ls -t target/debug/deps/perf_probe-* | grep -v '\.d$' | head -1)
//! /usr/bin/time -p $B --ignored --nocapture --test-threads=1
//! ```
//! user CPU ÷ 迭代数 = 每链 CPU；输出里的每轮墙钟仅参考（负载噪声大时不可判据）。

use std::time::Instant;

mod golden_common;
use golden_common as gc;

/// 迭代数：与 Go 侧 `-benchtime 10x` 对齐。
const ITERS: usize = 10;

fn fixture_ir() -> postprocess::ir::Ir {
    let ir_json = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/ir/golden_42274_2.json"),
    )
    .expect("读不到 IR fixture");
    serde_json::from_str(&ir_json).expect("IR JSON 反序列化失败")
}

/// 响亮 ignore：性能探针不是正确性判据，缺手动意图不许静默跑。
#[test]
#[ignore = "性能探针（Task 18）：手动 --ignored 运行，运行方式见模块文档；正确性判据在 golden_g1/g2"]
fn fixture_chain_probe() {
    let input = gc::read_golden("42274.2.gcode");
    let content: Vec<String> = gc::split_lines(&input);
    let cancel = postprocess::diag::CancelToken::new();
    let iv = postprocess::postproc::cancel::DEFAULT_CANCEL_CHECK_INTERVAL;

    // 预热一轮（页错误/首次分配不计入）
    {
        let mut ir = fixture_ir();
        let m = ir.machine.machine_type.clone();
        let mut p = |_, _: String| {};
        let p1 =
            postprocess::postproc::pass1::first_pass(&content, &mut ir, &m, &mut p, &cancel, iv)
                .unwrap();
        let h = p1.stats.max_z_height;
        let mut s = Default::default();
        let _ = postprocess::postproc::pass2::second_pass(
            p1, &mut ir, h, &m, &mut p, &mut s, &cancel, iv,
        )
        .unwrap();
    }

    let mut walls = Vec::with_capacity(ITERS);
    for i in 0..ITERS {
        let t0 = Instant::now();
        let mut ir = fixture_ir();
        let m = ir.machine.machine_type.clone();
        let mut p = |_, _: String| {};
        let p1 =
            postprocess::postproc::pass1::first_pass(&content, &mut ir, &m, &mut p, &cancel, iv)
                .unwrap();
        let h = p1.stats.max_z_height;
        let mut s = Default::default();
        let p2 = postprocess::postproc::pass2::second_pass(
            p1, &mut ir, h, &m, &mut p, &mut s, &cancel, iv,
        )
        .unwrap();
        walls.push(t0.elapsed().as_secs_f64());
        // 反空转：输出必须非零（防止探针空跑还自称测了东西）
        assert!(!p2.lines.is_empty(), "第 {i} 轮输出为空");
    }
    for (i, w) in walls.iter().enumerate() {
        println!("iter {i:2}: {w:.4}s");
    }
    let total: f64 = walls.iter().sum();
    println!("total {ITERS} iters: {total:.4}s (wall, 参考值)");
}
