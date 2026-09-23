//! 整链判据：**同一份输入 G-code，新项目的输出与来源仓库 `mkp-sr` 的输出逐字节相同**。
//!
//! 这是「`config.rs` + `pipeline.rs` 没搬错」的唯一整链证据。其余判据都只覆盖一段：
//! `tests/postproc_*.rs` 覆盖 pass1/pass2 的字节，`tests/config.rs` 覆盖配置解析，
//! `tests/pipeline.rs` 覆盖步骤顺序 —— 都不能证明「配置层 → 编排层 → 输出」这条整链等价。
//!
//! ## 参考物怎么来的（可复现，不是手搓）
//!
//! 参考输出 `tests/golden/42274.2.e2e-A1.reference.gcode`（668687 字节）由来源仓库
//! 自己的 CLI 生成，命令原样记录如下（在 `/Users/wzy/projects/mkp-rust/mkp-sr` 下执行）：
//!
//! ```text
//! cargo run -q -p mkp-cli --bin mkp-sr -- \
//!   tests/golden/42274.2.gcode \
//!   --preset tests/fixtures/presets/A1.toml \
//!   --out /tmp/ref-mkp-sr-A1.gcode
//! ```
//!
//! 本侧用的配置 `tests/fixtures/config/A1.toml` 也是**派生**的：由
//! `mkp-sr` 的 `ir::build()` 对同一份 A1 预设产出的 IR JSON
//! （`tests/fixtures/ir/build9/A1.json`）经 `config::to_toml` 转成 TOML，
//! 唯一的人工改动是 `Machine.MachineType = "A1"`（预设里的机型声明在 IR 之外，
//! 见 `tests/pipeline.rs` 的派生说明与每轮复查）。
//!
//! 所以这条判据实际验证的链条是：
//! **预设 →（来源仓库）IR → TOML 配置 →（本项目）IR → 12 步 → 输出字节**。
//! 中间那次 JSON→TOML 转换顺带真实验证了 PascalCase 键在 TOML 里可用。
//!
//! ## 证据等级（必须先说清）
//!
//! 它对照的是**当前 `mkp-sr` 的输出**，不是「正确的输出」。`mkp-sr` 自身的偏差会被
//! 一并复制成绿色（与 AGENTS §7③ 同款：一致 ≠ 正确）。这条判据能守住「搬迁没改变行为」，
//! **守不住「行为本来就对」**。
//!
//! 另外三件事这条判据管不到：
//! - 只跑了 A1 这一个机型 / 一份输入（42274.2.gcode，34 个有效层）；
//! - `printtime` 只进 stderr（两侧实测都是 `1478.3s`），不进字节，所以字节相同不代表估算相同；
//! - 参考物是这一刻的 `mkp-sr`；那边将来改了行为，这里会红，**该红**。
//!
//! ## 2026-09-12：判据收窄为「除 MKP 标记行外逐字节相同」
//!
//! 本仓开始按 MKP 标记协议 v1 往产物里插 `;MKP_BEGIN` / `;MKP_END` / `;MKP_INFO` /
//! `;MKP_STATS`，参考物里没有这些行。参考物由**来源仓库的 CLI** 生成、在本仓无法复现，
//! 直接拿本侧输出覆盖它等于把判据变成自比自（彻底空转）。所以改成比对前剔除标记行：
//! 判据由「逐字节相同」变成「**除新增标记行外**逐字节相同」——
//! 这比原判据更强，因为它同时证明了标记是纯增量，没有顺带移动或改写任何一行原有输出。

use std::path::PathBuf;
use std::process::{Command, Output};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_mkpse-pp")
}

fn golden_input() -> PathBuf {
    repo_root().join("tests/golden/42274.2.gcode")
}

fn reference_output() -> PathBuf {
    repo_root().join("tests/golden/42274.2.e2e-A1.reference.gcode")
}

fn config_a1() -> PathBuf {
    repo_root().join("tests/fixtures/config/A1.toml")
}

fn run_cli(args: &[&str]) -> Output {
    Command::new(bin())
        .args(args)
        .output()
        .expect("CLI 二进制必须能起来")
}

