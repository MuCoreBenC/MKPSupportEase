//! MKP 标记协议 v1 的自检判据（对照
//! `machine-motion/protocol/mkp-marks.v1.json` 的 invariants）。
//!
//! 为什么在本仓自己实现一份迷你校验器，而不是调对方的 `tools/mkp-lint.mjs`：
//! 那是另一个仓库的 Node 脚本，测试不该依赖它在不在、装没装。这里只覆盖
//! 与发射端直接相关的那几条 error 级判据 —— 它们正好是 doc §3 三个坑的探针：
//!
//! - `PAIRED` / `NO_NESTING`（**别名区间同样参与**）→ 抓 swap#2 越界包住 `;DISK_WIPE_*`
//! - `EV_SHAPE` → 抓 pass2 跳段把 swap#1 整段吃掉
//! - `EV_MONO` → 抓底面涂胶块被提前落地导致 ev 倒退
//! - `STATS_MATCH` → 抓 finalize 的计数与产物不符

mod golden_common;

use golden_common as gc;

const BEGIN_PREFIX: &str = ";MKP_BEGIN ";
const END_PREFIX: &str = ";MKP_END ";

/// 观看端别名表（protocol 的 `aliases`）：后处理已有的成对标记，等价参与嵌套检查。
const ALIASES: &[(&str, &str, &str)] = &[
    (
        "tower",
        ";Tower_Base_Layer_Gcode",
        ";Tower Base Layer Finished",
    ),
    ("tower", ";Tower_Layer_Gcode", ";Tower_Layer_Gcode Finished"),
    ("wipe", ";DISK_WIPE_START", ";DISK_WIPE_END"),
];

#[derive(Debug)]
struct Span {
    kind: String,
    ev: Option<i64>,
    line: usize,
    crossed_layer: Option<usize>,
}

/// `NO_LAYER_SPLIT` 用的换层标记（protocol 的 `layerMarkers`）。
fn is_layer_marker(line: &str) -> bool {
    let rest = match line.strip_prefix(';') {
        Some(r) => r.trim_start(),
        None => return false,
    };
    rest.starts_with("CHANGE_LAYER")
        || rest.starts_with("layer num")
        || rest.starts_with("LAYER:")
        || rest.starts_with("Z_HEIGHT:")
}

/// pass1 + pass2 全链（与 `golden_g2.rs` 同一条路径）。
fn run_pass2_chain(input: &str) -> Vec<String> {
    let content: Vec<String> = gc::split_lines(input);
    let ir_json = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/ir/golden_42274_2.json"),
    )
    .expect("读不到 IR fixture");
    let mut ir_data: postprocess::ir::Ir =
        serde_json::from_str(&ir_json).expect("IR JSON 反序列化失败");
    let toml_machine = ir_data.machine.machine_type.clone();
    let mut progress = |_pct: f64, _msg: String| {};
    let p1 = postprocess::postproc::pass1::first_pass(
        &content,
        &mut ir_data,
        &toml_machine,
        &mut progress,
        &postprocess::diag::CancelToken::new(),
        postprocess::postproc::cancel::DEFAULT_CANCEL_CHECK_INTERVAL,
    )
    .expect("first_pass 失败");
    let final_tower_height = p1.stats.max_z_height;
    let mut pass2_stats = Default::default();
    let p2 = postprocess::postproc::pass2::second_pass(
        p1,
        &mut ir_data,
        final_tower_height,
        &toml_machine,
        &mut progress,
        &mut pass2_stats,
        &postprocess::diag::CancelToken::new(),
        postprocess::postproc::cancel::DEFAULT_CANCEL_CHECK_INTERVAL,
    )
    .expect("second_pass 失败");
    postprocess::postproc::marks::finalize(p2.lines, &toml_machine)
}

fn kv(line: &str, key: &str) -> Option<String> {
    let needle = format!(" {key}=");
    let start = line.find(&needle)? + needle.len();
    let rest = &line[start..];
    let end = rest.find(' ').unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

/// 扫出全部区间：`MKP_BEGIN/END` + 别名成对标记。顺带校验 `PAIRED` 与 `NO_NESTING`。
fn collect_spans(lines: &[String]) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut open: Option<Span> = None;

    for (i, raw) in lines.iter().enumerate() {
        let line = raw.trim();
        let ln = i + 1;

        if let Some(span) = open.as_mut().filter(|_| is_layer_marker(line)) {
            span.crossed_layer = Some(ln);
        }

        let opened = if let Some(rest) = line.strip_prefix(BEGIN_PREFIX) {
            let kind = rest
                .split_whitespace()
                .next()
                .expect("MKP_BEGIN 后必须有 kind")
                .to_string();
            Some(Span {
                kind,
                ev: kv(line, "ev").map(|v| v.parse().expect("ev 必须是整数")),
                line: ln,
                crossed_layer: None,
            })
        } else {
            ALIASES
                .iter()
                .find(|(_, begin, _)| line == *begin)
                .map(|(kind, _, _)| Span {
                    kind: (*kind).to_string(),
                    ev: None,
                    line: ln,
                    crossed_layer: None,
                })
        };
        if let Some(span) = opened {
            assert!(
                open.is_none(),
                "NO_NESTING: 第 {} 行的 {} 套在第 {} 行的 {} 里",
                span.line,
                span.kind,
                open.as_ref().unwrap().line,
                open.as_ref().unwrap().kind
            );
            open = Some(span);
            continue;
        }

        let closed_kind = if let Some(rest) = line.strip_prefix(END_PREFIX) {
            Some(rest.trim().to_string())
        } else {
            ALIASES
                .iter()
                .find(|(_, _, end)| line == *end)
                .map(|(kind, _, _)| (*kind).to_string())
        };
        if let Some(kind) = closed_kind {
            let span = open
                .take()
                .unwrap_or_else(|| panic!("PAIRED: 第 {ln} 行的 END {kind} 没有对应的 BEGIN"));
            assert_eq!(
                span.kind, kind,
                "PAIRED: 第 {ln} 行 END {kind} 与第 {} 行 BEGIN {} 不同类",
                span.line, span.kind
            );
            assert!(
                span.crossed_layer.is_none(),
                "NO_LAYER_SPLIT: 第 {} 行开始的 {} 区间被第 {} 行的换层标记劈开",
                span.line,
                span.kind,
                span.crossed_layer.unwrap()
            );
            spans.push(span);
        }
    }

    assert!(
        open.is_none(),
        "PAIRED: 第 {} 行的 BEGIN {} 到文件尾都没有 END",
        open.as_ref().unwrap().line,
        open.as_ref().unwrap().kind
    );
    spans
}

