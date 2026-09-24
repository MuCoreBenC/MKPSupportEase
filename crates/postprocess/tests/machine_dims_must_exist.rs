//! 「别名认识 ≠ 尺寸表里有」这条守卫的判据。
//!
//! ## 这条判据在守什么（实测事实，不是假想）
//!
//! 机型清单（`presets/machines/*.toml` 的 `id` 与 `externalAliases`）把 23 个别名
//! 映射到 **6** 个规范名（A1 / A1_MINI / A2L / P1S / P2S / X1C），而其中有 `[dimensions]`
//! 的只有 **5** 台 —— **没有 A2L**。而 `get_machine_dimensions` 未命中时返回 zero-value
//! （M017「禁止机型回退」的语义，要保留），于是修之前：
//!
//! ```text
//! mkpse-pp check -c … --set Machine.MachineType=A2L
//! → 退出码 0，「配置可用」，机型 A2L（X 0.0..0.0 / Y 0.0..0.0），禁区 0 处
//! ```
//!
//! `check` 对一台运动范围 0×0 的机器说「可用」，真跑 `run` 才在边界检查处失败，
//! 报错点离真因很远。修法是在 `pipeline::config_ir`（`run` / `check` / `dump-ir`
//! **共用**的第 2 步本体）里加第二道：归一之后要求尺寸表命中，未命中即硬错误。
//!
//! ## 判据的扫描面是**算出来的**，不是写死 A2L
//!
//! 下面的 `a_canonical_name_without_dimensions()` 从两张内置表现算差集。这样：
//! - 将来给 A2L 补了尺寸 ⇒ 差集变空 ⇒ 那个函数**响亮 panic** 并说明该怎么处置
//!   （不是静默通过 —— 「0 命中」和「判据没执行」在终端上长得一样，见 AGENTS §7③）；
//! - 将来 aliasMap 又多一个没尺寸的机型 ⇒ 判据自动覆盖它，不用改一行。

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::{Command, Output};

use postprocess::postproc::machine_dims::has_machine_dimensions;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_mkpse-pp")
}

fn config_a1() -> PathBuf {
    repo_root().join("tests/fixtures/config/A1.toml")
}

fn golden_input() -> PathBuf {
    repo_root().join("tests/golden/42274.2.gcode")
}

/// 规范名集合：从**我们自己的机型清单**（`presets/machines/*.toml`）读，不复述。
///
/// 这一条以前读的是内核自带的 `assets/machine_catalog_extra.json`。
/// M3 把唯一来源换成了 `presets/`，判据跟着换 —— 否则它盯的东西
/// 与产品实际用的东西不是同一份（那正是这次改动要消灭的失效模式）。
fn canonical_names() -> Vec<String> {
    let dir = repo_root().join("../../presets/machines");
    let entries = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("读不到 {}（{e}）—— 判据已空转", dir.display()));
    let mut out: Vec<String> = Vec::new();
    for e in entries.flatten() {
        let path = e.path();
        if path.extension().and_then(|s| s.to_str()) != Some("toml") {
            continue;
        }
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("读不到 {}（{e}）", path.display()));
        let value: toml::Value = toml::from_str(&raw)
            .unwrap_or_else(|e| panic!("{} 不是合法 TOML：{e}", path.display()));
        let id = value
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("{} 里没有 id", path.display()));
        out.push(id.to_string());
    }
    out.sort();
    out.dedup();
    // 反空转哨兵：实测 6 个规范名（含没有尺寸的 A2L）。
    assert!(
        out.len() >= 4,
        "机型清单只解析出 {} 个规范名（期望 ≥ 4）—— 目录读坏了，判据不成立",
        out.len()
    );
    out
}

/// 取一个「别名表认识、尺寸表没有」的规范名。实测就是 `A2L`。
fn a_canonical_name_without_dimensions() -> String {
    let missing: Vec<String> = canonical_names()
        .into_iter()
        .filter(|m| !has_machine_dimensions(m))
        .collect();
    match missing.first() {
        Some(m) => m.clone(),
        None => panic!(
            "两张内置表现在一致了（每个规范名都有尺寸条目）—— 这条判据失去了扫描面。\n\
             这是好事，但**不许让它静默通过**：请把本文件的 CLI 三条判据改成\n\
             指向一个构造出来的缺失机型（或删掉它们并在提交信息里写清理由）。"
        ),
    }
}

fn run_cli(args: &[&str]) -> Output {
    Command::new(bin())
        .args(args)
        .output()
        .expect("CLI 二进制必须能起来")
}

fn stderr_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

