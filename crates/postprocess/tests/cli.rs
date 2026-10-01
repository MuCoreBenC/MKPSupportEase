//! CLI 面的判据：**真的把二进制跑起来**，不是调库函数。
//!
//! 为什么必须真跑：退出码、stdout/stderr 分流、clap 的参数面这三件事在库里不存在，
//! 它们只在进程边界上有意义。`env!("CARGO_BIN_EXE_mkpse-pp")` 由 Cargo 提供，
//! 指向本次构建出来的那个二进制（不会拿到 PATH 上装的旧版本）。
//!
//! **未覆盖的一条，先说清**：退出码 `130`（Ctrl-C）需要给子进程发真 SIGINT
//! 并赶在它跑完之前，这里**没有判据**（写成 sleep + kill 会变成机器速度依赖的假红／假绿）。
//! 取消语义本身在 `tests/cancel.rs` 里以库层面 5 条判据覆盖；缺的只是
//! 「SIGINT → CancelToken 这根线接上了没有」，登记为诚实边界。

// 测试写临时文件、造夹具是正当的：写盘纪律管的是**生产代码**
// （与源码扫描断言只看 `#[cfg(test)]` 之前那部分同一口径）。
#![allow(clippy::disallowed_methods)]

use std::path::{Path, PathBuf};
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

fn config_a1() -> PathBuf {
    repo_root().join("tests/fixtures/config/A1.toml")
}

/// 跑一次 CLI。**不经管道**取退出码（`Output.status` 就是被测进程自己的）。
fn run_cli(args: &[&str]) -> Output {
    Command::new(bin())
        .args(args)
        .output()
        .expect("CLI 二进制必须能起来")
}

fn code_of(out: &Output) -> i32 {
    out.status.code().expect("进程不该被信号杀死")
}

fn stdout_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn stderr_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

fn temp_copy(tag: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("mkpse-pp-cli-{tag}.gcode"));
    std::fs::copy(golden_input(), &path).expect("复制输入失败");
    path
}

fn part_of(p: &Path) -> PathBuf {
    let mut s = p.as_os_str().to_os_string();
    s.push(".part");
    PathBuf::from(s)
}

/// 成功路径：退出 0，**stdout 只有结果路径这一行**，进度在 stderr。
///
/// 「stdout 只有一行」这条是给脚本用的契约：`out=$(mkpse-pp run …)` 必须直接是路径。
#[test]
fn run_succeeds_and_stdout_is_only_the_output_path() {
    let input = temp_copy("ok");
    let out_path = std::env::temp_dir().join("mkpse-pp-cli-ok.out.gcode");
    let _ = std::fs::remove_file(&out_path);

    let out = run_cli(&[
        "run",
        input.to_str().unwrap(),
        "-c",
        config_a1().to_str().unwrap(),
        "-o",
        out_path.to_str().unwrap(),
    ]);
    assert_eq!(code_of(&out), 0, "stderr:\n{}", stderr_of(&out));

    let stdout = stdout_of(&out);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines.len(),
        1,
        "stdout 必须只有结果路径一行，实测 {lines:?}"
    );
    assert_eq!(lines[0], out_path.display().to_string());
    assert!(out_path.is_file(), "输出文件应存在");
    assert!(!part_of(&out_path).exists(), "不得残留 .part");

    let err = stderr_of(&out);
    assert!(err.contains("[progress]"), "进度必须走 stderr：{err}");
    assert!(err.contains("预计打印时间"), "摘要必须走 stderr：{err}");

    let _ = std::fs::remove_file(&out_path);
    let _ = std::fs::remove_file(&input);
}

/// 省略 `-o` ⇒ 原地覆盖，且 stdout 报的是**输入路径**。
#[test]
fn run_without_out_overwrites_in_place() {
    let input = temp_copy("inplace");
    let before = std::fs::metadata(&input).expect("输入应存在").len();

    let out = run_cli(&[
        "run",
        input.to_str().unwrap(),
        "-c",
        config_a1().to_str().unwrap(),
    ]);
    assert_eq!(code_of(&out), 0, "stderr:\n{}", stderr_of(&out));
    assert_eq!(stdout_of(&out).trim(), input.display().to_string());

    let after = std::fs::metadata(&input).expect("输入应仍存在").len();
    assert_ne!(before, after, "原地模式应当把文件改了（大小必然变）");
    assert!(!part_of(&input).exists(), "不得残留 .part");
    let _ = std::fs::remove_file(&input);
}

