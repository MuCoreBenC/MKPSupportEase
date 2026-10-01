//! 区间真源的判据（K-R1..K-R7）。
//!
//! 这一轮的立场：**参数区间只有一处真源 = `param_registry.toml`**。
//! 代码里不再写死区间数值；注册表里「按机型/变体的覆盖」也不再被静默丢弃。
//!
//! 判据分两类：
//! - **回归**（K-R1）：14 份预设照旧能读、钩子产物逐字节相同 —— 换真源不许改行为；
//! - **新能力**（K-R2/K-R3）：按机型的覆盖真的生效，且「是不是覆盖来的」可见。
//!
//! K-R4 是**元判据**：盯着「注册表里有、`ParamEntry` 却不认识」的键。
//! 这一轮就是被这类静默丢弃坑过（`machineMinVariants` / `machineMaxVariants` 白躺着没人读），
//! 所以它必须有个数字，且写进快照。

use std::collections::BTreeMap;

use preset::registry::{RangeSource, load_param_registry};

/// `ParamEntry` 认识的 serde 键名。**改结构就要改这里** —— K-R4 就是靠这份清单发现差集的。
const KNOWN_PARAM_KEYS: &[&str] = &[
    "key",
    "configKey",
    "defaultValue",
    "min",
    "max",
    "deprecated",
    "choices",
    "label",
    "desc",
    "unit",
    "tomlKey",
    "valueType",
    "uiComponent",
    "step",
    "section",
    "scope",
    "variantMode",
    "machineFilter",
    "pinned",
    "tomlComment",
    "selectable",
    "mergeable",
    "layout",
    "machineVariants",
    "machineMinVariants",
    "machineMaxVariants",
];

/// 注册表里出现过、但 `ParamEntry` 不认识的键 → 出现次数。
/// **快照**：今天是这 5 个。多出新键说明云端加了东西，得有人看一眼是不是该读。
const UNKNOWN_SNAPSHOT: &[(&str, usize)] = &[
    ("jsonKey", 74),
    ("mergeGroup", 5),
    ("parentKey", 43),
    ("serialization", 1),
    ("showWhen", 43),
];

#[test]
fn kr2_machine_variant_override_is_used_and_visible() {
    let reg = load_param_registry();

    // **这一轮真正的行为变化就在这条**：`wiping.wiper_x` 的全局上限是 226（A1 全尺寸的数），
    // 而 A1_MINI 的覆盖是 **150**（mini 的床只有 180）。今天程序读不到覆盖表，
    // 所以 A1_MINI 上 150..226 这一段本该被拒的值是被放过的。
    let mini = reg
        .lookup_range_for("WiperX", "A1_MINI", Some("fastv3.3"))
        .expect("WiperX 该有区间");
    assert_eq!(
        (mini.min, mini.max),
        (Some(10.0), Some(150.0)),
        "K-R2 红：A1_MINI:FASTV3.3 的 WiperX 上限该是 150（覆盖），不是 226（全局）"
    );
    assert_eq!(
        mini.source,
        RangeSource::MachineVariant,
        "K-R2 红：来源必须标成「按机型」，否则界面上分不出是不是回落"
    );

    // 覆盖存在但**与全局同值**的那一类（P1S:LITE 的 WiperX 是 10..226）：
    // 值不变，但来源仍标 MachineVariant —— 「这条被机型影响过」是事实，不该抹掉。
    let p1s = reg.lookup_range_for("WiperX", "P1S", Some("lite")).unwrap();
    assert_eq!((p1s.min, p1s.max), (Some(10.0), Some(226.0)));
    assert_eq!(p1s.source, RangeSource::MachineVariant);

    // 覆盖表**有洞**：`wiper_x` 有 `A1:FAST` / `A1:STANDARD`，但没有 `A1:FASTV3.3`
    // ⇒ 回落全局。这不是 bug，是资产现状，判据把它钉住。
    let a1_v33 = reg
        .lookup_range_for("WiperX", "A1", Some("fastv3.3"))
        .unwrap();
    assert_eq!((a1_v33.min, a1_v33.max), (Some(10.0), Some(226.0)));
    assert_eq!(
        a1_v33.source,
        RangeSource::Global,
        "K-R2 红：A1:FASTV3.3 在覆盖表里没有，应当回落全局并**如实标成 Global**"
    );

    // 老入口一个字没动（build.rs 还在用）：它永远给全局值
    assert_eq!(reg.lookup_range("WiperX"), Some((10.0, 226.0)));

    // 没有任何区间的参数 ⇒ None（编辑器据此说「没有量程作证」）
    assert!(
        reg.lookup_range_for("Slicer", "A1_MINI", None).is_none(),
        "K-R2 红：没有 min/max 的参数不该编出一个区间"
    );

    println!(
        "K-R2 绿：WiperX @A1_MINI = [10, 150]（覆盖，比全局严）/ @P1S:LITE = [10, 226]（覆盖同值）\
         / @A1:FASTV3.3 = [10, 226]（覆盖表有洞 ⇒ 回落，来源如实标 Global）"
    );
}