/// 跑一次 `run`，返回输出文件的字节。
fn run_and_read(tag: &str, extra: &[&str]) -> Vec<u8> {
    let out_path = std::env::temp_dir().join(format!("mkpse-pp-e2e-{tag}.gcode"));
    let _ = std::fs::remove_file(&out_path);
    let input = golden_input();
    let config = config_a1();
    let mut args: Vec<String> = vec![
        "run".into(),
        input.to_string_lossy().into_owned(),
        "-c".into(),
        config.to_string_lossy().into_owned(),
        "-o".into(),
        out_path.to_string_lossy().into_owned(),
    ];
    args.extend(extra.iter().map(|s| (*s).to_string()));
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let out = run_cli(&refs);
    assert_eq!(
        out.status.code(),
        Some(0),
        "run 应当成功；stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    std::fs::read(&out_path).expect("输出文件应当存在")
}

/// 剔除 MKP 标记协议 v1 的标记行 —— 判据侧只有
/// `postprocess::postproc::marks::strip_mark_lines` 这一份实现（全仓 8 处比对共用它；
/// 分成八份实现就等于八个可能各自跑偏的口径）。
fn strip_mkp_marks(bytes: &[u8]) -> Vec<u8> {
    postprocess::postproc::marks::strip_mark_lines(bytes)
}

/// 整链等价：新项目的输出与 `mkp-sr` 的参考输出**除 MKP 标记行外逐字节相同**。
#[test]
fn output_is_byte_identical_to_the_mkp_sr_reference() {
    let got = strip_mkp_marks(&run_and_read("same", &[]));
    let want = std::fs::read(reference_output()).expect("参考输出 fixture 应当存在");

    // 先报长度，再逐字节定位——直接 assert_eq! 两个 668KB 的 Vec 会刷屏。
    assert_eq!(
        got.len(),
        want.len(),
        "剔除 MKP 标记行后输出长度与参考不等（本侧 {} 字节 / 参考 {} 字节）",
        got.len(),
        want.len()
    );
    if let Some(i) = (0..got.len()).find(|&i| got[i] != want[i]) {
        let from = i.saturating_sub(80);
        panic!(
            "第 {i} 字节起与参考不同：\n本侧: {:?}\n参考: {:?}",
            String::from_utf8_lossy(&got[from..(i + 80).min(got.len())]),
            String::from_utf8_lossy(&want[from..(i + 80).min(want.len())]),
        );
    }
}

/// 反空转 ①：参考输出不是输入的副本 —— 否则上一条判据在「管线什么都不做」时也会绿。
#[test]
fn the_reference_is_not_just_a_copy_of_the_input() {
    let input = std::fs::read(golden_input()).expect("输入 fixture 应当存在");
    let want = std::fs::read(reference_output()).expect("参考输出 fixture 应当存在");
    assert!(input != want, "参考输出与输入相同 ⇒ 整链判据是空的");
    assert!(
        want.len() > input.len(),
        "参考输出（{} 字节）没有比输入（{} 字节）长：后处理是往里加涂胶/塔/擦拭动作的，\
         不该变短",
        want.len(),
        input.len()
    );
}

/// 反空转 ②：字节比对**对配置敏感**。
///
/// 改一个真实旋钮（涂胶遍数 1 → 2）后输出必须与参考不同；否则说明配置根本没走到管线里，
/// 上一条判据就只是在比较「两个固定文件」。
#[test]
fn the_byte_comparison_is_sensitive_to_the_config() {
    let baseline = strip_mkp_marks(&run_and_read("baseline", &[]));
    let perturbed = strip_mkp_marks(&run_and_read("perturbed", &["--set", "Glue.PassCount=2"]));
    let want = std::fs::read(reference_output()).expect("参考输出 fixture 应当存在");

    // 用 assert!(a == b) 而不是 assert_eq!：后者失败时会把两个 668KB 的 Vec 原样打出来
    // （实测 5.2MB 输出，真正的原因反而被埋了）。
    assert!(baseline == want, "基线跑法应当与参考一致（前置条件）");
    assert!(
        perturbed != want,
        "把 Glue.PassCount 改成 2 之后输出仍与参考一致 ⇒ 配置没进管线，整链判据是空的"
    );
}