/// **退出码 1 与 2 的分界线**（对应 `main.rs` 头注那段设计）：
/// 「模型超出打印边界」是 `PostprocError::InvalidConfig` 变体，但它是**处理失败**，
/// 必须退 1。若哪天有人改成按错误变体分类退出码，这条立刻红。
///
/// 触发方式：把喷头 Y 偏移推到 200mm ⇒ 涂胶路径落到 Y≈289.8mm，超出 A1 的 255mm。
/// **不用「换成 A1 mini」那个招**：实测那样跑得通（这份 A1 预设的 IR 在 A1 mini
/// 的床面上不越界），换机型不构成失败 —— 判据必须挑一个**真的**会失败的形态。
#[test]
fn processing_failure_exits_one_even_though_the_variant_says_config() {
    let input = temp_copy("boundary");
    let out = run_cli(&[
        "run",
        input.to_str().unwrap(),
        "-c",
        config_a1().to_str().unwrap(),
        "--set",
        "Toolhead.YOffset=200",
    ]);
    let err = stderr_of(&out);
    assert_eq!(code_of(&out), 1, "处理失败必须退 1，stderr:\n{err}");
    assert!(
        err.contains("E_GCODE_BOUNDARY_001"),
        "报错必须带诊断码：{err}"
    );
    assert!(
        stdout_of(&out).is_empty(),
        "失败时 stdout 必须为空（脚本不该拿到半个路径）"
    );
    let _ = std::fs::remove_file(&input);
}

/// 写不出去也是**处理失败**（退 1），且原地输入不受影响。
///
/// 这条与上一条是不同形态：错误变体是 `Io`、错误码甚至是 `E_FS_NOT_FOUND_001`
/// （长得像「输入不存在」那类），但它发生在管线里 ⇒ 仍然退 1，不退 2。
#[test]
fn unwritable_output_exits_one_and_leaves_input_alone() {
    let input = temp_copy("unwritable");
    let pristine = std::fs::read(&input).expect("读输入副本");
    let out = run_cli(&[
        "run",
        input.to_str().unwrap(),
        "-c",
        config_a1().to_str().unwrap(),
        "-o",
        "/nonexistent_dir_xyz/out.gcode",
    ]);
    let err = stderr_of(&out);
    assert_eq!(code_of(&out), 1, "stderr:\n{err}");
    assert!(err.contains("write_part"), "报错应点名失败的操作：{err}");
    assert_eq!(
        std::fs::read(&input).unwrap(),
        pristine,
        "写输出失败不该动输入"
    );
    let _ = std::fs::remove_file(&input);
}

#[test]
fn missing_input_exits_two() {
    let out = run_cli(&[
        "run",
        "/nonexistent/__nope__.gcode",
        "-c",
        config_a1().to_str().unwrap(),
    ]);
    assert_eq!(code_of(&out), 2);
    assert!(stderr_of(&out).contains("输入 G-code 不存在"));
}

#[test]
fn missing_config_exits_two() {
    let input = temp_copy("nocfg");
    let out = run_cli(&[
        "run",
        input.to_str().unwrap(),
        "-c",
        "/nonexistent/__nope__.toml",
    ]);
    assert_eq!(code_of(&out), 2);
    assert!(stderr_of(&out).contains("配置文件不存在"));
    let _ = std::fs::remove_file(&input);
}

/// 坏 `--set` 在**前置校验**就被拦（退 2），而且报错点名那个路径。
#[test]
fn bad_override_exits_two_and_names_the_path() {
    let input = temp_copy("badset");
    let out = run_cli(&[
        "run",
        input.to_str().unwrap(),
        "-c",
        config_a1().to_str().unwrap(),
        "--set",
        "Tower.NoSuchKey=1",
    ]);
    let err = stderr_of(&out);
    assert_eq!(code_of(&out), 2, "stderr:\n{err}");
    assert!(err.contains("NoSuchKey"), "报错必须点名写错的键：{err}");
    let _ = std::fs::remove_file(&input);
}

/// `init` 到 stdout：内容与库函数一致，且**能被自己读回去**。
#[test]
fn init_prints_a_config_that_parses() {
    let out = run_cli(&["init"]);
    assert_eq!(code_of(&out), 0);
    let text = stdout_of(&out);
    assert_eq!(
        text,
        postprocess::config::init_toml(),
        "init 与库函数应同源"
    );
    let parsed: toml::Value = toml::from_str(&text).expect("init 的产物必须是合法 TOML");
    assert!(
        parsed.as_table().is_some_and(|t| t.contains_key("Tower")),
        "init 的产物应含 Tower 段"
    );
}

/// `init -o` **不覆盖已存在的文件**（静默盖掉等于删用户数据）。
#[test]
fn init_refuses_to_overwrite() {
    let path = std::env::temp_dir().join("mkpse-pp-cli-init-existing.toml");
    std::fs::write(&path, "# 用户改过的东西\n").expect("写占位文件");
    let out = run_cli(&["init", "-o", path.to_str().unwrap()]);
    assert_eq!(code_of(&out), 2, "stderr:\n{}", stderr_of(&out));
    assert!(stderr_of(&out).contains("拒绝覆盖"));
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "# 用户改过的东西\n",
        "原文件必须一个字节都不动"
    );
    let _ = std::fs::remove_file(&path);
}

