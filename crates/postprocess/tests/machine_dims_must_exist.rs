//! 「别名认识 ≠ 尺寸表里有」这条守卫的判据。
//!
//! ## 这条判据在守什么（实测事实，不是假想）
//!
//! 两张表由 `machine_dims::load_presets_dir` 从**同一批清单文件**装出来
//! （`<presets>/machines/*.toml`），但收录条件不同：`id` 与 `externalAliases`
//! 无条件进 `alias_map`，而尺寸要 `[dimensions]` 那一段才进 `dimensions`。
//! 于是「别名认识、尺寸表没有」这个差集**可能非空**，而 `get_machine_dimensions`
//! 未命中时返回 zero-value（M017「禁止机型回退」的语义，要保留），于是修之前：
//!
//! ```text
//! mkpse-pp check -c … --set Machine.MachineType=<缺尺寸的那台>
//! → 退出码 0，「配置可用」，机型 X 0.0..0.0 / Y 0.0..0.0，禁区 0 处
//! ```
//!
//! `check` 对一台运动范围 0×0 的机器说「可用」，真跑 `run` 才在边界检查处失败，
//! 报错点离真因很远。修法是在 `pipeline::config_ir`（`run` / `check` / `dump-ir`
//! **共用**的第 2 步本体）里加第二道：归一之后要求尺寸表命中，未命中即硬错误。
//!
//! ## 前提由 **fixture** 自带，不借生产预设
//!
//! 要跑起来得先有一台「缺尺寸」的机型。上一版把这件事寄托在生产数据上（当时
//! `presets/machines/A2L.toml` 是唯一没有 `[dimensions]` 的那台），于是 A2L 一删，
//! 判据的反空转前提跟着没了 —— **生产数据不该为测试的需要背这个包袱**。两份清单
//! 现在躺在 `tests/fixtures/presets_dims/machines/`：
//!
//! - `A1.toml`    —— 有尺寸（正面对照，也供 `check` 走完整条管线）
//! - `NODIM.toml` —— 没有 `[dimensions]`（守卫要拦的那一台）
//!
//! 扫描面仍是**算出来的**（`canonical_names()` 与 `has_machine_dimensions()` 的差集），
//! 不是写死 `NODIM`：fixture 里再添一台没尺寸的，判据自动覆盖它。差集为空时那个
//! 函数**响亮 panic** 并说明该怎么处置（不是静默通过 —— 「0 命中」和「判据没执行」
//! 在终端上长得一样，见 AGENTS §7③）。

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::Once;

use postprocess::postproc::machine_dims::{has_machine_dimensions, install, load_presets_dir};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// 测试自带的那份机型表根目录（形状与 `presets/` 同：`machines/` 一层的清单）。
fn fixture_presets_dir() -> PathBuf {
    repo_root().join("tests/fixtures/presets_dims")
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

/// 把 fixture 那张机型表装进**本进程** —— 只有装过，`has_machine_dimensions` /
/// `get_machine_dimensions` 才会给出 fixture 的答案（否则它们会去读 `presets/`，
/// 于是「有没有尺寸」的差集变成生产数据的事，判据又回到了老路上）。
///
/// 子进程那侧不走 `install`，走 `MKPSE_PRESETS_DIR`（见 `run_cli`）—— 那是 pipeline
/// 自己的逃生链口子。
///
/// **只装一次**：`install` 第二次会返回 Err（那是它的语义，防的是"两处数据看谁先跑"）。
fn install_fixture_tables() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let tables =
            load_presets_dir(&fixture_presets_dir()).expect("fixture 机型表读不出来 —— 判据已空转");
        install(tables).expect("机型表只能装一次");
    });
}

/// 规范名集合：从**测试自带的机型清单**读，不复述。
///
/// 顺带把表装进本进程 —— 凡是算差集的调用方都要先有表，所以在这里收一次口，
/// 免得每个用例各记一遍。
fn canonical_names() -> Vec<String> {
    install_fixture_tables();
    let dir = fixture_presets_dir().join("machines");
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
    // 反空转哨兵：fixture 实测 2 个规范名（A1 有尺寸 / NODIM 没有）。
    assert!(
        out.len() >= 2,
        "fixture 机型清单只解析出 {} 个规范名（期望 ≥ 2）—— 目录读坏了，判据不成立",
        out.len()
    );
    out
}

/// 取一台「别名表认识、尺寸表没有」的机型。实测就是 `NODIM`。
fn a_canonical_name_without_dimensions() -> String {
    let missing: Vec<String> = canonical_names()
        .into_iter()
        .filter(|m| !has_machine_dimensions(m))
        .collect();
    match missing.first() {
        Some(m) => m.clone(),
        None => panic!(
            "fixture 里每个规范名都有尺寸条目了 —— 这条判据失去了扫描面。\n\
             这是好事，但**不许让它静默通过**：请在 \
             `tests/fixtures/presets_dims/machines/` 里补一份没有 `[dimensions]` 的清单\n\
             （或删掉本文件这几条判据并在提交信息里写清理由）。"
        ),
    }
}

/// 起 CLI 时**把机型表指向 fixture**：pipeline 的逃生链口子是 `MKPSE_PRESETS_DIR`，
/// 不设它就会去读 `presets/`（那正是这次要甩掉的依赖）。
fn run_cli(args: &[&str]) -> Output {
    Command::new(bin())
        .env("MKPSE_PRESETS_DIR", fixture_presets_dir())
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
    // 反空转：确实比过东西（fixture 2 台 × 3 种写法），且两种结论都出现过。
    assert!(checked >= 6, "只比了 {checked} 次 —— 判据近乎空转");
    let yes = agreements.values().filter(|v| **v).count();
    let no = agreements.len() - yes;
    assert!(
        yes > 0 && no > 0,
        "fixture 里现在 {yes} 个有尺寸 / {no} 个没有 —— 需要两种情况都存在这条判据才有意义"
    );
}
