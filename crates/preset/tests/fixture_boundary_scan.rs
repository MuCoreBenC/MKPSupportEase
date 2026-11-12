//! 夹具边界闸：**普通单元测试不许读生成基线**（对照基线目录）。
//!
//! # 为什么要有这一条
//!
//! 生成基线是**生成器的对照物**：`sync_baseline` 会跟着业务改名自动搬走它的文件
//! （改一次版本 id，`A1_MINI-fast.toml` 就变成 `A1_MINI-fast261111.toml`）。
//! 测试读它 ⇒ 业务改名一次，一批只关心「解析 / 写入 / 字段规则」的测试集体变红。
//! 2026-11-12 实测：四个文件因为这一次改名同时挂掉，而代码一行没动。
//!
//! 所以判据输入分三处，各有各的稳定来源：
//!
//! | 输入 | 谁用 | 稳定性 |
//! |---|---|---|
//! | `fixtures/contract/` | 解析 / 写入 / 字段规则 | 自造，名字与业务无关 |
//! | `fixtures/ir/presets/` | IR 构建链路 | 与 `build9/*.json` 成对冻结 |
//! | `assets/presets/` | 「当前参数」的行为判据 | 跟随参数更新，不跟随改名 |
//!
//! # 扫描口径
//!
//! 只看**测试那部分**：`tests/` 下的文件整份算，`src/` 下的从 `#[cfg(test)]` 起算。
//! 生产代码里的 `sync_baseline(assets, fixtures_dir())` 是正当的，不该被这条闸拦下。
//!
//! 注释行不参与（`tests/pipeline_wiping_source.rs` 里有一句解释搬家的注释提到那个路径）。

#![allow(clippy::disallowed_methods)]

use std::path::{Path, PathBuf};

/// 生成基线的两种入口：目录路径、以及那个算目录的函数。
const BANNED: &[&str] = &["fixtures/presets/", "fixtures_dir("];

/*
 * 白名单：**生成器与它自己的对照物**之间的判据。基线就是被测对象
 * （「产物与基线配对」「产物与基线逐字节相同」「基线里的每份都能过两层校验」），
 * 换成任何别的输入，判据就空了。
 *
 * 往这里加一条之前先回答一句：**为什么这份输入非它不可**？
 * 答案如果是「省事」「顺手」，那该去 `fixtures/contract/` 或 `assets/presets/`。
 */
const ALLOWED: &[&str] = &[
    "crates/preset/src/generate.rs",
    "crates/preset/src/validate.rs",
    "src-tauri/src/workbench/app/build.rs",
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            rs_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// `tests/` 下的文件整份算测试；其余文件从 `#[cfg(test)]` 起算。
///
/// 返回 `(测试那部分, 前面有多少行)` —— 报错时要给**文件里的真实行号**。
fn test_portion(rel: &str, text: &str) -> Option<(String, usize)> {
    let in_tests_dir = Path::new(rel)
        .components()
        .any(|c| c.as_os_str() == "tests");
    if in_tests_dir {
        return Some((text.to_owned(), 0));
    }
    text.find("#[cfg(test)]").map(|i| {
        let skipped = text[..i].matches('\n').count();
        (text[i..].to_owned(), skipped)
    })
}

/// 去掉整行注释与空行 —— 只留会被编译的代码。
fn code_lines(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.lines().enumerate().filter(|(_, l)| {
        let t = l.trim_start();
        !(t.is_empty() || t.starts_with("//"))
    })
}

/// 一份文件里的违例：`(1 基行号, 原行)`。
fn violations(rel: &str, text: &str) -> Vec<(usize, String)> {
    if ALLOWED.contains(&rel) {
        return Vec::new();
    }
    let Some((portion, skipped)) = test_portion(rel, text) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (idx, line) in code_lines(&portion) {
        if BANNED.iter().any(|n| line.contains(n)) {
            out.push((skipped + idx + 1, line.trim().to_owned()));
        }
    }
    out
}

#[test]
fn unit_tests_do_not_read_the_generation_baseline() {
    let root = repo_root();
    let mut files = Vec::new();
    rs_files(&root.join("crates"), &mut files);
    rs_files(&root.join("src-tauri/src"), &mut files);

    let this_file = "fixture_boundary_scan.rs";
    let mut offenders: Vec<String> = Vec::new();
    let mut scanned = 0usize;

    for path in &files {
        if path.file_name().is_some_and(|n| n == this_file) {
            continue; // 本文件的判据字符串会匹配到自己
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let rel = path
            .strip_prefix(&root)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        if test_portion(&rel, &text).is_none() {
            continue; // 没有测试部分的文件不扫
        }
        scanned += 1;
        for (line, src) in violations(&rel, &text) {
            offenders.push(format!("{rel}:{line}  {src}"));
        }
    }

    assert!(
        scanned > 20,
        "反空转：只扫到 {scanned} 个带测试的文件，路径找错了？"
    );
    assert!(
        offenders.is_empty(),
        "普通单元测试不许读生成基线（它会被生成动作搬走）。\n\
         解析 / 写入 / 字段规则 → `tests/fixtures/contract/`；\n\
         IR 构建链路 → `tests/fixtures/ir/`；\n\
         当前参数 → `crates/preset/assets/presets/`。\n\
         命中 {} 处：\n{}",
        offenders.len(),
        offenders.join("\n")
    );
    println!("夹具边界绿：{scanned} 个带测试的文件都没读生成基线");
}

/// 闸的自证：它必须真能抓到违规，也必须不误伤三种正当写法。
#[test]
fn the_scan_would_catch_a_baseline_read() {
    let caught = violations(
        "crates/preset/tests/fake.rs",
        "fn t() {\n    let d = preset::generate::fixtures_dir();\n}\n",
    );
    assert_eq!(
        caught.len(),
        1,
        "直接调 fixtures_dir() 必须被抓到：{caught:?}"
    );
    assert_eq!(caught[0].0, 2, "行号要对：{caught:?}");

    let by_path = violations(
        "crates/postprocess/tests/fake.rs",
        "fn t() {\n    let p = root.join(\"tests/fixtures/presets/a.toml\");\n}\n",
    );
    assert_eq!(by_path.len(), 1, "直接拼基线路径必须被抓到：{by_path:?}");

    // ① 注释里提到不算依赖
    let comment_only = violations(
        "crates/postprocess/tests/fake.rs",
        "//! 输入搬到 ir/presets，与 fixtures/presets 不再是一回事\nfn t() {}\n",
    );
    assert!(
        comment_only.is_empty(),
        "注释不该被算成依赖：{comment_only:?}"
    );

    // ② 生产代码那部分（`#[cfg(test)]` 之前）不算测试
    let production = violations(
        "crates/preset/src/fake.rs",
        "fn gen() {\n    let d = fixtures_dir();\n}\n#[cfg(test)]\nmod tests {\n    fn t() {}\n}\n",
    );
    assert!(
        production.is_empty(),
        "生产代码调 sync_baseline 是正当的：{production:?}"
    );

    // ③ 白名单文件里的确不算
    let allowed = violations(
        "crates/preset/src/generate.rs",
        "#[cfg(test)]\nmod tests {\n    fn t() { let _ = fixtures_dir(); }\n}\n",
    );
    assert!(allowed.is_empty(), "白名单文件不该被拦：{allowed:?}");

    println!("夹具边界闸自证绿：抓得到违规，也不误伤注释 / 生产代码 / 白名单");
}