/// `check`：尺寸表没有这个机型 ⇒ 必须非 0 退出，且错误里点名那个机型。
#[test]
fn check_rejects_a_machine_that_has_no_dimensions() {
    let machine = a_canonical_name_without_dimensions();
    let out = run_cli(&[
        "check",
        "-c",
        &config_a1().to_string_lossy(),
        "--set",
        &format!("Machine.MachineType={machine}"),
    ]);
    assert_ne!(
        out.status.code(),
        Some(0),
        "check 对一台尺寸表里没有的机型说「可用」了；stderr:\n{}",
        stderr_of(&out)
    );
    let err = stderr_of(&out);
    assert!(
        err.contains(&machine),
        "错误信息没点名那个机型（用户没法知道该改什么）：\n{err}"
    );
    assert!(
        err.contains("尺寸表"),
        "错误信息没说清是「尺寸表里没有」（别名不认识是另一件事，两者不该混）：\n{err}"
    );
}

/// `dump-ir`：同一条路，也必须非 0 —— 否则它会打出一份 0×0 的 IR 当成正常输出。
#[test]
fn dump_ir_rejects_a_machine_that_has_no_dimensions() {
    let machine = a_canonical_name_without_dimensions();
    let out = run_cli(&[
        "dump-ir",
        "-c",
        &config_a1().to_string_lossy(),
        "--set",
        &format!("Machine.MachineType={machine}"),
    ]);
    assert_ne!(
        out.status.code(),
        Some(0),
        "dump-ir 打出了一份运动范围 0×0 的 IR 并退 0；stdout 前 200 字：\n{}",
        String::from_utf8_lossy(&out.stdout)
            .chars()
            .take(200)
            .collect::<String>()
    );
}

/// `run`：必须在**前置校验**就退 2（输入/配置错），而不是进管线之后才炸。
#[test]
fn run_rejects_it_at_preflight_with_exit_two() {
    let machine = a_canonical_name_without_dimensions();
    let input = std::env::temp_dir().join("mkpse-pp-nodims.gcode");
    std::fs::copy(golden_input(), &input).expect("复制输入失败");
    let out_path = std::env::temp_dir().join("mkpse-pp-nodims.out.gcode");
    let _ = std::fs::remove_file(&out_path);

    let out = run_cli(&[
        "run",
        &input.to_string_lossy(),
        "-c",
        &config_a1().to_string_lossy(),
        "-o",
        &out_path.to_string_lossy(),
        "--set",
        &format!("Machine.MachineType={machine}"),
    ]);
    assert_eq!(
        out.status.code(),
        Some(2),
        "期望前置校验拦下（退 2）；stderr:\n{}",
        stderr_of(&out)
    );
    assert!(
        !out_path.exists(),
        "被拦下了却仍然产生了输出文件：{}",
        out_path.display()
    );
}

/// 反空转：**正常机型必须仍然通过**。
///
/// 少了这条，上面三条在「把所有机型都拦掉」的实现下同样全绿 —— 那是最坏的修法。
#[test]
fn a_machine_that_has_dimensions_still_passes() {
    let out = run_cli(&["check", "-c", &config_a1().to_string_lossy()]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "A1（尺寸表里有）也被拦了 ⇒ 守卫写成了「一律拒绝」；stderr:\n{}",
        stderr_of(&out)
    );
    // 顺带钉住一个真实数字：范围不是 0×0。
    let err = stderr_of(&out);
    assert!(
        err.contains("260.0") && err.contains("255.0"),
        "check 的证据里没有 A1 的实际范围（X..260.0 / Y..255.0）：\n{err}"
    );
}

/// `has_machine_dimensions` 与 `get_machine_dimensions` **对同一个名字必须给同一个答案**。
///
/// 两个函数各写一遍查法（精确 → 大小写不敏感）就会漂移，而漂移的表现是
/// 「守卫说没有、填充却填上了」这种最难查的形态。这里用大小写变形逐个对一遍。
#[test]
fn the_two_lookups_agree() {
    let mut checked = 0usize;
    let mut agreements: BTreeMap<String, bool> = BTreeMap::new();
    for m in canonical_names() {
        for form in [m.clone(), m.to_lowercase(), m.to_uppercase()] {
            let has = has_machine_dimensions(&form);
            let dims = postprocess::postproc::machine_dims::get_machine_dimensions(&form);
            let non_zero = dims.movement_range.max_x != 0.0 || dims.movement_range.max_y != 0.0;
            assert_eq!(
                has, non_zero,
                "两条查法对 {form:?} 给了不同答案（has={has} / 填充出来非零={non_zero}）"
            );
            checked += 1;
        }
        agreements.insert(m.clone(), has_machine_dimensions(&m));
    }
    // 反空转：确实比过东西，且两种结论都出现过（全 true 或全 false 说明扫描面选坏了）。
    assert!(checked >= 12, "只比了 {checked} 次 —— 判据近乎空转");
    let yes = agreements.values().filter(|v| **v).count();
    let no = agreements.len() - yes;
    assert!(
        yes > 0 && no > 0,
        "内置两张表现在 {yes} 个有尺寸 / {no} 个没有 —— 需要两种情况都存在这条判据才有意义"
    );
}