/// `dump-ir`：机型表填充**已发生**（这正是它与配置文件不同的地方）。
#[test]
fn dump_ir_shows_machine_table_filled_values() {
    let out = run_cli(&["dump-ir", "-c", config_a1().to_str().unwrap()]);
    assert_eq!(code_of(&out), 0, "stderr:\n{}", stderr_of(&out));
    let v: serde_json::Value = serde_json::from_str(&stdout_of(&out)).expect("必须是合法 JSON");

    // 配置文件里这些是 0（`build9/A1-standard.json` 的原值），机型表填充后才有真值。
    assert_eq!(v["Machine"]["MaxX"], 260.0);
    assert_eq!(v["Machine"]["MaxY"], 255.0);
    // 没给输入 ⇒ **不写** GcodeHeaderFacts 这个键（给空对象会被读成「刮出来是空的」）
    assert!(
        v.get("GcodeHeaderFacts").is_none(),
        "不带 --gcode 时不该有 GcodeHeaderFacts 键"
    );
    // 没给输入时 GcodeMachineType 走与「检测不到」同一条兜底
    assert_eq!(v["Machine"]["GcodeMachineType"], "UNKNOWN");
}

/// `dump-ir --gcode`：追加 4 条头部刮取事实，并真实检测出 G-code 自报的机型。
#[test]
fn dump_ir_with_gcode_adds_header_facts() {
    let out = run_cli(&[
        "dump-ir",
        "-c",
        config_a1().to_str().unwrap(),
        "--gcode",
        golden_input().to_str().unwrap(),
    ]);
    assert_eq!(code_of(&out), 0, "stderr:\n{}", stderr_of(&out));
    let v: serde_json::Value = serde_json::from_str(&stdout_of(&out)).expect("必须是合法 JSON");
    let facts = &v["GcodeHeaderFacts"];
    assert_eq!(facts["MachinePresetName"], "Bambu Lab A1 mini 0.4 nozzle");
    assert_eq!(facts["ProcessPresetName"], "0.20mm Standard @BBL A1M");
    assert_eq!(facts["SlicerIroningEnabled"], true);
    assert_eq!(facts["ThreeMfFileName"], "42274.2_.gcode.3mf");
    // 这份输入是 A1 mini 切出来的，而配置写的是 A1 ⇒ 两个字段**本来就该不一样**
    assert_eq!(v["Machine"]["GcodeMachineType"], "A1_MINI");
    assert_eq!(v["Machine"]["MachineType"], "A1");
}

/// `check`：好配置退 0 且报出**判定依据**；机型名不认识退 2。
#[test]
fn check_reports_evidence_and_rejects_unknown_machine() {
    let ok = run_cli(&["check", "-c", config_a1().to_str().unwrap()]);
    let ok_err = stderr_of(&ok);
    assert_eq!(code_of(&ok), 0, "stderr:\n{ok_err}");
    assert!(ok_err.contains("机型 A1"), "必须报机型：{ok_err}");
    assert!(ok_err.contains("涂胶区"), "必须报涂胶区范围：{ok_err}");

    let bad = run_cli(&[
        "check",
        "-c",
        config_a1().to_str().unwrap(),
        "--set",
        "Machine.MachineType=NoSuchPrinter",
    ]);
    let bad_err = stderr_of(&bad);
    assert_eq!(code_of(&bad), 2, "stderr:\n{bad_err}");
    assert!(
        bad_err.contains("NoSuchPrinter"),
        "必须点名那个写错的机型：{bad_err}"
    );
    // 文案不许再提「预设文件 / # machine:」—— 本项目没有这两样东西
    assert!(
        !bad_err.contains("# machine:") && !bad_err.contains("预设文件"),
        "错误文案还在说预设文件（来源仓库的措辞漏搬）：{bad_err}"
    );
}

/// `--set` 从命令行真的能改到结果里（走完整条链，不是只测解析）。
#[test]
fn set_from_command_line_reaches_the_output() {
    let input = temp_copy("setreach");
    let out = run_cli(&[
        "dump-ir",
        "-c",
        config_a1().to_str().unwrap(),
        "--set",
        "Tower.SafeZOffset=1.25",
    ]);
    assert_eq!(code_of(&out), 0, "stderr:\n{}", stderr_of(&out));
    let v: serde_json::Value = serde_json::from_str(&stdout_of(&out)).expect("必须是合法 JSON");
    assert_eq!(v["Tower"]["SafeZOffset"], 1.25);
    let _ = std::fs::remove_file(&input);
}
