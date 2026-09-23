//! `preferred_mode` 的**来源**判据（spec tasks.md Task 8.4 推迟到 Task 10.3 的那条）。
//!
//! 背景：来源仓库 mkp-sr 的 engine 在 `crates/engine/src/lib.rs:242` 传给
//! `decide_wiping` 的是 `preset.config.wiping.have_wiping_components` —— **直接读预设**。
//! 本项目没有预设了，改从 `ir.wiping.preferred_mode` 取。这是一次**换源**，
//! 而换源如果错了，表现是「擦拭决策静默变了」：tower / disk / 强制 tower 三条分支，
//! 输出仍然是一份合法 G-code，退出码 0，没有任何报错。
//!
//! G1/G2 字节判据**盖不住这件事**（它们的 fixture 里 `preferred_mode` 只有一个取值，
//! 换成读另一个字段也可能照样全绿），所以这条判据必须独立存在。
//!
//! 三条互相独立的断言，各自能抓的形态不同：
//! 1. [`preferred_mode_is_a_verbatim_copy_of_have_wiping_components`] —— 恒等关系的**数据证据**
//! 2. [`the_field_really_drives_the_decision`] —— 反空转：这个字段真的会改变结果
//! 3. [`pipeline_decision_matches_independent_recomputation`] —— 管线真的用了它

use std::path::PathBuf;

use postprocess::config;
use postprocess::diag::CancelToken;
use postprocess::ir::Ir;
use postprocess::pipeline::{ProcessRequest, process};
use postprocess::postproc::support::{apply_wiping_decision, decide_wiping};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// 9 份预设 fixture 与它们对应的 `ir::build()` 产物。
const PRESET_NAMES: [&str; 9] = [
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

/// 从预设 TOML 里刮 `have_wiping_components` 的值。
///
/// **刻意不引 toml 解析器去读预设**：本项目的立场是「不读预设文件」，判据里也不破例；
/// 这里只做一次文本刮取，且刮不到就**响亮失败**（不是返回默认值继续）。
fn have_wiping_components(preset_text: &str, name: &str) -> String {
    for line in preset_text.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("have_wiping_components") else {
            continue;
        };
        let Some(rest) = rest.trim_start().strip_prefix('=') else {
            continue;
        };
        // 形如 `"tower" # 注释`
        let rest = rest.trim();
        let Some(rest) = rest.strip_prefix('"') else {
            panic!("{name}.toml 的 have_wiping_components 不是字符串字面量：{line}");
        };
        let Some(end) = rest.find('"') else {
            panic!("{name}.toml 的 have_wiping_components 引号没闭合：{line}");
        };
        return rest[..end].to_string();
    }
    panic!("{name}.toml 里找不到 have_wiping_components ⇒ fixture 漂移，判据不许静默通过");
}

/// 断言 1：`ir::build()` 对这个字段是**逐字复制**，没有 trim / 小写化 / 默认值兜底。
///
/// **诚实边界（必须写在判据旁边）**：9 份 fixture 的取值**全都是 `"tower"`**，
/// 也就是说这条断言证明的是「`tower` 这个值不会被改写」，**不能**证明
/// 「任意值都不会被改写」。真正的恒等证据是源码那一行
/// （`crates/ir/src/build.rs:74`，纯 `.clone()`），本断言是它的数据侧复核。
/// 想把证据等级提上去只有一个办法：拿到一份 `have_wiping_components = "disk"` 的真实预设。
#[test]
fn preferred_mode_is_a_verbatim_copy_of_have_wiping_components() {
    let mut seen_values = std::collections::BTreeSet::new();
    for name in PRESET_NAMES {
        let toml_path = repo_root().join(format!("tests/fixtures/presets/{name}.toml"));
        let json_path = repo_root().join(format!("tests/fixtures/ir/build9/{name}.json"));
        let toml_text = std::fs::read_to_string(&toml_path)
            .unwrap_or_else(|e| panic!("读不到 {}: {e}", toml_path.display()));
        let json_text = std::fs::read_to_string(&json_path)
            .unwrap_or_else(|e| panic!("读不到 {}: {e}", json_path.display()));
        let ir: Ir = serde_json::from_str(&json_text).expect("build9 的 IR JSON 必须能反序列化");

        let from_toml = have_wiping_components(&toml_text, name);
        assert_eq!(
            ir.wiping.preferred_mode, from_toml,
            "{name}: ir.wiping.preferred_mode 与预设的 have_wiping_components 不一致 \
             ⇒ 中间有归一化，从 IR 取这个值是不安全的"
        );
        seen_values.insert(from_toml);
    }
    // 反空转：至少刮到过一个非空值（不是 9 次都在比 "" == ""）
    assert!(
        seen_values.iter().any(|v| !v.is_empty()),
        "9 份预设刮出来的值全是空串 ⇒ 刮取逻辑坏了，这条判据在空转"
    );
    // 把证据面如实打印出来：只有一个取值就是只有一个取值
    println!("9 份预设覆盖到的 have_wiping_components 取值：{seen_values:?}");
}

