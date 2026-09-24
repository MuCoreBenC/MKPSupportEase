//! golden 比对 harness 的公共件。
//!
//! 消费的数据在仓库根 `tests/golden/`（vendored 自 mkp-sr，脱钩代价见 spec doc.md §7③）。
//! 本模块只做机械比对，不含任何算法。
//!
//! 判据语义与旧 Go 侧逐字对齐：
//! - 行切分 = `processor/golden_test.go:301 splitLines`：
//!   `TrimRight(s, "\n")` 后按 `\n` 切（LF-only，golden 实测无 CR）；
//! - G1/G2 先比行数再逐行 `!=`（零容差）；
//! - G3 按 `=== CASE <name> ===` 分块**点名**比对（不整文件逐行——级联错位
//!   会把真差异淹掉，旧侧 CI run 32437912254 的教训）；
//! - `<nil>` 与 `<empty>` 是 golden 里的**字面行**：nil 输出与空切片被区分，
//!   退化成空转会被发现。

// 测试写临时文件、造夹具是正当的：写盘纪律管的是**生产代码**
// （与源码扫描断言只看 `#[cfg(test)]` 之前那部分同一口径）。
#![allow(clippy::disallowed_methods)]
// 本模块被 4 个测试二进制分别编译（g1/g2/g3/integrity），每个二进制只用其中
// 一部分函数 —— dead_code 警告在此是噪声而非信号，故在模块级放开。
// 反空转由各测试自身的断言（空文件/CASE 数 0/sha 不符即 panic）承担。
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

/// 仓库根。
///
/// 与来源仓库的**唯一差异**：那边 harness 在 `crates/postproc/tests/` 下，要 `ancestors().nth(2)`
/// 上溯两级；这里是单 package，`CARGO_MANIFEST_DIR` 本身就是仓库根，上溯会跑到 `projects/` 去，
/// 于是 `read_golden` 报的是「读不到文件」而不是「判据没接上」—— 归因错位，所以这行必须改。
pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn golden_path(name: &str) -> PathBuf {
    repo_root().join("tests").join("golden").join(name)
}

/// Go `splitLines` 语义：掐掉全部尾部 `\n` 后按 `\n` 切；全空 → 空向量。
pub fn split_lines(s: &str) -> Vec<String> {
    let s = s.trim_end_matches('\n');
    if s.is_empty() {
        Vec::new()
    } else {
        s.split('\n').map(|l| l.to_string()).collect()
    }
}

/// 读 golden 文件并做反空转（3.4）：读不到 / 空文件一律点名失败，不当成通过。
pub fn read_golden(name: &str) -> String {
    let path = golden_path(name);
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("读不到 golden {name}（{}）: {e}", path.display()));
    assert!(
        !text.is_empty(),
        "golden {name} 是空文件 —— 判据已空转，视为失败（tests/golden/ 丢文件了？）"
    );
    text
}

/// `UPDATE_GOLDEN=1` 再生开关（与旧侧同名同值）。
pub fn update_golden_requested() -> bool {
    std::env::var("UPDATE_GOLDEN").ok().as_deref() == Some("1")
}

/// 再生路径：把 actual 写回 golden 并提示人工审查。只在显式开关下会被调用。
pub fn write_golden(name: &str, actual: &str) {
    let path = golden_path(name);
    fs::write(&path, actual).unwrap_or_else(|e| panic!("写 golden {name} 失败: {e}"));
    println!(
        "已更新 golden {name}（{} 字节）—— 提交前必须人工逐块审查",
        actual.len()
    );
}

/// G1/G2 判据：先比行数，再逐行（零容差），最多报 20 条差异、每条带 1 基行号
/// （Task 11.8 要求 G1 能报出确切行号，这里是报数机制的唯一实现）。
pub fn compare_line_wise(label: &str, expected: &[String], actual: &[String]) {
    if expected.len() != actual.len() {
        panic!(
            "{label}: 行数不一致 expected={} actual={}",
            expected.len(),
            actual.len()
        );
    }
    let mut diffs = String::new();
    let mut diff_count = 0usize;
    for (i, (want, got)) in expected.iter().zip(actual).enumerate() {
        if want != got {
            diff_count += 1;
            if diff_count <= 20 {
                let _ = writeln!(
                    diffs,
                    "  line {}:\n    expected: {want:?}\n    actual:   {got:?}",
                    i + 1
                );
            }
        }
    }
    assert_eq!(diff_count, 0, "{label}: {diff_count} 行不一致\n{diffs}");
}