#[test]
fn marks_satisfy_protocol_invariants() {
    let lines = run_pass2_chain(&gc::read_golden("42274.2.gcode"));
    let spans = collect_spans(&lines);

    // EV_REQUIRED：core 类型必带 ev（≥1）
    let mut events: Vec<(i64, Vec<String>)> = Vec::new();
    for span in &spans {
        if span.kind != "swap" && span.kind != "paint" {
            continue;
        }
        let ev = span
            .ev
            .unwrap_or_else(|| panic!("EV_REQUIRED: 第 {} 行的 {} 没有 ev=", span.line, span.kind));
        assert!(
            ev >= 1,
            "EV_REQUIRED: 第 {} 行 ev={ev} 不是 >=1 的整数",
            span.line
        );
        match events.last_mut() {
            Some((last_ev, kinds)) if *last_ev == ev => kinds.push(span.kind.clone()),
            _ => events.push((ev, vec![span.kind.clone()])),
        }
    }

    assert!(!events.is_empty(), "42274.2 应当至少有一次换笔事件");

    // EV_MONO：按落盘行序单调递增，且每个 ev 只出现在一段连续区间里
    let evs: Vec<i64> = events.iter().map(|(ev, _)| *ev).collect();
    let mut sorted = evs.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(
        evs, sorted,
        "EV_MONO: ev 未按落盘行序单调递增（实际序列 {evs:?}）"
    );

    // EV_SHAPE：同 ev 拼成 swap→paint→swap
    for (ev, kinds) in &events {
        assert_eq!(
            kinds.join("→"),
            "swap→paint→swap",
            "EV_SHAPE: ev={ev} 是 {}，期望 swap→paint→swap",
            kinds.join("→")
        );
    }

    // STATS_MATCH：finalize 声明的条数与实际一致
    let stats = lines
        .iter()
        .find(|l| l.trim().starts_with(";MKP_STATS "))
        .expect("产物尾部应有 ;MKP_STATS");
    let swaps = spans.iter().filter(|s| s.kind == "swap").count();
    let paints = spans.iter().filter(|s| s.kind == "paint").count();
    let towers = spans.iter().filter(|s| s.kind == "tower").count();
    assert_eq!(
        kv(stats, "events").as_deref(),
        Some(events.len().to_string().as_str()),
        "STATS_MATCH: events 与实际不符（{stats}）"
    );
    assert_eq!(
        kv(stats, "swaps").as_deref(),
        Some(swaps.to_string().as_str()),
        "STATS_MATCH: swaps 与实际不符（{stats}）"
    );
    assert_eq!(
        kv(stats, "paints").as_deref(),
        Some(paints.to_string().as_str()),
        "STATS_MATCH: paints 与实际不符（{stats}）"
    );
    // 塔走别名（本仓一行没为它改过），但条数要报进 STATS 与观看端对账。
    // 口径：两种别名写法**各算一段**，且只在闭合时计入。
    assert_eq!(
        kv(stats, "towers").as_deref(),
        Some(towers.to_string().as_str()),
        "STATS_MATCH: towers 与实际不符（{stats}）"
    );

    // 文件头有 MKP_INFO，且版本是 1
    let info = lines
        .iter()
        .find(|l| l.trim().starts_with(";MKP_INFO "))
        .expect("产物头部应有 ;MKP_INFO");
    assert_eq!(kv(info, "version").as_deref(), Some("1"));

    // INLINE_MARK：五个 op 的标记必须独占一行、`;` 顶格
    for (i, raw) in lines.iter().enumerate() {
        let line = raw.trim();
        for op in [
            ";MKP_INFO",
            ";MKP_STATS",
            ";MKP_BEGIN",
            ";MKP_END",
            ";MKP_POINT",
        ] {
            if let Some(pos) = line.find(op) {
                assert_eq!(
                    pos,
                    0,
                    "INLINE_MARK: 第 {} 行把 {op} 写在了指令后面：{raw:?}",
                    i + 1
                );
            }
        }
    }
}

/// `;MKP_STAGE:` 这类历史标记必须原样留着（观看端归 LEGACY_MARK，只报数）。
#[test]
fn legacy_stage_marks_survive() {
    let lines = run_pass2_chain(&gc::read_golden("42274.2.gcode"));
    assert!(
        lines.iter().any(|l| l.contains(";MKP_STAGE: GlueStart")),
        "历史标记 ;MKP_STAGE: 不应被本轮改动清掉"
    );
}