#[test]
fn kr2b_which_overrides_actually_differ_from_global() {
    // **这条是这一轮的「行为差量清单」**：覆盖表里绝大多数值与全局逐值相同（纯冗余），
    // 只有少数几条真的不同 —— 那几条就是接上覆盖表之后**校验会变严**的地方。
    // 收口报告要照抄这份清单，别让「换了真源」听起来像没有后果。
    let reg = load_param_registry();
    let mut differs: Vec<String> = Vec::new();
    for p in &reg.params {
        for (key, v) in &p.machine_min_variants {
            if p.min != Some(*v) {
                differs.push(format!("{} min@{key} {:?}→{v}", p.param_key, p.min));
            }
        }
        for (key, v) in &p.machine_max_variants {
            if p.max != Some(*v) {
                differs.push(format!("{} max@{key} {:?}→{v}", p.param_key, p.max));
            }
        }
    }
    differs.sort();
    assert_eq!(
        differs,
        vec![
            "wiping.wiper_x max@A1_MINI:FAST Some(226.0)→150".to_string(),
            "wiping.wiper_x max@A1_MINI:FASTV3.3 Some(226.0)→150".to_string(),
            "wiping.wiper_x max@A1_MINI:STANDARD Some(226.0)→150".to_string(),
            "wiping.wiper_y max@A1_MINI:FAST Some(226.0)→150".to_string(),
            "wiping.wiper_y max@A1_MINI:FASTV3.3 Some(226.0)→150".to_string(),
            "wiping.wiper_y max@A1_MINI:STANDARD Some(226.0)→150".to_string(),
        ],
        "K-R2b 红：覆盖与全局的差量清单变了 —— 这直接改变校验严格程度，必须有人看一眼"
    );
    println!("K-R2b 绿：真正与全局不同的覆盖只有 6 条，全是 A1_MINI 的 wiper 上限 226 → 150");
}

#[test]
fn kr3_variant_lookup_is_case_insensitive() {
    let reg = load_param_registry();

    let lower = reg.lookup_range_for("WiperX", "P1S", Some("lite")).unwrap();
    let upper = reg.lookup_range_for("WiperX", "P1S", Some("LITE")).unwrap();
    let mixed = reg.lookup_range_for("WiperX", "P1S", Some("Lite")).unwrap();
    assert_eq!(lower, upper, "K-R3 红：大小写不该改变结果");
    assert_eq!(lower, mixed);

    // 注册表键是大写（`P1S:LITE`），而预设文件头写的是小写（`# variant: fastv3.3`）——
    // 这条断言就是那个转换规则本身。
    let none = reg.lookup_range_for("WiperX", "P1S", None).unwrap();
    assert_eq!(
        none.source,
        RangeSource::Global,
        "K-R3 红：没有变体时必须回落全局（老预设可能没有 # variant: 那一行）"
    );

    // 机型没归一（小写）也只是回落，不报错 —— 归一是调用方的事
    let bogus = reg
        .lookup_range_for("WiperX", "a1_mini", Some("fastv3.3"))
        .unwrap();
    assert_eq!(
        (bogus.min, bogus.max, bogus.source),
        (Some(10.0), Some(226.0), RangeSource::Global),
        "K-R3 红：机型大小写不匹配时应当回落（规范名归一在 load_ir 里做）"
    );

    println!("K-R3 绿：变体大小写不敏感；无变体/机型未归一都回落全局");
}

#[test]
fn kr1_all_presets_still_load_after_switching_the_source_of_truth() {
    use std::path::PathBuf;

    // 9 份 fixture（仓库内，必跑）
    let dir = preset::generate::fixtures_dir();
    let mut targets: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("fixtures 目录应存在")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    targets.sort();
    assert!(targets.len() >= 9, "反空转：fixture 至少 9 份");
    let fixtures = targets.len();

    // 5 份**真实用户预设**：只在这台机器上存在 ⇒ 有就跑、没有就跳过并报数。
    // 判据不能依赖用户目录（别人机器上会假红），但在这里跑到了就得说清跑了几份。
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    let real_dir = home.join("Documents/MKPSupportSSR/presets/mkp");
    let mut real = 0;
    if real_dir.is_dir() {
        for entry in std::fs::read_dir(&real_dir).expect("读真实预设目录") {
            let p = entry.expect("目录项").path();
            if p.extension().is_some_and(|x| x == "toml") {
                targets.push(p);
                real += 1;
            }
        }
    }

    for p in &targets {
        preset::load_ir(p, None)
            .unwrap_or_else(|e| panic!("K-R1 红：{} 读不出来了：{e}", p.display()));
    }

    println!(
        "K-R1 绿：{} 份预设换真源后照旧 load_ir 成功（fixture {fixtures} + 真实用户预设 {real}）",
        targets.len()
    );
}

