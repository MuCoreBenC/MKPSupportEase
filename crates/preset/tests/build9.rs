//! 判据 K4：**9 份真实预设过 `build(cfg, None)` 与 Go 侧导出的 IR 逐字段树相等**。
//!
//! 来源：`mkp-sr/crates/ir/tests/build9.rs`，搬入 mkp-ssr 时的改动只有三处：
//! ① crate 名（`mkp_ir::build` → `preset::build`）；
//! ② fixture 路径指向内核那一份（见下）；
//! ③ 加了一条反空转哨兵。
//!
//! ## fixture 为什么不复制一份到本 crate
//!
//! 9 份预设与 9 份 IR JSON 都已经在 `crates/postprocess/tests/fixtures/` 里
//! （Task 2 随内核整包搬入，且与 `mkp-sr` 侧实测逐字节相同）。
//! **再复制一份 = 两份判据资产**，而两份资产必然漂移；漂移的表现是
//! 「改了一处、另一处照旧绿」。所以这里跨 crate 指路径，不复制。
//!
//! ## 判据资产的派生关系与证据等级（**必须先说清**）
//!
//! `fixtures/ir/build9/*.json` 由**旧 Go 仓库的一次性工具**导出
//! （`ir.Build(cfg, nil, "", nil)` 的 `MarshalIndent`，**nil registry = 硬编码默认分支**，
//! 与本测试的 `build(&cfg, None, Fallback)` 同一分支）。
//! 证据等级 = **对照当时 Go 实现的输出**（不是独立快照）：Go 若有 bug，这里逐字段复制它。
//!
//! 两个坑（来源仓库踩过，原样保留）：
//! - Go 的 `MarshalIndent` 把 `4200.0` 写成 `4200`、nil 切片写成 `null` ⇒
//!   **逐字节 diff 必红**，必须走 `diff_values` 的数值归一 + `null↔空集合` 等价；
//! - `registry = None` 与 `Some` 走**不同**默认值分支，判据资产是 `None` 那支；
//!   钩子实际走的是 `Some`（Task 4.5 另有一条自比判据）。

use preset::{CalibrationExecMode, build, read_preset};
use serde_json::Value;
use std::path::{Path, PathBuf};

const NAMES: &[&str] = &[
    "A1",
    "A1F",
    "A1F_260628",
    "A1M",
    "A1MF",
    "A1MF_260628",
    "P1",
    "P2",
    "X1",
];

/// 内核那份 fixture 的根（从本 crate 的 manifest 目录出发）。
fn core_fixtures(rel: String) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../postprocess/tests/fixtures/{rel}"))
}

/// 数字统一按 f64 比较（Go MarshalIndent 把 4200.0 写成 4200，
/// serde_json 会把两侧解析成不同 Number 变体，树等价须做数值归一）。
fn diff_values(path: &str, want: &Value, got: &Value, out: &mut Vec<String>) {
    match (want, got) {
        (Value::Object(w), Value::Object(g)) => {
            for (k, wv) in w {
                match g.get(k) {
                    Some(gv) => diff_values(&format!("{path}.{k}"), wv, gv, out),
                    None => out.push(format!("{path}.{k}: Go 有，Rust 无")),
                }
            }
            for k in g.keys() {
                if !w.contains_key(k) {
                    out.push(format!("{path}.{k}: Rust 有，Go 无"));
                }
            }
        }
        (Value::Array(w), Value::Array(g)) => {
            if w.len() != g.len() {
                out.push(format!("{path}: 数组长度 Go={} Rust={}", w.len(), g.len()));
            }
            for (i, (wv, gv)) in w.iter().zip(g.iter()).enumerate() {
                diff_values(&format!("{path}[{i}]"), wv, gv, out);
            }
        }
        (Value::Number(w), Value::Number(g)) => {
            let wf = w.as_f64().unwrap_or(f64::NAN);
            let gf = g.as_f64().unwrap_or(f64::NAN);
            if wf != gf {
                out.push(format!("{path}: 数值 Go={w} Rust={g}"));
            }
        }
        // Go 的 nil 切片/map 序列化为 null，Rust 模型已按 null_to_empty 契约
        // 把 null 收成空集合（见 core 的 ir/types.rs）；两侧空形态（null/[]/{}）语义等价。
        (Value::Null, Value::Array(a)) if a.is_empty() => {}
        (Value::Array(a), Value::Null) if a.is_empty() => {}
        (Value::Null, Value::Object(o)) if o.is_empty() => {}
        (Value::Object(o), Value::Null) if o.is_empty() => {}
        (Value::Null, Value::Null) => {}
        _ => {
            if want != got {
                out.push(format!("{path}: Go={want} Rust={got}"));
            }
        }
    }
}

/// 反空转哨兵：跨 crate 的相对路径一旦写错，两侧都读不到文件 ——
/// 而「读不到」与「全都一致」在终端上长得一模一样（AGENTS.md §7③）。
#[test]
fn the_fixture_path_actually_points_at_the_kernel_copy() {
    // 数量下限写字面量 9（不插值 NAMES.len()，否则删掉几条名字判据也照样绿 —— §7⑤）。
    assert!(NAMES.len() == 9, "预设集必须是 9 份，实测 {}", NAMES.len());
    for name in NAMES {
        let preset = core_fixtures(format!("presets/{name}.toml"));
        let ir_json = core_fixtures(format!("ir/build9/{name}.json"));
        assert!(
            preset.is_file(),
            "预设 fixture 不存在：{}",
            preset.display()
        );
        assert!(
            ir_json.is_file(),
            "IR 参考 fixture 不存在：{}",
            ir_json.display()
        );
    }
}

#[test]
fn nine_presets_match_go_build_output_field_by_field() {
    let mut total_diffs = 0;
    for name in NAMES {
        let preset_path = core_fixtures(format!("presets/{name}.toml"));
        let file = read_preset(&preset_path).unwrap();
        let ir = build(&file.config, None, CalibrationExecMode::Fallback)
            .unwrap_or_else(|e| panic!("{name}: build 失败: {e}"));
        let got = serde_json::to_value(&ir).unwrap();

        let want_text =
            std::fs::read_to_string(core_fixtures(format!("ir/build9/{name}.json"))).unwrap();
        let want: Value = serde_json::from_str(&want_text).unwrap();

        let mut diffs = Vec::new();
        diff_values(name, &want, &got, &mut diffs);
        if !diffs.is_empty() {
            total_diffs += diffs.len();
            eprintln!("{name}: {} 处字段差异：", diffs.len());
            for d in diffs.iter().take(30) {
                eprintln!("  {d}");
            }
        }
    }
    assert_eq!(total_diffs, 0, "9 份预设共 {total_diffs} 处 build 输出差异");
}