// ---------------------------------------------------------------------------
// G3：tower_matrix.golden 的分块判据
// ---------------------------------------------------------------------------

const CASE_PREFIX: &str = "=== CASE ";

/// 判据档位（design.md §2.4 处置② / tasks.md 3.5）。
///
/// 所有 23 个 CASE **起步全是 ByteExact**：旧 Go 侧的金字塔里唯一跨架构不稳的
/// `rib/mid_tower_h250` 已被移出字节 golden（vendored 的这份就没有它）。
/// 降档通道存在但只能走 Task 8.8 的证明流程（先打印中间 cos 值证明是 FMA 类
/// 差异，写进 HONEST-BOUNDARIES），**不允许悄悄降**。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    ByteExact,
    NumericTable,
}

/// 23 个 CASE 的档位登记表。名字不在表里 = 登记腐烂 = 点名失败（不许静默放过）。
pub fn case_tier(name: &str) -> Tier {
    match name {
        // —— mini_spiral 族 ——
        "mini_spiral/out_of_range_high_nil"
        | "mini_spiral/out_of_range_neg_nil"
        | "mini_spiral/quadrant0"
        | "mini_spiral/quadrant0_skip_safez"
        | "mini_spiral/quadrant1"
        | "mini_spiral/quadrant2"
        | "mini_spiral/quadrant3" => Tier::ByteExact,
        // —— rib 族（当前全 A 档；降档须过 Task 8.8 的证明，见 Tier 文档）——
        "rib/first_layer_fillet"
        | "rib/first_layer_nofillet"
        | "rib/non_first_layer_fillet"
        | "rib/ribwidth_zero_nil"
        | "rib/short_tower_h5"
        | "rib/tall_tower_h350"
        | "rib/towerheight_zero_nil" => Tier::ByteExact,
        // —— sheath 族 ——
        "sheath/first_layer_brim"
        | "sheath/first_layer_rib_clamp"
        | "sheath/first_layer_sheath"
        | "sheath/layer1_brim_nil"
        | "sheath/layer1_sheath_rect_lowaccel"
        | "sheath/layer1_sheath_short_nil"
        | "sheath/layer3_sheath_rect_highaccel"
        | "sheath/layer4_expansion_below_min_nil"
        | "sheath/layer5_converged_nil" => Tier::ByteExact,
        other => panic!(
            "CASE {other:?} 不在档位登记表里 —— golden 与登记表已脱钩，\
             先同步 case_tier()（tasks.md 3.5）"
        ),
    }
}

/// 把 golden 文本解析成 `名字 → 块体行`。块头 `=== CASE <name> ===`，
/// 块体原样保留（`<nil>` / `<empty>` 是字面行，保持 nil 与空切片可区分）。
/// 反空转（3.4）：一个 CASE 都解析不到即失败。
pub fn parse_case_blocks(text: &str) -> BTreeMap<String, Vec<String>> {
    let mut blocks: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut current: Option<(String, Vec<String>)> = None;
    for line in split_lines(text) {
        if let Some(rest) = line.strip_prefix(CASE_PREFIX) {
            let Some(name) = rest.strip_suffix(" ===") else {
                panic!("疑似 CASE 头但格式不对: {line:?}");
            };
            if let Some((prev_name, body)) = current.take() {
                assert!(
                    blocks.insert(prev_name, body).is_none(),
                    "CASE 名字碰撞: {name}"
                );
            }
            assert!(!name.is_empty(), "CASE 名字为空: {line:?}");
            current = Some((name.to_string(), Vec::new()));
        } else {
            let (_, body) = current
                .as_mut()
                .unwrap_or_else(|| panic!("CASE 头之前出现游离行: {line:?}"));
            body.push(line);
        }
    }
    if let Some((name, body)) = current.take() {
        assert!(blocks.insert(name, body).is_none(), "CASE 名字碰撞");
    }
    assert!(
        !blocks.is_empty(),
        "golden 里一个 CASE 块都没解析到 ⇒ 解析侧已空转，判据不成立"
    );
    blocks
}