#[test]
fn kr7_variant_header_is_read_and_missing_is_not_an_error() {
    // 真实预设（fixture）：A1_MINI 的 fast 那份，文件头有 `# variant: fast`
    let dir = preset::generate::fixtures_dir();
    let raw = std::fs::read_to_string(dir.join("A1_MINI-fast.toml")).expect("读 fixture");
    let file = preset::read_preset_from_bytes(raw.clone()).expect("读预设");
    assert_eq!(
        file.variant.as_deref(),
        Some("fast"),
        "K-R7 红：`# variant:` 没被读出来"
    );
    assert_eq!(file.machine, "A1_MINI", "回归：machine 照旧");

    // 去掉那一行 ⇒ None，且**仍然读得成功**（老预设可能没有这一行）
    let without: String = raw
        .lines()
        .filter(|l| !l.trim_start().starts_with("# variant"))
        .map(|l| format!("{l}\n"))
        .collect();
    assert_ne!(raw, without, "构造失败：没找到 # variant 那一行");
    let file2 = preset::read_preset_from_bytes(without).expect("缺 variant 也要读得成功");
    assert_eq!(file2.variant, None);

    // `# machine:` 缺失照旧是硬错误（回归，不许被顺手放宽）
    let no_machine: String = raw
        .lines()
        .filter(|l| !l.trim_start().starts_with("# machine"))
        .map(|l| format!("{l}\n"))
        .collect();
    let err = preset::read_preset_from_bytes(no_machine).expect_err("缺 machine 必须报错");
    assert_eq!(err.code(), "E_CFG_PARSE_001", "K-R7 红：错误码变了");

    // 手写匹配器的边界：`# variant_x:` 不许命中 `variant`
    assert_eq!(
        preset::parse_variant_from_content("# variant_x: v\n"),
        None,
        "K-R7 红：键名前缀被误命中（`variant_x` 不是 `variant`）"
    );
    // 空值视同未命中（与 release_time 同规则）
    assert_eq!(preset::parse_variant_from_content("# variant:   \n"), None);
    // 原样返回，不改大小写（归一在查表那一侧做）
    assert_eq!(
        preset::parse_variant_from_content("#  variant :  FastV3.3  \n").as_deref(),
        Some("FastV3.3")
    );

    println!(
        "K-R7 绿：`# variant:` 读到 `fast`；缺失 ⇒ None 且不报错；缺 machine 照旧 E_CFG_PARSE_001"
    );
}

#[test]
fn kr4_unknown_registry_keys_stay_on_the_snapshot() {
    // 直接读快照文本：`ParamEntry` 认不认识是这条判据要查的事，不能用它自己去解析。
    let raw: toml::Value = toml::from_str(preset::PARAM_REGISTRY_TOML).expect("快照是合法 TOML");
    let params = raw
        .get("params")
        .and_then(|v| v.as_array())
        .expect("params 是数组");
    assert_eq!(params.len(), 74, "反空转：注册表应有 74 条参数");

    let mut unknown: BTreeMap<String, usize> = BTreeMap::new();
    for p in params {
        for (k, _) in p.as_table().expect("每条参数是表") {
            if !KNOWN_PARAM_KEYS.contains(&k.as_str()) {
                *unknown.entry(k.clone()).or_default() += 1;
            }
        }
    }

    let want: BTreeMap<String, usize> = UNKNOWN_SNAPSHOT
        .iter()
        .map(|(k, n)| ((*k).to_string(), *n))
        .collect();
    assert_eq!(
        unknown, want,
        "K-R4 红：注册表里「程序不认识的键」变了。\
         多出来的键要么该读（像这一轮的 machineMin/MaxVariants 那样），\
         要么确认无用后更新 UNKNOWN_SNAPSHOT —— 不许静默丢弃"
    );

    // 这一轮认领的那两个字段必须**不在**未知集合里
    assert!(
        !unknown.contains_key("machineMinVariants") && !unknown.contains_key("machineMaxVariants"),
        "K-R4 红：按机型的区间覆盖又变成静默丢弃了"
    );

    println!("K-R4 绿：74 条参数，未知键 {unknown:?}（与快照一致）");
}

#[test]
fn kr4b_the_override_tables_are_exactly_where_we_measured_them() {
    // 实测只有 5 组覆盖。这条钉住「有几组、挂在哪」——
    // 将来云端给别的参数加了覆盖，这里会红，而那正是我们想知道的事。
    let reg = load_param_registry();
    let with_override: Vec<&str> = reg
        .params
        .iter()
        .filter(|p| !p.machine_min_variants.is_empty() || !p.machine_max_variants.is_empty())
        .map(|p| p.param_key.as_str())
        .collect();
    assert_eq!(
        with_override,
        vec![
            "toolhead.offset.x",
            "toolhead.offset.y",
            "toolhead.offset.z",
            "wiping.wiper_x",
            "wiping.wiper_y",
        ],
        "K-R4b 红：按机型覆盖的参数集合变了"
    );
    println!("K-R4b 绿：5 组覆盖，挂在 {with_override:?}");
}
