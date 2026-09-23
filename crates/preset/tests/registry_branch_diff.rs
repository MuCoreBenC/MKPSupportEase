//! 判据（Task 4.5）：**`registry = Some` 与 `None` 两支的差异清单必须与快照一致**。
//!
//! 为什么需要它：K4（`build9.rs`）比的是 `registry = None` 那一支 —— 因为判据资产
//! （Go 侧导出的 9 份 IR JSON）就是 nil registry 分支的产物。而**钩子实际走的是 `Some`**
//! （param_registry 快照编进了二进制）。来源仓库对 `Some` 分支只断言「build 成功」，
//! 不比字段 ⇒ 真实用户路径上的默认值分支**没有任何字段级外部真值**。
//!
//! 本判据补的不是「真值」，是「**别悄悄变**」：把两支的差异逐字段列出来存成快照，
//! 以后任何一支的默认值改动都会让清单变。**它是自比，不是外部证据**，
//! 证据等级低于 K4，收口报告不许把两者混为一谈。
//!
//! ## 快照怎么生成（派生，不是手搓）
//!
//! ```text
//! MKPSSR_UPDATE_REGISTRY_DIFF=1 cargo test -p mkpse-preset --test registry_branch_diff
//! ```
//!
//! 生成用的**就是本文件里同一条代码路径**（先算清单、再写盘），所以快照与判据不可能
//! 因为「生成脚本和判据两套实现」而漂移。改快照必须在提交信息里说明为什么。
//!
//! 快照缺失时**响亮失败**并打印上面那条命令 —— 不静默跳过（AGENTS.md §6.1）。

use std::path::{Path, PathBuf};

use preset::{CalibrationExecMode, build, load_param_registry, read_preset};
use serde_json::Value;

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

fn core_fixtures(rel: String) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../postprocess/tests/fixtures/{rel}"))
}

fn snapshot_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/registry_branch_diff.snapshot")
}

/// 只收「值不同」的叶子，路径 + 两侧值，按路径排序 —— 输出要稳定，否则快照会因
/// HashMap 遍历顺序假红。
fn collect_diffs(prefix: &str, none_side: &Value, some_side: &Value, out: &mut Vec<String>) {
    match (none_side, some_side) {
        (Value::Object(a), Value::Object(b)) => {
            let mut keys: Vec<&String> = a.keys().chain(b.keys()).collect();
            keys.sort();
            keys.dedup();
            for k in keys {
                match (a.get(k), b.get(k)) {
                    (Some(x), Some(y)) => collect_diffs(&format!("{prefix}.{k}"), x, y, out),
                    (Some(_), None) => out.push(format!("{prefix}.{k}: 只有 None 支有")),
                    (None, Some(_)) => out.push(format!("{prefix}.{k}: 只有 Some 支有")),
                    (None, None) => unreachable!("键来自两侧的并集"),
                }
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            if a.len() != b.len() {
                out.push(format!(
                    "{prefix}: 数组长度 None={} Some={}",
                    a.len(),
                    b.len()
                ));
            }
            for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
                collect_diffs(&format!("{prefix}[{i}]"), x, y, out);
            }
        }
        (Value::Number(a), Value::Number(b)) => {
            let (x, y) = (
                a.as_f64().unwrap_or(f64::NAN),
                b.as_f64().unwrap_or(f64::NAN),
            );
            if x != y {
                out.push(format!("{prefix}: None={a} Some={b}"));
            }
        }
        _ => {
            if none_side != some_side {
                out.push(format!("{prefix}: None={none_side} Some={some_side}"));
            }
        }
    }
}

fn compute_report() -> String {
    let reg = load_param_registry();
    let mut lines = Vec::new();
    for name in NAMES {
        let file = read_preset(&core_fixtures(format!("presets/{name}.toml")))
            .unwrap_or_else(|e| panic!("{name}: 预设读不到：{e}"));
        let ir_none = build(&file.config, None, CalibrationExecMode::Fallback)
            .unwrap_or_else(|e| panic!("{name}: None 支 build 失败：{e}"));
        let ir_some = build(&file.config, Some(&reg), CalibrationExecMode::Fallback)
            .unwrap_or_else(|e| panic!("{name}: Some 支 build 失败：{e}"));

        let mut diffs = Vec::new();
        collect_diffs(
            name,
            &serde_json::to_value(&ir_none).unwrap(),
            &serde_json::to_value(&ir_some).unwrap(),
            &mut diffs,
        );
        diffs.sort();
        lines.push(format!("=== {name}: {} 处", diffs.len()));
        lines.extend(diffs);
    }
    let mut text = lines.join("\n");
    text.push('\n');
    text
}

#[test]
fn registry_branch_diff_matches_the_snapshot() {
    let report = compute_report();
    let path = snapshot_path();

    if std::env::var_os("MKPSSR_UPDATE_REGISTRY_DIFF").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).expect("建 fixtures 目录");
        std::fs::write(&path, &report).expect("写快照");
        eprintln!("已更新快照：{}（{} 字节）", path.display(), report.len());
        return;
    }

    let want = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "读不到快照 {}（{e}）。要生成请跑：\n  \
             MKPSSR_UPDATE_REGISTRY_DIFF=1 cargo test -p mkpse-preset --test registry_branch_diff",
            path.display()
        )
    });

    // 反空转哨兵：快照必须真的含 9 个 === 段，否则「空快照 + 空报告」会双绿。
    let sections = want.matches("=== ").count();
    assert!(sections == 9, "快照必须有 9 个机型段，实测 {sections}");

    if want != report {
        // 大文本不整体 assert_eq!：只打首个不同行（§7⑦ 同款理由）。
        let (mut wl, mut rl) = (want.lines(), report.lines());
        let mut n = 0;
        loop {
            n += 1;
            match (wl.next(), rl.next()) {
                (None, None) => break,
                (a, b) if a == b => continue,
                (a, b) => panic!(
                    "两支差异清单与快照不一致，首个不同在第 {n} 行：\n快照：{a:?}\n实测：{b:?}"
                ),
            }
        }
    }
}