/// 渲染一个 CASE 块。`None` → `<nil>`，`Some(vec![])` → `<empty>`，
/// 其余逐行原样 —— 与旧侧 `renderCaseBlock` 逐字一致。
pub fn render_case_block(name: &str, lines: Option<&[String]>) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{CASE_PREFIX}{name} ===");
    match lines {
        None => {
            let _ = writeln!(out, "<nil>");
        }
        Some([]) => {
            let _ = writeln!(out, "<empty>");
        }
        Some(lines) => {
            for l in lines {
                let _ = writeln!(out, "{l}");
            }
        }
    }
    out
}

/// G3 判据：逐块点名比对。两边块集合必须一致（缺块/多块点名），
/// ByteExact 档块体必须逐行相等；差异报块名 + 块内 1 基行号。
pub fn compare_case_blocks(
    label: &str,
    expected: &BTreeMap<String, Vec<String>>,
    actual: &BTreeMap<String, Vec<String>>,
) {
    for name in expected.keys() {
        assert!(
            actual.contains_key(name),
            "{label}: golden 有用例 {name:?}，本次运行没有产出该块（用例被删或改名？）"
        );
    }
    for name in actual.keys() {
        assert!(
            expected.contains_key(name),
            "{label}: 本次运行产出了 golden 里没有的用例 {name:?}（新增用例须先 UPDATE_GOLDEN 并审查）"
        );
    }
    let mut failures = Vec::new();
    for (name, want) in expected {
        if case_tier(name) != Tier::ByteExact {
            continue; // 降档块不参与字节比对（当前 23 块全是 ByteExact）
        }
        let got = &actual[name];
        if want == got {
            continue;
        }
        let mut detail = format!(
            "CASE {name}: 块体不一致 expected={} 行 actual={} 行",
            want.len(),
            got.len()
        );
        for (i, (w, g)) in want.iter().zip(got.iter()).enumerate() {
            if w != g {
                detail.push_str(&format!(
                    "\n    块内 line {}:\n      expected: {w:?}\n      actual:   {g:?}",
                    i + 1
                ));
                break; // 每块报第一处差异即可定位，其余靠 UPDATE_GOLDEN 审查
            }
        }
        failures.push(detail);
    }
    assert!(
        failures.is_empty(),
        "{label}: {} 个 CASE 块不一致\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// harness 自检：nil / empty / 非空 三态在渲染与解析两侧往返后互不混淆。
    #[test]
    fn nil_and_empty_are_distinguishable() {
        let nil_block = render_case_block("a/nil_case", None);
        let empty_block = render_case_block("a/empty_case", Some(&[]));
        let real_block = render_case_block("a/real", Some(&["G1 X1".into(), "G1 X2".into()]));

        assert!(nil_block.contains("<nil>"));
        assert!(empty_block.contains("<empty>"));
        assert!(!nil_block.contains("<empty>"));
        assert!(!empty_block.contains("<nil>"));

        let all = format!("{nil_block}{empty_block}{real_block}");
        let blocks = parse_case_blocks(&all);
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks["a/nil_case"], vec!["<nil>".to_string()]);
        assert_eq!(blocks["a/empty_case"], vec!["<empty>".to_string()]);
        assert_eq!(
            blocks["a/real"],
            vec!["G1 X1".to_string(), "G1 X2".to_string()]
        );
    }

    /// harness 自检：行数不等先报行数；行数相等时报确切 1 基行号。
    #[test]
    fn line_compare_reports_line_numbers() {
        let expected = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let actual = vec!["a".to_string(), "X".to_string(), "c".to_string()];
        // downcast 必须在 Box 上调（借 &Box 去 downcast 会把 Box 当具体类型）
        let err = std::panic::catch_unwind(|| compare_line_wise("T", &expected, &actual))
            .expect_err("有差异时必须 panic");
        let msg = err
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| err.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_else(|| "<非字符串 panic>".to_string());
        assert!(msg.contains("line 2"), "报错须带 1 基行号: {msg}");
    }

    /// harness 自检：档位登记表与 vendored golden 的 23 块一致（脱钩即点名）。
    #[test]
    fn tier_registry_matches_vendored_golden() {
        let blocks = parse_case_blocks(&read_golden("tower_matrix.golden"));
        assert_eq!(blocks.len(), 23, "vendored golden 应有 23 个 CASE 块");
        // 每个名字都能查到档位（查不到 case_tier 自己会 panic 点名）
        for name in blocks.keys() {
            let _ = case_tier(name);
        }
    }
}