/// 断言 2（反空转）：这个字段**真的**会改变结果。
///
/// 如果它不改变任何东西，断言 1 与断言 3 都可以在读错字段的情况下照样绿。
#[test]
fn the_field_really_drives_the_decision() {
    // 有支撑体 ⇒ needed=true 且不是 support_fallback ⇒ 走 preferred_mode 分支
    let content: Vec<String> = [
        "; FEATURE: Support interface",
        "G1 X1 Y1",
        "; FEATURE: Support body",
        "G1 X2 Y2",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    let mut as_tower = Ir::default();
    apply_wiping_decision(&mut as_tower, &decide_wiping("tower", &content));
    let mut as_disk = Ir::default();
    apply_wiping_decision(&mut as_disk, &decide_wiping("disk", &content));

    assert_eq!(as_tower.wiping.mode, "tower");
    assert!(as_tower.tower.use_towers);
    assert_eq!(as_disk.wiping.mode, "disk");
    assert!(!as_disk.tower.use_towers);
    assert_ne!(
        as_tower.wiping.mode, as_disk.wiping.mode,
        "preferred_mode 不影响决策 ⇒ 本文件另两条判据都会变成空转"
    );
}

/// 断言 3：管线跑完之后的擦拭决策，等于**从配置文件与输入独立重算**的结果。
///
/// 这条不预设哪条分支会触发（golden 输入实际走哪条由数据决定），只钉住
/// 「管线用的 preferred 来源 == 配置里那个字段」。若有人把第 5 步改成读别的字段
/// （或改回读一个不存在的预设结构），左右两边会立刻不同。
#[test]
fn pipeline_decision_matches_independent_recomputation() {
    let config_path = repo_root().join("tests/fixtures/config/A1.toml");
    let input = repo_root().join("tests/golden/42274.2.gcode");
    let out_path = std::env::temp_dir().join("mkpse-pp-wiping-source.gcode");

    // 独立重算：直接从配置文件读 preferred，再对同一份输入跑 decide + apply。
    let cfg_ir = config::load(&config_path).expect("配置 fixture 必须能读");
    let raw = std::fs::read_to_string(&input).expect("golden 输入必须在");
    let content: Vec<String> = raw
        .lines()
        .map(|l| l.trim_end_matches('\r').to_string())
        .collect();
    let mut expected = cfg_ir.clone();
    let decision = decide_wiping(&cfg_ir.wiping.preferred_mode, &content);
    let applied = apply_wiping_decision(&mut expected, &decision);

    let result = process(
        ProcessRequest {
            gcode_path: input,
            config_path,
            overrides: Vec::new(),
            output_path: Some(out_path.clone()),
        },
        &mut |_| {},
        &CancelToken::new(),
    )
    .expect("全链应成功");
    let _ = std::fs::remove_file(&out_path);

    assert_eq!(
        result.detail_facts.ir.wiping.needed, expected.wiping.needed,
        "wiping.needed 与独立重算不一致"
    );
    assert_eq!(
        result.detail_facts.ir.wiping.mode, expected.wiping.mode,
        "wiping.mode 与独立重算不一致 ⇒ 管线的 preferred 来源不是配置里那个字段"
    );
    assert_eq!(
        result.detail_facts.support_fallback, applied.support_fallback,
        "support_fallback 与独立重算不一致"
    );
    assert_eq!(
        result.detail_facts.fallback_objects, applied.fallback_objects,
        "fallback_objects 与独立重算不一致"
    );
    // 反空转：这份输入确实需要擦拭（否则 mode 恒为 "none"，比较毫无信息量）
    assert!(
        result.detail_facts.ir.wiping.needed,
        "golden 输入被判为「不需要擦拭」⇒ 这条判据在比较两个 \"none\"，换输入或换判法"
    );
    println!(
        "实测：needed={} mode={} support_fallback={}",
        result.detail_facts.ir.wiping.needed,
        result.detail_facts.ir.wiping.mode,
        result.detail_facts.support_fallback
    );
}
